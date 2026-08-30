//! **A look at the lab box.** Renders `lab::scene::LabBox` through the same
//! `Renderer` the game draws with, as a contact sheet across frames, so the
//! bed can be judged by eye before anything is measured in it.
//!
//! `CLAUDE.md`'s first method rule: *look before you measure* — every metric
//! written before anyone had looked at the artifact has measured the wrong
//! thing. A hand-built scene is exactly where that bites, because a scene
//! that contradicts the code looks identical to a bug in the code.
//!
//! ```text
//! cargo run --release --example labshot
//! cargo run --release --example labshot -- frames=0,600,3000,9000 out=lab.png
//! ```

use pixel_physics::lab::scene::LabBox;
use pixel_physics::render::Renderer;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

fn main() {
    let out: String = arg("out").unwrap_or_else(|| "labshot.png".to_string());
    let stops: String = arg("frames").unwrap_or_else(|| "0,600,3000,9000".to_string());
    let stops: Vec<u64> = stops.split(',').map(|s| s.parse().expect("a frame number")).collect();
    let zoom: i32 = arg("zoom").unwrap_or(1);

    let spec = LabBox {
        width: arg("width").unwrap_or(512),
        height: arg("height").unwrap_or(320),
        soil_depth: arg("soil").unwrap_or(80),
        founders: arg("founders").unwrap_or(8),
        colonies: arg("colonies").unwrap_or(1),
        compartments: arg("walls").unwrap_or(1),
        ..LabBox::default()
    };
    println!(
        "labshot: {}x{} soil={} founders={} colonies={} walls={} frames={:?}",
        spec.width, spec.height, spec.soil_depth, spec.founders, spec.colonies,
        spec.compartments, stops
    );

    let mut world = spec.build();
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();

    let (vw, vh) = (spec.width as u32, spec.height as u32);
    let mut renderer = Renderer::new();
    for _ in 1..zoom {
        renderer.adjust_zoom(1);
    }

    let mut tiles: Vec<Vec<u8>> = Vec::new();
    let last = *stops.last().expect("at least one stop");
    let mut next = 0usize;
    for f in 0..=last {
        if next < stops.len() && stops[next] == f {
            let mut buf = vec![0u8; (vw * vh * 4) as usize];
            let touched = world.take_touched_chunks();
            renderer.draw(&world, &particles, &touched, &mut buf, (vw, vh), true);
            let ids = world.live_organism_ids();
            let cells: usize =
                ids.iter().filter_map(|id| world.organism(*id)).map(|s| s.cells.len()).sum();
            let seeds: u32 =
                ids.iter().filter_map(|id| world.organism(*id)).map(|s| s.seeds_set).sum();
            println!("  frame {f:>6}: cells {cells:>6}  orgs {:>5}  seeds {seeds:>5}", ids.len());
            tiles.push(buf);
            next += 1;
        }
        if f < last {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        }
    }

    // One column, so a tall thin bed stacks readably.
    let (sw, sh) = (vw, vh * tiles.len() as u32);
    let mut sheet = vec![0u8; (sw * sh * 4) as usize];
    for (i, tile) in tiles.iter().enumerate() {
        let y0 = i as u32 * vh;
        for y in 0..vh {
            let src = (y * vw * 4) as usize;
            let dst = ((y0 + y) * sw * 4) as usize;
            sheet[dst..dst + (vw * 4) as usize].copy_from_slice(&tile[src..src + (vw * 4) as usize]);
        }
    }
    image::save_buffer(&out, &sheet, sw, sh, image::ColorType::Rgba8).expect("writing the sheet");
    println!("wrote {out} ({sw}x{sh})");
}
