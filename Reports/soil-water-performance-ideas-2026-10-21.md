# Soil water: where the cost is, and what would move it

*Fable review lane, 2026-10-21; written in the project folder (`/mnt/project-files/soil-water-perf/`) and copied here 2026-10-22 so nothing is lost. Scratch branch `claude/project-thread-mblj3z` (counters, switches and the `soilcost` instrument; never for main). Owner's rules applied: one physics for every world, training stays deterministic, anything that changes results is the owner's decision.*

## The one-paragraph version

The soil pass costs what it costs because of **how the water moves, not how the code is written.** In the played lab box (8 plants, one colony, mister `Light`, drain on) the pass is **41% of a 2.4 ms tick**, and 6,000 of its writes a tick are drainage moves, **63% of them exactly one unit**. The drainage rule moves a quarter of a cell's surplus and truncates, so every slug of rain decays into a one-unit-per-tick trickle that walks every cell of every wetted column, from the surface to the floor drain, for ever. The bed sits at field capacity **+3** everywhere because that is where a quarter truncates to zero. Writes a tick are roughly *rain in × depth ÷ packet size*, and the packet is one. Bit-identical work can shave the pass by maybe a third (one such change is measured below); only a change to that rule removes the conveyor, and that is a behaviour change for the owner. Three candidate rules are sized below, with pictures.

## What was measured

`examples/soilcost.rs` on the scratch branch: the shipped `LabBox`, `RAYON_NUM_THREADS=1`, one window of 2,000 ticks after a pre-run, the pass timed on its own, writes counted by source, and a cells+field hash at the end. Callgrind on the pass alone for the instruction counts.

| bed | tick | soil pass | visited / changed a tick | drainage writes a tick | one-unit moves |
|---|---|---|---|---|---|
| empty box (no plants, no colony), frame 8,000-10,000 | 0.58 ms | 0.30 ms (52%) | 4,100 / 1,880 | 1,870 | 69% |
| played box (8 founders, 1 colony), frame 20,000-22,000 | 2.42 ms | 0.99 ms (41%) | 13,900 / 6,020 | 5,980 | 63% |

Per visited cell the pass costs **75 ns in both beds** (375 instructions). Where those go, callgrind, empty box: `soil_partner` 29% (PR #771 inlines it), `set_moisture` 10%, `mark_moist_neighbours` 3%, the rest is `update_soil_water`'s own reads and arithmetic. The plan sort is 0.1%.

Capillary exchange is 10-30 writes a tick. Infiltration, drip, root pull: single digits. **The pass is the drainage conveyor.** The whole bed below the top band reads 623 (field capacity 620 + 3) because `(surplus × 0.25) as u16` is zero at a surplus of three and one at four to seven.

## Ideas that keep results bit-identical (the lanes can take these now)

Ranked by measured or expected gain on the pass. Together they are worth perhaps 1.3x on the pass, about 10% of the lab tick. None of them changes a hash.

1. **Skip the infiltration reads where no liquid can be adjacent.** `update_soil_water` reads four neighbours and their material kinds for every visited cell looking for a `Liquid`; a chunk's `has_liquid` flag already says when none exists. Prototyped with a `CellSurface::liquid_may_touch` that answers from the chunk flag for interior cells: **−9.6% instructions in the pass** on the empty box (2.843G → 2.570G Ir over 1,800 ticks), hashes equal. **On the played box it fires almost never**, because drips in galleries and standing drops make most awake chunks `has_liquid`. It needs a finer mask to pay there: a per-16×16-block "may hold liquid" bitmask kept beside `stale_blocks` (set in `set_world` where `is_liquid` is already computed, cleared on the settle rescan that already recomputes `has_liquid`). Expected: the same ~10% on the played box.
2. **Compact per-material tables on the view.** Each visited cell makes ~8-10 `materials().get(id).water_capacity` / `kind(id)` calls into 616-byte `Material` records. A `[u16; N]` capacity table and a `[u8; N]` kind table built once per pass (the lab-world lane is doing exactly this for the field's block scan) should take 5-10% off the pass. Textured worlds also pay `texture_of(shade)` per cell and per partner; the same table idea applies.
3. **One write per cell instead of two.** In a conveyor column every cell is written twice a tick: once by its own visit (it passed a unit down) and once by the cell above (it received a unit). `set_moisture` plus `mark_moist_neighbours` is 13% of the pass, so merging the pair is worth ~5%. The walk order has to stay row-major bottom-first for the sideways exchanges to see the same values; a column-major walk is provably equivalent only with `soil_through_roots` off, so do this as a one-row lookbehind inside the current order rather than a reordering.
4. **Reorder `schedule_damp_soil`'s rejects.** It runs on every changed cell (6,000 a tick) and pays a material lookup before the cheap `get(x, y−1).is_empty()` test. Tiny, free.
5. **Wavefront parallelism across chunks.** The pass is serial by decision. Its dependencies are exactly "after the chunk to the left and the chunk below" (left and below are walked first; the right and above chunks only read pre-walk state; cross-chunk writes can be deferred to the end of each wavefront step and applied in serial order, including the one diagonal case, side-drain into the chunk below-right). That is a bit-identical schedule, but the lab's soil is three chunk rows deep, so the wavefront is at most three wide, the training beds are one chunk wide, and this repo has measured rayon dispatch as a net cost for jobs this size. Worth it for the outdoor world (40 chunk rows), not for the lab.
6. Not worth doing: the plan sort (0.1%); a prefilter over the marked set (already a dead end, 88% of marks are soil); narrowing the 4-neighbourhood dilation (not identical: a side neighbour has to be visited to know it does nothing).

## The lever that actually moves it: the drainage rule (owner's decision)

All three arms below change results, so they are the owner's call. All three were run with one small coupling fixed first, described after the table. Empty box and played box, same windows as above.

| rule | empty box tick | empty pass | played box tick | played pass | drainage writes (played) | bed rests at | top three soil rows |
|---|---|---|---|---|---|---|---|
| shipped: move ¼ of the surplus, truncate | 0.58 ms | 0.30 ms | 2.42 ms | 0.99 ms | 5,980 a tick, 63% one unit | FC + 3 | 500 / 528 / 531 |
| **A. a surplus under 16 leaves whole, else ¼** | 0.37 ms | 0.10 ms | **1.59 ms** | **0.33 ms** | 1,380 a tick, none of one unit | FC exactly | 478 / 510 / 538 |
| B. move ½, truncate | 0.43 ms | 0.15 ms | not run | | 750 a tick (empty), 52% one unit | FC + 1 | 540 / 600 / 603 (empty) |
| C. move the whole surplus (rate 1.0) | 0.25 ms | 0.02 ms | 1.31 ms | 0.10 ms | 90 a tick, 63% over 50 | FC exactly | 503 / 525 / 536 |

What each one does to the water, in the world's terms:

- **A** keeps the slow, spreading wetting front for anything bigger than a few units (a drop still seeps down over tens of ticks) and only stops the last dribble: once a cell is holding less than 16 above field capacity it hands the lot to the cell below in one go. It is the shipped rule with the geometric tail cut off. Writes fall four-fold, the pass three-fold.
- **B** is simply a faster seep; the tail is still one-unit, so it buys about half.
- **C** makes a drop fall through the soil as a compact slug with no spreading. Cheapest by far (the pass all but disappears; the played box tick nearly halves), but the wetting front the drainage constant exists to show becomes a line. The constant's own doc says 0.25 is there so drainage is "a visible wetting front descending over time rather than an instant teleport".

**What every arm changes besides the cost, and the gate it must pass.** Under all three, cells rest at exactly field capacity instead of field capacity + 3 (ants read the field's moisture tile, not the cell, and the tile derives from held ÷ capacity, so this is a 0.3% shift). The top three soil rows come out 10-25 units **drier** and the box's water cycle runs faster (more drains than rain fell, because evaporated water condenses under the lid and comes back as drops). The dead-ended hold band (`Reports/dead-ends.md`, `SOIL_DRAINAGE_HOLD`) also cut the conveyor, by making cells **hold** up to 40 extra units; it was twice as fast where aimed and bad for the colony (animals underground 18.5 → 2.5 over 12 seeds). These arms err the other way (a few units drier, never damper), but the gate is the same one: `scripts/labbench.py` on the played bed, 12 seeds paired, and the owner's eye on the front.

**The coupling found on the way, which any of these needs.** The drying schedule is armed from inside the soil pass only when a cell's value *changed in the visit*. A surface cell that drinks a drop and passes every unit of it on in the same visit ends where it began, so it was never put on the drying schedule and the surface stopped evaporating: under rule C without the fix, not one cell in the bed was below field capacity after 10,000 frames, and all the rain went to the drain. The fix is one line (arm the schedule on "drank" as well as "changed"). It is unreachable at the shipped rate on an empty box (hash identical there) but reachable on the played box, where roots and capillary flow can net a drink to zero, so even on its own it is a small behaviour change and should land with the rule, not before it.

## Pictures

`soil-water-perf-2026-10-21/front-three-rules.png`: one drop's column in the empty box (columns 44-76, the top 46 soil rows, 7x), the shipped rule beside A and C, at 0, 2, 4, 8, 16, 32 and 64 ticks after frame 6,000, a mister tick. Moisture on a fixed ramp: below field capacity red to black, field capacity to saturation dark blue to white, a standing drop cyan. Same seed, same drops. Under the shipped rule the drop seeps as a fading plume and the faint streak of an earlier drop is still working its way down below it; under A the plume is the same shape and is simply gone sooner; under C the drop is a compact bright block that works down the column with no plume at all. In A and C the surface band is visibly redder (drier) than under the shipped rule, which is the evaporation difference described above. The full-width 2x versions and the scratch `labshot` sheets stay in the project folder (`soil-water-perf/pictures/`); `soil-water-perf-2026-10-21/zoom-{base,snap16,rate1}.png` are the three columns of the composite.

## For the training beds specifically

The plant lane's rung bed (48 wide) is 73% soil pass for the same reason at ten times the rain per column; PR #773's rain-per-column halves it. Any of the rules above compounds with that, and on a 48-wide bed the ceiling is the whole 73%. Nothing bit-identical reaches it.

## Reproducing

```
git fetch origin claude/project-thread-mblj3z
cargo build --release --example soilcost
RAYON_NUM_THREADS=1 target/release/examples/soilcost founders=0 colonies=0 pre=8000 window=2000
SCRATCH_SKIP_INFILTRATION=1 ...                       # idea 1, hashes must match
SCRATCH_SCHEDULE_INFILTRATION=1 SCRATCH_DRAIN_SNAP=16 ...   # rule A
SCRATCH_SCHEDULE_INFILTRATION=1 SCRATCH_DRAIN_RATE=1.0 ...  # rule C
... pic=front.png picstops=0,3,6,12,24,48 picrows=40  # the pictures
```

## Outcome, 2026-10-22

The owner ruled (2026-10-21 23:48Z): the visual does not matter; if plant growth is unaffected, take the fastest rule (C, rate 1.0). The plant performance thread ran the growth gate on main 910268b3 + #773 with the two switches ported, off against on and a noise-floor arm at rate 0.26: **no difference beyond the noise floor** on the four species' rungs (establish, grow, life; bright and dim; 12 layouts), on 4 community seeds over 120k frames, or on 2 dry-spell seeds. On the planted community bed the top soil band ends ~0.025 *wetter* under rate 1.0 (0.542 -> 0.569; outside the rate-0.26 control on 2 of 3 paired seeds, and the control itself is wetter than off on all 3, so the effect is monotone in the rate: 0.542 / ~0.556 / 0.569, about 2.7% of saturation), the opposite sign from the empty box measured above, and water beside roots rises slightly. Soil-moisture writes ~1,000 -> ~70 a tick; rung beds 40-45% faster in core-seconds. Remaining gate: the 12-seed ant bench (lab world lane), then the lab world lane lands SOIL_DRAINAGE_RATE 1.0 with the drying-schedule fix.

**Ant bench (lab world lane, 2026-10-22 00:28Z):** `labbench` on the played bed, 12 seeds paired, scratch 3604bf2e base against the two switches: every gate row inside noise (births median 676 -> 728, 5/7, p 0.77; starved per million ant-frames 0.9 -> 1.2, p 0.77; ants underground 26.2% -> 26.3%; colonies died out 0 -> 0); whole-run wall time median 220 -> 180 s. Tables: `soil-water-perf-2026-10-21/labbench-drain-rate-1.0-ants.txt` (the switches) and `labbench-drain-rule-shipped-ants.txt` (the lab world lane's re-bench of the real change). **Gap in the switches, found by that lane:** `SCRATCH_DRAIN_RATE` overrides the rate only at the `drainage_rate` site; the root-mat pull reads `SOIL_DRAINAGE_RATE` directly and stayed at 0.25 in both lanes' "on" arms (`soil_through_roots` is on by default in the lab). The real change moves both sites, so the lab world lane re-benches the real change against main before its PR, and the plant rung gate should be re-run on the same commit.

## Hand-off (2026-10-22 00:47Z, lane closed by the owner)

This lane has no runs in flight. Everything that follows is owned elsewhere: the **lab world performance** lane re-benches the real change (`SOIL_DRAINAGE_RATE` 0.25 -> 1.0 at both sites, plus the drying-schedule fix "arm on drank as well as changed") against main and lands it through the merge desk, with the constant's doc, the `SOIL_DRAINAGE_HOLD` dead-end re-test line and the wiki freshness note updated; the **plant performance** lane re-runs its rung gate on that commit when asked. The bit-identical ideas in the list above are the lab world lane's to pick from. The scratch branch `claude/project-thread-mblj3z` (commit 3604bf2e) is measurement kit only and must never reach main.
