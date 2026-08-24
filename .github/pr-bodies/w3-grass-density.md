Follow-up to PR #38 (W3), answering the owner's verdict on the density card.
Full write-up: `Reports/grass-sowing-and-divergence-2026-08-23.md` §13-§14, with the
handoff at §15.

## 0. This branch carries a fix `main` needs whether or not you take the density

`examples/ascii`'s foraging scene — and the same helper in
`examples/ant_ablation.rs` (1 site) and `examples/creature_space.rs` (5) —
computed "the surface" as the topmost `Solid` **or** `Powder` cell. That is the
ground right up until something stands on it: a `seed` is a `Powder` and a
grown blade is a `Solid`, so a sown ground layer makes it return the top of a
*plant*. `main` does not trigger it at `grass_density` 0.35, but the bug is
live in `main` today and the next thing that puts vegetation on those columns
hits it — here, raising the density stamped the ant nest a row above the soil,
planted 55 ants into the vegetation, and took the suite from green to
**1,901 pickups and zero deliveries**. Fixed by asking for ground (skip cells
carrying an `organism_id`). **If the density change is not wanted, this hunk
still should be.**

## 1. The density call

Card `20260823T235145284Z-f19cb5` came back:

> *"I would say noticeable more grass, but it should also spread over time, so
> this could be ok to start. Maybe increase it a little bit"*

Taken as a modest step — **×1.43 on every preset**, keeping their relative
design: `rolling`/`terraced` 0.35 → 0.50, `canyon` 0.22 → 0.31, `wetland`
0.45 → 0.63.

| | before | after |
|---|---|---|
| grass over sixteen seeds at 2,048 cols (min/med/max) | 7 / **24** / 60 | 9 / **38** / 76 |
| conifer, creeper, shrub, tree | 2/6/27, 2/12/23, 1/6/25, 1/16/35 | **all four identical** |
| shipped world, established grass | 71 | 102 |
| shipped world, grass cells | 1,251 | 1,816 |
| shipped world, organism slots | 288 / 4,095, 0 refused | 320 / 4,095, 0 refused |

**Both guard bars re-derived with it**, as PR #38 said would be needed:
`median >= 8` → `>= 12`, `median <= 72` → `<= 114` (a third and three times the
new median of 38 — the same discipline), and the pooled establishment bar
20 → 40 against a measured 110 of 116 (0.95). A bar left at the old density's
numbers has stopped meaning anything.

## 2. Raising the density relocates the sward — it does not thicken it

Worth stating because it decided how the follow-up card had to be built. Grass
is spaced against its own last tussock, so a denser field lands plants at
*different* columns rather than packing more into the same ones. On `canyon`,
world-wide grass rose **574 → 729 cells (+27%)** while the specific 192-column
window the owner had already judged went **100 → 88**. Moss's median moves
19 → 20 for the same reason — the wrong direction for "more grass crowds moss
out", and not a defect.

**So a one-window before/after cannot show a density change here.** The
follow-up card (`20260824T011019066Z-63c0d2`) is a single pane asking "is this
enough now?" with the before/after numbers in `meta`, rather than an A/B that
would have shown the owner the opposite of the change.

It also says plainly what this knob cannot do. "It should also spread over
time" is an expectation the density cannot meet: grass establishes at 95% and
is then shaded out as the canopy closes — 3 of 40 still standing at 45,000
frames, against 63 of 43 with the woody layer off. Raising the density raises
the *starting* amount. What would make grass persist is disturbance (W2's fire)
or shade tolerance (lane P's `shade_death`).

## 3. The regression it exposed, which was not in the flora

`examples/ascii` went from green to a panic:

> no ant completed the loop: 1901 pickups but nothing delivered home

Attributed against `origin/main` first, which runs all 31 scenes clean — so it
belonged to this change. It is a **scene** bug the change exposed. The foraging
scene's `surface` helper found the topmost `Solid` *or* `Powder` cell, which is
the ground right up until something is standing on it. A `seed` is a `Powder`
and a grown blade is a `Solid`, so a sown ground layer makes it return the top
of a plant: the nest patch at x=16..90 was stamped a row above the soil wherever
a tussock had landed, and the 55 ants were planted into the vegetation instead
of onto the hillside. 0.35 happened to leave that stretch clear; 0.50 does not.

`CLAUDE.md`'s own entry in a fourth costume: *a scene that contradicts the code
will look like a bug in the code.* Fixed by asking for **ground** — skipping
cells with an `organism_id` — which is what the scene meant and is immune to
whatever the flora does next. With it: 31 scenes, 0 skipped, forage trips 45
against a bar of 8.

The same helper appears in `examples/ant_ablation.rs` (1 site) and
`examples/creature_space.rs` (5), both building generated worlds at default
densities and carrying the identical exposure. Fixed there too — the harness
`ascii`'s own comment cites for its numbers should not be measuring a different
placement than the scene it backs. No behaviour change for any world without a
ground layer.

## 4. Cost

Two runs a side against `origin/main`, same session, same machine. Means
unchanged everywhere: 8,192×2,560 spring ON 13.00 → 13.02 ms, spring OFF
11.17 → 11.13 ms; 512×320 spring ON 2.85 → 2.75 ms. The worst-frame column is
unreadable at this sample size — `main` alone spans 47.0 to 73.4 ms across two
runs of one binary — and is not claimed either way.

Stated as a limit rather than a clean bill: `ascii`'s scenes carry almost no
grass (75 organisms against 76), so this measurement has little power to detect
a grass-specific cost. The shipped-world means are the load-bearing numbers.

## 5. Handoff update: exposure landed, and the wind blocker moved

W4's terrain-derived exposure is on main — `weather::exposure(world, x, y,
wind)`, `ground_exposure`, `exposure_detail` are public. §14 of the report is
updated, because "this is blocked on W4" is no longer true and the real
obstacle is now in **this** instrument:

Exposure is a pure function of terrain and wind direction, and
`common::PlantScene` builds a dead-flat bed. Measured with W4's own control
(`wind_probe -- preset=flat`): shelter 0.000, prominence 0.000, *"most
sheltered 0.500, most exposed 0.500, spread 0.000"*. **The scene cannot express
the axis at all**, so pointing the instrument at wind without noticing would
return an exact zero that looks exactly like the control passing — the failure
this instrument exists to catch, arriving through its own front door.

The recommended way out keeps the one-axis claim intact: **the same shaped bed
in both patches, read with opposite wind signs.** `exposure` already takes
`wind` and walks the fetch upwind, so a one-sided ridge is sheltered for wind
from one side and exposed from the other — identical terrain, identical
founders, one difference. Shaping the two beds *differently* also works and is
worse: different terrain means different slope, drainage, light angle and soil
depth, and the axis stops being one axis.

## 6. "How long does it take to fill an ideal area" — measured: it does not

The owner's follow-up on the density card, answered from §9's control arm
because that arm *is* the treeless case. Woody layer off, two seeds,
2,048-column worlds with ~500 plantable columns each, re-measured on `main`
**after** W4's wind geography landed (the earlier 63-from-43 figure predates it
and is not comparable):

| frames | seed 1 — plants (sown 64) | cells | seed 2 — plants (sown 63) | cells |
|---|---|---|---|---|
| 5,000 | 64 | 1,381 | 61 | 1,072 |
| 10,000 | 65 | 1,391 | 60 | 1,070 |
| 20,000 | 70 | 1,458 | 59 | 1,059 |
| 30,000 | 69 | 1,433 | 60 | 1,064 |
| **45,000** | **75** | **1,507** | **62** | **1,081** |

**It does not fill, and the plateau is immediate.** Grass reaches its sown
footprint inside 5,000 frames and holds it: the next 40,000 frames — nine times
longer than establishment — add 11 plants on one seed and one on the other.
Slots peak at 91 of 4,095 with 0 refused, so nothing is throttled. Full cover
of ~500 plantable columns would be ~250 plants; at the faster seed's rate that
is on the order of **700,000 frames**, and that extrapolation is generous.

**Re-measured after P2's economy landed (PR #40), and the answer holds.** The
table above predates it, and `grass.ron`'s header had said a retired mat cannot
starve for exactly one reason — *"superlinear maintenance respiration is
package P2's"* — which P2 then shipped. Re-run on post-P2 `main`: plant counts
hold within ±1 at both ends (seed 1 63 → 76, seed 2 61 → 63) and the curve is
still flat. What moved is plant **size**: standing cells fall ~20% on every seed
at both ends, so superlinear maintenance makes each grass plant a fifth smaller
without changing how many stand — a datum lane P may want. Card
`20260824T030939054Z-088ae6` carries the pre-P2 cell figures; its answer stands
and its cell numbers are ~20% high.

**Why: dispersal range is one cell.** `plant::set_seed` places a seed in an
empty 8-neighbour of the parent, after which it falls and rolls. Offspring land
inside or against the clump that made them, so grass cannot cross a gap and the
sown positions are very nearly the final ones. Two independent supports — the
code says one cell, the measurement says the footprint does not grow — and
**explicitly not isolated**: crowding, the seed bank's 18,000-frame half-life
and marginal-ground moisture could each also cap the stand. Separating them
needs a single founder on uniformly ideal ground, which is a scene this package
did not build and which the handoff now names.

**What would change it is queued**: review item A5, dispersal — per-species seed
mass, float and carry. `grass_density` cannot substitute for it. Promoted in the
handoff (§15) to the item with a named consumer.

Card `20260824T030939054Z-088ae6` carries the curve in `meta` alongside a render
of the shipped world with the woody layer switched off at 43,200 frames — still
scattered tussocks with bare soil between them.

## Gates

- `cargo test --release --locked -- --skip root_and_shoot_branching_read_different_slots` — **932 / 9 / 2 / 44, 0 failed** (`--test worldgen` is **44/0**: main's 42 plus this branch's two grass guards — not 42/0, which has been propagating)
- `cargo clippy --all-targets -- -D warnings` — clean
- `bash scripts/acceptance.sh` — all cases met their expectations
- `bash scripts/docscheck.sh` — clean
- `examples/ascii` — 31 scenes run, 0 skipped

Run with `main` merged in, 0 behind.
