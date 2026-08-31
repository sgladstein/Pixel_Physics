# `cell_scale` reaches the living

**2026-08-30. Lane: creature scale and shape. Branch `claude/creature-scale-and-shape`.**

## The one-line finding

**Nothing alive read `World::cell_scale`.** A world generated at twice the
cell density scaled the gnome (`player::Player::at_scaled`) and left every
animal at its authored cell count — that is, at **half its physical size**.
It is the owner's own *"our gnome shouldn't have shrunk"* defect
([`resolution-step-2026-08-29.md`](resolution-step-2026-08-29.md)) arriving
for everything that is not the gnome, and it had been sitting there since the
resolution step landed. This closes it for creatures.

Measured, `ant_block` on `rolling`, seed 7:

| | grid | plan | on screen | bounding box | **physical** |
|---|---|---|---|---|---|
| authored | 1x | 9 cells | 9 cells | 3x3 | **3.0 x 3.0** |
| **the defect** | 2x | 9 cells | 9 cells | 3x3 | **1.5 x 1.5** |
| fixed | 2x | 36 cells | 36 cells | 6x6 | **3.0 x 3.0** |

Review card `20260830T203214131Z-d133c8` (board `creatures`) is those three
as one sheet, each panel the same number of *physical* units of ground.
Panels two and three share a world, so they differ by the scaling pass and by
nothing else — the control is real rather than a 1x world stood next to a 2x
one, which would have differed in the grid **and** in the terrain that grid
generated.

## 1. What was scaled

`CreatureDef::scaled(k)` (`src/sim/organism.rs`), the sibling of
`player::Tuning::scaled` and deliberately the same four classes:

| class | factor | fields |
|---|---|---|
| a length in cells | `k` | `body`, `sensor_offset`, `sight_range` |
| ticks per decision | `1/k` | `tick_interval` |
| a rate per body cell per decision | `1/(cells x k)` | `idle_cost_per_cell`, `move_cost_per_cell` |
| dimensionless, or joules | `1` | everything else |

and, at the call sites that hold a `&World` rather than a species file
(`src/sim/creature.rs`, through one `scaled_cells` helper):
`CROWDING_RADIUS`, `COLONY_ANT_SPACING`, `COLONY_HALF_WIDTH`,
`SIGHT_EYE_LIFT`, `FORAGE_REACH_BUCKETS` and `FORAGE_TRIP_MIN`.

**`tick_interval` is the row that is easy to miss.** A creature steps one
cell per decision, so at `k=2` a decision carries it half as far physically.
Leaving it alone gives an animal the right size moving at half speed — the
same class of error as the gnome's, right shape and wrong character.

**The per-cell energy row had to be derived, and the divisor contains the
cell count.** Idle burn per frame is `idle_cost_per_cell * cells /
tick_interval`. At `k` the cell count goes up by `R` (2x for a chain, 4x for
a rigid plan) and the interval goes down by `k`, so burn per frame would go
up by `R*k`: a supersampled ant at `k=2` would starve **eight times as fast
as itself**. Dividing both rates by the same `R*k` holds physical metabolism
invariant, which is the only reading under which this is the same animal.

**This is not the Kleiber question and must not be filed as one.**
`dead-ends.md` records that scaling `start_energy` with *body size* was
identified and deliberately not built, because an n-cell animal burning per
cell out of a flat tank has `1/n` of a two-cell animal's horizon. That entry
is about an animal that is genuinely **bigger**. This is the same animal on a
finer grid, so its tank must stay flat and its per-cell rate must fall, and
its horizon comes out unchanged. Scaling the tank here would be the
double-count.

**`CROWDING_SCALE` moved with `CROWDING_RADIUS`, and that pairing is not
optional.** At `k=2` the same physical neighbourhood is 9x9 rather than 5x5
and the *fraction* of it that is flesh is unchanged, so a fixed divisor would
have read 3.3x the crowding for the same physical crush and pinned
`(Crowding, Move, -0.3)` at its floor. `CLAUDE.md`'s "fixing a bug exposes a
constant that was compensating for it", in its second shape: changing what a
term can express reallocates the whole weighted sum.

## 2. What was deliberately not scaled, and why

- **`body_energy`.** It is per-cell flesh pricing and wants the same `1/R`.
  It is pinned to the `ant`/`chitin_*`/`corpse` materials' `food_energy` by
  the equality `EnergyLedger` documents, and those live in
  `assets/materials/**`, which this lane does not own. Halving it here
  without them would let a predator eat a cell of flesh for more than the
  flesh cost to build — energy creation in a ledger that is asserted closed.
  **The consequence is real and is §4.**
- **`MAX_REACH` and `CHUNK_SIZE`.** `MAX_REACH == CHUNK_SIZE / 2` is a proof
  obligation for the threaded sweep's write-disjointness. Untouched.
- **`start_energy`, `reproduce_threshold`, `hunger_fraction`,
  `synapse_fraction`.** Joules, or fractions of joules. Invariant by
  construction, and asserted so.
- **`nest_memory`, `dig_force`.** A decay time in ticks, and a force compared
  against a material's `penetration_resistance`. Neither moves with the grid.
- **The flight constants** — `GRAVITY`, `MAX_FLIGHT_SPEED`, `LAUNCH_LIFT`,
  `AIR_DENSITY`. `player::Tuning::scaled` scales its own gravity by `k` and
  the same argument applies, so this is a **known gap rather than a
  decision**. It was left because the CA's own fall speed is one cell per
  frame and does not scale, so a flier whose gravity scaled alone would be
  inconsistent with everything else falling in the same world — and because
  the flight model prices drag off body mass, which the supersample has just
  quadrupled. Nothing in the scenes measured here flies. Whoever takes the
  beetle to 2x owns this.

## 3. Two findings that were not the deliverable

**A chain scales in length and cannot scale in width.** `BodyPlan`'s own doc
says why: a chain is a path and *"a path has no width"*. So `Chain(2)` scales
to `Chain(4)` — the right physical length, still one cell wide, i.e. **half
its physical width at `k=2`**. Measured: the shipped `ant` at 2x is 4x1
cells, `2.0 x 0.5` physical. There is no version of this pass that fixes it,
and that is the point: *"creatures should be more than chains of pixels"* and
the resolution step are **one problem**, not two. Pinned as
`a_chain_scales_in_length_only`.

**The supersample's anchor is where the work actually went, and it hid behind
correct geometry.** Written the obvious way — authored cell `d` growing into
the block `[d*k ..= d*k + k-1]` — the body is *physically identical* and
`plant_creature_seed` refused **every site in the world**. `ant_block` is
authored with its head at `(0,0)` and everything else at negative offsets, so
growing the head block downward put two rows of flesh *below* the cell the
placer had chosen to stand on. The size guard passed throughout: the box was
the right size, in the wrong place. The fix is `[d*k - (k-1) ..= d*k]`, which
keeps the anchor meaning what it meant. `dead-ends.md` carries it.

## 4. Mobility, against `Chain(2)`'s 5%

`examples/creature_scale mode=walk`, `rolling` seed 7, 24 founders, 3,000
frames, `RAYON_NUM_THREADS=4`. Blocked is
`moves_blocked / (moves + moves_blocked)`.

| species | plan | k=1 | k=2 |
|---|---|---|---|
| `ant` | `Chain(2)` | **5.2%** | 4.4% |
| `ant_long` | `Chain(6)` | 4.6% | 3.5% |
| `ant_wide` | `Rigid` 5x2 | 47.7% | 50.5% |
| `ant_block` | `Rigid` 3x3 | 62.1% | **72.8%** |

The `Chain(2)` row is the **positive control**: 5.2% against the 5% that
[`creature-appearance-design.md`](creature-appearance-design.md) §5 reports,
which says this harness is measuring the quantity that report measured. The
rigid rows come out higher here than §5's 43% because the scene is generated
`rolling` terrain rather than §5's bed; the ordering and the magnitude are
the finding, not the third digit.

**So: a chain walks at any resolution and a rigid body does not, and
supersampling makes a rigid body worse.** 62.1% -> 72.8% is what "the same
physical shape on a finer grid" costs, and it is not noise — a 6x6 block has
to find six clear rows where a 3x3 needed three.

**This is the blocker between "more cells" and "looks like an animal", and it
is not solved here.** §6 is the design.

## 5. The birth economy, measured on `main` rather than inherited

The lane brief said *"PR #174 has since landed — ants now reach generation
13"*. **It had not**, when this branch was cut: #174 was open, `main` was at
#173, and the economy below is what was actually there. **It merged while
this lane was running** (`0eaa125f`), and §5b re-measures against it rather
than leaving the inherited claim standing either way — which is the point of
measuring instead of quoting.

`creature_probe frames=24000 ants=24 terrain=world`, on the `main` this
branch was cut from:

```text
economy: start_energy 200 body_energy 480 hunger_fraction 0.50
         reproduce_threshold 1100 birth_grant 0.40 (= 80 energy)
reproduction: births 0 ... deepest generation 0 ...
              richest bank 203 against a birth cost of 1040
```

**Zero births**, exactly as `creature-birth-grant-2026-08-30.md` and
`creature-stamp-routes-2026-08-30.md` describe.

## 5b. Re-measured after #174 landed, and the point survives

`main` merged #174 at `0eaa125f` mid-lane, so this branch now carries it.
Same command, same seed, on the merged tree:

| | `main` at #173 | `main` with #174 |
|---|---|---|
| births | 0 | **1** |
| richest bank | 203 | **500** |
| birth cost | 1,040 | **1,040** |

Which is exactly what #174's own account predicts for the outdoor world — *"0
births to 1 against `origin/main` in a paired A/B"* — and it moves the
**ceiling**, by keeping an animal eating at the nest while it is short of a
child's price. **The bar does not move, and the bar is what resolution
multiplies.** So §5's conclusion is unchanged by the merge: at `k=2` the same
36-cell body still prices a child at 17,360 against a ceiling that has gone
from 203 to 500.

The mobility table in §4 was re-measured on the merged tree too, because
`creature.rs` gained 345 lines in it and a number quoted from before a merge
is a number about a tree nobody else has: `ant` 5.2% -> **4.9%**, `ant_long`
4.6% -> **3.9%**, and `ant_wide` (47.7% / 50.5%) and `ant_block` (62.1% /
72.8%) **identical to the digit**. The finding does not move.

**What resolution does to that arithmetic is the part this lane owns.** The
bar is `grant + body_energy * cells`, and `body_energy` is not scaled (§2),
so the bar multiplies by the cell ratio:

| species at k=2 | cells | birth cost | against a bank ceiling of ~460 |
|---|---|---|---|
| `ant` | 4 | 2,000 | 4.3x |
| `ant_block` | 36 | 17,360 | 37.7x |

Against 1,040 and 2.1x on the merged tree. **#174 raised the ceiling and did
not touch the bar** — measured, in §5b — so its landing does not change this.
The right-hand side is the stamp, and at 2x density the stamp is the whole
problem rather than half of it.

**The fix is the `1/R` on `body_energy` that §2 declines, taken *with* the
materials.** `stamp_probe`'s `body_energy=` knob already moves
`ant`/`corpse` `food_energy` in step "holding the flesh-pricing invariant" —
that is the shape of the change, and it belongs to whoever owns
`assets/materials/**`. Done there, the bar is invariant under resolution and
this section stops existing.

## 5a. The owner's verdicts, same evening

Both cards came back within three minutes, and the second one **fails the
lane's step 2 outright** — which is the finding, and is why it is written
here rather than left in the queue.

**`20260830T203214131Z-d133c8`, the size proof: rating 5, "Yes."** §1 stands.

**`20260830T203238476Z-08d795`, blind A/B, two 36-cell silhouettes:**
`choice_label` **waisted (10x4)**, comment *"Both are smudges but A is
closer."* `blind_was: [1, 0]`, so the displayed "A" was the stored **waisted**
arm and the prose and the click agree — decoded, as the review skill
requires, because raw they look like a contradiction.

**"Both are smudges" is the sentence that matters.** 36 cells is four times
the ink of the shipped ant and it did not buy an animal. Looking at the
bodies at 28 px per physical unit, which nobody had done, says why in one
frame:

- the `ant` material's palette is **(38,30,28) / (52,42,38) / (28,22,20)** —
  three near-black browns spanning fourteen units of luma. Against sand the
  body reads as a *hole*, not as a creature.
- `ShadeRule::Countershade` fires and is **invisible**, because it grades
  *within* that palette. It is value-only, and `PLAN.md`'s M19 research says
  in as many words that value-only differences vanish at small pixel sizes.
  This is a channel with a writer and a reader whose output cannot be seen —
  a fifth instance of the class `dead-ends.md` says this project has hit
  three times.
- the body is a **solid filled rectangle**. Nothing breaks the outline: no
  legs, no antennae, no gap between head and thorax. The waisted plan's only
  advantage is a one-cell notch, and the owner picked it — which says the
  outline is doing *something*, at the smallest possible amplitude.

**So step 2 is not a resolution problem and more cells will not fix it**, and
that is `plant-appearance-design.md` arriving on the creature line exactly as
it warned: *"a lever that relabels a cell cannot move a silhouette that
texture and colour set."* The plant line lost a phase to this. The creature
line has now lost one card.

**The fork, and it is asked rather than assumed** (card
`20260830T204942304Z-a6b871`): the same 36-cell block in `ant` against
`chitin_pale` (214,202,188), identical cell for cell, colour the only
difference. The previous pair varied outline at fixed colour; this one varies
colour at fixed outline, and between the two answers the next lane knows
whether to spend on a silhouette or on a palette. `CLAUDE.md`'s "a complaint
could mean two things — render both readings rather than spending the whole
detour on the wrong one".

**One thing the fork already rules in.** A creature's whole body is painted
from **one** `material_id`, looked up from the species name in
`place_creature` — so a pale head on a dark thorax, which is the cheapest
real composition an insect has, is **not authorable today**. `chitin_pale`
and `chitin_mid` exist as separate materials and separate species; nothing
lets one animal use two. That is a small engine change (a second material on
`CreatureDef`, chosen by `CellType` at the one call site that already
computes it) and it is the thing to build if the card says colour.

## 6. What a body that reads as an animal *and* walks would have to be

Not built. Written down because the analysis is the expensive part and a
later session cannot reconstruct it from the diff.

**The mechanism: a `Ribbon` — a chain of nodes, each stamping a
cross-section.**

```rust
/// `nodes` nodes following the head's path, each stamping `shape`.
Ribbon { nodes: u8, shape: Vec<(i8, i8)> },
```

`Chain(n)` is `Ribbon { nodes: n, shape: [] }`. It fits the engine's existing
motion model almost exactly:

- `body_after_step` already has the two arms. A ribbon takes the **chain**
  arm at node granularity: the new node path is `[head] + old[0..n-1]`, and
  the cells are the union of `shape` stamped at each node.
- **Passability is nearly as free as a chain's.** Trailing nodes move into
  cells the node ahead has vacated, so the only genuinely new cells are the
  head's cross-section. That is why this is the answer to §4: a 3-tall
  cross-section needs three clear cells at the head, where a 3x3 rigid body
  needs nine anywhere.
- **`relocate_chain` needs no change** if the cell list is ordered
  node-major: `to[i]` receives `from[i]`, so each node's cells stay that
  node's cells, and `ShadeRule`'s "a cell's index is invariant under motion"
  keeps holding.
- **Footing asks like a chain** (head only), because a ribbon can stretch.

**The one unsolved problem, and it is why this is a design rather than a
patch: a ribbon climbing vertically self-overlaps.** With a vertical
cross-section `[(0,-1),(0,-2)]`, a node at `(5,10)` covers `(5,8..10)` and a
node at `(5,11)` covers `(5,9..11)` — two cells in common. The cell list then
holds duplicates, and every consumer that reads `chain.len()` as a body-cell
count is wrong by the overlap: `live_body_cells` over-charges metabolism, and
the corpse conversion books `body_energy * chain.len()` into a ledger that is
asserted closed. Three candidate resolutions, none free:

1. **Make the cross-section perpendicular to the local heading.** Two
   adjacent nodes' sections are then disjoint for all eight directions
   (checked by hand for the diagonal case). It is rotation of a *line*, which
   does not alias the way rotating a shape does, and D1 rejected rotating a
   shape. Cost: the silhouette flexes as the animal turns, which may be
   desirable and is a judge-by-eye question.
2. **Store the node path in `OrganismState::chain` and derive cells.** Correct
   and clean, and it changes what `chain` means for every reader of it.
3. **Dedup and let the body compress on a climb.** Cheapest, and it breaks
   the energy ledger. Rejected on inspection.

**Route 1 is the recommendation**, and the pre-flight is the question
`CLAUDE.md` says to ask before building: *which object does this rule
evaluate?* — a node, and the quantities it needs (a local heading, a
perpendicular) are defined for a node. Budget the ledger check as part of it.

## 7. Not done

- **A body that reads as an animal** (§5a). Attempted, measured, and
  **rejected by the owner** — both 36-cell silhouettes came back "smudges".
  The next move is the palette/composition fork, not more cells.
- **The ribbon** (§6). The lane's step 3 is a design and a measurement, not a
  shipped body.
- **`body_energy` scaling** (§2, §5) — needs `assets/materials/**`.
- **Flight constants** (§2).
- **Plants.** `plant.rs` and `organism.rs`'s growth path read `cell_scale`
  nowhere either, so a tree at 2x is still half its physical size. The same
  defect, a different lane; this report is the evidence that the class exists
  rather than the fix for it.
- **Frame cost at 2x.** Not measured, deliberately: four agents shared this
  box and [`measurement-under-contention.md`](measurement-under-contention.md)
  says any clock here is untrustworthy. The counters above are load-independent
  at fixed parallelism and are pinned to `RAYON_NUM_THREADS=4`.

## 8. Guards, and that each one can fail

Five in `sim::organism::tests`, and every one was put back-to-front and
watched go red for the fault it is named for:

| guard | fault put back | went |
|---|---|---|
| `scaling_a_body_does_not_move_it_off_its_anchor` | anchor grows the wrong way | **red** |
| `a_scaled_rigid_body_is_the_same_physical_size` | *the same fault* | **green** |
| `metabolism_per_frame_does_not_move_with_resolution` | per-cell rates left authored | **red** |
| `a_registry_scales_species_that_arrive_late` | `upsert` does not rescale (the F5 path) | **red** |
| `a_chain_scales_in_length_only` | a chain does not stretch | **red** |

**The second row is the one worth reading.** The size guard is the obvious
test to write and it is *blind* to the bug that actually cost the time — the
box was the right size, in the wrong place. Green there was evidence about
the test, not about the code.

`scaling_to_one_changes_nothing` is the safety claim: `cell_scale` is 1.0 in
every world the app builds today, so the pass is the identity there and
nothing shipped moves.
