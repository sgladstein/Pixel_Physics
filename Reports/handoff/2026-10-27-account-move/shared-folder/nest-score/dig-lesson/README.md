# Dig lesson from the old ant (dig-scoring thread, 2026-10-22 evening)

The Teacher lane asked (20:04Z) for a lesson copying the old ant's underground dig, carry-spoil and drop choices onto
hA1, labelled on hA1's own states (DAgger), and checked at 32 seeds sel-off on the fixed nestcount / roomshape.

**Result in one line.** The labeller works. Both copies broke the ant: starved 21-23 of 24 against 3. Acting directly,
the old ant's cut choices alone give a bigger roofed nest and less crater, but no rooms. Its whole underground program
nearly fixes the nest (nest x5, crater halved, spoil kept out of the nest, first second rooms) but cuts home food by
two thirds.

## The labeller (`PP_OLDLAB`)

- Before a mind ant's tick in a dig state, the world is cloned. A dig state is: holding spoil, or empty, not hungry,
  below the datum, within 40 columns of the door. In the clone the ant loses its mind and takes one tick as the
  game's own ant (chooser + act).
- What the ant did in the clone is read off its body as option indices, the same mapping as the foraging lane's
  PP_SHADOW: a cut, a put-down, a pick-up, a step, a turn or a rest. That is the label, in the copy log's 16th field.
- **The real world is untouched.** events.csv is byte-identical with and without it (hA1, 20k frames).
- Cost: ~2.7x frame time with K=4.
- Patch: `oldlab-wshape-vs-529ff42a.patch`. It also carries w_shape, and everything is env-gated.
- Scripts: `dlog.sh` (logs), `filt.py` (keeps cut / no-cut / spoil-carry labels), `copier_d.py` (copier_c.py with
  those groups), `cg.sh`/`cg2.sh` (common garden: 200k, sel off, world s*1000+99), `summ.py`.

6 logs (hA1 acting, seeds 41-46, K=4) gave about 1,200 cut labels, 380 no-cut labels (the mind cut where the old ant
would not) and 475k spoil-carry labels. Where the old ant cuts in hA1's states: mostly down or diagonal-down, 2
columns from the door (median). hA1's own cuts are 4-5 columns out.

## 32 seeds, sel off, 200k, the fixed counters (medians; paired = seeds up/down vs hA1)

| arm | room score | colonies w/ 2nd room | nest | crater | kept late cuts | spoil let go inside | home food | starved /24 |
|---|---|---|---|---|---|---|---|---|
| hA1 | 0 | 0 | 19 | 110 | 5.5 | 140 | 279 | 3 |
| d1 copy (cut + no-cut + spoil carry) | 0 | 0 | 1.5 | 95 | 0 | 601 | 99 | 23 |
| d2 copy (cut + no-cut only) | 0 | 0 | 0 | 61 | 0 | 361 | 116 | 21 |
| old ant's cut calls acting (`PP_OLDLAB_ACT=1`) | 0 (4/2) | 0 | 36 (24/7) | 87 (4/28) | 6 | 108 | 255 (14/18) | 2.5 (14/15) |
| old ant's underground program acting (`=2`) | 0 (12/0) | 2 | **91 (31/1)** | **64 (2/30)** | **21 (28/4)** | **27 (0/32)** | **101 (1/31)** | 2 (5/19) |

- **The copies.** d1 fit the old ant's cuts 44% of the time (from 1%), but behaviour elsewhere moved 16%. d2 held
  elsewhere to 11%. Both collapsed. This is the third lane today whose teacher copy into hA1 broke the ant (foraging
  s4/d1, trail tA1/oA1/oA2). The far-from-home lane's copy worked only by training just the weights of a new sense.
- **Cut calls acting** is the ceiling for a perfect copy of "what to cut". It is a modest nest and crater fix at no
  food or starvation cost, and it makes no rooms. Rooms are not in the cut choice alone.
- **The underground program acting** moves every dig number a long way and starts rooms on 2 of 32 colonies. It costs
  two thirds of home food, without extra starvation. A likely reason: empty, fed ants underground do nest work as the
  old ant would and do not go out until hungry. That is untraced.

## What I'd do next (Teacher lane's call)

1. Don't copy this teacher into hA1 as it stands. Two copies failed the same way.
2. If the room route is still wanted, the old ant's underground program is a teacher worth keeping. The question is
   how to hand it over without breaking foraging. Possibilities: train only the dig-verb job and scorer weights
   (the far-home lane's trick), or use it as an acting crutch during selection. Either is a new run, not a tweak.
3. The food drop under `=2` should be traced before anything is built on it.

---

# Round 2 (Teacher lane's 22:16Z plan): trace the food drop, then a restricted copy onto hcs

**Result in one line.** The food drop under `=2` is the old ant's spoil carry jamming in a mind ant's body. With the
carry left to the brain (`=3`, the old ant acts only while the jaws are empty), the old ant's underground program on
**hcs** gives nest 13 -> 60.5 at no food or starvation cost. **None of the three restricted copies held**: each lost
the nest it had (nest 0-1.5) and let go 2.5-3x more spoil inside.

## 1. Why home food falls under `=2` (individual ants, hA1, seeds 1-2, trace=100)

- Ant-time holding spoil on the surface within 12 columns of the door: **0.61-0.65 against 0.17-0.18** for hA1.
  Spoil spells last a median 2,300-2,550 frames against 800. Foraging time halves (food in jaws 0.17-0.19 -> 0.01).
- Time underground with empty jaws barely moves (0.10-0.11 against 0.04-0.05). So it is **not** ants staying in to
  dig. The old ant's carry program, run in a mind ant's body, walks spoil around and does not let go.
- Fix tested: `PP_OLDLAB_ACT=3`, the old ant acts only while the jaws are empty. On hA1, 32 seeds: nest 19 -> 58.5
  (27 up / 4 down), crater 110 -> 89.5, food 279 -> 239 (14/18), starved 3 -> 2.5.

## 2-3. On hcs (`PP_HOMEVEC=40:30`), 32 seeds, sel off, 200k, fixed nestcount / roomshape (medians; paired vs hcs)

Labels: 6 worlds (41-46) with hcs acting, `PP_OLDLAB_EMPTY=1` (empty-jaw states only), `PP_COPY_SAMPLE=50:1`. About
4,400 old-ant cut labels, 1,450 no-cut labels, 150k underground step/turn/rest labels. Keep logs: fetch (8 worlds),
trail (4), both hcs with `PP_HOMEVEC`. Copier: `copier_d.py` with `only_verbs`, which trains only each unit's link
to the named verbs, their job head and the hidden layer's link to them, so the shared scorer weights (foraging,
the home sense) cannot move.

| arm | nest | crater | kept late cuts | spoil let go inside | home food | starved /24 | rooms |
|---|---|---|---|---|---|---|---|
| hcs | 13 | 115 | 2.5 | 153 | 318 | 1 | 0 |
| hd: copy, Dig weights only (60 slots) | 1 (2/27) | 69.5 (1/31) | 0 | 369 (32/0) | 222 (6/26) | 2 (16/9) | 0 |
| hd2: as hd, cut labels only (no must-not-dig) | 1.5 (2/27) | 43.5 (0/32) | 0 | 465 (32/0) | 191.5 (3/29) | 2.5 (17/10) | 0 |
| hu: Dig + Walk/Turn/Rest weights (240 slots), + underground step labels | 0 (4/26) | 60 (1/31) | 0 | 438 (32/0) | 241 (5/27) | 6 (29/2) | 0 |
| **acting crutch, `PP_OLDLAB_ACT=3`** | **60.5 (31/1)** | **96 (6/25)** | **11.5 (28/4)** | 140 (10/22) | **307 (14/18)** | **1 (9/14)** | 0 |

(ACT=1 on hcs: re-running, the worker restart killed it.)

- **Why the copies lose the nest.** Training only the verb's own weights can change *when* the ant digs, not *which
  cell*: which cell is scored by the shared feature weights, which the restriction freezes. The old ant's cut labels
  sit near the door and shallow, so the copy learns "dig there", and the deep shaft goes: cuts deeper than 8 below
  the surface 11% of all cuts in hcs -> 0% in hd and hu (deeper than 5: 21% -> 6-8%). Crater falls most likely
  because there is less hole (not checked cell by cell). Spoil then gets let go inside the shallow scrape.
- The copies did fit the labels somewhat (cut overlap 0.02 -> 0.21-0.26) without moving fetch / trail (overlap
  0.98-1.0). Underground stepping did not copy at all (0.09 -> 0.10): verb weights cannot pick a direction.
- **What does work is the old ant doing it.** `=3` is the Teacher lane's target shape (nest and crater move, food
  and starvation flat), but as a crutch, not a lesson.

## What I'd do next (Teacher lane's call)

1. Don't stack hd / hd2 / hu. Files kept as `hcs/*-not-recommended.raw.txt`.
2. A copy that can hold "which cell" has to train the shared scorer weights, which is exactly what broke hA1 in
   round 1. Two ways I can see: (a) train the shared weights but anchor every non-dig decision hard (fetch, trail,
   surface, carrying; round 1's d2 still moved 11% of them); (b) give digging its own feature head (new slots, like the
   far-home lane's sense) so cell choice for Dig is learned in weights nothing else reads.
3. Or use `=3` as a crutch inside colony selection so other genes (foraging, starvation) are selected in a world
   with a real nest, and then wean it. That is a new design question for the Teacher lane, not a tweak.
4. hcs here scores nest 13 on these garden settings against the 50 quoted from colsel8; the settings differ
   (this is `cg.sh`'s common garden). Paired comparisons above are like for like.

Patch for the hcs tree (a99909de): `oldlab-vs-a99909de.patch` (`PP_OLDLAB`, `_EMPTY`, `_K`, `_ACT=1/2/3`, all
env-gated). Scripts and configs: `hcs/`. `copier_d.py` now has `only_verbs` and kind 4 (underground step) labels.

---

# Round 3 (Teacher lane's 23:46Z pick): Dig's own feature head

**Result in one line.** A small head with its own weights, reading 12 new per-cut senses plus the option's 45, now
decides both which cell to cut and (in empty-jaw dig states only) whether to dig. On hcs, 32 seeds sel-off: nest 13 ->
25 (21 up / 10 down), crater 115 -> 100 (24 down), home food flat (318 -> 305.5, 15/17), starved 1 -> 2 (17/10). That
is about half the ACT=1 crutch's nest gain at no food cost. hcs's genome is unchanged; the head lives in a side file.

## What it is (`PP_DIGFEAT`, `PP_DIGHEAD`, `PP_DIGHEAD_GATE`; all off = byte-identical)

- **12 new senses per Dig option** (`scratch_log::dig_feats`):
  - away from or toward the door;
  - down or up;
  - the target's depth below the datum;
  - whether its column is open to the sky;
  - how much ground is over it;
  - its open neighbours;
  - its distance from the door;
  - the ant's own depth, whether it stands sky-open, and how open the 7x7 round it is;
  - a level cut;
  - the bottom-of-hole reading.
- **The head**: 8 units over those 12 plus the option's 45 features, with two read-outs.
  - **Rank**: re-ranks the Dig cells. Its weight is multiplied by the shared scorer's, so it can only choose among cuts hcs
    accepts.
  - **When**: `c2 + max over Dig options`, added to the Dig job's sum. With `PP_DIGHEAD_GATE=1` it applies only in
    empty-jaw dig states: no spoil, crop empty, below the datum, within 40 columns of the door.
- **Training** (`head/copier_dh.py`, `dh7.json`):
  - The genome is frozen; only the head's 482 slots train.
  - Labels: the old ant's calls on 12 worlds (6 with hcs acting, 6 with ACT=3 acting), recorded with `dlog2.sh`.
  - Groups:
    - its cuts (which cell and dig now), with hanging-bite cuts dropped;
    - its "would not cut here" (don't dig now);
    - every other soil decision anchored to hcs.
- Held-out fit:
  - P(the old ant's cell, given Dig) 0.57 -> 0.69.
  - P(Dig) where the old ant cuts 0.05 -> 0.42.
  - P(Dig) where it would not 0.23 -> 0.19.
  - P(Dig) everywhere else 0.008 -> 0.017.

## The garden table (32 seeds, sel off, 200k, fixed counters; medians)

| arm | nest | crater | kept late | spoil let go inside | food | starved /24 |
|---|---|---|---|---|---|---|
| hcs | 13 | 115 | 2.5 | 153 | 318 | 1 |
| old ant's cut calls acting (ACT=1, the crutch) | 38.5 (26/5) | 91.5 (7/25) | 6 | 119.5 | 376.5 (19/13) | 1 |
| old ant picks the cell whenever hcs digs (ACT=4: any cell-only head's ceiling) | 16 (15/16) | 116.5 | 6 | 148 | 350.5 | 1 |
| dh3: head alone picks the cell | 17 | 118 | 5 | 111.5 | 371.5 | **8.5 (29/1)** |
| dh5: + "when", ungated | 28 (20/11) | 108.5 | 4 | 134.5 | 274.5 (10/22) | 3 (22/6) |
| dh6: as dh5, stronger anchors | 28 (22/10) | 109 | 4 | 131.5 | 248 (7/25) | 3 (21/3) |
| **dh7: as dh6, "when" gated to dig states** | **25 (21/10)** | **100 (8/24)** | 5.5 | 127.5 | **305.5 (15/17)** | **2 (17/10)** |

## Three things that cost a run each

1. **Which cell is worth almost nothing.** The old ant's cell whenever hcs digs (ACT=4) changes about 12 cuts a run and
   moves nest 13 -> 16. Under ACT=1 the gain comes from *when*: about 780 of hcs's digs a run are replaced by the old
   ant's move, and about 170 cuts are added. Nothing in hcs's Dig job reads the cut scores much (its weight on the best
   Dig target is -0.29), so a rank-only head could never get there.
2. **hcs's cell choice is at chance.** It scores every Dig option at 0.97-0.99, so a head added before the squash could
   only flatten it. The first two trainings did exactly that (every output weight ~4.4, P 0.485 -> 0.489). The head now
   multiplies into the target stage instead.
3. **Copying the old ant's hanging-bite cuts buries mind ants.** The old ant takes a hanging bite in a quarter of its
   cuts. dh3's diggers did the same, were buried, sank to the floor (39 rows down) and starved there holding spoil:
   8.5 of 24 against 1, traced ant by ant on seed 25. Those labels are now dropped, and the head can only re-rank cuts
   hcs accepts.

The remaining starvation in dh5-dh7 (seed 28 traced) is ants carrying spoil for 80-93% of their lives far from home.
That is hcs's own carry, hit harder when more ants dig. The gate is what brought food back.

How predictable the old ant's cell is at all from these senses (`predict.py`, a plain conditional logit): top pick 58%
against 39% by chance. A lot of its choice comes from its own state (dig trip, heading), not the cell.

## Files

- `head/dh7.head`: the head. Run hcs (unchanged) with
  `PP_HOMEVEC=40:30 PP_DIGHEAD=/mnt/project-files/nest-score/dig-lesson/head/dh7.head PP_DIGHEAD_GATE=1`.
- `head/dighead-v4-vs-05eff7dc.patch`: what the compute branch (05eff7dc) needs on top of the earlier patch. It builds
  there, and on seed 3 it reproduces both the hcs garden (off) and the dh7 garden (on) byte for byte.
- `oldlab-dighead-vs-a99909de.patch`: everything, against the earlier base.
- `head/copier_dh.py`, `head/dh*.json`, `head/dlog2.sh`, `head/cg3.sh`, `head/predict.py`.

## Round 4 (2026-10-23 ~05:30Z): the dig head with the body rules on

Built on c5e87f68 (PP_BODYRULES=both) plus head/dighead-evolve-vs-1a98de26.patch (applies cleanly). Counted with c5e87f68's own nestcount/roomshape (flood passes stored food), so numbers are not comparable with rounds 1-3. 32 seeds, sel=off, 200k frames, PP_HOMEVEC=40:30, medians, paired up/down vs the first arm.

| arm (rules on) | nest | crater | cuts | food | starved |
|---|---|---|---|---|---|
| hcs | 171.5 | 8.5 | 144 | 569.5 | 1 |
| hcs + dh7 (old labels) | 191.5 (28/4) | 8 | 163.5 (29/2) | 518 (11/21) | 1 |
| hcs + dh8 (labels re-recorded with rules on) | 190.5 (23/9) | 5.5 | 162 (24/7) | 474 (8/24) | 1 |

- The body rules do almost all the work: nest 13 -> 171.5, crater 115 -> 8.5, food 318 -> 569.5 from rules alone (rounds 1-3 numbers used the old counter; the size of the jump is far beyond the counter change).
- On top of the rules the head adds ~20 nest cells (+11%) and costs 9-17% food. Re-recording its labels with the rules on (dh8) did not help: same nest, more food lost (dh8 vs dh7 food 7/25 down).
- Recommendation: with the rules permanent, use plain hcs + rules; leave the head out, or use dh7 only as an evolvable start (PP_DIGHEAD_EVOLVE=1) so selection can turn it down. dh8 is not recommended.
- Files: head/cg5.sh, dlog5.sh, summ5.py, dh8.json, rules-on-*.summ, dh8-bodyrules-not-recommended.head.
