# A beetle that can see — E15 built

**Status: shipped, 2026-08-30, on `claude/creature-lane-j-sight-sense`.**
Builds `Reports/creature-vision-sizing-2026-08-30.md` §7's specification
without amending it: reach **64**, **all-round**, occluded by rock and soil,
eye **one cell up**. Every number that report predicted came back inside its
own range when the sense was run for real, which is the strongest thing this
document has to say about the pre-flight.

Supersedes nothing. It is the *build* whose *sizing* is that report.

**One correction to that report, measured here**: the built sense reads
**1,020–1,100 cells per cast** where §5 predicted **485**, because prey has
to be tested in the un-lifted frame and blockers in the lifted one, which is
a second `World::get` per step. The sense is still 0.3% of a frame. §4.

---

## 0. What it is, in one paragraph

**Two brain inputs and no new mechanic.** `PreyNear` says how close the
nearest animal this gut would eat is, as `1 - distance / sight_range`;
`PreyBearing` says which way to turn to face it, signed, positive to the
right, `±1` for directly behind. They are written by a fan of 16 rays cast
from one cell above the head, each marched to the first rock or soil cell or
to 64 cells, whichever comes first. The beetle authors `sight_range: 64` and
one weight, `(PreyBearing, Turn, -2.5)`. **Nothing else in the world has
eyes**, and an eyeless species pays one `i32` compare per tick.

There is no `Strike` verb, no third pheromone plane and no slot 12. E15 is
vision; predation stays unauthorised as a milestone.

---

## 1. It fires, and it fires at the rate the pre-flight predicted

The sizing study's central number was the fraction of beetle samples with
prey in sight at r64 on `wetland`: **median 0.572**, p10 0.389. The built
sense, measured by `CreatureStats::sightings / sight_casts` over 8 generated
seeds and 6,000 frames each:

| arm | casts | seen | **seen/cast** |
|---|---|---|---|
| eyes open, wired to nothing | 18,750 | 9,279 | **0.495** |
| `(PreyBearing, Turn, -2.5)` | 18,750 | 9,459 | **0.504** |

**0.50 against a predicted 0.57**, from a geometric pre-flight that sampled
every 100 frames against an engine that casts every 8. That is the
pre-flight transferring, and it is worth saying plainly because a
specification that survives contact with its own implementation is not the
normal case here.

The number is higher where the terrain is kinder: on `scene=hunt`'s
demonstration ground the same sense reads **177 of 195 casts, 0.908**.

## 1a. The headline test, stated accurately

E15's own test was that `beetles=0` and `beetles=9` should stop running
bit-identical. **Two things have to be said about it, and only one of them
is about this change.**

**It was already broken before this landed.** `mode=ab frames=3000 seeds=2`,
run on this branch: the two *blind* rows of seed 0 differ on `eats` (5 vs 3),
`injuries` (0 vs 1) and `deaths` (0 vs 2). Something between the null being
measured and tonight — the landings of the last week — already made a beetle
move a counter. So "the null is gone" is not this change's claim to make.

**Against the right control — the same nine beetles, eyes on against eyes
off — it is one seed in two.** Seed 0 moves (`eats` 3 → 1, `pickups` 1,630 →
1,805, `injuries` 1 → 2, prey caught 38 → 40); seed 1 is *identical on every
column* while the eye reports **502 sightings**. That is §5 arriving in the
headline: seed 1's beetles are standing somewhere the terrain will not let
them turn.

The claim this change can make is the one §3 measures over 8 seeds rather
than 2: pooled, the beetle spends its sighted time **closer** and catches
**more**. A two-seed table is not a sweep, and it is shown here because it is
the test E15 named, not because it is the evidence.

## 2. Three counters, near side to far side, and one of them is a trap

`CLAUDE.md` asks that a "did it fire" counter be paired with an effect
counter from the far side of the call. This ships five, and the reason there
are five rather than two is that the obvious effect counter turned out to
answer a different question.

| counter | says |
|---|---|
| `sight_casts` | the eye ran |
| `sightings` | it had something to report |
| `sight_dist_sum` | how far away, summed — `/ sightings` is the mean sighted range |
| `sight_facing` | the prey was inside 45° of the heading |
| `sight_approaches` | the head ended the tick closer to what it saw |

**`sight_approaches` cannot fire on the ticks the sense exists for, and this
was measured rather than reasoned.** A walking creature steps only
ahead-left, ahead or ahead-right, so when prey is *behind* it no available
step reduces the distance however hard it turns. A harder turn therefore
*lowers* the ratio. At a zero-weight control it reads **0.132**; with
pursuit wired it reads **0.130**, and at the strongest setting **0.083** —
while prey actually caught went **up** at every step. It is arithmetically
correct and about the wrong thing.

**`sight_facing` is confounded by range in the same way**, in the opposite
direction: a beetle that closes on prey is *near* it, and near prey a
one-cell lateral offset is a large angle. It falls from 0.301 to 0.227
across exactly the settings that bring the animal closest.

**The two that are clean are `mean sighted range` and prey caught**, and
they are clean because neither is a per-tick ratio over a population whose
geometry the change itself moves.

## 3. The pursuit sweep

`predation_probe mode=sweep frames=6000 seeds=8`, 9 beetles and 52 ants on
generated `wetland`. Row 1 is the control — **eyes open, wired to nothing** —
which is the positive-control half of the house rule: a counter that stays
quiet when nothing is wrong has not been shown to move when something is.

| `Turn` weight | `Persist` release | seen/cast | facing/seen | appr/seen | **mean range** | **prey caught** | injuries |
|---|---|---|---|---|---|---|---|
| 0 | 0 | 0.495 | 0.301 | 0.132 | **15.2** | **302** | 5 |
| **-2.5** | **0** | 0.504 | 0.293 | 0.130 | **12.5** | **323** | 7 |
| -2.5 | -3.0 | 0.532 | 0.273 | 0.118 | **12.2** | **345** | 7 |
| -2.5 | -6.0 | 0.519 | 0.227 | 0.083 | **9.1** | **355** | 7 |
| -5.0 | -6.0 | 0.505 | 0.291 | 0.107 | 12.4 | 329 | 6 |

**Two independent far-side counters move together and monotonically** down
the first four rows — the beetle spends its sighted time closer (15.2 → 9.1
cells) and catches more (302 → 355). That is the result.

### 3a. Why the shipped animal takes the first row of the two and not the best

`(PreyNear, Persist, w)` is the stronger lever and it is **deliberately not
shipped**. `Persist` is the straight-ahead score; with nothing authored it
sits at 1.0, and a `Turn` output saturates at 1.0, so a turn candidate can
at best *tie* the straight-ahead default. Releasing persistence in
proportion to how near the prey is breaks the tie, and the table says it
works.

It also cost a close-quarters case. In the sealed 20-cell chamber
`a_predator_eats_a_creature_and_needs_no_predation_code_to_do_it` builds,
swept over 8 starting positions:

| arm | ant killed outright | ant cells taken |
|---|---|---|
| control | 6/8 | 8/8 |
| `Turn` only | 6/8 | 8/8 |
| `Turn` + release | 4/8 | 8/8 |

**The predation rate is identical — one catch per run in every arm.** What
moves is whether the bite landed on the head, which for a `Chain(2)` ant is
the difference between death and injury, and which is a chaotic tie-break
rather than a worse predator. Eight positions is not a sweep either.

So the evidence is a real disagreement between a field sweep and a corridor,
at small samples on both sides, over a knob whose whole subject is how an
animal *moves*. `CLAUDE.md` settles that class of question by eye rather
than by picking the number one likes. The shipped beetle turns and nothing
else; the knob is `predation_probe`'s `release=`, the sweep is above, and
the question is with the owner.

## 4. What it costs, and where the specification was 2x out

The sizing study priced a radius-64 fan of 16 rays at **485 cells read per
beetle per cast** and **0.004 ms/frame** at five beetles. It priced it that
way deliberately: a charge that small is below what a wall clock on a shared
box can resolve, so the argument runs through a deterministic read count and
a directly measured 14–16 ns per `World::get`. `CreatureStats::sight_cells_read`
is that same quantity in the engine, so the built sense can be held against
its own specification instead of inheriting it.

**It reads about twice what was predicted:**

| where | cells read per cast |
|---|---|
| sizing study, `cast_fan` | 485 |
| built sense, generated `wetland`, 2 seeds | **1,020 and 1,077** |
| built sense, `scene=hunt` | **1,100** |

**The cause is a frame of reference, and getting it wrong would have been
much worse than paying for it.** Prey is tested in the *un-lifted* frame and
blockers in the lifted one, which is the geometry the study's own pairwise
test used — the line runs eye to eye while the animal it is looking for
stands on the ground. That is a second `World::get` per step wherever the eye
rose, so a little over double. Testing prey at the *lifted* cell instead
costs one read and sails every horizontal ray straight over every ant in the
world: the sense would fire on almost nothing and read as a design failure
rather than a frame-of-reference one.

At the study's own 14–16 ns per read that is **0.009 ms/frame at five
beetles** rather than 0.004 — **0.3%** of a 2.98 ms mean frame, still below
what a clock resolves, and still under 10% of a frame only past roughly 170
predators rather than 358. Nothing about the decision changes; the number in
§5 of that report should be read as half of what a build costs.

**Nothing scans for beetles**, which is the part of §5 that transferred
exactly. Its `locate` arm exists to keep the harness's own whole-world sweep
out of the answer — it overstated the cost **thirtyfold** — and the built
sense is dispatched by the active-site scheduler at the creature's own
position, which is what that arm was arguing for.

**And `ascii` is unchanged by construction, not by luck.** No scene in it
runs a species with `sight_range`, the gate is at the call site, and
`a_species_with_no_sight_range_sees_nothing_and_costs_nothing` asserts a
world of eyeless animals casts exactly zero rays. Measured on this branch:
31 scenes, 0 skipped, `frame cost with 143 live organisms: worst 37.681 ms,
mean 4.319 ms over 12,000 frames`.

## 5. The finding that is not about vision

**`BrainOutput::Turn` is very nearly inert for a surface walker on level
ground, and nothing had measured that before.**

A creature's three candidates are ahead-left, ahead and ahead-right, and the
turn output biases the two outer ones. On a dead-flat floor, for an animal
standing on it: the **downward** diagonal is inside the floor and fails the
passability test, and the **upward** diagonal is over thin air and fails the
foothold test. So a turn request toward either is discarded, and the heading
changes only by `tumble` — a *random* re-roll.

Measured on a hand-built stone slab, one beetle and eight ants 37 cells
apart, eyed against blind: the eye reported **139 sightings of 195 casts**
and the two arms came back with **byte-identical** `moves 898 blocked 28
falls 167 pickups 4`. The sense fired perfectly and could not steer, because
the terrain forbade turning at all.

Two consequences, and the second is the one to carry:

- **The demonstration scene needs relief and now has it.** `scene=hunt` lays
  a one-cell ripple, which puts a step under the diagonal and leaves the
  line of sight near-horizontal. A first attempt used a shallow *ramp*
  instead and traded one failure for another — sight lines then ran along a
  littered slope and `seen` fell from 139 to **15**, which is §4 of the
  sizing report arriving by a second route.
- **This is a movement finding, not a perception one**, which is exactly the
  split E13 and E15 were written around. Filed as
  `open-bugs-handoff.md` §R4.

It is also why the field sweep is the evidence and the slab is not: generated
terrain has slopes, and a slope is what gives a turn somewhere to put its
foot.

## 6. What was checked, and how each guard can fail

Eight guards in `creature.rs`, each written so that green is not the default
state:

| guard | goes red if |
|---|---|
| `a_beetle_sees_an_ant_across_a_bare_floor` | the sense never fires, or nearness stops scaling with distance |
| `a_wall_stops_the_sight_line` | occlusion is stuck off |
| `floor_litter_does_not_blind_a_beetle_but_a_two_cell_pile_does` | **both halves**: the eye lift stops working, *or* it "works" by turning occlusion off |
| `prey_past_the_eye_is_not_seen` | the radius stops being a radius |
| `a_species_with_no_sight_range_sees_nothing_and_costs_nothing` | the opt-in leaks — asserts the eyed control sees first, then that the counters stay at exactly zero |
| `the_ant_has_no_eyes_and_the_shipped_ant_is_unchanged_by_this_input` | `ant.ron` grows an eye by accident |
| `the_bearing_says_which_way_to_turn` | the sign inverts, or "directly behind" collapses to 0 |
| `a_beetle_does_not_see_another_beetle` | the prey filter becomes a creature-detector |

**One of them was written wrong and the scene caught it.** The bearing
guard's first draft built a ledge above the beetle and stood an ant on it —
and the ledge blocked the very ray that would have reached the ant. A scene
that contradicts the code reads exactly like the mechanism being inert. It
asks the question with the *heading* now instead, which needs no terrain:
facing north, east is on the right hand.

**And the harness lied first, in the way this repo has a name for.** The
first run of `mode=sweep` came back **bit-identical across all eight
settings** — 7500/4301/528 on every row. The species genome is compiled from
its wiring lists once at load, so overriding `instincts` and calling
`set_creature` changed nothing a creature ever thought with. Identical output
across settings is the tell that a knob was never connected, and it is the
only reason that sweep is trustworthy now.

## 7. Gates

`cargo test --lib` **1,106 passed / 0 failed / 54 ignored** · clippy on the
CI toolchain **1.98.0 clean** · `ascii` **31 scenes, 0 skipped** ·
structural acceptance **all cases met their expectations** · `docscheck`
clean · `bugindex --check` index current and identifiers unique.

**The genome manifest moved, lawfully**: `BRAIN_INPUTS` 16 → 18 lights up
two columns of a 64-wide reserve that already existed and were already zero.
`GENOME_LEN` is unchanged at 12,352 and not one existing weight moves — this
is exactly the append S2 reserved the dimensions for, and the pinned literal
in `the_genome_manifest_is_pinned` is updated with that reasoning at the
line.

## 8. What this does not answer

- **Whether it feels like hunting.** The sweep says the beetle gets closer
  and catches more; whether that reads as a predator in motion is the
  owner's, and the card is posted.
- **Radius 128.** Still not measured, still the honest gap the sizing report
  left. The curve was climbing at 64.
- **Foliage.** Still binary, still not shipped as a blocker, still the
  attenuation the ethos asks for and nobody has priced.
- **Whether the pursuit weight should be stronger** is §3a's question and
  it is the owner's.

**And one thing this deliberately no longer worries about.** The brief for
this work flagged `open-bugs-handoff.md` §R3 — *no creature body above two
cells leaves a living colony* — as a possible confound, since a beetle is
2x2. It is not one, and the reason is worth carrying rather than re-deriving:
§R3 was root-caused the same night as *a `Chain(n >= 3)` overwrites its own
head cell in `relocate_chain`*, because `body_after_step` builds the next
body as `[head, chain[0], .., chain[n-2]]` and a head stepping into its own
body's cell puts one position in that list twice. **The colony was never
dying — a head-cell counter was reading zero over a living population.**
`beetle.ron` is `body: Rigid(..)` and not a chain at all, and the shipped ant
is `Chain(2)`, whose two positions are always distinct. Neither animal in any
measurement here goes near that path, so every population figure above is
this change's to own.
