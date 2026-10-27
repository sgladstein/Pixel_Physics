---
name: scout-seat-demo
description: Lone scout seat demo (demo=scout): tandem runs on trained ants, findings, first-look page, status (2026-10-25)
metadata:
  type: project
  modified: 2026-10-25T11:54:21.389Z
---
Scout seat thread cmsg_01Mei3dwkcdzPxa5Jmuqi2bB4BAX9fz9kWQxgEhyKnVPS3, branch claude/project-thread-x1ms83. Built src/sim/tandem.rs + src/lab/demo/scout.rs + examples/scoutdemo.rs; stacked on #904 launcher and keeper's recall PR #907. First look for owner: https://claude.ai/artifact/C7RKKSVn3qPZdFAFMhedyr (asked: do pauses read as checking the way or stalling?). Report: Reports/teaching-the-colony-2026-10-25/scout-demo/results.md.

Rules learned (apply to any seat that drives a trained body):
- Tandem contact = follower head to ANY cell of the leader's body (tandem::contact_gap); head-to-head deadlocks.
- A steered worker must go through mind::tick (narrowed hand-off, queen only on hand-built path).
- Timeline Heap drops entomb any ant standing there.
- recall::SAME_PLACE is 6 cells: a lesson about a wide patch must be renewed on each trip.
- Lab::show_help=false before headless captures.

Results (4 seeds): ~12 sisters led per 5 min, nearly all arrive; recall gain 6 raises taught-ant trips 4/4 seeds (small); cascade 0-2/run; column arm B leads 2-3x more. Frame cost within noise. Related: [[teaching-the-colony-plan]] [[images-via-artifact]]
