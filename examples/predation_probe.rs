//! **Can a beetle smell an ant?** The T6 pre-flight, before any predation
//! is wired.
//!
//! `creature-evolution-plan.md` §5 / `creature-review-2026-08.md` §T6 park
//! predation behind one question, because the answer decides whether the
//! next piece of work is a *perception* problem or a *movement* problem —
//! and those are not the same size. The measured null is that `beetles=0`
//! and `beetles=9` run **bit-identical** over 6,000 frames: a predator that
//! moves no counter is an absent one, and the standing hypothesis is that
//! the beetle simply has no way to find its prey.
//!
//! The cheap wiring available is the existing channel-B along-gradient
//! (`brain::BrainInput::PheroBAlong`) — the ant's own food trail, laid by
//! `(Carrying, EmitB, 2.5)` in `ant.ron`. Before spending a `.ron` edit on
//! it, this prints the two numbers §5 asks for:
//!
//! * **total channel-B mass** across the whole world, and
//! * **the fraction of prey heads within a sensor offset of a nonzero B
//!   cell** — i.e. does the trail actually mark where the ants are.
//!
//! If the trail barely exists where beetles are, wiring a sense to it buys
//! nothing and the predator is blocked on *movement*, which is a much
//! larger piece of work and worth knowing early.
//!
//! # Why this is a new file and not a mode on `creature_space`
//!
//! `Reports/instruments.md` was checked first. `creature_space` builds the
//! scene the null was measured in, but it is a *fitness* harness: every
//! number it prints is per-ant survival or advantage, and it deliberately
//! rewrites the ant's economy (herbivore gut, corpses off the menu) to keep
//! food scarce. `creature_probe` prints one creature's inputs per tick with
//! no aggregation and no beetles. Neither can say what the B plane looks
//! like world-wide, which is the whole question.
//!
//! # The three things this file is careful about
//!
//! 1. **A counter is paired with an effect counter from the far side of the
//!    call** (`CLAUDE.md`). "The beetle sensed something" is not "the beetle
//!    caught something". `feeds` counts beetles whose *energy rose* between
//!    samples — a beetle's only income is eating, every other term in its
//!    budget is a cost — which is the same detector `creature_space` uses
//!    for a fed ant, and it lives on the far side of the verb.
//! 2. **The positive control runs in the same binary** (`mode=control`).
//!    A pre-flight that reports zero is worthless unless it can be shown
//!    reporting nonzero. It paints a saturated trail and puts a beetle
//!    mouth-to-mouth with an ant, and asserts every counter moves.
//! 3. **The gradient-degeneracy trap is measured, not assumed**
//!    (`CLAUDE.md`: a coarse-field read is block-nearest, so neighbouring
//!    cells sample the same value — hit four times on three different
//!    lines, never once caught by a test). The pheromone planes are at CA
//!    resolution rather than `FIELD_SCALE`, so the trap should not apply —
//!    but "should not" is what the previous four believed. `differs` counts
//!    the samples in which the beetle's two B reads (underfoot and at the
//!    forward sensor) actually hold different values. A beetle whose two
//!    reads are always equal has a gradient of exactly zero and will steer
//!    on whatever tie-break follows, which is a constant direction.
//!
//! # What `differs = 0.000` does and does not prove
//!
//! **It is sampled, and a beetle ticks faster than the sampler.** At the
//! default `every=200` this looks at one tick in twenty-five
//! (`tick_interval` is 8), so a run reporting 0.000 has been *observed*
//! never to hold a gradient, not *proved* to. The claim that matters — "a
//! weight on this input would change nothing" — was therefore settled the
//! other way round as well, by authoring `(PheroBAlong, Move, 2.0)` into
//! `beetle.ron`, rebuilding (the assets are `include_str!`ed, so the
//! rebuild is the experiment) and checking whether the run moved.
//!
//! **It moved on exactly the two seeds whose sampled `differs` was nonzero
//! (0 and 3) and on none of the other six**, which are byte-identical
//! across the whole 8-seed run. Both halves matter: the two that moved
//! prove the `.ron` knob was really connected — an identical-everywhere
//! result is what a *stale binary* looks like — and the six that did not
//! prove the gradient was zero at every tick rather than merely at every
//! sampled one, since a single differing tick would have diverged the run.
//! The catch counters came out identical in both arms (302 carry-samples
//! holding ant, 5 injuries, 6 deaths), so what the wiring bought on the
//! two live seeds was a reshuffled random walk. The wiring is therefore
//! deliberately **not** shipped.
//!
//! Lower `every=` if you want the sampled figure tighter — the cost is
//! linear and the census is the expensive part.
//!
//! ```text
//! cargo run --release --example predation_probe
//! cargo run --release --example predation_probe -- mode=control
//! cargo run --release --example predation_probe -- mode=ab frames=6000
//! cargo run --release --example predation_probe -- mode=cost frames=600
//! cargo run --release --example predation_probe -- seeds=8 every=40
//! ```

use pixel_physics::sim::brain::BrainInput as I;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::organism::{self, CellType, CreatureDef};
use pixel_physics::sim::pheromone::Channel;
use pixel_physics::sim::{creature, material, parallel, Cell, World};

/// The scene `creature_space::run_one` builds, and the scene the
/// `beetles=0`/`beetles=9` null was measured in. Reproduced rather than
/// invented: a hand-built stand-in is 3-for-3 against this codebase
/// (`creature-evolution-plan.md` §6), and a pre-flight measured on a
/// different world than the null it is explaining explains nothing.
const W: i32 = 512;
const H: i32 = 160;
const ANTS: usize = 52;
const BEETLES: usize = 9;
const TREES: i32 = 2;
const PRESET: &str = "wetland";
/// Frames of world settling before any creature is placed — the same
/// warmup `creature_space` uses, so leaves have fallen and the terrain has
/// come to rest before anything walks on it.
const WARMUP: usize = 2400;

/// **Species overrides, applied through `SpeciesRegistry::set_creature`.**
///
/// The eye and the two weights that act on it are knobs here rather than
/// edits to `beetle.ron`, and that is `CLAUDE.md`'s `include_str!` gotcha
/// paid up front: the assets are compiled into the binary, so a sweep that
/// edits the `.ron` and re-runs a prebuilt example produces bit-identical
/// "runs" — three of them, once, before anyone noticed the knob was not
/// connected. With the knob here, `mode=ab` can put an eyed beetle and a
/// blind one in the **same process**, on the same seeds, differing in one
/// field, which is the paired comparison the house rule asks for.
///
/// A negative value means "leave the species file alone".
#[derive(Clone, Copy)]
struct Overrides {
    sight: i32,
    pursue: f32,
    release: f32,
}

impl Default for Overrides {
    fn default() -> Self {
        Self { sight: -1, pursue: f32::NAN, release: f32::NAN }
    }
}

impl Overrides {
    /// Eyes off, everything else as authored — the blind arm.
    fn blind() -> Self {
        Self { sight: 0, ..Self::default() }
    }

    fn apply(self, world: &mut World) {
        use pixel_physics::sim::brain::{BrainInput, BrainOutput, Instinct};
        let species = world.species.id_of("beetle").expect("beetle species");
        let mut def = world.species.get(species).creature.clone().expect("beetle is a creature");
        if self.sight >= 0 {
            def.sight_range = self.sight;
        }
        for (input, output, w) in
            [(BrainInput::PreyBearing, BrainOutput::Turn, self.pursue), (BrainInput::PreyNear, BrainOutput::Persist, self.release)]
        {
            if w.is_nan() {
                continue;
            }
            def.instincts.retain(|i| !(i.0 == input && i.1 == output));
            if w != 0.0 {
                def.instincts.push(Instinct(input, output, w));
            }
        }
        // **`set_creature` alone is not enough, and the sweep said so
        // before this line existed.** The species' genome is compiled from
        // its wiring lists once at load (`Species::from_def`) and
        // `place_creature` stamps *that*, so overriding `instincts` and
        // stopping there changes nothing a creature ever thinks with. The
        // first run of `mode=sweep` came back **bit-identical across all
        // eight settings** — 7500/4301/528 on every row — which is exactly
        // `CLAUDE.md`'s tell that a knob was never connected, and it is the
        // reason to keep this comment rather than just the call.
        let genome = pixel_physics::sim::brain::genome_from_wiring(&def.instincts, &def.hidden_wiring, &def.hidden_outputs, &def.recurrence);
        world.species.set_creature(species, def);
        world.species.set_genome(species, genome);
    }
}

fn main() {
    let mut frames = 6000usize;
    let mut every = 200usize;
    let mut seeds = 1u64;
    let mut mode = "preflight".to_string();
    let mut beetles = BEETLES;
    let mut over = Overrides::default();
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "frames" => frames = v.parse().expect("frames"),
            "every" => every = v.parse().expect("every"),
            "seeds" => seeds = v.parse().expect("seeds"),
            "beetles" => beetles = v.parse().expect("beetles"),
            "mode" => mode = v.to_string(),
            "sight" => over.sight = v.parse().expect("sight"),
            "pursue" => over.pursue = v.parse().expect("pursue"),
            "release" => over.release = v.parse().expect("release"),
            // **An unknown argument is silently ignored, and that has cost
            // this repo a 3.5-hour study** (`CLAUDE.md`). Panic instead.
            other => panic!("unknown arg {other:?}; known: mode, frames, every, seeds, beetles, sight, pursue, release"),
        }
    }

    // **Echo the parameters.** A log that does not name its own seed was
    // written by a binary that never had one.
    println!(
        "predation_probe: mode={mode} frames={frames} every={every} seeds={seeds} beetles={beetles} \
         scene={PRESET} {W}x{H} ants={ANTS} trees={TREES} warmup={WARMUP}\n\
         overrides: sight={} pursue={} release={} (negative sight / NaN weight = as authored)\n",
        over.sight, over.pursue, over.release
    );

    match mode.as_str() {
        "preflight" => preflight(frames, every, seeds, beetles, over),
        "control" => control(),
        "ab" => ab(frames, every, seeds, over),
        "cost" => cost(frames, beetles),
        "sweep" => sweep(frames, every, seeds),
        other => panic!("unknown mode {other:?}; known: preflight, control, ab, cost, sweep"),
    }
}

// ---------------------------------------------------------------------------
// the measurement
// ---------------------------------------------------------------------------

/// One pheromone plane, measured the way a beetle would have to find it.
///
/// The distance histogram is the part that decides the design, and it is a
/// histogram rather than a mean on purpose: the mean over a heavy tail says
/// "47 cells" for a population that is half standing on the trail and half
/// on the other side of the map, and those two halves want opposite fixes.
#[derive(Default, Clone, Copy)]
struct Plane {
    /// Summed value over every cell in the world, averaged over samples.
    /// **The headline number §5 asks for.** Read against `max`: mass alone
    /// cannot tell one saturated trail from a world-wide haze.
    mass: f64,
    max: u32,
    cells: f64,
    /// Same as `mass`/`cells` but at the **last** sample rather than
    /// averaged. The average includes the opening frames, when the plane is
    /// empty by construction; if the two disagree badly the run was short.
    mass_last: u64,
    cells_last: u64,
    /// Fraction of live ant heads with a nonzero cell within Chebyshev
    /// `sensor_offset` — **the second number §5 asks for**: does the trail
    /// mark where the prey is.
    prey_near: f64,
    /// The same test from the predator's side.
    beetle_near: f64,
    /// Mean Chebyshev distance from a beetle head to the nearest nonzero
    /// cell, capped at `NEAR_CAP`.
    nearest: f64,
    /// Cumulative: fraction of beetle samples whose nearest nonzero cell is
    /// within 6, 16, 32, 64, 128 cells. Bucket 0 is `beetle_near` by
    /// another route and is kept as its own arithmetic check.
    reach: [f64; 5],
    /// **The degeneracy proof.** Fraction of beetle samples whose two reads
    /// — underfoot and at the forward sensor — hold *different* values. At
    /// 0.000 the along-gradient is identically zero and any steering built
    /// on it resolves to a constant direction, which is the trap
    /// `CLAUDE.md` records four times.
    differs: f64,
    /// Mean |along-gradient| over beetle samples, and its largest value.
    along_abs: f64,
    along_max: f32,
    /// Fraction of beetle samples whose forward sensor read nonzero.
    front_nonzero: f64,
}

/// Everything one run reports. Kept as a struct so `seeds=N` can order-
/// statistic it rather than quoting one sample from a wide distribution.
#[derive(Default, Clone, Copy)]
struct Row {
    /// **Both planes, measured the same way.** §5 names channel B, and
    /// asking only about B would have hidden the comparison that decides
    /// the design: B is laid by *laden* ants only (`(Carrying, EmitB, 2.5)`)
    /// while A is leaked by every ant on every move (`(Bias, EmitA, 2.0)`),
    /// so the two planes differ by orders of magnitude in how much of the
    /// world they mark. Both mark where ants are; only one of them is
    /// findable.
    b: Plane,
    a: Plane,
    /// **The effect counter, from the far side of the verb.** Samples at
    /// which a beetle's energy was higher than at the previous sample. A
    /// beetle's only income is eating; everything else is a cost.
    beetle_feeds: u64,
    /// Beetles that ever fed at all.
    beetles_fed: usize,
    /// **The other half of the same verb, and leaving it out would have
    /// published a null.** `act`'s eat branch only *swallows* when the
    /// animal is hungry; a full one **picks the mouthful up instead**. The
    /// ant cell is removed from the world either way — that is a kill —
    /// but only the swallow raises energy. A beetle starts at 1600 and
    /// burns roughly 0.4 a tick, so it does not cross its 0.8
    /// `hunger_fraction` until about 6,000 frames in: for most of a
    /// default run the *only* observable form a catch can take is a beetle
    /// walking around carrying a piece of ant.
    /// **Split by what is in the mandibles, because the unsplit counter is
    /// ambiguous between the two things this probe exists to tell apart.**
    /// A beetle's menu is ant *and* corpse; a beetle holding a corpse cell
    /// scavenged, a beetle holding an ant cell caught something. The first
    /// run of this probe reported "83 carry-samples over 3 beetles" and
    /// that number cannot distinguish a predator from an undertaker.
    beetle_grabs_prey: u64,
    beetle_grabs_carrion: u64,
    beetles_grabbed: usize,
    /// Creatures that lost a body cell and survived — the victim's side of
    /// the same event, counted by the engine rather than by this probe.
    injuries: u64,
    /// Live ant heads at the last sample, and beetle heads.
    ants_alive: usize,
    beetles_alive: usize,
    /// Whole-world counters, for "did anything happen at all".
    deposits_b: u64,
    pickups: u64,
    deliveries: u64,
    eats: u64,
    deaths: u64,
    /// **The three sight counters, near side to far side.** `sight_casts`
    /// says the eye ran, `sightings` says it had something to report, and
    /// `sight_approaches` says the animal then moved toward it. All three,
    /// because a null hides in any one of them: an eye that never ran, an
    /// eye that ran over an empty world, and an eye wired to nothing are
    /// three different failures that a single number cannot tell apart.
    sight_casts: u64,
    sightings: u64,
    sight_approaches: u64,
    /// Sightings with the prey inside 45 degrees of the heading, and the
    /// summed sighted range. **The two counters `sight_approaches` was
    /// unable to be** — see `CreatureStats::sight_facing`.
    sight_facing: u64,
    sight_dist_sum: u64,
    /// Cells read by the sense, summed. `/ sight_casts` is the number the
    /// sizing study's cost argument is built on.
    sight_cells_read: u64,
}

fn preflight(frames: usize, every: usize, seeds: u64, beetles: usize, over: Overrides) {
    println!(
        "per channel: mass = summed value world-wide (mean over samples); cells = nonzero cells; \n\
         prey<=6 / beetle<=6 = fraction of heads with a nonzero cell within the beetle's sensor offset;\n\
         d = mean Chebyshev distance from a beetle head to the nearest nonzero cell (cap {NEAR_CAP});\n\
         differ = fraction of beetle samples whose two sensor reads hold different values (the degeneracy proof);\n\
         front = fraction whose forward sensor read nonzero; |along| = mean |along-gradient| the wiring would read.\n"
    );
    println!("{:>4} {:>3} {:>9} {:>8} {:>5} {:>8} {:>10} {:>7} {:>8} {:>8} {:>8}",
        "seed", "ch", "mass", "cells", "max", "prey<=6", "beetle<=6", "d", "differ", "front", "|along|");
    let mut rows = Vec::new();
    for s in 0..seeds {
        let r = run(SEED_BASE + s, frames, every, beetles, Paint::None, over);
        for (label, pl) in [("B", r.b), ("A", r.a)] {
            println!("{:>4} {label:>3} {:>9.0} {:>8.0} {:>5} {:>8.3} {:>10.3} {:>7.1} {:>8.3} {:>8.3} {:>8.4}",
                s, pl.mass, pl.cells, pl.max, pl.prey_near, pl.beetle_near, pl.nearest, pl.differs, pl.front_nonzero, pl.along_abs);
        }
        println!("{:>4} {:>3}   beetle -> nearest cell, cumulative fraction within {:?}: B {:?}  A {:?}",
            s, "", REACH_EDGES,
            r.b.reach.map(|v| (v * 1000.0).round() / 1000.0),
            r.a.reach.map(|v| (v * 1000.0).round() / 1000.0));
        println!("{:>4} {:>3}   last sample: B mass {} over {} cells; A mass {} over {} cells; ants alive {}, deposits B {}",
            s, "", r.b.mass_last, r.b.cells_last, r.a.mass_last, r.a.cells_last, r.ants_alive, r.deposits_b);
        rows.push(r);
    }
    summarise(&rows);
}

const SEED_BASE: u64 = pixel_physics::sim::world::DEFAULT_WORLD_SEED;

fn summarise(rows: &[Row]) {
    let n = rows.len() as f64;
    let mean = |f: &dyn Fn(&Row) -> f64| rows.iter().map(f).sum::<f64>() / n;
    println!("\nmean over {} seed(s):", rows.len());
    for (label, pick) in [("B", 0usize), ("A", 1usize)] {
        let pl = move |r: &Row| if pick == 0 { r.b } else { r.a };
        println!(
            "  channel {label}: mass {:.0} over {:.0} cells | prey within sensor {:.3} | beetle within sensor {:.3} \
             | mean distance beetle -> nearest cell {:.1} | reads differ {:.3} | |along| {:.4}",
            mean(&|r| pl(r).mass),
            mean(&|r| pl(r).cells),
            mean(&|r| pl(r).prey_near),
            mean(&|r| pl(r).beetle_near),
            mean(&|r| pl(r).nearest),
            mean(&|r| pl(r).differs),
            mean(&|r| pl(r).along_abs),
        );
    }
    let feeds: u64 = rows.iter().map(|r| r.beetle_feeds).sum();
    let fed: usize = rows.iter().map(|r| r.beetles_fed).sum();
    let prey_grabs: u64 = rows.iter().map(|r| r.beetle_grabs_prey).sum();
    let carrion_grabs: u64 = rows.iter().map(|r| r.beetle_grabs_carrion).sum();
    let grabbed: usize = rows.iter().map(|r| r.beetles_grabbed).sum();
    let injuries: u64 = rows.iter().map(|r| r.injuries).sum();
    println!(
        "effect counters (far side of the verb): {feeds} feed events over {fed} beetle(s); \
         carry-samples {prey_grabs} holding ant / {carrion_grabs} holding carrion over {grabbed} beetle(s); \
         {injuries} injuries world-wide"
    );
    println!(
        "colony: {} pickups, {} deliveries, {} deaths across all seeds",
        rows.iter().map(|r| r.pickups).sum::<u64>(),
        rows.iter().map(|r| r.deliveries).sum::<u64>(),
        rows.iter().map(|r| r.deaths).sum::<u64>(),
    );
}

/// What to paint into the world before the run — the positive controls.
#[derive(Clone, Copy, PartialEq)]
enum Paint {
    None,
    /// A saturated channel-B trail along the surface, re-laid every sample
    /// so decay cannot erase it. Proves the instrument reports nonzero mass
    /// and a nonzero near-fraction when there genuinely is a trail.
    SaturatedTrail,
}

/// The scene, built once and shared by every mode — so a cost figure and a
/// perception figure are taken on the same world rather than on two that
/// merely resemble each other.
/// **`over` is applied before a single creature is placed, and that
/// ordering is load-bearing.** `place_creature` stamps the genome from the
/// species' `instincts` at birth, so a weight overridden after placement
/// would change nothing at all in the founders and would read as a lever
/// that does not work. `sight_range` is read from the def every tick and
/// would survive either order; the weights would not.
fn build_scene(seed: u64, beetles: usize, over: Overrides) -> World {
    let mut world = World::new(Rect::new(0, 0, W - 1, H - 1));
    world.seed = seed;

    over.apply(&mut world);

    let (presets, _) = pixel_physics::worldgen::WorldgenPresets::load();
    let params = presets.get(PRESET).expect("wetland preset");
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed });

    let surface_at: Vec<i32> = (0..W).map(|x| surface(&world, x)).collect();

    let nest = world.materials.id_of("nest").expect("nest");
    for x in 16..90 {
        world.set(x, surface_at[x as usize], Cell::new(nest, 0).with_attached(true));
    }
    for i in 0..TREES {
        let x = 150 + i * (300 / TREES.max(1));
        world.plant_tree(x, surface_at[x as usize] - 1);
    }
    for _ in 0..WARMUP {
        world.step_active_sites();
        world.step_fields();
    }

    // Place ants until there are ANTS of them, not at ANTS fixed columns:
    // `plant_creature_seed` refuses a site it does not fit and returns
    // quietly, and on wetland the target cell is standing water often
    // enough that a fixed-column loop stands up half the colony it asked
    // for (`creature_space`'s own note).
    let mut planted = 0usize;
    let mut x = 24i32;
    while planted < ANTS && x < W - 8 {
        if let Some(site) = creature::plant_creature_seed(&mut world, x, surface_at[x as usize] - 1, "ant") {
            world.schedule_active_site(site);
            planted += 1;
            x += 4;
        } else {
            x += 1;
        }
    }
    let mut beetles_placed = 0usize;
    for i in 0..beetles {
        let bx = 40 + i as i32 * 45;
        if let Some(s) = creature::plant_creature_seed(&mut world, bx, surface_at[bx as usize] - 1, "beetle") {
            world.schedule_active_site(s);
            beetles_placed += 1;
        }
    }
    assert!(planted > 0, "no ant was placed; the scene does not contain the situation this probe is about");
    assert!(
        beetles == 0 || beetles_placed > 0,
        "no beetle was placed; a predation probe with no predator measures nothing"
    );
    world
}

fn run(seed: u64, frames: usize, every: usize, beetles: usize, paint: Paint, over: Overrides) -> Row {
    let mut world = build_scene(seed, beetles, over);
    let surface_at: Vec<i32> = (0..W).map(|x| surface(&world, x)).collect();

    let ant_mat = world.materials.id_of("ant").expect("ant");
    let beetle_mat = world.materials.id_of("beetle").expect("beetle");
    let beetle_def: CreatureDef = world
        .species
        .get(world.species.id_of("beetle").expect("beetle species"))
        .creature
        .as_ref()
        .expect("beetle is a creature")
        .clone();
    let so = beetle_def.sensor_offset;

    let mut row = Row::default();
    let mut samples = 0.0f64;
    let mut beetle_energy: std::collections::HashMap<u16, f32> = std::collections::HashMap::new();
    let mut ever_fed: std::collections::HashSet<u16> = std::collections::HashSet::new();
    let mut ever_grabbed: std::collections::HashSet<u16> = std::collections::HashSet::new();

    for frame in 0..frames {
        if paint == Paint::SaturatedTrail && frame % 60 == 0 {
            // Re-laid, because `build_decay_lut` forces every nonzero value
            // strictly downward every pass: a trail painted once and left
            // alone is a trail that is measurably gone by the end.
            for px in 0..W {
                let py = surface_at[px as usize] - 1;
                world.deposit_pheromone(Channel::B, px, py, 255);
            }
        }
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
        if frame % every != 0 {
            continue;
        }

        // --- where the heads are ------------------------------------------
        let mut prey: Vec<(i32, i32)> = Vec::new();
        let mut preds: Vec<(i32, i32, u16)> = Vec::new();
        for py in 0..H {
            for px in 0..W {
                let c = world.get(px, py);
                if organism::cell_type(c.aux()) != Some(CellType::Head) {
                    continue;
                }
                if c.material == ant_mat {
                    prey.push((px, py));
                } else if c.material == beetle_mat {
                    preds.push((px, py, c.organism_id()));
                }
            }
        }

        // --- both planes, measured identically ----------------------------
        for (channel, acc, front_slot, along_slot) in [
            (Channel::B, &mut row.b, I::PheroBFront, I::PheroBAlong),
            (Channel::A, &mut row.a, I::PheroAFront, I::PheroAAlong),
        ] {
            let (mut mass, mut cells, mut peak) = (0u64, 0u64, 0u8);
            for py in 0..H {
                for px in 0..W {
                    let v = world.pheromone_at(channel, px, py);
                    if v > 0 {
                        mass += v as u64;
                        cells += 1;
                        peak = peak.max(v);
                    }
                }
            }
            acc.mass += mass as f64;
            acc.cells += cells as f64;
            acc.max = acc.max.max(peak as u32);
            acc.mass_last = mass;
            acc.cells_last = cells;

            let near = |cx: i32, cy: i32| -> bool {
                (-so..=so).any(|dy| (-so..=so).any(|dx| world.pheromone_at(channel, cx + dx, cy + dy) > 0))
            };
            if !prey.is_empty() {
                acc.prey_near += prey.iter().filter(|&&(px, py)| near(px, py)).count() as f64 / prey.len() as f64;
            }
            if !preds.is_empty() {
                acc.beetle_near += preds.iter().filter(|&&(px, py, _)| near(px, py)).count() as f64 / preds.len() as f64;
                let n = preds.len() as f64;
                let mut sum = 0.0f64;
                let mut buckets = [0.0f64; 5];
                for &(px, py, _) in &preds {
                    let d = nearest_ring(&world, channel, px, py);
                    sum += d as f64;
                    for (i, &edge) in REACH_EDGES.iter().enumerate() {
                        if d <= edge {
                            buckets[i] += 1.0;
                        }
                    }
                }
                acc.nearest += sum / n;
                for (slot, hit) in acc.reach.iter_mut().zip(buckets) {
                    *slot += hit / n;
                }
            }

            // --- what a beetle would actually read ------------------------
            // `creature::probe` is the non-mutating evaluation, so looking
            // is free and cannot perturb the run being measured.
            let (mut differs, mut front_nz, mut along_abs, mut n) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
            for &(px, py, id) in &preds {
                let Some(state) = world.organism(id) else { continue };
                let (dx, dy) = creature::DIRS[state.heading as usize % 8];
                // The two cells the along-gradient is a difference of. The
                // planes are at CA resolution and `PheromonePlane::sample`
                // is nearest-cell with no interpolation, so these are two
                // genuinely different cells 6 apart — but "should differ"
                // is exactly what the four previous victims of this trap
                // believed, so it is counted rather than argued.
                let here = world.pheromone_at(channel, px, py);
                let ahead = world.pheromone_at(channel, px + dx * so, py + dy * so);
                if here != ahead {
                    differs += 1.0;
                }
                let (inputs, _outputs, _active) = creature::probe(&world, px, py, id, &beetle_def);
                let along = inputs[along_slot as usize];
                along_abs += along.abs() as f64;
                acc.along_max = acc.along_max.max(along.abs());
                if inputs[front_slot as usize] > 0.0 {
                    front_nz += 1.0;
                }
                n += 1.0;
            }
            if n > 0.0 {
                acc.differs += differs / n;
                acc.front_nonzero += front_nz / n;
                acc.along_abs += along_abs / n;
            }
        }

        // --- the effect counters ------------------------------------------
        for &(_px, _py, id) in &preds {
            let Some(state) = world.organism(id) else { continue };
            // Effect counter, far side of the verb: a beetle's energy only
            // ever rises by eating.
            let e = state.energy;
            match beetle_energy.insert(id, e) {
                Some(prev) if e > prev + 1e-3 => {
                    row.beetle_feeds += 1;
                    ever_fed.insert(id);
                }
                _ => {}
            }
            // ...and the branch a hunger gate sends a full beetle down: it
            // takes the mouthful and carries it. Both foods on a beetle's
            // menu are flesh, so anything a beetle holds is something it
            // caught.
            if let Some(held) = state.carrying {
                if held.material == ant_mat {
                    row.beetle_grabs_prey += 1;
                } else {
                    row.beetle_grabs_carrion += 1;
                }
                ever_grabbed.insert(id);
            }
        }

        row.ants_alive = prey.len();
        row.beetles_alive = preds.len();
        samples += 1.0;
    }

    let s = samples.max(1.0);
    for acc in [&mut row.b, &mut row.a] {
        acc.mass /= s;
        acc.cells /= s;
        acc.prey_near /= s;
        acc.beetle_near /= s;
        acc.nearest /= s;
        acc.differs /= s;
        acc.front_nonzero /= s;
        acc.along_abs /= s;
        for v in acc.reach.iter_mut() {
            *v /= s;
        }
    }
    row.beetles_fed = ever_fed.len();
    row.beetles_grabbed = ever_grabbed.len();
    row.injuries = world.creature_stats.injuries;
    row.deposits_b = world.pheromones.stats.deposits_b;
    row.pickups = world.creature_stats.pickups;
    row.deliveries = world.creature_stats.deliveries;
    row.eats = world.creature_stats.eats;
    row.deaths = world.creature_stats.deaths;
    row.sight_casts = world.creature_stats.sight_casts;
    row.sightings = world.creature_stats.sightings;
    row.sight_approaches = world.creature_stats.sight_approaches;
    row.sight_facing = world.creature_stats.sight_facing;
    row.sight_dist_sum = world.creature_stats.sight_dist_sum;
    row.sight_cells_read = world.creature_stats.sight_cells_read;
    row
}

/// How far away the nearest nonzero channel-B cell is, in Chebyshev cells,
/// searched by expanding rings and capped.
///
/// Capped rather than unbounded on purpose: an uncapped search over a world
/// with an empty plane walks the whole plane per beetle per sample, and the
/// answer "further than this cap" is all the decision needs.
const NEAR_CAP: i32 = 128;

fn nearest_ring(world: &World, channel: Channel, cx: i32, cy: i32) -> i32 {
    if world.pheromone_at(channel, cx, cy) > 0 {
        return 0;
    }
    for r in 1..=NEAR_CAP {
        for d in -r..=r {
            for (px, py) in [(cx + d, cy - r), (cx + d, cy + r), (cx - r, cy + d), (cx + r, cy + d)] {
                if world.pheromone_at(channel, px, py) > 0 {
                    return r;
                }
            }
        }
    }
    NEAR_CAP
}

/// The upper edges of the distance histogram. 6 is the beetle's authored
/// `sensor_offset`, so bucket 0 is "already in reach"; the rest double, so
/// the shape says how much wider a sense would have to be to matter.
const REACH_EDGES: [i32; 5] = [6, 16, 32, 64, 128];

fn surface(world: &World, x: i32) -> i32 {
    (0..H)
        .find(|&y| {
            world.get(x, y).organism_id() == 0
                && matches!(world.materials.kind(world.get(x, y).material), material::MaterialKind::Solid | material::MaterialKind::Powder)
        })
        .unwrap_or(H - 1)
}

// ---------------------------------------------------------------------------
// the positive controls
// ---------------------------------------------------------------------------

/// **The half of the rule this repo keeps missing.** A pre-flight that
/// reports "no trail" is only worth something if the same instrument, on a
/// world that provably has a trail, reports one. Both arms run the same
/// code path as `preflight`; only the world differs.
fn control() {
    println!("positive controls — the instrument reporting the opposite of the null.\n");

    let bare = run(SEED_BASE, 1200, 200, BEETLES, Paint::None, Overrides::default());
    let painted = run(SEED_BASE, 1200, 200, BEETLES, Paint::SaturatedTrail, Overrides::default());
    println!("{:>18} {:>10} {:>8} {:>10} {:>10} {:>12} {:>8}", "arm", "B mass", "B cells", "prey<=6", "beetle<=6", "reads differ", "|along|");
    for (label, r) in [("bare", bare), ("saturated trail", painted)] {
        println!("{label:>18} {:>10.0} {:>8.0} {:>10.3} {:>10.3} {:>12.3} {:>8.4}",
            r.b.mass, r.b.cells, r.b.prey_near, r.b.beetle_near, r.b.differs, r.b.along_abs);
    }
    assert!(painted.b.mass > bare.b.mass, "the B-mass census cannot see a saturated trail: it is blind, not quiet");
    assert!(painted.b.prey_near > 0.99, "the prey-near-trail fraction cannot see a trail painted under every ant");
    assert!(painted.b.differs > 0.0, "the two sensor reads never differ even on a painted trail — the gradient is degenerate by construction");
    println!("\n  PASS: mass census, near-fraction and the two-read difference all move when a trail exists.");

    // --- the catch control -------------------------------------------
    // Its own scene rather than the big one, because the control has to
    // *produce* the situation rather than wait for it: on generated
    // terrain a beetle that cannot find an ant will not find one here
    // either, which is the very null being checked.
    let apart = catch_scene(60, false);
    let touching = catch_scene(2, false);
    let hungry = catch_scene(2, true);
    println!("\n{:>26} {:>8} {:>8} {:>10} {:>10} {:>9}", "arm", "feeds", "grabs", "ant cells", "injuries", "deaths");
    for (label, r) in [("beetle 60 cells from prey", apart), ("beetle beside prey", touching), ("beetle beside prey, hungry", hungry)] {
        println!("{label:>26} {:>8} {:>8} {:>10} {:>10} {:>9}", r.0, r.1, r.2, r.3, r.4);
    }
    assert_eq!(apart.0 + apart.1, 0, "a beetle 60 cells away registered a catch: the counter fires on something other than catching");
    assert!(touching.1 + touching.0 > 0, "a beetle beside an ant never registered a catch: the effect counter is blind");
    assert!(hungry.0 > 0, "a hungry beetle beside an ant never swallowed: the feed counter is blind");
    println!("\n  PASS: the effect counters read zero at 60 cells and nonzero mouth-to-mouth, in both the swallow and the carry form.");
}

// ---------------------------------------------------------------------------
// what it costs
// ---------------------------------------------------------------------------

/// **Whole-frame cost on a world that has stopped moving**, plus the
/// counter the pheromone design goal is actually stated in.
///
/// Two rules govern this and neither is optional (`CLAUDE.md`). *Measure a
/// cost against the state the optimisation exists for* — what a per-frame
/// plane defeats is the dirty-rect render skip, which does its work
/// exactly when nothing is moving. And *quote the whole-frame figure*: a
/// sub-phase row that falls while the frame does not move is usually the
/// cost relocating, and a change that removed 91% of a phase's work once
/// made the frame slower.
///
/// Alternate this binary with one built from the other `.ron` and read the
/// pair, never one run against a remembered number.
fn cost(frames: usize, beetles: usize) {
    let mut world = build_scene(SEED_BASE, beetles, Overrides::default());
    // Settle: run without sampling until the creatures have spread out and
    // the terrain has stopped falling.
    for _ in 0..4000 {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
    }
    let tiles_before = world.pheromones.stats.tiles_processed;
    let passes_before = world.pheromones.stats.passes;
    let t0 = std::time::Instant::now();
    for _ in 0..frames {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0 / frames as f64;
    let tiles = world.pheromones.stats.tiles_processed - tiles_before;
    let passes = world.pheromones.stats.passes - passes_before;
    println!("settled world, {frames} frames after a 4,000-frame settle, beetles={beetles}");
    println!("  whole frame          {ms:.4} ms");
    println!("  pheromone passes     {passes}");
    println!("  tiles processed      {tiles}  ({:.1} per pass)", tiles as f64 / passes.max(1) as f64);
    println!("\n  A wall clock is only as trustworthy as the box was quiet; `tiles processed` is the deterministic half.");
}

/// A flat stone floor, one beetle, and ants `gap` cells away. Returns
/// `(feeds, grabs, ant cells left, injuries, deaths)`.
///
/// `gap = 60` is the negative arm — far enough that nothing should happen —
/// and it is what makes the nonzero arm evidence rather than an assertion
/// that the counter can be made to increment somehow.
fn catch_scene(gap: i32, hungry: bool) -> (u64, u64, usize, u64, u64) {
    let (w, h) = (160i32, 60i32);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    world.seed = SEED_BASE;
    let floor = h - 8;
    for x in 0..w {
        for y in floor..h {
            world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
    if hungry {
        // **`hunger_fraction`, not the individual's energy**, because
        // `World::organism_mut` is `pub(crate)` and an example cannot
        // reach it. Raising the fraction to 1.0 makes the beetle count as
        // hungry the instant it has spent anything, which sends `act`
        // down the swallow branch instead of the pick-up branch. Set
        // before the animal is placed, so nothing is planted against the
        // old def.
        let species = world.species.id_of("beetle").expect("beetle species");
        let mut def = world.species.get(species).creature.as_ref().expect("beetle is a creature").clone();
        def.hunger_fraction = 1.0;
        world.species.set_creature(species, def);
    }
    let beetle = creature::plant_creature_seed(&mut world, 40, floor - 1, "beetle").expect("the beetle is placed");
    world.schedule_active_site(beetle);
    let beetle_id = world.get(40, floor - 1).organism_id();
    assert_ne!(beetle_id, 0, "the beetle was not placed; the scene does not contain the situation this control is about");
    let mut ants = 0;
    for i in 0..12 {
        let x = 40 + gap + i * 3;
        if creature::plant_creature_seed(&mut world, x, floor - 1, "ant").map(|s| world.schedule_active_site(s)).is_some() {
            ants += 1;
        }
    }
    assert!(ants > 0, "no prey was placed; the control cannot see a catch that cannot happen");

    let ant_mat = world.materials.id_of("ant").expect("ant");
    let (mut feeds, mut grabs) = (0u64, 0u64);
    let mut last = f32::NAN;
    for frame in 0..3000 {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
        if frame % 20 != 0 {
            continue;
        }
        let Some(s) = world.organism(beetle_id) else { continue };
        if s.energy > last + 1e-3 {
            feeds += 1;
        }
        last = s.energy;
        if s.carrying.is_some() {
            grabs += 1;
        }
    }
    let left = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&(x, y)| world.get(x, y).material == ant_mat).count();
    (feeds, grabs, left, world.creature_stats.injuries, world.creature_stats.deaths)
}

// ---------------------------------------------------------------------------
// the null, re-measured
// ---------------------------------------------------------------------------

/// `beetles=0` against `beetles=N` on one seed — the arms that measured
/// bit-identical. Re-run here so any later claim about predation has a
/// baseline taken on *this* build rather than a remembered number
/// (`CLAUDE.md`: always re-measure the baseline in the same session).
/// **How hard to pursue — swept, with the control that makes the sweep
/// mean anything.**
///
/// The first row is `pursue=0 release=0`: **eyes open, wired to nothing.**
/// Without it `approaches` is uninterpretable, because a beetle wandering
/// at random already closes on a target it happens to be pointed at some
/// of the time, and that share is not zero. It is the positive-control
/// half of `CLAUDE.md`'s rule — a number that stays quiet when nothing is
/// wrong has not been shown to *move* when something is — pointed at the
/// only counter here that could be measuring the world rather than the
/// wiring.
///
/// `appr/seen` is the column to read: of the casts that had prey to
/// report, what fraction of them ended with the animal closer to it.
/// The grid. Kept as a constant so the sweep's own points are named once
/// and a log can be read against them.
const SWEEP_POINTS: [(f32, f32); 5] = [(0.0, 0.0), (-2.5, 0.0), (-2.5, -3.0), (-2.5, -6.0), (-5.0, -6.0)];

fn sweep(frames: usize, every: usize, seeds: u64) {
    println!("{:>7} {:>8} {:>6} {:>7} {:>7} {:>9} {:>10} {:>8} {:>9} {:>6} {:>6}",
        "pursue", "release", "seeds", "casts", "seen", "seen/cast", "facing/seen", "appr/seen", "mean range", "prey!", "injur");
    for (pursue, release) in SWEEP_POINTS {
        let (mut casts, mut seen, mut appr, mut facing, mut dist, mut prey, mut injur) = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
        for s in 0..seeds {
            let over = Overrides { sight: -1, pursue, release };
            let r = run(SEED_BASE + s, frames, every, BEETLES, Paint::None, over);
            casts += r.sight_casts;
            seen += r.sightings;
            appr += r.sight_approaches;
            facing += r.sight_facing;
            dist += r.sight_dist_sum;
            prey += r.beetle_grabs_prey;
            injur += r.injuries;
        }
        let d = seen.max(1) as f64;
        println!("{pursue:>7.1} {release:>8.1} {seeds:>6} {casts:>7} {seen:>7} {:>9.3} {:>10.3} {:>8.3} {:>9.1} {prey:>6} {injur:>6}",
            seen as f64 / casts.max(1) as f64, facing as f64 / d, appr as f64 / d, dist as f64 / d);
    }
    println!(
        "\nRow 1 (0.0/0.0) is the control: the eye runs and steers nothing. Its `appr/seen` is what a \
         beetle achieves by walking about, and every other row has to beat it to be a pursuit rather \
         than a coincidence."
    );
}

/// **The test of E15, and the only one that matters.**
///
/// The measured null this whole line started from is that `beetles=0` and
/// `beetles=9` ran **bit-identical** over 6,000 frames — a predator that
/// moves no counter is an absent one. So the first two rows of each seed
/// are that null, re-run.
///
/// The third row is the one the sense exists for: **the same nine beetles,
/// with eyes.** It is a paired arm rather than a remembered number, in the
/// same process and on the same seed, differing from the row above it in
/// one species field — which is what makes the difference attributable at
/// all (`CLAUDE.md`: an A/B needs its two arms to differ only by the
/// change).
fn ab(frames: usize, every: usize, seeds: u64, over: Overrides) {
    println!(
        "arm: `blind` forces sight_range=0; `eyed` takes beetle.ron as authored (or the sight=/pursue=/release= overrides).\n\
         casts/seen/appr are the sight counters: the eye ran / it had something to report / the animal then closed on it.\n\
         A blind arm MUST read 0/0/0 -- if it does not, the species opt-in leaked and every number below is confounded.\n"
    );
    println!("{:>4} {:>7} {:>7} {:>9} {:>6} {:>6} {:>7} {:>8} {:>6} {:>6} {:>6} {:>8} {:>7} {:>7}",
        "seed", "arm", "beetles", "B mass", "ants", "eats", "pickups", "injuries", "deaths", "feeds", "prey!", "casts", "seen", "appr");
    for s in 0..seeds {
        for (label, b, arm) in
            [("blind", 0usize, Overrides::blind()), ("blind", BEETLES, Overrides::blind()), ("eyed", BEETLES, over)]
        {
            let r = run(SEED_BASE + s, frames, every, b, Paint::None, arm);
            println!("{:>4} {label:>7} {b:>7} {:>9.0} {:>6} {:>6} {:>7} {:>8} {:>6} {:>6} {:>6} {:>8} {:>7} {:>7}",
                s, r.b.mass, r.ants_alive, r.eats, r.pickups, r.injuries, r.deaths, r.beetle_feeds, r.beetle_grabs_prey,
                r.sight_casts, r.sightings, r.sight_approaches);
            if r.sight_casts > 0 {
                // **The cost figure, deterministic.** A 0.004 ms/frame
                // charge is below what a wall clock on a shared box can
                // resolve, so the sizing study priced the sense through
                // this quantity instead and predicted 485 cells per cast at
                // r64. Printed here so the built sense can be held against
                // its own specification rather than inheriting it.
                println!("{:>4} {:>7}   {:.0} cells read per cast (sizing study predicted 485 at r64)",
                    s, "", r.sight_cells_read as f64 / r.sight_casts as f64);
            }
        }
    }
    println!(
        "\n`injuries` and `deaths` are the victim's side of the verb, counted by the engine. \
         If the two `blind` rows of a seed agree on every column, the predator did not exist -- which is \
         the measured null this line began from. The claim to check is that the `eyed` row does not agree \
         with them."
    );
}
