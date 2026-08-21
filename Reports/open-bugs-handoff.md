# Open bugs handoff

Rewritten at the end of the session that landed `15b2e51` … `ad1e227`.
Everything here was measured, not reasoned — where something is a guess it
says so, and where a plausible idea was measured and found wrong it is
recorded with its numbers so it is not tried twice.

Read `CLAUDE.md` first; it holds the method these bugs keep re-teaching.

---

## Open

### 0. The organism support search asks the wrong question — see `Reports/felling-blockers.md`

Not new, but newly written up. `structural::organism_is_supported` anchors
on `MaterialKind::Solid` (soil is a `Powder`, so it anchors nothing) and
searches outward from the cell under test bounded by
`max_unsupported_span`, so it answers "am I within 8 hops of stone" rather
than "can I reach a root". Any structural check fired mid-crown therefore
amputates the tree — measured at 772 cells against 20,213 (`plant.rs`'s
`shed_stranded_leaves`).

It is latent rather than live only because every organism path
deliberately schedules no check: growth, germination, abscission and
`player::shake` all say so in place. It goes live the moment anything
does, and it is the blocker under felling. The fix, the cost, and the six
paths that would trigger it are in `Reports/felling-blockers.md`.


### 1. Whiskers on a spreading front (the remaining half of "banding")

One-cell-tall sheets of water with open air above *and* below, drawing as a
comb of detached horizontal ledges along a spreading front. Reported from
live play. Distinct from the row banding that was fixed — that was a fill
deficit *inside* the body; this is the shape of its *edge*.

Barred, not fixed: `update.rs`'s
`a_spreading_front_does_not_shed_a_comb_of_detached_ledges`, at 400 against
a measured 290. The bar holds the line; it does not claim a fix.

**Three candidates measured and rejected** (numbers in the test's own doc):

| tried | result |
|---|---|
| Disable `find_lateral_descent` | −75% whiskers, and water reads as sand again — the original bug |
| Land the mover at `(tx, y)`, fall next frame | whiskers 2540 → 1635, but enclosed holes 289 → **1040** |
| Shrink `LIQUID_LATERAL_REACH` | pure trade against levelling, no path to zero: 24/12/6/3 → whiskers 290/175/151/119, levelling 343/557/1017/1661 frames |

**What the measurements say about the cause**, which is *not* what it looks
like: `find_lateral_descent` is not teleporting water. **75% of its moves
are a single-cell diagonal step and only 3% land with air two cells below**,
and whiskers survive at reach 3. So they are not primarily long jumps. They
look instead like the surface monolayer advancing one diagonal step per
frame with nothing above it to refill the row it vacates — which raises the
real possibility that the honest fix is not in the movement rule at all, but
in how a one-cell-thick sheet is *drawn*. See the grain prototype below.

Note this is **not** the VOF flotsam-and-jetsam the liquid research reports
diagnose. Their fix is a three-cell height function for partial-fill
droplets orphaned by interface reconstruction; measured here, the drained
basin strands 54 cells while producing **zero** films, and the films
elsewhere are mostly *full* cells. Do not adopt that fix without first
measuring which mechanism is producing the cells.

### 2. Sand-into-water displacement

Unchanged from the previous handoff and still the design gap it was.
`abffff2` is **kept** — the decision was made explicitly with numbers:

| metric | before `abffff2` | now |
|---|---|---|
| water rise | **29 rows/frame** | **1 row/frame** |
| sand/water/sand stripes | 41 | **1379** |
| sand cells with air beneath | 86 | 115 |

Water crossing 29 rows in one frame is a gross physics violation; the
striping it traded for is ugly. **Option 1 from the old list (sideways-
preferring displacement) was implemented as a mass-conserving 3-cycle and
measured: it does nothing** — stripes 1379 → 1370, stall unchanged, and it
*regressed* water rise to 2 rows/frame. Reverted, not committed. It cannot
work as specified: inside a pool there is no free-or-lighter cell beside the
mover, so the sideways path only opens where the blob is already at a free
surface, which is where striping was never the problem.

The striping follows from two individually-correct premises — displaced
material moves at most one row per frame, and displacement is a straight
vertical swap — so no local `try_move` tweak can remove it. Remaining
options: let an unsupported refused mover fall (fixes the 115 floating cells
only), move a coherent body *as a body* (`rigid.rs` — the only thing that
removes the premise), or accept it.

### 3. Scheduler under-enforces `max_active_tips` — **FIXED, and the tripwire earned its keep**

**Resolution (2026-08-17):** the tripwire fired exactly as this section
predicted it would — the session multiplicative crowding stopped crowded
tips from dying, simultaneous tips finally approached the cap, and the
under-enforcement measured 19 against 14. Fixed by the route this section
also predicted: `organism_active_tip_count` counts the organism's own
cell list (Decision 2's sidecar, maintained at the `World::set` seam under
both drivers) instead of scanning the schedule heap, so in-flight
dispatch is no longer invisible. That took the overshoot to 16, and the
remainder was a second gate nobody had needed before: `break_buds`
creates frontier too and never checked the cap — `supportable` is now
throttled by `max_active_tips`, one gate for both creators. The tripwire
test asserts the cap holds through 8,000 frames and passes.

The original finding, kept because its reasoning about *why it could not
bite yet* was correct and is the reason the tripwire existed at all:

### (was) Scheduler under-enforces `max_active_tips` (a tree bug) — measured, and it cannot bite yet

Review finding. `scheduler::step` pops the entire due batch into `due_sites`
*before* dispatching any of it, so `world.active_sites` does not hold the
batch while `plant::tick` runs. `organism_active_tip_count` counts only the
heap, so it cannot see any tip in the current batch — and when a tree's tips
all come due on the same frame, which is the normal case, the count it
returns is far too low and `Behavior::Grow`'s cap (`max_active_tips`, 14 and
10 in `tree.ron`) is under-enforced. **The reading is correct.**

**Now reproduced properly, and the answer is that the cap is unreachable.**
The previous attempt "grew no tips at all (`plant_tree` on a soil floor with
no field step)" — germination is light-gated, so a run that never steps the
field never germinates and can only ever report zero. With fields stepped
(`plant.rs`'s `a_trees_simultaneous_tip_count_stays_within_its_species_cap`,
8,000 frames), the **peak simultaneous `GrowingTip` count for one tree is
1**.

Not "under the cap" — one. Tip retirement converts a `GrowingTip` to
`MatureBody` in the same tick it grows, with the child carrying the frontier
forward, so a lineage holds exactly one live tip and branching only briefly
makes it two. `max_active_tips: 14` was sized for the pre-retirement system
where tips persisted; against the current one it has nothing to do.

So the bug is **real as read and unreachable as built**: a cap that is never
approached cannot be exceeded, however badly it is checked. Deliberately
*not* fixed on that basis — the fix (dispatch-one-at-a-time, which changes
the cap's meaning and risks a tip producing a due-now tip in the same frame;
or making the in-flight batch visible to the count) costs more than the
defect currently does.

**What changes that:** `Reports/plant-substrate-v2-design.md`'s bud break
(retrofit step 9) exists specifically to let a mature tree open new
frontiers, and is the first thing that would push simultaneous tips toward
the cap. The reproduction above is kept as a tripwire and should start doing
real work exactly then. Decision 2's sidecar also fixes it structurally for
free — `organism_active_tip_count` becomes a count over the organism's own
cell list rather than a scan of the schedule (design doc §3e), which has no
in-flight-batch blind spot at all.

### 4. Levelling is O(width²)

Not a bug so much as a known cost, quantified here because the previous
handoff's numbers were read before convergence and were wrong:

| frame | 1024-wide pool tilt | wall clock |
|---|---|---|
| 8,000 | 29 cells | 2¼ min |
| 40,000 | 3 cells | 11 min |
| 70,000 | 1 cell, asleep | 19 min |

It **does** converge flat and **does** sleep — there is no limit cycle, and
the earlier "residual tilt" figures were mid-convergence readings. A 512
world (the sandbox's own width) is ~4x faster: near-flat around 2 minutes.
The real cost is CPU, not appearance: the visible defect is gone early and
what persists is chunks awake doing invisible fill shuffling.

This is what the heightfield bodies exist to fix (O(width) instead), and
they are blocked on the promotion gap below.

### 4b. ~~A cell alone in the air drops its column's skyline~~ — **CLOSED, by removing the inference entirely**

Logged and closed in the same session. It was the tail of "shade under a
tree is way too intense": the skyline was the topmost non-empty cell, so
anything in the air above a column made everything below it draw as the
inside of a cave.

Fixed by not inferring it. `World::sky_surface` records the top of the
ground once, on the world's first frame, and nothing revises it —
`Reports/underground-definition.md` has the reasoning and the numbers.

**What is worth carrying forward is why every inferred version failed**, and
it is a case of `CLAUDE.md`'s "when a rule must tell apart two things that
can look identical, state the difference as data". Four shapes have to be
distinguished — a hill, a shaft someone dug, a roof someone built, and a
grain in mid-air — and from the world as it stands they are the same
arrangement of cells. Measured on the last inferred version, which took the
topmost cell and then repaired any column with higher ground within six
either side:

| shape | verdict |
|---|---|
| one floating cell | 20 rows of cave under it |
| plank 1 to 51 wide | identical to the floating cell |
| shaft ≤ 12 wide | tunnel (correct) |
| shaft ≥ 13 wide | open daylight 35 rows into the mountain |

No reach setting fixes that: the repair rule had a width threshold in one
direction and no rule at all in the other, and mining is the activity that
walks a shape across exactly that threshold. The difference between "I dug
this" and "this is a hill" is *history*, not geometry, and history has to be
stored.

### 5. Automatic promotion — blocker removed, still not ready

`promote_liquid_body` is called **only from tests**, so `liquid.rs` — the
pipe solver, the seam, ~1000 lines — never runs in play and every bug in it
is latent.

**The documented blocker is now fixed.** `127e177` reverted automatic
promotion because "the persistent-flux solver has no mechanism to drive an
internally-level body to expand into open floor space beside it", and
`edge_with_room` is that mechanism (`95c917f`, `68371d7`). A promoted body
that can still spill no longer sleeps through it and sheds its edge column
back to the CA, which is what §6c always said outflow should be.

**But promotion is still not worth turning on**, measured on the exact
scene the revert names — the 100-column block from
`a_wide_deep_water_column_levels_out_instead_of_only_eroding_at_the_edges`,
promoted deliberately at frame 0:

| | spread at 6000 |
|---|---|
| before the fix | **106, frozen from frame 10** |
| after the fix | 57–68, still moving |
| no promotion at all (plain CA) | **128** |

So the freeze is genuinely gone — the body sheds steadily, 100 columns and
4.9M fill down to 50 and 2.45M — and it still ends up *worse* than leaving
the water to the CA. Shedding one column per `DEMOTE_COOLDOWN_FRAMES` is
simply slower than the CA spreading it directly.

### 6. The heightfield does not deliver the speed it was built for

**Measured, and it inverts the premise the whole subsystem rests on.**
Report r2 §5's argument for the heightfield is a *speed* one — "levels a
pool in **O(width)** rather than the current O(width²)". Levelling time to
the 2% flatness bar, on a walled basin with water spanning every column
(the shape most favourable to the body — it never has to spread, only
redistribute):

| columns | CA | promoted body | ratio |
|---|---|---|---|
| 50 | 77 | 204 | 2.6x slower |
| 100 | 307 | 742 | 2.4x |
| 200 | 1,323 | 2,421 | 1.8x |
| 400 | 5,659 | 6,864 | 1.2x |

The CA quadruples per doubling — O(width²), as documented. **So does the
body** (3.6x, 3.3x, 2.8x per doubling). It is not O(width). The ratio is
closing, so a crossover presumably exists somewhere past 400 columns, but
the sandbox's world is 512 wide and the heightfield never wins on speed
inside it.

The persistent-flux solver was supposed to avoid exactly this — §7a's
"flux must be persistent state, **or you have rebuilt diffusion**". The
measurement says diffusion is what it behaves like. Whether the flux is
not persisting, or a clamp is throttling the wave, is unknown and is the
thing to look at first.

**What the body does measurably win at is accuracy, not speed**: it
finishes at a flatness of **1** where the CA leaves **11**, because
`terminal_snap` solves the exact analytic equilibrium. That is a real
property and worth something — it is just not the property the subsystem
was justified by.

Before spending anything more here, settle what the heightfield is *for*.
If the answer is exactness, it is much cheaper to reach that another way.
If it is speed, the flux solver needs diagnosing against §7a first, and
nothing downstream (promotion criteria, the trigger, the deferred B-8/B-2/
B-6/B-7 bars) is worth building until it delivers.

Two bugs found while measuring this are fixed: a body shed down to one
column stranded its fill instead of handing it back (`94a0c12`), and
`edge_with_room` always picked the left edge, so a body spread in one
direction only (`68371d7`).

The promotion *criteria* question — promote only once contained, since §4a
already argues quiescence is the wrong gate — is now moot until the above
is settled.

---

## Closed this session

- **Chunk-seam cliffs** (powders) and **terracing** (liquids), both from the
  chunk-by-chunk sweep order. `FLAG_UNDERCUT`. The previous handoff's
  leading hypothesis (seam cells never getting `flowing()`) was **measured
  false**.
- **Dark lines on horizontal chunk seams.** Fixed by sweeping chunk rows
  bottom-first (`pass_key`) rather than by penalising the crossing cell —
  two attempts at the latter were reverted, because they replace the tear
  with a *throttle* at the same seam (2236 and 1948 summed row-fill deficit
  against 988 for correct ordering).
- **Chunks awake but never swept.** `is_settled` now answers from
  `sweep_region`.
- **Four of five review findings**: liquids scanning through a promoted
  body's cells; explosions spawning debris made of `material::EMPTY`;
  `try_extend` freezing CA water it did not claim; `absorb_liquid`
  destroying fill at a body's edge. The fifth is §3 above.

`particle::step`'s landing check was flagged by the same review and
**deliberately left alone** — the reasoning is recorded in place.

---

## Awaiting a decision

Five `GrainMode` variants are prototyped behind a runtime switch, default
unchanged, with GIFs generated for comparison (`examples/filmstrip.rs`,
`grain=`). They address the report that a pool reads as *static* in the
middle while its edges move — the grain is keyed on world position, so water
flows through a pattern nailed to the screen.

Worth knowing before choosing: a settled pool changes 431 cells per step
with **zero occupancy changes**. Its interior genuinely does not move. So
`Cell` grain makes moving water *read* as moving, which it currently does
not, but nothing can animate an interior that is standing still — `Muted`
and `Animated` are the variants aimed at that half, and `Animated` is the
only one that costs the dirty-rect render skip.

---

## ~~Open~~ **CLOSED** — the three the polarity review raised (M18 plant v2)

All three are now fixed, each with a guard verified to fail against the old
code. Kept here rather than deleted, because what they have in common is
worth more than any one of them: **all three were invisible to the suite
for the same reason — nothing tallies held water, and nothing walked the
frontier cell types.** A new test that covers either of those covers a
whole class.

| finding | fixed in | guard |
|---|---|---|
| allometry gate permanently retiring roots | `ab39721` | `a_root_tip_that_ages_out_retires_instead_of_becoming_a_phantom` |
| `Grow` into soil destroying stored water | (next commit) | `a_root_growing_into_soil_displaces_its_water_rather_than_destroying_it` |
| capillary exchange over-filling a neighbour | `13bce0a` | `capillary_flow_never_pushes_a_neighbour_past_its_own_capacity` |

Two of them turned out differently from the review's framing, and the
difference is recorded at each site:

- The root bug was **not** fixable by marking the "not now" gates as
  `found_candidate`, which is what the framing suggests. That breaks
  `a_tree_eventually_stops_growing` immediately — the staleness counter is
  the only thing that makes growth terminate. The real defect was that
  ageing out had no landing site for `RootTip`.
- The capillary bug needed a **second water-holding material to be
  testable at all**. With equal capacities the drier cell is by definition
  below its own limit, so the clamp can never bind. The guard writes a
  `tightsoil` into a temp dir and loads it additively.

The original descriptions follow, since the reproductions are still the
cheapest way back into each area.

### 1. `MAX_ROOT_FRACTION` feeds the staleness counter, permanently retiring roots

`plant.rs`'s allometry gate `continue`s without setting `found_candidate`,
so a *transient* root:shoot ratio counts as a failed tick. After
`STALE_LIMIT` blocked ticks the `RootTip` stops rescheduling — and
`organism_upkeep` skips frontier cell types, so nothing ever visits it
again. It loses `Absorb`/`Transpire` permanently while still counting
toward `root_cells`, which ratchets the very ratio that blocked it.

The gate is meant to say "not now", which is the "temporary shortfall"
framing `Divide`'s own resource gate uses — that path sets
`found_candidate` and this one does not. Suspect this first if roots look
like they stop drinking on a mature tree.

### 2. `Grow` into soil destroys the soil's stored water

Growing a root into a penetrable soil cell overwrites the cell wholesale,
replacing its `aux` — which for a `Powder` is moisture — with cell-type
bits. In the `forest` scene each root cell silently deletes
`SOIL_FIELD_CAPACITY` (620) units; a 100-cell root system loses roughly 62
water cells' worth. No conservation tally covers held water, which is why
nothing noticed.

Note this interacts with the still-open `water_capacity` item below: any
liquid-conservation test taught about held water will start failing here.

### 3. Capillary exchange can push a neighbour above its own capacity

`update.rs`'s capillary step bounds the transfer by *this* cell's
`water_capacity` and writes `there + moved` without checking the
neighbour's. Latent today because `water_capacity` is opt-in and only
`soil` has it, so every exchange is soil-to-soil with equal capacity. It
goes live the moment a second water-holding powder exists with a different
capacity — which is exactly what widening `water_capacity` to sand would
do.
