//! What the support field is actually *made of*, and what a replacement for it
//! would have to reproduce.
//!
//! Built for `Reports/structural-support-model.md`, after
//! `open-bugs-handoff.md` §S concluded that `Cell::aux` is doing two jobs —
//! a short-range gradient and world-scale reachability — and that answering
//! the second by iterated shortest-path relaxation is what produces the
//! count-to-infinity climb. Two replacement families were sketched there and
//! neither was sized. This sizes them, offline, on a generated world, with
//! **no behaviour change of any kind**: it reads the field, builds candidate
//! fields beside it, and compares. Nothing here is a proposal to ship.
//!
//! Three questions, each of which can kill one of the candidates outright:
//!
//! 1. **How much of the field is magnitude nobody reads?** A histogram of
//!    every body cell's converged distance. If the mass of the world sits
//!    under 48 — `load::ROOTWARD_CHECK_STEPS`, the furthest any consumer
//!    walks — then the long tail really is reachability wearing a distance,
//!    and a short-horizon field is not throwing information away.
//! 2. **What does clamping cost?** A saturating gradient sounds local and
//!    free. It is neither, and the reason is that `load.rs` reads the field
//!    as a strict order: `dependants` is `aux > own`, `support_count` is
//!    `aux < own`, `support_parent` is an argmin over `aux + step`. Clamping
//!    creates **plateaus**, and a cell in the middle of one has no strictly
//!    lower neighbour, so it has no supports and carries no dependants. This
//!    counts the plateau cells at several horizons, and splits them by
//!    whether `load::is_structurally_interesting` would ever look at them.
//! 3. **Is the coarse layer really 5,120 nodes?** §S's sketch says
//!    connectivity should live on the chunk layer — "5,120 chunks at the
//!    shipped size, against 19.4 M cells". A chunk is not a node, though: a
//!    chunk holding two disconnected pieces of rock is two nodes, and if the
//!    real count is fifty thousand rather than five, the "microseconds"
//!    claim is a different claim. This censuses the connected components of
//!    body material *within* each chunk, which is the true node count, and
//!    the adjacencies between them, which is the true edge count.
//!
//! And then the one that decides whether any of it is buildable:
//!
//! 4. **Does a hierarchical potential produce the same load DAG?** Build
//!    `(coarse level, distance-to-portal within the component)` as a
//!    lexicographic potential — the shape §S points at and the shape
//!    `worldgen-design.md` §6b already plans for M10 — and compare its
//!    orientation, cell by cell, against the exact field's. The load model
//!    does not read distances; it reads which of my neighbours are below me.
//!    If the two orientations agree, the replacement is invisible to
//!    `load.rs`. If they disagree, the size of the disagreement is the size
//!    of the behaviour change, and it is a judge-by-eye question before it
//!    is an engineering one.
//!
//! **The positive control matters more than usual here** (`CLAUDE.md`: run
//! the case whose answer you know is non-zero). Question 4's instrument is a
//! set-difference over orientations, and a set-difference that is *silently
//! comparing a thing with itself* reports perfect agreement, which is the
//! answer the proposal wants. So `control=1` runs the same comparison
//! against a deliberately broken field (every cell at 0) and must report
//! near-total disagreement. A run whose control reads 0% has measured
//! nothing.
//!
//! ```text
//! cargo run --release --example support_census -- size=2048x640
//! cargo run --release --example support_census -- size=8192x2560 preset=rolling seed=1
//! cargo run --release --example support_census -- size=2048x640 control=1
//! cargo run --release --example support_census -- size=2048x640 seeds=1,7,24301
//! ```

use pixel_physics::sim::chunk::{Rect, CHUNK_SIZE};
use pixel_physics::sim::material::{self, MaterialId};
use pixel_physics::sim::structural;
use pixel_physics::sim::world::World;
use pixel_physics::worldgen::{self, Spec, WorldgenPresets};
use std::time::Instant;

/// The four-neighbourhood, in `structural::NEIGHBOURS_4`'s exact order.
/// Ties in the argmin below break on it, and a different order is a
/// different support forest — see that constant's own doc.
const N4: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

/// A flat mirror of everything either half of this census reads, for the
/// same reason `structural::compute_world_distances` keeps one: `World::get`
/// is a bounds check plus a `HashMap` lookup per read, and every cell here
/// is read at least five times over.
struct Grid {
    w: usize,
    h: usize,
    /// Material id per cell, `material::EMPTY` outside the body.
    mat: Vec<u16>,
    /// Is this cell part of the structural system at all — `is_body_material`
    /// and unowned by an organism, which is exactly `is_relaxable`.
    body: Vec<bool>,
    /// The converged distance the engine actually stores.
    exact: Vec<u16>,
    crack_right: Vec<bool>,
    crack_down: Vec<bool>,
    /// `(below, above, beside)` per material id, in
    /// `compute_world_distances`' tuple order so a step is priced the same
    /// way the writer prices it.
    costs: Vec<(u16, u16, u16)>,
}

impl Grid {
    fn idx(&self, x: i32, y: i32) -> usize {
        y as usize * self.w + x as usize
    }
    fn inside(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h
    }
    /// `structural::edge_is_cracked` against the mirror. Each edge is owned
    /// by exactly one of the two cells it separates, so reaching left or up
    /// asks the *neighbour* about its own right or down edge — which means
    /// this reads a cell one step *outside* the offset it was given, and has
    /// to bounds-check that rather than the destination.
    fn cracked(&self, x: i32, y: i32, dx: i32, dy: i32) -> bool {
        let (ox, oy) = match (dx, dy) {
            (-1, 0) => (x - 1, y),
            (0, -1) => (x, y - 1),
            _ => (x, y),
        };
        if !self.inside(ox, oy) {
            return false; // the world edge is a wall, not a fracture
        }
        match (dx, dy) {
            (1, 0) | (-1, 0) => self.crack_right[self.idx(ox, oy)],
            (0, 1) | (0, -1) => self.crack_down[self.idx(ox, oy)],
            _ => false,
        }
    }
    /// What it costs the cell at `(x, y)` to be held by its neighbour at
    /// `(dx, dy)`. `y` grows downward, so `dy == 1` is the cell beneath
    /// (standing on it, cheap) and `dy == -1` the cell above (hanging from
    /// it, dear). The cost is the **supported** cell's own material, which
    /// is what `compute_world_distances` and `structural::tick` both charge.
    fn step(&self, i: usize, dy: i32) -> u16 {
        let (below, above, beside) = self.costs[self.mat[i] as usize];
        match dy {
            1 => below,
            -1 => above,
            _ => beside,
        }
    }
    /// Is there body material at `(x, y)` we can reach from `(x-dx, y-dy)`
    /// without crossing a fracture.
    fn linked(&self, x: i32, y: i32, dx: i32, dy: i32) -> Option<usize> {
        let (nx, ny) = (x + dx, y + dy);
        if !self.inside(nx, ny) || self.cracked(x, y, dx, dy) {
            return None;
        }
        let ni = self.idx(nx, ny);
        self.body[ni].then_some(ni)
    }
}

fn mirror(world: &World) -> Grid {
    let bounds = world.bounds().expect("bounded world");
    let w = (bounds.max_x - bounds.min_x + 1) as usize;
    let h = (bounds.max_y - bounds.min_y + 1) as usize;
    let n = w * h;
    let mut g = Grid {
        w,
        h,
        mat: vec![material::EMPTY.0; n],
        body: vec![false; n],
        exact: vec![u16::MAX; n],
        crack_right: vec![false; n],
        crack_down: vec![false; n],
        costs: (0..world.materials.len())
            .map(|m| {
                let mat = world.materials.get(MaterialId(m as u16));
                (mat.support_cost_below, mat.support_cost_above, mat.support_cost_beside)
            })
            .collect(),
    };
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let c = world.get(bounds.min_x + x, bounds.min_y + y);
            let i = g.idx(x, y);
            g.mat[i] = c.material.0;
            g.body[i] = structural::is_body_material(world, c.material) && c.organism_id() == 0;
            g.exact[i] = c.aux();
            g.crack_right[i] = c.crack_right();
            g.crack_down[i] = c.crack_down();
        }
    }
    g
}

/// The set of 4-neighbours a field says are holding `(x, y)` up: strictly
/// lower potential, across an uncracked edge, body material.
///
/// This is `load::support_count`'s predicate and the inverse of
/// `load::dependants`'. It is the *only* thing the load model reads the
/// field for — `support_parent` additionally breaks ties on `+ step`, which
/// is a choice among these four bits, never a fifth option. So two fields
/// that produce the same four bits everywhere are indistinguishable to
/// `load.rs`, whatever their magnitudes.
fn supports<T: Ord + Copy>(g: &Grid, x: i32, y: i32, field: &[T]) -> u8 {
    let i = g.idx(x, y);
    let own = field[i];
    let mut bits = 0u8;
    for (k, (dx, dy)) in N4.into_iter().enumerate() {
        if let Some(ni) = g.linked(x, y, dx, dy) {
            if field[ni] < own {
                bits |= 1 << k;
            }
        }
    }
    bits
}

/// Connected components of body material **within one chunk**, 4-connected
/// and not crossing a fracture, which is the neighbourhood support travels
/// through.
///
/// The component, not the chunk, is the node any coarse connectivity
/// structure has to carry: a chunk holding a cliff and a detached boulder
/// cannot answer "does this reach bedrock" with one bit.
fn components(g: &Grid) -> (Vec<u32>, usize) {
    let mut label = vec![u32::MAX; g.w * g.h];
    let mut next = 0u32;
    let cw = CHUNK_SIZE as usize;
    let mut stack: Vec<(i32, i32)> = Vec::new();
    for cy0 in (0..g.h).step_by(cw) {
        for cx0 in (0..g.w).step_by(cw) {
            let (x1, y1) = ((cx0 + cw).min(g.w) as i32, (cy0 + cw).min(g.h) as i32);
            for sy in cy0 as i32..y1 {
                for sx in cx0 as i32..x1 {
                    let si = g.idx(sx, sy);
                    if !g.body[si] || label[si] != u32::MAX {
                        continue;
                    }
                    let id = next;
                    next += 1;
                    label[si] = id;
                    stack.push((sx, sy));
                    while let Some((x, y)) = stack.pop() {
                        for (dx, dy) in N4 {
                            let (nx, ny) = (x + dx, y + dy);
                            // Confined to the chunk: crossing the boundary
                            // is an *edge* of the coarse graph, not a
                            // merge, which is the whole point of the layer.
                            if nx < cx0 as i32 || nx >= x1 || ny < cy0 as i32 || ny >= y1 {
                                continue;
                            }
                            let Some(ni) = g.linked(x, y, dx, dy) else { continue };
                            if label[ni] == u32::MAX {
                                label[ni] = id;
                                stack.push((nx, ny));
                            }
                        }
                    }
                }
            }
        }
    }
    (label, next as usize)
}

/// A percentile of an already-sorted slice.
fn pct(sorted: &[usize], p: f64) -> usize {
    if sorted.is_empty() {
        return 0;
    }
    let k = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[k]
}

fn main() {
    let mut explicit = (2048i32, 640i32);
    let mut preset = String::new();
    let mut seeds: Vec<u64> = vec![1];
    let mut control = false;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "size" => {
                let (w, h) = v.split_once('x').expect("size=WxH");
                explicit = (w.parse().expect("width"), h.parse().expect("height"));
            }
            "preset" => preset = v.to_string(),
            "seeds" => seeds = v.split(',').map(|t| t.parse().expect("seeds=1,7")).collect(),
            "seed" => seeds = vec![v.parse().expect("seed=N")],
            "control" => control = v != "0",
            _ => eprintln!("ignoring unknown argument {arg}"),
        }
    }
    let (presets, err) = WorldgenPresets::load();
    if let Some(e) = err {
        eprintln!("preset load: {e}");
    }
    let name = if preset.is_empty() { presets.default_name() } else { preset.clone() };
    let params = presets.get(&name).unwrap_or_else(|| panic!("unknown preset {name:?}")).clone();
    let (w, h) = explicit;
    // **Echo the parameters.** A harness whose knobs are invisible is a
    // harness nobody can tell is disconnected — `CLAUDE.md`'s megastudy that
    // produced 24 logs of 3 populations.
    println!(
        "support_census: {w}x{h} preset {name} seeds {seeds:?} control {} | chunk {CHUNK_SIZE}, {} chunks",
        u8::from(control),
        (w / CHUNK_SIZE) * (h / CHUNK_SIZE)
    );

    for &seed in &seeds {
        let t = Instant::now();
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
        worldgen::generate_only(&mut world, Spec::Generated { params: &params, seed });
        structural::compute_world_distances(&mut world);
        let g = mirror(&world);
        drop(world);
        println!("\n=== seed {seed} — generated and converged in {:.0} ms ===", t.elapsed().as_secs_f64() * 1000.0);

        // ---- Q1: what the magnitude is ------------------------------------
        let body: usize = g.body.iter().filter(|&&b| b).count();
        let denom = body.max(1) as f64;
        let mut buckets = [0usize; 9];
        let (mut sum, mut max) = (0u64, 0u16);
        for i in 0..g.mat.len() {
            if !g.body[i] {
                continue;
            }
            let d = g.exact[i];
            let k = match d {
                0 => 0,
                1..=16 => 1,
                17..=48 => 2,
                49..=64 => 3,
                65..=128 => 4,
                129..=256 => 5,
                257..=1024 => 6,
                1025..=65534 => 7,
                u16::MAX => 8,
            };
            buckets[k] += 1;
            if d != u16::MAX {
                sum += d as u64;
                max = max.max(d);
            }
        }
        println!("  body cells {body}  max {max}  mean {:.1}", sum as f64 / denom);
        println!(
            "  aux: 0 {:.2}% | 1-16 {:.2}% | 17-48 {:.2}% | 49-64 {:.2}% | 65-128 {:.2}% | 129-256 {:.2}% | 257-1k {:.2}% | 1k+ {:.2}% | MAX {} cells",
            100.0 * buckets[0] as f64 / denom,
            100.0 * buckets[1] as f64 / denom,
            100.0 * buckets[2] as f64 / denom,
            100.0 * buckets[3] as f64 / denom,
            100.0 * buckets[4] as f64 / denom,
            100.0 * buckets[5] as f64 / denom,
            100.0 * buckets[6] as f64 / denom,
            100.0 * buckets[7] as f64 / denom,
            buckets[8]
        );

        // ---- Q2: what clamping costs --------------------------------------
        //
        // A cell is *stranded* by a horizon H if, after clamping every value
        // to H, it has no strictly-lower neighbour left. `load::dependants`
        // and `load::support_count` then both come back empty for it: it is
        // carried by nothing and carries nothing, which is not a weaker
        // answer than the truth, it is a different question being answered.
        //
        // Split by `attached`, because attached bulk with no free face and
        // no crack never reaches the load model at all
        // (`load::is_structurally_interesting`) — a plateau buried inside a
        // massif is harmless and a plateau on an exposed face is not.
        print!("  stranded by horizon:");
        for horizon in [32u16, 48, 64, 128, 256, 1024] {
            let clamp: Vec<u16> = g.exact.iter().map(|&d| d.min(horizon)).collect();
            let mut stranded = 0usize;
            let mut stranded_deep = 0usize;
            for y in 0..g.h as i32 {
                for x in 0..g.w as i32 {
                    let i = g.idx(x, y);
                    // Only cells that had support to lose: an anchor at 0
                    // legitimately has none, and neither does a cell the
                    // exact field already calls unreachable.
                    if !g.body[i] || g.exact[i] == 0 || g.exact[i] == u16::MAX {
                        continue;
                    }
                    if supports(&g, x, y, &g.exact) != 0 && supports(&g, x, y, &clamp) == 0 {
                        stranded += 1;
                        // Would the load model ever look at it? Its cheap
                        // test is "no empty 4-neighbour and no crack".
                        let exposed = N4.iter().any(|&(dx, dy)| {
                            g.cracked(x, y, dx, dy)
                                || !g.inside(x + dx, y + dy)
                                || g.mat[g.idx(x + dx, y + dy)] == material::EMPTY.0
                        });
                        if !exposed {
                            stranded_deep += 1;
                        }
                    }
                }
            }
            print!(
                "  H{horizon}: {:.2}% ({:.2}% buried)",
                100.0 * stranded as f64 / denom,
                100.0 * stranded_deep as f64 / denom
            );
        }
        println!();

        // ---- Q3: how big the coarse layer really is -----------------------
        let t = Instant::now();
        let (label, nodes) = components(&g);
        let mut size = vec![0usize; nodes];
        for i in 0..label.len() {
            if label[i] != u32::MAX {
                size[label[i] as usize] += 1;
            }
        }
        // Coarse edges: an adjacency between two components across a chunk
        // boundary. Counted once per unordered pair.
        let mut edges: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
        // Anchor nodes: components holding a cell that touches bedrock or
        // the world edge, which is `compute_world_distances`' seed rule.
        let mut anchor = vec![false; nodes];
        for y in 0..g.h as i32 {
            for x in 0..g.w as i32 {
                let i = g.idx(x, y);
                if !g.body[i] {
                    continue;
                }
                let a = label[i];
                for (dx, dy) in N4 {
                    let (nx, ny) = (x + dx, y + dy);
                    if !g.inside(nx, ny) {
                        anchor[a as usize] = true; // outside the world is bedrock
                        continue;
                    }
                    if g.mat[g.idx(nx, ny)] == material::BEDROCK.0 {
                        anchor[a as usize] = true;
                        continue;
                    }
                    let Some(ni) = g.linked(x, y, dx, dy) else { continue };
                    let b = label[ni];
                    if a != b {
                        edges.insert((a.min(b), a.max(b)));
                    }
                }
            }
        }
        let mut sizes: Vec<usize> = size.clone();
        sizes.sort_unstable();
        let tiny = sizes.iter().filter(|&&s| s <= 4).count();
        println!(
            "  coarse layer: {nodes} nodes, {} edges, {} anchored | component cells median {} p90 {} max {} | {tiny} nodes of <=4 cells ({:.1}%) | built in {:.0} ms",
            edges.len(),
            anchor.iter().filter(|&&a| a).count(),
            pct(&sizes, 0.5),
            pct(&sizes, 0.9),
            sizes.last().copied().unwrap_or(0),
            100.0 * tiny as f64 / nodes.max(1) as f64,
            t.elapsed().as_secs_f64() * 1000.0
        );

        // ---- Q4: does a hierarchical potential give the same DAG? ---------
        //
        // Level: BFS hops over the component graph from the anchored
        // components. Offset: a weighted multi-source search within each
        // component, from the cells that lead *out* of it into a strictly
        // lower level (or, in an anchor component, from the cells touching
        // bedrock). The potential is the pair, compared lexicographically —
        // which is exactly what makes it valid: descending offset reaches a
        // portal, crossing the portal drops the level, and the level cannot
        // rise. It is not the shortest path and does not try to be.
        let t = Instant::now();
        let mut level = vec![u32::MAX; nodes];
        let mut adj: Vec<Vec<u32>> = vec![Vec::new(); nodes];
        for &(a, b) in &edges {
            adj[a as usize].push(b);
            adj[b as usize].push(a);
        }
        let mut queue: std::collections::VecDeque<u32> = std::collections::VecDeque::new();
        for n in 0..nodes {
            if anchor[n] {
                level[n] = 0;
                queue.push_back(n as u32);
            }
        }
        while let Some(n) = queue.pop_front() {
            let d = level[n as usize];
            for &m in &adj[n as usize] {
                if level[m as usize] == u32::MAX {
                    level[m as usize] = d + 1;
                    queue.push_back(m);
                }
            }
        }
        let max_level = level.iter().filter(|&&l| l != u32::MAX).max().copied().unwrap_or(0);
        let orphan_nodes = level.iter().filter(|&&l| l == u32::MAX).count();

        // Offset, per cell. Dial-queue Dijkstra over the whole grid at once
        // rather than per component: the seed set already encodes which
        // component a cell belongs to, and a search may never leave it
        // because a step to a different component is a step to a different
        // level and is not an offset at all.
        let mut offset = vec![u16::MAX; g.mat.len()];
        let max_step = g.costs.iter().map(|&(b, a, s)| b.max(a).max(s)).max().unwrap_or(1).max(1) as usize;
        let ring = max_step + 1;
        let mut dial: Vec<Vec<u32>> = vec![Vec::new(); ring];
        let mut queued = 0usize;
        for y in 0..g.h as i32 {
            for x in 0..g.w as i32 {
                let i = g.idx(x, y);
                if !g.body[i] {
                    continue;
                }
                let my = level[label[i] as usize];
                if my == u32::MAX {
                    continue; // unreachable component: no portal to seed from
                }
                let portal = N4.iter().any(|&(dx, dy)| {
                    let (nx, ny) = (x + dx, y + dy);
                    if !g.inside(nx, ny) {
                        return my == 0; // the world edge is bedrock
                    }
                    if g.mat[g.idx(nx, ny)] == material::BEDROCK.0 {
                        return my == 0;
                    }
                    match g.linked(x, y, dx, dy) {
                        Some(ni) => level[label[ni] as usize] < my,
                        None => false,
                    }
                });
                if portal {
                    offset[i] = 0;
                    dial[0].push(i as u32);
                    queued += 1;
                }
            }
        }
        let mut lv = 0u16;
        while queued > 0 {
            let slot = lv as usize % ring;
            while let Some(packed) = dial[slot].pop() {
                queued -= 1;
                let i = packed as usize;
                if offset[i] != lv {
                    continue;
                }
                let (x, y) = ((i % g.w) as i32, (i / g.w) as i32);
                for (dx, dy) in N4 {
                    let Some(ni) = g.linked(x, y, dx, dy) else { continue };
                    // Stay inside the component: a step across the coarse
                    // graph is a level change, not an offset step.
                    if label[ni] != label[i] {
                        continue;
                    }
                    let step = g.step(ni, -dy);
                    let cand = lv.saturating_add(step);
                    if cand < offset[ni] {
                        offset[ni] = cand;
                        dial[cand as usize % ring].push(ni as u32);
                        queued += 1;
                    }
                }
            }
            if lv == u16::MAX {
                break;
            }
            lv += 1;
        }
        // Pack the pair. Level is the high half so the comparison is
        // lexicographic for free; the offset is what a real implementation
        // would have to fit in the low bits, so print what it would need.
        let max_offset = offset.iter().filter(|&&o| o != u16::MAX).max().copied().unwrap_or(0);
        let hier: Vec<u64> = (0..g.mat.len())
            .map(|i| {
                if !g.body[i] {
                    return u64::MAX;
                }
                match level[label[i] as usize] {
                    u32::MAX => u64::MAX,
                    l => ((l as u64) << 20) | offset[i] as u64,
                }
            })
            .collect();

        // **Which cells the load model would ever look at.**
        // `load::is_structurally_interesting` skips attached bulk with no
        // crack and no empty neighbour, and that is nearly the whole world
        // — so a whole-body disagreement figure is dominated by cells no
        // rule ever evaluates. Splitting it is the difference between "this
        // re-orients half the world" and "this re-orients half of the part
        // that matters"; they are not the same claim and only one of them
        // is about behaviour.
        let exposed: Vec<bool> = (0..g.mat.len())
            .map(|i| {
                if !g.body[i] {
                    return false;
                }
                let (x, y) = ((i % g.w) as i32, (i / g.w) as i32);
                N4.iter().any(|&(dx, dy)| {
                    let (nx, ny) = (x + dx, y + dy);
                    g.cracked(x, y, dx, dy) || (g.inside(nx, ny) && g.mat[g.idx(nx, ny)] == material::EMPTY.0)
                })
            })
            .collect();
        let exposed_n = exposed.iter().filter(|&&e| e).count();
        println!(
            "  of {body} body cells, {exposed_n} ({:.2}%) are exposed — the ones `is_structurally_interesting` lets through",
            100.0 * exposed_n as f64 / denom
        );

        // The comparison. `supports` is the four bits `load.rs` reads, so
        // agreement here is agreement about everything downstream of it.
        let compare = |field: &[u64], what: &str| {
            let (mut same, mut differ, mut lost, mut gained) = (0usize, 0usize, 0usize, 0usize);
            let mut count_moved = 0usize;
            let (mut exp_same, mut exp_differ) = (0usize, 0usize);
            // **Is the disagreement on the chunk grid?** `CLAUDE.md` names
            // chunk decomposition as a recurring root cause and says an
            // artifact lining up with the F1 grid is usually that rather
            // than the physics — and a hierarchical potential resets its
            // offset at every chunk boundary by construction, so this is
            // the artifact to look for before proposing one. Reported as a
            // *rate within each band*, never a count: the interior band
            // holds far more cells and would win on a count whatever the
            // truth is.
            let bands = [2i32, 8, 32];
            let (mut band_hit, mut band_all) = ([0usize; 3], [0usize; 3]);
            for y in 0..g.h as i32 {
                for x in 0..g.w as i32 {
                    let i = g.idx(x, y);
                    if !g.body[i] {
                        continue;
                    }
                    let a = supports(&g, x, y, &g.exact);
                    let b = supports(&g, x, y, field);
                    let edge = (x % CHUNK_SIZE)
                        .min(CHUNK_SIZE - 1 - x % CHUNK_SIZE)
                        .min(y % CHUNK_SIZE)
                        .min(CHUNK_SIZE - 1 - y % CHUNK_SIZE);
                    let band = bands.iter().position(|&b| edge < b).unwrap_or(2);
                    band_all[band] += 1;
                    if a == b {
                        same += 1;
                        if exposed[i] {
                            exp_same += 1;
                        }
                        continue;
                    }
                    differ += 1;
                    band_hit[band] += 1;
                    if exposed[i] {
                        exp_differ += 1;
                    }
                    if a.count_ones() != b.count_ones() {
                        count_moved += 1;
                    }
                    // The two asymmetric cases. "Lost" is the dangerous
                    // direction: a cell the exact field says is held and the
                    // candidate says is held by nothing reaches
                    // `is_supported`'s flood, and a region of them reads as
                    // a detached piece.
                    if a != 0 && b == 0 {
                        lost += 1;
                    }
                    if a == 0 && b != 0 {
                        gained += 1;
                    }
                }
            }
            println!(
                "  {what}: same {:.2}%  differ {:.2}%  (support_count moved {:.2}%)  held->unheld {lost}  unheld->held {gained}",
                100.0 * same as f64 / denom,
                100.0 * differ as f64 / denom,
                100.0 * count_moved as f64 / denom,
            );
            println!(
                "    ... exposed cells only: differ {:.2}% of {} | disagreement rate by distance to a chunk edge: 0-1 {:.2}%  2-7 {:.2}%  8+ {:.2}%",
                100.0 * exp_differ as f64 / (exp_same + exp_differ).max(1) as f64,
                exp_same + exp_differ,
                100.0 * band_hit[0] as f64 / band_all[0].max(1) as f64,
                100.0 * band_hit[1] as f64 / band_all[1].max(1) as f64,
                100.0 * band_hit[2] as f64 / band_all[2].max(1) as f64,
            );
        };
        println!(
            "  hierarchy: max level {max_level}, {orphan_nodes} unreachable nodes, max offset {max_offset} ({} bits) | built in {:.0} ms",
            (16 - max_offset.leading_zeros()).max(1),
            t.elapsed().as_secs_f64() * 1000.0
        );
        compare(&hier, "hierarchical vs exact");
        // Reachability has to agree too, and it is a separate claim from the
        // orientation: a coarse layer that says a piece is connected when
        // the cells are not has traded a slow wrong answer for a fast one.
        let (mut only_exact, mut only_hier) = (0usize, 0usize);
        for (i, &h) in hier.iter().enumerate() {
            if !g.body[i] {
                continue;
            }
            let e = g.exact[i] != u16::MAX;
            let hh = h != u64::MAX;
            if e && !hh {
                only_hier += 1;
            }
            if !e && hh {
                only_exact += 1;
            }
        }
        println!(
            "  reachability: exact says reachable & hierarchy does not {only_hier} | hierarchy says reachable & exact does not {only_exact}"
        );

        if control {
            // **The positive control.** A field that is flat zero everywhere
            // has no orientation at all, so every cell that had supports
            // must lose them. If this does not read ~100% differ, the
            // comparison above is comparing something with itself and its
            // agreement means nothing.
            let flat: Vec<u64> = vec![0; g.mat.len()];
            compare(&flat, "CONTROL flat-zero vs exact");
            // And the null control: the exact field against itself must be
            // exactly 100% same, which is what says the packing and the
            // generic comparison are not themselves introducing noise.
            let echo: Vec<u64> = g.exact.iter().map(|&d| d as u64).collect();
            compare(&echo, "CONTROL exact vs exact");
        }
    }
}
