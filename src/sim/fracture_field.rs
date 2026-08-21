//! The **joint fabric**: the grain rock already has, before anything hits it.
//!
//! Every cell in the world belongs to a Worley domain — the lattice cell of
//! its nearest feature point. Two adjacent cells in *different* domains have
//! a **joint** between them: a plane the rock is disposed to part along.
//! Nothing is stored; the domain of a cell is a pure function of the world
//! seed and the coordinate, so the fabric costs no memory and is identical
//! every time it is asked about.
//!
//! # Why this exists at all, and what it replaces
//!
//! Blast fracture used to be drawn by [`structural::FissureWalks`] — a fan of
//! random walkers turning up to `CRACK_WANDER` (0.9 rad) at *every cell*. The
//! owner rejected the result three times, in escalating terms: *"thin
//! criss-cross wiggly crack patterns — looks like a graphic, not physics"*,
//! then *"it shouldn't look like a scribble"*, then — after a blind A/B of two
//! tunings of that walker, both of which he declined — *"I thought we were
//! going to match the Voronoi type pattern from my worldgen example image."*
//!
//! Two properties of the pattern he pointed at are out of the walker's reach
//! by construction, which is why this is a new mechanism and not a knob:
//!
//! - **The edges are straight.** The scribble is the walker's *statistic*,
//!   not its tuning: a heading that is re-rolled every cell cannot draw a
//!   straight segment however small the wander is made. Worley boundaries
//!   are straight by construction — a domain boundary is a piece of the
//!   perpendicular bisector between two feature points.
//! - **The cells are closed.** A walker encloses a piece only by luck, and
//!   four rounds of work went into making that luck likelier (decomposed
//!   diagonals, both perpendicular edges, the mirror write). The severing
//!   rule here needs none of it: **an edge is a joint iff its two cells are
//!   in different domains**, and the set of such edges is *exactly* the
//!   boundary of each domain on the 4-connected grid. Support is
//!   4-connected (`NEIGHBOURS_4` throughout `structural.rs` and
//!   `rigid::take_fragment`), so a domain whose whole boundary is severed
//!   is enclosed — watertight, not lucky.
//!
//! Note the rule is an **identity comparison, not a threshold on a distance
//! field**. `worldgen`'s caves threshold `f2 - f1` low to carve the boundary
//! network, and a threshold has *width*: it leaks at the corners and needs
//! lateral patching. The identity has no width at all.
//!
//! # Why the kernel is copied and not imported
//!
//! `worldgen::noise::worley_f2_f1` is the same arithmetic and lives two
//! layers away. It is not imported for three separate reasons, each of which
//! would be a real problem on its own:
//!
//! - `src/worldgen/mod.rs` states as an invariant that nothing under
//!   `src/sim/` imports worldgen, and nothing does. This would be the first.
//! - The kernel is keyed on `worldgen::noise::Purpose`, whose discriminants
//!   are claimed 1..=26 with 27 reserved on the worldgen data track. A sim
//!   module adding a variant there — or borrowing a number — would collide
//!   with work in flight on another branch.
//! - It returns `(f1, f2)`, the two *distances*. What the fabric needs is the
//!   nearest point's **identity**, which that signature throws away.
//!
//! So: a private salt, a local hash, and about twenty lines of arithmetic.
//!
//! # The 3x3 neighbourhood is exact for what this asks
//!
//! `worley_f2_f1`'s doc records the 3x3 feature-point search as an accepted
//! approximation, because it can understate `F2` when the true second-nearest
//! point sits two lattice cells away. **That caveat does not apply here.**
//! One feature point lies in each unit lattice cell, so a point outside the
//! 3x3 block around the sample's own cell is at least one full lattice unit
//! further away on some axis than the point *in* the sample's own cell — it
//! can never be the nearest. `F1`'s identity is exact in 3x3, and widening
//! the search would cost 2.8x for a guaranteed-identical answer. Do not
//! "fix" it.

use super::rng;

/// This module's own hash salt, deliberately unrelated to
/// `worldgen::noise::Purpose`'s numbering — see the module doc. Any fixed
/// odd 64-bit constant works; this one is arbitrary.
const JOINT_SALT: u64 = 0x5F2D_1C4B_9A73_E681;

/// A second salt, for the per-joint activation draw, so a joint's *identity*
/// and its *chance of being woken* are not the same hash of the same input.
const ACTIVATION_SALT: u64 = 0xC3A5_C85C_97CB_3127;

/// SplitMix64-style finalizer over `(seed, salt, x, y)`.
///
/// Not a stream RNG: there is no state and no call order, so two callers
/// asking about the same lattice cell get the same answer whenever they ask.
/// That is what makes the fabric a property of the rock rather than of the
/// event history — a second charge on the same spot retraces the first one's
/// joints exactly.
fn hash(seed: u64, salt: u64, x: i32, y: i32) -> u64 {
    let mut z = seed
        ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (x as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (y as i64 as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Which Worley domain a world cell belongs to — the lattice coordinates of
/// its nearest feature point.
///
/// `cell_size` is the lattice pitch in world cells, and therefore the
/// characteristic width of a fragment: a domain is one lattice cell's worth
/// of area, so joints land roughly `cell_size` apart. It comes from the
/// material (`Material::joint_spacing`), so two materials with different
/// grain never share a domain map — see `joint_between`.
///
/// Degenerate `cell_size` (zero or negative) would divide the world by zero
/// and is clamped rather than asserted: `joint_spacing` is content, loaded
/// from a `.ron` file the player can edit, and content must not be able to
/// panic the simulation.
pub fn domain(seed: u64, x: i32, y: i32, cell_size: f32) -> (i32, i32) {
    let pitch = cell_size.max(1.0);
    let (u, v) = (x as f32 / pitch, y as f32 / pitch);
    let (u0, v0) = (u.floor() as i32, v.floor() as i32);
    let mut best = f32::MAX;
    let mut winner = (u0, v0);
    for j in -1..=1 {
        for i in -1..=1 {
            let (cx, cy) = (u0 + i, v0 + j);
            let h = hash(seed, JOINT_SALT, cx, cy);
            // Top 24 bits and the next 24: each is every bit an `f32`
            // mantissa holds, taken from disjoint parts of one finalized
            // hash. Two separate draws at hand-scrambled coordinates is the
            // classic way to put correlation back between the axes.
            let fx = (h >> 40) as f32 / (1u64 << 24) as f32;
            let fy = ((h >> 16) & 0x00FF_FFFF) as f32 / (1u64 << 24) as f32;
            let (dx, dy) = (cx as f32 + fx - u, cy as f32 + fy - v);
            let d = dx * dx + dy * dy;
            // Strictly less, so the scan order breaks a tie the same way
            // every run. Squared distance throughout: the ordering is all
            // this needs and the square root would only cost accuracy.
            if d < best {
                best = d;
                winner = (cx, cy);
            }
        }
    }
    winner
}

/// The activation draw for the joint between two domains, in `[0, 1)`.
///
/// **Keyed on the pair of domains, not on the individual edge**, and that is
/// the difference between a craquelure and a dotted line. A boundary between
/// two domains is a straight run of tens of edges; drawing each edge's own
/// chance independently would activate them in a dashed scatter, which is
/// the scribble complaint wearing a different hat. One draw for the whole
/// boundary means a joint is *either* a full straight segment *or* absent —
/// and since the caller compares it against a distance ramp, a boundary is
/// drawn from the blast outward and stops where the ramp falls below its own
/// draw. That is where *"some cracks might be short and only near the blast
/// and some could extend farther"* comes from: not a length distribution,
/// but one number per boundary against a falling ramp.
///
/// The pair is canonically ordered so that both cells of an edge agree about
/// which boundary they are on.
pub fn joint_draw(seed: u64, a: (i32, i32), b: (i32, i32)) -> f32 {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    // Fold the two lattice coordinates of each domain into one integer each.
    // 0x1_0001 rather than a power of two so the two axes do not simply
    // occupy separate bit fields, which would leave the finalizer correlating
    // neighbouring domains along a row.
    let ka = lo.0.wrapping_mul(0x1_0001) ^ lo.1.wrapping_mul(0x3B9A_CA07);
    let kb = hi.0.wrapping_mul(0x1_0001) ^ hi.1.wrapping_mul(0x3B9A_CA07);
    let h = hash(seed, ACTIVATION_SALT, ka, kb);
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// A second independent draw for the same boundary — its *delay* before the
/// growth front reaches it. Separate from [`joint_draw`] so that how likely a
/// joint is to wake and how late it wakes are not the same number; keyed the
/// same way, so a whole boundary still races outward as one line.
pub fn joint_delay(seed: u64, a: (i32, i32), b: (i32, i32)) -> f32 {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    // `rng::jitter3` rather than a third salt: it is the engine's existing
    // position-keyed draw, it takes three inputs, and nothing here needs the
    // 64-bit width.
    rng::jitter3(lo.0.wrapping_mul(31) ^ lo.1, hi.0.wrapping_mul(31) ^ hi.1, (seed ^ (seed >> 32)) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The property the whole mechanism rests on: the joint set is the exact
    /// boundary of the domains, so a domain is **enclosed** by its own
    /// joints on the 4-connected grid. Not "usually", not "at this
    /// threshold" — by construction.
    ///
    /// Written as a flood fill that may not cross a joint, exactly the way
    /// `structural`'s relaxation and `rigid::take_fragment` traverse, and it
    /// must not escape the domain it starts in.
    #[test]
    fn a_domain_is_enclosed_by_its_own_joints() {
        let seed = 0xDEAD_BEEF;
        let pitch = 9.0;
        let start = (40, 40);
        let home = domain(seed, start.0, start.1, pitch);

        let mut seen = std::collections::HashSet::new();
        let mut queue = vec![start];
        seen.insert(start);
        while let Some((x, y)) = queue.pop() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                // The world is unbounded here on purpose: if the flood can
                // leak it will run away, and a bounded scan would hide that
                // behind the bound. 4,000 cells is far more than one
                // 9x9 domain and far less than a runaway.
                assert!(seen.len() < 4_000, "the flood escaped its domain -- the joint web leaks");
                if domain(seed, nx, ny, pitch) != home {
                    continue; // a joint: support does not cross it
                }
                if seen.insert((nx, ny)) {
                    queue.push((nx, ny));
                }
            }
        }
        // And it is a real region, not a single cell that trivially "sealed".
        assert!(seen.len() > 20, "the domain at {start:?} is only {} cells at pitch {pitch}", seen.len());
    }

    /// Fragment size tracks `cell_size`, which is the only reason the
    /// per-material `joint_spacing` knob is worth having: a coarser pitch has
    /// to give visibly bigger pieces, not the same pieces relabelled.
    #[test]
    fn a_coarser_pitch_gives_bigger_domains() {
        let seed = 7;
        let count_at = |pitch: f32| -> usize {
            let mut domains = std::collections::HashSet::new();
            for y in 0..120 {
                for x in 0..120 {
                    domains.insert(domain(seed, x, y, pitch));
                }
            }
            domains.len()
        };
        let fine = count_at(6.0);
        let coarse = count_at(18.0);
        assert!(fine > coarse * 4, "pitch 6 found {fine} domains and pitch 18 found {coarse} -- the pitch is not driving fragment size");
    }

    /// Determinism, and the "same rock, same grain" property a repeat charge
    /// relies on: no state, no call order, no drift between two asks.
    #[test]
    fn the_fabric_is_a_pure_function_of_position() {
        let a = domain(11, 137, -42, 7.5);
        for i in 0..500 {
            let _ = domain(11, i, i * 3, 7.5);
            let _ = joint_draw(11, (i, 0), (i + 1, 0));
        }
        assert_eq!(a, domain(11, 137, -42, 7.5));
        assert_eq!(joint_draw(3, (1, 2), (4, 5)), joint_draw(3, (4, 5), (1, 2)), "the pair key must not depend on which cell asked");
    }

    /// Different worlds get different grain. A fabric that ignored the seed
    /// would put every world's joints in the same places, which nothing
    /// downstream could tell apart from the fabric working.
    #[test]
    fn the_seed_moves_the_grain() {
        let differ = (0..200).filter(|&i| domain(1, i, i * 2, 8.0) != domain(2, i, i * 2, 8.0)).count();
        assert!(differ > 100, "only {differ} of 200 samples moved between two seeds");
    }

    /// The activation draw has to spread over its range, or the ramp it is
    /// compared against is a step function wearing a ramp's name.
    #[test]
    fn the_joint_draw_spreads_over_its_range() {
        let mut buckets = [0usize; 4];
        for i in 0..400 {
            let v = joint_draw(5, (i % 20, i / 20), (i % 20 + 1, i / 20));
            assert!((0.0..1.0).contains(&v), "joint_draw returned {v}");
            buckets[(v * 4.0) as usize] += 1;
        }
        assert!(buckets.iter().all(|&b| b > 40), "joint_draw is lumpy: {buckets:?}");
    }
}
