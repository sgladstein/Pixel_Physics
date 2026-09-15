//! **What the 29.4 swept cells an ant dirties every frame are actually made
//! of** — the census round 36 asked for before anyone proposes a fix.
//!
//! [`Reports/evolution-lab-knee-2026-09-14.md`] §4 measured an ant at **51%
//! brain and 49% the hole it leaves in the CA sweep**, the second half being
//! **29.4 cells per ant per frame** inside awake chunks' expanded dirty
//! rects. An ant is a few cells; 29.4 is several times its own body, and
//! nothing in the tree said what the multiplier was. This harness answers
//! that and nothing else.
//!
//! **It publishes no timing at all, and that is deliberate.** Every number
//! here is a count taken off `World::chunks_to_sweep` and
//! `World::sweep_region` — the same two calls `parallel::step` uses to decide
//! what to walk — so there is no clock to be contended and no reps needed to
//! beat one. `antcost swept=1` is where the µs live; this is where the cells
//! live. The one thing to pin anyway is `RAYON_NUM_THREADS`, echoed in the
//! header, because `CLAUDE.md` is right that a counter downstream of
//! `parallel.rs`'s checkerboard is only load-independent at fixed
//! parallelism — and a cell-level count that moved with the thread count
//! would be a determinism failure, so `hash` is printed per arm to say it did
//! not.
//!
//! ## What it decomposes, and into what
//!
//! `swept` is a sum over awake chunks of one rect each:
//! `dirty.expanded_xy(reach, 1) ∩ bounds`. So a cell is swept for one of four
//! reasons, and the four are separated here rather than argued about:
//!
//! | class | why this chunk is awake | column |
//! |---|---|---|
//! | `ant` | it holds a cell an ant's body wrote last frame | `sw_ant` |
//! | `oth` | it holds a changed cell, none of them an ant's | `sw_oth` |
//! | `adj` | **nothing in it changed**; a write next door woke it through `World::touch_neighbours` | `sw_adj` |
//! | `stale` | nothing changed in it and nothing near it did | `sw_stale` |
//!
//! The awake set at frame `f` is classified against the cells frame `f-1`
//! actually changed, which is the causal pairing: a mark made during `f-1`'s
//! step is what `f`'s sweep is answering. `adj` reproduces
//! `touch_neighbours`' own arithmetic (`x ± MAX_REACH`, `y ± 1`, own chunk
//! excluded) rather than approximating it, so it is the mechanism's answer
//! and not a guess at it.
//!
//! Then a ladder of counterfactuals over the same changed set, each strictly
//! tighter than the one above, which prices the *shape* of the rect against
//! the *cells in it*:
//!
//! | column | rule |
//! |---|---|
//! | `bbox` | today's: one rect per chunk, `reach` sideways and one row up/down. **The control** — it must land near `swept` or the diff is not a faithful stand-in for what marked the chunks and nothing below it means anything ([`labperf`]'s own stated rule) |
//! | `rows` | the same neighbourhoods as one x-span per row instead of one rect per chunk |
//! | `rows1` | those spans with the horizontal expansion cut from `reach` to 1 |
//! | `cells` | the union of every changed cell's own 3x3 — **the floor**, what a rule that reconsidered only what can actually have changed would walk |
//!
//! `a_bbox` / `a_cells` are the top and bottom of that ladder over **only the
//! cells an ant's own body wrote**, which is the quantity a fix aimed at the
//! ant rather than at the sweep would have to move.
//!
//! ## The two controls it runs
//!
//! **Specificity.** The `ants=0` arm must report `sw_ant 0`, `ch_ant 0` and
//! `chg_ant 0`. A bed with no animal in it that attributes any swept cell to
//! an animal is measuring its own bookkeeping, and every share under it is
//! noise wearing a table. Printed as `CONTROL`.
//!
//! **Sensitivity.** `bbox/swept` is printed per arm. It is the only thing
//! that says the reconstruction is of the real dirty marks; a ladder whose
//! top rung does not reproduce today's rule ranks four rules nobody is
//! running.
//!
//! ## The pins, both of which have produced a wrong answer here before
//!
//! `founders=0` and `age=` are not conveniences. Round 32 measured a knee
//! that did not exist because bed age and the plant bill were both moving
//! under the ant axis; round 34 pinned them and the knee went away. Both
//! default to pinned here — no plants, and every arm ticked to a common
//! frame after stocking.
//!
//! ```text
//! cargo run --release --example antdirt
//! cargo run --release --example antdirt -- ants=0,80,240,420 frames=200 age=12000
//! RAYON_NUM_THREADS=1 cargo run --release --example antdirt -- ants=0,300 frames=120
//! ```

use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::Lab;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::{ChunkCoord, MAX_REACH};
use pixel_physics::sim::fxhash::ChunkMap;
use pixel_physics::sim::world::World;
use std::collections::{HashMap, HashSet};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// One changed cell: where, whether an ant's body was on either side of the
/// change, and whether the change was one that **marks the CA sweep at all**.
///
/// **Both sides of the ant test, because a move is two writes.** An ant
/// stepping from `p` to `q` leaves `p` empty and `q` occupied; reading only
/// the new cell charges half its footprint to `oth` and understates exactly
/// the term this harness exists to size.
struct Change {
    x: i32,
    y: i32,
    ant: bool,
    /// False for a **soil-moisture write**, which changes a cell and does
    /// *not* wake the sweep.
    ///
    /// **This is the correction that makes the ladder below mean anything,
    /// and `labperf`'s `est_` columns are known to fail their own control
    /// for want of it** (`chunk.rs`'s `row_spans_enabled`: "~90% soil
    /// moisture and — since moisture got its own dirty channel — do not wake
    /// the sweep at all, so they fail that instrument's own stated
    /// control"). Measured here before the correction: `bbox/swept`
    /// **2.33–4.39**, a reconstruction two to four times larger than the
    /// thing it reconstructs.
    ///
    /// The test is the shape of every moisture write in the engine:
    /// `World::set_soil_moisture` reaches the grid through
    /// `Chunk::set_world_quiet`, which marks nothing on the ordinary
    /// channel, and every caller of it in `update.rs` passes
    /// `cell.with_aux(..)` — so material and organism id are untouched and
    /// only `aux` moves. Requiring `water_capacity > 0` as well keeps a
    /// structural `aux` rewrite (solids) and a plant scalar out of it.
    marks: bool,
}

fn changed_cells(before: &[Cell], world: &World, w: i32, h: i32, is_creature: &[bool]) -> Vec<Change> {
    let mut out = Vec::new();
    let owned = |c: Cell| {
        let id = c.organism_id() as usize;
        id != 0 && is_creature.get(id).copied().unwrap_or(false)
    };
    for y in 0..h {
        for x in 0..w {
            let b = before[(y * w + x) as usize];
            let a = world.get(x, y);
            if b == a {
                continue;
            }
            let moisture_only = b.material == a.material
                && b.organism_id() == a.organism_id()
                && b.with_aux(0) == a.with_aux(0)
                && world.materials.get(a.material).water_capacity > 0;
            out.push(Change { x, y, ant: owned(b) || owned(a), marks: !moisture_only });
        }
    }
    out
}

fn snapshot(world: &World, w: i32, h: i32, into: &mut Vec<Cell>) {
    into.clear();
    for y in 0..h {
        for x in 0..w {
            into.push(world.get(x, y));
        }
    }
}

/// Which organism ids belong to something with a `creature` def, indexed by
/// id so the hot classification above is a `Vec` index rather than a lookup.
fn creature_ids(world: &World) -> Vec<bool> {
    let mut v = vec![false; u16::MAX as usize + 1];
    for id in world.live_organism_ids() {
        if let Some(st) = world.organism_state(id) {
            if world.species.get(st.species).creature.is_some() {
                v[id as usize] = true;
            }
        }
    }
    v
}

/// The rungs of the ladder, over whichever subset of the changed cells is
/// handed in.
///
/// Every rung uses the chunk's **own** `reach` except `rows1`, so the only
/// thing differing between them is the shape the marks are accumulated into —
/// one rect per chunk, one span per row, or the cells themselves. That is
/// what makes them a paired comparison of *rules* rather than several
/// measurements of several different worlds.
///
/// **`bbox_own` and `bbox` differ by one thing only: the neighbour marks.**
/// `World::write_cell` calls `touch_neighbours` on *every* write, which marks
/// the write's own coordinate in up to five neighbouring chunks; those marks
/// then get each neighbour's own `reach` expansion and are clipped to its
/// bounds. `bbox_own` leaves them out and `bbox` reproduces them, so the gap
/// between the two **is the adjacency tax priced in cells**, which is the
/// term round 36's brief named and nothing had measured.
#[derive(Default, Clone, Copy)]
struct Ladder {
    bbox_own: u64,
    bbox: u64,
    rows: u64,
    /// **The one rung that is both tighter than today's rule and exactly as
    /// conservative as it** — per-cell marks, each keeping its *full* `reach`.
    ///
    /// `rows` fills the gap between two marks that happen to share a chunk
    /// row: two marks 40 columns apart put all 40 columns between them in the
    /// set. This instead takes the union of each mark's own
    /// `[x - reach, x + reach]`, so a cell is in the set iff something within
    /// its own reach was written — which is the contract `sweep_region`'s doc
    /// already states. Every safety property `parallel.rs` rests on is stated
    /// against the bounding *box*, which this does not change.
    ///
    /// It is the number that says whether narrowing the region is worth
    /// anything at all: if the marks are dense the unions merge and it reads
    /// the same as `rows`.
    cellreach: u64,
    /// **`cellreach` with each mark's reach derived from its own
    /// neighbourhood instead of from its chunk's maximum** — the price of the
    /// one fix this census points at.
    ///
    /// `Chunk::recompute_reach` takes the *max* `sweep_reach` over all 4,096
    /// cells of a chunk, so one stray cell of a far-reaching material widens
    /// the sweep around everything else in it. Measured on this bed: **water
    /// sets the reach of 59 of 101 awake chunks to 24, and those chunks hold
    /// on average 1.5 water cells.** An ant's body write in dry soil is then
    /// expanded by ±24 where ±2 is all the soil around it can do.
    ///
    /// **Exactly as conservative as today's rule, not merely tighter.** A
    /// cell `q` can move into mark `p` only if `|q.x - p.x| <=
    /// sweep_reach(q)`, so the max of `sweep_reach(q)` over the `q` that
    /// satisfy that is a superset of every cell that can reach `p` — which is
    /// the contract `Chunk::sweep_region`'s own doc states. The chunk maximum
    /// is a superset of *that*, taken because `recompute_reach` has one
    /// number per chunk and not one per mark.
    cellloc: u64,
    /// **Today's rule with the reach taken over the rows the sweep will
    /// actually walk, instead of over all 64 rows of the chunk** — the
    /// candidate that keeps the single-rect shape.
    ///
    /// The rules only ever look **one row** up or down, so a cell in a row
    /// the region does not contain cannot be examined and its reach cannot
    /// matter. Taking the max over `dirty.min_y - 1 ..= dirty.max_y + 1`
    /// instead of the whole chunk is therefore conservative by the same
    /// argument today's rule is: every cell the sweep examines lies in those
    /// rows, and this bounds the reach of all of them.
    ///
    /// **It is the version worth building first**, because it changes no
    /// shape — one rect per chunk, exactly as now — so it does not inherit
    /// §E2, which is a divergence of the per-row *span* shape. A per-row
    /// `[u8; CHUNK_SIZE]` grown in `set_world` beside `self.reach` and
    /// refreshed in `recompute_reach` computes it for nothing.
    bbox_rowband: u64,
    rows1: u64,
    cells: u64,
}

fn ladder(changed: impl Iterator<Item = (i32, i32)>, world: &World, w: i32, h: i32) -> Ladder {
    // Per chunk: the bounding box of the marks in it, with and without the
    // marks `touch_neighbours` puts there from next door. World coordinates
    // and deliberately unclipped, exactly as `Chunk::mark_dirty` stores them
    // -- `sweep_region` is what clips, and it clips *after* expanding.
    let mut own: ChunkMap<(i32, i32, i32, i32)> = ChunkMap::default();
    let mut all: ChunkMap<(i32, i32, i32, i32)> = ChunkMap::default();
    let mut spans: HashMap<(ChunkCoord, i32), (i32, i32)> = HashMap::new();
    // The same keys as `spans`, holding every mark's x rather than their
    // hull, which is the whole difference between `rows` and `cellreach`.
    let mut marks: HashMap<(ChunkCoord, i32), Vec<i32>> = HashMap::new();
    let mut cells: HashSet<(i32, i32)> = HashSet::new();
    let mut nbrs: HashSet<ChunkCoord> = HashSet::new();
    let grow = |m: &mut ChunkMap<(i32, i32, i32, i32)>, c: ChunkCoord, x: i32, y: i32| {
        m.entry(c)
            .and_modify(|b| {
                b.0 = b.0.min(x);
                b.1 = b.1.min(y);
                b.2 = b.2.max(x);
                b.3 = b.3.max(y);
            })
            .or_insert((x, y, x, y));
    };
    for (x, y) in changed {
        let c = ChunkCoord::containing(x, y);
        grow(&mut own, c, x, y);
        grow(&mut all, c, x, y);
        nbrs.clear();
        wakes(x, y, &mut nbrs);
        for &n in &nbrs {
            // Only a resident chunk takes a mark -- `touch_neighbours` skips
            // the rest, and inventing one would inflate this rung above the
            // rule it is reproducing.
            if world.chunk_reach(n).is_some() {
                grow(&mut all, n, x, y);
                for row in (y - 1)..=(y + 1) {
                    spans
                        .entry((n, row))
                        .and_modify(|s| {
                            s.0 = s.0.min(x);
                            s.1 = s.1.max(x);
                        })
                        .or_insert((x, x));
                    marks.entry((n, row)).or_default().push(x);
                }
            }
        }
        // A changed cell constrains the row either side of it as well as its
        // own -- the vertical half of today's `expanded_xy(reach, 1)`.
        for row in (y - 1)..=(y + 1) {
            spans
                .entry((c, row))
                .and_modify(|s| {
                    s.0 = s.0.min(x);
                    s.1 = s.1.max(x);
                })
                .or_insert((x, x));
            marks.entry((c, row)).or_default().push(x);
            for col in (x - 1)..=(x + 1) {
                if (0..w).contains(&col) && (0..h).contains(&row) {
                    cells.insert((col, row));
                }
            }
        }
    }

    let mut out = Ladder { cells: cells.len() as u64, ..Ladder::default() };
    // `bbox_rowband`: the same rect, with the reach taken over the rows the
    // region occupies rather than over the whole chunk.
    for (c, b) in &all {
        let bounds = c.bounds();
        let mut reach = 1;
        for y in (b.1 - 1).max(bounds.min_y)..=(b.3 + 1).min(bounds.max_y) {
            if y < 0 || y >= h {
                continue;
            }
            for x in bounds.min_x..=bounds.max_x {
                if x < 0 || x >= w {
                    continue;
                }
                reach = reach.max(world.materials.get(world.get(x, y).material).sweep_reach());
            }
        }
        let min_x = (b.0 - reach).max(bounds.min_x).max(0);
        let max_x = (b.2 + reach).min(bounds.max_x).min(w - 1);
        let min_y = (b.1 - 1).max(bounds.min_y).max(0);
        let max_y = (b.3 + 1).min(bounds.max_y).min(h - 1);
        if max_x >= min_x && max_y >= min_y {
            out.bbox_rowband += ((max_x - min_x + 1) as i64 * (max_y - min_y + 1) as i64) as u64;
        }
    }

    for (dst, boxes) in [(&mut out.bbox_own, &own), (&mut out.bbox, &all)] {
        for (c, b) in boxes {
            let reach = world.chunk_reach(*c).unwrap_or(1);
            let bounds = c.bounds();
            let min_x = (b.0 - reach).max(bounds.min_x).max(0);
            let max_x = (b.2 + reach).min(bounds.max_x).min(w - 1);
            let min_y = (b.1 - 1).max(bounds.min_y).max(0);
            let max_y = (b.3 + 1).min(bounds.max_y).min(h - 1);
            if max_x >= min_x && max_y >= min_y {
                *dst += ((max_x - min_x + 1) as i64 * (max_y - min_y + 1) as i64) as u64;
            }
        }
    }
    for ((c, row), sp) in &spans {
        let bounds = c.bounds();
        if *row < bounds.min_y || *row > bounds.max_y || *row < 0 || *row >= h {
            continue;
        }
        let reach = world.chunk_reach(*c).unwrap_or(1);
        for (dst, r) in [(&mut out.rows, reach), (&mut out.rows1, 1)] {
            let min_x = (sp.0 - r).max(bounds.min_x).max(0);
            let max_x = (sp.1 + r).min(bounds.max_x).min(w - 1);
            if max_x >= min_x {
                *dst += (max_x - min_x + 1) as u64;
            }
        }
    }
    // `cellloc`: the same union, with each mark's interval sized by what can
    // actually reach it. One scan of `2 * MAX_REACH + 1` columns over three
    // rows per mark -- expensive, and the reason this harness samples nothing
    // else per cell.
    for ((c, row), xs) in marks.iter() {
        let bounds = c.bounds();
        if *row < bounds.min_y || *row > bounds.max_y || *row < 0 || *row >= h {
            continue;
        }
        let mut iv: Vec<(i32, i32)> = Vec::with_capacity(xs.len());
        for &x in xs.iter() {
            let mut r = 1;
            for qy in (row - 1).max(0)..=(row + 1).min(h - 1) {
                for qx in (x - MAX_REACH).max(0)..=(x + MAX_REACH).min(w - 1) {
                    let qr = world.materials.get(world.get(qx, qy).material).sweep_reach();
                    if (qx - x).abs() <= qr {
                        r = r.max(qr);
                    }
                }
            }
            let (a, b) = ((x - r).max(bounds.min_x).max(0), (x + r).min(bounds.max_x).min(w - 1));
            if b >= a {
                iv.push((a, b));
            }
        }
        iv.sort_unstable();
        let (mut lo, mut hi) = (i32::MIN, i32::MIN);
        for (a, b) in iv {
            if hi == i32::MIN {
                (lo, hi) = (a, b);
            } else if a <= hi + 1 {
                hi = hi.max(b);
            } else {
                out.cellloc += (hi - lo + 1) as u64;
                (lo, hi) = (a, b);
            }
        }
        if hi != i32::MIN {
            out.cellloc += (hi - lo + 1) as u64;
        }
    }

    // `cellreach`: per row, the union of every mark's own reach interval,
    // merged. Sorted first so one pass merges them -- `sort_unstable` is
    // safe here because the values are plain `i32` with no tie to break.
    for ((c, row), xs) in marks.iter_mut() {
        let bounds = c.bounds();
        if *row < bounds.min_y || *row > bounds.max_y || *row < 0 || *row >= h {
            continue;
        }
        let reach = world.chunk_reach(*c).unwrap_or(1);
        xs.sort_unstable();
        let (mut lo, mut hi) = (i32::MIN, i32::MIN);
        for &x in xs.iter() {
            let (a, b) = ((x - reach).max(bounds.min_x).max(0), (x + reach).min(bounds.max_x).min(w - 1));
            if b < a {
                continue;
            }
            if hi == i32::MIN {
                (lo, hi) = (a, b);
            } else if a <= hi + 1 {
                hi = hi.max(b);
            } else {
                out.cellreach += (hi - lo + 1) as u64;
                (lo, hi) = (a, b);
            }
        }
        if hi != i32::MIN {
            out.cellreach += (hi - lo + 1) as u64;
        }
    }

    out
}

/// The chunks `World::touch_neighbours` would wake for a write at `(x, y)` —
/// its own arithmetic, not an approximation of it.
///
/// **Its interior-skip guard is a no-op at today's constants and this
/// reproduces that faithfully.** `MAX_REACH` (32) is exactly `CHUNK_SIZE / 2`
/// (64), so the range it tests is `32..32`, empty, and `contains` is always
/// false — *every* write in the world runs the double loop below. That fact
/// is documented at the call site in `world.rs` and is the single largest
/// thing this census turns up, so it is reproduced here rather than
/// shortcut.
fn wakes(x: i32, y: i32, into: &mut HashSet<ChunkCoord>) {
    let owner = ChunkCoord::containing(x, y);
    let first = ChunkCoord::containing(x - MAX_REACH, y - 1);
    let last = ChunkCoord::containing(x + MAX_REACH, y + 1);
    for cy in first.y..=last.y {
        for cx in first.x..=last.x {
            let c = ChunkCoord::new(cx, cy);
            if c != owner {
                into.insert(c);
            }
        }
    }
}

#[derive(Default)]
struct Tally {
    frames: u64,
    awake: u64,
    swept: u64,
    reach_sum: u64,
    changed: u64,
    changed_ant: u64,
    /// Of `changed`, the ones that do not wake the sweep — see
    /// [`Change::marks`]. Printed rather than merely subtracted, because it
    /// is most of the diff on this bed and a reader who does not know that
    /// will read `chg/f` as the sweep's input.
    changed_moist: u64,
    /// Awake chunks and their swept area, by why they are awake.
    ch: [u64; 4],
    sw: [u64; 4],
    all: Ladder,
    ant: Ladder,
    /// **Which material is setting each awake chunk's `reach`, and how few
    /// cells of it there are.**
    ///
    /// `reach` is the *maximum* `Material::sweep_reach` over all 4,096 cells
    /// of a chunk (`Chunk::recompute_reach`), and every mark in that chunk is
    /// then expanded by it. So one cell of a far-reaching material anywhere in
    /// a chunk widens the sweep around an ant sixty cells away from it. This
    /// census says whether that is what is happening: keyed by material name,
    /// `(times it set an awake chunk's reach, summed count of cells that
    /// actually had that reach, summed chunk reach)`.
    ///
    /// Sampled every `reachevery` frames rather than every frame, because it
    /// is a full scan of every awake chunk and the answer moves slowly.
    reach_by: HashMap<String, (u64, u64, u64)>,
    reach_samples: u64,
    /// **Frames on which the ladder came out of order** — the second control.
    ///
    /// Every rung is a strict narrowing of the one above it by construction,
    /// so `cells <= cellloc <= cellreach <= rows <= bbox` must hold on every
    /// frame. A ladder whose rungs cross is not a ranking of four rules, it is
    /// an arithmetic bug wearing one, and nothing else here would notice: each
    /// rung on its own is a plausible number. Checked per frame on the levels
    /// rather than on the fitted slopes, because two slopes can legitimately
    /// cross where the levels do not.
    ladder_out_of_order: u64,
}

const CH_ANT: usize = 0;
const CH_OTH: usize = 1;
const CH_ADJ: usize = 2;
const CH_STALE: usize = 3;

struct Arm {
    want: usize,
    lab: Lab,
    stocked: usize,
    ants: f64,
    t: Tally,
    hash: u64,
}

fn world_hash(w: &World) -> u64 {
    fn fnv1a(h: u64, v: u64) -> u64 {
        (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
    }
    let b = w.bounds().expect("the lab box sets bounds");
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            h = fnv1a(h, c.material.0 as u64);
            h = fnv1a(h, c.aux() as u64);
            h = fnv1a(h, c.organism_id() as u64);
        }
    }
    h
}

/// The same stocking loop `antcost` uses, for the same reason: reaching a
/// population by breeding takes hundreds of thousands of frames, so the bed
/// is founded in rounds with dispersal frames between them. Kept identical so
/// the two harnesses are measuring one bed.
fn stock(lab: &mut Lab, species: &str, want: usize, ground_y: i32, width: i32, rounds: usize, settle: u64) -> usize {
    if want == 0 {
        return 0;
    }
    let cols: Vec<i32> = (1..=4).map(|i| width * i / 5).collect();
    for _ in 0..rounds {
        if lab.world.live_creature_count() >= want {
            break;
        }
        for &cx in &cols {
            if lab.world.live_creature_count() >= want {
                break;
            }
            lab.world.found_colony_of(cx, ground_y - 2, species, 16);
        }
        for _ in 0..settle {
            lab.tick_for_harness();
        }
    }
    lab.world.live_creature_count()
}

/// Least squares over `(x, y)`, returning `(intercept, slope)`. The slope is
/// the per-ant figure every headline here is quoted as, because the absolute
/// count is dominated by a background an ant did not make.
fn fit(pts: &[(f64, f64)]) -> (f64, f64) {
    let n = pts.len() as f64;
    if n < 2.0 {
        return (f64::NAN, f64::NAN);
    }
    let sx: f64 = pts.iter().map(|p| p.0).sum();
    let sy: f64 = pts.iter().map(|p| p.1).sum();
    let sxx: f64 = pts.iter().map(|p| p.0 * p.0).sum();
    let sxy: f64 = pts.iter().map(|p| p.0 * p.1).sum();
    let d = n * sxx - sx * sx;
    if d.abs() < 1e-9 {
        return (f64::NAN, f64::NAN);
    }
    ((sy * sxx - sx * sxy) / d, (n * sxy - sx * sy) / d)
}

fn main() {
    let d = LabBox::default();
    let ants_arg: String = arg("ants").unwrap_or_else(|| "0,80,240,420".to_string());
    let wants: Vec<usize> = ants_arg.split(',').map(|s| s.parse().expect("an ant count")).collect();
    let width: i32 = arg("width").unwrap_or(512);
    let height: i32 = arg("height").unwrap_or(512);
    let seed: u64 = arg("seed").unwrap_or(d.seed);
    let soil: i32 = arg("soil").unwrap_or(d.soil_depth);
    // **Pinned to nothing growing, and this is the round-34 pin.** With
    // plants in the bed the ants eat them, so the arms with more ants have
    // fewer plants and the ant term is measured against a moving
    // subtraction.
    let founders: usize = arg("founders").unwrap_or(0);
    let colony_species: String = arg("colony_species").unwrap_or_else(|| "longant".to_string());
    let frames: u64 = arg("frames").unwrap_or(200);
    let grow: u64 = arg("grow").unwrap_or(2_000);
    let rounds: usize = arg("rounds").unwrap_or(200);
    let settle: u64 = arg("settle").unwrap_or(40);
    // **The other round-34 pin, and it is derived rather than guessed.**
    // Every arm must be ticked to a *common* frame after stocking, or bed age
    // is a function of ant count and the difference is charged to the ants
    // because they are the x-axis. But the number has to sit just *above* the
    // oldest arm's post-stocking frame and not above that, and a hardcoded
    // default cannot know where that is: `founders=0` is a bed with no food
    // in it, so every frame past stocking is a frame the population starves.
    // Measured 2026-09-14 with `age=14000` against arms stocked at frames
    // 2,000–5,120: **stocked 0 / 80 / 241 / 420, standing 0 / 0 / 5 / 15** —
    // four arms wearing four labels and holding one population, which is a
    // harness that has silently deleted its own x-axis.
    //
    // So `age=0`, the default, stocks every arm first and then ages them all
    // to the latest frame any of them reached. `age=N` overrides it, and a
    // run that sets it below an arm's own stocking frame is told so.
    let age: u64 = arg("age").unwrap_or(0);
    // 0 turns the reach census off. It is a full scan of every awake chunk,
    // so it is sampled rather than run every frame.
    let reachevery: u64 = arg("reachevery").unwrap_or(20);
    let threads = std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "unset".to_string());

    println!(
        "antdirt: ants={wants:?} frames={frames} width={width} height={height} seed={seed} soil={soil} \
         founders={founders} colony_species={colony_species} grow={grow} rounds={rounds} settle={settle} \
         age={age} RAYON_NUM_THREADS={threads}"
    );
    println!("  no timings in this harness -- every column is a count off chunks_to_sweep/sweep_region");

    let mut arms: Vec<Arm> = Vec::new();
    for &want in &wants {
        let spec = LabBox {
            founders,
            width,
            height,
            soil_depth: soil,
            colonies: 0,
            colony_species: colony_species.clone(),
            seed,
            ..d.clone()
        };
        let ground_y = spec.ground_y;
        let mut lab = Lab::new(spec);
        for _ in 0..grow {
            lab.tick_for_harness();
        }
        let stocked = stock(&mut lab, &colony_species, want, ground_y, width, rounds, settle);
        println!(
            "  stocking: want {want:>5} -> standing {:>5} ants at frame {}",
            lab.world.live_creature_count(),
            lab.world.frame
        );
        arms.push(Arm { want, lab, stocked, ants: 0.0, t: Tally::default(), hash: 0 });
    }

    // See `age` above: the common frame is the latest any arm reached, so the
    // oldest arm is not aged at all and the youngest catches up to it.
    let common = age.max(arms.iter().map(|a| a.lab.world.frame).max().unwrap_or(0));
    if age > 0 && age < arms.iter().map(|a| a.lab.world.frame).max().unwrap_or(0) {
        println!("  age={age} is below an arm's own stocking frame and cannot pin anything; using {common}");
    }
    println!("  age pin: every arm ticked to frame {common}");
    for arm in arms.iter_mut() {
        while arm.lab.world.frame < common {
            arm.lab.tick_for_harness();
        }
        println!(
            "  aged:     want {:>5} -> standing {:>5} ants at frame {}",
            arm.want,
            arm.lab.world.live_creature_count(),
            arm.lab.world.frame
        );
    }

    let cells = (width * height) as usize;
    let mut before: Vec<Cell> = Vec::with_capacity(cells);
    for arm in arms.iter_mut() {
        let ants_before = arm.lab.world.live_creature_count();
        // The previous frame's changed cells, which is what this frame's
        // awake set is answering.
        let mut prev: Vec<Change> = Vec::new();
        for f in 0..frames {
            let active = arm.lab.world.chunks_to_sweep();
            if f > 0 {
                // Chunks holding a changed cell, and whether any of them was
                // an ant's. Built from `prev` rather than from the awake set
                // so a chunk that changed and then went to sleep is still
                // accounted.
                let mut inside: ChunkMap<bool> = ChunkMap::default();
                let mut woken: HashSet<ChunkCoord> = HashSet::new();
                // **Only the changes that mark.** A moisture write changed a
                // cell and woke nothing, so counting it here would attribute
                // an awake chunk to a channel that cannot wake one — and on
                // this bed it is most of the diff.
                for c in prev.iter().filter(|c| c.marks) {
                    inside
                        .entry(ChunkCoord::containing(c.x, c.y))
                        .and_modify(|a| *a |= c.ant)
                        .or_insert(c.ant);
                    wakes(c.x, c.y, &mut woken);
                }
                for c in &active {
                    let area = arm.lab.world.sweep_region(*c).map_or(0, |r| {
                        ((r.max_x - r.min_x + 1) as i64 * (r.max_y - r.min_y + 1) as i64) as u64
                    });
                    let class = match inside.get(c) {
                        Some(true) => CH_ANT,
                        Some(false) => CH_OTH,
                        None if woken.contains(c) => CH_ADJ,
                        None => CH_STALE,
                    };
                    arm.t.ch[class] += 1;
                    arm.t.sw[class] += area;
                }
                let l = ladder(
                    prev.iter().filter(|c| c.marks).map(|c| (c.x, c.y)),
                    &arm.lab.world,
                    width,
                    height,
                );
                arm.t.all.bbox_own += l.bbox_own;
                arm.t.all.bbox += l.bbox;
                arm.t.all.rows += l.rows;
                arm.t.all.cellreach += l.cellreach;
                arm.t.all.cellloc += l.cellloc;
                arm.t.all.bbox_rowband += l.bbox_rowband;
                if !(l.cells <= l.cellloc && l.cellloc <= l.cellreach && l.cellreach <= l.rows && l.rows <= l.bbox && l.bbox_rowband <= l.bbox) {
                    arm.t.ladder_out_of_order += 1;
                }
                arm.t.all.rows1 += l.rows1;
                arm.t.all.cells += l.cells;
                let a = ladder(
                    prev.iter().filter(|c| c.marks && c.ant).map(|c| (c.x, c.y)),
                    &arm.lab.world,
                    width,
                    height,
                );
                arm.t.ant.bbox_own += a.bbox_own;
                arm.t.ant.bbox += a.bbox;
                arm.t.ant.rows += a.rows;
                arm.t.ant.cellreach += a.cellreach;
                arm.t.ant.cellloc += a.cellloc;
                arm.t.ant.bbox_rowband += a.bbox_rowband;
                arm.t.ant.rows1 += a.rows1;
                arm.t.ant.cells += a.cells;
            }
            arm.t.awake += active.len() as u64;
            for c in &active {
                arm.t.swept += arm.lab.world.sweep_region(*c).map_or(0, |r| {
                    ((r.max_x - r.min_x + 1) as i64 * (r.max_y - r.min_y + 1) as i64) as u64
                });
                arm.t.reach_sum += arm.lab.world.chunk_reach(*c).unwrap_or(0) as u64;
            }
            arm.t.frames += 1;
            if reachevery > 0 && f.is_multiple_of(reachevery) {
                arm.t.reach_samples += 1;
                for c in &active {
                    let b = c.bounds();
                    let mut best = (0i32, String::new(), 0u64);
                    for y in b.min_y..=b.max_y {
                        for x in b.min_x..=b.max_x {
                            let m = arm.lab.world.materials.get(arm.lab.world.get(x, y).material);
                            let r = m.sweep_reach();
                            if r > best.0 {
                                best = (r, m.name.clone(), 1);
                            } else if r == best.0 && r > 0 {
                                best.2 += 1;
                            }
                        }
                    }
                    let e = arm.t.reach_by.entry(best.1).or_insert((0, 0, 0));
                    e.0 += 1;
                    e.1 += best.2;
                    e.2 += best.0 as u64;
                }
            }

            let is_creature = creature_ids(&arm.lab.world);
            snapshot(&arm.lab.world, width, height, &mut before);
            arm.lab.tick_for_harness();
            prev = changed_cells(&before, &arm.lab.world, width, height, &is_creature);
            arm.t.changed += prev.len() as u64;
            arm.t.changed_ant += prev.iter().filter(|c| c.ant).count() as u64;
            arm.t.changed_moist += prev.iter().filter(|c| !c.marks).count() as u64;
        }
        arm.ants = (ants_before + arm.lab.world.live_creature_count()) as f64 / 2.0;
        arm.hash = world_hash(&arm.lab.world);
        eprintln!("  arm want={} done", arm.want);
    }

    // Per frame, so every row is directly comparable and the slope below is
    // per ant per frame -- the unit §4's 29.4 is quoted in.
    println!(
        "\n{:>6} {:>7} {:>8} {:>9} {:>6} {:>8} {:>9} {:>9} {:>7} {:>8} {:>7} {:>8} {:>7} {:>8} {:>8} {:>18}",
        "want", "ants", "awake/f", "swept/f", "reach", "chg/f", "chgmoi/f", "chgant/f", "ch_ant", "sw_ant", "ch_adj", "sw_adj", "ch_oth", "sw_oth", "ch_stal", "hash"
    );
    for arm in &arms {
        let f = arm.t.frames as f64;
        let g = (arm.t.frames - 1) as f64;
        println!(
            "{:>6} {:>7.1} {:>8.0} {:>9.0} {:>6.1} {:>8.0} {:>9.0} {:>9.1} {:>7.1} {:>8.0} {:>7.1} {:>8.0} {:>7.1} {:>8.0} {:>8.2} {:>18x}",
            arm.want,
            arm.ants,
            arm.t.awake as f64 / f,
            arm.t.swept as f64 / f,
            arm.t.reach_sum as f64 / arm.t.awake.max(1) as f64,
            arm.t.changed as f64 / f,
            arm.t.changed_moist as f64 / f,
            arm.t.changed_ant as f64 / f,
            arm.t.ch[CH_ANT] as f64 / g,
            arm.t.sw[CH_ANT] as f64 / g,
            arm.t.ch[CH_ADJ] as f64 / g,
            arm.t.sw[CH_ADJ] as f64 / g,
            arm.t.ch[CH_OTH] as f64 / g,
            arm.t.sw[CH_OTH] as f64 / g,
            arm.t.ch[CH_STALE] as f64 / g,
            arm.hash
        );
    }
    println!("  stocked: {:?}", arms.iter().map(|a| a.stocked).collect::<Vec<_>>());

    println!(
        "\n{:>6} {:>7} {:>9} {:>9} {:>9} {:>9} {:>7} {:>9} {:>9} {:>10} {:>9} {:>9} {:>10} {:>9} {:>9}",
        "want", "ants", "swept/f", "bbox_own", "bbox", "bbox/swp", "nbr x", "rowband", "rows", "cellreach", "cellloc", "rows1", "cells", "a_bbox", "a_cells"
    );
    for arm in &arms {
        let f = arm.t.frames as f64;
        let g = (arm.t.frames - 1) as f64;
        let swept = arm.t.swept as f64 / f;
        let bbox = arm.t.all.bbox as f64 / g;
        println!(
            "{:>6} {:>7.1} {:>9.0} {:>9.0} {:>9.0} {:>9.2} {:>7.2} {:>9.0} {:>9.0} {:>10.0} {:>9.0} {:>9.0} {:>10.0} {:>9.0} {:>9.0}",
            arm.want,
            arm.ants,
            swept,
            arm.t.all.bbox_own as f64 / g,
            bbox,
            bbox / swept,
            bbox / (arm.t.all.bbox_own as f64 / g).max(1.0),
            arm.t.all.bbox_rowband as f64 / g,
            arm.t.all.rows as f64 / g,
            arm.t.all.cellreach as f64 / g,
            arm.t.all.cellloc as f64 / g,
            arm.t.all.rows1 as f64 / g,
            arm.t.all.cells as f64 / g,
            arm.t.ant.bbox as f64 / g,
            arm.t.ant.cells as f64 / g,
        );
    }
    println!(
        "  bbox/swp is THE CONTROL on this whole table: bbox is today's rule recomputed from the diff,\n  \
         so it must land near swept or the reconstruction is not of the real dirty marks."
    );

    // **The slope is the quantity, not the level.** An empty bed already
    // sweeps thousands of cells a frame, so every absolute above is mostly a
    // background no ant made. Fitted over the arms with ants in them as well
    // as over all of them, because knee §1b says the `ants=0` arm is not a
    // valid background to subtract -- its soil water was never disturbed by a
    // founding.
    println!("\nper ant per frame (least squares over arms)");
    println!("{:>12} {:>12} {:>12} {:>12}", "column", "all arms", "ants>0", "share of swept");
    let swept_all = fit(&arms.iter().map(|a| (a.ants, a.t.swept as f64 / a.t.frames as f64)).collect::<Vec<_>>()).1;
    let row = |name: &str, v: &dyn Fn(&Arm) -> f64| {
        let all = fit(&arms.iter().map(|a| (a.ants, v(a))).collect::<Vec<_>>()).1;
        let pos = fit(&arms.iter().filter(|a| a.ants > 0.5).map(|a| (a.ants, v(a))).collect::<Vec<_>>()).1;
        println!("{name:>12} {all:>12.2} {pos:>12.2} {:>12.1}%", 100.0 * all / swept_all);
    };
    let f = |a: &Arm| a.t.frames as f64;
    let g = |a: &Arm| (a.t.frames - 1) as f64;
    row("swept", &|a| a.t.swept as f64 / f(a));
    row("sw_ant", &|a| a.t.sw[CH_ANT] as f64 / g(a));
    row("sw_adj", &|a| a.t.sw[CH_ADJ] as f64 / g(a));
    row("sw_oth", &|a| a.t.sw[CH_OTH] as f64 / g(a));
    row("sw_stale", &|a| a.t.sw[CH_STALE] as f64 / g(a));
    row("bbox_own", &|a| a.t.all.bbox_own as f64 / g(a));
    row("bbox", &|a| a.t.all.bbox as f64 / g(a));
    row("bbox_rowband", &|a| a.t.all.bbox_rowband as f64 / g(a));
    row("rows", &|a| a.t.all.rows as f64 / g(a));
    row("cellreach", &|a| a.t.all.cellreach as f64 / g(a));
    row("cellloc", &|a| a.t.all.cellloc as f64 / g(a));
    row("rows1", &|a| a.t.all.rows1 as f64 / g(a));
    row("cells", &|a| a.t.all.cells as f64 / g(a));
    row("a_bbox_own", &|a| a.t.ant.bbox_own as f64 / g(a));
    row("a_bbox", &|a| a.t.ant.bbox as f64 / g(a));
    row("a_cellreach", &|a| a.t.ant.cellreach as f64 / g(a));
    row("a_cellloc", &|a| a.t.ant.cellloc as f64 / g(a));
    row("a_cells", &|a| a.t.ant.cells as f64 / g(a));
    row("changed", &|a| a.t.changed as f64 / f(a));
    row("chg_marks", &|a| (a.t.changed - a.t.changed_moist) as f64 / f(a));
    row("chg_moist", &|a| a.t.changed_moist as f64 / f(a));
    row("chg_ant", &|a| a.t.changed_ant as f64 / f(a));
    row("awake", &|a| a.t.awake as f64 / f(a));

    // **What is setting the reach every mark in a chunk gets expanded by.**
    println!("\nwhat sets an awake chunk's reach (sampled every {reachevery} frames)");
    println!("{:>6} {:>12} {:>10} {:>10} {:>12}", "want", "material", "chunks", "mean reach", "cells w/ it");
    for arm in &arms {
        let mut rows: Vec<(&String, &(u64, u64, u64))> = arm.t.reach_by.iter().collect();
        rows.sort_by_key(|(_, v)| std::cmp::Reverse(v.0));
        for (name, v) in rows {
            println!(
                "{:>6} {:>12} {:>10} {:>10.1} {:>12.1}",
                arm.want,
                name,
                v.0,
                v.2 as f64 / v.0 as f64,
                v.1 as f64 / v.0 as f64
            );
        }
    }
    println!(
        "  `cells w/ it` is how many cells of a 4,096-cell chunk actually carry that reach.\n           A small number means one cell is widening the sweep around everything else in the chunk."
    );

    let crossed: u64 = arms.iter().map(|a| a.t.ladder_out_of_order).sum();
    println!(
        "\nCONTROL ladder order (cells <= cellloc <= cellreach <= rows <= bbox, per frame): {} -> {}",
        crossed,
        if crossed == 0 { "PASS" } else { "FAIL -- the rungs cross, so the table is not a ranking of rules" }
    );

    // **The specificity control, and it has to read zero.** A bed with no
    // animal in it attributing swept cells to an animal is measuring its own
    // bookkeeping.
    if let Some(z) = arms.iter().find(|a| a.want == 0) {
        let bad = z.t.sw[CH_ANT] + z.t.ch[CH_ANT] + z.t.changed_ant;
        println!(
            "\nCONTROL ants=0: sw_ant {} ch_ant {} chg_ant {} -> {}",
            z.t.sw[CH_ANT],
            z.t.ch[CH_ANT],
            z.t.changed_ant,
            if bad == 0 { "PASS" } else { "FAIL -- every ant share above is suspect" }
        );
    } else {
        println!("\nCONTROL: no ants=0 arm in this run, so nothing checked the attribution");
    }
}
