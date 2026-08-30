//! Plan-space erosion: formations as side effects of simulated history.
//!
//! The implementation of `Reports/worldgen-erosion-design.md`, and the first
//! consumer of `world_age`. Between `column.rs`'s per-column decisions and
//! the realise passes, the surface heightfield `h[x]` is put through a small
//! deterministic erosion/deposition simulation — thermal shed that breaks
//! plumb faces into talus, and hydraulic carve that gives the terrain
//! drainage structure and sediment-floored valleys. Hardness comes per
//! *strata band* from the same field the shade pass draws, so what stands
//! proud after erosion is the banding the player can already see in every
//! cut face. The outcome is never authored, only the rates are.
//!
//! Everything here runs on `w` floats for `age * ITERS_PER_AGE` iterations
//! at build time — tens of milliseconds, not a cell pass — and is a pure
//! function of `(seed, params)`: fixed iteration counts, all randomness
//! through `Purpose::Hardness` keyed on the band index, ties in the
//! hydraulic walk broken by column index. `column.rs`'s purity tests extend
//! to the eroded plan on exactly that argument.
//!
//! **`world_age == 0.0` is a guaranteed no-op**: `erode` returns before
//! touching a column, so every pre-erosion world — including the sweep
//! baselines and the in-flight round-3 branch's guards — is preserved
//! bit-exactly until the per-preset defaults are deliberately flipped on.

use super::column::Terrain;

/// Iterations of the loop per unit of `world_age`. The default age of 1.0
/// on a preset that opts in should read as "weathered but still sharp";
/// 2.0 as subdued. Set from looking at strips, like every rate below.
const ITERS_PER_AGE: f32 = 600.0;

/// One hydraulic pass per this many thermal iterations. Water works in
/// events; rock creeps continuously. Also the cost lever: the hydraulic
/// pass is the only one that sorts.
const HYDRO_INTERVAL: u32 = 8;

/// ---- thermal (dry) rates ----
///
/// Slope a fully *soft* surface band holds without shedding, in cells of
/// rise per cell of run. Near powder repose on purpose: the first setting
/// (0.9, with a 2.6 hard bonus) converged every face to ~2.7 cells/cell,
/// which is still 70° and still reads as plumb — the probe showed 1,848
/// cells of height moved while only 1.5% of strip pixels changed. For a
/// face to read *stepped*, the soft bands have to cut back to a walkable
/// ramp while the hard bands hold; the contrast is the picture.
const THERMAL_STABLE_SOFT: f32 = 0.55;
/// Extra stable slope a fully hard band adds on top. Hard caps hold faces
/// several times steeper than repose — that difference is where ledges,
/// hoodoo caps and every stepped face come from, so it is deliberately
/// large. At 2.2 every residual in the canyon notch melted along with the
/// soft rock around it; a cap that is to leave a spire standing has to
/// hold near-vertical while its neighbours cut back to a ramp, so the
/// contrast is the knob that decides whether formations survive at all.
const THERMAL_STABLE_HARD_BONUS: f32 = 4.5;
/// Fraction of the over-steepness moved per contact per iteration. Kept
/// well under 0.5: at 0.5 a symmetric pair oscillates instead of relaxing.
const THERMAL_RATE: f32 = 0.15;
/// Hillslope creep: a plain Laplacian smoothing of `h`, scaled by surface
/// softness, applied every iteration. The threshold rule above never
/// touches ground shallower than its stable angle, so without this an old
/// world keeps every crest and knick exactly as sharp as a young one and
/// "subdued" never happens — creep is what rounds a hill, the textbook
/// pairing with stream-power incision. Volume-conserving by construction.
/// At 0.03 × 600 iterations the smoothing length is ~4 columns: rounding,
/// not blurring.
const SOFT_CREEP: f32 = 0.03;
/// Share of thermally shed volume recorded as talus (gravel) at the column
/// it lands on, rather than as plain rock lowering. The rest of the moved
/// height still moves — this is an annotation for the realise side, not
/// extra volume.
const TALUS_SHARE: f32 = 0.6;

/// ---- hydraulic rates ----
///
/// Rain supply per column per hydraulic pass, scaled by `1 - aridity`.
const RAIN_SUPPLY: f32 = 1.0;
/// Stream-power carve coefficient: `dh = HYDRO_RATE * flow^0.5 * slope *
/// softness`. Sublinear in flow (the design's a = 0.5) so big flows widen
/// their valleys rather than knifing one-column slots.
const HYDRO_RATE: f32 = 0.045;
/// Carrying capacity per unit of `flow^0.5 * slope`. Where slope flattens,
/// capacity collapses and the carried load drops as sediment — valley fill.
const CAPACITY: f32 = 0.9;
/// Hard cap on carve per column per pass, in cells. A safety bound, not a
/// behaviour knob: the size-cap lesson says a cap must bound work, and this
/// one bounds the step so the explicit scheme stays stable whatever the
/// rates above are set to.
const MAX_CARVE_PER_PASS: f32 = 0.5;

/// ---- differential lowering: what turns stratigraphy into relief ----
///
/// **This is the term the rest of the module was missing.** Every other rate
/// here is driven by *slope* — thermal shed relaxes over-steep faces, creep
/// diffuses curvature, stream power carves in proportion to gradient — so on
/// ground that is already gentle they all do nothing, and gentle is what our
/// ground is. Measured before this landed: no column ever peaked above its
/// iteration-0 prominence, 0 of 2048, in either preset
/// (`Reports/worldgen-prior-art-and-dead-ends-2026-08-29.md` #22). Erosion
/// could only ever remove.
///
/// Real landscapes lower everywhere — rain, frost, wind, solution — at a rate
/// set by the rock, not by the slope. That is denudation, and against a
/// uniform uplift it is the whole of differential erosion: a resistant
/// package stands proud, a soft one cuts back, and the scarp between them is
/// where one has been breached and the other has not.
///
/// Written as one signed term rather than as separate uplift and lowering,
/// which is the same arithmetic: a column whose section resists exactly as
/// well as its neighbourhood holds still, harder rises, softer falls. See
/// [`STRIP_REACH`] for why the reference is the neighbourhood and not a
/// constant — it is the difference between this term shaping a landscape and
/// it erasing one.
///
/// It is bounded by the geology rather than by a cap. A column that stands
/// proud rises out of its resistant bed into whatever lies above; a column
/// that cuts back sinks until it meets one that holds. So the excursion is a
/// walk over the strata that stops at the first section that resists, and its
/// scale is set by the vertical correlation of hardness — see
/// `HardnessField::section`, which is why that reads a package and not a bed.
///
/// Cells of surface movement per iteration at full resistance contrast. Set
/// by measurement over the three W1 measures (`wg_ceilings mode=relief`) and
/// by looking at the result at the shipped size, not by argument.
const STRIP_RATE: f32 = 1.50;

/// Half-width, in columns, of the neighbourhood a bed's resistance is
/// measured **against**.
///
/// **The term is a contrast, not a level, and that is what stops it planing
/// the world.** Written as an absolute — lower where the rock is soft,
/// raise where it is hard, against a fixed reference hardness — it was
/// built, measured as a gain on every number in `wg_ceilings mode=relief`,
/// and rendered as the flattest world this generator has ever made: the
/// surface reached its fixed point everywhere, and since a bed's resistance
/// has no `x` in it, that fixed point is the bedding plane. Long dead-level
/// tables with sharp notches, the hills gone.
///
/// Subtracting a running mean over `+-STRIP_REACH` leaves exactly the part of
/// the signal that varies *within* a landscape and takes out everything
/// longer. So the macro shape — the region layout, the massif, the composition
/// guarantee `RegionMap::new` enforces — passes through untouched, and
/// differential erosion does what it actually does in the field: it decides
/// which parts of a hillside stand out, not how high the hill is.
///
/// It also deletes a constant. Against a fixed reference hardness the
/// reference has to be calibrated against the field's own mean, and that mean
/// moves whenever the hardness draw or the regional gain does; a contrast
/// against the local mean is invariant to both.
///
/// Sized a little under a player screen: a formation that is wider than the
/// view is not a formation, it is the ground.
const STRIP_REACH: i32 = 200;

/// Iterations of settling run after the lowering term stops.
///
/// Denudation makes steps; thermal shed and creep are what turn a step into a
/// scarp with a slope its rock can hold. Running them together to the last
/// iteration leaves the newest steps unrelaxed — one-column jumps of a
/// fraction of a cell, invisible, but they are also the ones `cliff_edges`
/// reads. This is a tail of the *existing* passes with the new term switched
/// off, not a new mechanism.
const SETTLE_ITERS: u32 = 60;

/// ---- hardness field ----
///
/// Half-width of the lateral smoothing window over surface hardness, in
/// columns. The design's coherence floor: residual features narrower than
/// the window cannot survive, and a 1-column residual is the keyhole
/// artifact wearing a costume (review finding 1b). The per-band draw and
/// its floor live in `Terrain::surface_hardness`, beside the strata
/// coordinate they are coupled to.
const HARDNESS_SMOOTH_REACH: i32 = 2;

/// ---- boulder sockets ----
///
/// Cumulative thermal shed *from a hard surface* past which a column is
/// marked as a boulder socket. Markers are data on the plan — the later
/// boulder realise pass reads them and never infers "boulder-worthy" from
/// shape (the state-the-difference-as-data lesson).
const BOULDER_SHED_THRESHOLD: f32 = 6.0;
/// Hardness above which shed counts toward a socket.
const BOULDER_HARDNESS: f32 = 0.65;

/// What erosion moved and left behind, alongside the eroded `h`.
///
/// `talus` and `sediment` are depths in cells of loose cover at each
/// column; in the current stage they realise through the soil-blanket
/// machinery (added to `soil_depth`, clamped by the same repose taper), so
/// the at-rest guarantee is inherited rather than re-proved. Drawing talus
/// as gravel and seating boulders at the markers is the realise-side stage
/// that lands with the data track (`worldgen-erosion-design.md`
/// §Delegation).
pub struct Deposits {
    pub talus: Vec<f32>,
    pub sediment: Vec<f32>,
    pub boulder: Vec<bool>,
    /// Total plan volume moved (cells of height, summed absolute), the
    /// "did it fire" counter for the pass table.
    pub volume_moved: f32,
    /// Volume that flowed out at the world edges — the design treats edges
    /// as outlets, so conservation is `moved = deposited + exported`.
    pub exported: f32,
    pub iterations: u32,
    /// Cells of height the differential-lowering term removed, and raised,
    /// summed over every column and iteration.
    ///
    /// **Its own pair of counters because this term is the one part of the
    /// pass that is not volume-conserving**, and folding it into
    /// `volume_moved` would hide that. Denudation exports its product to the
    /// sea and uplift brings rock up from below; neither is a transfer
    /// between columns, so `moved = deposited + exported` is a statement
    /// about the thermal and hydraulic terms only. Reported separately in
    /// the detail line so "did the relief term fire, and by how much" is a
    /// number rather than an inference from the picture.
    pub stripped: f32,
    pub raised: f32,
    /// Wall-clock time the loop itself took, in milliseconds. Round-4 task
    /// 5's "did it fire, and what did it cost" counter -- printed beside
    /// the pass table by `generate_reported`, never asserted on: timing is
    /// read by eye, not gated (`Reports/worldgen-erosion-design.md`'s ≤50ms
    /// budget line is a by-eye check, and CI machine speed varies run to
    /// run per CLAUDE.md's re-measure-the-baseline rule).
    pub wall_time_ms: f32,
}

impl Deposits {
    fn empty(w: usize) -> Self {
        Self {
            talus: vec![0.0; w],
            sediment: vec![0.0; w],
            boulder: vec![false; w],
            volume_moved: 0.0,
            exported: 0.0,
            iterations: 0,
            stripped: 0.0,
            raised: 0.0,
            wall_time_ms: 0.0,
        }
    }
}

/// Erode `h` in place for `params.world_age * ITERS_PER_AGE` iterations.
pub fn erode(t: &Terrain, h: &mut [f32]) -> Deposits {
    let w = h.len();
    // How much of the differential term this world has room to run: 1 at the
    // shipped size, 0 in a 320-row test world that has none. See
    // `Terrain::relief_room_fraction`.
    let room = t.relief_room_fraction();
    let strip = room > 0.0;
    let aged = (t.params.world_age.max(0.0) * ITERS_PER_AGE) as u32;
    let mut d = Deposits::empty(w);
    // **`world_age == 0` stays a guaranteed no-op, settle tail included.**
    // Adding the tail before this test was written first and is wrong: the
    // `flat` preset is the structural test bed and asks for age 0 precisely
    // so that its ground is the un-eroded curve, and sixty iterations of
    // thermal and creep on it is not that world.
    if aged == 0 || w < 3 {
        return d;
    }
    let iters = aged + if strip { SETTLE_ITERS } else { 0 };
    d.iterations = iters;
    let started = std::time::Instant::now();

    // Rain supply and a shed-from-hard accumulator, both per column.
    let rain: Vec<f32> =
        (0..w).map(|x| RAIN_SUPPLY * (1.0 - t.character(x as i32).aridity).max(0.0)).collect();
    let mut hard_shed = vec![0.0f32; w];
    // Scratch reused across hydraulic passes.
    let mut order: Vec<usize> = (0..w).collect();
    let mut hardness = vec![0.0f32; w];
    let mut raw_hardness = vec![0.0f32; w];
    let (mut d_strip, mut d_raise) = (0.0f32, 0.0f32);
    let mut section = vec![0.0f32; w];

    // The hardness sampler with its per-column invariants (strata offset,
    // regional resistance) precomputed — the loop below resamples hardness
    // every iteration because eroding a surface moves it into a different
    // band, and paying the offset fBm and the region blend each time was
    // most of the pass's cost for no information.
    let field = t.hardness_field();

    for it in 0..iters {
        // Surface hardness this iteration, smoothed laterally (the
        // coherence floor). Resampled because erosion lowers surfaces into
        // different bands — a cap that is stripped exposes the softer band
        // under it, which is the whole hoodoo mechanism.
        for x in 0..w {
            raw_hardness[x] = field.at(x as i32, h[x]);
        }
        for (x, out) in hardness.iter_mut().enumerate() {
            let mut sum = 0.0;
            let mut n = 0.0;
            for k in -HARDNESS_SMOOTH_REACH..=HARDNESS_SMOOTH_REACH {
                let i = x as i64 + k as i64;
                if i >= 0 && (i as usize) < w {
                    sum += raw_hardness[i as usize];
                    n += 1.0;
                }
            }
            *out = sum / n;
        }

        // **Differential lowering** — see `STRIP_RATE`. Runs before the
        // slope terms so that every step it makes is relaxed by them in the
        // same iteration rather than a whole iteration later, and stops
        // `SETTLE_ITERS` short of the end so the last steps are relaxed too.
        //
        // Reads the *section* under the surface, not the bed at it: a cap
        // rock is one bed and the thing that stands proud is the package
        // beneath it. Resampled every iteration for the same reason the
        // hardness above is — lowering a surface moves it into a different
        // part of the sequence, and a stripped cap exposing the softer rock
        // under it is the whole mechanism.
        if strip && it + SETTLE_ITERS < iters {
            for x in 0..w {
                section[x] = field.section(x as i32, h[x]);
            }
            // The neighbourhood mean, by running sum: O(w) per iteration
            // rather than O(w * STRIP_REACH), which at reach 200 is the
            // difference between this term being free and it being the pass.
            // The window is clamped at both ends rather than wrapped -- the
            // world edge is an outlet, not a seam.
            let (mut acc, mut lo, mut hi) = (0.0f32, 0i32, -1i32);
            for x in 0..w as i32 {
                let want_hi = (x + STRIP_REACH).min(w as i32 - 1);
                while hi < want_hi {
                    hi += 1;
                    acc += section[hi as usize];
                }
                let want_lo = (x - STRIP_REACH).max(0);
                while lo < want_lo {
                    acc -= section[lo as usize];
                    lo += 1;
                }
                let d = STRIP_RATE * room * (section[x as usize] - acc / (hi - lo + 1) as f32);
                h[x as usize] += d;
                if d < 0.0 {
                    d_strip -= d;
                } else {
                    d_raise += d;
                }
            }
        }

        // Thermal: each adjacent pair relaxes toward the stable slope of
        // whichever column stands higher (its band is the one shedding).
        for x in 0..w - 1 {
            let diff = h[x] - h[x + 1];
            let (hi, lo) = if diff >= 0.0 { (x, x + 1) } else { (x + 1, x) };
            let hard = hardness[hi];
            let stable = THERMAL_STABLE_SOFT + hard * THERMAL_STABLE_HARD_BONUS;
            let over = diff.abs() - stable;
            if over <= 0.0 {
                continue;
            }
            let moved = over * THERMAL_RATE * (1.0 - hard).max(0.05);
            h[hi] -= moved;
            h[lo] += moved;
            d.talus[lo] += moved * TALUS_SHARE;
            d.volume_moved += moved;
            if hard > BOULDER_HARDNESS {
                hard_shed[hi] += moved;
                if hard_shed[hi] > BOULDER_SHED_THRESHOLD {
                    d.boulder[hi] = true;
                }
            }
        }

        // Hillslope creep: soft ground diffuses a little every iteration,
        // which is what rounds crests and knicks the threshold rule above
        // can never touch. Read from a snapshot via `prev` so the pass is
        // order-independent (a sweeping in-place Laplacian drifts
        // features in the sweep direction).
        let prev: Vec<f32> = h.to_vec();
        for x in 1..w - 1 {
            let soft = (1.0 - hardness[x]).max(0.05);
            let lap = prev[x - 1] + prev[x + 1] - 2.0 * prev[x];
            let moved = SOFT_CREEP * soft * lap;
            h[x] += moved;
            d.volume_moved += moved.abs();
        }

        // Hydraulic, every HYDRO_INTERVAL iterations: walk columns from
        // high to low, accumulating flow downhill; carve by stream power,
        // drop load where capacity collapses.
        if !it.is_multiple_of(HYDRO_INTERVAL) {
            continue;
        }
        // Deterministic order: height descending, index as tie-break, so
        // two columns at exactly equal height never depend on sort
        // internals.
        order.sort_unstable_by(|&a, &b| h[b].total_cmp(&h[a]).then(a.cmp(&b)));
        let mut flow = rain.clone();
        let mut load = vec![0.0f32; w];
        for &x in order.iter() {
            // Receiving neighbour: the lower side; the world edge is an
            // outlet lower than everything.
            let left = if x > 0 { h[x - 1] } else { f32::NEG_INFINITY };
            let right = if x + 1 < w { h[x + 1] } else { f32::NEG_INFINITY };
            let (nh, nx) = if left <= right { (left, x.wrapping_sub(1)) } else { (right, x + 1) };
            if nh >= h[x] {
                // Local minimum: a basin. Everything carried settles here.
                d.sediment[x] += load[x];
                h[x] += load[x];
                continue;
            }
            let slope = (h[x] - nh.max(h[x] - 4.0)).max(0.0) / 1.0;
            let power = flow[x].max(0.0).sqrt() * slope;
            let soft = (1.0 - hardness[x]).max(0.05);
            let carve = (HYDRO_RATE * power * soft).min(MAX_CARVE_PER_PASS);
            h[x] -= carve;
            d.volume_moved += carve;
            let capacity = CAPACITY * power;
            let carrying = load[x] + carve;
            let (pass_on, drop_here) = if carrying > capacity {
                (capacity, carrying - capacity)
            } else {
                (carrying, 0.0)
            };
            if drop_here > 0.0 {
                d.sediment[x] += drop_here;
                h[x] += drop_here;
            }
            if nx == usize::MAX || nx >= w {
                // Off the edge: flow and load leave the world.
                d.exported += pass_on;
            } else {
                flow[nx] += flow[x];
                load[nx] += pass_on;
            }
        }
    }
    d.stripped = d_strip;
    d.raised = d_raise;
    d.wall_time_ms = started.elapsed().as_secs_f32() * 1000.0;
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worldgen::params::WorldgenParams;

    fn terrain(seed: u64, params: &WorldgenParams) -> Terrain<'_> {
        Terrain::new(seed, params, 512, 320, 33.0_f32.to_radians().tan(), 34.0_f32.to_radians().tan())
    }

    #[test]
    fn age_zero_is_a_bit_exact_no_op() {
        // `WorldgenParams::default()` is no longer age 0 as of round-4 task
        // 4 (`rolling` ships `world_age: 0.8`, and `rolling` is asserted
        // equal to the compiled default) -- the no-op guarantee itself is
        // unchanged, only how this test reaches age 0 to check it.
        let p = WorldgenParams { world_age: 0.0, ..Default::default() };
        let t = terrain(5, &p);
        let mut h: Vec<f32> = (0..t.w).map(|x| t.elev(x)).collect();
        let before = h.clone();
        let d = erode(&t, &mut h);
        assert_eq!(h, before, "age 0 must not touch a column");
        assert_eq!(d.volume_moved, 0.0);
        assert_eq!(d.iterations, 0);
    }

    #[test]
    fn erosion_is_pure() {
        let p = WorldgenParams { world_age: 1.0, ..Default::default() };
        let t = terrain(5, &p);
        let mut a: Vec<f32> = (0..t.w).map(|x| t.elev(x)).collect();
        let mut b = a.clone();
        let da = erode(&t, &mut a);
        let db = erode(&t, &mut b);
        assert_eq!(a, b, "two erosions of the same world disagree");
        assert_eq!(da.volume_moved, db.volume_moved);
        assert_eq!(da.talus, db.talus);
        assert_eq!(da.sediment, db.sediment);
    }

    #[test]
    fn erosion_moves_real_volume_and_deposits_it() {
        // "Did it fire" as a counter, not a picture: an aged world must
        // show moved volume and standing deposits, or the mechanism is
        // dead however plausible a strip looks.
        let p = WorldgenParams { world_age: 1.0, ..Default::default() };
        for seed in [1u64, 7] {
            let t = terrain(seed, &p);
            let mut h: Vec<f32> = (0..t.w).map(|x| t.elev(x)).collect();
            let d = erode(&t, &mut h);
            assert!(d.volume_moved > 10.0, "seed {seed}: only {} cells of height moved", d.volume_moved);
            let talus: f32 = d.talus.iter().sum();
            let sediment: f32 = d.sediment.iter().sum();
            assert!(
                talus > 1.0 && sediment > 1.0,
                "seed {seed}: talus {talus}, sediment {sediment} — one process never produced"
            );
        }
    }

    #[test]
    fn an_aged_world_arrives_at_rest() {
        // The non-negotiable, empirically, for the configuration nothing
        // else covers: every existing at-rest gate runs the default
        // `world_age` of 0, where erosion is a no-op by construction — so
        // without this, the aged world's at-rest guarantee would rest
        // entirely on the argument that deposits route through the same
        // gates, and arguments are not evidence in this repo. Two seeds,
        // 120 frames, not one cell may move (the same bar
        // `tests/worldgen.rs` holds the un-aged presets to).
        // The same claim `tests/worldgen.rs::generated_terrain_is_already_
        // at_rest` makes, held to the same bar: positions and materials,
        // not cell bit-equality. The liquid solver legitimately shuffles
        // sub-cell fill in a standing pond (a full-cell compare flagged
        // 2,185 pond-surface water cells whose aux drifted under
        // evaporation while not one cell moved), and the life pass is off
        // because growing flora rewrites cells by design.
        use crate::sim::chunk::Rect;
        use crate::sim::world::World;
        let params = WorldgenParams {
            world_age: 1.0,
            tree_density: 0.0,
            moss_density: 0.0,
            grass_density: 0.0,
            ..Default::default()
        };
        let snapshot = |w: &World| -> std::collections::HashSet<(i32, i32, u16)> {
            let mut set = std::collections::HashSet::new();
            for y in 0..320 {
                for x in 0..512 {
                    let c = w.get(x, y);
                    if c.material != crate::sim::material::EMPTY {
                        set.insert((x, y, c.material.0));
                    }
                }
            }
            set
        };
        for seed in [1u64, 7] {
            let mut w = World::new(Rect::new(0, 0, 511, 319));
            crate::worldgen::generate(&mut w, crate::worldgen::Spec::Generated { params: &params, seed });
            w.end_step();
            let before = snapshot(&w);
            for _ in 0..120 {
                crate::sim::parallel::step(&mut w);
            }
            let after = snapshot(&w);
            // The water cycle excluded, and only it. A standing pond's
            // surface legitimately ripples cell-to-cell under evaporation
            // even at genuine rest, and snow melts off on the sky's own
            // schedule now that the day/night swing reaches ~26C at noon --
            // neither is the *terrain* failing to settle, which is what
            // this test is named for. `residual.rs` has excluded water for
            // exactly this reason and its comment already cites this test
            // as documenting the caveat; this is that caveat, made true.
            //
            // Measured when the water-cycle branch merged: seed 1 moved
            // nothing at all, seed 7 moved 336 cells and **every one of
            // them was water** -- no solid moved on either seed. The
            // surplus shuffling itself is a known open bug (1f, "a pond
            // with rock in it never stops shuffling fill"), recorded
            // rather than fixed; if a solid ever appears in this set it is
            // a real regression and this filter will not hide it.
            let weather: Vec<u16> = ["water", "snow", "ice", "steam"]
                .iter()
                .filter_map(|n| w.materials.id_of(n).map(|m| m.0))
                .collect();
            let mut gone: Vec<_> = before
                .difference(&after)
                .filter(|&&(_, _, m)| !weather.contains(&m))
                .copied()
                .collect();
            gone.sort();
            let sample: Vec<String> = gone
                .iter()
                .take(8)
                .map(|&(x, y, m)| {
                    format!("({x},{y}) {}", w.materials.get(crate::sim::material::MaterialId(m)).name)
                })
                .collect();
            assert!(
                gone.is_empty(),
                "seed {seed}: {} cells left their position in an aged world; first: {}",
                gone.len(),
                sample.join(", ")
            );
        }
    }

    #[test]
    fn differential_erosion_builds_relief_that_slope_driven_erosion_cannot() {
        // **The guard for W1's differential-lowering term**, and it replaces
        // `an_old_world_is_smoother_than_a_young_one`, which is deleted rather
        // than ported. That test compared summed |slope| at age 0 against
        // age 2 on **seeds 1 and 7 only**, and asserted the old world was at
        // least 2% smoother. Measured over 16 seeds when W1 nudged seed 7
        // across the line, the ratio's median is **1.185 on the shipped code
        // and 1.104 with W1** -- i.e. an old world is *rougher* at the median
        // in both arms, and the property was already false before this lane
        // touched anything. Moving the comparison onto age 0.5, the stretch
        // the test's own comment called monotone, gives medians of 1.010 and
        // 1.003: still not the claim. It was a two-seed test over a
        // procedural system whose property does not hold, which is
        // `CLAUDE.md`'s *"a guard over a procedural system has to sweep the
        // procedure and gate an order statistic"* exactly, and it had been
        // rubber-stamping for as long as it has existed.
        //
        // What is guarded instead is the thing the module now claims and did
        // not before: erosion **creates** formation-scale relief rather than
        // only relaxing it. Dead ends #22/#23 record the old state as
        // measured -- no column ever peaked above its iteration-0 prominence,
        // 0 of 2048.
        //
        // **The two arms differ in one thing: whether the world has room.**
        // `Terrain::relief_room_fraction` scales the term by the rows left
        // after the sky guarantee, so a 320-row world runs none of it and a
        // 1200-row world runs all of it, while `elev` -- which does not read
        // `h` once `massif_amplitude` is zero -- hands both arms the identical
        // starting profile. So this is a paired A/B with the same seed, the
        // same terrain and the same binary, and `massif_amplitude: 0.0` is
        // what makes that true rather than nearly true.
        //
        // **Both a fired counter and an effect counter**, per `CLAUDE.md`: a
        // null hides equally well behind a mechanism that is quiet and a probe
        // that never reached it, so `stripped` says the term ran and the local
        // relief says it did something.
        let p = WorldgenParams { world_age: 1.0, massif_amplitude: 0.0, ..Default::default() };
        let (soil, sand) = (33.0_f32.to_radians().tan(), 34.0_f32.to_radians().tan());
        // Local relief at the formation scale, p90 over the world. Not
        // *prominence*: that is two-sided, so it scores zero on a scarp, a
        // bench rim and the whole interior of a plateau, which is most of what
        // this term makes.
        let relief_p90 = |h: &[f32]| -> f32 {
            const REACH: usize = 20;
            let mut v: Vec<f32> = (REACH..h.len() - REACH)
                .map(|x| {
                    let w = &h[x - REACH..=x + REACH];
                    w.iter().cloned().fold(f32::MIN, f32::max) - w.iter().cloned().fold(f32::MAX, f32::min)
                })
                .collect();
            v.sort_by(f32::total_cmp);
            v[(v.len() as f32 * 0.9) as usize]
        };
        let mut ratios = Vec::new();
        for seed in 0..8u64 {
            let arm = |h: i32| -> (f32, f32) {
                let t = Terrain::new(seed, &p, 512, h, soil, sand);
                let mut hv: Vec<f32> = (0..t.w).map(|x| t.elev(x)).collect();
                let d = erode(&t, &mut hv);
                (relief_p90(&hv), d.stripped)
            };
            let (no_room, stripped_off) = arm(320);
            let (room, stripped_on) = arm(1200);
            assert_eq!(stripped_off, 0.0, "seed {seed}: the term ran in a world with no room for it");
            assert!(stripped_on > 100.0, "seed {seed}: the term did not fire ({stripped_on} cells stripped)");
            ratios.push(room / no_room);
        }
        ratios.sort_by(f32::total_cmp);
        let median = ratios[ratios.len() / 2];
        // Measured 2.33 over these eight seeds, with two of them below 1.0 --
        // the spread is real and is why this gates a median rather than every
        // seed. The bar sits well under the measurement, per `CLAUDE.md`'s
        // "set bars from measurement with headroom, never sitting on the
        // measured value".
        assert!(median > 1.4, "differential erosion added no formation-scale relief: median ratio {median}, all {ratios:?}");
    }

}

#[cfg(test)]
mod probe {
    use super::*;
    use crate::worldgen::params::WorldgenPresets;

    /// Prints what erosion actually does to the shipped canyon at world
    /// size — volume moved, the largest single-column change, deposit
    /// sums — because the strips looked near-identical across ages and a
    /// picture cannot say whether the mechanism is weak or dead.
    ///
    /// ```text
    /// cargo test --lib erosion_probe -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn erosion_probe() {
        let (presets, _) = WorldgenPresets::load();
        for name in ["canyon", "rolling", "arid"] {
            let mut p = presets.presets.get(name).expect("preset exists").clone();
            p.world_age = 1.0;
            let t = Terrain::new(1, &p, 2048, 640, 33.0_f32.to_radians().tan(), 34.0_f32.to_radians().tan());
            let before: Vec<f32> = (0..t.w).map(|x| t.elev(x)).collect();
            let mut h = before.clone();
            let t0 = std::time::Instant::now();
            let d = erode(&t, &mut h);
            let elapsed = t0.elapsed();
            let max_dh = h.iter().zip(&before).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
            let max_slope_before =
                before.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
            let max_slope_after = h.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
            println!(
                "{name}: moved {:.1} exported {:.1} talus {:.1} sediment {:.1} boulders {} | max|dh| {:.2} max slope {:.1} -> {:.1} | {elapsed:.1?}",
                d.volume_moved,
                d.exported,
                d.talus.iter().sum::<f32>(),
                d.sediment.iter().sum::<f32>(),
                d.boulder.iter().filter(|&&b| b).count(),
                max_dh,
                max_slope_before,
                max_slope_after,
            );
        }
    }

    /// Round-4 task 4 diagnostic: is roughness monotone in `world_age` past
    /// the ages the shipped presets carry? Not asserted, printed -- this is
    /// deciding whether a test's *parameters* still probe the property it
    /// was written for, not measuring the mechanism itself.
    ///
    /// ```text
    /// cargo test --lib roughness_curve -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn roughness_curve() {
        use crate::worldgen::params::WorldgenParams;
        let base = WorldgenParams { world_age: 0.0, ..Default::default() };
        for seed in [1u64, 7] {
            let mut prev = f32::INFINITY;
            for age in [0.0f32, 0.5, 0.8, 1.0, 1.5, 2.0, 2.5, 3.0] {
                let p = WorldgenParams { world_age: age, ..base.clone() };
                let t = Terrain::new(seed, &p, 512, 320, 33.0_f32.to_radians().tan(), 34.0_f32.to_radians().tan());
                let mut h: Vec<f32> = (0..t.w).map(|x| t.elev(x)).collect();
                erode(&t, &mut h);
                let roughness: f32 = h.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
                let dir = if roughness < prev { "down" } else { "UP" };
                println!("seed {seed} age {age}: roughness {roughness:.1} ({dir})");
                prev = roughness;
            }
        }
    }
}
