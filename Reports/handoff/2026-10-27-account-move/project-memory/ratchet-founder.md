---
name: ratchet-founder
description: Owner rule 2026-10-24 — every training batch starts from the latest best pool, never from the old starter ant (hcs); no old-ant rule scaffold once the pool digs rules-off
metadata:
  type: feedback
  modified: 2026-10-24T03:25:54.595Z
---

Owner, 2026-10-24 03:13-03:15Z: "Why do we start each training session with the old ant roof rules?" and "Shouldn't every next test be using the best ant from the previous training, so we are always getting better and better".

Rule:
- The founder of every batch is the current best carried pool. That means a colony's top 30 by score at the final round, judged rules-off in the common garden.
- When a batch finishes, its best balanced colony becomes the next founder pool. Look for high nest, low crater, low starved and rooms > 0.
- A/B arms still share one founder, but it is always the latest pool. Never go back to hcs or h1.
- No body-rule scaffold (PP_BODYRULES / wean) when the founder pool already digs well rules-off.

The first standard pool is assets/minds/std_pool.txt: brood1ctl colony 7, round 31, 30 ants. Its CG was nest 377, crater 3, starved 3, rooms 14. colsel.py takes a `*pool.txt` as GENOME.

**Why:** batches restarted from hcs every time, so the line never improved between batches.
**How to apply:** update the pool file (or point the job at the new pool) after each batch. Related: [[teacher-lane]], [[no-oldant-body-rules]].
