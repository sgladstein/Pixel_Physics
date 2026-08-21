//! Does the gnome weave *through* a formation, or get sliced *by* it?
//!
//! Built for the round-6 formation respec, which takes speleothems from one
//! cell wide to 3-8 at the base. At one cell wide, "which side of this
//! formation is he on" and "which side of this column is he on" are the same
//! question, and the depth key was the world column. They stop being the same
//! question at width two: `TreeDepth::in_front` decorrelates adjacent keys by
//! design, so a run of columns never agrees with itself, and he rendered as
//! vertical stripes -- some of his columns drawn over the stone, the rest
//! hidden behind it. Measured over 4000 columns, the fraction of a `w`-wide
//! formation that put him wholly on one side was 100% at w=1, 48% at w=2 and
//! **0% from w=3 up**.
//!
//! `render.rs`'s `a_wide_formation_puts_the_gnome_wholly_in_front_or_wholly_
//! behind` is the guard; this is the picture, because a test can only say
//! *whether* the frames agree and the thing being judged is whether the weave
//! reads as depth at all.
//!
//! ```text
//! cargo run --release --example gnome_depth
//! cargo run --release --example gnome_depth -- zoom=6 depth=weave
//! ```
//!
//! One row per formation width; within a row the formation slides across him
//! a cell at a time, so a key that is stable for *some* alignments and not
//! others shows up as a row that is fine at one end and shredded at the other.

use pixel_physics::render::{Renderer, TreeDepth};
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player::Player;
use pixel_physics::sim::world::World;
use pixel_physics::sim::Cell;
use std::collections::HashSet;

const WIDTHS: [i32; 5] = [1, 2, 3, 5, 8];
/// Where the formation's left edge sits, relative to his own left edge. The
/// span covers "clear to his left", "over him", "clear to his right".
const OFFSETS: std::ops::Range<i32> = -4..8;

fn main() {
    let mut zoom = 4usize;
    let mut depth = TreeDepth::Weave;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "zoom" => zoom = v.parse().expect("zoom is a number"),
            "depth" => {
                depth = match v {
                    "front" => TreeDepth::Front,
                    "behind" => TreeDepth::Behind,
                    "haze" => TreeDepth::Haze,
                    _ => TreeDepth::Weave,
                }
            }
            _ => eprintln!("ignoring unknown argument {arg}"),
        }
    }

    // One tile is a whole little world, rendered at 1:1 and then blown up, so
    // what lands on the sheet is the renderer's own pixels rather than a
    // resample of them -- a half-drawn sprite column must stay one column
    // wide on the sheet or the artifact being hunted is smoothed away.
    let (tw, th) = (28u32, 26u32);
    let (cols, rows) = (OFFSETS.len() as u32, WIDTHS.len() as u32);
    let (sw, sh) = (cols * tw * zoom as u32, rows * th * zoom as u32);
    let mut sheet = vec![0u8; (sw * sh * 4) as usize];

    let mut sliced = 0usize;
    let mut occluding = 0usize;
    for (row, formation_w) in WIDTHS.iter().enumerate() {
        for (col, offset) in OFFSETS.enumerate_offsets() {
            let mut world = World::new(Rect::new(0, 0, tw as i32 - 1, th as i32 - 1));
            let flowstone = world.materials.id_of("flowstone").expect("flowstone is compiled in");
            for x in 0..tw as i32 {
                world.set(x, th as i32 - 1, Cell::new(material::STONE, 0));
            }
            let player = Player::at(tw as i32 / 2, th as i32 - 8);
            let (ox, _oy) = player.rect_origin();
            for y in 1..th as i32 - 1 {
                for x in ox + offset..ox + offset + formation_w {
                    world.set(x, y, Cell::new(flowstone, 0));
                }
            }
            world.end_step();
            world.player = Some(player);

            let particles = ParticleSystem::new();
            let frame = |d: TreeDepth| {
                let mut r = Renderer::new();
                r.tree_depth = d;
                let mut buf = vec![0u8; (tw * th * 4) as usize];
                r.draw(&world, &particles, &HashSet::new(), &mut buf, (tw, th), true);
                buf
            };
            // The counter next to the picture: `Front` never occludes and
            // `Behind` always does, so those two frames are the only two
            // uniform outcomes. A weave frame matching neither is a gnome cut
            // in half, which at this zoom is genuinely easy to misread as
            // sprite detail.
            let (over, under) = (frame(TreeDepth::Front), frame(TreeDepth::Behind));
            let shown = frame(depth);
            if over != under {
                occluding += 1;
                if shown != over && shown != under {
                    sliced += 1;
                }
            }

            for y in 0..th {
                for x in 0..tw {
                    let px = ((y * tw + x) * 4) as usize;
                    for by in 0..zoom as u32 {
                        for bx in 0..zoom as u32 {
                            let sx = col as u32 * tw * zoom as u32 + x * zoom as u32 + bx;
                            let sy = row as u32 * th * zoom as u32 + y * zoom as u32 + by;
                            let dst = ((sy * sw + sx) * 4) as usize;
                            sheet[dst..dst + 4].copy_from_slice(&shown[px..px + 4]);
                        }
                    }
                }
            }
        }
    }

    let out = "target/filmstrips/gnome_depth.png";
    std::fs::create_dir_all("target/filmstrips").expect("filmstrip directory");
    image::save_buffer(out, &sheet, sw, sh, image::ColorType::Rgba8).expect("writing the sheet");
    println!(
        "rows are formation widths {WIDTHS:?}, columns slide it across him; \
         {occluding} of {} tiles put stone over him, {sliced} of those cut him in half",
        (cols * rows) as usize
    );
    println!("contact sheet ({sw}x{sh}): {out}");
}

/// `Range<i32>` is not `Clone`-cheap to enumerate twice in a nested loop
/// without moving it, and spelling the index out at the call site read worse
/// than naming the intent.
trait Offsets {
    fn enumerate_offsets(&self) -> Vec<(usize, i32)>;
}
impl Offsets for std::ops::Range<i32> {
    fn enumerate_offsets(&self) -> Vec<(usize, i32)> {
        self.clone().enumerate().collect()
    }
}
