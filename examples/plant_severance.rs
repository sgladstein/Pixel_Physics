//! **Does a plant need its roots, and does it notice when the trunk is
//! cut?** Three arms on one bed, same seeds, same frames: `control`,
//! `sever` (a band of the plant's own cells removed just above the soil
//! line, so the shoot is cut free of the root system) and `deroot` (every
//! root cell removed, the shoot left standing).
//!
//! Built for the owner's report that *"many plants can grow just fine with
//! tiny or without any roots at all... I have seen plants continue to grow
//! after the trunk has been fully severed near the base."* Both halves are
//! claims about the **economy**, not about structure, and no instrument here
//! could answer them: `scene=fell` cuts a trunk and censuses what *falls*,
//! `plant_probe` reads a stand that nothing has happened to, and neither
//! reports whether a cut plant goes on earning and building.
//!
//! **The columns exist to separate three explanations that a cell count
//! alone cannot tell apart**, which is the whole reason this is not one
//! number:
//!
//! - `cells` moving after the cut is the claim itself — is it still
//!   building?
//! - `unreached` is the **positive control on the cut**: `anchor_support`
//!   writes `u16::MAX` into any cell with no path to an anchor, so a sever
//!   that leaves this at zero did not sever anything and every number
//!   beside it is measuring an intact plant. A cut that reads as a cut is
//!   the precondition for reading anything else here.
//! - `water`/`status`/`demand` are the balance the shoot is supposedly cut
//!   off from. `water_status` multiplies every photosynthetic credit, so it
//!   is the one number that says whether losing the roots cost the plant
//!   anything at all.
//!
//! ```text
//! cargo run --release --example plant_severance -- frames=24000 cut=12000 seeds=4
//! ```

mod common;

use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::organism::{self, CellType};
use pixel_physics::sim::parallel;
use pixel_physics::sim::world::World;

/// One plant's economy at one stop.
#[derive(Clone, Copy, Default)]
struct Row {
    cells: usize,
    shoot: u32,
    root: u32,
    contact: u32,
    unreached: usize,
    water: f32,
    capacity: f32,
    status: f32,
    demand: f32,
    uptake: f32,
    income: f32,
}

/// Every established plant in the world, keyed by organism id.
///
/// **Established, not registered**: an ungerminated seed is a live organism
/// with one cell, and counting those as plants is how `seedbed_probe`'s
/// first version reported 168 plants of which 143 had done nothing.
fn census(w: &World) -> Vec<(u16, Row)> {
    let mut out: Vec<(u16, Row)> = Vec::new();
    for id in w.live_organism_ids() {
        let Some(st) = w.organism(id) else { continue };
        if st.cells.len() < 2 {
            continue;
        }
        let unreached = st
            .cells
            .keys()
            .filter(|&&(x, y)| w.organism_cell(x, y).is_some_and(|c| c.support == u16::MAX))
            .count();
        out.push((
            id,
            Row {
                cells: st.cells.len(),
                shoot: st.shoot_cells,
                root: st.root_cells,
                contact: st.contact_root_cells,
                unreached,
                water: st.water,
                // `water_capacity_of` is private; it is
                // `WATER_SCALE * contact.max(1)` and is reproduced here
                // rather than exported, with the multiplication written out
                // so a drift shows up as a wrong number rather than as a
                // compile error nobody sees.
                capacity: organism::WATER_SCALE * st.contact_root_cells.max(1) as f32,
                status: st.water_status,
                demand: st.water_demand,
                uptake: st.water_uptake,
                income: st.income,
            },
        ));
    }
    out.sort_unstable_by_key(|&(id, _)| id);
    out
}

/// Cut every established plant free of its own roots, and return how many
/// cells that removed.
///
/// A band of the plant's **own** cells is emptied, not a radius of world:
/// an axe bite also throws soil and rock and would put a structural
/// disturbance in the arm alongside the severance, and the question here is
/// what the *economy* does with a shoot that has no root path. `rows` deep,
/// immediately above the soil surface, which is where a person cuts a tree.
fn sever(w: &mut World, ground_y: i32, rows: i32) -> usize {
    let mut doomed: Vec<(i32, i32)> = Vec::new();
    for id in w.live_organism_ids() {
        let Some(st) = w.organism(id) else { continue };
        if st.cells.len() < 2 {
            continue;
        }
        for &(x, y) in st.cells.keys() {
            if y < ground_y && y >= ground_y - rows {
                doomed.push((x, y));
            }
        }
    }
    doomed.sort_unstable();
    for &(x, y) in &doomed {
        w.set(x, y, Cell::EMPTY);
    }
    doomed.len()
}

/// Remove every root cell from every established plant.
///
/// `reinforces_powder` or a live `RootTip`, which is the discriminator
/// `organism_upkeep` itself uses — a retired root and a retired branch are
/// both `MatureBody`, so cell type alone would leave the root mat standing.
fn deroot(w: &mut World) -> usize {
    let mut doomed: Vec<(i32, i32)> = Vec::new();
    for id in w.live_organism_ids() {
        let Some(st) = w.organism(id) else { continue };
        if st.cells.len() < 2 {
            continue;
        }
        for &(x, y) in st.cells.keys() {
            let c = w.get(x, y);
            if w.materials.get(c.material).reinforces_powder || organism::cell_type(c.aux()) == Some(CellType::RootTip) {
                doomed.push((x, y));
            }
        }
    }
    doomed.sort_unstable();
    for &(x, y) in &doomed {
        w.set(x, y, Cell::EMPTY);
    }
    doomed.len()
}

/// One frame of the whole world.
///
/// **All three calls, and that is not optional**: `parallel::step` alone
/// leaves the active-site schedule and the fields unstepped, so nothing
/// germinates and nothing grows. The first version of this harness ran only
/// the sweep and reported eight registered organisms of one cell each after
/// 6,000 frames -- an ungerminated bed, which reads exactly like a scene
/// that cannot support plants.
fn step(w: &mut World) {
    parallel::step(w);
    w.step_active_sites();
    w.step_fields();
}

fn median(v: &mut [f32]) -> f32 {
    if v.is_empty() {
        return f32::NAN;
    }
    v.sort_unstable_by(f32::total_cmp);
    v[v.len() / 2]
}

/// Sum a field over the plants that were alive at the cut, so an arm is not
/// flattered by plants that germinated afterwards.
fn tracked(rows: &[(u16, Row)], live: &[u16], f: impl Fn(&Row) -> f32) -> Vec<f32> {
    rows.iter().filter(|(id, _)| live.contains(id)).map(|(_, r)| f(r)).collect()
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
    let frames: u64 = arg("frames", 24_000);
    let cut: u64 = arg("cut", 12_000);
    let seeds: u64 = arg("seeds", 3);
    // `seeds=0` runs the single unseeded bed -- `World::new`'s own seed,
    // which is what every stored plant sheet in the repo was measured on.
    let first_seed: u64 = if seeds == 0 { 0 } else { 1 };
    let trees: usize = arg("trees", 4);
    let species: String = arg("species", "tree".to_string());
    let rows: i32 = arg("rows", 3);
    let track: usize = arg("track", 4);
    // **Fine stops immediately after the cut.** The coarse schedule below
    // steps in thousands of frames, and the whole window this harness was
    // built to see can be shorter than one of those steps: measured, a
    // severed crown is gone from the organism between the cut at 24,000 and
    // the next stop at 28,000, so a run read at that resolution can say the
    // plant died and cannot say whether it went on earning and building
    // first. `fine=` is one organism tick by default (45 frames), and
    // `finefor=` is how long to keep that resolution.
    let fine: u64 = arg("fine", 0);
    let finefor: u64 = arg("finefor", 900);
    let arms: String = arg("arms", "control,sever,deroot".to_string());
    assert!(cut < frames, "cut={cut} must be before frames={frames}, or no arm ever differs from its control");

    // **Echoes its own parameters**, because a log that does not name its
    // seed was written by a binary that never had one (`CLAUDE.md`, the
    // 3.5-hour megastudy that was three populations wearing 24 logs).
    println!(
        "plant_severance: species={species} trees={trees} seeds={seeds} frames={frames} cut={cut} rows={rows} track={track} arms={arms}"
    );

    for arm in arms.split(',') {
        for seed in first_seed..=seeds {
            let scene = common::PlantScene {
                trees,
                species: species.clone(),
                seed: if seed == 0 { None } else { Some(seed) },
                ..Default::default()
            };
            let ground_y = scene.ground_y;
            let mut w = scene.build();

            while w.frame < cut {
                step(&mut w);
            }

            let before = census(&w);
            // **The biggest `track=` plants, not every plant in the bed.**
            // A stand recruits, so by the cut most organisms are seedlings
            // sitting at a dozen cells, and a median over all of them is a
            // median over the recruits -- measured, the founders' own
            // numbers were invisible behind a stand median flat at 106.
            // The claim is about a grown plant with a trunk to cut.
            let mut ranked = before.clone();
            ranked.sort_unstable_by_key(|&(_, r)| std::cmp::Reverse(r.cells));
            let live: Vec<u16> = ranked.iter().take(track).map(|&(id, _)| id).collect();
            // **The scene check, before the treatment rather than after.**
            // An arm applied to a bed that grew nothing reports a clean
            // "no effect" and is measuring an empty world.
            if live.is_empty() {
                let ids = w.live_organism_ids();
                let sizes: Vec<usize> = ids.iter().filter_map(|&i| w.organism(i).map(|s| s.cells.len())).collect();
                panic!(
                    "arm={arm} seed={seed}: nothing established by frame {cut}, so this arm asserts nothing \
                     ({} organisms registered, sizes {sizes:?})",
                    ids.len()
                );
            }

            let removed = match arm {
                "control" => 0,
                "sever" => sever(&mut w, ground_y, rows),
                // **The same cut with COLLAPSE UNDER LOAD off**, which is
                // how the owner plays the lab box. `structural.rs`'s
                // detached branch returns early for a **living** plant when
                // `plant_load_failure` is false, so a severed crown is
                // never taken apart -- and nothing in the economy reads
                // attachment, so it should go on earning and building. That
                // is the configuration the report describes, and the switch
                // is flipped **at the cut** rather than at world creation so
                // the warm-up is bit-identical to `sever`'s and the arms
                // differ by the treatment alone.
                "sever_noload" => {
                    w.plant_load_failure = false;
                    sever(&mut w, ground_y, rows)
                }
                "deroot" => deroot(&mut w),
                "deroot_noload" => {
                    w.plant_load_failure = false;
                    deroot(&mut w)
                }
                other => panic!(
                    "unknown arm {other:?}; known: control, sever, sever_noload, deroot, deroot_noload"
                ),
            };
            // The treatment has to have done something, or the arm is its
            // own control wearing a different label.
            if arm != "control" {
                assert!(removed > 0, "arm={arm} seed={seed}: removed no cells, so this arm is the control");
            }

            let mut b_cells: Vec<f32> = tracked(&before, &live, |r| r.cells as f32);
            let cells_at_cut = median(&mut b_cells);
            let mut b_status: Vec<f32> = tracked(&before, &live, |r| r.status);
            println!(
                "\narm={arm} seed={seed}: {} plants at the cut, median {cells_at_cut:.0} cells, \
                 median water_status {:.3}; removed {removed} cells",
                live.len(),
                median(&mut b_status)
            );
            println!("  frame  plants  cells  d_cells  unreached  shoot   root  contact   water/cap  status  demand  uptake  income");

            let mut last = cells_at_cut;
            let stops = 6u64;
            // The fine stops first, then the coarse ones past where they
            // stopped -- one ascending list, so the run steps forward only.
            let mut targets: Vec<u64> = Vec::new();
            if fine > 0 {
                let mut f = cut;
                while f <= cut + finefor {
                    targets.push(f);
                    f += fine;
                }
            }
            for stop in 0..=stops {
                let t = cut + (frames - cut) * stop / stops;
                if targets.last().is_none_or(|&last| t > last) {
                    targets.push(t);
                }
            }
            for target in targets {
                while w.frame < target {
                    step(&mut w);
                }
                let now = census(&w);
                let mut cells: Vec<f32> = tracked(&now, &live, |r| r.cells as f32);
                let mut unreached: Vec<f32> = tracked(&now, &live, |r| r.unreached as f32);
                let mut shoot: Vec<f32> = tracked(&now, &live, |r| r.shoot as f32);
                let mut root: Vec<f32> = tracked(&now, &live, |r| r.root as f32);
                let mut contact: Vec<f32> = tracked(&now, &live, |r| r.contact as f32);
                let mut water: Vec<f32> = tracked(&now, &live, |r| r.water);
                let mut cap: Vec<f32> = tracked(&now, &live, |r| r.capacity);
                let mut status: Vec<f32> = tracked(&now, &live, |r| r.status);
                let mut demand: Vec<f32> = tracked(&now, &live, |r| r.demand);
                let mut uptake: Vec<f32> = tracked(&now, &live, |r| r.uptake);
                let mut income: Vec<f32> = tracked(&now, &live, |r| r.income);
                let alive = cells.len();
                let m = median(&mut cells);
                println!(
                    "  {:>6}  {:>6}  {:>5.0}  {:>+7.0}  {:>9.0}  {:>5.0}  {:>5.0}  {:>7.0}  {:>5.1}/{:<4.0}  {:>6.3}  {:>6.2}  {:>6.2}  {:>6.3}",
                    w.frame,
                    alive,
                    m,
                    m - last,
                    median(&mut unreached),
                    median(&mut shoot),
                    median(&mut root),
                    median(&mut contact),
                    median(&mut water),
                    median(&mut cap),
                    median(&mut status),
                    median(&mut demand),
                    median(&mut uptake),
                    median(&mut income),
                );
                last = m;
            }
        }
    }
}
