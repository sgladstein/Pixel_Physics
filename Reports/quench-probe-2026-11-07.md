# Quench probe: does "lid, then splash" make a dam? (2026-11-07)

**Status: probe built and measured; verdict from the owner pending.**
Probe 5 of the six prototype lanes from the 2026-11-07 new-game brainstorm
(brief: `/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/prototypes/5-quench.md`).
Play it with `cargo run --release --bin quench`. Pictures and counts:
https://claude.ai/artifact/WXfXYnGWpgWG7jL87nTM7t

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
