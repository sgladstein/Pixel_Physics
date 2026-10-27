---
name: far-food-1024
description: Teacher lane 2026-10-24/25 — far food in the lab, link price 0.25 default, nest erosion hunt, current batch price25a
metadata:
  type: project
  modified: 2026-10-25T08:28:06.630Z
---

**Full record in the repo:** Reports/brain-training/teacher-training/colony-selection-log-2026-10-24.md (every batch, lab tables, what each overturned). Read that for numbers.

**Standard ant:** assets/minds/std_pool.txt = price25b ctl colony 10 (PR #906, 2026-10-25), before that laygene2 colony 8 (#875). Owner playtests main.

**Far food, settled 2026-10-25:** the brain's per-link energy bill (~1,950 links), not training, sets far-food range. Fruit 200 lab box alive: price 1 -> 28, .25 -> 114, 0 -> 150, hand-built ant 129. Owner said yes (07:29Z) -> PIXEL_PHYSICS_LINK_PRICE default 0.25 in every world (spiders too), PR #899 (merge desk). Same PR fixes pack_behind/escape_cut never pushing cut_log (in-run dig credit missed those cuts). Lab/training numbers shift across #899: re-measure baselines on the same build.

**Nest erosion (open):** garden median nest falls every round in every arm: erosion1 (no colony selection erodes faster), erosion2 (w_dig 3/7 no help, lab worse), erosion3 (food near at 25 erodes too, 276->156; far-food ramp not the cause). Suspect the in-run loop; first read = price25a ctl garden r7->r15 on the cut_log-fixed build. Choose founders by multi-seed garden, not one world.

**price25b (done 10:10Z):** ramp cap 120, all colonies live; ctl garden nest 458/454/501 = NO EROSION at price .25 + cut_log fix. rho and caste arms not carried. RATCHET: ctl c10 = new std_pool (PR #906): fresh gardens food 100 vs c8 nest 462 vs 344 (8/8), crater 2 vs 6.5, food home -5%, lab level. NOTE colsel gardens run at dist 25 (BASE_DIST default) even with distramp. door1-1025 (SHA ea19ff91, runvm-door1.log -> scratchpad/door1): ctl / PP_MIND_DOOR_OPEN=drop / =clear from new pool.

**Plants/door/herbs (to 15:00Z):** door1: DOOR_OPEN=drop even (could go), =clear halves food home. lab_herbs (old ~65 vs std ~17) did not move in herbs1 (summed) or herbs2 (labcase=, own case). Trace (labsurvive trace=25): trained ants wander whole box, hand-built stay at door on plants 82%; bite rates equal -> cause is walking away from home food. inrun near=N (home-side west heap, kept out of gardens) built; near1 SUPERSEDED by nest1 (owner 15:01Z: random food distance always; nest is one of the biggest issues): colsel heaps=20:120 (east+west heaps drawn per colony-round) + scalegate=50 (nest-scale donor gate); jobs/nest1 heaps / heapsgate, SHA dcc60b31 with 3rd arm heapsgatewean (PP_BODYRULES=both wean=6:20, owner 15:15Z asked to retry the old-rules scaffold), njobs 3 (ab67d611 line, hcasedrop = ramp reference), launch AFTER fw1 (SHA e603af52, fwseed05/15 pools, ctl hcasedrop). PR #916. No new founder from hcasedrop (food up, nest down). **fw1 DONE 15:52Z (seeded last-food weights fwseed05/15): FAILED** vs hcasedrop r23 garden: starved 26.5/29 vs 10 (higher 26/32, 24/31), fw05 food down 27/32, fw15 nest down 20/31; lab_herbs 148/112 vs 178. Not carried; log it with nest1. nest1-1025 launched ~16:00Z (task bitbqf6r9, 3 arms, SHA dcc60b31). Disk filled 15:52Z: deleted old batch pulls (all in bucket).

Keeper: taught ants spend 67-70% laden walking home slowly -- homeward walk is the far-food bottleneck.

**Lab traps:** landing-slot ids 6-25 die by ~1,100 frames whatever the brain (queen seat 09:39Z); timeline Heap drops entomb ants (lab_far* every 500 frames).

**Tools/traps:** colsel `distramp=`, `labgate=Q`, `labscens=A,B`, `cg_every=N`, `+scale` (dead colony = 0), PP_BED_W. inrun arg() takes FIRST match: EXTRA can't override colsel's own args. Harness deterministic: identical ctl reruns while lab scenarios unchanged. Run VM waiters as harness background tasks (nohup waiters die).

See [[teacher-lane]], [[batch-design-rules]], [[colsel-judging]], [[far64-lab-playtest]], [[plant-forage-playtest]], [[link-price-farfood]].
