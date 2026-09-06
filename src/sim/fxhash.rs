//! A deterministic, fast hasher for the engine's hottest keyed containers,
//! and type aliases for the maps and sets built on it.
//!
//! `RandomState` (std's default) seeds itself per process from OS entropy —
//! good for hash-flooding resistance, useless here, since nothing in this
//! engine is exposed to adversarial input, and actively costly, since every
//! `ChunkCoord`/`(i32, i32)` lookup pays a full SipHash round for keys that
//! are two or three `i32`s. A `perf` profile of the evolution lab tick
//! (`examples/lab_cost`, tree bed, 32k frames) showed ~14% of all CPU inside
//! `RandomState::hash_one`/`Sip13Rounds::write` and ~5.6% in `World::get`
//! itself, which is one such lookup per cell read. FxHash (the algorithm
//! rustc and Firefox use for exactly this reason) is a handful of
//! multiply-rotate-xor steps instead.
//!
//! This is safe to swap in because it was already checked, not because it is
//! being newly assumed: `Reports/open-bugs-handoff.md`'s frame-cost audit
//! (~line 5274) walked every hash-container iteration in `src/` and found
//! them all already order-safe — membership-only (`pending_*`), sorted
//! immediately after collection (organism `cells`, `rigid.rs`'s
//! `remaining`, the field's `awake`/`read`), or order-independent by
//! construction (counts, dirty-rect unions, writes to disjoint flat
//! indices) — and says a fixed-seed hasher "is a reasonable change for
//! *speed*". `tests/determinism.rs` passing today with std's per-instance
//! seeding is the same evidence from the other direction: if iteration order
//! mattered anywhere live, that test would already be flaky.
//!
//! And unlike `RandomState`, `FxBuild` is **deterministic across runs** —
//! same seed (there isn't one; the multiplier is a constant) every time,
//! same process or not — which is one fewer source of nondeterminism to
//! reason about, not one more, should anything downstream ever come to
//! depend on iteration order by accident.

use std::hash::Hasher;

/// FxHash: rotate-xor-multiply, folding 8 bytes at a time. The constant is
/// the golden-ratio-derived odd multiplier rustc and Firefox both use; its
/// only job is to scatter bits so nearby keys (`ChunkCoord { x: 3, y: 4 }`
/// next to `{ x: 3, y: 5 }`, which is the overwhelming majority of the
/// traffic here) land in different buckets.
const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

#[derive(Default)]
pub struct FxHasher {
    hash: u64,
}

impl FxHasher {
    #[inline]
    fn add(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, mut bytes: &[u8]) {
        while bytes.len() >= 8 {
            let (chunk, rest) = bytes.split_at(8);
            self.add(u64::from_ne_bytes(chunk.try_into().unwrap()));
            bytes = rest;
        }
        if bytes.len() >= 4 {
            let (chunk, rest) = bytes.split_at(4);
            self.add(u32::from_ne_bytes(chunk.try_into().unwrap()) as u64);
            bytes = rest;
        }
        for &b in bytes {
            self.add(b as u64);
        }
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add(i as u64);
    }
    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.add(i as u64);
    }
    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.add(i as u64);
    }
    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.add(i);
    }
    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add(i as u64);
    }
    #[inline]
    fn write_i32(&mut self, i: i32) {
        self.add(i as u32 as u64);
    }
    #[inline]
    fn write_i64(&mut self, i: i64) {
        self.add(i as u64);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

pub type FxBuild = std::hash::BuildHasherDefault<FxHasher>;
pub type FxHashMap<K, V> = std::collections::HashMap<K, V, FxBuild>;
pub type FxHashSet<K> = std::collections::HashSet<K, FxBuild>;

/// The engine's two hottest key shapes, named for what they hold rather than
/// how they hash — `ChunkMap<FieldTile>` reads at the call site the way
/// `HashMap<ChunkCoord, FieldTile>` used to, just cheaper.
pub type ChunkMap<V> = FxHashMap<crate::sim::chunk::ChunkCoord, V>;
pub type ChunkSet = FxHashSet<crate::sim::chunk::ChunkCoord>;
pub type PosMap<V> = FxHashMap<(i32, i32), V>;
pub type PosSet = FxHashSet<(i32, i32)>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_across_instances() {
        let mut a = FxHasher::default();
        let mut b = FxHasher::default();
        a.write(b"a mildly long key to exercise the tail path");
        b.write(b"a mildly long key to exercise the tail path");
        assert_eq!(a.finish(), b.finish());
    }

    #[test]
    fn distinguishes_nearby_chunk_coords() {
        use crate::sim::chunk::ChunkCoord;
        let mut map: ChunkMap<i32> = ChunkMap::default();
        for x in -2..=2 {
            for y in -2..=2 {
                map.insert(ChunkCoord::new(x, y), x * 10 + y);
            }
        }
        assert_eq!(map.len(), 25);
        assert_eq!(map.get(&ChunkCoord::new(1, -1)), Some(&9));
    }

    #[test]
    fn pos_set_basic() {
        let mut set: PosSet = PosSet::default();
        set.insert((3, 4));
        set.insert((3, 5));
        assert!(set.contains(&(3, 4)));
        assert!(!set.contains(&(4, 3)));
        assert_eq!(set.len(), 2);
    }
}
