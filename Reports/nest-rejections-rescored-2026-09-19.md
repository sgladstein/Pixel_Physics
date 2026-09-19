# The nest line's rejections, re-scored — three nulls that hold, one lever nobody swept

*2026-09-19. Lane N of the ant-survey follow-up round
(`Reports/ant-survey-followup-brief-2026-09-19.md`, on the coordinator's
branch `claude/ant-sim-research-review-eyuol2` as PR #475 at the time of
writing),
written against the owner's instruction to distrust the past results:
**"Past agents may have made mistakes and these are all very complex systems
so it may have rejected it based on a bad test or a failure coming from an
interconnected system."** Every number here is from a run in this tree, twelve
paired seeds, `RAYON_NUM_THREADS=1`, examples rebuilt in the same command as
the run. §9 has the commands.*

Companion to [`nest-digging-plan-2026-09-19.md`](nest-digging-plan-2026-09-19.md)
(the plan of record), [`nest-shape-three-negatives-2026-09-19.md`](nest-shape-three-negatives-2026-09-19.md)
(the four levers this re-scores) and [`nest-biology-2026-09-19.md`](nest-biology-2026-09-19.md)
§10 (the owner's ruling that the nest has no purpose yet, which bounds what
"better" means below).

---

## 0. The finding

**Two of the three faults the round suspected were real, and fixing them does
not change a single verdict. The fourth candidate nobody had swept is the one
that works.**

The nest line's rejections were scored on `roofed` — which undercounts the nest
about threefold, because a gallery with an ant standing in it is not empty —
and at **one run per arm** in a box that had no seed. Both are now fixed:
`room total` since PR #473, and `digbox seed=N` here. Re-scored on the right
number with twelve paired seeds, the three `Crowding` interventions are still
coin flips (5, 6 and 5 of 12), and the `LightHere` spoil-drop rejection is
still a rejection — **0 of 12 seeds better**, with the sensor its own entry
blamed replaced by a working one.

What the re-score *did* turn up is that the shape table those rejections were
argued from cannot separate its own arms. Over twelve seeds of the **unchanged
control**, the room's bounding box runs **96 to 191 cells wide**; the shape
report compares 129w / 208w / 113w across three arms at one run each. Three
of those four numbers sit inside the control's own spread.

And the lever that moves it was in the tree the whole time, default-off, swept
in the wrong axis. `PIXEL_PHYSICS_NEST_SITE_ROWS` — how far *down* an ant still
counts as being at home — was never scored on `room total`; only its sibling
`_NEST_SITE_COLS` was, and that one is the negative the shape report published.
At a reach of 40 rows:

| | control | site reach 40 rows | paired over 12 seeds |
|---|---|---|---|
| room total | 699 | 952 | **1.44x, bigger on 12 of 12** (p ≈ 0.0005) |
| room depth | 15 rows | 30 rows | |
| height over width | 0.11 | **0.20** | **1.81x, taller on 11 of 12** (p ≈ 0.006) |
| middle-half width | 31 cols | 29 cols | 0.90x, narrower on 9 of 12 |
| digs | 2,821 | 4,811 | |

**It is deeper, bigger and slightly narrower at once**, which is the first time
anything in this line has moved the aspect ratio at all, and the picture shows
shafts descending from the chamber where the control has a flat scrape. Card
`20260919T151821144Z-da82d9` is with the owner, blind.

**This refutes the plan's §1 structural claim in its literal form.** That claim
is *"interventions on **whether** cannot produce a shape"*, and it is what
retired the dig-target bias before it was built. `NEST_SITE_ROWS` is purely an
intervention on whether — it changes where the chamber gate fires and nothing
about where a dig lands. It produces a shape anyway, because **the region it
fires over has one**. The corrected claim is narrower and still useful: a
*scalar* intervention on whether cannot produce a shape; one applied over a
region inherits the region's.

---

## 1. What the instrument was missing, and the control that proves it mattered

**`digbox` had no seed.** The box is a fixed scene with no noise in it, which
made every run bit-identical and every arm a single sample. `seed=N` now
shuffles the order the nest patch's columns are founded in — `burrow_probe`'s
own device, and the only thing a seed can vary in a scene with no randomness
in its geometry. `seed=0` is the shipped walk and reproduces every published
figure to the cell (`digs=2836 roofed=81 room_total=641 room=129w x14h`).

Twelve seeds of the control, nothing else changed:

| | median | range |
|---|---|---|
| room total | 699 | **415 – 806** |
| roofed | 73 | **52 – 108** |
| bounding box, width | 153 | **96 – 191** |
| digs | 2,821 | 1,842 – 3,339 |

**`roofed` alone spans 2x and the bounding box spans 2x on one arm**, which is
the whole of the shape report's evidence for three of its four negatives.

**And the bounding box is a max statistic**, so one stray dug cell at the edge
of the box sets it and nothing about the other thousand moves it. `census` now
also returns `iqr` — the number of columns holding the **middle half** of the
room's cells — and `p50x`, where that middle half sits relative to the nest
patch. The selftest grows a fourth geometry to prove `iqr` is not a third blind
column, red both ways:

```
thirty cells in one chamber against thirty in ten scattered pits:
room total 30 vs 30, bbox 10w vs 172w, iqr 6 vs 96
```

The counts cannot tell them apart; the bounding box calls **scattering the
bigger nest**; `iqr` separates them 16x. `p50x` carries its own specificity
check in the results below — it reads 3 to 5 columns off centre in every arm
of a 400-wide box, so nothing here is drifting sideways and a change in `iqr`
is a change in spread rather than in position.

---

## 2. The three `Crowding` interventions — the nulls hold, and now they mean something

`nest-digging-plan` §0: *"three separate interventions on the `Crowding` →
`Dig` gate — making the reading local, re-centring the gain, sweeping
`ROOM_TARGET` — each achieved its own stated precondition and moved the nest
**not at all**"*, scored on `roofed`, one run per arm. Re-scored:

| arm | room total | iqr | vert | room vs control | iqr vs control |
|---|---|---|---|---|---|
| control | 699 | 31 | 0.11 | — | — |
| `CROWDING_LOCAL=near` | 628 | 32 | 0.11 | 0.90x, up **5**/12 | 0.98x |
| `CROWDING_LOCAL=wide` | 704 | 32 | 0.11 | 1.01x, up **6**/12 | 1.00x |
| `gate=25.5` (re-centred) | 720 | 34 | 0.10 | 0.98x, up **5**/12 | 1.04x |
| `LAB_ROOM_TARGET=0.5` | 756 | 34 | 0.10 | 1.05x, up **7**/12 | 0.97x |
| `LAB_ROOM_TARGET=8.0` | 748 | 34 | 0.11 | 1.03x, up **6**/12 | 1.02x |

Five arms, five coin flips, and no arm leaves `vert` 0.10–0.11.

**The re-score makes the null stronger rather than weaker, and that is the
point of running it.** The reason is the positive control, which the original
runs printed and nobody read against the result — how packed it felt while at
the nest, in tenths, on one seed:

```
control      [0, 0, 0, 0, 0, 0, 0, 0, 1097, 48530]   top tenth 97.8% of at-nest ticks
near         [17, 109, 1022, 1419, 2365, 5915, 5609, 14792, 10974, 540]   top tenth 1.3%
wide         [13, 92, 369, 3284, 6600, 11415, 8712, 7592, 6197, 175]      top tenth 0.4%
```

The shipped reading is pinned at its ceiling for **97.8%** of at-nest ticks.
The local reading desaturates it completely — 1.3% and 0.4% at the ceiling,
with mass spread across the whole band. So `dead-ends.md`'s condition (0),
*an input that never leaves saturation cannot demonstrate a mechanism about
its low end*, **is now met**, the input varies exactly as the biology asks,
and the nest does not move. That is a much harder null than the one it
replaces, which could always be answered with "the input was saturated".

### 2b. …and giving density an aggregation point does not rescue it either

The plan's Stage 2 is that the `Crowding` work was *"a correct mechanism with
nowhere to act"*, and that a nest site with real depth is the cheapest honest
aggregation point. The 2x2 separates the site from the reading:

| arm | room total | iqr | room h | vert | vert vs control |
|---|---|---|---|---|---|
| control | 699 | 31 | 15 | 0.11 | — |
| site reach 16 rows | 960 | 31 | 20 | 0.14 | 1.32x, up 8/12 |
| site reach 40 rows | 952 | **29** | **30** | **0.20** | **1.81x, up 11/12** |
| reach 40 + `CROWDING_LOCAL=near` | 888 | 30 | 21 | 0.16 | 1.43x, up 11/12 |

**The site depth does it and the local density reading does not — it makes it
worse.** Adding `near` to a 40-row site takes the depth back from 30 rows to
21 and `vert` from 1.81x to 1.43x. Stage 2's premise was that density needed
somewhere to act; given somewhere to act, it still subtracts.

**Stage 2's own check that can fail, applied honestly**: *"widening must be
**local** to the aggregation, not a uniform increase everywhere."* At a 16-row
reach it fails — room 1.49x with `iqr` flat at 1.03x, a uniform rise. At 40
rows it passes weakly — `iqr` 0.90x, narrower on 9 of 12 (p ≈ 0.15, not
significant on its own) while the room is 1.44x bigger, so the room grows and
its middle half does not. The strong reading is vertical rather than
horizontal: the nest at a 40-row site is **twice as deep** and no wider.

**What this is not.** `NEST_SITE_ROWS` sets how deep `AtNest` reads true, so
"deeper gate, deeper nest" is close to a definition and has to be checked. It
is not one: at a 16-row reach the room is **20** rows deep and at a 40-row
reach it is **30**, so the room is neither the dial nor proportional to it.
The cleaner control is already in the table — `rows16` and `rows40` dig the
**same amount** (4,814 against 4,811 digs) and produce 20 rows against 30. The
depth is not bought with digging; it is bought with where the gate reaches.

### 2c. …and while the box was open, Stage 3 — the 2.3x is a `roofed` number and it is 1.31x

Not in the brief, and it cost one command once the harness had a seed. The
plan's Stage 3 rests on *"curvature → `Dig` works: −0.6 gives **2.3x** roofed
chamber; +0.6 collapses it to 36"* — `roofed`, one run per arm, which is the
pair of faults §1 is about. Twelve paired seeds on `room total`:

| arm | room total | iqr | room h | vert | room vs control | iqr vs control |
|---|---|---|---|---|---|---|
| control | 699 | 31 | 15 | 0.11 | — | — |
| `curvdig=-0.6` (dig where buried) | 824 | **40** | 21 | 0.12 | 1.31x, up 10/12 | **1.26x, wider on 10/12** |
| `curvdig=+0.6` (dig at an exposed face) | 329 | 28 | 9 | 0.15 | **0.50x, up 0/12** | 0.87x |

**The sign result survives and the size does not.** The negative wire is real
(10 of 12, p ≈ 0.04) and it is **1.31x rather than 2.3x**; the positive wire is
the strongest single negative in this report, halving the room on 12 of 12.
But the negative wire makes the nest **wider** — `iqr` 1.26x on 10 of 12, the
only arm here that moves it upward at all — so curvature deepens hollows
everywhere rather than picking one, which is the plan's own diagnosis of the
lens arriving through a lever meant to cure it.

**One instrument caution, because this arm is where it bites.** `vert` is a
ratio, so a room that *collapses in width* reads as a taller nest:
`curvdig=+0.6` has the second-highest `vert` in this report (1.40x, up 10 of
12) on half the room and 9 rows of depth. **Read `vert` with `room total`
beside it, always** — it ranks shapes of comparable size and cannot rank a
shape against a ruin.

---

## 3. The downward dig bias, put at the turn instead of at the target

`nest-shape-three-negatives` §1b refuted the downward bias **from the code**:
the dig reads `DIRS[heading]` and `step_chain` then chooses among
`[heading+AHEAD_LEFT, heading, heading+AHEAD_RIGHT]` with the middle candidate
being the cell just dug, so **the dig is what licenses the next step** and an
override severs the coupling.

*Was that a test or an argument?* It was an argument, and it is a correct one
about the thing it describes — but it only rules out the **target**. The brief
names the third place: `home_weighted_pick` biases a *re-roll* rather than a
target and is inert at its default. `PIXEL_PHYSICS_DIG_DOWN=<w>` does the same
for the dig: with probability `w` per dig roll the animal **turns one octant
toward straight down and then digs where it is now facing**. The coupling
survives intact — the ant still digs the cell it will step into — and the
paper's claim is the direction rather than the cell (Buarque de Macedo et al.,
*PNAS* 118 (2021), *"ants tend to dig piecewise linearly downward"*, validated
through PubMed 2026-09-19).

Default unset takes **no draw at all**, and the control is bit-identical to
the pre-change binary (`digs=2872 room_total=709 room=165w x18h`,
`aimed_down=0`).

| arm | room total | iqr | room h | vert | digs | turned down |
|---|---|---|---|---|---|---|
| control | 699 | 31 | 15 | 0.11 | 2,821 | 0 |
| `DIG_DOWN=0.15` | 788 | 31 | 16 | 0.09 | 3,143 | 12,203 |
| `DIG_DOWN=0.5` | 891 | 28 | 16 | 0.10 | 3,715 | 40,204 |
| `DIG_DOWN=1.0` | 1,042 | 32 | 17 | 0.13 | 3,878 | 78,881 |
| reach 40 + `DIG_DOWN=1.0` | **1,182** | 35 | 24 | **0.20** | 4,784 | 198,139 |

**It works, and it is mostly volume rather than shape.** At `w=1.0` the room is
**1.51x, bigger on 12 of 12**, and the digging is only 1.37x — so the digging
got *more effective* rather than merely more frequent, which is the backfill
problem easing. But `vert` is 1.18x on 9 of 12 (p ≈ 0.15), against the site
reach's 1.81x on 11 of 12 from a third of the extra digging.

Stacked on the site reach it is the biggest room measured in this line
(1.71x, 12 of 12) at the same `vert` as the site reach alone, and `iqr` goes
the *wrong* way (1.08x, wider on 8 of 12). **So the two are not complements:
the site reach makes the nest deep and narrow, the downward turn makes it
big.** Which is wanted is the owner's call and not this lane's, under
`nest-biology` §10's ruling that the nest has no purpose to be better *for*
yet.

**The counter is the honest half.** `digs_aimed_down` is not `dig_rolls * w`,
because `turn_toward` returns the heading unchanged when the animal is already
pointed down — at `w=1.0`, 78,881 turns against 88,318 rolls, so about 11% of
rolls were already aimed down and the lever had nothing to add to them.

---

## 4. Khuong's deposition rule — the marker does not exist where the decision is

The survey's headline for construction is Khuong 2016: pellets carry a
building pheromone, deposition is proportional to local pellet density,
pillars follow, then chambers. The brief asked for the direct rule — a
`DropSpoil` preference for cells adjacent to existing `spoil` — built as a
wired instinct and judged by eye.

**It is not built, because its falsifier fires before the build, and the
number it fires on is not the one already on the record.**

`nest-shape-three-negatives` §4 measured *spoil in reach of a **diggable**
cell* at **0.1%** and concluded the marker barely exists. That is the right
denominator for the **dig** side of this stigmergy (the plan's Stage 4) and
the wrong one for the **deposition** side, because it averages over the whole
buried world while a laden ant is standing on the mound, where the pellets
are. `CLAUDE.md`'s *ask what your number counts*. So `creature.rs` now counts
the real denominator at the drop itself — over the eight neighbours that
animal actually had, how many were places a pellet would stay, and how many
of *those* had a pellet already in reach:

```
at the drop: 82 of 1095 places a pellet would stay had a pellet already in
             reach (7.5%); 30 of 2807 drops had both kinds to choose between
...and 2166 of 2807 drops went up the column instead, where there is no
             neighbour to prefer (77%)
```

**77% of drops never choose a neighbour at all** — an ant at the face has
nowhere beside it that would hold a pellet, so the pellet goes up the column
(`creature.rs`'s `lifted` branch, which is the haulage trip abstracted). Of
the 641 drops that do choose, **30 had both a marked and an unmarked candidate**
— so a deposition-follows-pellets rule would have a decision to make on
**roughly one drop in ninety**. A weight on it cannot move a mound.

**And the reason the marker is thin is now named rather than guessed at.** The
mound, censused by material, with the lining rule as the ablation:

| | `packedsoil` | `soil` | `spoil` |
|---|---|---|---|
| shipped | **433** | 121 | 96 |
| `PIXEL_PHYSICS_BURROW_LINING=off` | **0** | 171 | 9 |

`line_burrow` runs on all eight neighbours of every dig and reads
`spoil.packs_into = "packedsoil"`, so **two thirds of the mound is pellets
that were relabelled into gallery lining after they landed**, and the rest
mostly slumped back to `soil` through `needs_footing`. The lining rule is not
a bug — the same ablation collapses the nest from 709 cells of room to 214 —
but it is the eraser, and `spoil` going to zero when it is switched off is the
positive control that says so.

**So the three-negatives report's open question — *"the first question for
Stage 4 is not the response curve, it is how long a pellet has to stay a
pellet"* — has an answer: about 15% of a mound is still a pellet, 67% has been
laundered into lining, and the laundering is a rule this engine needs for
other reasons.** Two things would have to change before Khuong's rule has
anything to read: a pellet would have to stay distinguishable from wall, and
the haulage lift would have to stop taking three drops in four out of the
neighbour decision entirely. Neither is small, and neither is this candidate.

---

## 5. The `LightHere` spoil-drop gate — the rejection stands, on the sensor it asked for

`dead-ends.md`'s entry closes: *"An honest attempt at 'let the ants find the
outside' that failed on the **sensor**, not on the idea… If light ever reads
per cell rather than per field block, or if a dedicated in-the-open input
exists, this is the first thing to try again."*

There is one now. `under_cover` asks the per-cell question every nest census
in this repo asks — is there ground standing above me in my own column — and
`PIXEL_PHYSICS_SPOIL_DROP_COVER=<w>` scales the `DropSpoil` roll by `w` while
it is true. `w=0` is the hard rule the entry describes.

| arm | room total | room h | vert | digs | room vs control |
|---|---|---|---|---|---|
| control | 699 | 15 | 0.11 | 2,821 | — |
| `COVER=0.0` | 358 | 9 | 0.07 | 1,668 | **0.49x, better on 0 of 12** |
| `COVER=0.25` | 474 | 12 | 0.08 | 1,818 | **0.70x, better on 0 of 12** |
| `COVER=0.75` | 671 | 16 | 0.12 | 2,706 | 1.07x, better on 9 of 12 |

Monotone, and **not one seed of twenty-four improves** at the two settings that
actually bite. The mechanism fired hard — 112,406 drop rolls damped on one
seed — so this is a null about the rule and not a disconnected knob.

**The entry's explanation was right and its diagnosis was wrong.** It says *"the
gate mostly just stopped animals letting go"*, and that is exactly what the
digs column shows here (2,821 → 1,668) with a sensor that separates a gallery
from open ground perfectly. The blame went to the light field; the cause is
that **an ant that cannot put its pellet down stops digging**, which is the
same mechanism `PIXEL_PHYSICS_SPOIL_LIFT=none` demonstrated from the other side
on 2026-09-19 (*"more lifting is better"*). The rejection is therefore
**broader** than its entry claims, not narrower, and the entry is updated to
say so.

---

## 6. The dig-face pheromone — is the rejection broader than its evidence?

Not a build; the brief asks for a paragraph, and the answer is **no, and it
does not need to be**.

Bruce (2015), *Behavioural Processes* 122:12-15, is groups of **five**
*Acromyrmex lundi* choosing between a freshly exposed digging face and one
where digging ceased an hour earlier, with no significant difference found.
The authors scope their own null to the face: *"while digging pheromones may
play other roles in other parts of the digging system, they do not play an
important role in regulation of soil excavation at the digging face."* Read as
a general claim about chemical cues in construction it is badly
underpowered — n = 5 groups, one species, one comparison — and the survey is
right that Khuong's marker is on the **pellet** rather than on the face, so
Bruce cannot speak to it.

**But the engine does not need Bruce to decline a construction pheromone, and
this is the part worth writing down.** §4 above prices the mechanism Khuong's
marker would drive: the rule has a decision to make on one drop in ninety,
because 77% of pellets are hauled up the column and two thirds of the mound is
relabelled before anything could read it. A pheromone plane is a far dearer
version of a marker this engine **already has as a material** and still cannot
read. So the order is: make the pellet persist and make the haulage leave a
choice, then ask whether `spoil` adjacency concentrates anything — and only if
the material proxy works and is not enough does a plane become the question.

The one correction to carry: **`dead-ends.md` and the plan both cite Bruce as
ruling out "a digging pheromone", and the citation supports "a digging
pheromone *at the face*, at n = 5"**. Filed as a scope note on the entry
rather than as a reversal, because nothing currently proposed depends on the
difference.

---

## 7. What this does to the plan

| plan item | status after this |
|---|---|
| Stage 0 — honest instruments | **done, and the seed was the missing half.** `room total` fixed the census; the box still had one sample per arm |
| Stage 1 — gravity in the dig | **built and measured at the turn, not the target.** Real (room 1.51x, 12/12) and mostly volume; `vert` 1.18x on 9/12 is not significant |
| Stage 2 — an aggregation point | **the site depth works and the density reading subtracts from it.** §2b. The stage's premise — that `Crowding` was a correct mechanism needing somewhere to act — is not supported |
| Stage 3 — curvature on the dig | **re-scored, and the headline number shrinks.** The sign survives; the size is **1.31x on `room total`, not 2.3x on `roofed`**, and the negative wire makes the nest *wider* on 10 of 12. §2c |
| Stage 4 — fresh spoil attracts digging | **the marker is priced and it is worse than the record thought on the dig side and better on the drop side, and neither is enough.** §4 |
| Stage 5 — contents | untouched, and still gated behind §10's *what is a nest for here* |
| §1 *"interventions on whether cannot produce a shape"* | **refuted in its literal form** and repaired: a *scalar* intervention on whether cannot; one applied over a region inherits the region's shape. §0 |
| §3 *do not build a digging pheromone* | **stands, and the reason should be the engine's rather than Bruce's.** §6 |

**The recommendation, in one line: sweep `PIXEL_PHYSICS_NEST_SITE_ROWS` as a
shipped value rather than a measurement switch, and settle its default by eye.**
It is the only lever in this line that has moved the aspect ratio, it costs
nothing (`adjacent_nest`'s site branch is *cheaper* than the eight neighbour
reads it replaces), and the owner's verdict on the shipped control is on the
record from this morning: *"looks like nothing. a hole floating spoil"*
(card `20260919T091527031Z-675ecd`).

**What it is not.** Under `nest-biology` §10 the nest has no purpose in this
game — no granary, no brood, nothing environmental that kills — so "deeper and
narrower" is a claim about how it **looks**, which is why it went to the review
queue rather than being argued here. Nothing in this report is an argument for
building the nest's machinery ahead of its function.

---

## 8. How this could be wrong

- **The site reach may be measuring the gate rather than the nest.** The two
  controls against that are in §2b — the room is neither the dial nor
  proportional to it, and two reaches that dig identically produce 20 rows
  against 30 — but a third arm that moves the *gate* without moving the
  *region* would settle it outright and does not exist.
- **One bed.** `digbox` is a bare box with no food, and `(FoodAdjacent, Dig,
  0.8)` is the largest direct term in the shipped dig wiring, set to zero here
  by construction. Everything above is a statement about the nest mechanism in
  isolation; the played bed may weight it differently, and `dead-ends.md`'s
  own `(Crowding, Dig)` entry records a sweep giving three different answers
  on three trunks of one round.
- **`vert` cannot rank a shape against a ruin**, and §2c is the worked case —
  the arm with the second-highest `vert` in this report has half the room and
  nine rows of depth. Every `vert` claim above is quoted with `room total`
  beside it for that reason.
- **`iqr` is new.** It is proven red both ways in the selftest against a
  hand-drawn control, and the `p50x` specificity check says nothing drifted
  sideways, but no result here rests on `iqr` alone — the site reach's case is
  carried by depth and `vert`.
- **The nulls are nulls, with the power that implies.** Twelve paired seeds
  and a sign test detect a consistent effect, not a modest one;
  `dead-ends.md`'s `(Crowding, Dig, 0.6)` entry already works this out for a
  33-seed design (80% power against a true rate of 0.74, 17% against 0.60).
  Five coin flips at 5–7 of 12 rule out a large effect and say nothing about a
  small one.

---

## 9. Commands

```
cargo build --release --examples          # in the same command as any run

# the control, and the twelve-seed sweep any arm is read against
for s in $(seq 1 12); do
  RAYON_NUM_THREADS=1 ./target/release/examples/digbox \
      ants=300 rate=8 w=400 soil=80 frames=6000 seed=$s
done

# the arms, each an env switch over the identical command line
PIXEL_PHYSICS_CROWDING_LOCAL=near|wide          # the local density reading
PIXEL_PHYSICS_LAB_ROOM_TARGET=0.5|8.0           # the room-per-ant set point
PIXEL_PHYSICS_NEST_SITE_ROWS=16|40              # how deep "at the nest" reaches
PIXEL_PHYSICS_DIG_DOWN=0.15|0.5|1.0             # turn toward down at the dig roll
PIXEL_PHYSICS_SPOIL_DROP_COVER=0.0|0.25|0.75    # hold the pellet while under cover
digbox curvdig=-0.6|0.6                         # curvature on the dig (Stage 3)
PIXEL_PHYSICS_BURROW_LINING=off                 # the lining ablation, for the mound census
digbox gate=25.5                                # re-centre the chamber gate

./target/release/examples/digbox selftest       # four geometries, two polarities

# the picture
RAYON_NUM_THREADS=1 PIXEL_PHYSICS_NEST_SITE_ROWS=40 ./target/release/examples/digbox \
    ants=300 rate=8 w=400 soil=80 frames=6000 seed=2 stops=6000 \
    scale=5 crop=120,10,170,100 out=/tmp/rows40.png
```

Read `room total`, `iqr`, `room WxH` and `vert` off `SUMMARY`. Never `roofed`
alone, and never the `trace` line's *"spread over N columns × M rows"*, which
is the spread of at-nest ants and follows the reach dial by construction.
