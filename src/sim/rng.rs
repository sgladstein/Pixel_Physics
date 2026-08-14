//! A tiny xorshift64* generator.
//!
//! The simulation calls this several times per cell per frame, so it needs to be
//! trivially cheap. It is deliberately not a dependency: quality beyond "looks
//! unbiased" buys nothing here, and owning it avoids churn in `rand`'s API.
//!
//! **Determinism is required** (same-build), per `PLAN.md` — this doc used to
//! say the opposite, because the decision was originally "not required" and was
//! later reversed; `CLAUDE.md` flags the stale comments the reversal left
//! behind, and this was one of them. What that requirement does *not* by itself
//! give you is independence: a single shared generator is deterministic and
//! still couples every caller to every other caller's draw count. See `stream`.

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // A zero state is a fixed point for xorshift, so substitute a nonzero constant.
        Self(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A coin flip. Used to break the left/right symmetry of falling material.
    #[inline]
    pub fn flip(&mut self) -> bool {
        self.next_u64() & (1 << 33) != 0
    }

    /// Uniform in `0..n`. Returns 0 when `n == 0`.
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        // Use the high bits: the low bits of xorshift are the weakest.
        ((self.next_u64() >> 32) as u32) % n
    }

    /// True with probability `p`, clamped to `0.0..=1.0`.
    #[inline]
    pub fn chance(&mut self, p: f32) -> bool {
        if p <= 0.0 {
            return false;
        }
        if p >= 1.0 {
            return true;
        }
        ((self.next_u64() >> 40) as f32 / (1u64 << 24) as f32) < p
    }
}

impl Default for Rng {
    fn default() -> Self {
        Self::new(0x243F_6A88_85A3_08D3)
    }
}

/// A generator whose entire sequence is a pure function of the four values
/// identifying *who is drawing* — nothing else in the world can shift it.
///
/// **The problem this solves is independence, not determinism.** `World::rng`
/// is a single shared stream, and `plant.rs` and `creature.rs` both draw from
/// it. That is perfectly deterministic and still means every organism's
/// sequence depends on how many draws every *other* organism, decay event and
/// explosion made first — so planting a second tree silently changes the first
/// one's growth, and `plant.rs`'s own `two_trees_grown_from_the_same_seed_
/// differ` test relies on exactly that coupling to produce its difference.
///
/// That makes side-by-side comparison unsound, which matters because
/// `examples/debug_tree_variants.rs` plants six parameter variants in one scene
/// and compares them, and it is the harness the whole resource economy is
/// tuned with. Six entangled single runs cannot separate "this parameter is
/// better" from "this variant drew luckier numbers because of where it sat in
/// the draw order."
///
/// **Note both research reports name the wrong culprit.**
/// `plant-simulation-research.md` §7d and `population-dynamics-research.md` §7d
/// both attribute this to `Chunk::rng` being seeded from chunk coordinates,
/// making position a hidden inherited variable. Organisms and creatures never
/// touch `Chunk::rng` — it is reached only by the CA sweep through
/// `CellSurface::rng()`. The recommendation (a per-organism stream) is right;
/// the mechanism is order coupling, not position.
///
/// Position stays *in* the seed deliberately: two identical genomes planted in
/// different places should still grow differently, which is the property
/// `two_trees_grown_from_the_same_seed_differ` is really asserting. What
/// changes is that the difference now comes from where they are rather than
/// from what else happens to exist.
///
/// Mixing is splitmix64's finalizer, which is the standard cheap way to turn
/// correlated inputs (small, adjacent integers — exactly what cell coordinates
/// and organism ids are) into well-separated states. `Rng::new`'s own
/// zero-state guard still applies on top.
pub fn stream(a: u64, b: u64, c: u64, d: u64) -> Rng {
    let mut h = a
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(b.wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add(c.wrapping_mul(0x94D0_49BB_1331_11EB))
        .wrapping_add(d.wrapping_mul(0xD6E8_FEB8_6659_FD93));
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    Rng::new(h)
}

/// A stable pseudo-random value in `0.0..1.0` for a world position.
///
/// Used wherever a *decision* must come out the same every frame. Drawing from
/// the live generator instead would let a cell move only on the frames the dice
/// agreed — and if a chunk happened to settle on a frame they did not, it would
/// sleep with material still able to move and freeze it there. Keying on
/// position makes "this cell cannot move" a property of the world rather than
/// of the moment it was asked.
#[inline]
pub fn jitter(x: i32, y: i32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add((y as u32).wrapping_mul(0x85EB_CA6B));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    // Top 24 bits, so the result does not depend on the weakest low bits.
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// `jitter`'s per-*cell* counterpart: hashes a single byte, so a value that
/// travels with a cell (`Cell::shade`, which `CellSurface::move_cell` carries
/// to the destination) produces grain that moves with the material instead of
/// staying nailed to the screen. See `render::GrainMode::Cell`.
///
/// One byte is only 256 distinct grain levels, against `jitter`'s 2^24. That
/// is ample for a brightness wobble whose whole amplitude is 12%, and it is
/// the entropy actually available: `Cell::shade` is the only per-cell field
/// that survives a move and is not already carrying meaning.
pub fn jitter_u8(v: u8) -> f32 {
    let mut h = (v as u32).wrapping_mul(0x9E37_79B9).wrapping_add(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// Same shape as `jitter`, extended to a third input — `render.rs`'s fire
/// flicker uses this with a position plus a coarse time bucket, so the
/// result is stable *within* a bucket (no re-randomizing every single
/// frame, which at 60fps reads as noise rather than flicker) but changes
/// deterministically from one bucket to the next, still with no per-cell
/// state to maintain.
#[inline]
pub fn jitter3(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add((y as u32).wrapping_mul(0x85EB_CA6B))
        .wrapping_add((z as u32).wrapping_mul(0xC2B2_AE35));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

#[cfg(test)]
mod jitter_tests {
    use super::{jitter, jitter3};

    #[test]
    fn jitter_is_stable_for_a_position() {
        assert_eq!(jitter(17, -42), jitter(17, -42));
    }

    #[test]
    fn jitter_stays_in_range_and_varies_across_positions() {
        let mut sum = 0.0;
        let mut distinct = std::collections::HashSet::new();
        for y in -40..40 {
            for x in -40..40 {
                let j = jitter(x, y);
                assert!((0.0..1.0).contains(&j), "jitter({x}, {y}) = {j}");
                sum += j;
                distinct.insert(j.to_bits());
            }
        }
        let mean = sum / (80.0 * 80.0);
        assert!((0.4..0.6).contains(&mean), "mean {mean} is not centred");
        // Neighbouring positions must not collapse onto the same value, or
        // whole regions would share a reach and the slope would band.
        assert!(distinct.len() > 6000, "only {} distinct values", distinct.len());
    }

    #[test]
    fn jitter3_is_stable_within_a_bucket_but_varies_across_buckets() {
        assert_eq!(jitter3(17, -42, 5), jitter3(17, -42, 5), "same (x, y, z) should be stable");
        let mut distinct = std::collections::HashSet::new();
        for z in 0..40 {
            let j = jitter3(17, -42, z);
            assert!((0.0..1.0).contains(&j), "jitter3(17, -42, {z}) = {j}");
            distinct.insert(j.to_bits());
        }
        assert!(distinct.len() > 30, "the same position barely varied across 40 time buckets: {} distinct", distinct.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_seed_still_produces_varied_output() {
        let mut rng = Rng::new(0);
        let a = rng.next_u64();
        let b = rng.next_u64();
        assert_ne!(a, 0);
        assert_ne!(a, b);
    }

    #[test]
    fn below_stays_in_range_and_handles_zero() {
        let mut rng = Rng::default();
        assert_eq!(rng.below(0), 0);
        for _ in 0..1000 {
            assert!(rng.below(7) < 7);
        }
    }

    #[test]
    fn flip_is_roughly_balanced() {
        let mut rng = Rng::default();
        let heads = (0..10_000).filter(|_| rng.flip()).count();
        // A fair coin over 10k trials lands well inside this window; a stuck bit would not.
        assert!((4500..5500).contains(&heads), "heads = {heads}");
    }

    /// The property `stream` exists for: an organism's sequence must not
    /// depend on anything but its own identity. Asserted as "two different
    /// identities give different sequences, and the same identity gives the
    /// same one" — the second half is what makes determinism survive, the
    /// first is what makes side-by-side comparison sound.
    #[test]
    fn a_stream_is_a_pure_function_of_its_own_identity() {
        let draw = |a, b, c, d| {
            let mut r = stream(a, b, c, d);
            (0..8).map(|_| r.next_u64()).collect::<Vec<_>>()
        };
        assert_eq!(draw(7, 100, 99, 450), draw(7, 100, 99, 450), "the same identity must replay the same sequence");
        assert_ne!(draw(7, 100, 99, 450), draw(8, 100, 99, 450), "a different organism must draw differently");
        assert_ne!(draw(7, 100, 99, 450), draw(7, 101, 99, 450), "a different cell must draw differently");
        assert_ne!(draw(7, 100, 99, 450), draw(7, 100, 99, 495), "a different tick must draw differently");
    }

    /// A stream stays well-distributed along the axis it is actually
    /// advanced on. Organism cells tick every `ORGANISM_TICK_INTERVAL`
    /// frames, so the frame input moves in a fixed stride of 45 and nothing
    /// else changes — a mixer that handled random inputs but correlated on
    /// an arithmetic sequence would look fine in isolation and quietly bias
    /// every growth roll in the engine.
    ///
    /// Both tails are checked. `moss.ron`'s two division chances differ by
    /// more than two orders of magnitude (`damp_chance: 0.35`,
    /// `dry_chance: 0.002`), so a mixer that was uniform in the middle and
    /// short in the low tail would leave damp moss working and dry moss
    /// permanently frozen — a bug that would present as a behaviour
    /// question about moisture, nowhere near the generator.
    #[test]
    fn a_stream_stays_uniform_along_a_fixed_tick_stride() {
        let (mut common, mut rare, mut n) = (0u32, 0u32, 0u32);
        for cell in 0..200u64 {
            for tick in 0..400u64 {
                let mut r = stream(7, 100 + cell, 99, 45 * tick);
                let _ = r.below(4); // Divide draws a candidate first; chance is the *second* draw
                if r.chance(0.35) {
                    common += 1;
                }
                let mut r = stream(7, 100 + cell, 99, 45 * tick);
                let _ = r.below(4);
                if r.chance(0.002) {
                    rare += 1;
                }
                n += 1;
            }
        }
        let (common_rate, rare_rate) = (common as f32 / n as f32, rare as f32 / n as f32);
        assert!((0.33..0.37).contains(&common_rate), "chance(0.35) fired at {common_rate}");
        assert!((0.001..0.004).contains(&rare_rate), "chance(0.002) fired at {rare_rate}");
    }
}
