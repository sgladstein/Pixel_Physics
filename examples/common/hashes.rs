//! **The bit-identity digests every performance harness prints**, so two
//! harnesses compare the same quantities (the performance threads' shared
//! benchmark, 2026-10-21: a speed-up must reproduce existing runs bit for
//! bit). Included by path (`#[path = "common/hashes.rs"] mod hashes;`)
//! rather than through `common/mod.rs`, which carries the plant scene.
//!
//! Three digests, because each is blind where another sees:
//! - [`world`]: the grid, as `antcost`'s and `lab_cost`'s `world_hash` hash
//!   it, so the three compare.
//! - [`field`]: `field::field_hash`. A field-only change cannot move the grid
//!   for many frames.
//! - [`organisms`]: every live organism's and brood item's own state. It
//!   moves on the tick a decision differs -- a relabelled colony, a nudged
//!   brain state, an energy bill a bit off -- long before the grid does.

#![allow(dead_code)]

use pixel_physics::sim::world::World;

fn fnv1a(h: u64, v: u64) -> u64 {
    (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
}

/// The grid inside the world's bounds (the whole loaded grid's bounds when
/// it has none set is not this function's job: every bed it is used on is
/// bounded).
pub fn world(w: &World) -> u64 {
    let b = w.bounds().expect("hashes::world needs a bounded world");
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            h = fnv1a(h, c.material.0 as u64);
            h = fnv1a(h, c.aux() as u64);
            h = fnv1a(h, c.organism_id() as u64);
        }
    }
    h
}

pub fn field(w: &World) -> u64 {
    pixel_physics::sim::field::field_hash(w)
}

/// Every live organism and brood item, in id order: identity, group, body,
/// energy, water, traits and brain state, floats by their bits.
pub fn organisms(w: &World) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut ids = w.live_organism_ids();
    ids.extend(w.live_brood_ids());
    ids.sort_unstable();
    for id in ids {
        let Some(s) = w.organism(id) else { continue };
        for v in [id as u64, s.species.0 as u64, s.colony as u64, s.lineage as u64, s.born_frame, s.heading as u64] {
            h = fnv1a(h, v);
        }
        for &(x, y) in &s.chain {
            h = fnv1a(h, ((x as u32 as u64) << 32) | y as u32 as u64);
        }
        h = fnv1a(h, s.cells.len() as u64);
        for f in [s.energy, s.made, s.gnawed, s.phero_a_mem, s.water] {
            h = fnv1a(h, f.to_bits() as u64);
        }
        for f in s.traits.iter().chain(s.brain_state.iter()) {
            h = fnv1a(h, f.to_bits() as u64);
        }
    }
    h
}

/// The one line every harness prints: `world .. field .. organisms ..`.
pub fn line(w: &World) -> String {
    format!("world {:016x} field {:016x} organisms {:016x}", world(w), field(w), organisms(w))
}
