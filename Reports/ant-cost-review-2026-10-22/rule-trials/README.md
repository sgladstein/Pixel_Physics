# Rule trials from the ant cost review (Creature performance thread, 2026-10-22)

> **Landed 2026-10-24** by the report sweep (thread *Land unlanded thread reports*), from the thread's notes in
> project memory and its two patches in the shared folder `creature-perf/`. **Not re-measured.** It records what became of
> the review's section B rule changes ([`../README.md`](../README.md)). The bit-identical speed-ups the thread shipped
> are PRs #774, #785, #788, #792, #798, #801, #803, #805 and #813.

The cost review named rules that cost frame time. Speed-ups had to be bit-identical; a rule change needed the owner's
OK with numbers. Two rules were trialled behind switches, off by default:

| switch | what it does | result | outcome |
|---|---|---|---|
| `PIXEL_PHYSICS_EGG_RETRY=24` ([`egg-retry-switch.patch`](egg-retry-switch.patch)) | an egg carrier that found no pile site waits 24 frames before walking again (`OrganismState::egg_walk_after`) | labbench 24 seeds (game ant, forage bed): no gated metric moved; births higher 11/24, every p >= 0.15. The egg walk was ~11% of a mind colony's frame; the wait cuts ~9% | **shipped on** with the owner's approval (13:17Z): PR #810, merged as 5f4497a (`brood::EGG_RETRY_FRAMES = 24`; `PIXEL_PHYSICS_EGG_RETRY=off` restores) |
| `PIXEL_PHYSICS_MIND_WARMTH=off` ([`mind-warmth-switch.patch`](mind-warmth-switch.patch)) | drop the mind's warmth reads | Teacher lane's 64-seed cloud gate: food the same, colonies lost 17 vs 9 (p = 0.15) | **left on** (not shipped). "Warmth for the head only" would be the same trial plus a new input needing retraining, so nothing more to try |

Note: the in-run training bed had egg laying off at the time, so the egg-retry wait was invisible there; breeding is
on by default since PR #845.

**After #810 and #813** a mind colony's 600 frames cost 5.86G -> 5.20G instructions. Left, not done (recommended
stop): a per-chunk home count (~4%; the home definition is complex). Dead end: memoising `under_cover` per cell gave
no gain.

**Scene used**: `RAYON_NUM_THREADS=1 needscost seed=1 at=40000 handat=40000 window=600 arms=off,mind
mind=<genome>`, callgrind with `--instr-atstart=no cg=1`.
