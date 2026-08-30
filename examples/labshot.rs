//! **A look at the lab box.** Renders `lab::scene::LabBox` through the same
//! `Renderer` the game draws with, as a contact sheet across frames, so the
//! bed can be judged by eye before anything is measured in it.
//!
//! `CLAUDE.md`'s first method rule: *look before you measure* — every metric
//! written before anyone had looked at the artifact has measured the wrong
//! thing. A hand-built scene is exactly where that bites, because a scene
//! that contradicts the code looks identical to a bug in the code.
//!
//! **Every stop prints its counts beside the picture**, because a picture
//! cannot say whether the thing you built is what produced it — `CLAUDE.md`'s
//! standing rule, learned when a collapse rendered as coherent falling slabs
//! was read as "chunks are working" while the body count was zero for the
//! whole run. The same trap is live in this bed twice over: a box full of
//! green that is reproducing and one that is not are the same photograph, and
//! **a founder that never germinated and one too small to draw are the same
//! photograph too**. So the per-stop line carries the ant count, the standing
//! fruit, and each founder's cell count by the id it was given before the
//! first tick — a founder whose id no longer resolves is dead, one at three
//! cells is merely invisible, and only the id can tell them apart.
//!
//! ```text
//! cargo run --release --example labshot
//! cargo run --release --example labshot -- frames=0,600,3000,9000 out=lab.png
//! cargo run --release --example labshot -- founders=8 walls=4 frames=0,3000,12000,30000
//! ```

use pixel_physics::lab::scene::LabBox;
use pixel_physics::render::Renderer;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::organism::{cell_type, CellType};
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

    // Before a single tick: every organism the scene builder placed. This is
    // the only moment the founders are distinguishable from their offspring.
    let founders = world.live_organism_ids();
    println!("  {} organism(s) placed by the builder before the first tick", founders.len());

    let mut tiles: Vec<Vec<u8>> = Vec::new();
    let last = *stops.last().expect("at least one stop");
    let mut next = 0usize;
    for f in 0..=last {
        if next < stops.len() && stops[next] == f {
            let mut buf = vec![0u8; (vw * vh * 4) as usize];
            let touched = world.take_touched_chunks();
            renderer.draw(&world, &particles, &touched, &mut buf, (vw, vh), true);
            let ids = world.live_organism_ids();
            let (mut cells, mut seeds, mut plants, mut ants) = (0usize, 0u32, 0usize, 0usize);
            for id in &ids {
                let Some(s) = world.organism(*id) else { continue };
                if world.species.get(s.species).creature.is_some() {
                    ants += 1;
                } else {
                    plants += 1;
                    cells += s.cells.len();
                    seeds += s.seeds_set;
                }
            }
            // Standing reproductive organs, counted as *cells in the grid*
            // rather than as `seeds_set`. They are not the same quantity and
            // the difference is the whole of
            // `Reports/creature-stamp-routes-2026-08-30.md` §5: seeds set is
            // a plant's own tally, standing fruit is what an ant can walk to.
            let (mut fruit, mut flower) = (0usize, 0usize);
            for y in 0..spec.height {
                for x in 0..spec.width {
                    let c = world.get(x, y);
                    if c.organism_id() == 0 {
                        continue;
                    }
                    match cell_type(c.aux()) {
                        Some(CellType::Fruit) => fruit += 1,
                        Some(CellType::Flower) => flower += 1,
                        _ => {}
                    }
                }
            }
            let st = world.creature_stats;
            let (alloc, _) = world.organism_slot_usage();
            println!(
                "  frame {f:>6}: plants {plants:>4} cells {cells:>6} seeds {seeds:>5} \
                 fruit {fruit:>4} flower {flower:>4} | ants {ants:>4} births {:>4} deaths {:>4} \
                 | slots {alloc:>4}/4095",
                st.births, st.deaths,
            );
            // Each founder by the id it was given before the first tick — the
            // germination-versus-invisibility readout. `dead` means the id no
            // longer resolves; a small number means alive and unrenderable.
            let founder_line: Vec<String> = founders
                .iter()
                .map(|id| match world.organism(*id) {
                    Some(s) => format!("{}", s.cells.len()),
                    None => "dead".to_string(),
                })
                .collect();
            println!("            founders (cells): {}", founder_line.join(" "));
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
