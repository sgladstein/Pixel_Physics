# Queen seat demo: implementation plan v2 (revised after independent review, 2026-10-25)

*Step 4 of the seat-demo board. v1: [`plan.md`](plan.md). Its review: [`queen-plan-review.md`](queen-plan-review.md)
(sound with changes; 8 must-changes, all taken below). The feasibility review: [`queen-feasibility-review.md`](queen-feasibility-review.md).
M0 data: [`m0/`](m0/).*

## What changed from v1, and why

| v1 | v2 | Why (review item / measurement) |
|---|---|---|
| Her eggs carry her mind; Y swaps in a worker's | **She carries 30 fathers** (the pool members): every ordinary egg draws a father, so the colony is striped from the first cohort | Review must-change 1: a one-founder colony has one mind, so Y was a no-op and the tint one colour |
| "Whose line" is the core verb | **The wild egg is the core verb; "take her line" is how you keep a jewel** | M0a below: pool members barely differ |
| HOLD = 6,000, set on "Lay output" | HOLD from the scripted founding's p90 lay interval x 1.5; `last_lay` starts at marking; written in `try_bud`'s success path | Review must-change 8 |
| Hold returns by her laying | **She is never suppressed while she holds**, and only she suppresses workers then | Review must-change 2: the queen rule counts any ant with children as a breeder for life, so the return locked her out |
| Store draw inside `mind::inherit`, world RNG | Draw at the `try_bud` site after `mind::inherit`, keyed `rng::stream(seed, child, frame, SLOT)`; store holds `Arc`s (and the flat genome), never live ids; queue spent only by her eggs | Review must-changes 3, 4 |
| `OrganismOverlay::Line` | `CreatureColour`-route tint, **ants and brood**; `U` toggles | Review: the overlay paints grass too; brood tint makes Y pay off the moment an egg is laid |
| Ledger top-right, by count | Bottom-left, by rate (deeds per 1,000 frames alive, minimum age), selection = `ui.inspected_organism()` (LOOK click) or `K` | Review: BATTLE/STATS dock top-right; counts just list the oldest ants |
| M0a: 30 members x 3 seeds, 2x rule | One mixed colony scored per member, per ant-frame, labels shuffled per seed, permutation null, Dig-ablated positive control, twin negative control | Review must-changes 5, 6 |
| M0b: spread vs seed spread | Outcome classes per width, 20 mutants x 12 seeds, one wild line among ordinary ones | Review must-change 7 |

## M0 results (done)

**M0a: do the 30 shipped minds behave differently?** `examples/queen_m0.rs`, steady_income bed (52 ants), every ant
thinking with one member, scored per ant from its own `LifeCounters`, 24 seeds, member order shuffled per seed.
Ants that lived past 3,000 frames: 3,449 of 5,552.
- **A thing found on the way: which founders die in the thin start is set by the ant's id, not its mind.** Unshuffled,
  members 6-25 lost both founders by ~1,100 frames with zero deliveries on every one of 16 runs; a copy of member 7 under
  another label foraged normally. So 40 of 52 lab founders on this bed starve regardless of brain. (Passed to the
  Teacher lane; it is the lab-box starvation the playtests saw.)
- **Foraging (trip deliveries per 1,000 ant-frames):** spread across members beyond an individual-shuffle null
  (p < 0.003), top 2.9 vs bottom 1.1. But the big lines score low and the small ones high: a line that bred a lot is
  full of young nest ants. Raw deliveries (which count crumbs moved inside the nest): no difference (p = 0.14).
- **Digging:** between-member SD 0.067 vs null p95 0.053; but the twin control (member 0 entered twice) scored
  0.35 vs 0.16, so a 2x gap is inside the noise once lineage clustering is counted.
- **Positive control:** member 5 with its Dig links zeroed: digs 0.00 (n = 119-192 ants). The census sees a real difference.
- **Verdict: no-go for "whose line" as the core lever.** Members are near-copies and what difference shows is
  confounded with each line's age structure. It stays as the follow-up verb, because a *wild* line does differ.

**M0b: what does a wild egg make?** 20 mutants of member 0 at three widths, each one line among ten control labels,
12 seeds, scored against a bootstrap band of the controls at the same ant count:

| Width (pm, sigma, events) | Jewel | Ordinary | Broken forager | Note |
|---|---|---|---|---|
| training's own (0.03, 1, 10) | 5 | 14 | 1 | about 2 jewels expected by chance |
| 3x (0.1, 1, 30) | 2 | 7 | 11 | one broken line bred 1,063 ants and barely foraged |
| 10x (0.3, 2, 100) | 2 | 1 | 17 | many broken lines dig 4-7x the control and never forage: **visible personalities**; one jewel dug 5x and foraged at par |

So the wild egg is a real gamble with a dial: **mild (3x) and feral (10x)**. Feral ants are visibly odd within a
minute (diggers that never leave), which is the "individual shows it, on screen" bar.

## The demo the owner plays (unchanged in spirit)

`cargo run --release --bin lab -- demo=queen` (opens paused: SPACE runs it).

- **Found (minutes 0-5).** Steer her with the arrows: eat at the heap (E), dig a chamber (G), lay (L). Her eggs
  are striped by father: 30 lines tinted on ants *and* eggs.
- **Raise (from the first eggs).** Drop food by larvae (Q) and they pupate sooner; the HUD counts hungry larvae.
- **Gamble (P).** The next egg is wild; P again before you lay makes it feral. It hatches with a fleck and a new
  line name; the notice says so.
- **Keep (Y).** Click any ant with LOOK (or move the ledger cursor with K): Y makes your next five eggs her line.
  The ledger (bottom-left) ranks living workers by digs, trips or brood fed per 1,000 frames (J cycles the deed) and
  shows each line's share and rate, this cohort against the last.
- **Hold.** A bar drains between lays. If it empties, workers lay their own lines; lay again and your rule returns.

## Build steps, each with its proof

1. **Body checks** (test): a steered mind queen lays; the egg's mind `Arc` is the drawn father's (or hers with the
   store empty); the 16-frame sweep never replaces it.
2. **Father store** (`World::fathers: Option<Box<Fathers>>`, `sim::fathers` or in `steer`): per colony a list of
   `(Arc<MindGenome>, Arc<Vec<f32>>, weight)`, a queue of `(Arc, Arc<Vec<f32>>)`, a wild level, and `p`. The draw at
   `try_bud` after `mind::inherit`, only when the store exists for the layer's colony; queue/wild only for the
   steered queen's own eggs; keyed stream. `mind::set(world, id, arc)`. Off: `None`, byte-identical. Shared with
   the colony mind seat (weights + `p`). Tests: empty = layer's mind; queue spent once per egg, not again at hatch;
   off-is-off hash.
3. **Wild egg**: `training::mutate_genome` (built), levels mild (0.1, 1, 30) and feral (0.3, 2, 100), keyed on the child.
   Test: a wild egg differs from its father in > 0 slots; an ordinary one in 0; the lay gate survives (rebuilt from the
   flat genome).
4. **Hold**: `Steered::last_lay` (init at mark, written in `try_bud` success when the layer is steered);
   `regime_of_colony` = Queen while held; in `suppress_bar` the steered queen is exempt while she holds. HOLD set
   from the scripted founding (step 7) p90 x 1.5. Test: lapse -> a worker lays; return -> **she lays a second egg**
   within M frames and no worker lays inside the hold.
5. **Lines**: line = `Arc::as_ptr`; the demo keeps a strong ref per named line; name from `lab::names` seeded by the
   genome hash; palette of 12 distinct colours cycled. Tint through the creature paint site (ants + brood) behind a
   demo-filled `id -> colour` map on `Renderer` (one `HashMap` lookup per creature cell, only while the map is
   non-empty). `U` toggles. Proof: screenshot + per-line cell counts printed beside it; `labperf` worst frame demo on
   vs off.
6. **HUD**: bottom-left ledger + hold bar + hungry larvae + wild level; notices (line hatched, wild one hatched, line
   gone, hold lapsed/returned). Extend the bar-overlap test to the demo panel. `Hud::text` in logical units.
7. **Scripted player + pictures**: `examples/queendemo.rs` drives her through `demo_key` and calls `demo_frame` each
   frame: walk to heap, E, home, G, L, feed larvae, then Y on the ledger's top row and two P. Prints eggs per line,
   wild eggs, lapses, workers, and frames to 10/20 workers (**M0c**, 6 seeds) -> sets HOLD and the reserve. Writes a
   GIF via `Lab::draw` (the labgif route) and stills.
8. **Artifact page** (GIF, stills, M0 tables, keys), report `Reports/queen-seat-demo-2026-10-25.md` + index line,
   README `Held`/lab status line for the demo, PR to the merge desk.

## Cut (unchanged from v1)
Provision/castes, walk genes, brood carry, tandem runs, kin bite, presence field, brood temperature, dream replay,
crossover, scent language, rival colony, mutiny, shelf-as-father. Reasons in v1 §6.

## Risks
- **The colony starves anyway** (thin start, far food, boom-bust): near steady food in the demo bed; the queen's
  reserve grant; graded notices so a bust reads as a bust.
- **Feral lines can take over** (one bred 1,063 non-foragers in M0b). That is the gamble's cost and a Goodhart story;
  the ledger shows the line's rate so the player can see it coming and stop taking it.
- render.rs is contested: the tint is a small additive change; land quickly.
