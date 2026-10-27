---
name: evolvable-pheromones-history
description: Evolvable pheromones lane run history 2026-10-25 (decode, B fade, first emergence test, follow-ups) -- detail behind evolvable-pheromones-lane
metadata:
  type: project
  modified: 2026-10-25T17:57:18.048Z
---

Full numbers are in Reports/brain-training/evolvable-pheromones-2026-10-25.md; this is the arc. Parent: [[evolvable-pheromones-lane]].

- **Adversarial review** put emergence <10% under today's training. Owner (05:12Z) chose: decode -> tune as constants -> emergence test; palette only if a convention forms.
- **Decode** (std_pool, sel off, 32 seeds): A is load-bearing (deaf: nest 432->51); B laid only by laden ants; alarm laid by empty-away ants and does nothing (a free channel). PP_SCENT_SILENT == deaf by construction (useless arm).
- **B fade 0.25**: better with sel off (stored 441->495, nest 432->528) but LOST under selection (Teacher price25b: nest 214 vs 501, more starving). Selection bred more foragers that dig less. B stays 0.03. Dead end creatures:065.
- **First emergence test** (#915): free arm relied on alarm but median colony 9 vs deaf 284. Drift died by gen 20. Dead end creatures:066.
- **Follow-ups** (owner 15:00Z, prefers to continue). Independent review: the test could not pass (alarm reach ~3 cells, one-seed eval r=0.23, nest score paid huddles). Constant-alarm control: 5-8/8 extinct at any level vs 0/8 real map, so the reliance was information. Chimera is social. Check 3 (B row from zero) failed 2/2; check 3b (dense start) passed. A graft of the re-evolved row onto the founder is worse (co-adapted).
- **Runner lesson:** a nohup'd process dies when the container sleeps. Use Bash run_in_background (2 h max) and relaunch; emerge.py resumes. Pace is ~1.25 min/gen at N=16, EVALS=2, P=4.
- **Pictures:** Ant Scent Lab https://claude.ai/artifact/XiD6BBQtmyZ9iF9qJYGDV3 (owner can't see chat images).

- **State 2026-10-26 08:00Z:** new-channel run DONE (owner approved 19:06Z). 2 replicates x 150 gens: real signals emerged and became load-bearing (rep1 empty-ants searching mark, rep2 traffic map), but deaf controls foraged 1.4-2.2x better (15-16/16); not the emission bill (PP_EMIT_FREE). Add-on test (channel on deaf best, 60 gens) free 410 < founder 478 < deaf 499. Results PR #929 to merge desk (report s9.5-9.7, dead-ends creatures:067). #919 merged 22bec6a. Decision card pending: stop the line if add-on fails (recommended yes). Teacher lane told: keep emit rows frozen. Data: /mnt/project-files/evolvable-pheromones/newchan/.
Rule: judge an emergence run by each arm's best on shared seeds, not the hearing-vs-deaf gap of one genome.
Container hazards: disk filled once (old events.csv in e0/runs); container restarted once; background tasks max 2 h, emerge.py resumes.

## Summary moved from MEMORY (10-27)
- First free run hurt (median 9 vs deaf 284, PR #915); deleted food trail regrew in ~40 gens from small random init (not zero), hearers beat deaf 4/4. New-channel run (owner OK 10-25 19:06Z, PR #929): deaf foraged 1.4-2.2x better; opening channel cost a good forager 13%. Keep alarm emit row frozen, third plane on air law, emit rows not mutable. Judge each arm's best on shared seeds. Owner card pending: stop line if add-on fails.
