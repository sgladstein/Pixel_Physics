//! Worldgen-private noise primitives.
//!
//! Deliberately **not** the `noise` crate. Three reasons, in the order they
//! mattered:
//!
//! 1. The dependency list is seven crates and stays that way by choice; this
//!    is ~130 lines of arithmetic with no upstream to track.
//! 2. Determinism is same-build (`PLAN.md`), so a third-party generator
//!    changing its lattice under a version bump is churn with no upside — the
//!    world would silently reshape on a `cargo update`.
//! 3. The caves follow-on needs Musgrave's ridged multifractal *as corrected
//!    in* `Reports/worldgen-design.md` §7. The `noise` crate ships the
//!    `1 - |fBm|` simplification that report explicitly calls wrong, so the
//!    dependency would have to be worked around exactly where it was supposed
//!    to help.
//!
//! Everything here is a pure function of `(seed, purpose, position)`:
//! stateless, order-independent, and identical however the caller iterates.
//! That is what lets worldgen become per-chunk later (`worldgen(seed, coord,
//! age)`, design doc §4) without any of these values changing — a chunk
//! generated alone must agree with the same chunk generated as part of a
//! whole world, and only position-indexed hashing gives that for free.

/// Which consumer a noise draw belongs to.
///
/// Every distinct use gets its own stream. Omitting this tag is the exact
/// mistake behind Noita's two shipped worldgen bugs
/// (`Reports/prior-art-worldgen-slicing.md` §6.3): without it, two features
/// that happen to sample the same coordinate are perfectly correlated, so
/// (for example) every soil-depth bump would sit on a height bump. The
/// engine's own `seed_from_coord` (`sim/chunk.rs`) is position-indexed but
/// has no purpose tag — it is left alone deliberately (it seeds *simulation*
/// streams, and the determinism suite pins its behaviour), and this is the
/// worldgen-side equivalent that does have one.
///
/// Discriminants are explicit and must never be renumbered: changing one
/// reshapes every world generated from a given seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum Purpose {
    /// Mid-frequency surface hills.
    Height = 1,
    /// Domain warp applied to the height sample position.
    Warp = 2,
    /// Fine surface detail.
    Detail = 3,
    /// Where terracing is allowed to appear.
    Mask = 4,
    /// Soil blanket thickness variation.
    Soil = 5,
    /// Reserved for soil-internal texture (unused until the soil pass grows
    /// a second noise term; kept numbered so later use cannot renumber the
    /// tags above it).
    SoilNoise = 6,
    /// Sand/gravel lens placement, and per-feature coin flips.
    Pocket = 7,
    /// Per-cell shade jitter.
    Shade = 8,
    /// Sedimentary banding in stone.
    Strata = 9,
    /// Bedrock band thickness.
    Bedrock = 10,
    /// Plant scatter.
    Life = 11,
    /// Dithered material transitions.
    Dither = 12,
    /// Regional layout: where the places of a world are and what kind they
    /// are.
    Region = 13,
    /// Dune crests in arid country.
    Dune = 14,
}

/// SplitMix64-style finalizer over `(seed, purpose, x, y)`.
///
/// Not a stream RNG: there is no state and no call order. Two callers asking
/// for the same coordinate get the same answer whenever they ask, which is
/// the property the whole module is built on.
pub fn hash(seed: u64, purpose: Purpose, x: i32, y: i32) -> u64 {
    let mut z = seed
        ^ (purpose as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (x as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (y as i64 as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Uniform in `[0, 1)`.
///
/// Takes the top 24 bits, which is every bit `f32` can represent exactly —
/// using the low bits of a hash is the classic way to reintroduce structure
/// that the finalizer just removed.
pub fn unit(seed: u64, purpose: Purpose, x: i32, y: i32) -> f32 {
    (hash(seed, purpose, x, y) >> 40) as f32 / (1u64 << 24) as f32
}

/// 1D value noise in `[0, 1)`: a lattice of `unit` samples, smoothstep-
/// interpolated.
///
/// Value rather than gradient (Perlin) noise on purpose. Gradient noise buys
/// a visibly better *isotropic* field in 2D and 3D; in 1D — which is all the
/// surface heightfield needs — the difference is a slightly different
/// spectrum for twice the arithmetic, and the terrain shape here is
/// dominated by the octave weights and the domain warp, not by the lattice
/// kernel.
pub fn value_1d(seed: u64, purpose: Purpose, x: f32) -> f32 {
    let x0f = x.floor();
    let t = x - x0f;
    let x0 = x0f as i32;
    let a = unit(seed, purpose, x0, 0);
    let b = unit(seed, purpose, x0 + 1, 0);
    // Smoothstep rather than linear: linear interpolation leaves a visible
    // crease at every lattice point, which on a heightfield reads as
    // regularly spaced kinks in the skyline.
    a + (b - a) * (t * t * (3.0 - 2.0 * t))
}

/// 1D fractional Brownian motion in `[0, 1)`.
///
/// `x` is expected to be pre-divided by the caller's wavelength, so an octave
/// count is the only frequency knob here. Amplitude halves and frequency
/// doubles per octave (gain 0.5, lacunarity 2.0) — the standard cascade, not
/// exposed as parameters because nothing in this milestone wants to vary
/// them and `Reports/design-philosophy.md` §2a keeps internal structure as
/// `const`s rather than data until someone has a reason to tune it.
pub fn fbm_1d(seed: u64, purpose: Purpose, x: f32, octaves: u32) -> f32 {
    let mut sum = 0.0f32;
    let mut amp = 1.0f32;
    let mut freq = 1.0f32;
    let mut norm = 0.0f32;
    for i in 0..octaves {
        // A distinct sub-seed per octave. Without it every octave samples the
        // same lattice at different scales, so they align at the origin and
        // at every power-of-two coordinate — visible as a repeating feature
        // at a fixed position regardless of seed.
        let s = seed.wrapping_add((i as u64 + 1).wrapping_mul(0xA076_1D64_78BD_642F));
        sum += amp * value_1d(s, purpose, x * freq);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    if norm > 0.0 {
        sum / norm
    } else {
        0.0
    }
}

/// [`fbm_1d`] centred to `[-1, 1)`, which is what every amplitude-scaled
/// term wants — an uncentred octave biases the whole surface upward by half
/// its amplitude.
pub fn fbm_1d_c(seed: u64, purpose: Purpose, x: f32, octaves: u32) -> f32 {
    fbm_1d(seed, purpose, x, octaves) * 2.0 - 1.0
}

/// The standard clamped Hermite ramp: 0 below `edge0`, 1 above `edge1`, and
/// a smooth S between them.
///
/// Used to gate features on a noise value without producing a hard on/off
/// boundary — a bare threshold puts a visible seam wherever the noise
/// crosses it.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if (edge1 - edge0).abs() < f32::EPSILON {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_pure_and_order_independent() {
        // The property the whole module rests on: no state, no call order.
        let a = hash(7, Purpose::Height, 12, -3);
        for _ in 0..4 {
            let _ = hash(99, Purpose::Soil, 5, 5);
        }
        assert_eq!(a, hash(7, Purpose::Height, 12, -3));
    }

    #[test]
    fn purpose_decorrelates_identical_coordinates() {
        // Without the purpose tag these would be equal, and every feature
        // keyed on the same coordinate would move together.
        assert_ne!(
            hash(1, Purpose::Height, 40, 0),
            hash(1, Purpose::Soil, 40, 0)
        );
    }

    #[test]
    fn unit_stays_in_range() {
        for x in -200..200 {
            let v = unit(0xABCD, Purpose::Shade, x, x * 3);
            assert!((0.0..1.0).contains(&v), "unit out of range at {x}: {v}");
        }
    }

    #[test]
    fn fbm_stays_in_range_and_varies() {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for i in 0..500 {
            let v = fbm_1d(42, Purpose::Height, i as f32 / 17.0, 4);
            assert!((0.0..1.0).contains(&v), "fbm out of range: {v}");
            min = min.min(v);
            max = max.max(v);
        }
        // A constant would pass the range check; this is the sanity-check
        // against a normalisation bug that flattens the output.
        assert!(max - min > 0.3, "fbm barely varies: {min}..{max}");
    }

    #[test]
    fn value_noise_is_continuous_across_lattice_points() {
        // Sampled either side of an integer, the values must be close --
        // a discontinuity here shows up as a cliff every cell in the terrain.
        for lattice in [-3, 0, 5, 61] {
            let a = value_1d(9, Purpose::Detail, lattice as f32 - 0.001);
            let b = value_1d(9, Purpose::Detail, lattice as f32 + 0.001);
            assert!((a - b).abs() < 0.01, "discontinuity at {lattice}: {a} vs {b}");
        }
    }

    #[test]
    fn different_seeds_give_different_fields() {
        let a: Vec<f32> = (0..50).map(|i| fbm_1d(1, Purpose::Height, i as f32 / 9.0, 3)).collect();
        let b: Vec<f32> = (0..50).map(|i| fbm_1d(2, Purpose::Height, i as f32 / 9.0, 3)).collect();
        assert_ne!(a, b);
    }

    #[test]
    fn smoothstep_clamps_and_ramps() {
        assert_eq!(smoothstep(0.2, 0.8, 0.0), 0.0);
        assert_eq!(smoothstep(0.2, 0.8, 1.0), 1.0);
        let mid = smoothstep(0.2, 0.8, 0.5);
        assert!((mid - 0.5).abs() < 1e-5, "midpoint should be 0.5, got {mid}");
    }
}
