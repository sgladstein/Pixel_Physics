# What to change, ranked by impact — the ant-survey review and its three lanes, read together

*Written 2026-09-19 by the coordinator of the ant-survey follow-up round,
at the owner's request, from
[`ant-sim-research-review-2026-09-19.md`](ant-sim-research-review-2026-09-19.md)
(the outside survey read against the engine),
[`ant-survey-round-2026-09-19.md`](ant-survey-round-2026-09-19.md) (what the
lanes overturned), and the three lane reports:
[`nest-rejections-rescored-2026-09-19.md`](nest-rejections-rescored-2026-09-19.md),
[`ant-field-wake-2026-09-19.md`](ant-field-wake-2026-09-19.md) with
[`gpu-field-design-2026-09-19.md`](gpu-field-design-2026-09-19.md), and
`Reports/ant-survey-trail-reevaluation-2026-09-19.md`, which is on PR #478's
branch until it lands. Every number below is one of theirs; nothing here was
re-measured. Ranked by what it would change on screen against what it costs,
which is the bar `CLAUDE.md` sets. `engine`/`lab`.*

## 0. The answer, stated once

**Of everything the survey recommended and the round re-tested, five changes
are worth making, and the two biggest were already in the tree switched off.**
The survey's headline mechanisms — a food-graded trail, negative feedback on
the emitter, a no-entry mark — all came back inert or unbuildable on a bed
where they could finally be tested. What moved instead were a decay rate the
register had rejected on the wrong plane, and a nest reach dial nobody had
swept in the right axis.

| rank | change | what it does in the world | the number | cost | who decides |
|---|---|---|---|---|---|
| 1 | **Channel B fades faster** — expose the food trail's decay as a dial, range 0.03–0.25 | the colony eats more of a patchy bed and more of it survives | intake median **54,722 → 127,339 J**, better on 10 seeds of 12; alive at 149,400 frames **2.5 → 10.5**; unvisited larder lower on 11 of 12 | the setter exists (`set_channel_rho`); a lab dial | owner: expose, or also change the default |
| 2 | **The nest reaches down** — `NEST_SITE_ROWS=40` shipped as a value | shafts under the chamber instead of a flat scrape | room **1.44x, bigger on 12 of 12**; height over width **0.11 → 0.20**, taller on 11 of 12 | none; cheaper than the code it replaces | owner: blind card `20260919T151821144Z-da82d9` |
| 3 | **Trust the de-saturated food reader** (shipped 09-18) and discard every trail number taken before it | ants hear a laid trail | **22 colonies of 36 alive against 4** with the old pair; 52% of ants at the food against 5% | done; record corrected on #478 | nobody — land #478's write-backs |
| 4 | **Turn the home bearing on** — `home_bias` above 0.0, with a dial | a laden ant walks toward where it came from, not only up the scent | none yet; built at `da4a461a`, never measured | a species-file field and one sweep | owner, under *ship everything on* |
| 5 | **Fix `trailfollow`'s default gate** — it is the stale `saturated` preset | every archived run without `gate=` is a three-change comparison | — | one line | any lane |

**And the one thing that binds, which none of the five touches:** the colony
cannot lay a trail worth following, and it pays to lay one. Ants that lay no
food trail keep **29 colonies alive against 22** and close **107 round trips
against 65**, and a self-laid trail recruits nobody in every configuration
tried. The survey's core recommendation presupposes the opposite. That is §2.

On performance the answer is short: the field is the **third** phase of a lab
tick, not the first, no wake rule can shrink it, and the GPU field's condition
is not met by 1.6x for a reason that will not change with the share. The
change worth making there is a measurement on the owner's own bed (§3).

## 1. The five changes

### 1.1 Channel B fades faster

**What it does.** A food trail that fades in a fifth of a round trip stops
pulling the colony back to a patch it has already eaten. On the shipped lab
box — patchy herbs, trophallaxis on, the food reader at full gain, channel A
at 0 — over 150,000 frames and twelve seeds paired within seed:

| channel B rho | alive at 149,400 (median) | intake (median) | unvisited larder (median) |
|---|---|---|---|
| 0.03, shipped | 2.5 | 54,722 J | 156 |
| 0.10 | 8.5 | 94,501 J | 146 |
| 0.25 | **10.5** (8/4/0) | **127,339 J** (10/2/0) | **106** (lower on 11 of 12) |

**Why the register said no.** The `DECAY_RHO` rejection was measured on `u8`
planes at interval 4 with both channels moved at once, on a scene where one
deposit is never re-laid. On that scene a faster fade "kills trails before
ants can use them", which is true of a trail nobody re-lays and not of one a
colony walks. On the single-pile gap bed with a hand-laid trail the band is
inert (25 and 24 colonies of 36 against 22). It is patch-dependent, which is
exactly what the survey says an ephemeral-resource decay should be.

**The cost, and the risk.** `Pheromones::set_channel_rho` already exists;
the work is a lab dial beside the diffusion one PR #432 added. The risk is the
one the lane named: 8 of 12 on survival and one seed the other way on reach
is what a real effect looks like here, not a tidy one. Under *expose, not
tune* this is a dial's range, and whether the default moves is the owner's.

### 1.2 The nest reaches down

**What it does.** `PIXEL_PHYSICS_NEST_SITE_ROWS` sets how far *down* an ant
still counts as at home. At 40 rows, over twelve seeds of the bare dig box:

| | control | site reach 40 | paired |
|---|---|---|---|
| room total | 699 | 952 | **1.44x, bigger on 12 of 12** |
| room depth | 15 rows | 30 rows | |
| height over width | 0.11 | **0.20** | **taller on 11 of 12** |
| middle-half width | 31 cols | 29 cols | narrower on 9 of 12 |

Deeper, bigger and slightly narrower at once — the first lever in the nest
line to move the aspect ratio at all, and the picture shows shafts descending
from the chamber where the control has a flat scrape. The owner's verdict on
the shipped control that morning was *"looks like nothing. a hole floating
spoil"*.

**Why nobody had it.** The dial shipped default-off as a measurement switch,
and only its sibling `_COLS` was ever scored — that one is the negative the
shape report published. It also refutes, in its literal form, the plan's
claim that an intervention on *whether* to dig cannot produce a shape: a
scalar cannot, but one applied over a region inherits the region's.

**The cost.** None. `adjacent_nest`'s site branch is cheaper than the eight
neighbour reads it replaces. **Do not stack `DIG_DOWN` on it unasked**: the
pair gives the biggest room in the line (1.71x) but the spread goes the wrong
way, so they are two different nests and which is wanted is a look, not a
number. The blind card is the first half of that decision.

### 1.3 Trust the de-saturated reader, and the trail numbers that predate it

**What it does.** On 2026-09-18 (`ac02ac03`) the food-trail reader's gate went
from `Bias 45 / CarryingFood −75` to `0.5 / −45.5`. Lane T measured that
change as **the whole of the hand-trail effect**: 22 colonies of 36 alive
against 4 with the old pair, 52% of ants at the food against 5%, and it wins
the register's own patchy-larder race 10 seeds of 12. The 2026-09-09
rejection that had called the full-gain reader "decisively worse" was taken
on `u8` planes whose far half read 0.000 — a reader that could only see the
near end of a trail ate its own doorstep.

**What to change.** Nothing in code. Three things in the record were still
saying *deaf on purpose* — the dead-ends entry, two paragraphs of the wiki,
the harness note — and the review quoted them the next day. PR #478 writes
all three back. Separately: **every `self` trail number taken before the
18th is a measurement of a deaf animal**, and the register carries several.

### 1.4 Turn the home bearing on

**What it does.** The survey's path integration — a running home vector per
ant — is in the tree: a fill-weighted homeward re-roll in `tumble`, gated by
`CreatureDef::home_bias`, which no species authors, so it ships at 0.0 with
no dial, no wiki line and no landed measurement. It is the one survey
mechanism that is built, plausible, and has never been run.

**What to expect.** The lifetime split found the same bill from the plane
side — trips up, intake down on a hungry bed, until a delivery is worth
something — and the two are the same verb approached from two sides, so they
must not land together. The number to have first is a six-seed sweep on
`trailfollow mode=gap`, scored on `carry@nest` and `trips`, paired against
the split. **Cost:** a species-file field and one sweep.

### 1.5 Fix the harness default

`trailfollow`'s default `gate=` is the stale `saturated` preset, which
saturates the homing pair as well as the food pair. Every archived run that
omitted `gate=` compared three changes at once. One line, no decision, and it
protects every future trail measurement.

## 2. The problem none of the five solves

**The colony cannot lay a trail worth following, and laying one costs it.**
On the return-leg bed at 36 seeds:

| arm | colonies alive / 36 | round trips |
|---|---|---|
| ants lay channel B as shipped | 22 | 65 |
| ants lay none (`carryb=0`) | **29** | **107** |

`self` ≡ `mute` on recruitment in every configuration: a trail the colony lays
itself recruits nobody, and the moves spent laying it are the difference
between the rows. A `(Crowding, EmitB, −4.5)` result that read as negative
feedback (31 colonies) is the same effect — emission cost falling — shown by
that control. Every candidate the survey ranks highest — the food-graded
odometer, the crowding-damped emitter, the no-entry mark — presupposes a
self-laid trail that recruits, and each came back inert *because* there is
none: the odometer fires (route peak 13.5–17.5 against 0 muted) and moves
nothing; the no-entry mark was declined because there is no recruitment to a
stale patch for it to correct.

This is the highest-upside item in the whole survey and no lane in this round
was scoped to build it. It is the trail session's line
(`claude/upbeat-shannon-cez0w4`, the owner's own), which is where the return
leg was fixed on the 18th. Two things this round hands it: the lifetime split
(`TRAIL_A_RHO 0`, the way home keeps) is the only bed with a return leg and
every headline above was taken on it; and laden right-of-way — one predicate
on `try_swap_with_kin`, diff at `8071d002` on the trail-lane branch — is
scored and unbuilt, with the 2026-09-16 rejection shown to have been measured
where no laden ant moved homeward.

## 3. Performance — what to change, and what the round rules out

**The measurement, first.** The per-phase stopwatch is built and free when
off (`PIXEL_PHYSICS_PHASE_CLOCK=1`, in the chronicle CENSUS row beside
`awake_chunks`). On the shipped lab bed at 52 ants it reads **field 20–21%,
`ca_sweep` 56–60%** of a tick. The one number that decides anything below is
the same row on the owner's own bed with a colony of thousands, and only he
can take it.

**What the round rules out**, each with its reason:

- **A wake rule cannot shrink the field's solve set on this bed.** The lab
  bed already solves ~65 of 128 tiles for plants and sky, and every chunk an
  ant stands in is inside the one-tile halo of one of them: **65.4 tiles a
  tick with the skip on, 65.4 with it off.** What ants add is CA block
  rescans (45.9 → 63.3 a tick at two hundred ants), 85% of them earned by
  digging, dropping and disturbing soil water. The bit-identical half ships
  as `FIELD_CREATURE_WAKE=blocked` and buys ~0.002 ms; the other half is
  not bit-identical, because the momentum passes are wind the player can
  see. Do not flip `blocked` to default on this evidence.
- **The GPU field's condition is not met by ~1.6x, and the share was never
  the deciding number.** The ants, plants, fire and renderer read the field
  on the CPU in the same tick, so a GPU solve buys a readback every frame;
  and the field's real optimisation is the sleeping-tile economy, which a
  per-pixel device cannot express — a shader over the whole grid solves 128
  tiles where the CPU solves 65. The case would have to be made on a world
  where the awake fraction is high and the readback can be deferred, and
  M10 streaming is the only roadmap item that changes either. `PLAN.md`'s
  decision row stands.
- **The review's own 2.09x was 1.44x.** Re-measured on the same box the same
  day: field 0.081 → 0.117 ms for 52 ants, not 0.090 → 0.188. Same direction,
  and the review's "27.9–31.1 awake chunks at three populations" was one
  population wearing three labels, because the stocking loop saturates.

**What is worth doing:** the parallel-creature switch on the owner's machine
(upstream of the 56–60% sweep, which is three times the field); one run with
the digging verb off to attribute the residual block rescans; and a decision
on **§Z31**, a real one-frame occupancy lag in the field's carry decision
with a reproduction and no owner.

## 4. Not to chase, with the number

| candidate | verdict | why |
|---|---|---|
| food-charged `EmitB` odometer | inert at both reader gains, 36 seeds, third test | fires and recruits nobody; the first two tests each varied one factor |
| `(Crowding, EmitB, −w)` | null at 2.35; the 4.5 "gain" is emission cost | the no-channel-B control reads 29 colonies |
| no-entry mark | not built | `self` ≡ `mute`: nothing to correct |
| trophallaxis off | stands, on | delays the founding cliff ~3,000 frames and steepens it; 12/0 better at 3,600, 3/9 at 9,000 |
| Khuong's pellet-attracted deposition | declined before building | one drop in ninety has a marked and an unmarked candidate; `line_burrow` relabels two thirds of the mound |
| `LightHere` spoil-drop gate | rejected harder | 0.49x room, better on 0 of 12, on the per-cell sensor its own entry asked for |
| the three `Crowding → Dig` interventions | coin flips (5, 6, 5 of 12) | the local reading desaturates the input and the nest does not move |
| curvature on the dig (Stage 3) | sign survives, size shrinks | 1.31x on `room total`, not 2.3x on `roofed`; the negative wire makes the nest wider on 10 of 12 |
| a digging pheromone | stands rejected | and the reason should be the engine's rather than the literature's |
| the field wake skip as an optimisation | ships as documentation | removes zero tiles; see §3 |

## 5. What the owner holds

1. **The lifetime split**: through PR #478, which carries the trail session's
   commits and is a snapshot of live work, or through that session's own PR
   with #478 shrunk to Lane T's docs. The conflict resolution is validated on
   `claude/ant-survey-trail-mainmerge`.
2. **The nest card** — and with it whether `NEST_SITE_ROWS=40` ships, alone
   or beside `DIG_DOWN`.
3. **Channel B's fade**: a dial, or a dial and a new default.
4. **`home_bias`**: on, with a dial, or registered as a known absence.
5. **§Z31**: is a one-frame lag in a coarse ambient channel worth a gate.

## 6. Sources

The review, the round record and the three lane reports named at the top;
the lane notes `Reports/lanes/ant-survey-nest.md`,
`Reports/lanes/ant-survey-perf.md` and, on PR #478's branch,
`Reports/lanes/ant-survey-trail.md`. The raw rows behind Lane T's tables are
archived under `Reports/data/ant-survey-trail-2026-09-19/` on that branch.
