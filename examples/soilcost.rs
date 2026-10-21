//! SCRATCH (never for main): what the soil-moisture pass costs in the shipped
//! lab box, and what its writes are made of.
//!
//! ```text
//! cargo run --release --example soilcost -- founders=0 colonies=0 pre=6000 window=2000
//! ```
//! Every unknown argument is refused.

use pixel_physics::lab::rain::Rain;
use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::update::SCRATCH_SOIL;
use pixel_physics::sim::world::SCRATCH_SOIL_NANOS;
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::time::Instant;

const KNOWN: &[&str] = &["founders", "colonies", "pre", "window", "rain", "seed", "every", "pic", "picstops", "picrows", "picx", "piczoom"];

fn args() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for a in std::env::args().skip(1) {
        let Some((k, v)) = a.split_once('=') else {
            eprintln!("soilcost: `{a}` is not key=value");
            std::process::exit(2);
        };
        if !KNOWN.contains(&k) {
            eprintln!("soilcost: unknown argument `{k}` (known: {})", KNOWN.join(" "));
            std::process::exit(2);
        }
        m.insert(k.to_string(), v.to_string());
    }
    m
}

fn get<T: std::str::FromStr>(m: &BTreeMap<String, String>, k: &str, default: T) -> T {
    m.get(k).map_or(default, |v| v.parse().unwrap_or_else(|_| panic!("{k}={v} does not parse")))
}

fn snapshot() -> [u64; 16] {
    let mut v = [0u64; 16];
    for (i, c) in SCRATCH_SOIL.iter().enumerate() {
        v[i] = c.load(Ordering::Relaxed);
    }
    v
}

fn main() {
    let m = args();
    let rain = match m.get("rain").map(String::as_str) {
        None | Some("light") => Rain::Light,
        Some("off") => Rain::Off,
        Some("steady") => Rain::Steady,
        Some(v) => panic!("rain={v}"),
    };
    let spec = LabBox {
        founders: get(&m, "founders", 0usize),
        colonies: get(&m, "colonies", 0usize),
        seed: get(&m, "seed", 1u64),
        rain,
        ..LabBox::default()
    };
    let pre: u64 = get(&m, "pre", 6_000);
    let window: u64 = get(&m, "window", 2_000);
    let every: u64 = get(&m, "every", 0);
    println!(
        "soilcost: {}x{} soil_depth={} founders={} colonies={} rain={:?} drain={} seed={} pre={pre} window={window} threads={}",
        spec.width,
        spec.height,
        spec.soil_depth,
        spec.founders,
        spec.colonies,
        spec.rain,
        spec.drain,
        spec.seed,
        std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "unset".into())
    );
    let mut w = spec.build();
    let (mut particles, mut blasts, tuning) = (ParticleSystem::new(), Blasts::new(), player::Tuning::default());
    let t0 = Instant::now();
    for f in 0..pre {
        pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &spec, None);
        if every > 0 && (f + 1) % every == 0 {
            let s = w.soil_water_stats;
            println!("  frame {:>7}: visited {:>6} soil {:>6} changed {:>6} chunks {:>3}", w.frame, s.visited, s.soil, s.changed, s.chunks);
        }
    }
    println!("  pre-run: {:.1} s", t0.elapsed().as_secs_f64());
    // `pic=PATH`: the top `picrows` soil rows (plus two of air) at the stops
    // `picstops=0,5,10,...` frames after the pre-run, 2x, moisture on a fixed
    // ramp: below field capacity red->black, field capacity->saturation
    // dark blue->white, standing water cyan, air black, else grey.
    let fc = pixel_physics::sim::material::SOIL_FIELD_CAPACITY;
    if let Some(path) = m.get("pic") {
        let stops: Vec<u64> = m.get("picstops").map_or("0,5,10,20,40,80".to_string(), |v| v.clone()).split(',').map(|v| v.parse().expect("picstops")).collect();
        let rows: i32 = get(&m, "picrows", 48);
        let water = w.materials.id_of("water");
        let soil_id = w.materials.id_of("soil");
        let zoom: i32 = get(&m, "piczoom", 2);
        let gap = 3i32;
        let (x0, x1): (i32, i32) = m.get("picx").map_or((0, spec.width), |v| { let (a, b) = v.split_once(',').expect("picx=lo,hi"); (a.parse().expect("lo"), b.parse().expect("hi")) });
        let (pw, ph) = ((x1 - x0) * zoom, (rows + 2) * zoom);
        let total_h = stops.len() as i32 * (ph + gap);
        let mut img = vec![40u8; (pw * total_h * 4) as usize];
        let mut last = 0u64;
        for (si, &stop) in stops.iter().enumerate() {
            for _ in last..stop {
                pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &spec, None);
            }
            last = stop;
            let oy = si as i32 * (ph + gap);
            for ry in 0..rows + 2 {
                let y = spec.ground_y - 2 + ry;
                for x in x0..x1 {
                    let c = w.get(x, y);
                    let rgb: [u8; 3] = if c.material == pixel_physics::sim::material::EMPTY {
                        [0, 0, 0]
                    } else if Some(c.material) == water {
                        [0, 220, 255]
                    } else if Some(c.material) == soil_id {
                        let mo = pixel_physics::sim::update::soil_moisture(c);
                        if mo < fc {
                            let t = mo as f32 / fc as f32;
                            [(60.0 + 160.0 * t) as u8, (20.0 * t) as u8, (20.0 * t) as u8]
                        } else {
                            let t = ((mo - fc) as f32 / (1000 - fc) as f32).min(1.0);
                            [(20.0 + 235.0 * t) as u8, (40.0 + 215.0 * t) as u8, (120.0 + 135.0 * t) as u8]
                        }
                    } else {
                        [90, 90, 90]
                    };
                    for dy in 0..zoom {
                        for dx in 0..zoom {
                            let i = (((oy + ry * zoom + dy) * pw + (x - x0) * zoom + dx) * 4) as usize;
                            img[i..i + 3].copy_from_slice(&rgb);
                            img[i + 3] = 255;
                        }
                    }
                }
            }
        }
        image::save_buffer(path, &img, pw as u32, total_h as u32, image::ColorType::Rgba8).expect("write pic");
        println!("  wrote {path} ({pw}x{total_h}), stops {:?} after frame {pre}", stops);
    }
    let c0 = snapshot();
    let n0 = SCRATCH_SOIL_NANOS.load(Ordering::Relaxed);
    let mut sw = [0u64; 4];
    let t = Instant::now();
    for _ in 0..window {
        pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &spec, None);
        let s = w.soil_water_stats;
        sw[0] += s.visited;
        sw[1] += s.soil;
        sw[2] += s.changed;
        sw[3] += s.chunks;
    }
    let dt = t.elapsed().as_secs_f64();
    let n1 = SCRATCH_SOIL_NANOS.load(Ordering::Relaxed);
    let c1 = snapshot();
    let per = |v: u64| v as f64 / window as f64;
    let tick_ms = dt * 1000.0 / window as f64;
    let pass_ms = (n1 - n0) as f64 / 1e6 / window as f64;
    println!("RESULT tick {tick_ms:.3} ms, soil pass {pass_ms:.3} ms ({:.0}%)", pass_ms / tick_ms * 100.0);
    println!("  a tick: chunks {:.1} visited {:.0} soil {:.0} changed {:.0}  -> {:.0} ns per visited cell, {:.0} ns per changed cell", per(sw[3]), per(sw[0]), per(sw[1]), per(sw[2]), pass_ms * 1e6 / per(sw[0]), pass_ms * 1e6 / per(sw[2]));
    let d: Vec<f64> = (0..16).map(|i| per(c1[i] - c0[i])).collect();
    println!("  writes a tick by source: infiltration {:.1} capillary {:.1} drain-down {:.1} drip {:.1} root-pull {:.1} side-drain {:.1} self {:.1}", d[0], d[1], d[2], d[3], d[4], d[5], d[6]);
    let tot: f64 = d[7..13].iter().sum();
    println!(
        "  drain-down move sizes: 1 {:.1}% 2 {:.1}% 3-5 {:.1}% 6-10 {:.1}% 11-50 {:.1}% 51+ {:.1}% (n {:.0} a tick)",
        d[7] / tot * 100.0,
        d[8] / tot * 100.0,
        d[9] / tot * 100.0,
        d[10] / tot * 100.0,
        d[11] / tot * 100.0,
        d[12] / tot * 100.0,
        tot
    );
    // Soil profile: mean moisture by 8-row band, and how many cells sit at each surplus 0..8 above FC.
    let soil = w.materials.id_of("soil");
    let fc = pixel_physics::sim::material::SOIL_FIELD_CAPACITY;
    let mut surplus = [0u64; 10];
    let mut bands: Vec<(u64, u64)> = Vec::new();
    for y in spec.ground_y..spec.ground_y + spec.soil_depth {
        let b = ((y - spec.ground_y) / 8) as usize;
        if bands.len() <= b {
            bands.push((0, 0));
        }
        for x in 0..spec.width {
            let c = w.get(x, y);
            if Some(c.material) != soil {
                continue;
            }
            let mo = pixel_physics::sim::update::soil_moisture(c);
            bands[b].0 += mo as u64;
            bands[b].1 += 1;
            if mo >= fc {
                surplus[((mo - fc) as usize).min(9)] += 1;
            }
        }
    }
    println!("  soil by 8-row band, mean moisture: {}", bands.iter().map(|(s, n)| format!("{:.0}", *s as f64 / (*n).max(1) as f64)).collect::<Vec<_>>().join(" "));
    println!("  cells at FC+k, k=0..8 and 9+: {:?}", surplus);
    // Top 16 soil rows: mean, min, and how many cells sit below field capacity.
    for y in spec.ground_y..spec.ground_y + 16 {
        let (mut sum, mut n, mut min, mut below) = (0u64, 0u64, u16::MAX, 0u64);
        for x in 0..spec.width {
            let c = w.get(x, y);
            if Some(c.material) != soil {
                continue;
            }
            let mo = pixel_physics::sim::update::soil_moisture(c);
            sum += mo as u64;
            n += 1;
            min = min.min(mo);
            if mo < fc {
                below += 1;
            }
        }
        println!("  row {y}: soil {n} mean {:.0} min {min} below-FC {below}", sum as f64 / n.max(1) as f64);
    }
    println!("  rain cells placed {} drained {}", w.rain_cells, w.drained_water);
    // Hashes: every cell (material, shade, aux, organism id) and the field.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for y in 0..spec.height {
        for x in 0..spec.width {
            let c = w.get(x, y);
            for v in [c.material.0 as u64, c.shade as u64, c.aux() as u64, c.organism_id() as u64] {
                h = (h ^ v).wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
    }
    println!("HASH frame={} cells={:016x} field={:016x}", w.frame, h, pixel_physics::sim::field::field_hash(&w));
}
