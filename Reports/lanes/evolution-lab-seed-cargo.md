# Round 29 — the seed is cargo

*Brief 1 of the late-game programme
([`evolution-lab-late-game-design-2026-09-12.md`](../evolution-lab-late-game-design-2026-09-12.md)
§2). Owner ruling carried into it, 2026-09-12: "You can ship everything on.
I will tell you to change it if I don't like it" — so the leaf-marginal
default is shipped, not gated.*

## What a later session cannot re-derive

**`grassblade.ron` authored neither `food_energy` nor `food_class`, so a
blade of grass has never been food at all.** Both fields are
`#[serde(default)]`, i.e. `0.0`, so `food_value` returned zero and every
mouth in the world was already blind to a sward. The design report's
instruction — "leaf.ron/grassblade.ron `food_energy` 480 → 40" — is a *cut*
for one file and a *rise* for the other. Shipped at 40 anyway (with an
explicit `food_class: -1.0`, which was also unauthored and therefore sat at
the flesh/plant midpoint), because 40 is under the 12 J bar at the shipped
neutral gut and so changes nothing today; what it buys is that a lineage
that evolves a plant gut can graze the meadow and not only the canopy.

**The bite site is a pickup, not a meal.** `creature.rs`'s ingest branch puts
the mouthful in `Crop` at face value and books nothing; the gut is applied at
digestion. So "the bite pays a provision fraction" had to be a change to the
`worth` that enters the crop, made *after* `seed_survives_bite` returns and
*before* the crop is written — `worth` is read off the standing cell earlier
because the roll rewrites that cell to `pip` in place.

**`seed_survives_bite` had to stop being a `bool`.** The two survivals are
priced differently and only that function knows which it saw, so it returns
`SeedBite::{Digested, SurvivedInFlesh, SurvivedBare}`. Re-deriving "was that a
bare seed?" at the call site from the material is the "readout derived
separately from the mechanism" failure `food_value`'s own doc records; it is
also *wrong* in one case, because `windfall_material` defaults to the literal
string `"seed"` for a species with no fruit, so `cell.material ==
windfall_id` is true for a grass seed and does not mean flesh. The flesh test
is therefore the narrower one and is asked first.

**There is no conservation gap at the drop, and it is worth knowing why**
before anyone "fixes" it: `Crop::passenger` rides *in place of* one flesh
cell, so the drop verb delivers the passenger **instead of** `unit.into_cell`,
never beside it. A bitten seed cannot therefore become a replanted seed *and*
a 480 J food cell on the floor.

**A seed riding in a crop owns no cell**, so any census that identifies the
waiting bank as "a one-cell organism whose cell reads `CellType::Seed`" puts
every passenger in the *plant* column instead. `World::is_carried_seed` was
added for that; `latecensus` and `labforage` both use it. Small (one per
carrying ant) and biased in the direction that flatters this build, which is
why it was closed rather than noted.

## What the second bite site does

`creature.rs`'s *other* `seed_survives_bite` call is the budding-provision
path — a parent topping up a birth shortfall from food in reach. It is a bite,
so the new rule reaches it, and a bare seed taken to fund a birth is now
spared. It has no crop, so there is no passenger: the survivor stands where it
was bitten as a `pip`. Its yield is re-priced by the same fraction, because a
parent paid the whole seed **and** left it standing would be the one free
lunch this build must not create.

## The kill switch and what it cannot switch

`PIXEL_PHYSICS_SEED_CARGO=0` restores the windfall-only material test in one
binary. It **cannot** restore `leaf.ron`'s 480: that is an asset, embedded by
`include_str!`, not a branch. The byte-identity control therefore needs the
switch *and* a build with the two food values put back —
`scratchpad/runs/revert_assets.py` in the session that built this, or by hand:
`leaf.ron` 40 → 480, `grassblade.ron` 40 → 0.

## Numbers, and the two that would surprise a later session

Full tables in PR #342. `played_bed`, `RAYON_NUM_THREADS=1`, 500,000 frames,
ants / plants / seed bank at the end, `main` (b9cd1f44) against this build:
seed 1 **108 / 58 / 39** against **0 / 201 / 1,574**; seed 2 0 / 102 / 368
against 0 / 230 / 1,090; seed 3 **0 / 2 / 0** against **0 / 114 / 402**.

**`main` reaches 3,182 ants on seed 1 at 300,000 frames** — above the owner's
own 500–1,000 report, on 13,897 starvation deaths — and leaves the bed at 58
plants over 39 seeds. This build never leaves single figures on that seed. **It
saves the bed and it shrinks the colony**; it does not keep a colony alive that
`main` loses, and on seed 1 `main`'s colony outlives it.

**`DeathCause::Killed` appears in every arm of this build and almost nowhere on
`main`** — `labforage` at 120,000 frames reads KILLED 56 / 37 / 153 on seeds
1–3 against 0 / 1 / 0. It is **not** nestmate predation by the food rule
(`a_colony_does_not_eat_itself` is asserted on the predicate and is green), so
the likely reading is more colonies meeting each other — but that is a reading
and not a measurement, and **nobody has looked**. Do it before Brief 2 adds a
second mortality channel on top, or the two will be inseparable.

**Lineage depth does not move in one direction and a clean story here would
have been the tell**: deepest generation 10 → 25 on seed 3, 10 → 8 on seed 1,
7 → 3 on seed 2. Do not quote the seed-3 number alone.

## Rendering the whole session as a card

**`labgif` cannot make a long-span animation on its own, and the reason is not
obvious from its `--help`.** Its GIF frame delay is derived from the capture
interval -- `delay_ms = every * 1000 / 60`, i.e. it always plays back at
real-world speed. That is right for the mister it was built for and useless for
a 500,000-frame session: at `every=2000` each frame would hold for **33
seconds**. The route that does not touch a contested file (`examples/labgif.rs`
carries salvage commits from two other lanes) is `png_dir=`, then assemble the
GIF outside with Pillow at a chosen delay. 251 captures at `every=2000`, cropped
to `(0, 34, 512, 242)` -- which drops the two top chrome lines and the whole
bottom UI bar but keeps the frame counter -- quantised to 255 colours, is
**1.0 MB** for ten seconds of playback; uncropped and per-arm-quantised it is 13 MB.

**Fit one palette across BOTH arms.** Quantising each arm separately gives the
arm with more foliage more greens, which is a colour difference the owner could
read as the answer without ever looking at the plants -- on a blind A/B that is
the card deciding itself.

**`labgif` overrides the scenario's own rain rate with `steady` by default**
(`rain=` in its header), while `latecensus` takes whatever the scenario ships.
`played_bed` ships `Off`, so a card rendered at the default is a *different
world* from the census whose counters are printed beside it. Pass `rain=off`.

## For Brief 2 and Brief 3

- `CreatureDef`/`Species` already carry `life_half_life` (the plant field);
  Brief 2's ant lifespan needs a *creature* one, not this.
- The `seed_provision_fraction` field is a plain `f32` on `SpeciesDef`, not on
  `ParamGenome`, so it is authored and not yet heritable. A probability's
  range `[0,1]` sits inside `PARAM_REACH`'s `[-4,4]`, so the fallback that
  starved `seed_launch` would not starve this one — the same note
  `seed_gut_survival` carries.
