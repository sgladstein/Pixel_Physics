//! **Is an animal the same physical size in a world built at twice the cell
//! resolution?** Until 2026-08-30 the answer was no, and nothing measured it.
//!
//! `World::cell_scale` says a world can be generated at `k` cells per
//! authored cell, and `player::Player::at_scaled` makes the gnome `k` times
//! as many cells across so that he stays the same *character*. Nothing else
//! alive read the scalar at all -- not `creature.rs`, not `organism.rs` --
//! so every animal came out at its authored cell count, which at `k=2` is
//! **half its physical size**. That is the owner's own "our gnome shouldn't
//! have shrunk" defect (`Reports/resolution-step-2026-08-29.md`) arriving
//! for everything that is not the gnome.
//!
//! ```text
//! cargo run --release --example creature_scale -- species=ant_block scales=1,2 out=/tmp/pair.png
//! cargo run --release --example creature_scale -- mode=walk species=ant_block scales=1,2 frames=4000
//! ```
//!
//! # Two modes, because size and mobility are different questions
//!
//! `mode=size` (the default) is the **picture**: one animal on generated
//! ground, cropped to a fixed number of *physical* units and upscaled so
//! that every arm is the same number of screen pixels. If the scaling is
//! right the body covers the same fraction of each panel and only the
//! detail changes. The numbers under it are the bounding box in cells and
//! the same box divided by `k` -- and it is the second column that has to
//! match, which is the whole claim.
//!
//! `mode=walk` is the **counter**, because a picture cannot say whether a
//! body can move: `moves_blocked / (moves + moves_blocked)` over a colony
//! on real terrain. `Reports/creature-appearance-design.md` §5 puts a
//! `Chain(2)` at 5% and a 3x3 `Rigid` at 43%, which is the bar any wider
//! body has to be read against. **A body that reads as an animal and cannot
//! walk is not a shipped animal**, so this number is reported beside the
//! picture rather than after it.
//!
//! **Both modes echo their own parameters**, per `CLAUDE.md`'s stale-harness
//! rule -- a log that does not name its settings was written by a binary
//! that never had them.

use std::collections::HashSet;

use pixel_physics::app::{HEIGHT, WIDTH};
use pixel_physics::render::Renderer;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::world::World;
use pixel_physics::sim::organism::CreatureDef;
use pixel_physics::sim::{creature, parallel};

/// Where in the day every frame is pinned. Noon, for the reason
/// `creature_look` pins it: the day/night cycle is a designed oscillator
/// and it aliases into anything read off a rendered frame, so two arms
/// sampled at different hours would differ by the hour rather than by the
/// change (`CLAUDE.md`).
const DAYLIGHT: f32 = 1.0;

/// Half-width of the crop, **in physical units** -- authored cells, not
/// grid cells. Every arm crops `2 * CROP` physical units wide, so at `k=2`
/// that is twice as many grid cells and the panels still show the same
/// piece of ground.
const CROP_DEFAULT: i32 = 13;

/// Pixels per physical unit in the output. Fixed across arms, which is what
/// makes the panels comparable at a glance: an arm at `k` upscales by
/// `PANEL_ZOOM / k`, so a correct scaling puts the body at the same size in
/// every panel and a broken one halves it.
const PANEL_ZOOM_DEFAULT: i32 = 12;

fn main() {
    let mut mode = "size".to_string();
    let mut species = "ant_block".to_string();
    let mut scales: Vec<i32> = vec![1, 2];
    let mut frames = 4000u64;
    let mut seed = 7u64;
    let mut preset = "rolling".to_string();
    let mut count = 24i32;
    let mut out = String::new();
    let mut control = true;
    let mut crop = CROP_DEFAULT;
    let mut zoom = PANEL_ZOOM_DEFAULT;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "mode" => mode = v.to_string(),
            "species" => species = v.to_string(),
            "scales" => scales = v.split(',').filter_map(|s| s.trim().parse().ok()).collect(),
            "frames" => frames = v.parse().unwrap_or(frames),
            "seed" => seed = v.parse().unwrap_or(seed),
            "preset" => preset = v.to_string(),
            "count" => count = v.parse().unwrap_or(count),
            "out" => out = v.to_string(),
            "control" => control = v != "off",
            "crop" => crop = v.parse().unwrap_or(crop),
            "zoom" => zoom = v.parse().unwrap_or(zoom),
            _ => {}
        }
    }
    println!("creature_scale: mode={mode} species={species} scales={scales:?} frames={frames} seed={seed} preset={preset} count={count} control={control} crop={crop} zoom={zoom} out={out:?}");

    match mode.as_str() {
        "size" => size_mode(&Sheet {
            species: &species,
            scales: &scales,
            seed,
            preset: &preset,
            out: &out,
            control,
            crop,
            zoom_px: zoom,
        }),
        "walk" => walk_mode(&species, &scales, seed, &preset, count, frames),
        other => panic!("unknown mode {other}; expected size or walk"),
    }
}

/// Build one world at `k` cells per authored cell.
///
/// **The bounds scale with `k`, not the preset alone.** A 2x world in a
/// 512x320 box is half the ground, so an arm measured in it would differ
/// from its 1x sibling by the terrain as well as by the resolution -- the
/// A/B-with-two-differences failure `CLAUDE.md` names.
fn build(k: i32, seed: u64, preset: &str) -> World {
    let (w, h) = (WIDTH as i32 * k, HEIGHT as i32 * k);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    world.seed = seed;
    let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("worldgen presets unavailable: {e}");
    }
    let params = presets.get(preset).unwrap_or_else(|| panic!("no worldgen preset {preset:?}")).scaled(k as f32);
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params: &params, seed });
    world
}

/// Every cell in the world belonging to organism `id`.
///
/// Read off the **world**, not off `BodyPlan`, deliberately: the plan is
/// what was asked for and this is what arrived, and the gap between them is
/// exactly what a placement refusal or a clipped body looks like.
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

fn render(world: &World, w: u32, h: u32) -> Vec<u8> {
    let mut frame = vec![0u8; (w * h * 4) as usize];
    let mut r = Renderer::new();
    r.pinned_light = Some(pixel_physics::sky::frame_for_daylight(DAYLIGHT));
    let particles = ParticleSystem::new();
    r.draw(world, &particles, &HashSet::new(), &mut frame, (w, h), true);
    frame
}

/// The arms of the size sheet: a resolution, and whether the body scaling
/// pass is switched on.
///
/// **The `false` arm is the control and it is not optional.** A 2x panel
/// beside a 1x panel differs in the grid *and* in the terrain that grid
/// generated; only an arm at the same resolution with the pass off isolates
/// the pass. See `SpeciesRegistry::set_creature`.
fn arms(scales: &[i32], control: bool) -> Vec<(i32, bool)> {
    let mut out = Vec::new();
    for (i, &k) in scales.iter().enumerate() {
        if control && k > 1 && i + 1 == scales.len() {
            out.push((k, false));
        }
        out.push((k, true));
    }
    out
}

/// One panel per (species, resolution, scaling) arm.
///
/// **`species=` takes a comma list**, because the question the owner sent
/// back -- *"both are smudges"* -- is a comparison between body plans at one
/// resolution, and a sheet that can only vary the grid cannot ask it.
struct Sheet<'a> {
    species: &'a str,
    scales: &'a [i32],
    seed: u64,
    preset: &'a str,
    out: &'a str,
    control: bool,
    crop: i32,
    zoom_px: i32,
}

fn size_mode(sheet: &Sheet) {
    let &Sheet { species, scales, seed, preset, out, control, crop, zoom_px } = sheet;
    let panel = (2 * crop * zoom_px) as usize;
    let species_list: Vec<&str> = species.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
    let mut panels: Vec<Vec<u8>> = Vec::new();
    // The authored def, read out of a 1x world so the control arm below is
    // the real thing rather than a hand-written copy of it.
    let authored: Vec<CreatureDef> = {
        let w = build(1, seed, preset);
        species_list
            .iter()
            .map(|sp| {
                let id = w.species.id_of(sp).unwrap_or_else(|| panic!("no species {sp}"));
                w.species.get(id).creature.clone().unwrap_or_else(|| panic!("{sp} is not a creature"))
            })
            .collect()
    };
    for (k, scaled) in arms(scales, control) {
    for (si, &species) in species_list.iter().enumerate() {
        let mut world = build(k, seed, preset);
        if !scaled {
            let id = world.species.id_of(species).expect("species");
            world.species.set_creature(id, authored[si].clone());
        }
        // Mid-map, on whatever ground `found_colony` would have accepted --
        // the same predicate, never a second copy of it
        // (`open-bugs-handoff.md` R2).
        let cx = WIDTH as i32 / 2 * k;
        let mut placed = None;
        for dx in 0..(60 * k) {
            for x in [cx + dx, cx - dx] {
                if placed.is_some() {
                    break;
                }
                let Some(sy) = creature::colony_ant_site(&world, x, 0) else { continue };
                if let Some(site) = creature::plant_creature_seed(&mut world, x, sy - 1, species) {
                    world.schedule_active_site(site);
                    placed = Some((x, sy - 1));
                }
            }
            if placed.is_some() {
                break;
            }
        }
        let (px, py) = placed.unwrap_or_else(|| panic!("no site for {species} at k={k}"));
        // One step, so the body is laid out by the sim rather than only by
        // the stamp -- a plan that places and then cannot hold together
        // should show here rather than in the picture.
        parallel::step(&mut world);
        world.step_active_sites();

        let id = world.get(px, py).organism_id();
        let cells = body_cells(&world, id);
        let (x0, x1) = (cells.iter().map(|c| c.0).min().unwrap_or(px), cells.iter().map(|c| c.0).max().unwrap_or(px));
        let (y0, y1) = (cells.iter().map(|c| c.1).min().unwrap_or(py), cells.iter().map(|c| c.1).max().unwrap_or(py));
        let (bw, bh) = (x1 - x0 + 1, y1 - y0 + 1);
        let def = world.species.get(world.organism(id).expect("live").species).creature.clone().expect("a creature");
        println!(
            "  {species:<18} k={k} body-scaling={:<3} plan={:>3} cells  on-screen={:>3} cells  bbox={bw}x{bh} cells = {:.1}x{:.1} physical  \
             tick_interval={}  sensor_offset={}  idle/cell={:.5}  move/cell={:.5}",
            if scaled { "on" } else { "OFF" },
            def.body.len(),
            cells.len(),
            bw as f32 / k as f32,
            bh as f32 / k as f32,
            def.tick_interval,
            def.sensor_offset,
            def.idle_cost_per_cell,
            def.move_cost_per_cell,
        );

        if out.is_empty() {
            continue;
        }
        let (fw, fh) = (WIDTH * k as u32, HEIGHT * k as u32);
        let frame = render(&world, fw, fh);
        // Crop `2*CROP` **physical** units around the body's centre, then
        // upscale by `PANEL_ZOOM / k` so every panel is the same pixel size.
        let (ccx, ccy) = ((x0 + x1) / 2, (y0 + y1) / 2);
        let half = crop * k;
        let zoom = (zoom_px / k).max(1);
        let mut img = vec![0u8; panel * panel * 4];
        for ry in 0..(2 * half * zoom) {
            for rx in 0..(2 * half * zoom) {
                let (sx, sy) = (ccx - half + rx / zoom, ccy - half + ry / zoom);
                let (dx, dy) = (rx as usize, ry as usize);
                if dx >= panel || dy >= panel {
                    continue;
                }
                let d = (dy * panel + dx) * 4;
                if sx < 0 || sy < 0 || sx >= fw as i32 || sy >= fh as i32 {
                    continue;
                }
                let s = ((sy as usize * fw as usize) + sx as usize) * 4;
                img[d..d + 4].copy_from_slice(&frame[s..s + 4]);
            }
        }
        panels.push(img);
    }
    }

    if out.is_empty() {
        return;
    }
    // Side by side, with a one-pixel gutter, so the pair is one card.
    let gutter = 4usize;
    let w = panels.len() * panel + gutter * panels.len().saturating_sub(1);
    let mut sheet = vec![24u8; w * panel * 4];
    for (i, p) in panels.iter().enumerate() {
        let ox = i * (panel + gutter);
        for y in 0..panel {
            let s = y * panel * 4;
            let d = (y * w + ox) * 4;
            sheet[d..d + panel * 4].copy_from_slice(&p[s..s + panel * 4]);
        }
    }
    write_png(out, &sheet, w, panel);
    println!("  wrote {out} ({w}x{panel}; {} panels, each {} physical units wide at {zoom_px} px/unit)", panels.len(), 2 * crop);
}

/// **Can it walk?** `moves_blocked / (moves + moves_blocked)` on generated
/// terrain, which is the number `Reports/creature-appearance-design.md` §5
/// reports 5% for a `Chain(2)` and 43% for a 3x3 `Rigid`.
///
/// A whole colony rather than one animal, because a single body samples one
/// piece of ground and the spread over terrain here is enormous
/// (`CLAUDE.md`: compare two runs, not one run against a remembered number).
fn walk_mode(species: &str, scales: &[i32], seed: u64, preset: &str, count: i32, frames: u64) {
    for &k in scales {
        let mut world = build(k, seed, preset);
        let cols: Vec<i32> = (0..WIDTH as i32 * k).filter(|&x| creature::colony_ant_site(&world, x, 0).is_some()).collect();
        assert!(cols.len() >= count as usize * 2, "only {} viable columns at k={k}", cols.len());
        let mut placed = 0;
        for i in 0..count {
            let x = cols[(i as usize * cols.len()) / count as usize];
            let Some(sy) = creature::colony_ant_site(&world, x, 0) else { continue };
            if let Some(site) = creature::plant_creature_seed(&mut world, x, sy - 1, species) {
                world.schedule_active_site(site);
                placed += 1;
            }
        }
        // **All three, in `ascii`'s order.** `parallel::step` is the CA
        // sweep alone -- creatures live on the active-site scheduler, so a
        // loop without `step_active_sites` runs a world in which nothing
        // decides anything. The first run of this harness did exactly that
        // and reported `moves=0 blocked=0` for every arm: a clean, tidy
        // null from a probe that never reached the mechanism, which is
        // `CLAUDE.md`'s counter trap in its purest form.
        for _ in 0..frames {
            parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
        }
        let s = world.creature_stats;
        let attempts = s.moves + s.moves_blocked;
        let blocked = if attempts == 0 { f64::NAN } else { s.moves_blocked as f64 / attempts as f64 };
        println!(
            "  k={k} placed={placed} alive={} ticks={} moves={} blocked={} => blocked {:.1}%  falls={} digs={}",
            world.live_creature_count(),
            s.ticks,
            s.moves,
            s.moves_blocked,
            blocked * 100.0,
            s.falls,
            s.digs,
        );
    }
}

/// A minimal non-interlaced RGBA PNG, stored (uncompressed) deflate blocks
/// -- the same writer `stamp_probe` and `terrain_shade` carry, and for the
/// same reason: nothing in the tree pulls an image crate.
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
    // zlib: no compression, stored blocks of at most 65,535 bytes.
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
