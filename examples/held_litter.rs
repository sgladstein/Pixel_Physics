//! **Does a held world stop the forest floor from rotting?**
//!
//! Owner report, 2026-09-15: *"there seems to be way more leaves piling up on
//! the ground in the druid game"* than in the evolution lab, which he expected
//! to be identical. The species files and the material files are the same set
//! in both games (`bin/lab.rs` and `druid::Druid::new` both `reload` the same
//! two asset dirs), so the rules are not where the two differ. This harness
//! asks whether the **held world** is.
//!
//! The suspicion is structural rather than a tuning difference:
//!
//!   - **Litter falls everywhere.** Nothing in `update.rs` or `parallel.rs`
//!     reads `World::held`, so the CA sweep moves a shed leaf to the floor
//!     whether or not time runs there.
//!   - **Litter only *rots* where time runs.** `decay::tick` is dispatched
//!     from `scheduler::step`, which bounces every site failing
//!     `World::time_runs_at` back onto the heap at `HELD_RECHECK` and never
//!     runs it.
//!
//! So a held world is a floor with a source and no sink outside the druid's
//! own circle — the exact shape `litter.ron`'s own note calls "an accumulator
//! instead of a cycle". The lab never sets `held`, so it never has this.
//!
//! **Three arms on one bed**, cloned after a shared grow phase so every arm
//! stands on the same ground (`World` is `Clone`; `druid_garden`'s header
//! says it is not, which was true when that file was written):
//!
//! - `running` — `held = false`, the evolution lab's regime.
//! - `quickened` — `held = true`, one quickening over the whole stand for the
//!   whole run. **This is a control, not a result**: a circle covering every
//!   cell makes `time_runs_at` true everywhere, so this arm must come back
//!   *identical* to `running`. If it does not, the two arms differ by
//!   something other than the gate and nothing below means what it says.
//! - `departed` — `held = true`, the same quickening for the first half of the
//!   run and then taken away: the druid walking off.
//!
//! **`senescent=1` is the druid's own default start**, not a flourish:
//! `druid::Start::Dead` grows a wood for 8,000 frames and then marks every
//! plant senescent, and `plant::rot_remains` carries a senescent body out
//! through `shed_to_litter` — *every* cell of it, trunk as well as leaf. So
//! the thing the druid quickens is a standing dead wood converting itself
//! into floor.
//!
//! ```text
//! cargo run --release --example held_litter
//! cargo run --release --example held_litter -- grow=8000 frames=12000 senescent=1
//! cargo run --release --example held_litter -- selftest
//! ```
//!
//! `selftest` is the positive control and it is not optional: it asserts the
//! `running` arm removes litter this harness can see, because an arm that
//! reports "nothing rotted" from a world where nothing *could* rot is
//! indistinguishable from the finding (`CLAUDE.md`: run the case whose answer
//! you know is non-zero).

mod common;

use pixel_physics::sim::explosion::{self, Blasts};
use pixel_physics::sim::frame;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::{Quickening, World};

/// Standing cells of one material, read off the grid rather than off any
/// event counter -- the grid is what the picture is drawn from, so the census
/// cannot disagree with what the owner saw.
fn count(world: &World, name: &str) -> usize {
    let Some(id) = world.materials.id_of(name) else { return 0 };
    let b = world.bounds().expect("bounded world");
    let mut n = 0;
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            if world.get(x, y).material == id {
                n += 1;
            }
        }
    }
    n
}

/// Living plant tissue, so an arm that simply has no canopy left cannot be
/// read as an arm whose floor rotted away.
fn plant_cells(world: &World) -> usize {
    use pixel_physics::sim::material::MaterialKind;
    let b = world.bounds().expect("bounded world");
    let mut n = 0;
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = world.get(x, y);
            if world.materials.kind(c.material) == MaterialKind::Plant {
                n += 1;
            }
        }
    }
    n
}

struct Arm {
    label: &'static str,
    held: bool,
    /// Frames the quickening stands before it is taken away; `u64::MAX` keeps it.
    leaves_at: u64,
    /// The three plant-rule switches the two games set differently, as each
    /// game sets them. `None` leaves `World::new`'s own value.
    ///
    /// **They are a set, not three knobs.** Running them one at a time
    /// answers "what does this switch do"; the owner's question is "why do my
    /// two games differ", and the two games differ by all three at once.
    size_cadence: Option<bool>,
    bending: Option<bool>,
    load_failure: Option<bool>,
}

impl Arm {
    fn plain(label: &'static str, held: bool, leaves_at: u64) -> Self {
        Arm { label, held, leaves_at, size_cadence: None, bending: None, load_failure: None }
    }
    /// `bin/lab.rs`'s box: `scene::LabBox::build_counted` sets the first two,
    /// nothing touches the third, and the lab never holds.
    fn lab() -> Self {
        Arm {
            label: "lab",
            held: false,
            leaves_at: u64::MAX,
            size_cadence: Some(true),
            bending: Some(false),
            load_failure: Some(true),
        }
    }
    /// `druid::Druid::new`: it sets only `plant_load_failure`, leaves the
    /// other two at the engine default, and holds.
    fn druid(leaves_at: u64) -> Self {
        Arm {
            label: "druid",
            held: true,
            leaves_at,
            size_cadence: Some(false),
            bending: Some(true),
            load_failure: Some(false),
        }
    }
}

fn run(arm: &Arm, base: &World, frames: u64, stops: u64, ground: i32, width: i32) -> (usize, usize, usize, Option<usize>) {
    let mut w = base.clone();
    w.held = arm.held;
    if let Some(v) = arm.size_cadence {
        w.plant_size_cadence = v;
    }
    if let Some(v) = arm.bending {
        w.plant_bending = v;
    }
    if let Some(v) = arm.load_failure {
        w.plant_load_failure = v;
    }
    // One circle wide enough to cover the whole stand, so "quickened" and
    // "not quickened" is the only thing separating the arms -- not which part
    // of the bed happened to fall inside a play-sized radius.
    let circle = Quickening::at(width / 2, ground - 20, width);
    if arm.held {
        w.quickenings = vec![circle];
    }
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::with_tuning(explosion::Tuning::load());
    let tuning = player::Tuning::load();
    let start = w.frame;
    let every = (frames / stops).max(1);
    let mut next = every;
    // The peak matters as much as the endpoint: the running arm's floor is a
    // *cycle* (it rises while the dead wood rots, then falls as the litter
    // weathers), and only the peak-to-endpoint drop shows the sink exists at
    // all. An endpoint on its own cannot tell a floor that cleared from a
    // floor that never filled.
    let mut peak = count(&w, "litter");
    // What the floor held the moment the circle left. The freeze is the
    // claim, and an endpoint alone cannot state it: whether a frozen floor is
    // *higher* than a running one depends entirely on where in its own
    // rise-and-fall the run happened to stop, which is `CLAUDE.md`'s
    // censused-before-it-settles trap wearing a different hat. "It did not
    // move after she left" is true at every budget.
    let mut at_departure = None;
    println!("  {:<10} litter {:>6}  plant {:>6}   (frame {})", arm.label, peak, plant_cells(&w), 0);
    while w.frame - start < frames {
        if arm.held && w.frame - start >= arm.leaves_at && !w.quickenings.is_empty() {
            w.quickenings.clear();
            at_departure = Some(count(&w, "litter"));
            println!(
                "  {:<10} ... the circle is taken away at frame {} ({} litter standing)",
                arm.label,
                w.frame - start,
                at_departure.expect("just set")
            );
        }
        frame::step(&mut w, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        peak = peak.max(count(&w, "litter"));
        if w.frame - start >= next {
            println!(
                "  {:<10} litter {:>6}  plant {:>6}   (frame {})",
                arm.label,
                count(&w, "litter"),
                plant_cells(&w),
                w.frame - start
            );
            next += every;
        }
    }
    (count(&w, "litter"), plant_cells(&w), peak, at_departure)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |k: &str| args.iter().find_map(|a| a.strip_prefix(k).map(str::to_string));
    let selftest = args.iter().any(|a| a == "selftest");

    let grow: u64 = arg("grow=").map_or(if selftest { 4_000 } else { 8_000 }, |v| v.parse().expect("grow=N"));
    let frames: u64 = arg("frames=").map_or(if selftest { 6_000 } else { 12_000 }, |v| v.parse().expect("frames=N"));
    let stops: u64 = arg("stops=").map_or(4, |v| v.parse().expect("stops=N"));
    let trees: usize = arg("trees=").map_or(8, |v| v.parse().expect("trees=N"));
    let senescent = arg("senescent=").is_none_or(|v| v != "0");

    let scene = common::PlantScene { trees, ..common::PlantScene::default() };
    let (ground, width) = (scene.ground_y, scene.width);
    let mut base = scene.build();
    if let Some(seed) = arg("worldseed=") {
        base.seed = seed.parse().expect("worldseed=N");
    }
    println!(
        "held_litter: trees={trees} grow={grow} frames={frames} senescent={senescent} worldseed={} ground={ground}",
        base.seed
    );

    // --- the shared grow phase, unheld, exactly as `Druid::new` runs it ----
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::with_tuning(explosion::Tuning::load());
    let tuning = player::Tuning::load();
    for _ in 0..grow {
        frame::step(&mut base, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
    }
    // **The gate, not decoration.** A stand that never grew makes every number
    // below it void, and `litter 0` from a bare bed is indistinguishable from
    // `litter 0` from a working one.
    let grown = base.live_organism_count();
    println!("grown: {grown} organisms, {} plant cells, {} litter", plant_cells(&base), count(&base, "litter"));
    assert!(grown > 0 && plant_cells(&base) > 0, "nothing grew -- every arm below would be void");

    if senescent {
        let ids: Vec<_> = base
            .live_organism_ids()
            .into_iter()
            .filter(|id| base.organism(*id).is_some_and(|st| base.species.get(st.species).creature.is_none()))
            .collect();
        let killed = ids.iter().filter(|id| base.mark_organism_senescent(**id)).count();
        println!("marked {killed} of {grown} plants senescent -- the wood is standing dead");
    }

    // `profile=1` is the two games as they ship, on one bed: every switch
    // each binary sets, set the way it sets it. The default three arms are
    // the held gate on its own, which is a different question -- *what does
    // holding do* against *why do my two games differ*.
    let arms: Vec<Arm> = if arg("profile=").as_deref() == Some("ablate") {
        // **Which of the three switches is the leaf fall.** Both directions,
        // because a one-way ablation cannot tell "this switch does it" from
        // "this switch does it *given the other two*": lab with the switch
        // flipped to the druid's value, and druid with it flipped back.
        // `CLAUDE.md`'s paired-comparison rule -- everything the arms are not
        // about cancels.
        vec![
            Arm::lab(),
            Arm { label: "lab+cadence_off", size_cadence: Some(false), ..Arm::lab() },
            Arm { label: "lab+bending_on", bending: Some(true), ..Arm::lab() },
            Arm { label: "lab+breaks_off", load_failure: Some(false), ..Arm::lab() },
            Arm { label: "druid", ..Arm::druid(u64::MAX) },
            Arm { label: "druid+cadence_on", size_cadence: Some(true), ..Arm::druid(u64::MAX) },
        ]
    } else if arg("profile=").is_some_and(|v| v != "0") {
        vec![Arm::lab(), Arm::druid(u64::MAX), Arm::druid(frames / 2)]
    } else {
        vec![
            Arm::plain("running", false, u64::MAX),
            Arm::plain("quickened", true, u64::MAX),
            Arm::plain("departed", true, frames / 2),
        ]
    };
    let mut finals = Vec::new();
    for (i, arm) in arms.iter().enumerate() {
        // Two `druid` arms in profile mode differ only by whether she walks
        // away, so the label alone would name them both the same thing.
        println!(
            "\n{}{}:",
            arm.label,
            if arm.leaves_at == u64::MAX { String::new() } else { format!(" (walks away at {})", arm.leaves_at) }
        );
        let _ = i;
        let (litter, plant, peak, at_departure) = run(arm, &base, frames, stops, ground, width);
        finals.push((arm.label, litter, plant, peak, at_departure));
    }

    println!("\nstanding litter at frame {frames}:");
    for (label, litter, plant, peak, at_departure) in &finals {
        let note = match at_departure {
            Some(n) if n == litter => format!("   frozen at {n} since she left"),
            Some(n) => format!("   {n} when she left -- NOT frozen"),
            None => String::new(),
        };
        println!("  {label:<10} {litter:>6} litter   {plant:>6} plant   (peak {peak}){note}");
    }

    if selftest {
        assert_eq!(finals[0].0, "running", "the selftest's controls are written against the default three arms");
        let (running, running_peak) = (finals[0].1, finals[0].3);
        let (quickened, quickened_plant) = (finals[1].1, finals[1].2);
        let (departed, departed_at) = (finals[2].1, finals[2].4.expect("the departed arm records its departure"));
        // **Sensitivity.** The running arm's floor must actually move -- rise
        // and then fall -- or this harness cannot tell "held stops the rot"
        // from "nothing rots here", which are the two readings it exists to
        // separate.
        assert!(
            running < running_peak,
            "positive control failed: the running arm's floor never fell from its peak \
             ({running} at the end against a peak of {running_peak}), so there is no sink here to be stopped"
        );
        // **Specificity.** A circle over every cell makes `time_runs_at` true
        // everywhere, so this arm is the unheld one wearing a flag. Anything
        // else means the arms differ by more than the gate.
        assert_eq!(
            (quickened, quickened_plant),
            (finals[0].1, finals[0].2),
            "control failed: held-with-full-coverage diverged from unheld, so the arms differ by more than the gate"
        );
        // **The effect, stated so it is true at every budget.** Not
        // "departed > running" -- that compares a frozen floor against one
        // partway through its own rise and fall, and it flips sign purely
        // with the frame count. What does not flip is that the departed floor
        // stopped moving the instant the circle left.
        assert_eq!(
            departed, departed_at,
            "the departed arm's floor kept moving after the circle was taken away -- the held gate is not what this measures"
        );
        assert_ne!(running, running_peak, "the running arm must still be cycling for the contrast to mean anything");
        println!(
            "\nselftest: running falls {} from its peak {running_peak}; full coverage is bit-identical to unheld; \
             departed is frozen at {departed_at}. Sensitive, specific, and the freeze is the effect.",
            running_peak - running
        );
    }
}
