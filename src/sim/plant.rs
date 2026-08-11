//! M16: plant growth — moss, and trees with roots.
//!
//! Dispatched from `scheduler::step` once per due active site. Everything
//! here writes through the ordinary `World::get`/`set` — this runs as its
//! own frame phase, not inside the CA sweep, so there is no `CellSurface`
//! genericity to worry about (see the M5 `surface.rs` doc for why that
//! mattered there and doesn't here).
//!
//! Grounded in `research/m16-plant-biology.md`, not invented from
//! plausible-looking rules — the module doc on each growth function says
//! which real mechanism it's translating and what's simplified. Read that
//! file before touching the constants below; they're not arbitrary.

use super::cell::Cell;
use super::material::{self, MaterialKind};
use super::scheduler::{ActiveKind, ActiveSite};
use super::world::World;

const NEIGHBOURS_4: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

// --- Moss ---------------------------------------------------------------
//
// Real moss is poikilohydric — no waterproof cuticle, no internal water
// regulation — so it depends entirely on ambient moisture and favours
// shade because shade slows evaporation, not because of any directional
// pull. That's translated directly: spread chance is gated hard on nearby
// water (a poikilohydric plant has no fallback without it) and modulated by
// local shade, read from the M13 light field this engine already computes.

/// Frames between a moss tip's growth attempts.
const MOSS_TICK_INTERVAL: u64 = 45;
/// How far a candidate cell is checked for standing water.
const MOSS_DAMP_RADIUS: i32 = 3;
/// Spread chance when the candidate cell is actually damp.
const MOSS_DAMP_CHANCE: f32 = 0.35;
/// Spread chance when it isn't — small but nonzero, since real moss can
/// survive brief dry spells on residual moisture rather than never
/// colonizing a marginal spot at all. Kept very small (not just "smaller
/// than damp"): a poikilohydric plant genuinely cannot establish on
/// substrate that is never wet, so this represents rare atmospheric
/// humidity/dew, not a real alternate growth path.
const MOSS_DRY_CHANCE: f32 = 0.002;
/// Consecutive no-growable-candidate checks a moss tip tolerates before it
/// stops rescheduling itself and goes dormant for good. Not an immediate
/// death on the first empty check — a candidate can be transiently
/// unavailable (another tip claims a shared neighbour first, water shifts
/// away and back) — but bounded, so a tip that's genuinely and permanently
/// enclosed doesn't sit on the active-site list forever. Nothing in this
/// engine currently has a mechanism to "wake" a dormant tip if conditions
/// change afterward (e.g. new water appears nearby later); that's a real
/// limitation, acceptable because a growing colony's *other* tips still
/// respond to it, not because it's unnoticed.
const MOSS_STALE_LIMIT: u8 = 4;

fn moss_tick(world: &mut World, x: i32, y: i32, stale_ticks: u8) -> Vec<ActiveSite> {
    let Some(moss_id) = world.materials.id_of("moss") else {
        return Vec::new();
    };
    // The tip cell may have burned, been erased, or already be something
    // else by the time its schedule comes due — nothing to grow from.
    if world.get(x, y).material != moss_id {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    for (dx, dy) in NEIGHBOURS_4 {
        let (nx, ny) = (x + dx, y + dy);
        if world.is_empty(nx, ny) && has_growable_neighbour(world, nx, ny, moss_id) {
            candidates.push((nx, ny));
        }
    }

    if candidates.is_empty() {
        if stale_ticks + 1 >= MOSS_STALE_LIMIT {
            return Vec::new(); // permanently enclosed (or believed to be) -- stop checking
        }
        return vec![reschedule(x, y, ActiveKind::Moss { stale_ticks: stale_ticks + 1 }, world.frame + MOSS_TICK_INTERVAL)];
    }

    // A candidate existed this tick, whether or not the growth roll below
    // succeeds -- reset the counter, since "had somewhere to try" is what
    // staleness tracks, not "successfully grew."
    let mut next = vec![reschedule(x, y, ActiveKind::Moss { stale_ticks: 0 }, world.frame + MOSS_TICK_INTERVAL)];

    let (tx, ty) = candidates[world.rng.below(candidates.len() as u32) as usize];
    let damp = is_damp(world, tx, ty, MOSS_DAMP_RADIUS);
    let chance = (if damp { MOSS_DAMP_CHANCE } else { MOSS_DRY_CHANCE }) * shade_factor(world, tx, ty);
    if world.rng.chance(chance) {
        let shades = world.materials.get(moss_id).palette.len().max(1) as u32;
        let shade = world.rng.below(shades) as u8;
        world.set(tx, ty, Cell::new(moss_id, shade));
        next.push(reschedule(tx, ty, ActiveKind::Moss { stale_ticks: 0 }, world.frame + MOSS_TICK_INTERVAL));
    }
    next
}

/// A candidate cell can grow moss if it touches either the stone/solid
/// surface moss first colonized, or existing moss itself — real moss forms
/// a thickening 2D patch by growing over its own earlier growth, not a
/// single-cell-wide line that can only ever hug the original rock. Without
/// the moss case, a cell whose one solid neighbour is already-grown moss
/// (not raw stone) reads as having no growable neighbour at all, and every
/// growth front dead-ends after one step.
fn has_growable_neighbour(world: &World, x: i32, y: i32, moss_id: material::MaterialId) -> bool {
    NEIGHBOURS_4.iter().any(|&(dx, dy)| {
        let m = world.get(x + dx, y + dy).material;
        world.materials.kind(m) == MaterialKind::Solid || m == moss_id
    })
}

fn is_damp(world: &World, x: i32, y: i32, radius: i32) -> bool {
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if world.materials.kind(world.get(x + dx, y + dy).material) == MaterialKind::Liquid {
                return true;
            }
        }
    }
    false
}

/// Lower ambient light reads as more shaded, which favours spreading —
/// real moss's actual preference (shade slows evaporation), not a made-up
/// bonus. Floored rather than let hit zero, since total darkness isn't a
/// hard "never" biologically, just a strong preference.
fn shade_factor(world: &World, x: i32, y: i32) -> f32 {
    let light = world.field_at(x, y).light;
    (1.0 - (light / 2.0).clamp(0.0, 1.0)).max(0.1)
}

// --- Trees and roots ------------------------------------------------------
//
// Space colonization (Runions/Lane/Prusinkiewicz 2007) for the canopy
// shape, extended with two more real mechanisms from the deep-dive
// research pass rather than left as a bare attractor-seeking loop:
//
// - **Auxin canalization** (Prusinkiewicz et al. 2009, PNAS) for branching
//   and apical dominance, translated at three levels rather than one —
//   worth being precise about which parts are genuine cross-tip
//   competition and which are simpler mechanisms that happen to produce a
//   similar-looking result:
//   1. Each `Tip` carries a scalar `channel` that grows on a successful
//      growth step and decays when the tip can't find anything to grow
//      toward, dying below a minimum. This part is *not* cross-tip
//      competition by itself — a tip's channel only ever responds to its
//      own history.
//   2. When a tip's channel is strong enough to spawn a branch, the new
//      branch's starting channel is *debited from the parent's*, not
//      created for free — this is the one place one tip's channel value
//      actually depends on another's, the minimum needed to call it
//      competition rather than N independent scalars that happen to share
//      a name with a real mechanism.
//   3. The bulk of the real "one leader, suppressed siblings" effect
//      actually comes from two *other*, more ordinary mechanisms already
//      needed elsewhere: tips share one `attractors` list that shrinks as
//      any tip consumes nearby points, so whichever tip reaches a cluster
//      first starves siblings of *targets* (plain space colonization); and
//      all tips (and now roots — see `ROOT_GROWTH_COST`) draw from one
//      flat `energy` pool, so a tip can starve even with attractors still
//      in reach. Real auxin canalization is a richer, more actively
//      cross-suppressing mechanism than this — competing transport
//      channels racing to self-reinforce, not shared-resource depletion —
//      and (1)-(3) above is what's actually implemented, not a full model
//      of it.
// - **Gravitropism vs. hydrotropism** (MIZ1 antagonism) for root growth
//   direction. Real roots pick one or the other, not a blend — gravity
//   normally wins, but a strong local moisture gradient actively suppresses
//   the gravity response so water wins instead. Translated as a hard
//   switch on whether a `RootTip`'s local water pull exceeds a threshold.
// - **Oscillator-based lateral root priming** (periodic priming pulses,
//   not a flat per-tick branch probability) for root branching — a root
//   tip only *considers* branching every `ROOT_PRIME_INTERVAL` ticks, and
//   only actually branches if local conditions (water) support it.
//
// Explicitly simplified rather than modeled in full: cytokinin is not a
// separate diffusing field (the research's "bonus" push-pull signal);
// gravitropic setpoint angle is a fixed constant per branch rather than a
// continuously-regulated flux balance; and canopy light competition is a
// direct light-field read rather than Palubicki et al.'s full shadow-voxel
// propagation — a tip's growth direction leans toward locally brighter
// cells, but branches don't yet cast their own shadows into the field.

const ATTRACTOR_COUNT: usize = 50;
const SCATTER_HALF_WIDTH: f32 = 36.0;
const SCATTER_HEIGHT: f32 = 56.0;
const INFLUENCE_RADIUS: f32 = 16.0;
const KILL_DISTANCE: f32 = 5.0;
const SEGMENT_LENGTH: f32 = 2.0;
const TREE_TICK_INTERVAL: u64 = 20;
const GROWTH_COST: f32 = 4.0;
/// Baseline photosynthesis-like energy trickle, independent of roots —
/// real trees don't stop growing entirely without root water, just slower.
const AMBIENT_GROWTH_ENERGY: f32 = 1.2;
const CHANNEL_GROWTH: f32 = 0.10;
const CHANNEL_DECAY: f32 = 0.22;
const MIN_CHANNEL: f32 = 0.15;
const NEW_BRANCH_CHANNEL: f32 = 0.35;
const BRANCH_CHANNEL_THRESHOLD: f32 = 0.7;
const BRANCH_MIN_ATTRACTORS: usize = 3;
const BRANCH_CHANCE: f32 = 0.12;
const BRANCH_ANGLE_RAD: f32 = 0.6;
const MAX_TIPS_PER_TREE: usize = 14;

const ROOT_SEGMENT_LENGTH: f32 = 2.0;
const ROOT_TICK_INTERVAL: u64 = 25;
const ROOT_SENSE_RADIUS: i32 = 4;
/// Minimum local-water-direction pull strength (fraction of sensed
/// neighbours that are water) before MIZ1-style suppression of gravity
/// kicks in and the root steers toward water instead of straight down.
const MIZ_THRESHOLD: f32 = 0.15;
const ROOT_WATER_ENERGY: f32 = 5.0;
/// Cost debited from the shared energy pool for each step of root growth
/// through soil (not water, which is a net gain via `ROOT_WATER_ENERGY`).
/// Real root growth costs the plant carbon same as shoot growth does — a
/// tree with a fully starved canopy (every tip dead) should not be able to
/// keep drilling roots outward for free, which the original version let it
/// do, contradicting `TreeState::energy`'s own doc that every tip *and
/// root* draws from one competitive pool. Smaller than `GROWTH_COST`: real
/// root tissue is metabolically cheaper to build than woody shoot tissue.
const ROOT_GROWTH_COST: f32 = 1.5;
const ROOT_PRIME_INTERVAL: u32 = 6;
const ROOT_BRANCH_ANGLE_RAD: f32 = 1.1;
const MAX_ROOTS_PER_TREE: usize = 10;
/// Consecutive energy-starved ticks a root tolerates before it stops
/// rescheduling itself and goes dormant, mirroring `MOSS_STALE_LIMIT` for
/// the same reason: once every tree tip has died (attractors exhausted,
/// nothing left generating `AMBIENT_GROWTH_ENERGY`) and there's no water
/// nearby to fund `ROOT_WATER_ENERGY`, a root waiting on `energy >=
/// ROOT_GROWTH_COST` would otherwise wait forever — an amount that will
/// never arrive is not a temporary shortfall. Caught by a test
/// (`a_tree_eventually_stops_growing`) that expected the whole tree to go
/// fully dormant and found one immortal root left checking every tick
/// instead, once root growth started costing energy at all.
const ROOT_STARVATION_LIMIT: u8 = 8;

#[derive(Clone, Copy)]
struct Tip {
    pos: (f32, f32),
    dir: (f32, f32),
    channel: f32,
    alive: bool,
}

#[derive(Clone, Copy)]
struct RootTip {
    pos: (f32, f32),
    dir: (f32, f32),
    ticks_since_prime: u32,
    /// Consecutive ticks spent waiting on `ROOT_GROWTH_COST` energy that
    /// never arrived. Reset on every successful growth step (including a
    /// water absorption, since that's itself a net energy gain, not a
    /// wait).
    starved_ticks: u8,
    /// Whether this tip's own active-site cell (its last-written position)
    /// currently holds this tree's `wood`, as opposed to `Empty`. A root
    /// that just grew through soil writes wood there and this is `true`;
    /// a root that just drank an adjacent water cell absorbs it and
    /// advances into the now-empty space with no wood cell left behind
    /// (see `root_tip_tick`'s own comment on why), so this is `false`.
    /// Exists so `root_tip_tick`'s own-cell validity check (mirroring
    /// `tree_tip_tick`'s and `moss_tick`'s) can distinguish "this root's
    /// last-written cell has actually been disturbed" from "this root's
    /// last-written cell is legitimately empty because it drank there" —
    /// the latter is not evidence of anything happening *to* the root, and
    /// checking `Cell::EMPTY` unconditionally would kill a perfectly
    /// healthy root the tick after it drinks.
    resting_on_wood: bool,
    alive: bool,
}

/// Per-tree state too large to fit in a `Cell`'s spare bits (an attractor
/// list, several growth tips, several root tips) — lives in `World`
/// alongside the schedule, indexed by `ActiveKind::TreeTip`/`RootTip`.
pub struct TreeState {
    attractors: Vec<(f32, f32)>,
    tips: Vec<Tip>,
    roots: Vec<RootTip>,
    /// Shared pool every tip and root draws from/credits to — this is what
    /// makes canalization actually competitive: two simultaneously-growing
    /// tips are fighting over the same energy, not each getting their own.
    energy: f32,
    wood: material::MaterialId,
}

/// Plant a tree seed: a trunk-base cell, a scattered attractor field above
/// it, and the first growth tip and root tip. No-ops if the position isn't
/// empty or the `wood` material isn't loaded.
pub fn plant_tree_seed(world: &mut World, x: i32, y: i32) -> Vec<ActiveSite> {
    let Some(wood) = world.materials.id_of("wood") else {
        return Vec::new();
    };
    if !world.is_empty(x, y) {
        return Vec::new();
    }

    let shades = world.materials.get(wood).palette.len().max(1) as u32;
    let shade = world.rng.below(shades) as u8;
    world.set(x, y, Cell::new(wood, shade));

    let mut attractors = Vec::with_capacity(ATTRACTOR_COUNT);
    for _ in 0..ATTRACTOR_COUNT {
        let ax = x as f32 + (world.rng.below(2001) as f32 / 1000.0 - 1.0) * SCATTER_HALF_WIDTH;
        let ay = y as f32 - (world.rng.below(1001) as f32 / 1000.0) * SCATTER_HEIGHT;
        attractors.push((ax, ay));
    }

    let tip = Tip { pos: (x as f32, y as f32), dir: (0.0, -1.0), channel: 0.5, alive: true };
    let root = RootTip { pos: (x as f32, y as f32), dir: (0.0, 1.0), ticks_since_prime: 0, starved_ticks: 0, resting_on_wood: true, alive: true };
    let state = TreeState { attractors, tips: vec![tip], roots: vec![root], energy: GROWTH_COST, wood };
    let tree_id = world.push_tree(state);

    vec![
        reschedule(x, y, ActiveKind::TreeTip { tree: tree_id, tip: 0 }, world.frame + TREE_TICK_INTERVAL),
        reschedule(x, y, ActiveKind::RootTip { tree: tree_id, root: 0 }, world.frame + ROOT_TICK_INTERVAL),
    ]
}

fn tree_tip_tick(world: &mut World, x: i32, y: i32, tree_id: u32, tip_id: u32) -> Vec<ActiveSite> {
    if !world.tree(tree_id).tips[tip_id as usize].alive {
        return Vec::new();
    }
    // Mirrors moss_tick's own check (see its comment) -- `alive` is set
    // false only by this function's own logic (channel decayed past
    // `MIN_CHANNEL`), never by anything happening *to* the tip. Without
    // this, burning the tree or erasing its trunk with the brush leaves
    // every tip's schedule intact: each tick still accrues
    // `AMBIENT_GROWTH_ENERGY`, still consults its attractors, and still
    // calls `world.set` from a position that is now open air, disconnected
    // from anything -- a burned tree keeps growing forever. `(x, y)` is
    // always this tip's own last-written wood cell (the position the site
    // was rescheduled at after every successful growth step, including the
    // seed cell for a freshly planted tree), so comparing against it
    // directly is exact, not approximate.
    let wood_id = world.tree(tree_id).wood;
    if world.get(x, y).material != wood_id {
        world.tree_mut(tree_id).tips[tip_id as usize].alive = false;
        reclaim_if_tree_is_fully_dead(world, tree_id);
        return Vec::new();
    }
    world.tree_mut(tree_id).energy += AMBIENT_GROWTH_ENERGY;

    let (pos, dir) = {
        let tip = &world.tree(tree_id).tips[tip_id as usize];
        (tip.pos, tip.dir)
    };
    let nearby: Vec<(f32, f32)> = world
        .tree(tree_id)
        .attractors
        .iter()
        .copied()
        .filter(|&a| dist(pos, a) <= INFLUENCE_RADIUS)
        .collect();

    if nearby.is_empty() {
        // A genuine dead end -- nothing left worth growing toward from
        // here -- decays the channel same as before.
        let channel = {
            let tip = &mut world.tree_mut(tree_id).tips[tip_id as usize];
            tip.channel -= CHANNEL_DECAY;
            tip.channel
        };
        if channel <= MIN_CHANNEL {
            world.tree_mut(tree_id).tips[tip_id as usize].alive = false;
            reclaim_if_tree_is_fully_dead(world, tree_id);
            return Vec::new();
        }
        return vec![reschedule(x, y, ActiveKind::TreeTip { tree: tree_id, tip: tip_id }, world.frame + TREE_TICK_INTERVAL)];
    }
    if world.tree(tree_id).energy < GROWTH_COST {
        // A temporary resource shortfall, not a pathing failure -- the
        // channel itself represents transport capacity toward a real
        // target, which doesn't wither just because the rest of the plant
        // is short on energy this tick. Wait without decaying; separated
        // from the `nearby.is_empty()` case above deliberately, since
        // conflating "no attractors" with "no energy" here was measured to
        // make `channel` decay far more often than it could grow (energy
        // waits happen roughly 2-3x more often than successful growth
        // ticks at this tuning), so channel could practically never cross
        // `BRANCH_CHANNEL_THRESHOLD` -- caught by a test that actually
        // checked whether branching ever happened, not just "did it grow."
        return vec![reschedule(x, y, ActiveKind::TreeTip { tree: tree_id, tip: tip_id }, world.frame + TREE_TICK_INTERVAL)];
    }

    let mut avg = (0.0f32, 0.0f32);
    for a in &nearby {
        let d = normalize((a.0 - pos.0, a.1 - pos.1));
        avg.0 += d.0;
        avg.1 += d.1;
    }
    avg = normalize(avg);

    // Gentle phototropic lean: bias toward the brighter of "here" and
    // "just above" rather than Palubicki's full shadow-voxel propagation —
    // see the module doc for why that's the one piece left simplified.
    let light_here = world.field_at(pos.0 as i32, pos.1 as i32).light;
    let light_above = world.field_at(pos.0 as i32, (pos.1 - 4.0) as i32).light;
    let photo = if light_above > light_here { (0.0, -0.25) } else { (0.0, 0.0) };

    let new_dir = normalize((avg.0 * 0.75 + dir.0 * 0.25 + photo.0, avg.1 * 0.75 + dir.1 * 0.25 + photo.1));
    let new_pos = (pos.0 + new_dir.0 * SEGMENT_LENGTH, pos.1 + new_dir.1 * SEGMENT_LENGTH);
    let (wx, wy) = (new_pos.0.round() as i32, new_pos.1.round() as i32);

    let wood = world.tree(tree_id).wood;
    let target = world.get(wx, wy);
    // Blocked by *any* non-empty cell, including wood — this tree's own,
    // or another tree's. `wood` is one shared `MaterialId` across every
    // tree, so comparing against it doesn't distinguish "my own trunk" from
    // "a different tree's" at all; the original version let a tip tunnel
    // straight through any already-grown wood as if it were open air,
    // spending energy on a step that created nothing and could keep
    // advancing (and branching) from a position physically inside another
    // tree's trunk.
    let blocked = !target.is_empty();

    if blocked {
        // Grew into something solid (or the world edge): this tip's
        // channel loses the competition here, same as finding no
        // attractors — it withers rather than forcing through.
        let channel = {
            let tip = &mut world.tree_mut(tree_id).tips[tip_id as usize];
            tip.channel -= CHANNEL_DECAY;
            tip.channel
        };
        if channel <= MIN_CHANNEL {
            world.tree_mut(tree_id).tips[tip_id as usize].alive = false;
            reclaim_if_tree_is_fully_dead(world, tree_id);
            return Vec::new();
        }
        return vec![reschedule(x, y, ActiveKind::TreeTip { tree: tree_id, tip: tip_id }, world.frame + TREE_TICK_INTERVAL)];
    }

    let shades = world.materials.get(wood).palette.len().max(1) as u32;
    let shade = world.rng.below(shades) as u8;
    world.set(wx, wy, Cell::new(wood, shade));
    world.tree_mut(tree_id).energy -= GROWTH_COST;
    world.tree_mut(tree_id).attractors.retain(|&a| dist(new_pos, a) > KILL_DISTANCE);

    let channel = {
        let tip = &mut world.tree_mut(tree_id).tips[tip_id as usize];
        tip.pos = new_pos;
        tip.dir = new_dir;
        tip.channel = (tip.channel + CHANNEL_GROWTH).min(1.0);
        tip.channel
    };

    let mut next = vec![reschedule(wx, wy, ActiveKind::TreeTip { tree: tree_id, tip: tip_id }, world.frame + TREE_TICK_INTERVAL)];

    let can_branch = channel > BRANCH_CHANNEL_THRESHOLD
        && nearby.len() >= BRANCH_MIN_ATTRACTORS
        && world.tree(tree_id).tips.len() < MAX_TIPS_PER_TREE;
    if can_branch && world.rng.chance(BRANCH_CHANCE) {
        let angle = if world.rng.flip() { BRANCH_ANGLE_RAD } else { -BRANCH_ANGLE_RAD };
        let branch = Tip { pos: new_pos, dir: rotate(new_dir, angle), channel: NEW_BRANCH_CHANNEL, alive: true };
        let branch_id = world.tree(tree_id).tips.len() as u32;
        world.tree_mut(tree_id).tips.push(branch);
        // The new channel draws capacity away from the parent's — real
        // auxin canalization is competing sources, not independent ones,
        // and without this a parent tip's channel value never depended on
        // anything but its own history, i.e. no cross-tip interaction at
        // all despite the name. Costing the parent exactly what the new
        // branch starts with keeps the tree's total channel "budget"
        // meaningful rather than manufacturing capacity for free.
        world.tree_mut(tree_id).tips[tip_id as usize].channel -= NEW_BRANCH_CHANNEL;
        next.push(reschedule(wx, wy, ActiveKind::TreeTip { tree: tree_id, tip: branch_id }, world.frame + TREE_TICK_INTERVAL));
    }

    next
}

fn root_tip_tick(world: &mut World, x: i32, y: i32, tree_id: u32, root_id: u32) -> Vec<ActiveSite> {
    if !world.tree(tree_id).roots[root_id as usize].alive {
        return Vec::new();
    }
    // Mirrors tree_tip_tick's own check, with one wrinkle: a root's
    // last-written cell is only *sometimes* wood (see
    // `RootTip::resting_on_wood`'s doc) -- when it drank water there
    // instead of growing through soil, the cell is legitimately `Empty`,
    // and checking for wood unconditionally would kill a perfectly healthy
    // root the tick after it drinks. Only validate when the tip actually
    // expects wood to still be there.
    let wood_id = world.tree(tree_id).wood;
    let resting_on_wood = world.tree(tree_id).roots[root_id as usize].resting_on_wood;
    if resting_on_wood && world.get(x, y).material != wood_id {
        world.tree_mut(tree_id).roots[root_id as usize].alive = false;
        reclaim_if_tree_is_fully_dead(world, tree_id);
        return Vec::new();
    }

    let (pos, dir, mut ticks) = {
        let r = &world.tree(tree_id).roots[root_id as usize];
        (r.pos, r.dir, r.ticks_since_prime)
    };

    // Roots drink from every adjacent water cell before moving, crediting
    // the tree's shared energy pool — the mechanic that actually ties
    // plant growth to the rest of the material physics.
    for (dx, dy) in NEIGHBOURS_4 {
        let (nx, ny) = (pos.0 as i32 + dx, pos.1 as i32 + dy);
        if world.materials.kind(world.get(nx, ny).material) == MaterialKind::Liquid {
            world.set(nx, ny, Cell::EMPTY);
            world.tree_mut(tree_id).energy += ROOT_WATER_ENERGY;
        }
    }

    // Gravitropism vs. hydrotropism: a real antagonism switch (MIZ1
    // suppresses the gravity response under a strong moisture gradient),
    // not a blend of the two pulls.
    let gravity = (0.0f32, 1.0f32);
    let water_pull = strongest_water_pull(world, pos.0 as i32, pos.1 as i32, ROOT_SENSE_RADIUS);
    let base = match water_pull {
        Some((dir, strength)) if strength >= MIZ_THRESHOLD => dir,
        _ => gravity,
    };
    let new_dir = normalize((base.0 * 0.7 + dir.0 * 0.3, base.1 * 0.7 + dir.1 * 0.3));
    let new_pos = (pos.0 + new_dir.0 * ROOT_SEGMENT_LENGTH, pos.1 + new_dir.1 * ROOT_SEGMENT_LENGTH);
    let (wx, wy) = (new_pos.0.round() as i32, new_pos.1.round() as i32);

    // A root pushes through loose/open ground, not solid rock — the same
    // "displaceable substrate" boundary the burrowing-creature research
    // (M18) uses, reused here since it's the same real constraint. Growing
    // into a water pocket is a third case, not a subset of either: it's
    // absorbed on the spot (same as the neighbour-drink check above) and
    // the tip advances into the now-empty space rather than leaving a
    // solid wood cell behind, which would be true growth *into* water
    // rather than the water actually being taken up. Without this case a
    // root approaching any wet ground would simply die at its edge, never
    // reaching the moisture it grew toward in the first place.
    let target_kind = world.materials.kind(world.get(wx, wy).material);
    let wood = world.tree(tree_id).wood;
    // Whether this tick's outcome leaves wood at (wx, wy) or leaves it
    // empty (the water-drink case) -- stored on the tip below so next
    // tick's own-cell validity check knows which of those two states is
    // the *expected*, undisturbed one. See `RootTip::resting_on_wood`.
    let resting_on_wood = target_kind != MaterialKind::Liquid;
    match target_kind {
        MaterialKind::Liquid => {
            world.set(wx, wy, Cell::EMPTY);
            world.tree_mut(tree_id).energy += ROOT_WATER_ENERGY;
            world.tree_mut(tree_id).roots[root_id as usize].starved_ticks = 0;
        }
        MaterialKind::Empty | MaterialKind::Powder => {
            // Growing through soil costs energy from the same shared pool
            // tree tips draw from — real root growth costs the plant
            // carbon too, same as shoot growth, and this is what makes
            // `TreeState::energy` an actually-competitive shared resource
            // rather than something only tips participate in. Not enough
            // yet: wait where it is (this tick's water-drink above still
            // happened and is kept) rather than moving for free.
            if world.tree(tree_id).energy < ROOT_GROWTH_COST {
                let starved = {
                    let r = &mut world.tree_mut(tree_id).roots[root_id as usize];
                    r.starved_ticks += 1;
                    r.starved_ticks
                };
                // An amount that will never arrive (every tip dead, no
                // water nearby to fund the shortfall) is not a temporary
                // wait — see `ROOT_STARVATION_LIMIT`'s doc.
                if starved >= ROOT_STARVATION_LIMIT {
                    world.tree_mut(tree_id).roots[root_id as usize].alive = false;
                    reclaim_if_tree_is_fully_dead(world, tree_id);
                    return Vec::new();
                }
                return vec![reschedule(
                    x,
                    y,
                    ActiveKind::RootTip { tree: tree_id, root: root_id },
                    world.frame + ROOT_TICK_INTERVAL,
                )];
            }
            let shades = world.materials.get(wood).palette.len().max(1) as u32;
            let shade = world.rng.below(shades) as u8;
            world.set(wx, wy, Cell::new(wood, shade));
            world.tree_mut(tree_id).energy -= ROOT_GROWTH_COST;
            world.tree_mut(tree_id).roots[root_id as usize].starved_ticks = 0;
        }
        _ => {
            world.tree_mut(tree_id).roots[root_id as usize].alive = false;
            reclaim_if_tree_is_fully_dead(world, tree_id);
            return Vec::new();
        }
    }

    ticks += 1;
    let mut next = Vec::new();
    let mut primed = false;
    if ticks >= ROOT_PRIME_INTERVAL {
        ticks = 0;
        primed = true;
    }
    {
        let r = &mut world.tree_mut(tree_id).roots[root_id as usize];
        r.pos = new_pos;
        r.dir = new_dir;
        r.ticks_since_prime = ticks;
        r.resting_on_wood = resting_on_wood;
    }
    next.push(reschedule(wx, wy, ActiveKind::RootTip { tree: tree_id, root: root_id }, world.frame + ROOT_TICK_INTERVAL));

    // Oscillator-based priming, not a flat per-tick probability: only
    // *consider* branching periodically, and only follow through if local
    // conditions (water nearby) actually support a new lateral root.
    if primed && is_damp(world, wx, wy, 3) && world.tree(tree_id).roots.len() < MAX_ROOTS_PER_TREE {
        let angle = if world.rng.flip() { ROOT_BRANCH_ANGLE_RAD } else { -ROOT_BRANCH_ANGLE_RAD };
        let branch =
            RootTip { pos: new_pos, dir: rotate(new_dir, angle), ticks_since_prime: 0, starved_ticks: 0, resting_on_wood, alive: true };
        let branch_id = world.tree(tree_id).roots.len() as u32;
        world.tree_mut(tree_id).roots.push(branch);
        next.push(reschedule(wx, wy, ActiveKind::RootTip { tree: tree_id, root: branch_id }, world.frame + ROOT_TICK_INTERVAL));
    }

    next
}

/// Direction and strength of the strongest nearby pull toward water — the
/// fraction of sensed neighbours in that general direction that are water,
/// used both to decide whether MIZ1-style suppression should kick in and
/// which way to steer if so.
fn strongest_water_pull(world: &World, x: i32, y: i32, radius: i32) -> Option<((f32, f32), f32)> {
    let mut sum = (0.0f32, 0.0f32);
    let mut water_count = 0;
    let mut total = 0;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx == 0 && dy == 0 {
                continue;
            }
            total += 1;
            if world.materials.kind(world.get(x + dx, y + dy).material) == MaterialKind::Liquid {
                water_count += 1;
                let d = normalize((dx as f32, dy as f32));
                sum.0 += d.0;
                sum.1 += d.1;
            }
        }
    }
    if water_count == 0 {
        return None;
    }
    Some((normalize(sum), water_count as f32 / total as f32))
}

fn reschedule(x: i32, y: i32, kind: ActiveKind, next_frame: u64) -> ActiveSite {
    ActiveSite { x, y, kind, next_frame }
}

/// Once every tip and root of a tree has died, its `attractors` list (up to
/// `ATTRACTOR_COUNT` points, by far the largest part of `TreeState`) serves
/// no further purpose — nothing left will ever consult it. `TreeState`
/// itself still never shrinks (`World::push_tree`'s own doc explains why:
/// `ActiveKind::TreeTip`/`RootTip` index into it by position, and
/// reassigning a slot would invalidate a live `ActiveSite` — the real fix
/// is generational indices with a free list, `pixel-physics-issues.md`
/// issue #8's own "Direction"), but freeing the one large allocation that
/// is genuinely dead is a real, low-risk partial mitigation cheap enough to
/// do inline at every death site, rather than waiting on that larger
/// rewrite. Checked (not assumed) at every call site rather than only when
/// the *last* tip or root dies, since which one dies last isn't tracked —
/// this runs a few extra times over a tree's life, each one O(tips + roots)
/// and cheap, in exchange for not needing to track that separately.
fn reclaim_if_tree_is_fully_dead(world: &mut World, tree_id: u32) {
    let fully_dead = {
        let tree = world.tree(tree_id);
        tree.tips.iter().all(|t| !t.alive) && tree.roots.iter().all(|r| !r.alive)
    };
    if fully_dead {
        world.tree_mut(tree_id).attractors = Vec::new(); // drops the old allocation; an empty Vec has none of its own
    }
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn normalize(v: (f32, f32)) -> (f32, f32) {
    let len = (v.0 * v.0 + v.1 * v.1).sqrt();
    if len < 1e-6 {
        (0.0, 0.0)
    } else {
        (v.0 / len, v.1 / len)
    }
}

fn rotate(v: (f32, f32), radians: f32) -> (f32, f32) {
    let (s, c) = radians.sin_cos();
    (v.0 * c - v.1 * s, v.0 * s + v.1 * c)
}

/// Dispatch one due active site to its growth function. Called from
/// `scheduler::step` for every `ActiveKind` except `StructuralCheck` and
/// `Creature`, which `scheduler::step` routes to `structural::tick`/
/// `creature::tick` instead -- the match here still has to name both
/// variants to stay exhaustive.
pub fn tick(world: &mut World, site: &ActiveSite) -> Vec<ActiveSite> {
    match site.kind {
        ActiveKind::Moss { stale_ticks } => moss_tick(world, site.x, site.y, stale_ticks),
        ActiveKind::TreeTip { tree, tip } => tree_tip_tick(world, site.x, site.y, tree, tip),
        ActiveKind::RootTip { tree, root } => root_tip_tick(world, site.x, site.y, tree, root),
        ActiveKind::StructuralCheck => unreachable!("scheduler::step routes StructuralCheck to structural::tick"),
        ActiveKind::Creature { .. } => unreachable!("scheduler::step routes Creature to creature::tick"),
    }
}

impl World {
    /// Plant a moss seed at `(x, y)` if it's empty, and schedule its first
    /// growth check. A no-op (returns nothing to schedule) if the position
    /// isn't empty or the `moss` material isn't loaded.
    pub fn plant_moss_seed(&mut self, x: i32, y: i32) {
        let Some(moss_id) = self.materials.id_of("moss") else {
            return;
        };
        if !self.is_empty(x, y) {
            return;
        }
        let shades = self.materials.get(moss_id).palette.len().max(1) as u32;
        let shade = self.rng.below(shades) as u8;
        self.set(x, y, Cell::new(moss_id, shade));
        let site = reschedule(x, y, ActiveKind::Moss { stale_ticks: 0 }, self.frame + MOSS_TICK_INTERVAL);
        self.schedule_active_site(site);
    }

    /// Plant a tree seed at `(x, y)` — see `plant_tree_seed` above.
    pub fn plant_tree(&mut self, x: i32, y: i32) {
        for site in plant_tree_seed(self, x, y) {
            self.schedule_active_site(site);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::update;

    fn test_world() -> World {
        World::new(Rect::new(0, 0, 199, 199))
    }

    fn run(w: &mut World, frames: usize) {
        for _ in 0..frames {
            update::step(w);
            w.step_active_sites();
        }
    }

    fn count(w: &World, id: material::MaterialId) -> usize {
        let b = w.bounds().unwrap();
        let mut n = 0;
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                if w.get(x, y).material == id {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn planting_on_occupied_ground_is_a_no_op() {
        let mut w = test_world();
        w.set(50, 50, Cell::new(material::STONE, 0));
        w.plant_tree(50, 50);
        w.plant_moss_seed(50, 50);
        assert_eq!(w.get(50, 50).material, material::STONE, "planting overwrote existing material");
        assert_eq!(w.active_site_count(), 0, "a no-op plant should not schedule anything");
    }

    #[test]
    fn moss_spreads_over_damp_stone_and_not_over_dry() {
        let mut w = test_world();
        // Two separate stone platforms, identical except one has a shallow
        // puddle resting directly on it. Both get a moss seed at their own
        // edge. The puddle is placed directly rather than poured from
        // above and left to find its own level: a narrow platform with
        // open edges lets water drain off the sides and fall away
        // entirely, leaving nothing actually damp by the time growth
        // starts checking.
        for x in 9..31 {
            w.set(x, 50, Cell::new(material::STONE, 0));
        }
        w.set(9, 49, Cell::new(material::STONE, 0)); // walls -- keep the
        w.set(30, 49, Cell::new(material::STONE, 0)); // puddle from draining
        for x in 12..18 {
            w.set(x, 49, Cell::new(material::WATER, 0));
        }
        for x in 60..80 {
            w.set(x, 50, Cell::new(material::STONE, 0)); // far from any water
        }
        w.plant_moss_seed(20, 49);
        w.plant_moss_seed(70, 49);
        run(&mut w, 4000);

        let moss = w.materials.id_of("moss").unwrap();
        let damp_moss = (10..30).filter(|&x| (44..51).any(|y| w.get(x, y).material == moss)).count();
        let dry_moss = (60..80).filter(|&x| (44..51).any(|y| w.get(x, y).material == moss)).count();
        // A relative comparison, not "dry must stay at exactly 1" -- the
        // dry side's small nonzero chance (real moss can survive brief dry
        // spells) means an occasional stray spread there is expected, not
        // a bug; what must hold is that damp spreads meaningfully more.
        assert!(damp_moss > 3, "moss did not spread over damp stone: {damp_moss} cells");
        assert!(
            damp_moss > dry_moss * 3,
            "moss should spread much more over damp stone ({damp_moss}) than dry ({dry_moss})"
        );
    }

    #[test]
    fn two_trees_grown_from_the_same_seed_differ() {
        // Two separate `World`s planting at the *same* position would draw
        // identical attractor scatters and grow identically -- `World::new`
        // always starts `Rng::default()` from the same fixed seed, so two
        // structurally-identical runs are, correctly, reproducible rather
        // than randomly different. That's a property of the RNG, not a
        // bug (see `rng.rs`'s own module doc on why determinism was never
        // required but reproducibility is free when nothing disturbs it).
        // The plan's actual claim -- "two trees grown from the same seed
        // differ" -- means within one running session, where the shared
        // `world.rng` has already advanced by the time a second tree is
        // planted. Plant both in the same world, far enough apart not to
        // compete for the same attractors, and compare shapes normalized
        // to each tree's own seed position.
        let mut w = test_world();
        w.plant_tree(50, 100);
        w.plant_tree(150, 100);
        run(&mut w, 3000);

        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let footprint_relative_to = |w: &World, origin: (i32, i32)| -> Vec<(i32, i32)> {
            let bounds = w.bounds().unwrap();
            let mut cells = Vec::new();
            for y in bounds.min_y..=bounds.max_y {
                for x in bounds.min_x..=bounds.max_x {
                    if w.get(x, y).material == wood && (x - origin.0).abs() < 40 {
                        cells.push((x - origin.0, y - origin.1));
                    }
                }
            }
            cells
        };
        assert_ne!(
            footprint_relative_to(&w, (50, 100)),
            footprint_relative_to(&w, (150, 100)),
            "two trees grown from the same seed position produced identical shapes"
        );
    }

    #[test]
    fn a_settled_world_with_a_growing_tree_still_sleeps_between_growth_ticks() {
        let mut w = test_world();
        w.plant_tree(50, 100);
        // A handful of frames is enough for the CA sweep itself to settle
        // (a single static wood cell has nothing to move); the tree keeps
        // growing on its own, much slower schedule.
        run(&mut w, 5);
        assert_eq!(
            w.active_chunk_count(),
            0,
            "a static plant cell should not keep the CA sweep's chunks awake"
        );
        assert!(w.active_site_count() > 0, "the tree should still have pending growth ticks");
    }

    #[test]
    fn a_tree_eventually_stops_growing() {
        let mut w = test_world();
        w.plant_tree(50, 100);
        run(&mut w, 6000);
        assert_eq!(
            w.active_site_count(),
            0,
            "a tree should eventually exhaust its attractors/energy and stop scheduling growth"
        );
        let wood = w.materials.id_of("wood").unwrap();
        assert!(count(&w, wood) > 1, "the tree should have grown at least beyond its single seed cell");
    }

    #[test]
    fn roots_consume_adjacent_water() {
        let mut w = test_world();
        w.plant_tree(50, 100);
        // A walled, floored tank directly below the seed -- water is a
        // liquid and falls/drains under gravity like anything else, so an
        // unconfined puddle sinks or spreads away before a root (which
        // only moves once every `ROOT_TICK_INTERVAL` frames) ever reaches
        // it. The tank keeps it exactly where the root's initial
        // straight-down growth will hit it.
        for x in 44..56 {
            w.set(x, 110, Cell::new(material::STONE, 0)); // floor
        }
        for y in 101..111 {
            w.set(44, y, Cell::new(material::STONE, 0)); // walls
            w.set(55, y, Cell::new(material::STONE, 0));
        }
        for x in 46..54 {
            for y in 102..109 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        let water_before = count(&w, material::WATER);
        run(&mut w, 500);
        let water_after = count(&w, material::WATER);
        assert!(water_after < water_before, "roots never consumed any adjacent water");
    }

    #[test]
    fn a_planted_tree_is_flammable_and_burns_to_ash() {
        let mut w = test_world();
        w.plant_tree(50, 100);
        run(&mut w, 300); // let it grow a little first
        let wood = w.materials.id_of("wood").unwrap();
        assert!(count(&w, wood) > 0, "test setup should have some wood to ignite");
        w.ignite_circle(50, 100, 5);
        run(&mut w, 4000);
        let ash = w.materials.id_of("ash").unwrap();
        assert!(count(&w, ash) > 0, "burned wood should have left ash behind");
    }

    #[test]
    fn roots_steer_toward_off_axis_water_via_hydrotropism() {
        // A regression an independent review flagged: the previous version
        // of this test placed water directly beneath the seed, on the
        // root's default straight-down gravitropic path -- it would have
        // passed identically even with the whole MIZ1 hydrotropism switch
        // deleted, since gravity alone reaches straight-down water. This
        // one places water only off to one side, where gravity alone would
        // never lead, so a measurable horizontal deviation can only be
        // explained by the hydrotropism steering actually firing.
        let mut w = test_world();
        w.plant_tree(50, 100);
        // A walled, floored tank whose *open interior* the root falls
        // straight down into (walls at x=47/63 don't block x=50, which
        // sits between them), but whose water is offset to the right
        // (x=52..60) rather than directly below the seed -- close enough
        // to be sensed (within `ROOT_SENSE_RADIUS`, 4 cells of x=50) as
        // soon as the root's descent reaches the water's depth, without
        // gravity alone ever carrying it there.
        for x in 47..63 {
            w.set(x, 115, Cell::new(material::STONE, 0)); // floor
        }
        for y in 101..115 {
            w.set(47, y, Cell::new(material::STONE, 0)); // walls
            w.set(62, y, Cell::new(material::STONE, 0));
        }
        for x in 52..60 {
            for y in 102..112 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        run(&mut w, 1500);

        // Tree id 0: the first (only) tree planted into a fresh `World`.
        let root_x_positions: Vec<f32> = w.tree(0).roots.iter().map(|r| r.pos.0).collect();
        let max_rightward_reach = root_x_positions.iter().cloned().fold(f32::MIN, f32::max);
        assert!(
            max_rightward_reach > 51.0,
            "no root deviated toward the off-axis water pocket (furthest reach: {max_rightward_reach}, seed at x=50)"
        );
    }

    #[test]
    fn a_tree_can_produce_multiple_simultaneous_tips_via_branching() {
        // Direct internal-state check, not a visual proxy -- `tests` is a
        // child module of `plant`, so `TreeState`'s private fields are
        // visible here despite not being `pub`. A dense, wide attractor
        // field and a long run give branching (gated on `channel >
        // BRANCH_CHANNEL_THRESHOLD` and `nearby.len() >=
        // BRANCH_MIN_ATTRACTORS`, see `tree_tip_tick`) every realistic
        // chance to fire; if a future change silently broke it (an
        // off-by-one in the threshold, the attractor-count gate never
        // satisfiable), this is what would actually catch that.
        let mut w = test_world();
        w.plant_tree(100, 150);
        run(&mut w, 6000);

        // Tree id 0: the first (only) tree planted into a fresh `World`.
        let tip_count = w.tree(0).tips.len();
        assert!(
            tip_count > 1,
            "a tree grown to completion with abundant attractors never branched (ended with {tip_count} tip(s))"
        );
    }

    #[test]
    fn an_orphaned_tree_tip_stops_growing_instead_of_extending_wood_from_open_air() {
        // Regression (pixel-physics-issues.md #9): tree_tip_tick checked
        // only its own `alive` flag, which `alive` false is set only by the
        // tip's own logic (channel decayed past MIN_CHANNEL) -- never by
        // anything happening *to* it. Burn the trunk, or erase it with the
        // brush, and every tip kept its schedule: still accruing energy,
        // still consulting attractors, still writing wood from a position
        // now disconnected from anything. Direct internal-state
        // construction, same as `a_tree_can_produce_multiple_simultaneous_
        // tips_via_branching` above -- `tests` is a child module of `plant`.
        let mut w = test_world();
        let wood_id = w.materials.id_of("wood").unwrap();
        w.set(50, 50, Cell::new(wood_id, 0));
        let tip = Tip { pos: (50.0, 50.0), dir: (0.0, -1.0), channel: 0.5, alive: true };
        let state = TreeState { attractors: Vec::new(), tips: vec![tip], roots: Vec::new(), energy: 1000.0, wood: wood_id };
        let tree_id = w.push_tree(state);

        // Erase the trunk out from under the tip -- standing in for either
        // fire consuming it or the brush erasing it; the effect on the tip
        // is identical either way, since both just leave the cell no
        // longer holding this tree's wood.
        w.set(50, 50, Cell::EMPTY);

        let produced = tree_tip_tick(&mut w, 50, 50, tree_id, 0);

        assert!(produced.is_empty(), "an orphaned tip should not reschedule itself");
        assert!(!w.tree(tree_id).tips[0].alive, "an orphaned tip should mark itself dead, not keep ticking forever");
        assert_eq!(w.get(50, 50).material, material::EMPTY, "the dead tip must not have written new wood from open air");
    }

    #[test]
    fn an_orphaned_root_tip_stops_growing_but_a_root_resting_in_drunk_water_is_unaffected() {
        // Same regression as the tree-tip test above, applied to roots --
        // with the added wrinkle roots have and tree tips don't: a root's
        // own last-written cell is only *sometimes* wood
        // (`RootTip::resting_on_wood`). This test checks both halves: an
        // orphaned wood-resting root actually dies, and a root that simply
        // drank water at its current position (a legitimately `Empty`
        // cell, not a disturbance) is correctly left alone.
        let mut w = test_world();
        let wood_id = w.materials.id_of("wood").unwrap();

        w.set(50, 50, Cell::new(wood_id, 0));
        let wood_root = RootTip { pos: (50.0, 50.0), dir: (0.0, 1.0), ticks_since_prime: 0, starved_ticks: 0, resting_on_wood: true, alive: true };
        // (60, 50) stays Empty -- standing in for a root that just drank an
        // adjacent water cell and advanced into the now-empty space.
        let drunk_root = RootTip { pos: (60.0, 50.0), dir: (0.0, 1.0), ticks_since_prime: 0, starved_ticks: 0, resting_on_wood: false, alive: true };
        let state = TreeState {
            attractors: Vec::new(),
            tips: Vec::new(),
            roots: vec![wood_root, drunk_root],
            energy: 1000.0,
            wood: wood_id,
        };
        let tree_id = w.push_tree(state);

        // Erase the wood-resting root's trunk -- (60, 50) is left exactly
        // as it already was (Empty), matching the drink-then-advance case.
        w.set(50, 50, Cell::EMPTY);

        let wood_root_result = root_tip_tick(&mut w, 50, 50, tree_id, 0);
        assert!(wood_root_result.is_empty(), "an orphaned wood-resting root should not reschedule itself");
        assert!(!w.tree(tree_id).roots[0].alive, "an orphaned wood-resting root should mark itself dead");

        // The drunk-water root must be judged on its own merits (whether it
        // can find somewhere to grow from (60, 50)), not killed outright
        // just because its own cell reads as Empty -- that is its expected,
        // undisturbed state.
        let before_alive = w.tree(tree_id).roots[1].alive;
        let _ = root_tip_tick(&mut w, 60, 50, tree_id, 1);
        assert_eq!(
            w.tree(tree_id).roots[1].alive, before_alive,
            "a root resting in a cell it legitimately left empty by drinking must not be killed by the orphan check alone"
        );
    }

    #[test]
    fn a_fully_dead_trees_attractors_are_reclaimed_but_not_a_partially_dead_ones() {
        // Interim mitigation for pixel-physics-issues.md issue #8:
        // TreeState never shrinks (tips/roots index into it by position,
        // and reassigning a slot would invalidate a live ActiveSite -- the
        // real fix is generational indices, not attempted here), but its
        // `attractors` list (up to ATTRACTOR_COUNT points, by far the
        // largest part of TreeState) can be freed the moment every tip and
        // root has died, since nothing will ever consult it again.
        let mut w = test_world();
        let wood_id = w.materials.id_of("wood").unwrap();
        let attractors = vec![(0.0, 0.0); ATTRACTOR_COUNT];
        let alive_tip = Tip { pos: (0.0, 0.0), dir: (0.0, -1.0), channel: 0.5, alive: true };
        let dead_tip = Tip { pos: (0.0, 0.0), dir: (0.0, -1.0), channel: 0.0, alive: false };
        let dead_root =
            RootTip { pos: (0.0, 0.0), dir: (0.0, 1.0), ticks_since_prime: 0, starved_ticks: 0, resting_on_wood: true, alive: false };
        let state = TreeState { attractors, tips: vec![alive_tip, dead_tip], roots: vec![dead_root], energy: 0.0, wood: wood_id };
        let tree_id = w.push_tree(state);

        reclaim_if_tree_is_fully_dead(&mut w, tree_id);
        assert!(!w.tree(tree_id).attractors.is_empty(), "a tree with a still-living tip should not have its attractors reclaimed yet");

        w.tree_mut(tree_id).tips[0].alive = false; // the last living tip dies
        reclaim_if_tree_is_fully_dead(&mut w, tree_id);
        assert!(w.tree(tree_id).attractors.is_empty(), "a fully dead tree's attractors should have been reclaimed");
    }
}
