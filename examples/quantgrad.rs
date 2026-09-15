//! **Does a decaying per-cell scalar still read as a gradient?**
//!
//! Built to test whether the root cause found on the pheromone line
//! (`Reports/pheromone-lifetime-and-wiring-2026-09-14.md` §3c) generalises.
//! The shape: a scalar that both **decays** and is read as a **gradient**.
//! Decay needs headroom under it or it hits a fixed point; a gradient needs
//! resolution *between neighbours* or it reads flat. Narrow storage takes
//! the gradient away first, silently, with the code correct and every gate
//! green.
//!
//! **This measures the number the CONSUMER computes, never the stored
//! value.** That distinction is the whole method: the pheromone trail's own
//! numbers looked healthy (peak 39-98 of 255, hundreds of cells standing)
//! the entire time the ant's own along-reading was `+0.000` past the trail's
//! midpoint. Same data, two readers, one of them silent.
//!
//! **Exactly zero is the signature** — a weak-but-working mechanism reads
//! 0.003, an exhausted representation reads 0.000 and keeps reading it. But
//! there are *two* causes of an exact zero and only one of them is this
//! defect: a **degenerate condition** (the rule keyed on something that
//! cannot vary) and an **exhausted representation**. This harness rules out
//! the first the cheap way, by printing the consumer's own decision
//! threshold next to the storage quantum: **where a designed threshold sits
//! far above the quantum, widening the storage cannot move a single
//! decision**, and there is nothing to fix however flat the channel reads.
//!
//! ```text
//! cargo build --release --examples          # ALWAYS; assets are include_str!'d
//! cargo run --release --example quantgrad
//! cargo run --release --example quantgrad -- frames=400
//! ```

use pixel_physics::sim::cell::{Cell, AMBIENT_TEMPERATURE};
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material;
use pixel_physics::sim::parallel;
use pixel_physics::sim::world::World;

fn arg(key: &str) -> Option<String> {
    std::env::args().find_map(|a| a.strip_prefix(key).map(str::to_string))
}

/// `fire::diffuse_heat`'s arithmetic with the minimum-progress nudge
/// **removed** — the naive arm, i.e. what the channel would do if it were
/// defended only by being correct.
///
/// Kept as a literal transcription rather than a call, because the shipped
/// function is private and, more to the point, the two arms have to differ
/// by exactly one clause for the comparison to mean anything.
fn naive_step(here: f32, neighbour_avg: f32, conductivity: f32) -> f32 {
    (here + (neighbour_avg - here) * conductivity).round()
}

/// ...and the shipped arm, nudge included.
fn shipped_step(here: f32, neighbour_avg: f32, conductivity: f32) -> f32 {
    let new_temp = here + (neighbour_avg - here) * conductivity;
    let raw_delta = new_temp - here;
    let rounded = new_temp.round();
    let already_settled = (here - AMBIENT_TEMPERATURE as f32).abs() <= 1.0;
    if rounded == here && raw_delta.abs() > 0.01 && !already_settled {
        here + raw_delta.signum()
    } else {
        rounded
    }
}

fn main() {
    let frames: usize = arg("frames=").and_then(|v| v.parse().ok()).unwrap_or(200);
    println!("quantgrad: frames={frames}   (echoing parameters, per CLAUDE.md)");
    println!();

    // ---------------------------------------------------------------
    // 1. Cell::temperature -- i16, quantum 1 degree C.
    //    Decays (toward the neighbourhood average) AND is read as a
    //    gradient (by diffuse_heat itself). The narrowest decaying
    //    gradient channel in the engine.
    // ---------------------------------------------------------------
    println!("== Cell::temperature (i16, quantum 1 C) ==");
    println!("consumer: fire::diffuse_heat, `here + (neighbour_avg - here) * conductivity`");
    println!("threshold: NONE -- it reads the raw difference, exactly as the ant read the raw trail difference");
    println!();
    println!("  the dead zone, per conductivity: the largest neighbour difference that still moves NOTHING");
    println!("  {:>12}  {:>14}  {:>16}", "conductivity", "naive dead zone", "shipped dead zone");
    for c in [0.02f32, 0.05, 0.08, 0.1, 0.15, 0.25] {
        let dead = |f: fn(f32, f32, f32) -> f32| {
            let here = AMBIENT_TEMPERATURE as f32 + 40.0; // well outside the settle epsilon
            let mut widest = 0.0f32;
            let mut d = 0.5f32;
            while d <= 60.0 {
                if f(here, here + d, c) == here {
                    widest = d;
                }
                d += 0.5;
            }
            widest
        };
        println!("  {:>12.2}  {:>13.1}C  {:>15.1}C", c, dead(naive_step), dead(shipped_step));
    }
    println!();

    // The positive control the bar demands: a shallow gradient, run to
    // rest, in BOTH arms. A representation that has run out reads exactly
    // zero and keeps reading it.
    println!("  a 4 C gradient at conductivity 0.10, {frames} steps -- does the heat actually move?");
    for (name, f) in [("naive (no nudge)", naive_step as fn(f32, f32, f32) -> f32), ("shipped (nudge)", shipped_step)] {
        let mut here = AMBIENT_TEMPERATURE as f32 + 4.0;
        let cold = AMBIENT_TEMPERATURE as f32;
        let start = here;
        for _ in 0..frames {
            here = f(here, cold, 0.10);
        }
        println!("    {name:<18}  {start:.0}C -> {here:.0}C   (moved {:+.0}C)", here - start);
    }
    println!();

    // ...and the same claim inside the real engine, because an arithmetic
    // transcription is not the sweep. `CLAUDE.md`: verify live.
    {
        let mut w = World::new(Rect::new(0, 0, 63, 63));
        let ash = w.materials.id_of("ash").expect("ash exists and conducts");
        for x in 20..44 {
            w.set(x, 32, Cell::new(ash, 0).with_temperature(AMBIENT_TEMPERATURE + 4));
        }
        let before: i32 = (20..44).map(|x| w.get(x, 32).temperature() as i32).sum();
        for _ in 0..frames {
            parallel::step(&mut w);
        }
        let after: i32 = (20..44).map(|x| w.get(x, 32).temperature() as i32).sum();
        let ambient_sum = 24 * AMBIENT_TEMPERATURE as i32;
        println!("  real engine, 24 ash cells at ambient+4, {frames} frames of parallel::step:");
        println!("    heat above ambient: {} -> {}  ({} of it shed)", before - ambient_sum, after - ambient_sum, before - after);
    }
    println!();


    // ---------------------------------------------------------------
    // 1b. The half the nudge CANNOT defend, and the reason this is not
    //     just a restatement of 1.
    //
    //     `already_settled` is measured against AMBIENT_TEMPERATURE, not
    //     against the cell's own local equilibrium. So the nudge is
    //     switched off exactly where a cell sits at ambient -- which is
    //     the *receiving* end of every shallow gradient. It rescues a hot
    //     cell cooling DOWN to ambient and is structurally incapable of
    //     rescuing an at-ambient cell warming UP.
    //
    //     Deliberate, and its doc says why: without the gate a connected
    //     mass of cooling cells nudges itself awake forever and no chunk
    //     ever sleeps. This prints what that gate costs, so the trade is
    //     visible rather than inferred.
    // ---------------------------------------------------------------
    println!("  the direction the nudge cannot reach: a cell AT ambient, warm neighbour, {frames} steps");
    println!("  {:>10}  {:>6}  {:>18}", "neighbour", "cond", "warmed by");
    for navg in [AMBIENT_TEMPERATURE as f32 + 1.0, AMBIENT_TEMPERATURE as f32 + 4.0, AMBIENT_TEMPERATURE as f32 + 10.0] {
        for c in [0.02f32, 0.10] {
            let mut here = AMBIENT_TEMPERATURE as f32;
            for _ in 0..frames {
                here = shipped_step(here, navg, c);
            }
            println!("  {:>9.0}C  {:>6.2}  {:>17.0}C", navg, c, here - AMBIENT_TEMPERATURE as f32);
        }
    }
    println!();

    // ...and live, in the sweep, which is the claim that actually counts.
    {
        let mut w = World::new(Rect::new(0, 0, 63, 63));
        let ash = w.materials.id_of("ash").expect("ash exists and conducts");
        // A warm block held against a cold bar of the same conducting
        // material. The bar starts at exact ambient -- the gated case.
        for x in 20..44 {
            w.set(x, 32, Cell::new(ash, 0).with_temperature(AMBIENT_TEMPERATURE));
        }
        for x in 16..20 {
            w.set(x, 32, Cell::new(ash, 0).with_temperature(AMBIENT_TEMPERATURE + 4));
        }
        for _ in 0..frames {
            parallel::step(&mut w);
        }
        let warmed = (20..44).filter(|&x| w.get(x, 32).temperature() > AMBIENT_TEMPERATURE).count();
        println!("  real engine, 24 ash cells at EXACT ambient beside a +4C block, {frames} frames:");
        println!("    cells that warmed at all: {warmed} of 24");
    }
    println!();
    // ---------------------------------------------------------------
    // 2. soil water in Cell::aux -- u16 on SOIL_SATURATED's 0..1000
    //    scale, so the quantum is 1/1000 of full. Decays (evaporation,
    //    drainage, root uptake) AND is read as a gradient, by
    //    organism::moisture_pull, which a RootTip steers on.
    // ---------------------------------------------------------------
    println!("== soil water, Cell::aux (u16, 0..{}, quantum {:.4} of scale) ==", material::SOIL_SATURATED, 1.0 / material::SOIL_SATURATED as f32);
    println!("consumer: organism::moisture_pull -> plant::MIZ_THRESHOLD");
    println!(
        "  quantum {:.4}   consumer threshold {:.4}   ratio {:.0}x",
        1.0 / material::SOIL_SATURATED as f32,
        pixel_physics::sim::plant::MIZ_THRESHOLD,
        pixel_physics::sim::plant::MIZ_THRESHOLD * material::SOIL_SATURATED as f32
    );
    println!("  capillary rest threshold is 60 units = 60x the quantum; flow stops there, not at the quantum.");
    println!("  -> the consumer's own decision boundary sits {:.0} quanta above the storage floor.", pixel_physics::sim::plant::MIZ_THRESHOLD * material::SOIL_SATURATED as f32);
    println!("  -> measured distribution is in `plant_probe`'s hydrotropism block, not here (it needs grown roots).");
    println!();

    // ---------------------------------------------------------------
    // 3. liquid fill in Cell::aux -- u16 on LIQUID_FULL's 0..1000 scale.
    //    Decays (evaporation) AND is read as a gradient (levelling reads
    //    the neighbour fill difference).
    // ---------------------------------------------------------------
    println!("== liquid fill, Cell::aux (u16, 0..{}, quantum {:.4} of scale) ==", material::LIQUID_FULL, 1.0 / material::LIQUID_FULL as f32);
    println!("consumer: update.rs levelling -> Material::min_transfer");
    let water_min = {
        let w = World::new(Rect::new(0, 0, 15, 15));
        w.materials.id_of("water").map(|id| w.materials.get(id).min_transfer).unwrap_or(16)
    };
    println!(
        "  quantum 1 unit   consumer threshold {water_min} units   ratio {water_min}x",
    );
    println!("  -> a transfer smaller than {water_min} units is refused by design, {water_min}x before the quantum can bind.");
}
