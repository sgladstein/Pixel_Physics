# Queen demo plan: independent review (2026-10-25)

Reviewed `seat-demos/queen/plan.md` v1 against the working tree (main 9e32979e plus the uncommitted demo edits).
Line numbers are for the working tree. No repo file was changed. I timed two runs with a copy of the existing
`labsurvive` binary, which predates `member=`.

## Verdict: sound with changes

The engine pieces are buildable as described. The launcher, the steering hand-off, the egg hook, the overlay and HUD
seams, and the GIF route all exist. Two design faults would leave the owner with a demo whose central verbs do
nothing, and one measurement rule passes by chance. All three are cheap to fix before step 3.

## Must change

**1. The verb behind Y has nothing to act on. A queen-founded colony has one mind.**
- An egg copies its layer's mind `Arc` (`creature.rs:6088-6090` -> `mind.rs:1730-1747`). The hatchling copies it again
  from the egg (`brood.rs:1593-1597`).
- The demo lands one founder, so she and every daughter share one `Arc`. The 16-frame sweep fills only ants that have
  no mind (`lab/mod.rs:1094-1108`, filter `!mind::has`), so it adds no diversity.
- So the §0 promise ("pick one and press Y: your next five eggs carry *her* mind, not yours") is a no-op. Every
  worker *is* the Crown line. It stays a no-op until P has made wild ants, and the tint shows one colour until then.
- The §3.2 lapse ("workers lay eggs of their *own* lines") is also invisible: worker eggs are Crown too.
- M0a then measures a question the demo never asks.
- **Change:** seed the father store with the 30 pool members at founding. Her ordinary egg draws its father uniformly
  (or by the store's weights), which is polyandry and real biology. The colony is then a mix of up to 30 patrilines
  from the first cohort. Y re-weights toward one line (the queue of K stays as the strong form), the tint shows
  stripes from minute 3, and lapse eggs carry *workers'* lines, which are now genuinely varied.
- This is the same §3.3 store, preloaded. Off stays off: the store is filled only by the demo.

**2. When the hold returns, it locks the queen out.**
- `Queen` suppression is "any *other* living breeder in the colony" (`suppress_bar`, `creature.rs:23607-23624`).
  It excludes only the ant trying to lay, and a breeder is anyone with `children > 0`, for life (`creature.rs:6068-6070`).
- After a lapse in which workers laid, she lays one egg under `graded`. That flips the regime back to `Queen`, and her
  next bar is `f32::INFINITY` because the workers who laid are "other breeders". Their bars are infinite too, because
  she is one.
- Nobody lays until HOLD runs out again. "Come back and lay, and your rule returns" therefore *cuts* births to about
  one queen egg per HOLD, and it lasts for the lifetimes of those workers.
- **Change:** in `suppress_bar`'s `Queen` arm, the steered queen is never suppressed while she holds
  (`world.steered` id == `organism`, role Queen), and only she counts as the breeder against workers.
- **Proof that can fail for this:** after a lapse in which ≥1 worker laid, the queen lays, the hold returns, she lays a
  *second* egg within M frames, and no worker lays inside the hold. The plan's proof (a worker lays after the lapse,
  none before) passes with the bug in.

**3. The father-store hook must sit at the egg site, not inside `mind::inherit`.**
- `mind::inherit` runs twice per ant: at laying (`creature.rs:6089`) and again at hatching (`brood.rs:1595`, egg ->
  adult). A draw inside it would fire twice, consume a "next K" queue twice per ant, and could re-father an egg at
  hatch.
- **Change:** put the draw at the `try_bud` call site, after `mind::inherit`, as an overwrite of `child`'s entry.
  Add a `pub fn mind::set(world, id, Arc<MindGenome>)` with the same blank `MindState`.
- Store an `Arc` when Y is pressed, never a "live ant id": the chosen worker may die before her five eggs are laid.
- Apply the queue only when `organism == steered_id`, so a worker's egg during a lapse cannot spend the queen's queue.

**4. Draw the store's randomness from a keyed stream, not "the world's own RNG stream".**
- `World::rng` (`world.rs:3996`) is shared state. One draw shifts every later consumer, so turning the demo's store
  on reshuffles unrelated systems.
- The house pattern is `rng::stream(world.seed, child, world.frame, SLOT)` (`rng.rs:124`; see `RNG_SLOT_BIRTH`,
  `creature.rs:136`). It is a pure function, so it is byte-identical when off and touches nothing else when on.
- Use it for the father draw and for P's mutation. Seed `train::Rng` (`training.rs:52`) from it.

**5. The M0a go rule passes by chance, and is run on the wrong bed.**
- **The 2x rule.** Best and worst of 30 noisy three-seed means differ by about four standard errors under the null.
  At the colony CVs seen here (my two 36k-frame runs: lived 26.7 on steady_income against 94.3 on played_bed; the
  review reports a boom-bust from 142 to 49), max/min > 2x is near-certain even if every member is identical. That is
  the winner's curse.
- **The SD rule.** "Between-member SD ≥ within SD" is a strict test, which is fine, but it compares a mean of three
  with single runs. State it as an ICC or F-test.
- **Change:**
  - (a) a label-permutation null for both statistics (shuffle member labels 1,000x);
  - (b) re-run the top 3 and bottom 3 members on 6 *fresh* seeds; go only if the gap replicates;
  - (c) report digs and deliveries *per ant-frame* (`digs / (lived × after)`) as well as raw totals, because raw
    totals mostly measure colony size, which boom-bust dominates;
  - (d) a negative control: member 0 entered under two labels must fail the rule.
- **The bed.** "`scenario=<demo bed>`" lands one ant. Unsteered, she runs `graded` and her own founding is close to
  all-or-nothing, so the census measures founding luck. Run M0a on a 52-ant bed (steady_income layout).
- **Better, given change 1:** one mixed-pool colony, scored *per member* from each ant's `LifeCounters`. That is what
  the ledger will show, it costs 6 runs rather than 90, and it adds a print to labsurvive rather than a new switch.

**6. M0a's positive control is too loud to calibrate anything.**
- `founders=old` against the pool is a different brain: the review measured 41 of 42 starved against 0. It proves only
  that the census can see an elephant.
- **Change:** use a graded control of the size the go rule needs. One example is member 0 against member 0 with
  `tax=2` (labsurvive's existing synapse-bill switch). Another is member 0 with its Dig output ablated, which should
  cut digs per ant-frame. The rule must fire on it.

**7. M0b's choice of W rewards breakage.**
- "The smallest width whose mutants spread at least 2x the seed spread" is maximised by mutants that kill their colony.
  Spread is not a gamble with jewels in it.
- With 10 mutants x 2 seeds, a seed SD from two seeds is also nearly unestimated.
- **Change:** choose W by outcome classes, each replicated on 3+ seeds: P(collapse) ≤ ~40% and P(better than member 0
  beyond the permutation band) ≥ ~10%. Use ≥20 mutants per width.
- And score what the player sees, *one* wild ant among Crown workers (per-ant deeds), not an all-mutant colony.

**8. HOLD = 6,000 is not "about two egg stages", and neglect is not the only way to lose it.**
- `egg_frames` is 250 and `pupa_frames` is 250 (`ant.ron:122-130`). The larva stage ends when it is fed to a target of
  about 1,060 J (`brood.rs:839`, `:1001`). Egg to adult is about 3,800 frames (the review, Little's law). So 6,000
  frames is 24 egg stages, about 1.6 generations.
- She lays only on a pressed L (steered `Lay` is ±1, `creature.rs:27756`; mind ants pass 0.0, which is above
  `LAY_HOLD_BELOW` of -0.1, `brain.rs:1577`) and only when her bank is at least the 1,100 J bar.
- A queen spending crop food on larvae (§3.7's own verb) can lapse while trying hard. The hold would then punish the
  feeding the demo asks for.
- **Change:**
  - Set HOLD from M0c: the scripted player's p90 inter-lay interval x 1.5.
  - Start `last_lay` at the frame she was marked. With 0 she lapses at frame 6,000 of the founding, and the bar is
    drained from the start.
  - Write `last_lay` in `try_bud`'s success path when `organism` is the steered id, not "where her Lay output
    succeeds". That also covers laying above 4x, where her mind lays for her.

## Improve

- **Tint through `CreatureColour`, not `OrganismOverlay`.**
  - `OrganismOverlay::Lineage` paints *every* organism, grass included (`render.rs:8530-8556`).
  - The creature paint site already has the cell's organism id and touches only the ~150 creature cells
    (`render.rs:7408-7425`).
  - `CreatureColour` already has per-animal readouts (`animal_readout_colour`, `render.rs:1531`).
  - Add `CreatureColour::Line`, or a demo-filled `id -> colour` map on `Renderer` that is read before the readout.
  - Key lines by `Arc::as_ptr`, with the demo holding a strong ref to every line it has named so that an address
    cannot be reused. Compute the name hash once per new `Arc`, never per cell.
  - Also: the `L` and `H` keys that cycle the overlays are swallowed in demo mode (`bin/lab.rs:607`, `:615`), so `U` must own the toggle.
- **Tint brood too.** Brood cells carry their organism id (`brood.rs:853`), but `animal_readout_colour` skips brood
  (`render.rs:1532`). Without it, Y's consequence is invisible for about 4,000 frames. With it, the next egg is visibly
  hers the moment it is laid, and the verb pays off at once.
- **Rank the ledger by rate, not count.** Cumulative `LifeCounters` rank the oldest ants first, which makes "digs" and
  "lived longest" the same list. Use deeds per 1,000 frames alive, with a minimum age, and keep age in the row.
- **Panel placement.** BATTLE docks at the top of the right-hand column and STATS under it (`ui.rs:413-415`), so a
  top-right ledger collides with them.
  - Draw it in `Lab::paint` after `ui.draw_chrome` (`lab/mod.rs`, the block ending at `paint_stats_note`), under
    `paint_owner::set("DEMO")`.
  - Place it through the `ui::Window` rule, or bottom-left.
  - Extend `the_bar_fits_the_screen_and_no_two_widgets_overlap` (`ui.rs:13395`) to a demo arm.
  - `Hud::text(frame, x, y, text, colour)` (`render.rs:9876`) is in logical units; the layer is 2x.
- **The selection route is `ui.inspected_organism()`** (`ui.rs:4055`, set by `Ui::inspect` from `use_tool`'s LOOK arm
  at `lab/mod.rs:2880`), not the roster `pinned()` (`ui.rs:4464`). LOOK is the default tool and `Z` is not swallowed,
  so clicking works. Y on her own row should say "THAT IS YOUR OWN LINE" rather than do nothing.
- **P should arm the next egg *and* latch L,** or the help line must say "P then L".
- **Say SPACE.** A demo opens paused (`bin/lab.rs` comment), and the help line never says SPACE.
- **`queendemo` must call `lab.demo_frame()` every step.** `Steered::input`, the body bind and `queen::frame` run only
  there (`demo/mod.rs` `demo_frame`). `tick_for_harness` does not call it.
- **Frame cost.** The lab's instrument is `labperf`, not `ascii`. The pulse mark is a world mark, and world marks force
  the full box redraw (`lab/mod.rs`, the `draw_layered` doc), so keep it to a few frames.
- **Mutating a `MindGenome`** (`mind.rs:283-298`):
  - `w` is the post-ban `GENOME_LEN` vector. The lay gate is stripped into `.lay`, and the dig head into `.head`.
  - To mutate: rebuild the flat vector as `w ++ lay` when `lay.is_some()`, then
    `neutral_mutate(&flat, &colony_bed::slots(), pm, sigma, events, rng)` (`training.rs:401`;
    `challenges.rs:2734-2747`), then `colony_bed::mind(&new)`.
  - Rebuilding from `w` alone silently drops the lay gate, because `MindGenome::new` only recognises a gate when the
    length matches (`mind.rs:304`). That would change the wild ant for a reason that has nothing to do with the gamble.
  - W is three numbers (`pm`, `sigma`, `events`), not one. `wild=` must name all three and echo them.
- **Rebuild `--examples` before M0.** The `labsurvive` binary on disk (08:07) has no `member=`; I checked with
  `strings`. Under the CLAUDE.md stale-binary gotcha, an unknown argument is silently ignored and M0a would run the
  whole pool 30 times.

## Verified correct

- **§3.1 checks.** A steered mind ant takes the hand-built path (`creature.rs:8007-8011`). It still reaches `settle` ->
  `try_bud` (`creature.rs:8861`, `:9196`) -> `mind::inherit` (`:6089`), so her eggs carry her `Arc`. The sweep never
  replaces an existing mind (`lab/mod.rs:1104`). Both tests are still worth writing.
- **Off is byte-identical.** `mind::inherit` uses no RNG, so a store that is empty or `None`, guarded by an `if`, is
  identical when off. `regime_of_colony` (`creature.rs:23045-23050`) returns the env regime when nobody is steered.
- **A lapse to `graded` does let workers lay.** The bar is ×1.0-1.25 within 24 cells of a breeder
  (`creature.rs:23081-23093`, `GRADED_MAX_SUPPRESSION` 1.25), and the trained colony bred 349 times under it (review
  §3). Under `graded` her own bar rises at most to ×1.25 (1,375 J) beside a breeder.
- **The `neutral_mutate` facts.** It is private and takes a flat `&[f32]` (`training.rs:401`), so a pub wrapper is
  needed. Minds are built by `colony_bed::mind` -> `arena_mind` -> `apply_bans` + `MindGenome::new`
  (`challenges.rs:171-175`, `:2734`).
- **The tint mechanics.** `OrganismOverlay` is an enum with a `next()` cycle and `label()` (`render.rs:1071`,
  `:1214-1258`). Lineage builds its ranking once per frame and redraws only on change (`render.rs:4885-4904`), which
  is the right template for any per-line colour.
- **The HUD and GIF routes.**
  - `Lab::draw` (`lab/mod.rs:2252`) draws bar, pages and chrome into a buffer headless. `labgif` does exactly this
    (`labgif.rs:747-748`, `show_help = false` at `:347`, GIF via `GifEncoder` at `:995`).
- **Keys.**
  - The demo layer consumes E Q F G B L T Y U P J K R H, Tab and the arrows before the lab (`bin/lab.rs:595-635`).
  - No two demo verbs collide.
  - Lost lab bindings, all acceptable: E food tool, Q fling, B place, K wall, J alarm, U lamp, L life overlay, H
    creature colour, Y life marks, P params, G shelf, F display floor, T reaction, Tab stats, R reset. Losing R is a
    benefit.
  - Space (pause), Z (LOOK) and PgUp/PgDn (speed) survive.
- **The named guards exist:** `every_shipped_scenario_loads_builds_and_places_what_it_says` (`scenario.rs:637`),
  `a_steer_on_no_living_body_changes_nothing` (`steer.rs:315`), `examples/deeptrace.rs`.
- **Feeding.** A larva eats one bite of food beside it per tick (`brood.rs:985-997`), and a hungry-larva reading
  exists (`brood.rs:1404-1457`) for the HUD count.
- **M0 is affordable.** Measured with two processes on 4 cores: one `labsurvive founders=std after=30000` (36k frames)
  took **6 s** on steady_income and **55 s** on played_bed, where the plants dominate. On a steady_income-like bed,
  90 runs take about 10 minutes wall. That is cheap enough to afford 6 seeds and the held-out replication above.
