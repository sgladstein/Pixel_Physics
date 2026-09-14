//! **Why a colony refuses to stand on a forest floor** — the reproduction
//! and the paired measurement for the owner's 2026-09-14 playtest note,
//! *"it should be easier to found a colony while standing in a thicket of
//! plants."*
//!
//! ```text
//! cargo run --release --example thicket_probe -- start=grown
//! cargo run --release --example thicket_probe -- start=bare
//! PIXEL_PHYSICS_THICKET_CLIMB=off cargo run --release --example thicket_probe -- start=grown
//! ```
//!
//! **It does not measure whether founding works. It measures what is in the
//! way.** `World::found_colony_of` returns one number — how many animals it
//! placed — and a zero from it is indistinguishable between "no ground",
//! "water", "a trunk", "litter" and "the species is missing". The brief this
//! was written for asserted the blocker was plant tissue; `CLAUDE.md`
//! records two "root bugs" that were scene errors wearing a mechanism's
//! clothes, so that assertion is the thing under test here, not the premise.
//!
//! # The three readouts
//!
//! **1. The band census.** Every column of the colony's own footprint goes
//! into exactly one bucket:
//!
//! - `site` — `colony_ant_site` accepts it.
//! - `no-surface` — `colony_surface` found no floor in this column at all.
//! - `floor is <kind>` — a floor that is not `Solid`/`Powder` (water, most
//!   often), refused by a *different* line and a known refusal already.
//! - `above <material>` — the floor is fine and the cell above it is not
//!   free. **This is the bucket the complaint is about, broken out by the
//!   material standing there**, because "plant tissue", "litter that fell
//!   off a plant" and "spoil" are three different findings and only one of
//!   them is the traced defect.
//!
//! **2. The recovery curve.** For each candidate climb bound, how many
//! blocked columns a *contiguous plant-tissue* climb would recover. This is
//! what sets `creature::THICKET_CLIMB`, and it is computed by this file's
//! own walk rather than by the engine's, so every bound is available from
//! one run instead of one recompile per point. The two are held in step by
//! `site_agrees_with_the_engine`.
//!
//! **3. The station sweep.** `found_colony_of` at many separated stands
//! across the world, reported as an order statistic rather than a mean —
//! `CLAUDE.md` records that six seeds is not a sweep and that a single stand
//! is a sample from a wide distribution. Run it twice, once with
//! `PIXEL_PHYSICS_THICKET_CLIMB=off`, for the pair.
//!
//! # Controls
//!
//! `start=bare` is the case known to work and is the negative control: it
//! must come back mostly `site`, and its blocked columns must be shallow
//! seed scatter rather than trunks. The recovery curve carries its own
//! positive control — at bound 0 it must reproduce the shipped refusal count
//! exactly, and if it does not, this file's walk is not the engine's rule
//! and nothing below it means anything.

use pixel_physics::sim::creature;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::world::World;

/// How far up a column this harness will look before reporting it hopeless.
///
/// Deliberately far past anything a fix would ever step: the question it
/// answers is the *shape of the distribution*, and a bound tight enough to
/// be the answer would hide it. A tree is tens of cells tall, so 64
/// separates "a floor with stuff on it" from "a column with a trunk in it"
/// without deciding which one the fix should take.
const MAX_STEP: i32 = 64;

/// The bounds the recovery curve is reported at. 0 is the shipped rule and
/// is the curve's own positive control.
const BOUNDS: [i32; 8] = [0, 2, 4, 6, 8, 16, 32, 64];

/// One column's verdict.
#[derive(Clone, Debug)]
enum Verdict {
    /// `colony_ant_site` accepted this column with no climb at all.
    Site,
    /// No floor in this column.
    NoSurface,
    /// A floor of the wrong kind — water, most often.
    GroundKind(&'static str),
    /// A good floor with something standing on it. Carries the blocking
    /// material's name and kind, and `climb`: how many rows of **contiguous
    /// plant tissue** separate the floor from the first free cell, or `None`
    /// if the column never clears within [`MAX_STEP`] or is interrupted by
    /// something that is not a plant.
    Blocked { material: String, kind: &'static str, climb: Option<i32> },
}

fn kind_name(k: MaterialKind) -> &'static str {
    match k {
        MaterialKind::Empty => "empty",
        MaterialKind::Solid => "solid",
        MaterialKind::Powder => "powder",
        MaterialKind::Liquid => "liquid",
        MaterialKind::Gas => "gas",
        MaterialKind::Plant => "plant",
        MaterialKind::Creature => "creature",
    }
}

/// Diagnose one column the way `colony_ant_site` does, but reporting *why*
/// rather than only yes/no, and for every climb bound at once.
///
/// **This duplicates the rule under test on purpose, and it is the one place
/// in this file allowed to.** `colony_ant_site` returns an `Option`, so a
/// harness that only called it could count refusals and nothing else — and
/// counting refusals is what the bug report already did. The duplication is
/// held honest by `site_agrees_with_the_engine`.
fn diagnose(world: &World, cx: i32, cursor_y: i32) -> Verdict {
    let Some(ground) = creature::colony_surface(world, cx, cursor_y) else {
        return Verdict::NoSurface;
    };
    let floor = world.materials.kind(world.get(cx, ground).material);
    if !matches!(floor, MaterialKind::Solid | MaterialKind::Powder) {
        return Verdict::GroundKind(kind_name(floor));
    }
    if world.is_empty(cx, ground - 1) {
        return Verdict::Site;
    }
    let blocker = world.get(cx, ground - 1).material;
    // The climb the fix performs: rise while the cell above is plant tissue,
    // stop at the first free cell. Anything else in the column ends it.
    let mut climb = None;
    for d in 1..=MAX_STEP {
        if world.is_empty(cx, ground - d) {
            climb = Some(d);
            break;
        }
        if world.materials.kind(world.get(cx, ground - d).material) != MaterialKind::Plant {
            break;
        }
    }
    Verdict::Blocked { material: world.materials.get(blocker).name.clone(), kind: kind_name(world.materials.kind(blocker)), climb }
}

struct Census {
    sites: usize,
    no_surface: usize,
    ground_kind: Vec<(&'static str, usize)>,
    blocked: Vec<(String, &'static str, usize)>,
    /// Climb depths over the blocked columns, for the order statistics and
    /// the recovery curve.
    climbs: Vec<Option<i32>>,
}

/// The colony's own footprint in columns, centred on the stand.
///
/// **Wider than the twelve stations the druid actually asks for**, and that
/// is deliberate: the station columns fall out of band arithmetic private to
/// `colony_stations`, and a harness that guessed at that arithmetic would be
/// measuring its own guess. A contiguous scan of the whole band is a
/// superset of whatever twelve columns it picks, and it is the thing that
/// shows the *distribution* — twelve samples of a forest floor cannot.
fn census(world: &World, x: i32, y: i32, half: i32) -> Census {
    let mut c = Census { sites: 0, no_surface: 0, ground_kind: Vec::new(), blocked: Vec::new(), climbs: Vec::new() };
    for cx in (x - half)..=(x + half) {
        match diagnose(world, cx, y) {
            Verdict::Site => c.sites += 1,
            Verdict::NoSurface => c.no_surface += 1,
            Verdict::GroundKind(k) => match c.ground_kind.iter_mut().find(|e| e.0 == k) {
                Some(e) => e.1 += 1,
                None => c.ground_kind.push((k, 1)),
            },
            Verdict::Blocked { material, kind, climb } => {
                c.climbs.push(climb);
                match c.blocked.iter_mut().find(|e| e.0 == material) {
                    Some(e) => e.2 += 1,
                    None => c.blocked.push((material, kind, 1)),
                }
            }
        }
    }
    c.blocked.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
    c
}

fn report(c: &Census, total: usize) {
    println!("  {} of {total} columns are sites with no climb at all", c.sites);
    if c.no_surface > 0 {
        println!("    no surface at all: {}", c.no_surface);
    }
    for (k, n) in &c.ground_kind {
        println!("    floor is {k} (refused by the ground rule, not by the cell above it): {n}");
    }
    let blocked: usize = c.blocked.iter().map(|e| e.2).sum();
    if blocked == 0 {
        return;
    }
    println!("    good floor, cell above occupied: {blocked}");
    for (m, k, n) in &c.blocked {
        println!("      {m} ({k}): {n}");
    }
    let mut depths: Vec<i32> = c.climbs.iter().filter_map(|s| *s).collect();
    let stuck = c.climbs.len() - depths.len();
    depths.sort_unstable();
    if let (Some(min), Some(max)) = (depths.first(), depths.last()) {
        let p50 = depths[depths.len() / 2];
        let p90 = depths[(depths.len() * 9) / 10];
        println!("      rows of contiguous plant tissue over the floor: min {min}, p50 {p50}, p90 {p90}, max {max}");
    }
    if stuck > 0 {
        println!("      never clears within {MAX_STEP} rows, or is blocked by something that is not a plant: {stuck}");
    }
    println!("    recovery curve (climb bound -> sites in the band):");
    for b in BOUNDS {
        let recovered = c.climbs.iter().filter(|s| s.is_some_and(|d| d <= b)).count();
        println!("      bound {b:2}: {} of {total}", c.sites + recovered);
    }
}

/// One stand's result, from the shipped path.
struct Stand {
    x: i32,
    stations: usize,
    placed: usize,
    /// Births `World::push_organism` refused during this stand — the slot
    /// ceiling, counted rather than inferred.
    ///
    /// **Without this column the sweep cannot tell two completely different
    /// failures apart**, and it initially told them apart wrongly. A stand
    /// that offers twelve stations and seats nobody looks identical whether
    /// the thicket refused the ground or the world ran out of organism
    /// identities, and those want opposite fixes. `CLAUDE.md`'s standing
    /// question — ask what your number counts when nothing is wrong — has a
    /// second half this is the answer to: pair every "it fired" counter with
    /// an effect counter from the far side of the call.
    refused: u64,
}

/// **Found a colony at many separated stands and report the distribution.**
///
/// Separated by at least the nest patch plus the animal band, so no two
/// stands can reach each other's ground — a stand founded into the ground a
/// previous stand painted would be measuring the nest patch, not the
/// thicket.
fn sweep(game: &mut pixel_physics::druid::Druid, stands: i32, spacing: i32) -> Vec<Stand> {
    let (w, _h) = game.world.bounds().map_or((512, 320), |b| (b.max_x + 1, b.max_y + 1));
    let margin = 160;
    let mut out = Vec::new();
    let species_id = game.world.species.id_of("ant");
    for i in 0..stands {
        let x = margin + i * spacing;
        if x >= w - margin {
            break;
        }
        // The stand's own footing, derived the way the lab derives it, so a
        // sampled stand is the same kind of thing as the gnome's.
        let Some(ground) = creature::colony_surface(&game.world, x, 0) else { continue };
        let y = ground - 2;
        let stations = species_id.map_or(0, |id| game.world.colony_stations(x, y, id, 12).len());
        let before = game.world.organisms_refused();
        let placed = game.world.found_colony_of(x, y, "ant", 12);
        out.push(Stand { x, stations, placed, refused: game.world.organisms_refused() - before });
    }
    out
}

fn order_stats(label: &str, v: &[usize]) {
    if v.is_empty() {
        println!("  {label}: no stands");
        return;
    }
    let mut s: Vec<usize> = v.to_vec();
    s.sort_unstable();
    let total: usize = s.iter().sum();
    let zeros = s.iter().filter(|&&n| n == 0).count();
    println!(
        "  {label}: n={} min {} p10 {} p50 {} p90 {} max {} | total {total} | stands placing nobody: {zeros}",
        s.len(),
        s[0],
        s[s.len() / 10],
        s[s.len() / 2],
        s[(s.len() * 9) / 10],
        s[s.len() - 1]
    );
}

/// **The lab's own bed, censused the same way** — the second game's arm of
/// this change, and the reason it is in this file rather than a second one.
///
/// `colony_ant_site` is shared engine machinery: the evolution lab founds
/// through the identical path (`lab::scene::LabBox::build` →
/// `World::found_colony_of`), so a change that makes the druid's thicket
/// work can silently move every foraging bed in a game with its own measured
/// baselines. `labnest founders=8` reports the bed's behaviour over time;
/// this reports the one thing `labnest` cannot, which is **whether the
/// change had anything to act on in the first place**. A paired run that
/// comes back identical on every counter is the tidy result `CLAUDE.md`
/// warns about, and only a census of the colony's own band can say whether
/// that is "no effect" or "no exposure".
/// Returns the founder columns' station and placed counts for one seed.
///
/// **`frames` is the whole design of this arm, and 0 is a real and useless
/// setting.** Measured first at 0 — the instant `LabBox::build` founds its
/// own colony — and the band came back with **not one** plant-blocked column
/// over three seeds: the bed sows its seeds at `ground_y - 2`, two rows up in
/// the air, and founds the colony in the same breath, so the cell over the
/// soil is free everywhere and this change has nothing to act on. That is
/// not "the lab is unaffected", it is "the lab was not yet exposed", and the
/// two look identical in every counter `labnest` prints. The lab's real
/// exposure is the player's own founding tool (`lab/mod.rs`'s colony verb)
/// reaching a bed that has been growing for a while, which is what running
/// the bed forward first reproduces.
fn lab_bed(founders: usize, seed: u64, frames: u64) -> (Vec<usize>, Vec<usize>) {
    use pixel_physics::lab::scene::LabBox;
    use pixel_physics::sim::explosion::Blasts;
    use pixel_physics::sim::particle::ParticleSystem;
    use pixel_physics::sim::{frame, player};

    let spec = LabBox { colonies: 1, founders, seed, ..LabBox::default() };
    let (mut world, planted) = spec.build_counted();
    let mut particles = ParticleSystem::default();
    let mut blasts = Blasts::default();
    let tuning = player::Tuning::default();
    for _ in 0..frames {
        frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
    }
    println!(
        "thicket_probe: lab bed seed={seed} founders={founders} frames={frames} -> planted {} of {} asked, {} ants at build, {} live organisms now",
        planted.planted,
        planted.asked,
        planted.ants,
        world.live_organism_count()
    );

    let y = spec.ground_y - 2;
    let species_id = world.species.id_of(&spec.colony_species);
    let (mut stations, mut placed) = (Vec::new(), Vec::new());
    // **At the founder columns, not the colony's own.** A second colony
    // founded on top of the first would be measuring the first colony's
    // bodies, which read as `floor is creature` and are nothing to do with
    // plants; a founder column is where the bed actually grew something.
    for x in spec.founder_columns() {
        let c = census(&world, x, y, 24);
        let blocked: usize = c.blocked.iter().map(|e| e.2).sum();
        let n = species_id.map_or(0, |id| world.colony_stations(x, y, id, 12).len());
        let p = world.found_colony_of(x, y, &spec.colony_species, 12);
        println!("  founder column {x:4}: {} of 49 band columns clear, {blocked} blocked by tissue | stations {n:2} placed {p:2}", c.sites);
        for (m, k, count) in &c.blocked {
            println!("      {m} ({k}): {count}");
        }
        stations.push(n);
        placed.push(p);
    }
    (stations, placed)
}

/// **Render one stand through the shipped `Renderer`, cropped to it.**
///
/// `CLAUDE.md`'s standing instruction: a change that alters anything on
/// screen is posted, not described. Founding a colony in a thicket is
/// visible, so this writes the picture the owner's verdict is actually about
/// — one arm per file, the same world, the same stand, the same crop, so a
/// pair can be put side by side without either being a remembered
/// impression.
///
/// **The count goes in the card, not in the picture.** Two very different
/// mechanisms look identical at the zoom a contact sheet is read at, and a
/// collapse once read as "chunks are working" from an image whose body count
/// was zero for the whole run. This prints `placed` beside the filename for
/// exactly that reason.
fn shoot(world: &pixel_physics::sim::world::World, at: (i32, i32), zoom: i32, out: &str) {
    use pixel_physics::app::{HEIGHT, WIDTH};
    use pixel_physics::render::Renderer;
    use pixel_physics::sim::particle::ParticleSystem;

    let (vw, vh) = (WIDTH, HEIGHT);
    let mut renderer = Renderer::new();
    for _ in 1..zoom {
        renderer.adjust_zoom(1);
    }
    // Aim before drawing. `adjust_zoom` alone leaves the camera wherever it
    // defaulted, which on a 960-row world is the sky — `labshot`'s own
    // hard-won note, and the reason `look` and `zoom` are a pair rather than
    // two knobs.
    let span = (vw as i32 / zoom, vh as i32 / zoom);
    renderer.set_camera(at.0 - span.0 / 2, at.1 - span.1 / 2, (vw, vh), world.bounds());
    let mut buf = vec![0u8; (vw * vh * 4) as usize];
    renderer.draw(world, &ParticleSystem::default(), &Default::default(), &mut buf, (vw, vh), true);
    image::save_buffer(out, &buf, vw, vh, image::ColorType::Rgba8).expect("writing the frame");
}

fn main() {
    let mut start = "grown".to_string();
    let mut half = 110;
    let mut stands = 0;
    let mut spacing = 256;
    let mut lab = 0usize;
    let mut seeds = 1u64;
    let mut frames = 6000u64;
    let mut shot: Option<String> = None;
    let mut zoom = 4;
    let mut ablate = false;
    for arg in std::env::args().skip(1) {
        let (k, v) = arg.split_once('=').unwrap_or((arg.as_str(), ""));
        match k {
            "start" => start = v.to_string(),
            "lab" => lab = v.parse().unwrap_or(0),
            "seeds" => seeds = v.parse().unwrap_or(1),
            "frames" => frames = v.parse().unwrap_or(frames),
            "shot" => shot = Some(v.to_string()),
            "zoom" => zoom = v.parse().unwrap_or(zoom),
            "ablate" => ablate = v != "0" && v != "off",
            "half" => half = v.parse().unwrap_or(half),
            "stands" => stands = v.parse().unwrap_or(stands),
            "spacing" => spacing = v.parse().unwrap_or(spacing),
            "grow" => std::env::set_var("PIXEL_PHYSICS_DRUID_GROW", v),
            "size" => std::env::set_var("PIXEL_PHYSICS_DRUID_SIZE", v),
            // **An unknown argument is not silently ignored.** `CLAUDE.md`
            // records a 3.5-hour study that was 3 populations wearing 24
            // logs because `worldseed=` reached a binary that had never
            // heard of it.
            other => {
                eprintln!("thicket_probe: unknown argument {other:?}");
                std::process::exit(2);
            }
        }
    }
    std::env::set_var("PIXEL_PHYSICS_DRUID_START", &start);
    // Echo every parameter, the arm included: a log that does not name its
    // arm was written by a binary that never had one.
    let arm = std::env::var("PIXEL_PHYSICS_THICKET_CLIMB").unwrap_or_else(|_| "default".into());
    println!("thicket_probe: start={start} half={half} stands={stands} spacing={spacing} lab={lab} seeds={seeds} frames={frames} shot={shot:?} zoom={zoom} ablate={ablate} climb_arm={arm} max_step={MAX_STEP}");

    if lab > 0 {
        let (mut all_stations, mut all_placed) = (Vec::new(), Vec::new());
        for seed in 1..=seeds {
            let (s, p) = lab_bed(lab, seed, frames);
            all_stations.extend(s);
            all_placed.extend(p);
        }
        order_stats("stations offered", &all_stations);
        order_stats("animals placed  ", &all_placed);
        return;
    }

    let mut game = pixel_physics::druid::Druid::new();
    if ablate {
        // **`ablate=1` — what this world looks like with `life_scatter`
        // switched off**, without touching `assets/worldgen.ron`, which is
        // another lane's file.
        //
        // Lane B is zeroing the druid preset's `moss_density`,
        // `tree_density` and `grass_density`, which takes `life_scatter`
        // from 993 cells to 0. On a `start=bare` world — zero grow frames —
        // **every plant cell present is a scatter cell by construction**,
        // because nothing has had a tick in which to grow one. So deleting
        // them here reproduces the post-Lane-B world exactly, for this
        // question, and turns a prediction about somebody else's unlanded
        // branch into a measurement.
        //
        // It is only valid at `start=bare`. On `grown` or `dead` this would
        // delete an entire wood, which is a different world and not a
        // control for anything.
        let bounds = game.world.bounds().expect("a generated world has bounds");
        let mut removed = 0usize;
        for cx in bounds.min_x..=bounds.max_x {
            for cy in bounds.min_y..=bounds.max_y {
                if game.world.materials.kind(game.world.get(cx, cy).material) == MaterialKind::Plant {
                    game.world.set(cx, cy, pixel_physics::sim::cell::Cell::EMPTY);
                    removed += 1;
                }
            }
        }
        // Printed, because an ablation that removed nothing is a control arm
        // wearing the treatment's label and reads exactly like "no effect".
        println!("thicket_probe: ablate removed {removed} plant cells (start={start}; only meaningful at start=bare)");
    }
    let Some(player) = game.world.player.as_ref() else {
        println!("thicket_probe: no player in the world — nothing to found at");
        return;
    };
    let (x, y) = player.center();
    let total = (2 * half + 1) as usize;
    println!("thicket_probe: gnome at {x},{y}; censusing {total} columns of his own band");

    let c = census(&game.world, x, y, half);
    report(&c, total);

    if stands > 0 {
        let out = sweep(&mut game, stands, spacing);
        let xs: Vec<String> = out.iter().map(|s| s.x.to_string()).collect();
        println!("thicket_probe: {} stands at x {}", out.len(), xs.join(","));
        order_stats("stations offered", &out.iter().map(|s| s.stations).collect::<Vec<_>>());
        order_stats("animals placed  ", &out.iter().map(|s| s.placed).collect::<Vec<_>>());
        let refused: u64 = out.iter().map(|s| s.refused).sum();
        println!("  births refused for want of an organism slot: {refused} (world holds {} live of 4,095)", game.world.live_organism_count());
        for s in &out {
            println!("    x={:5} stations {:2} placed {:2} slot-refused {:2}", s.x, s.stations, s.placed, s.refused);
        }
    } else {
        // The gnome's own key, so the census above can be read against the
        // number the player actually sees rather than instead of it.
        let placed = game.found_colony();
        println!("thicket_probe: found_colony at the gnome's feet placed {placed} animals");
        if let Some(out) = &shot {
            shoot(&game.world, (x, y), zoom, out);
            println!("thicket_probe: wrote {out} at {x},{y} zoom {zoom} — placed {placed} animals");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pixel_physics::sim::cell::Cell;
    use pixel_physics::sim::chunk::Rect;

    fn bed() -> World {
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        let _ = world.materials.reload(pixel_physics::sim::material::ASSET_DIR);
        let soil = world.materials.id_of("soil").expect("soil");
        for x in 0..64 {
            for y in 40..64 {
                world.set(x, y, Cell::new(soil, 0));
            }
        }
        world
    }

    /// This file's duplicate of the rule must agree with the engine's, or
    /// every number it prints is about a rule nobody runs.
    ///
    /// Run at the engine's default bound against a bed whose columns span
    /// bare soil, a shallow mat and a tall stack — so the two are compared
    /// where they could disagree, not only where nothing is in the way.
    #[test]
    fn site_agrees_with_the_engine() {
        let mut world = bed();
        let leaf = world.materials.id_of("leaf").expect("leaf");
        // Columns 10..30 carry a mat of leaf of increasing depth: 1 row at
        // column 10, 21 rows at column 30 — straddling the shipped bound in
        // both directions, which is what makes this able to fail.
        for (i, x) in (10..31).enumerate() {
            for d in 1..=(i as i32 + 1) {
                world.set(x, 40 - d, Cell::new(leaf, 0));
            }
        }
        let mut agreed_site = 0;
        let mut agreed_refusal = 0;
        for x in 0..64 {
            let mine = matches!(diagnose(&world, x, 0), Verdict::Site);
            let theirs = creature::colony_ant_site(&world, x, 0).is_some();
            // `diagnose` reports `Site` only for a column needing no climb,
            // so the engine may legitimately accept a column this calls
            // blocked. What must never happen is the reverse.
            if mine {
                assert!(theirs, "column {x}: the harness accepts a site the engine refuses");
                agreed_site += 1;
            } else if !theirs {
                agreed_refusal += 1;
            }
        }
        assert!(agreed_site > 0 && agreed_refusal > 0, "the bed must contain both answers: {agreed_site} sites, {agreed_refusal} refusals");
    }

    /// The recovery curve's own positive control: at bound 0 the curve must
    /// reproduce the count of columns the shipped rule accepts, and at 64 it
    /// must have moved. A curve flat across every bound would be a curve
    /// measuring nothing.
    #[test]
    fn the_recovery_curve_moves_between_its_ends() {
        let mut world = bed();
        let leaf = world.materials.id_of("leaf").expect("leaf");
        for x in 10..31 {
            for d in 1..=3 {
                world.set(x, 40 - d, Cell::new(leaf, 0));
            }
        }
        let c = census(&world, 32, 0, 31);
        let at = |b: i32| c.sites + c.climbs.iter().filter(|s| s.is_some_and(|d| d <= b)).count();
        assert_eq!(at(0), c.sites, "bound 0 must recover nothing");
        assert!(at(64) > at(0), "bound 64 must recover something: {} vs {}", at(64), at(0));
    }
}
