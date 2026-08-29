//! A world as a sequence of *places*, rather than one place with knobs.
//!
//! # The problem this exists to solve
//!
//! Every world the generator made before this looked like every other world.
//! Not similar — structurally identical in the ways that matter, for three
//! reasons that were all deliberate decisions and all wrong:
//!
//! 1. **The base relief wave had a fixed phase.** It was fixed on purpose, to
//!    guarantee a ridge and a valley in frame at every seed. It succeeded, and
//!    it also meant the ridge was in the *same place* at every seed. The macro
//!    silhouette — the first thing anyone sees — did not vary at all.
//! 2. **Parameters were constant across x.** One soil depth, one terrace
//!    strength, one water table for the whole world, so the left of a world
//!    had the same character as the right.
//! 3. **Presets were fixed vectors.** `canyon` and `wetland` differed only in
//!    amplitude, which is why they read as the same world taller and flatter
//!    rather than as different country.
//!
//! Turning any of those knobs harder does not fix it: a bigger amplitude on a
//! world with one character everywhere is still one character everywhere.
//!
//! # What replaces it
//!
//! A world is cut into two to five **regions** along x. Each draws its own
//! elevation and its own *character* — how rugged, how dry, how resistant the
//! rock, how much loose cover — and the parameters a column generates from are
//! those characters blended smoothly across the boundaries. Crossing a world
//! now means crossing from one kind of place into another: a dry sandy stretch
//! rising into benched rock, falling into a wooded basin.
//!
//! The composition guarantee survives, and in a better form. It used to be "a
//! ridge here and a valley there, always"; it is now "the regions of this
//! world differ in elevation by at least this much, wherever they happen to
//! be" — enforced in [`RegionMap::new`] by spreading the elevations apart when
//! a draw comes out too flat. That is a guarantee about *relief* rather than
//! about a silhouette, which is what was actually wanted.

use super::noise::{self, Purpose};
use super::params::WorldgenParams;

/// What kind of place a region is. Every field is a multiplier or a 0..1
/// axis, never an absolute size, so a region modulates whatever preset it
/// finds itself in rather than overriding it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Character {
    /// Where this region's ground sits, `-1..1`, scaled by the preset's
    /// relief amplitude. The macro silhouette.
    pub elev: f32,
    /// Multiplies hill amplitude. Low is smooth country, high is broken.
    pub relief: f32,
    /// `0` lush, `1` desert. Drives loose cover from soil toward sand, pushes
    /// the water table down, and thins out what can grow — one axis moving
    /// four things, because that is what makes a place read as *dry* rather
    /// than as four unrelated settings that happen to coincide.
    pub aridity: f32,
    /// Multiplies terracing. High is benched and cliffy, low is smooth.
    pub resistance: f32,
    /// Multiplies loose-cover depth, independent of how dry it is: thin soil
    /// over rock, or a deep blanket.
    pub sediment: f32,
    /// How much standing residual rock this region tends to leave behind as
    /// its softer ground retreats — tors, stacks, boulder-strewn country —
    /// as a multiplier on both how often `residual.rs` tries a site and how
    /// large its size draw's ceiling reaches. Near `0` is smooth country
    /// with nothing standing; above `1` is coarse, monument-strewn country.
    ///
    /// Independent of `resistance` (which strata band happens to be hard):
    /// a region can carry sharp hardness contrast and still be smooth if
    /// nothing ever gets tall enough to matter at this reach, or the
    /// reverse. This is the axis the owner's "some regions boulder-strewn,
    /// some a few monuments, some smooth" directive names directly
    /// (`Reports/worldgen-erosion-design.md`'s 2026-08-20 addendum).
    pub formation: f32,
}

impl Character {
    /// The neutral character: takes the preset exactly as written.
    pub fn neutral(aridity: f32) -> Self {
        Self { elev: 0.0, relief: 1.0, aridity, resistance: 1.0, sediment: 1.0, formation: 1.0 }
    }

    fn lerp(a: Self, b: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let f = |x: f32, y: f32| x + (y - x) * t;
        Self {
            elev: f(a.elev, b.elev),
            relief: f(a.relief, b.relief),
            aridity: f(a.aridity, b.aridity),
            resistance: f(a.resistance, b.resistance),
            sediment: f(a.sediment, b.sediment),
            formation: f(a.formation, b.formation),
        }
    }
}

/// Fewest and most regions **per window** of world. Two is enough to be a
/// journey; past five, each region is too narrow to establish itself and the
/// stretch reads as noise rather than as places.
///
/// Per window, not per world, and that distinction is what lets the world
/// grow. Spreading a fixed two-to-five regions across whatever width the world
/// happens to be means a world four times wider gets regions four times
/// longer: the same count of places, each taking four screens to cross, so
/// travelling stops revealing anything. Scaling the count with width keeps the
/// *density* of change constant, which is what a player actually experiences.
const MIN_REGIONS: i32 = 2;
const MAX_REGIONS: i32 = 5;

/// The width those per-window counts are expressed against — roughly one
/// screen at 1:1. Composition is a property of what fits in view, so this is
/// the natural unit; see `column::Terrain::base_wave` for the same reasoning
/// applied to the macro silhouette.
const COMPOSITION_WINDOW: f32 = 512.0;

/// Ceiling on total regions, so an enormous world cannot spend its whole
/// generation budget drawing region characters nobody will visit in one run.
///
/// **A bound on work, never a gate on the density guarantee** -- the
/// distinction `CLAUDE.md` records from `rigid::fracture` declining to break
/// the largest regions and so dissolving the biggest collapses into dust. At
/// 64 it had quietly become the second kind: the 4x world is
/// [`super::super::app::WORLD_WIDTH`] / [`COMPOSITION_WINDOW`] = 16 windows
/// across and asks for up to `16 * MAX_REGIONS` = 80, so every draw above 64
/// was silently widened and the per-window density this module exists to
/// hold constant stopped holding. Sized here for a world four times wider
/// again, so the same thing cannot happen at the next size;
/// `the_shipped_world_does_not_hit_the_region_ceiling` is what keeps it
/// honest rather than this comment.
const MAX_TOTAL_REGIONS: i32 = 320;

/// Rock country is a *place*, not a rate — the constants below.
///
/// `formation` was drawn as `raw * raw * 2.2 * variation`: a skew toward the
/// low end, which the comment at the draw site described as "mostly smooth
/// with occasional coarse, monument-strewn country". Squaring does not
/// deliver that, because **`raw²` never reaches zero**. Measured over 400k
/// draws at the shipped `region_variation: 0.75`, only 17% of regions came
/// out effectively barren and the *median* region had standing rock — which
/// is "some formations everywhere", the exact shape the comment forbids.
///
/// The owner's verdict on a uniform thinning of `residual_density`
/// (1.4 -> 0.45), which is what that shape invites you to reach for:
/// *"Spires should not just be thinned out. They should be part of a
/// specific biome. They should not exist at all in most biomes but some
/// biomes should have them and they can be more regular. I didn't mean a
/// uniform decrease in spires."* The thinning also measured as a **deletion**
/// rather than a thinning: on `rolling` at 0.45 the spire census was
/// identical to a residuals-off control, so the pass had stopped
/// contributing anything at all, and the monuments shrank with it
/// (`heights` p90 33 -> 23).
///
/// So: a **hard gate, not a skew** — and the gate is read off a field far
/// coarser than a region. Below `FORMATION_BARREN` a region draws exactly
/// `0.0` and gets no residuals at all; `residual::residuals` needs no branch
/// for this, because a density of zero already floors to zero whole sites and
/// `unit(..) < 0.0` is never true.
///
/// **Why the gate is not read off the region's own draw**, which was tried
/// first and is the obvious implementation: a region here is 102-256 columns,
/// a fifth to a half of one screen (`MIN_REGIONS`/`MAX_REGIONS` are per
/// *window*). Gating on it makes a rock country smaller than the view, so it
/// reads as a cluster rather than as a place — and it measures that way too.
/// At 87% of regions barren the per-screen spire census came back canyon
/// 0/2/4, rolling 0/1/2, which is **the same census the rejected uniform
/// thinning gave**. Two very different worlds, one statistic, because the
/// thing that was supposed to distinguish them was smaller than the window it
/// was measured in. `Purpose::RockCountry` is the coarse field instead.
///
/// The knob and the gate own different questions: the gate decides *whether*
/// there is rock country here, `residual_density` decides how emphatic it is
/// where there is.
///
/// **Accepted provisionally, and the owner has named a different end state.**
/// Verdict on the card that landed this (`20260822T165724375Z-7046bb`):
/// *"This is fine for now. We are spending too much time on this. My overall
/// desire is for rocks be of all different shapes and sizes not rock country
/// with these unusual tall pillar rocks and then barren with no boulders, but
/// we can revise the rock formation generation in the future."*
///
/// So do not read this gate as settled design. What it fixed was real -- a
/// uniform thinning that deleted the pass on `rolling` outright -- but the
/// target is **variety of rock *form* everywhere**, not a binary present/absent
/// gate over one form. Two specific complaints to answer when that is picked
/// up, neither of which this constant can reach:
///
/// - the residuals themselves are thin pillars, which is `residual.rs`'s
///   aspect draw (`MIN_ASPECT`/`MAX_ASPECT`), not their distribution;
/// - barren country reads as empty, because the only other standing-rock pass
///   is `boulders`, which is driven by `erosion::Deposits` and so puts little
///   in a region the erosion pass did not work on.
///
/// The distribution work is done; the *shape* work is what is left.
const FORMATION_BARREN: f32 = 0.70;

/// How wide rock country is, in columns of world — the wavelength of the
/// `Purpose::RockCountry` field.
///
/// Sized in *screens*, not in regions, because that is the scale the question
/// is asked at: a country the player crosses in under a screen-width is a
/// cluster. At `COMPOSITION_WINDOW` = 512 this is a little over three screens
/// per feature, so a rock country spans several regions and each of them
/// draws its own intensity within it.
const ROCK_COUNTRY_SCALE: f32 = 1700.0;

/// The weakest rock country that is still rock country. Not near zero: a
/// region that survives the gate and then draws a formation of 0.05 is a
/// barren region with extra steps, and re-introduces by the back door the
/// smooth continuum the gate exists to break.
const FORMATION_FLOOR: f32 = 0.9;

/// The coarsest. `residual_density * FORMATION_PEAK * variation` is the site
/// count per `residual::REGION` columns in boulder-strewn country.
const FORMATION_PEAK: f32 = 2.4;

/// Within rock country, still weighted toward the floor: "a few monuments"
/// should be commoner than "boulder-strewn". This is the *shape* the old
/// squaring was reaching for, applied where it belongs — inside the regions
/// that have rock — rather than across all of them, where it only ever
/// produced a sprinkle everywhere.
const FORMATION_SKEW: f32 = 1.5;

/// The `formation` value at which a region draws residuals at the full
/// `residual::MAX_HEIGHT` ceiling — the coarsest country there is.
///
/// **Re-derived with the gate above, not carried over.** The old divisor was
/// `1.5`, chosen against the old `raw * raw * 2.2 * variation` draw whose
/// maximum at the shipped `region_variation: 0.75` was `1.65`. So the ceiling
/// was only ever approached by the top ~5% of *all* regions, and the biggest
/// monuments were rarer than reading the constant suggests. A gated draw tops
/// out at `FORMATION_PEAK * variation` instead, which is what this tracks.
///
/// The `0.75` is the shipped default `region_variation` (`params.rs`), and it
/// is a *reference point*, not a claim that every preset uses it: a preset
/// asking for more regional variation gets more regions at the full ceiling,
/// which is the right direction for a knob named "how different are these
/// places from each other".
pub(super) const FORMATION_FULL_CEILING: f32 = FORMATION_PEAK * 0.75;

/// How far apart the highest and lowest region must sit, in units of
/// `elev`. The composition guarantee: below this the world is a plain, and
/// a plain every fifth seed is not variety, it is a dud.
const MIN_ELEV_SPREAD: f32 = 1.45;

/// Fraction of the gap between two region centres that the change actually
/// happens over. The rest is each region's flat core.
///
/// **This number decides whether the world has any soil on it.** Blending
/// evenly across the whole gap -- the obvious implementation, and the first
/// one -- turns every region boundary into one long uniform ramp, and a
/// sustained moderate slope is the worst possible case for loose cover: not
/// steep enough to read as a cliff, too steep to hold soil. Measured against
/// the cover cutoff, a full-width blend spends almost the entire slope budget
/// on the macro shape and leaves a thin crust everywhere, in every preset.
///
/// Concentrating the change into a band gives each region a flat core that
/// holds deep cover, and an escarpment between regions that is honestly
/// steep and honestly bare. Plateaus and scarps rather than ramps -- which is
/// both the better landscape and the one that can carry soil.
const TRANSITION: f32 = 0.42;

/// Remap a 0..1 position between two centres onto the transition band.
fn transition(t: f32) -> f32 {
    let edge = (1.0 - TRANSITION) * 0.5;
    let t = ((t - edge) / TRANSITION).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Regions across one world, and the blend between them.
pub struct RegionMap {
    /// Centre x and character, in x order.
    centres: Vec<(f32, Character)>,
}

impl RegionMap {
    /// Draw the regions of a world.
    pub fn new(seed: u64, p: &WorldgenParams, w: i32) -> Self {
        let variation = p.region_variation.clamp(0.0, 1.0);
        // A preset can ask for a world with no regional variation at all --
        // `flat` does, because a structural test bed whose character wandered
        // would be measuring a different question in each half.
        if variation <= 0.0 {
            return Self { centres: vec![(w as f32 * 0.5, Character::neutral(p.aridity))] };
        }

        let span = (MAX_REGIONS - MIN_REGIONS + 1) as f32;
        let per_window = MIN_REGIONS + (noise::unit(seed, Purpose::Region, 0, 0) * span) as i32;
        let per_window = per_window.clamp(MIN_REGIONS, MAX_REGIONS);
        // Regions per *window*, scaled up by how many windows wide the world
        // is. At the original 512 this is exactly the old behaviour.
        //
        // **Times `cell_scale`, because a window is one screen and a screen
        // is a number of cells, not a number of ground.** Without it a world
        // built at twice the cell resolution gets twice the regions rather
        // than the same regions twice as wide -- which is not a subtle
        // difference: measured on `rolling` seed 1, it is the whole reason a
        // rescaled world came out no more like the original than a different
        // seed did. See `WorldgenParams::cell_scale`.
        let windows = (w as f32 / (COMPOSITION_WINDOW * p.cell_scale.max(f32::EPSILON))).max(1.0);
        let count = ((per_window as f32 * windows).round() as i32).clamp(MIN_REGIONS, MAX_TOTAL_REGIONS);

        // Centres first, and the country field over them, because the rock
        // country gate needs the whole world's draws before it can decide its
        // threshold -- see the guarantee below.
        //
        // Evenly spaced, jittered by up to a third of a region so the
        // boundaries do not fall on a grid. Even spacing alone would be its
        // own kind of sameness.
        let centre_x: Vec<f32> = (0..count)
            .map(|i| {
                let nominal = (i as f32 + 0.5) / count as f32;
                let jitter = (noise::unit(seed, Purpose::Region, i, 1) - 0.5) * 0.66 / count as f32;
                (nominal + jitter).clamp(0.02, 0.98) * w as f32
            })
            .collect();
        let country: Vec<f32> = centre_x
            .iter()
            .map(|&cx| noise::value_1d(seed, Purpose::RockCountry, cx / ROCK_COUNTRY_SCALE))
            .collect();

        // **The rock-country guarantee, enforced rather than hoped for** --
        // the same shape as the elevation-spread guarantee further down, and
        // for the same reason. A world with no standing rock anywhere is not
        // variety, it is a missing feature, and rejecting the draw outright
        // would bias the distribution; dropping the bar to the best draw keeps
        // the shape that was drawn and only fixes the range.
        //
        // Two cases need it, and only one of them is exotic. At the 512-column
        // test size the world is narrower than a third of one period of the
        // country field, so it samples what is essentially a single value and
        // a world either is rock country or has none --
        // `every_pass_writes_something` failed on exactly that, reporting
        // "pass residuals never wrote a cell across 5 seeds". At the shipped
        // 8192 it is not exotic at all: measured over 16 seeds, 1 world in 16
        // drew no country. That is rare enough to look like bad luck in
        // testing and common enough for a player to meet.
        let best = country.iter().copied().fold(f32::MIN, f32::max);
        // When the guarantee fires, the best sample *defines* the country --
        // and a country is a place with extent, not an argmax. The first
        // version of the fallback set `gate = best` and kept the `>= gate`
        // test, which admits exactly the one region that drew the maximum:
        // a knife-edge, and on a world narrower than the country field's
        // period it selects between samples the field itself cannot tell
        // apart. Measured on the foraging scene's 512x120 world (rolling,
        // seed 1): two regions, country 0.4141 vs 0.4691, both far under
        // `FORMATION_BARREN` -- "essentially a single value", as the
        // paragraph above says -- yet only cx=459 passed, so the colony's
        // home range at cx=47 lost the residual towers its foraging bars
        // were measured on (`Reports/open-bugs-handoff.md` §L, 98 round
        // trips -> 2). On the shipped 8192 world the knife-edge is wrong
        // the same way: one 102-256-column region is "a fifth to a half of
        // one screen", the exact cluster-not-a-country shape the
        // `FORMATION_BARREN` comment records as rejected. So membership in
        // the fallback country is distance to the best draw's centre, at
        // the field's own scale: everything within half a
        // `ROCK_COUNTRY_SCALE` of the peak is the same country. A 512 world
        // becomes rock country whole; an 8192 fallback world gets one
        // country-sized band instead of one region-sized cluster.
        let best_cx = centre_x[country
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i)
            .unwrap_or(0)];
        let in_country = |i: usize| {
            if best >= FORMATION_BARREN {
                country[i] >= FORMATION_BARREN
            } else {
                (centre_x[i] - best_cx).abs() <= ROCK_COUNTRY_SCALE * 0.5
            }
        };

        let mut centres = Vec::with_capacity(count as usize);
        for i in 0..count {
            let cx = centre_x[i as usize];

            // Each axis drawn independently, so a region can be dry *and*
            // rugged, or wet and smooth, rather than sliding along one
            // "extremeness" dial that would make every region a variation of
            // the same trade.
            let draw = |slot: i32| (noise::unit(seed, Purpose::Region, i, slot) - 0.5) * 2.0;
            let character = Character {
                elev: draw(2),
                relief: (1.0 + draw(3) * 0.65 * variation).max(0.15),
                aridity: (p.aridity + draw(4) * 0.55 * variation).clamp(0.0, 1.0),
                resistance: (1.0 + draw(5) * 0.9 * variation).max(0.0),
                sediment: (1.0 + draw(6) * 0.8 * variation).max(0.05),
                // Gated, not skewed, and the only axis here that is: most
                // country has no standing rock *at all*, and the rest has it
                // properly. See `FORMATION_BARREN` for why a skew could not
                // deliver that and what the uniform-thinning alternative
                // measured as. Slot 7, claimed when this axis landed (0-6
                // were already spoken for).
                formation: {
                    // The gate is the *country* field, sampled at this
                    // region's own centre so the region blend carries it;
                    // the per-region draw only sets intensity within a
                    // country that already exists. See `FORMATION_BARREN`
                    // for why gating on `raw` instead measured as a
                    // uniform thinning.
                    let raw = noise::unit(seed, Purpose::Region, i, 7);
                    if !in_country(i as usize) {
                        0.0
                    } else {
                        // `raw` is unconditioned here -- the gate spent the
                        // country field, not this one -- so it runs the full
                        // 0..1 and `FORMATION_SKEW` shapes rock country end
                        // to end.
                        let strength =
                            FORMATION_FLOOR + (FORMATION_PEAK - FORMATION_FLOOR) * raw.powf(FORMATION_SKEW);
                        (strength * variation).max(0.0)
                    }
                },
            };
            centres.push((cx, character));
        }

        // The composition guarantee, enforced rather than hoped for. A draw
        // where every region landed at a similar elevation is a flat world,
        // and rejecting it outright would bias the distribution; stretching
        // it keeps the shape that was drawn and only fixes the range.
        let lo = centres.iter().map(|c| c.1.elev).fold(f32::MAX, f32::min);
        let hi = centres.iter().map(|c| c.1.elev).fold(f32::MIN, f32::max);
        let spread = hi - lo;
        if spread < MIN_ELEV_SPREAD {
            let n = centres.len();
            if spread <= 1e-4 || n < 2 {
                // Degenerate: every region drew the same elevation. Fan them
                // out rather than shipping a dead flat world.
                for (i, (_, c)) in centres.iter_mut().enumerate() {
                    let t = if n > 1 { i as f32 / (n - 1) as f32 } else { 0.5 };
                    c.elev = (t - 0.5) * MIN_ELEV_SPREAD;
                }
            } else {
                // Renormalise onto the target span, centred on zero.
                //
                // Scaling in place around the draw's own midpoint was the
                // first version and it did not hold: a draw sitting near the
                // top of the range stretches past `1.0`, the clamp cuts it
                // back, and the world ships flatter than the guarantee claims
                // — seed 15 spanned 1.23 against a promised 1.45. Centring
                // first means the result cannot leave the range, so there is
                // nothing to clamp and nothing to silently lose.
                let mid = (hi + lo) * 0.5;
                let gain = MIN_ELEV_SPREAD / spread;
                for (_, c) in &mut centres {
                    c.elev = (c.elev - mid) * gain;
                }
            }
        }

        Self { centres }
    }

    /// The character at a column, blended between the regions either side.
    ///
    /// Smoothstep rather than linear: a linear blend puts a visible crease at
    /// every region centre, because the interpolation's slope changes there.
    pub fn sample(&self, x: i32) -> Character {
        let xf = x as f32;
        if self.centres.len() == 1 {
            return self.centres[0].1;
        }
        // Outside the outermost centres, hold the end regions rather than
        // extrapolating -- extrapolation runs a character off its own scale
        // at exactly the world edges, where it is most visible.
        if xf <= self.centres[0].0 {
            return self.centres[0].1;
        }
        if xf >= self.centres[self.centres.len() - 1].0 {
            return self.centres[self.centres.len() - 1].1;
        }
        for pair in self.centres.windows(2) {
            let (ax, a) = pair[0];
            let (bx, b) = pair[1];
            if xf >= ax && xf <= bx {
                let t = (xf - ax) / (bx - ax).max(1e-3);
                return Character::lerp(a, b, transition(t));
            }
        }
        self.centres[self.centres.len() - 1].1
    }

    /// How many regions this world has, for diagnostics and tests.
    pub fn len(&self) -> usize {
        self.centres.len()
    }

    pub fn is_empty(&self) -> bool {
        self.centres.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> WorldgenParams {
        WorldgenParams::default()
    }

    #[test]
    fn a_world_with_no_variation_has_one_region() {
        let p = WorldgenParams { region_variation: 0.0, ..params() };
        let map = RegionMap::new(1, &p, 512);
        assert_eq!(map.len(), 1);
        // And it is the same everywhere, which is what a test bed needs.
        assert_eq!(map.sample(0), map.sample(511));
    }

    #[test]
    fn different_seeds_lay_out_different_regions() {
        // The headline claim. If this fails, worlds are interchangeable again.
        let p = params();
        let a = RegionMap::new(1, &p, 512);
        let b = RegionMap::new(2, &p, 512);
        let differs = (0..512).any(|x| a.sample(x) != b.sample(x));
        assert!(differs, "two seeds produced the same regional layout");
    }

    #[test]
    fn every_world_has_real_relief_between_its_regions() {
        // The composition guarantee, in its new form: not "a ridge in this
        // exact spot" but "the regions of this world genuinely differ in
        // height". A flat draw every few seeds would be a dud world.
        let p = params();
        for seed in 0..64u64 {
            let map = RegionMap::new(seed, &p, 512);
            let lo = (0..512).map(|x| map.sample(x).elev).fold(f32::MAX, f32::min);
            let hi = (0..512).map(|x| map.sample(x).elev).fold(f32::MIN, f32::max);
            assert!(
                hi - lo >= MIN_ELEV_SPREAD - 0.01,
                "seed {seed}: regions span only {:.2} of elevation",
                hi - lo
            );
        }
    }

    #[test]
    fn character_varies_across_a_world() {
        // Not just at the macro scale: the *kind* of place has to change too,
        // or every region is the same country at a different height.
        let p = params();
        let map = RegionMap::new(7, &p, 512);
        let arid: Vec<f32> = (0..512).step_by(16).map(|x| map.sample(x).aridity).collect();
        let lo = arid.iter().cloned().fold(f32::MAX, f32::min);
        let hi = arid.iter().cloned().fold(f32::MIN, f32::max);
        assert!(hi - lo > 0.1, "aridity barely changes across the world: {lo:.2}..{hi:.2}");
    }

    #[test]
    fn regions_stay_window_sized_as_the_world_grows() {
        // The property that lets the world get bigger without getting duller.
        // A fixed count spread over any width means a four-times-wider world
        // has regions four screens across, so walking reveals a quarter as
        // much. What must hold constant is regions *per window*, not per
        // world.
        let p = params();
        for &w in &[512, 1024, 2048, 4096] {
            let map = RegionMap::new(9, &p, w);
            let per_window = map.len() as f32 / (w as f32 / COMPOSITION_WINDOW);
            assert!(
                (MIN_REGIONS as f32..=MAX_REGIONS as f32).contains(&per_window),
                "{w} wide: {} regions is {per_window:.1} per window, outside {MIN_REGIONS}..{MAX_REGIONS}",
                map.len()
            );
        }
    }

    #[test]
    fn a_wide_world_still_changes_character_within_a_screen() {
        // The guarantee restated at the scale a player experiences it: pick
        // any window-sized span of a large world and it should contain real
        // relief, not just the world as a whole. Checked as a proportion
        // rather than for every span, because a genuinely broad plateau is a
        // legitimate landform and demanding variation everywhere would forbid
        // it.
        let p = params();
        let w = 4096;
        let map = RegionMap::new(4, &p, w);
        let window = COMPOSITION_WINDOW as i32;
        let mut varied = 0;
        let mut windows = 0;
        for start in (0..w - window).step_by(window as usize / 2) {
            let lo = (start..start + window).map(|x| map.sample(x).elev).fold(f32::MAX, f32::min);
            let hi = (start..start + window).map(|x| map.sample(x).elev).fold(f32::MIN, f32::max);
            windows += 1;
            if hi - lo > 0.25 {
                varied += 1;
            }
        }
        assert!(
            varied * 4 >= windows * 3,
            "only {varied} of {windows} window-sized spans have any relief in them"
        );
    }

    #[test]
    fn the_blend_is_continuous() {
        // A jump in character is a seam in the terrain, and seams are what
        // this whole layer exists to avoid.
        let p = params();
        let map = RegionMap::new(3, &p, 512);
        for x in 1..512 {
            let a = map.sample(x - 1);
            let b = map.sample(x);
            assert!((a.elev - b.elev).abs() < 0.05, "elevation jumps at x {x}");
            assert!((a.aridity - b.aridity).abs() < 0.05, "aridity jumps at x {x}");
        }
    }
}
