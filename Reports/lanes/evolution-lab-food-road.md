# Lane note — the food road and the harvest map

*Round 35, Lane A. The owner's ask: **"We should explore better instruments,
visualizations, whatever for the player to understand the food economy of each
colony. I want to know what they are eating, where it is coming from, if/where
it is being stored or movement paths, general colony food stats/balances."**
This lane owns the two halves that are **seen** rather than read — where the
food comes from, and how it moves. A sibling lane owns the books.*

## What landed

**`F7` in the lab cycles OFF / FOOD ROAD / HARVEST MAP / ROAD + HARVEST.**

- **The road** (`src/food_road.rs`, per *cell*): a decaying record of where
  animals stepped, split into **carrying** (a saturated red-orange running to
  near-white) and **empty-handed** (cool blue). Not per chunk, against the
  round's brief and for an arithmetic reason: a road is one cell wide, and at
  any tile size worth calling coarse it is a blob that cannot say which way
  the food went.
- **The harvest map** (per *tile*, per *colony*): the regions food has been
  bitten out of, in each colony's own `GROUP_COLOURS` hue, weighted by **face
  value taken** rather than by bites — a mouthful of flower is three of leaf.
- **`examples/foodroad.rs`** renders both headlessly and prints the numbers
  the picture cannot carry. Indexed in `Reports/instruments.md`.

**A tile is 8 cells, not a chunk.** A chunk is 64, the shipped bed is 512x320,
so a chunk grid is **8x5 tiles for the whole box** and cannot separate one
plant from the next — which is the question the map exists to answer. Eight is
64x40 over the same bed and still 1/64th of a per-cell channel. It is a field,
not a constant, so a harness can sweep it.

**Nothing is read out of `src/sim/`.** Both channels are derived from state
that already exists — `OrganismState::crop`, `::colony`, and the deltas of
`LifeCounters::moves` and `::bites` (*"mouthfuls taken into the crop"*). So
this lane changed no simulation file. The hooks are one line in `Lab::tick`
and one in `Renderer::draw`.

## What it costs, measured

`foodroad cost=4`, played bed at frame 20,000, arms alternated **inside one
run** with the order swapped each round, `RAYON_NUM_THREADS=4`:

| bed | off | on | delta |
|---|---|---|---|
| running (one tick per draw) | 3.93 ms | 4.80 ms road / 4.71 harvest / **5.60 both** | +0.87 / +0.82 / **+1.62 ms** |
| **settled** (no tick between draws) | 1.95 ms | **3.67 ms** both | **+1.72 ms (+88%)** |

**The settled row is the real price and it is the one the brief asked for**:
both channels decay every tick, so they defeat the dirty-rect render skip by
construction — exactly as `FieldOverlay` already does, and by the same line in
the same condition. **Off by default and free then**: `observe` returns on one
enum compare, the maps hold nothing, and the pixel loop never asks them
anything.

## Four things that cost time here, for whoever picks this up

**1. The colony's colour and the road's colour were the same colour.** Colony
1 is `GROUP_COLOURS[0]`, amber, and the laden road shipped amber — so on a
single-colony bed the road vanished into the patch it ran out of. Picking a
different hue does not fix it: `group_palette` generates hues right round the
wheel past the eighth colony. **The split has to be figure and ground, not
hue** — the harvest is a dim dithered wash, the road is the bright thing on
top of it.

**2. A flat tile-wide full replace erased the food it was pointing at.** The
heaviest tiles sit exactly on the plants being eaten. The wash is now an
**ordered 4x4 dither whose coverage is the magnitude**, so a tile that has fed
the colony a little is a scatter and one that has fed it everything is nearly
solid — the first law applied to a readout — and the plants always show
through.

**3. A single amber scaled toward black is unreadable on this world's
surface.** `render.rs`'s `scalar_ramp` is right over dark rock and wrong here:
a road is laid at the animal's *head*, an ant walks on the lit surface band,
and a dim amber over cream ground is a stain. Both channels now run between
**two stops**, and what carries a quiet reading is **saturation** rather than
dimness. Found by rendering it and looking; the counters said 262 cells the
whole time.

**4. No fixed ramp scale can span two shipped scenarios.** Over 9,000 frames
the played bed's heaviest harvest tile held **92,699** of face value and
`far_larder`'s held **753**. Both ramps therefore track the bed, eased in
**simulated frames rather than in calls** — written per call it converged in a
second in the app and took 3,600 frames to get two thirds of the way in a
harness capturing one frame in four hundred, so every sheet was drawn on a
stale ramp. The tell is in the probe line: `max 17.05 of full`. The cost of
tracking is stated rather than hidden — two sheets are not on one bar — and
`road_full`/`harvest_full` pin one when they must be.

## Two findings about the beds, not about the instrument

**`far_larder` seed 1 never reaches its larder.** The colony is placed at
`x: 107` and the heap at `x: 470`; measured over 4,000 frames, **39 bites,
every one within 180 cells of the nest**, all at 480 face value — the corpse
price. They are eating their own dead, and the colony falls 52 → 1 by frame
12,000. The scenario's own header predicted a null here for a different
reason (the carry-band gate); this is a stronger one, and the map is what
makes it a picture rather than a hypothesis.

**On the played bed the road has no length**, because the food grows where the
colony lives: 35 animals, **33 of them carrying**, 392 laden steps against 44
empty-handed over 3,600 frames. That is a true fact about that bed rather than
a defect of the channel — but it means **a card of a haul *route* needs a bed
with a distance in it that the colony survives**, and this round did not find
one. Worth a scenario.

## Review cards posted

*(Collect with `review.py get <id>`, never off `inbox`.)*

CARDS_GO_HERE

## Head and gates

Gates green at the head named in the PR: `cargo clippy --all-targets --release
--locked -- -D warnings`; `cargo test --release --lib` (1,759 passed, 0
failed); `cargo test --release --test worldgen --test determinism` (44 passed,
0 failed); `bash scripts/docscheck.sh` clean.
