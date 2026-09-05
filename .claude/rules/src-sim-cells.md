---
paths:
  - "src/sim/**"
---

# Gotchas for the sweep and the cell

Each of these has caused a real bug in `src/sim/`. They are here rather than
in `CLAUDE.md` because every one of them only matters when this code is
*changed*, and an edit is always preceded by a read -- so the rule arrives in
time. Measured, not assumed: `bash scripts/contextprobe.sh src/sim/plant.rs`.

- **Two conventions for `Cell::aux` point opposite ways.** On a `Liquid`,
  `aux == 0` means **full**. On a `Powder`, `aux == 0` means **dry**
  (`material::SOIL_SATURATED`). Both defaults are deliberate — liquids are
  created full, soil is created dry — and getting either backwards
  manufactures water out of nothing. A partly-drained liquid must be written
  as `with_aux(remaining)`, and a fully-drained one as `Cell::EMPTY`, never
  `with_aux(0)`.

- **A traversal must use the same neighbourhood the writer used.** `Grow`
  places organism cells at 8 neighbours; anything reading a grown organism
  back has to traverse 8 or it sees disconnected fragments. Transport
  (`diffuse_resource`) is the deliberate exception and stays at 4: an
  exchange crosses a shared face, and diagonal cells share only a corner.

- `Cell::is_empty()` is **managed-aware** — a promoted liquid body's container
  cells are materially empty but read as not-empty. Use the raw
  `cell.material == material::EMPTY` when the question is "is there material
  here", not "is this position available".

- `MAX_REACH == CHUNK_SIZE / 2` exactly, and that equality is load-bearing for
  `parallel.rs`'s cross-chunk write-safety proof *and* for its
  reinsert-then-replay loop. Changing it needs both re-derived.

- **An unstable sort's tie order is not a function of the comparator alone —
  it depends on the element type.** `sort_unstable_by` (ipnsort) specialises
  its small-sort strategy on the type's size and properties, so two sorts
  that ask the comparator identical questions in identical order can still
  order **equal** elements differently. Measured 2026-08-24 in
  `plant.rs`'s `allocate_to_frontier`: caching the sort key to stop the
  comparator calling `world.carbon_at` twice per comparison changed the
  element from `(i32, i32)` to `(f32, (i32, i32))`, and the stand diverged —
  tree heights 101 → 103, stem thickness 9 → 6, root depth histogram
  [49, 43, 7] → [47, 38, 13]. Donor carbon is equal constantly (mature cells
  sit pinned at `RESOURCE_SCALE`), so the tie order decides which donor is
  drained. So: **any "cache the sort key" or "change the element type"
  optimisation over an unstable sort is a behaviour change until the
  comparator breaks ties explicitly**, and the free-looking half of that
  trade does not exist. The standing risk this leaves, recorded in
  `Reports/dead-ends.md`: a Rust upgrade that retunes the sort can silently
  change how every plant in the world grows, and nothing in the suite would
  catch it.

- **Do not add `schedule_structural_check_around` to an organism growth
  path.** Growth only ever *adds* material, so it is not a disturbance, and
  a `GrowingTip` is expected to be transiently unsupported until it
  reconnects — checking it there prunes ordinary growth as if it were
  damage (`plant.rs`'s `Grow` and germination both say so at the call
  site). The historical reason was different and is now **stale**: the
  hop-bounded `organism_is_supported` that amputated crowns no longer
  exists, replaced by `plant::anchor_support`, a Dijkstra from the anchors
  outward with no span budget. `open-bugs-handoff.md` §0d has that story
  and the 26x measurement; read it before trusting any Phase 3 damage
  result written while the old search was live.

- The liquid heightfield bodies in `liquid.rs` are **test-only today** —
  nothing in production promotes a body, so bugs there are latent, not
  live, and go live the moment promotion lands. Why promotion was
  implemented and reverted is in `liquid.rs`'s own module doc and
  `Reports/liquid-heightfield-design.md`.

- **A coarse-field read is block-nearest, so neighbouring cells sample the
  same value — never build a per-cell decision on the difference between
  two of them.** At `FIELD_SCALE`, four sensors one cell apart land in the
  same field block roughly seven times in eight, so their differences are
  zero and whatever tie-break follows becomes a constant direction. **Hit
  four times, on three different lines, and never once caught by a test:**
  worm thermotaxis resolved to "always flee west"; tree phototropism
  reproduced the identical degeneracy; a third proposal for per-candidate
  self-avoidance was stopped only by a reviewer noticing the pattern; and
  it stands recorded as a live trap for the first liquid code to read
  pressure per cell. If a rule needs a *gradient*, interpolate or sample
  far enough apart to cross a block boundary — and prove the two reads can
  actually differ before trusting the sign.

- **A channel needs a writer and a reader, and the compiler checks neither.**
  A field that is written and never read is dead weight; one that is read
  and never written is worse, because **every consumer of it is dead code
  that looks alive** — the reads compile, the values are plausible, and the
  behaviour they drive silently does not exist. `Reports/dead-ends.md` calls
  this "the failure mode this project has hit three times": light with no
  writer, canopy density with an always-zero reader, pressure with no liquid
  consumer. It is a standing check, not three individual fixes — when you
  add or inherit a per-cell or per-tile channel, name its writer and its
  reader out loud before building on it, and if either is missing say so
  rather than assuming the other end is somewhere you have not looked.
