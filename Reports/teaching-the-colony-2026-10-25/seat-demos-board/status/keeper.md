# keeper seat: status

Thread: "Build the keeper seat demo" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bB6MX7UvF4jg8EL1hHuUi6hs). Branch: claude/project-thread-20o5zg.

Step: 3 (plan v1 under independent review, 09:20Z). Building the recall head meanwhile (scout depends on it).
Plan v1: keeper/plan-v1.md. Step-0 look: keeper/look/ (examples/keeperlook.rs on my branch).
STEP-0 FINDINGS FOR EVERYONE (CORRECTED 10:00Z by the plan review): ~~a hand-laid B trail evaporates in ~100 ticks~~ was a PROBE ERROR (my probe sat on a slumping food heap). On flat ground a hand trail decays at the lab's normal 3% per 12-frame pass: a third left at ~400 frames, 10% at ~900 (already documented, pheromone.rs:96-109). Alarm fades in ~150 frames by design (ALARM_RHO); measured in its live window it is a small graded repel (-7..-23% ant-ticks near the puff, 3 seeds). Untaught trained colonies DID find and empty a heap 150 columns out (3/3 seeds), laying their own B to it. nest_goal colonies starve out by ~frame 30k unless topped up. Adult feeds ~50-160 per 1,000 frames.
Verdict: FEASIBLE WITH CHANGES. Review: keeper/review-feasibility.md. Most "now" keeper verbs are false on the trained ant
(lamps give no heat, minds never carry brood, no emigration, dig cost ignores wetness and the pool leans away from wet,
ants don't drown). What works today: hand Dig sketch (pool's strongest weight pulls Dig onto lined walls), Alarm puff
(repels walk+dig), the larder. Demo pick: "teach a forager" (treat bead / carry an ant -> it commutes; the place spreads
by mouth-to-mouth feeding; one labelled scripted gain with an honest gain-0 key).
FOR EVERYONE: the slice-0 trail null was measured on the OLD ant; the lab runs the trained mind. Lab::new defaults to
the old ant (lab/mod.rs:657): harnesses must call set_new_ant(true).
Shared pieces: CLAIMED shared/recall-head.md (place memory a trained ant acts on; scout/mind welcome).
Using launcher.md (queen): DemoSeat::Keeper, demo_keeper.ron, src/lab/demo/keeper.rs.
scent-ladder.md (guest): the gnome as a point visitor is post-demo for keeper (gnome is a ghost; trust needs it).
Combine with: guest (keeper as symbiont: the gnome carries the guest's scent model). Talk early: guest thread.

## 2026-10-25 ~11:00Z: build steps 1-2 in a PR
- PR #907 (recall head, carry/set_down, treat, keeperlook): head 9abb556b, main and #904 merged in, handed to the merge desk.
- Off == main byte for byte (seeds 1-3); gain dose 60 cols / 1,500 frames: off 2, g2 12, g4 37, g6 124, g8 113.
- Next: step 3 demo box (demo_keeper.ron on the #904 launcher), 12-seed reach check.

## 2026-10-25 ~12:45Z: demo built, PR next
- PR #907 merged (041c9da). Demo on top of it: src/lab/demo/keeper.rs (seat keys Y carry, U treat, P listening, J overlay; knows + traffic overlays; HUD; 6k warm-up), demo_keeper.ron, examples/keeperdemo.rs.
- Result (12 paired seeds): carrying with listening OFF is already a real graded verb (station visits +52 for 10 carried, +105 for ~17; colony 120 -> 146 -> 168). Listening adds +12..+89. Meal copy neutral.
- Shared-engine changes others may care about: recall::prune now keeps dormant lessons (only the dead are pruned), SAME_PLACE 6 -> 12, RecallCounts::offered, creature::can_carry (set_down refuses stacked ants: a scripted hand must pick liftable ants).
- Page: https://claude.ai/artifact/617nmHWDrw4ZKVrgPqLEsn . Report: Reports/keeper-seat-demo-2026-10-25.md.

## 2026-10-25 13:43Z: DONE
PR #914 merged 1372548 on main (after merging main/scout in, union). demo=keeper is on main. Thread resolved.
