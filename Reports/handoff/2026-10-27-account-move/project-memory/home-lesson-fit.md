---
name: home-lesson-fit
description: Teacher lane 2026-10-26: fitting the come-home teacher into the c13 pool's weights (examples/homefit.rs) -- results, the in-world vs offline agreement gap, DAgger
metadata:
  type: project
  modified: 2026-10-26T01:50:04.113Z
---

Teacher lane, 2026-10-26 ~01-02Z. Goal: make "hungry and far -> walk home" the trained ant's own habit (teacher acting cuts starvation ~10x, see [[nest-priorities]] ceiling). Tool: `examples/homefit.rs` (uncommitted in /home/user/cloud, plus `mind::home_lesson_slots`): Adam on central differences over the home-sense slots, loss -ln P(label) on labelled copy-log rows + wkeep x KL(ref||new) on unlabelled rows. Args added: `ref=` (KL reference genome), `eval=` (P(label) by distance from home).

Results on 8 test worlds (seeds s*1000+7, everything off, patches=20000:40:120), vs c13 pool (starved med 240, deliveries 4,112):
- hf1 (wkeep 1): starved 184 but deliveries -35%, 2/8 colonies died out (leak).
- hf2 (wkeep 3, keep 60k): no leak, no gain.
- hf3 (unit=8 energy-gated, lr .08, 400 steps): starved 204.5, deliveries 3,811 (lower 7/8), 1 colony died out.

**The gap (traced):** offline P(label) on c13's own states rises 0.28 -> 0.45-0.66, but the logged in-world pick (copy log field 7 = d.pick) agrees only 0.33-0.38 on the fitted ant's OWN states; hungry/far ants still drift AWAY from home (-0.06..-0.29 columns/100 frames) vs teacher acting +2.25 (positive control, world 1007). Deaths stay far from home. Fix in progress: DAgger (label the fitted pool's own runs with PP_CARRY=0, refit from it with ref=c13 genome). Scripts: scratchpad hl/agree.py (in-world agreement), hl/drift.py, hl/deathwhere.py.

Copy log fields: 0 tag,1 Q,2 frame,3 id,4 x,5 y,6 settle,7 pick,8 last verb,9 carrying,10 labels,11 ctx,12 mem,13 opts,14 bottom,15 carry label (1|i;j),16 refused. Nest column 128. Logs are ~2 GB per 200k-frame world at sample 10:1 -- watch disk.

**Fits 4-5 (02:00Z):** DAgger refit (hf4) worse (starved 290); whole unit 8 free (hf5, 128 slots) plateaued P(label) 0.40, starved 260. Weight copying stopped here.
**Saturation (owner asked 02:14Z how big):** scores = squash(sharp*raw), sharp = (1+g[9]) clamped [0.25,4]; forager seed set g[9]=3 so every genome since c1 sits at 4x (c13: 5.3-6.7). 80% of walk scores past +-0.95, 72% of decisions all walks "no". Pinned share at 4/2/1/0.5x: 80/62/31/4%. c13 at 2x: starved 257 vs 240, open 350 vs 398; 1x: starved 296, open 178. New switch PP_SHARP_MAX (uncommitted, /home/user/cloud mind.rs `sharpness`), checked byte-identical to the gene edit. Plan: nest9 arms PP_SHARP_MAX=2 and =1 vs control, after nest8.

PR #925 merged eb1f387 (03:08Z). Lesson from the merge desk: run `bash scripts/docscheck.sh` and read ALL of it before handing in -- a new example needs a Reports/instruments.md row and a new dead-ends entry needs a screened.tsv verdict; tailing its output hid both (desk fixed in #926).

**HOME-LOW SENSE (done 06:30Z):** `PP_HOMELOW=k` (+-1 toward/away home column x clamp(1-e/(1+k*dx)), 0 fed) on slot 18 = FOOD_WAY, so requires PP_LASTFOOD=off; `PP_HOMELOW_STORE=1` = 0 while no nest store (ungated kills far lab boxes: ants land at 0.4-0.9 tanks). Clean fit hf8 (homefit feat=18): P 0.28->0.79, KL 0.0006. Pool assets/minds/homelow_pool.txt. Run with PP_LASTFOOD=off PP_HOMELOW=0.006 PP_HOMELOW_STORE=1: 16 worlds starved 44 vs ~245, alive 78-79, 0 extinct (c13 2); lab far mix 110 vs 102, fruit 70 vs 69, herbs 18 vs 13. k=0.012 kills 2/8 (founders turn too early). Failure 13099: food moved far at 40-60k, everyone low -> home, ate the crumb store down, all starved AT HOME (143/162 within 20 cols). PR #928 (default off) at merge desk; nest10 on VM (SHA cb11b93); owner card: make standard? Page https://claude.ai/artifact/JA6kxgzwxmxy4EoaEqir5B.
