# Round 36 — the forest being eaten, and two lanes that retracted themselves

*Coordinator `session_01Evmt6MKMGFbk4zA5rC7B3y`, 2026-09-14/15. Brief:
[`evolution-lab-round-36-brief-2026-09-14.md`](evolution-lab-round-36-brief-2026-09-14.md).
Landed **#431, #432, #433, #436, #440, #441, #444, #445, #446, #447**.
Six lanes, all `claude/opus-5` except the nest research on `claude-fable-5-1`.*

## The shape of the round

**It was briefed as four lanes on readability and cost, and a live regression
took the lead two hours in.** The owner was losing plants in his lab and had
isolated the condition himself — *a bed with no creatures keeps its plants, the
same bed with creatures loses them*. That became Lane E, and it is the round's
headline.

**The round's most valuable outputs were two retractions.** Both lanes
withdrew their own headline number after re-measuring, and in both cases the
corrected finding is better than the one it replaced. That is recorded here at
the top rather than in a footnote, because the withdrawn numbers were reported
to the owner before they were checked.

## Lane E — creatures do not attack plants (#440), the lead

**The loop.** An ant grazes a plant cell; because the victim is an organism the
eat path raises an **alarm**; alarm is the **only** wired route to attacking
(all nine armed species author exactly `(Alarm, Attack, 2.0)`); `nearest_foe`
targeted *any living non-kin organism*, so it found **the plant being grazed**;
and that bite has no diet gate. `wood`/`rootwood`/`grassroot` carry no
`food_energy`, so every cell the jaw took was **pure loss**.

**Both owner rulings shipped.** Creatures do not attack plants, no exception —
`nearest_foe` now skips any cell that is not `MaterialKind::Creature`, through
the same predicate its own odds count one line below already used, and **the
reopening condition (a plant that can damage an animal) is in `dead-ends.md`
`creatures:082` with the entry**, per the ruling. Eating a plant raises no
alarm — Lane D's `is_animal_cell` shape, taken verbatim.

**Sized, which is the half nobody had.** `latecensus
scenario=played_bed_longant`, 40,000 frames, `RAYON_NUM_THREADS=1`, one binary
with `PIXEL_PHYSICS_PLANT_FOE` the only difference:

| seed | swings | at plants | jaw cells | mouth cells | jaw share |
|---|---|---|---|---|---|
| 1 | 6,922 | **6,922** | 3,839 | 659 | 85% |
| 2 | 8,312 | **8,312** | 5,226 | 412 | 93% |
| 3 | 76 | **76** | 36 | 11 | 77% |

**100% of swings and 100% of cells plant-directed on every seed.** Paired
`no_colony=1` control exact: every column 0. After the fix, `attacks` and
`attack_plant_cells` are **0**.

### RETRACTION — "plants standing up on every seed" is withdrawn

The lane's own, and it matters because the withdrawn version was reported to
the owner. `main` landed 21 commits under the branch including `Cell::
organism_id` widening `u16 → u32`; re-measuring the baseline in the same
session **overturned the claim**:

| seed | loop live | fixed | no-colony control | ants live → fixed |
|---|---|---|---|---|
| 1 | 243 | **275** | 270 | 198 → 137 |
| 2 | 161 | **97** | 268 | 127 → 224 |
| 3 | 190 | 187 | 200 | 2 → 8 |

**Up on one, down hard on one, flat on one — median −3.** The withdrawn reading
(173→206, 134→191, 60→158) was a sample from a wide distribution on a tree that
no longer exists, and three seeds is not a sweep either way.

**The two columns together are the better finding.** Where the colony does not
grow, the stand recovers to the unhunted control (seed 1: **275 against a
no-ant 270**). Where it explodes, grazing replaces the jaw (seed 2: ants nearly
double, stand falls to 97). **The fix removes the pure loss completely; what
happens to the forest next is decided by what the colony does with the energy
it is no longer wasting** — which puts the birth bar and the starvation balance
in front of Lane D, **with seed 2 as the case to tune against**.

**A standing plant census is a poor instrument here, measured rather than
assumed**: the paired standing count moved **−84, +3,463 and +3,484**
pre-merge, one arm reading *more* plant with the colony on it. Hence
`eaten_plant_cells` — both routes counted in cells at the line where the cell
leaves the world.

**§Z22 measured the druid garden and found the same path a tenth of the gap
there. Both are true; they are different beds. "Worse than yesterday" remains
UNCONFIRMED** and is not written off.

## Lane B — what an ant's swept cells are made of (#433, #445)

**Three of the ~29 cells are the ant. The rest is one rule, and it is not about
animals**: a dirty mark is expanded sideways by the largest `sweep_reach` of any
cell in its whole 4,096-cell chunk — **24, set by water, in chunks holding an
average of 1.4 water cells**, against soil's own reach of 2. Two of the brief's
four candidates were **struck off with evidence** (`awake` is flat at 0.01
chunks/ant; `deposit_pheromone` never marks a chunk's dirty channel).

### RETRACTION — "29.4 cells per ant" is a per-bed number, not an ant's

Today's rule varies **1.75x across three seeds of one bed** while the ant's own
floor varies 6%, and the same seed reads **21.7 at `grow=6000` against 13.3 at
`grow=2000`**. **It should stop being quoted as an ant's cost** — including in
the round-36 brief, which led with it.

### And the fix is a shape prize, not a reach prize

The single-rect version buys **1.9%** (19,261 → 18,904 cells/frame at 440
ants), because at reach 24 on a 64-wide chunk **the rect is already clipped to
full width** — so no reach narrowing can pay while the shape stays one rect.
The 4.8–5.7x needs the narrower shape, and **§E2 stands in front of it**.

**§E2 bisected from a frame to a cell**, which is the lane's real deliverable:
the switch was a process-wide `OnceLock`, so two settings meant two processes
and two chaotic worlds. Made per-chunk, `examples/sweepgap.rs` steps both arms
in lockstep — **first divergence at frame 237, four cells, all `soil`, same
material and organism id, different `aux`. §E2 is in the soil-moisture
channel.** Frame 237 instead of 4,330. **Located, not diagnosed**, and the
report says so.

## Lane A — the food economy, made readable (#444)

**Reproducing the owner's reading before redesigning paid immediately**, as the
brief asked. Rendering the road and the harvest map apart showed **the road
already worked** and the amber hatch was burying it in an adjacent hue; and the
*"huge box"* is literal — every live harvest tile sits in the surface band, so
an 8-cell tile is about one row of ground and **seven rows of sky**, washed to
the tile grid's edge. **Card `…193144118Z-d8e147` came back rating 4** against
*"the amber hatch it bad"* for the same view.

**The FOOD page was rejected a third time and the owner wrote the
replacement** (card `…205807355Z-12829d`), opening *"We really should have
discussed this more."* Both offered arms were inside a design he did not want.
His spec — kill the graphs, a per-colony stats table, click a colony for a
detail page by food kind, a third layer (click `ant` → which colony, click
`flower` → which plants), and a **time range control (all time / 10 / 5 / 1
min)** — is built in #444. **The road half-life is a minute**, his pick on a
blind card he disambiguated by naming the property.

## Lane C — the pheromones (#432)

**A trail is a live map, not a memory, and `DECAY_RHO` is inert.** `DIFFUSE` is
the real lever. The lane also **corrected its own number** after the routing
poke: *every alarm in a one-colony bed is the grazing path* — which is the same
defect Lane E was root-causing, found independently from the pheromone side.
`ancestor.ron` could not hear an alarm; `flitter` neither lays nor reads a
trail; **the alarm plane's audible radius is two cells**, so a display deposit
is inaudible to anyone but the displayer. **No default moved.**

## Lane D — the rivalry economy (#436)

Closed §Z23's alarm half first and **wrote the routing collision up itself**
(§5a) when the coordinator's reassignment poke crossed its landing by eleven
minutes. It then **struck its own claim** that every remaining swing was
animal-directed — `xcol` and `killedA` are both *kill* counters and say nothing
about swings that take no cell. That correction is what sent Lane E to build
the victim-kind counter.

## The nest research (#446) — Fable 5.1, research only

**Handed over by the druid coordinator; the owner asked to be argued with
rather than implemented.** It was, on the half that mattered:

- **A site, not a material — his instinct confirmed.** `NestSite` already holds
  position and odour; the gap is one function.
- **Not a blob — refused with a measurement.** A footprint wider than the ant
  band **kills the colony monotonically on 3 of 3 seeds** (alive at 40k: 59 →
  38 → 19 → 2 over widths 36/72/144/288), and the case a blob defends does not
  arise: the patch loses **0/0/4 cells over 120k frames**.
- **"Nests never functioned" confirmed, and it picks the owner's own reading
  2** — channel A stands at 3,421 at frame 10,000 and **0–9 from 20,000 on**;
  cutting the homing circuit out of the genome leaves deliveries inside the
  noise; laden ants are 0–10% at the food.
- **Real ants do not home on the queen**: path integration for long range, nest
  odour at short range — which is what `AtNest` already is.

**It corrected two claims in the coordinator's own brief**: `NestSite` *is*
read for location (attribution, not navigation), and **the 414-delivery scene
places 15 of 55 ants with no channel A by frame 6,000, so it cannot carry a
homing or footprint measurement at all.** Do not quote 414 as a constraint
again. It also half-overturned the coordinator's sequencing call: the trail
gates homing **only because homing was built on the trail**; nest identity does
not depend on it, so **both can start now**.

## Coordinator's own work, and its own error

The zoom ladder (#431): the owner ruled *"get rid of stop 3"*, so
`render::ZOOM_OUT_RUNGS` is `1, 2, 4` in all three games, with stride 3
surviving as a **cap** rather than a stop. It was scoped as "a one-line ladder
edit" and is not — **the ladder is walked in two directions**, and both halves
were watched going red.

**The routing error worth recording**: the reassignment poke handing
`creature.rs` to Lane E fired **eleven minutes after** Lane D had already
committed and opened #436 on the same fix. *A poke crossed a landing.* There is
no delivery signal for a poke; **the branch head is the only check**, and it
was not taken before writing the poke. Lane D caught it, not the coordinator.

## Model-choice tally

| model | lane | verdict |
|---|---|---|
| Opus 5 | A, B, C, D, E | all five delivered; **two retracted their own headline** unprompted, which is the behaviour wanted |
| Fable 5.1 | nest research | delivered, **argued against the owner with a measurement**, and **found two errors in its own brief**. No unverified claim found. |

Round 30's single Fable data point was a lane that asserted something one
command disproved. This one was told to verify every inherited claim with a
command and did, catching two. **One data point each way; keep tallying.**
