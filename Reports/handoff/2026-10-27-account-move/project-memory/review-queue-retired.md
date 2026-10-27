---
name: review-queue-retired
description: Owner retired scripts/review.py (2026-10-21) — questions and image/GIF comparisons go in the chat thread or a report posted there
metadata:
  type: feedback
  modified: 2026-10-21T14:48:32.046Z
---

From 2026-10-21 the owner no longer uses the review queue (`scripts/review.py`), even though CLAUDE.md still calls it the main channel. Send every question, image and GIF A/B to the owner in the thread chat, either as attached files or in a report posted there.

**Why:** the owner reads the project chat, not the queue.

**How to apply:** where CLAUDE.md says to "post to review.py", attach the PNG/GIF to a `post_message` instead. Put the event count in the message text, which is the job the card's `meta` used to do. Related: [[merge-desk]].
