# Worldgen data track, round 2 — for the implementation session

Round 1 landed and was merged (`d966277`; review verdict in that merge
message). Same contract as round 1, inherited whole from
`Reports/worldgen-implementation-tasks-2026-08.md`: **read that file's
Ground rules section first and follow it verbatim** — same owned files,
same forbidden files, same per-commit gates (`cargo test`, `clippy -D
warnings`, at-rest suite, sweep compare, strips with every visual
change), same stop-and-write-a-finding rule. Branch:
`claude/worldgen-data-track-r2`, from
`claude/game-world-gen-planning-h12713`. Your round-1 Findings section
carries forward; append round-2 findings there in the same file.

One correction from your own round-1 findings, now canon:
`assets/worldgen.ron` is runtime-loaded, not `include_str!` — preset
edits take effect on the next run. Material `.ron`s still need the
rebuild.

**Explicitly NOT in this round**: the erosion core
(`Reports/worldgen-erosion-design.md` — its Delegation section reserves
the two erosion processes, the hardness field and first tuning for the
frontier session; you will get the follow-on work once it lands), any
render/sky work (crystal *glow* is the reviewing session's, your vaults
ship with a bright palette only), and anything in the forbidden-files
list.

## Task 1 — Tame the keyhole risers (your 1b finding, the design handed back)

Your 1b finding proved the keyhole slots are `round()` in the terrace
snap: the surface jumps `terrace_step × m` rows in one column, and your
census (five presets × four seeds) counted 0–16 single-column steps ≥ 6
rows per world, worst 34. Your note said the lever is `terrace_step`
against the local escarpment slope, not roughening. That is the design:

**Where the ground is steep, terracing must yield.** In
`column.rs::terraced()`, attenuate the snap by local slope: compute the
regional-scale slope at `x` from the *pre-terrace* elevation (the base +
hill sum, over a ±8-column central difference — pure function of x,
same purity contract as everything else in the file), and scale the
mask `m` down by `noise::smoothstep(s_lo, s_hi, slope)` so benches keep
their full snap on gentle ground and a snap riser can never stack on top
of an already-steep face. Derive `s_lo/s_hi` from the census: the goal
bar, pre-registered, is **no single-column surface step > 18 rows on
your existing census probe** (which is committed and `#[ignore]`d — it
is the acceptance instrument), with canyon seed 2's genuine escarpment
steps exempted the way your finding already separates them.

Constraints: pure function of x (no lookahead into plan arrays);
`terrace_strength: 0.0` worlds byte-identical (your sweep proves it);
mesas must survive — canyon s7's three big buttes are the review's
best landform, so render canyon s7/s13 before/after and put them in the
commit. Sweep compare after (terrace-driven counters may move; say by
how much).

## Task 2 — Soften the columnar family transitions

Merge-review finding: on jagged canyon terrain the palette families read
as full-height vertical columns of gray inside warm country (canyon s7,
piers at x≈290–320, 620–660, 880–990). Two causes to address together in
`passes.rs::palette_family()`:

- The family draw is per-cell on `Purpose::Palette` noise, but its
  *probability* comes from `character(x)` alone — so the transition
  dithers per cell yet its density is constant down a whole column. Add
  a slow 2-D modulation: blend the character-derived probability with a
  low-frequency field over `(x, y)` (wavelength ~40–60 columns/rows, a
  new appended `Purpose`) so a transition wanders with depth instead of
  being a vertical band.
- Consider widening the aridity smoothstep ranges so the dither band is
  broader; sweep both knobs behind preset params if a single setting
  doesn't hold across presets.

Acceptance: canyon s7 and s13 strips before/after — the gray zones
should read as ragged intergrown country, not piers; `flat` and any
`region_variation: 0.0` preset byte-identical (existing guarantee);
rolling/wetland/arid strips checked for collateral. Shade-only change:
sweep counters must not move at all (compare proves it).

## Task 3 — The sealed vault pass, step 1 (secret caves)

The reviewed design (world-review §2, underground lens; prosecutor-
approved) — grown from `pockets()`'s collect-then-verify-seal skeleton,
which round 1 already generalized to rotated shapes. Genesis-only, zero
standing cost, and everything here must arrive at rest by construction.

- **Shapes**: a `vaults` pass (finite margin — declare it) placing 0–2
  chambers per world (rarity knob `vault_density`, default such that
  roughly half of worlds have one): a *grotto* (union of 2–4 overlapping
  ellipses, floor filled flat) and a *geode vug* (ellipse with a 1–2
  cell attached crystal lining). Interior: flat gravel floor
  (`BURIED_FAMILY` shades), optionally standing water written exactly as
  `ponds` writes it (level, full, at its own surface) when the chamber
  floor sits below the local water table.
- **Materials**: new `assets/materials/crystal.ron` (Solid, attached
  span numbers near stone's, its own bright palette — pale luminous
  tones; NO glow flag, that is render-side and comes later) and
  `shard.ron` (Powder, gravel-like, crystal-toned) — appended ids,
  rebuild after (§7.2/7.5).
- **Placement**: depth band ≥ 200 rows below the local genesis surface
  and ≥ 16 above bedrock (concealment comes free from the viewport; no
  renderer work), horizontal position from a `Purpose` draw, rejected
  rather than forced when the massif is too thin (a cap bounds work,
  never gates whether the pass runs — but a world too shallow for the
  band simply has no vault, and the counter says so).
- **Seal**: the whole envelope (chamber + lining + a 2-cell stone rind)
  verified stone before any cell is written, exactly the pockets
  contract. Water inside a sealed chamber is moisture-inert (round 1
  verified the soil_moisture transform only seeds off liquid into
  capacity-bearing cells; state this in a test).
- **Counters**: `vaults` appears in the per-pass table (chambers placed,
  cells written); the sweep gains its row (re-baseline, and say so —
  §7.14).
- **Tests**: at-rest suite must hold with a forced-vault world (a test
  preset with `vault_density` high enough to guarantee placement);
  sealed-ness asserted (no vault cell adjacent to pre-existing air);
  determinism hash unchanged for `vault_density: 0.0` worlds.
- **Images**: a `filmstrip` `dump=`/crop of one vault cross-section per
  shape, plus a mined strip showing a shaft breaching one — the
  found-a-secret moment is the acceptance artifact, judged by the
  reviewing session.

## Task 4 — Water palette aliasing (your 1c incidental)

One-line data fix you flagged: `ponds` writes shades 0..3 against
water's three-entry palette, so entry 0 draws twice as often. Add a
fourth water colour (interpolate between the existing entries — no new
hue) so the tones weight evenly. Rebuild, one wetland strip before/
after, and note that `fill_dimming` stays untouched (open owner
question, not yours).

## After task 4

Stop, push, summarize in the final commit as before. Round 3 (erosion
follow-on: `world_age` plumbing, boulder realise pass from plan markers,
rate tuning) unlocks when the frontier erosion core lands.
