# Kudzu probe (probe 7): a cut that drops a whole sheet of vine

*2026-11-07. Brief: `/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/prototypes/7-kudzu.md`
(with `common.md` and `plant-engine-rules.md` beside it). Play: `cargo run --release --bin kudzu`.
Harness: `examples/kudzuprobe.rs` (row in `Reports/instruments.md`). Picture page:
https://claude.ai/artifact/BuCjsNG2G41TKGPr5eC327. Status: built, waiting on the owner's playtest.*

## The question

Is a cut that drops a whole sheet of vine satisfying, and can a player read which stem feeds which arm
well enough that cutting is a deduction rather than a guess? The hover preview (everything that would
fall lights up pale) is the bet.

## What was built

A 256x160 box: garden soil, two stone walls, one vine crown in a crack at the left wall's foot. The
vine grows 2,400 frames overnight; play is three minutes to dawn. Shears: 3 charges, one back every
8 s, a blade 2 cells in radius. Dig: hold 3 s on the roots, one per level. Stars at 50/75/95% of the
wall faces (two columns either side of each wall, two rows over its top) clear at dawn. The bar shows
both tools, the clock, cuts, dropped, regrown, walls clear, the vine's mood (THRIVING / HUNGRY /
STARVING, from the engine's own starvation test) and what the next click will do.

Not built: gutter lever, salt, torch, sound, the hill village.

## Results (12 paired seeds, final build)

| | walls clear at dawn, mean | stars 0 / 1 / 2 / 3 |
|---|---|---|
| do nothing | 42% | 7 / 2 / 3 / 0 |
| random snips | 88% | 0 / 2 / 6 / 4 |
| planned (cut where the preview lights most) | 99% | 0 / 0 / 1 / 11 |
| dig the crown, then planned | 99% | 0 / 0 / 0 / 12 |

Paired sign tests on walls clear at dawn (four comparisons, so the bar is p < 0.0125): planned beats
nothing 12-0 (p = 0.0005); random beats nothing 12-0 (p = 0.0005); **planned against random 9-2-1,
p = 0.065, not significant**; dig-first against planned 3-4-5 (p = 1.0).

- **The box is too easy.** Random snipping earns 2-3 stars on 10 of 12 nights. Scarcer cuts, a stronger
  vine or the village's hidden crowns are the levers for the plan to matter.
- **Doing nothing earns 1-2 stars on 5 of 12 nights**, where the overnight vine was weak (52-77% clear at
  the start) and spread little. On the other 7 it loses.
- **The preview shows what falls now, not what starves later.** A cut that parts the shoot from its roots
  drops few cells and kills the rest over the next minute. On seed 9 the planned script ended with no
  vine; on seed 1 a random cut did it by luck and beat the plan (100% to 93%).
- **Regrowth is a threat**: 700 cells regrow on a night nobody touches, ~2,000 under random snipping.
- **Readability**: one cut a quarter of the way up the left wall's face lights the whole wrap of that wall,
  both faces and the top (569 cells, seed 1). The still/preview pair is on the page.

**Frame cost** (world step alone, idle machine, 60 Hz): at the biggest vine (2,880 cells, seed 9, nobody
cutting) mean 1.3 ms, p99 11.8 ms, worst 21.6 ms, 13 of 10,800 frames over 16.7 ms. While a cut sheet
falls, worst ~8 ms. The spikes are the vine's own organism tick at 6x on one 2,500+ cell plant.

## What changed in the engine, and why (all off outside this game)

Each was found by looking at the vine, then tracing what stopped it. In order:

1. **`World::plant_speedup`** (1 everywhere else): divides the organism clock, and
   `World::nutrient_recovery_period` scales soil-nutrient recovery with it (at 6x the vine's nutrient
   status went 0.10 -> 0.00 in 1,600 frames without it).
2. **`World::climber` / `plant::Climber`**: for one species, a tip within 2 cells of stone or soil skips
   the turgor height bound; candidate preference is mapped onto 0..1 and multiplied by surface contact, so
   stems follow a wall over its top and down the far side (with a plain multiply the vine covered the two
   facing sides and stood still all game, 76% clear at start and dawn).
3. **Anchoring**: a climber is held by roots and by stems lying on soil, never by stone. Held from the
   wall top, the biggest single snip dropped 51 of 1,346 cells. Held by roots alone, one snip at the
   collar dropped the whole vine every time and nothing regrew.
4. **`sprout_climber` / `sprout_climber_root`**: the game's runners and roots. The seedling's first root
   tip retired "too poor" within five visits at 1x, 2x and 6x alike, leaving 2-4 root cells under 600 cells
   of shoot (water 0.03-0.08, dead inside 1,800 frames). Runners prefer a stem and open cell within reach
   of a surface and pressed against it; picked among all sites, most stepped off a two-cell mat and
   retired on height (seed 10: 2,185 runners, 4 cells grown).
5. **`day_budget_ratio`**: the engine's own starvation test, read-only; runners are gated on it at 1.2.
   Gated on `income` against `maintenance` the vine pushed while the starvation test said 0.97.
6. **`climber_would_sever`**: dieback never sheds a stem whose loss would strand an arm. The ring test
   counts leaves as connectors and the support walk does not; a hungry vine shed three collar cells and
   1,739 cells came off the wall in one frame. Bounded at 4 support walks a pass (unbounded, it was most of
   the vine's worst frames: 117 ticks over 12 ms against 30).
7. **The world edge is not a climb surface** (a vine climbed the left edge and ran along the sky).

Game-side, not engine: `plant_load_failure` off (the vine snapped at its collar under its own weight,
1,739 cells in one frame); starving ticks held one short of the death (a vine that shaded itself into
debt rotted away whole mid-game on half the seeds); the crown sown at the wall foot (sown mid-box, the
seedling grew a bush to its height bound and stopped on 2 seeds of 4); crowding weight 30 -> 4 (at 30 the
vine stood still through play on 3 seeds of 4); the garden watered; sky held at a clear noon. The species
file `src/kudzu/kudzu.ron` records its own changes against `creeper.ron`.

**The lab did not move**: `labshot frames=0,3000` on main (5890d28de) and on this branch: census text
identical, PNG identical to the byte.

## Guards

`kudzu::tests::preview_lights_what_the_engine_drops` makes the biggest cut with the game's own fall left
out and requires every lit cell to leave the vine, with unlit losses under a tenth of the lit count.
Positive control: with the climber anchor rule off it fails (no cut lights a sheet). The probe's
`predicted`/`fell` counts agree by construction (the game turns lit cells to litter itself) and are not a
check. `the_game_switches_are_off_by_default` holds the defaults.
