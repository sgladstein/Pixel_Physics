# Kudzu probe (probe 7): a cut that drops a whole sheet of vine

*2026-11-07. Brief: `/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/prototypes/7-kudzu.md`
(with `common.md` and `plant-engine-rules.md` beside it). Play: `cargo run --release --bin kudzu`.
Harness: `examples/kudzuprobe.rs` (row in `Reports/instruments.md`). Picture page:
https://claude.ai/artifact/BuCjsNG2G41TKGPr5eC327. Status: built; playtested 2026-11-08; fix 1 (WILTS in the preview) built, waiting on the owner.*

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

## Fix 1 (2026-11-08): the preview shows what a cut starves, not only what falls

**Why.** The playtest of 2026-11-08
(`/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/playtest-2026-11-08/kudzu-review.md`)
found the best move in the game invisible: cutting the vine off from its roots at the crown lit **0 cells**
("NOTHING ELSE LETS GO") and won 3 stars on 8 of 8 seeds with 4-12 cuts. The owner chose this as the
first fix (decision card, 11-08 03:26Z). Picture page: https://claude.ai/artifact/K6fTeh9vWxo63ZeRrYYdYu.

**What it does.** The hover preview now has two colours. Pale is **FALLS** (unchanged: the engine's
support walk). Dull orange is **WILTS**: cells the cut parts from every root while something else -- a stem
lying on the soil -- still holds them up. The bar says both: `CLICK: 0 FALL NOW, 1359 WILT IN 10 S`. A cut
that only drops keeps its old line. `Kudzu::starved_if_cut` is the walk: cells joined to a root through the
vine's own tissue now and not after the cut, less what falls.

**Traced before it was built** (`kudzuprobe mode=rootcut`, seeds 1, 2, 7, 8). After a crown cut every cell
parted from the roots left the vine within ~14 s (seed 1: 1,359 of 1,359; about two thirds by 5 s; the stems
lying on the soil go last) and the rooted stump stayed (115 of 115). **Not sap flow**: with
`PIXEL_PHYSICS_SAP_FLOW=off` the parted piece died the same way. The engine rule that kills it was not
found; the walk is checked against what dies instead (below). A dug vine is different: it grows new roots
from its stems on the soil within 300 frames (seed 1: 0 root cells -> 51), which is why dig alone leaves the
walls worse than doing nothing on 4 of 8 seeds. That is fix 4's, not this one's.

**Before and after, 8 paired seeds, the tester's players.** Same game; only the preview differs, and only
the players who read it can move.

| player | before: walls clear at dawn, mean (3-star nights) | after |
|---|---|---|
| do nothing | 48% (0) | 48% (0) |
| random snips | 92% (3) | 92% (3) |
| planned: cut where the preview lights most | 98% (7) | 99% (8) |
| fresh player: 6 spots near a random patch, once a second | 86% (4) | 95% (4) |
| cut at the roots | 100% (8) | 100% (8) |
| dig only | 48% (1) | 48% (1) |
| wait, then 6 planned cuts in the last 25 s | 95% (4) | 95% (4) |

Paired sign tests on walls clear (three comparisons, so the bar is p < 0.017): planned after against before
4-1-3, p = 0.375; fresh 6-1-1, p = 0.125; wait 4-3-1, p = 1.0. **None is significant.** The planned player
now finds the crown cut by itself: 7 of 8 seeds have three stars in hand within 1.5 s, and it needs 11-17
cuts where it used 12-25. The game was already too easy; this makes the best move visible, and fixes 2-5
in the playtest review (several crowns, waiting costs, fewer cuts) are what make it hard again.

**Does WILTS come true?** `Stats::wilted` against `Stats::wilt_judged` (promises whose 15 s window closed):
cut at the roots 10,052 of 10,052; random 4,309 of 4,309; planned 12,610 of 14,462 (87%); fresh 7,505 of
10,461 (72%); wait 8,010 of 12,252 (65%). **The broken promises were traced, every one** (`mode=promise`,
seed 7): the cells that lived were all joined to a root again, through shoot tissue grown since the cut
beside the blade's gap -- the vine grew back across a narrow cut before the parted piece died. A cut into the
crown, where the vine cannot reach back, never broke a promise. So `WILT IN 10 S` overpromises a cut the
vine can bridge; the page says so. Making the bridge visible, or the cut wider, is not built.

**Cost.** The preview is two floods and the support walk, run only while the pointer is over the vine: 0.41
ms a frame at 1,487 cells and 0.77 ms at 2,101 (seeds 1 and 7), against 0.13 and 0.20 for FALLS alone on
main, same machine. No engine file changed, so the lab cannot move.

## Guards

`kudzu::tests::preview_lights_what_the_engine_drops` makes the biggest cut with the game's own fall left
out and requires every lit cell to leave the vine, with unlit losses under a tenth of the lit count.
Positive control: with the climber anchor rule off it fails (no cut lights a sheet). The probe's
`predicted`/`fell` counts agree by construction (the game turns lit cells to litter itself) and are not a
check. `the_game_switches_are_off_by_default` holds the defaults. `preview_wilts_what_the_engine_starves`
(2026-11-08) makes the crown cut on seed 1 and requires the lit WILTS to have left the vine within 15 s,
the counter to agree, and the unlit stump to stay; positive control: with the walk returning every cell
rooted before the cut, it fails.
