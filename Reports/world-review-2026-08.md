# World review, August 2026: what the generated world looks like, and where it goes next

A planning-session review of the world itself — worldgen, water, underground,
graphics — run as a multi-agent review over *rendered images* of generated
worlds, per the method rule that has never yet been wrong here: look first.
This report carries the findings, the owner's decisions folded in as
directives, the rivers/waterfalls position, a sequenced roadmap with
per-item delegation notes for a cheaper implementation model, and the
landmines appendix that must travel with every delegated task.

Companion artifacts, same change: the review workflow itself
(`.claude/workflows/world-review.js`, re-runnable) and four rendered strips
under `Reports/img/world-review/`. **These are the first images of a
generated world ever committed to this repo** — sixty-two screenshots exist
under `docs/screenshots/` and not one shows what `worldgen::generate`
builds. Every other render referenced below regenerates exactly (worlds are
deterministic per seed; `tests/worldgen.rs` pins it) with the commands
given inline.

Baseline measured this session (`cargo run --release --example ascii`,
release build, this machine, 2026-08-19 — re-measure before comparing):
- Settled scenes: **0 chunks awake**, worst frames 0.4–3.1 ms; the
  dirty-rect render skip reads 0.002 ms.
- Full-screen stress + field, parallel driver: **worst 19.9 ms** (serial
  106.8 ms).
- The shipped 2048×640 world builds in **542 ms** (place 299 ms,
  structural pass 243 ms) — paid on start and on every F6/F7/F8/R.
- Field cost at 2048×1280: settled 0.008 ms / stepping 7.2 ms.

## §1 What the images show

Render matrix: all six presets × seed 1, rolling and canyon × seeds
{2, 7, 13}, plus rain, night, and mined variants — fifteen strips, each a
four-shot viewport traverse of a full 2048×640 world.
Regenerate: `cargo run --release --example viewshot -- seed=N preset=P shots=4`
(plus `rain=wet`, `frame=1800`, `mine=1 settle=240` for the variants).

**The good, first.** The strongest visual asset the game has today is the
sky: the dusk shot of `rolling-night.png` — orange horizon glow thinning
into a starfield over a silhouetted ridge, moon with a halo in the last
shot — is genuinely beautiful, and it is *earned*, because the same clock
drives plant light economics. Rock strata read well: tilted, gently folded
bands that a cut face exposes, followable across the world. Seed variety is
real: rolling seed 7 is a coastal world (a lake with a sand beach, a bright
sand bowl, water on the far horizon), seed 2 is a lake district with
islands, seed 1 is dry uplands — four seeds genuinely feel like four
places. And canyon seed 7 contains the best landscape yet generated:
proper flat-topped buttes with sheer stratified faces and brow overhangs,
straight out of the American Southwest (committed as
`img/world-review/canyon-s7.png`).

**The problems, in descending order of what they cost the picture:**

1. **Worlds arrive visually barren.** `life_scatter` places *seeds* — one
   pixel each — and sparse moss. At the default settle there is no visible
   vegetation anywhere in any strip; `wetland`, whose whole identity is
   lushness, reads as a mud flat with a pond. The mined strip (240-frame
   settle) shows the tell: tiny green sprouts appearing along the soil
   line. The world is *populated* but not *grown* — first impressions are
   of a dead planet, and the fix (generating a world already aged, or
   pre-running growth at build time) is the single largest beauty lever
   available. This is `worldgen(seed, coord, world_age)` from
   `worldgen-design.md` §6 arriving earlier than streaming needs it.
2. **Presets don't differentiate at play scale.** `rolling-s1`,
   `terraced-s1` and `wetland-s1` are near-indistinguishable at strip
   scale: same gray massif, same near-black soil crust, same pale pocket
   spots. Only `canyon` (sand, mesas) and `arid` (dune teeth) separate.
   Within a single world it is worse: regions vary the *heightfield* but
   not the *identity* — walking 2048 cells crosses escarpments but never a
   visibly different country. The variability the owner wants does not yet
   exist on screen, and the region `Character` vector that should drive it
   already exists in `region.rs` and drives almost nothing visible but
   elevation and sand-vs-soil.
3. **The palette is a monochrome.** Gray rock, near-black soil, near-black
   water, blue sky. The soil profile the wiki promises (dark rich top,
   paler below) is invisible at play scale; standing water renders so dark
   it reads as holes in the world. The world needs hue, and materials are
   data (`assets/materials/*.ron`) — this is cheap territory.
4. **Arid dunes read as a mechanical sawtooth.** One wavelength repeated
   ~30 times across the world, and the teeth are *rock* — the dune
   function displaces the stone surface, with only a dusting of sand — so
   the preset that should read as a sand sea reads as a zigzag rock
   ridge. (`arid-s1.png`, x≈250–1700.)
5. **A recurring "keyhole" artifact.** One-to-two-column vertical slots
   cut the full height of cliffs and mesas: canyon-s7 x≈590–680 (two
   parallel slots through the mesa), canyon-s13 x≈205, rolling-s1 x≈1300,
   rolling-s2 x≈500. It looks like a terrace/brow interaction bug, is
   visible at every zoom, and any formation work will inherit it if it is
   not traced first.
6. **Every pond draws a pale dashed line at its surface** — first read
   (mine and two lenses') as the monolayer/whisker artifact. The
   prosecutor overturned that: the whisker mechanism *requires air below*
   (`open-bugs-handoff.md` §1) and a settled pond's surface cells have
   water below, so it cannot fire there. A 6× crop shows the dashes are
   **shoreline sand** — the palette's only saturated hue — plus the fact
   that `water.ron` ships **`fill_dimming: 0.0`**, disabling the
   long-standing water-darkening look entirely (the field's own comment
   says 0.65 is the intended value, and CLAUDE.md's dark-row metric trap
   assumes it is on). Whether that zero is a deliberate live-tune or
   leftover sweep state is an owner question that re-anchors every water
   render judgment in this report.
7. **The underground is confirmed empty.** Three shafts of different
   widths (`canyon-mined.png`) find: uniform stone, strata shading, a
   sand pocket. The wiki's "a quarry rather than a destination" is exactly
   right, and the shafts themselves render as featureless black voids
   (correct under the stored-sky rule — and a preview of the cave-lighting
   question below).

Per-pass counters for three of these worlds (counter next to picture, per
method): every pass fires; canyon seed 1 generates **zero** standing water
(ponds 0, soil_moisture 0); `brows` places 34 cells and `talus` 148 in a
512×320 world — features currently too rare to register visually.

## §2 What the six lenses found (kept findings only)

Six agents reviewed independently — each grounded in the strips first,
code second — then a prosecutor read the raw Reports and attacked
everything (§3). What follows survived both. Full structured output is in
the workflow transcript; the workflow itself
(`.claude/workflows/world-review.js`) re-runs it.

**Landform & biomes** (the variability mandate):
- *The region layer works at the skyline and nowhere else.* Rolling seeds
  1/2/7/13 are four real places; aridity is the one `Character` axis that
  visibly reads. But below the skyline it is **one country everywhere** —
  one gray stone, one strata style (one tilt, one fold, no `Character`
  input to `strata_shade`), one pocket density, in every region of every
  preset. Crossing an escarpment changes height, never what the country
  is *made of*.
- *Preset identity has collapsed*: rolling and terraced differ by
  10–20% on a dozen scalars while `region_variation` draws the actual
  composition; no preset field changes what a region is made of. Remedy
  (lens 1, endorsed by prosecutor): presets become distribution-shapers
  and **regions become the biomes**, by routing `Character` into strata,
  materials, formations and vegetation placement.
- *The formation vocabulary is quantitatively absent*: `brows` wrote
  34/45/0 cells and `talus` 148/34/0 (rolling/canyon/wetland seed 1) in
  ~1.3M-cell worlds — invisible in every strip. `cliff_edges()`'s own
  comment records talus once writing zero against region-scale scarps;
  the RUN=4 fix was partial. **Rescuing the existing vocabulary comes
  before adding any new one.**
- *Terrace risers are dead-plumb one-column faces 40+ cells tall*, and the
  terrace-mask edge produces the **keyhole artifacts** (canyon-s7
  x≈600–620, rolling-s1 x≈1295, rolling-s2 x≈1465): `terraced()`'s snap is
  full-strength where the mask saturates, and `detail_amplitude` 2.5–3.0
  cannot break a 40–70-cell riser.
- *Dunes are a constant-pitch sawtooth* (`dunes()`: the linear phase term
  dominates; nothing varies per dune) and *pockets are a uniform
  polka-dot field* (per-region uniform draw, indifferent to depth,
  bedding, and `Character`).

**Graphics** (how the world is drawn):
- *No vertical light axis on solid terrain* — deep rock draws at surface
  brightness, so the lower half of every world is flat wallpaper. The
  ranked fix: a **depth-graded terrain light + one-row skyline
  highlight**, a pure function of `(x, y, horizon[x])` that keeps the
  dirty-rect skip by the same argument the existing cave fade makes,
  estimated <0.5 ms and the substrate everything else composes onto.
- *One palette family for the whole planet*; the fix is **region-keyed
  palette ramps baked at genesis via `Cell::shade`** — zero frame cost
  (shade already indexes the palette), the work is data.
- *At night the terrain draws brighter than the night sky*, inverting the
  silhouette — to be judged by the owner against the exact night strip
  the journey lens called the most beautiful frame in the set.
- *Mining leaves chunk-aligned tonal panels* — hard value steps at
  x%64==0 after digs. Needs the diagnosis counter first (per-chunk
  cracked-cell count beside a fresh render).
- *Parallax background silhouettes are affordable* — they ride the
  camera-moved full redraw that already happens, charged only on EMPTY
  pixels.
- *Waterfall spray must never be particles*: any non-empty
  `ParticleSystem` forces a ~10 ms full-screen redraw for as long as it
  lives. Foam must be drawn like rain is — a pure function of cell state
  inside already-dirty chunks.

**Hydrology** (rivers/waterfalls — position folded into §4):
- Ready-made waterfall geometry exists today: terraced-s1 has a ~55-cell
  face dropping into an existing pond; canyon-s1 a ~120-cell sheer wall;
  canyon-s7's stepped benches naturally host a pool-fall-pool staircase.
- **Evaporation cannot sink a fed pool**: humidity self-shelter
  (`HUMID_STOP`) stops evaporation dead on bodies wider than ~40–50
  cells. A prototype needs an explicit capped drain plus a
  drowned-spring throttle (spring stops emitting while standing water
  covers its outlet — which also makes *damming the spring* a legible,
  graded player interaction).
- Springs should be **worldgen-marked emitter cells** (a data bit, per
  the state-the-difference-as-data lesson) placed where the planned
  water table daylights on a face — geology, not inference.

**Cost & streaming** (the pricing lens):
- The settled-world guarantee holds on real generated worlds: all three
  counted presets arrive at 0/40 chunks awake with standing water and
  pockets in place, and the render skip measures 0.002 ms.
- **The single number that gates the rivers track**: `field::step`'s
  early-out requires *zero* active chunks; past it, the step clones every
  sleeping field tile — measured ~7 ms/frame this session for ONE held
  disturbance, roughly independent of size. Any permanent CA activity
  anywhere pays it. This dominates the river bill and must be audited
  (there is a recorded reverted attempt at partial-step optimization) —
  see §4.
- CA sweep cost for a river band is comparatively small (~0.1 ms per
  awake chunk at river-band dirty-rect size; 8 chunks ≈ 0.8 ms).
- Precipitation bypasses the dirty-rect skip frame-wide while falling —
  accepted today as an event cost, but it prices the water-cycle
  closure: rain's duty cycle, and the O(width²) levelling tail rain
  re-arms on wide ponds, need a standing-awake-tail measurement before
  closure ships.
- The render dirty region is a single union rect: two disturbances at
  opposite screen corners repaint nearly the full screen. Fine today;
  matters for any permanent disturbance.
- Worldgen build measured 542 ms this session (structural BFS is 45% of
  it); generation-time erosion must fit an F7-regen budget (~≤800 ms
  proposed, recorded with the measurement beside it).

**Underground & secret caves**:
- *The player has X-ray vision into rock*: `UNDERGROUND` darkening
  applies only to EMPTY cells below the genesis horizon, so sealed
  pockets — and any future secret chamber inside the viewport — draw
  fully lit at any depth, even at midnight. "Hidden" currently means
  "below the viewport's reach", never "concealed by rock". The graphics
  lens's depth-graded light partially dissolves this (dim-never-black
  over 40–80 rows conceals shallower than a hard depth band, while
  keeping strata legible).
- *The vault design that survived prosecution*: a **sealed vault pass**
  grown from `pockets()`'s verified collect-then-verify-seal skeleton —
  grotto shapes as unions of 2–4 ellipses with flattened floors, geode
  vugs with 1–2-cell attached **crystal linings** (new `.ron` materials,
  data-only), flat gravel/shard floors, standing water written exactly as
  ponds writes it (born level and full, at rest by construction).
  Genesis-only, zero standing cost, arrives-asleep by the same pattern
  the tests already enforce.
- *Concealment is free*: place vaults ≥ ~220 cells below the local
  surface (and ≥16 above bedrock) and the 320-tall viewport over a
  640-deep world hides them with zero renderer work — depth-band
  secrecy, one knob.
- *Finding one feels earned via veins*: thin pale mineral veins threading
  the banding, safely allowed to outcrop (solid-replaces-solid,
  attached), leading down 100+ cells to a vault. Plus the cheap
  unblocker: **gravel pockets are currently illegible** (gray-on-gray) —
  half the existing buried vocabulary is invisible and the vein/hint
  system builds on it.
- *Crystal glow, honestly scoped*: a Material flag flooring the sky-light
  factor makes a breached vault's lining shine *at night* while the
  world dims — presence without being a light source. A true local light
  is a separate, unpriced mechanism; dark-at-noon-then-reveal stays the
  behavior unless the owner wants more.
- *Ceiling spans*: bounded chamber widths (≤48 cells) sit within the
  load model's tolerance by the lens's arithmetic, but the prosecutor
  flags all breach-tolerance numbers as **contaminated by the live
  dig-over-removal bug** — vault work firewalls itself from breach
  tuning until the load session lands its fix.

**Player journey** (images only):
- Most memorable: canyon-s2's staircase-terraced butte (the one
  screenshot-on-sight landform in the set); rolling-night shot 4 (moon,
  starfield — "the single most beautiful frame in the review"); arid's
  dune-capped ridgeline as the strongest this-biome-is-not-that-biome
  signal.
- Dullest: wetland (a preset named wetland that reads as thicker
  topsoil); rolling seeds 2/7/13 put a screen-and-a-half of *featureless
  open ocean* mid-strip — seed variance on water coverage swings too
  wide with no island/shoreline vocabulary to spend it on; and every
  underground.
- *Pockets read as ore/loot* — isolated warm highlights against gray —
  but are sand. The world's most legible "point of interest" is a false
  promise; either give them identity or dim their loot-coding.
- Digs look like scars, not discoveries: uniform black slots with
  nothing revealed. (The vault pass plus breach-spill feedback is
  exactly aimed here.)

## §3 Prosecutor verdicts — what got killed or corrected

The adversarial pass read the raw Reports (the do-not-retry record) and
verified claims in code. Kept here so the discarded ideas stay discarded
(a revert keeps the knowledge):

1. **Killed: pond dashes as the D3 whisker artifact** (two lenses and my
   own first read). The whisker requires air *below*; pond surfaces have
   water below. Verified: dashes are shoreline sand, and
   `water.ron fill_dimming: 0.0` disables water darkening wholesale. The
   proposed sub-threshold-fill metric would have read the same before
   and after — a metric that measures nothing.
2. **Killed: "chambers already exist" (journey lens)** — the roofed-void
   counter it cited counts empty cells below rock *after the harness's
   own cut*; no chamber pass exists. The want is legitimate; the vault
   pass is the answer.
3. **Killed: opening the world edge for rivers** (journey lens's framing).
   The edge seals carry recorded bugs (worlds drained dry); the
   literature record and the in-plane drain design settle off-plane flux
   without touching them. **No edge seal opens in the recommended path.**
4. **Corrected: the river kill-bar collision.** Hydrology pre-registered
   2.0 ms; cost lens measured that *any* held CA activity re-arms
   `field::step` at ~7 ms regardless of river size. Resolution: the
   number is real but partly an auditable overhead, so the river track
   starts with **instrumentation** (a river-cost harness scene + the
   field-step audit), not construction.
5. **Half-dead: fake AO** — tried and cut at ~10 ms (recorded in
   `render.rs` ~1651). Re-priceable only under a genuinely new access
   pattern (scanline cache), against its recorded bar.
6. **Flagged unfalsifiable**: aspiration-shaped bars (dune pitch variance
   targets with no measured good state — the repo's answer for pure-look
   questions is a runtime selector judged by the owner), and
   counter-equals-knob checks ("vaults placed: N reads the knob") that
   measure config, not mechanism.
7. **Unresolved sightings needing one-line censuses before any build**:
   the blue sliver at canyon-s1 x≈920 (three hypotheses, one Liquid-cell
   census settles it — ponds reported *zero* cells for canyon, so a real
   Liquid there is a leak worth knowing about), and the chunk-aligned
   mining panels (per-chunk cracked-cell counter beside a fresh render).
8. **Cost red lines reaffirmed**: permanent particles ≈ permanent 10 ms;
   live-keyed render channels force full redraws forever (palettes must
   bake at genesis); neighbour-reading draw rules go stale by one pixel
   at touched-chunk borders unless rects widen by one cell; vegetation
   density changes re-run the arrives-asleep suite, which has only ever
   been verified at 10–59 seeds.

## §4 Rivers and waterfalls: yes — and the repo already did the hard thinking

**The conservation worry is already answered by shipped code.** Water in
this engine is deliberately non-conserved in both directions: rain
*creates* full water cells out of the sky (`weather.rs`,
`WATER_CELL_CHANCE = 0.06`, work-capped at 24 columns/frame regardless of
world width), snow melts into full water cells, and evaporation *deletes*
fill unbanked (`weather-handoff.md`: "Evaporated water is gone, not
banked"). Every conservation test is scene-scoped to exclude weather and
soil. An edge source is the same class of mechanism as rain — a budgeted
boundary condition — not a new architectural transgression. Dwarf Fortress
has shipped exactly this for two decades (rivers enter at one map edge and
drain off the other); Terraria's liquids are non-conserved by declared
design. The precedent is respectable.

**The design fork is already written down.** `worldgen-design.md` §0
frames the world as a 2D slice of 3D worldgen (the `ChunkCoord.slice`
field is already reserved), and §5a poses the exact question the owner
asked, verbatim: off-plane flux can be **real** — a coarse `(x,z)`
drainage map genuinely computes upstream and downstream and the slice
honours it — or **plausible** — water appears at the boundary at a
believable rate — and the report demands the choice be made explicitly.

**Position: build *plausible* flux first, in-plane, and say so in the
code.** The shape that survived the review and the prosecution:

- **Source**: a *perched spring* — a **worldgen-marked emitter cell** (a
  data bit on the cell: stored, never inferred from shape) placed by a
  small finite-margin pass where the planned water table sits above a
  neighbouring lower floor — "the aquifer daylights here", which is what
  `worldgen-design.md` §7 already calls a spring. Sim side it is an
  active site (evaporation's scheduling machinery), emitting ~1–2 water
  cells/frame — calibrated against rain's *measured* creation rate (a
  maximum storm makes ~1.4 cells/frame world-wide), hard-capped at ~2
  springs per world. Emission must clear the local evaporation floor or
  the fall flickers out mid-drop.
- **Sink: an in-plane capped drain, NOT the world edge, and NOT
  evaporation.** Evaporation cannot sink a fed pool — humidity
  self-shelter stops it dead on bodies wider than ~40–50 cells. The
  drain is the spring's exact inverse at the basin's low point, deleting
  arriving fill at ≤ emission rate ("water leaves toward the valley
  behind the plane", made literal). Plus the **drowned-spring
  throttle** — the spring stops emitting while standing water covers its
  outlet — which is both the flood guard and a *satisfying graded player
  verb*: dam the outlet and the spring visibly chokes. **No edge seal
  opens anywhere in this path** — the seals carry recorded bugs (worlds
  once drained dry) and the in-plane drain makes touching them
  unnecessary.
- **Promise falls and pools, not a horizontal river.** With no
  hydrostatic pressure, water never leaves a basin except over its rim,
  and on soil slopes infiltration eats a gliding stream. What this
  terrain *naturally* hosts — measured against the actual strips — is a
  **pool-fall-pool staircase** down stepped benches (canyon-s7's mesas
  are ready-made), and the cheapest first demo drops terraced-s1's
  ~55-cell face into its *already-existing* genesis pond. Waterfalls are
  the cheap half: free fall is whole-cell and unthrottled, in-flight
  water is exempt from evaporation scheduling, and a spring on an
  existing brow over a basin is a waterfall with zero new physics.
  Step-pool cascades are among the most beautiful water forms in nature;
  the owner signs off on that reframe before judging the prototype.

**The real cost gate is the field grid, and instrumentation comes before
construction.** The chunk-sweep cost of a modest cascade is small
(~0.8 ms for 8 awake chunks at river-band dirty-rect size). The dominant
bill is that *any* permanently-awake chunk defeats `field::step`'s
early-out, after which the step clones every sleeping field tile —
**measured ~7 ms/frame this session for one held disturbance, roughly
independent of its size**. That number is partly auditable overhead (a
naive partial-step fix was already tried and reverted; the audit must
respect why), and it gates the track. So the sequence is:
1. **River-cost harness first** (~an hour of code): an `ascii` scene
   holding a spring/fall/pool at steady state, printing cells emitted /
   drained / standing census / awake tiles / worst frame, paired against
   a spring-off run in the same session.
2. **Pre-registered kill criteria** (kill the approach, not the tuning):
   steady-state worst-frame delta > 2.0 ms *after* the field-step audit
   (headroom under the 3.55 ms wind-revert precedent, not sitting on
   it), or awake tiles failing to stabilize at a bounded set (≤ ~6 of
   40 on canyon) because continuous inflow keeps the pool levelling
   forever (the O(width²) tail). If the second fires, the fallback is a
   pulsed/intermittent spring — a different design, judged separately.
2b. **First measurement (2026-08-20, harness landed and run same
   session)**: `river_cost_scene` in `examples/ascii.rs` — canyon seed 1
   at 512×320, spring at (255, 120) over a 30-cell drop, capped drain in
   the basin, 1400-frame steady window, paired arms. **Standing bill:
   mean +1.54 ms/frame — under the 2.0 ms pre-registered bar.** Awake
   chunks stabilize at exactly 9 of 40 (bounded — the fed pool is not
   levelling forever). The water ledger closes within 6.6% (emitted
   2.0M, drained 1.42M, standing 445k; the residual is evaporation and
   soil infiltration). Two riders for the field audit: the *control*
   world already runs at 2.58 ms mean with **zero** awake chunks and 23
   unsettled field tiles — the day/night sky keeps the field stepping on
   a generated world regardless, so a river's *marginal* field cost is
   smaller than the ~7 ms fear (which was measured at larger sizes and
   still needs the audit before the 2048×640 verdict); and the spring
   arm shows a 43 ms worst-frame spike (mean is the standing bill; the
   spike wants attribution — likely the initial cascade or a structural
   check — before the prototype ships; a later same-session run measured
   the same arm at worst 7.0 ms, so the spike is not reproducible and is
   plausibly container noise).

   **Shipped-size verdict (2048×640, same session, scene re-run at both
   sizes):** standing bill mean **+0.97 ms/frame** — comfortably under
   the 2.0 ms bar, and *smaller* than at harness size, because the
   feared field-step overhead turns out to be a pre-existing background
   cost, not a river cost: the spring-off control at 2048×640 already
   runs at **10.5 ms mean with zero awake chunks and 79 unsettled field
   tiles** (the day/night amplitude ramp keeps surface tiles stepping on
   any generated world). Awake chunks stabilize at 5 of 320; the water
   ledger closes **exactly** (emitted 2.0M = drained 1.49M + standing
   0.51M, unaccounted 0). **Rivers are affordable at the shipped world
   size. The gate is open.** Two consequences: the spring/fall/pool
   prototype can proceed on the review's design without waiting on any
   field-step work, and the audit's real target shifts to the *control*
   number — the 10.5 ms background bill of an idle generated world —
   which is a pre-existing cost worth its own line in the next
   performance pass (the `apply_sky`-subset mechanism in 2c is the named
   lever).
2c. **Field-step audit (measure-first, finding not fix).** The feared
   "~7 ms for any held activity" decomposes, on numbers already on
   record: a full solve at 2048×1280 is 90.6 ms over 640 tiles
   (~0.14 ms/tile), so a held disturbance's bounded awake ring (~25
   tiles with halo) accounts for ~3.5 ms of the 7.15 ms measured — the
   other ~3.6 ms is the **per-sleeping-tile clone** (`FieldTile` owns
   five boxed slices; ~615 untouched tiles cloned every frame, ~6 µs
   each). The clone is the world-area-proportional half, which is why it,
   not the ring, decides the shipped-size verdict. The naive fix
   (merge solved tiles into the live map) was **tried and reverted** —
   recorded in `field.rs` with the mechanism: `apply_sky` must write
   before convergence is judged, or every lit tile reads "changed"
   forever (one impulse went 5.2 → 47 ms). The safe seam is also already
   named there: `apply_sky` writing into the solved subset while walking
   full columns — *a second mechanism, not a reordering*. Recommendation
   standing: implement that mechanism only if the shipped-size river
   measurement breaks the bar; otherwise bank it as the known lever. One
   more rider from the harness: on a generated world the field never
   fully settles during day/night amplitude ramps anyway (23 tiles
   unsettled, control mean 2.58 ms at 512×320), so a river's *marginal*
   field cost during ramps is small; the plateaus (flat noon/night) are
   where the river alone pays the wake bill.
3. **The priced fallback if the bill doesn't fit**: the *settled river*
   — a river bed generated with water born at rest (level per pool
   step, exactly as ponds are born), reading as moving via the
   already-prototyped animated-grain machinery on visible liquid chunks
   only, with real CA flow only on disturbance. The only river with
   ~zero standing cost; honestly framed as presentation.

**What survives the later "real flux" upgrade:** springs and drains
become *outputs of the coarse (x,z) drainage map* (their budgets computed
from upstream area instead of authored constants); the emission
machinery, throttle, harness and tests carry over unchanged. Nothing
about plausible-first is throwaway.

## §5 Roadmap

Sequenced by the prosecutor's recommended order, folded with the owner's
priorities (variability/biomes first-class; rivers if affordable; secret
caves; graphics quality; process-over-authored formations; no plant
development in this track). Each item carries its delegability for a
cheaper implementation model (landmines in §7 travel with every
delegated task): **[cheap]** = executable cold
from the spec by a cheaper model, verified by rendered images +
counters; **[frontier]** = needs the strongest model or owner judgment;
**[mixed]** = frontier designs/verifies, cheap model executes.

**0. Ambiguity-clearing measurements** (hours, before any building)
   **[cheap]**: (a) owner question: is `water.ron fill_dimming: 0.0`
   deliberate or leftover sweep state? (b) one Liquid-cell census at
   canyon-s1 x≈920 and arid-s1 x≈1215 (the blue slivers — canyon
   generated zero pond cells, so a real Liquid there is a leak);
   (c) per-chunk cracked-cell counter beside a fresh render of the
   mining panels; (d) trace the keyhole-slot artifact in
   `column.rs::terraced()`'s mask edge.

**1. Depth-graded terrain light + skyline highlight** **[mixed:
   frontier specs the ramp and the night-composition decision, cheap
   model implements]**: the vertical light axis the picture is missing —
   pure function of `(x, y, horizon[x])`, keeps the dirty-rect skip,
   <0.5 ms estimated, judged by the owner on one F7 screenshot *and*
   the night strip (the silhouette-inversion decision folds into the
   same ramp). This is the substrate palettes, concealment and cave
   reveals compose onto — it goes first among visuals.

**2. The Character-consumer workstream — regions become biomes** (owner
   priority: variability) **[mixed]**: build the 16-seed p90 census
   sweep FIRST (worldgen is procedural content; the sweep precedes the
   model change, per the repo's own rule), then in order:
   - **Brows/talus rescue at region scale** **[cheap, after frontier
     re-derives the cliff-detection window]** — the canary that the
     formation pipeline is visible at all (34–148 cells today).
   - **Region-keyed palette ramps + strata with `Character` input**
     **[cheap]** — one rock family per region character (warm sandstone
     canyon, pale bleached arid, cool gray rolling, rich dark wetland
     loam), baked at genesis via `Cell::shade`; zero frame cost, the
     work is data.
   - **Pockets follow bedding and region** **[cheap]** — lenses
     elongated along strata bands, density keyed to `sediment`, gravel
     legibility fixed; kills the polka-dot read and the false
     loot-promise together.
   - **Dune-comb and plumb-riser legibility fixes** **[cheap]** —
     per-dune amplitude/wavelength variation; riser roughening; judged
     via a runtime A/B selector, not an aspiration bar.
   - **Plan-space process erosion** (the owner's preferred
     formation *method*, scoped honestly) **[frontier]**: iterate
     thermal + hydraulic erosion/deposition over the 1D column-plan
     arrays (2048 floats — tens of ms at build time) with per-band
     hardness from strata, so hoodoos, undercut profiles, sediment
     fills and talus are *side effects of simulated history*
     parameterized by world age. Full-grid cell-level erosion is
     priced (~40–80 ms/pass) and deferred; live in-sim erosion is out.
   - **Boulders as socketed attached-stone clusters** **[frontier
     spec, cheap execution]** — freestanding rounded boulders and
     fields below cliffs, density from relief×resistance; each must
     survive the structural model at rest (blocked on the load
     session's dig-fix for breach *tuning*, not for placement).

**3. Secret caves — the sealed vault pass** (owner priority) **[mixed]**,
   parallel with 2 (different passes, genesis-only): vault shapes grown
   from `pockets()`'s collect-then-verify-seal skeleton (grottos, geode
   vugs with crystal linings, capped chimneys joining rooms); interior
   materials as data (crystal Solid + shard Powder + flat gravel floors
   + standing water born level); **depth-band secrecy** (≥ ~220 cells
   below surface — concealment for free from the viewport, zero renderer
   work); **vein-and-drape hints** threading toward vaults; gravel
   legibility unblocker; **night-glow crystal** via a Material flag
   flooring the sky-light factor (presence, honestly not a light
   source). Firewall: no breach-tolerance bars or span numbers in docs
   until the load session lands the dig fix.

**4. River instrumentation** (owner priority: rivers if affordable —
   this is how affordability gets decided) **[frontier]**: the
   river-cost harness + the field-step ~7 ms audit + same-session
   re-derivation of every anchor number on the owner's machine. §4 has
   the full spec and kill criteria.

**5. Rivers, chosen by the numbers from 4** **[frontier]**: if the bill
   fits — in-plane spring/fall/pool-chain (spring as data bit,
   drowned-spring throttle, capped drain, all edge seals kept), owner
   signing off on pools-linked-by-falls before judging. If not —
   the settled-river presentation ships instead, honestly framed.

**6. Water-cycle closure, priced by duty cycle** **[mixed]**: bank
   evaporated water into the rain budget (counters are cheap), gated on
   the wetland awake-tail measurement after rain (precipitation bypasses
   the render skip; rain re-arms the O(width²) levelling tail) and the
   weather handoff's "write the *and-then-it-stops* test first" rule.
   The repo's own sequencing: water cycle before the coarse map.

**7. Coarse (x,z) map** **[frontier]**: localizes `ponds`' global rim
   scan (the streaming debt), supplies real discharge for springs
   (upgrading rivers to real flux), and carries the settled off-plane
   answer. **Streaming (M10) last** — nothing in six lens reports gives
   a reason to move it earlier.

**8. Graphics tier 2, spending the shared walking-frame pocket only
   after a camera-pan worst-frame line exists in `ascii`** **[mixed]**:
   parallax silhouette layers, region sky tint folded into palettes, AO
   re-priced against its recorded 10 ms bar under a scanline-cache
   access pattern, canopy shadow only when generated worlds actually
   have canopies (blocked on the age parameter / vegetation decision).

**Deliberately not on the roadmap**: heightfield-body revival, any edge
seal opening, permanent particle emitters, live-keyed render channels,
full-grid runtime erosion, plant development (below), breach-tuning
before the load fix.

### §5b Input to the plant workstream

Biome identity needs vegetation this track cannot build. Written in the
plant system's own levers (from `plant-species-authoring.md` /
`plant-substrate-v2-design.md`), as recommendations to the plant agent:

1. **2–3 additional named species `.ron` files worldgen can select by
   region `Character`** — the placement lever
   (`world.plant_tree_species(x, above, "tree")`) already exists and
   takes a string.
2. **A dry-country scrub form**: 2-value ByOrder plastochron (shrub tier
   structure), higher `branch_chance`, `turgor_per_cell` as the height
   lever (r = −0.74 on height — the knob that reads as *height*, not
   size). Needs a substrate call from the plant side: sand has
   `water_capacity 0`, so desert plants need either species-level low
   water demand or a small capacity on sand.
3. **A wetland/marsh reed form**: needs the waterlogging-tolerance lever
   (necrosis above saturation, v2 §4c) exposed per species, so a form
   can stand in saturated bank soil that kills the default tree.
4. **Stand texture**: ship new species with tuned `genotype_variance`
   and per-species spacing — even spacing is the artificiality both
   teams' docs name.
5. **Beauty hook, cheapest first**: leaf/wood palette variation per
   species (materials are data) — green-gray scrub vs deep-green wetland
   reads at contact-sheet zoom where structure does not.
6. **Known blockers, acknowledged not solved**: understorey/shade
   species are blocked on the light field having no gradient above the
   canopy (their own finding); juvenile-vs-adult form needs an age axis
   that does not exist.

And one worldgen-side note the prosecutor added: raising vegetation
*density* re-runs the arrives-asleep guarantee, which has only ever been
verified at 10–59 seeds — the settle/sleep suite gates any density
change. The deeper fix for barren-at-spawn remains
`worldgen(seed, coord, age)` (roadmap 2/6 feed it), not simply more
seeds.

## §6 Open questions for the owner

1. **Is `water.ron fill_dimming: 0.0` deliberate** (a live-tune you
   chose) or leftover sweep state? One answer re-anchors every water
   render judgment above.
2. **Pools-linked-by-falls**: do you accept the reframe (springs feeding
   step-pool cascades down benches) as *the* river for this world, with
   a horizontal gliding river explicitly out until the coarse map — or
   is a through-flowing river the bar the prototype must meet?
3. **Cave lighting**: is dark-at-noon-then-reveal (plus night-glow
   crystal) the secret-cave experience you want, or do discovered
   chambers justify building the game's first true local light source?
4. **Night silhouette**: terrain currently draws brighter than the night
   sky (inverted silhouette). The depth-ramp work can fix it — but the
   night strip is also the most-praised frame in the review. Judge the
   A/B when it exists.
5. **Presets vs biomes**: the recommended direction dissolves presets
   into distribution-shapers and makes *regions* the biomes. OK to
   retire "preset = world flavor" as the long-term model (F7 keeps
   working throughout)?
6. **Grain modes**: five prototyped modes still sit behind `G` awaiting
   your eye; the settled-river fallback would build on the animated one.

## §7 Landmines appendix — for the implementation model

Every one of these has already cost this project real time. They travel
with every delegated task; the delegating spec should quote the relevant
ones verbatim.

1. **`aux == 0` means FULL on a `Liquid` and DRY on a `Powder`.** Writing
   a literal 0 fill manufactures water. A drained liquid is
   `Cell::EMPTY`, never `with_aux(0)`.
2. **Editing an asset `.ron` does nothing until rebuild** — materials and
   species are `include_str!`-compiled. Identical output across sweep
   settings means the knob was never connected. Rebuild between sweep
   points.
3. **Generated terrain must arrive at rest and sleep fast.**
   `tests/worldgen.rs` enforces zero cells moving in 120 frames and
   `active_chunk_count() == 0` within 45 frames, every preset × 5 seeds.
   Any new pass output (formation, chamber, decoration) must hold both.
4. **Only three GLOBAL worldgen passes exist, and a test pins the list.**
   New passes declare a finite column margin
   (`only_the_water_passes_read_the_whole_world`).
5. **Registries are append-only**: material ids (`EMBEDDED` order) and
   noise `Purpose` discriminants must never be renumbered.
6. **Determinism is required** (same-build). No iteration over a
   `HashMap` may influence behaviour — `BTreeMap` or a sorted `Vec`. No
   wall-clock, no non-seeded randomness.
7. **Test both drivers.** `update::step` is serial; `parallel::step` is
   what the app runs. Behaviour only the player sees is behaviour only
   the parallel driver produces.
8. **A size cap must bound work, never gate whether something happens.**
   `if too_big { return }` claims the largest cases deserve the least
   behaviour — written twice here already.
9. **Sweep before changing anything that governs procedural content**, and
   gate an order statistic (p90/max over seeds), not a single seed —
   `scripts/seedsweep.sh` exists because eight green acceptance scenes
   twice rubber-stamped changes that ate 26–50× more world.
10. **Do not add structural checks to organism paths**
    (`schedule_structural_check_around` amputates crowns — measured 26×
    outcome difference from one line).
11. **Never `git add -A`; stage explicit paths.** Contested files
    (`src/app.rs`, `README.md`, `PLAN.md`, `CLAUDE.md`) get minimal
    diffs landed fast.
12. **Don't strip load-bearing comments** — they record reverted
    approaches that must not be retried. Add to them in the same voice.
13. **`cargo fmt` is all-or-nothing** (formats 28 files); the full pass
    is deliberately deferred — do not let it ride along.
14. **When a fix changes what a number means, re-derive the constants
    that read it** — that re-derivation is part of the fix, not scope
    creep.
15. **A green suite is not evidence the screen changed.** Every visual
    change ships with a rendered artifact (`viewshot`/`filmstrip`) and,
    for "did it fire at all", a printed counter next to the image.
16. **The world edge is sealed on purpose in three places** (bedrock
    sentinel, field wall, ponds rim) — opening any of them is a
    coordinated change with test rescoping, never a local edit.
17. **Do not build on `src/sim/liquid.rs` heightfield bodies** — never
    runs in production, measured slower than the CA it was meant to
    replace, purpose explicitly unsettled (`open-bugs-handoff.md` §6).
18. **`load.rs`, `structural.rs`, `rigid.rs` and the filmstrip structural
    scenes belong to the destruction workstream** — read-only from world
    work, and the "one dig eats too much world" bug lives there
    (`next-session-handoff.md`).
19. **Any non-empty `ParticleSystem` forces a ~10 ms full-screen redraw
    every frame it lives.** Nothing permanent may be a particle:
    waterfall foam, drifting leaves, fireflies, chamber dust all draw
    like rain does — a pure function of cell state inside already-dirty
    chunks — or not at all.
20. **Any permanently-awake chunk anywhere defeats `field::step`'s
    early-out** (~7 ms/frame measured, size-independent). A feature that
    never settles pays the field bill for the whole world, not for
    itself — measure it before building on it.
21. **Live-keyed render channels force full redraws forever** (the
    organism-overlay precedent). Palettes and tints bake at genesis into
    `Cell::shade`; never key a color to a live field read.
22. **A draw rule that reads neighbour cells goes stale by one pixel at
    touched/untouched chunk borders** — widen the touched-chunk screen
    rects by one cell or ship a new stale-pixel artifact class.
