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
//! - **pick** (Khuong et al. 2016, PNAS 113:1303, *Lasius niger*, SI): an
//!   empty ant cuts a cell from the layer beneath its head, floor or built
//!   soil alike, with `1 - exp(-eta_p,1 dt / n)`, `n` the ground round its
//!   head: lone pellets are taken and piles kept, which remodels the build
//!   ([`cut_cell`], gated by [`jaw_can_cut`] only).
//! - **put down** (the paper's Eq. 1): a laden ant takes, at random, an open
//!   cell beside its head that the engine will hold and that shuts no way
//!   ([`closes_a_way`]), and puts the pellet there with
//!   `1 - exp(-(eta_d,0 + b_d n) dt exp(-tau_m / tau))`, `tau_m` the time
//!   since the latest deposit round it. A side deposit (no ground beneath,
//!   held only by `update::spoil_corbel`) is further weighted by the body
//!   template `F(h)`, `h` the open height beneath it: the paper's ants put
//!   material on a pillar's side only from about a body length up, which is
//!   what turns pillars into caps, arches and a roof. **Where it can stand
//!   is engine-driven**: without `SPOIL_CORBEL` a pellet needs ground
//!   straight beneath (`needs_footing`) and no cap can stand at all.
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

/// **Slice A's parameters: Khuong et al. 2016's fitted model** (PNAS
/// 113:1303, Eq. 1 and SI; *Lasius niger*), each the authored constant a gene
/// will later scale (design §6). The paper's rates are per second for an ant
/// that steps half a millimetre a second; ours steps one cell, a quarter of
/// its 4 mm body (the body is two cells), so one decision stands for `dt`
/// = 4 s. **That scaling is ours**, as is everything marked so below.
///
/// **Rev 1 of this slice is superseded** (2026-10-15): it took level 1's
/// walker rule (`p0 + (1 - p0) m/(m+K)`, p0 0.002, no pick-up from built
/// soil, no body template) and built one-wide needles 15-40 body lengths
/// tall, which its census scored as pillars. Checked against the paper the
/// owner asked for, the pillars are a body length wide, stop rising at about
/// a body length and cap into arches (`pillars/README.md`, "Checked against
/// the paper").
#[derive(Clone, Copy, Debug)]
pub struct TextbookCfg {
    /// The chance a decision is a step rather than a pause (ours).
    pub p_move: f32,
    /// The chance a step first turns one point, either way at even odds
    /// (ours: the level-1 walkers' one in eight).
    pub turn: f32,
    /// Seconds one decision stands for (ours: 4, see above).
    pub dt: f32,
    /// Deposition rate with nothing round the ant, per second: eta_d,0 0.025.
    pub drop_base: f32,
    /// Deposition rate added per pellet round the ant, per second: b_d 0.11.
    pub drop_per: f32,
    /// Pick-up rate over the pellets round the ant, per second: eta_p,1 0.029.
    pub pick: f32,
    /// The pheromone's mean lifetime, in frames (the paper's 1/eta_m: 20 min,
    /// 1,800 frames at six frames a decision and `dt` 4 s). 0 is no pheromone
    /// (the paper: "no structure can be built"); infinite never fades.
    pub tau: f32,
    /// The body-size template, in cells of open height under a side
    /// deposit: the paper's lowest side deposit is 4.02 mm against a 4.1 mm
    /// body, so a side deposit is half-weighted at `template` cells (one body
    /// length, 2) and full one cell higher (ours: a linear ramp for the
    /// fitted CDF of Fig. 3C).
    pub template: f32,
    /// The material the floor is made of (picked from, as built soil is).
    pub source: material::MaterialId,
    /// Whether bedrock counts in `n` (ours, off by default). The paper's `n`
    /// is "the number of particles present in V26" -- building material --
    /// and its experiments counted "previously deposited pellets". Rev 2
    /// counted all ground, so an ant on the box's bare stone floor or its
    /// walls read three pellets where there were none, and dropped there
    /// at four times the base rate: the bed's own floor pulled the build
    /// wide. Off counts only what the ant could pick (the source and spoil).
    pub n_rock: bool,
    /// Where an empty ant picks from (ours, on by default): any loose pellet
    /// round the head with open sky above it, or, off, only the three cells
    /// under the head -- the paper's "bottom layer of V26" read literally.
    /// Measured on the bare rules in a side view (`pillars/bare-model`, cell
    /// half a body, 47 ants over 388 mm, 20 min pheromone, 3 seeds, 8 days):
    /// picking round the head keeps a field of ~20 pillars 0.6-0.8 body wide
    /// and 2.2-3.9 body lengths apart (the paper's 2.4) for all eight days;
    /// picking only from beneath leaves pillar flanks uneaten, builds 15-18
    /// heaps 2.4-3 bodies wide and 4-5 apart, and freezes by 48 h with every
    /// carrier laden and no fresh mark left to drop by.
    pub pick_around: bool,
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
    /// The pheromone factor `exp(-tau_m / tau)` at the rolls, summed.
    pub mark_sum: f64,
    /// Rolls at a side site (a deposit with no ground beneath, held by
    /// `SPOIL_CORBEL`), and how many of those were won.
    pub side_rolls: u64,
    pub side_drops: u64,
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

/// **The pheromone at `p`**: `exp(-tau_m / tau)`, `tau_m` the frames since
/// the latest deposit in the 3x3 round it whose cell still stands (not empty,
/// not an animal) -- Eq. 1's `exp(-tau_m * eta_m)`, read on the ant's
/// neighbourhood as the paper's `V26` is. 0 at `tau` 0 or with no deposit
/// round it; 1 for any deposit at `tau` infinite.
pub(crate) fn mark_round(world: &World, tau: f32, (x, y): (i32, i32)) -> f32 {
    if tau <= 0.0 || world.build_marks.is_empty() {
        return 0.0;
    }
    let mut latest: Option<u64> = None;
    for dy in -1..=1 {
        for dx in -1..=1 {
            let c = (x + dx, y + dy);
            let Some(&f) = world.build_marks.get(&c) else { continue };
            let cell = world.get(c.0, c.1);
            if cell.material == material::EMPTY || cell.organism_id() != 0 {
                continue;
            }
            latest = Some(latest.map_or(f, |l| l.max(f)));
        }
    }
    match latest {
        None => 0.0,
        Some(_) if tau.is_infinite() => 1.0,
        Some(f) => (-(world.frame.saturating_sub(f) as f32) / tau).exp(),
    }
}

/// Ground a pellet can rest on or be held by: not empty, not an animal, a
/// `Powder` or `Solid` (`is_footing`'s test).
fn ground(world: &World, x: i32, y: i32) -> bool {
    is_footing(world, x, y)
}

/// The pellets round the head: building material in its eight neighbours
/// (the paper's `n`, over `V26`) -- the source and spoil, or all ground
/// under [`TextbookCfg::n_rock`].
fn pellets_round(world: &World, cfg: &TextbookCfg, (x, y): (i32, i32)) -> u32 {
    let spoil = world.materials.id_of("spoil");
    DIRS.iter()
        .filter(|&&(dx, dy)| {
            let (px, py) = (x + dx, y + dy);
            ground(world, px, py)
                && (cfg.n_rock || {
                    let m = world.get(px, py).material;
                    m == cfg.source || Some(m) == spoil
                })
        })
        .count() as u32
}

/// Open cells straight beneath `(x, y)` before ground, up to 8: the height a
/// side deposit would stand over.
fn open_below(world: &World, x: i32, y: i32) -> u32 {
    (1..=8).take_while(|&d| !ground(world, x, y + d)).count() as u32
}

/// **Where a pellet can go**: an open cell beside the head that shares a
/// face with ground (the paper's rule) and that the engine will hold --
/// ground straight beneath, or, under `SPOIL_CORBEL`, a side neighbour that
/// is ground with ground beneath it -- and that shuts no way
/// ([`closes_a_way`]). Returns the site and whether it is a side deposit.
fn sites(world: &World, (x, y): (i32, i32)) -> Vec<((i32, i32), bool)> {
    let corbel = crate::sim::update::spoil_corbel();
    DIRS.iter()
        .map(|&(dx, dy)| (x + dx, y + dy))
        .filter(|&(px, py)| world.in_bounds(px, py) && world.is_empty(px, py))
        .filter_map(|(px, py)| {
            let footed = ground(world, px, py + 1);
            let held = corbel
                && [-1, 1]
                    .iter()
                    .any(|&sx| ground(world, px + sx, py) && ground(world, px + sx, py + 1));
            (footed || held).then_some(((px, py), !footed))
        })
        .filter(|&(p, _)| !closes_a_way(world, p))
        .collect()
}

/// **The verbs, in place of `act`: Khuong's Eq. 1.** An empty ant picks a
/// cell from the layer beneath it (the paper's "bottom layer of V26") with
/// `1 - exp(-eta_p,1 dt / n)`: lone pellets go, piles stay. A laden ant takes
/// a site at random and puts the pellet there with
/// `F(h) (1 - exp(-(eta_d,0 + b_d n) dt exp(-tau_m / tau)))`, `F` the body
/// template for a side deposit (1 on ground). Returns what the tick charges
/// for (a cut is a dig).
pub(super) fn act(world: &mut World, organism: OrganismId, def: &CreatureDef, draw: &mut rng::Rng) -> Did {
    let Some(cfg) = world.textbook.as_ref().map(|t| t.cfg) else {
        return Did::default();
    };
    let Some((head, laden)) = world.organism(organism).map(|s| (s.chain.first().copied(), s.spoil)) else {
        return Did::default();
    };
    let Some((x, y)) = head else { return Did::default() };
    count(world, |c| c.decisions += 1);
    let n = pellets_round(world, &cfg, (x, y));
    match laden {
        None => {
            let spoil = world.materials.id_of("spoil");
            let reach: Vec<(i32, i32)> = if cfg.pick_around {
                // up is -y: a pellet with no ground on top (an ant standing on it does not count)
                DIRS.iter()
                    .map(|&(dx, dy)| (x + dx, y + dy))
                    .filter(|&(px, py)| world.in_bounds(px, py - 1) && !ground(world, px, py - 1))
                    .collect()
            } else {
                (-1..=1).map(|dx| (x + dx, y + 1)).collect()
            };
            let src: Vec<(i32, i32)> = reach
                .into_iter()
                .filter(|&(px, py)| {
                    let c = world.get(px, py);
                    world.in_bounds(px, py)
                        && (c.material == cfg.source || Some(c.material) == spoil)
                        && c.organism_id() == 0
                        && jaw_can_cut(world, def, organism, c)
                })
                .collect();
            let p = 1.0 - (-cfg.pick * cfg.dt / n.max(1) as f32).exp();
            if src.is_empty() || draw.unit_f32() >= p {
                return Did::default();
            }
            let at = src[draw.below(src.len() as u32) as usize];
            cut_cell(world, organism, at, true);
            count(world, |c| c.picks += 1);
            Did {
                dug: 1,
                ..Did::default()
            }
        }
        Some(load) => {
            let options = sites(world, (x, y));
            if options.is_empty() {
                count(world, |c| c.no_site += 1);
                return Did::default();
            }
            let (at, side) = options[draw.below(options.len() as u32) as usize];
            let fresh = mark_round(world, cfg.tau, (x, y));
            let template = if side {
                (open_below(world, at.0, at.1) as f32 - cfg.template + 0.5).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let chance = template * (1.0 - (-(cfg.drop_base + cfg.drop_per * n as f32) * cfg.dt * fresh).exp());
            let won = draw.unit_f32() < chance;
            count(world, |c| {
                c.rolls += 1;
                c.mark_sum += f64::from(fresh);
                c.drops += u64::from(won);
                c.side_rolls += u64::from(side);
                c.side_drops += u64::from(side && won);
            });
            if won {
                place_pellet(world, organism, at, load.cell);
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
            dt: 4.0,
            drop_base: 0.025,
            drop_per: 0.11,
            pick: 0.029,
            tau,
            template: 2.0,
            source: soil,
            n_rock: false,
            pick_around: true,
        };
        // The layer is laid fresh at the start (the harness's bootstrap:
        // with no deposit anywhere, Eq. 1 never puts a pellet down).
        for x in 2..w - 2 {
            world.build_marks.insert((x, h - 10), 0);
        }
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

    /// **The pheromone reads the latest deposit still standing, and fades**:
    /// a mark on a cell that is now empty counts nothing; the latest of two
    /// wins; one `tau` old reads e^-1; `tau` 0 is no pheromone and infinite
    /// reads 1.
    #[test]
    fn mark_round_reads_the_latest_standing_deposit() {
        let mut w = World::new(Rect::new(0, 0, 19, 19));
        let spoil = w.materials.id_of("spoil").expect("spoil");
        w.set(5, 5, Cell::new(spoil, 0));
        w.set(4, 5, Cell::new(spoil, 0));
        w.build_marks.insert((5, 5), 0);
        w.build_marks.insert((4, 5), 100);
        w.build_marks.insert((6, 5), 250);
        w.frame = 400;
        let m = mark_round(&w, 300.0, (5, 5));
        assert!(
            (m - (-1.0f32).exp()).abs() < 1e-4,
            "the latest standing deposit is one tau old; read {m}"
        );
        assert_eq!(mark_round(&w, 0.0, (5, 5)), 0.0);
        assert_eq!(mark_round(&w, f32::INFINITY, (5, 5)), 1.0);
        assert_eq!(mark_round(&w, 300.0, (12, 12)), 0.0);
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
