//! **Reachability probe for `Reports/creature-genome-flexibility-2026-09-02.md`
//! §13's articulated-body proposal.** Not a shipped species and not a body
//! plan -- a measurement-only harness that asks one question `creature_scale
//! mode=walk` cannot, because the engine has no multi-part `BodyPlan` to
//! measure yet: **is a rigid footprint's blocked-move rate driven by its
//! width, its height, or its cell count?**
//!
//! §13c's corrected claim is that an articulated body's mobility is set by
//! its *leading part alone*, because trailing parts move into ground the
//! part ahead already proved passable. That claim is about a single rigid
//! footprint's own blocked rate -- exactly what a standalone `Rigid` body
//! already measures, with no new engine code. This harness holds cell count
//! fixed at 2 and varies only the footprint's shape (a 2-wide, 1-tall
//! domino against a 1-wide, 2-tall one), then adds a 3-wide, 1-tall strip,
//! to separate width from height from cell count in the existing
//! `creature_scale` result set (`ant`=Chain(2), `beetle`=Rigid 2x2,
//! `ant_block`=Rigid 3x3, `ant_wide`=Rigid 5x2).
//!
//! Every economic field (tick_interval, costs, brain, diet) is copied
//! byte-for-byte from `ant_block`'s authored `CreatureDef` and overridden
//! only in `body`, the same isolation `creature_scale.rs`'s control arm and
//! `creature-appearance-design.md`'s arms C/D/E use -- so whatever moves is
//! the footprint and nothing else.
//!
//! ```text
//! cargo run --release --example creature_body_probe -- shape=domino_h seed=7
//! ```

use pixel_physics::app::{HEIGHT, WIDTH};
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::organism::BodyPlan;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{creature, parallel};

fn build(seed: u64, preset: &str) -> World {
    let (w, h) = (WIDTH as i32, HEIGHT as i32);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    world.seed = seed;
    let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("worldgen presets unavailable: {e}");
    }
    let params = presets.get(preset).unwrap_or_else(|| panic!("no worldgen preset {preset:?}"));
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed });
    world
}

/// Named footprints, as `Rigid` offsets from the head (authored facing
/// east; `(0,0)` is implicit). Each holds cell count or width fixed against
/// its neighbour in the list so a single question is isolated per pair.
fn shape(name: &str) -> BodyPlan {
    match name {
        // 2 cells, 2 wide x 1 tall -- same cell count as `ant`'s Chain(2),
        // rigid instead of following. Isolates "rigid at all" from "wide".
        "domino_h" => BodyPlan::Rigid(vec![(-1, 0)]),
        // 2 cells, 1 wide x 2 tall -- same cell count and width as a single
        // column, height doubled. Isolates height from width.
        "domino_v" => BodyPlan::Rigid(vec![(0, -1)]),
        // 3 cells, 3 wide x 1 tall -- continues the width-only progression
        // domino_h starts, still at minimum height.
        "strip3" => BodyPlan::Rigid(vec![(-1, 0), (-2, 0)]),
        // 4 cells, 4 wide x 1 tall.
        "strip4" => BodyPlan::Rigid(vec![(-1, 0), (-2, 0), (-3, 0)]),
        // 14 cells: a 2x2 head (narrow, leading) then a 3x3 abdomen
        // (wide, trailing) -- the monolithic (non-articulated) silhouette
        // of §13c's "small head, big abdomen" case, matching
        // `creature_candidate_render.rs`'s `forward_taper` exactly. Moved
        // as one rigid piece, so this measures the no-following-benefit
        // floor: does a narrow leading section buy anything for a body
        // that cannot decouple its parts?
        "forward_taper" => BodyPlan::Rigid(vec![
            (-1, 0), (0, -1), (-1, -1), // head, 2x2
            (-2, 0),                    // waist
            (-3, 0), (-3, -1), (-3, -2), (-4, 0), (-4, -1), (-4, -2), (-5, 0), (-5, -1), (-5, -2), // abdomen, 3x3
        ]),
        // 14 cells: the mirror -- a 3x3 head (wide, leading) then a 2x2
        // tail (narrow, trailing), matching `creature_candidate_render.rs`'s
        // `backward_taper`.
        "backward_taper" => BodyPlan::Rigid(vec![
            (-1, 0), (-1, -1), (-1, -2), (-2, 0), (-2, -1), (-2, -2), (0, -1), (0, -2), // head, 3x3
            (-3, 0),                                                                   // waist
            (-4, 0), (-4, -1), (-5, 0), (-5, -1),                                      // tail, 2x2
        ]),
        other => panic!("unknown shape {other}; expected domino_h, domino_v, strip3, strip4, forward_taper or backward_taper"),
    }
}

fn main() {
    let mut shape_name = "domino_h".to_string();
    let mut seed = 7u64;
    let mut preset = "rolling".to_string();
    let mut count = 24i32;
    let mut frames = 4000u64;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "shape" => shape_name = v.to_string(),
            "seed" => seed = v.parse().unwrap_or(seed),
            "preset" => preset = v.to_string(),
            "count" => count = v.parse().unwrap_or(count),
            "frames" => frames = v.parse().unwrap_or(frames),
            _ => {}
        }
    }
    println!("creature_body_probe: shape={shape_name} seed={seed} preset={preset} count={count} frames={frames}");

    let mut world = build(seed, &preset);

    // Base every field except `body` on `ant_block`'s authored def, so the
    // only variable across shapes (and against `creature_scale`'s own
    // ant/beetle/ant_block/ant_wide runs, which share these same economics)
    // is the footprint.
    let base_id = world.species.id_of("ant_block").expect("ant_block species");
    let mut def = world.species.get(base_id).creature.clone().expect("ant_block is a creature");
    def.body = shape(&shape_name);
    // Keep the species name "ant_block" so `plant_creature_seed` finds a
    // matching material -- this is an override of the running world's
    // registry, not a new species, and matches `creature_scale.rs`'s own
    // control-arm technique (`SpeciesRegistry::set_creature`).
    world.species.set_creature(base_id, def);

    let cols: Vec<i32> = (0..WIDTH as i32).filter(|&x| creature::colony_ant_site(&world, x, 0).is_some()).collect();
    assert!(cols.len() >= count as usize * 2, "only {} viable columns", cols.len());
    let mut placed = 0;
    for i in 0..count {
        let x = cols[(i as usize * cols.len()) / count as usize];
        let Some(sy) = creature::colony_ant_site(&world, x, 0) else { continue };
        if let Some(site) = creature::plant_creature_seed(&mut world, x, sy - 1, "ant_block") {
            world.schedule_active_site(site);
            placed += 1;
        }
    }
    for _ in 0..frames {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
    }
    let s = world.creature_stats;
    let attempts = s.moves + s.moves_blocked;
    let blocked = if attempts == 0 { f64::NAN } else { s.moves_blocked as f64 / attempts as f64 };
    println!(
        "  shape={shape_name} placed={placed} alive={} ticks={} moves={} blocked={} => blocked {:.1}%  falls={} digs={}",
        world.live_creature_count(),
        s.ticks,
        s.moves,
        s.moves_blocked,
        blocked * 100.0,
        s.falls,
        s.digs,
    );
}
