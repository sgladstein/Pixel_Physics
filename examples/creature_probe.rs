//! Prints what a creature is actually sensing and deciding, per tick.
//!
//! **The companion every debug channel needs.** `render.rs`'s overlays and
//! `filmstrip`'s `channel=pheromone_a|pheromone_b` answer *what and where*;
//! they cannot answer *how much*, and this project has already paid twice
//! for reaching at an image with a quantitative question. A corrected
//! overlay was misread as "everything at the ramp floor" when the real
//! value was 40% of scale, because a one-cell-wide signal is genuinely
//! unjudgeable by eye (`CLAUDE.md`). An ant is one or two cells.
//!
//! So: when the question turns numeric — *was input 2 actually 0.03? is
//! the turn output doing anything? how many synapses is this brain paying
//! for?* — this is the tool, and `examples/plant_probe.rs` is the
//! precedent it follows.
//!
//! ```text
//! cargo run --release --example creature_probe
//! cargo run --release --example creature_probe -- frames=4000 every=500 ants=8
//! ```

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material;
use pixel_physics::sim::pheromone::Channel;
use pixel_physics::sim::{parallel, Cell, World};

// **The names come from `brain.rs` rather than being restated here.** This
// file used to carry its own abbreviated copy, which is one enum edit away
// from a probe that prints the wrong label against the right number -- the
// exact failure mode `CLAUDE.md` records for debug readouts, and harder to
// spot than a blank overlay because it looks like data.
use pixel_physics::sim::brain::{INPUT_NAMES, OUTPUT_NAMES};

fn main() {
    let mut frames = 6000usize;
    let mut every = 1000usize;
    let mut ants = 6usize;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "frames" => frames = v.parse().expect("frames"),
            "every" => every = v.parse().expect("every"),
            "ants" => ants = v.parse().expect("ants"),
            other => panic!("unknown arg {other:?}; known: frames, every, ants"),
        }
    }

    let (w, h) = (320i32, 120i32);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    let floor = h - 8;
    let nest = world.materials.id_of("nest").expect("nest is compiled in");
    let corpse = world.materials.id_of("corpse").expect("corpse is compiled in");
    let ant = world.materials.id_of("ant").expect("ant is compiled in");

    for x in 0..w {
        for y in floor..h {
            world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
    for x in 16..48 {
        world.set(x, floor, Cell::new(nest, 0).with_attached(true));
    }
    for x in 16..120 {
        world.set(x, floor, Cell::new(nest, 0).with_attached(true));
    }
    for x in 135..170 {
        for y in (floor - 5)..floor {
            world.set(x, y, Cell::new(corpse, 0));
        }
    }
    for i in 0..55 {
        world.plant_ant(20 + i as i32 * 2, floor - 1);
    }

    println!("creature probe: {frames} frames, reporting {ants} ants every {every}");
    for frame in 0..frames {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
        if frame % every == 0 {
            report(&world, frame, ants, ant, w, h);
        }
    }
    report(&world, frames, ants, ant, w, h);

    let st = world.creature_stats;
    println!(
        "\ntotals: moves {} blocked {} falls {} | eats {} pickups {} digs {} drops {} deliveries {} nest-visits {} deaths {}",
        st.moves, st.moves_blocked, st.falls, st.eats, st.pickups, st.digs, st.drops, st.deliveries, st.nest_visits, st.deaths
    );
    println!(
        "energy census: live {:.2} vs ledger {:.2} (delta {:.4})",
        world.live_creature_energy(),
        world.energy_ledger.expected_live_total(),
        world.live_creature_energy() - world.energy_ledger.expected_live_total()
    );
}

fn report(world: &World, frame: usize, ants: usize, ant_material: material::MaterialId, w: i32, h: i32) {
    println!("\n--- frame {frame} ---");
    // Head cells, found by scanning: the probe deliberately does not hold
    // handles across frames, because an ant that dies and has its slot
    // reused would otherwise be silently reported as the same individual.
    let heads: Vec<(i32, i32)> = (0..w)
        .flat_map(|x| (0..h).map(move |y| (x, y)))
        .filter(|&(x, y)| {
            let c = world.get(x, y);
            c.material == ant_material
                && pixel_physics::sim::organism::cell_type(c.aux()) == Some(pixel_physics::sim::organism::CellType::Head)
        })
        .collect();
    // **Carriers first.** The interesting ant is almost never a random one:
    // a colony with pickups and no deliveries has one question, and it is
    // "what does a laden ant think it should do".
    let mut heads = heads;
    heads.sort_by_key(|&(x, y)| {
        let carrying = world.organism(world.get(x, y).organism_id()).is_some_and(|s| s.carrying.is_some());
        (!carrying, x)
    });
    let heads: Vec<(i32, i32)> = heads.into_iter().take(ants).collect();

    for (x, y) in heads {
        let organism = world.get(x, y).organism_id();
        let Some(state) = world.organism(organism) else { continue };
        let Some(def) = world.species.get(state.species).creature.as_ref() else { continue };
        let (inputs, outputs, active) = pixel_physics::sim::creature::probe(world, x, y, organism, def);
        println!(
            "  ant {organism:>5} at ({x:>3},{y:>3}) heading {} energy {:>7.1} carrying {} since_nest {:>4} | {} active synapses",
            state.heading,
            state.energy,
            state.carrying.map_or("-", |_| "yes"),
            state.since_nest,
            active
        );
        let ins: Vec<String> = INPUT_NAMES.iter().zip(&inputs).map(|(n, v)| format!("{n} {v:+.3}")).collect();
        let outs: Vec<String> = OUTPUT_NAMES.iter().zip(&outputs).map(|(n, v)| format!("{n} {v:+.3}")).collect();
        println!("    in : {}", ins.join("  "));
        println!("    out: {}", outs.join("  "));
        println!(
            "    planes here: A {} B {}",
            world.pheromone_at(Channel::A, x, y),
            world.pheromone_at(Channel::B, x, y)
        );
        // **The number beside the picture.** `OrganismOverlay::FoodValue`
        // says what and where; on a one-cell mouthful it cannot say how
        // much, and the overlay-misread-as-empty failure has cost this
        // project a misdiagnosis twice. Anything nonzero here is food, and
        // a corpse reads its own stamped worth rather than a species
        // constant.
        let food: Vec<String> = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)]
            .iter()
            .map(|&(dx, dy)| (x + dx, y + dy))
            .map(|(nx, ny)| (world.materials.get(world.get(nx, ny).material).name.clone(), pixel_physics::sim::creature::food_value(world, world.get(nx, ny))))
            .filter(|(_, v)| *v > 0.0)
            .map(|(name, v)| format!("{name} {v:.0}"))
            .collect();
        println!("    food in reach: {}", if food.is_empty() { "none".to_string() } else { food.join("  ") });
    }
}
