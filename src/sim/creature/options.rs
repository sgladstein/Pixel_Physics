//! **The option list: everything an ant could legally do this decision, each
//! described by the same features** -- stage 2, step 5 of the brain plan
//! (`brain-training/design/general-brain-design-rev3.md`, *How it decides*;
//! stage 2 chosen by the owner 2026-10-17).
//!
//! The full brain picks a job, then that job's target, by scoring every
//! (verb, target) pair with one shared scorer. This builds the pairs and
//! their features: each of the eight steps the body can take, turning
//! around, resting, each cell the jaws can cut, each item in reach to pick
//! up or eat, each free cell to put a load down in, each nestmate or larva
//! to feed and each stranger to bite. A typical ant has 10-25.
//!
//! **Built as a shadow, and only a shadow**: nothing reads the list, so the
//! game is unchanged with it on. It exists to be timed at lab scale before
//! any brain uses it -- the design estimated that building the options would
//! cost more than the network itself, and the whole new ant has to fit in
//! 1.3x the game ant's frame (`needscost shadow=1`). Off unless
//! [`World::option_shadow`] is set.
//!
//! **Features that need something the engine does not have yet read 0**:
//! the building scent (it comes with its emit) and the way back to the last
//! food (a new body memory). They keep their columns so the cost and the
//! layout are the full list's.
//!
//! **No transcendental anywhere a decision reads** (`brain.rs`): squashes
//! are `x / (1 + |x|)`, and the home direction's cosine is a ratio of
//! integers' square roots, which IEEE rounds exactly.
//!
//! [`World::option_shadow`]: crate::sim::World::option_shadow

use super::{body_after_step, body_has_foothold, jaw_can_cut, kin_footing, parting_enabled, segment_authored_widths, spoil_site_open, stacker_of, under_cover, BodyShape, CreatureDef, DIRS};
use super::{flipped_body, landing_is_placeable_through_tissue, spoil_footing_drop};
use crate::sim::cell::OrganismId;
use crate::sim::material::{self, MaterialId, MaterialKind};
use crate::sim::pheromone::{self, Channel};
use crate::sim::World;
use std::cell::RefCell;

/// What the body can do, the design's "must" verbs for the first ant.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verb {
    /// One of the eight headings the body can enter.
    Walk,
    /// A long body reverses rather than stepping into its own neck.
    TurnAround,
    Rest,
    Dig,
    /// Food or a corpse in reach (brood carrying is a hand rule, off).
    PickUp,
    /// A load put down in a free cell beside the head.
    Drop,
    /// Food in reach, or the crop's own (target: the head).
    Eat,
    /// A nestmate or larva in reach, from a crop with food in it.
    Feed,
    /// A stranger in reach.
    Attack,
}

pub const VERBS: usize = 9;

impl Verb {
    pub const ALL: [Verb; VERBS] = [Verb::Walk, Verb::TurnAround, Verb::Rest, Verb::Dig, Verb::PickUp, Verb::Drop, Verb::Eat, Verb::Feed, Verb::Attack];

    pub fn name(self) -> &'static str {
        ["walk", "turn_around", "rest", "dig", "pick_up", "drop", "eat", "feed", "attack"][self as usize]
    }
}

/// Feature indices, in [`feature_names`] order.
pub mod f {
    /// What is in the target cell, one of nine (one-hot).
    pub const AIR: usize = 0;
    pub const SOIL: usize = 1;
    pub const SPOIL: usize = 2;
    pub const FOOD: usize = 3;
    pub const BROOD: usize = 4;
    pub const CORPSE: usize = 5;
    pub const STONE: usize = 6;
    pub const WATER: usize = 7;
    pub const ANT: usize = 8;
    /// Scent A, B and alarm there, each as strength, how much stronger
    /// than here, and how much stronger than here two cells further that
    /// way: `SCENT + 3 * channel + k`. The two differences are gained
    /// (`SCENT_GAIN`).
    pub const SCENT: usize = 9;
    /// Missing: comes with the building-scent emit.
    pub const BUILDING: usize = 18;
    /// The body can stand there; it would fall; it climbs.
    pub const STAND: usize = 19;
    pub const FALL: usize = 20;
    pub const CLIMB: usize = 21;
    /// The jaws can cut it, and how firm it is.
    pub const CUT: usize = 22;
    pub const HARD: usize = 23;
    /// Soil wetness there, by level.
    pub const WET: usize = 24;
    /// Up (+) or down (-) from the head; rows below the ant's last surface
    /// row; open cells beneath it, a cliff's height.
    pub const UP: usize = 25;
    pub const DEPTH: usize = 26;
    pub const DROP_BELOW: usize = 27;
    /// Open cells above it, in body heights; ground overhead.
    pub const HEADROOM: usize = 28;
    pub const ROOFED: usize = 29;
    /// Who is there: nestmate, stranger, laden, hungry, larva, the larva's
    /// hunger, dead.
    pub const NESTMATE: usize = 30;
    pub const STRANGER: usize = 31;
    pub const LADEN: usize = 32;
    pub const HUNGRY: usize = 33;
    pub const LARVA: usize = 34;
    pub const LARVA_HUNGER: usize = 35;
    pub const DEAD: usize = 36;
    /// Animals beside it; open cells around it per animal.
    pub const CROWD: usize = 37;
    pub const ROOM: usize = 38;
    /// How much the option points home, by the body's home vector.
    pub const HOME: usize = 39;
    /// Missing: the way back to the last food is a new body memory.
    pub const LAST_FOOD: usize = 40;
    /// Warmer (+) or cooler (-) there than here.
    pub const WARMTH: usize = 41;
    /// The option undoes the last step.
    pub const BACK: usize = 42;
}

/// Features per option.
pub const FEATURES: usize = 43;

/// The names of the features, for readouts.
pub fn feature_names() -> [&'static str; FEATURES] {
    [
        "air", "soil", "spoil", "food", "brood", "corpse", "stone", "water", "ant", "a", "a_diff", "a_ahead", "b", "b_diff", "b_ahead", "alarm", "alarm_diff", "alarm_ahead", "building", "stand", "fall", "climb", "cut", "hard", "wet", "up", "depth", "drop_below", "headroom", "roofed", "nestmate", "stranger", "laden", "hungry", "larva", "larva_hunger", "dead", "crowd", "room", "home", "last_food", "warmth", "back",
    ]
}

/// One thing the ant could do: a verb and the cell it acts on.
#[derive(Clone, Copy, Debug)]
pub struct Opt {
    pub verb: Verb,
    pub target: (i32, i32),
    pub feats: [f32; FEATURES],
}

/// The materials the features name, looked up once a decision rather than
/// per cell (`.claude/rules/measuring-the-world.md`, hot-path work).
struct Mats {
    spoil: Option<MaterialId>,
    packed: Option<MaterialId>,
    corpse: Option<MaterialId>,
}

/// What every option of one decision shares: where the head is, what is
/// sensed there, and the body's own memory.
struct Here {
    head: (i32, i32),
    neck: Option<(i32, i32)>,
    colony: u32,
    scent: [f32; 3],
    warmth: f32,
    home: Option<(i32, i32)>,
    surface: Option<i32>,
    mats: Mats,
}

fn squash(x: f32) -> f32 {
    x / (1.0 + x.abs())
}

fn scent(world: &World, ch: Channel, x: i32, y: i32) -> f32 {
    world.pheromone_at(ch, x, y) as f32 / pheromone::Scent::MAX as f32
}

const CHANNELS: [Channel; 3] = [Channel::A, Channel::B, Channel::Alarm];

/// **The gain on a scent difference, before the squash.** Along a trail
/// the scent changes by about 0.005-0.008 of full scale per cell and
/// 0.016-0.023 over three, so raw differences would need weights of 50-150
/// to move a pick, three times the rig's bound (ladder lane,
/// `trail-rung-controls.md`, measured on the trail arena's hold-out). At 32
/// a cell's slope reads about 0.15, three cells' about 0.4, and the edge of
/// a trail, a drop of 0.1-0.3, 0.75-0.9: the whole range in use.
const SCENT_GAIN: f32 = 32.0;

/// How far up and down the head-room and cliff reads look, in cells: two
/// body heights of a three-cell ant.
const REACH: i32 = 6;

/// **Every option `organism` has right now**, appended to `out` (cleared
/// first). A pure read of the world: no draw, no write.
pub fn build(world: &World, organism: OrganismId, def: &CreatureDef, out: &mut Vec<Opt>) {
    out.clear();
    let Some(st) = world.organism(organism) else { return };
    let Some(&head) = st.chain.first() else { return };
    let here = Here {
        head,
        neck: st.chain.get(1).copied(),
        colony: st.colony,
        scent: CHANNELS.map(|ch| scent(world, ch, head.0, head.1)),
        warmth: world.field_at_bilinear(head.0 as f32, head.1 as f32).temperature,
        home: (!world.home_blind).then_some(st.forage_anchor),
        surface: st.last_out_row,
        mats: Mats { spoil: world.materials.id_of("spoil"), packed: world.materials.id_of("packedsoil"), corpse: world.materials.id_of("corpse") },
    };
    let mut push = |verb: Verb, target: (i32, i32), extra: &dyn Fn(&mut [f32; FEATURES])| {
        let mut feats = describe(world, &here, target);
        extra(&mut feats);
        out.push(Opt { verb, target, feats });
    };

    // Walk: the eight landings, each legal if the body fits there, with the
    // foothold as a feature rather than a filter (`usable_headings` filters
    // it), so a step off an edge is an option the brain can refuse.
    let groups = &st.segment_groups;
    let authored = segment_authored_widths(def, st.fates);
    let parting = parting_enabled();
    let stacker = stacker_of(world, organism);
    let kin = kin_footing(world, organism, def);
    for d in 0..8u8 {
        let (dx, dy) = DIRS[d as usize];
        let t = (head.0 + dx, head.1 + dy);
        // `from` and `to` both `d`, as `usable_headings` asks it: the same
        // predicate the walk and the tumble use.
        let (landing, _) = body_after_step(world, def, BodyShape { chain: &st.chain, groups, authored: &authored }, t, d, d, parting);
        if !landing_is_placeable_through_tissue(world, &st.chain, &landing, parting, stacker) {
            continue;
        }
        let footed = body_has_foothold(world, def, &landing, t, kin);
        push(Verb::Walk, t, &|feats| {
            feats[f::STAND] = f32::from(u8::from(footed));
            feats[f::FALL] = f32::from(u8::from(!footed));
            feats[f::CLIMB] = f32::from(u8::from(footed && dy < 0));
        });
    }
    // Turning around is the walk's own flip (`flipped_body`): legal when
    // the reversed body lays out without crossing itself.
    if st.chain.len() > 1 && flipped_body(world, &st.chain, groups, &authored, parting).is_some() {
        let tail = *st.chain.last().expect("len > 1");
        push(Verb::TurnAround, tail, &|_| {});
    }
    push(Verb::Rest, head, &|_| {});

    let jaws_free = st.spoil.is_none();
    let crop_food = st.crop.is_some();
    for &(dx, dy) in DIRS.iter() {
        let t = (head.0 + dx, head.1 + dy);
        let cell = world.get(t.0, t.1);
        let m = world.materials.get(cell.material);
        let food = m.food_energy > 0.0 && cell.organism_id() == 0;
        let corpse = here.mats.corpse.is_some_and(|c| cell.material == c);
        if jaws_free && !crop_food && jaw_can_cut(world, def, organism, cell) {
            push(Verb::Dig, t, &|_| {});
        }
        if jaws_free && (food || corpse) {
            push(Verb::PickUp, t, &|_| {});
        }
        if food {
            push(Verb::Eat, t, &|_| {});
        }
        if (st.spoil.is_some() || crop_food) && spoil_site_open(world, t.0, t.1, spoil_footing_drop()) {
            push(Verb::Drop, t, &|_| {});
        }
        let other = cell.organism_id();
        if other != organism {
            if let Some(o) = animal(world, other) {
                if o.colony == here.colony {
                    if crop_food {
                        push(Verb::Feed, t, &|_| {});
                    }
                } else {
                    push(Verb::Attack, t, &|_| {});
                }
            }
        }
    }
    if crop_food {
        push(Verb::Eat, head, &|_| {});
    }
}

/// The animal standing in a cell, if any: a plant is an organism too.
fn animal(world: &World, id: OrganismId) -> Option<&crate::sim::organism::OrganismState> {
    (id != 0).then(|| world.organism(id)).flatten().filter(|o| world.species.get(o.species).creature.is_some())
}

/// The features every option reads at its target, verb-specific ones (the
/// walk's footing) left for the caller to set.
fn describe(world: &World, here: &Here, (x, y): (i32, i32)) -> [f32; FEATURES] {
    let mut v = [0.0f32; FEATURES];
    let cell = world.get(x, y);
    let kind = world.materials.kind(cell.material);
    let m = world.materials.get(cell.material);
    let who = animal(world, cell.organism_id());
    let class = if who.is_some_and(|o| o.brood.is_some()) {
        f::BROOD
    } else if who.is_some() {
        f::ANT
    } else if cell.material == material::EMPTY || kind == MaterialKind::Gas {
        f::AIR
    } else if here.mats.corpse.is_some_and(|c| cell.material == c) {
        f::CORPSE
    } else if m.food_energy > 0.0 {
        f::FOOD
    } else if here.mats.spoil.is_some_and(|s| cell.material == s) || here.mats.packed.is_some_and(|p| cell.material == p) {
        f::SPOIL
    } else if kind == MaterialKind::Liquid {
        f::WATER
    } else if kind == MaterialKind::Powder {
        f::SOIL
    } else {
        f::STONE
    };
    v[class] = 1.0;

    let (dx, dy) = ((x - here.head.0).signum(), (y - here.head.1).signum());
    for (c, ch) in CHANNELS.into_iter().enumerate() {
        let s = scent(world, ch, x, y);
        v[f::SCENT + 3 * c] = s;
        v[f::SCENT + 3 * c + 1] = squash(SCENT_GAIN * (s - here.scent[c]));
        v[f::SCENT + 3 * c + 2] = squash(SCENT_GAIN * (scent(world, ch, x + 2 * dx, y + 2 * dy) - here.scent[c]));
    }

    // Footing for a non-walk target: something filled under it.
    let below = world.get(x, y + 1).material != material::EMPTY;
    v[f::STAND] = f32::from(u8::from(below));
    if kind == MaterialKind::Powder || kind == MaterialKind::Solid {
        v[f::HARD] = squash(m.penetration_resistance);
    }
    // Wetness by level: a powder's own water, else what it stands on.
    let wet_of = |c: crate::sim::Cell| (world.materials.kind(c.material) == MaterialKind::Powder).then(|| f32::from(c.aux()) / f32::from(material::SOIL_SATURATED));
    v[f::WET] = wet_of(cell).or_else(|| wet_of(world.get(x, y + 1))).unwrap_or(0.0).min(1.0);

    v[f::UP] = (here.head.1 - y).signum() as f32;
    v[f::DEPTH] = here.surface.map_or(0.0, |s| squash((y - s) as f32 / 16.0));
    v[f::DROP_BELOW] = (1..=REACH).take_while(|&d| world.get(x, y + d).material == material::EMPTY).count() as f32 / REACH as f32;
    v[f::HEADROOM] = (1..=REACH).take_while(|&d| world.get(x, y - d).material == material::EMPTY).count() as f32 / REACH as f32;
    v[f::ROOFED] = f32::from(u8::from(under_cover(world, x, y)));

    if let Some(o) = who {
        let mine = o.colony == here.colony;
        v[f::NESTMATE] = f32::from(u8::from(mine));
        v[f::STRANGER] = f32::from(u8::from(!mine));
        v[f::LADEN] = f32::from(u8::from(o.crop.is_some() || o.spoil.is_some()));
        let start = world.species.get(o.species).creature.as_ref().map_or(1.0, |d| d.start_energy).max(1.0);
        let hunger = (1.0 - o.energy / start).clamp(0.0, 1.0);
        if o.brood.is_some() {
            v[f::LARVA] = 1.0;
            v[f::LARVA_HUNGER] = hunger;
        } else {
            v[f::HUNGRY] = f32::from(u8::from(hunger > 0.5));
        }
    }
    v[f::DEAD] = f32::from(u8::from(here.mats.corpse.is_some_and(|c| cell.material == c)));

    let (mut animals, mut open, mut near) = (0, 0, 0);
    for ny in -2..=2 {
        for nx in -2..=2 {
            if nx == 0 && ny == 0 {
                continue;
            }
            let c = world.get(x + nx, y + ny);
            let animal = c.organism_id() != 0;
            if animal && nx.abs() <= 1 && ny.abs() <= 1 {
                animals += 1;
            }
            near += i32::from(animal);
            open += i32::from(c.material == material::EMPTY);
        }
    }
    v[f::CROWD] = animals as f32 / 8.0;
    v[f::ROOM] = open as f32 / 24.0 / (1 + near) as f32;

    // How much this option points home: the cosine between the step and
    // the home vector, times how far home is, squashed. 0 at home, or blind.
    if let Some((ax, ay)) = here.home {
        let (hx, hy) = (ax - here.head.0, ay - here.head.1);
        let (sx, sy) = (x - here.head.0, y - here.head.1);
        let (h2, s2) = ((hx * hx + hy * hy) as f32, (sx * sx + sy * sy) as f32);
        if h2 > 0.0 && s2 > 0.0 {
            let cos = (hx * sx + hy * sy) as f32 / (h2.sqrt() * s2.sqrt());
            v[f::HOME] = cos * squash(h2.sqrt() / 32.0);
        }
    }
    v[f::WARMTH] = squash(world.field_at_bilinear(x as f32, y as f32).temperature - here.warmth);
    v[f::BACK] = f32::from(u8::from(here.neck == Some((x, y))));
    v
}

thread_local! {
    static SCRATCH: RefCell<Vec<Opt>> = const { RefCell::new(Vec::new()) };
}

/// **The shadow**: build `organism`'s options and throw them away, counting
/// what was built ([`World::creature_stats`]: decisions, options per verb,
/// and a sum of every feature so nothing is optimised out). Called once per
/// decision from `creature_tick` when [`World::option_shadow`] is set.
///
/// [`World::creature_stats`]: crate::sim::World::creature_stats
/// [`World::option_shadow`]: crate::sim::World::option_shadow
pub fn shadow(world: &mut World, organism: OrganismId, def: &CreatureDef) {
    SCRATCH.with(|s| {
        let mut opts = s.borrow_mut();
        build(world, organism, def, &mut opts);
        let st = &mut world.creature_stats;
        st.option_decisions += 1;
        st.options_built += opts.len() as u64;
        for o in opts.iter() {
            st.options_by_verb[o.verb as usize] += 1;
            st.option_feature_sum += o.feats.iter().map(|&v| f64::from(v)).sum::<f64>();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::creature::{targets, usable_headings};
    use crate::sim::Cell;

    fn colony_box() -> World {
        crate::lab::scene::LabBox {
            founders: 4,
            colonies: 1,
            colony_ants: 12,
            width: 256,
            height: 128,
            ground_y: 64,
            soil_depth: 40,
            ..crate::lab::scene::LabBox::default()
        }
        .build()
    }

    fn step(w: &mut World) {
        crate::sim::update::step(w);
        w.step_active_sites();
        w.step_fields();
        w.step_pheromones();
    }

    /// **The shadow leaves the game unchanged**: one box run twice from the
    /// same seed, the shadow on in one, is the same world cell for cell and
    /// ant for ant after 3,000 frames -- and the shadow did fire, at every
    /// decision of the run, with every verb a digging colony meets. Watched
    /// red with one draw from the world's stream added to `shadow`.
    #[test]
    fn the_shadow_leaves_the_game_unchanged() {
        let (mut off, mut on) = (colony_box(), colony_box());
        on.option_shadow = true;
        for _ in 0..3000 {
            step(&mut off);
            step(&mut on);
        }
        let b = off.bounds().expect("a lab box is bounded");
        for y in b.min_y..b.max_y {
            for x in b.min_x..b.max_x {
                let (a, b) = (off.get(x, y), on.get(x, y));
                assert!(
                    a.material == b.material && a.aux() == b.aux() && a.organism_id() == b.organism_id(),
                    "({x},{y}) differs: {a:?} against {b:?}"
                );
            }
        }
        let (ids_off, ids_on) = (off.live_organism_ids(), on.live_organism_ids());
        assert_eq!(ids_off, ids_on, "different organisms alive");
        for id in ids_off {
            let (a, b) = (off.organism(id).expect("live"), on.organism(id).expect("live"));
            assert_eq!((&a.chain, a.heading, a.energy.to_bits()), (&b.chain, b.heading, b.energy.to_bits()), "organism {id} differs");
        }
        let st = on.creature_stats;
        assert_eq!(off.creature_stats.option_decisions, 0, "the shadow ran with the switch off");
        assert!(st.option_decisions > 1000, "only {} decisions built", st.option_decisions);
        assert_eq!(st.options_by_verb[Verb::Rest as usize], st.option_decisions, "rest is offered at every decision");
        for v in [Verb::Walk, Verb::TurnAround, Verb::Dig, Verb::Drop] {
            assert!(st.options_by_verb[v as usize] > 0, "a digging colony never had a {} option", v.name());
        }
        let per = st.options_built as f64 / st.option_decisions as f64;
        assert!((3.0..40.0).contains(&per), "{per:.1} options a decision");
    }

    /// **The options are the body's own**: at every decision sampled from a
    /// working colony, rest is offered once at the head; the footed steps
    /// are exactly the walk's own (`usable_headings`); the digs are exactly
    /// the cells the jaws can cut (`targets::dig_candidates`) and only with
    /// empty jaws and crop; a load goes down only from a carrier and only
    /// where the spoil branch would put it; one colony has nobody to bite;
    /// each target is one kind of thing; every feature is in -1..1. Then food
    /// is put beside a head, and a pick-up and a meal appear on it: the
    /// positive control for the reach scan.
    #[test]
    fn the_options_are_the_bodys_own() {
        let mut w = colony_box();
        let sp = w.species.id_of("ant").expect("ant");
        let def = w.species.get(sp).creature.clone().expect("creature");
        let mut opts = Vec::new();
        let (mut decisions, mut carrying) = (0, 0);
        for f in 0..3000 {
            step(&mut w);
            if f % 25 != 0 {
                continue;
            }
            for id in w.live_organism_ids() {
                let Some(st) = w.organism(id).filter(|s| s.species == sp && s.brood.is_none()) else {
                    continue;
                };
                let Some(&head) = st.chain.first() else { continue };
                let (jaws, crop) = (st.spoil.is_some(), st.crop.is_some());
                build(&w, id, &def, &mut opts);
                decisions += 1;
                carrying += usize::from(jaws || crop);
                let of = |v: Verb| -> Vec<(i32, i32)> { opts.iter().filter(|o| o.verb == v).map(|o| o.target).collect() };
                assert_eq!(of(Verb::Rest), vec![head], "frame {f} ant {id}");
                let footed: Vec<(i32, i32)> = opts.iter().filter(|o| o.verb == Verb::Walk && o.feats[f::STAND] == 1.0).map(|o| o.target).collect();
                let usable: Vec<(i32, i32)> = usable_headings(&w, id, &def).iter().map(|&d| (head.0 + DIRS[d as usize].0, head.1 + DIRS[d as usize].1)).collect();
                assert_eq!(footed, usable, "frame {f} ant {id}: footed steps against the walk's own");
                let digs = if jaws || crop { Vec::new() } else { targets::dig_candidates(&w, &def, id, head) };
                assert_eq!(of(Verb::Dig), digs, "frame {f} ant {id}: digs");
                for t in of(Verb::Drop) {
                    assert!(jaws || crop, "frame {f} ant {id}: a drop with nothing to drop");
                    assert!(spoil_site_open(&w, t.0, t.1, spoil_footing_drop()), "frame {f} ant {id}: drop at {t:?}");
                }
                assert!(of(Verb::Attack).is_empty(), "frame {f} ant {id}: a stranger in a one-colony box");
                for o in &opts {
                    assert_eq!(o.feats[..f::SCENT].iter().filter(|&&v| v == 1.0).count(), 1, "frame {f} ant {id}: {:?} at {:?} is not one kind of thing", o.verb, o.target);
                    for (k, &v) in o.feats.iter().enumerate() {
                        assert!(v.is_finite() && (-1.0..=1.0).contains(&v), "frame {f} ant {id}: {} = {v} on {:?}", feature_names()[k], o.verb);
                    }
                }
            }
        }
        assert!(decisions > 1000 && carrying > 0, "{decisions} decisions, {carrying} carrying");

        // The control: food beside a free-jawed head.
        let food = w.materials.id_of("provisions").expect("provisions");
        let (id, head) = w
            .live_organism_ids()
            .into_iter()
            .filter_map(|id| w.organism(id).filter(|s| s.species == sp && s.brood.is_none() && s.spoil.is_none()).and_then(|s| s.chain.first().map(|&h| (id, h))))
            .find(|&(_, (x, y))| w.get(x + 1, y).material == material::EMPTY && w.get(x - 1, y).material == material::EMPTY)
            .expect("a free-jawed ant with open ground either side");
        let t = (head.0 + 1, head.1);
        build(&w, id, &def, &mut opts);
        assert!(!opts.iter().any(|o| o.target == t && o.verb == Verb::PickUp), "food offered before it was put down");
        w.set(t.0, t.1, Cell::new(food, 0));
        build(&w, id, &def, &mut opts);
        for v in [Verb::PickUp, Verb::Eat] {
            let o = opts.iter().find(|o| o.target == t && o.verb == v).unwrap_or_else(|| panic!("no {} on the food", v.name()));
            assert_eq!(o.feats[f::FOOD], 1.0, "the food reads as food");
        }
    }
}
