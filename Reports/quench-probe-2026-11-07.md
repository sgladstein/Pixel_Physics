# Quench probe: does "lid, then splash" make a dam? (2026-11-07)

**Status: round 2 built and measured (owner, 20:44Z: "You can modify the
engine as much as you want to get the game to work"); verdict pending.**
Round 2 is the next section; round 1 follows it unchanged.
Probe 5 of the six prototype lanes from the 2026-11-07 new-game brainstorm
(brief: `/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/prototypes/5-quench.md`).
Play it with `cargo run --release --bin quench`. Pictures and counts:
https://claude.ai/artifact/WXfXYnGWpgWG7jL87nTM7t

## Round 2: lid, then splash, now builds a dam

Page: https://claude.ai/artifact/Fadq9wkz9GakxhJ2GbkYVA. Plan `layered`:
two lids stacked over the dam site at x 150 (`(150,109)` then `(150,99)`),
and a splash under whichever lid the lava is flowing through, only when that
gap has run dry. Ten seeds, `vent=4`:

| plan | lava past the dam | houses burned | score |
|---|---|---|---|
| two lids, water under each (`layered`) | **71** all ten | 1 | 145 |
| same lids, no water (`layeredlids`) | 186-188 | 2 | 120 |
| do nothing | 198-237 | 2-3 | 95-120 |
| same water, no lids (`layeredwater`) | 301-320 | 3 | 95 |
| dig a pit (`sump`, 5 seeds) | 68 | 1 | 145 |
| round 1's `planned` (5 seeds) | 203-241 | 3 | 95 |

Lid and water against lids alone, and against nothing: less lava past on
10/10 seeds each, sign test p = 0.002, before correction for the comparisons
made. The seeds barely move the level (71 on all ten), so this is close to
one round per plan. The pit is not beaten: it ties.

**What changed, in the order it mattered:**

1. **A lid must hang `LID_CLEARANCE` = 4 open rows below its skirts.** With
   its skirts one row off the floor, lava through the gap crusts shut on its
   own: lids alone held back 122 cells against 125 with water. That was the
   round-1 level's lid being a wall, which no chemistry could improve on.
2. **A sill** (x 127-130, three rows up) between the forge's pit and the dam
   site: splash water ran back into the pit and quenched the forge (forge
   burned 296 cells against 415 with the same lids and no water).
3. **The flood is off by default** (`Quench::flood_seconds`, `flood=8` in
   the probe puts it back). At 8 s it stood three rows up 3-4 houses in every
   plan, and splash water draining to the village counted as a flood too;
   it scored itself and nothing else. "Flooded" now also means three rows of
   standing water, not a wet doorstep.
4. **Engine: water quenches lava through crust** (`fire::try_react_through_crust`,
   switch `World::crust_quench_depth`, 4 in this game, 0 everywhere else). A
   molten cell looks straight up through at most that many cells of solid or
   powder for water, and reacts at `chance * 0.6^cells`. Guard:
   `water_quenches_lava_through_thin_crust_only_when_a_game_asks`.

**Which engine rule did what** (the same plan, three seeds):

| lid drip (`F`) | through crust (`C`) | lava past |
|---|---|---|
| on | on (the game) | 71 |
| on | off | 104-170 |
| off | off (shipped engine) | 115 |
| off | on | 211-217 |

So on the shipped engine the round-2 level and plan already beat lids alone
and doing nothing; the two rules together take 115 to 71, and neither alone
helps. Water-through-crust without the lid drip is worse than both off.

**Where it is weak.** It wins only at this eruption size: at `vent=3`
(a third more lava) lid and water let 435-495 past against 346-395 for
nothing, because the pool overtops the upper lid; at `vent=5` lids alone
come close (92 against 70) and the pit wins outright (49, no house lost).
The thrown water drains to the village and stands among the houses (no
longer scored). The forge reads 70% in every plan but the vent block, so it
does not separate the dam plans.

Frame cost, 65 rounds: mean ~1.2 ms a tick; the median round's worst tick
7.4 ms, the single worst 46 ms.

## The question

Does the engine's own chemistry make combos fun? Lid then Splash should read
as a clever play: steam under a placed stone lid drips back onto the lava, so
one splash becomes a small quench engine that builds the dam where the roof is.

## Answer so far: no, on the engine as shipped, and not yet with help

1. **A lid the player places is not a roof to the engine.** `fire.rs`
   condensation asks `World::is_outdoors`, which reads the per-cell map frozen
   on the first frame, and that map lets the sky reach `SKY_PENETRATION` = 32
   cells in under cover. `examples/quenchfact.rs`'s first version (a 55x12
   lava pool, an 81-cell splash, a 69-wide lid 14 rows up): **81.0 of 81
   cells went to the sky** with no lid, with a lid present before frame one,
   and with a lid placed after it. Fixed for this game only by
   `World::set_game_roofed`, which the game writes under each lid; with it,
   12.2 cells left by 2 s (the rest evaporated off the hot crust later).
   Inert for every other game: empty unless written, and read only by
   `is_outdoors`, whose one caller is condensation.
2. **Steam leaves sideways before it drips** (flowing stream, five seeds,
   means; `quenchfact gap=6 r=4 every=2`): flat lid **4 drips**, engine ran
   2.0 s against 1.3 s open. A cup lid (three-row skirts) **41 drips**, 5.0 s,
   +27 rock in the site. A lid held at 15C: 5 drips (heat crosses from a gas
   slowly). So the game's lid has skirts, and a switch
   (`Quench::fast_drip`, key `F`, on by default in this game) makes steam
   in a declared roof bead on the roof's underside at once
   (`quench::bead_under_roofs`, fill carried exactly).
3. **The first splash crusts the lava, and later water lands on the crust.**
   One splash made ~136 quench events at radius 4 *and* 6, lid or not; with
   the bead rule a flat lid dripped 72-108 times and quench and rock did not
   move (137 events, 75 rock). The crust is the barrier; the drip direction
   was never the limiting step.
4. **In the level, the lid-and-splash plan scores below doing nothing** and a
   shovel beats every water plan (`quenchprobe`, five seeds):

| plan | forge % | houses burned | flooded | quench | under lid | score |
|---|---|---|---|---|---|---|
| do nothing | 70 | 2-3 | 3 | 0-8 | 0 | 70 |
| lids + splashes, bead rule on | 64 | 3-4 | 2 | 354-356 | 118 | 64 |
| lids + splashes, shipped engine | 64 | 3 | 2 | 294-296 | 90 | 64 |
| the same splashes, no lids | 65 | 3-4 | 3 | 469-471 | 0 | 65 |
| dig a pit in the dam site | 77 | **0** | 4 | 0 | 0 | 77 |
| every splash at the vent | 6-8 | 3-4 | 1-2 | 427-503 | 1-2 | 6-8 |
| random tools/places/times | 15-70 | 3-4 | 2 | 443-630 | 0-55 | 15-70 |

Pit against nothing: fewer houses burned on 5/5 seeds, sign test p = 0.06;
lids+splashes against nothing: lower score 5/5, p = 0.06. Nothing passes
p < 0.05 at five pairs, before any correction for seven comparisons.
**The seeds barely move this level** (counts near-identical across seeds,
even after the vent's mouth was made seed-dependent), so the table is
close to five copies of one round; read it as one sample per plan.

What the pictures show: lava fills around the lids and then climbs over
them; the lid is an obstacle, not a roof that builds a dam.

## What the design got right, measured

- **Doing nothing loses** (2-3 houses burned) and **blocking everything
  loses the forge** (6-8% against 70%): D's "the forge needs some lava"
  tension is real and cheap. It needed the forge to *use* lava
  (`FORGE_EATS_EVERY`); without a sink all 1,100 cells buried the village
  in every plan.
- A splash shows a result within a second: first quench 0.02-0.05 s after it.

## Level facts that cost time

- Three cells a tick from the vent drowned the whole gorge in 30 s; a pillar
  under the overhang walled the vent's pool in; the vent sealed itself under
  its own crust at ~20 s (it now re-melts its mouth).
- A forge placed past the village cannot be fed in a side view without
  crossing the houses. It sits upstream of the dam instead.
- A flat village loses every house to every plan; on steps, the lava's
  height grades the loss.
- 0.25 cells a tick on a flat floor crusts at the vent as a one-cell film and
  never flows (`quenchfact` first stream).

## Frame cost

256x160, headless, this container: mean ~1.2 ms per tick including the
game's census; worst single tick 4-20 ms in most rounds, one 39 ms spike.

## If the owner wants to keep going

Two levers the engine offers and this probe did not try: crust that breaks
under falling water (so a drip reaches fresh lava), and water that is
already waiting when the lava arrives (`quenchfact at=0.5` stopped a stream
on a flat floor, but the water had spread to the vent, so that measurement
is a confound, not a result). Or treat the pit's win as the finding: on
this engine the satisfying verb for lava is the shovel.
