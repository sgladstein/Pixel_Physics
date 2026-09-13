//! **The box's chronicle, as text.** Runs a `LabBox` for `frames=N` and prints
//! the run log the way the LOG page reads it -- as sentences about named
//! lines -- oldest first, so a run can be read back as a story rather than
//! inspected as a table.
//!
//! This is the first form of "the chronicle as an export": the same
//! `format_log_line` the lab's LOG page draws through, so what prints here is
//! byte-for-byte what a player would read, and a sentence that is wrong here
//! is wrong on screen too.
//!
//! ```text
//! cargo run --release --example chronicle -- frames=6000
//! cargo run --release --example chronicle -- frames=20000 seed=3 all=1
//! cargo run --release --example chronicle -- founders=8 colonies=1 lines=1
//! cargo run --release --example chronicle -- scenario=played_bed frames=20000
//! ```
//!
//! `lines=1` (the default) prints only the line-bounded kinds -- what the LOG
//! page's `LINES` filter shows, readable at any population; `all=1` prints
//! every hatch and death too. **The per-kind counts print last**, per
//! `CLAUDE.md`'s standing rule: a picture (or a page of prose) says what and
//! where, and only the count says whether the thing fired -- and here the
//! count is also the scale claim, line events against births and deaths.
//!
//! **`scenario=<name>` builds the whole bed from a saved scenario**,
//! `labforage.rs`/`windfall_probe.rs`'s own pattern, and it is what fixes
//! this binary's own instance of `CLAUDE.md`'s "an unknown argument is
//! silently ignored": before this, `scenario=played_bed` matched no `arg()`
//! key, so it parsed to nothing and this ran the eight-founder default bed
//! under the played bed's name with no warning at all -- the coordinator hit
//! this exactly, and it is why `frames=` alone is not proof of anything: the
//! first line has to name what bed actually ran.
//!
//! **`census=1` (the default) adds the same CENSUS section the lab's own
//! chronicle export carries** (`Lab::write_chronicle`, `census::
//! chronicle_section`) -- one row every `sample=` frames (default 10,000,
//! `Lab::CHRONICLE_CENSUS_EVERY`'s own value), so a headless run and a real
//! session's saved chronicle are the same table. `census=0` drops it back
//! to the plain LOG-only export this file always was. This is the second
//! form of "the chronicle as an export"; see `examples/latecensus.rs` for
//! what each CENSUS column answers.

use pixel_physics::lab::census;
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::ui::format_log_line;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::LogKind;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(6000);
    let all: u32 = arg("all").unwrap_or(0);
    let show_census: bool = arg::<u32>("census").unwrap_or(1) != 0;
    let sample_every: u64 = arg("sample").unwrap_or(pixel_physics::lab::Lab::CHRONICLE_CENSUS_EVERY);
    // A bad name refuses at load rather than quietly running the default bed
    // under the wrong label -- see this file's own doc comment for why that
    // silence is the bug this binary just had.
    let scenario: Option<Scenario> = arg::<String>("scenario").map(|n| {
        let mut sc = Scenario::load(&n).unwrap_or_else(|e| {
            eprintln!("scenario {n}: {e}");
            std::process::exit(2);
        });
        // `seed=` overrides the scenario's own bed seed -- `labforage.rs`'s
        // identical line, for the identical reason: it has to land on
        // `sc.bed`, not on `spec` below, or a seed sweep over a scenario is
        // the same world every time.
        if let Some(sd) = arg::<u64>("seed") {
            sc.bed.seed = sd;
        }
        sc
    });
    let spec = match &scenario {
        Some(s) => s.bed.clone(),
        None => LabBox {
            founders: arg("founders").unwrap_or(LabBox::default().founders),
            colonies: arg("colonies").unwrap_or(LabBox::default().colonies),
            seed: arg("seed").unwrap_or(LabBox::default().seed),
            species: arg::<String>("plant").unwrap_or_else(|| LabBox::default().species),
            ..LabBox::default()
        },
    };
    // Echo the parameters, scenario named right here on the first line --
    // `plant_probe`'s 3.5-hour lesson (`CLAUDE.md`): a knob nobody can see
    // the value of is a knob nobody can tell is disconnected.
    println!(
        "chronicle: founders={} of {} colonies={} seed={} frames={} showing={}{}",
        spec.founders,
        spec.species,
        spec.colonies,
        spec.seed,
        frames,
        if all == 1 { "ALL" } else { "LINES" },
        scenario.as_ref().map(|s| format!(" scenario={} ({})", s.name, s.question)).unwrap_or_default()
    );
    let mut world = match &scenario {
        Some(s) => {
            let (w, _planted, placed) = s.build();
            println!(
                "  scenario {}: {} cells, {} plants, {} animals, {} settings applied",
                s.name, placed.cells, placed.plants, placed.animals, placed.settings
            );
            w
        }
        None => spec.build(),
    };
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    // **CENSUS bookkeeping, `census::nest_columns`'s own reason: a nest does
    // not move once founded**, so this is resolved once rather than
    // recomputed every sample. `gut` starts at whatever colony the bed
    // founds at build time and updates when the timeline delivers another,
    // `examples/latecensus.rs`'s identical shape.
    let ids = census::Ids::resolve(&world);
    let nest_cols = census::nest_columns(&spec, scenario.as_ref());
    let mut gut: f32 = census::ant_gut_bias(&world);
    let mut census_rows: Vec<census::ChronicleRow> = Vec::new();
    for _ in 0..frames {
        // The scenario's own timeline, before this frame's step -- a colony
        // founded this frame belongs in this frame's log, not next frame's
        // (`labforage.rs`'s identical ordering).
        if let Some(sc) = &scenario {
            let arrived = pixel_physics::lab::scenario::tick_timeline(sc, &mut world, &spec);
            if arrived.animals > 0 {
                gut = census::ant_gut_bias(&world);
            }
        }
        frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        // Right after `frame::step`, the lab's own cadence (`Lab::tick`):
        // this frame's settled state, sampled once every `sample_every`.
        if show_census && world.frame % sample_every == 0 {
            census_rows.push(census::take_chronicle_row(&world, &spec, gut, &nest_cols, &ids, &spec.colony_species));
        }
    }

    // **The CENSUS section, right after the header** -- `Lab::write_
    // chronicle`'s own ordering (`ui::chronicle_text`), so this and a real
    // session's saved chronicle read the same way top to bottom.
    if show_census {
        print!("{}", census::chronicle_section(&census_rows));
        println!();
    }

    // Newest-first on the log; a story reads the other way.
    let mut events: Vec<_> = world.run_log.recent().copied().collect();
    events.reverse();
    let mut counts = std::collections::BTreeMap::<&'static str, u32>::new();
    let mut legend = std::collections::BTreeMap::<&'static str, String>::new();
    for e in &events {
        *counts.entry(e.kind.label()).or_insert(0) += 1;
        if all != 1 && !e.kind.is_line_event() {
            continue;
        }
        let (what, _, note) = format_log_line(&world, e);
        println!("F{:>7}  {what}", e.frame);
        // The hover note is per kind, not per row; once each, at the end.
        legend.entry(e.kind.label()).or_insert(note);
    }
    if !legend.is_empty() {
        println!();
        for (kind, note) in &legend {
            println!("{kind}: {note}");
        }
    }
    let line_events: u32 = events.iter().filter(|e| e.kind.is_line_event()).count() as u32;
    let individual_events = events.len() as u32 - line_events;
    println!();
    println!(
        "counts: {} | line events {} vs individual events {} | lineages claimed {} | log dropped {}",
        counts.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(", "),
        line_events,
        individual_events,
        world.lineages_claimed(),
        world.run_log.dropped()
    );
    let _ = LogKind::Born; // the kind table is the log's own; nothing here re-derives it
}
