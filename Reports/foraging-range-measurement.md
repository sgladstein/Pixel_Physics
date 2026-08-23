# Foraging range: the counter that was measuring loitering

**Status: MEASURED, instrument landed, 2026-08-23.** The metric described
here is built and on the `ascii` gate. The two findings it produced —
a colony that works a 19-cell bubble, and litter that lands in the canopy
— are measurements, not fixes. Nothing behavioural changed.

> **Landed into `Reports/` on 2026-08-23**, having been written on
> `claude/creatures-m18-merge-ijdlnp` and left there when that branch merged.
> §0 and §5 carry dated corrections; **everything else is as written and as
> measured, and the numbers in §§1–4 still stand.** The two corrected
> sections are the ones that describe the *state of the world around* this
> work — a missing branch and a pending owner's call — and both moved.

Written because the creature plan's next three stages all turn on
quantities nothing in the engine could report.

---

## 0. First: the branch this session was asked to merge does not exist

> **CORRECTION, 2026-08-23 (on landing this report).** The branch was not
> gone — it was on a machine that could still push it, and it did. It
> arrived as `origin/claude/creatures-m18-merge-ijdlnp` and merged to `main`
> as **`7b97c13` "Merge creatures-m18: S1–S4, and a forest floor that
> rots"**, carrying exactly the eleven commits this section could not find:
> `GENOME_LEN = 584`, `BRAIN_OUTPUTS = 10`, `food_energy` / `food_class` /
> `worth_in_aux`, corpse worth in `Cell::aux`, the `EnergyLedger` rework.
> The closing advice below — *do not reconstruct S1–S3 from the task
> description; if the branch exists on a machine that can still push it,
> push it* — is the advice that turned out right, and it is why the search
> record stays here in full rather than being deleted.
>
> **What the search proves is still true, and is the reason to keep it:**
> every check in it was sound at the time it ran. A sweep of all 29
> remote `brain.rs` files for `GENOME_LEN` is a correct instrument that
> returned a correct answer about the refs that existed *then*. The lesson
> is not that the search was wrong; it is that **"not on any remote" is a
> statement about a moment, not about the work** — an unpushed branch is
> invisible to every instrument git has, and the only thing that resolves
> it is a person with the other checkout. Budget for that before
> reconstructing anything.

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
of 12 cells, so a colony improving from 12-cell to 15-cell ranging would
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
| 55 ants, food 87 cells away | 5,793 | 3 | **12** | `[5793, 43, 12, 3, 0, 0, 0, 0]` |
| `ascii` foraging loop, 60 ants, 12,000 frames | 6,014 | 143 | **19** | `[6020, 787, 313, 143, 8, 0, 0, 0]` |
| `ascii` double bridge | 13,502 | 16 | **14** | `[13502, 193, 61, 16, 0, 0, 0, 0]` |

Three things fall straight out.

**The colony works a 19-cell bubble.** In a 512-wide world, the deepest
excursion any of 60 ants made over 12,000 frames was 19 cells, and *zero*
excursions in any scene reached 32. `wiki/ants.md` already said "ants do not
range far from home"; nothing had ever put a number on it, and the number is
much smaller than the prose suggests.

**The old counter is off by two to three orders of magnitude, and in the
flattering direction.** 13,502 against 16 on the double bridge is 844:1.

**A crowd is worse than a single ant.** The 55-ant arm reaches 12 cells
where one ant with nothing to forage for reached 42 — 0.055 trips per ant
against 3.0. It also blocks 13,136 of 21,847 moves. They jam each other, and
every verb counter stays healthy while it happens: `moves`, `pickups`,
`drops` and `nest_visits` all climb. This is precisely the failure mode
nothing could see.

### The guards now on the gate

`ascii`'s foraging scene asserts `forage_trips >= 20` (measured 143) and
`forage_depth_max >= 8` (measured 19); the double bridge asserts
`forage_trips >= 5` (measured 16). Bars at a seventh and a third of measurement,
per house convention, because outcome spread here is large. The
`nest_visits > 0` assertions stay, demoted in wording to what they actually
prove.

### The instrument was behaviour-neutral only on the second try

Worth recording, because every gate this project has said green and the
paired run is the only thing that caught it.

The first version of the trip counter **dropped `state.since_nest = 0`**
from the nest-contact block while adding the booking code around it — the
one line the homing gradient depends on, since the channel-A deposit is
scaled by `1 - since_nest / nest_memory`. With it gone, every ant's homing
trail decays to nothing.

What did **not** catch it: `cargo test` (827 passed), `cargo clippy` (clean),
`ascii`'s own foraging scene (still delivered food, still passed every
assertion including the new ones), and reading the diff, which showed only
lines added. The colony degrades rather than breaking, so nothing has a
threshold it crosses.

What caught it: running `ascii` on `origin/main` **in the same session on the
same machine** and comparing the counters, which is meant to be a frame-cost
check. `main` measured 13,980 moves / 222 deliveries / 72 organisms against
11,997 / 217 / 68. A measurement-only change must reproduce the baseline
*exactly*; any divergence at all is the finding.

Restored, the scenes now match `main` digit for digit — 13,980 moves, 6,014
nest visits, 222 deliveries, 74,649 moves on the double bridge — which is
the proof that the counters are inert. This is `CLAUDE.md`'s "re-read the
function, not the diff" arriving in a new costume: not a stash this time, a
regex replacement whose anchor swallowed a line.

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

### The owner's call, and what was built on it

Review card `20260823T030418481Z-3c42b3`, answered 2026-08-23:

> *"Ideally some would land in the crown but the vast majority would land on
> the ground. It is probably simpler to just make it all go to the ground
> though and that is fine. I don't want to overcomplicate it."*

So: all of it to the ground, no retention fraction. Two changes, both
measured as a paired comparison on the same stand at frame 10,800.

**1. `shed_to_litter` drops the leaf through its own crown.** It passes
through anything organism-owned and through air, and lands on the lowest
*air* cell it reaches before hitting something that is neither — terrain,
standing litter or water. Landing on the lowest **air** cell rather than the
lowest cell reached is what stops it overwriting the branch it fell past;
litter must never delete plant tissue. `LITTER_FALL_REACH` is 64, a cap on
work and never a gate on whether the leaf is shed.

**2. `litter.ron` declares its own decay rates.** New optional
`decay_chance_damp` / `decay_chance_dry` on `MaterialDef`, falling back to
`decay.rs`'s globals so nothing that does not ask changes. Litter takes
0.5 / 0.1 against ash's 0.05 / 0.002.

| at frame 10,800 | main | + landing | + landing & rates |
|---|---|---|---|
| standing litter | 4,330 | 4,054 | **1,018** |
| resting on terrain | 503 (12%) | 1,432 (35%) | 513 (50%) |
| resting on a branch | 3,825 (88%) | 2,617 (65%) | 503 (49%) |
| within 1 row of terrain | 22% | 23% | **61%** |
| within 4 rows | 28% | 38% | **80%** |
| within 8 rows | 32% | 51% | **86%** |
| within 32 rows | 57% | 89% | 99% |
| rotted, of ~4,600 ever shed | 263 | 609 | **3,659** |

Three things in that table are worth reading twice.

**The landing change alone did not finish the job**, and the reason is not a
placement failure. It took *height* to see it: the on-terrain/on-plant split
mis-sorts a deep drift piled against a trunk, which bottoms out on the root
collar and reads as "on a branch" while being unambiguously forest floor.
`litter.ron` asks for exactly those drifts (`friction_angle: 42.0` — "a drift
piles up against a trunk rather than running out to a level sheet"), so they
are the design working. The height profile is in `litter_probe` now for that
reason, and it is what showed the residual was **accumulation, not
misplacement** — the floor was growing because nothing was rotting.

**Damp decay events rose 4.3x from the landing change alone** (114 → 490),
before any rate changed. `Reports/dead-ends.md`'s note on this predicted it
and it is now confirmed independently: the moisture field is sampled at the
litter's own block, so litter in a canopy samples *air* and reads dry. Moving
litter to the ground moves it into damp ground's field. A placement fix
turned out to be a decay fix too.

**The floor is an equilibrium now rather than an accumulator.** 3,659 of
~4,600 shed cells have rotted, against 263 before.

### The part that is not finished, and is with the owner

**The layer works and you cannot see it.** Rendered at 4x on the real
`grove` scene, the floor is essentially invisible: litter's palette is
browns 88–126 and soil's is the same range, so the mat reads as more soil.
WP-B2 already flagged this as an open cosmetic question — *"litter's palette
may be too close to soil's to read"* — and the fix above makes it matter,
because before this there was barely any floor to look at. This is on the
review queue.

### The question the landing change did not settle



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

> **CORRECTION, 2026-08-23 (on landing this report).** Three of the four
> bullets below were resolved between the writing and the landing. They are
> kept with their resolutions attached rather than deleted, because what
> each one was blocked *on* is the useful part.

- ~~**The merge.** Blocked, §0.~~ **Done.** The branch was pushed and merged
  as `7b97c13`; see §0's correction.
- ~~**Where litter lands, and how fast it rots.** Blocked on the owner's call
  above, deliberately — both readings are defensible and building the wrong
  one costs a detour this project has taken before.~~ **Answered and built**
  — and the deliberate block is why it took one build rather than two. The
  owner's call is quoted in §4 above ("all of it to the ground, no retention
  fraction"); `shed_to_litter`'s drop-through and `litter.ron`'s own
  `decay_chance_damp` / `decay_chance_dry` landed via `bab6372` and
  `da252dc`, with `5a9e594` finishing the floor's appearance. §4's paired
  table is the measurement: standing litter 4,330 → 1,018, within 4 rows of
  terrain 28% → 80%, rotted 263 → 3,659.
- **Every S4 number in the creature reports is stale** and was already stale
  before this session: they were measured against a litter implementation
  that no longer exists anywhere. Re-measure before quoting any of them.
  **Still true, and now the standing instruction** — the merge did not
  re-measure them, it changed the world they were measured in again.
  `Reports/creature-evolution-plan.md` §4's guard table is the one to
  re-baseline first.
- **Bug H** (`ascii`'s moisture-gradient scene asserts a gradient the scene
  no longer has) and **bug A** (`root_and_shoot_branching_read_different_slots`)
  are both open and inherited from `main`; neither was touched here. ~~The
  suite is 827 passing, 1 failing, and the 1 is bug A.~~ **Both are still
  open**; the suite count has moved with the merge and is not worth quoting
  from here. What changed is CI: `.github/workflows/ci.yml` now quarantines
  both explicitly rather than carrying a red trunk — bug A is `--skip`ped in
  `test` / `test-debug` and runs in a `continue-on-error` `known-red-roots`
  job, and the `ascii` job is non-gating over bug H. **The quarantine keeps
  the gates honest; it does not close either bug.**
