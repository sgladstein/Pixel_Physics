# M19 research: visual polish — rendering techniques, palette theory, wgpu/pixels feasibility

Raw findings kept in full here because `PLAN.md`'s M19 section only carries
a condensed synthesis and execution plan. This is the source material to
build the actual M19 tiers against.

Three parallel research passes: how other falling-sand engines actually
render (not simulate) their materials; what `pixels`/wgpu concretely
support for custom rendering; and pixel-art palette/colour theory.

---

## 1. How other falling-sand engines render (not simulate)

### Noita

GDC 2019 talk ["Exploring the Tech and Design of Noita"](https://www.youtube.com/watch?v=prXuyMCgbTc)
([GDC Vault](https://www.gdcvault.com/play/1025695/Exploring-the-Tech-and-Design),
[recap](https://braindump.jethro.dev/posts/gdc_vault_exploring_the_tech_and_design_of_noita/))
covers simulation architecture (64x64 chunks, dirty rects, marching-squares
rigid bodies) but is **not** a rendering deep-dive — publicly available
sources don't document a bespoke lighting shader. What *is* documented, via
the [Noita Wiki "Light" page](https://noita.wiki.gg/wiki/Light) and
[Fandom wiki](https://noita.fandom.com/wiki/Light):

- **Darkness/light isn't a true dynamic-lighting system** — it's a
  particle-based "fog of war hole" (literally an asset named
  `fog_of_war_hole_128.xml`, internally an "explosion" particle) that
  punches soft-edged, alpha-falloff circles into a darkness/fog layer.
  Light sources (torches, light spells, explosions) just spawn more of
  these circles, with radius/alpha additively stacking.
- **Implication for this engine:** a full per-pixel lighting model isn't
  necessary to get a "Noita-like" feel — a cheap second buffer
  (fog/visibility layer) with soft radial falloff blended multiplicatively
  over the color framebuffer, populated by circles from a short list of
  active light sources (fire pixels, explosions, lava), gets ~80% of the
  effect at low cost. CPU-side, no shader required.
- Materials like `Glowing Matter` and `Fire` are simply emissive-tinted
  sprites/pixel colors, not physically lit — the "glow" read is mostly
  saturated warm color + the darkness-mask contrast around it, not a bloom
  pass.

### The Powder Toy

Primary source, actual shipped C++ renderer:
[`Renderer.cpp`](https://github.com/The-Powder-Toy/The-Powder-Toy/blob/master/src/graphics/Renderer.cpp).
Concrete, cheap, CPU-composited techniques directly applicable to a
`pixels`-crate framebuffer:

- **`PMODE_GLOW`**: additive radial blend — draw full-intensity color at
  the pixel center, then add the same color at ~96/255 alpha to immediate
  neighbors, decaying outward to a ~5-6px radius (`cola1 = 5*cola/255`).
  Literally a tiny hand-rolled box/radial blur done per glowing pixel,
  cheap enough to run on CPU for a bounded number of hot pixels.
- **`PROP_HOT_GLOW`**: temperature-driven color shift — once a particle
  exceeds `HighTemperature − 800F`, RGB channels get sinusoidally
  modulated (`colr += sin(gradv*addr)*226`, etc.) so hot material color
  cycles/brightens with heat rather than using a flat lookup table.
  Directly reusable for this engine's existing per-cell heat field.
- **`PMODE_BLUR`**: 7x7 neighborhood blend with falloff alpha (~20-30/255)
  — a manual blur kernel, not a shader pass.
- **`PMODE_FLARE`**: velocity-dependent glow gradient
  (`gradv = flicker + |vx|*17 + |vy|*17`) with `1/1.2` per-step decay —
  used for sparks/energy particles.
- **Heat/pressure display modes**: automatic gradient mapping (dark blue ->
  pink low-to-high temp), and pressure visualization mixes color by both
  pressure and temperature simultaneously (green=high pressure,
  purple=low, yellow=hot+high-pressure). Simple ramp-based, no shader
  needed — directly portable to this engine's coarse field grid (M13).

### Sandspiel (Max Bittker)

[Devlog](https://maxbittker.com/making-sandspiel/),
[source](https://github.com/MaxBittker/sandspiel). Its "attractive despite
simplicity" look comes from one specific trick:

- Each cell has an 8-bit `ra` register that's reused for rendering
  brightness — same-species pixels get slightly randomized brightness from
  this per-particle register, producing organic grain/texture "for free"
  without any explicit dithering algorithm or extra pass. **This is the
  single cheapest, most directly applicable technique found:** store a
  small per-cell random brightness jitter (this engine's `Cell::shade`
  field is almost exactly this slot already, currently only used to *pick*
  a palette entry rather than also modulate brightness) and modulate the
  flat material color by +/-N% at render time. Kills the "flat/plasticky"
  look almost for free, pure CPU-side, no shader.
- Rendering itself is a straight cell-buffer -> RGBA texture upload
  (WebGL), analogous to how `pixels` blits a CPU framebuffer — confirms
  this class of engine doesn't need per-pixel shading to look good; the
  win is in the color/brightness *content* written per pixel, not the
  blit path.

### General/adjacent — tile lightmap flood-fill

A 2D game devlog (["Devlog #10 — Lighting"](https://medium.com/@nerudaj/devlog-10-lighting-0d010414216b),
for the game *Rend*) documents a simple discrete lightmap: seed light
sources on a coarse grid, flood-fill brightness outward with a per-source
decay rate (e.g. 32 vs 64 per tile), then sample/dither against a noise
texture at render time. Cheap (single scan + flood fill, no per-frame
dynamic recompute needed unless sources move), and maps well onto this
engine's existing coarse field grid — M13's pressure/velocity/temperature/
light grid already has a "light" channel; this is exactly the algorithm to
populate/propagate it.

### General bloom reference

Standard technique ([LearnOpenGL bloom](https://learnopengl.com/Advanced-Lighting/Bloom)):
threshold-extract bright pixels to a second buffer, downsample/blur,
additive-composite back. Needs a second render target and a blur shader —
more machinery than Powder Toy's per-pixel radial-add trick. Given
`pixels`' simple GPU-blit setup, **the Powder Toy-style CPU-side additive
glow is the lower-effort, more idiomatic fit** than a true screen-space
bloom pipeline; true bloom would require inserting a small wgpu
post-process pass (feasible, see section 2, but a bigger lift).

---

## 2. What `pixels`/wgpu concretely support

### Does `pixels` support a custom GPU pipeline?

**Yes — a documented, first-class extension point, not a hack.**
`Pixels::render_with(closure)` hands over `&mut wgpu::CommandEncoder`,
`&wgpu::TextureView` (the surface render target), and `&PixelsContext`.
`PixelsContext` exposes `device`, `queue`, `texture`/`texture_extent`/
`texture_format` (the raw CPU-uploaded framebuffer texture, written by the
`&mut [u8]` buffer this engine already writes into), and
`scaling_renderer: ScalingRenderer` (the built-in nearest-neighbor upscale
blit). Arbitrary wgpu render/compute passes can be inserted before or after
the default scaling blit within that same closure. `PixelsBuilder`
additionally exposes `render_texture_format()` (lets you use e.g.
`Rgba16Float` for HDR-range bloom accumulation) and `surface_texture_format()`.

The repo ships exactly this pattern as the
[`custom-shader` example](https://github.com/parasyte/pixels/tree/main/examples/custom-shader)
([source](https://raw.githubusercontent.com/parasyte/pixels/main/examples/custom-shader/src/renderers.rs)).
It builds a `NoiseRenderer` that: creates its own offscreen texture, a
fullscreen triangle vertex buffer (the documented
`[-1,-1],[3,-1],[-1,3]` trick, see
[pixels#180](https://github.com/parasyte/pixels/issues/180)), a sampler, a
bind group (source texture + sampler + a small uniform buffer for `time`),
and a `wgpu::RenderPipeline` with a WGSL shader
(`wgpu::include_wgsl!`). Its `render()` opens its own `RenderPassDescriptor`
targeting the given `render_target` view, sets pipeline/bind
group/vertex buffer, and issues `draw(0..3, 0..1)`. In `main.rs` this is
invoked inside `pixels.render_with(|encoder, render_target, context| {
context.scaling_renderer.render(encoder, noise_texture);
noise_renderer.render(encoder, render_target, clip_rect); Ok(()) })` — i.e.
scale-then-postprocess, chained passes in one encoder. **This is the exact
template to copy for a bloom/emissive pass**: swap the noise WGSL for a
two-pass separable blur, source from a separate emissive-only offscreen
texture instead of noise.

### CPU-only techniques (no shader, write directly into the framebuffer before upload)

- **Cheap bloom/glow**: build a small "emissive" luminance buffer (e.g.
  `(temp - threshold).max(0)` for hot cells, or a fixed value where
  `burning==true`), then apply a separable box blur 2-3 times (box blur
  repeated ~3x approximates a Gaussian, central limit theorem) at reduced
  resolution (e.g. blur at 1/4 size, bilinear-upsample, additive-blend onto
  the main buffer: `out = min(255, base + bloom*intensity)`). A single
  horizontal pass + single vertical pass per iteration keeps it O(N) per
  pixel regardless of radius (running/sliding-window sum, not
  O(N*radius)).
- **Ordered (Bayer) dithering**: precompute a 4x4 threshold matrix
  `[[0,8,2,10],[12,4,14,6],[3,11,1,9],[15,7,13,5]]`, normalize to 0..1 by
  `(v+0.5)/16`. For each pixel, compute a fractional color error against
  the nearest palette step and add `(bayer[y%4][x%4]-0.5)*stepSize` before
  quantizing/rounding — breaks up flat material-color banding without
  adding palette entries, spatially stable across frames (good for a
  simulation that redraws every tick, unlike temporal dithering which
  would flicker). References:
  [Wikipedia](https://en.wikipedia.org/wiki/Ordered_dithering),
  [worked example with code](https://blog.42yeah.is/rendering/2023/02/18/dithering.html).
- **Fake AO / depth-to-surface darkening**: for each solid cell, count the
  number of empty/gas neighbors in a small radius (3x3, or a cheap 5-tap
  plus/cross sample); darken proportional to "enclosedness"
  (`brightness = 1.0 - occlusion_strength * enclosed_neighbor_fraction`).
  Cheaper alternative: precompute a per-chunk or per-cell "distance to
  nearest empty cell" via a couple of BFS/erosion passes (flood-fill from
  empty cells outward, incrementing distance), map distance -> darkening
  curve. Since the grid is chunked, this can run only on dirty chunks per
  frame.

### If going the shader route: minimal bloom + coarse lighting architecture

- **Emissive/bloom**: maintain a second small buffer holding emissive
  intensity (could be generated CPU-side from the same loop that already
  writes material colors, or computed at the existing 1/8-resolution
  secondary field grid since temperature is already tracked there). Upload
  as its own `wgpu::Texture` (mirror `NoiseRenderer::create_texture_view`),
  run a two-pass separable Gaussian/box-blur (ping-pong between two
  textures, horizontal then vertical), additive-blend the blurred result
  onto the scaled main texture in a final composite pass — three chained
  passes in the same `render_with` closure: main scale -> blur ping-pong
  -> composite.
- **2D lighting reusing the existing coarse field grid**: since the field
  grid already computes a "light" field per-cell at 1/8 resolution once per
  frame, this is the natural light source — no need for real GPU
  raymarching. A practical, cheap-on-CPU approach used widely in tile games
  (Minecraft/Terraria-style): BFS/flood-fill light propagation — seed a
  priority queue with light-emitting cells (fire, lava) at full intensity,
  propagate outward to 4/8-connected neighbors, subtracting a falloff
  amount per step, multiplying by an attenuation factor when crossing
  solid/opaque material. Using the coarse grid's resolution keeps this
  cheap (~40x64 cells instead of 512x320). Store the resulting light-
  intensity grid, upload as a low-res texture, let a fragment shader
  upsample (bilinear or a simple blur) and multiply it against the main
  color texture in the composite pass — soft shadows/occlusion "for free"
  from material solidity without true raycasting. References:
  [0fps.net voxel/flood-fill lighting](https://0fps.net/2018/02/21/voxel-lighting/),
  [FIFO-queue flood-fill demo (YouTube)](https://www.youtube.com/watch?v=7y1vdiz7vGE).

### Cost estimates

- **Weekend-scale**: Bayer dithering (a few hours, pure CPU, no new
  textures); fake AO via neighbor-density darkening (half a day); CPU
  box-blur bloom on an emissive sub-buffer blended additively before
  upload (a day, no wgpu code at all).
- **~1 week**: wiring a real `render_with` custom pass (copy the
  `custom-shader` example's boilerplate: texture, sampler, bind group,
  pipeline, WGSL) for a GPU-side bloom pass fed from an emissive texture
  already computed CPU-side — most of the week is wgpu bind-group/pipeline
  plumbing, not the blur math itself.
- **Multi-week**: a true propagated 2D lighting system (BFS flood-fill or
  GPU jump-flood) with soft shadows from occluders, color-tinted light
  (RGB channels propagated separately), and smooth reintegration with the
  coarse field grid's existing pressure/velocity/temperature/light fields
  — the propagation algorithm is simple, but tuning falloff curves,
  handling multi-source overlap, avoiding light "popping" as chunks
  load/unload, and staying performant at full 512x320 resolution is
  genuinely multi-week polish work.

### References (section 2)

- [pixels crate (GitHub)](https://github.com/parasyte/pixels)
- [pixels custom-shader example](https://github.com/parasyte/pixels/tree/main/examples/custom-shader)
- [pixels custom-shader renderers.rs (raw source)](https://raw.githubusercontent.com/parasyte/pixels/main/examples/custom-shader/src/renderers.rs)
- [Pixels::render_with docs](https://docs.rs/pixels/latest/pixels/struct.Pixels.html)
- [PixelsContext docs](https://docs.rs/pixels/latest/pixels/struct.PixelsContext.html)
- [PixelsBuilder docs](https://docs.rs/pixels/latest/pixels/struct.PixelsBuilder.html)
- [Fullscreen-triangle trick discussion, pixels#180](https://github.com/parasyte/pixels/issues/180)

---

## 3. Pixel-art palette and color theory

### How cohesive sandbox/pixel games choose material palettes

Lospec (the de facto pixel-art palette repository/community) frames good
palettes as **organized color ramps** — each material/color family gets a
light->dark ramp of ~3-5 steps, and ramps for different families sit
adjacently so hue, saturation, and value stay in a shared "grade." With a
small total color budget (16-64 colors), every color must justify its slot
— what makes a palette read as *designed* rather than random. Key
structural rule found repeatedly: **shift hue while shifting value** —
don't just darken/lighten a color, rotate darks toward blue/purple and
desaturate, rotate lights toward yellow and reduce saturation slightly.
This single trick is what unifies otherwise-independent material ramps into
one "grade."

### Concrete methodology for material-family palettes

- Use **HSL, not raw RGB**, to build families: fix hue per material
  category (e.g. ~30-45 degrees for earth/solids, ~190-210 for
  liquids/cool, ~0-25 for fire/hot, near-desaturated for gas/smoke), then
  vary **only lightness** (and slightly saturation) for per-cell grain —
  guarantees texture without hue drift/noise.
- Keep a **consistent saturation band across categories** (e.g. mid-tier
  materials all sit 45-65% saturation) so no single material's ramp looks
  washed-out or gaudy next to another; reserve the *highest* saturation
  exclusively for hot/energetic materials (fire, lava, electricity) so they
  visually "pop" as the outlier.
- Distinguish adjacent/interacting materials (sand vs. water vs. gravel)
  primarily via **hue separation**, not just value — value-only differences
  disappear at a glance when pixels are small and dithered together.
- Cap total distinct hues to a small handful (5-8) across the whole game;
  every material borrows from that set rather than inventing a new hue.

### Making fire/glow read without a lighting engine — pure palette trick

Pair peak saturation with peak lightness *only* for hot materials — nothing
else in the palette should simultaneously be that bright and that
saturated, and that specific combination is what the eye reads as
"emitting," not just "colored." Classic warm/cool contrast: keep the rest
of the palette (earth, water, stone) moderately desaturated/cooler-leaning,
so a small patch of high-chroma orange/yellow/white reads as foreign and
luminous by contrast.
[Fire And Ice](https://lospec.com/palette-list/fire-and-ice)
(`#ec1c5e #a11c7c #5a2172 #371ca1 #1c76ec`) is a worked example of bridging
a hot ramp into a cool one for legibility.

### Adaptable named palettes (hex codes)

- **[Resurrect 64](https://lospec.com/palette-list/resurrect-64)** (Kerrie
  Lake) — 64 colors, organized as ramps: browns/earth (`#7a3045 #9e4539
  #cd683d #e6904e #fbb954`), reds/oranges/fire (`#6e2727 #b33831 #ea4f36
  #f57d4a #ae2334 #e83b3b #fb6b1d #f79617 #f9c22b`), greens (`#165a4c
  #239063 #1ebc73 #91db69 #cddf6c`), teals/water (`#0b5e65 #0b8a8f #0eaf9b
  #30e1b9 #8ff8e2`), blues, neutrals/grays. Spans earth+fire+water+gas
  families in one cohesive grade already — good single-source starting
  point.
- **[Endesga 32](https://lospec.com/palette-list/endesga-32)** — 32 colors,
  warmer/more saturated than DB32, strong earth/fire/stone spread:
  `#be4a2f #d77643 #ead4aa #e4a672 #b86f50 #733e39 #3e2731 #a22633 #e43b44
  #f77622 #feae34 #fee761 #63c74d #3e8948 #265c42 #193c3e #124e89 #0099db
  #2ce8f5 #ffffff #c0cbdc #8b9bb4 #5a6988 #3a4466 #262b44 #181425 #ff0044
  #68386c #b55088 #f6757a #e8b796 #c28569`. Designed for a game, not
  decorative art — very usable as-is.
- **[Lava-GB](https://lospec.com/palette-list/lava-gb)** — 4-color hot
  ramp: `#051f39 #4a2480 #c53a9d #ff8e80`.
- **[Fire And Ice](https://lospec.com/palette-list/fire-and-ice)** —
  `#ec1c5e #a11c7c #5a2172 #371ca1 #1c76ec`.
- **[Soul Fire](https://lospec.com/palette-list/soul-fire)** — cyan-flame
  variant if a "cold fire"/plasma material is ever wanted: `#000000
  #ffffff #018387 #01a2a7 #01bdc3 #0bcfd5 #32e8ee #6df8fc #90fcff #c5fdff`.
- **[Eerie Glow](https://lospec.com/palette-list/eerie-glow)** — small
  glow-in-dark ramp: `#a9d6ba #85ada9 #55646d #40434f #1e1e25`.

**Recommended approach**: adopt Resurrect 64 or Endesga 32 wholesale (or a
curated subset) as the master palette, assign each material family a
contiguous ramp from it, and apply the HSL hue-shift-while-varying-value
rule for per-cell grain rather than freehand-picking new RGB values per
material.

### References (section 3)

- [Lospec Palette List](https://lospec.com/palette-list)
- [Resurrect 64 Palette](https://lospec.com/palette-list/resurrect-64)
- [Endesga 32 Palette](https://lospec.com/palette-list/endesga-32)
- [Lava-GB Palette](https://lospec.com/palette-list/lava-gb)
- [Fire And Ice Palette](https://lospec.com/palette-list/fire-and-ice)
- [Soul Fire Palette](https://lospec.com/palette-list/soul-fire)
- [Eerie Glow Palette](https://lospec.com/palette-list/eerie-glow)
- [The Pixel Art Color Palette Guide (DEV.to)](https://dev.to/krila_software/the-pixel-art-color-palette-guide-how-to-choose-colors-that-work-1bg7)
- [Color Theory for Pixel Artists: It's All Relative (Pixel Parmesan)](https://pixelparmesan.com/blog/color-theory-for-pixel-artists-its-all-relative)
- [Creating Pixel Art Using Color Theory (Munsell Color)](https://munsell.com/color-blog/creating-pixel-art-color-theory/)

---

## Summary: what this means for the M19 build

See `PLAN.md`'s M19 section for the tiered execution plan built from this
research. In short: tiers 1-3 (palette overhaul, per-cell brightness
jitter, temperature-driven color shift, Bayer dithering, Powder-Toy-style
radial glow, fake AO, coarse flood-fill light propagation on the existing
M13 light channel) are all CPU-side, need no shader pipeline, and are
self-verifiable via the same in-app framebuffer-dump screenshot technique
used for M7/M15. Tier 4 (real GPU bloom via `Pixels::render_with`) is
confirmed feasible and has a direct implementation template
(`custom-shader` example) but still needs live human visual judgment, so it
stays folded into M6's deferral.
