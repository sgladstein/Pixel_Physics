# Playtesting the new ant (in-run colony, h1 mix) — 2026-10-22

Setup: compute branch `claude/project-thread-8y5ak0` (d71be766), `examples/inrun`, the `ctl` job's arguments, founders = 30 mutants of
`h1.raw.txt`, seeds 11 and 21-26, 100k frames each (about 10 s per run on 4 cores). Nothing was changed in the code.
The shipped lab ant is still the old fixed-scaffold brain (`assets/species/ant.ron`); the new mind-brain ant only runs in this harness.

Picture: `playtest-findings-2026-10-22.png` (3 seeds). Close-up with ants marked: `nest-overlay-frame9500.png`, `nest-overlay-9000-10000.gif`.
Tools: `tools/overlay.py` (colours every ant by state over the game's render), `budget.py`, `roles.py`, `chart.py`.

## What the ant does (watching it)
- Founders go out to the food heaps within ~750-1,350 frames and stay there: 20-97% of their life at the heaps, tanks 20-90.
- 40% of ant-time is at the door, 20-33% at the heaps, 9-33% underground. Underground, a third to a half of ant-time is spent hungry (<3 tanks).
- Ants carrying soil sometimes climb the plant blades beside the door (frame 9500 picture, yellow boxes above ground).
- A founder (m05, seed 11) walked 120 columns out, climbed the bed's right wall to the lid and sat there for 8.8k frames until it was culled.

## Findings (numbers: 3 seeds x 100k frames unless noted)
1. **Every ant after the founders starts on 1 tank; founders start on 3** (`kid_grants=1` vs `grants=3`). Ants that came from the hall
   or refills and then starved lived a median 1.1k frames (n=295), kids 5.8k (n=62), founders 55k (n=23). A tank burns ~0.8 per 1,000
   frames, so a newcomer has ~1.2k frames to find food, and the median first food visit is 700-2,500 frames. Only ~29% of hall/refill ants
   (110 of 376) ever reached the food heaps; 69 of 72 founders did.
2. **Counterfactual, 6 seeds x 100k frames, paired: `kid_grants=3`** (newcomers start like founders): starved 828 -> 671, deliveries 285 -> 334,
   final nest score better in 6 of 6 (-59 -> -44, -88 -> -74, -111 -> -53, -59 -> -44, -96 -> -33, -51 -> +0.1), min headcount in 3 seeds
   stays at 24. One run per seed per arm, so it is a pointer, not a measurement; it says the headcount refill and the score are partly
   measuring the starting tank.
3. **Same genome, opposite lives.** Within family m16 the share of life at the heaps runs 0%-86%. Ants that never go out sit at 1-10 tanks and
   starve; heap residents sit at 20-90. Hungry ants spend <0.6% of their time at the heaps. This matches the lane's "hungry ants leave
   home less" probe from a different angle.
4. **Collapse is sudden.** Seed 21: 24 ants at 90k, 4 at 92k. The colony ends as one ant at 120 tanks, nothing in the game stops a
   forager gorging far past the breeding bar (5.5 tanks).
5. **Deliveries stay tiny**: 85 / 46 / 30 delivered vs 1,822 / 4,017 / 2,293 field bites (1-5%), as the lane found.

## Friction in the harness
- **A mistyped argument is silently ignored.** `framez=3000` ran the default 200k frames (21 s, no warning). The program already echoes its
  parameters; it should refuse unknown keys. About 80 keys are read; none is listed in one place.
- A job line carries ~25 `key=value` pairs, repeated in every jobs file. A preset name would stop silent drift between batches.
- The film (`film=N`) is a fixed 161x98 crop around the nest, and ants are dark pixels against a dark nest: you cannot see what they do.
  `tools/overlay.py` marks each ant by state from `trace.csv` (needs `film=N trace=N` with the same N).
- `trace.csv` ids look like 2097159; the ids are not a plain counter (births appear to add high bits), so filtering by id needs care.
- Speed is good: 100k frames in ~10 s with a trace, 6 seeds in parallel on 4 cores.
- Build from scratch on a fresh machine: 3 min 40 s.

## Not tested (yet)
- The old fixed-scaffold ant in the same bed for a side-by-side (the Foraging food loop thread owns that).
- The lab box (`--bin lab`) with the new ant: the shipped lab ant does not use the mind brain, so there is nothing to play yet.
- Long runs (>200k), selection rounds 1-5 across fresh worlds, extreme settings (zero food, drip off, `pile=0`).
