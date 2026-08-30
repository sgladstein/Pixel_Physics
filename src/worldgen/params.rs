//! Worldgen parameters, as data rather than constants.
//!
//! `Reports/design-philosophy.md` §2a is the rule this file exists to
//! satisfy: a constant graduates to hot-reloadable `.ron` the moment a
//! non-programmer might plausibly want to tune it, and *before* heavy tuning
//! starts rather than after — every experiment run before the migration costs
//! a recompile, and the migration gets harder as the constant count grows
//! (`Reports/worldgen-design.md` §11 step 1 puts this first for exactly that
//! reason).
//!
//! Everything here is shape, not structure. The internals that are *not*
//! here — fBm gain and lacunarity, the shade tone table, the pass order —
//! stay in the source because tuning them is a code change in disguise.

use std::collections::BTreeMap;

/// The reserved preset name for the hand-authored sandbox terrain.
///
/// Never appears in `assets/worldgen.ron`; the app resolves it to
/// [`super::Spec::Legacy`]. Kept selectable because a hand-authored scene with
/// known coordinates is what several filmstrip scenes and app tests are
/// written against, and because it is the control when judging whether
/// generated terrain is actually better.
pub const LEGACY: &str = "legacy";

/// One world's worth of generation shape.
///
/// All lengths are in cells and all wavelengths in cells of world x. Fields
/// are grouped the way the generator consumes them: surface composition
/// first, then the layers beneath it, then water, then life.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WorldgenParams {
    // ---- surface composition ----
    /// **How many cells this world spends per unit of ground**, relative to
    /// the size everything here was authored at. `1.0` is that size.
    ///
    /// Not a shape parameter like the rest of this struct -- it is the unit
    /// the rest are denominated in, and it exists because a handful of
    /// lengths live in the *source* rather than here and cannot otherwise
    /// find out that the world got finer. [`WorldgenParams::scaled`] is what
    /// sets it, and its doc has the whole argument.
    ///
    /// **The first consumer is the one that made it necessary.**
    /// `region::COMPOSITION_WINDOW` is 512 cells because that is "roughly one
    /// screen at 1:1" -- composition is a property of what fits in view, so a
    /// screen is the right unit and 512 is only its value at the resolution
    /// this was written at. Left hardcoded, a world with twice the cells gets
    /// *twice as many regions* rather than the same regions twice as wide, so
    /// the same seed builds a different landscape. Measured on `rolling` seed
    /// 1: rescaling with the regions keyed to a fixed 512 left the terrain
    /// **39.1 rows** from the original, against **42.5** for an entirely
    /// different seed -- no more like itself than a stranger. With the
    /// regions disabled the same rescale lands **1.18** rows out. So this one
    /// number was the whole difference between "the same world, finer" and "a
    /// different world".
    ///
    /// `worldgen` sits below `app` in this crate's layering and so cannot
    /// read `app::WIDTH` to work this out for itself, which is why it arrives
    /// as data.
    pub cell_scale: f32,

    /// Rows of clear sky kept above the highest possible ridge.
    pub sky_rows: f32,
    /// Amplitude of the base relief wave — the ridge-to-valley swing that
    /// guarantees a composition in every window. See `column::elev` for why
    /// its phase is deliberately *not* seeded.
    pub relief_amplitude: f32,
    /// Amplitude of the **massif**: country-scale relief, three to six
    /// screens between crest and crest.
    ///
    /// A different quantity from `relief_amplitude` rather than a bigger
    /// setting of it, and the distinction is the whole reason it exists.
    /// `relief_amplitude` is scaled by the region's `elev`, and a region is
    /// 96-241 columns at the shipped world size against a hill wavelength of
    /// 150-200 -- so it modulates the terrain **at the carrier frequency**,
    /// which does not read as high country and low country, it reads as
    /// noise (`Reports/worldgen-architecture-ceilings-2026-08-29.md` C4).
    /// A mountain has to be bigger than the view or the player never sees one:
    /// the same argument `region.rs`'s `ROCK_COUNTRY_SCALE` records for where
    /// rock country is, applied to how high the ground is.
    pub massif_amplitude: f32,
    /// Wavelength of that massif, in columns. Sized in *screens*.
    pub massif_wavelength: f32,
    /// Mid-frequency hills stacked on the base wave.
    pub hill_amplitude: f32,
    /// Wavelength of those hills.
    pub hill_wavelength: f32,
    /// Fine surface roughness, a few cells at most.
    pub detail_amplitude: f32,
    /// Wavelength of that roughness.
    pub detail_wavelength: f32,
    /// How far the height sample position is displaced before sampling.
    /// Domain warp is what turns symmetric bumps into asymmetric ridges with
    /// a steep face and a shallow one — the single cheapest thing that stops
    /// fBm terrain reading as fBm terrain.
    pub warp_strength: f32,
    /// Wavelength of the warp field.
    pub warp_wavelength: f32,
    /// Height of one terrace riser. Zero disables terracing.
    pub terrace_step: f32,
    /// How completely terracing replaces the smooth surface where it
    /// applies, 0..1.
    pub terrace_strength: f32,
    /// How ragged a terrace riser is, as a fraction of `terrace_step`.
    /// **`0.0` is the pre-review behaviour**, kept reachable so the change
    /// can be judged by eye rather than argued about.
    ///
    /// A riser is a single-column jump of `terrace_step * mask` rows — up to
    /// 34 on `canyon` — and `detail_amplitude` is 2.5 to 3.0, which is
    /// nowhere near enough to break a face that tall. So every bluff in the
    /// world had dead-plumb one-column sides. This adds a second, much
    /// larger detail term applied **only near a riser**: it fades in as the
    /// snap residual approaches the half-band where the jump happens, so
    /// benches stay flat and only the faces get column-scale variation.
    ///
    /// Scaled by `terrace_step` rather than being an absolute size, because
    /// what has to be broken up is the riser, and a riser is that tall by
    /// definition.
    pub riser_roughness: f32,
    /// Wavelength of the mask deciding *where* terracing applies. Terracing
    /// everywhere reads as a rendering artifact; terracing in patches reads
    /// as geology.
    pub mask_wavelength: f32,
    /// Regional slope at which the terrace snap starts to yield, and the
    /// slope by which it has yielded completely.
    ///
    /// **Where the ground is already steep, terracing must give way.** A
    /// riser is a single-column jump of `terrace_step * mask` rows whatever
    /// the ground under it is doing, so on an escarpment the snap stacks its
    /// own face on top of a face the relief already supplies: `canyon` seed 7
    /// put three risers of 27, 34 and 33 rows six columns apart, which is a
    /// ladder rather than a staircase. Attenuating by the *pre-terrace*
    /// regional slope leaves benches on gentle ground at full strength — a
    /// bluff standing out of quiet country is the landform this is for — and
    /// stops the snap contributing anything where the relief is already
    /// doing the work.
    ///
    /// Measured on the pre-terrace elevation over a +-8 column central
    /// difference, so it reads the escarpment and not the detail term.
    /// `terrace_slope_hi <= terrace_slope_lo` disables the attenuation and
    /// restores the pre-round-2 surface exactly.
    pub terrace_slope_lo: f32,
    pub terrace_slope_hi: f32,

    // ---- layers ----
    /// Nominal soil blanket thickness on flat ground.
    pub soil_depth: f32,
    /// Fraction of soil's own friction angle above which soil is omitted
    /// entirely. This is the at-rest guarantee for the soil pass: below the
    /// cutoff a slope cannot avalanche, so generated soil never moves.
    pub soil_slope_cutoff: f32,
    /// Depth of the noise-perturbed band above bedrock, so the world floor is
    /// not a ruler line.
    pub bedrock_band: f32,
    /// Thickness of one sedimentary band in the stone shade channel.
    pub strata_thickness: f32,
    /// Tilt of those bands, in cells of rise per cell of run.
    pub strata_tilt: f32,
    /// Amplitude of the fold applied to the bands, so strata bend rather than
    /// running dead straight across the world.
    pub strata_fold: f32,
    /// Expected sand/gravel lenses per 64x64 region. Fractional values mean
    /// "sometimes one".
    pub pocket_density: f32,
    /// How far a lens's outline departs from the ellipse it is built on,
    /// as a multiplier on `LENS_LOBE` and `LENS_GRAIN` in `passes.rs`.
    ///
    /// **`0.0` is the pre-review behaviour** -- an exact rotated ellipse --
    /// kept reachable so the change can be judged by eye rather than argued
    /// about, which is this repo's convention for a look-at-it question.
    ///
    /// Here because the owner has an opinion about it and it is a `.ron`
    /// field away from being tunable with F5 in the running app: *"The ovals
    /// of sand throughout the stone looks bad and should be fixed. It should
    /// be a more natural shape than perfect ovals."* `Reports/design-
    /// philosophy.md` §2a says a constant graduates to hot-reloadable data
    /// the moment a non-programmer might plausibly want to tune it, and
    /// *before* heavy tuning starts rather than after.
    ///
    /// Above about 1.5 the outline starts pinching lenses into disconnected
    /// lobes -- which real lenses do at their ends, so it is not a bug, but
    /// it stops reading as one body.
    pub lens_roughness: f32,
    /// Tallest gravel apron heaped at the base of a cliff. Zero disables.
    pub talus_max_height: f32,
    /// Chance that a given cliff edge grows an overhanging lip, 0..1. Zero
    /// disables.
    pub brow_chance: f32,

    // ---- water ----
    // Every field in this group is a pivot lever. `table_offset` alone, set
    // past the world height, produces a completely dry world with no ponds
    // and no moisture floor -- which is what the `arid` preset ships, and the
    // escape hatch if the water table turns out not to be fun
    // (`Reports/worldgen-design.md` deliberately does not assume it is).
    /// How strongly the water table copies surface relief, 0..1. The table is
    /// a *subdued replica* of topography — high under hills, low under
    /// valleys, but flatter than either.
    pub table_damping: f32,
    /// Cells the table sits below the mean surface. Large values (past the
    /// world height) mean no water at all.
    pub table_offset: f32,
    /// Cells above the table over which moisture ramps from saturated to dry.
    pub capillary_fringe: f32,
    /// Ponds shallower than this are not generated. A one-cell film of water
    /// renders as a black line rather than as water (`render.rs` dims liquid
    /// toward black by fill) and reads as an artifact.
    pub pond_min_depth: f32,
    /// Ponds narrower than this are not generated, for the same reason.
    pub pond_min_width: f32,

    // ---- regional variation ----
    /// How dry this world is on average, `0` lush to `1` desert.
    ///
    /// One axis moving four things — loose cover from soil toward sand, the
    /// water table down, dune shaping on, and what can grow thinned out —
    /// because that is what makes a place read as *dry*, rather than as four
    /// unrelated settings that happen to coincide.
    pub aridity: f32,
    /// Height of dune crests in country dry enough to be sand. Zero disables.
    pub dune_amplitude: f32,
    /// Spacing of those crests.
    pub dune_wavelength: f32,
    /// How much each individual dune varies from that amplitude and that
    /// spacing, `0`..`1`. **`0.0` is the pre-review behaviour**, kept
    /// reachable rather than deleted so the change can be A/B'd by eye
    /// instead of argued about (the repo's runtime-selector convention).
    ///
    /// The phase term was `x / wavelength + 0.6 * fbm`, and the linear part
    /// dominates it so completely that the whole desert came out as one
    /// wavelength repeated ~30 times across the world — a mechanical
    /// sawtooth comb, which the world review put fourth in what it costs the
    /// picture. This draws each dune's own amplitude and its own slip-face
    /// fraction from noise keyed on the *dune index*, so crests differ in
    /// height and sit at different distances apart.
    ///
    /// Every dune's slip face is still clamped to what sand can stand on,
    /// now against its **own** amplitude and its own fall fraction rather
    /// than the preset's — see `column::Terrain::dunes`, where getting that
    /// wrong once already produced a desert of bare grey spikes.
    pub dune_variation: f32,
    /// Extra cells the water table drops in fully arid country, on top of
    /// `table_offset`. This is what stops a desert having ponds in it.
    pub aridity_table_drop: f32,
    /// How far the regions of a world stray from the preset, `0`..`1`.
    ///
    /// Zero makes a world uniform end to end, which is what a structural test
    /// bed needs and what every world used to be. Above zero the world is cut
    /// into two to five regions that each draw their own elevation and
    /// character — see `region.rs` for why this is the difference between
    /// "different numbers" and "a different world".
    pub region_variation: f32,

    // ---- vaults ----
    /// Expected number of sealed chambers per world. Zero disables the pass
    /// entirely and leaves the world byte-identical.
    ///
    /// Fractional on purpose, and read as "roughly this many *draws*": the
    /// whole number is guaranteed and the remainder is one coin flip, the
    /// same shape `pockets` uses for its per-region count. A draw is not a
    /// system -- the depth band, the envelope fit and the seal all reject --
    /// and the default prices that in: at 1.6 (two draws at most, so a
    /// system stays a 0-2-per-world event) 32 of 40 preset x seed worlds
    /// carry a system and exactly one carries two, measured at the shipped
    /// size. The round-2 value of 0.6 left systems in barely a third of
    /// worlds once the round-3 envelope grew the rejection surface; the
    /// measured curve is chaotic in the seed (1.5 placed 20 of 40, 1.6
    /// placed 32), so re-tune against the placement probe, not by
    /// arithmetic on the fraction.
    pub vault_density: f32,
    /// How far below the local genesis surface a chamber must sit, in rows.
    ///
    /// Concealment comes free from the viewport rather than from any render
    /// work: a chamber this far down is simply never on screen until someone
    /// digs to it. **Note this interacts with world height** -- at the
    /// 512x320 test size the band between this depth and the bedrock margin
    /// is empty, so no vault can be placed at all. See the round-2 finding;
    /// it is the reason the vault tests build at the shipped size or lower
    /// this number explicitly.
    pub vault_min_depth: i32,
    /// How far above the bedrock band a chamber must stop, in rows.
    pub vault_bedrock_margin: i32,
    /// How far the slow 2-D field is allowed to displace a palette-family
    /// probability, in absolute probability.
    ///
    /// `0.0` restores the per-column threshold round 1 shipped (the aridity
    /// ramp widths changed with it, so it is not quite byte-identical -- see
    /// the round-2 finding), and a preset with `region_variation <= 0.0`
    /// never reaches this code at all. Behind a param because "does this read
    /// as country or as stipple" is a by-eye question, and the repo's
    /// convention for those is a runtime selector rather than an argument.
    pub palette_field: f32,

    // ---- residual landforms ----
    /// Expected residual sites (tors, stacks, pinnacles) per 256-column
    /// region, before the region's own `Character::formation` multiplies
    /// it up or down.
    ///
    /// **Back at 1.4 after a round trip through 0.45.** The knob answers
    /// *how emphatic is rock country where there is rock country*; whether
    /// there is any here at all belongs to `region::FORMATION_BARREN`, and
    /// conflating the two is what the round trip was.
    ///
    /// Phase 2 cut it 1.4 -> 0.45 against the owner's *"they shouldn't be
    /// common ... shouldn't be dominate"*, set from a sweep, and it was
    /// rejected on the render: *"Spires should not just be thinned out. They
    /// should be part of a specific biome. They should not exist at all in
    /// most biomes but some biomes should have them and they can be more
    /// regular. **I didn't mean a uniform decrease in spires.**"*
    ///
    /// The measurement that set 0.45, kept because it is the bar the gate had
    /// to beat -- `probe_p2_how_common_are_standing_features`, spires per
    /// 512-column screen over 48 screens, **paired against
    /// `residual_density: 0.0`** so the count is the residual pass and not
    /// `boulders`, `brows` or a proud talus toe (half of them on `canyon` and
    /// nearly all of them on `rolling`; tuning against the unpaired number
    /// would have been tuning `boulders`):
    ///
    /// | density | canyon med/p90/max | rolling med/p90/max | heights p90 |
    /// |---|---|---|---|
    /// | 3.0 | 2 / 5 / 7 | 2 / 4 / 5 | 35 / 36 |
    /// | 1.4 | 1 / 3 / 5 | 1 / 2 / 3 | 35 / 33 |
    /// | 0.45 | 0 / 2 / 4 | 0 / 1 / 2 | 28 / 23 |
    ///
    /// (controls: canyon 0 / 2 / 3, rolling 0 / 1 / 2.)
    ///
    /// Read across, that table is the whole argument against tuning this knob
    /// for the complaint. At 0.45 `rolling` is **identical to its control in
    /// all three statistics** -- the pass had stopped contributing anything,
    /// so what remained on screen was boulders and talus that this knob
    /// cannot reach, and no further cut could have helped. And `heights` p90
    /// fell with it (33 -> 23): thinning the count also shrinks the
    /// monuments, because a smaller count is fewer draws at the tall tail.
    /// A median of 1 at 1.4 is the other half -- every other screen holding a
    /// spire is "formations everywhere", which is what needed fixing.
    ///
    /// Zero disables the pass entirely and leaves the world byte-identical --
    /// the same contract `pocket_density` and `vault_density` make.
    ///
    /// Round 6 Track B, B2 (`Reports/worldgen-implementation-tasks-round6-
    /// formations.md`). B1 measured that plan-space erosion never produces
    /// a residual-scale candidate on its own -- max prominence at reach 15
    /// across a full erosion run peaked at 8.34 (canyon) / 5.00 (rolling),
    /// never once crossing into the 12-120 cell band a residual occupies --
    /// so this is authored placement, not a rate this pass tunes toward.
    pub residual_density: f32,

    /// Columns of spring emission to spend on this world -- the engine's own
    /// budget unit (`sim::spring::MAX_TOTAL_SPAN`), not an invented one, so
    /// the number here and the number the simulation enforces are the same
    /// number. `5.0` is one waterfall; `sim::spring::MAX_SPAN` (6) caps a
    /// single outlet, so a larger budget buys more *places*, not a wider
    /// sheet.
    ///
    /// `0.0` switches the pass off and leaves the world byte-identical --
    /// the same contract `residual_density`, `pocket_density` and
    /// `vault_density` make. `arid` and `flat` ship `0.0` explicitly, and
    /// would place nothing anyway: both put the water table past the world
    /// floor, so no cliff face can intersect it.
    ///
    /// **The measured price of switching it on**, at 8192x2560, from
    /// `ascii`'s river-cost scene (the instrument the rivers track was
    /// opened with): a spring, its fall and its pool cost **+2.645 ms/frame**
    /// standing -- 7.135 -> 9.780 ms over 1400 frames -- and take the world
    /// from **0 awake chunks to 7** of 5120. That is over the 2.0 ms bar the
    /// harness prints and under the ~3.5 ms wind-revert class. It is a
    /// standing cost with no end: the world never sleeps again while a
    /// spring runs.
    pub spring_flow: f32,

    // ---- history ----
    /// How much simulated history the terrain has been through, `0` none.
    ///
    /// The iteration budget for plan-space erosion (`erosion.rs`,
    /// `Reports/worldgen-erosion-design.md`): young worlds are sharp —
    /// plumb-ish terraces, thin talus, no valley fill — and old worlds are
    /// subdued and heaped. This is the `age` axis `worldgen-design.md` §6
    /// reserves in `worldgen(seed, coord, age)`, landed first for erosion;
    /// ecological age joins it there later. It is **not** a live process:
    /// nothing erodes at runtime.
    ///
    /// **`0.0` is the pre-erosion world, exactly** -- at zero the erosion
    /// pass returns before touching a column. That *was* the default for
    /// every preset (`Reports/worldgen-erosion-design.md`'s "Status,
    /// 2026-08"), kept there deliberately while the mechanism was tuned by
    /// eye at explicit ages against the sweep baselines. Round-4 task 4
    /// flips it on: `rolling` (and so `WorldgenParams::default`, which
    /// `rolling` is asserted equal to) ships `0.8`, and the 16-seed sweep
    /// was re-baselined in the same task to match. `flat` -- the structural
    /// test bed -- stays `0.0` explicitly in `assets/worldgen.ron`, since it
    /// must not inherit whatever `rolling`'s age becomes.
    pub world_age: f32,

    // ---- life ----
    /// Per-column moss probability scale. Zero disables plant scatter.
    pub moss_density: f32,
    /// Per-column tree probability scale.
    pub tree_density: f32,
    /// Per-column grass probability scale — **the ground layer's own knob,
    /// deliberately not a share of `tree_density`.**
    ///
    /// Grass is a second layer rather than a fifth woody species, and the
    /// divisor is why. The four woody weights split one budget
    /// (`weight / max(1, Σ weights)`), so a fifth entry in that table would
    /// take its columns *from* conifer, shrub, creeper and tree — thinning
    /// the four species the previous pass had just finished putting into the
    /// world, and changing what `tree_density` means at the same time. Moss
    /// has always been its own layer for the same reason; grass joins it
    /// there.
    ///
    /// Set from the establishment sweep rather than from an aspiration —
    /// see `GRASS` in `passes.rs` for what a founder density buys once
    /// reproduction is running, and for the organism-slot ceiling that
    /// bounds it from above.
    ///
    /// **0.50 is the owner's call, not a derived number**, and the record of
    /// it is the point. It shipped at 0.35, which put five tussocks across
    /// 192 columns on `canyon`; asked directly on a review card whether that
    /// was the right amount of ground cover, the answer was *"I would say
    /// noticeable more grass, but it should also spread over time, so this
    /// could be ok to start. Maybe increase it a little bit"*. So this is a
    /// modest step (x1.43 on every preset, keeping their relative design),
    /// not the large one the first clause on its own would suggest.
    ///
    /// **The second clause is the one to read before raising it again.**
    /// "It should also spread over time" is an expectation this knob cannot
    /// meet: raising it raises the *starting* amount, and grass thins as the
    /// canopy closes over it — 3 of 40 still standing at 45,000 frames on
    /// `rolling`, against 63 of 43 with the woody layer switched off. Most
    /// of any increase is shaded out too. What would make grass spread is
    /// disturbance (W2's fire) or shade tolerance (lane P's `shade_death`),
    /// and neither is this field.
    ///
    /// Whoever moves it next: `grass_is_sown_across_a_seed_sweep`'s median
    /// bars are re-derived from the sweep each time, and leaving them at the
    /// previous density's numbers is how a guard stops meaning anything.
    pub grass_density: f32,
    /// Wavelength of the clustering field that both densities are multiplied
    /// by. This is the anti-even-spacing device: uniform probability produces
    /// evenly scattered plants, which is exactly the failure mode a
    /// side-view world has to avoid.
    pub life_cluster_wavelength: f32,
}

/// The `rolling` preset's values.
///
/// Also the fallback for any field a preset omits (`#[serde(default)]` on the
/// struct), which gives presets inheritance for free: a preset lists only
/// what it changes and still loads if a later version adds a field.
impl WorldgenParams {
    /// This same shape at `k` times the cell resolution — the same world,
    /// built out of `k` times as many cells per unit of ground.
    ///
    /// **Written for the resolution step** (`Reports/resolution-step-2026-08-29.md`),
    /// whose whole content half is "make every feature `k` times as many cells
    /// across". The render side of that is already paid for; this is the first
    /// piece of the content side, and it exists because the obvious version of
    /// it — multiply the numbers by two — is wrong.
    ///
    /// **These 46 fields carry four different dimensions, and only one of them
    /// scales as `k`.** The classification is the content of this function; the
    /// arithmetic is trivial. Getting it wrong does not fail to compile and does
    /// not fail a test — it produces a world that is subtly the wrong density of
    /// everything, which is the failure `CLAUDE.md` describes under *fixing a bug
    /// often exposes a constant that was compensating for it*.
    ///
    /// | dimension | factor | why |
    /// |---|---|---|
    /// | a length or a wavelength, in cells | `k` | a ridge `n` cells tall is `nk` cells tall when a cell is `1/k` the size |
    /// | dimensionless — a ratio, a probability, a slope | `1` | rise-over-run is unitless; both terms scale and it cancels |
    /// | a per-column probability | `1/k` | there are `k` times as many columns across the same ground, so each must fire `1/k` as often |
    /// | a count per fixed *cell* region | `1/k²` or `1/k` | the region is a hardcoded cell size, so the same ground now holds `k²` (area) or `k` (column-strip) of them |
    ///
    /// **The last row is the trap**, and it is only visible from the consumer
    /// rather than the field: `pocket_density` is drawn once per 64x64 cell
    /// region over a *2-D* loop (`passes.rs`'s `REGION`), so it takes `1/k²`,
    /// while `residual_density` is drawn per 256-*column* region over a 1-D loop
    /// (`residual.rs`'s `REGION`), so it takes `1/k`. Two fields whose names,
    /// types and doc comments all read the same way, needing different factors.
    ///
    /// **What this deliberately does not do.** Those `REGION` constants are
    /// themselves lengths in cells and stay put, so the placement *grid* gets
    /// finer while the density compensates — the counts come out right and the
    /// grid is not what anyone looks at. The things sized by other source
    /// constants (`LENS_LOBE`, the cave and speleothem widths, `MIN_FRACTURE_
    /// CELLS`) do not move at all, so a params-only rescale gets the *surface*
    /// right and leaves everything below it at the old size. That is the honest
    /// boundary of this function and the next piece of work after it.
    ///
    /// `k == 1.0` returns an identical value, which
    /// `scaling_by_one_changes_nothing` checks — so this is safe to route the
    /// unscaled path through.
    pub fn scaled(&self, k: f32) -> Self {
        // **Destructured exhaustively on purpose.** No `..`, so adding a field
        // to `WorldgenParams` stops compiling here until somebody says which of
        // the four classes above it belongs to. That is the only mechanism in
        // this file that will still be working in six months: a test over the
        // current 46 fields says nothing about the 47th, and the failure this
        // guards against is silent.
        let Self {
            cell_scale,
            sky_rows,
            relief_amplitude,
            massif_amplitude,
            massif_wavelength,
            hill_amplitude,
            hill_wavelength,
            detail_amplitude,
            detail_wavelength,
            warp_strength,
            warp_wavelength,
            terrace_step,
            terrace_strength,
            riser_roughness,
            mask_wavelength,
            terrace_slope_lo,
            terrace_slope_hi,
            soil_depth,
            soil_slope_cutoff,
            bedrock_band,
            strata_thickness,
            strata_tilt,
            strata_fold,
            pocket_density,
            lens_roughness,
            talus_max_height,
            brow_chance,
            table_damping,
            table_offset,
            capillary_fringe,
            pond_min_depth,
            pond_min_width,
            aridity,
            dune_amplitude,
            dune_wavelength,
            dune_variation,
            aridity_table_drop,
            region_variation,
            vault_density,
            vault_min_depth,
            vault_bedrock_margin,
            palette_field,
            residual_density,
            spring_flow,
            world_age,
            moss_density,
            tree_density,
            grass_density,
            life_cluster_wavelength,
        } = *self;

        // A distance in cells, whichever axis.
        let len = |v: f32| v * k;
        // A distance in cells held as a row count.
        let rows = |v: i32| (v as f32 * k).round() as i32;
        // A probability evaluated once per column.
        let per_column = |v: f32| v / k;

        Self {
            // ---- the unit the rest are denominated in ----
            // Multiplied rather than assigned, so `scaled` composes: a world
            // already at 2x taken to 2x again is at 4x, and the source-side
            // lengths that read this get the right answer either way.
            cell_scale: cell_scale * k,

            // ---- lengths and wavelengths: x k ----
            sky_rows: len(sky_rows),
            relief_amplitude: len(relief_amplitude),
            massif_amplitude: len(massif_amplitude),
            massif_wavelength: len(massif_wavelength),
            hill_amplitude: len(hill_amplitude),
            hill_wavelength: len(hill_wavelength),
            detail_amplitude: len(detail_amplitude),
            detail_wavelength: len(detail_wavelength),
            warp_strength: len(warp_strength),
            warp_wavelength: len(warp_wavelength),
            terrace_step: len(terrace_step),
            mask_wavelength: len(mask_wavelength),
            soil_depth: len(soil_depth),
            bedrock_band: len(bedrock_band),
            strata_thickness: len(strata_thickness),
            strata_fold: len(strata_fold),
            talus_max_height: len(talus_max_height),
            table_offset: len(table_offset),
            capillary_fringe: len(capillary_fringe),
            pond_min_depth: len(pond_min_depth),
            pond_min_width: len(pond_min_width),
            dune_amplitude: len(dune_amplitude),
            dune_wavelength: len(dune_wavelength),
            aridity_table_drop: len(aridity_table_drop),
            life_cluster_wavelength: len(life_cluster_wavelength),
            vault_min_depth: rows(vault_min_depth),
            vault_bedrock_margin: rows(vault_bedrock_margin),
            // `spring_flow` is a budget denominated in *columns* of emission
            // (`sim::spring::MAX`), so the same physical spring wets the same
            // ground only if the budget grows with the column count.
            spring_flow: len(spring_flow),

            // ---- dimensionless: unchanged ----
            // Ratios and fractions of another field that is itself scaled
            // (`riser_roughness` of `terrace_step`, `soil_slope_cutoff` of the
            // friction angle), plain probabilities, and the two slopes --
            // `strata_tilt` and the terrace pair are rise over run, so both
            // terms scale and the quotient does not.
            terrace_strength,
            riser_roughness,
            terrace_slope_lo,
            terrace_slope_hi,
            soil_slope_cutoff,
            strata_tilt,
            lens_roughness,
            brow_chance,
            table_damping,
            aridity,
            dune_variation,
            region_variation,
            palette_field,
            // Count per *world*, not per region -- `passes.rs` takes its floor
            // plus a fractional chance once -- and the world is the same
            // world. A scaled `vault_density` would put four times the
            // chambers in the same ground.
            vault_density,
            // **Unresolved, and left alone deliberately rather than by
            // oversight.** This is an iteration budget for erosion -- a
            // *time*, not a length -- so `k` is not obviously right; but
            // erosion moves material a cell at a time, so at `k` times the
            // cells it plausibly needs `k` (or `k²`) times the steps to carve
            // the same physical valley. Nobody has measured which, it is the
            // one field here whose factor is a question rather than a
            // classification, and getting it wrong shows up as terrain that
            // is the right shape and the wrong age. See the report.
            world_age,

            // ---- densities: divided ----
            // Per-column probabilities. `k` times as many columns cross the
            // same ground, so each has to fire `1/k` as often or the world
            // grows `k` times the trees.
            moss_density: per_column(moss_density),
            tree_density: per_column(tree_density),
            grass_density: per_column(grass_density),
            // Per 256-*column* region, drawn in a 1-D loop over `rx` only
            // (`residual.rs`), so it divides once.
            residual_density: per_column(residual_density),
            // Per 64x64 *cell* region, drawn in a 2-D loop over `rx` and `ry`
            // (`passes.rs`), so the same ground now holds `k²` regions and it
            // divides twice. The difference from `residual_density` above is
            // the whole reason this function is a table and not a multiply.
            pocket_density: pocket_density / (k * k),
        }
    }
}

impl Default for WorldgenParams {
    fn default() -> Self {
        Self {
            cell_scale: 1.0,
            sky_rows: 190.0,
            relief_amplitude: 46.0,
            massif_amplitude: 260.0,
            massif_wavelength: 2600.0,
            hill_amplitude: 30.0,
            hill_wavelength: 150.0,
            detail_amplitude: 2.5,
            detail_wavelength: 28.0,
            warp_strength: 34.0,
            warp_wavelength: 130.0,
            terrace_step: 26.0,
            terrace_strength: 0.9,
            riser_roughness: 0.45,
            mask_wavelength: 150.0,
            terrace_slope_lo: 0.6,
            terrace_slope_hi: 2.0,
            soil_depth: 105.0,
            soil_slope_cutoff: 0.8,
            bedrock_band: 4.0,
            strata_thickness: 9.0,
            strata_tilt: 0.06,
            strata_fold: 6.0,
            pocket_density: 0.6,
            lens_roughness: 1.0,
            talus_max_height: 12.0,
            brow_chance: 0.8,
            table_damping: 0.35,
            table_offset: 12.0,
            capillary_fringe: 24.0,
            pond_min_depth: 2.0,
            pond_min_width: 4.0,
            aridity: 0.35,
            dune_amplitude: 14.0,
            dune_wavelength: 46.0,
            dune_variation: 0.7,
            aridity_table_drop: 90.0,
            region_variation: 0.75,
            palette_field: 0.30,
            residual_density: 1.4,
            spring_flow: 5.0,
            vault_density: 1.6,
            vault_min_depth: 200,
            vault_bedrock_margin: 16,
            world_age: 0.8,
            moss_density: 0.10,
            tree_density: 0.26,
            grass_density: 0.50,
            life_cluster_wavelength: 70.0,
        }
    }
}

/// Every preset in `assets/worldgen.ron`, plus which one a fresh world uses.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WorldgenPresets {
    /// Preset a fresh world starts on. May be [`LEGACY`].
    pub default: String,
    /// Named presets. A `BTreeMap` rather than a `HashMap` so iteration order
    /// is fixed — preset cycling has to land on the same next preset every
    /// run, and `HashMap` order is neither stable nor reproducible.
    pub presets: BTreeMap<String, WorldgenParams>,
}

impl Default for WorldgenPresets {
    fn default() -> Self {
        let mut presets = BTreeMap::new();
        presets.insert("rolling".to_string(), WorldgenParams::default());
        Self { default: "rolling".to_string(), presets }
    }
}

impl WorldgenPresets {
    /// Where the presets live, alongside the other tunable `.ron` files.
    pub const ASSET_PATH: &'static str = "assets/worldgen.ron";

    /// Load from [`Self::ASSET_PATH`], falling back to the compiled-in
    /// defaults when the file is absent or unparseable.
    ///
    /// Absent is a normal state (a fresh checkout, a binary run from another
    /// directory), not a reason to fail startup — the same call the
    /// explosion tuning makes for the same reason. Returns the parse error
    /// alongside the defaults so a typo shows up on screen instead of
    /// silently reverting the world to stock values, which is the failure
    /// mode that wastes a tuning session.
    pub fn load() -> (Self, Option<String>) {
        let text = match std::fs::read_to_string(Self::ASSET_PATH) {
            Ok(t) => t,
            Err(_) => return (Self::default(), None),
        };
        match ron::from_str::<Self>(&text) {
            Ok(p) if p.presets.is_empty() => {
                (Self::default(), Some(format!("{}: no presets", Self::ASSET_PATH)))
            }
            Ok(p) => (p, None),
            Err(e) => (Self::default(), Some(format!("{}: {e}", Self::ASSET_PATH))),
        }
    }

    /// Preset names in cycling order: every generated preset, alphabetically,
    /// then [`LEGACY`] last so the hand-authored control sits at the end of
    /// the ring rather than in the middle of the generated ones.
    pub fn cycle_order(&self) -> Vec<String> {
        let mut names: Vec<String> = self.presets.keys().cloned().collect();
        names.push(LEGACY.to_string());
        names
    }

    /// The named preset, or `None` for [`LEGACY`] and for names that are not
    /// in the file (a preset deleted while the app was running).
    pub fn get(&self, name: &str) -> Option<&WorldgenParams> {
        self.presets.get(name)
    }

    /// The preset a fresh world should start on, falling back to the first
    /// generated preset and then to [`LEGACY`] — so a `default:` naming a
    /// preset that no longer exists degrades to something that works instead
    /// of panicking.
    pub fn default_name(&self) -> String {
        if self.default == LEGACY || self.presets.contains_key(&self.default) {
            return self.default.clone();
        }
        self.presets.keys().next().cloned().unwrap_or_else(|| LEGACY.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `scaled(1.0)` is the identity, so the unscaled path can route through
    /// it without a special case -- and so that a field classified into the
    /// wrong bucket still cannot change today's worlds.
    #[test]
    fn scaling_by_one_changes_nothing() {
        let (presets, err) = WorldgenPresets::load();
        assert!(err.is_none(), "{err:?}");
        for name in presets.cycle_order() {
            let Some(p) = presets.get(&name) else { continue };
            assert_eq!(*p, p.scaled(1.0), "preset {name} moved under scaled(1.0)");
        }
    }

    /// **A world built at `k` times the cell resolution is the same world,
    /// `k` times as large.** This is the property the whole resolution step
    /// rests on: if it does not hold, "double the resolution" cannot mean
    /// "the same place, finer" and there is nothing to scale *to*.
    ///
    /// **The unrelated-seed control is inside the assertion, not beside it**,
    /// and that is the entire design of this test. The residual is a mean
    /// absolute difference in rows, and a small one means nothing on its own
    /// -- a preset whose relief happened to be gentle would pass with the
    /// rescale completely broken. Measured against a different seed of the
    /// same preset, "broken" has a value: **when this was first run the
    /// rescaled residual was 39.1 rows on `rolling` against a 42.5-row
    /// control**, i.e. the rescaled world was no more like the original than
    /// a stranger, which is exactly what the ratio below catches and what a
    /// bare threshold would not have.
    ///
    /// The cause was `region::COMPOSITION_WINDOW`, a fixed 512 cells; with it
    /// scaled by `cell_scale` the same five presets read 0.20 to 1.94 rows
    /// against controls of 17 to 62 -- ratios of 1% to 11%. The bar is 25%,
    /// which is a little over twice the worst measured value, per
    /// `CLAUDE.md`'s "set bars from measurement with headroom".
    ///
    /// The residual does not go to zero and should not be expected to:
    /// column `x` of the small world maps to column `round(kx)` of the big
    /// one, and `column::strata_offset` folds its bands on a hardcoded
    /// 130-cell wavelength that `scaled` cannot reach.
    #[test]
    fn a_rescaled_world_is_the_same_world_at_a_finer_grain() {
        use crate::worldgen::column::Terrain;
        const K: f32 = 2.0;
        const W: i32 = 512;
        const H: i32 = 320;
        let (soil, sand) = (33.0_f32.to_radians().tan(), 34.0_f32.to_radians().tan());
        let (presets, err) = WorldgenPresets::load();
        assert!(err.is_none(), "{err:?}");

        for name in ["rolling", "terraced", "canyon", "wetland", "arid"] {
            let base = presets.get(name).expect("shipped preset").clone();
            let big = base.scaled(K);
            let small = Terrain::new(1, &base, W, H, soil, sand);
            let large = Terrain::new(1, &big, (W as f32 * K) as i32, (H as f32 * K) as i32, soil, sand);
            // The control: the same preset and size, a different seed.
            let stranger = Terrain::new(2, &base, W, H, soil, sand);

            let (mut rescaled, mut control) = (0.0f64, 0.0f64);
            for x in 0..W {
                let here = small.elev(x);
                rescaled += (here - large.elev((x as f32 * K) as i32) / K).abs() as f64;
                control += (here - stranger.elev(x)).abs() as f64;
            }
            let (rescaled, control) = (rescaled / W as f64, control / W as f64);

            // The control has to be big, or the ratio below is meaningless --
            // two identical worlds would pass it trivially.
            assert!(
                control > 5.0,
                "{name}: the different-seed control is only {control:.2} rows, so this preset \
                 cannot tell a rescaled world from an unrelated one either way"
            );
            assert!(
                rescaled < control * 0.25,
                "{name}: rescaling to {K}x left the terrain {rescaled:.2} rows from the original, \
                 against {control:.2} for an unrelated seed -- the rescale is not preserving the world"
            );
        }
    }

    #[test]
    fn shipped_asset_file_parses() {
        // Guards the file the app actually reads: a syntax error here is
        // otherwise a silent fall back to compiled-in defaults, and the
        // world would look right enough that nobody would notice.
        let text = std::fs::read_to_string(WorldgenPresets::ASSET_PATH)
            .expect("assets/worldgen.ron is committed");
        let parsed: WorldgenPresets = ron::from_str(&text).expect("worldgen.ron parses");
        for name in ["rolling", "terraced", "canyon", "wetland", "arid"] {
            assert!(parsed.presets.contains_key(name), "missing preset {name}");
        }
    }

    #[test]
    fn arid_preset_is_a_dry_world() {
        // The stated pivot lever: if the water table turns out not to be fun,
        // one preset switch removes all of it.
        // The `table_offset > world height` half of the lever lives in
        // `tests/worldgen.rs`'s `the_dry_presets_keep_their_table_below_the_
        // world_floor`, not here: it has to be asserted against
        // `app::WORLD_HEIGHT`, and `worldgen` sits *below* `app` in this
        // crate's layering. It used to be a literal `> 320.0` in this
        // function, which is exactly how it came to be a bar that no longer
        // meant anything.
        let (presets, _) = WorldgenPresets::load();
        let arid = presets.get("arid").expect("arid preset exists");
        assert_eq!(arid.moss_density, 0.0);
        assert_eq!(arid.tree_density, 0.0);
        // Grass too, and explicitly rather than by inheritance: `serde(default)`
        // means an omitted field takes `WorldgenParams::default()`'s nonzero
        // value, so a preset that means "nothing grows here" has to say so on
        // every layer or the next layer added turns the desert green.
        assert_eq!(arid.grass_density, 0.0);
    }

    #[test]
    fn legacy_sorts_last_in_the_cycle() {
        let presets = WorldgenPresets::default();
        let order = presets.cycle_order();
        assert_eq!(order.last().map(String::as_str), Some(LEGACY));
    }

    #[test]
    fn missing_default_degrades_instead_of_panicking() {
        let presets = WorldgenPresets { default: "deleted-preset".to_string(), ..Default::default() };
        assert_eq!(presets.default_name(), "rolling");
    }

    #[test]
    fn partial_preset_inherits_the_rest() {
        // The inheritance `#[serde(default)]` buys: a preset lists only what
        // it changes, and adding a field later cannot break existing presets.
        let p: WorldgenParams = ron::from_str("(soil_depth: 3.0)").expect("partial preset parses");
        assert_eq!(p.soil_depth, 3.0);
        assert_eq!(p.relief_amplitude, WorldgenParams::default().relief_amplitude);
    }
}
