---
name: merge-desk-procedure
description: How the merge desk merges PRs in Pixel_Physics (approval, method, checks) as of 2026-10-21
metadata:
  type: project
  modified: 2026-10-21T23:10:49.645Z
---

User (Scotttt, 2026-10-21) ruled the merge desk may merge on its own once CI is green, without his review.
Method: merge commit (not squash/rebase) with the full 40-char expectedHeadSha. Full procedure is in `Reports/handoff/merge-desk.md` on main (stack 2+ code PRs on a `claude/merge-desk-*` branch; logic conflicts go back to the lane; run `bash scripts/docscheck.sh` after merges).
`docscheck` exits 1 on clean main too, from a `deadendindex.py` utcfromtimestamp DeprecationWarning; compare against clean main, do not treat exit 1 alone as a failure.
Old PRs 12, 140, 143, 217, 254, 348, 456, 471, 476, 478 are ignored by ruling.

The review queue tool (`scripts/review.py`) is retired (user, 2026-10-21): questions and image/GIF comparisons go in chat or in reports linked from chat.

Lanes do not watch CI after hand-off (owner, 2026-10-21 23:10Z). If a PR fails CI or cannot merge, I message the owning lane's thread directly with what broke (see [[merge-desk-owns-pr-watching]]).

**Why:** CI is the full suite (15 checks per head); lanes never run it.
**How to apply:** relate to [[ant-brain-research-lane]] and the merge-rule line in MEMORY.md.
