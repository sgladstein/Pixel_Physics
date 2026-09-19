# The trail line's rejections, re-evaluated — 2026-09-19

*Lane T of the ant-survey follow-up round
([`ant-survey-followup-brief-2026-09-19.md`](ant-survey-followup-brief-2026-09-19.md)),
session `session_018P1mVfE1HKA1uDesCHWw3Y`, coordinator
`session_01HTNLNphUPgpg5GqCwCQvmW`. The owner's instruction: **"Don't trust
past results."** This lane is review and test only — nothing in the engine
moves; the one code change is a harness rider. Every number here is from
`examples/trailfollow mode=gap` unless the table says otherwise, rebuilt in
the same command as the run, `RAYON_NUM_THREADS=1`, 36 seeds, paired within
seed.*

## 0. The answer, stated once

*(filled after the runs)*

## 1. The bed, and why it is this one

Branched from `claude/upbeat-shannon-cez0w4` at `a3190843` — the
trail-lifetime split, `TRAIL_A_RHO = 0` — rather than from `main`, and the
decision is not a compromise. On `main` the homing plane dies in 144 frames
and no laden ant nets homeward (+0.0002 cells per carrying tick); every
candidate here is a statement about a trail an ant walks *out* on and
*back* on, so a bed without a return leg cannot test any of them. The split
is two env-overridable constants (`PIXEL_PHYSICS_A_RHO`,
`PIXEL_PHYSICS_DEPOSIT_AT`, the second default-off), so `main`'s plane is one
rider (`arho=0.03`) away from any row below — running "on both" is a column,
not a second branch. The branch is 0 behind `main` and carries the instrument
fixes this lane needed (`carryb=` re-aimed at `CarryingFood`; `arho=`/`brho=`).

The scene is the odometer entry's own: gap 90, 20 ants, `food=200
refill=2000 stop=6000`, 24,000 frames, `onlyfood`. `gate=shipped` is passed
explicitly on every row because the harness's default preset is
`saturated`, which the file itself calls stale.

Read `refill=` and `stop=` off every header before comparing to an archived
log: the 2026-09-17 odometer rows are `food=200 refill=2000 stop=6000` at
**`gate=b2`**; the lifetime branch's 36-seed rows are `food=400 refill=400
stop=6000` at `gate=shipped`. They are not the same bed.

## 2. What each original test could not see

Stated before the runs, from the record alone.

**Candidate 1, the food-charged `EmitB` odometer** (`dead-ends.md`, the
hidden-unit-7 entry). Three blind spots, two already written into the entry
by its own author: the 2026-09-17 rows were taken on a bed with no return
leg; the 2026-09-19 re-run scored `came back / reached food`, a homing
share, where the mechanism's claim is recruitment. The third is the one this
lane adds: **both runs raced the odometer against a reader that cannot
hear it.** The 09-17 rows are `gate=b2`, so the reader was at full gain and
the bed had no homing; the 09-19 rows are `gate=shipped`, so homing worked
and units 2/3 sat at a gate sum of +30, where §Z7 measured a laid trail
moving `P(move)` by 0.004. A trail strongest at the food is read by an empty
ant walking out; the shipped empty ant is deaf to channel B by design. So
the test that can see the odometer is the 2x2: shape × reader gain, on a
bed with a return leg. Neither factor has been varied with the other held
in its working state.

**Candidate 2, the full-gain food reader** (`dead-ends.md`, the units-2/3
re-gate entry). Measured 2026-09-09 on **`u8` planes**, where the far half
of every trail read an along-gradient of exactly 0.000. A reader at full
gain on a `u8` plane could only ever see the near end of a trail, and
"eats out its own doorstep" is what a reader that can only see the near end
does. The `plant=tree` / `plant=conifer` arms the entry cites as "neither
reversing" are the same `u8` runs. The entry's own re-test conditions — a
patchy larder, a faster channel-B fade — were named and never run on
`u16`. This lane adds the survey's deposit-side term, `(Crowding, EmitB,
−w)`, as an arm.

**Candidate 3, `DECAY_RHO` in the literature band** (`dead-ends.md`
`creature-direction §13d`). Measured on `u8`, at `PHEROMONE_INTERVAL 4`,
with the strict-decrease LUT, for **both planes at once**. The lifetime
branch has since found rho 0 right for the way home; the question left is
channel B alone, on `u16`, in round-trip units, and against a reader that
can hear the plane — at the shipped gate the food plane's decay can only act
on the hand-laid trail's persistence, never on a decision.

**Candidate 4, trophallaxis** (`wiki/ants.md`, *Feeding each other*). Three
runs of the harness colony dropped on seedlings, scored on survivors, which
the design report's own §9 says is the wrong readout. Not on a played bed,
not at session length, no order statistic.

**Candidate 5, pass-through kin** (`pheromone-trail-direction` §7.12). The
symmetric swap, measured 2026-09-16 on a bed where a laden ant's net
homeward drift was +0.0002 cells per carrying tick — so the case an
asymmetric right-of-way exists for, a laden homing ant stuck behind an empty
one, did not occur in the run that rejected it. The survey's rule
(Dussutour 2009) is right of way *to the laden*, one predicate on
`try_swap_with_kin`. **Not built here**: the trail lane owns that verb and
this lane tests. What the test would be is in §7.

## 3. Candidates 1 and 2 — shape × reader gain

`trailfollow mode=gap gaps=90 seeds=36 gate=… frames=24000 food=200
refill=2000 stop=6000`, `TRAIL_A_RHO 0`. `visitors` is distinct ants that
reached the food over distinct ants that lived — recruitment; `trips` is
nest → food → nest closed on the return; colonies is seeds with any ant
alive at 24,000. Sign tests are paired within seed against `base`, better /
worse / tied, on the median visitors share and on trips.

| config | reader | odometer | arm | colonies /36 | visitors, median | trips | vis sign | trips sign |
|---|---|---|---|---|---|---|---|---|
| `base` | shipped (full gain) | — | hand | **22** | **51.7%** | **65** | — | — |
| | | | self | 2 | 5.0% | 3 | | |
| | | | mute | 4 | 5.0% | 10 | | |
| `odo` | shipped | additive (`Carrying` wire kept) | hand | 22 | 46.8% | 49 | 15/20/1 | 14/16/6 |
| | | | self | 2 | 5.0% | 3 | 10/13/13 | 2/1/33 |
| `odoR` | shipped | replacing (`carryb=0`) | hand | 18 | 55.1% | 41 | 19/17/0 | 11/18/7 |
| | | | self | 4 | 5.0% | 2 | 13/11/12 | 1/1/34 |
| `foodsat` | **deaf** (pre-09-18 food pair) | — | hand | **4** | **5.0%** | **18** | **2/34/0** | 3/22/11 |
| | | | self | 4 | 5.0% | 5 | 5/5/26 | 2/0/34 |
| | | | mute | 3 | 5.0% | 10 | 0/2/34 | 0/0/36 |
| `foodsatodo` | deaf | additive | hand | 1 | 5.0% | 8 | 1/35/0 | 1/24/11 |
| `foodsatodoR` | deaf | replacing | hand | 5 | 5.0% | 23 | 1/35/0 | 3/25/8 |
| | | | self | 3 | 5.0% | 36 | 16/11/9 | 6/1/29 |
| `carryb0` | shipped | none — ants lay **no channel B** | hand | **29** | **58.3%** | **107** | 20/16/0 | **20/13/3** |
| | | | self | 4 | 5.0% | 10 | 12/10/14 | 5/1/30 |
| `mainplane` | shipped, `arho=0.03` (main's homing plane) | — | hand | 27 | 63.8% | 39 | 27/9/0 | 7/20/9 |
| | | | self | 1 | 5.0% | 0 | 5/11/20 | 0/2/34 |

Positive controls, both required before any row above is read:

- the instrument sees recruitment — `hand` 51.7% against `mute` 5.0%, sign
  34/2;
- the emitter fires — `route pk` on `self` (nothing of ours laid) reads a
  median 13.5–17.5 route cells with the odometer riders against 0 with
  `EmitB` muted, and the standing `B nest->food` profile on `self` changes
  shape under them (`[0, 2, 2, 32, 138]` shipped, `[0, 3, 3, 7, 6]`
  additive, `[24, 51, 82, 414, 863]` replacing).

**What it says.**

1. **The odometer is inert at both gains.** Recruitment sign 15/20, 19/17,
   13/11, 13/10; trips level or down. It fires, and it changes nothing the
   colony does — on the bed with a return leg the entry itself named as the
   re-test condition. Verdict unchanged from the branch's 09-19 re-run,
   now on the right quantity and with the reader varied.
2. **The reader's gain is the whole hand-arm effect.** The deaf food pair
   turns 22 colonies into 4 and 51.7% into 5.0%, sign 34 to 2. That
   reproduces §Z7's original *"exactly zero, twice"* — the null was the
   reader, exactly as §Z7 diagnosed — and it means every hand-arm number
   in the record since `ac02ac03` (2026-09-18) rests on the de-saturation,
   which `dead-ends.md` still lists as rejected. §4 has the write-back.
3. **The colony's own channel B is a net cost on this bed.** The best
   `hand` arm is `carryb=0`: ants that lay nothing keep 29 colonies alive
   against 22 and close 107 trips against 65. The emission is billed by
   `emit_cost_in_moves` and, since `self` ≡ `mute` in every configuration,
   buys nothing. This is the odometer entry's own cost explanation
   confirmed from the other side.
4. **`main`'s homing plane, for the record** (`arho=0.03`): more colonies
   alive (27) and more ants reaching food (63.8%), fewer round trips (39
   against 65, sign 7/20) and a quarter of the laden ticks at the nest.
   The lifetime split's own finding — trips up, intake down — reproduced
   on this bed.

## 4. Candidate 2 — negative feedback on the emitter, and the patchy larder

**First, the record.** Units 2/3 have shipped de-saturated since `ac02ac03`
(2026-09-18) — the owner's argument that an eaten patch stops being marked
because only laden ants mark — and neither `dead-ends.md`, `wiki/ants.md`
nor `trailfollow`'s landed-state note was told. §3 already shows what the
repair is worth on the gap bed (22 colonies against 4). The entry's own
race, on its own re-test condition (a patchy larder), on `u16`:

`creature_arena arm=… mirror=on seeds=12 frames=24000 plant=…`, arm A the
shipped ant, arm B as named. The headline is the seed count, not the
median; both arms' survivor counts are small (0–12 of 52 per arm at
24,000 frames), as they were in the 2026-09-09 race.

| bed | arm B | seeds B < 50% / > 50% / tied | B share, median | pooled A : B |
|---|---|---|---|---|
| conifer | position-only control (`mirror=off`, same genome) | 4 / 5 / 3 | 50.0% | 28 : 23 |
| **conifer** | **deaf food pair** (`hidden=` foodsat spec) | **10 / 1 / 1** | **28.1%** | **71 : 37** |
| tree | deaf food pair | 6 / 3 / 3 | 41.6% | 21 : 23 |
| conifer | `(Crowding, EmitB, −2.35)` | 4 / 5 / 3 | 50.0% | 46 : 50 |

The 2026-09-09 race read the re-gated reader at 25% against a
searching-at-random 75%. On `u16`, with the far half of the trail readable,
the same race reversed: the reader takes the larger share in ten seeds of
twelve on the conifer bed. The rejection was the plane's quantisation, not
the animal.

**The deposit-side negative feedback** (survey §3, Czaczkes 2013), derived
against the `Crowding` band: away from the nest `Crowding` is the 5x5
creature count over 8, capped at 1, and `EmitB` is `squash(2.5 − w·c)`
against the shipped flat `squash(2.5) = 0.714`. w = 2.35 gives the
literature's 5.6x cut at saturation; w = 4.5 gives it at half saturation.

| wire | arm | colonies /36 | visitors | trips | vis sign | trips sign |
|---|---|---|---|---|---|---|
| none (base) | hand | 22 | 51.7% | 65 | — | — |
| w = 2.35 | hand | 22 | 49.1% | 70 | 17/17/2 | 15/14/7 |
| w = 4.5 | hand | **31** | 56.8% | 49 | 20/15/1 | 11/13/12 |
| **no channel B at all** (`carryb=0`) | hand | **29** | 58.3% | **107** | 20/16/0 | 20/13/3 |
| w = 2.35 | self | 0 | 5.0% | 3 | 8/9/19 | 2/2/32 |
| w = 4.5 | self | 0 | 5.0% | 3 | 8/9/19 | 2/2/32 |

w = 2.35 is a null on every column, on the gap bed and in the arena. w = 4.5
looks like negative feedback working — nine more colonies alive — until the
no-emission control is read: ants that lay nothing keep 29 alive and close
107 trips. Emission is billed per unit laid (`emit_cost_in_moves`), the bed
is at subsistence, and a wire that lays less is a wire that spends less. The
`self` rows are the tell that the wire is not *feeding back* on anything:
identical in aggregate at both weights, because the colony's own trail
recruits nobody either way.

**The no-entry mark** (Robinson 2005) was not built. It is a repellent for
recruitment to an exhausted patch, and on every bed here the colony's own
trail recruits nobody (`self` ≡ `mute`); it has no substrate until that
changes, and the change is the discovery problem the record already names.



## 5. Candidate 3 — channel B's decay band on `u16`

`pherolife sweep=rho`, one deposit at frame 0, nothing re-lays it, against
its own 2,200-frame round trip:

| channel B rho | trail gone | stops steering |
|---|---|---|
| 0.03 (shipped) | 1,476 frames, 0.67 trips | 1,080 frames, 0.49 trips |
| 0.10 (band, low) | 612, 0.28 | 480, 0.22 |
| 0.25 (band, mid) | 288, 0.13 | 180, 0.08 |
| 0.50 (band, top) | 144, 0.07 | 108, 0.05 |

On `trailfollow`, same bed, shipped reader, channel A at 0:

| channel B rho | arm | colonies /36 | visitors | trips | vis sign | trips sign |
|---|---|---|---|---|---|---|
| 0.03 (shipped) | hand | 22 | 51.7% | 65 | — | — |
| 0.10 | hand | 25 | 50.9% | 76 | 22/14/0 | 15/11/10 |
| 0.25 | hand | 24 | 51.9% | 76 | 16/20/0 | 17/12/7 |
| 0.10 | self | 4 | 5.0% | 6 | 3/5/28 | 2/2/32 |
| 0.25 | self | 2 | 5.0% | 4 | 4/5/27 | 1/1/34 |

The band does not kill a trail the colony uses: a laid trail does its work
while it is being re-laid and the readers act then, and what it outlives
after release is worth nothing at gap 90 at any rate in the band. The
entry's *"kills trails before ants can use them"* holds for a trail nobody
re-lays, which is `pherolife`'s scene and not the colony's. The other half
of the question — does a faster fade stop a trail recruiting to an *eaten*
patch — is the units-2/3 entry's named re-test on the played bed:

*(labforage bdecay table)*

## 6. Candidate 4 — trophallaxis on the played bed

*(table)*

## 7. Candidate 5 — laden right-of-way, the test not run

**Not built, by instruction**: the trail lane is building the foraging loop
and this lane tests. The predicate was written, pushed at `8071d002` and
withdrawn at `a1d1730a` in the same hour, so the diff exists if the trail
lane wants it: one field on `CreatureDef` and two conditions in
`try_swap_with_kin` — the mover must be carrying (a crop with cells in it,
or a spoil pellet under `SPOIL_IS_CARGO`), the nestmate it displaces must
not be, and the symmetric `passes_through_kin` overrides it when set.

**What the 2026-09-16 rejection could not see.** The symmetric swap was
measured on a bed where a laden ant's net homeward drift was +0.0002 cells
per carrying tick — the ants were not going anywhere, so a rule about who
yields to whom on the way home had no traffic to act on. What it measured
was ants passing *each other* on the way out, which is the dispersal it
reported (intake −35%, "ants that can pass each other disperse instead of
following"). The asymmetric rule cannot produce that: an empty ant never
displaces anybody, so the outbound leg is bit-identical to the shipped
queue.

**The run that answers it**, on the bed above (`trailfollow mode=gap
gaps=90 seeds=36 gate=shipped frames=24000 food=200 refill=2000 stop=6000`,
`TRAIL_A_RHO 0`): three rows, shipped queue / `kinpass` / the laden rule,
arms `hand` and `self`, scored on `trips` and `carry@nest` paired within
seed, with `kin swaps` (the counter already printed per row) as the "did it
fire" half and `blocked` as the effect half. The prediction the record
makes: `kinpass` reproduces −35% intake and dispersal; the laden rule moves
`blocked` on laden ticks only, and if `trips` does not move with it the
queue is not what a homing ant is stuck on. The falsifier is `kin swaps`
at 0 on the laden arm — no laden ant ever met an empty one head-on, which
is a statement about traffic density on this bed, not about the rule.



## 8. Traps met on the way

- **`trailfollow`'s default gate is `saturated`, and it saturates the
  homing pair too.** Every row above passes `gate=shipped` explicitly. A run
  that omits it is a three-change comparison wearing the shipped animal's
  name.
- **`gate=b2` is a no-op against today's file** — since `ac02ac03` the food
  pair ships at b2 — and the harness says so by panicking (*"changed no
  slot"*), which is the only reason the first queue's b2 rows are missing
  rather than silently equal to base. The deaf reader needed its own preset
  (`foodsat`: homing pair as shipped, food pair as the pre-09-18 file).
- **A commit that overturns a dead end without writing it back leaves two
  records disagreeing.** `ac02ac03` de-saturated the reader on the owner's
  argument, changed `ant.ron` and one report section, and left the
  `dead-ends.md` entry, `wiki/ants.md` (two paragraphs) and `trailfollow`'s
  own `LANDED_NOTE` saying the food route is deaf on purpose. The review
  written the next day quoted them. Fixed here.
- **The `wire=` rider echoed nothing.** A wire nobody can see the value of
  is the megastudy trap; it prints the slot's old and new value now.
- **Two configs with byte-identical `self` summaries** (`crowd235`,
  `crowd450`) were not a stale binary: 8 of 36 seed rows differ. The
  aggregate was identical because the arm's colonies are dead by the end in
  both. Check the rows, not the summary, before calling a binary stale — and
  check the binary before believing the rows.
- **A survival gain from an emitter-side wire is a cost effect until the
  no-emission arm says otherwise.** `(Crowding, EmitB, −4.5)` read 31
  colonies against 22 and looked like negative feedback working;
  `carryb=0` reads 29 with no channel B at all. Emission is billed per unit
  laid, and laying less is cheaper.
- **`pkill -f` on a pattern that matches the calling shell kills the
  shell.** Twice in one session (`CLAUDE.md` names the same trap for the
  druid capture). `for p in $(pgrep -x cargo); do kill $p; done`.


