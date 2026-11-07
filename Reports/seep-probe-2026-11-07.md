# Seep probe: a mine leaking gas, and a mould that eats it (2026-11-07)

New-game probe 4 of 6, from the brief at
`/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/prototypes/4-seep.md`.
Its own binary (`cargo run --release --bin seep`), not a lab seat; all code
is in `src/seep/` and touches nothing in `src/sim/`. Picture page:
https://claude.ai/artifact/TvVYwZUQ5Y6bsza7sUYUFK

**Status: probe built and measured; the verdict waits on the owner's
playtest.** The question is whether the mould is something a player *uses*
or decoration beside the gas. The stop rule: if a player never Damps or Cuts
after the first minute, the mould is decoration and Firedamp alone is the
honest result.

## What the scripted players say

Six paired seeds, five players, 4 minutes each (`examples/seepprobe.rs`).

| Player | Stars, seeds 1-6 | Carts | Lamp A lost | Cracks capped | Gas eaten | Gas burnt |
|---|---|---|---|---|---|---|
| nothing | 1 1 0 0 0 0 | 32-53 | 6 of 6, at 26-27 s | 1/2, late | 1,296 | 361 |
| random | 0 0 0 0 0 1 | 34-58 | 6 of 6 | 0/2 | 221 | 2,060 |
| planned | 3 2 3 3 3 2 | 96 | 0 of 6 | 2/2 on 4 seeds | 2,672 | 1,558 |
| gasonly | 2 on all 6 | 96 | 0 of 6 | 0-1/2 | 1,164 | 3,004 |
| nomould | 2 on all 6 | 96 | 0 of 6 | 0/2 | 0 | 4,111 |

`gasonly` is the plan with no Damp or Cut (the mould grows on its own);
`nomould` is the same plan in a mine with the mould removed, which is
Firedamp alone. Gas in the game's own units; each run leaks about 4,430.
Eaten and burnt are 6-seed means. **The books balance on every run**:
eaten + burnt + vented + still in the mine + lost under water = leaked.

Paired by seed, ranked by stars then carts:

- planned beats nothing 6-0 and random 6-0, sign test p = 0.031 each. With
  three comparisons that is 0.094 after Bonferroni, so six seeds do not clear
  0.05.
- planned beats nomould 4-0 with 2 ties (p = 0.125).
- gasonly and nomould tie on all 6.

**Reading.** Doing nothing loses lamp A at 26-27 s on every seed, inside the
brief's "about 45 s". A plan holds both lamps on every seed. **The mould is
not needed to survive**: sparking pink gas away from the lamps holds both
lamps with no mould at all. Herding the mould with Damp is the only way to
the third star (every crack capped), so on this level the mould rewards
mastery rather than being required. Whether herding it is *fun* is the
owner's playtest, not a script.

Two verbs never earned their keep in the script: the plan never needed Cut,
and Blew only on two seeds.

## Frame cost

Shipped release profile, quiet machine, planned seed 1, 4 minutes: tick plus
the full repaint **mean 4.0 ms, worst 23 ms**; worst during a burn with the
mould on screen **16 ms**. About a quarter of one core at 60 Hz. There is no
settled frame to skip: the gas and the mould move every tick.

## How it is built

- **Gas** (`gas.rs`): a staggered-grid incompressible solve on the fine grid
  (one gas cell per world cell, 256x160). Buoyancy from gas and heat,
  semi-Lagrangian velocity, red-black SOR pressure over the open cells only
  (24 iterations), flux-limited (minmod) conservative advection of the gas.
  Three bands by amount: under 0.05 too thin, 0.05-0.15 burns, over 0.15 too
  thick. Burn expansion feeds the pressure solve as a divergence source.
- **Mould** (`mould.rs`): a Physarum *graph* (junctions joined by tubes),
  not an agent swarm. Tips crawl the rock surface toward gas sensed over a
  radius-6 disc, pay reserve to grow, branch, join into loops and sprout from
  fed junctions. Flow through the tubes is a Kirchhoff solve every 4 ticks;
  tubes thicken with flow and fade when idle. A fed or thick tube is wet and
  stops a flame; a starving tube dries to straw and burns like a fuse.
- **Fire**: per-cell burn timer with a refractory period, spreading through
  pink gas and dry tubes, stopped by damp rock. A large burn near a watched
  lamp blows it; a very large one fires the shipped explosion.
- **Level** "Two Lamps": lower gallery with a damp sump (mould's start) and
  a dome over crack 1; a slanted shaft to the upper gallery and lamp A; a
  drip chimney between them; crack 2 opens in the upper gallery at 90 s.

## Bugs found and dead ends, so nobody repeats them

- **Water pooled over the crack swallowed about 30% of the leak.** The tell
  was the gasonly and nomould arms reporting identical burnt and vented
  totals, "a change that moves nothing". The leak now rises to the first open
  cell above the crack, and gas lost under water or rock is counted
  (`air.buried`), which is what lets the books balance.
- **Donor-cell gas advection smeared every pocket into a uniform pale blue**,
  so the three bands never showed. Minmod-limited advection keeps edges.
- **A single vent opening empties a mine too slowly** (the exchange-flow
  limit through one hole); open cells above `sky_row` lose 3% a tick instead,
  counted as vented.
- **Judging a lamp at the moment fire first touches it** called every blast a
  singe. A lamp now stays at risk for 90 ticks and is blown if the burn window
  passes the blast mass.
- **A fixed second damp road laid at 60 s dried out** before the tips arrived,
  and they went to the roof. The plan now leads the mould by waypoints,
  damping just ahead of the nearest tip.
- **Jones-style forward sensors** do not work for a surface walker seen side
  on; tips sense over a disc (see `mould.rs`'s module doc and the dead-ends
  entry it cites).

## Open

- Graph mould against probe 2's agent swarm: probe 2 had not landed when
  this was written. Add the side-by-side when it does.
- If the owner reached for Damp and it felt good, the next level is one where
  sparking alone cannot hold; if not, Firedamp alone is the result.
