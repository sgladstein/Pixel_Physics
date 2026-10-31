//! **Burn through the foot of the lab's tree and watch what the crown does.**
//!
//! Built 2026-10-31 for the owner's report: *"if I burn and destroy the bottom
//! of a tree ... totally disconnected from the ground, it'll still just hang
//! there floating for a really long time ... eventually it starts
//! disintegrating very very slowly from the top down. Sometimes it still
//! kind of grows a little more."* No harness reproduced that: `filmstrip
//! scene=fell` is the outdoor gnome's axe on an outdoor bed, and nothing in
//! `examples/` cuts a lab plant at all.
//!
//! It grows `played_bed` (the box the owner plays, with its one tree) for
//! `start=` frames through `Lab::tick_for_harness`, so the mister runs as in
//! the game, finds the biggest tree, and lights every cell of it in a band
//! `band=` rows tall at the foot of its trunk at the material's own flame
//! temperature -- exactly what the lab's FIRE brush (`Lab::fire_at`) does to
//! a cell. Then it steps `frames=` more and prints, every `every=` frames,
//! the "did it fire" numbers beside the picture:
//!
//! - `crown` -- living cells of the tree above the cut, and how many of them
//!   the support walk has marked as reaching no anchor (`support == MAX`);
//! - `new` -- crown cells that were not there at the cut (growth after it);
//! - `severed` / `pieces` -- `FailureCounts`' felling counters;
//! - `shed` -- `World::plant_cut_off_cells_shed`, the lab's wither path.
//!
//! ```text
//! cargo run --release --example treecut -- load=off out=/tmp/off.gif
//! cargo run --release --example treecut -- load=on  out=/tmp/on.gif
//! ```
//!
//! **`load=` sets `World::plant_load_failure`** (the lab's COLLAPSE UNDER
//! LOAD dial) for the whole run, because a saved `assets/lab_dials.ron` with
//! it off is how a player's box differs from the shipped one. **`cut=fire`**
//! (default) lights the band; **`cut=erase`** empties it and records the
//! disturbance, the brush's route; **`cut=none`** cuts nothing, the control
//! for what the box severs on its own (`box-wide living severed`, every
//! plant in the bed, not only this tree). Echoes every parameter it read.

use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::sim::cell::{Cell, OrganismId};
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::world::World;
use std::collections::HashSet;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

fn biggest_of(world: &World, species: &str) -> Option<OrganismId> {
    let sid = world.species.id_of(species)?;
    world
        .live_organism_ids()
        .into_iter()
        .filter(|&id| world.organism(id).is_some_and(|s| s.species == sid))
        .max_by_key(|&id| (world.organism(id).map_or(0, |s| s.cells.len()), std::cmp::Reverse(id)))
}

fn is_root_tissue(world: &World, x: i32, y: i32) -> bool {
    world.materials.get(world.get(x, y).material).reinforces_powder
}

fn set_load(lab: &mut Lab, load: &str) {
    match load {
        "on" => lab.world.plant_load_failure = true,
        "off" => lab.world.plant_load_failure = false,
        _ => {}
    }
}

fn main() {
    let scenario_name: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    let seed: u64 = arg("seed").unwrap_or(1);
    let start: u64 = arg("start").unwrap_or(30_000);
    let frames: u64 = arg("frames").unwrap_or(6_000);
    let every: u64 = arg("every").unwrap_or(30);
    let band: i32 = arg("band").unwrap_or(4);
    let species: String = arg("species").unwrap_or_else(|| "tree".to_string());
    let load: String = arg("load").unwrap_or_else(|| "keep".to_string());
    let cut: String = arg("cut").unwrap_or_else(|| "fire".to_string());
    let zoom: u32 = arg("zoom").unwrap_or(2).max(1);
    let out: String = arg("out").unwrap_or_else(|| "/tmp/treecut.gif".to_string());
    let colony: u32 = arg("colony").unwrap_or(0);
    println!(
        "treecut: scenario={scenario_name} seed={seed} start={start} frames={frames} every={every} band={band} \
         species={species} load={load} cut={cut} zoom={zoom} colony={colony} out={out}"
    );

    let mut sc = Scenario::load(&scenario_name).unwrap_or_else(|e| {
        eprintln!("scenario {scenario_name}: {e}");
        std::process::exit(1);
    });
    sc.bed.seed = seed;
    if colony == 0 {
        // The colony is not the question and its digging moves the soil
        // line under the tree; `colony=1` keeps the scenario's own landing.
        sc.timeline.clear();
    }
    let mut lab = Lab::new(sc.bed.clone());
    lab.show_help = false;
    let msg = lab.load_scenario(sc);
    println!("  {msg}");
    // **Set at the cut, not at the start**, so both arms cut the very same
    // tree: the dial also governs growth-time snapping, and setting it from
    // frame 0 grew two different trees (2,456 against 2,762 cells on seed 1).
    // `load_from=0` sets it from the start instead -- a box saved that way.
    let load_from: u64 = arg("load_from").unwrap_or(start);
    for f in 0..start {
        if f == load_from {
            set_load(&mut lab, &load);
        }
        lab.tick_for_harness();
    }
    set_load(&mut lab, &load);
    println!("  plant_load_failure = {} (from frame {load_from})", lab.world.plant_load_failure);
    let Some(tree) = biggest_of(&lab.world, &species) else {
        println!("  no living {species} at frame {start}");
        return;
    };
    let cells: Vec<(i32, i32)> = lab.world.organism(tree).map(|s| s.cells.keys().copied().collect()).unwrap_or_default();
    let shoot: Vec<(i32, i32)> = cells.iter().copied().filter(|&(x, y)| !is_root_tissue(&lab.world, x, y)).collect();
    let base_y = shoot.iter().map(|&(_, y)| y).max().unwrap_or(0);
    let top_y = shoot.iter().map(|&(_, y)| y).min().unwrap_or(0);
    // The trunk's columns at its foot: shoot cells on the lowest shoot row.
    let foot: Vec<i32> = shoot.iter().filter(|&&(_, y)| y == base_y).map(|&(x, _)| x).collect();
    let (fx0, fx1) = (*foot.iter().min().unwrap_or(&0), *foot.iter().max().unwrap_or(&0));
    let cx = (fx0 + fx1) / 2;
    println!(
        "  {species} #{tree} at frame {start}: {} cells ({} shoot), shoot rows {top_y}..={base_y}, foot x {fx0}..={fx1}",
        cells.len(),
        shoot.len()
    );

    // **The cut**: every cell of this tree in the band's rows, root tissue
    // included -- a fire at the soil line takes whatever is there. The band
    // is the bottom `band` shoot rows plus one row into the soil, so no
    // surface root holds the crown up through a cell the fire skipped.
    let y_lo = base_y - band + 1;
    let y_hi = base_y + 1;
    let mut lit = 0u32;
    for &(x, y) in &cells {
        if y < y_lo || y > y_hi {
            continue;
        }
        let mut c = lab.world.get(x, y);
        if c.organism_id() != tree {
            continue;
        }
        if cut == "none" {
            continue;
        }
        if cut == "erase" {
            lab.world.set(x, y, Cell::EMPTY);
            lab.world.record_disturbance(x, y, 1);
            lit += 1;
            continue;
        }
        let m = lab.world.materials.get(c.material);
        if m.flammability <= 0.0 || m.burn_duration == 0 {
            continue;
        }
        let hot = m.burn_temperature;
        c.ignite(m.burn_duration);
        if hot.is_finite() {
            c.set_temperature(hot.round() as i16);
        }
        lab.world.set(x, y, c);
        lit += 1;
    }
    println!("  cut rows {y_lo}..={y_hi}: {lit} cells {}", if cut == "erase" { "erased" } else { "lit" });
    let crown0: HashSet<(i32, i32)> = shoot.iter().copied().filter(|&(_, y)| y < y_lo).collect();
    println!("  crown above the cut: {} cells", crown0.len());

    // Camera on the tree, at `zoom`, centred halfway up it.
    for _ in 1..zoom {
        lab.renderer.adjust_zoom(1);
    }
    let (full_w, full_h) = (WIDTH, HEIGHT);
    let bounds = pixel_physics::sim::chunk::Rect::new(0, 0, lab.spec.width - 1, lab.spec.height - 1);
    let (span_x, span_y) = lab.renderer.visible_span((full_w, full_h));
    let cy = (top_y + base_y) / 2 + 10;
    lab.renderer.set_camera(cx - span_x / 2, cy - span_y / 2, (full_w, full_h), Some(bounds));

    let sev0 = lab.world.structural_failures.severed_organism_cells;
    let pcs0 = lab.world.structural_failures.severed_organism_pieces;
    let shed0 = lab.world.plant_cut_off_cells_shed;
    let living0 = lab.world.structural_failures.severed_living_cells;
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    for f in 0..=frames {
        if f % every == 0 {
            let w = &lab.world;
            let (mut alive, mut floating, mut fresh) = (0u32, 0u32, 0u32);
            if let Some(st) = w.organism(tree) {
                for &(x, y) in st.cells.keys() {
                    if y >= y_lo {
                        continue;
                    }
                    alive += 1;
                    if w.organism_cell(x, y).is_some_and(|c| c.support == u16::MAX) {
                        floating += 1;
                    }
                    if !crown0.contains(&(x, y)) {
                        fresh += 1;
                    }
                }
            }
            // Wood and leaf of any owner in the crown's old footprint that is
            // no longer this tree's -- what came off and is still lying there.
            let mut dead_in_place = 0u32;
            for &(x, y) in &crown0 {
                let c = w.get(x, y);
                if c.organism_id() != tree && w.materials.kind(c.material) != MaterialKind::Empty {
                    dead_in_place += 1;
                }
            }
            let gone = w.organism(tree).is_none();
            let (anchors, roots, senescent, stump) = w.organism(tree).map_or((0, 0, false, 0), |st| {
                (st.anchor_cells, st.root_cells, st.senescent, st.cells.keys().filter(|&&(_, y)| y >= y_lo).count())
            });
            println!(
                "  f+{f:>5} crown alive {alive:>5} (no anchor {floating:>5}, new since cut {fresh:>4}) | non-tree in old crown footprint {dead_in_place:>5} \
                 | severed {} pieces {} shed {} (box-wide living severed {}) | anchors {anchors} roots {roots} stump {stump}{}{}",
                w.structural_failures.severed_organism_cells - sev0,
                w.structural_failures.severed_organism_pieces - pcs0,
                w.plant_cut_off_cells_shed - shed0,
                w.structural_failures.severed_living_cells - living0,
                if senescent { " SENESCENT" } else { "" },
                if gone { " | TREE GONE" } else { "" }
            );
            let mut full = vec![0u8; (full_w * full_h * 4) as usize];
            lab.draw(&mut full, 60.0);
            if let Some(img) = image::RgbaImage::from_raw(full_w, full_h, full) {
                shots.push(img);
            }
        }
        if f < frames {
            lab.tick_for_harness();
        }
    }

    // **What is still up in the air at the end**: every non-empty cell in
    // the tree's old column, more than 20 rows above the ground, by owner and
    // material -- a crown that came down leaves this near zero, and a picture
    // at box scale cannot tell a stuck twig from a lamp.
    {
        let w = &lab.world;
        let (x_lo, x_hi) = crown0.iter().fold((i32::MAX, i32::MIN), |(a, b), &(x, _)| (a.min(x), b.max(x)));
        let mut aloft: std::collections::BTreeMap<String, (u32, i32, i32, i32, i32)> = std::collections::BTreeMap::new();
        for y in top_y.min(base_y)..(base_y - 20) {
            for x in x_lo..=x_hi {
                let c = w.get(x, y);
                if w.materials.kind(c.material) == MaterialKind::Empty {
                    continue;
                }
                let who = if c.organism_id() == 0 { "loose".to_string() } else { format!("organism {}", c.organism_id()) };
                let key = format!("{who} {}", w.materials.get(c.material).name);
                let e = aloft.entry(key).or_insert((0, i32::MAX, i32::MIN, i32::MAX, i32::MIN));
                *e = (e.0 + 1, e.1.min(x), e.2.max(x), e.3.min(y), e.4.max(y));
            }
        }
        println!("  still aloft (x {x_lo}..={x_hi}, rows {top_y}..{}):", base_y - 20);
        for (k, (n, a, b, c, d)) in &aloft {
            println!("    {k}: {n} cells, x {a}..={b}, y {c}..={d}");
        }
    }

    let delay_ms = arg::<u64>("delay").unwrap_or(80);
    let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(delay_ms));
    let n = shots.len();
    let gif_frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
    let file = std::fs::File::create(&out).unwrap_or_else(|e| panic!("treecut: {out}: {e}"));
    let mut encoder = image::codecs::gif::GifEncoder::new(file);
    encoder.set_repeat(image::codecs::gif::Repeat::Infinite).expect("repeat");
    encoder.encode_frames(gif_frames).expect("encode");
    println!("  wrote {out} ({n} frames)");
}
