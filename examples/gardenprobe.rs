//! **Where does the colony's plant food end up, and what lies in its nest?**
//!
//! Built for the plants-and-ants lane (2026-10-30), before the fungus garden:
//! a garden that grows on what the ants carry home is only worth building if
//! plant food actually comes to rest in the nest, and only the census can say
//! which food that is. Runs a lab scenario through `Lab::tick_for_harness`
//! (the game's own tick, mister included) with the trained ant the lab binary
//! launches with (`ant=old` for the hand-built one), and at each stop prints
//! every loose (unowned) food cell by material, split three ways:
//!
//! - `nest`: within two cells of worked nest ground (`nest`, `packedsoil`);
//! - `roofed`: elsewhere, but with ground over it (`garden::roofed`);
//! - `lit`: everything else.
//!
//! Plus the garden's own counters (`World::garden_*`), so the
//! same binary answers "did it fire" and "what is it standing on".
//!
//! `scenario=played_bed seed=1 frames=30000,60000,100000 ant=new|old`
//! `png=dir` writes `lab.draw` at each stop; `crop=x,y,w,h` (screen pixels);
//! `zoom=N center=x,y|nest` aims the camera; `prune=F:x0:x1` prunes at frame F.

use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::sim::material::MaterialKind;
use std::collections::BTreeMap;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

fn main() {
    pixel_physics::sim::creature::mutation_off_for_measuring();
    let scenario_name: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    let seed: u64 = arg("seed").unwrap_or(1);
    let stops: Vec<u64> = arg::<String>("frames")
        .unwrap_or_else(|| "30000,60000,100000".to_string())
        .split(',')
        .map(|s| s.parse().expect("frames wants a list of frame numbers"))
        .collect();
    let old_ant = arg::<String>("ant").as_deref() == Some("old");
    let png: Option<String> = arg("png");
    let crop: Option<(u32, u32, u32, u32)> = arg::<String>("crop").and_then(|s| {
        let p: Vec<u32> = s.split(',').filter_map(|v| v.parse().ok()).collect();
        (p.len() == 4).then(|| (p[0], p[1], p[2], p[3]))
    });
    let tag: String = arg("tag").unwrap_or_default();

    let mut sc = Scenario::load(&scenario_name).unwrap_or_else(|e| {
        eprintln!("scenario {scenario_name}: {e}");
        std::process::exit(1);
    });
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    lab.show_help = false;
    let msg = lab.load_scenario(sc);
    if !old_ant {
        lab.set_new_ant(true);
    }
    if lab.stats.showing() {
        lab.stats.toggle();
    }
    println!(
        "gardenprobe: scenario={scenario_name} seed={seed} ant={} garden={} frames={stops:?} tag={tag}",
        if old_ant { "old" } else { "new" },
        lab.world.garden_on
    );
    println!("  {msg}");
    if let Some(dir) = &png {
        std::fs::create_dir_all(dir).expect("png dir");
    }

    let nest = lab.world.materials.id_of("nest");
    let packed = lab.world.materials.id_of("packedsoil");
    // `prune=F:x0:x1` -- the player's PRUNE brush at frame F over every
    // column x0..=x1, top to bottom (`plant::prune_cell`), so a run can show
    // the verb without a window.
    let prune: Option<(u64, i32, i32)> = arg::<String>("prune").map(|v| {
        let p: Vec<i64> = v.split(':').map(|t| t.parse().expect("prune=F:x0:x1")).collect();
        (p[0] as u64, p[1] as i32, p[2] as i32)
    });
    // `zoom=N` with `center=x,y` or `center=nest` points the game's own
    // camera before each picture, as `labgif` does.
    let zoom: u32 = arg("zoom").unwrap_or(1).max(1);
    let center: Option<String> = arg("center");
    for _ in 1..zoom {
        lab.renderer.adjust_zoom(1);
    }
    let mut f = 0u64;
    for &stop in &stops {
        while f < stop {
            lab.tick_for_harness();
            f += 1;
            if let Some((pf, x0, x1)) = prune {
                if f == pf {
                    let mut clipped = 0;
                    for x in x0..=x1 {
                        for y in (0..lab.spec.height).rev() {
                            clipped += u32::from(pixel_physics::sim::plant::prune_cell(&mut lab.world, x, y));
                        }
                    }
                    println!("  PRUNED f={f} x={x0}..{x1}: {clipped} cells");
                }
            }
        }
        let w = &lab.world;
        let (ww, hh) = (lab.spec.width, lab.spec.height);
        let worked = |x: i32, y: i32| {
            let m = w.get(x, y).material;
            Some(m) == nest || Some(m) == packed
        };
        let mut rows: BTreeMap<String, [u64; 3]> = BTreeMap::new();
        let mut worth: BTreeMap<String, [f64; 3]> = BTreeMap::new();
        let mut nest_cells = 0u64;
        for y in 0..hh {
            for x in 0..ww {
                let c = w.get(x, y);
                if worked(x, y) {
                    nest_cells += 1;
                }
                if c.organism_id() != 0 {
                    continue;
                }
                let m = w.materials.get(c.material);
                if m.food_energy <= 0.0 || matches!(w.materials.kind(c.material), MaterialKind::Creature) {
                    continue;
                }
                let near = (-2..=2).any(|dy| (-2..=2).any(|dx| worked(x + dx, y + dy)));
                let col = if near {
                    0
                } else if pixel_physics::sim::garden::roofed(w, x, y) {
                    1
                } else {
                    2
                };
                rows.entry(m.name.clone()).or_default()[col] += 1;
                worth.entry(m.name.clone()).or_default()[col] += pixel_physics::sim::creature::food_value(w, c) as f64;
            }
        }
        let ants = w
            .live_organism_ids()
            .into_iter()
            .filter(|id| w.organism(*id).is_some_and(|st| w.species.get(st.species).creature.is_some()))
            .count();
        let plants = w
            .live_organism_ids()
            .into_iter()
            .filter(|id| w.organism(*id).is_some_and(|st| w.species.get(st.species).creature.is_none() && !st.dormant_seed))
            .count();
        println!("STOP {tag} seed={seed} f={f} ants={ants} plants={plants} nest_cells={nest_cells} garden_started={} garden_spread={} garden_ripened_j={} garden_wilted={}",
            w.garden_started, w.garden_spread, w.garden_ripened_j, w.garden_wilted);
        // Noon-equivalent light where a garden could be: on the food in the
        // nest band, and in the air of every dug cell, by depth below the
        // ground as built.
        let q = |v: &mut Vec<f32>| -> String {
            if v.is_empty() {
                return "n=0".to_string();
            }
            v.sort_by(|a, b| a.total_cmp(b));
            let at = |p: f32| v[((v.len() - 1) as f32 * p) as usize];
            format!("n={} p10={:.3} p50={:.3} p90={:.3}", v.len(), at(0.1), at(0.5), at(0.9))
        };
        let light = |x: i32, y: i32| pixel_physics::sim::plant::ambient_light_above(w, x, y);
        let raw = |x: i32, y: i32| w.field_at(x, y).light;
        let mut food_raw = Vec::new();
        let mut food_light = Vec::new();
        let mut roofed_fodder = 0u32;
        let mut by_depth: BTreeMap<i32, Vec<f32>> = BTreeMap::new();
        for y in 0..hh {
            for x in 0..ww {
                let c = w.get(x, y);
                if c.organism_id() == 0 && w.materials.get(c.material).garden_fodder && (-2..=2).any(|dy| (-2..=2).any(|dx| worked(x + dx, y + dy))) {
                    food_light.push(light(x, y));
                    roofed_fodder += u32::from(pixel_physics::sim::garden::roofed(w, x, y));
                    food_raw.push(raw(x, y));
                }
                if w.dug_cells.contains(&(x, y)) && w.is_empty(x, y) {
                    let top = w.room_surface_at(x).unwrap_or(0);
                    by_depth.entry(((y - top) / 4 * 4).clamp(-8, 40)).or_default().push(light(x, y));
                }
            }
        }
        println!("  nest fodder under a roof: {roofed_fodder}");
        println!("  light at nest fodder: {}  raw: {}  sky amplitude now {:.3}", q(&mut food_light), q(&mut food_raw), pixel_physics::sim::field::sky_light_amplitude(w.sky_frame()));
        for (d, v) in by_depth.iter_mut() {
            println!("  light in dug air, depth {d:>3}+: {}", q(v));
        }
        for (name, n) in &rows {
            let j = worth[name];
            println!("  {name:<12} nest {:>6} ({:>8.0} J)  roofed {:>6} ({:>8.0} J)  lit {:>6} ({:>8.0} J)", n[0], j[0], n[1], j[1], n[2], j[2]);
        }
        if let Some(dir) = &png {
            if let Some(c) = center.as_deref() {
                let w = &lab.world;
                let (cx, cy) = if c == "nest" {
                    let (mut sx, mut n) = (0i64, 0i64);
                    for y in 0..lab.spec.height {
                        for x in 0..lab.spec.width {
                            if worked_at(w, nest, packed, x, y) {
                                sx += x as i64;
                                n += 1;
                            }
                        }
                    }
                    let x = if n > 0 { (sx / n) as i32 } else { lab.spec.width / 2 };
                    (x, w.room_surface_at(x).unwrap_or(lab.spec.height / 2) + 8)
                } else {
                    let v: Vec<i32> = c.split(',').map(|t| t.parse().expect("center=x,y or nest")).collect();
                    (v[0], v[1])
                };
                let bounds = pixel_physics::sim::chunk::Rect::new(0, 0, lab.spec.width - 1, lab.spec.height - 1);
                let (span_x, span_y) = lab.renderer.visible_span((WIDTH, HEIGHT));
                lab.renderer.set_camera(cx - span_x / 2, cy - span_y / 2, (WIDTH, HEIGHT), Some(bounds));
            }
            let mut full = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
            lab.draw(&mut full, 60.0);
            let img = image::RgbaImage::from_raw(WIDTH, HEIGHT, full).expect("frame");
            let img = match crop {
                Some((x, y, cw, ch)) => image::imageops::crop_imm(&img, x, y, cw, ch).to_image(),
                None => img,
            };
            let img = image::imageops::resize(&img, img.width() * 3, img.height() * 3, image::imageops::FilterType::Nearest);
            let path = format!("{dir}/{tag}s{seed}_f{f}.png");
            img.save(&path).expect("save");
            println!("  wrote {path}");
        }
    }
}

fn worked_at(w: &pixel_physics::sim::world::World, nest: Option<pixel_physics::sim::material::MaterialId>, packed: Option<pixel_physics::sim::material::MaterialId>, x: i32, y: i32) -> bool {
    let m = w.get(x, y).material;
    Some(m) == nest || Some(m) == packed
}
