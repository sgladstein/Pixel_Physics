//! **Q3's rendering harness for
//! `Reports/creature-genome-flexibility-2026-09-02.md` §13e item 3**: "Does
//! anyone want these shapes?" There is no multi-part `BodyPlan` in the
//! engine to build one on (and building it is explicitly out of scope this
//! session -- see the report this harness accompanies), so this renders
//! single `Rigid` bodies shaped like candidate articulated silhouettes --
//! uniform 3- and 5-segment bodies, and a forward taper (small head, big
//! abdomen -- the insect silhouette) and backward taper (big head, shrinking
//! tail) -- at the exact crop and zoom `creature_scale.rs`'s `mode=size`
//! already ships, so the images are at "shipped resolution" and comparable
//! to the two shipped bodies (`ant`, `beetle`) rendered the same way.
//!
//! **A monolithic `Rigid` body's silhouette only stands in for an
//! articulated one while standing still or walking straight over flat
//! ground.** The moment a body turns or crosses uneven terrain, a
//! genuinely articulated body's trailing parts follow the leading part's
//! traced path and the whole body bends; a monolithic `Rigid` body can
//! only translate as one fixed shape. That bend is plausibly the visual
//! difference between "a chain" and "an animal", so a still render of these
//! shapes cannot show it -- a *positive* answer from the blind card these
//! renders feed still counts, but a *null* one does not settle whether
//! shape matters, only whether it matters while standing still. See the
//! report's §3 correction for the fuller account (credited to independent
//! review from the session on `claude/creature-genome-flexibility-ut3f71`).
//!
//! Writes one PNG per candidate (`out=` is a directory), for posting as a
//! blind gallery card. Economics are `ant_block`'s, unmodified except
//! `body`, matching every other harness on this line.
//!
//! ```text
//! cargo run --release --example creature_candidate_render -- shape=forward_taper seed=7 out=/tmp/candidates
//! ```

use std::collections::HashSet;

use pixel_physics::app::{HEIGHT, WIDTH};
use pixel_physics::render::Renderer;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::organism::BodyPlan;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{creature, parallel};

const DAYLIGHT: f32 = 1.0;
const CROP: i32 = 13;
const ZOOM_PX: i32 = 12;

fn build(seed: u64, preset: &str) -> World {
    let (w, h) = (WIDTH as i32, HEIGHT as i32);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    world.seed = seed;
    let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("worldgen presets unavailable: {e}");
    }
    let params = presets.get(preset).unwrap_or_else(|| panic!("no worldgen preset {preset:?}"));
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed });
    world
}

fn body_cells(world: &World, id: u16) -> Vec<(i32, i32)> {
    let b = world.bounds().expect("bounded world");
    let mut out = Vec::new();
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let cell = world.get(x, y);
            if cell.organism_id() == id && world.materials.kind(cell.material) == MaterialKind::Creature {
                out.push((x, y));
            }
        }
    }
    out
}

fn render(world: &World) -> Vec<u8> {
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let mut r = Renderer::new();
    r.pinned_light = Some(pixel_physics::sky::frame_for_daylight(DAYLIGHT));
    let particles = ParticleSystem::new();
    r.draw(world, &particles, &HashSet::new(), &mut frame, (WIDTH, HEIGHT), true);
    frame
}

/// Candidate silhouettes, 14 cells each (`ant`=2, `beetle`=4, unmodified,
/// for the two shipped references). Built as a raw cell set including the
/// origin (so segment blocks are easy to lay out), then the origin is
/// stripped before handing the rest to `BodyPlan::Rigid`, which takes the
/// head as implicit.
fn shape(name: &str) -> Option<BodyPlan> {
    let mut cells: Vec<(i32, i32)> = match name {
        "ant" | "beetle" => return None, // shipped species, used unmodified
        "uniform3" => {
            let mut c = Vec::new();
            for dx in 0..2 {
                for dy in 0..2 {
                    c.push((dx, dy));
                } // segment 1, 2x2
            }
            c.push((2, 0)); // pinch
            for dx in 3..5 {
                for dy in 0..2 {
                    c.push((dx, dy));
                } // segment 2, 2x2
            }
            c.push((5, 0)); // pinch
            for dx in 6..8 {
                for dy in 0..2 {
                    c.push((dx, dy));
                } // segment 3, 2x2
            }
            c
        }
        "uniform5" => {
            let mut c = Vec::new();
            for i in 0..5 {
                let base = i * 2;
                c.push((base, 0));
                c.push((base, 1));
                if i < 4 {
                    c.push((base + 1, 0)); // pinch
                }
            }
            c
        }
        "forward_taper" => {
            let mut c = Vec::new();
            for dx in 0..2 {
                for dy in 0..2 {
                    c.push((dx, dy));
                } // head, 2x2
            }
            c.push((2, 0)); // waist
            for dx in 3..6 {
                for dy in 0..3 {
                    c.push((dx, dy));
                } // abdomen, 3x3
            }
            c
        }
        "backward_taper" => {
            let mut c = Vec::new();
            for dx in 0..3 {
                for dy in 0..3 {
                    c.push((dx, dy));
                } // head end, 3x3
            }
            c.push((3, 0)); // waist
            for dx in 4..6 {
                for dy in 0..2 {
                    c.push((dx, dy));
                } // tail, 2x2
            }
            c
        }
        other => panic!("unknown shape {other}; expected ant, beetle, uniform3, uniform5, forward_taper or backward_taper"),
    };
    cells.retain(|&(dx, dy)| (dx, dy) != (0, 0));
    Some(BodyPlan::Rigid(cells.iter().map(|&(dx, dy)| (-dx as i8, -dy as i8)).collect()))
}

fn main() {
    let mut shape_name = "forward_taper".to_string();
    let mut seed = 7u64;
    let mut preset = "rolling".to_string();
    let mut out = String::new();
    let mut crop = CROP;
    let mut zoom = ZOOM_PX;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "shape" => shape_name = v.to_string(),
            "seed" => seed = v.parse().unwrap_or(seed),
            "preset" => preset = v.to_string(),
            "out" => out = v.to_string(),
            "crop" => crop = v.parse().unwrap_or(crop),
            "zoom" => zoom = v.parse().unwrap_or(zoom),
            _ => {}
        }
    }
    println!("creature_candidate_render: shape={shape_name} seed={seed} preset={preset} out={out:?}");

    let mut world = build(seed, &preset);
    let base_species = if matches!(shape_name.as_str(), "ant" | "beetle") { shape_name.as_str() } else { "ant_block" };
    let id = world.species.id_of(base_species).unwrap_or_else(|| panic!("no species {base_species}"));
    if let Some(plan) = shape(&shape_name) {
        let mut def = world.species.get(id).creature.clone().expect("a creature");
        def.body = plan;
        world.species.set_creature(id, def);
    }

    let cx = WIDTH as i32 / 2;
    let mut placed = None;
    for dx in 0..80 {
        for x in [cx + dx, cx - dx] {
            if placed.is_some() {
                break;
            }
            let Some(sy) = creature::colony_ant_site(&world, x, 0) else { continue };
            if let Some(site) = creature::plant_creature_seed(&mut world, x, sy - 1, base_species) {
                world.schedule_active_site(site);
                placed = Some((x, sy - 1));
            }
        }
        if placed.is_some() {
            break;
        }
    }
    let (px, py) = placed.unwrap_or_else(|| panic!("no site for {shape_name}"));
    parallel::step(&mut world);
    world.step_active_sites();

    let id = world.get(px, py).organism_id();
    let cells = body_cells(&world, id);
    println!("  {shape_name}: {} cells on screen", cells.len());
    let (x0, x1) = (cells.iter().map(|c| c.0).min().unwrap_or(px), cells.iter().map(|c| c.0).max().unwrap_or(px));
    let (y0, y1) = (cells.iter().map(|c| c.1).min().unwrap_or(py), cells.iter().map(|c| c.1).max().unwrap_or(py));

    if out.is_empty() {
        return;
    }
    std::fs::create_dir_all(&out).expect("out dir");
    let frame = render(&world);
    let panel = (2 * crop * zoom) as usize;
    let (ccx, ccy) = ((x0 + x1) / 2, (y0 + y1) / 2);
    let half = crop;
    let mut img = vec![0u8; panel * panel * 4];
    for ry in 0..(2 * half * zoom) {
        for rx in 0..(2 * half * zoom) {
            let (sx, sy) = (ccx - half + rx / zoom, ccy - half + ry / zoom);
            let (dx, dy) = (rx as usize, ry as usize);
            if dx >= panel || dy >= panel {
                continue;
            }
            let d = (dy * panel + dx) * 4;
            if sx < 0 || sy < 0 || sx >= WIDTH as i32 || sy >= HEIGHT as i32 {
                continue;
            }
            let s = ((sy as usize * WIDTH as usize) + sx as usize) * 4;
            img[d..d + 4].copy_from_slice(&frame[s..s + 4]);
        }
    }
    let path = format!("{out}/{shape_name}.png");
    write_png(&path, &img, panel, panel);
    println!("  wrote {path}");
}

fn write_png(path: &str, rgba: &[u8], w: usize, h: usize) {
    fn crc32(data: &[u8]) -> u32 {
        let mut table = [0u32; 256];
        for (i, e) in table.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
            *e = c;
        }
        let mut c = 0xFFFF_FFFFu32;
        for &b in data {
            c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
        }
        c ^ 0xFFFF_FFFF
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(body);
        let mut crc_in = kind.to_vec();
        crc_in.extend_from_slice(body);
        out.extend_from_slice(&crc32(&crc_in).to_be_bytes());
    }
    let mut raw = Vec::with_capacity((w * 4 + 1) * h);
    for y in 0..h {
        raw.push(0u8);
        raw.extend_from_slice(&rgba[y * w * 4..(y + 1) * w * 4]);
    }
    let mut z = vec![0x78, 0x01];
    for (i, block) in raw.chunks(65_535).enumerate() {
        let last = if (i + 1) * 65_535 >= raw.len() { 1u8 } else { 0u8 };
        z.push(last);
        z.extend_from_slice(&(block.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        z.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &raw {
        a = (a + byte as u32) % 65_521;
        b = (b + a) % 65_521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut png, b"IHDR", &ihdr);
    chunk(&mut png, b"IDAT", &z);
    chunk(&mut png, b"IEND", &[]);
    std::fs::write(path, png).expect("write png");
}
