# Why selected colonies dig a crater: an independent review (2026-10-23)

> **Landed 2026-10-24** by the report sweep (thread *Land unlanded thread reports*): copied verbatim from the project shared folder `teacher-lane/review-crater-2026-10-23.md`. The picture, scripts, the body-rules patch and the common-garden CSVs are in [`crater-review-2026-10-23/`](crater-review-2026-10-23/). **Owner ruling that followed (2026-10-23 12:18Z): "No permanent body rule from the old ant!"** The old ant's roof and room refusals (`PP_BODYRULES`) may at most be a weaned training scaffold; the final ant is judged with them off.

Reviewer: a fresh session, not anchored on the lane's conclusions. I re-measured everything below here unless I name
a source. The picture is `review-crater-2026-10-23.png` beside this file. Scripts, the 32-line patch and the run
recipe are in `review-crater-2026-10-23/`.

## In plain words

**The crater is not something the ants learned. It is what their digging looks like when nothing protects the roof.**

- **The amount of digging is not the problem.** The selected ants cut as much as the old ant: 300-450 original-ground
  cuts near the door in 200k frames, against the old ant's 250-390. They differ in **where** they cut.
- **The old ant does not choose to spare its roof. Its body refuses to cut it.** The shipped ant's dig passes through
  `dig_veto`, which refuses every cut within 6 rows of the original surface outside the door's columns
  (`PIXEL_PHYSICS_DIG_ROOF=6`, on since 2026-10-02). It also passes a room-shaping refusal (`PIXEL_PHYSICS_SHAFT_ROOMS`,
  on since 2026-10-13, labelled "a stopgap"): one flat room under the roof, then flat galleries. **A mind ant's Dig
  skips both.** It goes from the option list straight to `cut_cell`. So the lane has been asking selection to discover
  two rules that the benchmark ant is simply given.
- **Without those rules, a mind ant cuts wherever it happens to be, and that is mostly the door shaft.** Its Dig fires
  next to any dug space. Its choice of which cell to cut is close to random. Ants climbing out take one more pellet
  from the shaft wall near the top, and over time the shaft widens into an open pit.
- **The pay rewards this.** A pellet cut in the top 2 rows is let go about 100-130 frames after the cut. One cut 10 or
  more rows down takes about 1,000 frames. The pay per cut is the same, so a shallow cut pays about 8x faster.
- **The test.** I took the same trained brains (colsel10 r47, the hcs founder, colsel11 r47), gave their digging the
  shipped ant's own refusals, and did not retrain them. In common gardens of 8 worlds each:
  - **the crater fell to 0 in every colony group;**
  - the roofed nest grew to 240-490 cells (old ant 356);
  - home food rose in 5 of 6 groups.

  With the room rule as well, the same brains dig the old ant's plan: a wide flat room under the roof, with galleries
  going down from its ends.
- **Selection under the rule keeps the roof and the food.** In a small selection run (16 colonies, 6 rounds), the arm
  with the roof rule kept crater at 2-7 against 111-229, nest 244-373 against 72-140, and food as good or better. But
  digging still erodes over rounds in **both** arms, by about 20%. That is the separate "digging dies" problem, and the
  rule does not solve it.

**Verdict on the lane's explanation: half right.**
- **Right:** a penalty on crater cuts cannot work. There is no inherited variation in *which cell* an ant cuts, so the
  cheapest answer to the penalty is to dig less. That is what colsel11 did.
- **Wrong:** "no under-roof digging". 81-86% of the selected ants' cuts are made with ground still overhead. What is
  missing is *roof-sparing*, and the old ant does not have that as a behaviour either. It has it as a rule.
- **Therefore:** no learned behaviour source is needed for a roofed nest. The fastest route is to give the mind ant the
  old ant's two refusals and select under them.

## 1. The mechanism, with evidence

Gardens use the lane's standard arguments (`cgnest.sh`: 200k frames, 24 ants, food 25, sel=off), worlds `k*1000+99`,
and medians. "Selected" means the round-47 carry of colsel10 cs10std colonies 11, 35, 44 and 45: typical crater
diggers, whose garden crater is 179-244.

**1a. Volume is not the difference.** Original-ground cuts within 20 columns, 4 seeds each:
- old ant 253-388;
- selected 305-516;
- hcs 199-254.

**1b. The old ant never cuts its roof; the selected ants cut it at full rate.** The table gives cuts per 10k
ant-frames, by the depth of the cutter's head. Positions come from `trace=10`; the script is `occ.py`.

| head depth below datum | above | 0-2 | 3-5 | 6-9 | 10-14 | 15+ |
|---|---|---|---|---|---|---|
| old ant | 0.00 | 0.49 | 0.26 | 2.63 | 2.78 | 0.92 |
| selected (4 colonies) | 0.05-0.06 | 1.0-2.3 | 1.2-4.1 | 2.0-3.9 | 2.3-3.6 | 3.8-5.8 |
| hcs founder | 0.17 | 1.93 | 1.39 | 0.82 | 1.71 | - |

All 44 of the old ant's cuts shallower than 6 rows, over 4 runs, lie in the door's own columns (-2..+2). That is
`under_roof` at work. `how-the-ant-works.md` documents it under `PIXEL_PHYSICS_DIG_ROOF` and `SHAFT_ROOMS`. In
`mind.rs`, `dig()` calls `cut_cell` directly, and the Dig push in `options.rs` checks only `diggable`.

**1c. The ants cut the crater themselves; it is not a collapse.** Where the final crater cells came from (`origin.py`):
- 77-83% were cut as original ground at that exact spot;
- 3-7% were re-cut;
- 12-20% were never cut: the stamp, or ground that fell.

Within crater columns (`order.py`), 41-64% of cells were cut *after* the column was already open to the sky. Those
cuts widen and deepen an open shaft. Another 8-29% were cut under ground and lost their roof later.

**1d. Who cuts the rim.** I traced individually all 37 cuts at depth 0-2 within 10 columns of the door in one
selected run (cs10_45 seed 1). Every cutter:
- was well fed, with empty jaws;
- had been at the nest 1-2 decisions before;
- climbed out above the surface with its pellet within 400 frames.

57% of them cut upward or sideways from inside the hole. **These are ants on their way out, taking one more pellet
from the shaft wall.** The old ant made 5 such cuts, all in its first 900 frames.

**1e. The brain cannot express "not here".** Four facts combine:
- Dig is founded as "dig at the nest" (`enter_dig`: `AT_NEST` +6), and `AT_NEST` is true next to any dug cell below
  the datum (`NestHome::Dug`). That covers the shaft and the rim.
- The target pick is proportional to (score+1)/2. The dig-scoring thread measured every Dig target at 0.97-0.99, so
  the pick is close to uniform.
- The depth sense is `softsign(depth/16)`: 0.11 at 2 rows, 0.30 at 7. A depth gate would need large weights found by
  chance.
- Mutation (sigma 1) on a saturated scorer moves almost nothing, while turning Dig down overall is a single weight.

**1f. The pay favours the rim** (same runs). I followed each pellet from its cut to the moment it was let go:

| cut depth | 0-2 | 3-5 | 6-9 | 10+ |
|---|---|---|---|---|
| selected: frames from cut to let go (median) | 95-130 | 300-490 | 440-490 | 1,005-1,080 |
| selected: let go 15+ columns out (earns the carry) | 18-19% | 5-6% | 4-5% | 1-3% |
| old ant: frames to let go | 80 | 100 | 205 | 350 |

So in-colony selection (`turn=30000`, ranked on per-ant credit) favours whoever digs nearest the exit. The colony
cases do not push back:
- `-crater` is one case in six.
- `nest` and `nestcuts_late` count the roofed overhang along a slanted crater as nest. That overhang is the cyan wedge
  in every colsel10 picture, and it exists only because of the crater. Nest and crater are correlated across colonies
  at r47 (+0.36).

**1g. What colsel11's penalty actually did.** I ran gardens of colsel11 r47 colonies 17 and 42 with `cut_roof=1`
logging:
- 85% of their remaining cuts are `unjoined`, unpaid shallow scrapes 10-20 columns out on the foraging track;
- their "crater" of 68-77 is mostly that scrape, not a hole at the door;
- paid nest cuts fell to about 45 per run.

The penalty removed the paid digging and left the unpaid background digging. That is the "dig less" answer from 1e.

## 2. A measurement bug that affects selection

`nestcount.py` floods from the door through empty cells and `?` only, so stored food (`crumbs`) blocking the shaft
cuts the whole nest off. Every open cell that is then not "nest" counts as crater, roofed or not. Example:
`v6_cs10_35_1` is a 16x12 roofed chamber full of stored food. It scores nest 4, crater 77; fixed, 165 and 0.

`nestfix.py` here fixes both parts. It floods through anything that is not soil, packed soil, spoil or stone, as
`cut_join` already does. And it counts a cell as crater only if it is open to the sky. Checked against both kinds of
case:
- **Known-fine (old ant, 32 garden maps in `cgn/`):** nest 302 -> 390, crater 4.5 -> 0. 9 of 32 had a false crater
  over 20 cells.
- **Known-broken (selected crater):** unchanged (cs10_45_1: 251/249 -> 253/248).
- **In-run maps misjudged by more than 20 cells:** colsel10 r47 22/64 and r1 16/64; colsel11 r47 22/64 (median nest
  38 -> 80).

Two other places carry the same problem:
- **`read_nestmap` (`inherit_nest=1`) uses the same flood.** A nest plugged by stored food is not carried into the
  next round, which is exactly the nest selection should keep.
- **`roomshape` reads a column of stored food standing in one room as two rooms.** With the refusal on, single
  chambers scored "rooms 47-85" for that reason. Check room scores against pictures.

## 3. The intervention test

Common garden, 8 worlds per cell, medians, fixed meter. The arms:
- **roof:** the shipped ant's `under_roof` (6 rows, door columns free);
- **both:** roof plus `shaft_room_refuses`;
- both are applied by filtering the mind's Dig options (the patch).

| brains | as trained / roof / both | home food | cuts | nest | crater | rooms |
|---|---|---|---|---|---|---|
| old ant (reference) | - | 642 | 278 | 356 | 0 | 20 |
| selected c44 | | 2,254 / 2,942 / 2,528 | 404 / 398 / 330 | 123 / 432 / 413 | 249 / 0 / 0 | 0 / 3 / 23 |
| selected c35 | | 1,729 / 2,966 / 2,748 | 376 / 372 / 305 | 143 / 406 / 374 | 213 / 0 / 0 | 0 / 2.5 / 12 |
| selected c11 | | 1,904 / 2,018 / 2,024 | 385 / 208 / 213 | 157 / 238 / 238 | 168 / 0 / 0 | 10.5 / 4.5 / 2 |
| selected c45 | | 1,672 / 1,242 / 1,108 | 426 / 455 / 328 | 197 / 494 / 379 | 222 / 0 / 4 | 0 / 1 / 11 |
| colsel11 c17 | (no "both") | 1,590 / 2,934 | 172 / 48 | 30 / 76 | 74 / 0 | 0 / 0 |
| hcs founder | | 312 / 502 / 538 | 205 / 144 / 150 | 16 / 168 / 174 | 106 / 0 / 0 | 0 / 2 / 0.5 |

Starvation was 0-1.5 of 24 in every cell. Paired by world:
- **Crater:** smaller in 8 of 8 worlds in every group.
- **Nest:** larger in 7-8 of 8.
- **Food:** higher in 5-8 of 8, except c45, where it fell in 7 of 8.

The lane's `PP_RIMVETO=3` gives the same picture, with smaller nests and more food (c44: food 4,111, nest 217).

**The refusal does not just move the digging out of view.** Shallow cuts beyond 20 columns are 0 under "roof",
because the shipped rule holds everywhere. Under RIMVETO they are 22-28, against 17-32 without a rule.

**Patch controls.**
- Unset: byte-identical `events.csv` against the original binary.
- Set: shallow cuts outside the door's columns 46 -> 0.

**Look at the picture.** With the roof rule alone, the brains dig one tall cave under the roof. With both rules they
dig the old ant's plan. hcs gets one roofed room, and its ground stays full of packed spoil from swimming.

**A small selection test (local, 16 colonies x 6 rounds x 200k).** I re-ran colsel10's exact mode and arguments
(lex+spoil+crater+inherit, cut_join=32, hcs settings). Each colony started from its own colsel10 r47 carry. One arm had
no rule; the other had the shipped roof rule on throughout, gardens included. Gardens were held after rounds 1, 3 and
5, on the lane's meter. Medians:

| after round | food (no rule / roof) | nest | crater | kept late cuts | cuts |
|---|---|---|---|---|---|
| 1 | 1,847 / 1,533 | 116 / 373 | 229 / 3 | 58 / 140 | 379 / 346 |
| 3 | 1,605 / 2,371 | 140 / 315 | 200 / 1.5 | 93 / 116 | 402 / 340 |
| 5 | 2,054 / 2,286 | 72 / 244 | 111 / 6.5 | 26 / 64 | 299 / 278 |

Paired by colony at round 5, the roof arm:
- **food** higher in 10, lower in 6;
- **nest** larger in 12, smaller in 3;
- **crater** smaller in 15, larger in 1.

Starvation stayed at 0-1 of 24. So **selection under the rule keeps the food and keeps the roof.** It does not stop
digging from eroding:
- cuts fell about 20% in both arms;
- with the rule, in-run cuts per round were 396, 212, 317, 286, 321, 245;
- the nest shrank round on round, 373 -> 244.

The rule fixes the *shape*. The *amount* is the separate dig-loss problem (`digloss/why-digging-dies`), and the next
run must watch it.

## 4. Recommendations, ranked

**1. Give mind ants the old ant's two dig refusals (roof, then rooms) as permanent body rules, and select under them.**
- **What:** the patch here, about 15 lines in `options.rs` that filter Dig targets through `under_roof` and
  `shaft_room_refuses`.
- **How to judge it:** with the rules **on**. veto-1022 selected with `PP_RIMVETO` on and judged with it off, which
  tests whether the brain absorbed the rule. This architecture cannot.
- **Cost:** the patch, plus one colsel run (64x48).
- **Risks:**
  - The owner may want the roof learned. The honest framing is that the benchmark ant has it given too.
  - There is no waiver for a shut-in ant. The shipped ant waives every refusal except Floor when escaping; add that
    waiver.
  - Food fell in one group (c45).
  - Digging still erodes (section 3), so pair this with the lane's dig-loss work.
  - SHAFT_ROOMS is a fixed box at the door. That is fine for the lab; the open world needs each colony's own site.

**2. Fix the meters before reading more selection results.**
- **What:** flood through stored food in `nestcount.py`, `read_nestmap` and `roomshape`; count crater only when open
  to the sky; stop a food pillar splitting a room.
- **Cost:** an hour.
- **Risk:** none, though baselines move (old ant nest 302 -> 390).

**3. Stop paying per pellet regardless of depth.** Under the rule, the cheapest legal haul is still the shallowest.
- **What:** pay dig credit at colony level on nest growth (fixed meter), or scale a cut's pay by its haul.
- **Cost:** small.
- **Risk:** used *instead of* a rule, every per-cut stick so far has been a dead end. Use it only alongside 1.

**4. Rooms from a sense, later, to retire the SHAFT_ROOMS stopgap.** The old ant's own documentation calls the room
box "the control a local cue must beat".
- **What:** the mind already reads crowding. `+shape` could not work while there was no roofed nest to shape. Under
  rule 1 room counts vary (0-63 within a group), so the case has something to select on.
- **Cost:** medium.
- **Risk:** new work; start only after 1 holds.

**5. The learned dig head (the lane's plan), only if the owner rules out body rules.**
- **Needs:**
  - a Dig target feature, "in the roof zone, or would open the sky";
  - a target pick that can be sharp: softmax with a temperature gene, not (s+1)/2;
  - a scorer that is not saturated.
- **Cost:** high.
- **Risk:** high. Four copies into shared weights have broken foraging (d1, d2, hd/hu, r3).
- **The old ant's cut-choice crutch (colsel13) cannot fix the crater alone.** It acts only in "dig states" (empty
  jaws, below the datum), while rim cuts are made by ants on their way out with their own pick. On hcs it took the
  crater only from 115 to 91.5.

**Dead ends, so nobody repeats them:**
- Per-cut penalties or zero pay for crater or rim cuts (rim_rows, cut_roof, roof_debit, zone_debit). With no
  inherited cell choice to act on, they cut digging instead.
- The `-crater` and `+shape` colony cases with no rule underneath.
- Copying old-ant cut labels into the shared scorer.
- Selecting with a refusal on and judging with it off.

## Caveats

- The selection test is small: 16 colonies, 6 rounds, the lane's (buggy) meter in both arms. The 64x48 run is the real
  test.
- The gardens show the rule turns today's digging into a roofed nest. Section 3 shows selection keeps that shape. It
  does not show digging volume holds.
- 6-row against 3-row roof under selection is untested. In the garden, 6 rows gave bigger nests and 3 rows gave more
  food.
