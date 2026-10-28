# Copying the room rule into the dig head (nest lane, 2026-10-28) -- dropped

SCRATCH. The record behind `Reports/dead-ends.md` creatures:192. Owner's pick 2026-10-28: teach the evolving dig head the
old ant's room rule (`PP_BODYRULES=rooms`, `creature.rs` `shaft_room_refuses`), then fade it. Three copies failed; none
made passages with the rule off.

- `rooms_trace.patch` -- the trace logger (not on main): `PP_ROOMS_TRACE=path` in `mind.rs` `dig_prepare` writes one CSV
  row per Dig option below the datum: frame, org, x, y, verdict (0 allowed, 1+ShaftWhy refused), the 12 dig features,
  the 45 option features, the head's bonus, then four raw open-run senses. Flushed per row.
  Runs: `inrun` from std_pool, seeds 500004-500011, 100k, `sel=off nest_r=0`, rule off and `PP_BODYRULES_P=0.5`.
- `fit2.py` -- one founder's head fitted to the verdicts (rank only): per decision, the chance of picking a refused cut.
- `fit3.py`, `fit4.py`, `fit5.py` -- plus the "when" read-out: per-option hinge (fit3), on the decision's max (fit4),
  one-sided so digging may only fall where the rule refuses every cut (fit5).
  **A layout-5 std_pool genome is core 2797 + head 482 + lay gate 6: the head is `g[2797:3279]`, not the tail.**
- `evalwhen.py`, `stop.py`, `stopmax.py` -- does the when read-out separate the rule's "stop" decisions (AUC, by colony age);
  can the senses predict it at all (decision-level logistic 0.92, a fresh when-shaped head 0.94).
- `score.py` -- gallery share (nest space outside any 5x5 open block), width, alive, starved per round dir.
  **It is fooled by a narrow crumb-packed shaft**: copy 3 scored 20-41% on nests that have no passages. Look first.

Results (8 worlds 61-68, `scratch/nest/poolrounds.sh ... nest_r=0`, rule off):

| arm | round-1 gallery | vs control | lines dead within 3 rounds |
|---|---|---|---|
| control (std_pool) | 3-15% | | 0/8 |
| copy 1, rank only | 8-19% | 5/8 | 0/8 |
| copy 2, + stop (fit4) | 13-42% | 8/8 | 4/8 |
| copy 3, + one-sided stop (fit5) | 20-41% (a shaft, not passages) | 8/8 | 5/8 |
| rule on (not a copy) | 55-75% | | 5/8 within 12 rounds (control 2/8) |
