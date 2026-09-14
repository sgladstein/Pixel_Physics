# The pheromone planes: how long a trail lives, and what is wired to it

*Round 36, Lane C. The owner asked two questions on 2026-09-14 — **"do they
fade too fast to be useful?"** and **"make sure they are all wired up
properly"** — and this is the answer to both, with the instruments that
produced it.*

*Nothing here is tuned. Per the standing direction (*"give me the tools,
data, access to the parameters that need to be tweaked and I do that testing
myself in the game"*), every default ships unchanged and the one thing added
is a dial that did not exist.*

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
plane's writers fire (336 alarm deposits in a 9,000-frame bed, plane live).
Nothing is disconnected in the Rust. What the table shows is four findings in
the *authored* half.

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
