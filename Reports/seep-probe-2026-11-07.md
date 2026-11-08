# Seep probe: a mine leaking gas, and a mould that eats it (2026-11-07)

New-game probe 4 of 6, from the brief at
`/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/prototypes/4-seep.md`.
Its own binary (`cargo run --release --bin seep`), not a lab seat; all code
is in `src/seep/` and touches nothing in `src/sim/`. Picture page:
https://claude.ai/artifact/TvVYwZUQ5Y6bsza7sUYUFK

**Status (2026-11-08): playtested twice. Round 2 made the level need the
mould for the third star; the owner then played it (gas and Spark
satisfying; Blow not; the mould's idea liked but "I click water along the
edge and then it kind of slowly grows there"; instructions and "doing well"
unclear). Round 3, at the end, answers those four. Waits on the owner's
third look.** Round 1 below
is the first build, kept as measured.
**Round 1 status: probe built and measured; the verdict waits on the owner's
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

## Round 2 (2026-11-08): fixes from the playtest

Before/after page: https://claude.ai/artifact/FvUwqbkRK4DwheTaAeSdJu

The 2026-11-08 playtest (`/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/playtest-2026-11-08/seep-review.md`)
answered the probe's question: **for a fresh player the mould was decoration.** Sparking alone held both lamps; one
curious Cut by the sump killed the mould for the level; Blow was never used; doing nothing beat the 30-cart goal. Its
ranked fixes, and what was done with each:

1. **Make the level need the mould.** A dripping roof alcove beside the sump, *the stall*, with lamp B hung in its top and
   a third seep in its own wet roof. Gas pools round lamp B ("B: GAS, NO DIGGING") and nothing in the stall will light, so
   no spark clears it. Lamp B is now a **safety lamp**: it never lights gas itself, but gas at the face stops its digging,
   and a flame that reaches it still singes or blows it. The answer is to lead the mould up the sump wall and along the
   roof (a Damp stroke), where it runs fast on the wet stall, caps the seep and eats the pocket.
2. **Draw the mould the way the swarm looks.** Every tube is at least 3 cells wide, wider with flow, with a glow round the
   fed ones, and its state is its colour: pulsing bright yellow when fed, straw when starving (it will burn), grey-brown
   when cut off from the core, and a grey husk where a tube has died.
3. **Give Cut a job and make it safe.** Tubes touching the core cannot be cut ("TOO TOUGH"), the sump feeds the core a
   trickle, and a mould with fewer than 2 tips sprouts from the free ends of what is still rooted. A dry tube burning
   within 32 cells of a lamp sets off a "FUSE! CUT IT" alarm.
4. **Crack 2 hisses for 30 s before it opens**: a growing hairline, "HISS", and a banner.
5. **Stars are carts**: 40 / 65 / 85 of a possible 96, shown against the next star. A choked, singed or lost face hauls
   nothing, so the score is the thing the lamps are for.

### Before and after, same scripts, 5 paired seeds

The playtest's strategies were added as scripted players (`script.rs`): `fresh` sparks the biggest pink pocket every
2 s wherever it is and tries each other tool once (including one Cut by the sump at 20 s); `freshdamp` is the same
without the Cut, damping just ahead of the mould toward the nearest open crack after the first minute; `mouldonly` is
the plan without Blow, Spark or Fuse; `ring` damps rings round the lamps and nothing else. "Before" is the merged
game (PR #1031) with these players added and nothing else changed. Stars below are carts-stars on both sides, so the
columns compare (the old game's own stars rule is in its runs).

| Player | Before: carts | Before: stars | After: carts | After: stars | After: lamp B choked |
|---|---|---|---|---|---|
| nothing | 32-53 | 1 1 0 0 0 | 28-29 | 0 on all 5 | 116-121 s |
| planned (mould led to the stall, then crack 1, then crack 2) | 96 | 3 on all 5 | 90 | 3 on all 5 | 26-27 s |
| gasonly (the plan, no Damp or Cut) | 96 | 3 on all 5 | 71-72 | 2 on all 5 | 118-121 s |
| fresh | 96 | 3 on all 5 | 71-72 | 2 on all 5 | 118-121 s |
| freshdamp | 96 | 3 on all 5 | 77-78 | 2 on all 5 | 87-91 s |
| mouldonly | 53 | 1 on all 5 | 47 | 1 on all 5 | 26-27 s |
| ring | 53 | 1 on all 5 | 27-29 | 0 on all 5 | 117-129 s |

- **The mould now earns the third star and about 18 carts**, and nothing else does. The plan that leads it beats sparking
  alone on 5 of 5 seeds (sign test p = 0.0625: five seeds cannot clear 0.05, and the seeds barely differ -- the stall's
  timing is set by the rock, not the seed, so this is a deterministic level more than a sample).
- **Left alone the mould finds the stall at 105-106 s**, too late to save most of lamp B's shift; led by a Damp stroke it
  caps the seep at 26 s.
- The damp-ring cheese is gone (0 stars), and so is the curious snip: it lands on the core and does nothing.
- **Still unanswered: Cut and Blow.** No tube caught fire in any of the 35 after-runs, so the fuse alarm never fired and
  no scripted player cut anything; Blow was used only by the plan's upper-roof stroke. Cut's rescue is real (the paired
  test `snipping_a_fuse_saves_the_lamp`) but no level situation makes it happen yet: tubes in a pocket eat and stay wet,
  tubes outside one are dry but have no flame near them. A level where an old starved network lies between a pocket and a
  lamp is the next step for Cut.

### Two mould bugs found on the way

- **Twin tips at a corner closed a one-cell loop and both died.** A tip climbing the sump wall branched at the roof
  corner; the two tips stood side by side and each "met another tube", its twin's, on the next step. Every led mould
  stopped dead at that corner. A tip now ignores tubes within 3 cells of the junction it left.
- **"A starving tube burns like a fuse" was never true past one tube.** A burning tube lit all its cells at once on
  random timers, and only its end cell touched the next tube; when that cell's timer was short the flame died at the
  junction (8-tube dry line at a lamp: 1 tube burnt, lamp untouched). The flame now runs along the tube cell by cell,
  about 10 cells a second, and takes the next dry tube for certain.

### Tried and rejected this round

- **A naked lamp B in a stall over a floor seep**: the plume reached the lamp first and it was blown at 8 s, before
  anything could be done.
- **A safety lamp B in that same stall (seep in the floor under it)**: left alone it choked lamp B 188 of 240 s, but the
  fresh player who sparks the biggest pink every 2 s burned the plume on its way up and held lamp B on 5 of 5 seeds --
  sparking still cleared it. Hence the dripping stall with the seep in its own wet roof.
- **Regrowth from the core itself**: the core sits in the sump boxed in by its own tubes and the water; a tip put there
  died on its first step, and 40 s after a mould was cut back to the core it had regrown one tube.

Frame cost, paired on one quiet machine and one build profile (the fast iteration profile, not the shipped one), the
plan on seed 1 for 4 minutes: tick plus full repaint **mean 3.25 ms before, 3.53 ms after** (+0.3 ms: the husk pass and
the wider, glowing tubes); worst 19 ms before, 12 ms after, which is within run-to-run noise.

## Round 3 (2026-11-08): the owner's own playtest

Page: https://claude.ai/artifact/3NDjQ5H1shLGDboZiZVSwW

The owner, verbatim: "The gas is fun and satisfying. Sparking it satisfying blowing is not very satisfying. I like the
idea of the mold growing. It's still not super satisfying. I just kind of click water along the edge and then it kind of
slowly grows there. The instructions need to be better. When you're doing well it needs to be clearer."

### What changed

- **The mould answers a Damp stroke at once.** On water the player has just put down (damp above the rock's own) a tip
  runs at 12 cells a second (`RACE`, was `FAST`, 4.2), slowing as the stroke dries over ~20 s. For 4 s after a stroke,
  any tip within 48 cells leans toward it and moves at damp speed even over dry rock; if no tip is in range a new one
  sets out from the nearest rooted junction. The same hand stroke up the sump wall and along the roof (2 s, seed 1):
  the mould's first tube enters lamp B's stall 2.0 s after the stroke ends, against 6.9 s before; lamp B digs again
  21.8 s after it, against 27.8 s -- eating the stall's pocket, not the journey, is now most of the wait. Fresh damp glistens, and a racing
  tip burns white with a halo.
- **Blow is a gust you can see.** Radius 6 to 8, push 0.035 to 0.055 a tick, centred 2 cells ahead of the hand, and
  every stroke throws pale wind streaks that ride the air (placed by a hash of the tick, not the game's dice, so they do
  not perturb a paired run).
- **Doing well is on screen.** A "+1 CART" pops at a lamp for every cart; lamp labels are green when digging, pink when
  choked, on a dark plate; the status row is a star bar (carts fill it, the three star marks light when passed, a dim
  fill ahead shows where the shift ends at the current rate) that flashes when a star is won, with a "STAR 1! NEXT STAR
  AT 65 CARTS" banner. "DIGGING AGAIN" when a choked lamp clears.
- **A coach.** Four one-time tips, each with a pulsing ring round what it is about: the first pink pocket ("PINK GAS
  BURNS. PRESS 2 AND CLICK IT"), gas nearing lamp A, lamp B choked for 3 s ("NO SPARK LIGHTS HERE"), and after 25 s of
  choke, the mould's job ("THE MOULD EATS GAS. DAMP A PATH TO LAMP B"). Banners now queue rather than overwrite, a
  displaced one goes back to the head of the queue, and advice about a lamp is dropped when that lamp is lost.

### Before and after, same scripts, 6 paired seeds (1-6)

| Player | Carts before | Carts after | Stars after | Lamp B choked, s (before -> after) |
|---|---|---|---|---|
| nothing | 28-29 | 28-29 (identical) | 0 | 116-121 -> same |
| planned (mould led to the stall) | 90 | 92 | 3 x6 | 26-27 -> 16-18 (stall capped 26 -> 18-19 s) |
| gasonly (plan, no Damp or Cut) | 71-72 | 71-72, **43 on seed 2** | 2, 1 on seed 2 | 117-121 -> same |
| fresh (spark the biggest pink every 2 s) | 71-72 | 72-73 | 2 x6 | 118-121 -> 111-116 |
| freshdamp | 77-78 | 79 | 2 x6 | 87-91 -> 83-84 |
| mouldonly | 47 | 49 | 1 x6 | 26-27 -> 16-18 |
| ring (Damp rings round the lamps) | 27-29 | **45-46** | **1 x6** (was 0) | 117-129 -> 34-37 |
| random | 5-37 | 9-35 | 0 x6 | mixed |

- The plan beats sparking alone on 6 of 6 seeds (92 vs 72-73; sign test p = 0.031).
- Doing nothing is bit-identical: it never Damps or Blows, and the streaks do not touch the game's dice.
- **The ring player now earns a star**: Damp laid round lamp B calls the mould into the stall from 48 cells, so it is
  capped at 34 s. That is the brush luring the mould, which is the point; it is still one star short of the plan.
- **gasonly lost lamp A on seed 2**: a blast at 94 s in the plan's single upper-roof Blow stroke, now harder. 1 of 6
  seeds, the planned player (same stroke, with the mould) did not; recorded rather than tuned away.
- Frame cost, quiet machine, same fast profile, the plan on seed 1, two runs each: mean 3.58-3.67 ms before, 3.67-3.70
  ms after; worst 16-24 ms before, 13-15 ms after.

### Tried and rejected this round

- **A lure that sent a new tip whenever none was within 12 cells**: at a roof stroke the floor tips could not reach
  yet, it sent seven tips in six seconds (28 reserve) and none arrived. Now a tip is sent only when none is within 48.
- **Lured tips at their own speed**: a tip 10 cells from a stroke on dry rock crawled at `SLOW` (3 cells in 6 s) and,
  being "near", blocked the new tip that would have answered. Lured tips now move at damp speed.

