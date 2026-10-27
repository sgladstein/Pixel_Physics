---
name: merge-desk
description: Project rule since 2026-10-21 — a merge desk thread does all merging and full test suites; lanes only open PRs and hand off
metadata:
  type: feedback
  modified: 2026-10-21T13:57:35.442Z
---

Since 2026-10-21 (owner, via coordinator) a Sonnet-level **Merge desk** thread (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBBjZgzija4YCh6ciFtqnPbN) owns all merging and runs the full test suite.

**Why:** owner wants lanes not to spend time on full suites or merges.

**How to apply:** in a work thread, run only the touched module's tests, open the PR, then `message_thread` the merge desk with PR number, head SHA, and any ordering constraints (e.g. which overlapping PR must land first). Don't merge yourself, and don't ask the owner whether to merge.

**Don't watch CI after hand-off (owner, 2026-10-21 23:10Z):** once a PR is with the merge desk, unsubscribe from its activity and keep working; the merge desk reports any problem.
