---
name: land-before-resolve
description: Owner rule (2026-10-24): a thread's reports, findings and rules must reach the repo (Reports/, how-we-test.md) via the merge desk before it resolves; how the first sweep did it
metadata:
  type: feedback
  modified: 2026-10-24T15:18:02.932Z
---

Owner (Scotttt, 2026-10-24 15:04Z): threads must not do a lot of work writing reports and then resolve without it reaching the repo, along with the general project guidelines, so that future sessions can find it. Relevant findings also go as messages to the active sessions they affect. The project instructions now say the same: a thread is not done until (1) its reports, findings and decisions are in the repo (Reports/, and testing rules in Reports/how-we-test.md) through a PR to the Merge desk, (2) lasting rules are in project memory, and (3) affected active threads have been messaged (the Teacher lane for anything about ant training).

**First sweep (thread "Land unlanded thread reports", PR #864, docs only, head 369b497e):** 49 threads and 35 shared-folder subfolders were checked against main. About 20 were already landed, and 3 were in open PRs (#859, #860, #861). These gaps were filled: body genes, dig-scoring (why digging dies, old-ant dig lesson, roomshape), the crater review, the in-run design-doc adversarial review, in-run state 10-21, dig-on-copy-v2, moisture-settle gate, fast-drainage growth check, a Teacher lane batch log colsel-1022 to farfood3 (compiled from memory), inherited nest (#834), cost-review rule trials, soil-water stress (#777), and training speed. Nine traps were added to how-we-test.md §4.
- After the sweep: the plant-food lane's two findings files land in PR #863. The Teacher lane's maintained first-hand batch log is Reports/brain-training/teacher-training/colony-selection-log-2026-10-24.md (PR #862); my compiled batches-2026-10-22-to-24.md now points there. Append new batches to the Teacher lane's log, not mine.

**How to check cheaply:** list the shared folder (never grep its contents, since it is a network mount), take each .md's H1 title, and `git grep -F` it on origin/main under Reports. Then grep key switch or variable names, because titles change when a report lands. A file with the same basename is not proof that the report landed (many folders use README.md).

**Why:** several resolved threads (Dig and nest scoring, Body genes, Inherited nest, the perf lanes) had shipped their code, but their findings lived only in /mnt/project-files and memory.
**How to apply:** before resolving any thread, land its write-ups, then message the lanes they affect. When a later sweep is asked for, start from the method above. See [[merge-desk]], [[teacher-lane]].
