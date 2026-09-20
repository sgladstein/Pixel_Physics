# The return leg works one lap, and one lap is longer than one ant's life

*Result, 2026-09-20. `engine`. Executes Step 1 of
[`ant-return-leg-plan-2026-09-20.md`](ant-return-leg-plan-2026-09-20.md);
diagnosis in `Reports/open-bugs-handoff.md` §Z29; handoff in
[`Reports/lanes/homing-return-arm.md`](lanes/homing-return-arm.md).*

**In one line:** `BrainInput::HomeAligned` gives a laden ant a sense that knows
where home is; the share of food-finding ants that complete the return goes
**6.8% → 28.1%**, better in 8 seeds of 8, and colonies where nobody ever
completes a lap go **4 of 8 → 0 of 8**. It ships **on**.

**And the loop still does not repeat.** One ant in 733 completed it twice,
because the homeward leg alone (1,924 ticks median) is longer than an ant's
whole life (1,491 ticks mean). That is the next problem on this line and this
wire does not touch it.

**Two things in it outlive the verdict.** The plan's wiring was unsound in a way
one run of the real evaluator catches and no amount of world-running would have
explained — recorded here and in `dead-ends.md` (`other:133`). And the
`P(move)` separation is a clean mechanism result on ~500k decisions, which
stands whatever is decided about shipping it.

## Contents

- [Reproduced first, bit-identically](#reproduced-first-bit-identically)
- [The plan's circuit does not work](#the-plans-circuit-does-not-work-and-eval_brain-says-so-in-one-run)
- [The mechanism bar: passed](#the-mechanism-bar-passed-decisively)
- [The outcome bar, read on the LOOP](#the-outcome-bar-read-on-the-loop--owners-ruling-2026-09-20)
- [It is NOT a repeating loop](#it-is-not-a-repeating-loop-and-that-is-the-finding-that-matters)
- [Yes, the ant eats its cargo on the way home](#yes-the-ant-eats-its-cargo-on-the-way-home--and-that-is-the-whole-economy)
- [Where the loop is actually lost](#where-the-loop-is-actually-lost--and-it-is-not-the-anchor)
- [Why they do not drop — read off the brains](#why-they-do-not-drop--read-off-the-brains-186067-laden-ant-ticks)
- [`AtNest` is not a drop gate](#atnest-is-not-a-drop-gate--it-is-the-colonys-whole-sense-of-home)
- [The cue beside the nest exists, and the nose cannot see it](#the-cue-beside-the-nest-exists-and-the-nose-cannot-see-it)
- [The sensor repair erodes the trail — dead end](#the-sensor-repair-works-on-the-reading-and-erodes-the-trail--dead-end)
- [Step 2: `vacated` is not attributable](#step-2-of-the-plan-deposit_atvacated-is-not-attributable-drop-it)
- [What shipped, and what is open](#what-shipped-and-what-is-open)
- [Reproducing the two arms](#reproducing-the-two-arms)

### Reproduced first, bit-identically

`Reports/data/align-census-8seed-2026-09-20.log` came back to four decimals on
every row, on both the `homebias=0.5` and the `DEPOSIT_AT=vacated` arms.
Pointed at home −0.2003 / 5.3%; pointed away −0.2399 / 3.9%. The diagnosis
holds and this tree is the one it was measured on. The `vacated` arm doubles as
a sensitivity control on the census: it widens the home/away gap 0.0396 →
0.0651, so the instrument can register separation.

### The plan's circuit does not work, and `eval_brain` says so in one run

`squash` is `x / (1 + |x|)`, so a shut `(Bias, 7, -45)` unit reads **−0.978,
not 0**. The gated pair is neutral when shut only because it has two units at
−0.978 and subtracts them — **the mirror is the neutraliser, not decoration.**
Contribution to the `Move` sum (`brain.rs`'s ignored `what_the_home_wire_emits`,
`Reports/data/home-wire-response-curve-2026-09-20.log`):

| arm | away | across | home | **empty ant** |
|---|---|---|---|---|
| plan's single gated unit | −2.115 | **+0.833** | +2.167 | **−2.439** |
| the pair (needs two units) | −4.282 | 0.000 | +4.282 | 0.011 |
| **shipped** | −2.143 | 0.000 | +2.143 | **0.000** |

A walking ant's whole `Move` sum is about **+0.25**, so −2.439 is `squash`
clamped to `P(move) = 0`: **a colony that never leaves the nest.** The +0.833
is the subtler defect — a *laden ants move more* lever wearing a homing lever's
name, which would have lifted every alignment bin together.

**What shipped:** the gate moved upstream of `squash` into the sensor
(`creature::sense` reads `HomeAligned` as 0.0 for an empty ant) and the wire is
a direct `(HomeAligned, Move, w)` instinct with **no hidden unit at all**.
Costs the gate as a gene; buys that **unit 7 stays free**, so trap 1's
collision with the fold-change plan does not happen.

### The mechanism bar: passed, decisively

8 seeds, ~500k laden decisions, `homewire=0` against `homewire=3`. **The
control is this same genome with this one slot unread**, so `mutation_rate`,
`live_slots` and every dimension are identical across the pair.

| `P(move)`, laden | pointed **away** | pointed **home** | ratio |
|---|---|---|---|
| control | 0.0782 | 0.1152 | 1.5x |
| gain 3.0 | 0.0282 | **0.5089** | **18x** |
| gain 6.0 | 0.0021 | **0.7990** | 380x |

Pre-registered: *"`P(move)` in the pointed-at-home bin rises from 0.078 toward
the up-gradient figure of 0.59"*. It reads **0.509**. And the two bins move in
**opposite** directions, which is the thing the sensor-side gate was designed
to guarantee and is why this is steering rather than ladenness.

**Read `P(move)`, not `mean along`.** The plan's acceptance line *"the
alignment census must separate"* is loose: `mean along` is the `PheroAAlong`
reading and **this change cannot move it**, because it does not touch the
plane.

### The outcome bar, read on the LOOP — owner's ruling, 2026-09-20

**The axis is the loop and not starvation.** Owner, 2026-09-20: *"I don't care
about starvation. I care about the loop... are most of them following the trail
to the food, picking up food, turning around, going back to the nest, dropping
it off, repeating."* An earlier draft of this report gated the
recommendation on colony survival and intake, and that was the wrong weighting.

**The loop counter is `trips_laden`** — an ant that reached the food, carried
it, and got back to the nest. Not `DELIVERED`, which runs ~100x it here.

| | neither | **wire** | vacated | vacated + wire |
|---|---|---|---|---|
| ants that ever lived | 218 | 162 | 193 | 160 |
| reached the food | 133 | 64 | 106 | 66 |
| **completed the loop (distinct ants)** | **9** | **18** | 11 | 19 |
| **as a share of ants that reached food** | **6.8%** | **28.1%** | 10.4% | 28.8% |
| colonies where **nobody** ever completes it | **4 of 8** | **0 of 8** | 3 of 8 | 1 of 8 |
| completed it **twice** | 0 | **1** | 0 | 0 |

Paired within seed on the completion *rate*, the wire is better in **8 seeds of
8**; per-seed median 2.2% → 23.6%. The gain is not the lever: 1.5 / 3.0 / 6.0
give 26.2% / 29.7% / 31.8%.

**And the leg got shorter, which is the pre-registered branch point.** Median
laden leg **2,671 → 1,924 ticks**, shorter in all four seeds where both arms
completed one. The plan says *"if `P(move)` rises and the leg does not shorten,
the step choice (Step 5) is next"*. `P(move)` rose **and** the leg shortened, so
**Step 5 is not needed** and §R4 stays closed.

### It is NOT a repeating loop, and that is the finding that matters

**Across all four arms and 733 ants, exactly ONE ant ever completed the loop
twice.** `max loops by one ant` is 1 in every seed but one. The repeat step the
loop is named for does not happen.

**The reason is arithmetic rather than behaviour**, and it is the same standing
fact the handoff already carried, now measured on both sides:

| | neither | wire |
|---|---|---|
| mean decision ticks an ant **lives** | 1,721 | 1,491 |
| median ticks for the **homeward half alone** | 2,671 | 1,924 |

**The return leg alone is longer than the average ant's whole life.** A full lap
is well over two lifetimes, so a second lap is unavailable to almost every ant
no matter how well it steers. The ants that do complete one are the long-lived
tail.

So what this wire bought is precisely **one lap becoming reachable** — 6.8% →
28% of food-finders — and not a forage cycle. **A repeating loop needs the lap
to fit inside a life**, which is either a longer life (the colony is starving,
so lifespan is the binding constraint — starvation matters here as a
*mechanism*, not as a value) or a shorter lap (a nearer larder, or faster
travel). That is the next question on this line, and nothing in the current plan
addresses it.

### Yes, the ant eats its cargo on the way home — and that is the whole economy

Asked by the owner, 2026-09-20: *"Don't ants have food in their mouth for the
whole return arm? Do they not digest that?"* They do. `digest_rate_of` is
applied to the crop every tick the animal holds one, and the face value is
credited to that animal's own energy. **The crop is not freight, it is the
forager's packed lunch**, and the return leg is eaten out of it.

That makes delivering and surviving the *same* resource, and the two columns
show the trade directly:

| | neither | wire |
|---|---|---|
| face value the gut **absorbed** | 187,200 J | **83,520 J** |
| cells **put down at the nest** | 209 | **1,944** |
| chews **parked** mid-digestion by a drop | ~224 | **~1,670** |

**The control ant eats its cargo and never arrives. The wire ant arrives and
goes hungry.** The parked-chew counter is the mechanism caught in the act: it
fires when an animal puts its last crop cell down part-way through chewing it,
and it is **7.5x higher** with the wire — ants carrying food home, mid-meal, and
dropping the remainder.

**So the cost recorded above is not a side effect of homing, it is homing.** A
forager that delivers has given away the food it was living on, and nothing at
the nest gives it back (§7.28). This is also why the honest framing of the
lifespan limit is a loop through the economy rather than a bare constant:

> deliver → give up your own supply → shorter life → one lap is already longer
> than a life → no second lap.

**Both arms hit that wall**: mean lifetime is 1,721 (neither) and 1,491 (wire)
decision ticks, against a median homeward leg of 2,671 and 1,924. **In neither
arm does the average ant live long enough to complete the leg it is on.** The
ones that do are the tail.

### Where the loop is actually lost — and it is NOT the anchor

**The stall is at the drop, not the return.** Traced on one ant with the wire
on: it covered the homeward leg in **186 laden ticks**, then spent **3,300 more
inside its own nest band holding food with `P(drop)` exactly 0.0000** — 732 of
them standing on its remembered home cell — and died there. `Drop` is a
knife-edge on `AtNest`: 0.4886 when nest material is in reach of the head,
**0.0000 otherwise at any crop fill, including a full one**. That ant had
`AtNest` false for all 7,950 ticks of its life.

**The knife-edge is deliberate and must not be loosened.** §Z28 measured
deliveries **511 → 180** when any away-from-nest drop probability was added: an
ant that may drop short of home does, and the longer the walk the likelier.

**The obvious culprit was already known.** §7.37 (2026-09-18) established that
`forage_anchor` is the **birth cell**, so nine of twenty founders born off the
comb carry a private wrong home for life — the traced ant above reproduces it
exactly. §7.37 also left the deciding question open, and this session answered
it. **It took three attempts and the first two were invalid**; both are recorded
in §7.37 because each fails in a way this repo warns about — a birth-site split
that was underpowered by an order of magnitude *and* contaminated (the anchor is
re-set on every nest contact, so the "wrong anchor" group fills up with corrected
ants), then a fix whose denominator was swamped by ants loitering on the comb,
which reported a correct anchor as **37x worse**.

The valid form, 24 seeds, restricted to pickups by ants that had reached the
food:

| wire on | loops / return-leg pickups | rate |
|---|---|---|
| anchor **on** nest material at pickup | 25 / 73 | **34.25%** |
| anchor **off** it | 35 / 123 | 28.46% |
| | +5.79 pts, 0.85 SE, paired 11/10/1 | no detectable effect |

**The finding is the ceiling rather than the difference**, and that is what makes
it robust: **66% of return legs fail with a verified-correct anchor** (95%
interval 23–45% on the good-anchor rate, so at best 55% still fail). Repairing
the anchor moves the bad-anchor legs to at most the good-anchor rate — about
**seven more completed legs in 196**. Worth doing on its own terms; not the
blocker.

**Do not start the granary on this.** Nothing banks a delivered cell (§7.28) and
that is true — but it is downstream of a delivery step that fails for reasons
not yet localised, so a granary would be tuned against a delivery rate that is
about to move.

### Why they do not drop — read off the brains, 186,067 laden ant-ticks

Owner's rule: check the brains at every tick that mattered. `trailfollow` now
probes `Drop` for **every** laden ant (it already ran `probe_full` on them and
nothing read it) and buckets the tick by the one input that decides the verb —
how far the nearest nest material is from the head.

| nearest nest material | laden ticks | % of laden | `P(drop)` | drops |
|---|---|---|---|---|
| **adjacent** (`AtNest` true) | 26,425 | **14.2%** | **0.4916** | **331** |
| 2 cells | 17,322 | 9.3% | **0.0000** | 3 |
| 4 cells | 3,276 | 1.8% | 0.0000 | 1 |
| 8 cells | 10,692 | 5.7% | 0.0000 | 3 |
| 16 cells | 12,174 | 6.5% | 0.0000 | 2 |
| 32 cells | 36,072 | 19.4% | 0.0000 | 11 |
| further / none | 80,106 | **43.1%** | 0.0000 | 8 |

**The `Drop` decision is not broken.** On the comb the verb fires readily —
`P(drop)` 0.4916, 331 drops out of 26,425 adjacent ticks. Every earlier account
on this line, this report's own included, treated the drop as the failure. It is
not.

**They are almost never on the comb: 85.8% of laden ant-time is spent where
`P(drop)` is exactly zero.**

**The second row is the sharp one.** 17,322 laden ticks — 9.3% of laden life —
**exactly two cells** from nest material: ants that walked home, are standing
beside the comb, and cannot put the load down. **Three drops in seventeen
thousand ticks.** `(AtNest, Drop, 1.0889)` against `(Bias, Drop, -0.2)` is a
step function at adjacency, so a two-cell miss is as good as a mile.

**So the loop fails on spatial precision, not on a decision.** Homing delivers
the ant to the nest *region*; the verb demands *adjacency*. The 43.1% at 32+
cells or with no material findable is the §7.37 wrong-anchor population plus
ants in transit — the anchor defect showing up again, and still not the whole
story, exactly as the ceiling measurement said.

**This re-opens a question §Z28 looks like it closed.** §Z28 measured deliveries
**511 → 180** when away-from-nest drop probability was added and concluded the
knife-edge must stay. That measured *terrain-triggered* drops — `MoistureGrad`,
`SurfaceCurvature` — which fire **anywhere on the walk home**, so an ant dumps
its load mid-route. A **distance-graded** drop, full at adjacency and zero by
two or three cells, cannot do that: it is only non-zero where the ant has
already arrived. §Z28's numbers do not cover it and do not forbid it. **Measure
it before building it** — the 17,322 ticks in row two are the population it
would convert, and that is a bounded, checkable prediction.

### `AtNest` is not a drop gate — it is the colony's whole sense of home

**This is the finding that decides the option space, and it was measured by an
oracle rather than argued.** The drop census said laden ants are almost never
adjacent to the comb, which reads as *the target is too small*. So: widen it.
`PIXEL_PHYSICS_NEST_REACH=r<N>` makes nest material count within N Chebyshev
cells instead of the shipped 8-neighbourhood. 24 seeds, paired within seed:

| reach | median loop rate | drops | paired vs shipped |
|---|---|---|---|
| **r1 (shipped)** | **34.8%** | **743** | base |
| r3 | 26.1% | 8 | 11/11/2 |
| r8 | **0.0%** | **0** | **1/23/0** |

**Widening the target takes deliveries to zero.** Not worse — *zero*, in 23 of
24 seeds.

**The mechanism, and the engine states it in its own comment.** `AtNest` drives
**five wires across four verbs** in `ant.ron`: `Drop` (1.0889), `DropSpoil`
(0.9), the `Crowding`->`Dig` gate through units 5/6 (30.0), and — the one that
matters — `(AtNest, 4, 0.05)` into a unit with recurrence `0.99995` whose output
is `(4, EmitA, 32.0)`. That is the **nest-charged odometer**, and `creature.rs`
says what it is for: *"Touching the nest resets the scent clock, which is what
makes channel A a gradient rather than a uniform smear."*

Widen `AtNest` and the odometer charges over a wide region, so the homing trail
stops being a ramp. **Measured directly** — the ants' own channel A, sampled at
five points nest->food:

| arm | profile | peak route cells |
|---|---|---|
| shipped | `[0, 4875, 0, 0, 0]` — a localised mark | 43.5 |
| r8 | `[4045, 9582, 3290, 13, 0]` — smeared over three | 32.5 |

**So the fix defeats itself through the navigation it depends on**: a wider door
destroys the gradient that brings ants to the door. That kills the
widen-the-target family outright, and for a better reason than §Z28's — §Z28 is
about dropping short of home, this is about not finding home at all.

**It also indicts the body-reach repair.** `nest_within_reach` widens `AtNest`
too, by one body cell rather than eight — the same defect in miniature, and the
likely reason its drops fell **743 -> 451**. It fires (the 2-cell bucket goes
`P(drop)` 0.0000 -> 0.0040/0.1140, so the mechanism is connected) and the loop
does not move: **12/11/1**. It ships **off**, behind
`PIXEL_PHYSICS_NEST_REACH=body`.

**And it is structurally incapable of fixing the miss it was built for.**
Measured on the focal ant's 253 real steps, **72.3% are purely horizontal**, so
the trailing body cell sits at the *same height* as the head. The body extends
backwards along the path, not downwards — for a vertical miss the tail is two
rows up as well.

**What a valid repair now has to look like.** It must move `Drop` **without
touching `AtNest`**, because `AtNest` is load-bearing for the trail gradient.
That means a separate sense — a `Drop`-only input meaning *nest material is
near* — or changing the geometry so the shipped radius-1 test succeeds more
often, rather than changing what "at the nest" means for the whole animal.

**One trap removed on the way.** `adjacent_nest`'s site branch
(`PIXEL_PHYSICS_NEST_SITE_ROWS/_COLS`) looks like the ready-made widening and
**cannot answer this question**: it measures its rows from `NestSite::surface`,
which is `colony_surface` at the site's **centre column only**, while the comb
follows the ground across all 53 columns. On terrain that is not flat it
therefore *excludes* most of the comb — drops **743 -> 11** at `ROWS=2,
COLS=26`. That is a defect in the switch, not a verdict on widening.

### The cue beside the nest exists, and the nose cannot see it

**The question option C turns on.** Two-phase homing — run the path-integration
vector far out, close the last cells on a sensory cue — is what a real ant does
and it needs no omniscience, but only if the cue is both *there* and *readable*.
§Z29 established a laden ant cannot read channel A **on the route**; whether
that also holds **beside the comb**, where the nest's own odometer emission is
strongest and the ant's mark is one tick old, had never been asked.

24 seeds, 60,654 laden ticks at a 2–4 cell miss:

| | |
|---|---|
| mean channel A one step **toward** the comb | **4,785.5** |
| mean channel A one step **away** | 4,172.7 |
| toward is stronger on | **62.8%** of ticks |
| the two are equal on | 0.1% |

**The signal is there.** 62.8% against a 50% null, on a 15% magnitude
difference, with the equal-rate at 0.1% so the plane is not degenerate — that
is a usable gradient beside the nest.

| the ant's own `PheroAAlong` at the same ticks | |
|---|---|
| pointed **at** the comb | **−0.3437** |
| pointed **away** | −0.3852 |

**And the nose cannot see it.** Both strongly negative, separated by **0.0415**
— the same order as the −0.20/−0.24 gap §Z29 measured on the route, and for the
same reason: `here` is the animal's own freshest deposit, so `(ahead − here)` is
negative whichever way it faces. The defect does not weaken near the nest.

**So C splits, and the sensory half is the one worth having.**

- **C-omniscient** — aim at `world.nearest_nest_site()`. Works, and buys the fix
  by telling the ant something it cannot sense.
- **C-sensory** — climb the gradient. The gradient is real (62.8%) and the
  *current reader* is blind to it. It is not available today and it is not far
  away: **§Z29 already named the repair and nobody built it** — its third
  candidate, *"compare two forward samples (`so` and `2·so`) so neither term
  carries the animal's own mark"*. That removes `here` from the reading
  entirely, which is the whole defect.

**That reordered the work**, and the next section is what came of it.
C-sensory is gated on a sensor repair that was already specified and cheap, so
the comparator went first as the thing both halves of the homing problem were
waiting on. **It was built, measured, and is a dead end** — so read the two
bullets above as *the cue is there and no reader we can build today reaches
it*, and C-sensory as closed rather than pending.

### The sensor repair works on the reading and erodes the trail — dead end

**Built, because the section above made it the prerequisite for everything
else.** §Z29's third repair candidate, named 2026-09-16 and never tried:
*"compare two forward samples (`so` and `2·so`) so neither term carries the
animal's own mark"*. Behind `PIXEL_PHYSICS_TRAIL_READ=fwd`, **channel A only**,
bit-identical when unset. The register entry is `dead-ends.md` `other:134`.

**The precondition held, which is why it was worth building.** §7.47 found the
single sensor lands in open sky or solid rock on ~70% of laden ticks, so a
reading that needs *two* samples to land looked hopeless in advance. Measured
with a new oracle in `trailfollow` (`tr_cmp`, which reads the plane directly
rather than through the sensor): both samples read zero on **11.8%** of laden
ticks at gap 90, because the plane carries diffused value into cells no
creature can stand in. The fear was wrong by a factor of six.

**24 seeds, paired within seed, one binary, gap 90.** Gaps 140 and 200 ran
too and cannot discriminate — the shipped arm closes **6** laps and **0**
laps there — so they are a floor, not a replication.

| | shipped | comparator | paired |
|---|---|---|---|
| closed laps, nest → food → nest | 91 | 88 | 10 up / 11 down |
| ants that reached the food | 199 | 193 | 10 / 11 |
| cells dropped at the nest | 3,781 | 3,682 | 8 / 15 |
| cells carried homeward | 9,560 | 9,938 | 12 / 12 |
| `net cells homeward`, per seed | 500.9 | 500.3 | 13 / 10 |

**Every one a coin flip — and the null is about the mechanism, not the
wiring.** The switch is provably connected, and moves the reading exactly as
designed: `MEAN |PheroAAlong|` **0.3603 → 0.3230**, down in 18 of 23 seeds
(p 0.011), and the trail term into `Move` **−2.218 → −1.922**, up in 17 of 23
(p 0.035). The animal reads a different number and walks to the same place.

**Why — and this is the part worth keeping.** `tr_cmp` reads the plane, not
the sensor, so the same row in the two arms asks *how separable is the trail
these ants actually laid*. Per-seed mean over 24 seeds at gap 90:

| near/far | shipped world | comparator world | paired | p |
|---|---|---|---|---|
| 1 / 2 | +0.0075 | **+0.0240** | 19 up / 5 | 0.007 |
| 1 / 3 | +0.0156 | **+0.0449** | 18 / 6 | 0.023 |
| 2 / 4 | +0.0165 | **+0.0399** | 18 / 6 | 0.023 |
| 3 / 6 | +0.0300 | +0.0380 | 12 / 12 | 1.0 |
| **6 / 12** ← the runtime pair | **+0.0617** | **−0.0160** | 4 / **20** | **0.0015** |

`sensor_offset` is 6, so 6/12 is the pair the ants actually read — and on
their own world it **goes negative**: the reading points away from home on
average. The short pairs get *better*. That shape is a tighter local mark with
no ramp under it.

**The candidate mechanism, and it is a candidate rather than a finding.**
Nest-band ant-ticks fall **124,560 → 86,252**, median 4,332 → 2,880 — but
paired it is **9 seeds up against 15 down** (p 0.31), so the direction is
suggestive and the magnitude rides a handful of seeds. What makes it the
candidate rather than one of several is that `(AtNest, 4, 0.05)` →
`(4, EmitA, 32.0)` is channel A's **only** writer in `ant.ron` —
`(Bias, EmitA, 2.0)` was removed deliberately — so the nest band is the only
place the long-range ramp gets built at all. **The erosion itself is the
finding; why it happens is one seed sweep away from being one.**

**So the sizing measurement was valid and inapplicable.** It is `CLAUDE.md`'s
*a cost that vanishes may be work that vanished* pointing the other way: a
**benefit** measured on a world the change does not produce. Channel A is laid
by the same animals that read it, so any repair to *how* they read it is
measured against a trail it will then change.

**The re-test condition is a different precondition, not a retune.** Give
channel A a writer that is not the foraging ants — a nest that emits on its
own, a fixed beacon — and the erosion cannot happen. Do **not** re-test by
moving the offsets: the short pairs are already better in the comparator world
and still buy nothing, because what the brain lacks is range, not contrast.

**One thing the attempt established that outlives it: the channel scoping is
load-bearing.** Applied to *both* trail planes — the reading loop sweeps them
together, which is `CLAUDE.md`'s *adding a member to a set something sweeps
enrols it in every rule over that set* arriving as a one-line edit — the
outbound leg collapses outright: **0 ants reached food in all 24 seeds**,
against 7–10 of 20 shipped. Units 2/3 read `PheroBAlong` gated on *not*
carrying food, and an empty ant emits A (from the odometer) but never B, so on
B the `here` term is somebody else's trail and the subtraction is doing its
job. The two planes are asymmetric in who laid them, and only A has the animal
standing on its own mark.

**The speculation contract was the one real cost of building it.** `sense`
now reads a sixth off-body cell, so `SENSE_RECTS` goes 5 → 6 and
`sense_read_rects` declares `forward_far` **from the same `trail_sample_point`
call `sense` uses** rather than restating the geometry. The rect is spent
whether or not the switch is on, so it cannot go stale against it. A sample
`sense` reads and the footprint does not declare is a cell a neighbour can
write without invalidating the speculation — a wrong world, only under
parallelism, and silent.

Two checks, and they are different claims.
`the_declared_footprint_contains_the_trail_sensor_cell` now loops both reaches
and was **proved sensitive the way `CLAUDE.md` asks** — drop `forward_far`
from the declaration and it goes red on every heading. And
`antcost par=verify ants=400` runs clean at **`cached%` 37.3**, which is the
number that says the check was not vacuous: speculation actually happened and
was verified against a fresh read 37% of the time. The unit guard is the
sensitivity evidence; `Verify` is the runtime corroboration.

**And a measurement error worth recording, because it is new here.** The first
pass at these numbers was wrong twice over: the analysis parsed the log *while
it was still being written*, and `trailfollow` sweeps three commute distances
(90 / 140 / 200) which the parser silently pooled into one seed key, last write
wins. It produced a plausible, tidy, entirely different table. The tell was two
of my own instruments disagreeing on the same file — 189 against 144 for one
column. **Key a parse on every dimension the harness sweeps, and do not read a
log until its writer has exited** (`pgrep -x`, never `-f`).

### Step 2 of the plan: `DEPOSIT_AT=vacated` is not attributable, drop it

Crossed with the wire rather than measured beside it, which is what Step 2 asked
for. **Alone** it moves loop completion 6.8% → 10.4%, paired **3/4/1** — a coin
flip. **On top of the wire** it moves 28.1% → 28.8%, which is nothing. The wire
carries the whole result and `vacated` is not a component of it. It stays behind
its env switch, off.

### What shipped, and what is open

**The wire ships ON at `(HomeAligned, Move, 3.0)`.** It is the loop that this
line exists to fix, the loop is better in 8 seeds of 8, and it takes the number
of colonies where *nobody* ever completes a lap from 4 of 8 to **0 of 8**.

**Recorded, and explicitly not gating that:** larder intake falls 12,699 →
2,280 J (2/6/0) and colonies die sooner, because nothing at the nest banks a
delivered cell (§7.28's granary), so the walk home is energy the colony does not
get back. Owner's ruling: the loop is the axis, not starvation.

**What is open is no longer a ship/don't-ship question. It is this:** one lap is
longer than one ant's life, so the loop cannot repeat. Whichever way that is
attacked — a granary so the trip pays for itself and ants live longer, a nearer
larder, or a faster lap — it is a new piece of work and not a tuning of this
wire. **Do not re-test this by raising the gain**: 1.5 / 3.0 / 6.0 all land
within 6 points of each other on loop completion and none of them makes a
second lap fit.

### Reproducing the two arms

**`ant.ron` holds `(HomeAligned, Move, 3.0)` — the wire ships ON**, so the
control is `homewire=0` and the test is the default. (An earlier draft of this
section described the arms the other way round, from the hours when the wire
was authored at 0.0 pending the loop ruling; `homewire=3` now trips the rider's
own *"already what `ant.ron` holds"* assertion, which is that assertion doing
its job.)

```
cargo build --release --example trailfollow          # set -o pipefail; ant.ron is include_str!'d
RAYON_NUM_THREADS=2 ./target/release/examples/trailfollow \
  mode=gap gate=shipped gaps=90 arms=hand seeds=8 seed0=1 ants=20 \
  frames=24000 relay=60 near=10 food=400 refill=400 stop=6000 trace homewire=0
```

**And the comparator's two arms, which are an env switch rather than a rider,
so one binary serves both.** `gaps=` omitted on purpose: the sweep's own
90/140/200 is the experiment, and **a parse of the output must be keyed on the
gap as well as the seed** — pooling them is how the first reading of this run
came out wrong.

```
B=./target/release/examples/trailfollow
C="mode=gap gate=shipped frames=24000 seeds=24 seed0=1 ants=20 relay=60 near=10 food=400 refill=400 stop=6000 trace"
RAYON_NUM_THREADS=2                              $B $C > shipped.log
RAYON_NUM_THREADS=2 PIXEL_PHYSICS_TRAIL_READ=fwd $B $C > fwd.log
```

### Data

- `Reports/data/homewire-loop-2x2-8seed-{a_none,b_wire,c_vac,d_vac_wire}-2026-09-20.log`
  — the 2x2 that carries the loop table and Step 2. It is the only one with the
  `LOOPERS ... repeat ... most loops by one ant` columns, which were added to
  `examples/trailfollow.rs` on 2026-09-20 to answer *"is it a repeating loop"*;
  `trips_laden` is a sum over ants and structurally cannot.
- `Reports/data/homewire-{24k,100k}-8seed-{ctrl0,gain3}-2026-09-20.log` — the
  earlier run length pair, taken before the wire's default changed, so their
  arms read `homewire=0` / `homewire=shipped`.
- `Reports/data/home-wire-response-curve-2026-09-20.log` — the four wiring
  forms through `eval_brain`.
- `Reports/data/trail-comparator-24seed-2026-09-20-{shipped,fwd}.log` — the
  comparator's two arms, 24 seeds x 7 bed arms x **3 gaps**. Everything quoted
  from them in this report is **gap 90 only**; a parse keyed on the seed alone
  silently pools all three.

### Also worth knowing

- **Only `ant.ron` is wired.** `longant.ron` and `ancestor.ron` carry the same
  homing pair on units 0/1 and the same defect; left alone until this is proven.
- **The cost was paid**: `BRAIN_INPUTS` 32 → 33, `live_slots` 918 → 942, every
  species' `mutation_rate` re-derived to `3.18 / 942 = 0.0033758`. Every
  breeding scene's numbers move from birth 1 — that is not a regression and
  cannot be checked by a diff.
- **Do not re-test by retuning the gain.** 1.5, 3.0 and 6.0 all give the same
  qualitative picture and the trend in colony survival is monotone against it.
- **Grading the sensor by crop fill will not help either**, and it is worth
  saying so before someone spends a cycle on it: this bed's larder is 960 J
  fruit against a 1,440 J crop, so a laden ant is at 0.667 fill and **cannot
  hold two cells**. Grading by fill is a 33% gain reduction wearing a
  distribution's clothes, and gain 1.5 already measured that region.
