//! **What happens when two colonies meet?**
//!
//! Nothing in this repo could answer that before this file, and
//! `Reports/instruments.md` was checked first. `creature_arena` races two
//! *genomes* in one bed for the same plants and space — **exploitation**
//! competition, by construction: it founds one colony, attributes by
//! lineage, and has no way to make two groups strangers to each other.
//! `predation_probe` asks whether a beetle can find an ant, which is
//! predation across species. `labforage` censuses one colony's larder.
//! **Interference** — what two groups of the same kind do to each other when
//! they are in contact — had no harness at all.
//!
//! `Reports/animal-conflict-research-2026-09-14.md` is the design; this is
//! the instrument it is read with.
//!
//! # What it reports, and why each column is there
//!
//! ```text
//! cargo run --release --example conflict_arena
//! cargo run --release --example conflict_arena -- control=selftest
//! cargo run --release --example conflict_arena -- seeds=6 frames=24000 spread=1.0
//! cargo run --release --example conflict_arena -- assess=off        # the A/B's other arm
//! cargo run --release --example conflict_arena -- numbers=0         # is it strength or numbers
//! cargo run --release --example conflict_arena -- spread=0          # the supercolony control
//! ```
//!
//! * **`contests` against `attacks`** — the near and far sides of the same
//!   call. `contests` counts every tick an animal willing to fight actually
//!   stood in front of somebody it could hit; `attacks` counts the ones that
//!   became a bite. `CLAUDE.md`'s first law is read off the ratio: a
//!   mechanic whose every encounter escalates has no middle, however busy it
//!   looks, and real inter-colony contact is nearly all withdrawal.
//! * **`eats` and `gnaws` beside them, and this is the column that surprised
//!   me.** `ant` material carries `food_class: 1.0` and the shipped ant's
//!   gut sits at a neutral bias, so **a stranger is food**: the moment
//!   `scent_spread` puts two colonies outside each other's tolerance, each
//!   is prey to the other's ordinary mouth, with no `Attack` weight involved
//!   anywhere. A harness that only counted `attacks` would report a peaceful
//!   bed while the colonies ate each other. See §4 of the report.
//! * **`x-kills`** — kills where attacker and victim carry different colony
//!   labels, read off `World::kills_log`, which records both. The verb-blind
//!   readout: it counts the same event whether the mouth or the fist did it,
//!   which is exactly what makes it the control on the two counters above.
//! * **`casts` / `prey` / `threat`** — the eye. A rival that nothing ever
//!   *sees* is being met by walking into it, and whether the eye can resolve
//!   a rival at all is the question `sight=` exists to ask.
//!
//! # Three things this file is careful about
//!
//! 1. **`control=selftest` is the positive control and it runs in seconds.**
//!    `CLAUDE.md`'s worst-recurring failure is a number that is
//!    arithmetically correct and about the wrong thing, and its remedy is to
//!    construct the case whose answer is known to be non-zero and check the
//!    instrument reports it. The selftest checks both directions — that the
//!    counters move when strangers are put in contact, **and** that they stay
//!    at zero on the supercolony control where every ant is everyone's
//!    nestmate.
//! 2. **Both colonies are edited identically.** This is not an A/B between
//!    two genomes; it is one genome in two groups, and every asymmetry in
//!    the result has to come out of the bed. A dial that reached one side
//!    only would make every number here a statement about the dial.
//! 3. **Read it as an order statistic over seeds** (`seeds=`, default 4), not
//!    as one trajectory. The lab's own coordinator note records adjacent
//!    20,000-frame stops of one run reading 3,099 and 16 ants; a single
//!    conflict run is a sample from a wide distribution and nothing here is
//!    less chaotic than that.
//!
//! # What it cannot say
//!
//! It cannot price the mechanism. Every column is a count, and `CLAUDE.md`
//! is explicit that a counter downstream of the parallel sweep is only
//! load-independent at fixed parallelism — pin `RAYON_NUM_THREADS` before
//! comparing two runs of this on a shared box. For frame cost, `antcost` and
//! `ascii` are the instruments; this one is about behaviour.

use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::brain::{self, BrainInput, BrainOutput};
use pixel_physics::sim::frame;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::explosion::Blasts;

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.parse().ok().expect("parses")))
}

fn arg_str(name: &str) -> Option<String> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.to_string()))
}

/// One arena run's whole answer.
#[derive(Default, Clone, Copy)]
struct Row {
    seed: u64,
    /// Animals alive at the end, per colony, in founding order.
    alive: [usize; 2],
    /// Colony labels actually minted, so a run that founded one colony
    /// because the second had nowhere to stand is visible rather than
    /// silently halved.
    colonies_founded: usize,
    contests: u64,
    attacks: u64,
    displays: u64,
    attack_cells: u64,
    attack_kills: u64,
    gnaws: u64,
    eats: u64,
    /// Kills whose attacker and victim carry different colony labels.
    cross_kills: u64,
    /// ...and the ones inside one colony, which is the control on the line
    /// above: a bed whose cross-colony kills are high and whose within-colony
    /// kills are equally high is not a war, it is a famine.
    own_kills: u64,
    casts: u64,
    prey_seen: u64,
    threat_seen: u64,
    deaths: u64,
    /// **Distinct colony labels alive at the end.** More than were founded
    /// means `World::regroup_by_scent` minted a group because a lineage
    /// drifted out of its own colony -- speciation in progress, and the one
    /// thing that can make a bed founded at `spread=0` hold strangers
    /// anyway. Printed rather than asserted away, because it is a finding
    /// and not a fault.
    groups: usize,
    /// **The fraction of ordered live-ant pairs that read as non-kin**, from
    /// `creature::scent_accepts` -- the predicate the mouth, the eye and the
    /// fist all share, asked directly rather than inferred from behaviour.
    ///
    /// This is the premise check for every other column: a bed whose ants
    /// are all nestmates cannot produce a contest however aggressive the
    /// genome is, and a `spread=` that failed to reach the species would
    /// look exactly like a mechanism that does not work.
    stranger_frac: f64,
}

impl Row {
    /// **Encounters with an animal that became a bite.**
    ///
    /// Derived rather than counted, and it is `contests - displays` rather
    /// than `attacks` **because `attacks` is not an animal counter**.
    /// `nearest_foe`'s target rule is any living non-kin *organism*, and a
    /// plant is an organism — so an armed ant bites herbs and every one of
    /// those closures lands in `attacks`. Reading escalation off `attacks`
    /// gave 0.860 on a bed whose real escalation was 0.093, and would have
    /// exceeded 1.0 on a leafier one. See the report's §8.4.
    fn escalations(&self) -> u64 {
        self.contests.saturating_sub(self.displays)
    }

    /// **Jaw closures the `Attack` verb spent on something that was not an
    /// animal** — vegetation, in practice. Printed rather than subtracted
    /// away: it is the shipped verb's own behaviour, nobody had seen it
    /// because no shipped species authors `Attack`, and a harness that hid
    /// it would be hiding the thing that caught it.
    fn plant_bites(&self) -> u64 {
        self.attacks.saturating_sub(self.escalations())
    }

    /// **How often an encounter became a bite.** The number the first law is
    /// read off. `f64::NAN` when nothing was ever met, which prints as `-`
    /// rather than as a misleading 0.000.
    fn escalation(&self) -> f64 {
        if self.contests == 0 {
            f64::NAN
        } else {
            self.escalations() as f64 / self.contests as f64
        }
    }
}

struct Cfg {
    frames: u64,
    ants: i32,
    plants: usize,
    spread: f32,
    sight: Option<i32>,
    attack_w: f32,
    /// Fraction of the bed width between the two nests, 0..1. The one knob
    /// that decides whether the colonies are in contact at all, which is the
    /// prior question to every other number here.
    gap: f32,
    width: i32,
    height: i32,
    /// **`TRAIT_ARMOUR` given to the second colony's founders only** — the
    /// one asymmetry this harness can make, and without it the strength half
    /// of the assessment is unreachable.
    ///
    /// Two identical colonies are identical: `bite_progress` is the same
    /// number in both directions, so `mine - theirs` is exactly zero and the
    /// *only* live term is the local numerical one. Measured, that makes
    /// `boldness=` and `numbers=` the same knob — they returned
    /// byte-identical summaries — which is a true statement about a
    /// symmetric bed and says nothing at all about the mechanism.
    ///
    /// **Founders only, deliberately.** It is a scene edit, not a species
    /// edit: children revert to the authored allele, so a run long enough to
    /// breed reports the plate wearing off rather than a second species
    /// quietly appearing. At the default 4,000 frames, well inside one
    /// ~12,000-frame generation, that distinction does not arise.
    armour_b: Option<f32>,
}

/// **The same run, rendered.** `gif=<path>` captures the bed through
/// `render::Renderer` -- the renderer the game draws with, not a bespoke
/// path of this file's own -- every `every=` frames, and encodes a looping
/// GIF the way `main.rs`'s own `CaptureSequence::finish` does.
///
/// **Animals are drawn by `CreatureColour::Colony`**, which is the lab's own
/// default and is the entire reason a border is visible at all: the two
/// colonies wear different colours, so *where the line is* -- which §2d of
/// the report says is the readout a tournament actually produces -- can be
/// seen rather than inferred.
///
/// **Movement, not stills**: the coordinator note records that a scrubbable
/// sequence is how animals get seen here and that a grid of stills is not,
/// and a follow camera ruins a colony card ("shaking gif"), so this holds
/// the whole bed still and lets the animals move inside it.
fn render_gif(cfg: &Cfg, seed: u64, out: &str, every: u64, delay_ms: u64) -> Row {
    use pixel_physics::render::{CreatureColour, Renderer};
    let mut ctx = Setup::new(cfg, seed);
    // **The capture viewport is not the world size**, and it has to be
    // sayable separately. The review skill records a card that went out at
    // 190x130 and was judged on nothing, because at that size there was
    // nothing to see; the stills the owner has actually been able to read
    // are 700-950 px across. A bed is 384 cells wide, so the only way to
    // reach that is a viewport larger than the world with a zoom inside it.
    // Defaults to the world, which is what every earlier call assumed.
    let (vw, vh) = match arg_str("shot") {
        Some(spec) => {
            let v: Vec<u32> = spec.split('x').map(|p| p.trim().parse().expect("shot wants WxH")).collect();
            assert_eq!(v.len(), 2, "shot wants exactly WxH, got {spec:?}");
            (v[0], v[1])
        }
        None => (cfg.width as u32, cfg.height as u32),
    };
    let mut renderer = Renderer::new();
    renderer.creature_colour = CreatureColour::Colony;
    // **`zoom=` and `look=` are a pair and neither works alone.** `adjust_zoom`
    // decides how big a cell is and leaves the camera wherever it defaulted --
    // which on this box is the ceiling, so a zoomed sheet of a bed comes back
    // as rows of empty air. `labshot` records the same trap. An ant is two
    // cells, and the coordinator note is explicit that a one-cell event is
    // unreadable on a card even ringed: at 1x this GIF shows the *shape* of a
    // border and at 3x it shows the animals holding it.
    let zoom: u32 = arg("zoom").unwrap_or(1);
    for _ in 1..zoom {
        renderer.adjust_zoom(1);
    }
    if let Some(spec_look) = arg_str("look") {
        let v: Vec<i32> = spec_look.split(',').map(|p| p.trim().parse().expect("look wants x,y")).collect();
        assert_eq!(v.len(), 2, "look wants exactly x,y, got {spec_look:?}");
        let bounds = pixel_physics::sim::chunk::Rect::new(0, 0, cfg.width - 1, cfg.height - 1);
        renderer.set_camera(v[0], v[1], (vw, vh), Some(bounds));
    }
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    let seq_dir = arg_str("seq");
    if let Some(dir) = seq_dir.as_deref() {
        std::fs::create_dir_all(dir).expect("seq= wants a directory this process may create");
    }
    for f in 1..=cfg.frames {
        ctx.step();
        if !f.is_multiple_of(every) {
            continue;
        }
        let mut buf = vec![0u8; (vw * vh * 4) as usize];
        let touched = ctx.world.take_touched_chunks();
        renderer.draw(&ctx.world, &ctx.particles, &touched, &mut buf, (vw, vh), true);
        shots.push(image::RgbaImage::from_raw(vw, vh, buf).expect("the buffer is exactly vw*vh*4"));
        // **`seq=<dir>` writes every captured frame as its own PNG**, which
        // is what a scrubbable review card wants. The review skill is
        // explicit that a frame sequence is preferred over a GIF -- tested
        // head to head, the sequence played for the owner and the GIF, valid
        // by every check available on the posting side, showed as one static
        // frame. Same buffer, so the two outputs cannot disagree.
        if let Some(dir) = seq_dir.as_deref() {
            let path = format!("{dir}/f{:04}.png", shots.len() - 1);
            if let Err(e) = shots.last().expect("just pushed").save(&path) {
                eprintln!("conflict_arena: could not write {path}: {e}");
            }
        }
    }
    let row = ctx.finish(seed);
    // **`still=<path>` writes the last captured frame as a PNG.** Not a
    // second render path: it is the same buffer the GIF's final frame is
    // made of. It exists because framing a card is an iteration -- `zoom=`
    // and `look=` are a pair that is easy to get wrong (see above) -- and
    // checking the framing by re-encoding a whole animation is slow enough
    // that the temptation is to skip the check.
    if let (Some(path), Some(last)) = (arg_str("still"), shots.last()) {
        if let Err(e) = last.save(&path) {
            eprintln!("conflict_arena: could not write {path}: {e}");
        } else {
            println!("conflict_arena: wrote {path}");
        }
    }
    let delay = image::Delay::from_numer_denom_ms(delay_ms as u32, 1);
    let frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
    let n = frames.len();
    match std::fs::File::create(out) {
        Ok(file) => {
            let mut encoder = image::codecs::gif::GifEncoder::new(file);
            if let Err(e) = encoder.set_repeat(image::codecs::gif::Repeat::Infinite) {
                eprintln!("conflict_arena: gif set_repeat failed: {e}");
            }
            if let Err(e) = encoder.encode_frames(frames) {
                eprintln!("conflict_arena: gif encode failed: {e}");
            }
            println!("conflict_arena: wrote {out} -- {n} frames, every={every}, delay={delay_ms}ms");
        }
        Err(e) => eprintln!("conflict_arena: could not create {out}: {e}"),
    }
    row
}

/// **One arena world, set up once and then either stepped to the end or
/// stepped with a camera on it.**
///
/// Split out of `run_world` so that the GIF and the census run **the same
/// bed**, not two beds that agree by inspection. A card rendered from a
/// second construction of "the same" world is the shape `CLAUDE.md`'s
/// stale-binary gotcha keeps arriving in: two things that must be identical,
/// kept identical by hand.
struct Setup {
    world: pixel_physics::sim::world::World,
    particles: ParticleSystem,
    blasts: Blasts,
    tuning: player::Tuning,
    species_id: pixel_physics::sim::organism::SpeciesId,
    /// Colony labels in minting order.
    labels: Vec<u32>,
    founded: usize,
}

impl Setup {
    fn new(cfg: &Cfg, seed: u64) -> Self {
        // **`colonies: 0`, and the founding is done by hand below.** `LabBox`
        // spreads its colonies evenly and founds them during `build()`, which
        // is before the species can be patched -- and `scent_spread` is read
        // at *placement* (`creature::apply_colony_scent`), so a spread written
        // after `build()` reaches nobody and the bed silently stays one
        // family. That is the same trap `labforage`'s `hidden=` records from
        // the genome side.
        let spec = LabBox { seed, colonies: 0, founders: cfg.plants, width: cfg.width, height: cfg.height, ..LabBox::default() };
        let mut w = spec.build();
        let species_name = arg_str("species").unwrap_or_else(|| LabBox::default().colony_species);
        let species_id = w.species.id_of(&species_name).unwrap_or_else(|| panic!("species {species_name} is not compiled in"));

        // --- the edits, applied to the species and therefore to both sides ---
        let mut def = w.species.get(species_id).creature.as_ref().expect("the colony species is a creature").clone();
        def.scent_spread = cfg.spread;
        if let Some(s) = cfg.sight {
            def.sight_range = s;
        }
        w.species.set_creature(species_id, def);

        // **The `Attack` weight goes on the species genome, before founding.**
        // Founders copy it at placement and every child inherits it, so a
        // colony that breeds does not quietly revert to the shipped pacifist.
        // Writing it per founder afterwards -- which is what `creature_arena`
        // does, for its own good reason -- would leave the second generation
        // unarmed and turn a long run into an accidental sweep of how long
        // aggression lasts.
        let mut genome = w.species.get(species_id).genome.clone();
        genome[brain::io_slot(BrainInput::Bias, BrainOutput::Attack)] = cfg.attack_w;
        w.species.set_genome(species_id, genome);

        // --- two colonies, placed symmetrically about the middle -----------
        let half = (cfg.gap.clamp(0.0, 1.0) * cfg.width as f32 / 2.0) as i32;
        let mid = cfg.width / 2;
        let mut founded = 0usize;
        for x in [mid - half, mid + half] {
            if w.found_colony_of(x, spec.ground_y, &species_name, cfg.ants) > 0 {
                founded += 1;
            }
        }

        // Colony labels in minting order, recovered from the grid rather than
        // assumed to be 1 and 2: `next_colony` is a world-wide counter and a
        // bed that painted a nest first would shift them.
        let mut labels: Vec<u32> = Vec::new();
        for id in w.live_organism_ids() {
            if let Some(s) = w.organism(id) {
                if s.species == species_id && !labels.contains(&s.colony) {
                    labels.push(s.colony);
                }
            }
        }
        labels.sort_unstable();

        if let Some(a) = cfg.armour_b {
            // The *second* colony by minting order. Read from the grid
            // rather than assumed, exactly as the labels are.
            if let Some(&target) = labels.get(1) {
                for id in w.live_organism_ids() {
                    if w.organism(id).is_some_and(|st| st.species == species_id && st.colony == target) {
                        w.set_organism_trait(id, pixel_physics::sim::organism::TRAIT_ARMOUR, a);
                    }
                }
            }
        }

        Setup { world: w, particles: ParticleSystem::default(), blasts: Blasts::default(), tuning: player::Tuning::default(), species_id, labels, founded }
    }

    fn step(&mut self) {
        frame::step(&mut self.world, &mut self.particles, &mut self.blasts, player::PlayerInput::default(), &self.tuning);
    }

    fn finish(&self, seed: u64) -> Row {
        let w = &self.world;
        let mut row = Row { seed, colonies_founded: self.founded, ..Row::default() };
        for id in w.live_organism_ids() {
            let Some(s) = w.organism(id) else { continue };
            if s.species != self.species_id {
                continue;
            }
            if let Some(i) = self.labels.iter().position(|&c| c == s.colony) {
                if i < row.alive.len() {
                    row.alive[i] += 1;
                }
            }
        }
        // **Read off the kill log rather than off either verb's counter**, so
        // this column cannot agree with the fight by construction. A colony
        // eaten by the ordinary mouth and a colony beaten by `Attack` are the
        // same number here, which is what makes the two counters readable.
        for k in &w.kills_log {
            if k.attacker_colony == k.victim_colony {
                row.own_kills += 1;
            } else {
                row.cross_kills += 1;
            }
        }
        let st = &w.creature_stats;
        row.contests = st.contests;
        row.attacks = st.attacks;
        row.displays = st.displays;
        row.attack_cells = st.attack_cells;
        row.attack_kills = st.attack_kills;
        row.gnaws = st.gnaws;
        row.eats = st.eats;
        row.casts = st.sight_casts;
        row.prey_seen = st.sightings;
        row.threat_seen = st.threat_sightings;
        row.deaths = st.deaths;

        // **The premise, measured rather than assumed.** Every pair, both
        // ways round -- `scent_accepts` is deliberately asymmetric (the
        // judge's tolerance against the other's scent), so counting unordered
        // pairs would average away exactly the asymmetry `TRAIT_TOLERANCE`
        // exists to express. Capped at a sample so a big bed does not turn an
        // O(n^2) census into the run's dominant cost.
        //
        // **It saturates at 0.5 and that is the healthy reading, not a bug**:
        // with two equal colonies, half of all ordered pairs are cross-colony,
        // so 0.5 means *every* stranger pair reads as a stranger and every
        // nestmate pair still reads as kin. A number above that would mean the
        // colonies had started rejecting their own.
        let mut traits: Vec<[f32; pixel_physics::sim::organism::CREATURE_TRAITS]> = Vec::new();
        let mut groups: Vec<u32> = Vec::new();
        for id in w.live_organism_ids() {
            let Some(s) = w.organism(id) else { continue };
            if s.species != self.species_id {
                continue;
            }
            if !groups.contains(&s.colony) {
                groups.push(s.colony);
            }
            if traits.len() < 160 {
                traits.push(pixel_physics::sim::creature::expressed_traits(s, w.plasticity, w.trait_reach));
            }
        }
        row.groups = groups.len();
        let (mut pairs, mut strangers) = (0u64, 0u64);
        for (i, a) in traits.iter().enumerate() {
            for (j, b) in traits.iter().enumerate() {
                if i == j {
                    continue;
                }
                pairs += 1;
                if !pixel_physics::sim::creature::scent_accepts(a, b) {
                    strangers += 1;
                }
            }
        }
        row.stranger_frac = if pairs == 0 { f64::NAN } else { strangers as f64 / pairs as f64 };
        row
    }
}

fn run_world(cfg: &Cfg, seed: u64) -> Row {
    let mut ctx = Setup::new(cfg, seed);
    for _ in 0..cfg.frames {
        ctx.step();
    }
    ctx.finish(seed)
}

/// **The positive control, and its specificity half.**
///
/// `CLAUDE.md` asks for both and this repo has paid six times for running
/// only one. The sensitivity arm puts two colonies in contact as strangers
/// and asserts every counter this file exists to print actually moves; the
/// specificity arm runs the identical bed at `scent_spread = 0` — the shipped
/// setting, where every ant in the box is every other's nestmate — and
/// asserts the conflict counters stay at exactly zero. A harness that reports
/// a war in a one-family bed is measuring something else.
fn selftest() {
    let base = Cfg { frames: 4_000, ants: 24, plants: 4, spread: 1.0, sight: None, attack_w: 4.0, gap: 0.12, width: 384, height: 224, armour_b: None };
    let hot = run_world(&base, 0);
    println!("selftest  strangers: {}", line(&hot));
    let cold = run_world(&Cfg { spread: 0.0, ..base }, 0);
    println!("selftest  one family: {}", line(&cold));

    assert_eq!(hot.colonies_founded, 2, "the bed founded {} colonies; with one colony there is nothing to contest", hot.colonies_founded);
    assert!(hot.contests > 0, "no encounter was ever assessed, so every column below it is a zero about a bed that never met");
    assert!(hot.escalations() > 0, "encounters were assessed and none escalated; the fight cannot be measured through a harness that never produces one");
    assert!(hot.displays > 0, "every encounter escalated -- the mechanic has no middle, which is the thing this file is built to see");
    assert!(hot.cross_kills > 0, "strangers in contact for {} frames killed none of each other", base.frames);

    // The specificity half. `scent_spread = 0` is the shipped bed: one
    // family, so `nearest_foe` finds nobody and the whole verb is unreachable
    // however large the `Attack` weight is.
    assert_eq!(cold.contests, 0, "a one-family bed assessed {} encounters; the kin predicate is not reaching this verb", cold.contests);
    assert_eq!(cold.displays, 0, "a one-family bed produced {} displays, which it has no stranger to display at", cold.displays);
    // **Not `attacks`, and the difference is the point.** A one-family bed
    // still bites vegetation, because the target rule is any non-kin
    // organism; asserting `attacks == 0` here was the first version and it
    // went red on a bed that was behaving correctly.
    assert_eq!(cold.escalations(), 0, "a one-family bed fought {} times", cold.escalations());
    assert_eq!(cold.cross_kills, 0, "a one-family bed produced {} cross-colony kills, which it has no second colony to produce", cold.cross_kills);

    // And the arithmetic, which needs no bed at all: the assessment must be
    // able to say *no*, and must never say never.
    use pixel_physics::sim::contest;
    let hopeless = contest::Assessment { mine: 0.0, theirs: 1.0, numbers: -1.0 };
    let easy = contest::Assessment { mine: 1.0, theirs: 0.0, numbers: 1.0 };
    let (lo, hi) = (contest::commitment(hopeless, 4.0, 1.0), contest::commitment(easy, 4.0, 1.0));
    assert!(lo >= contest::COMMIT_FLOOR, "the floor is the capacity-not-exemption rule and it did not hold: {lo}");
    assert!(hi > 0.9 && lo < 0.2, "the assessment does not span the decision: hopeless {lo}, easy {hi}");
    println!("selftest  PASS");
}

fn line(r: &Row) -> String {
    let esc = if r.escalation().is_nan() { "-".to_string() } else { format!("{:.3}", r.escalation()) };
    format!(
        "seed {:>2}  alive {:>4}/{:<4}  contests {:>7}  fights {:>7}  displays {:>7}  escalation {:>6}  plantbites {:>7}  cells {:>6}  kills {:>5}  eats {:>7}  gnaws {:>7}  x-kills {:>5}  own-kills {:>5}  deaths {:>6}  casts {:>9}  prey {:>7}  threat {:>7}  groups {:>3}  strangers {:.3}",
        r.seed,
        r.alive[0],
        r.alive[1],
        r.contests,
        r.escalations(),
        r.displays,
        esc,
        r.plant_bites(),
        r.attack_cells,
        r.attack_kills,
        r.eats,
        r.gnaws,
        r.cross_kills,
        r.own_kills,
        r.deaths,
        r.casts,
        r.prey_seen,
        r.threat_seen,
        r.groups,
        r.stranger_frac
    )
}

/// Median of a sorted-in-place copy. The order statistic the lab reads
/// everything at, rather than a mean that one runaway seed owns.
fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.partial_cmp(b).expect("no NaNs are pushed here"));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

fn main() {
    // **The dials are set here, before any world exists.** `sim::contest`
    // reads them once into a `OnceLock`, which is what makes them free at the
    // call site -- and it means they have to be in the environment before the
    // first fight tick, not before the first `main` line. Setting them from
    // an argument keeps the A/B inside one binary, which is `CLAUDE.md`'s
    // stale-binary gotcha made avoidable: a recompile between two arms is the
    // thing that actually changed four times in this repo's history.
    if let Some(v) = arg_str("assess") {
        std::env::set_var("PIXEL_PHYSICS_CONTEST", v);
    }
    if let Some(v) = arg_str("boldness") {
        std::env::set_var("PIXEL_PHYSICS_CONTEST_BOLDNESS", v);
    }
    if let Some(v) = arg_str("numbers") {
        std::env::set_var("PIXEL_PHYSICS_CONTEST_NUMBERS", v);
    }
    if let Some(v) = arg_str("display") {
        std::env::set_var("PIXEL_PHYSICS_CONTEST_DISPLAY", v);
    }

    if arg_str("control").as_deref() == Some("selftest") {
        selftest();
        return;
    }

    let cfg = Cfg {
        frames: arg("frames").unwrap_or(12_000),
        ants: arg("ants").unwrap_or(32),
        plants: arg("plants").unwrap_or(6),
        // **1.0 by default, and that is the arena's premise rather than a
        // tuning choice.** At the shipped 0 every colony sits at the species'
        // authored scent point, so two clicks are one family and there is no
        // contest to measure -- the bed is a supercolony, in the precise
        // sense the invasive-ant literature uses the word. `spread=0` is
        // kept reachable because it is this harness's own negative control.
        spread: arg("spread").unwrap_or(1.0),
        sight: arg("sight"),
        attack_w: arg("attack").unwrap_or(4.0),
        gap: arg("gap").unwrap_or(0.25),
        width: arg("width").unwrap_or(512),
        height: arg("height").unwrap_or(320),
        armour_b: arg("armour"),
    };
    let seeds: u64 = arg("seeds").unwrap_or(4);

    println!(
        "conflict_arena  frames={} seeds={} ants={}/colony plants={} spread={} sight={:?} attack={} gap={} armour_b={:?} {}x{}  assess={} boldness={} numbers={} display={}",
        cfg.frames,
        seeds,
        cfg.ants,
        cfg.plants,
        cfg.spread,
        cfg.sight,
        cfg.attack_w,
        cfg.gap,
        cfg.armour_b,
        cfg.width,
        cfg.height,
        pixel_physics::sim::contest::enabled(),
        pixel_physics::sim::contest::boldness(),
        pixel_physics::sim::contest::numbers_weight(),
        pixel_physics::sim::contest::display_deposit(),
    );

    // **`gif=<path>` renders one seed instead of censusing several**, because
    // a card is a picture of one world and a finding is a median over many.
    // Printed with the same row every other run prints, so the counters
    // `CLAUDE.md`'s review-card convention asks for beside the image come out
    // of the same census the tables do.
    if let Some(out) = arg_str("gif") {
        let seed: u64 = arg("seed").unwrap_or(0);
        let row = render_gif(&cfg, seed, &out, arg("every").unwrap_or(30), arg("delay").unwrap_or(60));
        println!("{}", line(&row));
        return;
    }

    let rows: Vec<Row> = (0..seeds).map(|s| run_world(&cfg, s)).collect();
    for r in &rows {
        println!("{}", line(r));
    }

    // **The summary is medians, and the escalation median skips the seeds
    // that met nobody** -- a run with no encounters has no escalation rate,
    // and folding it in as a zero would report a peaceful border where there
    // was no border at all.
    let met: Vec<f64> = rows.iter().filter(|r| r.contests > 0).map(|r| r.escalation()).collect();
    let n_met = met.len();
    println!(
        "SUMMARY contests {} fights {} displays {} escalation_median {:.3} met_seeds {}/{} x_kills {} own_kills {} eats {} gnaws {} plantbites {} strangers_median {:.3} groups_median {:.1} survivors_median {:.1}/{:.1}",
        median(rows.iter().map(|r| r.contests as f64).collect()),
        median(rows.iter().map(|r| r.escalations() as f64).collect()),
        median(rows.iter().map(|r| r.displays as f64).collect()),
        median(met),
        n_met,
        rows.len(),
        median(rows.iter().map(|r| r.cross_kills as f64).collect()),
        median(rows.iter().map(|r| r.own_kills as f64).collect()),
        median(rows.iter().map(|r| r.eats as f64).collect()),
        median(rows.iter().map(|r| r.gnaws as f64).collect()),
        median(rows.iter().map(|r| r.plant_bites() as f64).collect()),
        median(rows.iter().filter(|r| !r.stranger_frac.is_nan()).map(|r| r.stranger_frac).collect()),
        median(rows.iter().map(|r| r.groups as f64).collect()),
        median(rows.iter().map(|r| r.alive[0] as f64).collect()),
        median(rows.iter().map(|r| r.alive[1] as f64).collect()),
    );
}
