---
name: playtest-lane
description: Playtest thread for the new mind-brain ant and the in-run training harness (2026-10-22): findings, tools, what is untested
metadata:
  type: project
---

Thread "New ant playtesting" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBSjTAk35841TZAa9CDSv5pc), started 2026-10-22 02:55Z at Scotttt's request; exploration only, no fixes. Worktree /home/user/playtest (compute branch d71be766). Report + tools: /mnt/project-files/playtest/.

- The shipped lab ant is still the old fixed-scaffold brain; the mind-brain ant only runs in examples/inrun, so "playtesting" means that harness (100k frames ~10 s).
- Finding: kids/hall/refill start on 1 tank, founders on 3; newcomers that starve lived median 1.1k frames. kid_grants=3 paired rerun (6 seeds): starved 828->671, nest score better 6/6. Sent to Teacher lane and Foraging food loop threads.
- Harness friction: unknown args silently ignored (framez=...), ~80 keys undocumented in one place, film crop too small / ants invisible (tools/overlay.py fixes).

**Why:** a later playtest or training-process thread should not redo these. **How to apply:** see [[inrun-selection]], [[inrun-cloud]].

**Standing rule (Scotttt, 2026-10-22 05:34Z):** the Teacher lane owns the one ant. Other lanes deliver findings, lessons and teachers to it, not separate trained ants. Small test runs only; nothing long that could produce a divergent ant that won't be used.
