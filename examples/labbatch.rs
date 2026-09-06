//! **How different are two chambers that differ only by their seed?**
//!
//! The owner's question, 2026-08-31: *"is there enough randomness for this to
//! work or is our code too deterministic?"* This is that question as a
//! measurement rather than an opinion, and it is also the runner's own
//! positive and negative control.
//!
//! ```text
//! cargo run --release --example labbatch                        # 12 seeds, 9,000 frames
//! cargo run --release --example labbatch -- arm=same            # the control: one seed, N copies
//! cargo run --release --example labbatch -- runs=24 frames=20000
//! cargo run --release --example labbatch -- sweep=soil_depth values=48,96,144 runs=6
//! ```
//!
//! # Two arms, and neither means anything without the other
//!
//! `CLAUDE.md`'s most-recurring failure is a number that is arithmetically
//! right and about the wrong thing. *"12 runs completed"* is true of twelve
//! runs of the same world, so:
//!
//! | arm | what it must report | what a wrong answer means |
//! |---|---|---|
//! | `same` | **spread exactly 0** on every column | the engine is not reproducible, and no comparison between any two chambers means anything |
//! | `seeded` | **spread > 0** | the seed is not reaching the copies, and a rack is one run wearing N labels |
//!
//! `arm=both` (the default) runs them in that order, because the `same` arm
//! is cheap and a failure there invalidates everything after it.
//!
//! # Read the spread, never the mean
//!
//! Outcomes in this engine are chaotic in the seed: measured elsewhere in
//! this repo with **no true effect present**, two copies of one genome gave
//! one arm between **40.9% and 80.5%** of a bed, and twelve identical trees
//! from one genome span **31 to 153 cells**. So every column here prints
//! min / median / max and the ratio between the extremes, and a sweep prints
//! them **per setting** — a difference of two means over a spread like that
//! is not a result, which is why `CLAUDE.md` insists on an order statistic
//! and records that *"six seeds is not a sweep"*.
//!
//! The number this exists to produce is the **seeded arm's spread**: it is
//! how much two chambers differ for no reason at all, and therefore the bar
//! any real comparison has to clear.

use pixel_physics::lab::batch::{BatchSpec, PlannedRun, Sweep};
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::scene::LabBox;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}="))?.parse().ok())
}

fn main() {
    let arm: String = arg("arm").unwrap_or_else(|| "both".to_string());
    let runs: u32 = arg("runs").unwrap_or(12);
    let width: i32 = arg("width").unwrap_or(512);
    let founders: usize = arg("founders").unwrap_or(8);
    let colonies: usize = arg("colonies").unwrap_or(1);
    let sweep_field: Option<String> = arg("sweep");
    let values: Option<String> = arg("values");
    // `scenario=<name>` loads a saved starting box instead of building one
    // from the flags above -- see `lab::scenario`. A bad name refuses at
    // load rather than silently falling back to the default bed, same as
    // `bin/lab.rs`'s own rule for it.
    let scenario_name: Option<String> = arg("scenario");
    let scenario: Option<Scenario> = scenario_name.as_deref().map(|n| {
        Scenario::load(n).unwrap_or_else(|e| {
            eprintln!("scenario {n}: {e}");
            std::process::exit(1);
        })
    });
    // The scenario's own horizon when one is loaded and `frames=` was not
    // given on the command line -- the report's own read-at frame count,
    // rather than this harness's unrelated default.
    let frames: u64 = arg("frames").unwrap_or_else(|| scenario.as_ref().filter(|s| s.horizon > 0).map(|s| s.horizon).unwrap_or(9_000));

    // **Echoes its own parameters**, because a harness that does not is one
    // whose knobs nobody can tell are connected — the 3.5-hour study that
    // produced eight byte-identical logs per species is the case on record.
    println!(
        "labbatch: arm={arm} runs={runs} frames={frames} width={width} founders={founders} colonies={colonies} sweep={} values={}{}",
        sweep_field.as_deref().unwrap_or("-"),
        values.as_deref().unwrap_or("-"),
        scenario.as_ref().map(|s| format!(" scenario={} ({})", s.name, s.question)).unwrap_or_default()
    );

    let base = match &scenario {
        Some(s) => s.bed.clone(),
        None => LabBox { width, founders, colonies, ..LabBox::default() },
    };
    let sweep = match (&sweep_field, &values) {
        (Some(f), Some(v)) => Some(Sweep {
            field: f.clone(),
            values: v.split(',').filter_map(|s| s.trim().parse().ok()).collect(),
        }),
        (Some(f), None) => {
            eprintln!("sweep={f} needs values=a,b,c");
            return;
        }
        _ => None,
    };

    let spec = BatchSpec { base, replicates: runs, sweep, frames, seed0: 1, keep_bytes: 0, scenario: scenario.clone() };
    let planned = spec.runs();
    let per = BatchSpec::world_bytes(&spec.base) as f64 / (1024.0 * 1024.0);
    println!(
        "  {} runs, {:.1} MB per world if held ({:.0} MB for all of them)\n",
        planned.len(),
        per,
        per * planned.len() as f64
    );

    if arm == "same" || arm == "both" {
        // Every copy at ONE seed. The specificity half: this must be a row of
        // zeroes, and if it is not, nothing below it is interpretable.
        let one = planned[0].clone();
        let copies: Vec<PlannedRun> = (0..runs.min(4)).map(|i| PlannedRun { index: i as usize, ..one.clone() }).collect();
        report("same seed, N copies -- MUST be all-zero spread", &run_all(&copies, frames), false);
    }
    if arm == "seeded" || arm == "both" {
        report("one seed each -- the spread two chambers differ by for no reason", &run_all(&planned, frames), spec.sweep.is_some());
    }
}

/// One column of the spread table: its heading and how to read it off a row.
type Column = (&'static str, fn(&Row) -> f64);

/// A run's numbers, flattened for printing.
struct Row {
    index: usize,
    seed: u64,
    setting: Option<f32>,
    plants: f64,
    plant_cells: f64,
    animals: f64,
    generation: f64,
    borne: f64,
    sprouted: f64,
}

fn run_all(planned: &[PlannedRun], frames: u64) -> Vec<Row> {
    let started = std::time::Instant::now();
    // `keep_bytes: 0` -- worlds are not wanted here, this harness reads
    // censuses. Dropping them keeps a 100-run sweep flat in memory instead of
    // the ~270 MB it would otherwise hold.
    let batch = pixel_physics::lab::batch::Batch::start_runs(planned.to_vec(), frames, 0);
    let mut rows: Vec<Row> = Vec::new();
    let mut seen = 0usize;
    while !batch.is_finished() || seen < planned.len() {
        for r in batch.drain() {
            seen += 1;
            let c = &r.census;
            rows.push(Row {
                index: r.index,
                seed: r.spec.seed,
                setting: r.setting,
                plants: c.plants as f64,
                plant_cells: c.plant_cells as f64,
                animals: c.animals as f64,
                generation: c.plant_generation as f64,
                borne: c.seeds_borne as f64,
                sprouted: c.germinations as f64,
            });
            eprintln!("  {seen} / {} runs done", planned.len());
        }
        if seen >= planned.len() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    rows.sort_by_key(|r| r.index);
    println!("  ({:.1} s wall, {:.1} s per run)", started.elapsed().as_secs_f64(), started.elapsed().as_secs_f64() / planned.len() as f64);
    rows
}

fn report(title: &str, rows: &[Row], by_setting: bool) {
    println!("\n--- {title} ---");
    println!("{:>4} {:>6} {:>8} {:>7} {:>7} {:>8} {:>5} {:>7} {:>9}", "run", "seed", "setting", "plants", "cells", "animals", "gen", "borne", "sprouted");
    for r in rows {
        let set = r.setting.map(|v| format!("{v:.0}")).unwrap_or_else(|| "-".into());
        println!(
            "{:>4} {:>6} {:>8} {:>7.0} {:>7.0} {:>8.0} {:>5.0} {:>7.0} {:>9.0}",
            r.index, r.seed, set, r.plants, r.plant_cells, r.animals, r.generation, r.borne, r.sprouted
        );
    }

    let cols: [Column; 6] = [
        ("plants", |r| r.plants),
        ("plant cells", |r| r.plant_cells),
        ("animals", |r| r.animals),
        ("generation", |r| r.generation),
        ("seeds borne", |r| r.borne),
        ("sprouted", |r| r.sprouted),
    ];
    println!("\n{:<12} {:>8} {:>8} {:>8} {:>9}", "", "min", "median", "max", "max/min");
    let mut any_spread = false;
    for (name, get) in cols {
        let mut v: Vec<f64> = rows.iter().map(get).collect();
        v.sort_by(|a, b| a.partial_cmp(b).expect("censuses are never NaN"));
        let (lo, hi) = (v[0], v[v.len() - 1]);
        let mid = v[v.len() / 2];
        // **The ratio, not the difference.** "12 plants apart" means nothing
        // without knowing whether that is 12 against 14 or 12 against 400.
        let ratio = if lo > 0.0 { format!("{:.2}x", hi / lo) } else if hi > 0.0 { "inf".into() } else { "-".into() };
        if hi > lo {
            any_spread = true;
        }
        println!("{name:<12} {lo:>8.0} {mid:>8.0} {hi:>8.0} {ratio:>9}");
    }

    // The control, stated as a verdict rather than left for the reader to
    // infer from the table.
    if title.starts_with("same seed") {
        println!(
            "\n  [{}] identical seeds: spread is {}",
            if any_spread { "FAIL" } else { "PASS" },
            if any_spread { "NON-ZERO -- the engine is not reproducible and nothing below is interpretable" } else { "zero on every column, as it must be" }
        );
    } else {
        println!(
            "\n  [{}] different seeds: spread is {}",
            if any_spread { "PASS" } else { "FAIL" },
            if any_spread {
                "non-zero -- there is genuine variation to select on"
            } else {
                "ZERO -- the seed is not reaching the copies, so a rack is one run wearing N labels"
            }
        );
    }

    if by_setting {
        println!("\n  (a sweep's settings must be read per setting, at the order statistic --");
        println!("   a difference of two means over the spread above is not a result)");
    }
}
