//! **How far does the vein model actually carry carbon through a grown
//! tree?** The companion to `plant_severance`, which asks whether the plant
//! *economy* notices its own plumbing; this asks what the plumbing can do
//! on its own.
//!
//! `plant::allocate_to_frontier` pools every non-frontier cell's carbon and
//! writes each growing tip its share directly, so in ordinary running the
//! conductance model's reach is never the thing that funds a tip and cannot
//! be read off a stand. Here nothing runs but `organism::transport`:
//! whatever appears further up the tree arrived through the conductance
//! model, because there is nothing else in the world to put it there.
//!
//! ```text
//! cargo run --release --example plant_reach -- cut=24000 ticks=40
//! ```

mod common;

use pixel_physics::sim::organism;
use pixel_physics::sim::parallel;
use pixel_physics::sim::world::World;

/// One frame of the whole world -- all three calls, for the reason
/// `plant_severance::step` records: the sweep alone germinates nothing.
fn step(w: &mut World) {
    parallel::step(w);
    w.step_active_sites();
    w.step_fields();
}

fn arg<T: std::str::FromStr>(name: &str, default: T) -> T
where
    T::Err: std::fmt::Debug,
{
    std::env::args()
        .find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.parse().expect(name)))
        .unwrap_or(default)
}

fn main() {
    let cut: u64 = arg("cut", 24_000);
    let seeds: u64 = arg("seeds", 2);
    let trees: usize = arg("trees", 4);
    let species: String = arg("species", "tree".to_string());
    let ticks: usize = arg("ticks", 40);
    let band: i32 = arg("band", 4);
    println!("plant_reach: species={species} trees={trees} seeds={seeds} cut={cut} ticks={ticks} band={band}");

    let first = if seeds == 0 { 0 } else { 1 };
    for seed in first..=seeds {
        let scene = common::PlantScene {
            trees,
            species: species.clone(),
            seed: if seed == 0 { None } else { Some(seed) },
            ..Default::default()
        };
        let mut w = scene.build();
        while w.frame < cut {
            step(&mut w);
        }
        println!("\nseed={seed}");
        // **Both arms off one warm-up, and the finite one first.** `hold`
        // overwrites the band every tick, so running it first would leave
        // the plant's carbon field in a state the finite arm did not
        // start from -- and the finite arm re-seeds from zero anyway, so
        // this order needs no reset between them.
        reach(&mut w, ticks, band, false);
        reach(&mut w, ticks, band, true);
    }
}

// --- how far the vein model actually carries carbon --------------------

/// **How far does one organism tick of `organism::transport` move carbon
/// through a grown tree?**
///
/// The second half of the question `examples/plant_severance` asks. Its
/// `sever` and `deroot` arms ask whether the *economy* notices the plumbing;
/// this asks what the plumbing can do on its own, with the pooled allocation
/// that normally masks it switched off — not by a knob, but by running
/// nothing except `transport`.
///
/// Method: zero every cell of the plant, refill the deepest `band` rows of
/// root to `RESOURCE_SCALE`, then call `organism::transport` and nothing
/// else. Whatever appears higher up arrived through the conductance model,
/// because nothing else in the world is running to put it there.
///
/// **Two fronts, not one.** A *trace* front (any carbon at all) says how far
/// the numerics reach; a *usable* front (a tenth of the scale, well under
/// `tree.ron`'s `cost: 0.2` for one growth step) says how far a tip could
/// actually be funded from the roots. Reporting only the first would call a
/// front that cannot pay for anything a delivery.
///
/// Total carbon is printed every stop as the **instrument's own control**,
/// and what it should be doing differs by arm. In the finite arm `transport`
/// conserves carbon except at the `RESOURCE_SCALE` clamp, where a cell
/// already at the cap receiving more has the excess clipped — so the total
/// falls, and a total that *rose* there would be a bug in the probe rather
/// than a finding about trees. In the held arm the refill is a source, so the
/// total must **climb**: a held run whose total went flat or fell would mean
/// the refill was not reaching the band, and the stalled front it reported
/// would be the probe's own fault rather than the model's.
///
/// **`hold` is the control the finite-source arm needs, and without it the
/// result is arguable.** With a fixed charge the front necessarily stalls
/// once the charge has spread — a real root system does not run dry, it
/// keeps drinking, so `hold` refills the seeded band to `RESOURCE_SCALE`
/// after every tick. Run both: the finite arm says how far one tankful
/// travels, the held arm says how far a *maintained* source pushes a front,
/// which is the question about a living tree.
fn reach(w: &mut World, ticks: usize, band: i32, hold: bool) {
    let mut biggest: Option<(u16, usize)> = None;
    for id in w.live_organism_ids() {
        let Some(st) = w.organism(id) else { continue };
        if biggest.is_none_or(|(_, n)| st.cells.len() > n) {
            biggest = Some((id, st.cells.len()));
        }
    }
    let (id, n) = biggest.expect("plant_reach: no organism in the world at all");
    assert!(n > 100, "plant_reach: the largest plant is {n} cells, which is a seedling -- raise cut=");
    let cells: Vec<(i32, i32)> = {
        let st = w.organism(id).expect("checked");
        let mut v: Vec<(i32, i32)> = st.cells.keys().copied().collect();
        v.sort_unstable();
        v
    };
    let top = cells.iter().map(|&(_, y)| y).min().expect("non-empty");
    let bottom = cells.iter().map(|&(_, y)| y).max().expect("non-empty");

    let mut seeded = 0usize;
    for &(x, y) in &cells {
        if let Some(c) = w.organism_cell_mut(x, y) {
            c.carbon = if y > bottom - band { organism::RESOURCE_SCALE } else { 0.0 };
        }
        if y > bottom - band {
            seeded += 1;
        }
    }
    // The scene check: a band that caught no cell would leave the whole
    // plant at zero, and a front that never moves off zero reads exactly
    // like a transport model that carries nothing.
    assert!(seeded > 0, "plant_reach: the bottom {band} rows caught no cell of the plant");

    let usable = organism::RESOURCE_SCALE * 0.1;
    println!(
        "\nhold={hold}: plant {id}, {n} cells, rows {top}..={bottom} (span {}), \
         seeded {seeded} cells in the bottom {band} rows to {:.1}; the crown top is {} rows above the charge",
        bottom - top,
        organism::RESOURCE_SCALE,
        bottom - band - top
    );
    println!("  ticks  frames  trace front  usable front  advanced  total carbon");
    for t in 0..=ticks {
        if t > 0 {
            organism::transport(w, id);
            if hold {
                for &(x, y) in &cells {
                    if y > bottom - band {
                        if let Some(c) = w.organism_cell_mut(x, y) {
                            c.carbon = organism::RESOURCE_SCALE;
                        }
                    }
                }
            }
        }
        let (mut trace, mut use_front, mut total) = (bottom, bottom, 0.0f32);
        for &(x, y) in &cells {
            let c = w.carbon_at(x, y);
            total += c;
            if c > 1e-3 {
                trace = trace.min(y);
            }
            if c >= usable {
                use_front = use_front.min(y);
            }
        }
        println!(
            "  {t:>5}  {:>6}  {trace:>11}  {use_front:>12}  {:>8}  {total:>12.1}",
            t * 45,
            bottom - band - use_front,
        );
    }
}
