//! Dumps every organism-owned cell's per-cell channels for a grown tree.
//!
//! The companion to `filmstrip`'s organism overlay, and deliberately second
//! to it: the sheet answers *what and where* (canopy density reads as the
//! ramp floor on every cell of every tile), and this answers *how much* and
//! *why*. Written because the sheet's answer — "zero everywhere, always" —
//! is the same picture a genuinely-zero channel and a channel that is
//! merely decaying faster than it is deposited would both produce, and
//! those need different fixes.
//!
//! ```text
//! cargo run --release --example plant_probe -- frames=200
//! ```

use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::organism;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{material, parallel};

const WIDTH: i32 = 512;
const HEIGHT: i32 = 320;
/// Matches `filmstrip`'s `TREE_GROUND_Y` — see that constant's doc for why
/// the depth is pinned to `field.rs`'s light profile rather than chosen.
const GROUND_Y: i32 = 40;

fn main() {
    let frames: u64 = std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix("frames=").map(|v| v.parse().expect("frames")))
        .unwrap_or(400);

    let mut w = World::new(Rect::new(0, 0, WIDTH - 1, HEIGHT - 1));
    for x in 0..WIDTH {
        for y in GROUND_Y..(GROUND_Y + 6) {
            w.set(x, y, Cell::new(material::STONE, 0));
        }
    }
    w.plant_tree(200, GROUND_Y - 1);

    let mut awake_frames = 0u64;
    for _ in 0..frames {
        parallel::step(&mut w);
        w.step_active_sites();
        w.step_fields();
        if w.active_chunk_count() > 0 {
            awake_frames += 1;
        }
    }

    println!("after {frames} frames: {} active sites, {} awake chunks", w.active_site_count(), w.active_chunk_count());
    println!(
        "chunks were awake on {awake_frames}/{frames} frames ({:.1}%) -- this is how often `diffuse_resource` ran at all, \
since it is dispatched from the CA sweep and the sweep skips settled chunks",
        100.0 * awake_frames as f32 / frames as f32
    );

    let mut cells = Vec::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let c = w.get(x, y);
            if c.organism_id() == 0 {
                continue;
            }
            let (ty, resource) = organism::unpack_aux(c.aux());
            cells.push((x, y, ty, resource, organism::canopy_density(c.aux())));
        }
    }

    println!("\n{} organism cells", cells.len());
    println!("{:>5} {:>5}  {:<12} {:>9} {:>9}", "x", "y", "type", "resource", "canopy");
    for (x, y, ty, resource, canopy) in &cells {
        println!("{x:>5} {y:>5}  {:<12} {resource:>9.3} {canopy:>9.3}", format!("{ty:?}"));
    }

    let max_canopy = cells.iter().map(|c| c.4).fold(0.0f32, f32::max);
    let max_resource = cells.iter().map(|c| c.3).fold(0.0f32, f32::max);
    println!("\nmax resource {max_resource:.3} / {:.1}", organism::RESOURCE_SCALE);
    println!("max canopy   {max_canopy:.3} / {:.1}", organism::CANOPY_DENSITY_SCALE);
    // `with_canopy_density` packs into 4 bits, so 15 steps span the scale --
    // the number to compare the decay ladder above against, and the concrete
    // version of `plant-substrate-v2-design.md` §3a's claim that this channel
    // is quantization-limited rather than behaviour-limited.
    println!(
        "one quantization step of canopy density is {:.3} (4 bits, 15 steps)",
        organism::CANOPY_DENSITY_SCALE / 15.0
    );
}
