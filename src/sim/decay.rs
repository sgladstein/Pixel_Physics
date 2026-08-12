//! M16 extension: ash decays into soil over time, moisture-gated, and a
//! freshly-formed soil cell gets one chance to reseed plant growth above
//! it — architecture report §5f/§5e, closing M16's own "a forest burns and
//! regrows" verify criterion (only the burning half existed before this).
//!
//! Dispatched from `scheduler::step` the same way M17's structural checks
//! and M18's creatures are, via `ActiveKind::Decay`. Deliberately scheduled
//! *reactively*, not for every ash cell that could ever exist: only
//! `fire::tick_burn`'s own burnout path — the moment a burning cell
//! actually turns to ash — schedules a decay site for it, matching the
//! architecture report's own "cheap: one material, one slow transformation"
//! framing rather than a fully general decay system. Ash painted directly
//! by the brush, or loaded from a save that predates this mechanic, simply
//! never decays — a documented simplification, not an oversight.

use super::cell::Cell;
use super::scheduler::{ActiveKind, ActiveSite};
use super::world::World;

/// Frames between an ash cell's decay checks. Slow, matching the report's
/// own "ash → soil, slowly" — this is weathering, not growth, and should
/// read as much less frequent than a moss or tree tick. `pub(crate)`:
/// `fire::tick_burn` needs it too, to schedule a burnout's first check.
pub(crate) const DECAY_TICK_INTERVAL: u64 = 200;
/// Per-check decay chance once the ash reads as damp (`DECAY_MOISTURE_
/// THRESHOLD`). Untuned against anything real, same as every other
/// probability on this channel.
const DECAY_CHANCE_DAMP: f32 = 0.05;
/// Per-check decay chance when dry — small but nonzero, the same
/// "poikilohydric plants survive brief dry spells" reasoning `plant.rs`'s
/// `MOSS_DRY_CHANCE` uses: real weathering does not stop completely just
/// because it hasn't rained recently, it only slows down a great deal.
const DECAY_CHANCE_DRY: f32 = 0.002;
/// Matches `plant.rs`'s own `DAMP_MOISTURE_THRESHOLD` — the same "how wet
/// counts as wet" reading, kept as its own constant rather than shared
/// since the two channels' thresholds are free to diverge later and
/// shouldn't be coupled just because they happen to start equal.
const DECAY_MOISTURE_THRESHOLD: f32 = 0.3;
/// Chance a newly-formed soil cell reseeds plant growth in the empty cell
/// directly above it, checked once at the moment of decay rather than
/// scheduled to keep trying — succession happens, but not on every patch of
/// soil forever. A documented simplification: a soil cell that misses this
/// one roll stays bare soil for good, it doesn't get a second chance later.
const RESEED_CHANCE: f32 = 0.15;

/// Dispatch a due `ActiveKind::Decay` site. `scheduler::step` never routes
/// any other `ActiveKind` here.
pub fn tick(world: &mut World, site: &ActiveSite) -> Vec<ActiveSite> {
    debug_assert!(matches!(site.kind, ActiveKind::Decay), "scheduler::step only routes ActiveKind::Decay here");
    let (x, y) = (site.x, site.y);

    let Some(ash_id) = world.materials.id_of("ash") else {
        return Vec::new();
    };
    // The cell may have burned into something else, been erased, or been
    // buried and dug back out as something else entirely by the time this
    // check comes due -- nothing to decay.
    if world.get(x, y).material != ash_id {
        return Vec::new();
    }

    let damp = world.field_at(x, y).moisture > DECAY_MOISTURE_THRESHOLD;
    let chance = if damp { DECAY_CHANCE_DAMP } else { DECAY_CHANCE_DRY };
    if !world.rng.chance(chance) {
        return vec![ActiveSite { x, y, kind: ActiveKind::Decay, next_frame: world.frame + DECAY_TICK_INTERVAL }];
    }

    let Some(soil_id) = world.materials.id_of("soil") else {
        return Vec::new(); // soil isn't loaded -- nothing to decay into
    };
    let shades = world.materials.get(soil_id).palette.len().max(1) as u32;
    let shade = world.rng.below(shades) as u8;
    world.set(x, y, Cell::new(soil_id, shade));

    // Reseed roll: only if there's actually room to grow into, and only
    // ever this one chance -- see RESEED_CHANCE's own doc.
    if world.is_empty(x, y - 1) && world.rng.chance(RESEED_CHANCE) {
        if world.rng.flip() {
            world.plant_moss_seed(x, y - 1);
        } else {
            world.plant_tree(x, y - 1);
        }
    }

    Vec::new() // decayed -- nothing further to schedule for this cell
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::field;
    use crate::sim::material;
    use crate::sim::scheduler;

    fn test_world() -> World {
        World::new(Rect::new(0, 0, 199, 199))
    }

    fn run(w: &mut World, frames: usize) {
        for _ in 0..frames {
            w.begin_step();
            field::step(w);
            scheduler::step(w);
            w.end_step();
        }
    }

    #[test]
    fn damp_ash_decays_into_soil_but_dry_ash_does_not() {
        let mut w = test_world();
        let ash = material::ASH;
        let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");

        // Damp platform: a contained puddle sitting directly on the ash,
        // same "walled trough" shape `plant.rs`'s own damp-vs-dry moss test
        // uses, for the same reason (an open puddle drains/spreads away
        // before the field even registers it as a source).
        for x in 10..20 {
            w.set(x, 100, Cell::new(ash, 0));
        }
        w.set(9, 99, Cell::new(material::STONE, 0));
        w.set(20, 99, Cell::new(material::STONE, 0));
        for x in 12..18 {
            w.set(x, 99, Cell::new(material::WATER, 0));
        }
        w.schedule_active_site(ActiveSite { x: 14, y: 100, kind: ActiveKind::Decay, next_frame: DECAY_TICK_INTERVAL });

        // Dry platform, far away, no water anywhere near it.
        for x in 60..70 {
            w.set(x, 100, Cell::new(ash, 0));
        }
        w.schedule_active_site(ActiveSite { x: 64, y: 100, kind: ActiveKind::Decay, next_frame: DECAY_TICK_INTERVAL });

        run(&mut w, 20_000);

        assert_eq!(w.get(14, 100).material, soil, "damp ash never decayed into soil");
        assert_eq!(w.get(64, 100).material, ash, "dry ash decayed as readily as damp ash");
    }

    #[test]
    fn decayed_soil_sometimes_reseeds_plant_growth_above_it() {
        // Architecture §5e: the "regrows" half of M16's own verify
        // criterion. Ash directly under a puddle decays (as the test
        // above proves) but can never reseed -- `is_empty(x, y - 1)` reads
        // the puddle itself, not open air, so `RESEED_CHANCE` never even
        // gets rolled there. This places a strip of ash just past the
        // puddle's edge instead: close enough for the moisture field to
        // diffuse over and still read damp, but with open air directly
        // above it -- reseeding actually has room to happen. Many ash
        // cells, not one, so a single unlucky RESEED_CHANCE roll on any
        // given cell doesn't sink the test.
        let mut w = test_world();
        let ash = material::ASH;
        let moss = w.materials.id_of("moss").expect("moss is a compiled-in material");
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");

        // Several small, separately-walled puddles along one long floor,
        // not one -- moisture only diffuses a handful of field cells past
        // its source (matching the light channel's own steep "diffuse
        // fast, decay hard" falloff), so any single puddle's edge only
        // gives a few damp-and-open cells to roll `RESEED_CHANCE` against.
        // Spreading several puddles along the strip multiplies the number
        // of independent rolls so one unlucky stretch doesn't sink the test.
        let mut ash_x = Vec::new();
        for &puddle_start in &[10, 40, 70, 100] {
            w.set(puddle_start - 1, 99, Cell::new(material::STONE, 0));
            w.set(puddle_start + 6, 99, Cell::new(material::STONE, 0));
            for x in puddle_start..puddle_start + 6 {
                w.set(x, 99, Cell::new(material::WATER, 0));
            }
            // Open-air ash on both sides of each puddle's walls.
            for x in (puddle_start - 5)..puddle_start - 1 {
                ash_x.push(x);
            }
            for x in (puddle_start + 7)..(puddle_start + 12) {
                ash_x.push(x);
            }
        }
        for &x in &ash_x {
            w.set(x, 100, Cell::new(ash, 0));
            w.schedule_active_site(ActiveSite { x, y: 100, kind: ActiveKind::Decay, next_frame: DECAY_TICK_INTERVAL });
        }

        run(&mut w, 20_000);

        let reseeded = ash_x.iter().any(|&x| {
            let m = w.get(x, 99).material;
            m == moss || m == wood
        });
        assert!(reseeded, "no ash cell near any puddle's edge reseeded plant growth in twenty thousand frames");
    }

    #[test]
    fn a_burned_out_cell_schedules_its_own_decay_check() {
        // Integration-level: fire.rs's burnout path is what actually
        // schedules decay in real play, not a hand-built ActiveSite like
        // the test above. Oil burns into ash (see oil.ron) -- ignite it,
        // let it burn out, and confirm a Decay site now exists rather than
        // the ash sitting inert forever.
        let mut w = test_world();
        let mut burning = Cell::new(material::OIL, 0);
        burning.ignite(3);
        w.set(30, 30, burning);

        // fire::update directly, pinned to one cell -- the same approach
        // `fire.rs`'s own `oil_ignites_next_to_fire_and_eventually_burns_
        // out_to_ash` test uses for its own burnout half, and for the same
        // reason: this is checking fire's burnout hook specifically, not
        // movement, and oil is `Liquid` (falls/spreads under gravity like
        // anything else), so running it through the full CA sweep would
        // require a floor and could still relocate the cell before it
        // burns out, for reasons that have nothing to do with what this
        // test is actually checking.
        for _ in 0..10 {
            crate::sim::fire::update(&mut w, 30, 30);
        }
        assert_eq!(w.get(30, 30).material, material::ASH, "test setup should have burned out to ash");
        assert!(w.active_site_count() > 0, "burning out to ash should have scheduled a decay check");
    }
}
