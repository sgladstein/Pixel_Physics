# The held world is made of soil

**What it does.** `cargo run --release --bin druid` now generates somewhere to
garden instead of somewhere to mine. The ground is soil past the bottom of the
screen with roots visible in it; rock is a floor under the world and the
occasional hill that breaks the surface, rather than the banded grey mass that
filled two thirds of every frame.

**Where it sits.** The held world's country, second pass — the owner rejected
the first one outright (*"This looks too much like The gnome game… it should be
way more soil, way less rock"*, then *"The camera is ok, but it should be mostly
soil, little to no rock"*). This is the ground; what grows on it and how much
water it holds are still open.

Review card `20260913T160236083Z-f1ca6f`.

## The numbers

Measured over 8 player viewports × 3 seeds at the shipped 2560×960, both arms
from one binary via `PIXEL_PHYSICS_DRUID_PRESET`
(`world_look mode=composition world=2560x960`):

| | before | after |
|---|---|---|
| ground on screen that is rock | **79%** | **36%** |
| ground on screen that is loose (soil/sand/gravel) | 10% | **54%** |
| soil alone | 8% | **54%** |
| soil above the first rock, median column | 22 cells | **115 cells** |
| columns with bare rock at the surface | 17% | **2.5%** |
| plants grown before the world is held | 3,772 | **4,095** |
| seconds to build the world | 56 | **107** |

The organism counts are paired and alternating with `RAYON_NUM_THREADS=4`, two
reps per arm, byte-identical within each arm. **The build time roughly doubles**
and that is real: there is 22× more soil to wet and settle (`soil_blanket`
44,860 → 919,411 cells at 2560×960). It is paid once at startup, not per frame,
and the world is held afterwards — but it is a cost, not a rounding error.

## Preset data only

Exactly one preset block changes. The other six are byte-identical and
reproduce their composition census to a tenth of a point; `worldgencheck.sh` is
clean. No worldgen pass is touched.

## What the lever actually is

`soil_depth` is close to vacuous, and this is the part worth carrying away.
`column::plan_from` gives **zero** soil to any column steeper than
`soil_slope_cutoff × repose`, and `column::taper_cover` then walks that zero
outward one repose step (~0.7 cells) per column — so the blanket is bounded by
*the distance to the nearest bare column*, and one crag thins it for a couple of
hundred columns either side. Swept: `soil_depth` **210 / 400 / 700 / 1000** gives
a viewport soil share of **8.4 / 8.5 / 8.5 / 8.5 %**. 4.8× on the knob, a tenth
of a point on the answer.

What moves it is taking the crags out:

- **`world_age` 0.8 → 0**, the single biggest one. Erosion cuts the gullies and
  leaves the hard bands standing, and every one of those is a bare column.
  Measured the wrong way round first: *more* erosion made it worse, `world_age`
  1.0 → 8.0 taking bare columns 7.3% → 12.7%.
- **the short-wavelength terrain terms down** — a hill term of amplitude 30 at
  wavelength 150 peaks at a slope of 1.26 against a soil cutoff of 0.65.
- **the shape put back at long wavelengths**, where it is safe: `relief_amplitude`
  34 → 48 and the massif 70 → 170 over 3,200 columns, which peaks at 0.24.
- **`soil_depth` 210 → 380**, now that it binds. Not higher: `passes::soil_shade`
  ramps the topsoil-to-subsoil profile over the blanket's *own* depth, so a
  600-deep blanket puts the whole visible cutaway in one tone and the ground
  reads as a flat dark slab.

## The at-rest half — and a note for the `settle_frames` lane

`generated_terrain_is_already_at_rest` caught one soil grain in motion. The
cause is **`warp_strength`**: domain warp displaces the sample position, so its
own gradient *multiplies* the slope of every term downstream and can push a
column past repose that no amplitude in the preset would. Turning it off costs
**0.2 points** of rock and clears the grain.

`soil_slope_cutoff` also comes **down**, 1.1 → 1.0. The file header says soil
above 1.0 avalanches; the held world's first version shipped 1.1, past it. The
guard sweeps **five** seeds, and CLAUDE.md's *"six seeds is not a sweep"*
applies — widened to twenty locally, only one combination tried holds:

| | at-rest, 20 seeds × 7 presets |
|---|---|
| `warp 0 / cutoff 1.0` | **clean** |
| `warp 0 / cutoff 1.1` | fails, seed 12 |
| `warp 4 / cutoff 1.0` | fails, seed 14 |

**This may make `settle_frames` unnecessary, and the reason is worth checking
before that field lands.** The parallel lane measured `soil_slope_cutoff` at
1.1 → 15 cells moving and every value from 1.05 down to 0.8 → 2 cells, and added
`settle_frames` for the residual. That sweep was taken with `main`'s druid
`warp_strength: 34.0` at wavelength 130 — a warp gradient of up to 1.64, i.e. an
amplifier of up to ~2.6× on every local slope. With the warp off the residual is
not 2 cells, it is **zero over twenty seeds and seven presets with no settling
pass at all**. That is CLAUDE.md's *"a constant nobody can tune in either
direction may be a counterweight, not a model"*: a settling pass may be
compensating for placement the warp made unstable, rather than for a placement
rule that is wrong. Worth one run before shipping the field — and if
`settle_frames` lands anyway it is free headroom here, not a conflict.

This PR and that branch both edit the `druid` block, so one of them will
conflict; whichever lands second should take the union — `warp_strength: 0.0`
from here, and `settle_frames` from there.

## Tried and rejected — both in `Reports/dead-ends.md`

- **Promoting erosion's two stable-angle constants (`THERMAL_STABLE_SOFT`,
  `THERMAL_STABLE_HARD_BONUS`) to preset data**, so the held world could weather
  to a single gentle angle below the soil cutoff. Landed cleanly, all six other
  presets byte-identical, and did nothing it was built for: 0.30/0.12 read
  **79.1%** rock against the unmodified **79.8%**, and 0.18/0.05 at `world_age`
  2.0 read **76.9%** with *more* bare columns than the baseline. Two settings an
  order of magnitude apart failing the same way is the approach being wrong —
  thermal relaxation can only remove over-steepness; the gullies are cut by the
  hydraulic pass and the differential strip term, and neither angle gates those.
  Reverted; the pass is untouched and `world_age: 0.0` reaches the goal for free.
- **Raising `soil_slope_cutoff` to buy soil coverage.** 5.0 reaches 9.7% rock and
  would avalanche the world on frame one. It buys soil by placing it on ground it
  cannot stand on; coverage has to come from gentler ground.

## The instrument

`world_look` grows three things, because the numbers this question needs did not
exist:

- **`world=WxH`** — it was pinned to the sandbox's 8192×2560, so every number it
  produced was a statement about a world the held game does not build.
- **a `ground:` roll-up by material kind** — rock arrives under six names and
  reads as one grey thing, which is how 8.4% soil sat behind four separately
  unalarming rock rows.
- **a `cover to rock:` percentile line**, plus the share of columns bare at the
  surface. That is the quantity `taper_cover` actually governs; read it rather
  than `soil_depth`.

Positive control, unchanged and checked before any of the above was believed:
`flat` reads **ROCK 100.0%**, cover 0, bare 95.7%.

## Gates

`cargo test --release` (lib **and** the integration binaries — `--lib` cannot
reach `tests/worldgen.rs`), `cargo clippy --all-targets --release --locked
-- -D warnings`, `scripts/worldgencheck.sh`, `scripts/seedsweep.sh`,
`scripts/docscheck.sh`. `main` merged in at 80 commits behind; the dead-ends
index and triage verdicts regenerated for the two new register entries.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01TngZpRY8LoqpFWHuXjUTTD
