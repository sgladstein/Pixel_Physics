<!-- Synthesis of the 2026-09-28 home-holding workflow (3 tracers, 35 skeptics). Scratch paths inside it refer to the session container. Report: Reports/ant-scenes-2026-09-23.md §22n. -->
# SYNTHESIS: why ants at home hold food (colony bed, pile 90 cells east; 24 paired seeds)

Arms from one binary: hmoff = both switches off, hmon = PR #504 defaults. Parses are keyed by (seed, ant): 495 off / 538 on.
Home-holding (HH) = zone home (|dx|<=26) with crop fill > 0 after act, weighted by frame gap: 26.3% -> 36.6% of ant-time
(1,948,297 / 3,017,112 frames; +9.6 pp per seed, 24/24). Fed = body energy >= 200 J (the Energy input clamps at 1).
Store origin = the crop's first fill rise since it was last empty happened in the home zone. Synthesizer tables are
S/synth/{budget,cross,logbudget,fate,held}.txt and FINDINGS.md, S = /tmp/claude-0/-home-user-Pixel-Physics/1289a7cd-b802-5816-86ed-d6248c445388/scratchpad.

## 1. The answer

At home the crop is a spoon, not a suitcase. An ant beside the store picks up a cell, digests it while holding it, and puts the rest back.
1. Fed ants sipping the store: 66.8% -> 75.8% of HH; 18.5% -> 28.0% of ant-time per seed (23/24, p=3e-6). This is the whole rise.
2. Hungry ants (<200 J) sipping the store: 25.3% -> 18.2% of HH; flat per seed (6.2% -> 6.8% of ant-time, 14/24).
3. Hungry foragers eating their own pile load at home (by design, s13d): 5.8% -> 3.2%; this falls (on higher in 6/24, p=0.023).
4. Fed foragers holding a pile load at home, the case the question pictures: 2.1% -> 2.7%; 0.52% -> 0.95% of ant-time (19/24, p=0.007).

How it works (confirmed in code and data):
- **Pickup ignores hunger.** Feed = squash(0.4 + 0.8·FoodAdjacent) (ant.ron:1484-1485): an empty ant beside food bites at 0.545 per tick
  in every energy band (creature.rs:10561). A laden ant at E=1 beside food bites on 0.28-0.29 of decisions and puts down on 0.27-0.29
  (+0.06 no_room). Per-decision rates are the same in both arms (15/24, 16/24).
- **Put-back.** 98% of deliveries at home are home-picked cells, put down at the pickup spot (median |dx| 0) a median 12 frames later.
  The crop holds about 2 cells (median fill 0.126). On: 8,699 fill units picked up at home, 8,858 put down.
- **Pinned in place.** Move = squash(2.0 - 1.75E - 1.16·FoodAdj + ...) is <= 0 for a fed ant beside food (ant.ron:1175, 1239).
  p_move = 0 on 98.9% / 98.5% of laden time at the nest beside food (46-50% of laden home time). Under 1% of exits are a step;
  42-44% come when the ant has taken the last food within reach.
- **The drive cannot win.** Its gate reads the crop after act (creature.rs:5543-5546), so a bite excludes the ant that same tick
  (0 of 19,901 home bites by sensed-empty ants felt it). After emptying, an ant re-bites before leaving 99 times in 100 in both arms.
- **Why it grew under #504.** The rule did not change; the state did. The colony is richer (ant-time at >=400 J 18.6% -> 36.6%, 23/24)
  and more of the nest floor is beside food (P(beside food | home) .273 -> .354, 21/24; per-seed rise tracks it, Spearman 0.78).
  Of the +9.6 pp, +7.3 is more holding at a given energy and +2.9 is the shift toward richer bands.

Refuted and dropped:
- "Sated ants hold because drop 0.521 < feed 0.545": at the store sated ants empty faster (0.105 vs 0.070 per row, 22/24), hold less.
- ant.ron's "P(drop) = 0.25 at E=1" is P(prefer drop) x drop_urge from feed_gate. The real put-down rate at the nest at e~1 is
  0.35-0.36 per decision (0.34-0.37 in every seed). Do not reason from the comment.
- "not_asked with fill > 0 = share/attack/feed chosen": attack and share never return. A laden not_asked row is always a swallow
  (80,928 of 80,928 on).
- Off-nest holding (at_nest=0, founders' drop_p = 0): 21.4% / 21.2% of HH, share flat (13/24), mostly transit (median 24 f; 72-76%
  ends on the nest). Only runs >= 300 f look stranded: 3.8% / 4.2% of HH.
- The carry-patience reset on a home pickup (creature.rs ~10837-10847) fires, with no measured effect (.74 vs .78, 16/24).

## 2. Is the time a loss?

Every tick the crop holds food, it digests 3.3 J of food value, whatever the hunger (digest_hunger_weight 0.0, ant.ron:526; measured
3.298). The body gets 0.2375 J per J of food value (fruit quality 0.25 x 0.95). As an independent check, this clock matches the logs'
FOOD BUDGET "chewed" to 0.3% (off 1.751M vs 1.749M J; on 2.324M vs 2.330M J).

| part of HH (off -> on) | ant-frames | food value eaten | body J | paired by seed |
|---|---|---|---|---|
| (a) hungry, <200 J: eating it needs | 605,947 -> 646,616 | 333k -> 356k J | 79k -> 84k | flat (14/24) |
| (b) fed, >=200 J: eating above the grant | 1,342,350 -> 2,370,496 | 738k -> 1,304k J | 175k -> 310k | 30.0k -> 55.7k J/seed, 23/24 |
| ...of (b), >=1,100 J (past the bud bar) | 1,800 -> 8,141 | 1k -> 4.5k J | ~0 | |
| (c) held and not eaten | 0 by construction | 0 | 0 | |

- **(c) is empty.** No held tick goes uneaten (139 / 152 zero-chew intervals of 191k / 290k are rounding edges). A put-back returns
  the undigested rest (organism.rs unit_cell), so the cycle destroys no food. The nearest idle holding costs the ant time, not food:
  a pile load waiting at home (154,605 -> 179,949 frames, 81% later delivered), and off-nest runs >= 300 f (74,312 -> 125,281, 17/24, p=0.064).
- **Colony budget** (logs, food value): taken from the pile 2.121M -> 2.804M J (+684k); chewed 1.749M -> 2.330M J (+581k). Chewed as a
  share of taken holds at 0.825 -> 0.831 (per-seed median 0.819 -> 0.834). Of the extra +581k, +565k is (b). Standing at end 308k -> 414k J.
- **Where (b)'s body energy went.** Births 15 -> 58 at ~1,040 J each (15.6k -> 60.3k J; parents' bodies paid 6.1k -> 25.6k, the rest
  floor food at value x 0.25); all 56 matched parents were at home, 37 in the top quartile of HH time. Alive at end: 71k -> 133k J in
  bodies. 46 -> 52 ants died with >= 200 J, 17k -> 27k J in all (cause not traced).
- **Verdict: (b) is the colony eating and its route to births, not time taken from trips.** Pile pickups are up in 22/24 seeds. HH
  quartiles make 8.4 / 8.4 / 9.5 / 7.6 pile pickups. Visits that end in a departure are no longer (13/24) while bites per visit rose
  34 -> 48 (21/24). Every eater and every bud pays the same 0.24 conversion (bud provision counts floor food at value x 0.25,
  creature.rs:8554-8592), so moving (b) from bodies to the floor changes who can use it and when, not its worth. The only loss with a
  number: <= 27k body J died with fed ants (on), ~9% of (b)'s body income.
- **Not established:** trips fed ants would make if they did not sip; why 52 fed ants died.

## 3. Levers

Ranked. Dead ends checked by grepping Reports/dead-ends.md for keep, Feed, Drop, digest, share, store, provision, forage drive.

**L0. Leave it (null). Supported.** HH stays ~36.6%. Every colony outcome the switches moved is a gain (born 15 -> 58, starved
230 -> 174, food standing +35%, body energy +72%). The eaten share of intake did not change and trips were not displaced, so under
"default on unless a measured harm" nothing here is a harm. Cost: the Move brake caps the drive's reach.

**L4. Packed lunch: the drive sees through a crop that holds only store food.**
- Mechanism: flag the Crop at a home pickup (picked_at_home, creature.rs ~10586). Exempt a flagged crop from the drive gate
  (5543-5546), home_pull (13531) and scouting's !laden gate (13668). The drive's +1.75E lift turns the store Move sum to ~+0.84
  (p_move ~0.46).
- Effect: (b) holding time becomes trips while the ant keeps eating, since digestion ignores location. HH down, pile intake and loops
  up; births ~unchanged if ants eat on the road (sign uncertain). Pool: at most ~22% of ant-time on. Gain not predictable from traces.
- Risks: laden ants are heavier and slower (carried_cells; fill lowers P(move)). FoodAdjacent +0.8 on Dig sends fed ants to haul
  spoil, which the drive excludes (',keep' trace: 61% of decisions hauling dirt at step chance 0). Food can be lost in crops at death.
- Dead ends: not tried; ',keep' (1195) and ',fed' (1198) are neighbours, not this. The only lever aimed at trips that keeps eating; most code.

**L1 + L2. Granary: slower digestion for fed ants, and buds that draw on the store.**
- L1: digest_hunger_weight 0 -> w (ant.ron:526); the ramp at creature.rs:5936-5942 multiplies the rate by (1 - w + w·(1100 - E)/900).
  - Effect: (b) becomes standing floor food. At first order w=1 withholds 393k J on (23.7% of HH eating, 16.9% of all; 16.4k J per
    seed against a median 6,166 J standing). HH time flat or up, since the pickup loop ignores digestion.
  - Risk: births fall. At w=1 digestion alone never reaches 1,100 J; parents sit at a median 854 J; ',keep' cut births 67 -> 9 by
    stopping the same eating. Starved flat (nothing changes below 200 J).
  - Dead end 1184 (shipped OFF 2026-09-18; inert on re-tests 09-24 and 09-25 because carriers were hungry). Its re-test condition,
    "a loop pays the forager more than it needs", is now met: 78% of HH and 64% of ant-time are at >= 200 J. The `hungergate=` rider exists.
- L2: widen provisions_in_reach (creature.rs:8554, the head's 8 neighbours) to the store within radius r when at the nest.
  - Effect: store food goes straight into births, bypassing a body (5% better per J; the gain is timing). 45 of 56 parents already
    top up a median 325 J from 8 cells. Births up, store down, (b) time slightly down.
  - Risk: buds strip food hungry ants eat, so starved rises (the lesson of 1185: the store cycle is how the colony eats).
  - Dead end: none on provision reach.
- Together they meet ',keep''s own re-test condition, "a use for stored food other than its foragers' bodies". L1 alone is predicted
  to be a measured harm.

**L3. A satiety input on Drop.**
- Mechanism: clamp((ej - 200)/900) wired to Drop (sense at creature.rs:6585; ant.ron:1523-1526). Energy's clamp stays (Move reads it).
- Effect: rich ants put down faster with bites unchanged, so fed stretches and fed eating shrink together. It is L1's store-for-births
  trade reached through time. The freed time goes to standing empty at home, not trips (~1% leave first).
- Dead end: the Feed mirror (1185) was rejected twice for starving hungry ants; a term that is 0 below 200 J does not touch them.
  Weaker than L1: same trade, more brain changes.

## 4. What would settle it (L0 vs L4)

Does laden time at the store delay a fed forager's next trip, or fill time it would spend at home anyway?
- **Trace:** one row per home visit of every fed forager (>= 200 J, has been to the pile): frames empty vs laden, bites, and the
  departing tick's inputs (fill, food_adj, drive, p_move, E). Fit visit length = empty ticks / h + laden ticks, with h the per-tick
  departure hazard on empty ticks.
- **Reading:** h constant and visit length flat in laden ticks means sipping is filler and L4 buys nothing (L0). Visit length rising
  one-for-one with laden ticks at fixed h means laden time is a direct delay, and L4 has up to ~22% of ant-time to convert.
- **So far this leans L0,** from a cross-arm comparison that also varies the drive: visit length flat (13/24) while held frames per
  visit rose 831 -> 1,126 (20/24).
- **If L4 is built:** a paired 24-seed arm from one binary, read per seed for pile intake, loops per forager, births, starved, and the
  FOOD BUDGET eating share.
