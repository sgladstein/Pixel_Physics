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
//! ```
//!
//! `lines=1` (the default) prints only the line-bounded kinds -- what the LOG
//! page's `LINES` filter shows, readable at any population; `all=1` prints
//! every hatch and death too. **The per-kind counts print last**, per
//! `CLAUDE.md`'s standing rule: a picture (or a page of prose) says what and
//! where, and only the count says whether the thing fired -- and here the
//! count is also the scale claim, line events against births and deaths.

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
    let spec = LabBox {
        founders: arg("founders").unwrap_or(LabBox::default().founders),
        colonies: arg("colonies").unwrap_or(LabBox::default().colonies),
        seed: arg("seed").unwrap_or(LabBox::default().seed),
        species: arg::<String>("plant").unwrap_or_else(|| LabBox::default().species),
        ..LabBox::default()
    };
    let mut world = spec.build();
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    println!(
        "chronicle: founders={} of {} colonies={} seed={} frames={} showing={}",
        spec.founders,
        spec.species,
        spec.colonies,
        spec.seed,
        frames,
        if all == 1 { "ALL" } else { "LINES" }
    );
    for _ in 0..frames {
        frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
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
