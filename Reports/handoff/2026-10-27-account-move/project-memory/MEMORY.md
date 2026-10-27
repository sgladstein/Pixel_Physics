# Pixel Physics
- Repo: github.com/sgladstein/Pixel_Physics (Rust physics/game engine). Goal: pixel world where ants and plants behave realistically/emergently.
- Active (Oct 2026): ant brain training (in-run colony selection, Teacher lane), plant community; reports in Reports/.

## Compute (owner)
- 2026-10-23 03:06Z: Google Cloud = 64 cores (one VM); more via GitHub Actions or Haiku cloud runners. Lanes work autonomously overnight.
- NO WAITING ON THE VM (2026-10-24 02:08Z): lanes never idle waiting for the VM; use own cloud machines.

## Consolidate (owner)
- 2026-10-22 15:02Z (corr. 15:03Z): threads started 13:20-14:40Z take no new tasks but finish their to-do list, hand off to parent, resolve. New work to parents unless important/no parent. Parents: Teacher lane, Dig and nest scoring, Plant community, Lab interface, Lab world perf, maze, Merge desk.
- NO PARKED WORK (10-22 15:37Z): land finished work via merge desk (off by default if it changes results).
- LAND BEFORE RESOLVE (10-24 15:04Z): reports to Reports/ (+ how-we-test.md rules) via merge desk, rules to memory, tell affected lanes. Sweep PR #864. [[land-before-resolve]]
- 10-22 ~20:00Z started "Far-from-home sense" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBAGWiYCU5kfaYBKZMeh4ksH) and "Training harness on main" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bB4EBvL4nrkt1yHizQxB2HGZ).

## Ant training (owner, from 2026-10-21)
- Teacher lane (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBFFqqpwTYL4i4DtFihYcesb) owns main training (in-run colony selection); training results/requests go there first. It may hit message caps (10-22) and post in its own thread for relay.
- ONE ANT (10-22 05:34Z): Teacher lane owns the one ant line; others deliver lessons/teachers/scores/arenas/findings onto its founder, never separate ants. Small test trainings OK.
- POOL, NOT ONE BRAIN (10-22 14:01Z): colony's surviving genome mixture is the product. No hand-made castes.
- RATCHET (10-24 03:15Z): every batch starts from latest best pool; best colony = next founder; A/B arms share it. Current (10-26): std_pool = nest7 hweanrules c13 (prev price25b c10 #906). homelow_pool starves ~5x less, PR #928 owner card pending. [[home-lesson-fit]]
- NEST (10-25 15:05Z): order 1) too small 2) crater not mouth 3) no rooms. Trained ants never eat stored food (starve by full larder). Judge open room (nest_open). [[nest-priorities]] [[nest-batches-1025]]
- REPLAYS + FAR FOOD (10-25 19:09Z): replay page after most runs; food far and moving (patches=20000:40:120). [[replays-and-far-food]]
- STANDARD DEFAULTS (10-24 03:03Z): standard = default-on in training AND lab: PP_HOMEVEC, dig head + light, breeding (#845 lay_cap=48:8, 10-24 00:47Z), cut_join + crater/rooms cases, PP_MIND_GRAZE, nutrient-by-root (#847; =0 for old). Experimental off.
- GRADED SENSES (10-24 14:04Z): prefer graded (log-distance, no cap/dead zone) unless significantly worse and untrainable.
- DIG + FORAGE ALL GAME (10-22 13:16Z): never propose fixes that drop digging. Food is not the only goal (15:37Z): judge digging/nest too. Extra starvation OK if it fixes a clear problem and foraging trains on top (10-22).
- Dig focus (10-22 18:25Z): no crater near nest top + multiple rooms (roomshape case). Crater (10-23): charge digging top 3 soil rows near nest. [[room-shape-measure]] [[nest-cuts-rule]]
- Body should mutate (10-22 13:34Z): "Body genes in training" cmsg_01Mei3dwkcdzPxa5Jmuqi2bBJszhyE7kpUzR8QoiHEK7L1. [[body-genes-lane]]
- MUTATION LOAD (10-27): use colsel elite=1 always (PR #938). [[mutation-load]]
- BRAIN GATE (10-26, PR #933-935): per-unit gate (layout 6) + reach (layout 7) on by default; 16 worlds showed no "food at home" sense, asked owner. [[brain-gate-results]] [[brain-gate-review]] LATCH LANDSCAPE (10-27, PR #940): selection has no step-by-step path to the home-had-food latch; owner chose write-gate prototype, off: #942 merged eeb7611, layout 8 PP_WRITE=on [[latch-landscape]] [[write-gate]].
- PHEROMONES (10-26, PR #929): new free channel lost to deaf; emit rows stay frozen. [[evolvable-pheromones-lane]] [[evolvable-pheromones-history]]
- SELECTION REVIEW (10-27, PR #939): colony survival ~luck; nest cases 63-71% of vote; audit with selaudit.py. [[selection-review-1027]]
- ENV REVIEW (2026-10-27, PR #941 merged 44410be): std_pool alive 15/16 at 200k, 0/16 at 600k on its own bed (old ant 14/16); no windfall in training (two_larders lived 3.6-7.3 -> 37-101 when swapped); harness-only lay_cap/door drop/no old age/3 tanks each cost it in the game; nests hit 40-row floor; plants= rain never fell. [[training-environment-review]]
- LINK PRICE default 0.25 (#899). [[link-price-farfood]]
- Lab gap (10-24): trained pool starves in owner's far-food lab box; "trained by default in lab" held until a pool survives. [[far64-lab-playtest]] [[plant-forage-playtest]] (#890)
- Starvation (10-22): ~55-62% roamers, ~1/3 pellet jam. Trail: hA1 crowd trail = "go where others are"; useful trails via teacher. [[starve-trace-lane]] [[trail-teacher]]
- Reviews: Fable https://claude.ai/artifact/AYMLfoEd4jvMiJK2g2RgG9 ; scoring https://claude.ai/artifact/S2h242Q6ebWYKjywSEBsUt (#816).
- Open owner picks: Brood (eat buried scraps vs stop carrying half-eaten food) [[brood-lane]]; seat decision [[teaching-the-colony-plan]].

## Game seats (2026-10-25)
- Plan: queen recommended main seat; PR #896, doc https://claude.ai/artifact/AQmZ5E5towdrhgfXhtTJyt. [[teaching-the-colony-plan]] [[creatures-1996-lessons]]
- Queen #908 (pics https://claude.ai/artifact/BaPrQYh4UVRA58Ewc2v751) [[queen-seat-demo]]; Keeper #914 [[keeper-seat-demo]]; Guest #910 [[guest-seat]]; Colony mind #905 (test the TRAINED ant, score trip_deliveries) [[colony-mind-seat]]; Scout [[scout-seat-demo]]; Enemy (owner 10-25 19:06Z: forced spit annoying, combat a pile) [[enemy-seat-demo]].

## Design/physics rules (user)
- 2026-10-21: one brain design for every ant (dig + forage), no castes; later roles from signals. Ants can starve; eat from colony store. Chamber height 7.
- 2026-10-21: one set of physics for every world (lab, outdoor, held).
- 2026-10-23 12:18Z: no permanent old-ant body rules; at most weaned scaffold, judge rules-off. [[no-oldant-body-rules]]

## Review / merge / threads (user)
- 2026-10-21: review queue retired; questions and image comparisons go via chat; tell new lanes. 10-25: owner can't see chat images, use Artifact pages. [[images-via-artifact]] [[review-queue-retired]]
- 2026-10-21: Merge desk (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBBjZgzija4YCh6ciFtqnPbN) owns all merging + full suite; lanes open PR and hand off; desk merges once checks pass. [[merge-desk]] [[merge-desk-owns-pr-watching]]
- 2026-10-21: lanes may request Haiku runners/extra Opus lanes (since 10-22 15:02Z only when important). Fable lanes expensive: one review, resolve.
- Perf threads (bit-identical): Lab world cmsg_01Mei3dwkcdzPxa5Jmuqi2bB6YrKEBA1AimzzHSS3rvLcq, Creatures cmsg_01Mei3dwkcdzPxa5Jmuqi2bBArGt3ZWisrsCqBTXsz4QZH, Training speed cmsg_01Mei3dwkcdzPxa5Jmuqi2bB6AchEzy6PMTn6jEH7uZ5gk.
- Outside sessions (10-21/22, let finish, don't message): session_0155kwr6ZPrC8bXE2JopsdJ4, session_015nhaDw5whSLGJcrxHCt9uX, session_01CfwaroVA7ncQdU7qtDEcJy.

## Topic index
- Teacher: [[teacher-lane]] [[batch-design-rules]] [[far-food-1024]] [[teacher-plan-1022]] [[colsel-batches-1022]] [[colsel-batches-1022b]] [[colsel-batches-1023]] [[colsel-batches-1023b]] [[colsel-batches-1023c]] [[teacher-children-1023]] [[teacher-batches-1023-evening]] [[teacher-children-1024]] [[teacher-inbox-1022]] [[teacher-pictures]] [[colsel-judging]] [[parallel-merge-lane]] (#840)
- In-run: [[inrun-cloud]] [[inrun-batches]] [[inrun-batches-day2]] [[inrun-selection]] [[inrun-selection-traps]] [[hungry-lane]] [[brain-lane]] [[ant-brain-research-lane]]
- Dig/nest: [[nest-score]] [[dig-loss]] [[dig-lesson-oldant]] [[crater-refusal-copy]] [[inherited-nest-sinkhole]] (#834)
- Reviews/audits: [[colony-training-review-lane]] [[scoring-review-lane]] [[ant-cost-review-lane]] [[training-efficiency-audit]] [[sense-caps-audit]] (#857)
- Senses: [[far-home-sense-lane]] (#832)
- Plants: [[plant-community-lane]] (#814, shade seedling fix) [[shade-tolerance-lane]] (#818) [[rootless-plants-lane]] [[plant-food-lane]] (#863, weaning bed #836) [[soil-water-growth]]
- Perf: [[training-speed-lane]] [[soil-water-perf-lane]] [[plant-perf-lane]] [[lab-world-perf-lane]] [[fire-pass-lane]] [[creature-perf-lane]]
- Enemies: [[predator-lane]] (#852 off) [[nest-raider-lane]] (#887) [[colony-war-lane]]
- Lab/UI/other: [[lab-playtest-lane]] [[playtest-lane]] [[lab-ui-redesign-lane]] [[foraging-loop-lane]] [[maze-lane]]

## Lessons
- Check status updates for "wait until X" hand-offs; verify cross-lane messages arrived, relay if not.
- Owner questions are often for understanding: answer as a question.
- Ask lanes for to-dos vs truly parallel work separately; when a lane asks for N runners, start N.
- Don't overstate owner instructions ("no new tasks" != "stop your to-do list").
- Don't re-post replies from a thread where the owner is chatting live.
- Check numbers before relaying headlines ("19 rooms" was a score). Small-world wins (4 worlds) can reverse on 16 + lab boxes; say so.
