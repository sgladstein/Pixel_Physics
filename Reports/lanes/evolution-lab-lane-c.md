# Lane C — the biosphere page

*The evolution lab's stats page, `src/lab/stats.rs`. Coordinator note:
`Reports/lanes/evolution-lab-coordinator.md`. Design of record:
`Reports/evolution-lab-design-guide-2026-08-30.md` §8.9. Written 2026-08-30.*

## What landed

**The Running phase now has something to read.** `Tab` opens a page down the
right of the lab window with the bed still visible beside it: how many plants
and animals are alive and whether that is climbing, how much is growing in
here, what has been born and what has sprouted, how many generations deep the
population has got, and how close the box is to its hard limit on living
things.

**Where it sits.** Guide §8.9 is the open question this closes the cheap half
of: an ant is two dark cells at play zoom, findable only because it moves, and
a dead one has stopped moving. A box full of green that is not reproducing
looks exactly like one that is, and **the discrete event counts are the whole
value of this page**.

## Four departures from the colony panel, each a decision rather than an omission

`App`'s `SHIFT+Y` panel (`Reports/lanes/creature-lane-g.md`) is the model for
the shape — distributions not means, rates not totals, every row able to
explain itself — and its reasoning is reused wherever it transfers. Four
things do not:

1. **Both kingdoms.** The colony census discards every plant on purpose. Here
   the commonest organism *is* a plant and the plants are what breed.
2. **The history is kept whether the page is open or not.** The colony panel
   samples only while open, which is right for a sandbox you glance at. A
   Running phase *is* the lab's content: a player who fast-forwards 45,000
   frames with the page shut must not be handed an empty strip. Costed below.
3. **The strip decimates instead of scrolling.** When the ring fills it throws
   away every other sample and doubles the interval, so it always spans the
   *whole run* at declining resolution. A 3,840-frame window would show 8% of
   a 45,000-frame experiment and call it the population trend.
4. **Sampling is keyed on `world.frame`, never on being called.** One displayed
   frame is one tick at 1x and up to 256 at the top of `time`'s ladder, so a
   per-call sample would draw a strip whose x-axis is the speed dial. The strip
   is plotted against frame rather than sample index for the same reason.

## The oscillator, measured rather than assumed

The colony panel's rate window is one `DAY_NIGHT_PERIOD_FRAMES` because every
rate an outdoor colony produces rides the light. **The lab holds its sky at
noon and pins the weather clear under a stone ceiling, so that cycle is gone
by construction** — and `labstats control=steady` is the run that says so
rather than assuming it. 18,000 frames, plant count at each nominal day
boundary and again half a day later:

| | | | | |
|---|---|---|---|---|
| on the day boundary | 13 | 18 | 27 | 32 |
| half a day later | 13 | 26 | 26 | 34 |

The two rows interleave; there is no systematic gap. So the rate window is set
by what a reader needs (1,800 frames) rather than by a cycle that is not
running, and **every row that quotes a rate also prints the span it actually
got**, which is what makes that a display choice rather than a claim.

## The controls — what each number does when the answer is known

`examples/labstats.rs`, six `control=` modes. Every one is a box built so its
answer is known before the run.

| control | asserts | measured |
|---|---|---|
| `empty` | specificity: nothing alive, every figure 0 | all zero; the page says **THE BOX IS EMPTY** rather than drawing a headline of them |
| `plants` | a plants-only bed reports plants **and** no animals | 5,400 frames: plants 32, 671 cells, **animals 0, no birth economy**; borne 42, sprouted 28 |
| `ants` | the mirror | 3,600 frames: animals 52, 104 cells, **plants 0**, species `ANT` named |
| `cull` | **sensitivity** — half the stand killed mid-run, and the strip must fall | 26 killed; plants peak 14 → 3, biomass 394 → 3 |
| `steady` | the oscillator table above | no cycle |
| `cost` | paired, alternating, one process | below |

**`plants` and `ants` are each other's halves.** A plants-only bed reporting
`animals 0` is worth nothing alone — it is exactly what a blind census
reports — so the same run must show the plant side non-zero, and the ants-only
run the mirror.

## The breeding margin, and why zero births is the correct answer here

PR #162's finding, put on the page: what decides a birth is `ceiling − bar`,
every negative margin gave exactly zero births across twelve seeds, and the
shipped ant sits far below zero. Without that row a player reads `ANIMALS BORN
0` as a colony failing, when it is one that structurally cannot reproduce —
and those want opposite responses.

**It is priced on the food standing in *this box*, not on the material
table**, and that choice is the difference between a live readout and a
constant. Measured, two beds:

| bed | best mouthful | ceiling | bar | margin |
|---|---|---|---|---|
| `control=ants` — a colony, nothing planted | 120, its own flesh | 220 | 1,100 | **−880** |
| the standard lab bed at 27,000 frames | 360, a **flower** | 460 | 1,100 | **−640** |

The first reproduces PR #162's number exactly (`hunger_fraction 0.5 x
start_energy 200`, plus one mouthful). The second is the row doing the job a
table-priced one could not.

The standing set comes out of the census's own walk — the distinct materials
of every living organism's cells — so it costs no grid scan. It misses litter
and corpses on the floor, which is stated at the call site.

## → the coordinator: the flower is standing, and the deadlock in this bed is one mutation wide

Your note says the lab may already be Gate 0's experiment, and that the block
was *"no fruit or flower cell stands in any sampled world at any frame"*. That
finding was measured on worldgen worlds. **It does not hold in the lab bed.**

At 27,000 frames on the standard box the page's margin reads **−640, not
−880**, and 640 has only one explanation: the best mouthful it found is 360,
and 360 is a **1,440-worth `flower`** taken by a neutral gut. The herb stand
has flowered and the page detects it without a grid scan, because the flower
cells belong to a living organism and the census walks those.

**What that is worth.** `creature::diet_yield` squares the gut mismatch, so a
neutral gut (bias 0.0) draws 360 of a flower's 1,440 while a gut drifted to
**−1 draws the whole 1,440** — which clears the 1,100 bar on one mouthful, no
`body_energy` change, no species file, no engine work. So in *this* bed the
deadlock is one heritable step wide rather than closed, which is the shape
your item 2 was hoping for (a matched gut on a 960-point fruit gives +99) at a
better food. I have not run the arm; the page only says the food is there.

`labstats control=ants` separates the two readings: with nothing planted the
same page reads −880, so the −640 is the flower and not the arithmetic.

## What it costs

Measured by `labstats control=cost`, paired and alternating in one process on
a settled bed:

| bed | organisms | living cells | one census | amortised | one paint |
|---|---|---|---|---|---|
| standard, 10,800 frames | 53 | 774 | 0.020 ms | **0.0007 ms/frame** | 0.95 ms |
| 48 founders, 45,000 frames | 102 | 2,205 | 0.327 ms | **0.0109 ms/frame** | 1.63 ms |

That is what pays for departure 2: keeping the history while the page is shut
costs a hundredth of a millisecond. **And it gets cheaper as the dial rises** —
the census is due on `world.frame`, so at 256x one census covers 256 ticks
instead of 30.

**But the two rows are not the same cost, and reading them together is what
found the real problem.** 0.020 → 0.327 ms is 16x for 2.8x the cells, because
the census is two costs wearing one name: everything the page draws is
`O(live organisms)` with an `O(1)` read each, *except* the set of materials
standing in the box, which needs one `World::get` per **cell** — about 150 ns
each. The guide warns the lab will reach **1,812–2,503 live organisms**, where
that walk is several milliseconds **in one frame**, and at the top of the dial
a census falls on every displayed frame. So the material set is refreshed on
its own slower clock (`STANDING_INTERVAL`, 300 frames) and cached between
times; it changes only when a new *kind* of tissue first appears anywhere in
the box. The populations and the biomass are not cached — those are the
numbers a player watches move.

The paint is the same bargain every panel in this repo makes: an open page
forces a full redraw and loses the dirty-rect skip, which is why `Lab::draw`
already has `stats.showing()` in its `force_full`.

## Guards

`cargo test --lib lab::stats` — 12 tests. The ones that are not obvious:

- `the_census_counts_plants_and_animals_apart` asserts **both** halves in both
  directions, for the reason above.
- `killing_half_the_stand_moves_the_living_count_and_the_dying_back_row` is
  the sensitivity guard: a count that is right about a settled box may still
  be a constant.
- `the_page_stays_inside_its_own_border` counts **ink**, not lit pixels, and
  the numbers are why: the page lights **60,260** pixels and only **5,459** of
  them are ink. The plate is blended over black at about 8 per channel, so a
  lit-pixel floor of 2,000 passes for a page that has painted its own
  background and nothing else — the blind-guard shape exactly. Ink is pixels
  clearing 80 in a channel, which every text and bar colour does and no plate
  or rule does; the bar sits at 2,500 against the measured 5,459, and the test
  prints both so the next reader can re-derive it instead of trusting a
  comment.
- `the_history_keeps_the_start_of_the_run` asserts the decimation from both
  ends: the run's first reading must still be on the left *and* the newest one
  must survive every halving.
- `a_bred_organism_lands_past_the_first_generation_bucket` asserts the flat
  histogram first, then puts a generation-3 organism in and watches the bucket
  move.
- `every_string_the_page_builds_has_a_glyph_for_each_character` drives every
  row and every hover note through the font. Nearly all of them are composed
  at run time out of species names and formatted numbers, so a test over
  literals cannot see them.

## → the coordinator: one signature, and two things not built

**The signature.** The hover is built and unreachable. `Stats::draw_at(frame,
world, cursor)` exists and is tested; `Lab::draw` already holds the cursor and
passes it to the renderer, so turning it on is one line in `src/lab/mod.rs`:

```rust
self.stats.draw_at(frame_buf, &self.world, cursor);   // was: self.stats.draw(frame_buf, &self.world)
```

Written that way rather than by editing `mod.rs`, which is yours.

**Not built, both needing something the page cannot see:**

1. **A per-compartment split.** Guide §2c makes partitions the strongest
   single finding and §5 makes them the scoring move, and the page cannot show
   which compartment is doing what because `Stats` is handed a `&World` and
   not the `LabBox` that describes it. `Lab` holds `spec` already.
2. **Plant trait spreads.** The colony panel draws one row per creature trait
   slot, which is *"is there anything left to select on"*. The plant genome is
   alleles + a fate table rather than a flat trait vector, so the same row
   needs a vocabulary decision rather than a loop. `LINES` and `BIGGEST` cover
   the coarse version of the question today.

## Review

Card **`20260830T095249396Z-e41ec0`** on board `lab`, posted 2026-08-30: the
page over the standard bed at 1,800 frames and at 27,000, asking whether it is
*readable* cold — whether a reader can tell from it that something is being
born, and whether the right-hand side is the right place for it. Verdict not
yet collected: `python3 scripts/review.py inbox`.
