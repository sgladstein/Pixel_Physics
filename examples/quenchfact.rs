//! **The Quench prototype's engine-fact probe** (probe 5, `src/quench/`).
//!
//! Does a stone lid make one splash of water worth more stone? A lava stream
//! runs along a flat stone floor from a vent; at `t=10 s` one splash lands on
//! it. Arms, each over paired seeds:
//!
//! - `none`: no splash, no lid (the stream's own crust).
//! - `splash`: the splash in the open.
//! - `lid`: a lid over the splash site, placed after the first frame, with
//!   its shadow declared roofed (`World::set_game_roofed`) as the game does.
//! - `lidundeclared`: the same lid without the declaration (what the shipped
//!   engine does with a lid a player places).
//! - `lidonly`: the lid and no splash.
//!
//! ```text
//! cargo run --release --example quenchfact -- seeds=1,2,3,4,5 gap=6 r=4
//! ```
//!
//! Prints, at 30 s: quench events, rock (unattached stone and rubble) in the
//! lid's columns, lava that got past the site, drips (steam that turned back
//! to water under the lid), water sent to the sky, and the seconds from the
//! splash to the last quench event (how long the engine ran).
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::weather::Weather;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{frame, player};

const W: i32 = 256;
const H: i32 = 160;
const FLOOR: i32 = 120;
const SITE: (i32, i32) = (60, 84);

struct Out {
    quench: u32,
    rock: u32,
    passed: u32,
    drips: u32,
    to_sky: f64,
    ran_for: f32,
}

fn main() {
    let mut seeds = vec![1u64, 2, 3];
    let (mut gap, mut r, mut gif, mut every, mut skirt, mut splash_s) = (6, 4, None::<String>, 2u64, 3, 10.0f32);
    let mut arms: Vec<String> = ["none", "splash", "lid", "lidcap", "lidundeclared", "lidonly"].iter().map(|s| s.to_string()).collect();
    for a in std::env::args().skip(1) {
        let (k, v) = a.split_once('=').expect("key=value");
        match k {
            "seeds" => seeds = v.split(',').map(|s| s.parse().unwrap()).collect(),
            "arms" => arms = v.split(',').map(String::from).collect(),
            "gap" => gap = v.parse().unwrap(),
            "r" => r = v.parse().unwrap(),
            "gif" => gif = Some(v.to_string()),
            "every" => every = v.parse().unwrap(),
            "skirt" => skirt = v.parse().unwrap(),
            "at" => splash_s = v.parse().unwrap(),
            _ => panic!("unknown argument {k}"),
        }
    }
    println!("quenchfact: arms {arms:?} seeds {seeds:?} lid gap {gap} rows over the floor, splash radius {r}, vent 3 cells every {every} ticks, cap skirt {skirt}, splash at {splash_s} s");
    println!("{:<14} {:>4} {:>7} {:>6} {:>7} {:>6} {:>8} {:>9}", "arm", "seed", "quench", "rock", "passed", "drips", "to_sky", "ran_for_s");
    for arm in &arms {
        let mut sums = [0f64; 6];
        for &seed in &seeds {
            let o = run(arm, seed, gap, r, every, skirt, splash_s, gif.as_deref());
            println!(
                "{:<14} {:>4} {:>7} {:>6} {:>7} {:>6} {:>8.1} {:>9.2}",
                arm, seed, o.quench, o.rock, o.passed, o.drips, o.to_sky, o.ran_for
            );
            for (s, v) in sums.iter_mut().zip([o.quench as f64, o.rock as f64, o.passed as f64, o.drips as f64, o.to_sky, o.ran_for as f64]) {
                *s += v;
            }
        }
        let n = seeds.len() as f64;
        println!(
            "{:<14} mean {:>7.0} {:>6.0} {:>7.0} {:>6.0} {:>8.1} {:>9.2}",
            arm,
            sums[0] / n,
            sums[1] / n,
            sums[2] / n,
            sums[3] / n,
            sums[4] / n,
            sums[5] / n
        );
    }
}

#[allow(clippy::too_many_arguments)] // a probe's knobs, each echoed in the header line
fn run(arm: &str, seed: u64, gap: i32, r: i32, every: u64, skirt: i32, splash_s: f32, gif: Option<&str>) -> Out {
    let mut w = World::new(Rect::new(0, 0, W - 1, H - 1));
    w.seed = seed;
    w.weather_override = Some(Weather::CLEAR);
    w.set_sky_hold(pixel_physics::sim::clock::SkyPin::Noon.hold());
    let lava = w.materials.id_of("lava").unwrap();
    let steam = w.materials.id_of("steam").unwrap();
    for x in 0..W {
        for y in FLOOR..H {
            w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
    for y in 100..FLOOR {
        w.set(0, y, Cell::new(material::STONE, 0).with_attached(true));
    }
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::default();
    let tuning = player::Tuning::default();
    let lid_y = FLOOR - gap - 3;
    let has_lid = arm.starts_with("lid");
    let mut renderer = pixel_physics::render::Renderer::new();
    let mut buf = vec![0u8; (W * H * 4) as usize];
    let mut frames = Vec::new();
    let (r0, b0) = (w.phase_changes.reacted, w.atmospheric_bank);
    let (mut last_react, mut drips) = (0u64, 0u32);
    let splash_at = (splash_s * 60.0) as u64;
    // The seed moves the vent's rhythm as well as the world's own rolls, so
    // paired seeds are different streams and not one stream relabelled.
    let phase = seed % 2;
    for t in 0..1800u64 {
        if (t + phase).is_multiple_of(every) {
            for x in 2..5 {
                for y in (FLOOR - 4)..FLOOR {
                    if w.get(x, y).material == material::EMPTY {
                        w.set(x, y, Cell::new(lava, ((t * 37) & 0xff) as u8));
                        break;
                    }
                }
            }
        }
        if t == 1 && has_lid {
            for x in (SITE.0 - 2)..=(SITE.1 + 2) {
                for y in lid_y..(lid_y + 3) {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
                // `lidcap`: skirts down each side, an upturned cup.
                if arm.starts_with("lidcap") && (x <= SITE.0 || x >= SITE.1) {
                    for y in (lid_y + 3)..(lid_y + 3 + skirt) {
                        w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                    }
                }
                if arm != "lidundeclared" {
                    for y in (lid_y + 3)..FLOOR {
                        w.set_game_roofed(x, y, true);
                    }
                }
            }
        }
        if t == splash_at && arm != "none" && arm != "lidonly" {
            let (cx, cy) = ((SITE.0 + SITE.1) / 2, FLOOR - r - 1);
            for y in (cy - r)..=(cy + r) {
                for x in (cx - r)..=(cx + r) {
                    if (x - cx).pow(2) + (y - cy).pow(2) <= r * r && w.get(x, y).material == material::EMPTY {
                        w.set(x, y, Cell::new(material::WATER, 0));
                    }
                }
            }
        }
        // `...cold`: the lid is a heat sink -- its cells are put back to
        // ambient every tick, so steam that touches it cools and beads.
        if arm.ends_with("cold") && t >= 1 {
            for x in (SITE.0 - 2)..=(SITE.1 + 2) {
                for y in lid_y..(lid_y + 3 + skirt) {
                    let c = w.get(x, y);
                    if c.material == material::STONE && c.attached() {
                        w.set(x, y, c.with_temperature(15));
                    }
                }
            }
        }
        if arm.ends_with("fast") {
            drips += pixel_physics::quench::bead_under_roofs(&mut w, steam);
        }
        let before = w.phase_changes.reacted;
        let snapshot: Vec<bool> = if has_lid {
            ((SITE.0 - 2)..=(SITE.1 + 2)).flat_map(|x| (lid_y..FLOOR).map(move |y| (x, y))).map(|(x, y)| w.get(x, y).material == steam).collect()
        } else {
            Vec::new()
        };
        frame::step(&mut w, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        if has_lid {
            let mut i = 0;
            for x in (SITE.0 - 2)..=(SITE.1 + 2) {
                for y in lid_y..FLOOR {
                    if snapshot[i] && w.get(x, y).material == material::WATER {
                        drips += 1;
                    }
                    i += 1;
                }
            }
        }
        if w.phase_changes.reacted > before && t >= splash_at {
            last_react = t;
        }
        if gif.is_some() && seed == 1 && t % 6 == 0 && t < splash_at + 900 {
            let touched = w.take_touched_chunks();
            renderer.draw(&w, &particles, &touched, &mut buf, (W as u32, H as u32), true);
            frames.push(buf.clone());
        }
    }
    if let Some(path) = gif {
        if seed == 1 {
            let path = path.replace("{arm}", arm);
            write_crop(&path, &frames);
            println!("    gif {path}");
        }
    }
    let mut rock = 0;
    for x in SITE.0..=SITE.1 {
        for y in 0..FLOOR {
            let c = w.get(x, y);
            if matches!(w.materials.kind(c.material), MaterialKind::Solid | MaterialKind::Powder) && !c.attached() {
                rock += 1;
            }
        }
    }
    let mut passed = 0;
    for x in (SITE.1 + 6)..W {
        for y in 0..FLOOR {
            let c = w.get(x, y);
            if c.material == lava || (c.material == material::STONE && !c.attached()) {
                passed += 1;
            }
        }
    }
    let ran_for = if last_react >= splash_at { (last_react - splash_at) as f32 / 60.0 } else { 0.0 };
    Out { quench: w.phase_changes.reacted - r0, rock, passed, drips, to_sky: w.atmospheric_bank - b0, ran_for }
}

/// A 5x close-up of the splash site: 6 ticks a frame at 100 ms a frame,
/// which plays at 1.67x slower than real time.
fn write_crop(path: &str, frames: &[Vec<u8>]) {
    let (x0, y0, cw, ch, z) = (40u32, 96u32, 64u32, 26u32, 5u32);
    let file = std::fs::File::create(path).unwrap();
    let mut enc = image::codecs::gif::GifEncoder::new_with_speed(file, 10);
    enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
    let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(100));
    for f in frames {
        let mut img = image::RgbaImage::new(cw * z, ch * z);
        for y in 0..ch * z {
            for x in 0..cw * z {
                let s = (((y0 + y / z) * W as u32 + x0 + x / z) * 4) as usize;
                img.put_pixel(x, y, image::Rgba([f[s], f[s + 1], f[s + 2], 255]));
            }
        }
        enc.encode_frame(image::Frame::from_parts(img, 0, 0, delay)).unwrap();
    }
}
