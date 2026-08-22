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
pub(crate) const DECAY_MOISTURE_THRESHOLD: f32 = 0.3;
// The reseed chance moved onto the material (`ash.ron`'s `reseed_chance`,
// which keeps this constant's old 0.15). It is checked once at the moment of
// decay rather than scheduled to keep trying — succession happens, but not on
// every patch of soil forever, and a soil cell that misses its one roll stays
// bare soil for good. It had to stop being global when litter arrived: ash
// cells exist only where something burned, litter cells are produced by every
// shading leaf in a growing forest, and one shared 0.15 would have turned a
// stand into a mat of seedlings.

/// Dispatch a due `ActiveKind::Decay` site. `scheduler::step` never routes
/// any other `ActiveKind` here.
pub fn tick(world: &mut World, site: &ActiveSite) -> Vec<ActiveSite> {
    debug_assert!(matches!(site.kind, ActiveKind::Decay), "scheduler::step only routes ActiveKind::Decay here");
    let (x, y) = (site.x, site.y);

    // **What is here now, and what its own file says it turns into.** This
    // used to read `id_of("ash")` and write `id_of("soil")`. That was honest
    // while ash was the only material that weathered, and it became a wall
    // the moment litter (S4) needed the same channel: the scheduling side was
    // already generic over `ActiveKind::Decay`, only the transformation was
    // nailed to two names. `CLAUDE.md` -- when a rule must tell two things
    // apart, state the difference as data.
    //
    // The cell may have burned into something else, been erased, or been
    // buried and dug back out as something else entirely by the time this
    // check comes due. Reading the material *here* rather than remembering
    // what scheduled the site is what makes all of those a quiet no-op.
    let here = world.materials.get(world.get(x, y).material);
    let (Some(into), reseed_chance) = (here.decays_into, here.reseed_chance) else {
        return Vec::new();
    };

    // **The material's own rate if it states one, the shared one otherwise.**
    // Ash states none and keeps the constants below, which is why every ash
    // measurement in this file still reads what it always did. Litter states
    // its own, because sharing ash's gave it a lifetime sixteen times longer
    // than the runs it is measured in -- so it never reached equilibrium, it
    // only integrated the canopy's shedding.
    let damp = world.field_at(x, y).moisture > DECAY_MOISTURE_THRESHOLD;
    let (m_damp, m_dry) = (here.decay_chance_damp, here.decay_chance_dry);
    let chance = if damp {
        if m_damp > 0.0 { m_damp } else { DECAY_CHANCE_DAMP }
    } else if m_dry > 0.0 {
        m_dry
    } else {
        DECAY_CHANCE_DRY
    };
    if !world.rng.chance(chance) {
        return vec![ActiveSite { x, y, kind: ActiveKind::Decay, next_frame: world.frame + DECAY_TICK_INTERVAL }];
    }

    let shades = world.materials.get(into).palette.len().max(1) as u32;
    let shade = world.rng.below(shades) as u8;
    world.set(x, y, Cell::new(into, shade));

    // Reseed roll: only if there's actually room to grow into, and only ever
    // this one chance -- see `MaterialDef::reseed_chance`, which is now per
    // material rather than one global constant, because ash and litter differ
    // by two orders of magnitude in how many cells reach this line.
    if reseed_chance > 0.0 && world.is_empty(x, y - 1) && world.rng.chance(reseed_chance) {
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

    /// **The generalisation, tested on the material it was made for.**
    ///
    /// `tick` used to read `id_of("ash")` and write `id_of("soil")`. Making
    /// it read the cell's own `decays_into` left every ash test passing --
    /// which proves ash still works and says nothing about whether a *second*
    /// material can use the channel at all. `CLAUDE.md`: when you replace a
    /// mechanism, the old tests can keep passing while testing nothing.
    #[test]
    fn litter_weathers_into_soil_on_the_same_channel_ash_uses() {
        let mut w = test_world();
        let litter = w.materials.id_of("litter").expect("litter is a compiled-in material");
        let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");
        assert_eq!(
            w.materials.get(litter).decays_into,
            Some(soil),
            "test setup: this test is about litter weathering to soil, and that is not what the data says"
        );

        // Same walled trough the ash test uses, and for the same reason: an
        // open puddle drains away before the moisture field registers it.
        for x in 10..20 {
            w.set(x, 100, Cell::new(litter, 0));
        }
        w.set(9, 99, Cell::new(material::STONE, 0));
        w.set(20, 99, Cell::new(material::STONE, 0));
        for x in 12..18 {
            w.set(x, 99, Cell::new(material::WATER, 0));
        }
        w.schedule_active_site(ActiveSite { x: 14, y: 100, kind: ActiveKind::Decay, next_frame: DECAY_TICK_INTERVAL });

        run(&mut w, 20_000);

        assert_eq!(w.get(14, 100).material, soil, "damp litter never weathered into soil, so the decay channel is still ash-only");
    }

    /// **Litter must not reseed, and ash must still do so.**
    ///
    /// The reseed roll was one global `RESEED_CHANCE = 0.15`, which was fine
    /// while ash was its only caller: ash exists only where something burned.
    /// Litter is produced by every shading leaf in a growing forest -- 11,478
    /// cells in one measured run -- and a 15% chance per cell to plant a tree
    /// is a feedback loop that turns a stand into a solid mat of seedlings.
    /// The value is per material now, and this is the guard on the pair of
    /// them, because "0" is exactly the kind of value that reads as an
    /// oversight and gets "fixed" later.
    #[test]
    fn litter_does_not_reseed_the_forest_that_dropped_it() {
        let w = test_world();
        let litter = w.materials.id_of("litter").expect("litter");
        assert_eq!(w.materials.get(litter).reseed_chance, 0.0, "litter must not reseed: see the type doc for the feedback loop it would close");
        assert!(
            w.materials.get(material::ASH).reseed_chance > 0.0,
            "ash still has to reseed -- that is M16's own 'a forest burns and regrows' criterion, and moving the constant onto             the material must not have quietly dropped it"
        );
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
    fn a_reseeded_organism_keeps_growing_after_its_first_tick() {
        // Code-review-findings item #2 follow-up: `plant_moss_seed`/
        // `plant_tree` are called from *inside* `decay::tick` -- itself a
        // scheduler-dispatched tick -- and schedule the new organism's own
        // first growth check via `World::schedule_active_site` directly,
        // not through this function's own returned `Vec<ActiveSite>`.
        // Before `World::pop_due_active_site` replaced the old take-the-
        // whole-heap-out/put-it-back shape, that call landed in a
        // temporarily emptied `active_sites` field and was silently
        // discarded the moment `scheduler::step` finished writing the real
        // heap back over it -- so a decay-reseeded seed got planted (the
        // material check in the test above can't tell the difference) but
        // never grew a single cell beyond itself, forever. Confirmed to
        // fail against the pre-fix code: the moss/wood count below never
        // increases past whatever reseeding alone produced.
        let mut w = test_world();
        let ash = material::ASH;
        let moss = w.materials.id_of("moss").expect("moss is a compiled-in material");
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");

        let mut ash_x = Vec::new();
        for &puddle_start in &[10, 40, 70, 100, 130, 160] {
            w.set(puddle_start - 1, 99, Cell::new(material::STONE, 0));
            w.set(puddle_start + 6, 99, Cell::new(material::STONE, 0));
            for x in puddle_start..puddle_start + 6 {
                w.set(x, 99, Cell::new(material::WATER, 0));
            }
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

        // Scans the whole world, not just row 99 -- growth (moss dividing,
        // a tree's canopy/roots) spreads away from the exact reseed
        // position, so a narrower scan would undercount and risk a false
        // "no growth" reading even when growth is real.
        let count_moss_and_wood = |w: &World| -> usize {
            let bounds = w.bounds().unwrap();
            let mut n = 0;
            for y in bounds.min_y..=bounds.max_y {
                for x in bounds.min_x..=bounds.max_x {
                    let m = w.get(x, y).material;
                    if m == moss || m == wood {
                        n += 1;
                    }
                }
            }
            n
        };

        run(&mut w, 5_000);
        let after_reseed = count_moss_and_wood(&w);
        assert!(after_reseed > 0, "test setup: nothing reseeded at all in five thousand frames");

        // Long enough for moss's own near-zero Divide cost (or a tree's
        // Germinate/Grow cadence) to have produced several more cells if
        // -- and only if -- the reseeded organism's own scheduling
        // actually reached the heap.
        //
        // **The window used to be 20,000 then 20,000, and it had to move.**
        // Measured over this exact scene, the reseeded population grows
        // 71 -> 176 -> 225 cells at frames 5k/10k/15k and then stops dead:
        // 225 at every sample from 15k to 40k, with zero moss cells left
        // holding an empty growable neighbour. The old second window was
        // therefore entirely inside the saturated regime and asserted that
        // growth continued where, correctly, none could.
        //
        // It passed anyway until this session, for a reason worth writing
        // down: the plastochron change gave shoots persistent `Leaf` cells
        // and so more standing income, trees now reach their frontier
        // ceiling (every lineage retired to `MatureBody`, nothing able to
        // open a new one) sooner, and saturation crossed below the 20,000
        // mark. The per-organism RNG then shifted it just far enough to
        // fail. Neither change broke the property this test exists for --
        // they moved the point where the *proxy* for it stops being
        // measurable.
        //
        // 5k/5k sits well inside the growing regime at both ends (71 then
        // 176, against saturation at ~15k), per this repo's "set bars from
        // measurement with headroom" convention. The assertion itself is
        // unchanged and still fails outright if a reseeded organism never
        // advances past its first cell, which is the scheduler regression
        // (code-review item #2's bug #2) it was written to catch.
        run(&mut w, 5_000);
        let after_more_growth = count_moss_and_wood(&w);
        assert!(
            after_more_growth > after_reseed,
            "reseeded growth never advanced past its own first cell: {after_reseed} then {after_more_growth}"
        );
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
