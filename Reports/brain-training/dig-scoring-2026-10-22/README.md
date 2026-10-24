# Dig and nest scoring: what the thread found (2026-10-21 to 2026-10-23)

> **Landed 2026-10-24** by the report sweep (thread *Land unlanded thread reports*). The "Dig and nest scoring" thread
> finished without putting its write-ups in the repo; they lived in the project shared folder `nest-score/` and in
> project memory. This README is the summary, compiled from the thread's own dated notes, **not re-measured**. The
> three full write-ups it produced are beside it, copied verbatim. Code it shipped is already on `main`
> (`src/lab/nestscore.rs`: PRs #770, #780, #795, #802).

**What the thread was for.** The owner's goal for the ant: **keep digging a large multi-chamber nest and keep
foraging for the whole game** (owner, 2026-10-22 13:15Z; chambers are 7 cells tall). Selection kept breeding digging
out and leaving a crater (an open pit at the door) instead of a nest. This thread built the measurements for a nest,
found why digging died, and delivered score rules, refusal teachers and a dig lesson to the Teacher lane, which owns
the one ant.

## The three write-ups

| file | what it holds |
|---|---|
| [`why-digging-dies-2026-10-22.md`](why-digging-dies-2026-10-22.md) | why selection breeds digging out: the trace, the common-garden test, and the short-round fix that **was refuted** at 64 seeds (its header says so) |
| [`dig-lesson-from-the-old-ant-2026-10-22.md`](dig-lesson-from-the-old-ant-2026-10-22.md) | `PP_OLDLAB`: the old ant acts in a cloned world as a labeller; rounds 1-4; the dig head (`PP_DIGHEAD`, later evolvable and default-on through PR #849) |
| [`roomshape-2026-10-22.md`](roomshape-2026-10-22.md) | `roomshape.py`: a graded "on the way to multiple rooms" score from an end-of-round nest map |

## Findings, in order

1. **The crater is rim cuts.** 44 of 49 top-row cuts are "step down from the surface, cut the first wall, walk out";
   the in-run score paid them. A scratch rim veto cut a lone pit from 57-85 cells to 10-15. Retracted along the way:
   "the trial is too short" and "food at the door".
2. **The real reason digging dies (13:40Z 2026-10-22).** The dig price barely falls (3.0 -> 2.5). Late in a round,
   ants re-dig loose spoil in the crater: in hA1L20 round 0, new cuts fell 146 -> 49 per 50k frames while recuts rose
   254 -> 461, spoil carried back in 43 -> 206 and the pit 43 -> 109 (the same in rounds 3 and 6, and on founder c1).
   **The crater is a treadmill, and late generations are bred on bailing it.** Round length is **not** the lever
   (shortr-1022, 64 seeds: refuted). In common gardens hA1 x20 lost digging by round 10 in 7 of 13 colonies.
3. **Refusal teachers.** `PP_RIMVETO=3` (refuse rim Dig options of ground within 20 columns) with `PP_SPOILVETO`
   (refuse spoil dropped below the datum or near the door): one round, hA1, 8 seeds: recuts 437 -> 177, spoil carried
   in 221 -> 1, best home food, starvation unchanged. **The spoil veto alone starves** (162 vs 63). At the ceiling
   (h1, selection off, seeds 1-8) the rim veto took the crater within 20 columns to 0 on 8/8 and home food 76 -> 289.
   Copied into the founder as crater-refusal lessons r4/r5
   ([`../teacher-training/crater-refusal-copy-2026-10-22.md`](../teacher-training/crater-refusal-copy-2026-10-22.md)).
4. **Pit only near the nest.** The nest score charged every sky-open dent in the bed (field scrapes 20-100 columns
   out). Owner said yes to charging only near the nest: `Profile::pit_reach = Some(20)`, PR #802.
5. **Room credit did nothing.** Paying the cutter of cells that end in a chamber (`roomcredit-vs-98ad9dc4.patch`):
   rooms are worth ~0.07 a cell and the cutters are dead before they are paid. Per-ant shape pay (`w_shape`, pay a
   cut its marginal roomshape gain, cap 4): no selection response in a small test.
6. **Nest zone (owner's design, built here).** `nest=zone nest_pad=2 nest_top=3`: a never-shrinking zone that only
   grows by cuts inside it; only zone cuts pay and food counts only when put down in it. At 64x6 the crater was
   unchanged and home food fell by a third to a half: **not adopted.** A temporary cost outside the nest
   (`zone_debit=1`) killed 4/8 colonies: off. Owner rulings that came with it: buried food and a nest creeping toward
   the food are not problems; the top 3 rows never count as nest except the stamped mouth; extra starvation is
   acceptable if it buys a clear fix that foraging can be trained on top of.
7. **Every rim rule costs feeding.** Batch rimshape-1022 (5 rim rules, fixed score): each shrinks the crater by about
   a third and costs 10-19 more extinctions (control 20/64). Agreed with the Teacher lane: a **feeding gate** (<= 10/64
   extinct over 6 rounds, food at least the control's) before the dig frontier pays above 1x. Not an absolute veto.
8. **The old ant as a dig teacher.** Copies of its labels into the shipped weights fail (hA1 starves; hcs loses the
   nest). What carries is *when* to dig, not *which cell*: acting with its cut call alone, nest 13 -> 38.5 and food
   318 -> 376. Round 4 (with the old ant's body rules on mind ants): hcs nest 171.5, crater 8.5; the rules do the
   work and the dig head adds ~20 cells at a food cost. That led to the independent crater review
   ([`../teacher-training/crater-review-2026-10-23.md`](../teacher-training/crater-review-2026-10-23.md)) and the
   owner's ruling of 2026-10-23: **no permanent body rule from the old ant.**

## The scoring ladder the owner agreed (design doc, 2026-10-21)

Rungs inside one colony score, nest score as the final form: **1** dig down, not out (`rim_rows`) -> **2** spoil
leaves (`rim=15`) -> **3** door open (`door_open`; a closed door is fine part-time unless a forager is shut out or
hungry ants are shut in with a low store) -> **4** room credit to the cutter -> **5** nest score. Frontier rung pays
4x, a met rung keeps paying 1x. Later, under colony selection, the Teacher lane counts kept nest cuts at the colony
level (`nestcount.py`), and pays an ant only for cuts touching open space joined to the door (`cut_join`).

## Files here

- `scripts/`: `rungcheck.py BATCH --control ARM` (feeding gate + rungs per arm), `batchsheet.py` (end maps per arm),
  `closures.py` (offline door-closure cost; its laden-outside proxy is **not specific**), `zonecheck.py`,
  `nearpit.py` (near vs field pit), `roomshape.py`, the Teacher lane's `nestcount.py`, and the picture scripts.
- `patches/`: `rimveto-vs-03e499eb.patch`, `nestzone-vs-685b21da.patch` (+ `scripts/nestzone-jobs.txt`). All patches
  here are against compute-branch commits, scratch, off by default.
- `digloss/`: the why-digging-dies data and scripts, the spoil-veto, room-credit and `w_shape` patches. The two GIFs
  (round time-lapse, late-round close-up) stay in the shared folder, `nest-score/digloss/gif/`.
- `dig-lesson/`: the labeller patches, copiers, label summaries and dig-head files (`head/dh7.head` is the head that
  shipped embedded).
- `pictures/`: the crater and rim-ban pictures, the rim-veto ceiling, the nest-zone local test, the nest-score
  gradient.

## Testing rules this thread paid for

- **Measure digging in a common garden** (selection off, fixed worlds), never by in-run counts alone: in-run totals
  are confounded by which colonies survived and by cutting falling through the round.
- **Time-match before comparing two groups of ants** over a round: an unmatched comparison made survivors look like
  40% weaker diggers.
- An 8-seed result at p=0.06 did not survive 64 seeds.
- Raw label logs are ~250 MB each and filled the disk once: delete them after `filt.py`.
