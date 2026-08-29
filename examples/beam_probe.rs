//! **SCRATCH — validating the moment/section model before proposing it.**
//!
//! Asks one question of a grown tree: if the plant's failure criterion were
//! `bending moment > section capacity` (the criterion `load.rs` already runs
//! for rock) instead of `reach > constant`, would the answer be
//! *non-degenerate*? That is `CLAUDE.md`'s positive control, run before the
//! mechanism rather than after it.
//!
//! Prints, per woody cell of the largest organism:
//!   - `section` — the run of woody same-organism cells perpendicular to the
//!     local stem axis, which is what `thicken` already maintains
//!   - `torque`  — `|Sx − x·M|` over the cell's own subtree, mass-weighted by
//!     `MaterialDef::density`, accumulated up a parent forest rooted at the
//!     structural anchors
//!   - `stress`  — torque / (section² · k)
//!
//! ```text
//! cargo run --release --example beam_probe -- frames=7100 species=tree
//! ```
mod common;

use pixel_physics::sim::organism;
use pixel_physics::sim::world::World;
use std::collections::HashMap;

const NEIGHBOURS_8: [(i32, i32); 8] = [(0, -1), (0, 1), (-1, 0), (1, 0), (-1, -1), (1, -1), (-1, 1), (1, 1)];

fn main() {
    let arg = |k: &str| std::env::args().find_map(|a| a.strip_prefix(k).map(str::to_string));
    let frames: u64 = arg("frames=").map_or(7100, |v| v.parse().expect("frames=N"));
    let species = arg("species=").unwrap_or_else(|| "tree".to_string());
    let trees: usize = arg("trees=").map_or(1, |v| v.parse().expect("trees=N"));
    let scene = common::PlantScene { trees, species: species.clone(), ..common::PlantScene::default() };
    let mut w = scene.build();
    println!("beam_probe: species={species} trees={trees} frames={frames} worldseed={}", w.seed);
    while w.frame < frames {
        pixel_physics::sim::parallel::step(&mut w);
        w.step_active_sites();
        w.step_fields();
    }

    // The biggest organism in the world, by owned cell count.
    let b = w.bounds().expect("bounded world");
    let mut counts: HashMap<u16, usize> = HashMap::new();
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let id = w.get(x, y).organism_id();
            if id != 0 {
                *counts.entry(id).or_insert(0) += 1;
            }
        }
    }
    let Some((&id, &n)) = counts.iter().max_by_key(|(_, &n)| n) else {
        println!("no organisms");
        return;
    };
    println!("largest organism {id}: {n} cells");

    let mut cells: Vec<(i32, i32)> = w.organism(id).expect("state").cells.keys().copied().collect();
    cells.sort_unstable_by_key(|&(x, y)| (y, x));
    let index: HashMap<(i32, i32), usize> = cells.iter().enumerate().map(|(i, &p)| (p, i)).collect();

    // A parent forest rooted at the anchors, BFS-ordered so children come
    // after parents. Anchors: any cell touching a Solid, or root tissue in
    // wet powder -- the same test `plant::is_structural_anchor` runs.
    let mut parent: Vec<Option<usize>> = vec![None; cells.len()];
    let mut seen = vec![false; cells.len()];
    let mut order: Vec<usize> = Vec::new();
    let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    let mut anchors = 0usize;
    for (i, &(x, y)) in cells.iter().enumerate() {
        if is_anchor(&w, x, y, id) {
            seen[i] = true;
            queue.push_back(i);
            anchors += 1;
        }
    }
    while let Some(i) = queue.pop_front() {
        order.push(i);
        let (x, y) = cells[i];
        for (dx, dy) in NEIGHBOURS_8 {
            let Some(&j) = index.get(&(x + dx, y + dy)) else { continue };
            if seen[j] {
                continue;
            }
            seen[j] = true;
            parent[j] = Some(i);
            queue.push_back(j);
        }
    }
    println!("{anchors} anchors; forest reached {} of {} cells", order.len(), cells.len());

    // Mass and first moment, accumulated up the forest. Mass is the
    // material's own density -- leaf 0.25 against wood 0.9, so foliage
    // weighs a quarter of what it costs in cell count.
    let mass: Vec<f64> = cells.iter().map(|&(x, y)| w.materials.get(w.get(x, y).material).density as f64).collect();
    let mut m: Vec<f64> = mass.clone();
    let mut sx: Vec<f64> = cells.iter().enumerate().map(|(i, &(x, _))| mass[i] * x as f64).collect();
    for &i in order.iter().rev() {
        if let Some(p) = parent[i] {
            m[p] += m[i];
            sx[p] += sx[i];
        }
    }

    // Section: the run of woody same-organism cells across the stem axis.
    let mut rows: Vec<(f64, u32, i32, i32, f64, f64)> = Vec::new(); // stress, section, x, y, torque, m
    let mut disagree = [0u32; 2];
    let mut section_sum = [0u64; 3];
    for (i, &(x, y)) in cells.iter().enumerate() {
        let c = w.get(x, y);
        if !w.materials.get(c.material).woody {
            continue;
        }
        // **Three section measures, reported side by side, because they
        // disagree and the disagreement is the point.** An earlier version
        // of this probe computed all three and published one without
        // saying which -- so its numbers could not be used to set a
        // constant. See `Reports/tree-mechanics-plan-2026-08-29.md` §9.
        //
        //  - `load_path`: perpendicular to the 8-BFS parent link, the way
        //    `load::section_cells` reads it for rock. **Its known failure:**
        //    where the parent arrives diagonally the walk runs *lengthwise
        //    down the stem*, which `plant.rs`'s `stem_run` doc records as
        //    having gone wrong three times already.
        //  - `narrowest`: the thinnest chord over the four axis pairs. A
        //    failure section is the narrowest place a break can run, but
        //    this reads every surface cell of a thick stem as thin.
        //  - `axis`: perpendicular to `organism::supply_direction`, which is
        //    what `plant::cross_section_axis` and `thicken` actually use --
        //    and therefore the one a shipped model should read.
        let vertical_path = parent[i].is_none_or(|p| cells[p].0 == x);
        let (ax, ay) = if vertical_path { (1, 0) } else { (0, 1) };
        let load_path = run_along(&w, x, y, id, ax, ay);
        let narrowest = min_section(&w, x, y, id);
        let axis = section_width(&w, x, y, id);
        disagree[0] += u32::from(load_path != axis);
        disagree[1] += u32::from(narrowest != axis);
        section_sum[0] += load_path as u64;
        section_sum[1] += narrowest as u64;
        section_sum[2] += axis as u64;
        // `axis` is the one a shipped model would read, so it is the one
        // the stress column below is built from.
        let section = axis;
        let torque = (sx[i] - x as f64 * m[i]).abs();
        // `load.rs`'s own shape: capacity goes as the square of the section.
        let cap = (section as f64).powi(2);
        rows.push((torque / cap, section, x, y, torque, m[i]));
    }
    rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    println!("\n{} woody cells", rows.len());

    let sections: Vec<u32> = rows.iter().map(|r| r.1).collect();
    let mut hist: std::collections::BTreeMap<u32, usize> = std::collections::BTreeMap::new();
    for &s in &sections {
        *hist.entry(s).or_insert(0) += 1;
    }
    println!("section widths (axis measure): {hist:?}");
    let n = rows.len().max(1) as f64;
    println!(
        "section measures disagree: load_path vs axis on {} of {} cells ({:.0}%), narrowest vs axis on {} ({:.0}%)",
        disagree[0],
        rows.len(),
        disagree[0] as f64 * 100.0 / n,
        disagree[1],
        disagree[1] as f64 * 100.0 / n
    );
    println!(
        "  mean section: load_path {:.2}, narrowest {:.2}, axis {:.2}",
        section_sum[0] as f64 / n,
        section_sum[1] as f64 / n,
        section_sum[2] as f64 / n
    );

    let mut st: Vec<f64> = rows.iter().map(|r| r.0).collect();
    st.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |p: f64| st[((st.len() - 1) as f64 * p) as usize];
    println!("stress percentiles: p10 {:.2} p50 {:.2} p90 {:.2} p99 {:.2} max {:.2}", q(0.1), q(0.5), q(0.9), q(0.99), q(1.0));
    println!("  ratio max/median = {:.1}x", q(1.0) / q(0.5).max(1e-9));

    println!("\ntop 15 by stress (the sections a moment model would break first):");
    println!("  {:>8} {:>7} {:>6} {:>6} {:>9} {:>8} {:>6}", "stress", "section", "x", "y", "torque", "mass", "arm");
    for r in rows.iter().take(15) {
        println!("  {:>8.2} {:>7} {:>6} {:>6} {:>9.0} {:>8.0} {:>6.1}", r.0, r.1, r.2, r.3, r.4, r.5, r.4 / r.5.max(1e-9));
    }
    println!("\nbottom 5 by stress (what the model would leave alone):");
    for r in rows.iter().rev().take(5) {
        println!("  {:>8.2} {:>7} {:>6} {:>6} {:>9.0} {:>8.0} {:>6.1}", r.0, r.1, r.2, r.3, r.4, r.5, r.4 / r.5.max(1e-9));
    }
    // Is the *base* the most-stressed section, as it must be for "narrow
    // base, heavy top" to be a mechanism rather than a wish? Read stress
    // against height above the anchor plate.
    let anchor_y = cells.iter().enumerate().filter(|(i, _)| parent[*i].is_none()).map(|(_, &(_, y))| y).max().unwrap_or(0);
    let mut band: std::collections::BTreeMap<i32, (usize, f64, f64)> = std::collections::BTreeMap::new();
    for r in &rows {
        let h = (anchor_y - r.3) / 10;
        let e = band.entry(h).or_insert((0, 0.0, 0.0));
        e.0 += 1;
        e.1 += r.0;
        e.2 = e.2.max(r.0);
    }
    println!("\nstress by height above the anchor plate (bands of 10 rows):");
    println!("  {:>6} {:>7} {:>10} {:>10}", "rows up", "cells", "mean", "max");
    for (h, (n, sum, mx)) in band {
        println!("  {:>6} {:>7} {:>10.1} {:>10.1}", h * 10, n, sum / n as f64, mx);
    }

    // The comparison that matters: what does today's criterion say about the
    // same cells? `support` is the weighted anchor distance the shipped rule
    // thresholds against `max_cantilever_reach`.
    let mut sup: Vec<u16> = rows.iter().filter_map(|r| w.organism_cell(r.2, r.3).map(|c| c.support)).filter(|&s| s != u16::MAX).collect();
    sup.sort_unstable();
    if !sup.is_empty() {
        let qs = |p: f64| sup[((sup.len() - 1) as f64 * p) as usize];
        println!("\ntoday's criterion, same cells -- support distance: p50 {} p90 {} max {} (fails past ~96)", qs(0.5), qs(0.9), qs(1.0));
    }
}

fn is_anchor(w: &World, x: i32, y: i32, id: u16) -> bool {
    use pixel_physics::sim::material::MaterialKind;
    let cell = w.get(x, y);
    if cell.organism_id() != id {
        return false;
    }
    let root = w.materials.get(cell.material).reinforces_powder || organism::cell_type(cell.aux()) == Some(organism::CellType::RootTip);
    [(0, -1), (0, 1), (-1, 0), (1, 0)].iter().any(|&(dx, dy)| {
        let n = w.get(x + dx, y + dy);
        let m = w.materials.get(n.material);
        (m.kind == MaterialKind::Solid && m.anchors_organisms) || (root && m.kind == MaterialKind::Powder && m.water_capacity > 0)
    })
}

/// The run of woody same-organism cells through `(x, y)` perpendicular to
/// the local stem axis -- the stem's true cross-section, the same quantity
/// `plant::thicken`'s pipe-model gate already measures.
fn section_width(w: &World, x: i32, y: i32, id: u16) -> u32 {
    let (ax, ay) = organism::supply_direction(w, x, y).unwrap_or((0.0, -1.0));
    let (px, py) = (-ay, ax);
    const T: f32 = 0.383;
    let sx = if px > T {
        1
    } else if px < -T {
        -1
    } else {
        0
    };
    let sy = if py > T {
        1
    } else if py < -T {
        -1
    } else {
        0
    };
    let (sx, sy) = if sx == 0 && sy == 0 { (1, 0) } else { (sx, sy) };
    run_along(w, x, y, id, sx, sy)
}

/// The **narrowest** chord of woody tissue through the cell, over the four
/// axis pairs. A failure section is the narrowest place a break can run, and
/// taking the axis-perpendicular chord alone reports a wide bole as thin
/// whenever `supply_direction` comes back diagonal.
fn min_section(w: &World, x: i32, y: i32, id: u16) -> u32 {
    [(1, 0), (0, 1), (1, 1), (1, -1)].iter().map(|&(sx, sy)| run_along(w, x, y, id, sx, sy)).min().unwrap_or(1)
}

fn run_along(w: &World, x: i32, y: i32, id: u16, sx: i32, sy: i32) -> u32 {
    let mut n = 1;
    for dir in [-1, 1] {
        let mut step = 1;
        while step < 64 {
            let (cx, cy) = (x + dir * sx * step, y + dir * sy * step);
            let c = w.get(cx, cy);
            if c.organism_id() != id || !w.materials.get(c.material).woody {
                break;
            }
            n += 1;
            step += 1;
        }
    }
    n
}
