# Foraging range: the counter that was measuring loitering

**Status: MEASURED, instrument landed, 2026-08-23.** The metric described
here is built and on the `ascii` gate. The two findings it produced —
a colony that works an 18-cell bubble, and litter that lands in the canopy
— are measurements, not fixes. Nothing behavioural changed.

Written because the creature plan's next three stages all turn on
quantities nothing in the engine could report.

---

## 0. First: the branch this session was asked to merge does not exist

Recorded at the top because the next session will otherwise repeat the
search, which took a while and produced a definite answer.

The task was to merge `origin/main` into `creatures-m18` — a branch holding
eleven commits (S1 crowding / `synapse_fraction` / the `Feed` verb; S2 the
584-slot genome, manifest hash, `BRAIN_OUTPUTS` 10; S3 `food_energy` /
`food_class` / `worth_in_aux`, `body_energy`, corpse worth in `Cell::aux`,
the `EnergyLedger` rework and its conservation guards). **None of it is
reachable.** Checked, not assumed:

- No `creatures-m18` ref on `origin` (`git ls-remote`: 33 refs, none
  matching `creat`).
- No ref anywhere carries the work. Swept **every** remote branch for the
  tell — `GENOME_LEN` — and all 29 with a `brain.rs` read `248` with
  `BRAIN_OUTPUTS: usize = 9`. The 584-slot genome is on nothing.
- Nothing unreachable locally: `git fsck --lost-found` is empty, the reflog
  has four entries and all are this container's clone, `git stash list` is
  empty, and `git log --all -S"food_energy" -- src/` finds no commit.
- No second checkout on disk (`.claude/worktrees/` does not exist here; the
  container was cloned fresh).

The merge base `cc40557` *is* an ancestor of `main`, so main carries the
creature work up to it and no further: `eat_energy`, `GENOME_LEN = 248`,
`BRAIN_OUTPUTS = 9`, and `litter.ron`'s comment still saying "the creature
branch's S3 will extend this with the edibility fields". That is exactly the
pre-merge state the task describes, which is the confirmation that main is
where it was believed to be — and that the other side is gone.

**Do not reconstruct S1–S3 from the task description.** It is a paragraph
summarising eleven commits; what came back would not be that work, and it
would be indistinguishable from it in the log. If the branch exists on a
machine that can still push it, push it — everything in this report was
built on `main` and none of it conflicts with the genome or energy work.

---

## 1. `nest_visits` counts loitering

`CreatureStats::nest_visits` increments on any move made while
nest-adjacent, guarded on `OrganismState::since_nest > 0`. But `since_nest`
is incremented **unconditionally every tick** (`creature.rs`, the pheromone
block), so the guard is false exactly once in a creature's life. Every
nest-adjacent move scores.

The control makes it unarguable — one ant, a nest, and no food anywhere, so
foraging is impossible by construction:

```
moves 678 | pickups 0 | deliveries 0
nest_visits 340  (0.501 of moves)
```

A trip counter reads ~0 on that scene. `nest_visits` reads 340.

The consequence that matters: `examples/ascii.rs`'s
`assert!(st.nest_visits > 0, "no ant ever reached the nest")` **is not a
sessility guard.** A colony that never leaves the nest mouth passes it
trivially, and the counter goes *up* as the colony gets more sessile, not
down.

### Why repairing `since_nest` cannot work

Two independent reasons, either one fatal:

- It accumulates while the ant is standing **on** the nest, so "time since
  away" and "time since arriving" are the same number.
- It counts **ticks**, and `tick_interval` is 6, so its scale is a species
  constant rather than a distance. Two species with different tick rates
  would report different "ranges" for identical walks.

So the replacement is spatial, and does not read `since_nest` at all.

---

## 2. The instrument

`OrganismState::forage_anchor` records where a creature last touched nest
material; `forage_max` is the furthest it has been from that point since, in
Chebyshev cells. **Measurement only** — nothing in `decide` or the brain
reads either, and an ant still has no idea where home is, which is
load-bearing for the pheromone homing model.

Re-anchoring at *every* contact is what makes it immune to the failure
above: an ant strolling the length of a 32-cell nest patch touches nest at
every step, so the anchor follows it and the depth stays at 1. Loitering
cannot manufacture an excursion.

At each nest contact the completed excursion is booked into:

| Stat | What it is |
|---|---|
| `forage_reach: [u64; 8]` | **The instrument.** Cumulative count of excursions reaching ≥ 1, 2, 4, 8, 16, 32, 64, 128 cells. No threshold anywhere in it. |
| `forage_trips` | Headline count, excursions ≥ `FORAGE_TRIP_MIN`. |
| `forage_depth_sum` | Summed depth of those, so `/trips` is a mean. |
| `forage_depth_max` | Deepest single excursion, booked outside the bar — the run where `forage_trips` is 0 needs it most. |

**The profile is the instrument and the bar is a convenience.** A single
count needs a threshold, and a threshold set from an aspiration is how this
project gets numbers that cannot fail for the right reason. The profile
needs none: an immobile colony is a spike in bucket 0 that vanishes by
bucket 2, and a ranging one carries weight out to the distance of whatever
it is ranging to. It also separates two colonies a mean cannot — a hundred
short hops and ten real trips average the same as a hundred medium ones.

### Setting `FORAGE_TRIP_MIN` from the control

The sessile control's profile over 6,000 frames:

```
>=1: 340   >=2: 4   >=4: 3   >=8: 3   >=16: 1   >=32: 1   >=64: 0
```

The loitering spike is at depth **exactly 1** and nowhere else — 340
collapses to 4 at the first doubling. So a bar of 1 is useless: it
reproduces `nest_visits` exactly (340 against 340, same run), which is the
counter it exists to replace.

**8**, then: three doublings above the noise floor and equal to the diameter
of `CROWDING_RADIUS`'s neighbourhood, so a jammed knot of ants shuffling at
the nest mouth provably cannot make one. **16 was tried and rejected** —
it takes the 55-ant foraging arm to 0 trips against a real deepest excursion
of 10 cells, so a colony improving from 10-cell to 14-cell ranging would
read 0 → 0 and the headline would hide the progress it exists to report.

### The probe is paired, deliberately

`examples/forage_probe.rs` runs the sessile control **and** a real foraging
arm, and neither is worth anything alone. A metric that is only ever
trustworthy when it reads 0 is a vacuous probe waiting to happen, and this
project has already paid four times for harnesses that did not contain the
situation they claimed to measure.

---

## 3. What it says about the colony as built

| Scene | `nest_visits` | `forage_trips` | deepest | profile |
|---|---|---|---|---|
| control: 1 ant, nest, no food | 340 | 3 | 42 | `[340, 4, 3, 3, 1, 1, 0, 0]` |
| 55 ants, food 87 cells away | 5,834 | 5 | **10** | `[5834, 49, 12, 5, 0, 0, 0, 0]` |
| `ascii` foraging loop, 60 ants, 12,000 frames | 4,758 | 107 | **18** | `[4764, 563, 236, 107, 7, 0, 0, 0]` |
| `ascii` double bridge | 13,179 | 21 | **15** | `[13179, 193, 78, 21, 0, 0, 0, 0]` |

Three things fall straight out.

**The colony works an 18-cell bubble.** In a 512-wide world, the deepest
excursion any of 60 ants made over 12,000 frames was 18 cells, and *zero*
excursions in any scene reached 32. `wiki/ants.md` already said "ants do not
range far from home"; nothing had ever put a number on it, and the number is
much smaller than the prose suggests.

**The old counter is off by two to three orders of magnitude, and in the
flattering direction.** 13,179 against 21 on the double bridge is 628:1.

**A crowd is worse than a single ant.** The 55-ant arm reaches 10 cells
where one ant with nothing to forage for reached 42 — 0.091 trips per ant
against 3.0. It also blocks 13,201 of 21,773 moves. They jam each other, and
every verb counter stays healthy while it happens: `moves`, `pickups`,
`drops` and `nest_visits` all climb. This is precisely the failure mode
nothing could see.

### The guards now on the gate

`ascii`'s foraging scene asserts `forage_trips >= 20` (measured 107) and
`forage_depth_max >= 8` (measured 18); the double bridge asserts
`forage_trips >= 5` (measured 21). Bars at roughly a fifth of measurement,
per house convention, because outcome spread here is large. The
`nest_visits > 0` assertions stay, demoted in wording to what they actually
prove.

---

## 4. Litter lands in the canopy, not on the floor

Measured in the same session because S4 and S7 both depend on it.
`Reports/plant-implementation-plan.md`'s WP-B2 landed litter and recorded
*"Not run: the edible-cells-near-surface count. It is a creature-side
quantity and the creature branch sets its own bar when it consumes this."*
`examples/litter_probe.rs` is that harness.

`plant.rs::shed_to_litter` writes litter **in place** at the leaf's own
position and lets the powder fall, so a leaf shed mid-crown lands on the
first branch under it. Standard stand (8 trees, `common::PlantScene`), at
noon:

| frame | canopy top | litter | on terrain | on a branch | ≤3 rows of surface | rotted |
|---|---|---|---|---|---|---|
| 3,600 | 139 | 190 | 36 | 152 | 50 | 0 |
| 7,200 | 94 | 1,959 | 258 | 1,696 | 281 | 41 |
| 10,800 | 82 | 4,330 | 503 | **3,825** | 410 | 263 |

**88% of standing litter is resting on a branch.** Only 9.5% is within three
rows of the terrain surface. The classification walks *down* through
contiguous litter to whatever is actually holding the pile up — a one-cell
test answers "is this a pile", not "what is this pile standing on", and a
drift on a branch and the same drift on the floor are indistinguishable
until the walk bottoms out.

`litter_probe out=x.png` renders that same classification: **magenta for
litter on a branch, cyan for litter on the ground**, over a world at quarter
brightness. A full replace on fixed colours, not a tint — litter's palette is
browns and so is wood's, deliberately (`litter.ron` keeps shed leaves close
in value so a layer reads as ground texture), and WP-B2 already flagged the
two may be too close to tell apart.

**Decay is not keeping up.** 263 cells rotted against 4,593 ever shed. Litter
has no per-material rate and inherits `decay.rs`'s globals — `DECAY_CHANCE_DRY
= 0.002` per `DECAY_TICK_INTERVAL = 200` frames, a ~100,000-frame lifetime
against runs of ~10,000. The standing count climbs monotonically across every
sample; it integrates the canopy's shedding rather than reaching equilibrium.
(WP-B2's "converging" refers to *pending decay sites*, which is a different
quantity and is not in dispute.)

### The question that is with the owner, unanswered

The creature plan reads canopy litter as a defect: floor litter is what a
foraging ant can reach, and litter on a branch costs sweep time and feeds
nobody. **That may be exactly backwards.** The owner has said he eventually
wants ants climbing trees; S4's premise that they cannot reach the canopy is
already false (measured: 7/9/0/11 cells up a tree against 0/0/0/0 on bare
ground); and a crown full of food is the motivation a climb needs, which the
plan itself identifies as the missing half ("the barrier is motivation, not
ability").

So *where litter lands* is a design decision, not a bug to fix quietly, and
it is on the review queue as card `20260823T030418481Z-3c42b3`. Nothing has
been changed pending the answer. The same applies to the rot rate: it is a
**design knob**, because S7's two-larders-and-a-barrier only pays if the
surface does not already feed a colony.

---

## 5. What is not done

- **The merge.** Blocked, §0.
- **Where litter lands, and how fast it rots.** Blocked on the owner's call
  above, deliberately — both readings are defensible and building the wrong
  one costs a detour this project has taken before.
- **Every S4 number in the creature reports is stale** and was already stale
  before this session: they were measured against a litter implementation
  that no longer exists anywhere. Re-measure before quoting any of them.
- **Bug H** (`ascii`'s moisture-gradient scene asserts a gradient the scene
  no longer has) and **bug A** (`root_and_shoot_branching_read_different_slots`)
  are both open and inherited from `main`; neither was touched here. The
  suite is 827 passing, 1 failing, and the 1 is bug A.
