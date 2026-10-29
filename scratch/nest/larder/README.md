# Larder seed (chambers lane, 2026-10-29) -- screened, not adopted

Question: can hand-set brain links make an ant putting food down at home prefer a spot beside food already lying
there, so the store gathers in one larder and the passages stay open? Selection may keep or drop the links; no body
rule changes.

- Sense: `PP_FOODBESIDE=1` (opt-in, src/sim/creature/options.rs, local branch `claude/chambers-larder`): on each
  Drop option that would put crop food down, the share of the target's 8 neighbours holding loose food, on option
  slot 18 (f::BUILDING); 0 on every other option. Before it, a put-down target read "air" whatever lay beside it.
- Seed (`scratch/nest/larder/make.py`, on H3 + eat seed nest=60): slot 18 -> unit 12 (-32) and unit 2 (+32).
  `lt` = target only. `lh` = also hold: the Drop job reads its best target (+10) and a food carrier's Drop job is
  lowered 16 unless a beside-food spot is in reach (a pellet carrier's is unchanged).
- Caveat found on the way: the switch alone is NOT a no-op (it equals PP_LASTFOOD=off byte for byte, which differs
  from the default), so the baseline was rerun with it on. That baseline lost lines that the earlier H3+eat run
  kept (r3 alive 0/1/extinct/71 against 58/56/38/64): this screen's run-to-run spread is that large.

Smoke, w68 round 0, sel=off (`smoke_drops`): food put down at home beside food 74% (baseline) -> 86% (lt) -> 99% (lh).

Screen, poolrounds 4 x 200k sel=score nest_r=0, worlds 68 73 61 64 (fixed) + 62 63 65 66 67 69 70 71 (added):
lines extinct by round 3: base 1/12, lh 6/12, lt 3/12. lh starved more in 6/6 surviving pairs (p .03).
No deciding measure moved in either arm (sign tests p >= .2). Full table: tables-2026-10-29.txt.

Pictures (round 3, base / lh / lt, counts on each): w64_r3.png, w71_r3.png, w67_r3.png; smoke3.png is w68 round 0.
Every arm still lines the door shaft with crumbs: crumbs already lie in one joined mass in the baseline, so a
"beside food" preference grows the mass where it already is (the door shaft) rather than moving it.
