# The pheromone planes: how long a trail lives, and what is wired to it

*Round 36, Lane C. The owner asked two questions on 2026-09-14 — **"do they
fade too fast to be useful?"** and **"make sure they are all wired up
properly"** — and this is the answer to both, with the instruments that
produced it.*

*Nothing here is tuned. Per the standing direction (*"give me the tools,
data, access to the parameters that need to be tweaked and I do that testing
myself in the game"*), every default ships unchanged and the one thing added
is a dial that did not exist.*

***Every measurement here was taken on `e01dd1a1`, branched from `f4e3b471`,
and is therefore BEFORE Lane E's plant-grazing fix
(`claude/evolution-lab-forest-eaten`).** That matters for the alarm plane and
not for the trail planes: §2e measures that **100% of alarm deposits in this
bed come from the grazing path E is removing**, so any alarm *rate* taken
here is a rate for a writer that is going away. The channel A/B work in §1 is
untouched by it.*

---

## The short answer

**A trail here is a live map of where ants are standing now. It is not a
memory of where they went.** Both halves of that are measured, and which one
the game wants is a choice of value rather than a bug:

- At full colony strength the network is real — **149–393 standing cells at
  a peak under 100 of 255**, across six seeds of the standard bed.
- Every one of those cells is held up by **traffic, not persistence**. An
  unreinforced deposit is gone in **144 frames**, against a colony round trip
  of roughly 2,200 — **0.065x**, where the module's own arithmetic reads
  1.4x. A cell needs re-laying **every 36 frames or less** to hold anything.
- So the one job a trail is classically for — **a scout recruiting the
  colony to a patch it found** — is not reachable. The scout's trail is gone
  long before the scout is home.

**And the knob documented as the one that matters is inert.** `DECAY_RHO` is
*"the parameter the whole mechanism balances on"*; setting it to **zero**
leaves an unreinforced trail's life **unchanged at 144 frames**. `DIFFUSE` is
doing the work, and until this branch it had no setter at all.

**The alarm plane does not reach anybody.** Its audible radius is about two
cells, in the loudest case the engine can produce.

---

## 1. Question one — the trail's life

### 1a. Why no existing number could answer it

`pheromone::tests::trail_following_sweep` is the harness both `DIFFUSE` and
`DECAY_RHO` were set from. It runs **a single follower re-laying its trail
every pass**, so decay never has to be *survived* — it measures how well a
value **tracks** a trail that is being continuously rewritten underneath the
follower. That is a real question and it is not this one, and `DECAY_RHO`'s
own doc says so while still carrying the value that harness chose.

`examples/pherolife` is the missing shape: **lay a trail, stop, and watch
it.**

### 1b. The ceiling is real; the margin is not

The standing argument runs: `build_decay_lut` forces every nonzero value
strictly downward, so a cell loses at least 1 per pass whatever `DECAY_RHO`
says; 255 passes is therefore the longest any trail can survive
unreinforced; at `PHEROMONE_INTERVAL = 12` that is ~3,060 frames against a
~2,200-frame round trip, about **1.4x**.

Every step of that is correct and the conclusion does not follow, **because
255 passes is what a cell *at 255* can survive and nothing writes 255.** A
move lays `DEPOSIT` (40), scaled by the brain's own `EmitA`/`EmitB` output.
The floor is subtractive, so a cell's ceiling in passes **equals its own
value**: a cell laid at 40 has a 40-pass ceiling, not a 255-pass one. That is
480 frames — **0.22x the round trip** — before any diffusion at all.

With the shipped constants it is worse, and the decomposition says why:

| arm | trail gone | vs round trip |
|---|---|---|
| shipped (`rho 0.03`, `blend 0.25`) | 144 frames | 0.065x |
| **decay only** (`blend 0`) | 432 frames | 0.20x |
| **blend only** (`rho 0`) | **144 frames** | 0.065x |
| neither — the LUT floor alone | 480 frames | 0.22x |
| `blend 0.10` | 228 frames | 0.10x |
| `blend 0.50` | 108 frames | 0.05x |

`cargo run --release --example pherolife -- sweep=diffuse`

**Read the third row against the first.** Turning `DECAY_RHO` off entirely
changes nothing. The trail's life is set by the blend and the LUT floor, and
the constant whose doc calls it *"the parameter the whole mechanism balances
on"* is the smaller term by a factor of about six.

The arithmetic: a trail is a **one-cell-wide line**, so a cell on it sees a
3x3 mean of about a third of its own value and the blend takes **16.7% per
pass** against decay's 2.9%. The realised evaporation rate of a shipped trail
is therefore **~0.19 per pass — inside the literature band of 0.1–0.5** that
`DECAY_RHO` is documented as sitting deliberately below.

Sweeping `DECAY_RHO` across that whole band moves the life 144 -> 60 frames.
It is close to inert because it is not the term doing the work.

### 1c. It stops steering before it is gone

The height of a trail is the "it fired" half. The **effect** half is what an
ant's own wiring does with it, and the two come apart, because the input is
scale-free:

```
PheroAAlong = (ahead - here) / (ahead + here + 1)
```

`pherolife` reports the **run drive** — what `ant.ron`'s authored path
(`(PheroAAlong, 0, 6.0)`, `(PheroAAlong, 1, -6.0)`, `(0, Move, 2.5)`,
`(1, Move, -2.5)`) puts into `Move` — beside the height:

```
  frame    peak   cells    drive
      0      40     120    0.889
     24      24     120    1.889
     48      15      77    0.000
     72      10      36    0.000
    144       0       0    0.000
```

**The trail stops steering at frame 48 and is still 77 cells long.** The
mechanism is quantization: the blend plus integer truncation flattens the
ramp into plateaus, and where `here == ahead` the ant reads exactly 0. A
picture of that plane at frame 72 shows a trail. No ant can follow it.

### 1d. What holds a trail up

A trail nobody walks *should* fade, so the other half of the answer is what
it costs to keep one. Re-laying the whole line every N passes:

| a pass every | holds at | drive | |
|---|---|---|---|
| 12 frames | 43 | 1.557 | followable |
| 24 frames | 15 | 2.546 | followable |
| 36 frames | 10 | 2.333 | followable |
| **60 frames** | **0** | 0.000 | **gone** |
| 120 frames | 0 | 0.000 | gone |

`cargo run --release --example pherolife -- mode=traffic`

Measured at mid-trail, which is where the ramp has fallen to about a fifth of
the deposit — the far end, and the end that has to reach for recruitment to
work. **A cell needs a pass every 36 frames or less.** On a 2,200-frame
circuit an ant crosses a given cell twice, so a route stays alive only with
something like **twenty to thirty ants walking it cell-for-cell**.

### 1e. The real bed agrees, and the control matters

An isolated harness overstates what the app sees, so the same question was
put to the standard lab bed through `frame::step`, the tick both binaries
share (`pherolife mode=world`). **`ants` is the control**: a standing-trail
count falling to zero has two causes that look identical — the trail decayed,
or the colony that laid it died — and the first reading of this table did not
have it.

```
  frame   ants  A deposits A cells A peak  B deposits B cells B peak
      0     52          0       0      0          0       0      0
   1000     52        552     208     58        107       7     23
   3000     52       3251     299     65        789      51     21
   4000     46       4921     342     66       1363      94     55
   5000     20       5813      35     26       2333      67     22
   7000      9       6729       2      1       2914      11     20
   9000      3       7039       0      0       3183       1     21
```

With the control in, the honest reading is **not** "7,039 deposits and no
trail" — the colony collapses from 52 ants to 3, and the late zeros are
mostly that. What the table does say:

- **At full strength the network is real**: 208–342 cells, peak 58–67.
- **It is superlinear in colony size.** 52 -> 46 ants (−12%) holds the
  network; 46 -> 20 ants (−57%) takes it from 342 cells to 35 (**−90%**).
  That is the traffic threshold in §1d showing up as a cliff: below about
  twenty ants no cell gets its pass every 36 frames, and the network does not
  thin, it goes.
- **Nothing is near saturation.** Peak 39–98 of 255 across six seeds.

Six seeds at 4,000 frames, all at ~50 ants: **149–393 standing cells, peak
39–98**. The pattern is not one seed's.

### 1f. So: register, do not tune

**`DEPOSIT` must not be halved.** P-14 says to halve it if trails pin at 255;
measured, the loudest cell in the bed is 98 of 255 and the trigger has never
fired. The plane is three-quarters empty at its peak and the problem is at
the other end.

What this branch ships is the **dial that was missing**:
`Pheromones::set_channel_diffuse`, per channel, with `DIFFUSE` unchanged at
0.25 and the alarm plane's pending-value split handled the same way
`alarm_rho` already does it. `DECAY_RHO` has had a setter since the alarm
plane arrived; the larger term had none, and a sweep over it was unreachable
without a `#[cfg(test)]` constructor — the `include_str!` trap in another
costume.

**If the owner wants a trail that outlasts its traffic, `DIFFUSE` is the
knob, and lowering it is not free.** `DIFFUSE`'s own doc holds the sweep that
chose 0.25 on tracking quality: 0.10 scores 0.623 on-trail against 0.25's
0.817. Long-lived and hard to track, or short-lived and easy — the two ends
want opposite settings and the existing sweep only ever saw one of them.

---

## 2. Question two — is all of it wired?

`examples/pherowire` expands every shipped genome and walks it. It does not
grep the `.ron`, for two reasons that both bite: `ant.ron` wires
`PheroAAlong` through hidden units, so a grep for a direct weight finds
nothing and the slot is live; and a weight can be authored, expanded, and
still be **dead on arrival** under `brain::W_EPS`, which `eval_brain` treats
as no connection. `W_EPS` is applied at **both legs** of a two-hop path,
because `eval_brain` does.

"Wired" has two meanings and the interesting case is the gap between them:

- **engine-live** — `creature::sense` computes the input and `creature::act`
  writes the plane. A property of the Rust, the same for every species.
- **species-live** — some authored genome carries a live path from that slot
  to an output. A property of `assets/species/*.ron`.

```
species            Afrnt  Alat  Alng Bfrnt  Blat  Blng alarm  layA  layB
ant                   .     .     y     .     .     y     y     y     y
ant_long              .     .     y     .     .     y     y     y     y
ant_block             .     .     y     .     .     y     y     y     y
ant_wide              .     .     y     .     .     y     y     y     y
ant_block_shaded      .     .     y     .     .     y     y     y     y
chitin_pale           .     .     y     .     .     y     y     y     y
hopper                .     .     y     .     .     y     y     y     y
longant               .     .     y     .     .     y     y     y     y
ancestor              .     .     y     .     .     y     .     y     y
flitter               .     .     .     .     .     .     y     .     .
beetle                .     .     .     .     .     .     .     .     .
```

**Everything is engine-live.** All seven reader slots are computed every
tick, both trail planes have a live writer and a live reader, and the alarm
plane's writers fire. Nothing is disconnected in the Rust. What the table
shows is four findings in the *authored* half — and **§2e is a correction to
the sentence you just read**, because "the writers fire" turned out to be
true of a writer nobody intended.

### 2a. Four of the seven reader slots are read by no species at all

`PheroAFront`, `PheroALateral`, `PheroBFront`, `PheroBLateral` — **0 of 11**.
They are sampled every tick for every animal and reach no output anywhere.

For the **laterals** this is deliberate and documented: `BrainInput::
PheroAAlong`'s doc records that both lateral sensors sit in open air in a
side-view world, measured at exactly 0.000 for an ant standing on a cell
holding `A = 27`, and says they are *"kept, unwired"* for a flier or a
swimmer. That is a real reason and it stands.

**The Fronts are not documented as unwired, and they are the only
concentration inputs there are.** `PheroAAlong`/`PheroBAlong` are pure
gradients, scale-free by construction — *"a faint trail and a saturated one
both produce a usable −1..1"*. So **no shipped animal can tell a strong trail
from a weak one.**

That collides with the justification for `DIFFUSE`'s value. Its doc rejects
the better-tracking end of its own sweep like this: *"the height of a
well-used trail against a lightly-used one is differential reinforcement,
which is the entire path-selection algorithm"* — 0.25 is chosen to **preserve
peak height**, paying 96%-of-tracking for 40%-of-peak. **Nothing reads
height.** The trade was made for a benefit no shipped genome can collect, and
it is the same knob §1b shows is setting the trail's whole lifetime.

*Not proposed here*, because it is a behaviour change over a shared budget
and this lane's job was to measure: wiring a Front slot to anything is the
cheap experiment that would make `DIFFUSE`'s peak argument true, and it wants
a seed sweep gating an order statistic, not an A/B.

### 2b. `ancestor.ron` cannot hear the alarm — routed to Lane D

Every ant-family species carries `(Alarm, Move, -1.0)` and
`(Alarm, Attack, 2.0)`. **`ancestor.ron` carries neither**, and it is the only
species that reads the trail planes and not the alarm. The word "alarm" does
not appear in the file at all, so this is an omission rather than a recorded
choice.

It matters now specifically because round 35 shipped **rivalry on by
default**: in a bed founded on the ancestor, fights write the alarm plane
every time and the founding lineage cannot act on it. `BrainInput::Alarm`'s
own doc calls this *"the one signal that lets a colony act as a colony in a
fight"*.

**Lane D owns `assets/species/*.ron`.** The change wanted is the two weights
the other nine species already carry, added to `ancestor.ron`'s instinct
list — see §2d first, which prices what they would buy.

### 2c. `flitter` is the flier, and it neither lays nor reads a trail

It carries the two alarm weights and nothing else: no `EmitA`/`EmitB`, no
reader on any trail slot. The laterals in §2a are kept in the codebase
explicitly *for* something moving in open space, and the one animal in the
tree that moves in open space does not read them. Also Lane D's file. Whether
a flier should use trails at all is a design question, not a defect — it is
recorded because the slots' stated justification names this animal.

`beetle` has a brain and no pheromone wiring of any kind, which for a
solitary animal is coherent and is noted rather than flagged.

### 2d. The alarm plane's reach is about two cells

The site `contest.rs` flagged as never measured — `DISPLAY_DEPOSIT`, 40
against 240 for a wound — plus the wound itself. `BrainInput::Alarm` is read
**at the cell the animal stands on**, so the row that matters is `d=1`, the
ant *beside* the fight; `d=0` is the victim.

One wound (`ALARM_DEPOSIT` 240):

```
  frame     d=0    d=1    d=2    d=4    d=8
      0     240      0      0      0      0
     24      82      6      0      0      0
     48      29      5      0      0      0
     96       3      0      0      0      0
```

**Loudest a neighbour one cell away ever hears: 6 of 255** — input 0.024,
contributing **+0.047** to `Attack` against an authored weight of 2.0. Two
cells away: **exactly zero, ever.**

One display (`DISPLAY_DEPOSIT` 40): a neighbour one cell away hears **zero,
ever.** The verb writes into a plane nobody but the displaying animal can
read. That is the measurement `contest.rs` asked for.

A single-bite scene could be the wrong scene, so the sustained arm is the
control — a wound every 6 frames (`ant.ron`'s `tick_interval`) over a 2-cell
body, 40 bites:

```
  frame     d=0    d=1    d=2    d=4    d=8   bites
     96     157    157     15      0      0      16
    240     157    157     15      0      0      40
```

Even then: **two cells off reads 15/255 = 0.059 -> +0.118 into Attack, and
four cells off reads zero for the whole fight.**

`creature::sense` already states that a here-read *"carries no direction at
all"* and calls that *"a limitation of one slot, not of the plane"*, pointing
at the trail planes for distance recruitment. **The measurement says it is
the plane.** At `ALARM_RHO = 0.25` the signal is ground down faster than
`DIFFUSE` can spread it, so there is no distance for a directional slot to
read even if one existed. Recruitment beyond the animals literally touching
the fight is not expressible on this plane at these constants.

`ALARM_RHO`'s doc already calls itself *"a dial on the parameters page rather
than a tuned constant — what the box wants has not been measured"*. This is
that measurement, and `set_channel_diffuse` now makes the other half of it
reachable too.

### 2e. Every alarm in a one-colony bed is the grazing path — **a correction**

**Measured after round 36's coordinator flagged the plant-loss regression
(Lane E, `claude/evolution-lab-forest-eaten`), and it overturns a number
published earlier in this same report.** §2's first draft cited *"336 alarm
deposits in a 9,000-frame bed, plane live"* as evidence the alarm writers
fire. They do. The writer is not a fight.

Same bed, same seed, same colony, one variable — the plants:

| arm | alarm deposits | plane |
|---|---|---|
| `founders=8` (plants present) | **336** | live |
| `founders=0` (no plants) | **0** | **never written** |

`cargo run --release --example pherolife -- mode=world frames=9000 seed=1
founders=0`

**Not "mostly" — all of it.** Remove the plants and the alarm plane is never
allocated. So in the shipped single-colony bed, **100% of alarm traffic comes
from the grazing path**, which is the loop Lane E is fixing: an ant grazes a
plant cell, the victim is an *organism* so the eat path calls `cry_alarm`,
and `(Alarm, Attack, 2.0)` — the only wired route to attacking — sends the
colony at a plant that feeds nobody.

This is the owner's own positive control reproduced from the other
direction: he sees alarm signals in a box containing only trees, where
nothing can attack anything; this takes the trees away and the plane stops
existing.

**What it means for the constants.** `ALARM_RHO` (0.25) and `ALARM_DEPOSIT`
(240) have therefore **never been calibrated against fight traffic, because
in this bed there has not been any.** When E's fix lands, the alarm deposit
rate in a single-colony bed goes to **exactly zero**, and every alarm number
in this report — §2d included — is measured on a bed whose only alarm writer
is about to be removed. §2d is unaffected in substance: it is a
*propagation* measurement over a hand-placed deposit and does not care who
wrote it. What is affected is any claim about **rate**.

**And it sharpens the open ruling** E is putting to the owner — *should
eating another creature raise an alarm, or should alarm mean only "I was
attacked"?* §2d is the evidence that bears on it: **at these constants
nobody beyond touching distance can hear an alarm however it was raised.**
So the recruitment argument for keeping eating-raises-alarm buys nothing
measurable today. If the owner wants that signal to *mean* something, the
constants have to move with the decision — and that is a second change, on a
seed sweep, not a rider on E's fix.

---

## 3. What this branch ships

- **`Pheromones::set_channel_diffuse`** (+ `channel_diffuse`,
  `alarm_diffuse`) — the dial the lifetime actually hangs on, per channel,
  defaults unchanged. Guarded by `the_blend_dial_reaches_the_pass`, which was
  watched going red with the setter stubbed out.
- **`examples/pherolife`** — lifetime, the diffusion/decay decomposition,
  sustaining traffic, alarm reach, and the real bed. `selftest` is the
  positive control.
- **`examples/pherowire`** — the connectivity walk. `selftest` is the
  positive control.
- Corrections to `DECAY_RHO`, `PHEROMONE_INTERVAL` and `DEPOSIT`'s doc
  comments, which each carried an argument this measurement overturns.

**No default moved.** Two items are routed to Lane D (§2b, §2c) and one
experiment is named and not run (§2a).

---

## 3b. What was then done about it (same day, `claude/pheromone-trail-lifetime`)

The owner read §3 and asked the question it deserved — *"are we fixing any of
these?"* Two answers, and they are different because the two planes are.

### The alarm is fixed

**§2d's numbers are not a tuning failure and the fix is not a constant.** The
ceiling is the *stencil*: a 3x3 mean attenuates about nine per cell, so at
`DIFFUSE = 1.0` — the largest the blend can be — a wound still reads 20 at one
cell and 1 at two. Nothing in the parameter space reaches.

**The error was modelling a shout as a substance.** A mean filter conserves.
That is exactly right for a trail, where a dozen ants' deposits adding up *is*
the path-selection algorithm, and exactly wrong for an alarm: spreading one
deposit over area makes every cell small and a `u8` floors small at zero. Real
ants do not share a chemistry between the two — a trail pheromone is heavy and
substrate-bound, an alarm pheromone is a small volatile molecule, and what it
makes is an **active space**, the volume around a source in which
concentration is over the response threshold (Bossert & Wilson's term; ~6 body
lengths for *Pogonomyrmex badius*, gone inside a minute).

`Spread::ActiveSpace` propagates by distance falloff — a cell takes the louder
of what it holds and its neighbour minus `ALARM_FALL` (12). Measured:

| one wound | d=1 | d=2 | d=4 |
|---|---|---|---|
| before (`arm=diffuse`, kept reachable) | 4 | **0** | 0 |
| after | **148** | **88** | 24 |

`->Attack` **+1.161** and **+0.690** against `ant.ron`'s weight of 2.0, where
it was +0.047 and exactly zero; and the plane still empties in **12 passes,
144 frames**, which is the second-and-a-half `ALARM_RHO`'s doc asks for. **The falloff is also
the grading** — middle-of-the-fight and edge-of-the-fight now read different
numbers through the same weight, so the response is a distribution rather than
a binary, for free.

**`ALARM_RHO` moved 0.25 -> 0.35 with it, and that is part of the fix.** At
0.25 the constant never was the alarm's forget rate — a lone deposit also lost
about a fifth of itself per pass to the 3x3 mean, so diffusion was doing a
share of decay's job and the doc's *"gone in about a hundred and fifty
frames"* was right by accident. With the plane no longer spreading its value
away, 0.25 left a bite audible for **204 frames**; 0.35 restores the
documented **144**. This is `CLAUDE.md`'s *fixing a bug often exposes a
constant that was compensating for it* — and the thing that caught it was
`creature.rs`'s `the_alarm_forgets_faster_than_a_trail` going **red rather
than quiet**, which is the whole argument for that guard existing. It passes
again untouched; no other lane's file was edited.

**Reach and duration also separated, which may be the more useful outcome.**
They were one quantity while the plane conserved. Now `ALARM_FALL` sets how
far a cry carries and `ALARM_RHO` how long it lasts: sweeping the latter
0.25 -> 0.50 moves clearing time **204 -> 96 frames** while one cell out only
moves **171 -> 114**. Two dials the owner can turn independently, where
before there were none that reached.

### The trail is not, and the fix I proposed for it was wrong

**Recorded because it was proposed in this report's own PR discussion.** The
plan was to replace `build_decay_lut`'s `min(v - 1)` floor with a
threshold snap-to-zero. Two things kill it:

* **`dead-ends.md` already rejects half of it** — *"at small rho, or with
  rounding instead of truncation, a low value maps to itself forever"*. The
  rounding the snap needs is a recorded ghost-trail bug.
* **It targets the wrong term anyway**, which is §1b's mistake repeated. The
  floor is not what caps lifetime — **truncation** is. `(v * (1-rho)) as u8`
  loses at least 1 whenever `v * rho < 1`, so **lifetime ≤ deposit in passes
  for every rho**, and the explicit floor only binds at `rho = 0`.

With that understood, the whole candidate space was measured rather than
argued. Passes until an unreinforced line is gone, against a 183-pass round
trip:

| arm | passes | vs round trip |
|---|---|---|
| shipped | 11 | 0.06x |
| `DEPOSIT` 120 | 16 | 0.09x |
| diffuse every 4 passes | 22 | 0.12x |
| decay every 4 passes | 17 | 0.09x |
| `DEPOSIT` 120 + diffuse every 4 + decay every 4 | 61 | 0.33x |
| `DEPOSIT` 240 + diffuse every 8 + decay every 4 | **117** | **0.64x** |

**Stacking three behavioural changes at extreme settings still does not reach
one round trip.** And the ceiling is not quantization: diffusion at 0.25 costs
a one-cell line **16.7% of its peak per pass**, so even at infinite precision
an unreinforced trail is gone in ~30 passes. **Diffusion and long trail life
are the same knob pulling opposite ways**, and `DIFFUSE`'s own sweep prices
what lowering it costs (0.623 on-trail at 0.10 against 0.817 at 0.25).

So this one is **not a defect with a fix — it is a trade**, and by the
standing direction it is the owner's to make rather than a lane's to settle.
`set_channel_diffuse` (§3) is the dial for it; a *cadence* dial is the
companion worth building, because rate and frequency reach the same 2x while
trading different things — rate makes the spread permanently shallower,
cadence keeps its shape and delays it.

### The front sensors stay unwired, and that corrects §2a

**The owner's follow-up: wire them if I recommend it, but find out why it was
not done first.** Researched, and the recommendation is **no** — §2a's
observation is true and the implication I left standing under it was wrong.

**It was never done, rather than undone.** `git log -S "PheroAFront" --
assets/species/` returns nothing: those slots have never carried a weight in
any species file in any commit.

**The design intended concentration to be read somewhere else entirely.**
`creature-direction.md`'s motor stage specifies a probabilistic choice among
three forward candidates weighted by `(k + s_i)^2` — Deneubourg's nonlinearity
over three sampled *concentrations*. That is not what ants do here: `p_move`
comes from the brain and the animal steps or tumbles, with `choose_weighted`
used for other contested decisions but never scored on pheromone. So the
concentration reader the design called for was to be a movement rule, not a
brain input, and the run-and-tumble that replaced it is what `PheroAAlong`'s
own doc argues for on a surface.

**And the biology says the absent reader is the wrong one.**
`stigmergy-research.md` §2: Perna et al. measured individual Argentine ants
showing a **proportional (Weber's Law)** response to pheromone, *not* the
sigmoidal absolute response the classical model assumes, and agent
simulations with the Weber response still reproduced the literature's trails.
`PheroAAlong` is `(ahead - here) / (ahead + here + 1)` — a relative
difference normalised by the total. **That is a Weber response.** The engine
already implements the individual rule the biology has; a front-sensor weight
would add the one it does not. §2 puts the colony's side of it plainly:
*"the colony finds the shorter path without any ant measuring anything."*

**Measured, the existing input discriminates** (`pherolife mode=junction`, a
fork whose strong branch carries ten times the traffic of the weak one):

| blend | strong branch | weak branch | the ant's discrimination |
|---|---|---|---|
| 0.10 | 234 | **19** | 0.845 |
| 0.25 | 221 | **2** | 0.969 |
| 0.50 | 162 | **0** | 0.899 |
| 1.00 | 99 | **0** | 0.881 |

An ant on the trunk reads about **-0.01 down the strong branch and -0.98 down
the weak one**, with nothing reading a height anywhere. **Height is not what a
choice point needs; contrast is**, and a scale-free reader gets contrast free.

**Read the two middle columns rather than the last one.** Discrimination is
high at every blend, but above 0.25 it is high because **the weak branch has
been erased** — 0 of 255, not merely quieter. That is `CLAUDE.md`'s *a cost
that vanishes may be work that vanished*, and it is the real finding here: a
colony that cannot re-find an abandoned route is the ossification
`DECAY_RHO`'s doc guards against, reached by another road.

**So two doc corrections land instead of a wiring change.** `DIFFUSE`'s
stated justification — preserve peak height, because height *is* the
path-selection algorithm — is measurably not what path selection needs, and
its 4%-of-tracking payment buys nothing there. The value 0.25 survives for a
reason nobody had measured: it is a **trail-lifetime** knob, the same axis
§3b's table is about, and it **sits close to a cliff** (the weak branch at 2
of 255). `BrainInput::PheroAAlong` now records why the front slots stay
empty, matching how the laterals are already treated.

**And the geometry is the deeper reason, which the biology alone does not
give.** The owner's follow-up — *consider what real ants do, but also our 2D
geometry and if that changes anything* — and it does, in the direction of
making the answer firmer.

The Jones/Physarum triad this input list was modelled on (a front sample plus
two laterals at ±45°) assumes an agent in **open 2D**, whose problem is
*staying on* a line it could drift off in any direction. A creature here
cannot drift off. The whole-chain support rule holds it to the surface — a
chain falls unless some cell of it touches solid — so it walks a
**one-dimensional manifold** through a 2D world, and the surface *is* the
line. That is the real reason the laterals measure 0.000: at full offset they
point into open air and into rock.

A creature confined to a line does not need *"am I on it"*. It needs **which
way along it**, which is one signed scalar — and that is `PheroAAlong`.
Absolute concentration answers a different question (*"is this branch the
busy one"*), and in this geometry that question rarely arises: **there is no
fork on open ground.** So the triad was imported from open-2D prior art
without being re-derived for a side-view world; `PheroAAlong` *is* that
re-derivation, and the laterals are rightly kept because the import still
fits the one thing here that does move in open 2D — a flier.

**This also caveats the table above, and the caveat is load-bearing.** Those
branches are laid in **open air**, which no ant can walk. The measurement is
honest about the *input's* discriminating power — a real, answerable question
— and says nothing about how often the situation occurs. **Where forks
genuinely exist in this world is underground**, in the galleries the colony
digs, and over/under an obstacle. That is where a concentration reader would
first be worth measuring, and pointing `mode=junction` at a dug nest is the
follow-on nobody has run.

**Kept rather than removed**, for the laterals' reason plus one: a zero
weight is one mutation from existing (`MUT_ABS_FLOOR`), so a lineage for
which absolute concentration *is* worth something can evolve the connection.
Authoring one now pre-judges that.

---

## 3c. The planes widened to `u16`, and that is the fix §3b said was a trade

**Owner, 2026-09-15, on the recommendation below: *"More memory is fine as
long as speed is unchanged."*** So the acceptance condition was speed, and it
is measured rather than argued.

### Why a width and not a constant

§3b measured the whole candidate space and found no combination of `DEPOSIT`,
`DIFFUSE` and decay cadence that reaches a round trip, then called it a trade.
That was right about the arms and wrong about the reason. **The binding
constraint is the `u8` dynamic range**, and it binds because channel A is a
*ramp*: the odometer that lays it falls off with distance from the nest, so
the far end of a trail is the faint end, and at a byte that end runs out of
bits. Measured on the shipped ramp, the ant's own along-reading:

| distance along trail | peak 120 | **peak 40 (shipped)** | peak 8 |
|---|---|---|---|
| 0.1 | −0.098 | −0.098 | +0.000 |
| 0.5 | −0.043 | −0.062 | −0.250 |
| **0.7** | −0.028 | **+0.000** | +0.000 |
| **0.9** | −0.033 | **+0.000** | +0.000 |

**The trail went flat past its own midpoint.** An ant more than halfway out
read *exactly zero* — the trail was still there and had stopped pointing
anywhere. That single mechanism is also §1c's "stops steering while 77 cells
are still standing" and §1's recruitment failure; they were one defect seen
three ways.

**`DEPOSIT` could not buy it**, which is why the type moved instead: the
busiest trails already peak at 39–98 of 255 in a real bed, so tripling the
deposit to fix the faint end clips the loud end into saturation and flattens
the differential reinforcement P-14 exists to protect. **The scale-free reader
is scale-free in *ratio*, not in *resolution*.**

### What it cost, which was the condition

`Scent = u16` as 8.8 fixed point: the same 0..255 *semantic* range with eight
fractional bits under it, so every constant keeps its meaning and only the
resolution moves. Measured with `examples/pherocost`, **written to compile
against both widths** so the two binaries differ only by the library, with
identical tile counts in every row:

| world | cells | `u8` | `u16` | ratio |
|---|---|---|---|---|
| 512x320 | 164 K | 1,261 ms | 1,075 ms | **0.85x** |
| 2048x1024 | 2.1 M | 4,178 ms | 3,839 ms | **0.92x** |
| 4096x2048 | 8.4 M | 4,705 ms | 4,322 ms | **0.92x** |

**Faster at every size, including far out of cache**, and the margin narrows
with world size exactly as a memory cost should. The win is that the decay
**table became arithmetic**: at `u8` a 256-byte LUT was free in L1, which is
why it was a table; at `u16` the same shape is **128 KB**, out of L1 and into
the middle of the plane's own working set. A multiply and a shift touch no
memory at all. The table was an optimisation for a width that no longer
applies.

**`world=` is the load-bearing argument** and the reason the small number is
not the answer: at 512x320 a plane is cache-resident and a width change is
pure arithmetic. `CLAUDE.md` — the current world is a test environment, not
the target.

### What it bought

Same constants, nothing tuned:

| | `u8` | `u16` |
|---|---|---|
| unreinforced trail gone | 144 frames (0.07x) | **1,476 frames (0.67x)** |
| stops steering | 36 frames (0.02x) | **1,080 frames (0.49x)** |
| standing network, 52 ants | 208–342 cells | **1,405–2,057 cells** |
| peak, real bed | 39–98 of 255 (15–38%) | 15,160–25,004 of 65,535 (23–38%) |

**Ten times the life, thirty times the steering range, a six-fold network, and
no saturation** — from resolution alone.

### Three things this corrects

- **§1d's "a cell needs re-laying every ≤36 frames"** was measured *mid-ramp*,
  at the trail's faintest point. Measured with a uniform deposit there is **no
  traffic cliff at all**: a trail settles at a nonzero value at any spacing,
  even one pass per 480 frames. The cliff is in **deposit height**.
- **§2d's "two cells away reads zero, ever"** was the *byte's* way of saying
  inaudible. Widened, the old diffuse arm reads **100 of 65,535** two cells
  out — 0.15% of scale, **+0.003** into `Attack` against a weight of 2.0. Still
  inaudible; no longer zero. The active-space change still earns its keep by
  **226x** there, and its guard now asserts that ratio rather than a zero.
- **The tile-seam guard's `assert_eq!` was passing on quantisation.** At `u8`
  the two sides of a seam were bit-equal at every distance; widened they are
  57887/57894, 30861/30880, 12542/12559 — and the byte was rounding all of it
  to 226/226, 120/120, 49/49. The residual is float rounding rather than a
  seam bias, checked rather than assumed: the absolute difference *shrinks*
  with distance (19 at d=1 to 0 by d=6) and never exceeds one quantum in the
  tail, where a real bias would grow. The guard now allows 1% and still
  catches an injected seam block by **15x**.

**A constant the widening would have changed silently, caught and scaled**:
`PheroAAlong`'s divide-by-zero guard is `+ 1.0` in *value* units, so at the new
width a literal 1 would have become 256 times weaker without a word written.
One faint cell against an empty one reads 0.500 at the old width and 0.996 at
the new — not more resolution, a different input. It is `+ SCALE` now, which
confines the change to what happens *below* one old unit, which is the point.

---

## 4. What would overturn this

- §1 is measured at `PHEROMONE_INTERVAL = 12` unscaled. `World::step_
  pheromones` scales it by the creature clock, which keeps passes-per-tick
  exact — so the frame numbers move with the clock and the *ratios* do not.
  A change that broke that coupling would invalidate every frame count here.
- §1e is one bed shape (512x320, 8 founders, 1 colony, 1 compartment) at six
  seeds. The owner's bed is 1000+ long ants and nobody here can build it; a
  denser colony crosses each cell more often and the §1d threshold is a rate,
  so a big enough colony would hold trails this one cannot.
- §2 is the **authored** genomes. Mutation can connect any slot in §2a at any
  birth, so an evolved population is not bound by that table — which is the
  argument for keeping the slots, and is not an argument that anything
  shipped uses them.
