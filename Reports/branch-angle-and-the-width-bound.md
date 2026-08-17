# Branch angle, the straightness budget, and the width bound they exposed

**Status: built, measured, working, and NOT merged.** It lives on branch
`plant-branch-angle`. It makes `a_tree_eventually_stops_growing` fail, and
the failure is a real gap in the model rather than a stale test bar — which
is the whole reason this file exists instead of a commit on the main line.

Follows `plant-appearance-design.md` §2.3–2.4, which named branch angle and
the missing internode as causes 3 and 4 of "three species look like one
plant".

## 1. What was built

Two parameters on `Behavior::Grow`, deliberately landed as one mechanism:

- **`branch_angle: ByOrder<f32>`** — the angle in degrees at which a lateral
  leaves its parent axis. Index `n` is read by a cell *of order n* when it
  throws a child, so the list is "the angle at which each tier throws its
  children".
- **`internode: ByOrder<u8>`** — how many steps a fresh lateral holds its
  departure direction before light, wind and tropism get a vote.

**Neither works without the other**, which is why they are one change. A
lateral that departs at 90° is re-scored against `upward_weight` on its very
next step and bends straight back alongside the trunk: that is the
parallel-ropes look, and an angle alone does not touch it. The straightness
budget costs no per-cell state — a lateral is rescheduled with
`plastochron: 0`, so the lineage step the active site already carries *is*
its age in cells.

## 2. Three attempts, and the counter that made the first two visible

The achieved-departure-angle counter (`OrganismState::departure_angle_sum` /
`lateral_departures`) reports the angle laterals *actually left at*, not how
often the scoring ran. That distinction is the only reason this converged.

| attempt | achieved angle, target 70° |
|---|---|
| weight the primary candidate score by angular closeness | **40°** |
| discard the score, weight by closeness alone | **48°** |
| give the lateral its own candidate set | **63–87°** |

The first two both "worked" — the code ran, the counter incremented, the
sheets looked plausible — and both did almost nothing. `CLAUDE.md`'s "two
fixes failing the same way means the approach is wrong, not the tuning"
applies exactly, and the number is what showed it.

**The cause was the candidate set, not the weighting.** A lateral reused
`candidates`, which is the *primary* scoring's survivors — filtered on
`score > 0` against a preference carrying `upward_weight` (0.9 on a trunk).
A near-horizontal step off a vertical axis scores about zero on every term
and does not survive that filter, so a wide departure was not merely
unlikely: **it was never in the set**. Laterals now score over every growable
neighbour, with crowding still the only environmental vote, because a
branch's departure angle is a developmental property and the *primary* child
is what answers to the environment.

A counter that had said "the angle code executed 400 times" would have been
true and useless. This is why the counter records the achieved angle.

## 3. What it bought

Paired probe, 8 trees / 30,000 frames, `tree`, against the merged state
(palette bands + `leaf_cluster: 10`):

| | before | after |
|---|---|---|
| mean cells | 3,414 | 3,720 |
| smallest individual | 670 | 1,724 |
| foliage share | 11% | 10% |
| crown profile | [100, 80, 41, 0, 0] | [100, 95, 0, 0, 0] |
| foliage centre | 80 | 89 |
| rigid steps / plant | — | 334–913 |
| mean departure angle | — | 63–87° |

By eye (`target/crown/angle-tree.png` against `foliage-tree.png`): trunk →
limb → twig is legible. Limbs leave the bole at a wide angle and run
straight, where before the plant was a tangle of similar-looking strands.
The crown flattened into a higher, shallower plate — a tuning question, not
a defect.

**Unlooked-for, and the reason this is worth finishing: the conifer lean is
largely gone** (`target/crown/angle-conifer.png`). Handoff §4's open bug —
every conifer sweeping up-and-right in unison, three theories already dead.
This was predicted in `conifer.ron`'s own comment *before* the run: a tier
that holds its departure direction for ten steps cannot be walked sideways
by the sign-of-heading ratchet in that time, so if the lean survived it was
not accumulated drift. It did not survive. **This is picture evidence, not
proof** — the left/right side counter the handoff asks for is still the
thing that would confirm it, and should be added before the bug is closed.

## 4. The gap it exposed: the model bounds height, not width

`a_tree_eventually_stops_growing` fails. The budget is **not** a stale bar,
and raising it would have been the exact rubber-stamp this repo keeps
warning about. Measured curves, single tree, the test's own scene:

| planting position | headroom | outcome |
|---|---|---|
| `(50, 20)` — what the test uses | 20 rows | **never plateaus**: still +180–400 wood per window at frame 295,000, 24,946 cells |
| `(100, 190)` | 190 rows | **plateaus at frame 180,000**, 16,943 wood, flat for six consecutive windows |

And in `PlantScene` (200 rows of sky) a single tree's `MatureBody` count is
identical at 120,000, 200,000 and 300,000 frames.

So termination is intact where a plant can grow *up*, and absent where it
cannot. The reason is in `plant.rs`:

```rust
// Rows *above* the collar; y grows downward.
let height = (collar - y).max(0) as f32;
let margin = turgor_source - turgor_per_cell * height - turgor_yield;
```

**The turgor bound is purely vertical.** A cell two hundred columns sideways
at collar height has `height = 0` and full margin. Nothing in the model
bounds lateral extent at all — width is limited only by self-shading and
crowding, which are enough in a tall scene and are nothing in a shallow one.

Wide branch angles did not create this. They made lateral spread efficient
enough to reach it, in a test scene that plants a tree with twenty rows of
sky — a scene `common::PlantScene`'s own doc already calls void ("a run whose
canopy reaches row 0 should be discarded, not interpreted").

### The fix this argues for

**Bound turgor by path length from the collar, not by height.** Water
potential falls with the hydraulic path the xylem actually has to lift and
push through, not with altitude — a 200-cell horizontal limb is under the
same constraint as a 200-cell trunk, and `Reports/tree-extension-biology.md`
§2c's own source is about path resistance. That is one quantity change and it
bounds both axes with the mechanism already there, rather than bolting on a
width cap.

It is not free: path length is not currently tracked per cell, and the
property that made height attractive — "the apex stays at the top and the
collar at the bottom, permanently", so the signal never equalises when
growth stops — has to be shown to hold for path length too. It plausibly
does (path length from the collar is also monotone and does not relax), but
that is an argument, not a measurement.

**This is a design call, not a tuning one, and it is why the branch is not
merged.**

## 5. What to do with this branch

1. Decide the width bound (§4). If path-length turgor lands, re-measure this
   branch against it — the failing test should then pass in its own scene,
   because a tree that cannot go up also cannot go sideways forever.
2. Add the left/right departure counter and confirm the conifer lean (§3)
   rather than closing it on a contact sheet.
3. Re-derive `a_tree_eventually_stops_growing`'s budget once, from a measured
   curve, in whatever scene it ends up in. Note that a headroom scene plateaus
   at 180,000 frames, so a >2x budget there is a very slow unit test; the
   honest options are a narrower world or a coarser window, not a bigger
   number quietly.
4. The crown flattening (profile `[100, 95, 0, 0, 0]`, foliage centre 89) is
   for the owner's eye. Lower `branch_angle` on the outer tiers is the knob.
