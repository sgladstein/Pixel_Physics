# Chambered nests from the brain: what was tried, 2026-10-29 to 10-30

**Status: measurement, closed with open owner decision (lab lay brake).** The owner asked (2026-10-29 04:40Z) for an
out-of-the-box exploration of how the trained colony's nest could become several chambers joined by tunnels, rather than
one hole. Hard constraint: the final behaviour comes from the brain, not a hard-coded digging rule; a rule may serve
only as a scaffold the ant keeps the behaviour without. The owner waived the project's process rules for the thread.
Pictures, in order, with counts beside each: the "Chambered Nest Probes" artifact page (sections 1-12). The brainstorm
itself is a Claude Doc. Data for every step: `/mnt/project-files/chambers/<step>/README.md`.

## Short answer

The brain can hold a tunnel digger that keeps a colony alive, but only one that is seeded by hand. The digger is the
Nest lane's hand-set dig head H3, and the colony stays alive because of an eat seed (below). Selection keeps that pair
for 16 rounds with the cone's survival and about twice its passages. It does not improve the digger, and under the
plain score a digger that can change wears down. Four things stand between that and a nest of rooms. Two are on the
world side: the lab colony overgrows, so nobody is free to dig (no lay brake), and the 40-row garden bed is too
shallow for long tunnels. Two are brain-side failures: the crumbs filling every passage are the colony's food store,
and neither of the two fixes tried for it held; and a "distance to the nearest room" sense the dig head lacked did not
turn arms into rooms.

## What each step found

| Step | Verdict | Numbers (paired by world, sign test) | Where |
|---|---|---|---|
| 1. Rain floods the cone (`PP_FLOOD_HARM`) | no | tunnel gain 10 up / 6 down, p .45; water underground soaks in <1k frames | `chambers/flood/`, branch `claude/chambers-flood` |
| 2. Food buried in the soil (`PP_BURIED_FOOD`) | no, hurt tunnels | tunnel alive 52 -> 31, 6/16 extinct; ants camp at a pocket and starve | `chambers/buried/`, `claude/chambers-buried` |
| 3. Layered soil (`PP_BED_LAYERS`) | no | gallery share up 8/8 only because stone shelves count as open; a cut costs the same whatever the hardness | `chambers/layers/`, `claude/chambers-layers` |
| 4. Why tunnel nests lose ants (trace) | cause found | H3 vs std_pool, 4 worlds x 4 rounds: underground starvation 947 vs 268, a median 2-3 cells from stored food; 95% of hungry home pickups are put straight back, in both ants | `chambers/trace/`, `scratch/nest/chambers/` |
| 5. Eat seed | yes for survival | H3+eat alive at round 3 up 4/4 (7->58, 18->56, 25->38, 47->64); Teacher exam (game, 600k) level with std_pool | `scratch/nest/eatseed/` |
| 6. Digger retunes on top; larder seed (`PP_FOODBESIDE`) | no | retunes killed 2-5 of 8 lines; the larder seed grew the one store in the door shaft | `claude/chambers-larder` (`scratch/nest/larder/README.md`) |
| 7. Selection, digger frozen (chambers-sel1) | keeps tunnels | r15: H3+eat vs cone passages 26/5 p 2e-4, alive 15/17; shape score added nothing (17/15), alive 10/22 p .05 | `chambers/sel1/` |
| 8. Selection, digger "free" (chambers-sel2) | invalid | all 19 H3 head links deleted (see dead ends) | `chambers/sel2/` |
| 9. Selection, digger tunable (chambers-sel3) | not confirmed | tunable vs frozen under shape score 23/9 p .02 at r15 only (r8-14 level), fresh-world garden 20/12 p .22; plain score: tunable worse, extinct 10 vs 2 | `chambers/sel3/` |
| 10. Lab played bed | cause found | no lay brake: 150-400 ants, 2,400-4,900 starved by 600k; P(dig) at home 0.003 vs garden 0.021; `lay_cap=off` in the garden reproduces it 4/4 | `chambers/labbed/` |
| 11. Crumbs | cause found, two fixes failed | 72% of standing crumbs re-handled by fed ants at home; eat-at-home crumb share 25% -> 9% but 3/8 lines dead; store sense crumb share 25% -> 66% (8/8) | `chambers/crumbs/` |
| 12. Room-distance sense (`PP_DIGROOM`) | no | 88-90% of digs within 4 cells of open space; rate-raising weightings cone, rank-only level (3/5, p .73) | `chambers/chambersense/` |

## Mechanisms worth keeping

- **The eat seed** (`scratch/nest/eatseed/make.py POOL OUT nest=60`): brain weights only. The Eat switch g[6] goes from
  -0.95 to +1; the Eat job's row is zeroed, then given energy -60, at_nest +60 and constant -38.4; the Eat hidden-unit
  and memory links are zeroed. A hungry ant at home eats what it holds, and a full ant, or one away from home, almost
  never does. A first version that let ants eat anywhere killed 3 of 8 colonies: carriers ate their loads in the field.
  Every carry of 16 rounds of selection still holds it.
- **The `tunnels` passage measure** (`scratch/nest/selrun/tunnels.py`). It counts open passage cells without depending
  on reach from the door, so a crumb plug in the door neck cannot zero it. Controls: H3 nests beat the trained ant's in
  8/8 worlds, and no crumb-packed copy pit scores above every H3 nest. It does count the edges of sunk soil (lab
  "craters") as passages, so look at the picture.
- **Instruments**, all no-ops when unset:
  - `inrun PP_DIGHEAD_RATE=F` / `PP_DIGHEAD_GROW=P`: the dig head's own mutation rate, and its own add-or-drop event
    chance.
  - `labsurvive laycap=T:D` and `nestrows=N`: the default nest map cuts the played bed at 40 of its 80 rows.
  - `PP_DIGTRACE=path`: one sampled dig-head row per Dig decision.
  - `PP_ROOMBESIDE`, `PP_STORESENSE`, `PP_DIGROOM`: three senses.
  - The crumb-source tracer (`scratch/nest/crumbs/`).

## Corrections to what the project believed

- **The lab colony being uncapped was already on record.** `far64-lab-2026-10-24` found that `lay_cap` lives only in
  `inrun`. This thread rediscovered it from the nest side: without the brake, almost no ant is idle at home to dig. With
  `laycap=48:8` in the lab, starvation fell 5-10x and the nest grew in 4/4 seeds, to about a third of the garden's. A
  second suppression is still untraced: fed, uncrowded ants at home dig at P 0.005-0.009 against the garden's 0.021.
- **"Alive is level" in lab pictures hides mass starvation**, and some lab "craters" are soil that sank to the stone
  with 1-2k digs, not nest.
- **The crumbs are not scraps.** Eating starts the moment a cell is picked up, so every food cell set down counts as
  crumbs. The store is the colony's buffer between food patches: eat it and colonies starve in the gaps (38 ants at once
  in w62). Re-handling by fed ants is also one of the ways the store gets eaten, which is why blocking it grew the
  store.

## Next, in order

1. A lab lay brake (owner decision pending), measured by the Teacher lane, before any chamber work in the main game.
2. Deeper soil (`PP_BED_SOIL=60`, PR #986) for any garden chamber test, since every nest hits the floor by round 2.
3. Only then: room10 (rank-only room sense) with a tunable head under the shape score, and a stored-food-per-ant sense
   for the crumbs, so the store is eaten down to a reserve rather than moved.
