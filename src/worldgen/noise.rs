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
    /// Which region palette family a cell's shade sits in.
    ///
    /// Its own stream rather than sharing `Dither`, which already decides
    /// the soil/gravel contact at the same `(x, y)`: sharing would put every
    /// palette-family change exactly on a contact boundary, which is the
    /// correlation the purpose tag exists to prevent
    /// (`Reports/prior-art-worldgen-slicing.md` §6.3).
    Palette = 15,
    /// Per-dune amplitude and slip-face fraction, keyed on the dune index.
    ///
    /// Separate from `Dune`, which places the crests: keying both on the
    /// same stream would tie a dune's height to where it happens to sit,
    /// which is the correlation the tag exists to prevent.
    DuneShape = 16,
    /// Column-scale roughening applied to terrace risers.
    Riser = 17,
    /// The slow 2-D field that makes a palette-family transition wander with
    /// depth instead of standing as a vertical pier.
    ///
    /// Its own stream rather than sharing `Palette`, which supplies the
    /// per-cell dither draw at the same `(x, y)`: sharing would correlate
    /// where the band *is* with which cells inside it flip, so the band would
    /// dither hardest exactly along its own centre line and reintroduce an
    /// edge.
    PaletteField = 18,
    /// Sealed vault placement: where a chamber sits, how big it is, and which
    /// shape it takes.
    ///
    /// Its own stream rather than sharing `Pocket`, which decides lens
    /// placement over the same rock: sharing would tie a vault's position to
    /// whether a lens happened to be drawn nearby, which is exactly the
    /// correlation the purpose tag exists to prevent.
    Vault = 19,
    /// Worley feature points for the cave-system field ([`worley_f2_f1`]).
    ///
    /// Its own stream rather than sharing `Vault`, which draws the system's
    /// placement and per-system coin flips over small integer coordinates:
    /// the Worley lattice indices are small integers too, so sharing would
    /// correlate where a system sits with the shape it takes inside.
    Cave = 20,
    /// Cave floors: gravel fill thickness, breakdown mounds, and the
    /// per-system waterline draw.
    ///
    /// Separate from `Cave`, whose lattice coordinates overlap the same
    /// small-integer range as floor-segment representatives — sharing would
    /// tie how deep a floor is buried to where the chambers happen to sit.
    CaveFloor = 21,
    /// Speleothems: where a stalactite or stalagmite grows, how tall, how
    /// wide, and whether it is crystal.
    Speleothem = 22,
    /// Per-band rock hardness for plan-space erosion: one draw per strata
    /// band index, so a band is hard or soft along its whole length —
    /// which is what makes eroded ledges coherent rather than speckled
    /// (`Reports/worldgen-erosion-design.md`). 20–22 were appended by the
    /// concurrent round-3 cave branch and this by the erosion track; the
    /// two landed without colliding because each reserved its numbers in
    /// advance — keep doing that.
    Hardness = 23,
    /// Boulder-socket shape: which run of `erosion::Deposits::boulder`
    /// markers becomes one cluster, and its width, height and shade.
    ///
    /// Its own stream rather than sharing `Pocket` (lens placement over the
    /// same rock) or `Hardness` (which decided *whether* this socket exists
    /// in the first place) — sharing either would tie a boulder's drawn
    /// shape to an unrelated decision at the same coordinate, the
    /// correlation every purpose tag exists to prevent. Claimed by the
    /// round-4 data track; 24 is the next free number after `Hardness`.
    Boulder = 24,
    /// The round-5 monumental chamber: per-system half-extent draw for the
    /// one dilated room grown around a system's point of greatest
    /// clearance.
    ///
    /// Its own stream rather than sharing `Vault` (the system's placement
    /// draws) or `Cave` (the Worley lattice) — either would tie the
    /// chamber's size to an unrelated decision keyed on the same small
    /// integers, the correlation every purpose tag exists to prevent.
    /// 26 (`Drip`) and 27 (`CeilingGrain`) are reserved for later round-5
    /// tasks; appended when each lands, not claimed unused here.
    CaveChamber = 25,
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

/// 2D value noise in `[0, 1)`: the same lattice as [`value_1d`], bilinearly
/// interpolated with the same smoothstep weights.
///
/// Added for the palette-family modulation, which needs a field that varies
/// slowly in *both* directions. Everything else in this file is 1D because
/// the surface heightfield is a function of `x` alone; a shade decision is
/// not, and reusing a 1D field for it is exactly what produced the artifact
/// this exists to fix — a probability that is constant down a whole column
/// draws a full-height vertical pier of one colour however finely the
/// per-cell dither is stippled.
///
/// Value rather than gradient noise for the same reason [`value_1d`] is: the
/// axis-alignment gradient noise buys is worth paying for when the field is
/// the *shape* of something, and this one is a slow weight on a probability
/// that is then dithered per cell, which destroys any lattice signature long
/// before it reaches a pixel.
pub fn value_2d(seed: u64, purpose: Purpose, x: f32, y: f32) -> f32 {
    let (x0f, y0f) = (x.floor(), y.floor());
    let (tx, ty) = (x - x0f, y - y0f);
    let (x0, y0) = (x0f as i32, y0f as i32);
    let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let c00 = unit(seed, purpose, x0, y0);
    let c10 = unit(seed, purpose, x0 + 1, y0);
    let c01 = unit(seed, purpose, x0, y0 + 1);
    let c11 = unit(seed, purpose, x0 + 1, y0 + 1);
    let a = c00 + (c10 - c00) * sx;
    let b = c01 + (c11 - c01) * sx;
    a + (b - a) * sy
}

/// 2D fractional Brownian motion in `[0, 1)`. Same cascade as [`fbm_1d`],
/// including the per-octave sub-seed and the reason for it.
pub fn fbm_2d(seed: u64, purpose: Purpose, x: f32, y: f32, octaves: u32) -> f32 {
    let mut sum = 0.0f32;
    let mut amp = 1.0f32;
    let mut freq = 1.0f32;
    let mut norm = 0.0f32;
    for i in 0..octaves {
        let s = seed.wrapping_add((i as u64 + 1).wrapping_mul(0xA076_1D64_78BD_642F));
        sum += amp * value_2d(s, purpose, x * freq, y * freq);
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

/// Worley (cellular) noise: distances to the nearest and second-nearest
/// feature point, in lattice units, as `(f1, f2)`.
///
/// One feature point per unit lattice cell, both of its coordinates split out
/// of a single [`hash`] — two separate `unit` draws at hand-scrambled
/// coordinates is the classic way to reintroduce correlation between the two
/// axes. The neighbourhood searched is the naive 3x3 around the sample's own
/// cell, which can understate F2 near a cell corner when the true
/// second-nearest point sits two cells away; `Reports/worldgen-design.md` §7
/// accepts that for this use, because the field is thresholded, evaluated
/// once at genesis, and the error only perturbs where a passage wall sits —
/// a 5x5 search would cost 2.8x for a difference no one can see in rock.
///
/// The property the caves are built on: **`f2 - f1` is zero along the
/// boundaries of the Worley cells** (where two feature points are
/// equidistant) and grows toward each cell's centre, so thresholding it low
/// carves the boundary network — passages — and the junctions where three
/// boundaries meet open into wider bulges — chambers. One field, one
/// threshold, and the chamber-and-passage anatomy comes out of the geometry
/// rather than being drawn.
pub fn worley_f2_f1(seed: u64, purpose: Purpose, x: f32, y: f32) -> (f32, f32) {
    let (x0, y0) = (x.floor() as i32, y.floor() as i32);
    let mut f1 = f32::MAX;
    let mut f2 = f32::MAX;
    for j in -1..=1 {
        for i in -1..=1 {
            let (cx, cy) = (x0 + i, y0 + j);
            let h = hash(seed, purpose, cx, cy);
            // Top 24 bits and the next 24: each is every bit an f32 mantissa
            // holds, from disjoint parts of one finalized hash.
            let fx = (h >> 40) as f32 / (1u64 << 24) as f32;
            let fy = ((h >> 16) & 0x00FF_FFFF) as f32 / (1u64 << 24) as f32;
            let (dx, dy) = (cx as f32 + fx - x, cy as f32 + fy - y);
            let d = (dx * dx + dy * dy).sqrt();
            if d < f1 {
                f2 = f1;
                f1 = d;
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    (f1, f2)
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
    fn worley_distances_are_ordered_and_continuous() {
        // f1 <= f2 by construction, both non-negative, and f1 is bounded by
        // the lattice: the nearest feature point of a 3x3 neighbourhood is
        // never further than the diagonal of one cell from a sample inside
        // the centre cell.
        for k in 0..400 {
            let (x, y) = (k as f32 * 0.173 - 20.0, k as f32 * 0.311 - 30.0);
            let (f1, f2) = worley_f2_f1(77, Purpose::Cave, x, y);
            assert!(f1 >= 0.0 && f2 >= f1, "disordered at ({x}, {y}): {f1} {f2}");
            assert!(f1 < 1.5, "f1 {f1} further than a cell diagonal at ({x}, {y})");
        }
        // Continuity across a lattice line: the feature points either side do
        // not change when the sample crosses it, so the distances cannot jump.
        let (a1, a2) = worley_f2_f1(77, Purpose::Cave, 4.999, 2.5);
        let (b1, b2) = worley_f2_f1(77, Purpose::Cave, 5.001, 2.5);
        assert!((a1 - b1).abs() < 0.01 && (a2 - b2).abs() < 0.01);
    }

    #[test]
    fn worley_f2_f1_reaches_low_and_high() {
        // The caves threshold this difference, so it has to actually span a
        // range: near-zero somewhere (a cell boundary crosses the sampled
        // window) and well above the threshold somewhere else (a cell
        // interior). A field that never dips low carves nothing; one that
        // never rises high carves everything.
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for j in 0..60 {
            for i in 0..60 {
                let (f1, f2) = worley_f2_f1(3, Purpose::Cave, i as f32 / 8.0, j as f32 / 8.0);
                lo = lo.min(f2 - f1);
                hi = hi.max(f2 - f1);
            }
        }
        assert!(lo < 0.05, "no cell boundary found in a 7.5-cell window: min {lo}");
        assert!(hi > 0.4, "no cell interior found: max {hi}");
    }

    #[test]
    fn smoothstep_clamps_and_ramps() {
        assert_eq!(smoothstep(0.2, 0.8, 0.0), 0.0);
        assert_eq!(smoothstep(0.2, 0.8, 1.0), 1.0);
        let mid = smoothstep(0.2, 0.8, 0.5);
        assert!((mid - 0.5).abs() < 1e-5, "midpoint should be 0.5, got {mid}");
    }
}
