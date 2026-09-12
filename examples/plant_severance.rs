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
//! - `unreached` is the **positive control on the cut** for `sever` and
//!   `deroot`, and **is not one for `crown`** -- measured 2026-09-12 at
//!   one-frame resolution. A mid-crown cut disconnects the crown above it,
//!   `anchor_support` marks it, and `organism_upkeep` sheds it **inside the
//!   same organism tick**: on seed 1 the whole event is frames 12,011-12,012,
//!   `unreached` peaks at **3**, and `cells` falls 4,774 -> 4,101 -> 2,699.
//!   There is no frame at which the marked cells are still standing to be
//!   counted, so a zero here says nothing about whether the cut bit. What
//!   does move, immediately and hugely, is `q_now` at the bole: 1,997.95 ->
//!   1,254.43 -> **409.94** over those two frames. Read that, and `cells`
//!   against `removed`, for whether a `crown` cut landed.
//!
//!   For the arms it does control: `anchor_support`
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
    /// **The standing support deficit at the bole**, `q_peak - q_now`, read
    /// once per organism at the cell the anchor walk starts from.
    ///
    /// **Per organism, at the bole, and never per cell.** `accumulate_support`
    /// walks a *spanning tree*, and a thickened trunk is a blob, so
    /// `q_now == 0` across most of a trunk's girth means "not on this tick's
    /// path", not "carries no foliage". `plant.rs`'s die-back records what a
    /// per-cell rule keyed on that did: a stand from 3,437 cells to 704. At
    /// the bole -- `support == 0`, where the walk begins -- the basipetal sum
    /// is the whole live crown and no path artifact exists.
    ///
    /// Both halves are kept rather than only the difference, because the
    /// difference alone cannot tell a plant that lost half a large crown from
    /// one that lost all of a small one.
    q_peak: f32,
    q_now: f32,
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
        // **The bole**: among the cells the anchor walk starts from
        // (`support == 0`), the one carrying the most. A plant has several
        // anchored cells and only one of them is the trunk base; the
        // basipetal sum is what says which.
        let bole = st
            .cells
            .keys()
            .filter_map(|&(x, y)| w.organism_cell(x, y))
            .filter(|c| c.support == 0)
            .map(|c| (c.q_peak, c.q_now))
            .max_by(|a, b| a.0.total_cmp(&b.0));
        out.push((
            id,
            Row {
                cells: st.cells.len(),
                shoot: st.shoot_cells,
                root: st.root_cells,
                contact: st.contact_root_cells,
                unreached,
                water: st.water,
                // **The engine's own function, not a copy of its
                // arithmetic.** This line used to reproduce the formula
                // behind a comment arguing that a drift would show up as a
                // wrong number -- and it did drift, the moment
                // `water_tank_contact_cap` landed: this printed 1244 where
                // the engine used 128, and silently corrupted `fill` with
                // it. A wrong number only helps if someone reads it.
                capacity: pixel_physics::sim::plant::water_capacity_of(st.contact_root_cells),
                status: st.water_status,
                demand: st.water_demand,
                uptake: st.water_uptake,
                income: st.income,
                q_peak: bole.map_or(0.0, |c| c.0),
                q_now: bole.map_or(0.0, |c| c.1),
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

/// Cut a band out of the **middle of each plant's own crown**, leaving the
/// root system attached, and return how many cells that removed.
///
/// **The arm `sever` cannot stand in for.** `sever` cuts at the soil line,
/// which removes the shoot's water path along with its crown, so "did it
/// rebuild a crown" is confounded with "did it dry out" -- the roots are on
/// the other side of the cut. Here the roots, the contact cells and the
/// lower trunk are all untouched: what is removed is foliage-bearing crown,
/// which is the disturbance a resprout mechanism is supposed to answer and
/// the one a felled tree actually suffers.
///
/// The band is placed at `cut_frac` of **each plant's own height**, not at a
/// fixed row: a stand is not uniform, and a fixed row cuts one plant at the
/// waist and another above the top. Height is measured from the plant's own
/// cells, so a seedling gets a proportionally placed cut or none at all.
///
/// This is also the instrument `structural:074` asks for -- a mid-crown
/// disturbance, to run the positive control on `anchor_support`'s
/// replacement of the hop-bounded search. Built once, used by both.
fn crown(w: &mut World, ground_y: i32, rows: i32, cut_frac: f32) -> (usize, Vec<(u16, usize)>) {
    let mut doomed: Vec<(i32, i32)> = Vec::new();
    // **Cells strictly above the cut line, per organism**, counted before
    // the cut. This is what makes `structural:074`'s control an answer
    // rather than a bound: the question is whether the support search
    // removes the cut-off subtree *and nothing else*, and without it the
    // only available claim is "the loss was under some multiple of the
    // band". **Per organism and not a world sum** -- every other figure on
    // these rows is a median over the tracked plants, and a world total
    // beside a per-plant median describes no plant that exists (the same
    // mistake `fill` records below, made one row further up).
    let mut above: Vec<(u16, usize)> = Vec::new();
    for id in w.live_organism_ids() {
        let Some(st) = w.organism(id) else { continue };
        if st.cells.len() < 2 {
            continue;
        }
        // y decreases upward, so the top of the plant is its smallest y and
        // the height above ground is `ground_y - top`.
        let Some(&top) = st.cells.keys().map(|(_, y)| y).min() else { continue };
        let height = ground_y - top;
        if height <= rows {
            // Nothing to cut in the middle of: the whole plant is shorter
            // than the band. Skipped rather than flattened, so this arm does
            // not quietly become `sever` for the seedlings in the bed.
            continue;
        }
        let y_cut = ground_y - (height as f32 * cut_frac) as i32;
        let mut n_above = 0usize;
        for &(x, y) in st.cells.keys() {
            if y < y_cut - rows {
                n_above += 1;
            }
            // Above the cut line by at most `rows`, and never at or below
            // the soil surface -- the roots are the point of this arm.
            if y < y_cut && y >= y_cut - rows && y < ground_y {
                doomed.push((x, y));
            }
        }
        above.push((id, n_above));
    }
    doomed.sort_unstable();
    for &(x, y) in &doomed {
        w.set(x, y, Cell::EMPTY);
    }
    (doomed.len(), above)
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

/// **Does a depletion zone exist around roots at all?** Median
/// plant-available water in soil cells touching root tissue, against soil
/// cells at the same depths that are far from any root.
///
/// This is the positive control for the whole `SOIL_UPTAKE_PER_TICK`
/// question, and it is a *contrast* rather than a level for the reason
/// `CLAUDE.md` prefers paired comparisons: bed-wide moisture rides the water
/// cycle at about +/-1,700 cells, so a single "how wet is the soil" number is
/// that frame's phase plus the drawdown and the two are not separable. Near
/// and far are read on the same frame from the same bed, so the cycle
/// divides out.
///
/// `far` is >= `FAR` cells from any root by a capped BFS, and is restricted
/// to the row band the roots actually occupy — comparing rooted topsoil
/// against unrooted subsoil would report a depth profile as a depletion
/// zone.
///
/// Returns `(near, far, n_near, n_far)`. A run where `n_near` or `n_far` is
/// tiny is reporting noise; both counts are printed for that reason.
fn depletion_contrast(w: &World, width: i32, height: i32) -> (f32, f32, usize, usize) {
    const FAR: u8 = 8;
    let idx = |x: i32, y: i32| (y * width + x) as usize;
    let mut dist: Vec<u8> = vec![u8::MAX; (width * height) as usize];
    let mut queue: std::collections::VecDeque<(i32, i32)> = std::collections::VecDeque::new();
    let (mut lo, mut hi) = (i32::MAX, i32::MIN);
    for y in 0..height {
        for x in 0..width {
            let c = w.get(x, y);
            if c.organism_id() == 0 {
                continue;
            }
            let root = w.materials.get(c.material).reinforces_powder
                || organism::cell_type(c.aux()) == Some(CellType::RootTip);
            if root {
                dist[idx(x, y)] = 0;
                queue.push_back((x, y));
                lo = lo.min(y);
                hi = hi.max(y);
            }
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let d = dist[idx(x, y)];
        if d >= FAR {
            continue;
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= width || ny >= height {
                continue;
            }
            if dist[idx(nx, ny)] > d + 1 {
                dist[idx(nx, ny)] = d + 1;
                queue.push_back((nx, ny));
            }
        }
    }
    let (mut near, mut far) = (Vec::new(), Vec::new());
    if lo <= hi {
        for y in lo..=hi {
            for x in 0..width {
                let c = w.get(x, y);
                if w.materials.get(c.material).water_capacity == 0 || c.organism_id() != 0 {
                    continue;
                }
                let a = pixel_physics::sim::update::plant_available_fraction(c);
                match dist[idx(x, y)] {
                    1 => near.push(a),
                    d if d >= FAR => far.push(a),
                    _ => {}
                }
            }
        }
    }
    let (n_near, n_far) = (near.len(), far.len());
    (median(&mut near), median(&mut far), n_near, n_far)
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
    // **Rows of soil over the stone floor.** Exposed because the one arm
    // that moves this line's numbers is rooting *volume*, not moisture:
    // `plant-water-scarcity-2026-08-30.md` §2d took water status 1.000 ->
    // 0.678 by going from 34 rows to 4, where a 6.3x moisture range moved it
    // not at all. A bed the roots cannot exhaust cannot test a rule about
    // running out of soil.
    let soil: i32 = arg("soil", common::PlantScene::default().soil_depth);
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
    // **Where the `crown` band goes, as a fraction of each plant's own
    // height.** Echoed below, because an unknown argument is silently
    // ignored by this harness and a knob nobody can see the value of is a
    // knob nobody can tell is disconnected.
    let cut_frac: f32 = arg("cut_frac", 0.5);
    assert!(
        cut_frac > 0.0 && cut_frac < 1.0,
        "cut_frac={cut_frac} must be strictly inside (0,1): at 0 the band is the soil line \
         (that is `sever`) and at 1 it is above the plant"
    );
    let arms: String = arg("arms", "control,sever,deroot".to_string());
    assert!(cut < frames, "cut={cut} must be before frames={frames}, or no arm ever differs from its control");

    // **Echoes its own parameters**, because a log that does not name its
    // seed was written by a binary that never had one (`CLAUDE.md`, the
    // 3.5-hour megastudy that was three populations wearing 24 logs).
    println!(
        "plant_severance: species={species} trees={trees} seeds={seeds} frames={frames} cut={cut} rows={rows} cut_frac={cut_frac} track={track} soil={soil} arms={arms}"
    );

    for arm in arms.split(',') {
        for seed in first_seed..=seeds {
            let scene = common::PlantScene {
                trees,
                species: species.clone(),
                seed: if seed == 0 { None } else { Some(seed) },
                soil_depth: soil,
                ..Default::default()
            };
            let ground_y = scene.ground_y;
            let (width, height) = (scene.width, scene.height);
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

            let mut above_cut: Vec<(u16, usize)> = Vec::new();
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
                // **The mid-crown cut, roots left attached.** The arm
                // `plants:124` and `structural:074` both need; see `crown`.
                "crown" => {
                    let (n, a) = crown(&mut w, ground_y, rows, cut_frac);
                    above_cut = a;
                    n
                }
                "crown_noload" => {
                    w.plant_load_failure = false;
                    let (n, a) = crown(&mut w, ground_y, rows, cut_frac);
                    above_cut = a;
                    n
                }
                #[allow(unreachable_patterns)]
                "" => unreachable!(),
                "deroot" => deroot(&mut w),
                "deroot_noload" => {
                    w.plant_load_failure = false;
                    deroot(&mut w)
                }
                other => panic!(
                    "unknown arm {other:?}; known: control, sever, sever_noload, crown, \
                     crown_noload, deroot, deroot_noload"
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
                 median water_status {:.3}; removed {removed} cells{}",
                live.len(),
                median(&mut b_status),
                if above_cut.is_empty() {
                    String::new()
                } else {
                    let mut v: Vec<f32> =
                        above_cut.iter().filter(|(id, _)| live.contains(id)).map(|&(_, n)| n as f32).collect();
                    format!(", median {:.0} cells stood above the cut", median(&mut v))
                }
            );
            println!(
                "  frame  plants  cells  d_cells  unreached  shoot   root  contact    fill  cap   status  worst  demand  uptake  income   q_peak    q_now  deficit   near    far   gap  n_near/n_far"
            );

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
                // **A per-plant fill fraction, not a ratio of medians.** The
                // first version printed `median(water)` beside `median(cap)`
                // and invited the reader to divide -- and those two medians
                // come from *different plants*, so the pair describes no
                // plant that exists. Measured on seed 1 at frame 32,000 it
                // printed 202.6/1088 (0.19) where the plant carrying the rest
                // of that row was at 969.7/1088 (0.89).
                //
                // `plant.rs`'s own stomatal-closure census records this exact
                // mistake being made once before -- "stock/capacity was read
                // as 0.41 ... from a ratio of *medians* taken across
                // different plants at one final frame" -- so this is the
                // second occurrence, and the fix is to divide inside the
                // plant and take the median of the fractions.
                let mut fill: Vec<f32> = tracked(&now, &live, |r| r.water / r.capacity.max(f32::EPSILON));
                let mut cap: Vec<f32> = tracked(&now, &live, |r| r.capacity);
                let mut status: Vec<f32> = tracked(&now, &live, |r| r.status);
                // **The minimum beside the median, because the median hides
                // the finding.** `status` is clipped at 1.0, so an upper
                // median over four plants reads 1.000 whenever any two are
                // saturated -- on seed 1 at frame 48,000 the median said
                // 1.000 while the largest plant sat at 0.677. A ceiling-
                // clipped channel needs its low tail printed or the run
                // reports "the coupling is dead" when it is merely quiet.
                let mut status_min: Vec<f32> = status.clone();
                status_min.sort_unstable_by(f32::total_cmp);
                let worst_status = status_min.first().copied().unwrap_or(f32::NAN);
                let mut demand: Vec<f32> = tracked(&now, &live, |r| r.demand);
                let mut uptake: Vec<f32> = tracked(&now, &live, |r| r.uptake);
                let mut income: Vec<f32> = tracked(&now, &live, |r| r.income);
                // **The deficit is taken inside the plant and then
                // median'd**, never as a difference of two medians -- that is
                // the same mistake `fill` records above, and the two medians
                // would come from different plants.
                let mut q_peak: Vec<f32> = tracked(&now, &live, |r| r.q_peak);
                let mut q_now: Vec<f32> = tracked(&now, &live, |r| r.q_now);
                let mut deficit: Vec<f32> = tracked(&now, &live, |r| r.q_peak - r.q_now);
                let alive = cells.len();
                let m = median(&mut cells);
                let (near, far, n_near, n_far) = depletion_contrast(&w, width, height);
                println!(
                    "  {:>6}  {:>6}  {:>5.0}  {:>+7.0}  {:>9.0}  {:>5.0}  {:>5.0}  {:>7.0}  {:>6.3}  {:>4.0}  {:>6.3}  {:>5.3}  {:>6.2}  {:>6.2}  {:>6.3}  {:>7.2}  {:>7.2}  {:>7.2}  {:>5.3}  {:>5.3}  {:>+5.3}  {}/{}",
                    w.frame,
                    alive,
                    m,
                    m - last,
                    median(&mut unreached),
                    median(&mut shoot),
                    median(&mut root),
                    median(&mut contact),
                    median(&mut fill),
                    median(&mut cap),
                    median(&mut status),
                    worst_status,
                    median(&mut demand),
                    median(&mut uptake),
                    median(&mut income),
                    median(&mut q_peak),
                    median(&mut q_now),
                    median(&mut deficit),
                    near,
                    far,
                    far - near,
                    n_near,
                    n_far,
                );
                last = m;
            }
        }
    }
}
