//! **What today's trained lab ants do with the keeper's hand tools** -- the
//! keeper seat's step-0 look (`Reports/` keeper seat demo, 2026-10-25).
//!
//! The keeper seat's adversarial review found that most "works today" keeper
//! verbs rest on mechanisms the trained ant does not have, and named three it
//! could not rule out from the weights alone: a **hand-dug gallery** (the
//! pool's strongest feature weight is on lined walls, and a hand cut is
//! lined), an **alarm puff** (alarm weights lean away on walk and dig), and
//! a **hand-laid B trail** (slice 0's null was measured on the *old* brain).
//! This runs each as a paired arm against `arm=none` on the same seed, on the
//! trained mind (`set_new_ant(true)` -- `Lab::new` alone opens on the old
//! ant), and prints the count the arm is about beside a GIF of the window.
//!
//! ```text
//! cargo run --release --example keeperlook -- arm=sketch seed=1 out=/tmp/sketch.gif
//! cargo run --release --example keeperlook -- arm=none   seed=1 out=/tmp/none.gif
//! cargo run --release --example keeperlook -- arm=alarm  seed=1
//! cargo run --release --example keeperlook -- arm=trail  seed=1
//! ```
//!
//! Every arm places the same far heap (`far=` columns west of the nest) so
//! the `trail` arm and its control see the same food; only the hand action
//! differs between arms.

use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::sim::creature;
use pixel_physics::sim::material;
use pixel_physics::sim::pheromone::{self, Channel};
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

/// First non-empty cell from the top in column `x`, below the lid.
fn surface(world: &World, x: i32, from: i32) -> i32 {
    (from..HEIGHT as i32).find(|&y| world.get(x, y).material != material::EMPTY).unwrap_or(HEIGHT as i32)
}

fn ant_heads(lab: &Lab) -> Vec<(i32, i32)> {
    let w = &lab.world;
    let Some(sid) = w.species.id_of(&lab.spec.colony_species) else { return Vec::new() };
    w.live_organism_ids()
        .into_iter()
        .filter_map(|id| w.organism(id).filter(|st| st.species == sid).and_then(|st| st.chain.first().copied()))
        .collect()
}

fn count_food(world: &World, x0: i32, x1: i32) -> u32 {
    let Some(food) = world.materials.id_of("provisions") else { return 0 };
    let mut n = 0;
    for y in 0..HEIGHT as i32 {
        for x in x0..=x1 {
            if world.in_bounds(x, y) && world.get(x, y).material == food {
                n += 1;
            }
        }
    }
    n
}

/// The full-grid digest `lab_cost` prints (material, aux, organism per
/// cell), so two builds of this harness can be compared for identical output.
fn world_hash(w: &World) -> u64 {
    fn fnv1a(h: u64, v: u64) -> u64 {
        (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
    }
    let b = w.bounds().expect("the lab box sets bounds");
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

fn main() {
    let arm: String = arg("arm").unwrap_or_else(|| "none".to_string());
    let seed: u64 = arg("seed").unwrap_or(1);
    let scenario: String = arg("scenario").unwrap_or_else(|| "nest_goal".to_string());
    let start: u64 = arg("start").unwrap_or(12_000);
    let frames: u64 = arg("frames").unwrap_or(9_000);
    let every: u64 = arg("every").unwrap_or(60);
    let far: i32 = arg("far").unwrap_or(90);
    let out: Option<String> = arg("out");
    let crop: Option<(u32, u32, u32, u32)> = arg::<String>("crop").and_then(|s| {
        let p: Vec<u32> = s.split(',').filter_map(|v| v.parse().ok()).collect();
        (p.len() == 4).then(|| (p[0], p[1], p[2], p[3]))
    });

    let mut sc = Scenario::load(&scenario).unwrap_or_else(|e| panic!("scenario {scenario}: {e}"));
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    lab.show_help = false;
    lab.load_scenario(sc);
    if lab.stats.showing() {
        lab.stats.toggle();
    }
    let msg = lab.set_new_ant(true);
    println!("keeperlook: arm={arm} scenario={scenario} seed={seed} start={start} frames={frames} far={far} new_ant={} ({msg})", lab.new_ant);
    assert!(lab.new_ant, "keeperlook measures the trained ant; set_new_ant(true) did not take");

    for _ in 0..start {
        lab.tick_for_harness();
    }
    // The nest: the colony's mean head column at the window's start.
    let heads = ant_heads(&lab);
    assert!(!heads.is_empty(), "no colony alive at frame {start}");
    let minds = {
        let w = &lab.world;
        let sid = w.species.id_of(&lab.spec.colony_species);
        w.live_organism_ids().into_iter().filter(|&id| w.organism(id).is_some_and(|st| Some(st.species) == sid) && pixel_physics::sim::creature::mind::has(w, id)).count()
    };
    println!("  {minds} of {} colony ants think with a trained mind", heads.len());
    assert!(minds > 0, "no colony ant has a mind: this would measure the old brain");
    // The nest: the median head column (a mean is dragged by foragers out on the surface).
    let mut xs: Vec<i32> = heads.iter().map(|h| h.0).collect();
    xs.sort_unstable();
    let nest_x: i32 = arg("nest").unwrap_or(xs[xs.len() / 2]);
    let ground = surface(&lab.world, nest_x, 20);
    let far_x = (nest_x - far).max(8);
    // The same far heap in every arm, so `trail` and its control see the same food.
    {
        let food = lab.world.materials.id_of("provisions").expect("provisions");
        let g = surface(&lab.world, far_x, 20);
        let half: i32 = arg("heap").unwrap_or(4);
        for dy in 1..=half {
            for dx in -half..=half {
                let mut c = lab.world.get(far_x + dx, g - dy);
                if c.material == material::EMPTY {
                    c.material = food;
                    lab.world.set(far_x + dx, g - dy, c);
                }
            }
        }
    }
    let far_food0 = count_food(&lab.world, far_x - 12, far_x + 12);
    println!("  nest at column {nest_x}, surface y {ground}; far heap at column {far_x} ({far_food0} cells); {} ants", heads.len());

    // The arm's hand action, at the window's first frame.
    let mut sketch: Vec<(i32, i32)> = Vec::new();
    let puff = (nest_x + 15, ground - 1);
    // The sketch's footprint, built in every arm so the control counts the
    // same band: a gallery a player would draw, down and east from the nest
    // at a slant, then a flat room 5 tall. Radius 1, as the Dig brush.
    let footprint: Vec<(i32, i32)> = {
        let (ax, ay) = (nest_x + 2, ground + 2);
        let (bx, by) = (nest_x + 26, ground + 22);
        let (cx, cy) = (nest_x + 56, ground + 22);
        let mut cells = Vec::new();
        for (p, q) in [((ax, ay), (bx, by)), ((bx, by), (cx, cy))] {
            let n = (q.0 - p.0).abs().max((q.1 - p.1).abs()).max(1);
            for i in 0..=n {
                let x = p.0 + (q.0 - p.0) * i / n;
                let y = p.1 + (q.1 - p.1) * i / n;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        cells.push((x + dx, y + dy));
                    }
                }
            }
        }
        for x in bx..=cx {
            for y in (by - 3)..=(by + 1) {
                cells.push((x, y));
            }
        }
        cells.sort_unstable();
        cells.dedup();
        cells
    };
    let band: Vec<(i32, i32)> = {
        let fp: std::collections::HashSet<(i32, i32)> = footprint.iter().copied().collect();
        let mut b = std::collections::HashSet::new();
        for &(x, y) in &footprint {
            for dy in -3..=3 {
                for dx in -3..=3 {
                    if !fp.contains(&(x + dx, y + dy)) {
                        b.insert((x + dx, y + dy));
                    }
                }
            }
        }
        b.into_iter().collect()
    };
    match arm.as_str() {
        "sketch" => {
            let cut = creature::hand_dig(&mut lab.world, &footprint);
            sketch = footprint.iter().copied().filter(|&(x, y)| lab.world.get(x, y).material == material::EMPTY).collect();
            println!("  sketch: {cut} cells cut by hand_dig (lined), {} open", sketch.len());
        }
        "trail" => {
            // Nest -> food, ramped from 0 at the press point as the lab's
            // Scent brush does, so it is strongest at the food end.
            let ramp = 160.0f32;
            let (x0, x1) = (far_x, nest_x);
            for x in x0..=x1 {
                let y = surface(&lab.world, x, 20) - 1;
                let t = (((nest_x - x) as f32) / ramp).clamp(0.0, 1.0);
                for dy in -1..=1 {
                    lab.world.deposit_pheromone(Channel::B, x, y + dy, (t * pheromone::DEPOSIT as f32) as pheromone::Scent);
                }
            }
            println!("  trail: B ramp laid from column {nest_x} (0) to {far_x} (strongest)");
        }
        "alarm" | "none" => {}
        other => panic!("arm={other}: want none, sketch, alarm or trail"),
    }

    // Solid at the window's start, so "cut since" is a plain difference.
    let band_solid0: Vec<(i32, i32)> = band.iter().copied().filter(|&(x, y)| lab.world.get(x, y).material != material::EMPTY).collect();
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    let dug0 = lab.world.creature_stats.digs;
    let cells0 = lab.world.dug_cells.len();
    let mut near_puff_ticks = 0u64;
    let mut corridor_ticks = 0u64;
    let mut in_room_ticks = 0u64;
    for f in 0..=frames {
        if arm == "alarm" && (f == 0 || f == 600 || f == 1200) {
            lab.world.deposit_pheromone(Channel::Alarm, puff.0, puff.1, pheromone::ALARM_DEPOSIT);
        }
        let heads = ant_heads(&lab);
        for &(x, y) in &heads {
            if (x - puff.0).abs() <= 8 && (y - puff.1).abs() <= 8 {
                near_puff_ticks += 1;
            }
            if x >= far_x - 4 && x < nest_x - 8 && y < ground + 4 {
                corridor_ticks += 1;
            }
            if x >= nest_x + 26 && x <= nest_x + 56 && y >= ground + 16 && y <= ground + 26 {
                in_room_ticks += 1;
            }
        }
        if f % 1500 == 0 || f == 100 || f == 300 {
            let fed_adult = lab.world.minds.as_ref().map_or(0, |m| m.counts.fed_adult);
            let knows = {
                let w = &lab.world;
                w.live_organism_ids().into_iter().filter(|&id| w.organism(id).is_some_and(|st| st.last_food.is_some_and(|(x, _, at)| (x - far_x).abs() <= 12 && w.frame.saturating_sub(at) < 4000))).count()
            };
            println!("    adult feeds so far {fed_adult} | ants whose last food is the far heap (fresh) {knows}");
            let band_cut = band_solid0.iter().filter(|&&(x, y)| lab.world.get(x, y).material == material::EMPTY).count();
            let open = sketch.iter().filter(|&&(x, y)| lab.world.get(x, y).material == material::EMPTY).count();
            let mid = (far_x + nest_x) / 2;
            let my = surface(&lab.world, mid, 20) - 1;
            println!(
                "    scent: B at trail middle ({mid},{my}) {} | B at far end {} | alarm at puff {}",
                lab.world.pheromone_at(Channel::B, mid, my),
                lab.world.pheromone_at(Channel::B, far_x + 6, surface(&lab.world, far_x + 6, 20) - 1),
                lab.world.pheromone_at(Channel::Alarm, puff.0, puff.1)
            );
            println!(
                "  f+{f:>5}: ants {:>3} | digs {:>5} (dug cells {:>5}) | sketch open {:>4}/{} | cut beside sketch {:>4} | ants near puff (ant-ticks) {:>6} | corridor ant-ticks {:>7} | room ant-ticks {:>6} | far food {:>3}",
                heads.len(),
                lab.world.creature_stats.digs - dug0,
                lab.world.dug_cells.len() as i64 - cells0 as i64,
                open,
                sketch.len(),
                band_cut,
                near_puff_ticks,
                corridor_ticks,
                in_room_ticks,
                count_food(&lab.world, far_x - 12, far_x + 12)
            );
        }
        if out.is_some() && f % every == 0 {
            let mut full = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
            lab.draw(&mut full, 60.0);
            let (cx, cy, w, h) = crop.unwrap_or((0, 0, WIDTH, HEIGHT));
            let mut img = image::RgbaImage::new(w, h);
            for y in 0..h {
                for x in 0..w {
                    let i = (((cy + y) * WIDTH + cx + x) * 4) as usize;
                    img.put_pixel(x, y, image::Rgba([full[i], full[i + 1], full[i + 2], 255]));
                }
            }
            shots.push(img);
        }
        lab.tick_for_harness();
    }
    println!("  world hash at the window's end: {:#018x}", world_hash(&lab.world));
    if let Some(path) = out {
        let n = shots.len();
        let frames: Vec<image::Frame> =
            shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, image::Delay::from_numer_denom_ms(80, 1))).collect();
        let file = std::fs::File::create(&path).expect("gif file");
        let mut enc = image::codecs::gif::GifEncoder::new(file);
        enc.set_repeat(image::codecs::gif::Repeat::Infinite).ok();
        enc.encode_frames(frames).expect("encode gif");
        println!("  wrote {path} ({n} frames)");
    }
}
