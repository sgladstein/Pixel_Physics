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

    // **Read from the cell, not from a hardcoded material.** This used to
    // test `!= ash_id` and produce soil unconditionally; both ends are data
    // now (`Material::decays_into`) because litter needed the same channel
    // and a second hardcoded branch is not a mechanism.
    //
    // A cell with no `decays_into` also lands here: it may have burned into
    // something else, been erased, or been buried and dug back out as
    // something else entirely since the site was scheduled. Nothing to do,
    // and nothing to reschedule.
    let cell = world.get(x, y);
    let Some(into) = world.materials.get(cell.material).decays_into else {
        return Vec::new();
    };

    let damp = world.field_at(x, y).moisture > DECAY_MOISTURE_THRESHOLD;
    let chance = if damp { DECAY_CHANCE_DAMP } else { DECAY_CHANCE_DRY };
    if !world.rng.chance(chance) {
        return vec![ActiveSite { x, y, kind: ActiveKind::Decay, next_frame: world.frame + DECAY_TICK_INTERVAL }];
    }

    let shades = world.materials.get(into).palette.len().max(1) as u32;
    let shade = world.rng.below(shades) as u8;
    world.set(x, y, Cell::new(into, shade));

    // Reseed roll: only if there's actually room to grow into, only ever
    // this one chance (see RESEED_CHANCE's own doc), and only for a material
    // that asked for it. Ash does; litter deliberately does not, because
    // leaf fall under a standing canopy is not a succession event -- see
    // `Material::decay_reseeds`.
    if world.materials.get(cell.material).decay_reseeds && world.is_empty(x, y - 1) && world.rng.chance(RESEED_CHANCE) {
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

    /// `run` with a real physics driver in front of it, in `App::update`'s
    /// own order. Every other test in this module uses `run`, which drives
    /// no driver at all -- so nothing in them can fall, and a scheduled
    /// site can never be stranded by its cell moving out from under it.
    /// Anything asking a question about a cell that *moves* has to use
    /// this one.
    fn run_with_physics(w: &mut World, frames: usize) {
        for _ in 0..frames {
            crate::sim::update::step(w);
            w.step_active_sites();
            field::step(w);
        }
    }

    fn count(w: &World, id: crate::sim::material::MaterialId) -> usize {
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

    /// **A decay site is a coordinate, and nothing makes it follow its
    /// cell.** `CellSurface::move_cell` copies the cell and its flags and
    /// touches no scheduler state, and `tick` above *unschedules* on a
    /// material mismatch (the "burned into something else, erased, buried"
    /// arm) -- which is also what a cell simply having fallen out of the
    /// coordinate looks like. So any ash that moves before its first check
    /// comes due, 200 frames later, is immortal.
    ///
    /// Every other test in this module hides this, and hides it the way
    /// CLAUDE.md warns about: their `run` drives `field::step` and
    /// `scheduler::step` and **no physics driver at all**, so their ash is
    /// on a platform that cannot fall because nothing in the test can make
    /// anything fall. They pass because the scenario is trivially stable,
    /// which looks exactly like passing because the code is correct.
    ///
    /// Found while scoping WP-B2 (litter), which this blocks outright:
    /// litter is shed in a canopy and falls to the ground every single
    /// time, so *all* of it would strand and the ground would accumulate
    /// litter that never drains. The live consequence for ash is narrower
    /// but real -- fire makes ash where the fuel was, the fuel below it has
    /// just burned away, so ash usually falls too, and M16's own "a forest
    /// burns and regrows" criterion is only half-working.
    ///
    /// **Fixed**, and this is now the guard. The fix was not to make sites
    /// follow cells -- it was to stop scheduling at creation and schedule on
    /// **settle** instead (`World::end_step`'s awake->settled transition),
    /// which is what the rule always meant: weathering happens to matter
    /// that has come to rest. A cell that moves loses its site and gets a
    /// fresh one when it stops, which is the correct behaviour rather than a
    /// repair of the broken one.
    #[test]
    fn ash_that_falls_before_its_first_check_still_decays() {
        let mut w = test_world();
        let ash = material::ASH;
        let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");

        // A walled basin, flooded, so both arms end up damp on the same
        // floor -- a paired comparison that cancels everything except
        // whether the cell moved.
        for x in 10..=31 {
            w.set(x, 110, Cell::new(material::STONE, 0));
        }
        for y in 100..=109 {
            w.set(10, y, Cell::new(material::STONE, 0));
            w.set(31, y, Cell::new(material::STONE, 0));
        }
        for y in 103..=109 {
            for x in 11..=30 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }

        // Control: wedged between two stone blocks on the floor, so it
        // physically cannot move and its site cannot strand.
        w.set(12, 109, Cell::new(material::STONE, 0));
        w.set(14, 109, Cell::new(material::STONE, 0));
        w.set(13, 109, Cell::new(ash, 0));
        w.schedule_active_site(ActiveSite { x: 13, y: 109, kind: ActiveKind::Decay, next_frame: DECAY_TICK_INTERVAL });

        // The arm under test: identical in every way except that it is
        // released above the waterline and falls to the same floor. Its
        // site is scheduled where it was *created*, which is exactly what
        // `fire::tick_burn` does for a real burnout.
        w.set(25, 100, Cell::new(ash, 0));
        w.schedule_active_site(ActiveSite { x: 25, y: 100, kind: ActiveKind::Decay, next_frame: DECAY_TICK_INTERVAL });

        run_with_physics(&mut w, 20_000);

        assert_eq!(
            w.get(13, 109).material, soil,
            "the wedged control never decayed -- the scene is wrong, not the mechanism",
        );
        assert_eq!(
            count(&w, ash), 0,
            "ash that fell before its first decay check is still ash after 20,000 frames: \
             its decay site stranded at the coordinate it was created in",
        );
    }

    /// **Litter drains, and the assertion is strict decrease to zero.**
    /// WP-B2's acceptance asks for exactly this, and for the reason the
    /// u8-decay ghost trails taught: a channel that decrements toward a
    /// floor can look like it is draining forever while never arriving.
    ///
    /// The scene is a sealed trough so the litter cannot leave the sampled
    /// region by falling out of the world -- if it could, "count reached 0"
    /// would be true for a reason that has nothing to do with decay, which
    /// is the trap this repo calls "a metric that counts what falling water
    /// looks like".
    #[test]
    fn litter_rots_away_instead_of_accumulating_forever() {
        let mut w = test_world();
        let Some(litter) = w.materials.id_of("litter") else { return };
        let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");

        // Sealed trough, flooded, so the litter lands damp and stays put.
        for x in 10..=31 {
            w.set(x, 110, Cell::new(material::STONE, 0));
        }
        for y in 100..=109 {
            w.set(10, y, Cell::new(material::STONE, 0));
            w.set(31, y, Cell::new(material::STONE, 0));
        }
        for y in 106..=109 {
            for x in 11..=30 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        // Dropped from above the waterline, so every cell moves before its
        // first check -- the case that used to strand every site.
        for x in 12..=28 {
            w.set(x, 101, Cell::new(litter, 0));
        }
        let before = count(&w, litter);
        assert_eq!(before, 17, "scene should start with 17 litter cells, has {before}");

        run_with_physics(&mut w, 10_000);
        let mid = count(&w, litter);
        assert!(mid < before, "litter is not draining at all: {before} -> {mid} in 10,000 frames");

        run_with_physics(&mut w, 40_000);
        let after = count(&w, litter);
        assert_eq!(after, 0, "litter never fully drained: {before} -> {mid} -> {after}");
        assert!(count(&w, soil) > 0, "litter drained but produced no soil -- decays_into is not being read");
    }

    /// **The sanity check: what does this metric say when nothing is
    /// wrong?** WP-B2's acceptance asks for it by name, and this repo has
    /// already shipped one metric that counted every droplet in the world
    /// because nobody asked what it read on a healthy case.
    ///
    /// Litter with no `decays_into` would sit forever; litter that decays
    /// on a schedule nothing scheduled would also sit forever. Neither is
    /// what this asserts. This asserts the *other* end: a world where no
    /// plant sheds contains exactly zero litter, so a non-zero count in any
    /// other test is abscission and not scenery.
    #[test]
    fn a_world_where_nothing_sheds_holds_exactly_no_litter() {
        let mut w = test_world();
        let Some(litter) = w.materials.id_of("litter") else { return };
        let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");
        for x in 0..60 {
            for y in 100..104 {
                w.set(x, y, Cell::new(soil, 0));
            }
        }
        run_with_physics(&mut w, 5_000);
        assert_eq!(count(&w, litter), 0, "bare soil with no plants somehow produced litter");
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
