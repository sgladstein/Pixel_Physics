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
//! # All four operators, and the operator is the shipped one
//!
//! This harness originally mutated its own copy of the rule table with its own
//! point-mutation code, which measured *retargeting a cell-type field* and
//! nothing else — the one operator of four that `FateGenome::mutate` actually
//! performs. The recorded 92% (woody) and 97% (determinate) are that operator.
//! The other three — recondition, insert, delete — shipped with **no viability
//! gate at all**.
//!
//! Two things changed, and the second matters more than the first. The gate now
//! covers all four; and it reaches them by calling `FateGenome`'s own operators
//! rather than by reimplementing them, because the harness's version had
//! quietly diverged from the shipped one. It drew from six cell types on the
//! woody base where the real operator draws from eight on every base, so its
//! number was about a mutation nothing in the engine performs. Measuring a
//! lookalike of the mechanism is `CLAUDE.md`'s counter-that-counted-calls with
//! the counter written in Rust.
//!
//! `op=` forces one operator so each can be measured at its own N; `op=all`
//! (the default) reproduces the shipped 60/15/15/10 mixture.
//!
//! ```text
//! cargo run --release --example fate_viability -- mutants=60 frames=12000
//! cargo run --release --example fate_viability -- base=herb op=insert mutants=40
//! ```

mod common;

use pixel_physics::sim::organism::{self, CellType, Fate, FateGenome, FateOp, FateWhen};
use pixel_physics::sim::parallel;
use pixel_physics::sim::rng;
use pixel_physics::sim::world::World;

/// The species the mutants are variants of.
///
/// **Two, and which one is in force decides what the run means.** `tree` is
/// the indeterminate woody base the recorded 92% was measured on and is still
/// the default, so that figure keeps meaning what it meant. `herb` is the
/// determinate base added with the organ package, and it exists because
/// `tree.ron` cannot answer the organ question at all: it declares no organ
/// materials, no `Ripen` behaviour and no `Ripe` rule, so a mutant that
/// pointed a `becomes` at `Flower` there would grow a wood-coloured cell that
/// never ripens and read as a dead end that is really three missing lines of
/// `.ron`.
const TREE_RON: &str = include_str!("../assets/species/tree.ron");
const HERB_RON: &str = include_str!("../assets/species/herb.ron");

/// Which base this run mutates, from `base=tree|herb`.
#[derive(Clone, Copy, PartialEq)]
enum Base {
    Tree,
    Herb,
}

impl Base {
    fn source(self) -> &'static str {
        match self {
            Base::Tree => TREE_RON,
            Base::Herb => HERB_RON,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Base::Tree => "tree",
            Base::Herb => "herb",
        }
    }
    /// Whether this base can actually *express* an organ.
    ///
    /// **The shipped operator does not ask, and that is the correction this
    /// harness needed.** `organism::PLANT_CELL_TYPES` — the set
    /// `FateGenome`'s operators draw from — carries `Flower` and `Fruit` on
    /// every base, so a `tree` lineage can be handed a rule that builds one.
    /// `tree.ron` declares no organ materials, no `Ripen` behaviour and no
    /// `Ripe` rule, so that organ is built out of wood, never ripens, and
    /// reads as a dead end that is really three missing lines of `.ron`.
    ///
    /// This harness used to *prevent* that, by narrowing its own draw set to
    /// six types on the woody base. That made the number readable and made it
    /// the wrong number — it measured an operator nothing ships. The draw set
    /// is the shipped one now, and mutants reaching an organ on a base that
    /// cannot express one are **counted and reported separately** instead:
    /// the confound is named rather than designed out.
    fn can_express_organs(self) -> bool {
        match self {
            Base::Tree => false,
            Base::Herb => true,
        }
    }
}

/// Every cell type's name, for the RON writer and the per-mutant log line.
///
/// The creature types are here only because `CellType` carries them; the
/// mutation operator's own draw set (`organism::PLANT_CELL_TYPES`) excludes
/// `Head` and `Segment`, so a fate can never be pointed at one.
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
        CellType::Flower => "Flower",
        CellType::Fruit => "Fruit",
    }
}

fn when_name(w: FateWhen) -> &'static str {
    match w {
        FateWhen::Grew => "Grew",
        FateWhen::Node => "Node",
        FateWhen::Stale => "Stale",
        FateWhen::Flush => "Flush",
        FateWhen::Ripe => "Ripe",
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
fn base_table(base: Base) -> Table {
    // `after_metamers: None` on every rule this builds *except* the herb
    // base's determinate one. A determinacy mutation -- varying the number
    // rather than a cell type -- is a different experiment and is deliberately
    // not folded in here; what the herb base changes is the *vocabulary* the
    // draw may reach, and the determinate rule is carried across only because
    // the control has to reproduce the shipped plant.
    let f = |when, becomes, child, lateral| Fate { when, becomes, child, lateral, after_metamers: None };
    let at = |when, becomes, n| Fate { when, becomes, child: None, lateral: None, after_metamers: Some(n) };
    if base == Base::Herb {
        // Must agree with `herb.ron` rule for rule, for the reason this
        // function's doc gives: the positive control is grown from *this*
        // table, so drift shows up as the control failing rather than as a
        // silent shift in what "unmutated" means.
        return vec![
            (
                CellType::GrowingTip,
                vec![
                    at(FateWhen::Node, CellType::Flower, 8),
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
            (CellType::Flower, vec![f(FateWhen::Ripe, CellType::Fruit, None, None)]),
            (CellType::Fruit, vec![f(FateWhen::Ripe, CellType::Seed, None, None)]),
        ];
    }
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
            if let Some(n) = r.after_metamers {
                s.push_str(&format!(", after_metamers: Some({n})"));
            }
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
fn variant_ron(base: Base, name: &str, t: &Table) -> String {
    let src = base.source();
    let start = src.find("    fates: [").expect("the base species declares a fates block");
    let mut depth = 0usize;
    let mut end = start;
    for (i, c) in src[start..].char_indices() {
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
    let end = src[end..].find('\n').map_or(end, |n| end + n + 1);
    let mut out = String::with_capacity(src.len() + 512);
    out.push_str(&src[..start]);
    out.push_str(&table_to_ron(t));
    out.push_str(&src[end..]);
    // Rename so the variant does not replace the base in the registry.
    let quoted = format!("name: \"{name}\"");
    out.replacen(&format!("name: \"{}\"", base.name()), &quoted, 1)
}

/// Apply one mutation **with the shipped operator**, reporting which operator
/// ran, whether it changed anything, and what moved.
///
/// **It routes through `FateGenome` rather than reimplementing the operator,
/// and the version this replaced is why.** The harness used to mutate its own
/// `Table` with its own point-mutation code: one field of one rule, redrawn
/// from a six- or eight-type set chosen per base. The operator that actually
/// ships does four different things, draws from a fixed eight-type set on
/// *every* base, and can decline. A gate measuring a lookalike of the operator
/// is `CLAUDE.md`'s counter-that-counted-calls with the counter written in
/// Rust — every number arithmetically right, and about a mutation nothing in
/// the engine performs.
///
/// The round trip is genome -> mutate -> table -> RON -> founder genome.
/// `FateGenome::to_table` regroups by owner, which preserves every answer
/// `fate()` can give, and `a_genome_survives_the_round_trip_through_a_table`
/// is the guard over that claim rather than this comment.
fn mutate(t: &mut Table, forced: Option<FateOp>, rng: &mut rng::Rng) -> (FateOp, bool, String) {
    let before = FateGenome::from_table(t);
    let mut after = before;
    let (op, applied) = match forced {
        Some(op) => (op, after.apply(op, rng)),
        // Only returns `None` for an empty genome, which no base has.
        None => match after.mutate(rng) {
            Some(m) => (m.op, m.applied),
            None => return (FateOp::Retarget, false, "(empty genome)".to_string()),
        },
    };
    *t = after.to_table();
    (op, applied, describe_delta(before, after))
}

/// Render what one mutation moved, by diffing the two genomes.
///
/// Read off the genomes rather than reported by the operator: the operator
/// knows which *branch* it took, and this says which rule ended up different —
/// the half a reader needs to judge why a lineage died.
fn describe_delta(before: FateGenome, after: FateGenome) -> String {
    let b: Vec<_> = before.rules().collect();
    let a: Vec<_> = after.rules().collect();
    let at = (0..b.len().min(a.len())).find(|&i| b[i] != a[i]);
    match a.len().cmp(&b.len()) {
        std::cmp::Ordering::Greater => {
            let i = at.unwrap_or(b.len());
            format!("+[{i}] {}", rule_str(a[i]))
        }
        std::cmp::Ordering::Less => {
            let i = at.unwrap_or(a.len());
            format!("-[{i}] {}", rule_str(b[i]))
        }
        std::cmp::Ordering::Equal => match at {
            Some(i) => format!("[{i}] {} -> {}", rule_str(b[i]), rule_str(a[i])),
            None => "(declined -- genome unchanged)".to_string(),
        },
    }
}

fn rule_str((owner, f): (Option<CellType>, Option<Fate>)) -> String {
    let Some(owner) = owner else { return "?".to_string() };
    let Some(f) = f else { return format!("{}.<undecodable>", type_name(owner)) };
    let mut s = format!("{}.{}>{}", type_name(owner), when_name(f.when), type_name(f.becomes));
    if let Some(c) = f.child {
        s.push_str(&format!(" c:{}", type_name(c)));
    }
    if let Some(l) = f.lateral {
        s.push_str(&format!(" l:{}", type_name(l)));
    }
    if let Some(n) = f.after_metamers {
        s.push_str(&format!(" @{n}"));
    }
    s
}

/// Does this table name an organ cell type anywhere?
///
/// The confound `Base::can_express_organs` describes, counted rather than
/// designed out: on the woody base such a mutant is measuring `tree.ron`'s
/// missing organ configuration at least as much as the substrate's tolerance,
/// and a rate that folds those in silently is a rate about two things.
fn reaches_organ(t: &Table) -> bool {
    let organ = |c: Option<CellType>| matches!(c, Some(CellType::Flower) | Some(CellType::Fruit));
    t.iter().any(|(ct, rules)| {
        organ(Some(*ct)) || rules.iter().any(|f| organ(Some(f.becomes)) || organ(f.child) || organ(f.lateral))
    })
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

/// Per-operator outcome counts.
///
/// **`declined` and `silent` are different failures and both have to be
/// visible.** `declined` means the operator itself made no change — a redraw
/// that never found a different value, a `delete` at the one-rule floor, an
/// `insert` at `MAX_FATES`. `silent` means the genome *did* change and the
/// stand still came out identical to the base, so the field it moved is never
/// read in this scene. Every declined mutant is necessarily silent; the
/// reverse does not hold, and folding the two together bills the substrate for
/// the operator's own no-ops.
///
/// `established` and `reproduced` count **effective** mutants only — a silent
/// mutant establishes because the base does, and counting that as tolerance is
/// quoting the positive control back as a result.
#[derive(Default, Clone, Copy)]
struct Tally {
    drawn: usize,
    declined: usize,
    silent: usize,
    established: usize,
    reproduced: usize,
    organ: usize,
}

fn main() {
    let arg = |k: &str, d: u64| {
        std::env::args().find_map(|a| a.strip_prefix(k).map(|v| v.parse().expect(k))).unwrap_or(d)
    };
    let mutants = arg("mutants=", 40) as usize;
    let frames = arg("frames=", 12000);
    let founders = arg("founders=", 3) as usize;
    let worldseed = arg("worldseed=", 7);
    // **The base is echoed, and it has to be.** `CLAUDE.md`'s harness rule,
    // paid for by a 3.5-hour study that turned out to be three populations
    // wearing 24 logs: a log that does not name its own parameters was written
    // by a binary that never had them. The draw set is echoed for the same
    // reason -- the whole difference between the two runs is which cell types
    // a mutation may reach, and that is invisible in every other line.
    let base = match std::env::args().find_map(|a| a.strip_prefix("base=").map(str::to_string)).as_deref() {
        None | Some("tree") => Base::Tree,
        Some("herb") => Base::Herb,
        Some(other) => panic!("unknown base {other:?} (tree|herb)"),
    };
    // **`op=` forces one operator, and the gate needs it.** `mutate` picks
    // 60/15/15/10, so 40 mutants spend about four draws on `delete` -- too few
    // to gate on. Forcing measures each operator at its own N; `op=all`
    // reproduces the shipped mixture and is the default.
    let forced = match std::env::args().find_map(|a| a.strip_prefix("op=").map(str::to_string)).as_deref() {
        None | Some("all") => None,
        Some("retarget") => Some(FateOp::Retarget),
        Some("recondition") => Some(FateOp::Recondition),
        Some("insert") => Some(FateOp::Insert),
        Some("delete") => Some(FateOp::Delete),
        Some(other) => panic!("unknown op {other:?} (all|retarget|recondition|insert|delete)"),
    };
    println!(
        "fate_viability: base={} op={} mutants={mutants} frames={frames} founders={founders} worldseed={worldseed}",
        base.name(),
        forced.map_or("all (the shipped 60/15/15/10 mixture)", FateOp::name)
    );
    println!(
        "fate_viability: operator is organism::FateGenome's own, drawing cell types from [{}]",
        organism::PLANT_CELL_TYPES.iter().map(|&t| type_name(t)).collect::<Vec<_>>().join(", ")
    );
    if !base.can_express_organs() {
        println!("fate_viability: NOTE {} declares no organ material, no Ripen behaviour and no Ripe", base.name());
        println!("fate_viability:      rule, so a mutant reaching Flower/Fruit measures that gap as well");
        println!("fate_viability:      as the substrate. They have their own column below.");
    }

    // --- positive control: the unmutated table ---
    let base_t = base_table(base);
    let (be, bs) = trial(&variant_ron(base, "fv_base", &base_t), "fv_base", frames, founders, worldseed);
    println!("\npositive control (unmutated table): {be}/{founders} established, {bs} seeds set");

    // --- negative control: a table that must be dead ---
    //
    // **Its index is found rather than hardcoded**, because the herb base
    // carries a determinate rule ahead of the ordinary `Grew` one and
    // `[0].1[1]` names a different rule in the two tables. A negative control
    // that silently mutates the wrong rule is a control that proves nothing --
    // and it would fail *open*, reading as "even the lethal mutation lived".
    let mut lethal = base_table(base);
    let grew = lethal[0]
        .1
        .iter_mut()
        .find(|f| f.when == FateWhen::Grew)
        .expect("the base's shoot has a Grew rule to poison");
    grew.child = Some(CellType::Seed);
    let (le, ls) = trial(&variant_ron(base, "fv_lethal", &lethal), "fv_lethal", frames, founders, worldseed);
    println!("negative control (shoot child -> Seed):  {le}/{founders} established, {ls} seeds set");

    if be == 0 {
        println!("\nSTOP: the positive control did not establish. Every mutant number below would be");
        println!("measuring this harness -- the bed, the frame budget or the species -- and not the");
        println!("mutation. Fix the control before reading anything else.");
        return;
    }

    // --- the mutants ---
    //
    // **Four outcomes, not two, and the two extra ones are what a naive rate
    // hides.** A *declined* mutation never changed the genome, so the plant
    // that grows is the base plant. A *silent* one changed it and the stand
    // came out identical anyway, so nothing read the field it moved --
    // measured on the first run of this harness, `RootTip.Grew.lateral`
    // pointed at four different cell types produced exactly the base's 80
    // seeds every time, because a root never takes the lateral path in this
    // scene. Both are the identical-output-across-settings tell `CLAUDE.md`
    // names for a knob that was never connected, and counting either as
    // "viable" inflates the headline with cases that could not have failed.
    let mut per: [Tally; 4] = [Tally::default(); 4];
    let op_index = |o: FateOp| FateOp::ALL.iter().position(|&x| x == o).expect("FateOp::ALL is exhaustive");
    // **Printed as they land, not buffered to the end.** An earlier version
    // collected every line and printed at the finish, which meant a 25-minute
    // run produced no output at all -- indistinguishable, while you are
    // watching it, from a hung one. A long harness that cannot show progress
    // is one nobody can tell is working.
    println!("\nper mutation (established of {founders} founders, and seeds set):");
    for i in 0..mutants {
        let mut t = base_table(base);
        let mut r = rng::stream(worldseed, 0xF8, i as u64, 0);
        let (op, applied, what) = mutate(&mut t, forced, &mut r);
        let organ = !base.can_express_organs() && reaches_organ(&t);
        let name = format!("fv_{i}");
        let (e, s) = trial(&variant_ron(base, &name, &t), &name, frames, founders, worldseed);
        let quiet = e == be && s == bs;
        let k = op_index(op);
        per[k].drawn += 1;
        if !applied {
            per[k].declined += 1;
        }
        if quiet {
            per[k].silent += 1;
        } else {
            if e > 0 {
                per[k].established += 1;
            }
            if s > 0 {
                per[k].reproduced += 1;
            }
            if organ {
                per[k].organ += 1;
            }
        }
        let flag = if !applied {
            "  [declined: the operator changed nothing]"
        } else if quiet {
            "  [silent: identical to base]"
        } else if organ {
            "  [reaches an organ this base cannot ripen]"
        } else {
            ""
        };
        println!("  {:<11} {:<44} plants {e:>2}  seeds {s:>3}{flag}", op.name(), what);
    }
    let pct = |n: usize, d: usize| if d == 0 { f32::NAN } else { 100.0 * n as f32 / d as f32 };
    let row = |label: &str, t: Tally| {
        let eff = t.drawn - t.silent;
        println!(
            "  {:<12} {:>5} {:>9} {:>7} {:>10}   {:>3} ({:>3.0}%)   {:>3} ({:>3.0}%)",
            label,
            t.drawn,
            t.declined,
            t.silent,
            eff,
            t.established,
            pct(t.established, eff),
            t.reproduced,
            pct(t.reproduced, eff)
        );
    };
    println!("\n{mutants} mutations of the production rule, by operator:");
    println!(
        "  {:<12} {:>5} {:>9} {:>7} {:>10}   {:>11}   {:>11}",
        "operator", "drawn", "declined", "silent", "EFFECTIVE", "established", "set a seed"
    );
    let mut tot = Tally::default();
    for op in FateOp::ALL {
        let t = per[op_index(op)];
        if t.drawn == 0 {
            continue;
        }
        row(op.name(), t);
        tot.drawn += t.drawn;
        tot.declined += t.declined;
        tot.silent += t.silent;
        tot.established += t.established;
        tot.reproduced += t.reproduced;
        tot.organ += t.organ;
    }
    row("ALL", tot);
    if !base.can_express_organs() && tot.drawn > 0 {
        println!(
            "\n  {} of the {} effective mutants reach Flower/Fruit on a base that declares no organ",
            tot.organ,
            tot.drawn - tot.silent
        );
        println!("  material, no Ripen behaviour and no Ripe rule. Those measure that gap as well as");
        println!("  the substrate -- read `base=herb` for the same operators where an organ can ripen.");
    }
    // **One world seed, and that bounds what may be concluded.** A viability
    // *rate* is legitimate here: the sample is the mutations, not the seeds,
    // and a mutation that destroys the frontier destroys it at any seed. A
    // *fitness comparison between arms* is not: twelve identical genomes span
    // 31 to 153 cells in this engine, so an arm reading 109 seeds against the
    // base's 80 is one sample from a wide distribution and is a hypothesis,
    // not a result. Comparing arms needs an order statistic over seeds --
    // which is gate 3's experiment, not this one's.
    println!("\nONE world seed. The rate above is a rate over mutations and stands;");
    println!("the per-arm seed counts are NOT comparable between arms -- within-genome");
    println!("spread here runs 31-153 cells, so a higher count is a hypothesis.");
    println!("\nRead against the controls, never alone: base {be} plants / {bs} seeds,");
    println!("lethal {le} plants / {ls} seeds. `plants` counts every established");
    println!("organism including recruits, so it can exceed the {founders} founders.");
}
