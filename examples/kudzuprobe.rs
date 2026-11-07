//! **The Kudzu probe's headless harness** -- see `src/kudzu/mod.rs`.
//!
//! ```text
//! cargo run --release --example kudzuprobe -- mode=grow frames=6000 every=600 out=/tmp/kudzu
//! ```

use pixel_physics::kudzu::{hud, Kudzu, Level, HEIGHT, WIDTH};
use pixel_physics::render::Renderer;

fn arg<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
        .unwrap_or(default)
}

fn save(frame: &[u8], scale: u32, path: &str) {
    let (w, h) = (WIDTH as u32 * scale, HEIGHT as u32 * scale);
    let mut img = image::RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let i = (((y / scale) * WIDTH as u32 + x / scale) * 4) as usize;
            img.put_pixel(x, y, image::Rgba([frame[i], frame[i + 1], frame[i + 2], 255]));
        }
    }
    img.save(path).expect("write png");
}

fn to_image(frame: &[u8], scale: u32) -> image::RgbaImage {
    let (w, h) = (WIDTH as u32 * scale, HEIGHT as u32 * scale);
    let mut img = image::RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let i = (((y / scale) * WIDTH as u32 + x / scale) * 4) as usize;
            img.put_pixel(x, y, image::Rgba([frame[i], frame[i + 1], frame[i + 2], 255]));
        }
    }
    img
}

fn write_gif(shots: Vec<image::RgbaImage>, delay_ms: u32, out: &str) {
    let delay = image::Delay::from_numer_denom_ms(delay_ms, 1);
    let frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
    let n = frames.len();
    let file = std::fs::File::create(out).expect("create gif");
    let mut encoder = image::codecs::gif::GifEncoder::new_with_speed(file, 10);
    encoder.set_repeat(image::codecs::gif::Repeat::Infinite).expect("repeat");
    encoder.encode_frames(frames).expect("encode gif");
    println!("  wrote {out} ({n} frames)");
}

/// The snip that would drop the most vine: every vine cell tried as a blade
/// centre. What a player who reads the preview perfectly would pick.
fn best_cut(game: &Kudzu) -> Option<((i32, i32), usize)> {
    let mut cells: Vec<(i32, i32)> = game.vine_cells().into_iter().collect();
    cells.sort_unstable();
    let mut best: Option<((i32, i32), usize)> = None;
    for &(x, y) in cells.iter().step_by(2) {
        let n = game.preview(x, y).0.len();
        if best.is_none_or(|b| n > b.1) {
            best = Some(((x, y), n));
        }
    }
    best
}

fn cut_mode(seed: u64, pre: u64, after: u64, out: &str) {
    let mut game = Kudzu::new(Level::Walls, seed);
    for _ in 0..pre {
        game.tick();
    }
    let mut renderer = Renderer::new();
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let before = game.vine_cells().len();
    let Some(((x, y), n)) = best_cut(&game) else {
        println!("  no vine to cut");
        return;
    };
    println!("  best cut at {x},{y}: preview says {n} of {before} vine cells let go");
    hud::draw_world(&mut game, &mut renderer, &mut frame);
    to_image(&frame, 3).save(format!("{out}/cut-0-still.png")).unwrap();
    let lit = hud::draw_preview(&game, &mut frame, x, y);
    to_image(&frame, 3).save(format!("{out}/cut-1-preview.png")).unwrap();
    println!("  preview lit {lit}");
    let mut shots = vec![to_image(&frame, 2)];
    game.snip(x, y);
    let t = std::time::Instant::now();
    let mut worst = 0.0f64;
    for f in 1..=after {
        let ft = std::time::Instant::now();
        game.tick();
        worst = worst.max(ft.elapsed().as_secs_f64() * 1000.0);
        if f % 4 == 0 {
            hud::draw_world(&mut game, &mut renderer, &mut frame);
            shots.push(to_image(&frame, 2));
        }
        if [30, 60, 120, 240, 600].contains(&f) {
            println!(
                "  +{f}: vine {} | predicted {} fell {} | green dropped {} | regrown {}",
                game.vine_cells().len(),
                game.stats.predicted,
                game.stats.fell,
                game.stats.green_dropped,
                game.stats.regrown
            );
        }
    }
    println!("  {after} frames after the cut: mean {:.2} ms, worst {:.2} ms", t.elapsed().as_secs_f64() * 1000.0 / after as f64, worst);
    write_gif(shots, 66, &format!("{out}/cut.gif"));
}

fn main() {
    let mode: String = arg("mode", "grow".to_string());
    let seed: u64 = arg("seed", 1);
    let frames: u64 = arg("frames", 6000);
    let every: u64 = arg("every", 600);
    let out: String = arg("out", "/tmp/kudzu".to_string());
    println!("kudzuprobe: mode={mode} seed={seed} frames={frames} every={every} out={out}");
    std::fs::create_dir_all(&out).ok();
    if mode == "cut" {
        cut_mode(seed, arg("pre", 1200), arg("after", 600), &out);
        return;
    }
    let t = std::time::Instant::now();
    let mut game = Kudzu::new(Level::Walls, seed);
    println!("  overnight {} frames in {:.1}s", game.layout.overnight, t.elapsed().as_secs_f32());
    let mut renderer = Renderer::new();
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    for f in 0..=frames {
        if f % every == 0 {
            let cells = game.vine_cells().len();
            println!("  frame {f}: vine cells {cells}, vines {}, wall clear {:.0}%, runners {}", game.vines().len(), game.wall_clear() * 100.0, game.runners);
            for id in game.vines() {
                let s = game.world.organism(id).unwrap();
                let carbon: Vec<f32> = s.cells.keys().map(|&(x, y)| game.world.carbon_at(x, y)).collect();
                let max = carbon.iter().cloned().fold(0.0f32, f32::max);
                let sum: f32 = carbon.iter().sum();
                println!("    carbon max {max:.2} mean {:.3} tips {}", sum / carbon.len().max(1) as f32, game.world.organism_active_tip_count(id, pixel_physics::sim::organism::CellType::GrowingTip));
            }
            if let Some(c) = game.world.grow_census.get(game.vine.0 as usize) {
                let names = ["notasked", "organ", "poor", "height", "taper", "rootshare", "tipcap", "boxed", "hard", "nodir", "grew", "flowered"];
                for kind in 0..2 {
                    let v: Vec<String> = names.iter().enumerate().filter(|(i, _)| c.visits[kind][*i] > 0).map(|(i, n)| format!("{n} {}", c.visits[kind][i])).collect();
                    let r: Vec<String> = names.iter().enumerate().filter(|(i, _)| c.retired[kind][*i] > 0).map(|(i, n)| format!("{n} {}", c.retired[kind][i])).collect();
                    println!("    {} visits: {} | retired: {}", if kind == 0 { "shoot" } else { "root" }, v.join(", "), r.join(", "));
                }
            }
            for id in game.vines() {
                let s = game.world.organism(id).unwrap();
                let mut types = std::collections::BTreeMap::new();
                for &(x, y) in s.cells.keys() {
                    let t = pixel_physics::sim::organism::cell_type(game.world.get(x, y).aux());
                    *types.entry(format!("{t:?}")).or_insert(0) += 1;
                }
                println!("    types {types:?}");
            }
            for id in game.vines() {
                let s = game.world.organism(id).unwrap();
                println!(
                    "    #{id} shoot {} root {} anchors {} income {:.2} maint {:.2} unpaid {:.2} starving {} water {:.2} nutr {:.2} age {} senescent {} starved_cells {}",
                    s.shoot_cells, s.root_cells, s.anchor_cells, s.income, s.maintenance, s.maintenance_unpaid, s.starving_ticks,
                    s.water_status, s.nutrient_status, s.age_ticks, s.senescent, s.starved_cells
                );
            }
            hud::draw_world(&mut game, &mut renderer, &mut frame);
            save(&frame, 2, &format!("{out}/grow-{f:06}.png"));
        }
        game.tick();
    }
}
