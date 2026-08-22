//! Renders a scene to a **contact sheet**: several frames of one run laid out
//! in a grid, through the same `Renderer` the sandbox itself draws with, with
//! no window and no GPU.
//!
//! This exists because a numeric metric only ever sees the one quantity it was
//! written to see, and choosing that quantity correctly turns out to be the
//! hard part. Repeatedly, a metric written before anyone had looked at the
//! artifact measured the wrong thing and reported "nothing here" on a scene
//! that visibly had something: surface height hid a 9x chunk-seam effect that
//! column volume showed plainly, and occupancy hid torn seam rows that were a
//! fill deficit rather than a hole. Worse, a metric cannot see what it was not
//! asked about, so a fix that cleared one artifact while introducing a larger
//! one passed its own test and had to be reverted from live play
//! (`e816477`). An image has the opposite property: it shows everything at
//! once, including whatever nobody thought to measure.
//!
//! `examples/ascii.rs` already makes this argument ("movement rules are far
//! easier to judge by eye than by assertion") and answers it in the terminal.
//! This answers it in actual pixels, which is what the bug reports are about.
//!
//! # Why a grid rather than a GIF
//!
//! `main.rs`'s capture hook already writes a `sequence.gif`, and it is the
//! right thing for a human watching it. But several of these bugs are only
//! legible *in motion* — a fringe that regenerates every frame reads very
//! differently from one that sits still — and an animation cannot be taken in
//! at a glance or quoted in a report. Laying time out along space solves both:
//! one image, one look, motion visible as difference between neighbouring
//! tiles.
//!
//! # Why crop and zoom are not optional extras
//!
//! At 1:1 a one-pixel-tall dark line across a 512-wide world is genuinely
//! easy to miss — that is exactly how the horizontal chunk-seam tearing went
//! unnoticed until it was pointed out. `crop` and `zoom` are the difference
//! between "I looked at it" and "I saw it".
//!
//! ```text
//! cargo run --release --example filmstrip -- scene=pour start=100 every=60 count=6
//! cargo run --release --example filmstrip -- scene=pour crop=160,180,120,80 zoom=4
//! ```

use std::collections::HashSet;

use pixel_physics::render::{FieldOverlay, GrainMode, OrganismOverlay, Renderer, TreeDepth};
mod common;

use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::pheromone::{Channel, DEPOSIT};
use pixel_physics::sim::world::World;
use pixel_physics::sim::rng;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::{explosion, material, parallel, update};

/// Name the live `chain_reach` (`F9`) for the sheet.
///
/// `SPREAD` is `i32::MAX`, and printing that as a number tells a reader
/// nothing at all -- while a sheet that does not name the mode it was run
/// at cannot be compared with the one beside it, which is the whole point
/// of sweeping the setting.
fn chain_reach_name(reach: i32) -> String {
    pixel_physics::sim::structural::CHAIN_MODES
        .iter()
        .find(|m| m.reach == reach)
        .map_or_else(|| reach.to_string(), |m| m.name.to_string())
}

const WIDTH: i32 = 512;
const HEIGHT: i32 = 320;
const FLOOR_THICKNESS: i32 = 8;

/// Ground level for the plant scenes, in world rows from the top.
///
/// Chosen against `field.rs`'s measured light profile, not by eye: at the
/// current `LIGHT_DECAY` the reading crosses `Germinate`'s `0.1` threshold
/// roughly **75** cells below open sky, and `ambient_light_above` samples a
/// further `FIELD_SCALE` (8) rows *above* the cell it is asked about. `40`
/// leaves most of that band as headroom, so a seed here germinates on
/// light margin rather than on the edge of it -- and so a canopy growing
/// upward from here has somewhere lit to grow into. Do not deepen this
/// without re-reading `LIGHT_DECAY`'s own doc; a scene where nothing
/// germinates looks identical to a scene where growth is broken.
const TREE_GROUND_Y: i32 = 40;

/// Water with a varied `shade`, the way the brush lays it down
/// (`World::paint_capsule` rolls a random shade per cell). The scenes below
/// would otherwise use `Cell::new(WATER, 0)` and give every cell an
/// identical shade — which silently flattens `GrainMode::Cell` to no grain
/// at all, since that mode keys entirely off this byte. Worth knowing as a
/// real caveat of that mode and not just a harness detail: any water created
/// without a varied shade renders flat under it.
fn water_at(x: i32, y: i32) -> Cell {
    Cell::new(material::WATER, (rng::jitter(x, y) * 255.0) as u8)
}

fn stone_floor(w: &mut World) {
    for x in 0..WIDTH {
        // Terrain, so it declares itself attached the way `build_terrain`
        // does -- otherwise it is foreground material that has to hold
        // itself up, and a floor that is only anchored at the world edges
        // erodes inward from every free face.
        for y in (HEIGHT - FLOOR_THICKNESS)..HEIGHT {
            w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
}

/// The scenes the current bug list is about. Adding one is three lines, and
/// is much preferred to editing an existing one — a scene that quietly
/// changed underneath a recorded measurement is worse than no scene.
fn build(args: &Args) -> World {
    let mut w = World::new(Rect::new(0, 0, WIDTH - 1, HEIGHT - 1));
    // Set before the scene is built, because several scenes cut into the
    // world during construction and the rule has to be in force for that
    // cut as much as for the run that follows it.
    w.crush_confined = args.confine;
    w.arch_relief = args.arch;
    if let Some(reach) = args.chain_reach {
        w.chain_reach = reach;
    }
    // Before the scene is built, like the three above: `scene=worldgen`
    // cuts caves during construction, and a material property has to be in
    // force for that as much as for the run.
    if let Some(spacing) = args.joint_spacing {
        if let Some(stone) = w.materials.id_of("stone") {
            w.materials.get_mut(stone).joint_spacing = spacing.max(0.0);
        }
    }
    if let Some(contrast) = args.joint_bands {
        if let Some(stone) = w.materials.id_of("stone") {
            w.materials.get_mut(stone).joint_band_contrast = contrast.clamp(0.0, 0.9);
        }
    }
    let floor_y = HEIGHT - FLOOR_THICKNESS;
    match args.scene.as_str() {
        // A large body released against the left wall, spreading right across
        // seven vertical chunk seams. The terracing/banding reproduction.
        "pour" => {
            stone_floor(&mut w);
            for x in 0..200 {
                for y in 30..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
        }
        // Falling and spreading, rather than resting on the floor already:
        // the state the horizontal seam tearing shows up in.
        "fall" => {
            stone_floor(&mut w);
            for x in 20..250 {
                for y in 20..200 {
                    w.set(x, y, water_at(x, y));
                }
            }
        }
        // Stage 2 of the creature milestone: two synthetic trails on the
        // pheromone planes, over ordinary terrain, so the overlay ramps can
        // be judged **against a real signal**. An empty plane renders as a
        // flat ramp floor, which is correct and proves nothing -- the
        // failure this scene exists to catch is a trail that is present in
        // the data and unreadable on screen, which is exactly how the
        // canopy-density sheet came to read as blank.
        //
        // Look at it with `channel=pheromone_a` and `channel=pheromone_b`.
        "pheromone" => {
            stone_floor(&mut w);
            for x in 40..470 {
                for y in 200..floor_y {
                    w.set(x, y, Cell::new(material::SAND, 0));
                }
            }
            // Laid repeatedly, as ants would: a single deposit evaporates
            // before it spreads (measured -- see pheromone.rs's DIFFUSE).
            for i in 0..60u64 {
                for x in 60..450 {
                    w.deposit_pheromone(Channel::A, x, 190, DEPOSIT);
                    let y = 150 + (x - 60) / 12;
                    w.deposit_pheromone(Channel::B, x, y, DEPOSIT);
                }
                w.frame = i * 4;
                w.step_pheromones();
            }
        }
        // A dense blob dropped into a walled pool: the displacement striping.
        "blob" => {
            stone_floor(&mut w);
            for y in 0..floor_y {
                w.set(120, y, Cell::new(material::STONE, 0));
                w.set(392, y, Cell::new(material::STONE, 0));
            }
            for x in 121..392 {
                for y in 160..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
            w.paint_circle(256, 80, 34, material::SAND);
        }
        // Sand blobs dropped on a floor: the original chunk-seam cliffs.
        "sand" => {
            stone_floor(&mut w);
            w.paint_circle(120, 70, 40, material::SAND);
            w.paint_circle(250, 70, 40, material::SAND);
            w.paint_circle(380, 70, 24, material::SAND);
        }
        // M15 explosions. A settled sand pile with a stone slab buried in
        // it and open air above -- the geometry the owner's own live report
        // used ("two explosions in a sand pile"), plus enough solid
        // material for the crater edge and the fireball ring to read
        // against something that cannot avalanche. `explode=` fires into it.
        "boom" => {
            stone_floor(&mut w);
            for x in 100..412 {
                for y in 180..floor_y {
                    w.set(x, y, Cell::new(material::SAND, (rng::jitter(x, y) * 255.0) as u8));
                }
            }
            for x in 150..360 {
                for y in 230..244 {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
        }
        // The same blast against `Solid` only, so nothing can slump in and
        // hide what the blast itself actually did.
        "boom_stone" => {
            stone_floor(&mut w);
            for x in 100..412 {
                for y in 180..floor_y {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
        }
        // A deep, flat sand bed with nothing else in it -- for firing several
        // blasts at different *depths* below the free surface in one image.
        // The owner's own framing of the M15 complaint: material only blasts
        // around when the charge is near the edge, and "it just doesn't
        // happen if you're not really close."
        "sandbed" => {
            stone_floor(&mut w);
            for x in 20..492 {
                for y in 120..floor_y {
                    w.set(x, y, Cell::new(material::SAND, (rng::jitter(x, y) * 255.0) as u8));
                }
            }
        }
        // The same depth sweep in the other material the complaint named.
        // A liquid holds no angle of repose, so a crater here closes by
        // flowing rather than by avalanching.
        // A puddle and a lake side by side, the same depth, so the only
        // thing that differs is width -- `evaporation.rs`'s own paired
        // scene, so a sheet from this can be read against what the guards
        // measure rather than against a fresh scene nobody has a prior on.
        // Walled, because an open puddle spreads away across the floor
        // before the field registers it as a source at all, and the sheet
        // would then be a picture of spreading.
        //
        // Both bodies sit in *one* world here where the tests use one each,
        // and that is a real difference: the lake humidifies the puddle
        // (1.82 above the puddle against 1.45 for the same puddle alone),
        // so the puddle here dries somewhat slower than the guard's number.
        // Worth it -- one image showing both is the whole point.
        "evaporate" => {
            let floor = 160;
            for x in 0..WIDTH {
                for y in floor..(floor + 6) {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            for (x0, width) in [(40, 6), (120, 240)] {
                for y in (floor - 4)..floor {
                    w.set(x0 - 1, y, Cell::new(material::STONE, 0));
                    w.set(x0 + width, y, Cell::new(material::STONE, 0));
                    for x in x0..(x0 + width) {
                        w.set(x, y, water_at(x, y));
                    }
                }
            }
        }
        "waterbed" => {
            stone_floor(&mut w);
            for x in 20..492 {
                for y in 120..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
        }
        // M16 plants. A single seed on a stone shelf with a puddle beside
        // it -- deliberately the same geometry as the committed live
        // verification shots (`docs/screenshots/tree-rewrite-live-
        // verification/`), so a sheet from this scene can be compared
        // directly against the artifact the owner's "still a tiny tree, one
        // cell thick, ~18 cells, no leaves, no roots" report was made about
        // rather than against a fresh scene nobody has a prior on.
        //
        // **The shelf height is load-bearing, not cosmetic.** `Germinate`'s
        // light gate reads `field_at(x, y - FIELD_SCALE).light`, and
        // `field.rs`'s `LIGHT_DECAY` puts the `0.1` crossing roughly 75
        // world cells below open sky. The ordinary `stone_floor` at
        // `HEIGHT - 8` is 300+ cells down and a seed there never germinates
        // at all -- which is exactly how the ported tree tests started
        // failing when they kept the old system's y=100-150 planting depth
        // (see `PLAN.md`'s tree-rewrite step 7 entry). `TREE_GROUND_Y` sits
        // comfortably inside the lit band with headroom, per this repo's
        // "set bars from measurement with headroom" convention.
        "tree" => {
            for x in 0..WIDTH {
                for y in TREE_GROUND_Y..(TREE_GROUND_Y + 6) {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            w.plant_tree(200, TREE_GROUND_Y - 1);
            w.paint_circle(150, TREE_GROUND_Y - 4, 7, material::WATER);
        }
        // The same shelf, but a soil bed over a water table and several
        // seeds spaced across it. This is the scene the later phases are
        // aimed at -- roots growing into soil, soil moisture being drunk
        // down, canopies competing for light -- and today it should show
        // *none* of that, which is the point of shooting it now: it is the
        // before-picture for work that has not started.
        "forest" => {
            // `soil` has no `material::` constant of its own -- it was
            // appended to `EMBEDDED` deliberately (see that array's own
            // comment on why inserting rather than appending would
            // renumber the well-known ids), so it is looked up by name
            // like `wood` and `moss` already are elsewhere.
            let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");
            for x in 0..WIDTH {
                for y in (TREE_GROUND_Y + 40)..(TREE_GROUND_Y + 46) {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
                // Soil all the way down to the stone.
                //
                // An earlier version buried a band of free `water` in the
                // lower soil as a stand-in water table. That does not work
                // and the reason is worth keeping: `soil` is a `Powder` and
                // sinks through `Liquid`, so within a few hundred frames the
                // soil had swapped places with the water and the "table" was
                // a film lying on the *surface* — with every seed then
                // germinating onto water rather than soil, and no root ever
                // starting. Correct physics, useless scene, and it read as a
                // root bug.
                //
                // A real water table needs moisture held *inside* soil
                // cells, which is Decision 3 (§4a, per-cell fill in a
                // `Powder`'s own `aux`). Until that lands the honest scene
                // is dry soil, and the puddle below is on the surface where
                // it will stay put.
                // Starts at field capacity -- damp, the way real ground
                // between rain events is, and the state a root system
                // actually lives in. `Powder` aux is moisture now
                // (`material::SOIL_SATURATED`, where 0 means *dry*, the
                // opposite of a liquid's fill), and a scene of bone-dry
                // soil would sit below the wilting point where `Absorb`
                // correctly credits nothing at all.
                for y in TREE_GROUND_Y..(TREE_GROUND_Y + 40) {
                    w.set(x, y, Cell::new(soil, (rng::jitter(x, y) * 255.0) as u8).with_aux(material::SOIL_FIELD_CAPACITY));
                }
            }
            w.paint_circle(260, TREE_GROUND_Y - 3, 6, material::WATER);
            // Dropped from well above the ground on purpose: a seed is a
            // Powder, so this exercises the fall and landing rather than
            // pre-placing each seed on the surface.
            for x in [80, 200, 320, 440] {
                w.plant_tree(x, TREE_GROUND_Y - 25);
            }
        }
        // **A scene built for growing plants, rather than one built for
        // particle physics and reused.** `Reports/tree-architecture-
        // research.md` §6: every judgement about tree shape up to this
        // point was made in `forest`, which puts ground at y=40 in a
        // 320-tall world -- 40 rows of sky against 280 of dirt, because it
        // was laid out when depth was the interesting axis. Trees reached
        // that ceiling and could only spread sideways, and the resulting
        // silhouette was read as "canopies merge into a slab" and chased
        // as a plant bug for two sessions. Measured: at 40 rows of sky the
        // widest above-ground row is 56 cells; at 70 it is **7**.
        //
        // So this scene inverts the proportions -- a deep sky over a soil
        // bed just thick enough for a real root system -- and is the one
        // to judge plant *shape* in. `forest` is kept as it is: it is
        // still the right scene for root/soil work, and every earlier
        // sheet was shot in it.
        // Built from `common::PlantScene`, the same code `plant_probe`
        // uses -- see that module for why these two harnesses may not build
        // their own worlds any more.
        "grove" => {
            return common::PlantScene::default().build();
        }
        // `grove`, plus a gnome who walks the length of it once the trees
        // are actually trees. The one question it exists to answer: does he
        // *get through*, or does he wedge against the first trunk the way
        // he did before living tissue stopped being a wall.
        //
        // Read the **distance** printed beside the tile, not the picture. A
        // gnome standing against a trunk and a gnome standing in one are
        // the same few pixels at contact-sheet zoom, and the difference
        // between them is the entire change.
        "wood" => {
            let mut world = common::PlantScene::default().build();
            world.player = Some(pixel_physics::sim::player::Player::at(12, 190));
            return world;
        }
        // The same grown stand, but he walks until he has hold of a tree
        // and then goes up it. Read the **climbed** counter beside the
        // tile, not the picture: a gnome at the top of a tree and a gnome
        // shoved up there by the depenetration pass are the same few pixels
        // at this zoom, and only a number separates them.
        "climb" => {
            let mut world = common::PlantScene::default().build();
            world.player = Some(pixel_physics::sim::player::Player::at(12, 190));
            return world;
        }
        // Walk to a tree and shake it. Read the counters: a tree that shed
        // nothing and a shake that never fired are the same picture, and
        // `shake_shed` is graded by shade, so a healthy stand is *supposed*
        // to drop very little.
        "shake" => {
            let mut world = common::PlantScene::default().build();
            world.player = Some(pixel_physics::sim::player::Player::at(12, 190));
            return world;
        }
        // The sandbox's *real* starting terrain, built by the same
        // `app::build_terrain` the running game calls -- not a replica, so
        // what this renders is what a player actually sees. Exists to answer
        // one question by eye: with structural checks computed at generation
        // rather than skipped, does any of it move? The three ledges float
        // with no in-plane path to bedrock and stand only because 6 cells is
        // thicker than stone's confinement diameter, which is exactly the
        // claim `Reports/worldgen-design.md` §6b says would collapse the
        // world if it were wrong.
        // **A colony you can actually look at**, which until now did not
        // exist anywhere. Everything about ants had been judged by counters
        // -- 402 genomes of statistics and not one picture -- which is the
        // exact inversion `CLAUDE.md` opens by warning against. An image
        // says *what and where*; the numbers already say how much.
        //
        // `genome=` picks who is being drawn, by the same label
        // `creature_space` prints, so a row in that sweep and a sheet here
        // are the same animal (they share `brain::random_genome`). The three
        // worth looking at, from the 400-genome run:
        //
        //   genome=r029   short-range forager, the best survivor found
        //   genome=r017   long-range directed commuter
        //   genome=r059   sessile grazer -- never moves a cell, still eats
        //   genome=authored   the hand-tuned ant, which r029 beats
        //   genome=zero   cannot move at all; the floor, and the control
        //
        // Wetland, deliberately: it is the only preset where moss both
        // exists at ant height and replenishes (86 -> 1,194 cells over a
        // run, against 10 -> 10 on "rolling"), and moss is what makes the
        // sessile strategy possible at all. On dry terrain r059 would just
        // look like a dead ant, and the sheet would be evidence of nothing.
        //
        // Look at it with `channel=pheromone_b` to see the food trail form,
        // and `channel=pheromone_a` for the nest scent.
        //
        // **Capture at noon, or the sheet is unreadable.** The day/night
        // oscillator has a period of 3600 frames and frame 0 is noon
        // (`field::DAY_NIGHT_PERIOD_FRAMES`), and this scene's 2,400-frame
        // warmup leaves capture starting two thirds through the cycle -- in
        // the dark. The first sheet rendered was six tiles of night. Use
        // `start=1200 every=3600` so every tile lands at exactly noon and
        // the tiles differ by behaviour rather than by time of day:
        //
        //   cargo run --release --example filmstrip -- scene=colony         //     genome=r029 start=1200 every=3600 count=6 cols=3 zoom=2
        "colony" => {
            let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
            if let Some(e) = err {
                panic!("worldgen presets unavailable: {e}");
            }
            let params = presets.get("wetland").expect("the wetland preset");
            pixel_physics::worldgen::generate(&mut w, pixel_physics::worldgen::Spec::Generated { params, seed: args.seed });

            let species = w.species.id_of("ant").expect("ant species");
            let genome = match args.genome.as_str() {
                "authored" => w.species.get(species).genome.clone(),
                "zero" => vec![0.0; pixel_physics::sim::brain::GENOME_LEN],
                label => {
                    let n: u64 = label.trim_start_matches('r').parse().unwrap_or_else(|_| {
                        panic!("genome={label:?}: expected \"authored\", \"zero\", or \"rNNN\"")
                    });
                    pixel_physics::sim::brain::random_genome(pixel_physics::sim::brain::sweep_genome_seed(n))
                }
            };
            w.species.set_genome(species, genome);

            // Let the trees put leaves out before the ants arrive; a
            // seedling is not a food source, and a scene whose food has not
            // grown yet measures the warmup rather than the colony.
            for _ in 0..2400 {
                pixel_physics::sim::parallel::step(&mut w);
                w.step_active_sites();
                w.step_fields();
            }
            // Found it on the ground, the same call the `Y` key makes, so
            // the sheet shows what a player actually gets.
            // **Find DRY land, not just "the first thing from the top".**
            // The obvious version put the colony at mid-width, where this
            // seed happens to have a lake, so the surface it found was
            // water: 48 ants were placed on a surface they cannot stand on
            // and the contact sheet was a picture of a lake. Wetland has
            // water in it by definition -- that is the point of it -- so the
            // scene has to say which surface it wants.
            use pixel_physics::sim::material::MaterialKind;
            let dry_surface = |w: &World, x: i32| -> Option<i32> {
                let y = (0..HEIGHT).find(|&y| !matches!(w.materials.kind(w.get(x, y).material), MaterialKind::Empty | MaterialKind::Gas))?;
                matches!(w.materials.kind(w.get(x, y).material), MaterialKind::Solid | MaterialKind::Powder).then_some(y)
            };
            // Widest stretch of dry ground nearest mid-width, so the colony
            // has somewhere to walk rather than a two-cell island.
            // **Score the thing actually wanted: how many ants would land.**
            // Scoring "dry columns within 60 cells" instead was a proxy, and
            // it picked a plateau too narrow for the colony -- 31 of 52 ants
            // placed, the rest quietly dropped into a lake. Count the real
            // 52 placement sites.
            let would_place = |w: &World, x: i32| -> i32 {
                (0..52).filter(|i| dry_surface(w, x - 102 + i * 4).is_some()).count() as i32
            };
            // Only where the colony's whole 204-cell span fits inside the
            // world: `found_colony` centres 52 ants at spacing 4, and
            // founding it near an edge silently drops every ant that lands
            // outside (16 of 52, the first time this scene ran).
            let half_span = 102;
            let (cx, cy) = (half_span..WIDTH - half_span)
                .filter_map(|x| dry_surface(&w, x).map(|y| (x, y)))
                // Most dry ground within reach, ties broken toward the
                // middle of the map. A score rather than a hard window: on a
                // wetland seed there may be no unbroken 200-cell beach, and
                // demanding one made the scene panic rather than degrade.
                .max_by_key(|&(x, _)| (would_place(&w, x), -(x - WIDTH / 2).abs()))
                .expect("some dry ground");
            let placed = w.found_colony(cx, cy - 2);
            assert!(placed > 0, "the colony scene placed no ants -- the scene is not showing what it claims to");
            println!("scene=colony genome={} : {placed} ants founded at x={cx}, surface y={cy}", args.genome);
            println!("  suggested crop: crop={},{},240,110", cx - 120, cy - 70);
        }
        "terrain" => {
            pixel_physics::app::build_terrain(&mut w);
        }
        // **One dig into a generated world.** Reported from play: "one crack
        // in the ground basically propagates throughout the whole world and
        // slowly breaks everything."
        //
        // Kept separate from `scene=worldgen` rather than added to it as a
        // flag: that scene is worldgen's own, it asserts that a generated
        // world arrives at rest and *stays* there, and a scene that
        // sometimes digs would make its zero-failure reading mean two
        // different things.
        //
        // The cut lands on the surface at mid-width, found by walking down
        // rather than assumed -- generated terrain puts the surface
        // wherever it likes, and a dig into open air would reproduce
        // nothing while looking like it had.
        // M9: the gnome on an obstacle course -- flat floor, a 2-cell curb
        // the step-up should climb without a jump, and a 10-cell platform
        // the scripted full jump (`gnome_script`) should clear. Judged on
        // the contact sheet: is the arc an arc, does the landing look like
        // a landing, does he climb the curb without stuttering.
        "gnome" => {
            stone_floor(&mut w);
            for x in 150..170 {
                for y in (floor_y - 2)..floor_y {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            for x in 280..420 {
                for y in (floor_y - 10)..floor_y {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            w.player = Some(pixel_physics::sim::player::Player::at(40, floor_y - 4));
        }
        // M9 phase 2: the gnome tunnels into a cliff face, held-digging
        // while walking into his own bore. What this is read for, in
        // order: does a bore actually *open* (rubble and stone are near
        // the same grey, so a dig that only loosens looks identical to
        // one that never fired -- read the bite counter next to the
        // image), does the spoil come out of the mouth, and can he walk
        // over what he has thrown behind him. The massif is deep enough
        // that the tunnel never breaks through, so the whole run is the
        // confined case rather than the easy one.
        "tunnel" => {
            stone_floor(&mut w);
            for x in 180..WIDTH {
                for y in (floor_y - 90)..floor_y {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            w.player = Some(pixel_physics::sim::player::Player::at(150, floor_y - 4));
        }
        // M9's own acceptance line, verbatim: "player can be buried by a
        // sand dump and dig out". A column of sand drops on a standing
        // gnome, entombs him, and he digs his way clear -- the counters
        // report which tick he went under and which tick he came back,
        // because "buried" is a flag no picture can show.
        //
        // A *dump*, sized deliberately: 20x30 of sand, which settles to a
        // heap burying him about a dozen cells deep. Not a mountain --
        // the first version of this scene dropped 2240 cells and buried
        // him thirty deep, where no opening exists within any bounded
        // spoil throw and he stays under for good. That is the correct
        // outcome for being at the bottom of a hill (`BURIED_THROW` is
        // finite on purpose) but it is not the case M9 names, and a scene
        // that tests the unreachable one tells you nothing about the
        // reachable one.
        "bury" => {
            stone_floor(&mut w);
            for x in 246..266 {
                for y in 240..270 {
                    w.set(x, y, Cell::new(material::SAND, (x % 3) as u8));
                }
            }
            w.player = Some(pixel_physics::sim::player::Player::at(256, floor_y - 4));
        }
        // M9 phase 3: water. He is dropped into a walled pool from a
        // height and the script runs the three things swimming has to get
        // right in order -- he sinks under his own momentum, floats back
        // up rather than walking the bottom, is pulled under again by
        // held `S`, and finally jumps clear of the surface. The last of
        // those is the one that needed a mechanism of its own (the coyote
        // window while submerged); the rest is buoyancy and damping.
        "swim" => {
            stone_floor(&mut w);
            for y in 150..floor_y {
                for x in 180..190 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
                for x in 330..340 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            for x in 190..330 {
                for y in 190..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
            w.player = Some(pixel_physics::sim::player::Player::at(260, 120));
        }
        // M9's other acceptance line: "stands on a tumbling rigid body".
        // The `undercut` recipe with a passenger -- a shelf cut off its
        // support, which the structural pass promotes to a chunk body and
        // drops. He is standing on it when it goes. Bodies live off-grid,
        // so before phase 3 a grid read could not see one and the shelf
        // fell straight through him; what this sheet is read for is
        // whether he rides it down and is still on top when it lands.
        "ride" => {
            stone_floor(&mut w);
            for y in 120..260 {
                for x in 0..90 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            // A *short* shelf. The full 120-cell `undercut` span shatters
            // into twenty-odd bodies the moment it goes, which is correct
            // for that scene (it exists to show a big collapse) and
            // useless for this one: there is no raft to ride, only debris
            // to fall through. Sized down until it promotes as a couple
            // of coherent pieces.
            for y in 150..158 {
                for x in 90..186 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            pixel_physics::sim::structural::compute_world_distances(&mut w);
            for x in 92..184 {
                for y in 155..158 {
                    w.paint_capsule((x, y), (x, y), 0, material::EMPTY, 1.0);
                }
            }
            w.player = Some(pixel_physics::sim::player::Player::at(150, 143));
        }
        "worldcrack" => {
            let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
            if let Some(e) = err {
                panic!("{e}");
            }
            let name = if args.preset.is_empty() { presets.default_name() } else { args.preset.clone() };
            let Some(params) = presets.get(&name) else { panic!("unknown preset {name:?}") };
            pixel_physics::worldgen::generate(&mut w, pixel_physics::worldgen::Spec::Generated { params, seed: args.seed });
            let x = WIDTH / 2;
            let surface = (0..HEIGHT).find(|&y| w.get(x, y).material != material::EMPTY).expect("ground somewhere under mid-width");
            println!("worldcrack {name} seed {} -- cut at ({x}, {})", args.seed, surface + args.dig);
            // The verb matters more than the radius.  pulverizes a
            // core, loosens a chip zone *and* scores cracks out to three
            // times its radius -- and a crack severs a support path rather
            // than merely weakening it, so it manufactures pieces that have
            // to find a new way to the ground.  is the quiet cut. The
            // owner plays with the hammer, and nothing here covered it.
            // Can a tunnel be dug and stay open? The question the whole
            // milestone is about, and nothing here has ever asked it: every
            // scene cut *once*. A tunnel is the case where each bite stands
            // on the last one's spoil and under the last one's roof, so it
            // is the one that compounds.
            //
            // Driven in one burst rather than over time, deliberately: a
            // player digging at their own pace lets each bite settle before
            // the next, so this is the harsher reading of the same verb.
            if args.tunnel > 0 {
                let step = args.step.unwrap_or(args.dig).max(1);
                let depth = surface + args.depth.unwrap_or(args.dig * 3);
                for i in 0..args.tunnel {
                    pixel_physics::sim::rigid::mine(&mut w, x + i * step, depth, args.dig, args.dig_yield);
                }
            } else if args.strike > 0 {
                let force = args.strike as f32 * 0.9;
                pixel_physics::sim::rigid::strike(&mut w, x, surface + args.strike, args.strike, force);
            } else if args.dig > 0 {
                pixel_physics::sim::rigid::mine(&mut w, x, surface + args.dig, args.dig, args.dig_yield);
            }
            if args.relax {
                pixel_physics::sim::structural::compute_world_distances(&mut w);
            }
        }
        // The reference room `B` stamps, standing on the app's real terrain
        // rather than on a flat test floor. `scene=room` answers "does it
        // hold"; this answers "is it a sensible size", which is a different
        // question and needs the actual world in frame for scale. Render it
        // whole-world at zoom 1 -- the moment it is cropped or magnified the
        // only comparison it exists to make is gone.
        "refroom" => {
            let mut app = pixel_physics::app::App::new();
            app.stamp_reference_room(WIDTH / 2, 40);
            w = app.world;
        }
        // Generated terrain, the thing worldgen is judged on. Reads
        // `assets/worldgen.ron` rather than a replica of it, for the same
        // reason `terrain` calls the real `build_terrain`: a scene that
        // approximated the generator would stop being evidence about the
        // generator the first time either drifted.
        //
        // `seed=` and `preset=` pick the world; a run with `count=1` is a
        // single still, which is what the seed sweep in the worldgen plan
        // uses. Stepping it at all is also the at-rest check in visual form
        // — generated terrain that settles is terrain that moved.
        "worldgen" => {
            let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
            if let Some(e) = err {
                panic!("{e}");
            }
            let name = if args.preset.is_empty() { presets.default_name() } else { args.preset.clone() };
            let Some(params) = presets.get(&name) else { panic!("unknown preset {name:?}") };
            let report = pixel_physics::worldgen::generate_reported(
                &mut w,
                pixel_physics::worldgen::Spec::Generated { params, seed: args.seed },
            );
            pixel_physics::sim::structural::compute_world_distances(&mut w);
            // Printed next to the image on purpose. A contact sheet cannot
            // show whether a feature pass ran -- a terrace and an overhang
            // read the same at this zoom -- so a pass reporting zero here is
            // the only way to catch one that silently never fires.
            println!("worldgen {name} seed {}", args.seed);
            for (pass, cells) in &report {
                println!("  {pass:<14} {cells:>7} cells");
            }
        }
        // The payoff mechanic, and the one M17 "was built for and has never
        // had a real test case" (`Reports/worldgen-design.md` §7): mine
        // upward into a ledge until the roof left above the excavation is
        // thinner than stone can hold, and watch it come down. The bite is
        // taken out of the ledge's underside through the ordinary eraser
        // brush, so this goes through the same reactive path a player does.
        // Undercut an attached cliff shelf: erase the material *under* one
        // end of it so the outer part is left hanging over open air. This is
        // the case the whole model exists for, and the one that produced
        // nothing before `detach_exposed_neighbours` -- attached rock
        // anchors outright, so carving it used to just delete cells while
        // everything around them stayed permanently held.
        "undercut" => {
            stone_floor(&mut w);
            for y in 120..260 {
                for x in 0..90 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            // A shelf continuing out from the cliff face, also part of the
            // massif.
            for y in 150..162 {
                for x in 90..210 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            pixel_physics::sim::structural::compute_world_distances(&mut w);
            // Dig the shelf's support away from underneath, through the
            // ordinary eraser brush.
            for x in 92..208 {
                for y in 156..162 {
                    w.paint_capsule((x, y), (x, y), 0, material::EMPTY, 1.0);
                }
            }
        }
        // A solid attached cliff, struck once. Nothing here is structurally
        // unsound, so every piece that leaves is leaving because it was hit
        // -- which is the whole point of having a verb at all.
        "strike" => {
            stone_floor(&mut w);
            for y in 60..280 {
                for x in 140..380 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            pixel_physics::sim::structural::compute_world_distances(&mut w);
            pixel_physics::sim::rigid::strike(&mut w, 260, 150, 14, 12.0);
        }
        // Work one spot on an attached shelf with repeated blows, the way a
        // player would with `C`. Each hit drives the fissure deeper and cuts
        // what the rock around it can carry, so the shelf should give way
        // after several -- rather than needing to be chewed away cell by
        // cell, which is what erasing amounts to.
        "worked" => {
            stone_floor(&mut w);
            for y in 120..280 {
                for x in 0..90 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            for y in 150..164 {
                for x in 90..250 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            pixel_physics::sim::structural::compute_world_distances(&mut w);
            // Six blows on the shelf where it leaves the cliff -- the most
            // stressed point of a cantilever, and where a person would aim.
            for _ in 0..6 {
                pixel_physics::sim::rigid::strike(&mut w, 100, 157, 7, 6.0);
            }
        }
        // The reported case: a tall thick player-built column with a cap
        // overhanging it on both sides. Built as foreground (no
        // `with_attached`), exactly as the stone brush lays it down, so this
        // is what a player actually gets. It used to tear its own cap off
        // and dissolve most of it to dust.
        "capped" => {
            stone_floor(&mut w);
            for y in 120..floor_y {
                for x in 226..286 {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            for y in 90..126 {
                for x in 196..316 {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            w.schedule_structural_check_around(200, 108);
            w.schedule_structural_check_around(312, 108);
        }
        "mine" => {
            pixel_physics::app::build_terrain(&mut w);
            // The 60..200 ledge spans y=200..206. Erase its lower rows
            // across most of its length, leaving a roof too thin to stand.
            for x in 70..190 {
                for y in 202..206 {
                    w.paint_capsule((x, y), (x, y), 0, material::EMPTY, 1.0);
                }
            }
        }
        // Acceptance case 4, and the owner's original case: a big overhang
        // joined to the cliff by a deliberately thin ligament. It must snap
        // at the *neck*, not at the tip and not at all.
        //
        // This is the shape the reach model could not get right in
        // principle rather than by tuning. The ligament sits at *low*
        // distance -- it is right next to solid rock -- so reach says it is
        // fine, while the tip, being far out, is the part that fails. That
        // is backwards: rock fails where the stress is highest, and the
        // stress is highest where the section is thinnest.
        //
        // No blow and no erasing: the geometry alone has to be enough, so
        // nothing here can be mistaken for the strike doing the work.
        "ligament" => {
            stone_floor(&mut w);
            for y in 100..280 {
                for x in 0..100 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            // The neck: 4 cells deep, 10 long. Everything beyond it has to
            // carry its moment through these forty cells.
            for y in 150..154 {
                for x in 100..110 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            // The overhang: 40 deep and 110 long, hung off that neck.
            for y in 130..170 {
                for x in 110..220 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            pixel_physics::sim::structural::compute_world_distances(&mut w);
            // One structural check at the neck, which is all a disturbance
            // would do. Nothing is removed, nothing is struck.
            w.schedule_structural_check_around(105, 152);
        }
        // What a player actually builds, painted through the ordinary
        // brush at the radius they use (R2, so 5 cells thick).
        //
        // Reconstructed from a playtest screenshot after the report "most
        // things are still falling apart": a column with two arms, a tall
        // hook, and a wide arch, all foreground, all standing on the floor.
        // The screenshot's stress view read almost entirely *green* -- so
        // whatever is bringing these down is not the torque criterion, and
        // guessing which of the two failure paths it is from a picture is
        // exactly what the failure counters exist to stop.
        "built" => {
            stone_floor(&mut w);
            let paint = |w: &mut World, a: (i32, i32), b: (i32, i32)| {
                w.paint_capsule(a, b, 2, material::STONE, 1.0);
            };
            // The "F": a column off the floor with two short arms.
            paint(&mut w, (60, floor_y - 1), (60, 120));
            paint(&mut w, (60, 150), (110, 150));
            paint(&mut w, (60, 190), (105, 190));
            // The hook: up, then a long arm back over open air.
            paint(&mut w, (300, floor_y - 1), (300, 70));
            paint(&mut w, (300, 70), (210, 60));
            // The arch: two feet and a span, the case the model has the
            // most trouble with because an arch carries its load in
            // compression along its curve and this model only knows about
            // bending moment.
            let arch: Vec<(i32, i32)> = (0..=20)
                .map(|i| {
                    let t = i as f32 / 20.0;
                    let angle = std::f32::consts::PI * t;
                    (360 + (angle.cos() * -110.0) as i32, floor_y - 1 - (angle.sin() * 120.0) as i32)
                })
                .collect();
            for pair in arch.windows(2) {
                paint(&mut w, pair[0], pair[1]);
            }
        }
        // **The unzip reproduction.** A hollow room built exactly the way
        // `Tool::Room` builds one, then one cut into a wall -- one click of
        // `D`, which is what the report was.
        //
        // Two things have to be faithful for this to reproduce, and both
        // are easy to get wrong:
        //
        // - The walls go down through `paint_capsule_as`, not `set`,
        //   because that is what marks them intact and reruns the scoped
        //   relaxation. A room laid down with `set` is unattached
        //   foreground and fails for an entirely different reason.
        // - The four walls are drawn as four *overlapping* capsule runs, so
        //   the corners are covered twice. Four independent segments leak
        //   at every corner, which structurally means the roof is not
        //   carried by the walls at all -- the scene would then be
        //   measuring a bug in itself.
        //
        // The cut is at mid-height on the left wall, where a doorway goes.
        //
        // **What this scene found.** `Tool::Room` sets wall thickness from
        // `brush_radius` and `App::mine` passes the *same* `brush_radius`
        // through as the cut radius. A capsule of radius r is `2r+1` thick
        // and a dig of radius r is `2r+1` across, so a cut into a wall
        // built by the room tool severs it completely, every time, at any
        // height, at any brush size. There is no ligament left because
        // there cannot be one. The roof then hangs off the far wall alone
        // and duly overloads -- correctly. Two verbs sharing one number
        // where the whole point is that one must be smaller than the
        // other.
        // `wall=` and `dig=` are separate knobs *because the app gives them
        // the same number* and that turns out to be the whole bug -- see
        // the note above. Being able to vary them independently here is how
        // the question "how much thicker than a cut does a wall have to be"
        // gets an answer instead of an argument.
        "room" => {
            stone_floor(&mut w);
            let (x0, y0) = (140, 150);
            let (x1, y1) = (x0 + args.span, floor_y - 1);
            for (a, b) in [((x0, y0), (x1, y0)), ((x0, y1), (x1, y1)), ((x0, y0), (x0, y1)), ((x1, y0), (x1, y1))] {
                w.paint_capsule_as(a, b, args.wall, material::STONE, 1.0);
            }
            // One click, at mid-height on the left wall. `dig=0` makes no
            // cut at all, which is the control: it says whether the room
            // even stands untouched, and that has to be established before
            // any number from a cut means anything.
            if args.dig > 0 {
                pixel_physics::sim::rigid::mine(&mut w, x0, (y0 + y1) / 2, args.dig, args.dig_yield);
            }
        }
        // A thin shelf cantilevered off a thick pillar, with the join then
        // cut so the shelf detaches whole.
        //
        // This is the case that actually produces an M8 chunk body, and the
        // contrast with `mine` is the point. A mined roof fails
        // *progressively* -- its cells sit at genuinely different distances
        // (17, 18, 19...) and cross their span on different ticks, so at the
        // instant the first one breaks its neighbours are still supported
        // and there is no region to promote. A *detached* region has no
        // anchor at all, so its cells climb in lockstep (the
        // count-to-infinity dynamic in `structural.rs`'s module doc) and
        // cross together, which is what gives `try_promote_failing_region` a
        // whole connected region to find at once.
        //
        // The shelf is deliberately 3 cells thick: thicker than stone's
        // confinement diameter and it would anchor itself and hang there
        // forever, which is documented, intended behaviour and not what this
        // scene is for.
        "snap" => {
            stone_floor(&mut w);
            // The pillar is a cliff: attached, so it anchors the shelf and
            // does not crumble under its own weight.
            for y in 120..200 {
                for x in 60..80 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            // The shelf is *not* attached -- it is the thing under test, and
            // cutting it off its pillar should drop it.
            for y in 140..143 {
                for x in 80..112 {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            pixel_physics::sim::structural::compute_world_distances(&mut w);
            // Cut the shelf off its pillar, through the ordinary eraser
            // brush so this goes down the same reactive path a player does.
            for y in 140..143 {
                w.paint_capsule((80, y), (80, y), 0, material::EMPTY, 1.0);
            }
        }
        // A natural cave: a void inside *attached* rock, the situation every
        // mining blast actually happens in. `boom_stone` answers "what does a
        // blast do to a solid mass"; this answers the two questions that mass
        // cannot ask -- what happens at a cave *wall* (one free face beside
        // the charge) and under a cave *roof* (free face below, gravity
        // pointing into it). Place the charge with the ordinary `explode=`
        // arg: (186,240) is inside the left wall, (256,200) is inside the
        // roof. The printed "roofed void (cave volume)" line is the number to
        // read -- a blast that widens the cave grows it, one that caves the
        // roof in shrinks it.
        "cavern" => {
            stone_floor(&mut w);
            for x in 106..406 {
                for y in 130..floor_y {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            // The cave: an ellipse, 120 wide by 56 tall, roof 82 cells thick.
            let (cx, cy, a, b) = (256i32, 240i32, 60f32, 28f32);
            for x in (cx - 60)..=(cx + 60) {
                for y in (cy - 28)..=(cy + 28) {
                    let (dx, dy) = ((x - cx) as f32 / a, (y - cy) as f32 / b);
                    if dx * dx + dy * dy <= 1.0 {
                        w.set(x, y, Cell::EMPTY);
                    }
                }
            }
            pixel_physics::sim::structural::compute_world_distances(&mut w);
        }
        other => panic!(
            "unknown scene {other:?}; known: pour, fall, blob, sand, boom, boom_stone, sandbed, waterbed, tree, forest, grove, terrain, worldgen, mine, snap, undercut, strike, worked, capped, ligament, built, room, refroom, worldcrack, gnome, tunnel, bury, swim, ride, cavern, wood, climb, shake"
        ),
    }
    w
}

struct Args {
    scene: String,
    /// `seed=N` -- which generated world `scene=worldgen` builds.
    seed: u64,
    /// `yield=F` -- the gnome's `dig_yield`, for comparing the spoil
    /// modes the app cycles with `F2`. Whether a bore actually opens is
    /// decided entirely by this number (see `player::Tuning::dig_yield`),
    /// so a harness that could not vary it could not show the difference
    /// between "you cannot dig" and "rock simply goes".
    dig_yield: f32,
    /// `preset=NAME` -- which entry of `assets/worldgen.ron` it uses. Empty
    /// means that file's own `default`.
    preset: String,
    /// `wall=N` / `dig=N` -- capsule radii for `scene=room`'s walls and for
    /// the cut made into them. Both default to 3, which is what the app
    /// itself does, because the app has only one number for both.
    wall: i32,
    dig: i32,
    /// Strike radius for scene=worldcrack. Zero means dig instead.
    strike: i32,
    /// Number of dig bites driven horizontally into the hill, as a tunnel.
    tunnel: i32,
    /// `relax=1` -- run a **converged** distance pass straight after the
    /// dig, instead of letting the scheduled relaxation reconverge over the
    /// following frames.
    ///
    /// The instrument for one specific question: is a failure real, or is
    /// it a cell judged mid-convergence against a stale distance? Those look
    /// identical in an image and `load.rs` has been bitten by the
    /// distinction before. Answered for the dig cascade: no, staleness is
    /// not the cause -- see `Reports/next-session-handoff.md` 1b.
    relax: bool,
    /// `span=N` -- how wide `scene=room` is drawn, outer edge to outer
    /// edge. The knob the whole "what can a player actually build" question
    /// turns on, and the reason it is a knob rather than a constant: a
    /// single width says the room stands or does not, and what is wanted is
    /// the *envelope*.
    span: i32,
    start: usize,
    every: usize,
    count: usize,
    cols: usize,
    zoom: i32,
    crop: Rect,
    parallel_driver: bool,
    /// `genome=` for `scene=colony`: `authored`, `zero`, or `rNNN` naming a
    /// genome from `creature_space`'s sweep by the label it printed.
    genome: String,
    out: String,
    grain: GrainMode,
    /// `trees=weave|haze|front|behind` -- which `TreeDepth` the sheet is
    /// shot in. The whole value of a selector is being able to put its
    /// settings side by side, and a still image is the only way to compare
    /// two of them at once.
    tree_depth: TreeDepth,
    /// `channel=` -- render the sheet through one of `render.rs`'s debug
    /// overlays instead of ordinary material colour. The whole reason the
    /// plant work needs this harness: resource, canopy density and (later)
    /// vein conductance are per-cell scalars that decide plant shape and
    /// have **never been drawn**, which is how `tree-rewrite-design.md`
    /// §2b's self-avoidance mechanism shipped inert past two design
    /// reviews. A contact sheet in a channel shows both what the value is
    /// and how it evolves across the tiles, which is the question.
    organism_overlay: OrganismOverlay,
    field_overlay: FieldOverlay,
    /// `daylight=<0.0..1.0>` -- draw every tile at one fixed hour instead
    /// of at whatever time of day the run happened to reach. `1.0` is noon,
    /// `0.0` the darkest the lighting term goes. Unset is the ordinary
    /// day/night cycle, so every sheet recorded before this existed still
    /// reproduces exactly.
    ///
    /// **Render-only, and not a cheat.** It reaches `Renderer::
    /// daylight_pin` and nothing else: the simulation's clock, the light
    /// field, plants and weather all still run on `world.frame`, so this
    /// changes what the sheet looks like and nothing about what happened
    /// in it. The nine-blast harness fires charges 400 frames apart into a
    /// 3,600-frame day, so its nine panels are nine different exposures --
    /// columns 3-6 come out at night and are genuinely hard to read -- and
    /// the tiles are being compared *to each other*. A variable that is not
    /// the one under test has to be held constant (`CLAUDE.md`: a channel
    /// that oscillates by design must be divided out of decisions).
    daylight: Option<f32>,
    /// Write an animated GIF of every frame in the range instead of a grid.
    /// The grid is for *me* to read; motion is for a human to watch, and
    /// some of these artifacts only read correctly in motion.
    gif: bool,
    /// `explode=x,y,radius,strength,frame` -- fire one `explosion::trigger`
    /// at the given frame. Repeatable, for several blasts in one run.
    explosions: Vec<(i32, i32, i32, f32, usize)>,
    /// `blast=x,depth,radius,strength,frame` -- the same charge, placed
    /// `depth` cells below the **local solid surface** at column `x`.
    /// Repeatable. Held as `(x, depth, radius, strength, frame)`; the `y`
    /// is not known until it fires.
    ///
    /// **Why this exists next to `explode=` rather than replacing it.** A
    /// fixed `y` is a different situation on every seed -- open sky on one,
    /// bedrock on the next -- so a seed sweep over absolute coordinates
    /// measures the terrain and not the change. `CLAUDE.md`: a guard over a
    /// procedural system has to sweep the procedure. One `blast=` list is
    /// valid on every seed; one `explode=` list is valid on exactly one.
    ///
    /// Resolved inside `fire_due_explosions`, at the frame it fires, not at
    /// parse time -- so a later charge fired into the crater an earlier one
    /// left sees the crater rather than the surface that used to be there.
    ///
    /// `depth` is signed: negative is an airburst above the surface,
    /// `0` is the surface cell itself, positive is into the ground.
    blasts: Vec<(i32, i32, i32, f32, usize)>,
    /// `panels=W,H,age1[,age2,...]` -- write a **second** contact sheet:
    /// one column per charge fired, one row per age, each cell a `W`x`H`
    /// crop centred on that charge's own resolved site and captured
    /// `age` frames after **that charge's own detonation**.
    ///
    /// Per-charge age rather than absolute frame, and that is the whole
    /// point of the sheet: nine charges fired at nine different frames and
    /// sampled at one absolute frame are nine blasts at nine different
    /// points in their life, which is exactly the confound that makes "are
    /// these nine outcomes different?" unanswerable. Anchoring each column
    /// on its own bang makes a row a controlled comparison.
    ///
    /// Nothing is labelled on the image on purpose. The counters are on
    /// stdout; a label baked into a PNG is a second source of truth that
    /// goes stale.
    panels: Option<(i32, i32, Vec<usize>)>,
    /// `cut=x,y,w,h,frame` -- erase a rectangle at the given frame.
    ///
    /// **A surgical alternative to `explode`, and it exists because the
    /// blast was the wrong instrument for the question.** Asking "does a
    /// damaged tree respond" with `explode=224,175,6,2.0` vaporised the
    /// whole tree: a trunk here is two or three cells wide, so any blast
    /// big enough to reach across it is big enough to remove everything
    /// nearby, and what the sheet showed was a debris pile rather than a
    /// severed stem. A rectangle removes exactly what is named, delivers no
    /// impulse and no heat, and leaves the rest of the plant untouched --
    /// which is what isolates the *physiological* response from the
    /// mechanical one.
    cuts: Vec<(i32, i32, i32, i32, usize)>,
    /// `depowder=frame` -- erase every `Powder`-kind cell in the world at
    /// the given frame, and say how many.
    ///
    /// **The paired control for the powder surcharge**, and it exists
    /// because neither `cut` nor `explode` can express it. The question R4
    /// has to answer is "did the roof come down because the muck on top of
    /// it now weighs something, or because the blast quietly took the
    /// shell's capacity away" -- and the only way to separate those is to
    /// run the identical blast and then take the muck away, leaving the
    /// cracked, detached shell exactly as the blast left it. `cut` erases a
    /// rectangle including the rock, which changes the thing under test;
    /// this erases only what is loose.
    depowder: Option<usize>,
    /// `load=x,y` -- print `sim::load::evaluate` at that cell for every
    /// tile. Repeatable. The structural counterpart of the `bodies` line:
    /// an image says a shelf is still up, and only a number says whether it
    /// is up with 3% of its capacity used or 97%.
    probes: Vec<(i32, i32)>,
    /// `repeat=N` -- run the whole scene N times and report the **minimum**
    /// worst-frame with the spread beside it, rather than one sample.
    ///
    /// This machine is contended enough that a single sample is not a
    /// measurement. Observed within one session: 18.0 ms twice running on
    /// `scene=terrain`, which schedules zero structural checks and cannot
    /// be doing the work that number would imply, and 40.5 / 55.6 ms on
    /// scenes that measured 14-19 ms moments later. Three separate
    /// near-misses where contention was almost read as a regression.
    ///
    /// The minimum is the right statistic, not the mean: contention can
    /// only ever make a frame *slower*, so the fastest observed run is the
    /// closest thing to the machine's actual cost. The spread is printed
    /// beside it so a sample that is all noise is visible as such.
    repeat: usize,
    /// `min_overloaded=N` / `max_failures=N` -- exit non-zero unless the
    /// run produced at least / at most that many structural failures. See
    /// `check_expectations`.
    min_overloaded: Option<u32>,
    max_failures: Option<u32>,
    /// `max_frame_ms=N` -- exit non-zero if the **minimum** worst-frame
    /// across `repeat` runs exceeds N.
    ///
    /// Checked against the minimum specifically, and that is what makes it
    /// safe to gate CI on. A single sample from a contended machine is not
    /// a measurement -- this session saw 18.0 ms twice running on a scene
    /// that schedules no structural work at all -- so a bar checked against
    /// one run, or against a mean, would be permanently flaky and would
    /// train everyone to ignore it. Contention can only make a frame
    /// slower, so the fastest of several runs is the closest thing to the
    /// machine's real cost, and a bar it still fails is a real regression.
    max_frame_ms: Option<f64>,
    /// `min_bodies=N` -- exit non-zero unless at least N coherent chunk
    /// bodies were in flight at once at some point in the run.
    ///
    /// A different question from `min_overloaded`, and the `strike` scene
    /// is why both exist. That scene is about a *blow throwing pieces*,
    /// and the mechanism there is `rigid::strike`'s own fracture, not the
    /// load criterion -- so asserting overload failures tested something
    /// the scene is not about, and duly broke when an unrelated change to
    /// the fragment ladder shifted how many separate events the same
    /// material came away in. Peak concurrent bodies is the quantity that
    /// actually says "it threw pieces".
    min_bodies: Option<usize>,
    /// `min_travelled=N` -- exit non-zero unless the scripted gnome covered
    /// at least N cells after setting off.
    ///
    /// The gnome path had no gated case at all before this, which is how a
    /// character who could be walled in by a *tree* went unnoticed. Distance
    /// is the right quantity for the same reason `min_overloaded` is the
    /// right one for a collapse: "he is standing near a trunk" and "he is
    /// standing *in* one" are the same picture.
    min_travelled: Option<i32>,
    /// `loadmap=1` -- also report the single most-stressed cell in the
    /// world per tile. `CLAUDE.md`: sanity-check a new metric against a
    /// case you know is fine before trusting it about one you don't, and
    /// "nothing anywhere is over 1.0" on a scene that visibly stands is
    /// exactly that check.
    loadmap: bool,
    /// `min_cave=P` -- exit non-zero unless at least P percent of the
    /// roofed void present at the cut is still there at the end.
    ///
    /// The gate for "a cave can be dug and it does not collapse", which is
    /// the owner's own statement of what this has to do. A fraction rather
    /// than an absolute so one bar covers every bore size and length.
    min_cave: Option<i64>,
    /// `max_cave=P` -- exit non-zero unless the roofed void has fallen to at
    /// most P percent of what was there at the cut. The mirror of
    /// `min_cave`, and the gate for "this roof came **down**".
    ///
    /// It exists because the thing it replaces was an *event* count.
    /// `roomcut` was gated on `min_overloaded=5`, and a change to how a
    /// failing region is shaped merged what had been dozens of separate
    /// failures into one large paced one -- same roof, same rubble, one
    /// event instead of thirty-seven. The bar duly went red on a scene whose
    /// outcome had not got worse. That is the second time an event-count bar
    /// in this file has caught a mode shift rather than a behaviour change;
    /// see case 6's own note about `strike`. **Measure what the scene is
    /// about** -- and what `roomcut` is about is whether the ceiling is
    /// still up.
    max_cave: Option<i64>,
    /// `step=N` -- how far apart consecutive `tunnel=` bites are placed.
    /// Defaults to `dig`, i.e. each bite overlaps the last by half.
    ///
    /// It used to be `dig * 2 + 1`, and that was **not a tunnel**. A disc
    /// of radius r is `2r + 1` across on its centre line and narrower
    /// everywhere else, so centres spaced `2r + 1` apart leave solid rock
    /// standing between every pair of bites. Dumped, four bites came out
    /// as four separate chambers joined only near the floor, with 2-4 cell
    /// pillars between them -- a string of beads. It looked like a row of
    /// circles because it *was* a row of circles, and every measurement of
    /// "does a tunnel hold" was really measuring a row of small caverns
    /// separated by thin pillars, which is about the least representative
    /// geometry available.
    step: Option<i32>,
    /// `depth=N` -- how far below the surface a `worldcrack` tunnel is
    /// driven, in cells. Defaults to `dig * 3`.
    ///
    /// It exists because the default **couples two variables**, and that
    /// invalidated a measurement: comparing bore sizes at equal tunnel
    /// length showed a 13-cell gallery holding (14 cells of rock lost)
    /// where a 5-cell ant tunnel collapsed (1,105) -- the exact inverse of
    /// what the owner expects -- but the big bore was also being driven
    /// three times deeper, under three times the roof. A knob that moves
    /// something else along with it is not a knob. Set this to hold depth
    /// fixed while bore varies, which is what requirement 2 (collapse
    /// depending on height, not only span) has to be measured against.
    depth: Option<i32>,
    /// `dump=x,y,w,h` -- print the materials in that rectangle as ASCII,
    /// once per captured tile.
    ///
    /// For questions a contact sheet cannot answer however far it is
    /// zoomed. "Why will the gnome not walk into his own tunnel" turned
    /// into three wrong guesses about the geometry of the mouth read off a
    /// 20x magnification -- whether the floor was a step up or a drop,
    /// whether a lip of rock survived at the threshold -- when what was
    /// needed was the cells themselves. `.` is air, `#` solid, `o` powder,
    /// `~` liquid, `P` where the player is standing.
    dump: Option<Rect>,
    /// `max_lost=N` -- exit non-zero if the world ended with more than N
    /// cells fewer than it started with.
    ///
    /// **A failure count is not a damage count**, and this is the number
    /// that closes that gap. `FailureCounts` counts cells that *failed*,
    /// and a failed cell that became rubble is still standing there --
    /// `CLAUDE.md` records two digs whose event counts looked comparable
    /// removing 894 and 23,042 cells, with nothing in the engine able to
    /// tell them apart. This is the census it asks for: how much material
    /// the world actually holds, before and after.
    ///
    /// The baseline is taken **after** the scene is built, so a scene's
    /// own cut is not counted: this measures what the *simulation*
    /// subsequently ate, which is the quantity that separates 894 from
    /// 23,042. Deliberately so, and it is what makes the bar safe to gate
    /// on -- the bite is a constant that moves whenever `yield` is
    /// retuned, and a guard that moved with it would need re-baselining on
    /// every legitimate spoil change while staying just as blind to a
    /// cascade. A gnome scene is the exception by construction: he digs
    /// during the run, so his bites do count, and `world holds` on his own
    /// report line is the number to read there.
    max_lost: Option<i64>,
    /// `confine=0` -- turn off `World::crush_confined`, so a failing
    /// region displaces whether or not it has anywhere to go.
    ///
    /// The control that isolates the rule. A sweep only varies its knob,
    /// and anything that landed *with* a mechanism is constant across
    /// every data point -- which has already read as "the approach is
    /// wrong" here once when a rider was the whole effect. Running the
    /// same binary with the rule off is what makes a before/after a
    /// measurement rather than a memory of an earlier build.
    confine: bool,
    /// `arch=0` -- turn off `World::arch_relief`, so a roof carries the
    /// whole column above it again. The control for the arching change.
    arch: bool,
    /// `chain_reach=N` -- how far from something actually disturbed a
    /// failure may happen, in cells. Unset means no limit, the shipped
    /// behaviour. `0` is "only what you struck ever fails".
    chain_reach: Option<i32>,
    /// `joints=<spacing>` -- override stone's `Material::joint_spacing`, the
    /// pitch of the joint fabric (`sim::fracture_field`), in cells.
    ///
    /// It exists because the alternative is a rebuild per sweep point: the
    /// `.ron` files are `include_str!`d into the binary, so editing
    /// `stone.ron` and re-running a prebuilt harness produces bit-identical
    /// "runs" -- which `CLAUDE.md` records as having produced three of them
    /// before anyone noticed the knob was not connected. With this, the
    /// density A/B is one binary and one flag, and *differing* output across
    /// settings is itself the proof the knob reaches the mechanism.
    ///
    /// `0` turns jointing off entirely, which is the control.
    joint_spacing: Option<f32>,
    /// `bands=<contrast>` -- override stone's
    /// `Material::joint_band_contrast`: how far the grain varies from place
    /// to place. `0` (the shipped default) is a uniform grain everywhere.
    ///
    /// The A/B for the owner's *"could the pattern of cracks be more
    /// heterogeneous"*, and it is a knob rather than a decision because the
    /// trade is a judgement: `0.4` gives a visibly varied web, narrows the
    /// promoted-cell spread (best case down a quarter, worst case up 7%),
    /// and costs 10-14 ms on the worst frame. See
    /// `sim::fracture_field::pitch_at` for the four-seed table.
    joint_bands: Option<f32>,
    /// `jreach=`, `jopen=`, `jdensity=` -- the three `explosion::Tuning`
    /// knobs of the same mechanism, for the same no-rebuild reason.
    joint_reach: Option<f32>,
    joint_open: Option<f32>,
    joint_density: Option<f32>,
    /// `crack_rays=` -- the hybrid knob. `0` (the default) is pure fabric;
    /// 4-6 puts the old radial walker back on top of it for an A/B.
    crack_rays: Option<u32>,
    /// `smoke=<fraction>` -- override `explosion::Tuning::smoke_fraction`,
    /// how much of a cleared crater is backfilled with `SMOKE`.
    ///
    /// **`smoke=0` is the control for "do chunks fall into the hole".**
    /// `rigid::clear_or_displaceable` shoves `Powder` and `Liquid` aside and
    /// treats everything else as a real obstruction, `Gas` included -- so at
    /// the shipped `0.18` a promoted chunk falling into a fresh crater can be
    /// stopped dead by the blast's own smoke, stall for
    /// `STALL_FRAMES_BEFORE_SETTLING` frames and re-embed roughly where it
    /// started. Running the same charge at `0` and at the default is the
    /// paired comparison that says whether that is actually happening, and
    /// it costs one flag rather than a rebuild.
    smoke: Option<f32>,

}

fn parse() -> Args {
    let mut a = Args {
        scene: "pour".into(),
        dig_yield: pixel_physics::sim::player::Tuning::default().dig_yield,
        seed: 1,
        preset: String::new(),
        start: 100,
        every: 60,
        count: 6,
        cols: 3,
        zoom: 1,
        genome: String::from("authored"),
        crop: Rect::new(0, 0, WIDTH - 1, HEIGHT - 1),
        parallel_driver: true,
        out: std::env::temp_dir().join("filmstrip.png").display().to_string(),
        grain: GrainMode::Position,
        tree_depth: TreeDepth::default(),
        organism_overlay: OrganismOverlay::Off,
        field_overlay: FieldOverlay::Off,
        daylight: None,
        gif: false,
        explosions: Vec::new(),
        blasts: Vec::new(),
        panels: None,
        cuts: Vec::new(),
        depowder: None,
        probes: Vec::new(),
        loadmap: false,
        repeat: 1,
        min_overloaded: None,
        max_failures: None,
        max_frame_ms: None,
        min_bodies: None,
        min_travelled: None,
        max_lost: None,
        dump: None,
        depth: None,
        step: None,
        min_cave: None,
        max_cave: None,
        confine: true,
        arch: true,
        chain_reach: None,
        joint_spacing: None,
        joint_bands: None,
        joint_reach: None,
        joint_open: None,
        joint_density: None,
        crack_rays: None,
        smoke: None,
        wall: 3,
        dig: 3,
        strike: 0,
        tunnel: 0,
        relax: false,
        span: 200,
    };
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "scene" => a.scene = v.into(),
            "seed" => a.seed = v.parse().expect("seed"),
            "yield" => a.dig_yield = v.parse().expect("yield"),
            "preset" => a.preset = v.into(),
            "start" => a.start = v.parse().expect("start"),
            "every" => a.every = v.parse().expect("every"),
            "count" => a.count = v.parse().expect("count"),
            "cols" => a.cols = v.parse().expect("cols"),
            "zoom" => a.zoom = v.parse().expect("zoom"),
            "genome" => a.genome = v.to_string(),
            "driver" => a.parallel_driver = v != "serial",
            "out" => a.out = v.into(),
            "gif" => a.gif = v != "false",
            "grain" => {
                a.grain = match v {
                    "position" => GrainMode::Position,
                    "cell" => GrainMode::Cell,
                    "muted" => GrainMode::Muted,
                    "animated" => GrainMode::Animated,
                    "motion" => GrainMode::Motion,
                    other => panic!("unknown grain {other:?}"),
                }
            }
            "trees" => {
                a.tree_depth = match v {
                    "weave" => TreeDepth::Weave,
                    "haze" => TreeDepth::Haze,
                    "front" => TreeDepth::Front,
                    "behind" => TreeDepth::Behind,
                    other => panic!("unknown trees {other:?}"),
                }
            }
            // One flag for both overlay families, resolved by name: they are
            // one question ("which channel am I looking at") from the
            // caller's side, and keeping them as two arguments would invite
            // setting both and getting a sheet that is hard to attribute.
            "channel" => match v {
                "off" => {}
                "celltype" => a.organism_overlay = OrganismOverlay::CellType,
                "resource" => a.organism_overlay = OrganismOverlay::Resource,
                "canopy" => a.organism_overlay = OrganismOverlay::CanopyDensity,
                "vein" => a.organism_overlay = OrganismOverlay::VeinConductance,
                "soil" => a.organism_overlay = OrganismOverlay::SoilMoisture,
                "light" => a.field_overlay = FieldOverlay::Light,
                "moisture" => a.field_overlay = FieldOverlay::Moisture,
                "temperature" => a.field_overlay = FieldOverlay::Temperature,
                "pressure" => a.field_overlay = FieldOverlay::Pressure,
                "pheromone_a" => a.field_overlay = FieldOverlay::PheromoneA,
                "pheromone_b" => a.field_overlay = FieldOverlay::PheromoneB,
                other => panic!(
                    "unknown channel {other:?}; known: off, celltype, resource, canopy, vein, soil, light, moisture, temperature, pressure, pheromone_a, pheromone_b"
                ),
            },
            "daylight" => {
                let f: f32 = v.parse().expect("daylight=<0.0..1.0>");
                assert!((0.0..=1.0).contains(&f), "daylight=<0.0..1.0>, got {f}");
                a.daylight = Some(f);
            }
            "repeat" => a.repeat = v.parse::<usize>().expect("repeat").max(1),
            "wall" => a.wall = v.parse().expect("wall"),
            "dig" => a.dig = v.parse().expect("dig"),
            "strike" => a.strike = v.parse().expect("strike"),
            "tunnel" => a.tunnel = v.parse().expect("tunnel"),
            "relax" => a.relax = v != "false",
            "span" => a.span = v.parse().expect("span"),
            "min_overloaded" => a.min_overloaded = Some(v.parse().expect("min_overloaded")),
            "max_failures" => a.max_failures = Some(v.parse().expect("max_failures")),
            "max_lost" => a.max_lost = Some(v.parse().expect("max_lost")),
            "depth" => a.depth = Some(v.parse().expect("depth")),
            "step" => a.step = Some(v.parse().expect("step")),
            "min_cave" => a.min_cave = Some(v.parse().expect("min_cave")),
            "max_cave" => a.max_cave = Some(v.parse().expect("max_cave")),
            "dump" => {
                let n: Vec<i32> = v.split(',').map(|p| p.parse().expect("dump=x,y,w,h")).collect();
                assert_eq!(n.len(), 4, "dump=x,y,w,h");
                a.dump = Some(Rect::new(n[0], n[1], n[0] + n[2] - 1, n[1] + n[3] - 1));
            }
            "confine" => a.confine = v != "0" && v != "false",
            "arch" => a.arch = v != "0" && v != "false",
            "chain_reach" => a.chain_reach = Some(v.parse().expect("chain_reach")),
            "joints" => a.joint_spacing = Some(v.parse().expect("joints=<spacing in cells>")),
            "bands" => a.joint_bands = Some(v.parse().expect("bands=<grain contrast 0..0.9>")),
            "jreach" => a.joint_reach = Some(v.parse().expect("jreach")),
            "jopen" => a.joint_open = Some(v.parse().expect("jopen")),
            "jdensity" => a.joint_density = Some(v.parse().expect("jdensity")),
            "crack_rays" => a.crack_rays = Some(v.parse().expect("crack_rays")),
            "smoke" => a.smoke = Some(v.parse().expect("smoke=<fraction 0..1>")),
            "max_frame_ms" => a.max_frame_ms = Some(v.parse().expect("max_frame_ms")),
            "min_bodies" => a.min_bodies = Some(v.parse().expect("min_bodies")),
            "min_travelled" => a.min_travelled = Some(v.parse().expect("min_travelled")),
            "loadmap" => a.loadmap = v != "false",
            "load" => {
                let n: Vec<i32> = v.split(',').map(|s| s.parse().expect("load")).collect();
                assert_eq!(n.len(), 2, "load=x,y");
                a.probes.push((n[0], n[1]));
            }
            "explode" => {
                let n: Vec<f32> = v.split(',').map(|s| s.parse().expect("explode")).collect();
                assert_eq!(n.len(), 5, "explode=x,y,radius,strength,frame");
                a.explosions.push((n[0] as i32, n[1] as i32, n[2] as i32, n[3], n[4] as usize));
            }
            // Parsed through `f32` like `explode=` above, which also gets
            // the sign of a negative `depth` (an airburst) for free.
            "blast" => {
                let n: Vec<f32> = v.split(',').map(|s| s.parse().expect("blast")).collect();
                assert_eq!(n.len(), 5, "blast=x,depth,radius,strength,frame");
                a.blasts.push((n[0] as i32, n[1] as i32, n[2] as i32, n[3], n[4] as usize));
            }
            "panels" => {
                let n: Vec<i64> = v.split(',').map(|s| s.parse().expect("panels")).collect();
                assert!(n.len() >= 3, "panels=W,H,age1[,age2,...]");
                assert!(n[0] > 0 && n[1] > 0, "panels=W,H,... needs a positive crop");
                a.panels = Some((n[0] as i32, n[1] as i32, n[2..].iter().map(|&f| f as usize).collect()));
            }
            "cut" => {
                let n: Vec<i32> = v.split(',').map(|s| s.parse().expect("cut")).collect();
                assert_eq!(n.len(), 5, "cut=x,y,w,h,frame");
                a.cuts.push((n[0], n[1], n[2], n[3], n[4] as usize));
            }
            "depowder" => a.depowder = Some(v.parse().expect("depowder")),
            "crop" => {
                let n: Vec<i32> = v.split(',').map(|s| s.parse().expect("crop")).collect();
                assert_eq!(n.len(), 4, "crop=x,y,w,h");
                a.crop = Rect::new(n[0], n[1], n[0] + n[2] - 1, n[1] + n[3] - 1);
            }
            other => panic!("unknown argument {other:?}"),
        }
    }
    a
}

/// Fire every scheduled explosion whose frame has arrived, removing it from
/// the pending list so it cannot fire twice. Draining rather than
/// index-matching makes this safe to call both inside the stepping loop and
/// immediately before a capture, which is what lets `frame=0` work at all
/// (with `start=0` the loop body never runs before the first tile).
/// Erase every scheduled cut whose frame has arrived, draining it so it
/// cannot fire twice -- same shape as `fire_due_explosions`, and called
/// from the same three places for the same reason.
///
/// Reports what it actually removed rather than what it was asked to
/// remove. "Did the cut land on the tree" is a counter question, not a
/// picture question: a rectangle a few cells off the trunk looks identical
/// on a contact sheet to one that severed it, and this branch has already
/// spent a session reading a collapse as a feature that had never once
/// executed.
fn fire_due_cuts(world: &mut World, pending: &mut Vec<(i32, i32, i32, i32, usize)>, now: usize) {
    let mut i = 0;
    while i < pending.len() {
        if pending[i].4 <= now {
            let (x, y, w, h, _) = pending.remove(i);
            let mut removed = 0;
            let mut organism_cells = 0;
            for cy in y..y + h {
                for cx in x..x + w {
                    let cell = world.get(cx, cy);
                    if cell.material == material::EMPTY {
                        continue;
                    }
                    removed += 1;
                    if cell.organism_id() != 0 {
                        organism_cells += 1;
                    }
                    world.set(cx, cy, Cell::EMPTY);
                }
            }
            println!("  cut: ({x}, {y}) {w}x{h} at frame {now} -- removed {removed} cells, {organism_cells} of them living tissue");
        } else {
            i += 1;
        }
    }
}

/// Keep the world free of loose material from `frame` onward -- erase every
/// `Powder`-kind cell, every frame, and say how many the first pass took.
///
/// **Continuous, not one-shot, and that was measured rather than assumed.**
/// A single sweep at the blast frame removed *zero* cells (the muck is still
/// in the particle system, not in the grid), one at frame 75 removed 74 and
/// one at frame 100 removed 123 -- the plug is still arriving, so any single
/// instant is an arbitrary fraction of it. What the control has to hold
/// still is "nothing ever stands on the shell", which is a standing state,
/// not an event (`CLAUDE.md`: measure the standing state, not the event
/// rate).
///
/// Reports the first pass's count, because "the control removed the plug" is
/// a counter question: a sheet of a cave with no rubble in it and a sheet of
/// a cave whose rubble had already poured away look identical.
///
/// The cave-volume and cells-lost lines are **not comparable across this
/// flag** -- vacuuming rubble empties cells the census would otherwise still
/// count. Compare a vacuumed run only against another vacuumed run.
fn fire_due_depowder(world: &mut World, pending: &mut Option<usize>, first: &mut bool, now: usize) {
    match *pending {
        Some(frame) if frame <= now => {}
        _ => return,
    }
    let mut removed = 0;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if world.materials.kind(world.get(x, y).material) == MaterialKind::Powder {
                world.set(x, y, Cell::EMPTY);
                // Taking a load off is a disturbance exactly as putting one
                // on is, so the rock underneath has to be asked the question
                // again -- the same fan-out `World::paint_capsule` already
                // does for an ordinary erase. Around the erased cell only:
                // a world-wide reschedule every frame would swamp the site
                // budget and measure the scheduler instead of the model.
                world.schedule_structural_check_around(x, y);
                removed += 1;
            }
        }
    }
    if *first {
        println!("  depowder: from frame {now} on, the world is kept clear of loose material -- first pass took {removed} cells");
        *first = false;
    }
}

/// One charge that has actually gone off, in fire order: where it landed,
/// how big it was and which frame it detonated on.
///
/// Recorded rather than read back off `Args`, because `blast=`'s site is
/// not known until it fires and because the per-charge counters and the
/// `panels=` sheet both have to anchor on the frame it *did* fire, not the
/// frame it was asked to. A charge scheduled past the end of a run never
/// appears here at all, which is the honest answer.
#[derive(Clone, Copy, Debug)]
struct FiredCharge {
    x: i32,
    y: i32,
    radius: i32,
    frame: usize,
}

/// The topmost **solid** cell in column `x`: the smallest `y` whose cell is
/// neither materially empty, nor `Gas`, nor `Liquid`.
///
/// `Liquid` is excluded deliberately rather than by oversight. A charge
/// under the sea should be measured from the **seabed**, so `blast=120,8`
/// reads as "8 cells into the seabed with water overhead" -- which is the
/// near-water case this harness exists to fire. Measuring from the water
/// surface instead would put the same argument at a different depth on
/// every seed, which is the exact failure `blast=` exists to remove.
///
/// The emptiness test is the raw material comparison, not
/// `Cell::is_empty()`: that one is managed-aware and a promoted liquid
/// body's container cells read as *not* empty while holding no material at
/// all (`CLAUDE.md` gotchas; `explosion.rs`'s `clear_annulus` carries the
/// same note).
///
/// Panics on a column with no solid cell in it. A charge that quietly
/// relocated itself to somewhere it could be placed would make the sheet
/// lie about what it fired, and a sheet that lies is worse than no sheet.
fn solid_surface_y(world: &World, x: i32) -> i32 {
    for y in 0..HEIGHT {
        let m = world.get(x, y).material;
        if m == material::EMPTY {
            continue;
        }
        match world.materials.kind(m) {
            MaterialKind::Gas | MaterialKind::Liquid => continue,
            _ => return y,
        }
    }
    panic!("blast= at column {x}: no solid surface in that column, so the charge would have fired into open sky");
}

fn fire_due_explosions(
    world: &mut World,
    particles: &mut ParticleSystem,
    blasts: &mut explosion::Blasts,
    pending: &mut Vec<(i32, i32, i32, f32, usize)>,
    pending_blasts: &mut Vec<(i32, i32, i32, f32, usize)>,
    fired: &mut Vec<FiredCharge>,
    now: usize,
) {
    // **Order within one frame: every `explode=` due now, then every
    // `blast=` due now.** Arbitrary, and written down *because* it is
    // arbitrary -- determinism is required here (`PLAN.md`), and an order
    // nobody wrote down is an order somebody will reverse while tidying.
    let mut i = 0;
    while i < pending.len() {
        if pending[i].4 <= now {
            let (x, y, r, strength, _) = pending.remove(i);
            println!("  boom: ({x}, {y}) r={r} strength={strength} at frame {now}");
            blasts.trigger_with(world, particles, x, y, r, strength);
            fired.push(FiredCharge { x, y, radius: r, frame: now });
        } else {
            i += 1;
        }
    }
    let mut i = 0;
    while i < pending_blasts.len() {
        if pending_blasts[i].4 <= now {
            let (x, depth, r, strength, _) = pending_blasts.remove(i);
            // Resolved *here*, one line before the charge goes off, which
            // is the whole reason `blast=` is a separate list rather than
            // being folded into `explosions` at parse time: the surface it
            // measures has to be the surface as it is now, craters and all.
            let surface = solid_surface_y(world, x);
            let y = surface + depth;
            // The `-- blast=` suffix says what the argument resolved to, so
            // the sheet can be read without re-deriving the terrain. It
            // appears only for `blast=`; `explode=`'s line above is
            // unchanged byte for byte, because recorded measurements in
            // `Reports/explosion-stone-review.md` parse out of it.
            println!("  boom: ({x}, {y}) r={r} strength={strength} at frame {now} -- blast={x},{depth} (surface y={surface})");
            blasts.trigger_with(world, particles, x, y, r, strength);
            fired.push(FiredCharge { x, y, radius: r, frame: now });
        } else {
            i += 1;
        }
    }
}

/// One full frame, in `App::update`'s own phase order.
///
/// This harness originally ran the CA sweep and nothing else, which is fine
/// for liquids but makes an explosion unviewable: debris lives in
/// `ParticleSystem` and does not move without `particles.step`, and the
/// pressure/heat a blast writes never propagates without `step_fields`. The
/// added phases are no-ops or non-CA-affecting for the four pre-existing
/// scenes (nothing promotes a liquid body, nothing schedules an active site,
/// and no liquid rule reads the field), so they do not move the measurements
/// already recorded against those.
fn advance(
    world: &mut World,
    particles: &mut ParticleSystem,
    blasts: &mut explosion::Blasts,
    parallel_driver: bool,
    step_no: usize,
    gnome: &mut Gnome,
    per_charge_reports: bool,
) {
    if parallel_driver {
        parallel::step(world);
    } else {
        update::step(world);
    }
    world.step_liquid_bodies();
    // M8 chunk bodies, in `App::update`'s own slot. Without this a promoted
    // body is lifted out of the grid and then never moves -- a collapse
    // would render as material simply disappearing, which is exactly the
    // kind of thing this harness exists to catch by eye.
    pixel_physics::sim::rigid::step_chunk_bodies(world);
    // M9: the gnome, in his `App::update` slot too. Input is scripted --
    // run right, jump once a second -- because a filmstrip's job is to
    // show the arc, not to be played. A no-op for every scene that
    // summons no player.
    if world.player.is_some() {
        gnome.act(world, step_no);
    }
    world.step_active_sites();
    // R5's report line: printed the frame a blast's last stage finishes,
    // not at the trigger frame (`fire_due_explosions`'s own `boom:` line is
    // that one) -- `cells_cleared`/`cells_held_by_containment` accumulate
    // across every stage, so the report is only complete once `blasts` goes
    // back to empty. Checked by transition (was active, now is not) rather
    // than printed unconditionally every frame `blasts` is non-empty, or
    // this would spam one line per stage instead of one per blast.
    let was_active = !blasts.is_empty();
    blasts.step(world, particles);
    if was_active && blasts.is_empty() {
        println!("  blast report: {}", blasts.last_blast_report());
    }
    // One line per blast that finished this frame, each naming its own
    // site. The line above cannot do this job for a run that fires more
    // than one charge: `last_blast_report` is a single slot, so two
    // overlapping blasts collapse into one line and eight of nine reports
    // are silently overwritten -- `CLAUDE.md`'s "did it fire at all", one
    // level up. The queue is drained every frame regardless of whether
    // anything is printed, so `Blasts::finished` never accumulates.
    //
    // **Gated, and the gate is not cosmetic.** With a single charge this
    // line says exactly what the line above it says, only with an `(x, y)`
    // in front -- and every measurement recorded in
    // `Reports/explosion-stone-review.md` §8-§13 was taken from a
    // single-charge run, so an unconditional second line would change the
    // stdout of every one of them for no information. See
    // `per_charge_reports` at its definition in `run_once`.
    for (bx, by, report) in blasts.drain_finished_reports() {
        if per_charge_reports {
            println!("  blast report ({bx}, {by}): {report}");
        }
    }
    particles.step(world);
    world.step_fields();
}

/// The scripted gnome, and the tally of what his verbs actually did.
///
/// Scripted rather than played because a contact sheet is judged on the
/// shape of an arc or the opening of a bore, and a script that varied per
/// run would make two sheets incomparable. The counters are here for the
/// separate reason `CLAUDE.md` records: "did it fire at all needs a
/// counter". A bore full of loose rubble and a bore the dig never touched
/// are the same picture at the zoom these are read at, and `buried` is a
/// flag no picture shows at all.
struct Gnome {
    /// The tuning this run digs with — `Tuning::default` except for
    /// whatever `yield=` overrode.
    tuning: pixel_physics::sim::player::Tuning,
    /// Which script — set from the scene name, since the M9 scenes each
    /// exist to show a different verb.
    script: Script,
    /// Bites that actually landed (the cooldown swallows most frames).
    bites: usize,
    /// Where he was standing when he set off, so the sheet can report how
    /// far he actually got. See `Script::Wood`.
    start_x: Option<f32>,
    /// Whether `Script::Climb` has hold of something yet, and the height it
    /// had when it grabbed. The rise from there is the number the sheet is
    /// read for — see `Script::Climb`.
    grabbed: bool,
    grabbed_at: f32,
    highest: f32,
    shakes: usize,
    dislodged_by_shaking: usize,
    shed: usize,
    seeds: usize,
    shaken_cells: usize,
    shaken_shoot: u32,
    /// Loose cells shoved clear of a bore, summed over every bite.
    displaced: usize,
    dusted: usize,
    /// First tick he read as buried, and the first tick he was free
    /// again after that — the two numbers `scene=bury` exists to produce.
    went_under: Option<usize>,
    came_back: Option<usize>,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum Script {
    /// `scene=gnome`: run right, jumping once a second.
    #[default]
    Course,
    /// `scene=tunnel`: walk right into the cliff, digging ahead of him.
    Tunnel,
    /// `scene=bury`: stand still until the sand lands, then dig out.
    Bury,
    /// `scene=swim`: sink, float, pull under with `S`, then jump clear.
    Swim,
    /// `scene=ride`: no input at all — the shelf under him gives way and
    /// the only question is whether he goes with it.
    Ride,
    /// `scene=wood`: stand still while the stand grows, then walk the
    /// length of it.
    Wood,
    /// `scene=climb`: walk until something is in reach, then go up it.
    Climb,
    /// `scene=shake`: walk until a tree is in reach, then keep shaking it.
    Shake,
}

/// How long `Script::Wood` waits before setting off.
///
/// A grove is planted as *seeds*, and the sheets that judge tree shape are
/// shot at `start=8000`. Walking into a plot of bare soil would answer
/// nothing, so he holds still until there is a wood to walk into.
const WOOD_WALK_FROM: usize = 6000;

/// How long `Script::Climb` walks before it starts reaching for a hold —
/// far enough to be standing in a tree rather than beside a stray twig.
/// See the arm in `act` for the run that made this necessary.
const CLIMB_WALK_TICKS: usize = 60;

impl Gnome {
    fn for_scene(scene: &str, dig_yield: f32) -> Self {
        let script = match scene {
            "tunnel" => Script::Tunnel,
            "bury" => Script::Bury,
            "swim" => Script::Swim,
            "ride" => Script::Ride,
            "wood" => Script::Wood,
            "climb" => Script::Climb,
            "shake" => Script::Shake,
            _ => Script::Course,
        };
        Self {
            script,
            tuning: pixel_physics::sim::player::Tuning { dig_yield, ..Default::default() },
            bites: 0,
            start_x: None,
            grabbed: false,
            grabbed_at: 0.0,
            highest: 0.0,
            shakes: 0,
            dislodged_by_shaking: 0,
            shed: 0,
            seeds: 0,
            shaken_cells: 0,
            shaken_shoot: 0,
            displaced: 0,
            dusted: 0,
            went_under: None,
            came_back: None,
        }
    }

    /// One tick of scripted input, plus the dig the scripts that dig ask
    /// for. Ordered dig-then-step, matching `App`: the click is handled
    /// on the input event, the character steps afterwards, so the
    /// depenetration pass in `step` sees the cells the dig just freed and
    /// can stand him up in them on the same frame.
    fn act(&mut self, world: &mut World, step_no: usize) {
        use pixel_physics::sim::player::{self, PlayerInput};
        let tuning = self.tuning;
        let phase = step_no % 60;
        let input = match self.script {
            Script::Course => PlayerInput {
                right: true,
                jump_pressed: phase == 30,
                jump_held: (30..48).contains(&phase),
                ..Default::default()
            },
            // Dig standing still, then walk in — alternating, rather
            // than holding `right` the whole time as this first did.
            // Walking constantly into the face makes him climb his own
            // spoil, and the sheet then measures *ramp* rather than
            // *cave*: at `yield=1.0`, where by construction no volume
            // leaves and no cave can exist, he travelled furthest of any
            // setting purely by walking up rubble.
            Script::Tunnel => PlayerInput { right: step_no % 90 >= 60, ..Default::default() },
            Script::Bury => PlayerInput::default(),
            // Three acts, on a fixed clock so two sheets compare: fall in
            // and float (to 150), pull under (to 260), then stroke up and
            // jump out.
            Script::Swim => PlayerInput {
                down: (150..260).contains(&step_no),
                jump_held: step_no >= 260,
                jump_pressed: step_no >= 260,
                ..Default::default()
            },
            Script::Ride => PlayerInput::default(),
            Script::Wood => PlayerInput { right: step_no >= WOOD_WALK_FROM, ..Default::default() },
            // Walk until he has a handhold, then hold `W` and nothing
            // else. Holding a direction *while* climbing shimmies him
            // sideways out of the trunk, which is how you leave a tree and
            // is not what this scene is showing.
            //
            // **Walk a fixed distance first, then climb.** The first
            // version reached for the first handhold it met, which was a
            // creeping twig at ground level twelve cells from where he
            // spawned. He gripped it, rose, left it, launched, fell back
            // in, gripped again -- and the counter reported "climbed 30
            // cells" off a stack of grab-and-launch cycles at knee height,
            // with the trees still a hundred cells away. The number was
            // real and meant nothing, which is the exact trap `CLAUDE.md`
            // opens by warning about; the picture is what caught it.
            Script::Climb if self.grabbed => PlayerInput { grab: true, jump_held: true, ..Default::default() },
            // Same walk-first delay `Script::Climb` needed, and for the
            // same reason: the first thing in reach of the spawn point is a
            // creeping twig, not a tree.
            Script::Shake => PlayerInput {
                right: step_no >= WOOD_WALK_FROM && (step_no < WOOD_WALK_FROM + CLIMB_WALK_TICKS || !self.grabbed),
                ..Default::default()
            },
            Script::Climb => PlayerInput {
                right: step_no >= WOOD_WALK_FROM,
                // Reaching only starts once he is clear of the twig — walk
                // first, then walk *and* reach until something takes.
                // `grab` is the reach: climbing has its own key now, so
                // holding `W` alone takes hold of nothing.
                grab: step_no >= WOOD_WALK_FROM + CLIMB_WALK_TICKS,
                jump_held: step_no >= WOOD_WALK_FROM + CLIMB_WALK_TICKS,
                ..Default::default()
            },
        };
        if self.script == Script::Wood && step_no == WOOD_WALK_FROM {
            self.start_x = world.player.as_ref().map(|p| p.x);
        }
        if self.script == Script::Shake && step_no >= WOOD_WALK_FROM + CLIMB_WALK_TICKS {
            let target = world
                .player
                .as_ref()
                .and_then(|p| player::shake_target(world, p, (WIDTH, 190), &tuning));
            if let Some(at) = target {
                self.grabbed = true;
                let shaken = world.get(at.0, at.1).organism_id();
                self.shaken_shoot = world.organism(shaken).map(|o| o.shoot_cells).unwrap_or(0);
                if let Some(s) = player::shake(world, at, &tuning) {
                    self.shaken_cells = s.cells;
                    self.shakes += 1;
                    self.dislodged_by_shaking += s.dislodged;
                    self.shed += s.shed;
                    self.seeds += s.seeds;
                }
            }
        }
        if self.script == Script::Climb {
            if let Some(p) = world.player.as_ref() {
                if p.climbing && !self.grabbed {
                    self.grabbed = true;
                    self.grabbed_at = p.y;
                    self.highest = p.y;
                }
                self.highest = self.highest.min(p.y);
            }
        }
        // Aim: straight ahead at his own height for the tunnel, and
        // anywhere at all while buried, since a buried bite auto-aims.
        let digging = match self.script {
            Script::Course | Script::Swim | Script::Ride | Script::Wood | Script::Climb => false,
            // Handled below rather than through the dig path: the same
            // left button, a different verb.
            Script::Shake => false,
            Script::Tunnel => true,
            Script::Bury => step_no > 90,
        };
        if digging {
            // Aimed past his reach, not at a fixed 20 cells ahead, which
            // is what this did first. The aim ray is clipped to
            // `dig_reach`, so a cursor closer than the working face makes
            // the bite land in the open bore behind it — the tunnel
            // stopped advancing once it grew past the aim point, and the
            // dust count collapsed to a handful of cells a bite while
            // looking like the mechanism had failed.
            let (cx, cy) = world.player.as_ref().expect("scene summoned one").center();
            let far = cx + tuning.dig_reach as i32 * 2;
            if let Some(bite) = player::dig(world, (far, cy), &tuning) {
                self.bites += 1;
                self.displaced += bite.displaced;
                self.dusted += bite.dusted;
            }
        }
        player::step(world, input, &tuning);
        let p = world.player.as_ref().expect("still summoned");
        if p.buried && self.went_under.is_none() {
            self.went_under = Some(step_no);
        }
        if !p.buried && self.went_under.is_some() && self.came_back.is_none() {
            self.came_back = Some(step_no);
        }
    }

    /// How far he has come since setting off, or 0 if he never did.
    fn travelled(&self, world: &World) -> i32 {
        match (self.start_x, world.player.as_ref()) {
            (Some(from), Some(p)) => (p.x - from) as i32,
            _ => 0,
        }
    }

    /// The line printed beside each tile. Empty for scenes with no gnome,
    /// so the sheets that predate M9 read exactly as they did.
    fn report(&self, world: &World) -> Option<String> {
        let p = world.player.as_ref()?;
        let mut s = format!(
            "    gnome: at ({:.0}, {:.0}), {}, bites {} ({} cells displaced)",
            p.x,
            p.y,
            if p.buried {
                "BURIED"
            } else if p.swimming {
                "swimming"
            } else if p.wading {
                "wading"
            } else if p.grounded {
                "grounded"
            } else {
                "airborne"
            },
            self.bites,
            self.displaced
        );
        s.push_str(&format!(", {} dusted", self.dusted));
        // How much material is left in the world at all. The one number
        // that says whether a bore can exist: `mine` conserves cells, so
        // without thinning this never moves and no cave is possible
        // however much rubble is thrown about.
        let held: usize = (0..HEIGHT)
            .map(|y| (0..WIDTH).filter(|&x| world.get(x, y).material != material::EMPTY).count())
            .sum();
        s.push_str(&format!(", world holds {held} cells"));
        if let Some(from) = self.start_x {
            s.push_str(&format!(", travelled {:.0} cells", p.x - from));
        }
        // How much of him a tree is actually covering this frame.
        //
        // The depth effect is invisible in a still unless the sheet happens
        // to catch a frame with real overlap, and hunting for one by eye is
        // exactly the "an image says what and where, only a number says how
        // much" trap. This says whether there was anything to see.
        let (px0, py0, px1, py1) = p.bounds();
        let covered = (py0..=py1)
            .flat_map(|y| (px0..=px1).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let c = world.get(x, y);
                c.organism_id() != 0 && world.materials.get(c.material).climbable
            })
            .count();
        s.push_str(&format!(", {covered}/{} cells behind foliage", (px1 - px0 + 1) * (py1 - py0 + 1)));
        if self.script == Script::Shake {
            // What is being shaken, as well as what came of it. A shake
            // that reaches only the trunk it was grabbed by and one that
            // reaches the crown look identical on a contact sheet, and the
            // first version of the cap made exactly that mistake.
            s.push_str(&format!(", shaking {} of a {}-shoot plant", self.shaken_cells, self.shaken_shoot));
            s.push_str(&format!(
                ", {} shakes: {} knocked loose, {} leaves down, {} sown",
                self.shakes, self.dislodged_by_shaking, self.shed, self.seeds
            ));
        }
        if self.script == Script::Climb {
            match self.grabbed {
                true => s.push_str(&format!(", climbed {:.0} cells (gripped at y={:.0})", self.grabbed_at - self.highest, self.grabbed_at)),
                false => s.push_str(", NEVER GRIPPED"),
            }
        }
        if let Some(under) = self.went_under {
            s.push_str(&format!(", went under at {under}"));
            match self.came_back {
                Some(back) => s.push_str(&format!(", dug out by {back} ({} ticks under)", back - under)),
                None => s.push_str(", still under"),
            }
        }
        Some(s)
    }
}

/// Print the materials in `args.dump` as ASCII. See `Args::dump`.
fn dump_materials(world: &World, args: &Args) {
    let Some(r) = args.dump else { return };
    let player = world.player.as_ref().map(|p| p.bounds());
    println!("    dump x {}..{} y {}..{}:", r.min_x, r.max_x, r.min_y, r.max_y);
    for y in r.min_y..=r.max_y {
        let mut line = String::new();
        for x in r.min_x..=r.max_x {
            if let Some((x0, y0, x1, y1)) = player {
                if x >= x0 && x <= x1 && y >= y0 && y <= y1 {
                    line.push('P');
                    continue;
                }
            }
            let cell = world.get(x, y);
            line.push(match world.materials.kind(cell.material) {
                _ if cell.material == material::EMPTY => '.',
                MaterialKind::Solid => '#',
                MaterialKind::Powder => 'o',
                MaterialKind::Liquid => '~',
                MaterialKind::Gas => ':',
                _ => '?',
            });
        }
        println!("    {y:>3} {line}");
    }
}

/// Print whatever structural probes were asked for. Separate from the tile
/// line above because these scan the world and the tile line does not — a
/// scene that isn't about structure should pay nothing for this existing.
fn report_loads(world: &World, args: &Args) {
    for &(x, y) in &args.probes {
        match pixel_physics::sim::load::evaluate(world, x, y) {
            Some(l) => println!(
                "    load ({x},{y}): mass {} torque {} capacity {} stress {:.2}{}{}",
                l.mass,
                l.torque,
                l.capacity,
                l.stress(),
                if l.supported { "" } else { " UNSUPPORTED" },
                if l.truncated { " TRUNCATED" } else { "" },
            ),
            // Says *which* of the reasons, because "nothing here" covers
            // both "solid rock that cannot fail" and "this cell is gone",
            // and confusing those wastes a session.
            None => {
                let cell = world.get(x, y);
                let name = &world.materials.get(cell.material).name;
                println!("    load ({x},{y}): not evaluated -- {name}, aux {}, attached {}", cell.aux(), cell.attached());
            }
        }
    }
    if !args.loadmap {
        return;
    }
    let mut worst: Option<((i32, i32), pixel_physics::sim::load::Load)> = None;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let Some(l) = pixel_physics::sim::load::evaluate(world, x, y) else { continue };
            if worst.is_none_or(|(_, w)| l.stress() > w.stress()) {
                worst = Some(((x, y), l));
            }
        }
    }
    match worst {
        Some(((x, y), l)) => println!(
            "    worst stress: {:.2} at ({x},{y}) -- mass {} torque {} capacity {}{}",
            l.stress(),
            l.mass,
            l.torque,
            l.capacity,
            if l.supported { "" } else { " UNSUPPORTED" }
        ),
        None => println!("    worst stress: nothing evaluable in the world"),
    }
}

/// Assert what the scene was supposed to do, and exit non-zero if it did
/// not. Returns whether everything asked for held.
///
/// # Why the mechanism is asserted and not just the outcome
///
/// Because "it still stands" is true of a structure that is standing and
/// equally true of one nothing ever looked at. That is not hypothetical:
/// `scene=capped` was recorded as passing its acceptance case -- "the
/// thick column still stands, worst stress 0.00" -- while the whole
/// 15,840-cell structure was frozen, every cell still at `aux 0`, and not
/// one of them had ever been load-evaluated. The assertion was true and
/// meant nothing, which is `CLAUDE.md`'s vacuous-test failure arriving in
/// the acceptance harness instead of the test suite.
///
/// So a scene that is supposed to collapse must show the *criterion
/// firing* (`min_overloaded`), not merely that material moved; and a scene
/// that is supposed to stand must show that nothing fired
/// (`max_failures`), which is only meaningful once the same binary has
/// demonstrated it can fire at all on the collapsing scenes.
fn check_expectations(world: &World, args: &Args, gnome: &Gnome, best_ms: f64, peak_bodies: usize, cells_before: (i64, i64), cave_before: i64) -> bool {
    let f = world.structural_failures;
    let mut ok = true;
    if let Some(pct) = args.min_cave {
        let now = roofed_void(world);
        let kept = if cave_before == 0 { 0 } else { now * 100 / cave_before };
        if kept < pct {
            println!("  FAIL: the cave did not survive -- {kept}% of its roofed void left ({now} of {cave_before} cells), wanted {pct}%");
            ok = false;
        }
    }
    if let Some(pct) = args.max_cave {
        let now = roofed_void(world);
        let kept = if cave_before == 0 { 100 } else { now * 100 / cave_before };
        if kept > pct {
            println!("  FAIL: the roof is still up -- {kept}% of its roofed void left ({now} of {cave_before} cells), wanted at most {pct}%");
            ok = false;
        }
    }
    if let Some(max) = args.max_lost {
        let lost = (cells_before.0 + cells_before.1) - occupied(world);
        if lost > max {
            println!("  FAIL: expected the world to lose at most {max} cells after the cut, lost {lost}");
            ok = false;
        }
    }
    if let Some(limit) = args.max_frame_ms {
        if best_ms > limit {
            println!("  FAIL: worst frame {best_ms:.2} ms over the {limit:.1} ms budget (best of {} runs)", args.repeat);
            ok = false;
        }
    }
    if let Some(min) = args.min_bodies {
        if peak_bodies < min {
            println!("  FAIL: expected at least {min} chunk bodies in flight at once, peaked at {peak_bodies}");
            ok = false;
        }
    }
    if let Some(min) = args.min_travelled {
        let went = gnome.travelled(world);
        if went < min {
            println!("  FAIL: expected the gnome to cover at least {min} cells, he covered {went}");
            ok = false;
        }
    }
    if let Some(min) = args.min_overloaded {
        if f.overloaded < min {
            println!("  FAIL: expected at least {min} overload failures, got {}", f.overloaded);
            ok = false;
        }
    }
    if let Some(max) = args.max_failures {
        let total = f.overloaded + f.unsupported;
        if total > max {
            println!("  FAIL: expected at most {max} structural failures, got {total}");
            ok = false;
        }
    }
    // Printed unconditionally, not only when `min_bodies` is set: "did any
    // piece actually come away" is the counter half of every destruction
    // sheet (`CLAUDE.md`, "did it fire at all" needs a counter), and a body
    // whose whole life falls between two tiles is invisible in the per-tile
    // lines above.
    println!("  peak chunk bodies in flight at once: {peak_bodies}");
    if ok
        && (args.min_overloaded.is_some()
            || args.max_failures.is_some()
            || args.max_frame_ms.is_some()
            || args.min_bodies.is_some()
            || args.min_travelled.is_some()
            || args.max_cave.is_some())
    {
        println!("  OK: scene={} met its expectations", args.scene);
    }
    ok
}

fn main() {
    let args = parse();
    // Repeated runs are for the *timing* only -- the image and the
    // expectations come from the last one, which is a full run like any
    // other. Deliberately re-simulated from scratch rather than reusing a
    // warm world, since a second pass over an already-settled scene
    // measures nothing.
    let mut samples: Vec<f64> = Vec::new();
    for _ in 1..args.repeat {
        samples.push(run_once(&args, false).0);
    }
    let (last_ms, world, gnome, peak_bodies, cells_before, cave_before) = run_once(&args, true);
    samples.push(last_ms);
    let best = samples.iter().cloned().fold(f64::INFINITY, f64::min);
    if args.repeat > 1 {
        let worst = samples.iter().cloned().fold(0.0, f64::max);
        println!("worst frame over {} runs: {best:.2} ms (spread {best:.2}-{worst:.2})", args.repeat);
    }
    if !check_expectations(&world, &args, &gnome, best, peak_bodies, cells_before, cave_before) {
        std::process::exit(1);
    }
}

/// Gutter between tiles, in pixels, and the mid-grey it is filled with --
/// so a tile that is legitimately all-black stays distinguishable from the
/// space between tiles. Shared by the main sheet and the `panels=` one,
/// because two sheets read side by side with different gutters read as two
/// different instruments.
const TILE_GAP: i32 = 2;
const GUTTER_GREY: u8 = 90;

/// Blit one `crop`-sized window of a rendered world frame into `dst` at
/// `(ox, oy)`, magnified `zoom`x with nearest-neighbour replication.
///
/// One copy, used by the contact sheet, the GIF branch and the `panels=`
/// sheet. It was three copies of the same nested loop; the third was the
/// one that would have drifted.
fn blit_tile(dst: &mut [u8], dst_w: i32, (ox, oy): (i32, i32), frame: &[u8], crop: Rect, zoom: i32) {
    for y in 0..crop.height() {
        for x in 0..crop.width() {
            let (sx, sy) = (crop.min_x + x, crop.min_y + y);
            if sx < 0 || sy < 0 || sx >= WIDTH || sy >= HEIGHT {
                continue;
            }
            let src = (((sy * WIDTH) + sx) * 4) as usize;
            for zy in 0..zoom {
                for zx in 0..zoom {
                    let (dx, dy) = (ox + x * zoom + zx, oy + y * zoom + zy);
                    let d = (((dy * dst_w) + dx) * 4) as usize;
                    dst[d..d + 4].copy_from_slice(&frame[src..src + 4]);
                }
            }
        }
    }
}

/// The `W`x`H` window centred on a charge, **shifted** inside the world
/// rather than shrunk when it would run off an edge: every tile of a grid
/// has to be the same size or the grid does not compose, so a charge near
/// the world edge gets an off-centre view and never a small one.
fn panel_crop(x: i32, y: i32, w: i32, h: i32) -> Rect {
    let min_x = (x - w / 2).clamp(0, (WIDTH - w).max(0));
    let min_y = (y - h / 2).clamp(0, (HEIGHT - h).max(0));
    Rect::new(min_x, min_y, min_x + w - 1, min_y + h - 1)
}

/// `out`'s path with `-panels` inserted before the extension.
fn panels_path(out: &str) -> String {
    match out.rfind('.') {
        Some(i) if !out[i..].contains('/') && !out[i..].contains('\\') => {
            format!("{}-panels{}", &out[..i], &out[i..])
        }
        _ => format!("{out}-panels"),
    }
}

/// The `panels=` sheet in progress: one column per charge fired, one row
/// per age, each cell a crop centred on that charge's own site and captured
/// that many frames after **that charge's own detonation**.
struct PanelSheet {
    /// Crop size in world cells.
    w: i32,
    h: i32,
    ages: Vec<usize>,
    cols: usize,
    zoom: i32,
    sheet: Vec<u8>,
    sheet_w: i32,
    sheet_h: i32,
    /// Which grid cells have already been filled. Needed because the outer
    /// capture loop revisits a tile boundary frame -- `fire_due_*` is called
    /// once at `step_no == target` and again at the top of the next inner
    /// loop -- so without this a cell on such a frame would be rendered and
    /// blitted twice.
    filled: Vec<bool>,
    /// Its own `Renderer` and frame buffer, deliberately **not** the main
    /// sheet's. `Renderer::draw` advances an internal frame counter that the
    /// animated grain modes read, so sharing one would make the main sheet a
    /// function of how many panel captures happened to fall between its
    /// tiles. The main sheet has to stay byte-identical to a run without
    /// `panels=`, and this is how that is guaranteed rather than argued.
    renderer: Renderer,
    frame: Vec<u8>,
}

impl PanelSheet {
    fn new(args: &Args, w: i32, h: i32, ages: Vec<usize>, cols: usize) -> Self {
        let (tile_w, tile_h) = (w * args.zoom, h * args.zoom);
        let rows = ages.len() as i32;
        let sheet_w = cols as i32 * tile_w + (cols as i32 - 1) * TILE_GAP;
        let sheet_h = rows * tile_h + (rows - 1) * TILE_GAP;
        let mut sheet = vec![GUTTER_GREY; (sheet_w * sheet_h * 4) as usize];
        for p in sheet.chunks_exact_mut(4) {
            p[3] = 255;
        }
        let mut renderer = Renderer::new();
        renderer.grain = args.grain;
        renderer.organism_overlay = args.organism_overlay;
        renderer.field_overlay = args.field_overlay;
        renderer.daylight_pin = args.daylight;
        Self {
            w,
            h,
            filled: vec![false; ages.len() * cols],
            ages,
            cols,
            zoom: args.zoom,
            sheet,
            sheet_w,
            sheet_h,
            renderer,
            frame: vec![0u8; (WIDTH * HEIGHT * 4) as usize],
        }
    }

    /// Fill every grid cell whose moment is this frame. Renders the world at
    /// most once per frame and only when something actually wants it.
    fn capture(&mut self, world: &World, particles: &ParticleSystem, fired: &[FiredCharge], step_no: usize) {
        let mut drawn = false;
        for (col, c) in fired.iter().enumerate().take(self.cols) {
            for row in 0..self.ages.len() {
                let idx = row * self.cols + col;
                if self.filled[idx] || c.frame + self.ages[row] != step_no {
                    continue;
                }
                if !drawn {
                    // `force_full`, for the same reason the main sheet uses
                    // it: this must draw the whole world regardless of what
                    // moved, or a tile inherits pixels from whichever frame
                    // last touched them. The empty `touched` set is not a
                    // shortcut -- `force_full` makes it unread.
                    self.renderer.draw(world, particles, &HashSet::new(), &mut self.frame, (WIDTH as u32, HEIGHT as u32), true);
                    drawn = true;
                }
                let (tile_w, tile_h) = (self.w * self.zoom, self.h * self.zoom);
                let origin = (col as i32 * (tile_w + TILE_GAP), row as i32 * (tile_h + TILE_GAP));
                blit_tile(&mut self.sheet, self.sheet_w, origin, &self.frame, panel_crop(c.x, c.y, self.w, self.h), self.zoom);
                self.filled[idx] = true;
            }
        }
    }
}

/// One full run. Returns its worst frame in ms, the finished world, the
/// peak concurrent body count and how much material the world held *before*
/// the first step. `render` is false for the extra timing samples, which do
/// not need an image and should not pay for one.
fn run_once(args: &Args, render: bool) -> (f64, World, Gnome, usize, (i64, i64), i64) {
    let mut world = build(args);
    // Censused before the first step and after the last, because a failure
    // count cannot answer "how much did this eat" -- see `Args::max_lost`.
    // Taken here rather than in `build` so it includes whatever the scene
    // cut on construction: the dig is part of what the run costs.
    let cells_before = census(&world);
    let cave_before = roofed_void(&world);
    // The whole material grid, not a total. `census` answers *how much* was
    // lost; this answers *where*, which is the containment question and the
    // one nothing in the engine could answer honestly -- see
    // `damage_radius`. One `Vec<MaterialId>` over a 512x320 world is 320 kB
    // and is taken once per run.
    let materials_before: Vec<material::MaterialId> = (0..HEIGHT).flat_map(|y| (0..WIDTH).map(move |x| (x, y))).map(|(x, y)| world.get(x, y).material).collect();
    let mut renderer = Renderer::new();
    renderer.grain = args.grain;
    renderer.tree_depth = args.tree_depth;
    renderer.organism_overlay = args.organism_overlay;
    renderer.field_overlay = args.field_overlay;
    renderer.daylight_pin = args.daylight;
    let mut particles = ParticleSystem::new();
    let mut pending = args.explosions.clone();
    let mut pending_blasts = args.blasts.clone();
    // Where each charge actually went off, in fire order. `blast=`'s site
    // is not known until it fires, so this cannot be read off `Args`.
    let mut fired: Vec<FiredCharge> = Vec::new();
    // Whether this run prints the **per-charge** report and census lines.
    //
    // Gated rather than unconditional, and not to flatter a diff: with a
    // single `explode=` charge every one of those lines is a restatement of
    // a line already printed (`blast report:` and the boxed
    // `cracked cells within 3x radius of ...` are both about that one
    // charge), and every measurement recorded in
    // `Reports/explosion-stone-review.md` §8-§13 was taken from exactly
    // such a run. Adding a duplicate line to all of them buys nothing and
    // changes every recorded baseline. The lines earn their place the
    // moment a run fires more than one charge, which is what they were
    // built for -- and `blast=` is never a single-charge idiom in practice,
    // so it opts in on its own.
    let per_charge_reports = !args.blasts.is_empty() || args.explosions.len() > 1;
    let mut pending_cuts = args.cuts.clone();
    let mut pending_depowder = args.depowder;
    let mut depowder_first = true;
    let mut blasts = explosion::Blasts::new();
    if let Some(v) = args.joint_reach {
        blasts.tuning.joint_reach = v;
    }
    if let Some(v) = args.joint_open {
        blasts.tuning.joint_open_fraction = v;
    }
    if let Some(v) = args.joint_density {
        blasts.tuning.joint_density = v;
    }
    if let Some(v) = args.crack_rays {
        blasts.tuning.crack_rays = v;
    }
    if let Some(v) = args.smoke {
        blasts.tuning.smoke_fraction = v;
    }
    let mut gnome = Gnome::for_scene(&args.scene, args.dig_yield);
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];

    let (cw, ch) = (args.crop.width(), args.crop.height());
    let (tile_w, tile_h) = (cw * args.zoom, ch * args.zoom);
    let gap = TILE_GAP;
    let rows = args.count.div_ceil(args.cols) as i32;
    let sheet_w = args.cols as i32 * tile_w + (args.cols as i32 - 1) * gap;
    let sheet_h = rows * tile_h + (rows - 1) * gap;
    // Mid-grey gutters, so a tile that is legitimately all-black stays
    // distinguishable from the space between tiles.
    let mut sheet = vec![GUTTER_GREY; (sheet_w * sheet_h * 4) as usize];
    for p in sheet.chunks_exact_mut(4) {
        p[3] = 255;
    }

    // GIF branch: motion is for a human to watch, and several of these
    // artifacts (a fringe that regenerates every frame, water that reads as
    // frozen because its grain is nailed to the screen) simply do not survive
    // being sampled into stills. Consecutive frames, real playback speed, and
    // a NETSCAPE loop -- the same reasoning `main.rs`'s capture hook records.
    if args.gif {
        let mut frames = Vec::with_capacity(args.count);
        let mut step_no = 0usize;
        for i in 0..args.count {
            let target = args.start + i * args.every;
            while step_no < target {
                fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, &mut pending_blasts, &mut fired, step_no);
                fire_due_cuts(&mut world, &mut pending_cuts, step_no);
            fire_due_depowder(&mut world, &mut pending_depowder, &mut depowder_first, step_no);
                advance(&mut world, &mut particles, &mut blasts, args.parallel_driver, step_no, &mut gnome, per_charge_reports);
                step_no += 1;
            }
            fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, &mut pending_blasts, &mut fired, step_no);
            fire_due_cuts(&mut world, &mut pending_cuts, step_no);
            fire_due_depowder(&mut world, &mut pending_depowder, &mut depowder_first, step_no);
            let touched: HashSet<_> = world.take_touched_chunks();
            renderer.draw(&world, &particles, &touched, &mut frame, (WIDTH as u32, HEIGHT as u32), true);

            let mut tile = vec![0u8; (tile_w * tile_h * 4) as usize];
            blit_tile(&mut tile, tile_w, (0, 0), &frame, args.crop, args.zoom);
            frames.push(tile);
        }

        // 60 ticks/second is the sandbox's own fixed simulation rate, so
        // `every` ticks between captures maps directly to elapsed time.
        let delay_ms = (args.every as u64 * 1000) / 60;
        let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(delay_ms.max(16)));
        let file = std::fs::File::create(&args.out).expect("creating the gif");
        let mut encoder = image::codecs::gif::GifEncoder::new(file);
        encoder.set_repeat(image::codecs::gif::Repeat::Infinite).expect("gif repeat");
        for tile in frames {
            let buf = image::RgbaImage::from_raw(tile_w as u32, tile_h as u32, tile).expect("gif frame");
            encoder.encode_frame(image::Frame::from_parts(buf, 0, 0, delay)).expect("gif frame");
        }
        drop(encoder);
        println!("animated gif ({tile_w}x{tile_h}, {} frames): {}", args.count, args.out);
        // The gif branch is for watching motion, not for measuring; it has
        // no per-frame timing of its own and `repeat`/expectations do not
        // apply to it.
        // The gif branch is for watching motion, not measuring: no
        // per-frame timing and no body sampling, so it reports neither.
        return (0.0, world, gnome, 0, cells_before, cave_before);
    }

    // `panels=`: a second sheet, one column per charge and one row per age.
    // Built after the GIF branch has had its chance to return, because that
    // branch writes an animation rather than a grid and has no second sheet
    // to write.
    let charges = args.explosions.len() + args.blasts.len();
    let mut panels = match (&args.panels, render) {
        (Some((pw, ph, ages)), true) => {
            assert!(charges > 0, "panels= needs at least one explode= or blast= charge -- a column per charge is the whole grid");
            Some(PanelSheet::new(args, *pw, *ph, ages.clone(), charges))
        }
        _ => None,
    };
    // The last frame any panel wants a picture of. A charge fired at 3400
    // and sampled 900 frames later needs 4300, which the main sheet's own
    // `start`/`every`/`count` knows nothing about -- so the run has to be
    // extended to reach it. Computed for the timing-only repeats as well as
    // the rendered run, so `repeat=` compares runs of the same length.
    let panel_last_frame = args.panels.as_ref().map(|(_, _, ages)| {
        let last_age = ages.iter().copied().max().unwrap_or(0);
        let last_charge = args.explosions.iter().map(|c| c.4).chain(args.blasts.iter().map(|c| c.4)).max().unwrap_or(0);
        last_charge + last_age
    });

    let mut captured = 0usize;
    let mut step_no = 0usize;
    // Worst *single* frame, not the mean, and reported per tile rather than
    // once at the end. `Reports/fracture-mechanics-design.md` §3.4 names the
    // cascade spike as the thing to watch -- each break changes loads and
    // triggers more breaks in the same frame -- and a mean over 250 frames
    // hides exactly that. Per tile, because it localizes the spike to a
    // phase of the scene instead of just proving one happened.
    let mut worst_ms = 0.0f64;
    let mut worst_frame = 0usize;
    // Sampled every frame, not just at capture: a body's whole life can
    // fall between two tiles, and "bodies 0" in every tile of a scene that
    // visibly threw rock is exactly the confusion this harness exists to
    // prevent.
    let mut peak_bodies = 0usize;
    while captured < args.count {
        let target = args.start + captured * args.every;
        while step_no < target {
            fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, &mut pending_blasts, &mut fired, step_no);
            fire_due_cuts(&mut world, &mut pending_cuts, step_no);
            fire_due_depowder(&mut world, &mut pending_depowder, &mut depowder_first, step_no);
            // Outside the timed region below on purpose: a panel capture is
            // harness cost, and folding it into the worst-frame number would
            // make the sheet's own instrument report the sheet.
            if let Some(p) = panels.as_mut() {
                p.capture(&world, &particles, &fired, step_no);
            }
            let began = std::time::Instant::now();
            advance(&mut world, &mut particles, &mut blasts, args.parallel_driver, step_no, &mut gnome, per_charge_reports);
            let ms = began.elapsed().as_secs_f64() * 1000.0;
            // Frame 0 is excluded, and not to flatter the number: every
            // scene spikes there, including `terrain`, which runs no
            // structural work at all. It is chunk and field allocation plus
            // first-touch page faults, paid once at startup, and leaving it
            // in made all seven scenes report the same ~70-110 ms and hid
            // the differences between them entirely.
            peak_bodies = peak_bodies.max(world.chunk_bodies.len());
            if ms > worst_ms && step_no > 0 {
                worst_ms = ms;
                worst_frame = step_no;
            }
            step_no += 1;
        }
        fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, &mut pending_blasts, &mut fired, step_no);
        fire_due_cuts(&mut world, &mut pending_cuts, step_no);
            fire_due_depowder(&mut world, &mut pending_depowder, &mut depowder_first, step_no);
        if let Some(p) = panels.as_mut() {
            p.capture(&world, &particles, &fired, step_no);
        }
        // `force_full`, not the dirty-rect path: this must draw the whole
        // world every time regardless of what moved, or a tile would inherit
        // pixels from whichever frame last touched them.
        let touched: HashSet<_> = world.take_touched_chunks();
        renderer.draw(&world, &particles, &touched, &mut frame, (WIDTH as u32, HEIGHT as u32), true);

        let (gx, gy) = (captured as i32 % args.cols as i32, captured as i32 / args.cols as i32);
        blit_tile(&mut sheet, sheet_w, (gx * (tile_w + gap), gy * (tile_h + gap)), &frame, args.crop, args.zoom);
        // `bodies` reports M8 chunk bodies in flight. Worth printing rather
        // than inferring from the image: a coherent falling slab and a
        // tightly-packed scatter of loose grains look nearly identical at
        // the zoom levels these sheets are usually read at, so "did this
        // actually promote to a body" is a question the picture cannot
        // answer on its own.
        if !render {
            captured += 1;
            continue;
        }
        println!(
            "  tile {captured}: frame {target}, awake {}/{}, sites {}, particles {}, bodies {} ({} cells)",
            world.active_chunk_count(),
            world.chunk_count(),
            world.active_site_count(),
            particles.len(),
            world.chunk_bodies.len(),
            world.chunk_bodies.iter().map(|b| b.cells.len()).sum::<usize>(),
        );
        // Which failure fired, cumulatively. An overloaded piece and a
        // piece that was never held look identical falling, so the image
        // cannot say which mechanism produced what is on screen -- and
        // those are the two halves of the model, with different causes and
        // different bugs. `CLAUDE.md`: print the count next to the image
        // and read both.
        if let Some(line) = gnome.report(&world) {
            println!("{line}");
        }
        let f = world.structural_failures;
        println!(
            "    failures: overloaded {} ({} cells), unsupported {} ({} cells)",
            f.overloaded, f.overloaded_cells, f.unsupported, f.unsupported_cells
        );
        // **Relabelled to what it actually holds.** It was printed as
        // "furthest a failure landed from its trigger", which is not what
        // `max_chain_reach` is: it is Manhattan `|failure.at - (x, y)|`,
        // the distance from the checked cell to the failing ancestor,
        // bounded by construction to `ROOTWARD_CHECK_STEPS` hops. Read as a
        // containment number it is worse than useless -- on a rolling-world
        // blast it reads 1 cell while damage is landing everywhere.
        println!("    furthest a failure's root was from the cell that was checked: {} cells", f.max_chain_reach);
        // ...and the number that *is* the containment measure, in the units
        // the `F9` setting is written in so the two can be read against each
        // other. The reach is named rather than printed raw: `i32::MAX` as a
        // number is unreadable, and a sheet that does not say which mode
        // produced it cannot be compared to the one beside it.
        // ...and the number that *was* meant to be the containment measure.
        // **It cannot report a containment failure, and it is kept only so
        // the two can be read side by side.** It is recorded exclusively at
        // sites downstream of `clip_region_to_licence`, and for any cell
        // that clip retains `within_disturbance` guarantees a live
        // disturbance within `chain_reach + extent` while
        // `distance_to_live_disturbance` takes the *min* over disturbances
        // of `distance - extent` -- so it is `<= chain_reach` by arithmetic.
        // A run reading exactly 48 at LOCAL and exactly 16 at TIGHT is a
        // saturated ceiling. See `damage_radius`, printed beneath it, which
        // reads none of that machinery.
        println!(
            "    furthest damage landed from a live disturbance: {} cells (chain_reach = {}) -- CEILING, see below",
            f.max_damage_reach,
            chain_reach_name(world.chain_reach)
        );
        let (blast_reach, blast_past) = damage_radius(&world, &materials_before, &fired);
        println!("    furthest cell this run actually changed, from the charge that made it: {blast_reach} cells ({blast_past} past that charge's own radius)");
        // R3a's "did it fire at all" counter. A failure too big for one
        // tick comes down over several, and the `bodies` line above shows
        // that as a *series* of bursts -- but a series of bursts is also
        // what several independent failures look like, and a contact sheet
        // cannot tell them apart at all. The count says which it was.
        println!("    of those, paced across ticks: {} slice(s), {} cells", f.staged_slices, f.staged_cells);
        // The only line in this block that counts a *displacement*. Every
        // number above it -- `failures`, the reach, the paced slices -- is
        // recorded at `structural.rs`'s `record` call, which runs before
        // the free-face test, the boundary erosion, the slicing and the
        // fracture. So "unsupported 400 (12,000 cells)" is entirely
        // consistent with not one cell having moved, and that is not a
        // hypothetical: it is the exact shape of the owner's "no pieces
        // move, ever, not even chunks well over 8 cells, and nothing turns
        // to dust either" against a harness reporting hundreds of
        // failures. The census line below closes the gap one step further
        // along for *material*; this closes it for *motion*, and the two
        // are different questions -- rubble standing where it fell is a
        // loss to neither.
        //
        // Both halves are printed, never one: their ratio is the block-
        // size distribution the ethos is about, and a run that promotes
        // nothing and a run that shatters nothing are two different bugs
        // that either number alone reads as the same.
        println!(
            "    of those, actually moved: {} bodies ({} cells promoted), {} cells shattered to rubble",
            f.promoted_bodies, f.promoted_cells, f.shattered_cells
        );
        // ...and the *shape* of that number, which the pair above cannot
        // give. See `FailureCounts::promoted_sizes`: a mean cannot tell a
        // run where everything came off the same size from one with a few
        // blocks, more cobbles and a lot of grit, and the second is what
        // the ethos asks for.
        let sz = f.promoted_sizes;
        println!(
            "    body sizes: <8:{} 8-15:{} 16-31:{} 32-63:{} 64-127:{} 128-255:{} 256+:{}",
            sz[0], sz[1], sz[2], sz[3], sz[4], sz[5], sz[6]
        );
        // How much of the damage happened to rock with nowhere to go --
        // the mid-mountain collapse the owner reports as looking fake.
        // A picture cannot answer this: a collapse at a cliff edge and one
        // eighty cells inside a massif are the same grey rubble at the
        // zoom a contact sheet is read at.
        println!("    of those, confined (no free face anywhere): {} ({} cells), deepest {} cells from air, {} cells fissured", f.confined, f.confined_cells, f.deepest_confined, f.crushed_cells);
        // The complaint itself, counted rather than looked at -- see
        // `severed_islands`. Printed beside `confined` because the two are
        // the same story from opposite ends: `confined` counts failures the
        // model *judged* and refused to move, this counts rock that is cut
        // free and standing there whether or not anything ever judged it.
        let (pieces, islands, island_cells, largest_piece) = severed_islands(&world);
        println!("    rock the fissures actually cut loose: {pieces} piece(s), largest {largest_piece} cells -- of those, wedged with no free face: {islands} ({island_cells} cells)");
        // How much the world has actually *lost* since the cut was made,
        // which the failure counts above cannot say: a failed cell that
        // became rubble is still standing there. Printed per tile rather
        // than once at the end so the trajectory is visible -- a run that
        // has stopped eating and one that is still going look identical in
        // a single total. See `Args::max_lost`.
        println!("    roofed void (cave volume): {} cells, was {} at the cut", roofed_void(&world), cave_before);
        println!("    gas cells standing in the world: {}", smoke_census(&world));
        let (solid, powder) = census(&world);
        println!(
            "    cells lost since the cut: {} (rock {:+}, rubble {:+})",
            (cells_before.0 + cells_before.1) - (solid + powder),
            solid - cells_before.0,
            powder - cells_before.1
        );
        // R1's halo, censused rather than inferred from the image: a
        // fissure line a cell wide reads as noise at the zoom a contact
        // sheet is usually read at (baseline measured 47 nearly-invisible
        // cells on `boom_stone`, per `Reports/explosion-stone-review.md`
        // §1a), so "did the crack halo actually fire" needs the same
        // counter-next-to-the-image treatment as `bodies` above. Boxed
        // around the *last* `explode=` site specifically -- a scene may
        // schedule more than one, and the box only means anything relative
        // to a single blast's own radius.
        if let Some(&(ex, ey, er, ..)) = args.explosions.last() {
            println!("    cracked cells within 3x radius of ({ex}, {ey}): {}", cracked_census(&world, ex, ey, er));
            // The anti-"permanent sticker" counter. Scorched stone has no
            // conductivity, so nothing in `fire.rs` ever cools it (its
            // thermally-inert fast path returns before any decay) -- a hot
            // ring written by a blast used to be *permanent*, and an image
            // cannot tell a glow that is fading from one that is frozen:
            // both are orange in a still. Max and count together, because
            // they answer different halves: the max says how bright the
            // brightest cell still is, the count says how much of the ring
            // is still lit at all.
            let (hottest, lit) = heat_census(&world, ex, ey, er);
            println!("    hottest cell within 3x radius: {hottest} C, cells above ambient: {lit}");
        }
        // The census above is boxed around the **last** `explode=` site,
        // which is one charge of however many the run fired -- with nine
        // charges it measures one of them and says nothing about the other
        // eight. It stays exactly as it is because recorded baselines quote
        // it; these are the ones that answer the question the sheet is
        // actually about. See `per_charge_reports` for why they are gated.
        if per_charge_reports {
            for c in &fired {
                println!("    cracked cells within 3x radius of ({}, {}): {}", c.x, c.y, cracked_census(&world, c.x, c.y, c.radius));
            }
            // A whole-world scan, and worth saying what it costs because
            // `CLAUDE.md` is emphatic that harness cost is still cost: it is
            // 512x320 cell reads **per tile**, not per frame -- a handful of
            // scans across a run of thousands of frames, next to which it
            // does not register. It is here because the boxes above cannot
            // answer it: cracks spread, and a halo that has walked out of
            // every box reads as the crack mechanism switching off.
            println!("    cracked cells in the world: {}", cracked_world_census(&world));
        }
        // Pieces or grit. A region below `MIN_FRACTURE_CELLS` is not
        // fractured at all -- it falls through to per-cell conversion,
        // which *is* powder -- so a run whose failures average 1 or 2
        // cells has already decided to produce dust no matter what the
        // fragment ladder is set to. Printed next to the image because the
        // two are indistinguishable at the zoom a contact sheet is read
        // at.
        let events = f.overloaded + f.unsupported;
        if events > 0 {
            println!(
                "    failing region size: mean {:.1} cells, largest {}",
                (f.overloaded_cells + f.unsupported_cells) as f64 / events as f64,
                f.largest_failure
            );
        }
        if render {
            println!("    worst frame so far: {worst_ms:.2} ms (frame {worst_frame})");
            report_loads(&world, args);
            dump_materials(&world, args);
        }
        captured += 1;
    }

    // Keep stepping to the last frame `panels=` wants a picture of. Only
    // entered when `panels=` is set, so a run without it ends exactly where
    // it used to -- which matters, because `check_expectations` and the
    // final census both read the world this leaves behind.
    if let Some(limit) = panel_last_frame {
        while step_no < limit {
            fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, &mut pending_blasts, &mut fired, step_no);
            fire_due_cuts(&mut world, &mut pending_cuts, step_no);
            fire_due_depowder(&mut world, &mut pending_depowder, &mut depowder_first, step_no);
            if let Some(p) = panels.as_mut() {
                p.capture(&world, &particles, &fired, step_no);
            }
            let began = std::time::Instant::now();
            advance(&mut world, &mut particles, &mut blasts, args.parallel_driver, step_no, &mut gnome, per_charge_reports);
            let ms = began.elapsed().as_secs_f64() * 1000.0;
            peak_bodies = peak_bodies.max(world.chunk_bodies.len());
            if ms > worst_ms && step_no > 0 {
                worst_ms = ms;
                worst_frame = step_no;
            }
            step_no += 1;
        }
        fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, &mut pending_blasts, &mut fired, step_no);
        fire_due_cuts(&mut world, &mut pending_cuts, step_no);
        fire_due_depowder(&mut world, &mut pending_depowder, &mut depowder_first, step_no);
        if let Some(p) = panels.as_mut() {
            p.capture(&world, &particles, &fired, step_no);
        }
        // The run went past its last tile, so the per-tile worst-frame line
        // above stopped before the end. Say what the whole run cost, or the
        // frames the extension paid for would be the only ones nobody timed.
        if render {
            println!("  panels: ran on to frame {step_no}; worst frame over the whole run {worst_ms:.2} ms (frame {worst_frame})");
        }
    }

    if render {
        image::save_buffer(&args.out, &sheet, sheet_w as u32, sheet_h as u32, image::ColorType::Rgba8)
            .expect("writing the contact sheet");
        println!("contact sheet ({sheet_w}x{sheet_h}, {} tiles): {}", args.count, args.out);
        // Said out loud, because a pinned exposure is invisible in the
        // image and a sheet nobody can reproduce is not evidence.
        if let Some(f) = args.daylight {
            println!("  drawn at a pinned daylight of {f} (render only -- the run itself was unaffected)");
        }
        if let Some(p) = panels {
            let path = panels_path(&args.out);
            image::save_buffer(&path, &p.sheet, p.sheet_w as u32, p.sheet_h as u32, image::ColorType::Rgba8)
                .expect("writing the panels sheet");
            println!("panels sheet ({}x{}, {} rows x {} cols): {}", p.sheet_w, p.sheet_h, p.ages.len(), p.cols, path);
            // A grid cell that never got a picture is a charge that never
            // fired, and a grey square is not a reading -- say so rather
            // than letting the gutter colour be mistaken for "nothing
            // happened there".
            let missing = p.filled.iter().filter(|f| !**f).count();
            if missing > 0 {
                println!("  panels: {missing} of {} cells never captured -- a charge scheduled past the end of the run", p.filled.len());
            }
        }
    }
    (worst_ms, world, gnome, peak_bodies, cells_before, cave_before)
}

/// How much rock and rubble the world is holding: `Solid` and `Powder`
/// only.
///
/// **Liquids and gases are excluded, and that is not tidiness -- counting
/// them makes the metric lie.** The first version counted every non-empty
/// cell and reported `canyon` *gaining* 167 cells over a run in which
/// nothing failed at all. A `Liquid` cell holds continuous fill, so one
/// full cell spreading into two half-full ones is +1 occupancy at
/// unchanged volume, and a preset with standing water manufactures
/// occupancy all run just by settling. That is `CLAUDE.md`'s "ask what a
/// metric counts when nothing is wrong" catching a bad metric on the case
/// that is fine, before it was trusted about a case that is not.
///
/// `Solid` + `Powder` is also exactly the right *question*: destruction
/// turns rock into rubble and rubble into nothing, and both of those are
/// in this number while neither is in a failure count.
///
/// `Cell::is_empty()` is managed-aware -- a promoted liquid body's
/// container cells are materially empty and still read as not-empty -- so
/// this asks the material directly.
/// How much **roofed void** the world holds: empty cells with rock
/// somewhere above them in the same column.
///
/// This is the "is it still a cave" number, and it exists because nothing
/// measured that. `rock destroyed` cannot: a 2-cell roof and an 8-cell
/// roof both come down completely, but the thin one contains less rock, so
/// the *worse* outcome read as the smaller number. It conflates how badly
/// something failed with how much material happened to be in it.
///
/// A cave dies in exactly two ways and this catches both. Fill it in and
/// the empty cells stop being empty; drop its roof and they stop having
/// rock above them, becoming ordinary sky. Either way the count falls.
///
/// The column test is deliberately "any solid above", not "solid directly
/// overhead within N": a tunnel under a mountain and a tunnel under a
/// metre of crust are both caves, and picking a depth threshold would be
/// choosing which one counts.
fn roofed_void(world: &World) -> i64 {
    let mut n = 0i64;
    for x in 0..WIDTH {
        let mut roofed = false;
        for y in 0..HEIGHT {
            let m = world.get(x, y).material;
            if world.materials.kind(m) == MaterialKind::Solid {
                roofed = true;
            } else if roofed && m == material::EMPTY {
                n += 1;
            }
        }
    }
    n
}

/// Returned split into `(solid, powder)` because the two answer different
/// questions and the split is the whole reading of a destruction run.
/// Rock turning to rubble is **damage** and moves nothing out of the world;
/// rubble disappearing is **removal**. A single total cannot tell a slab
/// that shattered in place from one that was carried off, and those are
/// exactly the two things the confinement rule exists to separate.
fn census(world: &World) -> (i64, i64) {
    let (mut solid, mut powder) = (0i64, 0i64);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            match world.materials.kind(world.get(x, y).material) {
                MaterialKind::Solid => solid += 1,
                MaterialKind::Powder => powder += 1,
                _ => {}
            }
        }
    }
    (solid, powder)
}

fn occupied(world: &World) -> i64 {
    let (solid, powder) = census(world);
    solid + powder
}

/// Cells with either crack bit set (`Cell::cracked`, the OR of
/// `crack_right`/`crack_down`) within a `3 * radius` box centred on a blast
/// site -- the census R1's report line needs `explosion.rs`'s own R5 doc:
/// "did the crack halo actually fire" is a counter question, the same way
/// "did the chunk-body mechanism fire" turned out to be earlier in this
/// project's history (`CLAUDE.md`, "did it fire at all" needs a counter).
/// `3x radius` rather than the crack halo's own `length` so the box stays
/// meaningful across a sweep of `crack_reach` without having to be
/// recomputed by hand each time.
/// The hottest cell in the same box, and how many cells in it are still
/// above ambient at all -- `(hottest, lit)`.
///
/// Companion to `cracked_census`, and it exists for the same "a counter,
/// not a picture" reason: whether a blast's glow is *going away* is a
/// trajectory, and one still frame of an orange ring looks identical
/// whether it is cooling or frozen forever.
fn heat_census(world: &World, cx: i32, cy: i32, radius: i32) -> (i16, u32) {
    let box_r = radius * 3;
    let (mut hottest, mut lit) = (pixel_physics::sim::cell::AMBIENT_TEMPERATURE, 0u32);
    for y in (cy - box_r)..=(cy + box_r) {
        for x in (cx - box_r)..=(cx + box_r) {
            let t = world.get(x, y).temperature();
            hottest = hottest.max(t);
            if t > pixel_physics::sim::cell::AMBIENT_TEMPERATURE {
                lit += 1;
            }
        }
    }
    (hottest, lit)
}

/// Every `Gas` cell standing in the world right now.
///
/// A *standing* count, not a count of dissipation events, and that is the
/// point: what the owner sees is a grey cap that is still there, and
/// `CLAUDE.md`'s own rule is that a complaint about something visible and
/// persistent is answered by the standing state rather than the event rate
/// (the film hunt learned that the expensive way). It is also the "did it
/// fire at all" counter for `Material::dissipation` — smoke thinning and
/// smoke drifting off the top of the crop look identical on a contact
/// sheet, and only a number distinguishes them.
///
/// Whole world rather than boxed around the blast, because smoke *leaves*:
/// a box would show the plume clearing when it had only walked out of the
/// box, which is exactly the wrong reading.
fn smoke_census(world: &World) -> u32 {
    let mut n = 0u32;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if world.materials.kind(world.get(x, y).material) == MaterialKind::Gas {
                n += 1;
            }
        }
    }
    n
}

/// Every cracked cell in the world, whichever blast scored it.
///
/// The companion the boxed census needs for the same reason `smoke_census`
/// is whole-world rather than boxed: a crack star that has grown past
/// `3 * radius` leaves the box, and a box that loses its subject reports
/// the mechanism switching off. With several charges it is also the only
/// figure that is not double-counted where two halos overlap.
fn cracked_world_census(world: &World) -> u32 {
    let mut n = 0u32;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if world.get(x, y).cracked() {
                n += 1;
            }
        }
    }
    n
}

/// **Containment, measured without asking the thing that enforces it.**
///
/// How far from the charge that made it the furthest cell of **rock that
/// stopped being rock** sits, over every charge fired in the run, in cells
/// past that charge's own radius. A cell is attributed to its nearest
/// charge, so two blasts do not blame each other.
///
/// # Why this exists, and what it replaces
///
/// `FailureCounts::max_damage_reach` cannot report a containment failure.
/// It is recorded only at sites downstream of `clip_region_to_licence`, and
/// for any cell that clip retains, `within_disturbance` guarantees some live
/// disturbance within `chain_reach + extent` while
/// `distance_to_live_disturbance` takes the **min** over disturbances of
/// `distance - extent`. So the recorded value is `<= chain_reach` by
/// arithmetic, at every site. A table reading "LOCAL 48, TIGHT 16, NONE 0"
/// against leashes of 48, 16 and 0 is a saturated ceiling, not a
/// measurement, and `CLAUDE.md` names the shape: *a debug readout must not
/// be a function of the thing it debugs.*
///
/// This reads none of that machinery -- not `chain_reach`, not
/// `World::disturbances`, not `within_disturbance`, not `licence_radius`,
/// and not the `chain_window` age test. It compares two material grids and
/// measures a distance. It can therefore return 200 at TIGHT, which is the
/// entire point of having it.
///
/// # Two numbers, and the difference matters
///
/// `(furthest, past_radius)`. The first is measured from the epicentre and
/// includes the crater the charge is *supposed* to make; the second nets
/// that off, and is the one to compare against a leash, since `chain_reach`
/// leashes the chain beyond the wound rather than the wound itself
/// (`structural::Disturbance`). Reported as `-1` when nothing changed at
/// all, never `0`: `CLAUDE.md` asks what a metric says when nothing is
/// wrong, and "perfectly contained" and "nothing happened" must not share a
/// reading.
fn damage_radius(world: &World, before: &[material::MaterialId], fired: &[FiredCharge]) -> (i32, i32) {
    if fired.is_empty() {
        return (-1, -1);
    }
    let (mut furthest, mut past) = (-1i32, -1i32);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let idx = (y * WIDTH + x) as usize;
            let was = before[idx];
            // **Rock that stopped being rock**, and nothing else. The first
            // draft of this counted any changed cell and read 296 on a
            // single charge -- almost all of it smoke drifting to the far
            // edge of the world and debris grains landing. Both are real
            // changes and neither is damage, and `CLAUDE.md` has the rule
            // this broke: the whisker hunt's metric counted every droplet
            // in the world because its definition was what falling water
            // looks like. So: the cell was `Solid` before, and is not the
            // same material now. Deposition is excluded on purpose -- a
            // grain landing on a hillside is the blast's litter, not its
            // reach.
            if world.materials.kind(was) != MaterialKind::Solid || world.get(x, y).material == was {
                continue;
            }
            // Nearest charge, so a cell that two blasts could both claim is
            // charged to the one it is actually near.
            let mut best = i32::MAX;
            let mut best_past = i32::MAX;
            for c in fired {
                let d = (x - c.x).abs().max((y - c.y).abs());
                if d < best {
                    best = d;
                    best_past = d - c.radius;
                }
            }
            furthest = furthest.max(best);
            past = past.max(best_past);
        }
    }
    (furthest, past)
}

/// **The owner's first complaint, as a number.**
///
/// > *"Chunks of rock that seem fully cracked all the way around stay put
/// > too often and don't fall into the leftover hole/crater."*
///
/// An **island** here is a maximal 4-connected component of body material
/// that the fissures have cut completely free -- every step out of it is
/// either a cracked edge into more rock, or the edge of the world. If any
/// step reaches air, gas or loose material, the piece has somewhere to go
/// and is not what he is reporting; it is excluded.
///
/// Returned as `(pieces, islands, cells, largest)` -- `pieces` is every
/// component the fissures genuinely cut loose from the massif, and `islands`
/// is the subset of those with no free face anywhere. The pair is the whole
/// point: if `pieces` is near zero the web on screen is not cutting anything
/// at all, and no amount of work on what a *severed* piece is allowed to do
/// will change what the player sees.
///
/// All are printed, never one:
/// a run with many one-cell chips stuck in the massif and one with a single
/// 200-cell slab hanging in a wall are different bugs that any single number
/// reads as the same, and `MIN_BODY_CELLS` is 8 -- an island under that was
/// never going to fly whatever the support model said.
///
/// **Why this is a counter and not a picture.** A contact sheet shows a web
/// of dark polygon outlines whether those polygons are about to come apart
/// or have been welded in place for two thousand frames, and at the zoom a
/// sheet is read at the two are indistinguishable. `CLAUDE.md`: *"did it fire
/// at all" needs a counter, not a picture.*
///
/// 4-connected, matching `load::is_supported` and `rigid::take_fragment` --
/// the two consumers that decide whether this piece is held and how it comes
/// apart. Asking in 8 would call a piece free that neither of them can
/// actually separate.
fn severed_islands(world: &World) -> (usize, usize, usize, usize) {
    use std::collections::VecDeque;
    let idx = |x: i32, y: i32| y as usize * WIDTH as usize + x as usize;
    let mut seen = vec![false; (WIDTH * HEIGHT) as usize];
    let solid = |x: i32, y: i32| {
        world.in_bounds(x, y) && matches!(world.materials.kind(world.get(x, y).material), MaterialKind::Solid)
    };
    // A component bigger than this is the massif, not a chunk. Bounding the
    // *walk* rather than declining to answer: the walk is abandoned and the
    // cells stay marked, so the hillside is paid for once instead of once
    // per cell in it.
    const MAX_ISLAND: usize = 512;
    let (mut pieces, mut islands, mut cells, mut largest) = (0usize, 0usize, 0usize, 0usize);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if seen[idx(x, y)] || !solid(x, y) {
                continue;
            }
            let mut queue = VecDeque::from([(x, y)]);
            seen[idx(x, y)] = true;
            let mut member = Vec::new();
            let mut enclosed = true;
            let mut overflowed = false;
            while let Some((cx, cy)) = queue.pop_front() {
                member.push((cx, cy));
                if member.len() > MAX_ISLAND {
                    overflowed = true;
                    break;
                }
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if pixel_physics::sim::structural::edge_is_cracked(world, cx, cy, dx, dy) {
                        continue; // a joint: this is where the island ends
                    }
                    if !world.in_bounds(nx, ny) {
                        continue; // the world's edge holds it like rock does
                    }
                    if !solid(nx, ny) {
                        // Air, gas, powder or plant across an *uncracked*
                        // edge: this piece has an open side, so whatever is
                        // keeping it up, it is not being wedged.
                        enclosed = false;
                        continue;
                    }
                    if !seen[idx(nx, ny)] {
                        seen[idx(nx, ny)] = true;
                        queue.push_back((nx, ny));
                    }
                }
            }
            if overflowed {
                // Drain the rest so the massif is marked and not re-walked.
                while let Some((cx, cy)) = queue.pop_front() {
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let (nx, ny) = (cx + dx, cy + dy);
                        if pixel_physics::sim::structural::edge_is_cracked(world, cx, cy, dx, dy) || !solid(nx, ny) || seen[idx(nx, ny)] {
                            continue;
                        }
                        seen[idx(nx, ny)] = true;
                        queue.push_back((nx, ny));
                    }
                }
                continue;
            }
            pieces += 1;
            largest = largest.max(member.len());
            if enclosed {
                islands += 1;
                cells += member.len();
            }
        }
    }
    (pieces, islands, cells, largest)
}

fn cracked_census(world: &World, cx: i32, cy: i32, radius: i32) -> u32 {
    let box_r = radius * 3;
    let mut n = 0u32;
    for y in (cy - box_r)..=(cy + box_r) {
        for x in (cx - box_r)..=(cx + box_r) {
            if world.get(x, y).cracked() {
                n += 1;
            }
        }
    }
    n
}
