//! A tiny xorshift64* generator.
//!
//! The simulation calls this several times per cell per frame, so it needs to be
//! trivially cheap. It is deliberately not a dependency: statistical quality
//! beyond "looks unbiased" buys nothing here, and owning it avoids churn in
//! `rand`'s API.
//!
//! An earlier version of this doc said the sim "does not require
//! reproducibility" — that decision was **reversed** (`PLAN.md`: same-build
//! deterministic replay is required), and this module is now load-bearing for
//! it: every seed is fixed (`Rng::default`) or position-derived
//! (`Chunk::rng`, `jitter`), never drawn from time or OS entropy, so two
//! identical runs draw identical streams. `tests/determinism.rs` asserts
//! this end to end; do not add an entropy-seeded constructor.

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
}
