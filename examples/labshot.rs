//! **A look at the lab box.** Renders `lab::scene::LabBox` through the same
//! `Renderer` the game draws with, as a contact sheet across frames, so the
//! bed can be judged by eye before anything is measured in it.
//!
//! `CLAUDE.md`'s first method rule: *look before you measure* — every metric
//! written before anyone had looked at the artifact has measured the wrong
//! thing. A hand-built scene is exactly where that bites, because a scene
//! that contradicts the code looks identical to a bug in the code.
//!
//! **Every stop prints its counts beside the picture**, because a picture
//! cannot say whether the thing you built is what produced it — `CLAUDE.md`'s
//! standing rule, learned when a collapse rendered as coherent falling slabs
//! was read as "chunks are working" while the body count was zero for the
//! whole run. The same trap is live in this bed twice over: a box full of
//! green that is reproducing and one that is not are the same photograph, and
//! **a founder that never germinated and one too small to draw are the same
//! photograph too**. So the per-stop line carries the ant count, the standing
//! fruit, and each founder's cell count by the id it was given before the
//! first tick — a founder whose id no longer resolves is dead, one at three
//! cells is merely invisible, and only the id can tell them apart.
//!
//! ```text
//! cargo run --release --example labshot
//! cargo run --release --example labshot -- frames=0,600,3000,9000 out=lab.png
//! cargo run --release --example labshot -- founders=8 walls=4 frames=0,3000,12000,30000
//! ```

use pixel_physics::lab::scene::LabBox;
use pixel_physics::render::Renderer;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::enclosure::Enclosure;
use pixel_physics::sim::field;
use pixel_physics::sim::organism::{self, CellType};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;

/// Three numbers a contact sheet cannot give, in one pass over the grid.
///
/// - **still a seed** — organisms with a live `Seed` cell and nothing else.
///   This is the half of "five founders of eight are visible" that means
///   *germination failed*; the other half means *too small to see*, and only
///   `biggest` against the stand can tell them apart.
/// - **biggest** — cells in the largest organism, so "everything germinated
///   and stayed at one cell" is distinguishable from "three never started".
/// - **roots reach** — rows below the surface the deepest root cell sits at.
///   §2a's standing obligation: soil costs 1.9x the frame at 240 rows, and a
///   bed whose bottom third is never entered is paying it for nothing.
fn census(world: &World, ground_y: i32) -> (usize, usize, i32) {
    let ids = world.live_organism_ids();
    let mut ungerminated = 0usize;
    let mut biggest = 0usize;
    for id in &ids {
        let Some(state) = world.organism(*id) else { continue };
        biggest = biggest.max(state.cells.len());
        // A founder that has not germinated is still exactly its seed.
        let seed_only = state.cells.len() <= 1
            && state.cells.keys().all(|&(x, y)| {
                organism::cell_type(world.get(x, y).aux()) == Some(CellType::Seed)
            });
        ungerminated += usize::from(seed_only);
    }
    // Material-keyed, not type-keyed: a root matures into `MatureBody` while
    // keeping the species' root material, so cell type alone misses all but
    // the growing ends (`root_contact`'s own note).
    let mut deepest = 0;
    for id in &ids {
        let Some(state) = world.organism(*id) else { continue };
        let root = world
            .materials
            .id_of(&world.species.get(state.species).root_material);
        let Some(root) = root else { continue };
        for &(x, y) in state.cells.keys() {
            if world.get(x, y).material == root {
                deepest = deepest.max(y - ground_y + 1);
            }
        }
    }
    (ungerminated, biggest, deepest)
}

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

fn main() {
    let out: String = arg("out").unwrap_or_else(|| "labshot.png".to_string());
    let stops: String = arg("frames").unwrap_or_else(|| "0,600,3000,9000".to_string());
    let stops: Vec<u64> = stops.split(',').map(|s| s.parse().expect("a frame number")).collect();
    let zoom: i32 = arg("zoom").unwrap_or(1);

    // **The before/after arm, and it is one binary.** `CLAUDE.md` asks for a
    // paired comparison rather than one run against a remembered impression,
    // and the stale-example gotcha says the surest way to compare two
    // renderers is not to build two. `interior=0` simply declines to declare
    // the box an enclosure, so the identical world draws through the sky
    // path — same cells, same frame, same binary, one branch different.
    let interior: i32 = arg("interior").unwrap_or(1);
    let spec = LabBox {
        width: arg("width").unwrap_or(512),
        height: arg("height").unwrap_or(320),
        soil_depth: arg("soil").unwrap_or(LabBox::default().soil_depth),
        founders: arg("founders").unwrap_or(8),
        colonies: arg("colonies").unwrap_or(1),
        // **Which animal, because the answer is now visibly different.**
        // `ancestor` (`assets/species/ancestor.ron`) declares no home
        // material, so `found_colony_of` paints no nest patch for it -- and
        // the whole question `Reports/creature-genome-flexibility-2026-09-02.md`
        // asks is what a colony looks like when the scene did not paint the
        // precondition for one. That is a judge-by-eye question and this is
        // the harness that answers it.
        colony_species: arg::<String>("species").unwrap_or_else(|| LabBox::default().colony_species),
        predators: arg("predators").unwrap_or(0),
        compartments: arg("walls").unwrap_or(1),
        ..LabBox::default()
    };
    println!(
        "labshot: {}x{} soil={} founders={} colonies={} of {} predators={} walls={} interior={interior} light={} frames={:?}",
        spec.width, spec.height, spec.soil_depth, spec.founders, spec.colonies, spec.colony_species, spec.predators,
        spec.compartments,
        arg::<f32>("light").map_or("held at noon".to_string(), |f| format!("{f}")),
        stops
    );

    let (mut world, placed) = spec.build_counted();
    if interior == 0 {
        world.set_enclosure(None);
    }
    // **The light schedule, as a knob, because it is the game's largest
    // lever** — 2.4x reproduction at full amplitude against a day/night
    // cycle (design guide §2). `LabBox` holds it at the measured-brightest
    // frame; `light=` moves it anywhere on the day's curve, so "the lights
    // are on" and "the lights are off" are two runs of one binary rather
    // than an assertion. `frame_for_daylight` is the inverse of the same
    // cosine the field reads, so the picture and the plants agree.
    if let Some(fraction) = arg::<f32>("light") {
        world.set_sky_hold(Some(pixel_physics::sky::frame_for_daylight(fraction)));
    }
    // **The fixtures, off** — and since 2026-08-30 this arm answers a
    // different question, which is worth stating rather than leaving to be
    // rediscovered. It used to be "do the grow lights help or hurt", and the
    // answer was *neither*: the fixtures were `crystal`, whose glow never
    // reached the bench, so pulling them left the stand byte-identical and
    // the crop lived on sky light through the shell.
    //
    // The box is sunless now and the fixtures are `growlamp`, so this is the
    // direct inversion: **a bed with no lights, which is a dark bed** (0.000
    // at the bench, nothing sets seed). That makes it the positive control
    // for "the lamps are what light the crop" rather than an A/B of their
    // value. **It is not the old world** — for that arm, which needs the sun
    // switched back on as well, use `lamp_probe mode=cost`, whose `roof` arm
    // is exactly it.
    //
    // Done from the example rather than as a `LabBox` knob because a scene
    // the harness can turn off is still the game's scene, where a second
    // builder would not be.
    // `loadfail=0` is the lab's own COLLAPSE UNDER LOAD row, reachable
    // headlessly -- it is a setting rather than an ablation, so unlike
    // `BEND`/`BREAK` it has no `env::var` and nothing could measure it.
    if let Some(v) = arg::<i32>("loadfail") {
        world.plant_load_failure = v != 0;
    }
    if arg::<i32>("lamps") == Some(0) {
        for cx in spec.lamps_in(&world) {
            spec.remove_lamp(&mut world, cx);
        }
        world.set_enclosure(Some(Enclosure::new(spec.room_top(), spec.ground_y)));
    }
    // **The mechanic, as one binary and one branch.** `movelamp=from,to`
    // drags the fixture whose bar is centred at `from` so it sits at `to` and
    // changes nothing else, so "the same bed with the light somewhere else"
    // is a second run rather than a second scene. `LabBox::move_lamp` is the
    // call the parameters panel will make.
    if let Some(spec_arg) = arg::<String>("movelamp") {
        let (from, to) = spec_arg.split_once(',').expect("movelamp=from,to");
        let (from, to): (i32, i32) = (from.parse().expect("a column"), to.parse().expect("a column"));
        let moved = spec.move_lamp(&mut world, from, to);
        println!("  movelamp {from} -> {to}: {}", if moved { "moved" } else { "REFUSED" });
    }
    // The counter half of "five founders of eight are visible", printed
    // before a single frame runs. A seed that was never planted and a plant
    // too small to see look identical on a contact sheet and mean opposite
    // things.
    println!(
        "  placed: {} of {} founders, {} ants; lamps at {:?}; partitions at {:?}",
        placed.planted,
        placed.asked,
        placed.ants,
        spec.lamp_columns().0,
        spec.partition_columns()
    );
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();

    let (vw, vh) = (spec.width as u32, spec.height as u32);
    let mut renderer = Renderer::new();
    for _ in 1..zoom {
        renderer.adjust_zoom(1);
    }

    // Before a single tick: every organism the scene builder placed. This is
    // the only moment the founders are distinguishable from their offspring.
    let founders = world.live_organism_ids();
    println!("  {} organism(s) placed by the builder before the first tick", founders.len());

    let mut tiles: Vec<Vec<u8>> = Vec::new();
    let last = *stops.last().expect("at least one stop");
    let mut next = 0usize;
    for f in 0..=last {
        if next < stops.len() && stops[next] == f {
            let mut buf = vec![0u8; (vw * vh * 4) as usize];
            let touched = world.take_touched_chunks();
            renderer.draw(&world, &particles, &touched, &mut buf, (vw, vh), true);
            let ids = world.live_organism_ids();
            let cells: usize =
                ids.iter().filter_map(|id| world.organism(*id)).map(|s| s.cells.len()).sum();
            let seeds: u32 =
                ids.iter().filter_map(|id| world.organism(*id)).map(|s| s.seeds_set).sum();
            let (ungerminated, biggest, deepest) = census(&world, spec.ground_y);
            // **The light the crop is actually standing in**, as a fraction
            // of `field::MAX_LIGHT`. A sealed ceiling stops sky light, so
            // how much reaches the bench is a property of the *shell* — its
            // thickness, and whether the fixtures are emitting more than the
            // stone they replaced is blocking. Nothing else in the lane
            // would notice this being wrong: a dim box germinates and then
            // simply grows less, which reads as a species problem.
            let cols = spec.founder_columns();
            let mut lit = 0.0f32;
            let mut dimmest = f32::INFINITY;
            for &x in &cols {
                let v = world.field_at(x, spec.ground_y - 2).light / field::MAX_LIGHT;
                lit += v;
                dimmest = dimmest.min(v);
            }
            let mean = lit / cols.len().max(1) as f32;
            println!(
                "  frame {f:>6}: cells {cells:>6}  orgs {:>5}  seeds {seeds:>5}  \
                 still a seed {ungerminated:>3}  biggest {biggest:>4}  roots reach {deepest:>3} rows  \
                 light at the bench {mean:.3} (dimmest {dimmest:.3})",
                ids.len()
            );
            // **The creature and standing-organ half**, beside the plant one
            // above rather than instead of it. `orgs` counts plants *and*
            // ants together, so a colony dying while the stand grows moves it
            // by zero -- and the fruit columns are counted as **cells standing
            // in the grid** rather than as `seeds_set`, which is a different
            // quantity: seeds set is a plant's own tally, standing fruit is
            // what an ant could walk to.
            // `Reports/evolution-lab-gate-1-2026-08-30.md` §4.3 turns on that
            // difference.
            let (mut plants, mut ants) = (0usize, 0usize);
            for id in &ids {
                let Some(st) = world.organism(*id) else { continue };
                if world.species.get(st.species).creature.is_some() {
                    ants += 1;
                } else {
                    plants += 1;
                }
            }
            let (mut fruit, mut flower) = (0usize, 0usize);
            for y in 0..spec.height {
                for x in 0..spec.width {
                    let c = world.get(x, y);
                    if c.organism_id() == 0 {
                        continue;
                    }
                    match organism::cell_type(c.aux()) {
                        Some(CellType::Fruit) => fruit += 1,
                        Some(CellType::Flower) => flower += 1,
                        _ => {}
                    }
                }
            }
            let stats = world.creature_stats;
            let (alloc, _) = world.organism_slot_usage();
            println!(
                "            plants {plants:>4} ants {ants:>4} births {:>4} deaths {:>4} | \
                 standing fruit {fruit:>4} flower {flower:>4} | slots {alloc:>4}/4095",
                stats.births, stats.deaths,
            );
            // Each founder by the id it held before the first tick. `dead`
            // means the id no longer resolves -- which is a *death*, and is
            // the reading the `still a seed` column above cannot give: a
            // founder that germinated and was then eaten and one that never
            // germinated are the same absence from the picture and opposite
            // findings.
            let founder_line: Vec<String> = founders
                .iter()
                .map(|id| match world.organism(*id) {
                    Some(st) => format!("{}", st.cells.len()),
                    None => "dead".to_string(),
                })
                .collect();
            println!("            founders (cells): {}", founder_line.join(" "));
            // **Why a stand is shrinking, split by the mechanism that did
            // it.** A plant census says the stand got smaller and cannot say
            // which rule took it -- and they want opposite responses. Owner,
            // 2026-09-01: *"I turned COLLAPSE UNDER LOAD off, but trees are
            // still falling over"*, which is exactly the question a pooled
            // count cannot answer. `snapped` and `over span` are the two the
            // switch governs; `severed` folds in limbs that simply lost their
            // anchor, which it deliberately does not.
            let f = world.structural_failures;
            println!(
                "            felling: snapped under load {:>4} | severed by support check {:>4} ({:>3} of them alive) | leaned {:>4} | shed {:>4} | rotted {:>4} | load rule {}",
                f.snapped_under_load,
                f.severed_organism_cells,
                f.severed_living_cells,
                f.bends_applied,
                world.shed_shade + world.shed_drought + world.shed_stranded,
                world.rotted_to_nothing + world.rotted_onward,
                if world.plant_load_failure { "ON" } else { "OFF" },
            );
            tiles.push(buf);
            next += 1;
        }
        if f < last {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        }
    }

    // One column, so a tall thin bed stacks readably.
    let (sw, sh) = (vw, vh * tiles.len() as u32);
    let mut sheet = vec![0u8; (sw * sh * 4) as usize];
    for (i, tile) in tiles.iter().enumerate() {
        let y0 = i as u32 * vh;
        for y in 0..vh {
            let src = (y * vw * 4) as usize;
            let dst = ((y0 + y) * sw * 4) as usize;
            sheet[dst..dst + (vw * 4) as usize].copy_from_slice(&tile[src..src + (vw * 4) as usize]);
        }
    }
    image::save_buffer(&out, &sheet, sw, sh, image::ColorType::Rgba8).expect("writing the sheet");
    println!("wrote {out} ({sw}x{sh})");
}
