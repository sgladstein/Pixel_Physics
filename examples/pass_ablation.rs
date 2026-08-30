//! Which generation pass eats which other pass's output.
//!
//! **This generator's recurring defect is not a broken mechanism, it is a
//! mechanism whose cells were already taken.** Five have been found so far,
//! every one by accident and one per round:
//!
//! | eater | eaten | found |
//! |---|---|---|
//! | `pockets` | `vaults` — one sand grain rejected a whole cave system | round-5 review |
//! | `brows` | `boulders` — a lip over the dome's air | round-4 finding R4-1 |
//! | `soil_blanket` | `talus` — erosion's apron folded in first | round-4 finding R4-2 |
//! | `brows` | `ponds` — a lip roofed water filled from both sides | round-4 finding R4-3 |
//! | erosion | formation-scale relief — flattened, never rebuilt | erosion design |
//!
//! None of them is visible in a pass's own counter, because each pass
//! reports only what *it* wrote: a pass that wrote nothing because its cells
//! were taken looks exactly like a pass whose noise draw came up empty. So
//! the counters were green, the tests were green, and the features were
//! absent from the screen.
//!
//! The fix is to stop finding them one at a time. Build the world once per
//! pass with that pass switched off, difference the whole report vector, and
//! read the result as a matrix: **row = the pass switched off, column = what
//! that did to another pass's output.** A positive entry means the switched-
//! off pass was suppressing that column; a negative entry means it was
//! feeding it.
//!
//! Reported as an order statistic over seeds, never a single seed — cave and
//! boulder placement are noise draws whose per-seed outcome reshuffles on any
//! legitimate change (CLAUDE.md's rule for anything guarding procedural
//! content).
//!
//! ```text
//! cargo run --release --example pass_ablation                    # 6 seeds, every preset
//! cargo run --release --example pass_ablation -- seeds=12 preset=canyon
//! cargo run --release --example pass_ablation -- threshold=2     # show weaker interactions
//! cargo run --release --example pass_ablation -- gate=1 seeds=2  # exit 1 on a deleted feature
//! ```
//!
//! # `gate=1`, and why the matrix needed one
//!
//! **Nothing ran this.** R4-1 -- `brows` deleting 100% of boulders -- was
//! measured on 2026-08-20, written up in
//! `Reports/pass-interference-2026-08.md`, and was still deleting the feature
//! nine days later. The instrument found it; no gate carried it, so it went
//! back to being something a session had to remember.
//!
//! `gate=1` asserts the two things this matrix can say that a pass counter
//! cannot, and exits 1 if either fails:
//!
//! 1. **No pass APPEARS when another is switched off.** That is the R4-1
//!    signature exactly -- a feature at zero whose cells another pass had
//!    already taken -- and it is the one entry in the matrix that cannot be
//!    a matter of degree.
//! 2. **Every pass writes cells somewhere.** A pass at zero on *every* preset
//!    has stopped existing, whatever the render suggests.
//!
//! It deliberately does **not** gate the magnitude of a suppression. Several
//! are correct: `soil_blanket` feeds `residuals` its socket, `ponds` refuses
//! a spring basin that would merge with standing water, and a bar on those
//! would be a bar on the generator working.

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::world::World;
use pixel_physics::worldgen;

fn main() {
    let mut seeds: u64 = 6;
    let mut only = String::new();
    // Percent change below which an interaction is not worth printing. Not
    // zero: every pass jitters a little against a different world, and a row
    // of 0.4% noise buries the 60% entries that matter.
    let mut threshold = 5.0f32;
    let mut gate = false;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "seeds" => seeds = v.parse().expect("seeds=N"),
            "preset" => only = v.to_string(),
            "threshold" => threshold = v.parse().expect("threshold=PCT"),
            "gate" => gate = v != "0",
            _ => panic!("unknown argument {arg:?}"),
        }
    }

    let (presets, err) = worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("{e}");
    }
    let names = worldgen::pass_names();
    println!(
        "pass ablation: {seeds} seeds, {} passes, world {}x{}",
        names.len(),
        pixel_physics::app::WORLD_WIDTH,
        pixel_physics::app::WORLD_HEIGHT
    );
    println!("  row = pass switched off; entry = median % change in that column's own cell count");
    println!("  positive = the switched-off pass was SUPPRESSING that column");
    println!("  negative = the switched-off pass was FEEDING it");
    println!();

    // What `gate=1` reads at the end. Accumulated across every preset,
    // because a pass writing nothing on `arid` and plenty on `canyon` is the
    // generator working, and only the pooled view can tell that apart from a
    // pass that has stopped existing.
    let mut appeared: Vec<String> = Vec::new();
    let mut wrote_somewhere = vec![false; names.len()];
    for preset in presets.cycle_order() {
        if !only.is_empty() && preset != only {
            continue;
        }
        let Some(params) = presets.get(&preset) else { continue };
        let build = |seed: u64, skip: &str| -> Vec<(&'static str, usize)> {
            let bounds = Rect::new(
                0,
                0,
                pixel_physics::app::WORLD_WIDTH as i32 - 1,
                pixel_physics::app::WORLD_HEIGHT as i32 - 1,
            );
            let mut world = World::new(bounds);
            worldgen::generate_ablated(&mut world, worldgen::Spec::Generated { params, seed }, skip)
        };

        // Per (skipped pass, observed pass), one percent-change sample per
        // seed, reduced to a median at the end. A mean would let one seed
        // where a pass happened to write nothing dominate the row.
        let mut samples: Vec<Vec<Vec<f32>>> = vec![vec![Vec::new(); names.len()]; names.len()];
        let mut baseline_totals = vec![0usize; names.len()];
        for seed in 1..=seeds {
            let full = build(seed, "");
            for (i, (_, n)) in full.iter().enumerate() {
                baseline_totals[i] += n;
            }
            for (si, skipped) in names.iter().enumerate() {
                let ablated = build(seed, skipped);
                for oi in 0..names.len() {
                    if oi == si {
                        continue;
                    }
                    let before = full[oi].1 as f32;
                    let after = ablated[oi].1 as f32;
                    // A pass that writes nothing in the baseline has no
                    // percentage to change. Recorded as "appeared" only if
                    // the ablation makes it write something, which is the
                    // strongest possible interference signal and would be
                    // lost to a divide-by-zero guard that skipped it.
                    if before < 1.0 {
                        if after >= 1.0 {
                            samples[si][oi].push(f32::INFINITY);
                        }
                        continue;
                    }
                    samples[si][oi].push(100.0 * (after - before) / before);
                }
            }
        }

        println!("### {preset}");
        let mut any = false;
        for (si, skipped) in names.iter().enumerate() {
            let mut row: Vec<(f32, &str)> = Vec::new();
            for (oi, observed) in names.iter().enumerate() {
                let v = &mut samples[si][oi];
                if v.is_empty() {
                    continue;
                }
                v.sort_by(f32::total_cmp);
                let med = v[v.len() / 2];
                if med.is_finite() && med.abs() < threshold {
                    continue;
                }
                row.push((med, observed));
            }
            if row.is_empty() {
                continue;
            }
            any = true;
            row.sort_by(|a, b| b.0.abs().total_cmp(&a.0.abs()));
            print!("  without {skipped:<14}:");
            for (med, observed) in row {
                if med.is_infinite() {
                    appeared.push(format!("{preset}: without {skipped}, {observed} APPEARS (was zero)"));
                    print!("  {observed} APPEARS (was zero)");
                } else {
                    print!("  {observed} {med:+.0}%");
                }
            }
            println!();
        }
        if !any {
            println!("  no interaction above {threshold}%");
        }
        // The baseline itself, because a percentage on a pass that writes
        // eleven cells is not the same finding as the same percentage on one
        // that writes a million, and the matrix cannot show that.
        print!("  baseline cells/world:");
        for (i, n) in names.iter().enumerate() {
            if baseline_totals[i] > 0 {
                wrote_somewhere[i] = true;
            }
            print!("  {n} {}", baseline_totals[i] / seeds as usize);
        }
        println!();
        println!();
    }
    if !gate {
        return;
    }
    let mut fails = 0;
    println!("gate: no pass may APPEAR when another is switched off, and every pass must write somewhere.");
    for a in &appeared {
        println!("  FAIL  {a}");
        fails += 1;
    }
    // **Only evaluable pooled over presets**, so it is skipped outright when
    // `preset=` narrows the run. `flat` writes nothing from eight of the
    // fourteen passes by design and `arid` stands no water; "wrote nothing
    // on any preset" is a claim about the pass, and "wrote nothing on
    // `canyon` seed 1" is a claim about that world. Reporting the second
    // under the first's wording is how a selftest ends up satisfied by a
    // failure that has nothing to do with the defect it injected.
    let silent: Vec<&str> = if only.is_empty() {
        names
            .iter()
            .zip(&wrote_somewhere)
            .filter(|(_, &w)| !w)
            .map(|(n, _)| *n)
            .collect()
    } else {
        println!("  (skipping the every-pass-writes-somewhere half: preset={only} cannot answer it)");
        Vec::new()
    };
    // **`springs` writes no terrain cells by design** and is the one row that
    // must be excused rather than counted. It registers emitters on `World`
    // and the only cells it writes are its source pool, so a preset with no
    // basin to cut reports zero without anything being wrong. Excused by
    // name, not by a blanket "ignore zeroes": an exclusion that stops being
    // true should fail loudly, which is the same reasoning
    // `every_generation_pass_writes_cells` uses for its own three.
    for n in &silent {
        if *n == "springs" {
            println!("  note  springs wrote no cells on any preset -- it registers emitters, see its own test");
            continue;
        }
        println!("  FAIL  {n} wrote nothing on any preset: the pass has stopped existing");
        fails += 1;
    }
    if fails > 0 {
        println!("gate: {fails} finding(s)");
        std::process::exit(1);
    }
    println!("gate: clean");
}
