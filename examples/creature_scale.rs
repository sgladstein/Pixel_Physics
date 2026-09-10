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


use pixel_physics::app::{HEIGHT, WIDTH};
use pixel_physics::render::Renderer;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::world::World;
use pixel_physics::sim::organism::{BodyPlan, CellType, CreatureDef, Segment};
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::creature::BlockedWhy;
use pixel_physics::sim::material;
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
    let mut body_override = String::new();
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
            "body" => body_override = v.to_string(),
            _ => {}
        }
    }
    // Not this harness's own arg -- read by `creature::place_creature`
    // through its own `OnceLock` -- but echoed here anyway, for the same
    // reason every other setting on this line is: a log that does not name
    // its settings was written by a binary that never had them.
    let body_laterals = std::env::var("PIXEL_PHYSICS_BODY_LATERALS").unwrap_or_else(|_| "1 (default)".to_string());
    println!(
        "creature_scale: mode={mode} species={species} scales={scales:?} frames={frames} seed={seed} preset={preset} count={count} control={control} crop={crop} zoom={zoom} out={out:?} body={body_override:?} PIXEL_PHYSICS_BODY_LATERALS={body_laterals}"
    );

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
        "walk" => walk_mode(&species, &scales, seed, &preset, count, frames, &body_override),
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

/// **Is this preset a hand-built scene rather than a worldgen preset?**
///
/// `flat` and `rolling` are open ground and answer "can this body walk";
/// the two below are the terrain the owner's complaint is actually about
/// -- *"these larger ants get stuck or cannot move easily in more
/// complicated terrain"* -- and no generated preset contains either. They
/// are hand-built for the same reason `ascii`'s structural cases are:
/// the situation has to be *guaranteed present*, and a generated world
/// that happens not to contain a one-wide tunnel would measure a body's
/// mobility in a tunnel as excellent (`CLAUDE.md`, "a scene that
/// contradicts the code will look like a bug in the code").
fn is_scene(preset: &str) -> bool {
    matches!(preset, "tunnel" | "chamber")
}

/// Carve `(x, y)` out to open air.
fn carve(world: &mut World, x: i32, y: i32) {
    world.set(x, y, Cell::EMPTY);
}

fn carve_rect(world: &mut World, x0: i32, x1: i32, y0: i32, y1: i32) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            carve(world, x, y);
        }
    }
}

/// **The tunnel scene: solid rock with one-wide passages cut through it.**
///
/// Three features, in one world so one run measures all of them:
///
/// * a **dead-end tunnel** (row 56, running right from the chamber and
///   closed at its far end) -- an animal that walks in must come back out
///   the way it came, and cannot turn round to do it;
/// * a **one-wide bend** (row 64, then a vertical leg at x=60, then row
///   40) -- two right-angle corners a body has to flow round;
/// * a **vertical shaft** (x=20, rising out of the chamber roof and closed
///   at the top) -- the straight-up case.
///
/// Everything is one cell wide deliberately. A two-wide passage would let
/// a long body turn round inside it and the scene would stop asking the
/// question. The chamber at the left is the only open ground, so an animal
/// that gets out of a passage has somewhere to be.
///
/// **Half the animals are placed inside the passages, not in the chamber.**
/// Hoping a wanderer finds a tunnel is how a scene ends up measuring
/// wandering; starting them in there is the positive control that says the
/// mechanism under test was actually reached.
fn tunnel_scene(species: &str, count: i32, body_override: &str) -> (World, i32, Option<usize>) {
    let (w, h) = (200i32, 120i32);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    for y in 0..h {
        for x in 0..w {
            world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
    apply_body_override(&mut world, species, body_override);
    // The open chamber.
    carve_rect(&mut world, 10, 30, 48, 66);
    // A dead-end tunnel, one cell high.
    carve_rect(&mut world, 30, 90, 56, 56);
    // A one-wide passage with two right-angle bends, ending blind.
    carve_rect(&mut world, 30, 60, 64, 64);
    carve_rect(&mut world, 60, 60, 40, 64);
    carve_rect(&mut world, 60, 100, 40, 40);
    // A vertical shaft out of the chamber roof, closed at the top.
    carve_rect(&mut world, 20, 20, 22, 48);

    let mut placed = 0;
    // **Read off the world, the same way `walk_mode` does**, so
    // `PIXEL_PHYSICS_BODY_LATERALS=0` visibly moves this number before any
    // blocked fraction below it is trusted -- CLAUDE.md's stale-switch
    // tell is identical output across a change that must have moved
    // something.
    let mut first_body_cells = None;
    // Along the chamber floor, and inside both horizontal passages.
    let seats: Vec<(i32, i32)> = (0..count)
        .map(|i| match i % 4 {
            0 => (14 + (i / 4) * 3, 65),
            1 => (40 + (i / 4) * 9, 56),
            2 => (40 + (i / 4) * 9, 64),
            _ => (70 + (i / 4) * 9, 40),
        })
        .collect();
    for (x, y) in seats {
        if let Some(site) = creature::plant_creature_seed(&mut world, x, y, species) {
            world.schedule_active_site(site);
            placed += 1;
            if first_body_cells.is_none() {
                let id = world.get(x, y).organism_id();
                first_body_cells = Some(body_cells(&world, id).len());
            }
        }
    }
    (world, placed, first_body_cells)
}

/// **The chamber scene, lifted from `ascii`'s `nest_dig_scene`** -- a soil
/// bank on a stone floor with a nest beside it, and the colony digging its
/// own gallery into the bank.
///
/// It is the scene §12 diagnosed and left red, and it is here for one
/// reason: the passages an animal moves through in it are the ones the
/// animal itself cut, so the terrain is exactly as wide as the body that
/// made it. That is the case a hand-built tunnel cannot pose.
fn chamber_scene(species: &str, count: i32, body_override: &str) -> (World, i32, Option<usize>) {
    let (w, h) = (200i32, 120i32);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    let floor = h - 8;
    let soil = world.materials.id_of("soil").expect("soil");
    let nest = world.materials.id_of("nest").expect("nest");
    for x in 0..w {
        for y in floor..h {
            world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
    for x in 40..160 {
        for y in (floor - 30)..floor {
            world.set(x, y, Cell::new(soil, 0).with_attached(true));
        }
    }
    for x in 16..40 {
        world.set(x, floor, Cell::new(nest, 0).with_attached(true));
    }
    apply_body_override(&mut world, species, body_override);
    let mut placed = 0;
    let mut first_body_cells = None;
    for i in 0..count.min(6) {
        let (x, y) = (8 + i * 6, floor - 1);
        if let Some(site) = creature::plant_creature_seed(&mut world, x, y, species) {
            world.schedule_active_site(site);
            placed += 1;
            if first_body_cells.is_none() {
                let id = world.get(x, y).organism_id();
                first_body_cells = Some(body_cells(&world, id).len());
            }
        }
    }
    (world, placed, first_body_cells)
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
    r.draw(world, &particles, &pixel_physics::sim::fxhash::ChunkSet::default(), &mut frame, (w, h), true);
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
/// **`body_override`, the second ablation §7d asks for.** `"segmented"`
/// takes `species`'s own authored `Chain(n)` -- so it has to be a plain
/// chain, and `ant_long`'s `Chain(6)` is the one this project already ships
/// as its length-matched control (§7a) -- and swaps it, in the live
/// registry, for a `Segmented` body of the identical length: same head,
/// same `n - 1` `Segment` cells, zero laterals. `plant_creature_seed` then
/// places bodies from a species whose *material, economy and name* are
/// completely unchanged and whose *code path* is not: `body_after_step`
/// now takes the `Segmented` arm (`segmented_body_after_step`, which
/// degenerates to the exact `chain_follow` rule when every group is 1 --
/// see that function's own doc) instead of the plain `Chain` arm. If the
/// two arms still differ, the difference is the code path, not the length,
/// the lateral count, or the species' own numbers -- nothing else moved.
/// **Re-lay `species`'s body plan for an ablation arm, before anything is
/// placed.**
///
/// `body=segmented` turns a plain `Chain(n)` into an `n`-segment
/// `Segmented` body with no laterals -- the arm whose *numbers* are
/// unchanged and whose *code path* is not, which is how the cost fork's
/// hard invariant (a lateral-free `Segmented` body is byte-identical to a
/// `Chain`) gets measured rather than asserted. `body=chainN` re-lays the
/// species as a plain `Chain(N)`, varying length alone with no `Segmented`
/// code in it at all -- and `chain2` is how the **shipped two-cell ant**
/// is recovered from a binary whose `ant.ron` now grows a five-segment
/// articulated body.
///
/// **It clears the fate table too, and until §13 it did not.** `place_
/// creature` grows the body from `body_fates` whenever the species carries
/// a `fates` table and only falls back to `def.body` when it does not, so
/// an override that writes `def.body` alone is *silently ignored* on `ant`
/// and `hopper` -- both of which author one. `SpeciesRegistry::set_fates`'s
/// own doc names this trap in as many words; this call site is the second
/// place to have walked into it. The tell that catches it is the one
/// `CLAUDE.md` names: `body_cells=` in the row below must move when the
/// override does, and it did not.
fn apply_body_override(world: &mut World, species: &str, body_override: &str) {
    if body_override.is_empty() {
        return;
    }
    let id = world.species.id_of(species).unwrap_or_else(|| panic!("no species {species}"));
    let mut def = world.species.get(id).creature.clone().expect("a creature");
    if body_override == "segmented" {
        let BodyPlan::Chain(n) = def.body else {
            panic!("body=segmented expects {species} to author a plain Chain(n); it did not (BodyPlan has no Debug impl to print the shape it got instead)");
        };
        let mut segments = vec![Segment { cell: CellType::Head, lateral: None }];
        for _ in 1..n {
            segments.push(Segment { cell: CellType::Segment, lateral: None });
        }
        def.body = BodyPlan::Segmented(segments);
    } else if let Some(n) = body_override.strip_prefix("chain") {
        let n: i32 = n.parse().unwrap_or_else(|_| panic!("body=chainN needs a number, got {body_override:?}"));
        def.body = BodyPlan::Chain(n as u8);
    } else {
        panic!("unknown body override {body_override:?}; expected segmented or chainN");
    }
    world.species.set_creature(id, def);
    world.species.set_fates(id, Vec::new());
}

fn walk_mode(species: &str, scales: &[i32], seed: u64, preset: &str, count: i32, frames: u64, body_override: &str) {
    for &k in scales {
        // **The scene branch comes first, before `build`.** A hand-built
        // scene has no worldgen preset of its own name, so generating one
        // to throw away would panic on the lookup rather than merely waste
        // the work.
        if is_scene(preset) {
            // A hand-built scene owns its own placement: `colony_ant_site`
            // is a *surface* finder and there is no surface inside a
            // tunnel, so the generated path below would place nothing and
            // report a clean `blocked NaN` from a probe that never reached
            // the mechanism.
            let (mut scene, placed, first_body_cells) = match preset {
                "tunnel" => tunnel_scene(species, count, body_override),
                _ => chamber_scene(species, count, body_override),
            };
            for _ in 0..frames {
                parallel::step(&mut scene);
                scene.step_active_sites();
                scene.step_fields();
            }
            report(&scene, k, placed, first_body_cells);
            continue;
        }
        let mut world = build(k, seed, preset);
        apply_body_override(&mut world, species, body_override);
        let cols: Vec<i32> = (0..WIDTH as i32 * k).filter(|&x| creature::colony_ant_site(&world, x, 0).is_some()).collect();
        assert!(cols.len() >= count as usize * 2, "only {} viable columns at k={k}", cols.len());
        let mut placed = 0;
        // **The first placement's own cell count, read off the world the
        // same way `size_mode` does (`body_cells`'s own doc: off the world,
        // not off `BodyPlan`).** `PIXEL_PHYSICS_BODY_LATERALS=0` must move
        // this number for a `Segmented` species and leave it alone for a
        // plain `Chain` -- CLAUDE.md's stale-switch tell is identical
        // output across a change that must have moved something, and this
        // is the cheapest place to catch that before trusting the blocked%
        // below at all.
        let mut first_body_cells: Option<usize> = None;
        for i in 0..count {
            let x = cols[(i as usize * cols.len()) / count as usize];
            let Some(sy) = creature::colony_ant_site(&world, x, 0) else { continue };
            if let Some(site) = creature::plant_creature_seed(&mut world, x, sy - 1, species) {
                world.schedule_active_site(site);
                placed += 1;
                if first_body_cells.is_none() {
                    let id = world.get(x, sy - 1).organism_id();
                    first_body_cells = Some(body_cells(&world, id).len());
                }
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
        report(&world, k, placed, first_body_cells);
    }
}

/// The row: the blocked fraction this harness has always printed, and --
/// when `PIXEL_PHYSICS_BLOCKED_CENSUS=1` -- **why** it is what it is.
///
/// **`boxed` is the number this instrument was built for, not `blocked`.**
/// A blocked tick that `tumble` can fix by re-aiming costs one tick; a tick
/// on which none of the eight headings can be walked costs the animal
/// everything after it. `blocked%` cannot separate those and has been
/// asked to for three rounds. `boxed_self` splits the second again: boxed,
/// with at least one heading refused by nothing but this body's own cells
/// -- which is the reverse a short animal makes and a long one cannot.
fn report(world: &World, k: i32, placed: i32, first_body_cells: Option<usize>) {
    let s = world.creature_stats;
    let attempts = s.moves + s.moves_blocked;
    let blocked = if attempts == 0 { f64::NAN } else { s.moves_blocked as f64 / attempts as f64 };
    println!(
        "  k={k} placed={placed} body_cells={} alive={} ticks={} moves={} blocked={} => blocked {:.1}%  falls={} digs={} impulses={} flight_moves={}",
        first_body_cells.map(|n| n.to_string()).unwrap_or_else(|| "?".to_string()),
        world.live_creature_count(),
        s.ticks,
        s.moves,
        s.moves_blocked,
        blocked * 100.0,
        s.falls,
        s.digs,
        s.impulses,
        s.flight_moves,
    );
    let total: u64 = s.blocked_why.iter().sum();
    if total == 0 {
        println!("    (blocked census off -- set PIXEL_PHYSICS_BLOCKED_CENSUS=1)");
        return;
    }
    let why: Vec<String> = BlockedWhy::NAMES
        .iter()
        .zip(s.blocked_why.iter())
        .map(|(n, &v)| format!("{n} {v} ({:.1}%)", 100.0 * v as f64 / total as f64))
        .collect();
    println!("    why: {}", why.join("  "));
    let bx = |v: u64| if s.moves_blocked == 0 { f64::NAN } else { 100.0 * v as f64 / s.moves_blocked as f64 };
    // **Deaths by cause, beside the mobility numbers.** A mobility arm
    // whose animals died is not a mobility measurement, and `alive=` alone
    // cannot say whether they starved (which a scene with no food does to
    // everybody) or lost a head (which would be this rule breaking a body).
    let deaths: Vec<String> = pixel_physics::sim::organism::DEATH_CAUSE_LIST
        .iter()
        .filter(|c| world.deaths_by_cause[c.index()] > 0)
        .map(|c| format!("{c:?} {}", world.deaths_by_cause[c.index()]))
        .collect();
    println!("    deaths: {}", if deaths.is_empty() { "none".to_string() } else { deaths.join("  ") });
    println!(
        "    reversals={} refused={}  (PIXEL_PHYSICS_REVERSE={})",
        s.reversals,
        s.reversals_refused,
        std::env::var("PIXEL_PHYSICS_REVERSE").unwrap_or_else(|_| "off (default)".to_string()),
    );
    println!(
        "    boxed={} ({:.1}% of blocked ticks)  boxed_self={} ({:.1}%)  width_changes={} ({:.3}/move)  tucked_segment_steps={}",
        s.boxed_ticks,
        bx(s.boxed_ticks),
        s.boxed_self_ticks,
        bx(s.boxed_self_ticks),
        s.width_changes,
        if s.moves == 0 { f64::NAN } else { s.width_changes as f64 / s.moves as f64 },
        s.tucked_segment_steps,
    );
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
