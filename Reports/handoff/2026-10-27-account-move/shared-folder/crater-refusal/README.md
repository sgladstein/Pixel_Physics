# Crater refusal copied onto hA1 (2026-10-22, 13:30-14:45Z)

**In the repo:** Reports/brain-training/teacher-training/crater-refusal-copy-2026-10-22.md (+ folder of the same name), branch claude/project-thread-ng2n8a, PR to main.

**Goal.** The dig lane's crater refusal (`PP_RIMVETO=3`: no cutting ground in the top 3 rows within 20 columns of
the door) stops the crater treadmill that breeds digging out. It only works while the refusal is switched on. This
thread tried to copy it into the founder hA1's brain, so the ant keeps the habit with the refusal off.

**Result.** One usable candidate, **`genomes/r4.raw.txt`**. With the refusal off it shrinks the crater by about 40% and
brings home about 24% more food, but about 1.6 more ants per colony die. A perfect copy would take the crater to 0,
so r4 got part of the lesson. It is a candidate for the Teacher lane to judge at 64x6 with selection, not a new
founder yet.

## Numbers (bed, 24 ants, 200k frames, sel=off, seeds 1-32, refusal OFF unless the arm says "acting")

Means per colony; "paired" = seeds where the arm is higher than hA1, out of 32. Full table: `judge-32seeds.txt`.

| arm | food home | died of 24 | collapsed | crater near nest (cells) | nest dug | new cuts | re-cuts | spoil let go inside |
|---|---|---|---|---|---|---|---|---|
| hA1 (control) | 269 | 4.3 | 2 | 39 | 111 | 320 | 860 | 149 |
| r2: rim states only, weight 5 | 302 (21 higher) | 5.2 | 3 | 32 (16 smaller) | 111 | 292 | 819 | 128 |
| **r4: rim states only, weight 20** | **333 (24 higher)** | **5.9 (21 more, 8 fewer)** | **4** | **23 (22 smaller)** | 117 | 268 | 760 | 121 |
| r3: crater + spoil refusals, weight 20 | 215 (10 higher) | 7.2 | **8** | 54 (24 LARGER) | 93 | 247 | 502 | 131 |
| ceiling: hA1 + crater refusal acting (16 seeds) | 491 (16 of 16 higher) | 2.8 | 0 | **0 (16 of 16)** | 135 | 252 | 888 | 97 |
| ceiling: hA1 + both refusals acting (8 seeds) | 525 (8 of 8) | 3.2 | 0 | 4 | 134 | 212 | 595 | 2 |

"Crater near nest" is the dig lane's `nearpit.py` count from the final map: sky-open cells below ground within 20
columns of the door, the mouth's own 4 columns left out. "Collapsed" means 12 or more of 24 died.

## What happened, in order

1. **Build.** Compute branch 98ad9dc4 + the foraging lane's patch (copy-log sampler, shadow/carry switches) + a new
   switch `PP_RIMVETO_LABEL=1`: the refusal writes its label in the copy log but never acts. Positive control: a
   label-only run of hA1 is byte-identical (events.csv) to plain hA1. Patch: `crater-refusal-vs-98ad9dc4.patch`
   (also pushed as branch `claude/project-thread-ng2n8a`, scratch, no PR).
2. **Logs.** hA1 acting in 6 bed colonies (seeds 41-46), the refusal labelling its states (`scripts/rlog.sh`), plus
   soil/fetch/trail keep logs of hA1 (`copylog`, run seeds 9101-9103).
3. **r1 (foraging lane's recipe as is): did not move**, best step 0. hA1 already avoids the refused cuts in 99.5% of
   the ~46k labelled states per colony, so the lesson drowns, and the memory path left a 0.055 KL floor on every
   bed group that hid it.
4. **r2/r4**: `copier_r.py` adds `rim_min_mass` (keep only the ~9.6k states where hA1 puts 2%+ of its choice on a
   refused cut; the rest anchor to hA1) and runs with `mempath: false` (floor gone: 0.000 at step 0). Weight 5 (r2):
   agreement with the refusal 0.872 -> 0.877, too weak. Weight 20 (r4): 0.872 -> 0.900, kept skills 0.988-0.996.
5. **r3** added the dig lane's spoil refusal (`PP_SPOILVETO=5`) to the same label set (the copier now handles refused
   Drops per verb). It copied well on paper (0.886 -> 0.945) but in the bed the ants learned "don't let go of spoil"
   everywhere, not just by the door: spoil let go far from the door fell 937 -> 583, the crater grew, and 8 of 32
   colonies collapsed. **Don't use r3.** The acting ceiling for both refusals is the best arm in the table, so the
   spoil refusal is worth having, but it needs a different copy (e.g. weight the door Drops only, or a body sense of
   "near the door").

## Reading

- The crater refusal does copy, partly. Each 4x on the weight bought about half again of the crater cut, and more
  food came with it. The cost is a small rise in deaths (4.3 -> 5.9 per colony), which selection might absorb; the
  Teacher lane's batch is the test for that.
- New cuts fall in every copy (320 -> 268 in r4) and in the acting ceiling too (252), while the nest still grows
  (111 -> 117; ceiling 135). The ants stop the shallow cuts that fed the crater, which is the point, but check digging
  in a common garden after selection as the dig lane does, since erosion of digging is the open problem.
- Not tried for lack of time: weight 40+, more rim states (more colonies logged), a second DAgger round on r4's own
  states.

## For the Teacher lane

Judge `genomes/r4.raw.txt` against hA1 at 64x6 with selection, refusal off, and with a common garden for digging.
Pack move on (compute default). Same genome format as hA1.

## Teacher lane, 15:10Z
r4 queued: arm r4_1..64 (6x200k, selection on, refusal off) in batch switches-1022 at compute abe5f614, control hA1c from ha1next-1022 (same code). It starts when the body-genes batch frees the VM (~15:15Z), ~25 min run, then common garden 32 seeds (home, starved, cuts).

## r5: all three teachers (done 15:40Z)

The dig lane's positive spoil teacher (`PP_SPOILDROP=15`: holding spoil above the datum 15+ columns from the door,
every Drop on offer is the label, written as the 16th field) beside both refusals, logged on hA1 acting (3 colonies,
memory limit), copied at r4's settings plus carry weight 10. The far-drop habit copied strongly (agreement 0.25 ->
0.89); the crater part slipped (0.905 -> 0.884). Bed, 32 seeds, sel=off, teachers off, vs hA1:

| arm | food home | died of 24 | collapsed | crater | nest dug | new cuts | re-cuts | let go far out | let go inside |
|---|---|---|---|---|---|---|---|---|---|
| hA1 | 269 | 4.3 | 2 | 39 | 111 | 320 | 860 | 937 | 149 |
| r4 | 333 | 5.9 | 4 | 23 | 117 | 268 | 760 | 802 | 121 |
| **r5** | 271 (14 higher, 17 lower) | 4.6 | 1 | 28 (19 smaller) | 115 | **388 (29 higher)** | **2,055 (32 higher)** | **2,097 (32 higher)** | **267 (32 higher)** |

Reading: r5 is the only copy that digs MORE than hA1 (new cuts +21%) with no extra deaths, but it opens a new
treadmill: it lets go of twice as much spoil out in the field and re-cuts more than twice as much (most re-cuts are of
spoil, mound or fill). Spoil let go inside the nest also rose, so the spoil refusal did not copy. Food home and crater
barely moved. Neither r4 nor r5 is a clear win: r4 for crater and food, r5 for digging and survival. Both are with
the Teacher lane to judge under selection.
