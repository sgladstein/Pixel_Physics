//! **The textbook ant: a third way for an ant to decide, from cited rules
//! only.** Built beside today's ant and the needs walk, and off until a
//! harness sets `World::textbook`. The design of record is
//! `/mnt/project-files/emergence-sandbox/textbook-ant/design-textbook-ant-2026-10-15.md`
//! (rev 2, after an adversarial review beside it), and the owner's calls
//! were "Design doc first", then "Build, pillars first" (2026-10-15).
//!
//! **What it is for.** The textbook tests (Khuong pillars, the double
//! bridge, sorting) failed on both brained ants, and in the pillar box the
//! game ant's keep rules held 99.6% of its carriers before the drop roll was
//! reached (`/mnt/project-files/emergence-sandbox/pillars/README.md`). A
//! reference ant with only the textbook's rules, in the same engine and the
//! same body, says which failures are the engine's and which are our rules':
//! where it passes and the game ant fails, adding one game rule to it at a
//! time names the rule that blocks.
//!
//! **Slice A, soil (the pillar test), is all that is built.** Three rules,
//! each a named part with its parameter in [`TextbookCfg`]:
//! - **walk**: a correlated random walk over the body's usable headings,
//!   the same walk the level-1 walkers take (`examples/pillarbox`): keep the
//!   heading, turn one point either way with `turn`, and take the first
//!   usable heading nearest it. Modelling standard; not checked.
//! - **pick**: an empty ant beside the source material cuts a cell of it
//!   with `pick` ([`cut_cell`], gated by [`jaw_can_cut`] only). Built spoil
//!   is never taken (`rework=0` at level 1, where taking from a column's
//!   side brought the column down).
//! - **put down**: a laden ant considers the cells beside its head where a
//!   pellet would stand on ground ([`spoil_site_fits`] footed, not on an
//!   animal) and would not shut a way ([`closes_a_way`]), takes one at
//!   random, and puts the pellet there with `p0 + (1 - p0) m / (m + K)`,
//!   `m` the fading build mark in the 3x3 round it. Khuong et al. 2016,
//!   PNAS 113:1303 (*Lasius niger*). **Where** is engine-driven, not cited:
//!   a pellet with nothing but an ant beneath it turns loose and falls
//!   (`needs_footing`), so the choice is the ant's and the reason is the
//!   engine's -- the design's §3 A.
//!
//! **What the tick still does for it, and what it replaces** (the design's
//! §4 table). The brain still runs, as it does under the needs walk, so the
//! synapse tax is paid like for like and the speculation pre-pass is
//! untouched; its outputs are ignored except where [`outputs_for`] zeroes
//! them (trail emission, the launch, laying and provisioning: births wait,
//! and slice A lays no trail). `act`, the move roll and the walk are
//! replaced; the nest-scent blend, metabolism, digestion and death are not.
//! Falling is checked every decision, as under the chooser.
//!
//! **Its draws are its own** (`RNG_SLOT_TEXTBOOK`), so nothing it does
//! moves another ant's stream, and with `World::textbook` unset no line
//! here runs.
use super::*;

/// The textbook ant's own random stream; see [`super::RNG_SLOT_MOVE`].
const RNG_SLOT_TEXTBOOK: u64 = 17;

/// **Slice A's parameters**, each the authored constant a gene will later
/// scale (design §6). The defaults are level 1's (`examples/pillarbox`,
/// where walkers built free pillars on 8 of 8 seeds at these values with
/// `tau` 300), except `p_move`, which the walkers do not have: they step
/// every frame, and an ant steps on its decision ticks.
#[derive(Clone, Copy, Debug)]
pub struct TextbookCfg {
    /// The chance a decision is a step rather than a pause.
    pub p_move: f32,
    /// The chance a step first turns one point, either way at even odds.
    pub turn: f32,
    /// The chance an empty ant beside the source cuts a cell of it.
    pub pick: f32,
    /// Khuong's bare put-down chance per decision, at an unmarked site.
    pub p0: f32,
    /// Khuong's half-saturation of the mark.
    pub k: f32,
    /// The mark's lifetime in frames: 0 is no mark (the control),
    /// infinite a mark that never fades (the blanket control).
    pub tau: f32,
    /// The material an ant picks from.
    pub source: material::MaterialId,
}

/// What the rules did, for a harness's census. Nothing reads these back.
#[derive(Clone, Copy, Debug, Default)]
pub struct TextbookCounts {
    /// Decisions taken by a textbook ant.
    pub decisions: u64,
    /// Cells cut from the source.
    pub picks: u64,
    /// Laden decisions with no site beside the head.
    pub no_site: u64,
    /// Put-down rolls taken, and won.
    pub rolls: u64,
    pub drops: u64,
    /// The mark read at the rolled sites, summed, for its mean.
    pub mark_sum: f64,
    /// Steps taken, and decisions with no usable heading.
    pub steps: u64,
    pub stuck: u64,
}

/// **`World::textbook`**: which species decides by it, its parameters and
/// its counts. Set by a harness at a frame, never from the environment.
#[derive(Clone, Debug)]
pub struct Textbook {
    pub species: crate::sim::organism::SpeciesId,
    pub cfg: TextbookCfg,
    pub counts: TextbookCounts,
}

impl Textbook {
    pub fn new(species: crate::sim::organism::SpeciesId, cfg: TextbookCfg) -> Textbook {
        Textbook {
            species,
            cfg,
            counts: TextbookCounts::default(),
        }
    }
}

/// Whether `organism` decides by the textbook rules this tick.
pub(super) fn decides(world: &World, organism: OrganismId) -> bool {
    match (world.textbook.as_ref(), world.organism(organism)) {
        (Some(t), Some(s)) => s.species == t.species,
        _ => false,
    }
}

/// The brain's outputs as the rest of the tick will read them: the ones the
/// textbook ant replaces with nothing are zeroed. `EmitA`/`EmitB` (slice A
/// lays no trail), `Impulse` (it walks), and `Lay`/`Provision` (births are
/// the harness's to pause; a zero `Lay` holds them, `try_bud`).
pub(super) fn outputs_for(mut outputs: [f32; brain::BRAIN_OUTPUTS]) -> [f32; brain::BRAIN_OUTPUTS] {
    use brain::BrainOutput as O;
    for o in [O::EmitA, O::EmitB, O::Impulse, O::Lay, O::Provision] {
        outputs[o as usize] = 0.0;
    }
    outputs
}

/// The textbook ant's draw for this tick.
pub(super) fn draw_for(world: &World, organism: OrganismId) -> rng::Rng {
    rng::stream(world.seed, organism as u64, world.frame, RNG_SLOT_TEXTBOOK)
}

/// The chance this decision is a step.
pub(super) fn p_move(world: &World) -> f32 {
    world.textbook.as_ref().map_or(0.0, |t| t.cfg.p_move)
}

/// **The fresh mark round `p`**: the sum of `exp(-age / tau)` over the
/// marks in the 3x3 round it, counting only cells still spoil, as level 1
/// reads it. 0 at `tau` 0; each mark counts 1 at `tau` infinite.
pub(crate) fn mark_round(world: &World, tau: f32, (x, y): (i32, i32)) -> f32 {
    if tau <= 0.0 || world.build_marks.is_empty() {
        return 0.0;
    }
    let spoil = world.materials.id_of("spoil");
    let mut m = 0.0;
    for dy in -1..=1 {
        for dx in -1..=1 {
            let c = (x + dx, y + dy);
            let Some(&f) = world.build_marks.get(&c) else { continue };
            if Some(world.get(c.0, c.1).material) != spoil {
                continue;
            }
            m += if tau.is_infinite() {
                1.0
            } else {
                (-(world.frame.saturating_sub(f) as f32) / tau).exp()
            };
        }
    }
    m
}

/// **The verbs, in place of `act`**: pick from the source, or put down.
/// Returns what the tick charges for (a cut is a dig).
pub(super) fn act(world: &mut World, organism: OrganismId, def: &CreatureDef, draw: &mut rng::Rng) -> Did {
    let Some(cfg) = world.textbook.as_ref().map(|t| t.cfg) else {
        return Did::default();
    };
    let Some((head, laden)) = world.organism(organism).map(|s| (s.chain.first().copied(), s.spoil)) else {
        return Did::default();
    };
    let Some((x, y)) = head else { return Did::default() };
    count(world, |c| c.decisions += 1);
    match laden {
        None => {
            let src: Vec<(i32, i32)> = DIRS
                .iter()
                .map(|&(dx, dy)| (x + dx, y + dy))
                .filter(|&(px, py)| {
                    let c = world.get(px, py);
                    world.in_bounds(px, py)
                        && c.material == cfg.source
                        && c.organism_id() == 0
                        && jaw_can_cut(world, def, organism, c)
                })
                .collect();
            if src.is_empty() || draw.unit_f32() >= cfg.pick {
                return Did::default();
            }
            let p = src[draw.below(src.len() as u32) as usize];
            cut_cell(world, organism, p, true);
            count(world, |c| c.picks += 1);
            Did {
                dug: 1,
                ..Did::default()
            }
        }
        Some(load) => {
            let sites: Vec<(i32, i32)> = DIRS
                .iter()
                .map(|&(dx, dy)| (x + dx, y + dy))
                .filter(|&(px, py)| {
                    world.in_bounds(px, py) && spoil_site_fits(world, px, py, true) && !closes_a_way(world, (px, py))
                })
                .collect();
            if sites.is_empty() {
                count(world, |c| c.no_site += 1);
                return Did::default();
            }
            let p = sites[draw.below(sites.len() as u32) as usize];
            let m = mark_round(world, cfg.tau, p);
            let chance = cfg.p0 + (1.0 - cfg.p0) * m / (m + cfg.k);
            let won = draw.unit_f32() < chance;
            count(world, |c| {
                c.rolls += 1;
                c.mark_sum += f64::from(m);
                c.drops += u64::from(won);
            });
            if won {
                place_pellet(world, organism, p, load.cell);
                world.creature_stats.spoil_dumped += 1;
            }
            Did::default()
        }
    }
}

/// **The walk, in place of the move**: level 1's walker on the ant's body.
/// Turn one point with `turn`, then take the first usable heading in the
/// order straight, one point either way, two, three, and back.
pub(super) fn step(
    world: &mut World,
    organism: OrganismId,
    heading: u8,
    def: &CreatureDef,
    draw: &mut rng::Rng,
) -> bool {
    let turn = world.textbook.as_ref().map_or(0.0, |t| t.cfg.turn);
    let Some((chain, groups, fates)) = world
        .organism(organism)
        .map(|s| (s.chain.clone(), s.segment_groups.clone(), s.fates))
    else {
        note_outcome(world, DecisionOutcome::NoBody);
        return false;
    };
    if chain.is_empty() {
        note_outcome(world, DecisionOutcome::NoBody);
        return false;
    }
    let usable = usable_headings(world, organism, def);
    let want = if draw.unit_f32() < turn {
        (heading + if draw.flip() { 1 } else { 7 }) % 8
    } else {
        heading
    };
    let Some(to) = [0u8, 1, 7, 2, 6, 3, 5, 4]
        .iter()
        .map(|&o| (want + o) % 8)
        .find(|d| usable.contains(d))
    else {
        count(world, |c| c.stuck += 1);
        note_outcome(world, DecisionOutcome::RollFailedIdle);
        return false;
    };
    let authored = segment_authored(def, fates);
    let authored_widths: Vec<u8> = authored
        .iter()
        .map(|s| if s.lateral.is_some() { 2 } else { 1 })
        .collect();
    let body = BodyShape {
        chain: &chain,
        groups: &groups,
        authored: &authored_widths,
    };
    commit_step(
        world,
        organism,
        def,
        body,
        &authored,
        &authored_widths,
        heading,
        to,
        parting_enabled(),
    );
    count(world, |c| c.steps += 1);
    true
}

fn count(world: &mut World, f: impl FnOnce(&mut TextbookCounts)) {
    if let Some(t) = world.textbook.as_mut() {
        f(&mut t.counts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::Rect;

    /// A stone box with a soil floor and `n` textbook ants on it, as
    /// `examples/pillarbox` builds it at a tenth the size.
    fn bed(n: usize, tau: f32) -> World {
        let (w, h) = (60, 40);
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
        let stone = world.materials.id_of("stone").expect("stone");
        let soil = world.materials.id_of("soil").expect("soil");
        for y in 0..h {
            for x in 0..w {
                if x < 2 || y < 2 || x >= w - 2 || y >= h - 2 {
                    world.set(x, y, Cell::new(stone, 0));
                } else if y >= h - 10 {
                    world.set(x, y, Cell::new(soil, 0));
                }
            }
        }
        for _ in 0..100 {
            crate::sim::update::step(&mut world);
        }
        world.births_paused = true;
        let id = world.species.id_of("ant").expect("ant ships");
        for i in 0..n {
            let x = 6 + (i as i32 * 47) % (w - 12);
            if let Some(site) = plant_creature_seed(&mut world, x, h - 11, "ant") {
                world.schedule_active_site(site);
            }
        }
        let cfg = TextbookCfg {
            p_move: 1.0,
            turn: 0.125,
            pick: 0.05,
            p0: 0.002,
            k: 1.0,
            tau,
            source: soil,
        };
        world.textbook = Some(Box::new(Textbook::new(id, cfg)));
        world
    }

    fn run(world: &mut World, frames: u64) {
        for _ in 0..frames {
            crate::sim::update::step(world);
            world.step_active_sites();
        }
    }

    /// **The three verbs fire on the game's body**: the ants step, cut the
    /// source floor and put pellets down where they stand (`spoil` left in
    /// the world). The control for "it did nothing":
    /// every count starts at zero.
    #[test]
    fn textbook_ants_walk_pick_and_put_down() {
        let mut w = bed(6, 300.0);
        assert_eq!(w.textbook.as_ref().expect("set").counts.decisions, 0);
        run(&mut w, 6_000);
        let c = w.textbook.as_ref().expect("set").counts;
        assert!(c.decisions > 1_000 && c.steps > 500, "the ants did not walk: {c:?}");
        assert!(c.picks > 0 && c.drops > 0, "no pick or no put-down: {c:?}");
        let spoil = w.materials.id_of("spoil");
        let (wd, ht) = (60, 40);
        let standing = (0..ht)
            .flat_map(|y| (0..wd).map(move |x| (x, y)))
            .filter(|&(x, y)| Some(w.get(x, y).material) == spoil)
            .count();
        assert!(standing > 0, "nothing put down stands: {c:?}");
    }

    /// **The mark reads only standing spoil, and fades**: a mark on a cell
    /// that is no longer spoil counts nothing; one `tau` old counts e^-1;
    /// `tau` 0 is no mark and infinite counts 1.
    #[test]
    fn mark_round_counts_standing_spoil_and_fades() {
        let mut w = World::new(Rect::new(0, 0, 19, 19));
        let spoil = w.materials.id_of("spoil").expect("spoil");
        let stone = w.materials.id_of("stone").expect("stone");
        w.set(5, 6, Cell::new(stone, 0));
        w.set(5, 5, Cell::new(spoil, 0));
        w.build_marks.insert((5, 5), 0);
        w.build_marks.insert((6, 5), 0);
        w.frame = 300;
        let m = mark_round(&w, 300.0, (5, 5));
        assert!(
            (m - (-1.0f32).exp()).abs() < 1e-4,
            "one tau old should read e^-1, read {m}"
        );
        assert_eq!(mark_round(&w, 0.0, (5, 5)), 0.0);
        assert_eq!(mark_round(&w, f32::INFINITY, (5, 5)), 1.0);
        assert_eq!(mark_round(&w, 300.0, (10, 10)), 0.0);
    }

    /// The outputs the textbook ant replaces read zero; the rest pass.
    #[test]
    fn outputs_for_zeroes_only_what_it_replaces() {
        use brain::BrainOutput as O;
        let out = outputs_for([0.7; brain::BRAIN_OUTPUTS]);
        for o in [O::EmitA, O::EmitB, O::Impulse, O::Lay, O::Provision] {
            assert_eq!(out[o as usize], 0.0);
        }
        assert_eq!(out[O::Move as usize], 0.7);
    }
}
