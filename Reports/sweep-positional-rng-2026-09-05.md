# A positional draw for the CA sweep — the case, and the plan

*Measured 2026-09-05 on `claude/evolution-lab-perf-analysis-5ra0jn`, cut from
`main` at `ab2f425` + the 2026-09-05 merge (`src/` unchanged by that merge, so
every timing below holds against this tree). Continues
[`evolution-lab-frame-cost-2026-09-01.md`](evolution-lab-frame-cost-2026-09-01.md)
§5, which named the unlock and did not price it.*

**The proposal:** replace the CA sweep's per-chunk stateful `Rng` with a draw
seeded from `(world seed, x, y, frame)`, so narrowing the swept region stops
changing behaviour.

> **BUILT AND MEASURED 2026-09-05 — §9 supersedes the recommendation below.**
> Steps 1 and 2 are done. **Both of step 2's stop conditions fired**: the
> positional draw costs **+0.149 ms/tick**, which is the whole of what the
> spans save, so the change end to end is worth **nothing measurable**; and
> the premise turns out to be **false** — with the RNG removed from the
> question entirely, the two sweep arms still diverge, first at **frame
> 4,330**. Step 3+4 is not started and should not be. §1–§7 are left as
> written, because the case they make is the one the measurement answered.

**The recommendation as put on 2026-09-05, before the work: green-light steps
1 and 2 only — about a day's work, nothing irreversible — and re-decide on
their numbers.** §5 priced this at
**1.19x of the whole frame**; re-measured here it is **1.05x**, because #228,
#234 and #235 shrank the frame around it. The phase win is unchanged. But the
case for going further than step 2 cannot be made from anything measurable
today, for two reasons found while writing this and confirmed under
independent review: **the ceiling is not measurable until the change itself
exists** (§2.2), and **the premise the whole line rests on has never been
tested** (§5.0). Step 2 produces both, for one run, before anything diverges.

**This document was reviewed adversarially before being put forward, and the
review changed it substantially** — §8 records what, including two claims that
were simply wrong.

---

## 1. What is actually being bought

Not 5%. **The removal of a standing veto.**

Today `update_liquid` and `update_powder` each reach a `surface.rng().flip()`
before they know whether anything will move, and that draw comes from a
per-chunk stream advanced in visit order. So any narrowing of the swept
region — however provably it only removes cells no rule could have acted on —
shifts the stream and moves every pile, front and stand downstream. Per-row
dirty spans are the first proposal to hit that veto. They will not be the
last: `labperf` already prices a second narrowing behind them, and every
future one arrives at the same wall.

**One correction owed at source, found in review.** `chunk.rs`'s
`row_spans_enabled` doc, `dead-ends.md:1339` and the predecessor report §5 all
say the two functions "open with `surface.rng().flip()`, on every visit". They
do not: `update_powder` starts at `update.rs:701` and its flip is at `:966`,
behind five `return`s, and `update_liquid`'s flip at `:1352` sits after a
straight-down `try_move` that returns at `:1349`. Neither runs at all for
`Empty`, `Solid`, `Plant` or `Creature` (`:238`). The *conclusion* is
unaffected — the draws are still consumed per visited cell in visit order, so
narrowing still shifts the stream — but the wording overstates how many cells
draw, and §3.2's "a cell that never draws never pays the hash" depends on the
corrected version. Fixing those three places is owed work, not done here
because nothing in the engine is being touched before a decision.

It is also the last shared-mutable-stream holdout in the hot path.
`plant.rs` already draws positionally through `rng::stream` — 20-odd sites,
keyed on `(world_seed, x, y, slot)`. `scheduler.rs` draws not at all. The CA
sweep is the one system left whose per-cell behaviour is a function of visit
order, and `dead-ends.md` records why: one shared `World` stream could not be
made parallel-safe, so it became one stream per chunk. A positional draw
removes the shared mutable state instead of sharding it.

---

## 2. The measured case

Bed for every number in this section: `lab_cost`, 1024x288, `soil=96`,
`founders=6`, `colonies=0`, `species=tree`, `seed=1`, 40,000 frames,
`plant_load=0`, `RAYON_NUM_THREADS=4`. The A/B is an env switch inside one
binary, which is the shape `CLAUDE.md` asks for on a box that is not quiet.

### 2.1 The prize, on today's tree

Paired, alternating, three runs a side:

**Median of three, with the full range beside it** — the three box ticks were
2.730, 2.731, 2.751:

| | box (shipped) | `PIXEL_PHYSICS_SWEEP=rows` | |
|---|---|---|---|
| `ca_sweep` ms | 1.211 `[1.203–1.220]` | 1.018 `[1.016–1.024]` | **1.19x** |
| whole tick ms | 2.731 `[2.730–2.751]` | 2.592 `[2.586–2.599]` | **1.05x** |

Ranges do not overlap on either row. Phase table on the box arm: `ca_sweep`
1.181, `field` 1.127, `active_sites` 0.370, of a 2.680 ms tick.

**Both arms are different worlds, and that is not a flaw in the A/B — it is
the thing being proposed.** It does mean the comparison is between two beds
rather than two builds of one bed, so it must be read as a bound rather than
a difference. It reads as a **lower** bound: measured on the same run,
the rows arm carries the *busier* world — 20.9 chunks awake against 18.9,
523.9 cells moving per tick against 445.0 — and is faster anyway. Whatever
the spans are worth, they are worth at least this.

**Against §5's own measurement, taken 2026-09-01: tick 6.51 -> 5.49 ms, a
saving of 1.02 ms.** Today the saving is **0.139 ms**. The ratio on the
*phase* is identical; the ratio on the *frame* fell from 1.19x to 1.05x
because the frame shrank around it. This is the handed-forward-estimate trap
from §15.3 happening to §5 — **a prize is a measurement of the build it was
taken on**, and this one outlived its build by four sections.

### 2.2 "Up to 3x of this phase" is not there — and no ceiling can be quoted today

**This section replaces an earlier one that extrapolated a ceiling of ~1.16x.
That extrapolation was withdrawn under independent review, and then a further
measurement showed the whole method was unavailable.** Both reasons are worth
recording, because each on its own is enough.

**First: the `est_` columns measure the wrong footprint.** `labperf` builds
them by diffing every `Cell` over the whole tick — `aux` included — and
expanding a neighbourhood around each changed cell. But since #212, ~90% of
what changes on this bed is soil moisture (labperf's own row: soil 402.7 of
445.0), and a moisture write goes through `World::set_soil_moisture` ->
`Chunk::set_world_quiet`, which marks *only* the moisture channel and never
wakes the CA sweep. So `est_bbox` is dominated by cells that cannot ask the
sweep for anything. The two builds show it: `est_bbox` was 28,612 against a
`swept` of 45,442 pre-#212 (0.63x) and is 24,222 against 10,326 today
(2.35x). `labperf`'s own doc states the criterion and it is failing it —
*"if it does not land near `swept`… `est_rows` means nothing."*

**Second, and this closes the door: `swept` cannot be compared across the two
arms either, because the arms are different worlds.** Measured, same bed,
same seed, 40,000 frames:

| | `swept` | `awake` | `moved` |
|---|---|---|---|
| box | 10,326 | 18.9 | 445.0 |
| rows | **12,242** | 20.9 | 523.9 |

The narrowed arm sweeps **more** cells. It cannot be doing so by narrowing —
a span is a strict subset of its chunk's bounding box, so on *one* world rows
must sweep no more than box. It sweeps more because by frame 40,000 it is a
different, busier bed. This is `CLAUDE.md`'s *two runs that diverge on one
frame are different worlds by the next* in the one place nobody looked for
it.

**So the ceiling is not measurable today, by cell count or by anything else,
and the report quotes none.** The only defensible number here is §2.1's
measured 1.05x, read as the lower bound it is. What makes a ceiling
measurable is the change itself: under positional draws, spans-off and
spans-on produce the *same* world, and then `swept` in the two arms is a
like-for-like count and the elasticity can be fitted properly. That falls out
of step 2 for free (§4).

What can be said without a model: the one time a narrowing was actually
built, it moved the phase **1.19x**, and the residue is per-chunk fixed cost
plus the cells that genuinely act — `CLAUDE.md`'s *removing work is not the
same as removing cost*. A second narrowing would be fitting the same curve
further out, and there is a floor under it that nobody has located.

### 2.3 Blast radius — measured, and the record is stale

**Every test binary, not just the lib** — `cargo test --lib` excludes
`tests/worldgen.rs` (62), `tests/determinism.rs` (3) and the bin target,
which is the surface CI actually gates. `cargo test --release --tests
--no-fail-fast`, same tree, matched control:

| binary | baseline | `PIXEL_PHYSICS_SWEEP=rows` |
|---|---|---|
| lib (1,358 + 55 ignored) | 1,358 pass, **0 fail** | 1,356 pass, **2 fail** |
| `tests/worldgen.rs` (44 + 18 ignored) | pass | **pass** |
| `tests/determinism.rs` (3) | pass | **pass** |
| `tests/*` others (9 + 0) | pass | **pass** |

**Two tests of 1,414, and both are in the lib.** Every integration binary is
green in both arms — including `tests/worldgen.rs`, whose subject is
generated water reaching rest, which is exactly the quantity a change to the
sweep's draws would be expected to move. (`.github/workflows/ci.yml` warns
that two of its tests are red on `main`; on this tree they are not
`#[ignore]`d and they pass. That comment is stale.) And they are not the two on record. §5 and
`dead-ends.md` both name `frame_step_matches_the_sequence_app_update_ran_
before_extraction` and `a_determinate_species_terminates_its_axes_in_organs_
and_an_indeterminate_one_does_not`. Today the first is still red and **the
determinacy guard passes**; the red one beside it is
`a_spread_leaf_cluster_is_longer_than_a_blob`. So the fragile-guard set
**reshuffles between builds** — which is a fact about guards of that shape,
not about this change, and means the two to fix are whichever two are red on
the day rather than the two written down.

Both red tests are shapes `CLAUDE.md` already names:

- `frame_step_...` is a deliberate cross-build fingerprint. Its own doc
  states the re-take procedure, and **the repo has already run it twice** —
  for `FIELD_SCALE` 8 -> 16 and for the evaporation vapour term — each time
  with a two-sided control: leave the change in place, neutralise it, and
  confirm the *old* hash reproduces exactly.
- `a_spread_leaf_cluster_is_longer_than_a_blob` is a paired comparison whose
  two arms are each a single seed, asserting a mean over emergent clusters.
  It needs a look, not a re-baseline (see §5.3).

**One hardcoded cross-build hash exists in the whole tree**
(`frame.rs`'s `PRE_EXTRACTION_HASH`). Every other `world_hash` assertion —
15 of them, across `worldgen/mod.rs`, `tests/worldgen.rs`,
`tests/determinism.rs` — is a *self*-comparison, two builds inside one
process, and survives untouched. `tests/determinism.rs` is a two-runs-in-one-process
self-comparison, so it survives too — but it is **not** the guard for this
change, as an earlier draft claimed: a positional draw is *more*
deterministic than a chunk stream, so it passes trivially and cannot see the
key-collision failure §5.4 names. The guard for that has to be written
(§5.4).

### 2.4 Saved boxes are recipes, not snapshots

`lab::params::save` writes the bed spec, the dials, and `.ron` asset edits.
Nothing persists cells; the `&World` it takes is read only to extract dial
values. So a saved box after this change **grows a different stand from the
same recipe, once** — not a corrupt world, and nothing to migrate.

### 2.5 Quality is settled, and the instrument was made to fail first

`rng::stream`'s splitmix64 finaliser, three inputs, `flip()` taken as the
statistic — the draw `update_powder` actually opens with.

| offset | phi | | |
|---|---|---|---|
| x+1 | −0.00040 | stream lag 1 *(the shipped bar)* | +0.00044 |
| y+1 | +0.00027 | stream lag 2 | −0.00010 |
| x+1, y+1 | −0.00011 | stream lag 1024 | −0.00001 |
| frame+1 | −0.00005 | | |
| x+64 (a chunk over) | −0.00002 | | |

11.8M draws; 1 SE ≈ 0.0003. **A positional draw is indistinguishable from
the generator that ships today, measured against that generator rather than
against zero.** `chance()` reproduces in both tails on the stride-1 frame
axis — 0.35006 and 0.00200 against 0.35 and 0.002 — which is the check
`rng.rs`'s own `a_stream_stays_uniform_along_a_fixed_tick_stride` makes at
stride 45.

**The instrument was checked against a fault before its clean result was
believed** (`CLAUDE.md`: a guard whose green you cite must be watched going
red). Feeding it a deliberately-bad key — block-nearest at /16, the
`FIELD_SCALE` gotcha put back on purpose — reads **+0.94**. It can see a bad
hash.

Two further "bad" mixers came back *clean*: the weighted sum with no
finaliser, and `jitter`'s 32-bit mixer widened. That is a finding rather than
a null. `Rng::new` + `next_u64` applies a full xorshift64\* round on top of
whatever seed it is handed, and that round launders a weak seed for the first
draw. **So the only quality failure mode that survives is an actual key
collision — two cells sharing a key.** The design rule that falls out is
stated in §3.1 and is easy to hold.

**And one risk phi cannot see, tested separately.** A positional key nails a
cell's draw to its *position*, so a mixer with any per-cell bias makes that
cell behave differently for ever — the "grain nailed to the screen" failure
`rng::jitter_u8` exists to avoid, in the movement rules instead of the
renderer. Per-cell fire rates over 4,000 frames, 16,384 cells:

| p | mean | variance / binomial | worst cell |
|---|---|---|---|
| 0.5 | 0.49998 | **1.002** | 0.5295 |
| 0.35 | 0.35005 | **0.986** | 0.3800 |
| 0.02 | 0.01998 | **1.012** | 0.0297 |

Exactly binomial. The fault control for *this* statistic — the same mixer
with the frame input dropped, so every cell is pinned for ever — reads
**3,800–4,000x**. No cell becomes sticky.

### 2.6 Per-draw cost: small, and the sign is not knowable from a microbench

Two benches, same generators, opposite signs:

| bench | delta, ms/tick at 10,326 visits |
|---|---|
| bare loop, draws only | **−0.021** (positional *faster*) |
| same, with a scattered 2.4 MB cell fetch in the loop | **+0.016** (positional *slower*) |

The bare loop favours positional for a real reason that may not survive
contact with the sweep: a per-chunk stream is a serial dependency chain —
each `next_u64` needs the last state — while positional draws are independent
and the CPU overlaps them. Put realistic memory traffic in the loop and that
headroom is already spent.

What transfers is the order of magnitude, not either percentage: **|Δ| of
order 0.02 ms/tick, under 1% of a 2.73 ms frame either way.** Worst case
measured, the draw cost eats ~11% of the 0.139 ms the spans save.

**Call that an indication rather than a bound**, for two reasons. The
per-visit figure is normalised over all 10,326 swept cells, and the change
adds a hash per *drawing* visit — a smaller set — while the draws-per-visit
multiplier is greater than one wherever a cell draws at all; neither
quantity is counted. And it is measured at box density, where after step 4
the shipped configuration is narrower. **The sign is not settled and no
microbench can settle it.** Only building it and measuring the whole tick
will — which is exactly what step 2 is for, and why nothing irreversible is
scheduled before it.

---

## 3. The design

### 3.1 The key

```rust
// in update.rs, beside the sweep
const SWEEP_SALT: u64 = /* a fresh 64-bit constant */;
rng::stream(world.seed ^ SWEEP_SALT, x as u32 as u64, y as u32 as u64, world.frame)
```

Four inputs, through the existing `rng::stream` — no new mixer, and the one
whose tail behaviour `rng.rs` already tests.

- **The salt is load-bearing, not decoration**, and the collision partner is
  nearer than an earlier draft said. `lab/mod.rs:2091` already draws
  `rng::stream(self.world.seed, sx, sy, self.world.frame)` — **the identical
  shape, unsalted**, so an unsalted sweep key would collide with it exactly
  rather than incidentally. `plant.rs:2161` is the same shape with its own
  salt; `plant.rs:1378`'s `slot` is a small constant stream id and a weaker
  partner. Existing convention: `SEED_LAUNCH_SALT` (`plant.rs:1997`),
  `APPENDED_JITTER_SALT` (`:2092`).
- **`x as u32 as u64`**, so negative coordinates wrap injectively rather than
  sign-extending into the same high bits as a large positive.
- **Injectivity over `(x, y, frame)` is the whole quality requirement**
  (§2.5). Nothing else about the mixer needs defending.

### 3.2 Where it is constructed

**One `Rng` per *cell visit*, not per draw**, lazily.

`update_cell` is the sole per-cell entry point in the engine — verified: every
other mention of it in `src/` is a comment, and `fire::update` is called from
*inside* it, so one per-visit stream covers all 20 draw sites. Within a visit
the draw order is fixed by the code path and identical regardless of which
region was swept, which is exactly the property the change exists to buy.

**The invariant a `(x, y, frame)` key needs is at most one drawing visit per
*position* per frame** — and an earlier draft got this wrong, citing
`update_cell`'s "at most one `fire::update` per cell per frame". That is the
wrong quantity: `moved` is a per-*cell* flag that travels with the material
(`CellSurface::move_cell` writes it onto the mover and the displaced), so it
says nothing about positions. Independent review checked the property and it
does hold, for three different facts — which are worth stating because they
are what a later change could break:

- `Chunk::sweep_region` intersects with `self.coord.bounds()`
  (`chunk.rs:524`) and `sweep_plan` keeps those bounds, so a chunk only ever
  visits its own cells, and chunk bounds are disjoint.
- `World::chunks_to_sweep` (`world.rs:4964`) walks `chunks.values()`, and
  `parallel::step` gives each coord exactly one `pass_key` (`parallel.rs:150`),
  so each active chunk is swept exactly once per frame.
- `run_pass`'s reinsert-then-replay loop (`parallel.rs:255`) replays
  **writes**, never re-sweeps — so no position is visited twice. A cell that
  moves cross-chunk lands at a *different* position and draws from a
  different key regardless. `MAX_REACH == CHUNK_SIZE / 2` is about write
  footprints and is untouched by any of this.

`world.frame` is incremented in `begin_step` (`world.rs:5204`) before any
pass, so the key's frame input is stable across a frame in both drivers.

- `CellSurface` gains `begin_visit(&mut self, x: i32, y: i32)`. Called once,
  at the top of `update_cell`.
- Each impl stores `pending: (i32, i32)` and `rng: Option<Rng>`.
  `begin_visit` records the position and sets `rng = None`; `rng()`
  constructs on first use and hands back `&mut` thereafter.
- **Lazy, so a cell that never draws never pays the hash.** Empty and `Solid`
  visits are a large share of the 10,326 and skip it entirely.
- A `debug_assert!` in `rng()` that a visit has begun. Two test callers reach
  `fire::update` directly (`decay.rs:1006`, `creature.rs:7833`, both inside
  `#[test]` fns) and need one line each; the assert is what finds a third.

**The 20 draw sites do not change.** Nine in `update.rs` (966, 967, 1273,
1352, 1791, 1990, 1998, 2007, 2326) and eleven in `fire.rs` (590, 595, 596,
636, 759, 808, 834, 870, 888, 1216, 1327 — `:1161` is a doc comment), all
reached through `surface.rng()`, all untouched. Two `CellSurface` impls
change — `World` (`world.rs:5836`) and `ChunkView` (`parallel.rs:715`) — and
both can already reach the key's inputs: `ChunkView` holds `world: &'w World`,
and `World` carries `pub frame` and `pub seed`.

### 3.3 What this does to `World::rng`

`World`'s `CellSurface::rng()` currently hands out `&mut self.rng`, and that
single shared world stream has **24 other draw sites across 10 files**. After
the change the sweep stops advancing it, so **every one of them draws
differently too.** An earlier draft called this "`decay.rs`, `app.rs` and a
handful of `plant.rs` shade draws", which badly understated where it reaches:

| file | sites | what draws |
|---|---|---|
| `decay.rs` | 5 | decay odds, reseeding, shade |
| `explosion.rs` | 4 | debris fraction, ignition odds, smoke fraction |
| `player.rs` | 3 | the gnome's verbs, the seed shake |
| `plant.rs` | 3 | shade |
| `rigid.rs` | 2 | **the fragment-size ladder — the rubble size distribution** |
| `world.rs`, `app.rs` | 4 | placement density, shade |
| `structural.rs`, `liquid.rs`, `update.rs` | 3 | incl. `dissipation_tick`, on the active-site schedule |

So the one-time divergence reaches the **destruction line and the player's
verbs**, not just the lab. That is why step 3 gates on `acceptance.sh` and a
`seedsweep.sh` run to rest, and it is the strongest argument for taking the
divergence deliberately and once rather than discovering it later.

It also makes both drivers key identically, which buys back a control this
repo relies on: `update::step_monolithic` is what `CLAUDE.md` names for *is
this the movement rules or the chunk decomposition?*, and today serial and
parallel differ in **visit order and RNG source at once**, so the control
cannot separate them. Positional draws leave only visit order — the question
the control exists to ask. It does not make the two drivers agree bit-for-bit;
movement is still order-dependent.

### 3.4 What this does *not* unblock

**Parallelising `active_sites`.** §15.4 names it the largest remaining item
and the RNG looks like the obstacle. It is not: the growth path already draws
positionally, and `world.rs:2624` states the actual constraint — growth reads
and writes go through ordinary `World::get`/`set`. The blocker is
unrestricted `&mut World` aliasing, untouched by anything here. Claiming this
unlock would be the fifth estimate this line has had overturned by a run.

---

## 4. The plan, in order, with the gate for each step

**There is no bit-identical intermediate.** A positional draw *is* the
divergence; the value of doing it in two steps is that the reshuffle happens
at a moment of your choosing, with nothing else moving at the same time.

**Step 1 — the positional draw, behind a switch, default OFF.**
`PIXEL_PHYSICS_RNG=positional` selects it; unset keeps the chunk stream.
Ships green, changes nothing by default. *Gate:* `cargo test --release --tests
--no-fail-fast` and `cargo test --locked` both green with the switch unset,
`frame_step_...` unchanged. That is a **one-sided** check — nothing changes by
default, so it proves the code is inert when off and nothing more. The
two-sided control belongs to step 3+4, and an earlier draft used the term
here wrongly.

**Step 2 — a 2x2, and it is the decisive step.** One binary, four arms:
`PIXEL_PHYSICS_RNG` x `PIXEL_PHYSICS_SWEEP`, paired and alternating on the §2
bed. This shape came out of review and it delivers three things one run:

1. **The end-to-end delta** — the number §2.6 could not produce.
2. **The cost at the density that will actually ship**, not at box density,
   which is the configuration a `spans-off` measurement would wrongly price.
3. **The first real test of the premise the whole report rests on.** *"Any
   narrowing removes only cells no rule could have acted on"* is asserted in
   `chunk.rs`, in `dead-ends.md:1339`, in the predecessor's §5 and in §1
   here, and **it has never been measured** — every red guard to date is
   attributed to the stream shift by assertion, not by separation. Under
   positional draws the two sweep arms must produce a **bit-identical world**.
   `lab_cost` already prints the hash. If the two hashes down the positional
   column differ, the premise is false, the spans are dropping real work, and
   this is a different piece of work. **That comparison is the gate.**

*Gate:* whole-tick deltas stated with ranges, and the positional column's two
hashes identical. If positional costs more than the spans save in the
shipping configuration — the quantity that actually decides it, rather than a
threshold picked in advance against the wrong arm — stop and re-put the
decision.

**Step 3+4 — flip the default and turn the spans on, together.** An earlier
draft split these, which would leave the tree carrying the divergence with
none of the benefit for a step, and would spend the expensive irreversible
work (the fingerprint re-take, `seedsweep` to rest, repairing emergent
guards) before the payoff was demonstrated. Re-take `PRE_EXTRACTION_HASH`
**with the two-sided control its own doc prescribes**: with `PIXEL_PHYSICS_RNG`
forced back to the chunk stream and every other edit in place, the scene must
reproduce the *old* value exactly. Note the constant's own doc records that
its second re-take already cost it its provenance — it is "a regression pin
now, not a cross-check" — so a third is not free, and §2.3 was wrong to cite
the precedent as if it were. Repair whichever emergent guards are red on the
day; repair, not re-baseline (§5.3). Record `dead-ends.md:1339`'s condition as
**met** rather than deleting the entry, and correct the "3.0x / 7.8x" ceiling
it still carries (§2.2). *Gate:* `cargo test --release --tests --no-fail-fast`
**and** `cargo test --locked` — the debug profile, because `[profile.release]`
compiles `debug_assert!` out and §5.5's guard is a `debug_assert!`, so the
release run cannot see it. Plus `scripts/acceptance.sh`,
`scripts/worldgencheck.sh`, and `scripts/seedsweep.sh` run to rest, because
this changes a model over procedural content and six seeds is not a sweep.

**Note on the seed sweep's baseline.** `Chunk::rng` is seeded from chunk
*coordinates* only (`chunk.rs:749`) — not from `world.seed` — so today the
sweep's tie-breaks are identical in every world. After the change they vary
with the seed for the first time. Arguably a gain, but it changes the
character of the distribution `seedsweep` samples, so a pre-change baseline is
not like-for-like and should be re-taken rather than compared against.

**Step 5 — a gallery, before calling it done.** Every world in the lab
diverges once. That is judge-by-eye and no test speaks to it: post a paired
`filmstrip` of the same bed and seed either side, with the stand's cell count
in `meta`, and get a verdict. Step 3+4 is reversible until this lands.

Steps 1–2 are the honest decision point: they cost little and produce the two
things nobody has — the end-to-end cost, and the first evidence that the
premise is true at all. **Nothing after step 2 should start before step 2's
hash comparison is in**, and if those hashes differ, nothing after step 2
should start at all.

---

## 5. What could go wrong, and what would catch it

### 5.0 The premise is false — the narrowing drops real work

**The largest risk, and it was invisible until review.** Everything here
rests on *"a span is a strict subset of the bounding box and removes only
cells no rule could have acted on"*. That is asserted in four places in this
repo and **measured in none of them**; every red guard has been attributed to
the stream shift rather than to a lost cell by assertion alone, and §2.3's own
evidence — the red set reshuffling between builds — is equally consistent with
the narrowing not being neutral in some configurations.

**Caught by step 2's hash comparison**, which is why that comparison is the
gate rather than a nicety. It costs one run and it comes before anything
diverges. If the two hashes differ, this is a different piece of work and the
1.05x is not available at all.

### 5.1 The draw costs more than the spans save

Bounded at ≲0.02 ms/tick by §2.6 against a 0.139 ms saving, but the sign is
unmeasured. **Caught by step 2, before anything diverges.** Mitigation if it
bites: the lazy construction in §3.2 already limits the hash to cells that
draw, and a cheaper mixer is available — §2.5 measured two weaker ones as
statistically indistinguishable, because the xorshift round on top does the
work. That trade is only worth taking with a number in hand.

### 5.2 The new stands look worse

No test speaks to it and no measurement will. **Caught only by step 5**,
which is why step 5 is a gate and not a postscript.

### 5.3 A red guard gets re-baselined instead of repaired

The live risk in step 3, and the expensive one.
`a_spread_leaf_cluster_is_longer_than_a_blob` compares two single-seed arms
on a mean over emergent clusters; identical genomes in this engine span 31 to
153 cells. Moving its numbers until it passes converts a real assertion into
a rubber stamp — `CLAUDE.md`'s *a superseded mechanism's tests keep passing
while testing nothing*. **The repair is to make the arms an order statistic
over N seeds**, which is what the test needed anyway and what would have
stopped it reshuffling between builds in the first place. Budget it as work,
not as a chore: it is the same shape as the `a_tree_eventually_stops_growing`
repair.

### 5.4 The key collides

The one quality failure §2.5 leaves live. Held by the salt and by
`x as u32 as u64` (§3.1). **A unit test asserting injectivity over a
realistic `(x, y, frame)` box belongs in the step-1 diff**, beside `rng.rs`'s
existing stream tests — and it is the only guard for this, since
`tests/determinism.rs` passes trivially (§2.3).

**Two correlation offsets are missing from §2.5 and belong in the same
diff**, both found in review. §2.5 measured a *static* cell's neighbours; the
sweep traverses two axes it did not test. A grain falling one cell per frame
samples the **diagonal `(y+1, frame+1)`**, and the sweep's `rightward`
alternation has period 2 in the frame, so a **stride-2 correlation in the
frame input** would bias left against right in exactly the place the
alternation exists to cancel. Two more rows in the same table.

### 5.5 A draw site is reached without a visit

`fire::update` has two direct callers outside the sweep, both tests.
`debug_assert!` in `rng()` (§3.2) catches those and any third added later —
**but only in a debug build**, because `[profile.release]` leaves
`debug-assertions` off and every measurement in §2 is a release run. That is
why step 3+4's gate names `cargo test --locked` as well as the release run.

**And the release behaviour has to be chosen, not left implicit.** With no
visit begun, `rng()` either panics or hands back the previous visit's stream.
The second is the silent one and is worse: it would present as a behaviour
bug far from its cause. **Construct from a sentinel key and panic in debug** —
so the release path is at least deterministic and position-free rather than
carrying a neighbour's state, and the debug run names the caller.

---

## 6. Deliberately not in scope

- **Parallelising `active_sites`** — §3.4; different blocker, and on today's
  bed it is worth ~0.28 ms against these 0.139 ms, so it is the larger item
  and should be judged on its own.
- **The second narrowing** (`labperf`'s "expansion cut to one cell"). It
  becomes *available* after step 3+4, is a separate correctness argument about
  `reach`, and has no size estimate any more — §2.2 withdrew the one it had.
- **`World::rng`'s remaining non-sweep callers.** They change behaviour as a
  consequence (§3.3); converting them is a different change with its own
  case.

---

## 7. Reproducing everything here

```
cargo build --release --examples          # --examples, or the sweep is stale

# §2.1 -- paired, alternating, three a side; PIXEL_PHYSICS_SWEEP is the only variable
RAYON_NUM_THREADS=4 ./target/release/examples/lab_cost \
  width=1024 height=288 soil=96 founders=6 colonies=0 species=tree seed=1 \
  frames=40000 every=40000 phases=1 render_every=0 plant_load=0

# §2.2 -- the cell counts and the est_ control
RAYON_NUM_THREADS=4 ./target/release/examples/labperf \
  arms=empty,plants width=1024 height=288 seed=1 settle=40000 probe=400

# §2.3 -- blast radius, with its matched control
cargo test --lib --release
PIXEL_PHYSICS_SWEEP=rows cargo test --lib --release
```

```
# 2.3 -- blast radius over the full surface, with its matched control
cargo test --release --tests --no-fail-fast
PIXEL_PHYSICS_SWEEP=rows cargo test --release --tests --no-fail-fast
```

**§2.5 and §2.6 are the two subsections a reader cannot currently check**,
and that is a real gap in a document whose whole argument is measurement.
They were taken with three standalone `rustc -O` benches over the generators
copied verbatim from `rng.rs` — a correlation bench, its fault-injection
control, and the two cost loops. **§2.5's statistics and both fault controls
belong in `rng.rs`'s own test module in the step-1 diff**, where CI runs them
and where the injectivity and stride tests of §5.4 join them; §2.6 is
superseded by step 2's 2x2 and should not be reproduced at all.

**Divergence is real and immediate**, if a one-line check is wanted:
`lab_cost ... frames=2000` prints `world hash at frame 2000:
0xc149d7087558064d` on the box arm and `0xa65364120c4997fd` on the rows arm.

---

## 8. What the independent review changed

Reviewed 2026-09-05 by an agent with no stake in the proposal, briefed to
attack it and to verify every factual claim against the tree. It reported
findings against a draft of this document; what follows is what survived and
what it cost, because a review whose findings are absorbed silently is a
review nobody can audit.

**Two claims were wrong and are gone:**

- **The invariant in §3.2.** The draft justified the per-visit key with
  `update_cell`'s "at most one `fire::update` per cell per frame". That is a
  per-*cell* property and a `(x, y, frame)` key needs a per-*position* one.
  The property does hold — the review checked the parallel driver's
  reinsert-then-replay loop specifically, looking for a position visited
  twice, and found none — but for three different reasons, now stated.
- **The draw-site count**, 19 against an actual 20; `fire.rs:1161` is a doc
  comment and there are eleven real sites in that file, not ten.

**Three claims were overstated and are corrected:** the `est_` cell counts
(§2.2), the scope of `World::rng`'s other callers (§3.3 — 24 sites across 10
files, reaching the destruction line and the gnome, not "a handful of shade
draws"), and `tests/determinism.rs` as a guard for this change (§2.3 — it
passes trivially).

**Three gaps were real and are closed by measurement rather than by
concession:**

- `cargo test --lib` excludes the integration binaries, so the blast radius
  was measured on the wrong surface. Re-run over everything: **still exactly
  2 of 1,414**, every integration binary green in both arms (§2.3). The
  review expected two `tests/worldgen.rs` tests to be red from `ci.yml`'s
  warning; on this tree they pass, and that comment is stale.
- The plan's gates named only release runs, which compile `debug_assert!`
  out — the guard in §5.5 is a `debug_assert!` (§4, §5.5).
- Step 2 measured positional's cost with the spans off, i.e. at a density
  that will never ship. It is now a 2x2 (§4).

**One finding went further than the review knew.** It argued §2.2's cell
counts measure the moisture footprint rather than the sweep's, and proposed
fixing it by comparing measured `swept` in the two arms. Run: **box 10,326,
rows 12,242** — the *narrowed* arm sweeps more, because by frame 40,000 the
arms are different worlds. So the repair is unavailable too, and §2.2 now
quotes no ceiling at all. That is `CLAUDE.md`'s *two runs that diverge on one
frame are different worlds by the next*, found in a place nobody had looked.

**And the review's own best contribution is §5.0**, which no amount of
measuring this build would have produced: the premise that a narrowing drops
only inert cells is asserted in four places in this repo and measured in
none, and there is a one-run test for it that the plan did not contain.

Two of its findings were not accepted. It read §2.1's `2.731 [2.730–2.751]`
as an impossible mean; it is the median of three, now labelled. And it could
not find `dead-ends.md`'s record of why the sweep's stream became per-chunk;
it is there, at line 1360, under the `README.md` M5 address rather than a
`chunk.rs` one.

---

## 9. Built, measured, stopped — 2026-09-05

Steps 1 and 2 as §4 specifies them. **Both of step 2's stop conditions fired,
so step 3+4 was not started.** The switch ships off by default and
bit-identical, as the instrument that measured this — the same disposition
`PIXEL_PHYSICS_SWEEP=rows` has, and for the same reason.

### 9.1 Step 1 landed green and inert

`PIXEL_PHYSICS_RNG=positional` selects `rng::sweep(seed, x, y, frame)`;
unset keeps the per-chunk stream. `CellSurface::begin_visit` opens the visit,
and `surface::VisitRng` constructs the generator lazily on the visit's first
draw, so a cell that never draws never pays the hash. Both `CellSurface`
impls changed; **the 20 draw sites are untouched, as designed.**

**§5.5's guard earned its place immediately, and changed the design.** §3.2
said `update_cell` was the only place that needed to open a visit, on a grep
for `fire::update` callers *outside* `fire.rs` — which found two, both tests.
The `debug_assert!` found **22 failing tests** on its first run: `fire.rs`'s
own test module calls `fire::update` directly **26 times**. Fixing 26 call
sites would have been the wrong repair, and left the 27th to whoever wrote
the next fire test. Instead **`fire::update` opens its own visit** — it is a
per-cell entry point in its own right, taking `(x, y)` — and `VisitRng::begin`
became **idempotent for a position already open**, so the call `update_cell`
already made is not disturbed and the order of the two stops mattering.

That also removes a fragility §3.2 had documented without eliminating: a
second `begin` mid-visit would have reset the generator and handed the
movement rules a sequence fire had already drawn from, which is precisely the
key collision §2.5 identifies as the only quality failure that survives. The
idempotence makes it unreachable rather than merely unlikely.

After the redesign the debug run is **7 failures, down from 22** — the other
15 were the guard. All seven panic in their own assertions rather than in
`surface.rs`, checked, so the guard is satisfied and what is left is ordinary
divergence: the positional arm is a different world, so emergent tests move,
exactly as the two red under `PIXEL_PHYSICS_SWEEP=rows` do. **That is the
arm's expected state, not a defect** — the shipping configuration is the
default, which is green on all 1,414 tests and bit-identical.

**The redesign did not move the arm**, checked rather than assumed: the
positional bed still hashes `0xddb39ab874e7afb4` at frame 2,000 with box and
rows identical, and the divergence boundary is still exactly 4,329 identical
/ 4,330 diverged. Every number in §9.2 and §9.3 stands against the code as
landed.

**The default is bit-identical, checked rather than assumed**: the bed
reproduces `0xc149d7087558064d` at frame 2,000, exactly the pre-change value.

§2.5's statistics and both their fault controls are now `rng.rs` unit tests
(`a_sweep_key_is_injective_over_a_realistic_box`,
`a_sweep_draw_is_uncorrelated_with_its_neighbours`,
`a_sweep_draw_has_no_per_cell_bias`), with §5.4's two missing offsets added —
the falling grain's diagonal and the stride-2 the `rightward` alternation
runs on. **The fault arms are assertions inside the tests, not prose**, so CI
proves the statistics can fail every run. 0.14 s for all three.

### 9.2 Gate A: the draw costs the whole of what the spans save

The 2x2, three reps a side, alternating. Medians, with ranges:

| whole tick, ms | `sweep=box` | `sweep=rows` |
|---|---|---|
| **chunk stream** (shipped) | **2.631** `[2.621–2.649]` | 2.542 `[2.538–2.555]` |
| **positional** | 2.780 `[2.770–2.808]` | **2.636** `[2.592–2.639]` |

Read down a column for the draw's cost, across a row for the spans' saving:

- **The positional draw costs +0.149 ms/tick at box density** (+5.7%), +0.094
  at rows density. `ca_sweep` alone goes 1.154 -> 1.283.
- The spans save 0.089 ms under the chunk stream and 0.144 under positional.
- **End to end — what shipping the whole proposal would buy — 2.631 -> 2.636.
  The ranges overlap. There is no measurable gain at all.**

**§2.6's bound was wrong by about 7x, and in the direction that matters.** It
put |Δ| at ≲0.02 ms/tick; the truth is +0.149. The report flagged the *sign*
as unknowable from a microbench and treated the *magnitude* as settled, which
was the error — a microbench that cannot tell you the sign of a difference
has no claim on its size either. The cost is not only the hash: it is
`begin_visit` on every one of ~10,326 visited cells, an `Option` test on every
draw, and a generator that now lives behind a field instead of in the tight
per-chunk path.

This is exactly the stop condition §4 named — *"if positional costs more than
the spans save in the shipping configuration"* — and it is worth noting that
the threshold as originally drafted (+0.05 ms measured with spans off) would
have **passed** this change at rows density while the end-to-end result is
nil. The review's fix, measuring in the configuration that would actually
ship, is what caught it.

### 9.3 Gate B: the premise is false

§5.0's test, and it is the more important result because it outlives this
proposal. Under positional draws the two sweep arms **must** produce a
bit-identical world if narrowing only ever drops cells no rule could act on:

| frames | `positional` box vs rows |
|---|---|
| 2,000 | identical — `0xddb39ab874e7afb4` |
| 4,000 | identical |
| **4,330** | **first diverging frame** |
| 40,000 | `0xc686c0eaaeb4fc96` vs `0x34fef6b42b4cb6fe` |

Bisected to the frame; every arm's hash reproduces exactly across all three
reps, so the engine's determinism is intact and this is a real behaviour
difference rather than noise. The control sits in the same table: under the
chunk stream the same two arms differ from the first frames, so the test can
report a difference and does.

**So per-row dirty spans are not behaviour-neutral, and the RNG was never the
whole reason.** Removing the stream shift moved the divergence from frame ~1
to frame 4,330; it did not remove it. Four documents in this repo assert the
premise — `chunk.rs`'s `row_spans_enabled`, `dead-ends.md:1339`, the
predecessor report's §5, and §1 here — and it is false.

**A single event, not a drift.** 4,329 frames identical then a split is one
cell behaving differently once, not an accumulating error. The leading
hypothesis is chunk wakefulness: `field::step` is gated on
`active_chunk_count()`, so a narrower sweep letting one chunk settle a frame
earlier changes the field's solve set, and the field feeds light, heat and
moisture back into everything. **That is a hypothesis and nothing here
measures it** — it is written down so the next session starts from a
candidate rather than from the beginning. Frame 4,330 on `seed=1` is the
handle.

### 9.4 What this changes, and what is worth keeping

- **The proposal is dead as a performance change**, on this bed. Not "small" —
  nil, with overlapping ranges.
- **Per-row dirty spans are dead as a *free* change**, which is a stronger
  statement than the one `dead-ends.md` currently carries. Today's entry says
  the spans cost an RNG stream shift and names the positional draw as the
  unlock. The unlock exists now, and the spans still change the world.
- **The three `rng.rs` tests are worth keeping regardless.** They are the
  first fault-controlled quality guards over a positional key in this repo,
  and any future positional draw inherits them.
- **`rng::sweep` and `VisitRng` are worth keeping as the instrument**, off by
  default, exactly as `PIXEL_PHYSICS_SWEEP` is. Without them nobody can re-run
  §9.3, and §9.3 is the finding.
- **What the frame budget still wants** was handed forward here as §15.4's
  serial `active_sites`. **§10 measures that and it is wrong** — including as
  I first wrote it in this section, from arithmetic rather than a run, one
  section after the same mistake was the subject of §9.2.

---

## 10. The next target was named from arithmetic, and it does not survive a run

*Added the same session, after §9.4 handed `active_sites` forward on §15.4's
Amdahl estimate. That estimate has a numerator nobody measured.*

### 10.1 On the owner's own bed, four cores buy 1.02x

**Every number above is the 1024x288 `founders=6` `species=tree`
`colonies=0` bed. The owner's default box is 512x320, `founders=8`, `herb`,
with a colony of ants** (`LabBox::default`) — a different bed, and this report
has already been caught once by a ratio that did not transfer. Measured there,
40,000 frames, three alternating reps a side, box quiet, medians with ranges:

| ms | `RAYON_NUM_THREADS=1` | `=4` | delta |
|---|---|---|---|
| **whole tick** | **2.088** `[2.063–2.123]` | **2.040** `[2.023–2.057]` | **−0.048, i.e. 1.02x** |
| `field` | 0.860 `[0.850–0.868]` | 0.735 `[0.733–0.742]` | **−0.125** |
| `ca_sweep` | 0.697 `[0.690–0.709]` | 0.755 `[0.743–0.761]` | **+0.058** |
| `active_sites` | 0.393 `[0.388–0.406]` | 0.408 `[0.406–0.411]` | +0.015 |
| `pheromones` | 0.137 | 0.141 | +0.004 |

Non-overlapping on the tick and on both phases that move. **Going from one
core to four is worth 2%.**

The field parallelises and earns its rayon: −0.125 ms, −15%. **The CA sweep is
*slower* on four threads than on one** — +0.058 ms, +8% — because at 7.7 awake
chunks the dispatch costs more than the work it splits. The two nearly cancel,
and that is the whole story of the frame's scaling.

### 10.2 So the Amdahl ceiling on `active_sites` is not there

§15.4 reasoned: `scheduler.rs`, `plant.rs`, `creature.rs` and `structural.rs`
contain no rayon, so `active_sites` is serial; Amdahl on 4 cores therefore
predicts ~34% utilisation and parallelising it is worth up to 0.54 ms. **The
serial half of that is true and the conclusion does not follow**, because
Amdahl also assumes the *parallel* half scales. Measured, the parallel half
returns 0.048 ms for a 4x increase in cores. There is no speedup being
diluted by a serial remainder; there is barely a speedup.

`active_sites` is 0.408 ms of a 2.040 ms tick here — 20%, not the 65% §15.4
was working from. Parallelising all of it perfectly would be worth 0.31 ms in
theory, and this bed's own evidence is that a newly-parallelised phase of that
size on this box will return a fraction of it and may return less than
nothing, exactly as the sweep does. **It is the riskiest change on the board
— shared-world writes under a determinism requirement — and its prize is
unmeasured and probably small. It should not be started on this evidence.**

### 10.3 What this does surface, and it is nearly free

**Thread count is behaviour-neutral, verified rather than assumed**: the bed
hashes `0xaf47c0c463f9845d` at 1, 2 and 4 threads. So the sweep's dispatch
width is a pure performance dial with no divergence attached — unlike
everything else this report has looked at.

That makes "**do not fan the sweep out when there is nothing to fan**" a
small, safe, measurable change worth **~0.058 ms (2.8%)** on this bed, gated
on the awake-chunk count rather than applied flatly, because a big outdoor
world will scale where a 512x320 box with 7.7 awake chunks does not. It is
not large. It is the only item this session found that is both free of
behaviour change and positive.

**Not built, and not proposed as more than it is** — the bar this report set
in §9.2 applies to its own recommendations: 0.058 ms is one rep's worth of
noise on many beds, and the *right* next step is to measure the dispatch
threshold across bed sizes before writing any code, not to assume this bed
generalises. That is the mistake §10.1 exists to correct.
