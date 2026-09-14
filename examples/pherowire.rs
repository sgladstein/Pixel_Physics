//! **Is all of it actually wired up?**
//!
//! The owner's second question, 2026-09-14. There are three planes and
//! seven brain slots reading them, and "wired" has two different meanings
//! that a grep cannot tell apart:
//!
//! * **engine-live** — `creature::sense` computes the input every tick and
//!   `creature::act` writes the plane. This is a property of the Rust and
//!   is the same for every species.
//! * **species-live** — some authored genome carries a nonzero path from
//!   that input to an output. This is a property of `assets/species/*.ron`
//!   and differs per animal.
//!
//! A slot can be engine-live and species-dead, which is the interesting
//! case and the one that reads as "the mechanism does nothing": the value
//! is computed, costs its sample every tick, and reaches no output. That
//! is not the same as broken, because mutation can connect it later — but
//! it does mean nothing *shipped* uses it, and it is invisible to anyone
//! reading `pheromone.rs`.
//!
//! ```text
//! cargo run --release --example pherowire
//! cargo run --release --example pherowire -- selftest
//! ```
//!
//! # Why this walks the genome and does not grep the .ron
//!
//! Because the authored form is sparse and indirect. `ant.ron` wires
//! `PheroAAlong` to hidden units 0 and 1, and those to `Move` — so a grep
//! for `(PheroAAlong, Move` finds nothing and the slot is live. The
//! reverse trap is worse: a weight can be authored, expanded into the
//! genome, and still be **dead on arrival** because it lands under
//! `brain::W_EPS`, which `eval_brain` treats as no connection at all. Only
//! expanding the genome and walking it answers either.
//!
//! So: `SpeciesRegistry::builtin()` -> each `CreatureDef::genome` ->
//! `brain::wiring_from_genome` -> follow every path.

use pixel_physics::sim::brain::{self, BrainInput, BrainOutput};
use pixel_physics::sim::organism::{SpeciesId, SpeciesRegistry};

/// The seven slots that read a pheromone plane, and the two that write
/// one. Named here rather than derived so that a slot appended to
/// `BrainInput` and forgotten here shows up as a diff rather than as
/// silence.
const READERS: [(BrainInput, &str); 7] = [
    (BrainInput::PheroAFront, "A concentration ahead"),
    (BrainInput::PheroALateral, "A right minus left"),
    (BrainInput::PheroAAlong, "A gradient along heading"),
    (BrainInput::PheroBFront, "B concentration ahead"),
    (BrainInput::PheroBLateral, "B right minus left"),
    (BrainInput::PheroBAlong, "B gradient along heading"),
    (BrainInput::Alarm, "alarm underfoot"),
];

const WRITERS: [(BrainOutput, &str); 2] = [(BrainOutput::EmitA, "lay channel A"), (BrainOutput::EmitB, "lay channel B")];

/// Is there any live path from this input to any output, direct or
/// through a hidden unit?
///
/// **`W_EPS` is applied at both legs of a two-hop path**, because
/// `eval_brain` does: a hidden unit reached by a live weight and leaving
/// by a dead one contributes nothing, and counting it would report a
/// mechanism that cannot fire.
fn input_paths(w: &brain::Wiring, input: BrainInput) -> (f32, Vec<String>) {
    let mut best = 0.0f32;
    let mut how = Vec::new();
    for i in &w.instincts {
        if i.0 as usize == input as usize && i.2.abs() >= brain::W_EPS {
            best = best.max(i.2.abs());
            how.push(format!("{:?} {:+.2}", i.1, i.2));
        }
    }
    for h in &w.hidden {
        if h.0 as usize != input as usize || h.2.abs() < brain::W_EPS {
            continue;
        }
        for o in &w.outputs {
            if o.0 == h.1 && o.2.abs() >= brain::W_EPS {
                best = best.max(h.2.abs().min(o.2.abs()));
                how.push(format!("h{}->{:?}", h.1, o.1));
            }
        }
    }
    (best, how)
}

/// Is there any live path *into* this output?
fn output_paths(w: &brain::Wiring, output: BrainOutput) -> (f32, Vec<String>) {
    let mut best = 0.0f32;
    let mut how = Vec::new();
    for i in &w.instincts {
        if i.1 as usize == output as usize && i.2.abs() >= brain::W_EPS {
            best = best.max(i.2.abs());
            how.push(format!("{:?} {:+.2}", i.0, i.2));
        }
    }
    for o in &w.outputs {
        if o.1 as usize != output as usize || o.2.abs() < brain::W_EPS {
            continue;
        }
        let fed = w.hidden.iter().any(|h| h.1 == o.0 && h.2.abs() >= brain::W_EPS) || w.recurrence.iter().any(|r| r.0 == o.0 && r.1.abs() >= brain::W_EPS);
        if fed {
            best = best.max(o.2.abs());
            how.push(format!("h{}", o.0));
        }
    }
    (best, how)
}

struct Row {
    name: String,
    readers: Vec<(f32, Vec<String>)>,
    writers: Vec<(f32, Vec<String>)>,
}

fn audit() -> Vec<Row> {
    let reg = SpeciesRegistry::builtin();
    let mut rows = Vec::new();
    for id in 0..reg.len() {
        let sp = reg.get(SpeciesId(id as u16));
        // `genome` lives on `Species`, not `CreatureDef` -- it is the
        // authored wiring expanded once at load. The `creature` test is
        // still the one that says "this is an animal": a plant has a
        // genome slot and no brain to put in it.
        if sp.creature.is_none() {
            continue;
        }
        let w = brain::wiring_from_genome(&sp.genome);
        rows.push(Row {
            name: sp.name.clone(),
            readers: READERS.iter().map(|(i, _)| input_paths(&w, *i)).collect(),
            writers: WRITERS.iter().map(|(o, _)| output_paths(&w, *o)).collect(),
        });
    }
    rows
}

fn mark(best: f32) -> &'static str {
    if best >= brain::W_EPS {
        " y "
    } else {
        " . "
    }
}

fn main() {
    if std::env::args().any(|a| a == "selftest") {
        selftest();
        return;
    }
    let rows = audit();
    println!("pheromone wiring audit -- {} species with a brain", rows.len());
    println!();
    println!("`y` = a live path exists in the AUTHORED genome (both legs >= W_EPS = {}).", brain::W_EPS);
    println!("`.` = the slot is computed every tick and reaches no output in this species.");
    println!();

    let head: Vec<&str> = READERS.iter().map(|(i, _)| slot_abbr(&format!("{i:?}"))).chain(WRITERS.iter().map(|(o, _)| slot_abbr(&format!("{o:?}")))).collect();
    print!("{:<18}", "species");
    for h in &head {
        print!("{h:>6}");
    }
    println!();
    for r in &rows {
        print!("{:<18}", r.name);
        for (best, _) in r.readers.iter().chain(r.writers.iter()) {
            print!("{:>6}", mark(*best));
        }
        println!();
    }

    println!();
    println!("--- per slot, across every species with a brain ---");
    println!();
    for (n, (input, what)) in READERS.iter().enumerate() {
        let live: Vec<&str> = rows.iter().filter(|r| r.readers[n].0 >= brain::W_EPS).map(|r| r.name.as_str()).collect();
        println!("{:<14} {:<26} {} of {} species", format!("{input:?}"), what, live.len(), rows.len());
        if live.is_empty() {
            println!("{:<14} NO SPECIES READS THIS. Computed every tick, reaches nothing.", "");
        } else {
            let (_, how) = &rows.iter().find(|r| r.readers[n].0 >= brain::W_EPS).unwrap().readers[n];
            println!("{:<14} e.g. {}", "", how.join(", "));
        }
    }
    for (n, (output, what)) in WRITERS.iter().enumerate() {
        let live: Vec<&str> = rows.iter().filter(|r| r.writers[n].0 >= brain::W_EPS).map(|r| r.name.as_str()).collect();
        println!("{:<14} {:<26} {} of {} species", format!("{output:?}"), what, live.len(), rows.len());
        let dead: Vec<&str> = rows.iter().filter(|r| r.writers[n].0 < brain::W_EPS).map(|r| r.name.as_str()).collect();
        if !dead.is_empty() {
            println!("{:<14} never written by: {}", "", dead.join(", "));
        }
    }

    println!();
    println!("--- the asymmetries, which is what this is for ---");
    println!();
    for r in &rows {
        let reads_trail = r.readers[..6].iter().any(|(b, _)| *b >= brain::W_EPS);
        let writes_trail = r.writers.iter().any(|(b, _)| *b >= brain::W_EPS);
        let reads_alarm = r.readers[6].0 >= brain::W_EPS;
        if writes_trail && !reads_trail {
            println!("  {:<16} lays a trail and reads none -- a writer with no reader.", r.name);
        }
        if reads_trail && !writes_trail {
            println!("  {:<16} reads a trail and lays none -- a parasite, or an oversight.", r.name);
        }
        if !reads_alarm {
            println!("  {:<16} DOES NOT READ THE ALARM PLANE. Bites write it; nothing here acts on it.", r.name);
        }
    }
}

fn slot_abbr(s: &str) -> &'static str {
    match s {
        "PheroAFront" => "Afrnt",
        "PheroALateral" => "Alat",
        "PheroAAlong" => "Alng",
        "PheroBFront" => "Bfrnt",
        "PheroBLateral" => "Blat",
        "PheroBAlong" => "Blng",
        "Alarm" => "alarm",
        "EmitA" => "layA",
        "EmitB" => "layB",
        _ => "?",
    }
}

/// **The positive control.** Every "." in the table above is a null, and a
/// null looks the same whether the slot is genuinely unwired or the walk
/// never reached it — `CLAUDE.md`'s most-repeated failure. So: construct
/// the case whose answer is known.
///
/// * A hand-built wiring with a known two-hop path must read live
///   (sensitivity: the walk can see through a hidden unit at all). If this
///   fails, every `y` in the table is luck and every `.` is meaningless.
/// * A path whose second leg is under `W_EPS` must read dead — the walk
///   must apply the same threshold `eval_brain` does, at both legs.
/// * An empty wiring must read dead everywhere (specificity).
fn selftest() {
    println!("pherowire selftest");
    let mut fail = 0;

    let empty = brain::Wiring::default();
    let (b, _) = input_paths(&empty, BrainInput::PheroAAlong);
    println!("  empty wiring, PheroAAlong ......... {b:.3} (want 0)");
    fail += usize::from(b >= brain::W_EPS);

    // A two-hop path the walk must see: input -> hidden 0 -> Move.
    let two_hop = brain::Wiring {
        hidden: vec![brain::HiddenWire(BrainInput::PheroAFront, 0, 6.0)],
        outputs: vec![brain::OutputWire(0, BrainOutput::Move, 2.5)],
        ..Default::default()
    };
    let (b, how) = input_paths(&two_hop, BrainInput::PheroAFront);
    println!("  two-hop path ...................... {b:.3} via {how:?} (want >= W_EPS)");
    fail += usize::from(b < brain::W_EPS);

    // The same path with a dead second leg must NOT count.
    let broken_leg = brain::Wiring {
        hidden: vec![brain::HiddenWire(BrainInput::PheroAFront, 0, 6.0)],
        outputs: vec![brain::OutputWire(0, BrainOutput::Move, 0.001)],
        ..Default::default()
    };
    let (b, _) = input_paths(&broken_leg, BrainInput::PheroAFront);
    println!("  two-hop, second leg under W_EPS ... {b:.3} (want 0 -- eval_brain would not fire it)");
    fail += usize::from(b >= brain::W_EPS);

    // And the real corpus must not be uniformly anything: a table that is
    // all-live or all-dead is the tell that the walk collapsed.
    let rows = audit();
    let total: usize = rows.iter().map(|r| r.readers.iter().filter(|(b, _)| *b >= brain::W_EPS).count()).sum();
    let cells = rows.len() * READERS.len();
    println!("  live reader cells in the corpus ... {total} of {cells} (want neither 0 nor all)");
    fail += usize::from(total == 0 || total == cells);

    println!();
    println!("{}", if fail == 0 { "selftest: PASS" } else { "selftest: FAIL" });
    if fail > 0 {
        std::process::exit(1);
    }
}
