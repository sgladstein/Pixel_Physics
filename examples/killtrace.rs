//! **Who killed whom inside one colony, and why the kin test let them.**
//!
//! The owner's 2026-10-03 playtest chronicle booked colony ANT 4 losing 14 of
//! its 33 dead to "KILLED BY ANT 4", ANT 1 39 to itself and ANT 9 5. A colony
//! label is a census grouping -- `World::regroup_by_scent`, run by the lab
//! once per tick -- while a bite is decided pairwise by
//! `creature::is_living_kin`. This harness runs the real `Lab` (so the
//! regroup runs exactly as in the game, which `labforage` does not do) and,
//! for every kill booked to the attacker's own label, reads the pair back off
//! `World::kills_log`'s `KillDetail` and the label as it stood at the start
//! of that tick:
//!
//! - `d` -- scent distance at the bite, against both radii (`ra`, `rv`);
//! - `chain` -- hops between the two in the label's mutual-kin graph, the
//!   graph `regroup_by_scent` unions over. `1` cannot happen for a kill (the
//!   pair would be kin); `2+` means the label held them together through
//!   go-betweens; `-` means they were in different clusters, and `vsz` (the
//!   victim's cluster size) says why that cluster was never minted;
//! - `nest` -- each party's scent distance from the nearest seeded nest
//!   site's odour;
//! - `split` -- frames until the label next split, or `-`.
//!
//! ```text
//! cargo run --release --example killtrace -- frames=120000 seed=1                      # two colonies, default bed
//! cargo run --release --example killtrace -- scenario=played_bed frames=120000 seed=1
//! cargo run --release --example killtrace -- colonies=2 founders=8 rows=0                # summary only
//! ```
//!
//! **Read the category counts, not the rows.** `CHAINED` = the label's
//! single-linkage held two mutual strangers in one group; `STRAY` = the victim
//! sat in a cluster smaller than `MIN_SPLIT_GROUP` and so was never minted;
//! `UNSPLIT` = a cluster big enough to mint that had not been (the regroup
//! lagging); `KIN` = the attacker's own test called the victim kin (only an
//! `eats_kin` mouth or a scent that moved mid-tick can do that).

use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::Lab;
use pixel_physics::sim::cell::OrganismId;
use pixel_physics::sim::creature;
use pixel_physics::sim::world::{World, KILL_VERB_BITE, KILL_VERB_EAT, MIN_SPLIT_GROUP};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}="))
            .map(|v| v.parse().ok().expect("parses"))
    })
}

#[derive(Clone)]
struct Member {
    id: OrganismId,
    species: u16,
    colony: u32,
    scent: [f32; 3],
    radius: f32,
    head: (i32, i32),
    generation: u16,
}

fn snapshot(world: &World) -> Vec<Member> {
    let mut out = Vec::new();
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        if world.species.get(st.species).creature.is_none() {
            continue;
        }
        let t = creature::expressed_traits(st, world.plasticity, world.trait_reach);
        out.push(Member {
            id,
            species: st.species.0,
            colony: st.colony,
            scent: creature::scent_of(&t),
            radius: creature::tolerance_radius(&t),
            head: st.chain.first().copied().unwrap_or((0, 0)),
            generation: st.generation,
        });
    }
    out
}

fn dist(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    creature::scent_distance_sq(a, b).sqrt()
}

fn accepts(judge: &Member, other: &Member) -> bool {
    dist(&judge.scent, &other.scent) <= judge.radius
}

/// Hops between `a` and `b` over mutual-kin edges among `group`, and the size
/// of `b`'s component. `None` hops = different components.
fn chain(group: &[&Member], a: OrganismId, b: OrganismId) -> (Option<usize>, usize) {
    let n = group.len();
    let ia = group.iter().position(|m| m.id == a);
    let ib = group.iter().position(|m| m.id == b);
    let (Some(_ia), Some(ib)) = (ia, ib) else {
        return (None, 0);
    };
    // BFS from b: its component, and the distance to a.
    let mut depth = vec![usize::MAX; n];
    depth[ib] = 0;
    let mut queue = std::collections::VecDeque::from([ib]);
    let mut size = 0;
    while let Some(i) = queue.pop_front() {
        size += 1;
        for j in 0..n {
            if depth[j] == usize::MAX && accepts(group[i], group[j]) && accepts(group[j], group[i]) {
                depth[j] = depth[i] + 1;
                queue.push_back(j);
            }
        }
    }
    let hops = ia.and_then(|i| (depth[i] != usize::MAX).then_some(depth[i]));
    (hops, size)
}

/// Scent distance to the nearest seeded nest odour, and which site that is.
fn nest_gap(world: &World, scent: &[f32; 3]) -> (f32, usize) {
    world
        .nest_sites
        .iter()
        .enumerate()
        .filter(|(_, s)| s.seeded)
        .map(|(i, s)| (dist(&s.scent, scent), i))
        .fold((f32::INFINITY, usize::MAX), |a, b| if b.0 < a.0 { b } else { a })
}

struct OwnKill {
    frame: u64,
    label: u32,
    verb: u8,
    d: f32,
    ra: f32,
    rv: f32,
    hops: Option<usize>,
    vsz: usize,
    label_size: usize,
    nest_a: (f32, usize),
    nest_v: (f32, usize),
    /// The site nearest in space to the victim's head: where the kill was.
    at_site: usize,
    gen_a: u16,
    gen_v: u16,
    a_accepts: bool,
    v_accepts: bool,
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(120_000);
    let rows: usize = arg("rows").unwrap_or(60);
    let sample: u64 = arg("sample").unwrap_or(10_000);
    let scenario: Option<Scenario> = arg::<String>("scenario").map(|n| {
        let mut sc = Scenario::load(&n).unwrap_or_else(|e| {
            eprintln!("scenario {n}: {e}");
            std::process::exit(2);
        });
        if let Some(sd) = arg::<u64>("seed") {
            sc.bed.seed = sd;
        }
        sc
    });
    let spec = LabBox {
        founders: arg("founders").unwrap_or(8),
        colonies: arg("colonies").unwrap_or(2),
        seed: arg("seed").unwrap_or(1),
        ..LabBox::default()
    };
    let mut lab = Lab::new(spec.clone());
    if let Some(sc) = scenario.clone() {
        lab.load_scenario(sc);
    }
    println!(
        "killtrace: frames={frames} seed={} colonies={} founders={} scenario={} min_split_group={MIN_SPLIT_GROUP}",
        lab.spec.seed,
        lab.spec.colonies,
        lab.spec.founders,
        scenario.as_ref().map_or("-".to_string(), |s| s.name.clone())
    );

    let mut own: Vec<OwnKill> = Vec::new();
    let (mut cross, mut seen) = (0u64, 0usize);
    let mut splits: Vec<(u64, u32)> = Vec::new(); // (frame, parent label)
    let mut parents_seen = lab.world.colony_parents.len();
    for f in 0..frames {
        let before = snapshot(&lab.world);
        lab.tick_for_harness();
        for &(_, parent) in &lab.world.colony_parents[parents_seen..] {
            splits.push((lab.world.frame, parent));
        }
        parents_seen = lab.world.colony_parents.len();
        while seen < lab.world.kills_log.len() {
            let k = lab.world.kills_log[seen];
            seen += 1;
            if k.victim_species != k.attacker_species || k.victim_colony != k.attacker_colony {
                cross += 1;
                continue;
            }
            let label = k.attacker_colony;
            let group: Vec<&Member> = before
                .iter()
                .filter(|m| m.species == k.attacker_species.0 && m.colony == label)
                .collect();
            let a = group.iter().find(|m| m.id == k.detail.attacker).copied().cloned();
            let v = group.iter().find(|m| m.id == k.detail.victim).copied().cloned();
            let (hops, vsz) = chain(&group, k.detail.attacker, k.detail.victim);
            let (am, vm) = (
                a.clone().unwrap_or(Member {
                    id: k.detail.attacker,
                    species: 0,
                    colony: label,
                    scent: k.detail.attacker_scent,
                    radius: k.detail.attacker_radius,
                    head: (0, 0),
                    generation: 0,
                }),
                v.clone().unwrap_or(Member {
                    id: k.detail.victim,
                    species: 0,
                    colony: label,
                    scent: k.detail.victim_scent,
                    radius: k.detail.victim_radius,
                    head: (0, 0),
                    generation: 0,
                }),
            );
            own.push(OwnKill {
                frame: k.frame,
                label,
                verb: k.detail.verb,
                d: dist(&k.detail.attacker_scent, &k.detail.victim_scent),
                ra: k.detail.attacker_radius,
                rv: k.detail.victim_radius,
                hops,
                vsz,
                label_size: group.len(),
                nest_a: nest_gap(&lab.world, &k.detail.attacker_scent),
                nest_v: nest_gap(&lab.world, &k.detail.victim_scent),
                at_site: lab.world.nearest_nest_site(vm.head.0, vm.head.1).unwrap_or(usize::MAX),
                gen_a: am.generation,
                gen_v: vm.generation,
                a_accepts: dist(&k.detail.attacker_scent, &k.detail.victim_scent) <= k.detail.attacker_radius,
                v_accepts: dist(&k.detail.attacker_scent, &k.detail.victim_scent) <= k.detail.victim_radius,
            });
        }
        if (f + 1) % sample == 0 {
            // Within-label strangers: of all same-label pairs, the share that
            // are NOT mutual kin, and the widest pair.
            let snap = snapshot(&lab.world);
            let mut labels: Vec<(u16, u32)> = snap.iter().map(|m| (m.species, m.colony)).collect();
            labels.sort();
            labels.dedup();
            let mut parts = Vec::new();
            for (sp, col) in labels {
                let g: Vec<&Member> = snap.iter().filter(|m| m.species == sp && m.colony == col).collect();
                let (mut pairs, mut strangers, mut widest) = (0u64, 0u64, 0f32);
                for i in 0..g.len() {
                    for j in (i + 1)..g.len() {
                        pairs += 1;
                        let d = dist(&g[i].scent, &g[j].scent);
                        widest = widest.max(d);
                        if !(accepts(g[i], g[j]) && accepts(g[j], g[i])) {
                            strangers += 1;
                        }
                    }
                }
                parts.push(format!(
                    "{}:{}n {:.1}%strange wide{:.2}",
                    lab.world.group_label(pixel_physics::sim::organism::SpeciesId(sp), col),
                    g.len(),
                    100.0 * strangers as f64 / pairs.max(1) as f64,
                    widest
                ));
            }
            println!(
                "F {:>7} kills own={} cross={} splits={} blends={} refused={} | {}",
                lab.world.frame,
                own.len(),
                cross,
                splits.len(),
                lab.world.creature_stats.nest_blends,
                lab.world.creature_stats.nest_blends_refused,
                parts.join(" | ")
            );
        }
    }

    let label_name = |l: u32| {
        lab.world.group_label(
            lab.world
                .species
                .id_of("ant")
                .unwrap_or(pixel_physics::sim::organism::SpeciesId(0)),
            l,
        )
    };
    if rows > 0 {
        println!("\nOWN-LABEL KILLS (first {rows})");
        println!("  frame   label  verb  d     ra    rv    a_acc v_acc chain vsz lsz  nest_a   nest_v   at gen_a gen_v split_in");
        for k in own.iter().take(rows) {
            let split_in = splits
                .iter()
                .find(|(fr, p)| *p == k.label && *fr >= k.frame)
                .map_or("-".to_string(), |(fr, _)| format!("{}", fr - k.frame));
            println!(
                "  {:>7} {:>6} {:>5} {:.3} {:.3} {:.3} {:>5} {:>5} {:>5} {:>3} {:>4} {:.3}@{} {:.3}@{} {:>2} {:>5} {:>5} {}",
                k.frame,
                label_name(k.label),
                match k.verb {
                    KILL_VERB_BITE => "bite",
                    KILL_VERB_EAT => "eat",
                    _ => "?",
                },
                k.d,
                k.ra,
                k.rv,
                k.a_accepts,
                k.v_accepts,
                k.hops.map_or("-".to_string(), |h| h.to_string()),
                k.vsz,
                k.label_size,
                k.nest_a.0,
                k.nest_a.1,
                k.nest_v.0,
                k.nest_v.1,
                k.at_site,
                k.gen_a,
                k.gen_v,
                split_in
            );
        }
    }
    let kin = own.iter().filter(|k| k.a_accepts).count();
    let chained = own.iter().filter(|k| !k.a_accepts && k.hops.is_some()).count();
    let stray = own
        .iter()
        .filter(|k| !k.a_accepts && k.hops.is_none() && k.vsz < MIN_SPLIT_GROUP)
        .count();
    let unsplit = own
        .iter()
        .filter(|k| !k.a_accepts && k.hops.is_none() && k.vsz >= MIN_SPLIT_GROUP)
        .count();
    let bites = own.iter().filter(|k| k.verb == KILL_VERB_BITE).count();
    let eats = own.iter().filter(|k| k.verb == KILL_VERB_EAT).count();
    let asym = own.iter().filter(|k| k.v_accepts && !k.a_accepts).count();
    let mut hops: Vec<usize> = own.iter().filter_map(|k| k.hops).collect();
    hops.sort();
    let mut ds: Vec<f32> = own.iter().map(|k| k.d).collect();
    ds.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let med = |v: &[f32]| if v.is_empty() { f32::NAN } else { v[v.len() / 2] };
    println!(
        "\nSUMMARY seed={} frames={frames} gate={} kills={} own={} cross={} | CHAINED={chained} STRAY={stray} UNSPLIT={unsplit} KIN={kin} | bite={bites} eat={eats} victim_accepted_attacker={asym} | median_d={:.3} median_hops={} splits={} unlogged={}",
        lab.spec.seed,
        if creature::nest_kin_gate() { "on" } else { "off" },
        lab.world.kills_log.len(),
        own.len(),
        cross,
        med(&ds),
        hops.get(hops.len() / 2).map_or("-".to_string(), |h| h.to_string()),
        splits.len(),
        lab.world.kills_unlogged
    );
}
