//! The **decide** half of worldgen: what a single column of the world
//! contains, as a pure function of `(seed, params, x)`.
//!
//! Nothing here reads the world, allocates, or depends on what any other
//! column decided. That is not tidiness — it is the property that lets this
//! milestone's whole-world generator become the per-chunk
//! `worldgen(seed, coord, age)` of `Reports/worldgen-design.md` §4 without
//! any of these numbers changing. A chunk generated on its own has to agree
//! exactly with the same chunk generated as part of a whole world, and the
//! only cheap way to guarantee that is for the answer never to have depended
//! on the traversal in the first place.
//!
//! Everything is written in **y-down** terms, as the rest of the engine is:
//! `y = 0` is sky, `y = h - 1` is the world floor, and *higher ground means a
//! smaller `surface_y`*. Elevation (`elev`) is the one quantity that runs the
//! other way — it is +up, because reasoning about terrain shape upside down
//! is how sign errors get written — and it is converted exactly once, in
//! [`Terrain::plan`].

use super::erosion;
use super::noise::{self, Purpose};
use super::params::WorldgenParams;
use super::region::{Character, RegionMap};

/// What one column of the world contains, in cell coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnPlan {
    /// Topmost cell of solid ground — soil if there is any, stone if not.
    pub surface_y: i32,
    /// Thickness of the soil blanket. Zero on slopes too steep to hold it.
    pub soil_depth: i32,
    /// The water table. May be at or past `h`, which means this column has no
    /// water at all — the intended state for the `arid` preset.
    pub table_y: i32,
    /// Topmost bedrock cell.
    pub bedrock_top_y: i32,
}

/// One world's generation shape, bound to a seed and a size.
///
/// `soil_tan` is passed in rather than hardcoded because the angle it comes
/// from lives in `assets/materials/soil.ron`, and a generator that assumed a
/// value the material no longer has would place soil that immediately
/// avalanches — the one failure this milestone is most concerned to avoid.
pub struct Terrain<'a> {
    pub seed: u64,
    pub params: &'a WorldgenParams,
    pub w: i32,
    pub h: i32,
    /// Tangent of soil's angle of repose, from the material registry.
    pub soil_tan: f32,
    /// Tangent of sand's, for the columns that carry sand instead.
    pub sand_tan: f32,
    /// The places this world is made of. See `region.rs`.
    pub regions: RegionMap,
}

impl<'a> Terrain<'a> {
    pub fn new(seed: u64, params: &'a WorldgenParams, w: i32, h: i32, soil_tan: f32, sand_tan: f32) -> Self {
        let regions = RegionMap::new(seed, params, w);
        Self { seed, params, w, h, soil_tan, sand_tan, regions }
    }

    /// What kind of place this column is in.
    pub fn character(&self, x: i32) -> Character {
        self.regions.sample(x)
    }

    /// Whether this column is dry enough to carry sand rather than soil.
    ///
    /// A threshold rather than a blend because the *material* is discrete:
    /// a column holds sand or it holds soil, and half-way is not a thing the
    /// grid can express. The blend that matters happens either side of it —
    /// aridity itself varies smoothly, so the sand/soil boundary lands
    /// wherever the terrain happens to cross the line and comes out ragged
    /// rather than ruled.
    pub fn is_sandy(&self, x: i32) -> bool {
        self.character(x).aridity > SAND_ARIDITY
    }

    /// Repose tangent for whatever loose cover this column carries.
    pub fn cover_tan(&self, x: i32) -> f32 {
        if self.is_sandy(x) {
            self.sand_tan
        } else {
            self.soil_tan
        }
    }
}

/// Aridity past which loose cover is sand rather than soil.
const SAND_ARIDITY: f32 = 0.62;

/// See [`Terrain::hardness_field`] — the strata-band hardness sampler with
/// its per-column invariants (strata offset, regional resistance) baked.
pub(crate) struct HardnessField {
    seed: u64,
    datum: f32,
    band_thickness: f32,
    offset: Vec<f32>,
    regional: Vec<f32>,
}

impl HardnessField {
    /// Hardness of the band outcropping at elevation `e` in column `x`,
    /// `0..=1`. Columns outside the world clamp to the nearest edge
    /// column's invariants — erosion only asks about in-world columns
    /// today, but a clamp beats a panic the day a margin read appears.
    /// The floor exists because a zero-hardness band erodes without limit
    /// in one pass, and the differential rates that make ledges stop
    /// being differential.
    pub(crate) fn at(&self, x: i32, e: f32) -> f32 {
        const FLOOR: f32 = 0.15;
        let i = (x.max(0) as usize).min(self.offset.len() - 1);
        let band = (((self.datum - e) + self.offset[i]) / self.band_thickness).floor() as i32;
        let raw = noise::unit(self.seed, Purpose::Hardness, band, 0);
        ((FLOOR + (1.0 - FLOOR) * raw) * self.regional[i]).clamp(0.0, 1.0)
    }
}

/// Half-width of the central difference the terrace attenuation measures its
/// slope over, in columns. Escarpment reach, not neighbour reach — see
/// [`Terrain::terrace_yield`].
const SLOPE_REACH: i32 = 8;

/// Wavelength of the riser-roughening noise, in columns.
///
/// **Deliberately near the grid**, which is the opposite of every other
/// wavelength here and is the whole point. A riser is a *single-column* jump
/// in a heightfield, and a smooth term cannot split one however large its
/// amplitude: at wavelength 14 the roughening moved a whole bench up or down
/// by six rows and left canyon seed 7's worst riser at 34 cells, exactly
/// where it started. Only a term that differs sharply between x and x + 1
/// can turn one 34-cell jump into a short flight of smaller ones, which is
/// what a broken face looks like when the surface is a function of x.
const RISER_WAVELENGTH: f32 = 2.5;

impl Terrain<'_> {
    /// Where the elevation curve is sampled, after domain warping.
    ///
    /// Warping is what stops the surface reading as noise-on-a-line: it
    /// displaces the sample position along x, so a hill's two sides get
    /// sampled at different rates and come out asymmetric — one steep face,
    /// one shallow — the way real relief is.
    fn warped_x(&self, x: i32) -> f32 {
        let p = self.params;
        x as f32 + p.warp_strength * noise::fbm_1d_c(self.seed, Purpose::Warp, x as f32 / p.warp_wavelength, 2)
    }

    /// The macro silhouette: where the regions of this world sit.
    ///
    /// **This used to be one sine with a fixed phase**, and that was the
    /// single biggest reason every world looked the same. It was fixed
    /// deliberately, to guarantee a ridge and a valley in frame at every
    /// seed; it did, and it put them in the *same place* every time, so the
    /// first thing anyone sees never varied. Amplitude knobs could not fix
    /// that, because the shape was not a parameter.
    ///
    /// Now the shape comes from the regional layout (`region.rs`): each
    /// region draws its own elevation, and the blend between them is the
    /// skyline. The composition guarantee moved with it, and is stronger for
    /// being about relief rather than about a silhouette — `RegionMap::new`
    /// enforces a minimum spread between the highest and lowest region, so a
    /// flat draw is stretched rather than shipped.
    fn base_wave(&self, x: i32) -> f32 {
        self.params.relief_amplitude * self.character(x).elev
    }

    /// Dune shaping for arid country: asymmetric waves, steep on the lee.
    ///
    /// Dry country needs its own *shape*, not just its own material. Sand
    /// dunes are the one landform whose asymmetry is obvious at a glance —
    /// a long windward ramp and a short steep slip face — and reproducing it
    /// is what stops a desert reading as "the same hills, yellow".
    ///
    /// The steep side is deliberately kept just under sand's angle of repose:
    /// a slip face at the real angle is right in principle and avalanches on
    /// the first frame, which is the whole reason `soil_slope_cutoff` exists.
    fn dunes(&self, x: i32, aridity: f32) -> f32 {
        let p = self.params;
        if p.dune_amplitude <= 0.0 || aridity <= SAND_ARIDITY {
            return 0.0;
        }
        // Fades in past the sand threshold rather than switching on, or the
        // edge of a desert is a wall of dunes.
        let strength = ((aridity - SAND_ARIDITY) / (1.0 - SAND_ARIDITY)).clamp(0.0, 1.0);
        let wavelength = p.dune_wavelength.max(8.0);
        // The slip face may not be steeper than the sand can stand on.
        //
        // Not a nicety: the first version drew dunes at a real dune's angle,
        // which is sand's angle of repose, and the cover pass then refused to
        // put sand on them -- so the *shape* of a dune was cut into bare
        // stone and the desert came out as grey spikes with sand clinging in
        // patches. A landform made of a material has to be a shape that
        // material can hold. The fall occupies `FALL` of the period, so this
        // is the amplitude at which that fall sits exactly at the cover
        // cutoff, and the amplitude is clamped to it whatever the preset asks.
        const FALL: f32 = 0.45;
        let max_slope = p.soil_slope_cutoff * self.sand_tan;
        // The preset's own clamped amplitude. Still needed once dunes vary,
        // because it is the *datum*: see the trough note below.
        let base_amplitude = p.dune_amplitude.min(max_slope * FALL * wavelength);
        let phase = x as f32 / wavelength + noise::fbm_1d_c(self.seed, Purpose::Dune, x as f32 / 180.0, 2) * 0.6;
        // The dune this column belongs to. A pure function of x, which is
        // what lets each dune be given its own shape without any running
        // total: varying the *period* directly would mean summing dune
        // widths from the world origin, and the whole module is built on
        // never needing to know what any other column decided.
        let index = phase.floor() as i32;
        let saw = phase - phase.floor();
        let v = p.dune_variation.clamp(0.0, 1.0);
        // Each dune's own slip-face fraction, and its own height.
        //
        // These two between them are what breaks the comb: moving the crest
        // within its own cell (by varying how much of that cell the fall
        // occupies) and varying how tall it is turns a repeated tooth into a
        // dune field.
        let fall = (FALL * (1.0 + (noise::unit(self.seed, Purpose::DuneShape, index, 1) - 0.5) * 0.7 * v))
            .clamp(0.22, 0.62);
        // **Varied *downward* from the repose cap, not outward from the
        // preset's number** — and this is the whole difference between the
        // knob working and the knob doing nothing.
        //
        // The obvious form is `dune_amplitude * (1 ± v)`, which was written
        // first and measured as inert: `arid` asks for 18 against a cap of
        // `max_slope * FALL * wavelength` = 13.2, so three dunes in four
        // were *already* pinned at the cap and drawing a taller one changed
        // nothing at all. The clamp was not limiting the variation, it was
        // absorbing it. Crest-height spread moved 0.273 -> 0.281 across the
        // entire knob range, which reads exactly like a dead lever and was
        // not one.
        //
        // A dune cannot be taller than repose allows, so the only direction
        // variety can exist in is shorter — which is also what a real dune
        // field looks like, since not every dune in one is fully developed.
        let cap = max_slope * FALL * wavelength;
        let base = p.dune_amplitude.min(cap);
        // Re-clamped against this dune's *own* fall, not the preset's. A
        // shorter fall is a steeper face at the same height, and the failure
        // that produces is not subtle: the cover pass refuses the slope and
        // the desert comes out as bare grey spikes with sand in patches.
        let amplitude = (base * (1.0 - noise::unit(self.seed, Purpose::DuneShape, index, 0) * 0.55 * v))
            .max(0.0)
            .min(max_slope * fall * wavelength);
        // A long windward ramp and a shorter lee fall, both eased so the
        // crest is a crest and not a corner.
        let profile = if saw < 1.0 - fall {
            let t = saw / (1.0 - fall);
            t * t * (3.0 - 2.0 * t)
        } else {
            let t = (saw - (1.0 - fall)) / fall;
            1.0 - t * t * (3.0 - 2.0 * t)
        };
        // Measured from the **trough**, not from the dune's own midpoint.
        //
        // `(profile - 0.5) * amplitude` was right while every dune shared one
        // amplitude and is a cliff once they do not: `profile` is 0 at both
        // ends of a dune's cell, so that form puts the trough at
        // `-0.5 * amplitude` and two neighbouring dunes of different height
        // meet at a step of half their difference. Anchoring every trough to
        // the same datum makes the field continuous, and reduces to exactly
        // the old expression when `amplitude == base_amplitude`.
        (profile * amplitude - 0.5 * base_amplitude) * strength
    }

    /// The elevation the terrace snap is applied *to*: the macro silhouette
    /// plus the hill octaves, with no detail, no dunes and no snap.
    ///
    /// Factored out of [`Terrain::elev`] so that `terraced` can measure the
    /// slope of the ground it is about to step without recursing — `slope()`
    /// differences `elev`, and `elev` calls `terraced`, so a terrace rule
    /// that asked `slope()` how steep it is would call itself forever. This
    /// is also the *right* quantity rather than merely the available one:
    /// what has to be detected is a regional escarpment, and detail and
    /// dune terms are exactly the short-wavelength content that would make
    /// a gentle bench read as steep.
    fn pre_terrace_elev(&self, x: i32) -> f32 {
        let p = self.params;
        let ch = self.character(x);
        let wx = self.warped_x(x);
        // Hills scale with the region's own ruggedness, so smooth country and
        // broken country can sit in the same world.
        let hills =
            p.hill_amplitude * ch.relief * noise::fbm_1d_c(self.seed, Purpose::Height, wx / p.hill_wavelength, 4);
        self.base_wave(x) + hills
    }

    /// Surface elevation in cells, **+up**, before conversion to a row.
    pub fn elev(&self, x: i32) -> f32 {
        let p = self.params;
        let ch = self.character(x);
        let detail =
            p.detail_amplitude * noise::fbm_1d_c(self.seed, Purpose::Detail, x as f32 / p.detail_wavelength, 2);
        // Detail is added *after* terracing, not before. Snapping a surface
        // that already contains fine roughness throws that roughness away
        // wherever the snap bites, which is exactly where it is most needed:
        // benches come out glassy flat and their edges perfectly straight,
        // and the whole feature reads as cut from a solid rather than
        // weathered out of one. Applied afterwards, the same noise roughens
        // the treads and ragged the lips.
        self.terraced(x, self.pre_terrace_elev(x)) + detail + self.dunes(x, ch.aridity)
    }

    /// How far the sedimentary banding is displaced at this column: a steady
    /// tilt plus a slow fold.
    ///
    /// Shared with the shade pass so that the bands drawn in the rock and the
    /// benches cut into the surface are the *same* bands. Having them
    /// disagree is what makes layered terrain look painted rather than
    /// carved.
    pub fn strata_offset(&self, x: i32) -> f32 {
        let p = self.params;
        p.strata_tilt * x as f32 + p.strata_fold * noise::fbm_1d_c(self.seed, Purpose::Strata, x as f32 / 130.0, 2)
    }

    /// How resistant the strata bands of this world are, as a sampler:
    /// the erosion pass's material axis (`erosion.rs`,
    /// `Reports/worldgen-erosion-design.md`), with its per-column
    /// invariants precomputed.
    ///
    /// One draw per **band index**, in the same banded coordinate
    /// `strata_offset`/`terraced` share, so a band is hard or soft along
    /// its whole outcrop and eroded ledges sit on the banding the shade
    /// pass draws — the same one-field-two-jobs argument `terraced` makes.
    /// Scaled by the region's `resistance` so broken, benched country
    /// erodes as broken country. Lives here rather than in `erosion.rs`
    /// because the band coordinate does — the coupling to the drawn strata
    /// is the point.
    ///
    /// `strata_offset` is an fBm and `character` a region blend, and
    /// neither depends on elevation — but the erosion loop resamples
    /// hardness for every column on every iteration because the *band* a
    /// surface sits in changes as it erodes. Building this once turns the
    /// per-iteration cost into one hash per column (the per-band draw),
    /// which is the difference between the erosion budget and a second
    /// build.
    pub(crate) fn hardness_field(&self) -> HardnessField {
        HardnessField {
            seed: self.seed,
            datum: self.datum(),
            band_thickness: self.params.strata_thickness.max(1.0),
            offset: (0..self.w).map(|x| self.strata_offset(x)).collect(),
            regional: (0..self.w)
                .map(|x| (0.5 + 0.35 * self.character(x).resistance).clamp(0.0, 1.0))
                .collect(),
        }
    }

    /// How much of the terrace snap survives the ground it is standing on:
    /// 1 on gentle country, 0 on an escarpment.
    ///
    /// A riser is a single-column jump of `terrace_step * m` rows *regardless
    /// of the slope beneath it*, so on steep ground the snap adds its own
    /// face to a face the relief already supplies. Round 1's finding 1b
    /// measured the result: canyon seed 7 carries risers of 27, 34 and 33
    /// rows at x 610, 616 and 622, six-column treads between 30-cell faces.
    /// Its note named the lever — `terrace_step` against the local
    /// escarpment slope, not more roughening, because roughening makes each
    /// riser shorter and *more numerous* rather than rarer.
    ///
    /// Read off the **pre-terrace** elevation, which matters twice over.
    /// `slope()` differences `elev()`, and `elev()` calls this, so asking
    /// `slope()` would recurse; and detail and dune terms are short-
    /// wavelength by construction, so including them would make a gently
    /// benched surface read as steep and switch terracing off across worlds
    /// that should keep it.
    fn terrace_yield(&self, x: i32) -> f32 {
        let p = self.params;
        if p.terrace_slope_hi <= p.terrace_slope_lo {
            return 1.0;
        }
        // A central difference at escarpment reach rather than at the
        // neighbouring column: the quantity wanted is "is this a regional
        // face", and a one-column difference on a hill octave answers a
        // different question. Eight columns is the same order as the treads
        // being protected.
        let slope =
            (self.pre_terrace_elev(x + SLOPE_REACH) - self.pre_terrace_elev(x - SLOPE_REACH)).abs()
                / (2 * SLOPE_REACH) as f32;
        1.0 - noise::smoothstep(p.terrace_slope_lo, p.terrace_slope_hi, slope)
    }

    /// Snap the surface onto strata boundaries, where the mask allows.
    ///
    /// Terracing against a *global* height grid was the first version and it
    /// looked wrong immediately: every bench the same width, every riser the
    /// same height, all of them level — a staircase, which reads as the
    /// surface having been rounded off, because it was. Real benched relief
    /// is irregular because it is the outcrop of resistant layers, and its
    /// benches tilt and wander with those layers.
    ///
    /// So the snap happens in the *strata's* coordinate, not the world's. A
    /// bench forms exactly where a band surfaces, which means bench spacing
    /// varies with the tilt and the fold, benches on opposite sides of a hill
    /// sit at different heights, and the terrace edges line up with the
    /// banding visible in every cut face below them. One noise field doing
    /// two jobs that have to agree.
    ///
    /// Still gated on a low-frequency mask, so bluffs appear in patches with
    /// smooth ground between them rather than over every slope in the world.
    fn terraced(&self, x: i32, e: f32) -> f32 {
        let p = self.params;
        if p.terrace_step <= 0.0 || p.terrace_strength <= 0.0 {
            return e;
        }
        let mask = noise::fbm_1d(self.seed, Purpose::Mask, x as f32 / p.mask_wavelength, 2);
        // A high gate on purpose. Two-octave fBm sits around 0.5 most of the
        // time, so a threshold anywhere near that terraces most of the world
        // and the result reads as a flight of stairs rather than as a bluff.
        // Bluffs want to be occasional.
        let m = (p.terrace_strength * self.character(x).resistance).clamp(0.0, 1.0)
            * noise::smoothstep(0.62, 0.82, mask)
            * self.terrace_yield(x);
        if m <= 0.0 {
            return e;
        }
        // Into band space (y-down, offset by the strata displacement), snap,
        // and back out.
        let offset = self.strata_offset(x);
        let band_coord = (self.datum() - e) + offset;
        let bands = band_coord / p.terrace_step;
        let snapped = bands.round() * p.terrace_step;
        let stepped = self.datum() + offset - snapped;
        let terraced = e + (stepped - e) * m;
        if p.riser_roughness <= 0.0 {
            return terraced;
        }
        // Roughen the riser, and only the riser.
        //
        // A riser is where `bands.round()` changes, so it is where the
        // residual `bands - bands.round()` crosses a half-band -- and the
        // *distance* from that crossing is exactly "how far into the bench
        // are we". Reading the residual rather than differencing neighbouring
        // columns is what keeps this a pure function of x with no lookahead.
        //
        // Written after four terms that all failed to touch it: the surface
        // detail term is 2.5-3.0 cells against a riser of up to 34, so every
        // bluff in the world had a dead-plumb one-column face. This is
        // deliberately scaled by `terrace_step` -- the thing that has to be
        // broken up is as tall as the snap made it.
        let residual = (bands - bands.round()).abs();
        // The window is wide on purpose, and the reason is what makes this
        // gate work at all without a second elevation evaluation: on a steep
        // escarpment `bands` sweeps its whole range every few columns, so a
        // high residual is common there, while on a gentle bench it changes
        // slowly and sits low for long stretches. The residual therefore
        // separates "riser" from "bench" by itself. A tight window
        // (0.30..0.50) was the first try and covered barely a column either
        // side of each jump -- four cells of ragging against a 34-cell face,
        // which is invisible, and looked exactly like the term not being
        // applied at all.
        let nearness = noise::smoothstep(0.22, 0.46, residual);
        if nearness <= 0.0 {
            return terraced;
        }
        // Short wavelength on purpose: this is column-scale ragging of a
        // face, not another landform. Its own noise stream, so a rough riser
        // does not sit on a detail bump.
        let rough = noise::fbm_1d_c(self.seed, Purpose::Riser, x as f32 / RISER_WAVELENGTH, 1);
        terraced + rough * p.riser_roughness * p.terrace_step * m * nearness
    }

    /// The low-frequency part of the elevation curve: the base wave plus one
    /// hill octave, with no detail and no terracing.
    ///
    /// This is the water table's shape. A table is a *subdued replica* of
    /// surface topography — it follows the big landforms, not the small ones,
    /// because groundwater smooths out everything shorter than its own
    /// drainage length. Taking a partial sum of the terms the surface is
    /// already built from gets that exactly, with no smoothing pass and no
    /// second noise field to keep in step
    /// (`Reports/prior-art-worldgen-slicing.md` §7).
    fn low_elev(&self, x: i32) -> f32 {
        let p = self.params;
        let wx = self.warped_x(x);
        self.base_wave(x) + p.hill_amplitude * noise::fbm_1d_c(self.seed, Purpose::Height, wx / p.hill_wavelength, 1)
    }

    /// Where elevation zero sits, in rows. Chosen so the highest possible
    /// ridge still leaves `sky_rows` of clear sky above it.
    ///
    /// `pub(crate)` rather than private: `passes.rs`'s valley-floor check
    /// needs to invert `surface_y` back to elevation (`elev = datum -
    /// surface_y`) on the *eroded* surface, which only `ColumnPlan` carries
    /// -- round-4 task 4.
    pub(crate) fn datum(&self) -> f32 {
        self.params.sky_rows + self.params.relief_amplitude
    }

    /// Local surface steepness, in cells of rise per cell of run.
    ///
    /// A central difference over the *unclamped* curve, so a column whose
    /// `surface_y` was clamped at the world edge still reports the slope the
    /// terrain actually has there rather than a flat one.
    pub fn slope(&self, x: i32) -> f32 {
        ((self.elev(x + 1) - self.elev(x - 1)) / 2.0).abs()
    }

    /// The steepest slope seen looking up to three columns either side.
    ///
    /// What the soil blanket is thinned by, rather than the immediate slope.
    /// A terrace tread is perfectly level, so the immediate slope says
    /// "flat, pile it on" right up to the lip — and a full-depth blanket
    /// sitting on a narrow bench renders as a brown box with a vertical face,
    /// which is what a soil slab is and not what a hillside looks like.
    /// Ground about to fall away should already be losing its cover, which is
    /// also how it works: soil creeps off a lip long before the lip itself
    /// goes.
    pub fn slope_near(&self, x: i32) -> f32 {
        (1..=3)
            .map(|k| ((self.elev(x + k) - self.elev(x - k)) / (2 * k) as f32).abs())
            .fold(0.0f32, f32::max)
    }

    /// Decide one column, pre-erosion.
    ///
    /// The un-aged decision: elevation and slopes straight off the noise
    /// curve. `plan_all` routes through [`Terrain::plan_from`] with the
    /// *eroded* surface instead; at `world_age == 0` the two are
    /// bit-identical (asserted in `plan_all_at_age_zero_matches_plan`),
    /// which is what keeps every pre-erosion test and baseline meaningful.
    pub fn plan(&self, x: i32) -> ColumnPlan {
        self.plan_from(x, self.elev(x), self.slope(x), self.slope_near(x), 0.0)
    }

    /// Decide one column from an already-chosen surface elevation and its
    /// measured slopes, plus `extra_cover` cells of erosion deposit
    /// (talus + sediment) to realise as loose cover.
    ///
    /// Deposits deepen the blanket rather than adding height because the
    /// soil pass converts the top `soil_depth` rows *of the massif* —
    /// erosion already banked the deposit's volume in the surface it
    /// handed over, so counting it again here would mint elevation. And
    /// they pass through the same slope gate as soil: a deposit the sim
    /// dropped near a face that is steeper than repose realises as rock,
    /// not as a powder ledge waiting to avalanche — the at-rest guarantee
    /// is inherited, not re-proved.
    fn plan_from(&self, x: i32, e: f32, slope: f32, slope_near: f32, extra_cover: f32) -> ColumnPlan {
        let p = self.params;
        let surface_y = (self.datum() - e).round() as i32;
        // The clamp keeps the massif inside the world with room for bedrock
        // beneath it; it bites only on extreme parameter values.
        let surface_y = surface_y.clamp(4, self.h - 12);

        // Soil thins as the ground steepens and is gone entirely past the
        // cutoff. This is the at-rest guarantee, not an aesthetic choice: a
        // powder resting on a slope shallower than its angle of repose cannot
        // avalanche, so soil placed under this rule never moves. Bare rock on
        // the steep faces is the same rule read as a picture.
        let ch = self.character(x);
        // The repose of whatever this column actually carries. A sandy column
        // gated against soil's angle would place sand on ground steeper than
        // sand can hold, and the world would avalanche on frame one.
        let cutoff = (p.soil_slope_cutoff * self.cover_tan(x)).max(0.0);
        // The neighbourhood slope, not the immediate one: a bench is flat at
        // the exact column and the drop is two cells away. Still an upper
        // bound on the true local slope, so the at-rest guarantee this gate
        // provides only gets stricter, never looser.
        let steepness = slope.max(slope_near);
        let soil_depth = if cutoff <= 0.0 || steepness >= cutoff {
            0
        } else {
            // Quadratic rather than linear taper. A linear one thins soil as
            // soon as the ground tilts at all, so a world with any relief in
            // it wears a uniformly thin crust everywhere and reads as painted
            // on. Squaring keeps the blanket at close to full depth across
            // gentle ground and takes it away sharply near the limit, which
            // is both the better picture and the more accurate one — soil
            // creep is negligible until a slope approaches repose.
            let taper = 1.0 - (steepness / cutoff).powi(2);
            let jitter = 3.0 * noise::fbm_1d_c(self.seed, Purpose::Soil, x as f32 / 37.0, 2);
            // Regional sediment supply, and dry country holding less of it --
            // except where it is dry *enough* to be sand, which piles deeper
            // than soil does rather than thinner. A desert is not bare rock.
            let supply = if ch.aridity > SAND_ARIDITY {
                ch.sediment * 1.35
            } else {
                ch.sediment * (1.0 - ch.aridity * 0.55)
            };
            let depth = (p.soil_depth * supply * taper + jitter).max(0.0).round() as i32;
            // Where the blanket has already worn thin, let the rock through
            // entirely rather than leaving a one- or two-cell skin over it.
            //
            // A uniform crust of soil over every surface was the strongest
            // remaining tell that the ground was generated: real hillsides
            // are patchy, and the bare rock shows through exactly where the
            // cover is thinnest. It also costs nothing to be sure of — taking
            // powder away can never make a world less at rest than leaving it
            // there.
            let depth = if depth <= 3 && noise::unit(self.seed, Purpose::SoilNoise, x, 0) < 0.45 {
                0
            } else {
                depth
            };
            // Erosion deposits stack on (or substitute for) the native
            // blanket, *after* the patchiness rule: an apron heaped at the
            // foot of a cliff is real accumulated material and must not be
            // deleted by the bare-rock-shows-through draw, which models a
            // thin native crust wearing away — the opposite situation.
            depth + extra_cover.round() as i32
        };

        // Dry country's water table sits deeper, which is most of what makes
        // a desert a desert: no pools in the hollows, and nothing green.
        let table_drop = p.table_offset + ch.aridity * p.aridity_table_drop;
        let table_y = (self.datum() - p.table_damping * self.low_elev(x) + table_drop).round() as i32;

        // A perturbed band so the world floor is not a ruler line (there are
        // no straight horizontal edges in this world except water surfaces).
        let band = (p.bedrock_band * noise::fbm_1d(self.seed, Purpose::Bedrock, x as f32 / 23.0, 2)).round() as i32;
        let bedrock_top_y = self.h - 2 - band.max(0);

        ColumnPlan { surface_y, soil_depth, table_y, bedrock_top_y }
    }

    /// Every column of the world, in x order. Computed once and shared by
    /// every pass, since several passes need their neighbours' plans and
    /// recomputing the noise per lookup would be the generator's whole cost.
    ///
    /// This is also where the terrain gets its history: the raw elevation
    /// curve goes through plan-space erosion (`erosion.rs`,
    /// `params.world_age` iterations of thermal + hydraulic weathering)
    /// before any column is decided, and every slope the soil gate reads is
    /// measured on the *eroded* surface — soil placed against the curve the
    /// world no longer has would be the at-rest failure this file exists to
    /// prevent. At `world_age == 0` (every preset today) `erode` is a
    /// guaranteed no-op and this is bit-identical to mapping [`Terrain::plan`]
    /// over the columns, which the tests pin.
    ///
    /// Columns outside the world fall back to the un-eroded `elev(x)` for
    /// slope reads at the two edges — out-of-world ground was never eroded
    /// (the sim treats the edge as an outlet), and two columns of slightly
    /// stale slope beats pretending the world extends.
    ///
    /// Drops the [`erosion::Deposits`] this computes along the way — see
    /// [`Self::plan_all_with_deposits`], which is the real body. Kept as the
    /// name every pure/no-op test already calls, so the round-4 restructure
    /// that let the realise side see which cover is deposit and which is
    /// native blanket needed no edits to those tests at all.
    pub fn plan_all(&self) -> Vec<ColumnPlan> {
        self.plan_all_with_deposits().0
    }

    /// [`Self::plan_all`], plus the [`erosion::Deposits`] erosion left behind
    /// on the way to those plans.
    ///
    /// Split out so the realise side (`passes.rs`) can tell a deposit apart
    /// from the native soil blanket it landed in — before this, `plan_from`
    /// folded `talus + sediment` straight into `soil_depth` and nothing
    /// downstream could recover which cells were which, so the gravel-as-
    /// talus and boulder-socket passes had nowhere to read from. Pure
    /// plumbing: the plans returned are bit-identical to before at every
    /// `world_age`, which `plan_all_at_age_zero_matches_plan` and the
    /// erosion purity tests confirm without needing to change.
    pub fn plan_all_with_deposits(&self) -> (Vec<ColumnPlan>, erosion::Deposits) {
        let mut h: Vec<f32> = (0..self.w).map(|x| self.elev(x)).collect();
        let deposits = erosion::erode(self, &mut h);
        let at = |x: i32| -> f32 {
            if x < 0 || x >= self.w {
                self.elev(x)
            } else {
                h[x as usize]
            }
        };
        let mut plans: Vec<ColumnPlan> = (0..self.w)
            .map(|x| {
                let slope = ((at(x + 1) - at(x - 1)) / 2.0).abs();
                let slope_near = (1..=3)
                    .map(|k| ((at(x + k) - at(x - k)) / (2 * k) as f32).abs())
                    .fold(0.0f32, f32::max);
                let extra = deposits.talus[x as usize] + deposits.sediment[x as usize];
                self.plan_from(x, at(x), slope, slope_near, extra)
            })
            .collect();
        self.taper_cover(&mut plans);
        (plans, deposits)
    }

    /// Thin the loose cover toward wherever it runs out, so its own free face
    /// obeys repose.
    ///
    /// The per-column slope gate answers "is this *ground* too steep to hold
    /// cover". It cannot answer the other question: on the level top of an
    /// escarpment the ground is flat, the gate is happy, and the blanket's
    /// exposed side is a vertical wall of powder whose top grain rolls off
    /// the first time it is looked at. Two cells did exactly that, found by
    /// the at-rest sweep and invisible in any render.
    ///
    /// So cover may only deepen by one repose-step per column away from bare
    /// ground. Bare columns are already zero, so the limit propagates outward
    /// from every cliff lip and every steep face, and the blanket reaches an
    /// edge at nothing — which is what soil does at a scarp anyway.
    ///
    /// Two sweeps rather than a per-column rule because the constraint is
    /// inherently about a *run* of columns. Its reach is bounded — a 26-cell
    /// blanket at half a cell per column resolves within about fifty columns
    /// — so this stays chunk-local with a declared margin rather than being
    /// the sort of global pass that has to wait for the coarse map.
    fn taper_cover(&self, plans: &mut [ColumnPlan]) {
        let cutoff = self.params.soil_slope_cutoff;
        // Accumulated as a float and rounded once at the end. Rounding the
        // *step* was the first version, and a step of 0.5 rounds to 1 -- a
        // one-cell-per-column taper is a 45 degree face, which is steeper
        // than any powder here stands at. It cost exactly one rolling grain
        // in one arid seed, which is the size of defect this sweep exists to
        // catch.
        let mut depth: Vec<f32> = plans.iter().map(|c| c.soil_depth as f32).collect();
        for x in 1..depth.len() {
            let step = (cutoff * self.cover_tan(x as i32)).max(0.05);
            depth[x] = depth[x].min(depth[x - 1] + step);
        }
        for x in (0..depth.len().saturating_sub(1)).rev() {
            let step = (cutoff * self.cover_tan(x as i32)).max(0.05);
            depth[x] = depth[x].min(depth[x + 1] + step);
        }
        for (plan, d) in plans.iter_mut().zip(depth) {
            plan.soil_depth = d.max(0.0).floor() as i32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terrain(seed: u64, params: &WorldgenParams) -> Terrain<'_> {
        Terrain::new(seed, params, 512, 320, 33.0_f32.to_radians().tan(), 34.0_f32.to_radians().tan())
    }

    #[test]
    fn plan_is_pure() {
        let p = WorldgenParams::default();
        let t = terrain(3, &p);
        let a = t.plan(200);
        for x in 0..400 {
            let _ = t.plan(x);
        }
        assert_eq!(a, t.plan(200));
    }

    #[test]
    fn plan_all_at_age_zero_matches_plan() {
        // The restructure that routed `plan_all` through the eroded surface
        // must be invisible while `world_age` is 0 — every baseline, every
        // sweep, and the concurrent data-track branch depend on the
        // pre-erosion world staying bit-identical. Whole plans compared,
        // not just surfaces, so a slope-plumbing slip in the soil gate
        // cannot hide.
        //
        // `WorldgenParams::default()` stopped being age 0 in round-4 task 4
        // (`rolling` ships `world_age: 0.8`); the no-op guarantee this test
        // pins is about age 0 itself, not about the default, so it is
        // reached explicitly here rather than assumed from `default()`.
        let p = WorldgenParams { world_age: 0.0, ..Default::default() };
        for seed in 0..6u64 {
            let t = terrain(seed, &p);
            let mut expected: Vec<ColumnPlan> = (0..t.w).map(|x| t.plan(x)).collect();
            t.taper_cover(&mut expected);
            assert_eq!(t.plan_all(), expected, "seed {seed}: age 0 changed a plan");
        }
    }

    #[test]
    fn every_seed_has_a_ridge_and_a_valley() {
        // Aesthetic requirement A1, as a property rather than a picture: the
        // fixed-phase base wave is supposed to make a flat world impossible.
        // If someone seeds the phase, this fails.
        let p = WorldgenParams::default();
        for seed in 0..24u64 {
            let t = terrain(seed, &p);
            let plans = t.plan_all();
            let hi = plans.iter().map(|c| c.surface_y).min().unwrap();
            let lo = plans.iter().map(|c| c.surface_y).max().unwrap();
            assert!(
                lo - hi >= 60,
                "seed {seed}: only {} cells of relief across the world",
                lo - hi
            );
        }
    }

    #[test]
    fn terrain_stays_inside_the_world() {
        let (mut presets, _) = super::super::params::WorldgenPresets::load();
        // Also covers the shipped presets, which is where an extreme
        // amplitude would first show up.
        presets.presets.insert("stress".into(), WorldgenParams { relief_amplitude: 400.0, ..Default::default() });
        for (name, p) in &presets.presets {
            for seed in 0..6u64 {
                let t = terrain(seed, p);
                for x in 0..t.w {
                    let c = t.plan(x);
                    assert!(c.surface_y >= 0 && c.surface_y < t.h, "{name}: surface {} off world", c.surface_y);
                    assert!(c.bedrock_top_y > c.surface_y, "{name}: bedrock above the surface");
                    assert!(c.bedrock_top_y < t.h, "{name}: bedrock off world");
                }
            }
        }
    }

    #[test]
    fn steep_ground_carries_no_soil() {
        // The at-rest guarantee. Anywhere loose cover exists, the ground it
        // rests on must be shallower than *that cover's* angle of repose.
        //
        // **Against `cover_tan`, not `soil_tan`.** This read soil's angle for
        // every column, which is the wrong material's number on any column
        // dry enough to carry sand -- sand stands at 34 degrees against
        // soil's 33, so the bar was 2% too strict there. It never fired
        // because nothing had put a sandy column that close to its limit;
        // riser roughening did, at seed 0 x 77, slope 0.5305 against soil's
        // 0.5195 and sand's 0.5396. The generator was right and the test was
        // measuring the wrong angle, which is the failure mode CLAUDE.md
        // names as a constant that was compensating for a case nobody had
        // reached yet.
        //
        // The empirical half of this guarantee lives in
        // `tests/worldgen.rs::generated_terrain_is_already_at_rest`, which
        // steps every preset x 5 seeds for 120 frames and asserts that not
        // one cell moved. This one is the cheaper statement of *why*.
        let p = WorldgenParams::default();
        for seed in 0..8u64 {
            let t = terrain(seed, &p);
            for x in 0..t.w {
                if t.plan(x).soil_depth > 0 {
                    let cutoff = p.soil_slope_cutoff * t.cover_tan(x);
                    assert!(
                        t.slope(x) < cutoff,
                        "seed {seed} x {x}: {} on a {} slope, past its {cutoff} cutoff",
                        if t.is_sandy(x) { "sand" } else { "soil" },
                        t.slope(x)
                    );
                }
            }
        }
    }

    #[test]
    fn arid_table_is_below_the_world() {
        // The pivot lever, checked at the level that decides it: no column
        // may want water anywhere inside the world.
        let (presets, _) = super::super::params::WorldgenPresets::load();
        let arid = presets.get("arid").expect("arid preset");
        let t = terrain(11, arid);
        for x in 0..t.w {
            assert!(t.plan(x).table_y >= t.h, "arid preset put the table inside the world at x {x}");
        }
    }

    #[test]
    fn table_follows_relief_but_more_gently() {
        // What "subdued replica" means, as a measurement: the table moves
        // with the surface, and moves less than it does.
        let p = WorldgenParams::default();
        let t = terrain(5, &p);
        let plans = t.plan_all();
        let surface_range = plans.iter().map(|c| c.surface_y).max().unwrap()
            - plans.iter().map(|c| c.surface_y).min().unwrap();
        let table_range =
            plans.iter().map(|c| c.table_y).max().unwrap() - plans.iter().map(|c| c.table_y).min().unwrap();
        assert!(table_range > 10, "table is essentially flat: {table_range}");
        assert!(table_range < surface_range, "table {table_range} not subdued vs surface {surface_range}");
    }

    // -----------------------------------------------------------------------
    // Step-0 probe for the August 2026 world review (task 1b)
    // -----------------------------------------------------------------------

    /// A terrain at the size the app ships, on a named preset. The review's x
    /// coordinates are shipped-size coordinates, and the regional layout
    /// scales with width, so the 512-wide helper above samples a different
    /// world entirely at the same x.
    fn shipped(seed: u64, params: &WorldgenParams) -> Terrain<'_> {
        Terrain::new(seed, params, 2048, 640, 33.0_f32.to_radians().tan(), 34.0_f32.to_radians().tan())
    }

    #[test]
    #[ignore = "probe: prints, never asserts (task 5 riser tuning)"]
    fn probe_5_riser_step_sizes() {
        // The metric for "risers are dead-plumb 40-cell one-column faces":
        // the size of the largest single-column step, and how many steps
        // there are. Breaking a riser means the worst step falls *and* the
        // count rises -- one tall jump becoming a flight of shorter ones. A
        // worst that falls while the count also falls would mean the relief
        // had simply been flattened, which is a different and worse outcome.
        let (presets, err) = super::super::params::WorldgenPresets::load();
        assert!(err.is_none(), "{err:?}");
        println!("\n=== single-column steps >= 6 rows, by riser_roughness ===");
        println!("  {:>6} {:>10} {:>5} {:>7} {:>7} {:>9}", "rough", "preset", "seed", "steps", "worst", "mean_step");
        for r in [0.0f32, 0.2, 0.35, 0.5, 0.7] {
            for (preset, seed) in [("canyon", 7u64), ("canyon", 13), ("rolling", 1), ("terraced", 2)] {
                let base = presets.get(preset).expect("preset");
                let p = WorldgenParams { riser_roughness: r, ..base.clone() };
                let t = shipped(seed, &p);
                let surf = |x: i32| (t.datum() - t.elev(x)).round() as i32;
                let mut steps: Vec<i32> = Vec::new();
                for x in 1..2048 {
                    let d = (surf(x) - surf(x - 1)).abs();
                    if d >= 6 {
                        steps.push(d);
                    }
                }
                let n = steps.len().max(1) as f32;
                println!(
                    "  {r:>6.2} {preset:>10} {seed:>5} {:>7} {:>7} {:>9.1}",
                    steps.len(),
                    steps.iter().copied().max().unwrap_or(0),
                    steps.iter().sum::<i32>() as f32 / n
                );
            }
        }
    }

    #[test]
    #[ignore = "probe: prints, never asserts (task 5 dune tuning)"]
    fn probe_5_dune_comb_statistics() {
        // "A mechanical sawtooth comb" is a statement about *regularity*, so
        // the metric is the spread of crest heights and of crest spacings,
        // not a picture -- at strip zoom a comb and a dune field differ by
        // an amount the eye cannot put a number on, and this knob has to be
        // tuned to a number.
        //
        // Paired: the same seed and preset with `dune_variation` at 0 and at
        // the value under test, which is exactly what the A/B param is for.
        let (presets, err) = super::super::params::WorldgenPresets::load();
        assert!(err.is_none(), "{err:?}");
        let arid = presets.get("arid").expect("arid preset");
        println!("\n=== arid dune crests, 2048 wide ===");
        println!("  {:>6} {:>5} {:>7} {:>9} {:>9} {:>9} {:>9}", "var", "seed", "crests", "mean_h", "cv_h", "mean_gap", "cv_gap");
        for v in [0.0f32, 0.4, 0.7, 0.85, 1.0] {
            for seed in [1u64, 7] {
                let p = WorldgenParams { dune_variation: v, ..arid.clone() };
                let t = shipped(seed, &p);
                let elev: Vec<f32> = (0..2048).map(|x| t.elev(x)).collect();
                // A crest is a local maximum with a real drop either side, so
                // surface roughness is not counted as a dune.
                //
                // **Window and prominence sanity-checked against the case
                // that is known to be fine**: `arid`s wavelength is 58, so a
                // 2048-wide world must contain roughly 35 crests, and the
                // first version of this -- a 4-cell window at 3 cells of
                // prominence -- reported *zero* at every setting. It was
                // asking for a 3-cell drop within 4 columns on a dune whose
                // whole flank falls 13 cells over 26, which never happens.
                // Half a wavelength of reach, and a prominence set from the
                // amplitude rather than guessed.
                const REACH: usize = 14;
                let mut crests: Vec<(i32, f32)> = Vec::new();
                for x in REACH..2048 - REACH {
                    let here = elev[x];
                    let left = (1..=REACH).map(|d| elev[x - d]).fold(f32::MAX, f32::min);
                    let right = (1..=REACH).map(|d| elev[x + d]).fold(f32::MAX, f32::min);
                    let peak = (1..=REACH)
                        .flat_map(|d| [elev[x - d], elev[x + d]])
                        .fold(f32::MIN, f32::max);
                    if here >= peak && here - left.max(right) > 2.0 {
                        crests.push((x as i32, here));
                    }
                }
                // A flat-topped crest matches at several adjacent columns;
                // keep the first of each run so the count is dunes, not
                // cells.
                crests.dedup_by(|a, b| a.0 - b.0 <= 2);
                // Crest **height above its own troughs**, which is the
                // quantity `dune_variation` actually varies. The first
                // version used the drop within the detection window and it
                // measured almost nothing: half a wavelength here is 29
                // columns, so a 14-column window never reaches a trough and
                // what it reported was the underlying hill slope. It moved
                // by 0.01 across the whole knob range and would have been
                // read as "the lever is dead" -- which is the sweep-lies
                // failure in CLAUDE.md, in the metric rather than the knob.
                let heights: Vec<f32> = crests
                    .windows(3)
                    .map(|w| {
                        let trough = |a: i32, b: i32| {
                            (a..=b).map(|x| elev[x as usize]).fold(f32::MAX, f32::min)
                        };
                        w[1].1 - (trough(w[0].0, w[1].0) + trough(w[1].0, w[2].0)) * 0.5
                    })
                    .collect();
                let n = heights.len().max(1) as f32;
                let mean_h = heights.iter().sum::<f32>() / n;
                let var_h = heights.iter().map(|h| (h - mean_h).powi(2)).sum::<f32>() / n;
                let gaps: Vec<f32> =
                    crests.windows(2).map(|w| (w[1].0 - w[0].0) as f32).collect();
                let gn = gaps.len().max(1) as f32;
                let mean_g = gaps.iter().sum::<f32>() / gn;
                let var_g = gaps.iter().map(|g| (g - mean_g).powi(2)).sum::<f32>() / gn;
                println!(
                    "  {v:>6.2} {seed:>5} {:>7} {mean_h:>9.2} {:>9.3} {mean_g:>9.2} {:>9.3}",
                    crests.len(),
                    var_h.sqrt() / mean_h.max(1e-3),
                    var_g.sqrt() / mean_g.max(1e-3)
                );
            }
        }
    }

    #[test]
    #[ignore = "probe: prints, never asserts (review task 1b)"]
    fn probe_1b_the_keyhole_slots() {
        // The sighting: 1-2 column vertical slots cut the full height of
        // cliffs. The review's suspect is `terraced()` -- the snap is
        // full-strength where the mask saturates, and the mask *edge* can
        // flip a single column between snapped and unsnapped ground, which
        // would cut a one-column cliff out of otherwise continuous rock.
        //
        // Confirm or refute by printing the elevation chain term by term, so
        // the column that steps names itself. Everything here is a pure
        // function of `(seed, params, x)`, which is what makes this a unit
        // test rather than a world build.
        let (presets, err) = super::super::params::WorldgenPresets::load();
        assert!(err.is_none(), "{err:?}");
        for (preset, seed, centre) in
            [("canyon", 7u64, 610i32), ("canyon", 13, 205), ("rolling", 1, 1315), ("rolling", 2, 1490)]
        {
            let params = presets.get(preset).expect("preset");
            let t = shipped(seed, params);
            println!("\n=== {preset} seed {seed} around x {centre} (2048x640) ===");
            println!(
                "  {:>6} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>6}  note",
                "x", "mask", "m", "base", "hills", "snapdlt", "detail", "elev", "surf"
            );
            let p = t.params;
            let mut prev_surface: Option<i32> = None;
            for x in (centre - 20)..=(centre + 20) {
                let ch = t.character(x);
                let wx = t.warped_x(x);
                let base = t.base_wave(x);
                let hills = p.hill_amplitude
                    * ch.relief
                    * noise::fbm_1d_c(t.seed, Purpose::Height, wx / p.hill_wavelength, 4);
                let detail =
                    p.detail_amplitude * noise::fbm_1d_c(t.seed, Purpose::Detail, x as f32 / p.detail_wavelength, 2);
                // The mask, recomputed exactly as `terraced` computes it, so
                // the number printed is the number that decided.
                let mask = noise::fbm_1d(t.seed, Purpose::Mask, x as f32 / p.mask_wavelength, 2);
                let m = (p.terrace_strength * ch.resistance).clamp(0.0, 1.0) * noise::smoothstep(0.62, 0.82, mask);
                let pre = base + hills;
                let snap_delta = t.terraced(x, pre) - pre;
                let elev = t.elev(x);
                let surf = (t.datum() - elev).round() as i32;
                let jump = prev_surface.map(|s| surf - s).unwrap_or(0);
                let note = if jump.abs() >= 6 { format!("STEP {jump:+} rows from x-1") } else { String::new() };
                prev_surface = Some(surf);
                println!(
                    "  {x:>6} {mask:>8.3} {m:>8.3} {base:>8.2} {hills:>8.2} {snap_delta:>8.2} {detail:>8.2} {elev:>8.2} {surf:>6}  {note}"
                );
            }
        }
    }

    #[test]
    #[ignore = "probe: prints, never asserts (round-2 task 1)"]
    fn probe_r2t1_slope_at_the_steps() {
        // What pre-terrace slope does a keyhole riser actually sit on?
        //
        // The round-2 design attenuates the snap by regional slope, so the
        // thresholds have to be *derived* from where the bad steps are rather
        // than guessed: print the slope distribution at every big step beside
        // the distribution over the whole world, so the two can be separated.
        // Measuring first is the point -- a cutoff chosen before anyone had
        // looked would be the repo's recurring mistake.
        let (presets, err) = super::super::params::WorldgenPresets::load();
        assert!(err.is_none(), "{err:?}");
        println!("\n=== pre-terrace regional slope (+-8 central difference) ===");
        println!(
            "  {:>10} {:>5} {:>7} {:>8} {:>8} {:>8} {:>8} {:>8}",
            "preset", "seed", "worst", "@worst", "step>=6", "med@stp", "min@stp", "world p50"
        );
        for preset in ["rolling", "terraced", "canyon", "wetland", "arid"] {
            let params = presets.get(preset).expect("preset");
            for seed in [1u64, 2, 7, 13] {
                let t = shipped(seed, params);
                let surf = |x: i32| (t.datum() - t.elev(x)).round() as i32;
                let slope = |x: i32| (t.pre_terrace_elev(x + 8) - t.pre_terrace_elev(x - 8)).abs() / 16.0;
                let mut at_steps: Vec<f32> = Vec::new();
                let mut all: Vec<f32> = Vec::new();
                let mut worst = 0i32;
                let mut worst_slope = 0.0f32;
                for x in 9..2039 {
                    all.push(slope(x));
                    let d = (surf(x) - surf(x - 1)).abs();
                    if d >= 6 {
                        at_steps.push(slope(x));
                        if d > worst {
                            worst = d;
                            worst_slope = slope(x);
                        }
                    }
                }
                all.sort_by(|a, b| a.partial_cmp(b).unwrap());
                at_steps.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let med = |v: &Vec<f32>| if v.is_empty() { f32::NAN } else { v[v.len() / 2] };
                println!(
                    "  {preset:>10} {seed:>5} {worst:>7} {worst_slope:>8.3} {:>8} {:>8.3} {:>8.3} {:>8.3}",
                    at_steps.len(),
                    med(&at_steps),
                    at_steps.first().copied().unwrap_or(f32::NAN),
                    med(&all)
                );
            }
        }
    }

    #[test]
    #[ignore = "probe: prints, never asserts (round-2 task 1)"]
    fn probe_r2t1_what_the_worst_steps_are() {
        // A step is not automatically an artifact. The review's complaint was
        // a *slot* -- a 1-2 column notch that drops and comes straight back
        // -- and a single drop that stays down is a bluff, which is what a
        // terrace is supposed to produce. Print the worst few steps of each
        // world with enough shape around them to tell the two apart, because
        // the fix for one is not the fix for the other.
        let (presets, err) = super::super::params::WorldgenPresets::load();
        assert!(err.is_none(), "{err:?}");
        println!("\n=== the three worst steps per world: slot or bluff? ===");
        println!("  {:>10} {:>5} {:>6} {:>6} {:>7} {:>8} {:>10}", "preset", "seed", "x", "step", "slope", "recover", "verdict");
        for preset in ["rolling", "terraced", "canyon"] {
            let params = presets.get(preset).expect("preset");
            for seed in [1u64, 2, 7, 13] {
                let t = shipped(seed, params);
                let surf = |x: i32| (t.datum() - t.elev(x)).round() as i32;
                let slope = |x: i32| (t.pre_terrace_elev(x + 8) - t.pre_terrace_elev(x - 8)).abs() / 16.0;
                let mut steps: Vec<(i32, i32)> = Vec::new();
                for x in 9..2039 {
                    let d = surf(x) - surf(x - 1);
                    if d.abs() >= 6 {
                        steps.push((d.abs(), x));
                    }
                }
                steps.sort_by_key(|s| std::cmp::Reverse(s.0));
                for &(mag, x) in steps.iter().take(3) {
                    let d = surf(x) - surf(x - 1);
                    // How much of the drop is given back within four columns
                    // either side -- a slot gives nearly all of it back.
                    // How much of the drop comes back within four columns.
                    // Measured from x+1 onward -- including k=0 compares the
                    // step against itself and reports "fully recovered" for
                    // every step in the world, which is what the first
                    // version of this line did.
                    let back = (1..=4).map(|k| (surf(x) - surf(x + k)) * d.signum()).max().unwrap_or(0);
                    let recover = (back as f32 / mag as f32).clamp(0.0, 1.0);
                    let verdict = if recover > 0.6 { "SLOT" } else { "bluff" };
                    println!("  {preset:>10} {seed:>5} {x:>6} {d:>6} {:>7.3} {recover:>8.2} {verdict:>10}", slope(x));
                }
            }
        }
    }

    #[test]
    #[ignore = "probe: prints, never asserts (round-2 task 1)"]
    fn probe_r2t1_threshold_sweep() {
        // Derive `terrace_slope_lo/hi` from the census rather than choosing
        // them. Prints the worst single-column step per preset x seed at each
        // setting, which is the pre-registered acceptance quantity, plus the
        // count of steps so a "worst fell" that is really "the relief was
        // flattened" is visible as the count falling with it.
        let (presets, err) = super::super::params::WorldgenPresets::load();
        assert!(err.is_none(), "{err:?}");
        let settings: &[(f32, f32)] =
            &[(0.0, 0.0), (1.0, 3.0), (0.6, 2.0), (0.4, 1.2), (0.25, 0.9), (0.15, 0.6), (0.10, 0.40)];
        println!("\n=== worst single-column step (and count >= 6) by attenuation window ===");
        print!("  {:>10} {:>5}", "preset", "seed");
        for (lo, hi) in settings {
            print!("  {:>11}", format!("{lo}-{hi}"));
        }
        println!();
        for preset in ["rolling", "terraced", "canyon", "wetland", "arid"] {
            let base = presets.get(preset).expect("preset");
            for seed in [1u64, 2, 7, 13] {
                print!("  {preset:>10} {seed:>5}");
                for &(lo, hi) in settings {
                    let params = WorldgenParams { terrace_slope_lo: lo, terrace_slope_hi: hi, ..base.clone() };
                    let t = shipped(seed, &params);
                    let surf = |x: i32| (t.datum() - t.elev(x)).round() as i32;
                    let (mut worst, mut n) = (0i32, 0i32);
                    for x in 1..2048 {
                        let d = (surf(x) - surf(x - 1)).abs();
                        if d >= 6 {
                            n += 1;
                            worst = worst.max(d);
                        }
                    }
                    print!("  {:>11}", format!("{worst}/{n}"));
                }
                println!();
            }
        }
    }

    #[test]
    #[ignore = "probe: prints, never asserts (round-2 task 1)"]
    fn probe_r2t1_rolling_needs_a_different_lever() {
        // `rolling` seeds 1 and 2 carry the two tallest risers that slope
        // attenuation cannot reach, because they stand on gentle ground --
        // which is exactly the country the attenuation is written to leave
        // alone. The remaining lever named by finding 1b is `terrace_step`
        // itself. Measure what it would cost, so the finding carries numbers
        // rather than a suggestion.
        let (presets, err) = super::super::params::WorldgenPresets::load();
        assert!(err.is_none(), "{err:?}");
        let base = presets.get("rolling").expect("preset");
        println!("\n=== rolling: worst step / count, by terrace_step (attenuation 0.6-2.0) ===");
        print!("  {:>5}", "seed");
        for step in [26.0f32, 22.0, 18.0, 15.0, 12.0] {
            print!("  {:>10}", format!("step {step:.0}"));
        }
        println!();
        for seed in [1u64, 2, 7, 13] {
            print!("  {seed:>5}");
            for step in [26.0f32, 22.0, 18.0, 15.0, 12.0] {
                let params = WorldgenParams {
                    terrace_slope_lo: 0.6,
                    terrace_slope_hi: 2.0,
                    terrace_step: step,
                    ..base.clone()
                };
                let t = shipped(seed, &params);
                let surf = |x: i32| (t.datum() - t.elev(x)).round() as i32;
                let (mut worst, mut n) = (0i32, 0i32);
                for x in 1..2048 {
                    let d = (surf(x) - surf(x - 1)).abs();
                    if d >= 6 {
                        n += 1;
                        worst = worst.max(d);
                    }
                }
                print!("  {:>10}", format!("{worst}/{n}"));
            }
            println!();
        }
    }

    #[test]
    #[ignore = "probe: prints, never asserts (review task 1b)"]
    fn probe_1b_how_often_the_surface_steps() {
        // The other half of 1b, and the half a 41-column listing cannot give:
        // *how many* single-column steps a world contains, and how many of
        // them the terrace mask is responsible for. A slot is only a bug
        // worth a fix if there are enough of them to be the artifact the
        // review saw; if a world has three, the picture was showing
        // something else.
        //
        // "Attributed to the snap" means: the step survives in the
        // `terraced`-included chain and vanishes when the snap is switched
        // off (`terrace_strength: 0.0`), holding everything else equal.
        let (presets, err) = super::super::params::WorldgenPresets::load();
        assert!(err.is_none(), "{err:?}");
        println!("\n=== single-column surface steps >= 6 rows, 2048 wide ===");
        println!("  {:>10} {:>6} {:>10} {:>12} {:>10}", "preset", "seed", "steps", "snap-caused", "worst");
        for preset in ["rolling", "terraced", "canyon", "wetland", "arid"] {
            let params = presets.get(preset).expect("preset");
            let flat = WorldgenParams { terrace_strength: 0.0, ..params.clone() };
            for seed in [1u64, 2, 7, 13] {
                let t = shipped(seed, params);
                let f = shipped(seed, &flat);
                let surf = |t: &Terrain<'_>, x: i32| (t.datum() - t.elev(x)).round() as i32;
                let mut steps = 0;
                let mut caused = 0;
                let mut worst = 0;
                for x in 1..2048 {
                    let d = surf(&t, x) - surf(&t, x - 1);
                    if d.abs() >= 6 {
                        steps += 1;
                        worst = worst.max(d.abs());
                        if (surf(&f, x) - surf(&f, x - 1)).abs() < 6 {
                            caused += 1;
                        }
                    }
                }
                println!("  {preset:>10} {seed:>6} {steps:>10} {caused:>12} {worst:>10}");
            }
        }
    }
}
