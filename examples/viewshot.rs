//! What the *player's viewport* shows of a world larger than itself.
//!
//! `filmstrip` renders whole small worlds, which was the same picture while
//! the world was exactly one screen. It no longer is: the world ships four
//! screens wide and twice as deep, and almost everything that can go wrong
//! with that is invisible in a whole-world render —
//!
//! - the camera clamped so hard at an edge that half the screen is outside
//!   the world,
//! - the sky's horizon band or its moon placed against *world* bounds, so it
//!   lands off-screen and is never seen,
//! - a world whose surface has drifted below the initial view, so a fresh
//!   start shows nothing but sky.
//!
//! So this renders viewport-sized frames at camera positions across the
//! world, through the same `Renderer` the app uses, and lays them out as a
//! contact sheet. One row is one traverse.
//!
//! ```text
//! cargo run --release --example viewshot
//! cargo run --release --example viewshot -- seed=7 preset=arid shots=6
//! cargo run --release --example viewshot -- frame=1800   # night, for stars and moon
//! ```
//!
//! Written for the world-size step, kept for weather: a rain or snow sky is
//! judged from the viewport too, and drawn precipitation is position-hashed
//! against *world* coordinates, so it must not slide as the camera pans.

use pixel_physics::app::{HEIGHT, WIDTH, WORLD_HEIGHT, WORLD_WIDTH};
use pixel_physics::render::Renderer;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::world::World;

struct Args {
    seed: u32,
    preset: String,
    shots: usize,
    frame: usize,
    settle: usize,
    out: String,
}

fn main() {
    let mut a = Args {
        seed: 1,
        preset: String::new(),
        shots: 4,
        // Mid-morning by default: the sky is lit, which is the harder case
        // for judging ground colour. `frame=1800` is the other half of the
        // cycle and is where stars and the moon are.
        frame: 600,
        settle: 60,
        out: "target/filmstrips/viewshot.png".to_string(),
    };
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "seed" => a.seed = v.parse().expect("seed=N"),
            "preset" => a.preset = v.to_string(),
            "shots" => a.shots = v.parse::<usize>().expect("shots=N").max(1),
            "frame" => a.frame = v.parse().expect("frame=N"),
            "settle" => a.settle = v.parse().expect("settle=N"),
            "out" => a.out = v.to_string(),
            _ => panic!("unknown argument {arg:?}"),
        }
    }

    let bounds = Rect::new(0, 0, WORLD_WIDTH as i32 - 1, WORLD_HEIGHT as i32 - 1);
    let mut world = World::new(bounds);
    let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("{e}");
    }
    let name = if a.preset.is_empty() { presets.default_name() } else { a.preset.clone() };
    let Some(params) = presets.get(&name) else { panic!("unknown preset {name:?}") };
    let build = std::time::Instant::now();
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed: a.seed as u64 });
    let build_ms = build.elapsed().as_secs_f64() * 1000.0;

    // Let it settle before looking. Generated terrain is meant to be at rest
    // by construction, so anything still moving here is worth seeing in the
    // image rather than hidden by rendering frame zero.
    for _ in 0..a.settle {
        pixel_physics::sim::parallel::step(&mut world);
        world.step_active_sites();
    }

    let mut renderer = Renderer::new();
    let particles = ParticleSystem::new();
    let (vw, vh) = (WIDTH as usize, HEIGHT as usize);
    let mut sheet = vec![0u8; vw * vh * a.shots * 4];
    let mut frame = vec![0u8; vw * vh * 4];

    // Camera targets spread across the world's width at the height of the
    // ground, so each shot frames a different region rather than a different
    // patch of sky. Reported next to the image, because a contact sheet
    // cannot show *where* it was taken and four pictures of the same hill
    // look exactly like four different hills.
    println!("world {}x{} ({name}, seed {}), built in {build_ms:.0} ms", WORLD_WIDTH, WORLD_HEIGHT, a.seed);
    for shot in 0..a.shots {
        let x = ((shot as f32 + 0.5) / a.shots as f32 * WORLD_WIDTH as f32) as i32;
        let ground = (0..WORLD_HEIGHT as i32)
            .find(|&y| world.get(x, y).material != material::EMPTY)
            .unwrap_or(WORLD_HEIGHT as i32 / 2);
        renderer.follow((x, ground), (WIDTH, HEIGHT), world.bounds());
        let (cam_x, cam_y) = (renderer.camera_x, renderer.camera_y);
        // Clamped hard against an edge is legitimate at the ends of the
        // world and a bug in the middle, so print the camera rather than
        // asserting: the reader can see which case this is.
        println!(
            "  shot {shot}: target ({x}, {ground}) -> camera ({cam_x}, {cam_y}), \
             showing world x {cam_x}..{}",
            cam_x + WIDTH as i32
        );

        // The frame buffer persists across shots and only the *first* draw
        // is forced full. That is deliberate and is the point of this
        // harness: every later shot has to be repainted by the camera move
        // alone. Without `last_camera` feeding the renderer's `full` flag,
        // the dirty-rect skip leaves the previous shot's pixels in place and
        // the sheet shows the same view four times over -- a failure that is
        // invisible in any single image, and the one thing most likely to go
        // wrong when a viewport starts moving.
        let touched = world.take_touched_chunks();
        renderer.draw(&world, &particles, &touched, &mut frame, (WIDTH, HEIGHT), shot == 0);

        for y in 0..vh {
            let src = y * vw * 4;
            let dst = (y * vw * a.shots + shot * vw) * 4;
            sheet[dst..dst + vw * 4].copy_from_slice(&frame[src..src + vw * 4]);
        }
        // Step between shots so the day advances a little and the sky is not
        // bit-identical in every tile -- a sheet of identical skies hides a
        // sky that is not being redrawn at all.
        for _ in 0..(a.frame / a.shots.max(1)) {
            pixel_physics::sim::parallel::step(&mut world);
        }
    }

    let (sw, sh) = ((vw * a.shots) as u32, vh as u32);
    if let Some(dir) = std::path::Path::new(&a.out).parent() {
        std::fs::create_dir_all(dir).expect("creating the output directory");
    }
    image::save_buffer(&a.out, &sheet, sw, sh, image::ColorType::Rgba8).expect("writing the sheet");
    println!("contact sheet ({sw}x{sh}, {} viewport shots): {}", a.shots, a.out);
}
