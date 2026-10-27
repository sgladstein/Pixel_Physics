---
name: merge-desk-handoff
description: Owner rule 2026-10-21: once a PR is handed to the merge desk, don't watch its CI; keep working, the desk reports problems
metadata:
  type: feedback
  modified: 2026-10-21T23:10:51.324Z
---

Once a lane hands a PR to the Merge desk thread, it does not subscribe to or watch that PR's CI. It keeps working; the merge desk tells it if something fails.

**Why:** the owner kept finding threads idle, just watching CI on PRs the merge desk already owned (2026-10-21).

**How to apply:** after messaging the merge desk with the PR number and head SHA, unsubscribe from PR activity (or never subscribe) and move to the next task. Related: [[brain-lane]] and the Merge rule in MEMORY.md.
