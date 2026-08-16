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

use super::noise::{self, Purpose};
use super::params::WorldgenParams;

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
}

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

    /// The base relief wave: one full period across `w`.
    ///
    /// **The phase is fixed, not seeded, and that is load-bearing.** It is
    /// what guarantees the composition requirement — a ridge and a valley in
    /// frame at every seed, rather than the statistically uniform wiggle that
    /// fBm alone produces and that makes every side-view world of this genre
    /// look the same.
    ///
    /// A **sine**, not a cosine, and the difference is the whole composition.
    /// A cosine peaks at both world edges and troughs in the middle, which
    /// produces one enormous symmetric bowl — rendered, it reads as a crater,
    /// not as landscape, and there is no ridge anywhere the player can stand
    /// on. A sine puts a ridge a quarter of the way across and a valley three
    /// quarters across, both entirely inside the frame, with the edges at
    /// mid-height. Two landforms rather than one, and neither of them cut in
    /// half by the world edge.
    ///
    /// **When the world grows past one screen** (M10 streaming), `w` here
    /// must stop being world width and become a fixed composition wavelength
    /// of roughly one window, or the whole world will flatten into a single
    /// enormous hill. Ultimately this entire term is the placeholder that the
    /// coarse `(x, z)` map replaces (design doc §5), at which point carrying
    /// composition at traversal scale becomes that map's job.
    fn base_wave(&self, x: i32) -> f32 {
        let period = self.w.max(1) as f32;
        self.params.relief_amplitude * (std::f32::consts::TAU * x as f32 / period).sin()
    }

    /// Surface elevation in cells, **+up**, before conversion to a row.
    pub fn elev(&self, x: i32) -> f32 {
        let p = self.params;
        let wx = self.warped_x(x);
        let hills = p.hill_amplitude * noise::fbm_1d_c(self.seed, Purpose::Height, wx / p.hill_wavelength, 4);
        let detail =
            p.detail_amplitude * noise::fbm_1d_c(self.seed, Purpose::Detail, x as f32 / p.detail_wavelength, 2);
        // Detail is added *after* terracing, not before. Snapping a surface
        // that already contains fine roughness throws that roughness away
        // wherever the snap bites, which is exactly where it is most needed:
        // benches come out glassy flat and their edges perfectly straight,
        // and the whole feature reads as cut from a solid rather than
        // weathered out of one. Applied afterwards, the same noise roughens
        // the treads and ragged the lips.
        self.terraced(x, self.base_wave(x) + hills) + detail
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
        let m = p.terrace_strength * noise::smoothstep(0.62, 0.82, mask);
        if m <= 0.0 {
            return e;
        }
        // Into band space (y-down, offset by the strata displacement), snap,
        // and back out.
        let offset = self.strata_offset(x);
        let band_coord = (self.datum() - e) + offset;
        let snapped = (band_coord / p.terrace_step).round() * p.terrace_step;
        let stepped = self.datum() + offset - snapped;
        e + (stepped - e) * m
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
    fn datum(&self) -> f32 {
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

    /// Decide one column.
    pub fn plan(&self, x: i32) -> ColumnPlan {
        let p = self.params;
        let surface_y = (self.datum() - self.elev(x)).round() as i32;
        // The clamp keeps the massif inside the world with room for bedrock
        // beneath it; it bites only on extreme parameter values.
        let surface_y = surface_y.clamp(4, self.h - 12);

        // Soil thins as the ground steepens and is gone entirely past the
        // cutoff. This is the at-rest guarantee, not an aesthetic choice: a
        // powder resting on a slope shallower than its angle of repose cannot
        // avalanche, so soil placed under this rule never moves. Bare rock on
        // the steep faces is the same rule read as a picture.
        let cutoff = (p.soil_slope_cutoff * self.soil_tan).max(0.0);
        // The neighbourhood slope, not the immediate one: a bench is flat at
        // the exact column and the drop is two cells away. Still an upper
        // bound on the true local slope, so the at-rest guarantee this gate
        // provides only gets stricter, never looser.
        let steepness = self.slope(x).max(self.slope_near(x));
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
            let depth = (p.soil_depth * taper + jitter).max(0.0).round() as i32;
            // Where the blanket has already worn thin, let the rock through
            // entirely rather than leaving a one- or two-cell skin over it.
            //
            // A uniform crust of soil over every surface was the strongest
            // remaining tell that the ground was generated: real hillsides
            // are patchy, and the bare rock shows through exactly where the
            // cover is thinnest. It also costs nothing to be sure of — taking
            // powder away can never make a world less at rest than leaving it
            // there.
            if depth <= 3 && noise::unit(self.seed, Purpose::SoilNoise, x, 0) < 0.45 {
                0
            } else {
                depth
            }
        };

        let table_y = (self.datum() - p.table_damping * self.low_elev(x) + p.table_offset).round() as i32;

        // A perturbed band so the world floor is not a ruler line (there are
        // no straight horizontal edges in this world except water surfaces).
        let band = (p.bedrock_band * noise::fbm_1d(self.seed, Purpose::Bedrock, x as f32 / 23.0, 2)).round() as i32;
        let bedrock_top_y = self.h - 2 - band.max(0);

        ColumnPlan { surface_y, soil_depth, table_y, bedrock_top_y }
    }

    /// Every column of the world, in x order. Computed once and shared by
    /// every pass, since several passes need their neighbours' plans and
    /// recomputing the noise per lookup would be the generator's whole cost.
    pub fn plan_all(&self) -> Vec<ColumnPlan> {
        (0..self.w).map(|x| self.plan(x)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terrain(seed: u64, params: &WorldgenParams) -> Terrain<'_> {
        Terrain { seed, params, w: 512, h: 320, soil_tan: 33.0_f32.to_radians().tan() }
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
        // The at-rest guarantee. Anywhere soil exists, the ground it rests on
        // must be shallower than its angle of repose.
        let p = WorldgenParams::default();
        for seed in 0..8u64 {
            let t = terrain(seed, &p);
            let cutoff = p.soil_slope_cutoff * t.soil_tan;
            for x in 0..t.w {
                if t.plan(x).soil_depth > 0 {
                    assert!(t.slope(x) < cutoff, "seed {seed} x {x}: soil on a {} slope", t.slope(x));
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
}
