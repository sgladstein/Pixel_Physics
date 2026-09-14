//! **Which cell does a narrower sweep region get wrong, and why?** — §E2
//! bisected to a *cell* instead of to a frame.
//!
//! ## Why this exists
//!
//! `Reports/evolution-lab-ant-dirty-cells-2026-09-14.md` measured that the CA
//! sweep is asked for **4.8–5.7x** the cells a per-mark region would need, and
//! that the whole prize needs the region's *shape* to change: one rect per
//! chunk is already clipped to the full chunk width at the reaches this engine
//! runs at, so narrowing the reach alone is arithmetically incapable of
//! helping (measured, **1.9%**).
//!
//! In front of that sits `Reports/open-bugs-handoff.md` **§E2**:
//! `PIXEL_PHYSICS_SWEEP=rows` is a strictly narrower region and it
//! **diverges** — identical through frame 4,329 and first differing at 4,330
//! on the standard lab bed, with the RNG-stream coupling ruled out as the only
//! cause and `chunk.rs` recording that "the spans are *not* a superset of
//! every cell the rules can act on ... and the second [coupling] is
//! unidentified".
//!
//! **It could be bisected to a frame and never to a cell, and the reason is
//! mechanical: the switch was a process-wide `OnceLock`.** Two settings meant
//! two processes, and two processes of a chaotic simulation are two different
//! worlds — so the only available comparison was a world hash, which says
//! *that* they differ and never *which cell*. `Chunk::sweep_rows` is now a
//! per-chunk field with `World::set_sweep_rows` beside it (shipped default
//! unchanged, straight from the same environment variable), which is what
//! lets this harness hold **both arms in one process** as `CLAUDE.md`
//! requires.
//!
//! ## What it does
//!
//! Two `Lab`s, same spec, same seed, stepped in lockstep. One runs the box
//! rule, one runs the per-row-span rule. Every frame it compares the two
//! worlds cell by cell and, at the **first** frame they differ, prints every
//! differing cell: where it is, what each arm has there, which chunk, that
//! chunk's `reach`, and **how far the cell sat from the nearest real dirty
//! mark in its row band** — the quantity that says whether the narrow rule
//! skipped it.
//!
//! That last number is the whole point. If the differing cells sit *inside*
//! both regions, the narrowing did not skip them and the divergence is a
//! coupling elsewhere — the visit *order* changed, or a chunk's wakefulness
//! did, which is §E2's own leading hypothesis (`field::step`'s
//! `active_chunk_count()` gate). If they sit outside the narrow region, the
//! spans genuinely drop cells the rules act on and the shape is unsafe as
//! built. **Those are opposite findings and a hash cannot tell them apart.**
//!
//! ## Its controls
//!
//! - **The arms must agree until they diverge.** A differing cell at frame 0
//!   means the two `Lab`s were not the same bed and nothing below it means
//!   anything. Printed as `CONTROL`.
//! - **`arm=box` runs both arms on the box rule**, which must then never
//!   diverge at all. That is the positive control from the quiet side: it
//!   proves the lockstep comparison itself introduces no difference, so a
//!   divergence under `arm=rows` belongs to the rule and not to the harness.
//!
//! ```text
//! cargo run --release --example sweepgap
//! cargo run --release --example sweepgap -- frames=6000 arm=box
//! cargo run --release --example sweepgap -- frames=5000 cells=40
//! ```

use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::Lab;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::{ChunkCoord, CHUNK_SIZE};
use pixel_physics::sim::fxhash::ChunkMap;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// One awake chunk's real marks, row by row, as `mark_dirty` recorded them —
/// world x, unexpanded, unclipped — plus the region today's rule derives from
/// them.
struct Marks {
    /// Indexed by `row - (chunk.min_y - 1)`, so index 0 is the row above the
    /// chunk. Exactly `Chunk::dirty_rows`' own indexing.
    rows: Vec<Option<(i32, i32)>>,
    min_y: i32,
    reach: i32,
}

impl Marks {
    /// The nearest mark to `(x, y)` in the row band the rules can see, as a
    /// column distance. `None` when no row within one of `y` carries a mark.
    fn nearest(&self, x: i32, y: i32) -> Option<i32> {
        let mut best: Option<i32> = None;
        for row in (y - 1)..=(y + 1) {
            let i = row - self.min_y;
            if i < 0 || i as usize >= self.rows.len() {
                continue;
            }
            if let Some((lo, hi)) = self.rows[i as usize] {
                // Inside the span is distance zero; outside it, the gap to
                // whichever end is closer.
                let d = if x < lo {
                    lo - x
                } else if x > hi {
                    x - hi
                } else {
                    0
                };
                best = Some(best.map_or(d, |b: i32| b.min(d)));
            }
        }
        best
    }

    /// Is `(x, y)` inside the per-row span plan — the region
    /// `PIXEL_PHYSICS_SWEEP=rows` walks?
    ///
    /// Reproduces `Chunk::sweep_plan`'s rows arm exactly: each row's span is
    /// the union of the marks on it and the two beside it, expanded by the
    /// chunk's `reach` and clipped to the chunk.
    fn in_rows(&self, x: i32, y: i32) -> bool {
        self.nearest(x, y).is_some_and(|d| d <= self.reach)
    }

}

fn snapshot(world: &World, w: i32, h: i32, into: &mut Vec<Cell>) {
    into.clear();
    for y in 0..h {
        for x in 0..w {
            into.push(world.get(x, y));
        }
    }
}

fn main() {
    let d = LabBox::default();
    let frames: u64 = arg("frames").unwrap_or(6_000);
    let report: u64 = arg("report").unwrap_or(1_000);
    let seed: u64 = arg("seed").unwrap_or(d.seed);
    let width: i32 = arg("width").unwrap_or(d.width);
    let height: i32 = arg("height").unwrap_or(d.height);
    // How many differing cells to print at the divergence frame.
    let cells: usize = arg("cells").unwrap_or(24);
    // `arm=box` puts BOTH arms on the box rule -- the positive control that
    // the lockstep comparison introduces no difference of its own.
    let arm: String = arg("arm").unwrap_or_else(|| "rows".to_string());
    let b_rows = match arm.as_str() {
        "rows" => true,
        "box" => false,
        other => panic!("arm= takes rows or box, not {other:?}"),
    };
    let threads = std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "unset".to_string());
    println!(
        "sweepgap: frames={frames} seed={seed} width={width} height={height} arm={arm} \
         cells={cells} RAYON_NUM_THREADS={threads}"
    );
    println!(
        "  arm A = the box rule (what ships). arm B = {}.",
        if b_rows { "the per-row-span rule (PIXEL_PHYSICS_SWEEP=rows)" } else { "the box rule too -- the control" }
    );

    let spec = LabBox { width, height, seed, ..d.clone() };
    let mut a = Lab::new(spec.clone());
    let mut b = Lab::new(spec);
    a.world.set_sweep_rows(false);
    b.world.set_sweep_rows(b_rows);

    let mut ga: Vec<Cell> = Vec::with_capacity((width * height) as usize);
    let mut marks_a: ChunkMap<Marks> = ChunkMap::default();
    let mut marks_b: ChunkMap<Marks> = ChunkMap::default();

    for f in 0..frames {
        // The real marks of both arms, read before the step -- the sets the
        // two sweeps are about to work from.
        read_marks(&a.world, &mut marks_a);
        read_marks(&b.world, &mut marks_b);
        snapshot(&a.world, width, height, &mut ga);

        a.tick_for_harness();
        b.tick_for_harness();

        let mut diffs: Vec<(i32, i32)> = Vec::new();
        for y in 0..height {
            for x in 0..width {
                if a.world.get(x, y) != b.world.get(x, y) {
                    diffs.push((x, y));
                    if diffs.len() > 4_000 {
                        break;
                    }
                }
            }
        }

        if !diffs.is_empty() {
            println!("\nFIRST DIVERGENCE at frame {} -- {} differing cell(s)", f, diffs.len());
            if f == 0 {
                println!("  CONTROL: the two arms were not the same bed. Nothing below means anything.");
                return;
            }
            println!(
                "  {:>6} {:>6} {:>12} {:>12} {:>7} {:>7} {:>9} {:>7} {:>8} {:>8} {:>8}",
                "x", "y", "arm A (box)", "arm B", "auxA", "auxB", "chunk", "reach", "distA", "distB", "in B?"
            );
            for &(x, y) in diffs.iter().take(cells) {
                let c = ChunkCoord::containing(x, y);
                let ca = a.world.get(x, y);
                let cb = b.world.get(x, y);
                let ma = marks_a.get(&c);
                let mb = marks_b.get(&c);
                // `in B?` is the question: did arm B's own narrowed region
                // contain this cell at all? "no" means the spans skipped a
                // cell the rules then acted on; "yes" means the narrowing did
                // not skip it and the divergence is a coupling elsewhere.
                let in_b = mb.map(|m| if m.in_rows(x, y) { "yes" } else { "NO" }).unwrap_or("asleep");
                println!(
                    "  {:>6} {:>6} {:>12} {:>12} {:>7} {:>7} {:>3},{:<3} {:>7} {:>8} {:>8} {:>8}",
                    x,
                    y,
                    a.world.materials.get(ca.material).name,
                    b.world.materials.get(cb.material).name,
                    ca.aux(),
                    cb.aux(),
                    c.x,
                    c.y,
                    mb.map(|m| m.reach).unwrap_or(-1),
                    ma.and_then(|m| m.nearest(x, y)).map(|d| d.to_string()).unwrap_or_else(|| "-".into()),
                    mb.and_then(|m| m.nearest(x, y)).map(|d| d.to_string()).unwrap_or_else(|| "-".into()),
                    in_b
                );
            }
            let moisture_only = diffs.iter().all(|&(x, y)| {
                let (ca, cb) = (a.world.get(x, y), b.world.get(x, y));
                ca.material == cb.material
                    && ca.organism_id() == cb.organism_id()
                    && ca.with_aux(0) == cb.with_aux(0)
                    && a.world.materials.get(ca.material).water_capacity > 0
            });
            println!(
                "\n  every differing cell is aux-only on a water-holding material: {} -- {}",
                moisture_only,
                if moisture_only {
                    "so the divergence is in the SOIL-MOISTURE channel, not in what moved"
                } else {
                    "so at least one difference is a real material or organism difference"
                }
            );
            let skipped = diffs
                .iter()
                .filter(|&&(x, y)| {
                    marks_b.get(&ChunkCoord::containing(x, y)).is_some_and(|m| !m.in_rows(x, y))
                })
                .count();
            let asleep = diffs
                .iter()
                .filter(|&&(x, y)| !marks_b.contains_key(&ChunkCoord::containing(x, y)))
                .count();
            println!(
                "\n  of {} differing cells: {} lay OUTSIDE arm B's own narrowed region,\n                   {} lay in a chunk arm B had asleep, {} lay inside it",
                diffs.len(),
                skipped,
                asleep,
                diffs.len() - skipped - asleep
            );
            println!(
                "\n  READ IT THIS WAY. Cells OUTSIDE arm B's region: the spans drop cells the\n                   rules act on, and the shape is unsafe as built. Cells INSIDE it: the narrowing\n                   skipped nothing and the divergence is a coupling elsewhere -- visit order, or\n                   chunk wakefulness feeding field::step's active_chunk_count() gate, which is\n                   §E2's own leading hypothesis. Those are opposite findings."
            );
            println!(
                "\nCONTROL arm=box would have to show no divergence at all; run it to prove this\n                 one belongs to the rule and not to the harness."
            );
            return;
        }

        if report > 0 && (f + 1).is_multiple_of(report) {
            println!("  frame {:>6}: arms identical", f + 1);
        }
    }

    println!("\nno divergence in {frames} frames (arm={arm})");
    if b_rows {
        println!(
            "  §E2 puts it at frame 4,330 on the standard lab bed. A run past that with no\n               divergence means the bed here is not §E2's bed, or the coupling has been\n               closed since -- either way it is a finding, not a pass."
        );
    } else {
        println!("CONTROL arm=box: no divergence -> PASS, the lockstep comparison adds nothing of its own");
    }
}

/// Every awake chunk's real marks and today's region, keyed by chunk.
fn read_marks(world: &World, into: &mut ChunkMap<Marks>) {
    into.clear();
    for c in world.chunks_to_sweep() {
        if world.sweep_region(c).is_none() {
            continue;
        }
        let min_y = c.bounds().min_y - 1;
        let rows: Vec<Option<(i32, i32)>> =
            (0..(CHUNK_SIZE + 2)).map(|i| world.dirty_marks_row(c, min_y + i)).collect();
        into.insert(c, Marks { rows, min_y, reach: world.chunk_reach(c).unwrap_or(1) });
    }
}
