---
name: soil-water-perf-lane
description: Soil-water performance review (2026-10-21): the one-unit drainage conveyor is the cost, bit-identical ceiling ~1.3x on the pass, owner chose drainage rate 1.0 (both gates passed), report landed via docs PR #821
metadata:
  type: project
  modified: 2026-10-22T15:46:43.008Z
---

Fable review lane "Soil water performance ideas" (thread cmsg_01Mei3dwkcdzPxa5Jmuqi2bB48EFF15PAyGRQgB1Uz6b3X), 2026-10-21/22, resolved. Report: PR #821 (docs only, branch `claude/project-thread-mblj3z-docs`) puts it at `Reports/soil-water-performance-ideas-2026-10-21.md` with `Reports/soil-water-perf-2026-10-21/` (pictures, both labbench tables); copy also at `/mnt/project-files/soil-water-perf/`. Scratch branch `claude/project-thread-mblj3z` (examples/soilcost.rs, env switches SCRATCH_DRAIN_RATE / SCRATCH_DRAIN_SNAP / SCRATCH_SKIP_INFILTRATION / SCRATCH_SCHEDULE_INFILTRATION; never for main, no PR).

- Played lab box, one thread: 2.42 ms a tick, soil pass 0.99 ms (41%), 5,980 drainage writes a tick, 63% of one unit; 75 ns a visited cell. Cause: `(surplus*0.25) as u16` truncates, so the bed rests at FC+3 and every drop becomes a one-unit conveyor to the floor drain.
- Bit-identical ideas sent to the lab-world lane (owns the pass after #771): infiltration-read skip (-9.6% Ir, needs a per-block liquid mask to fire in the played box), material tables, merged writes. Ceiling ~1.3x on the pass.
- Owner ruled 2026-10-21 23:48Z: visuals don't matter, take the fastest rule if plants are unaffected. Both gates passed 2026-10-22 (plant rungs: no difference beyond a rate-0.26 noise arm; ant bench 12 seeds: every row inside noise, run 220 -> 180 s). Lane closed by the owner 00:46Z (Fable cost); lab world lane lands rate 1.0 at both sites (drain-down AND root-mat pull) plus the schedule fix, plant perf lane re-runs its rung gate on that commit.
- Drainage rule sizing: "Surplus under 16 leaves whole" keeps the plume, played box 2.42 -> 1.59 ms; rate 1.0 -> 1.31 ms but the plume becomes a block. Both: bed at exactly FC, top 3 rows 10-25 units drier. Gate: `scripts/labbench.py` 12 seeds paired (the hold band failed it the other way).
- Coupling found: the drying schedule is armed only on a net change in the visit; a cell that drinks a drop and passes it all on is never armed (evaporation stops under rate 1.0). Fix "arm on drank too" is not bit-identical on the played box.
- Numbers explained to the owner: 46% is the tick window, 40-45% the rung training beds, 18% a whole 120k-frame lab run with a growing colony (the soil pass is a smaller share once ants and plants fill the box).

**Why:** the next session on soil-water cost should not re-derive the conveyor or re-try bit-identical work past the ceiling.
**How to apply:** rerun `soilcost` from the scratch branch for a paired number; any rule change goes to the owner first. Related: [[plant-perf-lane]], [[lab-world-perf-lane]].
