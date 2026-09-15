//! **Why two colonies in one box never fight, and what each missing piece is
//! worth on its own.**
//!
//! The owner's question ("why don't different colonies fight or eat each
//! other") has three candidate answers in the code, and they are not
//! alternatives — they are three links of one chain, any one of which alone
//! holds the whole thing shut:
//!
//! 1. **They are not strangers.** `CreatureDef::scent_spread` is `0`, so
//!    every colony founds at the species' authored scent point and
//!    `creature::is_living_kin` returns true for every ant-to-ant pair in
//!    the box. `nearest_foe` skips kin, so **no ant in the shipped bed has a
//!    target at all**, whatever its brain wants.
//! 2. **They are blind.** Every ant variant authors no `sight_range`, so
//!    `PreyNear`/`ThreatNear`/`KinNear` all read a constant 0 and nothing
//!    steers an ant toward another colony from more than one cell away.
//! 3. **Nothing initiates.** `ant.ron` wires exactly one route to
//!    `BrainOutput::Attack`: `(Alarm, Attack, 2.0)`. `Alarm` is raised only
//!    by `cry_alarm`, which is called only from a bite that already landed.
//!    The wire is **retaliation**, and there is nothing to retaliate against.
//!
//! So this harness does not measure "attacks" alone — a zero there is
//! consistent with all three and distinguishes none of them. It measures the
//! chain, link by link, on one run:
//!
//! - `strangers%` — of sampled living ant pairs, how many are mutually
//!   outside each other's tolerance. **Link 1's own readout.** Zero here and
//!   nothing downstream can fire, whatever else is set.
//! - `contacts` — creature ticks on which a living ant had a living
//!   **non-kin** animal in the same 8-neighbourhood `nearest_foe` walks.
//!   **The opportunity count**, and the number that separates "they never
//!   meet" from "they meet and do not bite". Computed here, in the harness,
//!   with `creature::scent_accepts` — the same predicate the verb uses — so
//!   the eye and the mouth cannot disagree.
//! - `attacks` / `cells` / `kills` — `CreatureStats`, the verb's own
//!   counters, paired exactly as `CLAUDE.md` requires: `attacks` is "it
//!   fired", `cells` and `kills` are the effect counters from the far side
//!   of the call. 23 swings removing 0 cells is the failure this pairing is
//!   written against.
//! - `xcol` / `own` — kills split by whether attacker and victim shared a
//!   colony label, off `World::kills_log`. **"Colonies fighting" is `xcol`,
//!   not `attacks`**: a colony that bites its own is a different finding.
//! - deaths by cause, and `starv%` — the owner's bed reports every death as
//!   starvation. If a gap closes and the only thing that moves is *which*
//!   deaths happen, the bed has not gained a war, it has gained a new way to
//!   die.
//! - `corpse_j` / `plant_j` — `EnergyLedger`, for the **eating** half of the
//!   question, which is separate from the fighting half. Corpses carry no
//!   colony, so a colony already scavenges its neighbours' dead and nothing
//!   in the engine says so; this is as close as a global ledger gets.
//!
//! ```text
//! cargo run --release --example rivalry -- control=selftest    # positive + specificity controls, ~seconds
//! cargo run --release --example rivalry -- frames=24000 seed=2
//! cargo run --release --example rivalry -- spread=1 tolerance=-1        # link 1 alone
//! cargo run --release --example rivalry -- sight=32                     # link 2 alone
//! cargo run --release --example rivalry -- wire=ThreatNear:Attack:2.0   # link 3 alone
//! ```
//!
//! **It echoes its own parameters on the first line and prints one
//! `SUMMARY` line per run**, per `CLAUDE.md`'s harness rule: a knob nobody
//! can see the value of is a knob nobody can tell is disconnected, and a
//! 3.5-hour study in this repo produced eight byte-identical logs for
//! exactly that reason.

use pixel_physics::sim::cell::OrganismId;
use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::Lab;
use pixel_physics::sim::creature;
use pixel_physics::sim::organism::{self, DEATH_CAUSE_LIST};
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// One trait allele on the species vector **and** on every standing animal
/// of it. `Lab::new` has already founded the bed by the time an override
/// runs and `place_creature` copies the genome at placement, so a
/// species-only write reaches nobody who is already on the ground — the
/// trap `labstats`' `beetlesight=` shipped with as a printed caveat.
fn set_allele_on(lab: &mut Lab, species: &str, slot: usize, v: f32) -> usize {
    let Some(sid) = lab.world.species.id_of(species) else { return 0 };
    if let Some(def) = lab.world.species.get(sid).creature.as_ref() {
        let mut def = def.clone();
        def.traits[slot] = v;
        lab.world.species.set_creature(sid, def);
    }
    let living = ants_of(lab, species);
    for id in &living {
        lab.world.set_organism_trait(*id, slot, v);
    }
    living.len()
}

/// [`set_allele_on`] for a raw genome index — a brain weight rather than a
/// body trait.
fn set_genome_slot_on(lab: &mut Lab, species: &str, slot: usize, weight: f32) -> usize {
    let Some(sid) = lab.world.species.id_of(species) else { return 0 };
    let mut genome = lab.world.species.get(sid).genome.clone();
    genome[slot] = weight;
    lab.world.species.set_genome(sid, genome);
    let living = ants_of(lab, species);
    for id in &living {
        let mut g = lab.world.organism(*id).expect("just filtered live").genome.clone();
        g[slot] = weight;
        lab.world.set_organism_genome(*id, g);
    }
    living.len()
}

fn ants_of(lab: &Lab, species: &str) -> Vec<OrganismId> {
    lab.world
        .live_organism_ids()
        .into_iter()
        .filter(|id| lab.world.organism(*id).is_some_and(|st| lab.world.species.get(st.species).name == species))
        .collect()
}

/// **Every living animal's head, with its expressed traits and colony.**
/// Expressed rather than raw, because `is_living_kin` compares expressed
/// vectors and a developmental channel can move a signature.
fn standing(world: &World) -> Vec<(OrganismId, i32, i32, u32, [f32; organism::CREATURE_TRAITS])> {
    let mut out = Vec::new();
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        if world.species.get(st.species).creature.is_none() {
            continue;
        }
        let Some(&(x, y)) = st.chain.first() else { continue };
        out.push((id, x, y, st.colony, creature::expressed_traits(st, world.plasticity, world.trait_reach)));
    }
    out
}

/// **The opportunity count: living animals standing in each other's
/// `nearest_foe` ring and mutually outside each other's tolerance.**
///
/// The whole point of this census is that `attacks == 0` is ambiguous, and
/// this is the number that disambiguates it. It uses `scent_accepts` —
/// the predicate `is_living_kin` is built out of — rather than a
/// hand-rolled distance test, for the standing house reason: a readout
/// derived separately from the verb is a readout that can be wrong on its
/// own.
///
/// **Counted over whole bodies, not heads**, because `nearest_foe` walks
/// every cell of the attacker's chain. Pairs, so an adjacency is one
/// contact rather than two.
fn contacts(world: &World) -> (u64, u64) {
    let mut all = 0u64;
    let mut cross = 0u64;
    let live = standing(world);
    for (i, (ida, _, _, cola, ta)) in live.iter().enumerate() {
        let Some(sta) = world.organism(*ida) else { continue };
        for (idb, _, _, colb, tb) in live.iter().skip(i + 1) {
            let Some(stb) = world.organism(*idb) else { continue };
            let touching = sta
                .chain
                .iter()
                .any(|&(ax, ay)| stb.chain.iter().any(|&(bx, by)| (ax - bx).abs() <= 1 && (ay - by).abs() <= 1));
            if !touching {
                continue;
            }
            all += 1;
            // Either direction: `nearest_foe` reads the *attacker's* gut, so
            // A may have a target in B while B has none in A. A contact is a
            // chance for a fight if either side would take it.
            if !creature::scent_accepts(ta, tb) || !creature::scent_accepts(tb, ta) {
                cross += 1;
                if cola == colb {
                    // A drifted split inside one label is still a stranger
                    // pair; the label is a name, the scent is the rule.
                }
            }
        }
    }
    (all, cross)
}

/// **Who is a stranger to whom, split by whether the pair share a colony
/// label — and the split is the whole value of this readout.**
///
/// A single pooled "strangers %" cannot tell the two findings apart, and
/// they want opposite responses. **Between** colonies is the thing being
/// built: two cohesive groups that are foreign to each other. **Within** a
/// colony is the failure mode `ant.ron`'s own `scent_drift` comment
/// records — *"measured at 0.5, `ANT 1 killed 22, 20 of them by ANT 1
/// itself`, the founding group wiped out"* — a colony eating itself, which
/// reads as rivalry in every pooled number and is the opposite of it.
///
/// Returns `(animals, between %, within %, mean between-colony distance)`.
/// The last is the one that explains a null: at `scent_spread = 1` the
/// per-colony offsets are a *draw*, so two colonies can land 0.6 apart
/// against a tolerance radius of 1.0 and be family however the dial is set.
/// Without it, a seed where the dial did nothing is indistinguishable from
/// a dial that is disconnected — the `include_str!` tell, met from the
/// other side.
fn stranger_share(world: &World) -> (usize, f64, f64, f64) {
    let sample: Vec<(u32, [f32; organism::CREATURE_TRAITS])> =
        standing(world).into_iter().map(|(_, _, _, col, t)| (col, t)).collect();
    let (mut bp, mut bs, mut wp, mut ws) = (0u64, 0u64, 0u64, 0u64);
    let (mut dsum, mut dn) = (0.0f64, 0u64);
    for (i, (ca, a)) in sample.iter().enumerate() {
        for (j, (cb, b)) in sample.iter().enumerate() {
            if i == j {
                continue;
            }
            let stranger = !creature::scent_accepts(a, b);
            if ca == cb {
                wp += 1;
                ws += u64::from(stranger);
            } else {
                bp += 1;
                bs += u64::from(stranger);
                if i < j {
                    dsum += creature::scent_distance_sq(&creature::scent_of(a), &creature::scent_of(b)).sqrt() as f64;
                    dn += 1;
                }
            }
        }
    }
    let pct = |n: u64, d: u64| if d > 0 { 100.0 * n as f64 / d as f64 } else { 0.0 };
    (sample.len(), pct(bs, bp), pct(ws, wp), if dn > 0 { dsum / dn as f64 } else { 0.0 })
}

/// Standing corpse cells, for the **eating** half of the question: a corpse
/// is a `Powder` and carries no colony, so it is edible by anybody and the
/// global ledger cannot say whose it was.
fn corpse_cells(world: &World, w: i32, h: i32) -> u64 {
    let Some(id) = world.materials.id_of("corpse") else { return 0 };
    let mut n = 0u64;
    for y in 0..h {
        for x in 0..w {
            if world.get(x, y).material == id {
                n += 1;
            }
        }
    }
    n
}

fn tick(lab: &mut Lab) {
    pixel_physics::sim::frame::step(
        &mut lab.world,
        &mut lab.particles,
        &mut lab.blasts,
        pixel_physics::sim::player::PlayerInput::default(),
        &pixel_physics::sim::player::Tuning::default(),
    );
}

fn main() {
    let control: String = arg("control").unwrap_or_default();
    if control == "selftest" {
        return selftest();
    }

    let frames: u64 = arg("frames").unwrap_or(24_000);
    let seed: u64 = arg("seed").unwrap_or(2);
    let colonies: usize = arg("colonies").unwrap_or(2);
    let founders: usize = arg("founders").unwrap_or(8);
    let spread: Option<f32> = arg("spread");
    let tolerance: Option<f32> = arg("tolerance");
    let drift: Option<f32> = arg("drift");
    let sight: Option<f32> = arg("sight");
    let wire: Option<String> = arg("wire");
    let label: String = arg("label").unwrap_or_else(|| "-".to_string());
    // **The card, and for creatures it has to be a GIF.** The owner, on a
    // contact sheet of a starving colony: *"visually, I cannot tell anything
    // from these. ants are mostly visible with there motion."* A grid of
    // stills cannot answer "is two colonies meeting worth watching"; only an
    // animation can. Captured through `Lab::draw` -- the identical call
    // `bin/lab.rs` makes every frame, bar and all -- so the card is the
    // thing the player sees, which is `labgif`'s own argument for its
    // existence and the reason this reuses the approach rather than a
    // renderer of its own.
    let gif: Option<String> = arg("gif");
    let gif_start: u64 = arg("gifstart").unwrap_or(0);
    let gif_every: u64 = arg("gifevery").unwrap_or(20);
    let gif_shots: usize = arg("gifshots").unwrap_or(150);
    let gif_delay: u64 = arg("gifdelay").unwrap_or(60);
    // **A frame sequence as well as the GIF, and the skill says prefer it.**
    // Tested head to head on one card with the same motion posted both ways,
    // the sequence played and the GIF did not -- the page's own timer does
    // not depend on the browser decoding a GIF. So a card about motion gets
    // the sequence; the GIF rides along for feel.
    let png_dir: Option<String> = arg("pngdir");
    // Nearest-neighbour integer upscale, for the sequence only. The stills
    // the owner has been able to judge are 700-950 px across and the bed is
    // 512 wide; `zoom=2` puts it at 1,024. Never applied to the GIF, which
    // it would only make heavier.
    let zoom: u32 = arg("zoom").unwrap_or(2).max(1);
    // **`crop=x,y,w,h` in world cells, applied before the zoom.** An ant is
    // two cells; at the whole bed's 512 width and a legible card size it is
    // four pixels, and the owner's standing verdict on creature cards is
    // that he cannot tell anything from a picture in which the animal cannot
    // be picked out. Cropping to the band where two colonies meet and then
    // zooming is what makes the fight the subject of the card rather than a
    // detail of it. Clamped to the frame, so an over-wide crop is a smaller
    // picture rather than a panic.
    let crop: Option<(u32, u32, u32, u32)> = arg::<String>("crop").map(|t| {
        let v: Vec<u32> = t.split(',').map(|p| p.trim().parse().expect("crop wants x,y,w,h")).collect();
        assert_eq!(v.len(), 4, "crop wants exactly x,y,w,h, got {t:?}");
        (v[0], v[1], v[2], v[3])
    });

    // **`colony_ants=` -- how many founders stand together, and at a live
    // rivalry dial that is also how big a war party is.** One of the three
    // constants #423's commit names as calibrated on a bed where no ant was
    // food. It was a bed field and a lab slider (`lab/params.rs`, span
    // 1..120) and reachable from no harness, so the one question nobody
    // could ask of it was what it does to a bed with two colonies in it.
    let colony_ants: i32 = arg("colony_ants").unwrap_or(creature::COLONY_ANTS);
    let spec = LabBox {
        width: arg("width").unwrap_or(512),
        height: arg("height").unwrap_or(320),
        soil_depth: arg("soil").unwrap_or(80),
        founders,
        colonies,
        colony_ants,
        compartments: arg("walls").unwrap_or(1),
        seed,
        ..LabBox::default()
    };
    println!(
        "rivalry: label={label} frames={frames} seed={seed} colonies={colonies} colony_ants={colony_ants} founders={founders} walls={} spread={} tolerance={} drift={} sight={} wire={}",
        spec.compartments,
        spread.map_or("-".into(), |v| v.to_string()),
        tolerance.map_or("-".into(), |v| v.to_string()),
        drift.map_or("-".into(), |v| v.to_string()),
        sight.map_or("-".into(), |v| v.to_string()),
        wire.as_deref().unwrap_or("-"),
    );

    let dims = (spec.width, spec.height);
    let mut lab = Lab::new(spec);
    // **Both overlays off before anything is drawn.** `Lab::new` opens with
    // the key-list help page up and `Stats::new` with the biosphere page
    // showing, and a card taken before both are closed is a picture of the
    // help page rather than of the box -- two display faults `labgif` had to
    // find by looking rather than by reading its diff.
    lab.show_help = false;
    if lab.stats.showing() {
        lab.stats.toggle();
    }

    // **The value the bed is actually founded at, echoed whether or not
    // `spread=` was passed.** It stopped being safe to leave implicit the
    // day `ant.ron` began authoring a non-zero `scent_spread`: before that a
    // log with no `spread=` on it meant "the unswitched bed" and now it
    // means "whatever the species file says today". `CLAUDE.md`'s harness
    // rule is that a knob nobody can see the value of is a knob nobody can
    // tell is disconnected, and this is its other half — a *default* nobody
    // can see the value of is a default nobody can tell has moved. Read off
    // the species registry rather than from the argument, so the two routes
    // into this field cannot be confused in a log.
    println!(
        "rivalry: ant scent_spread as authored = {} (species file), scent_drift = {}",
        lab.world
            .species
            .id_of("ant")
            .and_then(|id| lab.world.species.get(id).creature.as_ref().map(|d| d.scent_spread))
            .unwrap_or(0.0),
        lab.world
            .species
            .id_of("ant")
            .and_then(|id| lab.world.species.get(id).creature.as_ref().map(|d| d.scent_drift))
            .unwrap_or(0.0),
    );

    // **The economy this bed is founded on, echoed in the units the
    // questions are asked in.** A stranger is food now, so the three
    // constants the switch reallocates -- the birth bar, what a mouthful of
    // rival is worth, and how many founders stand together -- are what any
    // reading of this log is against, and none of them was visible in it.
    // `CLAUDE.md`'s harness rule, applied to the *economy* rather than to
    // the dial: a constant nobody can see the value of is a constant nobody
    // can tell has been re-derived.
    //
    // Priced through `creature::diet_quality`, the same function the mouth
    // credits a swallow with, so this line cannot disagree with the verb.
    if let Some(def) = lab.world.species.id_of("ant").and_then(|id| lab.world.species.get(id).creature.as_ref().cloned()) {
        let gut = def.traits[organism::TRAIT_GUT_BIAS];
        let flesh = lab
            .world
            .materials
            .id_of("ant")
            .map(|m| lab.world.materials.get(m).food_energy * creature::diet_quality(&lab.world, m, gut))
            .unwrap_or(0.0);
        // **Off `BodyPlan::len`, never a literal.** The shipped ant is
        // `Chain(2)`, so a hardcoded 2 would have been right today and
        // silently wrong for `ant_long` and for the day open question #1 is
        // answered -- a constant nobody can see the value of, in the arm of
        // this file whose whole job is making constants visible.
        let body = def.body.len();
        println!(
            "rivalry: economy -- birth bar {:.0} J, start_energy {:.0} J, gut_bias {gut:.2}; one cell of rival flesh yields {flesh:.0} J, \
             so a whole {body}-cell rival is {:.0} J = {:.0}% of one child, and a birth costs {:.1} rivals eaten whole. colony_ants={}",
            def.reproduce_threshold,
            def.start_energy,
            flesh * body as f32,
            if def.reproduce_threshold > 0.0 { 100.0 * flesh * body as f32 / def.reproduce_threshold } else { 0.0 },
            if flesh > 0.0 { def.reproduce_threshold / (flesh * body as f32) } else { 0.0 },
            colony_ants,
        );
    }

    // **Link 1.** The offset is drawn at founding, keyed on the seed and the
    // label (`creature::colony_scent_offset`), so re-deriving it for every
    // standing animal is byte-identical to having founded the bed at this
    // spread — `labstats`' own argument for the same move — and the species'
    // spread is set for anything founded later.
    //
    // **From the species' ancestral point, NOT from the scent the animal is
    // already carrying, and that distinction became load-bearing the day
    // `ant.ron` started authoring a non-zero `scent_spread`.** This block
    // used to add its offset on top of whatever the animal had. While the
    // authored default was 0, founding applied nothing and "add on top" and
    // "re-derive" were the same thing. At a live default they are not:
    // founding applies the authored offset and this block applied a *second*
    // one, so `spread=1` silently meant "authored plus another 1" and
    // `spread=0` was not an off arm at all — it left the authored offset
    // standing while claiming to have removed it.
    //
    // Caught by the check this file's own header demands of everything else:
    // the bed founded from the authored value against the same seed through
    // this override disagreed on `gap` (1.817 against 2.289) while every
    // outcome column matched — matched only because both were past the
    // recognition radius, which is a threshold, so the doubled offset changed
    // no decision and would have gone on not changing one until some arm sat
    // near the boundary. Resetting to `def.traits` first makes `spread=v`
    // mean "founded at v" for any authored default, including 0.
    if let Some(v) = spread {
        let ancestral = lab
            .world
            .species
            .id_of("ant")
            .and_then(|id| lab.world.species.get(id).creature.as_ref().map(|d| d.traits))
            .unwrap_or([0.0; organism::CREATURE_TRAITS]);
        if let Some(id) = lab.world.species.id_of("ant") {
            let mut def = lab.world.species.get(id).creature.as_ref().expect("creature").clone();
            def.scent_spread = v;
            lab.world.species.set_creature(id, def);
        }
        let world_seed = lab.world.seed;
        let living: Vec<(OrganismId, u32)> = lab
            .world
            .live_organism_ids()
            .into_iter()
            .filter_map(|id| lab.world.organism(id).map(|st| (id, st.colony, st.species)))
            .filter(|(_, _, sp)| lab.world.species.get(*sp).name == "ant")
            .map(|(id, col, _)| (id, col))
            .collect();
        for &(id, col) in &living {
            let off = creature::colony_scent_offset(world_seed, col, v);
            for (i, slot) in organism::SCENT_SLOTS.iter().enumerate() {
                lab.world.set_organism_trait(id, *slot, (ancestral[*slot] + off[i]).clamp(-1.0, 1.0));
            }
        }
        println!("rivalry: ant scent_spread = {v}, re-derived from the ancestral point on {} standing ant(s)", living.len());
    }
    if let Some(v) = tolerance {
        let n = set_allele_on(&mut lab, "ant", organism::TRAIT_TOLERANCE, v);
        println!("rivalry: ant tolerance allele {v} (radius {}), on {n} standing ant(s)", v + 1.0);
    }
    if let Some(v) = drift {
        if let Some(id) = lab.world.species.id_of("ant") {
            let mut def = lab.world.species.get(id).creature.as_ref().expect("creature").clone();
            def.scent_drift = v;
            lab.world.species.set_creature(id, def);
        }
        println!("rivalry: ant scent_drift = {v}");
    }
    // **Link 2.** The ant authors no `sight_range`, so every sight-fed input
    // reads a constant 0. This is the only knob here that changes what the
    // brain can *see* rather than what it may do about it.
    if let Some(v) = sight {
        let n = set_allele_on(&mut lab, "ant", organism::TRAIT_SIGHT_RANGE, v);
        // **The resolved reach, not the allele.** `sight_range_of` clamps the
        // allele to +-1 and scales it by `SIGHT_SPAN` (64), so every
        // `sight=` at or above 1 is the same eye and the number passed on
        // the command line says nothing about how far the animal can see.
        // A knob nobody can see the value of is a knob nobody can tell is
        // disconnected.
        let reach = lab
            .world
            .species
            .id_of("ant")
            .and_then(|id| lab.world.species.get(id).creature.as_ref().map(|d| creature::sight_range_of(d, &d.traits)))
            .unwrap_or(0);
        println!("rivalry: ant sight_range allele {v} -> a reach of {reach} cells, on {n} standing ant(s)");
    }
    // **Link 3.** `wire=Input:Output:w` writes the input-to-output block of
    // the genome. `ant.ron` wires `(Alarm, Attack, 2.0)` and nothing else
    // reaches `Attack`, so `wire=ThreatNear:Attack:2.0` is the initiation
    // this bed has never had.
    if let Some(spec) = wire.as_deref() {
        for entry in spec.split(',') {
            let bits: Vec<&str> = entry.split(':').collect();
            if bits.len() != 3 {
                eprintln!("wire= wants Input:Output:weight, e.g. wire=ThreatNear:Attack:2.0");
                std::process::exit(2);
            }
            let Ok(weight) = bits[2].parse::<f32>() else {
                eprintln!("wire= weight '{}' does not parse as a number", bits[2]);
                std::process::exit(2);
            };
            let Some(i) = pixel_physics::sim::brain::INPUT_NAMES.iter().position(|n| n.eq_ignore_ascii_case(bits[0])) else {
                eprintln!("wire= input '{}' is not one of brain::INPUT_NAMES", bits[0]);
                std::process::exit(2);
            };
            let Some(o) = pixel_physics::sim::brain::OUTPUT_NAMES.iter().position(|n| n.eq_ignore_ascii_case(bits[1])) else {
                eprintln!("wire= output '{}' is not one of brain::OUTPUT_NAMES", bits[1]);
                std::process::exit(2);
            };
            let slot = pixel_physics::sim::brain::io_slot(pixel_physics::sim::brain::INPUTS[i], pixel_physics::sim::brain::OUTPUTS[o]);
            let n = set_genome_slot_on(&mut lab, "ant", slot, weight);
            println!("rivalry: wire {}:{} = {weight} on {n} standing ant(s)", bits[0], bits[1]);
        }
    }

    // **The contact census is sampled, not swept every frame.** It is
    // O(animals^2) over whole chains and the answer it gives is a rate; a
    // sample every `sample=` frames over the whole run is the same rate for
    // a hundredth of the cost. Accumulated rather than read at the end,
    // because a meeting that happened at frame 900 and killed somebody is
    // invisible in a census at frame 24,000 — the standing-state trap
    // `CLAUDE.md` names from the other direction.
    let sample: u64 = arg("sample").unwrap_or(300);
    let mut contact_ticks = 0u64;
    let mut cross_ticks = 0u64;
    let mut samples = 0u64;
    let mut peak_alive = 0usize;
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    // **Attacks at the moment the capture opens**, so the count the card
    // carries is the count for the window it shows and not for the whole
    // run. The review skill's house rule is the discrete event count in the
    // card's `meta`; a cumulative figure beside a 50-second window is a
    // different number than the one the picture is of.
    let mut attacks_at_capture_start = 0u64;

    for f in 0..=frames {
        lab.world.regroup_by_scent();
        if f.is_multiple_of(sample) {
            let (all, cross) = contacts(&lab.world);
            contact_ticks += all;
            cross_ticks += cross;
            samples += 1;
            peak_alive = peak_alive.max(standing(&lab.world).len());
        }
        if f.is_multiple_of(6_000) || f == frames {
            let st = lab.world.creature_stats;
            let (n, between, within, gap) = stranger_share(&lab.world);
            println!(
                "  f={f:>7} alive {n:>4} strangers between {between:>6.2}% within {within:>6.2}% gap {gap:>5.3} | attacks {} cells {} kills {} | births {} deaths {}",
                st.attacks,
                st.attack_cells,
                st.attack_kills,
                lab.world.creature_stats.births,
                lab.world.deaths_by_cause.iter().sum::<u64>(),
            );
        }
        if gif.is_some() || png_dir.is_some() {
            if f == gif_start {
                attacks_at_capture_start = lab.world.creature_stats.attacks;
            }
            if f >= gif_start && (f - gif_start).is_multiple_of(gif_every) && shots.len() < gif_shots {
                let mut buf = vec![0u8; (dims.0 * dims.1 * 4) as usize];
                lab.draw(&mut buf, 60.0);
                let full = image::RgbaImage::from_raw(dims.0 as u32, dims.1 as u32, buf);
                let cropped = full.map(|img| match crop {
                    None => img,
                    Some((cx, cy, cw, ch)) => {
                        let cx = cx.min(img.width().saturating_sub(1));
                        let cy = cy.min(img.height().saturating_sub(1));
                        let cw = cw.min(img.width() - cx).max(1);
                        let ch = ch.min(img.height() - cy).max(1);
                        image::imageops::crop_imm(&img, cx, cy, cw, ch).to_image()
                    }
                });
                if let Some(img) = cropped {
                    if let Some(dir) = png_dir.as_deref() {
                        let _ = std::fs::create_dir_all(dir);
                        let path = std::path::Path::new(dir).join(format!("frame_{:04}_f{f}.png", shots.len()));
                        let saved = if zoom == 1 {
                            img.save(&path)
                        } else {
                            image::imageops::resize(&img, img.width() * zoom, img.height() * zoom, image::imageops::FilterType::Nearest)
                                .save(&path)
                        };
                        if let Err(e) = saved {
                            eprintln!("rivalry: pngdir frame {}: {e}", path.display());
                        }
                    }
                    shots.push(img);
                }
            }
        }
        if f < frames {
            tick(&mut lab);
        }
    }

    // **How many animals were actually inside the crop, and where they all
    // were.** `instruments.md`'s standing lesson, from `flora_census
    // where=`: a whole-world count in a card's `meta` cannot say whether the
    // thing is even in frame, and a card of an empty band looks exactly like
    // a card of a mechanism that did not fire. Read at the end of the
    // capture window, over the crop this run used.
    if gif.is_some() || png_dir.is_some() {
        let live = standing(&lab.world);
        let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        let mut inside = 0usize;
        for (_, x, y, _, _) in &live {
            x0 = x0.min(*x);
            x1 = x1.max(*x);
            y0 = y0.min(*y);
            y1 = y1.max(*y);
            if let Some((cx, cy, cw, ch)) = crop {
                if *x >= cx as i32 && *x < (cx + cw) as i32 && *y >= cy as i32 && *y < (cy + ch) as i32 {
                    inside += 1;
                }
            } else {
                inside += 1;
            }
        }
        if live.is_empty() {
            println!("rivalry: IN FRAME -- nothing alive to be in it");
        } else {
            println!(
                "rivalry: IN FRAME {inside} of {} living animal(s){} | they span x {x0}..{x1}, y {y0}..{y1}",
                live.len(),
                if inside == 0 { "  <-- THE CARD IS OF AN EMPTY BAND" } else { "" },
            );
        }
    }
    if let Some(dir) = png_dir.as_deref() {
        // **The shot's own size, never one recomputed from the knobs.**
        // `labgif` has a note recording a log that read `2560x1600` over an
        // image that was genuinely `512x320`, because the line multiplied
        // the dials instead of reading the picture.
        let (sw, sh) = shots.first().map_or((0, 0), |i| (i.width() * zoom, i.height() * zoom));
        println!("rivalry: wrote {} frame(s) to {dir} at zoom {zoom} ({sw}x{sh} each)", shots.len());
    }
    if let Some(path) = gif.as_deref() {
        let st = lab.world.creature_stats;
        let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(gif_delay.max(1)));
        let n = shots.len();
        let frames_out: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
        match std::fs::File::create(path) {
            Ok(file) => {
                let mut enc = image::codecs::gif::GifEncoder::new(file);
                if let Err(e) = enc.set_repeat(image::codecs::gif::Repeat::Infinite) {
                    eprintln!("rivalry: gif set_repeat failed: {e}");
                }
                if let Err(e) = enc.encode_frames(frames_out) {
                    eprintln!("rivalry: gif encode failed: {e}");
                }
                println!(
                    "rivalry: wrote {path} ({n} frames from f={gif_start} every {gif_every}, {gif_delay} ms each -> {:.1} s) \
                     | attacks IN THIS WINDOW {} (run total {})",
                    (n as f64) * (gif_delay as f64) / 1000.0,
                    st.attacks.saturating_sub(attacks_at_capture_start),
                    st.attacks,
                );
            }
            Err(e) => eprintln!("rivalry: failed to create {path}: {e}"),
        }
    }

    summary(&lab.world, &label, contact_ticks, cross_ticks, samples, peak_alive, dims);
}

fn summary(world: &World, label: &str, contact_ticks: u64, cross_ticks: u64, samples: u64, peak_alive: usize, dims: (i32, i32)) {
    let st = world.creature_stats;
    let (alive, between, within, gap) = stranger_share(world);
    // **Split by whether the victim was an animal at all, and that split is
    // not a nicety.** `nearest_foe` skips kin and checks nothing else, so a
    // PLANT cell -- an organism, never this ant's kin, standing in the ring
    // -- is a valid foe. `World::tally_kill`'s own doc asserts "plants are
    // never victims here"; this column is that assertion measured rather
    // than assumed, and on the played bed it is the whole of the count.
    let mut xcol = 0u64;
    let mut own = 0u64;
    let mut plantkill = 0u64;
    for k in &world.kills_log {
        if world.species.get(k.victim_species).creature.is_none() {
            plantkill += 1;
        } else if k.victim_species == k.attacker_species && k.victim_colony == k.attacker_colony {
            own += 1;
        } else {
            xcol += 1;
        }
    }
    let deaths: u64 = world.deaths_by_cause.iter().sum();
    let starved = world.deaths_by_cause[organism::DeathCause::Starved.index()];
    let killed = world.deaths_by_cause[organism::DeathCause::Killed.index()];
    // **The same census over ANIMALS ONLY, and the split is the whole reason
    // these columns exist.** `World::deaths_by_cause` is every organism in
    // the bed, and on a played lab box the bed is mostly plants: seed 1 of
    // the shipped arm reports 656 deaths of which **415 are `felled`**, over
    // a population that never held more than 94 animals. So `starv%` --
    // "starved as a share of all deaths" -- has a denominator that is three
    // parts vegetation, and anything that fells more plants lowers it
    // without one fewer ant starving.
    //
    // That is `CLAUDE.md`'s worst-recurring failure in its *share* costume,
    // and it reached a shipped finding: round 35 priced the rivalry switch
    // at "starvation share -4.3 points, because killing displaces starving".
    // The share is real; the mechanism read off it is not necessarily.
    //
    // **`World::group_deaths` is the animal-only tally and it already
    // exists** -- `World::kill_organism` books a row there only `if
    // creature`, so summing it is the same census restricted to animals,
    // taken from the engine's own booking rather than re-derived here.
    let mut a_by_cause = [0u64; organism::DEATH_CAUSES];
    for g in &world.group_deaths {
        for (i, n) in g.by_cause.iter().enumerate() {
            a_by_cause[i] += n;
        }
    }
    let a_deaths: u64 = a_by_cause.iter().sum();
    let a_starved = a_by_cause[organism::DeathCause::Starved.index()];
    let a_killed = a_by_cause[organism::DeathCause::Killed.index()];
    // **Predation income, in joules, from the engine's own books** --
    // `ColonyBooks::raided`, "joules of living flesh this colony has
    // swallowed off another colony's animals" (#419). This is the number the
    // birth-bar question needs and the reason it could not be asked before:
    // `EnergyLedger::harvested_plant` books a mouthful of rival and a
    // mouthful of leaf into one account, so predation income was invisible
    // in every ledger column this harness printed. Face value, per its own
    // doc -- what came off the victim, not what the eater digested.
    //
    // Over `all_colony_books`, not over the live groups: a colony that was
    // eaten to extinction has no live group and is exactly the colony whose
    // books this question is about.
    let raided: f64 = world.all_colony_books().iter().map(|b| b.raided).sum();
    let raided_by_others: f64 = world.all_colony_books().iter().map(|b| b.raided_by_others).sum();
    let causes: Vec<String> = DEATH_CAUSE_LIST
        .iter()
        .enumerate()
        .filter(|(i, _)| world.deaths_by_cause[*i] > 0)
        .map(|(i, c)| format!("{}={}", c.label().to_lowercase().replace(' ', "_"), world.deaths_by_cause[i]))
        .collect();
    let l = &world.energy_ledger;
    println!(
        "SUMMARY label={label} alive={alive} peak={peak_alive} between={between:.2}% within={within:.2}% gap={gap:.3} \
         contacts={contact_ticks} cross={cross_ticks} samples={samples} \
         attacks={} cells={} kills={} xcol={xcol} own={own} plantkill={plantkill} unlogged={} \
         births={} deaths={deaths} starved={starved} killed={killed} starv%={:.1} \
         corpse_j={:.0} plant_j={:.0} corpse_cells={} \
         deathsA={a_deaths} starvedA={a_starved} killedA={a_killed} starvA%={:.1} raided_j={raided:.0} raided_by_others_j={raided_by_others:.0}",
        st.attacks,
        st.attack_cells,
        st.attack_kills,
        world.kills_unlogged,
        st.births,
        if deaths > 0 { 100.0 * starved as f64 / deaths as f64 } else { 0.0 },
        l.harvested_corpse,
        l.harvested_plant,
        corpse_cells(world, dims.0, dims.1),
        if a_deaths > 0 { 100.0 * a_starved as f64 / a_deaths as f64 } else { 0.0 },
    );
    println!("SUMMARY-causes label={label} {}", causes.join(" "));
    // The animal-only causes beside the whole-bed ones, so the two are
    // readable against each other in one log rather than one replacing the
    // other -- the whole-bed row is still the right answer to "what died in
    // this box", it is just not the right answer to "did rivalry change how
    // ants die".
    let a_causes: Vec<String> = DEATH_CAUSE_LIST
        .iter()
        .enumerate()
        .filter(|(i, _)| a_by_cause[*i] > 0)
        .map(|(i, c)| format!("{}={}", c.label().to_lowercase().replace(' ', "_"), a_by_cause[i]))
        .collect();
    println!("SUMMARY-causes-animal label={label} {}", a_causes.join(" "));
}

/// **The controls, in one short run — and two of them started life as wrong
/// predictions, which is the reason they are here.**
///
/// `CLAUDE.md`'s worst-recurring failure is a number that is arithmetically
/// correct and about the wrong thing, and its remedy is a case whose answer
/// is known in both directions. Written for this harness that gave four
/// arms, of which **the first run falsified two**:
///
/// - `strangers-only` was predicted to report `attacks 0` because nothing
///   wires initiation. It reported **13**. The mouth is an initiator: a
///   stranger is not excluded by `adjacent_food`'s kin filter, so an ant
///   *eats* it, the swallow calls `cry_alarm`, and `ant.ron`'s shipped
///   `(Alarm, Attack, 2.0)` does the rest. **Predation is the ignition the
///   fight verb was said to lack.**
/// - `strangers+wire` was predicted to report `cross > 0` and reported
///   **0**, at 13 attacks' worth of fighting. A cross-contact is consumed
///   by the fight that follows it, so a sampled census of a standing state
///   is the wrong instrument for it — `CLAUDE.md`'s standing-versus-event
///   trap, met from the standing side. `cross` is printed and not asserted.
///
/// What is asserted is what the code guarantees, in both directions:
///
/// - **specificity** — at the shipped dials two colonies stand in each
///   other's ring (`contacts > 0`, so the census is not blind) and read
///   `cross 0`, `attacks 0`. A non-zero `cross` here would mean the
///   stranger predicate is broken and every reading downstream is artifact.
/// - **sensitivity** — pull the two colonies' scent apart and `attacks > 0`
///   with **no wire added**, of which `xcol > 0` is the half that says
///   *colonies* fought rather than one colony biting its own children.
/// - **the isolation result, as a control** — the wire alone, on a bed where
///   everyone is kin, reads `attacks 0`. `nearest_foe` skips kin, so an ant
///   with `Bias -> Attack` at 4.0 swinging every tick has no target. This is
///   link 3 measured against link 1 inside one binary.
///
/// **`scent_drift` is pinned to 0 in every arm**, and that is not tidiness.
/// `ant.ron` ships `scent_drift: 0.15` and `tolerance: -1` is a radius of
/// **zero — an exact match only** — so a newborn is a stranger to its own
/// mother the instant it is placed. `creature.rs`'s own
/// `attacking_costs_the_jaw_and_yields_no_food` records measuring 11 attacks
/// between animals it had never made strangers for exactly this reason. Any
/// arm that leaves drift running is measuring speciation-within-a-colony and
/// calling it rivalry — which is a live confound in `labstats`' `rivalry=1`
/// alias, since that sets `tolerance=-1 spread=1` and says nothing about
/// drift.
fn selftest() {
    println!("rivalry: control=selftest  (scent_drift pinned 0 in every arm)");
    let mut fails = 0;

    for (name, apart, wired) in [
        ("shipped", Apart::No, false),
        ("two-colonies", Apart::Spread, false),
        ("every-ant-alone", Apart::Rivalry, false),
        ("two-col+wire", Apart::Spread, true),
        ("wire-only", Apart::No, true),
    ] {
        let r = arena(apart, wired);
        // What each arm must show for the instrument to be worth reading.
        let mut want: Vec<(&str, bool)> = vec![("contacts>0", r.all > 0)];
        match (apart, wired) {
            (Apart::No, false) => want.extend([("cross==0", r.cross == 0), ("attacks==0", r.attacks == 0)]),
            (Apart::No, true) => want.push(("attacks==0", r.attacks == 0)),
            // **`xcol`, not `attacks`, and this assertion started as the
            // other one.** The separated arm reported `xcol 1` with
            // `attacks 0`: the kill went through the *mouth* (a stranger is
            // food) and no `Alarm`-driven swing followed it in a box this
            // small. Asserting `attacks > 0` would have failed a control
            // that was working, on the strength of a counter this file's
            // own header says is not a fighting counter. The claim the
            // harness makes is "colonies killed each other", and `xcol` is
            // that claim.
            (_, _) => want.push(("xcol>0", r.xcol > 0)),
        }
        let bad: Vec<&str> = want.iter().filter(|(_, ok)| !ok).map(|(n, _)| *n).collect();
        if !bad.is_empty() {
            fails += 1;
        }
        println!(
            "  {name:<16} seed {} gap {:>5.3} between {:>6.2}% | contacts {:>4} cross {:>3} | attacks {:>4} cells {:>4} kills {:>3} xcol {:>3} own {:>3} -- {}",
            r.seed,
            r.gap,
            r.between,
            r.all,
            r.cross,
            r.attacks,
            r.cells,
            r.kills,
            r.xcol,
            r.own,
            if bad.is_empty() { "ok".to_string() } else { format!("FAIL ({})", bad.join(" ")) },
        );
    }
    println!("rivalry: selftest {}", if fails == 0 { "PASSED" } else { "FAILED" });
    if fails > 0 {
        std::process::exit(1);
    }
}

/// How far apart a control arm's two colonies are made.
///
/// **`Spread` and `Rivalry` are not the same experiment, and conflating
/// them is the trap this enum exists to make unmissable.** `Rivalry` is
/// `labstats`' `rivalry=1` alias — `spread=1` *and* `tolerance=-1`, a
/// radius of **zero**, an exact match only. That does not make two colonies
/// into rivals; it makes **every ant a stranger to every other ant**,
/// which is precisely what `CreatureDef::scent_spread`'s own doc says it
/// does and is not what the phrase "colony rivalry" leads a reader to
/// expect. `Spread` leaves the ancestral tolerance alone (slot 13 is `0.0`
/// in `ant.ron`, a radius of 1.0) so a colony stays family to itself and
/// only the gap *between* foundings has to clear the radius.
///
/// Whether it does is **a draw, not a setting**: the offsets are uniform in
/// `-1..=1` per slot, so the expected gap between two colonies is about
/// 1.41 against a radius of 1.0 and a given seed can land either side of
/// it. `strangers%` is the readout that says which happened, and it is why
/// no arm here asserts from the dial alone.
#[derive(Clone, Copy, PartialEq)]
enum Apart {
    No,
    Spread,
    Rivalry,
}

/// What one control arm reports.
struct Arm {
    all: u64,
    cross: u64,
    attacks: u64,
    cells: u64,
    kills: u64,
    xcol: u64,
    own: u64,
    /// Which seed's draw the arm ran on — see [`arena`].
    seed: u64,
    /// Mean scent distance between the two colonies at founding, against a
    /// tolerance radius of 1.0. Printed so an arm's *state* is visible and
    /// not only its outcome.
    gap: f64,
    between: f64,
}

fn arena(apart: Apart, wired: bool) -> Arm {
    // Seeds tried in order; the first that separates is used. Bounded, and
    // a failure to find one is loud rather than a quiet zero.
    for seed in 1..=12u64 {
        if let Some(arm) = arena_at(apart, wired, seed) {
            return arm;
        }
    }
    panic!("no seed in 1..=12 gave a founding gap over the tolerance radius -- the offset draw or the tolerance moved");
}

/// One attempt at [`arena`]. `None` when the arm needed a separated pair and
/// this seed's draw did not give one.
fn arena_at(apart: Apart, wired: bool, seed: u64) -> Option<Arm> {
    // **The default geometry, narrowed — not a box of my own invention.**
    // The first version of this control was `128 x 96` with the default
    // `ground_y: 160`, i.e. a floor below the bottom of the world, and every
    // arm read `contacts 0` because nothing was alive. `CLAUDE.md`: a scene
    // that contradicts the code looks exactly like a bug in the code. Width
    // 160 with the bed's own height, soil and ground line puts two colonies
    // close enough to meet inside the window and changes nothing else.
    let spec = LabBox { width: 160, founders: 0, colonies: 2, compartments: 0, seed, ..LabBox::default() };
    let mut lab = Lab::new(spec);
    // Pinned before anything else so the founders already standing carry it
    // too -- see this function's caller for why it is not optional.
    if let Some(id) = lab.world.species.id_of("ant") {
        let mut def = lab.world.species.get(id).creature.as_ref().expect("creature").clone();
        def.scent_drift = 0.0;
        lab.world.species.set_creature(id, def);
    }
    // **EVERY arm pins `scent_spread` explicitly and re-derives each standing
    // ant's signature from the species' ANCESTRAL point.** Two repairs, and
    // both are #423's live default arriving inside a control written before
    // it existed -- `CLAUDE.md`'s "adding a member to a set enrols it in
    // every rule over that set", with the set being "every arm that did not
    // name its own spread".
    //
    // **1. `Apart::No` set nothing at all**, so it inherited whatever
    // `ant.ron` authored. That was 0 until #423 and the arm genuinely meant
    // "everyone is kin"; from #423 it silently meant "the shipped stranger
    // bed", and the two claims resting on it -- `shipped` reads `cross 0,
    // attacks 0`, and `wire-only` shows a swinging ant with no target --
    // have been failing ever since, on `main`, gated by nothing (CI does not
    // run `control=selftest`). Measured 2026-09-14 on unmodified `origin/
    // main`: byte-identical failures to this branch, which is what says the
    // defect is the default's and not this lane's.
    //
    // **2. The separated arms ADDED their offset to the scent an animal
    // already carried**, which is precisely the trap round 35 recorded and
    // repaired in the `spread=` path above -- and missed here, because this
    // function has its own copy. At a live default `Apart::Spread` was
    // measuring `authored 2.0 + requested 1.0`; the tell is in the log, where
    // it reported a founding gap of **2.170** against the 1.62 median round
    // 35 measured for a true `spread=1`.
    //
    // Resetting to the ancestral point first makes `requested` mean "founded
    // at this spread" for any authored default, 0 included -- so the kin arm
    // is kin again by construction rather than by luck.
    let requested = if apart == Apart::No { 0.0 } else { 1.0 };
    let ancestral = lab
        .world
        .species
        .id_of("ant")
        .and_then(|id| lab.world.species.get(id).creature.as_ref().map(|d| d.traits))
        .unwrap_or([0.0; organism::CREATURE_TRAITS]);
    if let Some(id) = lab.world.species.id_of("ant") {
        let mut def = lab.world.species.get(id).creature.as_ref().expect("creature").clone();
        def.scent_spread = requested;
        lab.world.species.set_creature(id, def);
    }
    let world_seed = lab.world.seed;
    let pairs: Vec<(OrganismId, u32)> =
        ants_of(&lab, "ant").into_iter().filter_map(|id| lab.world.organism(id).map(|st| (id, st.colony))).collect();
    for (id, col) in pairs {
        let off = creature::colony_scent_offset(world_seed, col, requested);
        for (i, slot) in organism::SCENT_SLOTS.iter().enumerate() {
            lab.world.set_organism_trait(id, *slot, (ancestral[*slot] + off[i]).clamp(-1.0, 1.0));
        }
    }
    if apart == Apart::Rivalry {
        set_allele_on(&mut lab, "ant", organism::TRAIT_TOLERANCE, -1.0);
    }
    if wired {
        let slot = pixel_physics::sim::brain::io_slot(pixel_physics::sim::brain::BrainInput::Bias, pixel_physics::sim::brain::BrainOutput::Attack);
        set_genome_slot_on(&mut lab, "ant", slot, 4.0);
    }
    // Walk the two colonies into one another: the box is small and the
    // compartment wall is off, so they meet on their own within a few
    // thousand frames. Censused every 60 rather than at the end, for the
    // reason the main run samples: a meeting is an event, not a state.
    // **The gate, read before a single frame is ticked.** `Apart::Spread`
    // leaves the ancestral tolerance alone, so the arm only means anything
    // if the draw put the two colonies outside it; `Apart::Rivalry` narrows
    // the radius to zero and separates by construction.
    let (_, between0, _, gap0) = stranger_share(&lab.world);
    if apart == Apart::Spread && between0 <= 0.0 {
        return None;
    }
    let (mut all, mut cross) = (0u64, 0u64);
    let alive0 = standing(&lab.world).len();
    for f in 0..6_000u64 {
        if f.is_multiple_of(10) {
            let (a, c) = contacts(&lab.world);
            all += a;
            cross += c;
        }
        tick(&mut lab);
    }
    assert!(alive0 > 0, "the control box founded no animals -- it is measuring its own geometry, not the verb");
    let st = lab.world.creature_stats;
    let (mut xcol, mut own) = (0u64, 0u64);
    for k in &lab.world.kills_log {
        if k.victim_species == k.attacker_species && k.victim_colony == k.attacker_colony {
            own += 1;
        } else {
            xcol += 1;
        }
    }
    Some(Arm { all, cross, attacks: st.attacks, cells: st.attack_cells, kills: st.attack_kills, xcol, own, seed, gap: gap0, between: between0 })
}

