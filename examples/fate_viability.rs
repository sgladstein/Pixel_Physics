//! **Gate 1: how many mutations of the production rule produce a plant that
//! can still live?**
//!
//! `Reports/plant-morphology-evolvability-2026-08-26.md` §7 names this as the
//! first of four gates and says it can return "no":
//!
//! > *"Most structural mutations are nonviable (§4). In a GA that is fatal;
//! > here it is largely fine, because a nonviable plant simply dies and the
//! > economy is already the filter. But the **rate** decides whether radiation
//! > moves or stalls, and it is a cheap experiment: mutate the rule table N
//! > ways, count how many reach reproductive size."*
//!
//! It is deliberately run **before** organs are priced and given materials.
//! A cheap "no" here is worth far more than an expensive one after paying for
//! two new cell types, two materials and a pricing pass.
//!
//! # The two controls, and why the number is worthless without them
//!
//! A viability rate is exactly the shape of number `CLAUDE.md` warns about:
//! arithmetically correct, plausible, and about the wrong thing. It has two
//! failure modes and one control each.
//!
//! - **Positive control (`--base`)**: the *unmutated* table, run identically.
//!   If the base does not come back viable then this bed, this frame budget or
//!   this species cannot grow a plant at all, and every mutant reading zero is
//!   measuring the harness rather than the mutation.
//! - **Negative control (`--lethal`)**: a table that *must* be dead — a shoot
//!   tip whose child is a `Seed`, so the frontier turns into seeds and growth
//!   stops. If that reads viable, "viable" is not measuring survival.
//!
//! Both run by default and both are printed beside the rate. A run that
//! reports the mutant rate alone has skipped the only two lines that make it
//! readable.
//!
//! ```text
//! cargo run --release --example fate_viability -- mutants=60 frames=12000
//! ```

mod common;

use pixel_physics::sim::organism::{CellType, Fate, FateWhen};
use pixel_physics::sim::parallel;
use pixel_physics::sim::rng;
use pixel_physics::sim::world::World;

/// The species the mutants are variants of.
const BASE_RON: &str = include_str!("../assets/species/tree.ron");

/// Cell types a mutation may point a fate at.
///
/// The creature types (`Head`, `Segment`) are excluded — they are not part of
/// a plant's vocabulary and a mutation reaching one would be measuring the
/// harness's carelessness rather than the substrate's tolerance.
const PLANT_TYPES: [CellType; 6] = [
    CellType::Seed,
    CellType::GrowingTip,
    CellType::MatureBody,
    CellType::Leaf,
    CellType::RootTip,
    CellType::DormantBud,
];

fn type_name(t: CellType) -> &'static str {
    match t {
        CellType::Seed => "Seed",
        CellType::GrowingTip => "GrowingTip",
        CellType::MatureBody => "MatureBody",
        CellType::Leaf => "Leaf",
        CellType::RootTip => "RootTip",
        CellType::DormantBud => "DormantBud",
        CellType::Head => "Head",
        CellType::Segment => "Segment",
    }
}

fn when_name(w: FateWhen) -> &'static str {
    match w {
        FateWhen::Grew => "Grew",
        FateWhen::Node => "Node",
        FateWhen::Stale => "Stale",
        FateWhen::Flush => "Flush",
    }
}

/// One species' whole production rule, in the harness's own hands so it can be
/// mutated and then written back out as RON.
type Table = Vec<(CellType, Vec<Fate>)>;

/// The shipped table, written out here rather than parsed back off the asset.
///
/// **A duplicate on purpose, and the duplication is the test.** If this drifts
/// from `tree.ron` the base control stops reproducing the shipped plant, and
/// the run says so: the positive control is grown from *this* table, so a
/// disagreement shows up as the control failing rather than as a silent shift
/// in what "unmutated" means.
fn base_table() -> Table {
    let f = |when, becomes, child, lateral| Fate { when, becomes, child, lateral };
    vec![
        (
            CellType::GrowingTip,
            vec![
                f(FateWhen::Node, CellType::DormantBud, Some(CellType::GrowingTip), Some(CellType::GrowingTip)),
                f(FateWhen::Grew, CellType::MatureBody, Some(CellType::GrowingTip), Some(CellType::GrowingTip)),
                f(FateWhen::Stale, CellType::MatureBody, None, None),
            ],
        ),
        (
            CellType::RootTip,
            vec![
                f(FateWhen::Grew, CellType::MatureBody, Some(CellType::RootTip), Some(CellType::RootTip)),
                f(FateWhen::Stale, CellType::MatureBody, None, None),
            ],
        ),
        (CellType::DormantBud, vec![f(FateWhen::Flush, CellType::GrowingTip, None, None)]),
    ]
}

/// Render a table as the `fates:` block of a species file.
fn table_to_ron(t: &Table) -> String {
    let mut s = String::from("    fates: [\n");
    for (ct, rules) in t {
        s.push_str(&format!("        ({}, [\n", type_name(*ct)));
        for r in rules {
            s.push_str(&format!("            (when: {}, becomes: {}", when_name(r.when), type_name(r.becomes)));
            if let Some(c) = r.child {
                s.push_str(&format!(", child: Some({})", type_name(c)));
            }
            if let Some(l) = r.lateral {
                s.push_str(&format!(", lateral: Some({})", type_name(l)));
            }
            s.push_str("),\n");
        }
        s.push_str("        ]),\n");
    }
    s.push_str("    ],\n");
    s
}

/// Splice a table and a name into the base species source.
///
/// The `fates:` block is bracket-matched rather than line-counted, so this
/// does not quietly truncate if the shipped block is reformatted.
fn variant_ron(name: &str, t: &Table) -> String {
    let start = BASE_RON.find("    fates: [").expect("tree.ron declares a fates block");
    let mut depth = 0usize;
    let mut end = start;
    for (i, c) in BASE_RON[start..].char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    // past the trailing comma and newline of the block
    let end = BASE_RON[end..].find('\n').map_or(end, |n| end + n + 1);
    let mut out = String::with_capacity(BASE_RON.len() + 512);
    out.push_str(&BASE_RON[..start]);
    out.push_str(&table_to_ron(t));
    out.push_str(&BASE_RON[end..]);
    // Rename so the variant does not replace `tree` in the registry.
    let quoted = format!("name: \"{name}\"");
    out.replacen("name: \"tree\"", &quoted, 1)
}

/// Apply one point mutation: pick a rule, pick a field, point it somewhere
/// **else** — never at the value it already holds.
///
/// The redraw is not fussiness. Drawing uniformly from the six plant types
/// means one mutation in six lands on the current value and changes nothing,
/// and a no-op counted as "viable" inflates the headline by exactly that
/// fraction. Measured on the first run: 7 of 48. A rate whose denominator
/// includes mutations that could not have done anything is not a tolerance,
/// it is a dilution.
fn mutate(t: &mut Table, rng: &mut rng::Rng) -> String {
    let ci = rng.below(t.len() as u32) as usize;
    let (ct, rules) = &mut t[ci];
    let ri = rng.below(rules.len() as u32) as usize;
    let rule = &mut rules[ri];
    // Only fields that exist on this rule: `child`/`lateral` are `None` on
    // `Stale`/`Flush`, which create no cell, and inventing one there would be
    // mutating a field the engine never reads.
    let mut slots: Vec<u8> = vec![0];
    if rule.child.is_some() {
        slots.push(1);
    }
    if rule.lateral.is_some() {
        slots.push(2);
    }
    let slot = slots[rng.below(slots.len() as u32) as usize];
    let current = match slot {
        0 => rule.becomes,
        1 => rule.child.expect("slot 1 is only offered when child is Some"),
        _ => rule.lateral.expect("slot 2 is only offered when lateral is Some"),
    };
    // Redraw until it actually moves -- see this function's doc.
    let mut to = current;
    while to == current {
        to = PLANT_TYPES[rng.below(PLANT_TYPES.len() as u32) as usize];
    }
    let what = match slot {
        0 => {
            rule.becomes = to;
            "becomes"
        }
        1 => {
            rule.child = Some(to);
            "child"
        }
        _ => {
            rule.lateral = Some(to);
            "lateral"
        }
    };
    format!("{}.{}.{} -> {}", type_name(*ct), when_name(rule.when), what, type_name(to))
}

/// Grow a stand of one variant and report (established plants, seeds set).
fn trial(source: &str, name: &str, frames: u64, founders: usize, worldseed: u64) -> (usize, u64) {
    let scene = common::PlantScene {
        trees: founders,
        width: 160,
        species: name.to_string(),
        // Registered inside `build`, before it plants -- see the field's doc
        // for the silent-empty-stand this ordering exists to prevent.
        species_ron: Some(source.to_string()),
        ..Default::default()
    };
    let mut w: World = scene.build();
    w.seed = worldseed;
    for _ in 0..frames {
        parallel::step(&mut w);
        w.step_active_sites();
        w.step_fields();
    }
    let mut established = 0usize;
    let mut seeds = 0u64;
    for id in w.live_organism_ids() {
        if let Some(s) = w.organism(id) {
            if s.cells.len() >= 20 {
                established += 1;
            }
            seeds += s.seeds_set as u64;
        }
    }
    (established, seeds)
}

fn main() {
    let arg = |k: &str, d: u64| {
        std::env::args().find_map(|a| a.strip_prefix(k).map(|v| v.parse().expect(k))).unwrap_or(d)
    };
    let mutants = arg("mutants=", 40) as usize;
    let frames = arg("frames=", 12000);
    let founders = arg("founders=", 3) as usize;
    let worldseed = arg("worldseed=", 7);
    println!("fate_viability: mutants={mutants} frames={frames} founders={founders} worldseed={worldseed}");

    // --- positive control: the unmutated table ---
    let base = base_table();
    let (be, bs) = trial(&variant_ron("fv_base", &base), "fv_base", frames, founders, worldseed);
    println!("\npositive control (unmutated table): {be}/{founders} established, {bs} seeds set");

    // --- negative control: a table that must be dead ---
    let mut lethal = base_table();
    lethal[0].1[1].child = Some(CellType::Seed); // GrowingTip.Grew.child -> Seed
    let (le, ls) = trial(&variant_ron("fv_lethal", &lethal), "fv_lethal", frames, founders, worldseed);
    println!("negative control (shoot child -> Seed):  {le}/{founders} established, {ls} seeds set");

    if be == 0 {
        println!("\nSTOP: the positive control did not establish. Every mutant number below would be");
        println!("measuring this harness -- the bed, the frame budget or the species -- and not the");
        println!("mutation. Fix the control before reading anything else.");
        return;
    }

    // --- the mutants ---
    //
    // **Three outcomes, not two, and the third is the one a naive rate hides.**
    // A mutation whose stand comes out *identical to the base* has not
    // demonstrated that the substrate tolerates it; it has demonstrated that
    // nothing read the field. Measured on the first run of this harness:
    // `RootTip.Grew.lateral` pointed at four different cell types produced
    // exactly the base's 80 seeds every time, because a root never takes the
    // lateral path in this scene. Counting those as "viable" is the
    // identical-output-across-settings tell that `CLAUDE.md` names for a knob
    // that was never connected -- and it inflates the headline with cases that
    // could not have failed.
    let mut viable = 0usize;
    let mut reproduced = 0usize;
    let mut silent = 0usize;
    // **Printed as they land, not buffered to the end.** An earlier version
    // collected every line and printed at the finish, which meant a 25-minute
    // run produced no output at all -- indistinguishable, while you are
    // watching it, from a hung one. A long harness that cannot show progress
    // is one nobody can tell is working.
    println!("\nper mutation (established of {founders} founders, and seeds set):");
    for i in 0..mutants {
        let mut t = base_table();
        let mut r = rng::stream(worldseed, 0xF8, i as u64, 0);
        let what = mutate(&mut t, &mut r);
        let name = format!("fv_{i}");
        let (e, s) = trial(&variant_ron(&name, &t), &name, frames, founders, worldseed);
        if e > 0 {
            viable += 1;
        }
        if s > 0 {
            reproduced += 1;
        }
        let quiet = e == be && s == bs;
        if quiet {
            silent += 1;
        }
        println!("  {:<44} plants {e:>2}  seeds {s:>3}{}", what, if quiet { "   [silent: identical to base]" } else { "" });
    }
    let pct = |n: usize, d: usize| if d == 0 { f32::NAN } else { 100.0 * n as f32 / d as f32 };
    let effective = mutants - silent;
    println!("\n{mutants} point mutations of the production rule:");
    println!("  silent (output identical to base -- the field is never read here)");
    println!("                          {silent}/{mutants}  ({:.0}%)", pct(silent, mutants));
    println!("  EFFECTIVE mutations     {effective}");
    println!("    established at all    {}/{effective}  ({:.0}%)", viable.saturating_sub(silent), pct(viable.saturating_sub(silent), effective));
    println!("    set at least one seed {}/{effective}  ({:.0}%)", reproduced.saturating_sub(silent), pct(reproduced.saturating_sub(silent), effective));
    println!("\n  (raw, including silent: established {viable}/{mutants}, reproduced {reproduced}/{mutants})");
    println!("\nRead against the controls, never alone: base {be} plants / {bs} seeds,");
    println!("lethal {le} plants / {ls} seeds. `plants` counts every established");
    println!("organism including recruits, so it can exceed the {founders} founders.");
}
