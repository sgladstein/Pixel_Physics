---
name: merge-desk-owns-pr-watching
description: Owner rule (2026-10-21): lanes do not subscribe to or watch their PRs; the merge desk does
metadata:
  type: feedback
  modified: 2026-10-21T22:52:28.826Z
---

Lanes do not watch their own PRs (no subscribe_pr_activity, no CI babysitting). Open the PR, hand it to the merge desk thread, and go back to the work. The owner said: "You don't need to watch a PR. That's the merge desk's. Continue with your work."

**Why:** the merge desk owns merging and CI; a lane watching too is duplicated effort that pulls it off its own work.
**How to apply:** after handing a PR to the merge desk, unsubscribe if subscribed and continue. Fix CI only if the merge desk reports a failure back. Related: [[plant-perf-lane]].
