# The teacher: design doc for the next steps (rules, planners, search, and the colony)

*2026-10-21, by the teacher-approach session. Status: **design v2, revised after three adversarial reviews** (method
and literature, biology and the game, evidence against the code). The findings and what was done with each are in
[`teacher-design-review-2026-10-21.md`](teacher-design-review-2026-10-21.md); v1 is commit 6aa3b4b4. **Nothing here is
built** beyond step 1 (the three-rule teacher, the copier, the probe) and the general nest score (section 6, a Python
prototype). It answers the owner's question of 2026-10-21: keep the current teacher or improve it, and how a teacher can
do more than simple rules (pathfinding for soil, food and digging; the nest tools choosing where to dig). Companions:
[`search-teacher-design-2026-10-21.md`](search-teacher-design-2026-10-21.md) (the search teacher in full; literature in
[`notes/search-teacher-literature.md`](notes/search-teacher-literature.md)) and
[`notes/nest-architecture-2026-10-21.md`](notes/nest-architecture-2026-10-21.md) (real nests in vertical section).
Background: [`guidelines-2026-10-20.md`](guidelines-2026-10-20.md) (T1-T9, C1-C6),
[`step1-results-2026-10-21.md`](step1-results-2026-10-21.md).*

## The answer, first

**Keep the current teacher, and grow it into planners -- but find out why copying fails before building any.** At 8
rows the hand teacher is not what limits us: driving s161 it scores 20.66 full loads a world against 6.8. Copying is:
"go down" copies (6.42 -> 10.57, repeated at 10.60 and 9.80), "dig at the foot" collapses the copy (3.35) though the
rule *acting* alone is fine (+5.16), and "walk home" spreads to foragers. Two rounds of DAgger made copies worse. A
planner is one more source of labels, and labels are not what is failing; built now, its first failure could not be
told apart from the copier's. So the order is:

1. **Step 0.5, about a day, offline on the step-1 logs** (section 4): trace the dig copy's collapse ant by ant; a
   joint-fit diagnostic that says whether the wall is the student's capacity, a missing context sense, or labels that
   contradict each other; the context probe (can the ant tell an empty digger from an empty forager?); and a retrofit
   of the new lesson form onto "go down", which must reproduce its +4 before anything relies on it. **Go/no-go**: if
   neither dig nor walk home can be made to copy, no planners -- selection from the go-down copy, or a capacity change
   through the brain lane.
2. **The search pilot on dig-or-not** (section 8), which goes straight at the lesson that failed and is the clean test
   of whether "dig needs descent beside it" is real.
3. **Planners**, each through the same gauntlet (section 9): **nest digging** scored by the general nest score the owner
   asked for (section 6), then **soil placement** for the mound the owner wants (section 5), then **foraging routes**.

**The answer to "will simple rules make a simple student"** stands, with one correction from the literature. The
student never copies a rule, only its choices on its own states, so a teacher may compute anything; the limit is what
the ant can tell apart, measured per decision type by the probe. **But a teacher far stronger than its student can make
a worse student even when the information is there** (distillation: Mirzadeh et al. 2020; guided policy search,
PLATO, A2D, dual policy iteration all make the planner *student-aware*). So every planner here breaks near-ties by what
the brain already prefers (section 1).

**The nest is scored by what it has, not matched to a blueprint** (owner, 2026-10-21: "can we reward more general
things like number of well proportioned tunnels and chambers"). The score is built and checked (section 6): well-built
cells, chambers graded on height (7 cells, the owner's ruling), proportion and a flat floor; tunnels graded on width and
credited only as much as the chambers they serve; cracks, open craters and stray voids charged. Of fourteen controls,
the real-nest blueprints score highest and every built-bad shape scores near zero or below. The owner's pick, the
harvester stack, becomes the *default species profile* of the planner that generates a fresh plan per world -- other
species (or other creatures) later are other profiles.

**Three ideas from the colony discussion stay in** (section 7): a central planner in a real crowd, colony-level choices
taught as outcomes, and nothing that reads crowding taught solo. The reviews add a pre-gate: probe the central
assignment on 12-ant logs before building it, and run the existing brain in 2- and 4-ant boxes first, because local
crowding may already size the nest without any teacher.

**Two things the owner should know, both from the reviews and the code:**

- **Placed soil never slides in this engine.** A pellet with footing stands at any slope, including a wall, until
  something undermines it; then it turns to loose soil, which runs at 33°. So a mound's shape has to be *taught*, the
  engine will not round off a badly placed pellet, and spill-back comes from cuts that undermine the mound or the rim,
  not from pellets rolling.
- **A full ring round the door is, in a 2D slice, a wall across the road to the food.** The record shows that this is
  why ants dug into the mound in the earlier game, and that stopping them starved the colony. Real ants also build a
  **one-sided crescent** (Trachymyrmex, Pheidole). Ring against crescent is put to the owner as a paired render with
  the foragers' route drawn on (section 5), not decided here.

## 1. What a teacher is, in this framework

Every teacher has four parts, which is what lets a rule, a planner and a search share one copier, one probe and one
judge.

| Part | The three rules (step 1) | A planner | The search teacher |
|---|---|---|---|
| **Goal** | Implicit: full loads | Explicit: a nest that scores well, a mound shape, food home | An outcome score (full loads, plus a value of what is in flight) |
| **Computation** | One step of geometry | A search on the true world (shortest paths; a dig order over the plan; a trial *cut* in a copy of the world) | Roll each option out in a copy of the world, with the brain carrying on |
| **Values per option** | Allowed or not | A cost-to-go per option | Q(a) with a standard error |
| **Lesson** | The brain's own odds renormalised over the allowed options | See below | pi\*(a) ∝ p(a) · exp(A~(a)/tau) |

**The lesson form is per decision type, and it is earned, not assumed.** v1 proposed one regularised target for every
teacher, pi\*(a) ∝ p(a) · exp(A(a)/tau) (the MPO/AWR family; Grill et al.'s exact form differs). The method review
showed that for steps, worth ~0.02 full loads each, any tau in {0.03, 0.1, 0.3} multiplies the best step's odds by
1.07-1.95, about a tenth of the push that made "go down" copy (its hard target took the deepest step from ~0.6 to 1.0).
So:

- **Big calls** (dig here or not, put down here or not) use the regularised target, tau chosen on the selection worlds
  for full loads *with fetch and trail held at their floor*.
- **Steps** keep the hard target that worked, unless the retrofit in step 0.5 shows the regularised form reproduces
  "go down" (at least +3 full loads). A is divided by the decision's stake before the exponent if the soft form is used.
- **p is the current student**, not s161, wherever the student has changed (the method review's reading of why DAgger
  hurt: anchoring the copy back to s161 exactly where it had learned to differ). Tested in step 0.5.
- **The exchange rate** from a planner's cost (cells of path) to full loads is the *current student's*, re-measured
  each round, per frame of the whole run, and stated as such; where the arena's energy budget binds, it is per joule
  instead, as search charges it.

**Every planner is student-aware**: among options within ε of the planner's best, it prefers the one the brain already
rates highest (the cheapest form of guided policy search's "stay near the policy"). And every planner gets its own
check against short rollouts (search's B1): on ~100 sampled decisions, does its ranking agree with what actually scores
under the brain's continuation?

**Abstain** where no option clearly beats the brain's average: the lesson is the brain itself. The copier needs no
change for that.

## 2. Today's teacher: what stays and what changes

| Part | Decision | Why |
|---|---|---|
| **Go down** | **Keep, as is.** Copies cleanly and repeatably (10.57 / 10.60 / 9.80; fetch +0.5 to -1.9 trips) | -- |
| **Dig only at the foot** | **Keep. Copy it next, after step 0.5 says why it collapsed.** v1 said "never copy it alone" because the ceiling showed a coupling; that was a misreading. *Acting* alone the rule gains +5.16 (memory-consistent) with one seed under 3; only the *copy* collapses, cause untraced | The search pilot (section 8) is the second, independent attack on it |
| **Walk home** | **Keep.** v1 "replaced" it with the soil planner's carry path, but that teaches a different decision: walk home teaches the *empty* ant's way back to dig, worth +5.40 acting on s171. Someone must teach the empty return trip in every arena; until the context probe passes, it is this rule, copied with fetch anchors, or left to selection | The method review's B3 |
| **The put-down rule** | Stays dropped (owner agreed) | -- |
| **Its role** | **The examiner, and the bar.** A planner replacing part of the rules must be non-inferior to what it replaces, acting (section 9) | -- |

## 3. The gate: what the ant can tell apart

The teacher may use anything; what the student can learn is what it can predict from its own inputs (Weihs et al.,
the imitation gap). **v1 misread step 1's probes.** The 94-100% figures were the *all-features* probe (the best model
on every option feature), and the 0.938 among them was the put-down rule, never copied. Through **the brain's own
channel** -- each option scored from its own features plus the shared context, and the job stage seeing one best score
per verb -- the dig rule read 0.65/0.87 recall/precision, and the brain's dig score separated a foot cell from a wall at
AUC 0.613. **That probe predicted the dig failure.** Go-down's channel was clean (`DEPTH` on each step, 1.000) and it
copied; walk home's channel was clean (0.995) and it spread, which no probe of its own label can see. So the gate is
three numbers, each with a bar (uncalibrated: three data points):

| Probe | What it asks | Bar |
|---|---|---|
| **All features** | Is the information anywhere in what the ant senses? | ≥ 0.95 balanced accuracy, held out by world |
| **Brain channel** | Can the brain's own wiring carry it? | ≥ 0.90 balanced accuracy; AUC ≥ 0.85 on the minority class |
| **Context** | Can the ant tell the taught states from the kept-skill states that look like them? | ≥ 0.85 AUC separating them through the brain channel |

Positive control: a pure function of the senses. Negative control: the within-decision shuffle.

**Where a probe fails, the ways out, cheapest first:**

| Way out | What it is | Example here |
|---|---|---|
| 1. **An existing sense** | Widen the feature set | `f::SPOIL` (placed spoil, apart from native soil and from `f::LINED` lining), `f::CUT` (the hole), `f::ROOF_DROP`/`c::ROOF_CUT` (a cut that drops a cell one cut ahead -- not the cascade when loose soil runs), `c::DEPTH` (compressed at depth: 48 rows reads 0.75, 64 reads 0.80), `HOME`, `HOME_DIST`, `UP`. **Corrections from the code**: `f::HEADROOM` is open cells above the *option's target* (capped at 6), so on a wall or roof cut it reads 0 whatever the chamber's height -- it is not a roof template; `f::CROWD` reads the ant's own body (0.125-0.25) and `f::ROOM` takes 32 values solo, so they vary without a crowd; only `c::CROWD` is 0 solo |
| 2. **Memory** | The 12 memory units, or a short history | Distance since the last chamber, for spacing |
| 3. **Teach the local rule itself** | Where real ants have a local rule, **the rule is the teacher**, and the planner earns its place only on the *disagreement set* -- the choices where planner and rule differ (stability, connectivity, never dropping a roof). Measure that set and probe it alone; if it is small, copy the rule and skip the planner. **This narrows the nest to shapes local rules make**, which the owner is told | Aphaenogaster's "down from a chamber's edge, widen at the bottom"; spacing by depth |
| 4. **A new local, biologically realistic sense** (owner: on the table) | Only when a probe names the missing fact, through the brain lane, new links at zero | **Headroom over the ant's own head** (the roof height a chamber digger stands under: no sense carries it today). **Body pitch** (head row minus tail row, graviception -- Pheidole's drop point moved with a 15° tilt), better grounded than "slope under the put-down cell". **A decaying fresh-pellet mark** (Khuong's pheromone; a permanent mark adds nothing `f::SPOIL` lacks, and `dead-ends` measured it discriminating on ~1% of drops). **Encounter rate**, a memory integrating contacts (Gordon; Pratt), for the colony |
| 5. **Teach the outcome instead** | Out of the copy loss; selection on a colony score | Section 7 |

**Already tried on the old hand-written creature** (each has a re-test condition that is a cheap pre-check): slope at
the put-down (`dead-ends` `SurfaceCurvature` on `DropSpoil`: moved nothing; census the reading at drops first), the
building mark (Khuong, declined twice), a room-height cap (`FLAT_ROOM`: brood rooms on 4 of 12 runs, then a halved
colony), crowding-driven room digging (Toffin's mechanism: fired, built no rooms, buds never moved). None of these was
a *taught* lesson, which is what is new here; the pre-checks still apply.

**Capacity is not only a brain change.** s161 uses 8 of its 16 scorer units; the other 8 have output weight 0. Reviving
one for a lesson is possible inside the genome, at an energy cost per link. That comes before asking the brain lane for
more units.

## 4. Step 0.5: why copying fails (first, about a day, offline)

| # | What | Reads | Decides |
|---|---|---|---|
| 1 | **Trace the dig copy's collapse ant by ant** (the `funnel` skill): every ant in the 15 seeds under 3, inputs and chosen option at every decision from the last good foot cut on | Where it strands, and on which inputs | Whether "dig needs descent" is the mechanism, or something else |
| 2 | **Joint-fit diagnostic**: fit dig (and walk home) plus the anchors offline three ways -- (a) trainable slots as in step 1; (b) every slot free, ignoring energy; (c) (a) plus a one-bit arena-context input. Held-out loss on lesson and anchor states separately; the cosine between lesson and anchor gradients on shared slots | Only (b) fits both: **capacity**. Only (c): **context** (a sense). Neither: the labels contradict each other on states the ant cannot tell apart, which is a **teacher** problem | Which lever, and whether planners are worth building |
| 3 | **The context probe** on walk home: empty soil-arena surface ants against empty fetch foragers | Section 3's bar | Whether walk home can copy without breaking fetch |
| 4 | **Retrofit the lesson form** onto go-down (A = path saved to the foot × rate) at each tau, and onto dig and walk home; plot soil gain against fetch loss, beside the same curve for step 1's per-group loss weight | If the curves coincide, tau buys nothing against spread | The lesson form (section 1) |
| 5 | **DAgger anchored to the current student** instead of s161, one round on the go-down copy | Against 9.50 | Whether DAgger is worth keeping |
| 6 | Does the go-down copy carry an unlabelled dig lesson? (Its 8 revived `DEPTH` links feed every verb; the GIF shows cuts concentrated at the foot.) Per-rule agreement on its own runs | -- | How far "go down copied cleanly" can be trusted as one lesson |

**Go/no-go, written before the results**: if after step 0.5 neither dig nor walk home copies (gain over two
seed-clustered s.e., fetch and trail at floor), do not build planners. Instead: selection from the go-down copy with its
links protected; copy agreement as a fitness term (T4's second mode); or the brain lane's capacity change if diagnostic
2(b) says capacity.

## 5. The soil-placement planner (for the mound, not for full loads)

**What the reviews changed.** v1 put this planner first and valued each put-down by a *trial drop* in a copy of the
world. Three facts from the code undo that:

1. **The arena credits a full load where the pellet is put down**, not where it ends up; the 2,000-frame settle only
   moves the refill penalty (-4 of `score` a cell back in the hole), never `full_loads`.
2. **Placed spoil never slides** (`spoil.ron`: `self_supporting`, `needs_footing`; `update_powder` returns before any
   movement rule). Put-down already requires footing, so a trial drop returns the same value for every option.
3. **At 8 rows there is almost nothing left to gain**: s161 carries 88% of its full-depth cuts out as full loads, and
   86% of its put-downs are already clear of the hole. A perfect soil planner adds at most ~0.8 full loads a world --
   under two standard errors.

**So this planner is for the mound the owner wants, scored on the mound, and it waits for the nest planner.** It
values a put-down by geometry: clear of the **planned** hole (today's hole plus the nest plan's cells, which is the one
privileged fact that bears on S3), outward of the crest, on the far side from the hole -- graded with a broad peak just
past the crest, as real drop points are distributed (Pheidole), and with each ant's preferred side drawn from its `SELF`
number (Cataglyphis diggers each keep a carry direction). That gives a lumpy, real-looking heap instead of an engineered
trapezoid.

**Its privileged computation is a trial *cut*, not a trial drop**: before the nest planner labels a cut into the mound
or near the rim, it runs the cut in a copy of the world and checks whether spoil above loses its footing and the slump
reaches the hole. That cascade is the delayed hazard no ant can see; `ROOF_DROP` sees only one cut ahead.

**Geometry with the engine's real angles**: loose soil runs at 33°, so a slumped mound of height h runs out about 1.54
h; r_crest ≥ half the mouth + 2 + 1.54 h.

**Ring or crescent: the owner's call, shown not argued.** A paired render of the same nest with a ring and with a
one-sided crescent away from the food, the foragers' route drawn on, and the dead-ends record beside it (`MOUND_DIG`:
refusing mound cuts starved colonies; `SPOIL_RING=1,2`: the mound arm was the worst at keeping the door in use, 30 of
72 samples against 54; a ring 21 columns out lost 2 of 4 colonies). **Digging through the mound is real biology** --
harvester shallow chambers lie in the excavated soil, fire-ant mounds are a matrix of tunnels -- so the failure to
avoid is spill-back and a lost road, not tunnels in the mound. **Mound cuts are never labelled "no"**: solo they are left
out of the copy loss (abstain), because a "no" copied solo would spread into the colony as the `MOUND_DIG` rule.

**The mound budget**: in a 2D slice the mound holds the nest's whole area. It is halved by sizing the nest to the
colony (section 6.3), and real ants in the one real 2D nest (a glass-sided enclosure) put about half their spoil back as
backfill and lining (Halfen & Hasiotis 2010, a conference abstract, 1,000 ants, lining included; the field figure is
2.5%). So over-dug margins and dead ends are legal put-down sites. (v1's "about 8 cells tall" was the renderer's cap,
and its "wider than any real disc" was wrong: a 34-86-cell reach is inside the harvester's range.)

## 6. The nest: a general score, and a planner that generates the plan

### 6.1 The general nest score (built; `measurements/blueprints/nestscore.py`)

The owner asked whether we must specify a structure or can reward general things. **The score rewards general
things**; a plan exists only inside the teacher, freshly generated per world (6.2). A fixed blueprint would also be
uncopyable: the ant cannot sense "column 140".

**What it counts, in well-built cells** (`chambers + 0.5 × tunnels − bad`):

- **Chambers**: roofed rooms whose every part contains a 7×5 core -- so a passage up to 6 wide, at any angle, is never a
  chamber. Each is graded on **height** (full credit 6-8 cells, the owner's 7; none at 3 or 13), **proportion** (full at
  width ≥ 3× height, none at 1×; real chambers are 3-20×) and a **flat floor**. Credit is area × grade, area capped at
  400 cells, so one enormous hall cannot stand in for several rooms.
- **Tunnels**: everything else in the nest, graded per cell on width across (full 3-6, none at 1 or 10), and **credited
  only as much as the chambers they serve** (plus a 60-cell allowance for the shaft), so a corridor maze is not a nest.
- **Layout** (species profile): the harvester is top-heavy -- the share of chamber area in the upper half scales the
  chamber credit 0.75-1, so it shapes but cannot dominate.
- **Charged**: cracks (open cells under 3 wide), a pit open to the sky away from the mouth, open cells not joined to the
  door. Only below-ground, roofed cells joined to the door count; the mouth may be open over 4 columns either side.
- **Size**: the score grows with what is dug, as real nests grow with their colony; `per_cell` says how well whatever
  the size; with a colony size, `target` is 25 open cells an ant (about 8% of floor under bodies, the real crowding
  figure) and `fit` the share reached.

**Why not the room census we have**: `rooms_of` marks a cell as room if 7 of its 9 neighbours are open, so a 4-wide
passage is all room and the owner's 4-cell passages join chambers into one; a 2-tall chamber has no room cells at all.
And `in_room`/`in_chamber` (v1 cited them) are rectangle tests of the founding cut, not a census.

**The controls** (`nestscore.py controls`, every case drawn with chambers green, tunnels blue, cracks red, pit orange,
stray purple):

| Case | Score (well-built cells) | Dug | Per cell | Chambers (graded) |
|---|---|---|---|---|
| Harvester stack, 7-cell chambers (the owner's pick) | 761 | 1,228 | 0.62 | 6 (4.3) |
| Garden-ant galleries | 936 | 1,383 | 0.68 | 6 (4.8) |
| Carpenter shallow | 631 | 1,004 | 0.63 | 5 (3.2) |
| Staircase stack | 275 | 577 | 0.48 | 4 (2.3) |
| Round room that buds | 210 | 621 | 0.34 | 4 (1.6) |
| Harvester as first drawn (5-cell chambers) | 148 | 755 | 0.20 | 4 (1.3) |
| The ladder's current plan (8-16-tall rooms) | 135 | 504 | 0.27 | 2 (0.8) |
| One huge flat hall, 140 × 7 | 428 | 1,044 | 0.41 | 1 (1.0) |
| Shaft only (today's 8-row arena) | 25 | 70 | 0.36 | 0 |
| Random-walk cave | 41 | 1,198 | 0.03 | 4 (0.8) |
| One tall room, 26 × 28 | 27 | 792 | 0.03 | 1 (0.0) |
| Corridor maze, no rooms | 23 | 3,060 | 0.01 | 0 |
| Many tiny 5 × 5 pockets | −5 | 700 | −0.01 | 0 |
| Cracks, 2 wide | −192 | 667 | −0.29 | 0 |
| Crater open to the sky | −367 | 127 | −2.89 | 0 |

**What the controls caught while it was being built** (each now fixed, and each the kind of thing a score gets wrong
silently): a 5 × 5 chamber core let 5-wide shafts count as chambers, merging the staircase into one tall "room" (score
43; the core is now 7 × 5); a wide crater mouth cut the whole nest under it off as "stray" (connectivity now runs
through the pit, which is charged but does not disconnect); and the random-cave control had hit the box floor and dug
one long flat hall along it, which the score *rightly* paid for -- the control was wrong, not the score. The score
also found a real defect in the first blueprint drawing: one harvester chamber met the nest only through a gap open to
the sky (100 stray cells before the connectivity fix; 7 pit cells charged after).

**What it does not yet do**: score real dug nests from the game -- the shipped colony's nests from `deeptrace` are being
scored next (section 6.4); grade chambers by what is in them (brood or food on a flat floor, the biology review's
suggestion); or run inside an arena (a Rust port beside `nestcensus`).

### 6.2 The planner generates the plan

The owner's pick, the **harvester stack**, is the default **species profile**: chambers 7 cells tall and at least 3×
as wide, hung off alternating sides of a zig-zag shaft, larger and closer near the top. Per world the planner draws a
fresh plan from the profile's ranges (sides, spacing, sizes), so the student learns the rules, not a picture. Other
species, or other creatures later (the owner's idea), are other profiles of the same score and generator.

**Dig order**: keep the nest connected (shaft before the chamber it serves, top down); never a cut whose trial cut
drops a roof, the rim, or the mound into the hole; nearest first; among near-equal frontier cells, the one the brain
already rates highest. **Grow in stages** -- a niche off the shaft, then a lens, then a lobe, then the next chamber down
(Tschinkel's deep chambers begin as a circular niche) -- so there is a watchable middle at every moment.

**What the ant would need to follow it** (to be probed in the brain channel, not assumed):

| The plan says | Sensed by | Status |
|---|---|---|
| Dig down here (the shaft) | `DEPTH`, `CUT`, the foot | step 1: channel probe clean for go-down |
| Start a chamber at this depth | `DEPTH` (compressed deep), memory for spacing | to probe |
| Widen sideways, not up | **nothing today**: `f::HEADROOM` is not the roof over the ant | candidate sense: headroom over the head (way out 4) |
| Stop widening | crowding or openness (`f::ROOM` varies solo) | to probe solo and in a crowd |
| Don't undermine the rim or the mound | `ROOF_DROP` one cut ahead | the cascade is unsensed: the planner's trial cut labels it |

### 6.3 Sized to the colony

The blueprints drawn so far are nests of colonies 10-1,000 times the game's: across real colony sizes the architecture
changes kind (9-300 workers dig a single tunnel; stacked chambers appear at 368-1,000; meshed networks need thousands,
Gautrais et al. 2014's compilation). So the profile scales by colony size: at 12 ants, a shaft and 2-3 chambers (~300
cells at 25 an ant); at 24, 3-4 chambers; a second shaft series later. That halves the mound problem too.

The plan-view "2D lab nests" (Toffin, Buhl, Gautrais) are horizontal sand discs, with no floor, roof or lifting: they
supply colony-level laws (area roughly proportional to workers), not section shapes. The round-room-that-buds blueprint
was a plan view drawn sideways and is demoted.

### 6.4 Real nests

The shipped (hand-written) colony in the goal box (`deeptrace seed=1..3 ants=0 frames=150000 mapevery=25000`, the
same score on `write_map`'s frame). Before the colony lands (frame 6,000) the nest is the 26-cell founding shaft:
**-7**. Then:

| Frame | Seed 1 | Seed 2 | Seed 3 | Ants (seed 1) |
|---|---|---|---|---|
| 25,000 | 78 (248 dug) | 65 (266) | 73 (272) | -- |
| 75,000 | 111 (374) | 100 (347) | 126 (383) | -- |
| 150,000 | **176** (488) | **185** (510) | **180** (498) | 600 |

Every seed builds **one** room -- a flat rectangle about 30 wide and 10 tall from the colony's room rule, graded 0.4
(too tall for the 7-cell profile, and boxy) -- plus small side pockets, and the score rises slowly and smoothly with
them: graded, not stepped. Two readings: the score agrees with the eye on a nest nobody drew (one room, some pockets,
a lattice mound above it that the score does not see); and by the real crowding figure (25 open cells an ant), **a
500-ant colony in ~500 dug cells has about a twenty-fifth of the room a real colony that size would dig**. Pictures:
`measurements/blueprints/nestscore-real.png`.

## 7. The colony tier

**Kept**: a central planner in a real crowd; colony-level choices (how big, when to stop, how many dig against
forage) taught as outcomes; crowding never taught solo; the world varied before the brain for division of labour (C2).

**Changed by the reviews**:

- **Pre-gate the central assignment.** "Your face, given where the other eleven are" depends on the other ants, which
  is privileged *information*, not computation, and multi-robot work finds local-only students fail to imitate central
  planners in dense settings (PRIMAL needed RL beside imitation; GNN planners needed neighbours' information). Compute
  the assignment post hoc on existing 12-ant logs and probe it. If it fails, the central planner is kept only as an
  *option generator* for one-ant-at-a-time counterfactual search (force one ant's option, let the rest run, score the
  colony -- SPARTA; difference rewards).
- **Try a local rule before outcome scoring.** Buhl et al. 2005 get nest volume growing with colony size from local
  density feedback alone, so sizing may be a local lesson (encounter rate, or `f::ROOM`) before it is handed to
  selection -- remembering that the old creature's crowding-driven digging fired and built no rooms.
- **Teacher-free first**: run the existing brain in 2- and 4-ant boxes. Does dug area already grow with N? And vary the
  switching cost. Both are cheap and say whether the colony tier needs a teacher for sizing at all.
- **Price it**: outcome scoring sends the hardest parts to selection on a colony score; the guidelines priced one
  Ferrante-sized run at $40-50 of the $100 budget. The labelling cost of counterfactual search grows with ants ×
  decisions, so it is sampled.
- **T1 is amended in the guidelines file** (privileged computation is allowed; privileged *information* is allowed only
  where a probe shows the student can recover it), with the owner's date.

## 8. The search teacher's place

In full in [`search-teacher-design-2026-10-21.md`](search-teacher-design-2026-10-21.md). **Moved ahead of the
planners**: its pilot, dig-or-not on the 8-row step, attacks the lesson that failed, and its sharp prediction -- labels
computed under the brain's own continuation never veto a wall cut descent needs -- is the clean test of the coupling
story. It also judges planners ("the planner proposes, the search disposes") and labels the big calls, while planners'
cost-to-go labels the steps.

Decisions on its open questions: **R1** (the brain inside the rollouts) as the main arm, **R2** (hand teacher inside) as
the control; **energy charged** at the current student's loads per joule, with the uncharged score beside it;
**persistent deviations allowed** (they name a kind of behaviour, not when to use it).

## 9. The gauntlet for every planner

| Stage | What | Pass | Controls |
|---|---|---|---|
| 0. **Look at the plan** | The plan cut into a real bed in the engine (lining, ants walking in it), not a flat drawing; event counts in the card's `meta` | The owner's eye | -- |
| 1. **The planner acting** (T9) | Drives s161 on the decisions it labels; every force and veto counted; GIF | Above the brain alone by > 2 seed-clustered s.e., **and non-inferior** (within 2 s.e.) to the rules it replaces, acting | Known-negative plan (e.g. a mound whose crest sits inside the scoring zone); known-positive: the hand rules |
| 1b. **Rollout check** | ~100 sampled decisions: planner ranking against short rollouts under the brain's continuation | Agreement ≥ the rollouts' own test-retest agreement − 0.1 | -- |
| 2. **The three probes** | Section 3 | The bars there | Positive: a pure function of the senses; negative: within-decision shuffle |
| 3. **Copy alone** | Into quieted s161 (and s171) | Gain > 2 s.e. | The copier's controls (round trip, reachable target, shuffle) |
| 4. **Closed loop** | 96 evaluation worlds, seed-clustered; fetch and trail | Kept skills within 1 trip / 0.03 of the start | -- |
| 5. **Combine** | Onto the previous lesson's copy | Above both alone by > 1 s.e. | -- |
| 6. **Select from the copy** | Selection with the kept-skill floor and protected links | Holds or improves the copy's gain | Selection from the uncopied start, same budget |
| 7. **Look again** | GIF of the copy against the start | The owner's eye | -- |

## 10. Order of work, and cost

All local unless the owner says otherwise (no VM without the owner's go).

| # | Step | Cost (estimate) | Gives |
|---|---|---|---|
| 0 | Step 1 finished: the copy repeats (10.57 / 10.60 / 9.80); DAgger round 1 9.50; s171's headroom is dig and walk home | done | -- |
| 0.5 | **Why copying fails** (section 4), with the go/no-go | ~1 day | Which lever: capacity, context, or the teacher |
| 1 | **Search instrument and pilot on dig-or-not** (force hook, determinism, clone cost, pairing S1, then B1-B3, C) | ~1-2 days; 1-2 CPU-hours | Whether search finds "dig at the foot" unaided, and whether its labels copy |
| 2 | **Nest score on real nests; Rust port** into the arena beside `nestcensus` | ~1 day | A nest score the dig rungs can pay |
| 3 | **Nest planner** from the harvester profile, solo, on the roof and chamber rungs | ~2 days | Chambers, copyable or not |
| 4 | **Ring against crescent** render for the owner; then the soil planner, scored on the mound | ~1 day | The mound |
| 5 | **Colony**: teacher-free 2/4-ant scaling and switching-cost runs; the assignment pre-gate on 12-ant logs | days | Whether the colony tier needs a teacher |
| 6 | Foraging planner (needs `LAST_FOOD`, a memory, if a probe asks) | ~1 day | Straighter trips; measure the fetch ceiling first |

## 11. Risks, and when to stop

| Risk | Signal | Response | Abandon if |
|---|---|---|---|
| **The student cannot hold more lessons** | Step 0.5: only the all-slots fit holds lesson and anchors | Revive a dead scorer unit; then the brain lane | Neither dig nor walk home copies after step 0.5 (the go/no-go) |
| **A planner improves its own goal, not full loads** | Stage 1: nest score up, full loads flat or down | Ask the owner which the arena should pay | -- |
| **The nest score is gamed** | A GIF that looks wrong while the score rises | Add the gaming shape to the controls; fix the score | A shape that scores above the harvester and looks wrong to the owner, twice after fixes |
| **Lessons interfere** | Stage 5 below either alone | Step 0.5's lever | -- |
| **A crowd decision depends on other ants** | The assignment pre-gate fails | Counterfactual search; outcome scoring | -- |
| **Search too noisy** | S1: dig call needs K > 50 | -- | As the search design's own table |
| **Planner step labels too weak** | Retrofit: soft target loses go-down's gain | Hard targets for steps, or persistent lessons (a whole path as one) | -- |

## Questions for the owner

1. **Ring or crescent?** To be shown as a paired render with the foragers' route; the record says a 2D ring is a wall
   across the road to food.
2. **Should a freshly placed surface pellet settle a cell or two before it packs?** It would make the real ants' "loads
   roll outward" true here, and is visible feedback; the cost is the record against loose pellets near galleries.
