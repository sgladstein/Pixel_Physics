//! M15: explosions, built entirely from the three systems that came before —
//! the M13 field (pressure impulse, shock propagation), M14 (heat, ignition),
//! and M7 (debris as free particles). Nothing here is a new simulation
//! primitive; it is three existing primitives triggered together.
//!
//! Per the plan: "an explosion writes three things: a pressure impulse into
//! the field, a temperature spike, and a radius of cells converted to free
//! particles or vacuum. The shock then propagates and reflects through the
//! field for free — that is the whole reason for building it."

use super::cell::Cell;
use super::field::FIELD_SCALE;
use super::material;
use super::particle::ParticleSystem;
use super::rng;
use super::world::World;

/// How much of the blast's `strength` becomes the field's heat spike, versus
/// the pressure impulse (which uses `strength` directly). Heat needs to be
/// smaller in absolute terms — `strength` values large enough to throw debris
/// convincingly would otherwise inject enough heat to overshoot `field::
/// MAX_TEMPERATURE` immediately, clamping rather than spiking.
const HEAT_FRACTION: f32 = 3.0;

/// Fraction of `radius` that also gets a forced ignition check — the
/// explosion's "fireball," smaller than the full clearing radius, matching
/// the everyday intuition that an explosion's blast reaches further than the
/// flame it leaves behind.
const FIREBALL_FRACTION: f32 = 0.5;

/// Base debris speed at `strength = 100` — flat across the whole blast
/// radius; only *direction* varies with position (`debris_velocity`'s own
/// pressure-gradient read). Picked by eye against `App::spawn_burst`'s
/// 3.0–6.0 range for a visually comparable "thrown," not measured against
/// anything physical.
const SPEED_PER_STRENGTH: f32 = 0.05;

/// Fraction of `radius` that genuinely vaporizes — no debris, nothing left,
/// full stop. Small and deliberate: real high explosives do pulverize a
/// small core beyond any hope of throwing it as identifiable debris, but an
/// explosion's actual visual signature is *material flying outward*, not a
/// clean hole. An earlier version instead rolled `1.0 - sqrt(dist / radius)`
/// odds of debris per cell — a curve that drops off steeply even close to
/// the centre (50% debris odds by a quarter of the radius, 13% by three
/// quarters), and since a circle's *area* is dominated by its outer band,
/// that meant most of the blast's affected area vaporized with nothing to
/// show for it: reads as "a clean circle disappears, thin ring of sparks,"
/// not "an explosion." Every cell outside this small core now becomes
/// debris unconditionally instead.
const VAPORIZE_FRACTION: f32 = 0.12;

/// How far past `radius` the shockwave (step 2.5) still has a chance to
/// pick up loose material, as a multiple of `radius` itself. `1.0` would
/// mean no shockwave at all (the annulus is empty); picked to reach
/// noticeably further than the crater without being unbounded — untuned
/// against anything real, same as every other constant in this file.
const SHOCKWAVE_RADIUS_MULTIPLIER: f32 = 1.8;

/// Scales the position-keyed jitter `debris_velocity` adds to each cell's
/// launch velocity, as a fraction of that cell's own computed `speed` — not
/// a flat value, and deliberately **not** scaled by raw `strength`.
/// `strength` values large enough to throw debris convincingly (150-200 in
/// this module's own tests) already push `speed` at or past
/// `particle::MAX_SPEED_PER_AXIS` once multiplied by `SPEED_PER_STRENGTH`,
/// so a `* strength` jitter term
/// would pin every particle straight to that clamp and make debris *more*
/// uniform, not less — caught during planning, before this was ever
/// implemented against raw `strength`. `0.4`, roughly ±20% of launch speed,
/// is enough to break the same-field-tile cohesion `debris_velocity`'s own
/// doc describes (every cell within one `FIELD_SCALE` tile reads a
/// near-identical pressure gradient and would otherwise launch with
/// near-identical velocity, reading as a moving block rather than a
/// scatter), while staying small enough that the gradient's own shape still
/// dominates the overall blast direction.
const DEBRIS_JITTER_STRENGTH: f32 = 0.4;

/// Offset applied to the *x* input of the *y* jitter sample, so a cell's x
/// and y jitter values are not the same number twice. Without this, jitter
/// would only ever push diagonally (`vx`/`vy` jitter identically), not
/// scatter in every direction. Arbitrary — any fixed nonzero offset works,
/// this just needs to be a value distinct from a real world x-coordinate
/// range colliding in a way that would systematically correlate the two.
const JITTER_AXIS_OFFSET: i32 = 7919;

/// Trigger an explosion centred on `(cx, cy)` with the given `radius` (world
/// cells) and `strength` (feeds both the pressure impulse and, scaled down,
/// the heat spike — see `HEAT_FRACTION`).
///
/// Takes `&mut ParticleSystem` alongside `&mut World` rather than living as a
/// `World` method: debris becomes free particles, and `World` does not own
/// the particle system (`App` does, the same way it does not own the
/// renderer) — the same shape as `fire::update` and `tick_burn` taking
/// `&mut World` rather than being `World` methods.
pub fn trigger(world: &mut World, particles: &mut ParticleSystem, cx: i32, cy: i32, radius: i32, strength: f32) {
    // 1. Pressure impulse and heat spike — the field carries the shock from
    // here; nothing else in this function propagates it.
    world.add_pressure_impulse(cx, cy, radius, strength);
    world.add_heat(cx, cy, radius, strength / HEAT_FRACTION);

    // 2. Clear the blast radius, converting almost everything to flying
    // debris — matching the plan's "free particles OR vacuum," but
    // "vacuum" is now only the small core `VAPORIZE_FRACTION` marks out
    // (see its own doc for why), not most of the affected area.
    let r2 = (radius * radius) as f32;
    let vaporize_r2 = r2 * VAPORIZE_FRACTION * VAPORIZE_FRACTION;
    for y in (cy - radius)..=(cy + radius) {
        for x in (cx - radius)..=(cx + radius) {
            let (dx, dy) = (x - cx, y - cy);
            let dist2 = (dx * dx + dy * dy) as f32;
            if dist2 > r2 {
                continue;
            }
            let cell = world.get(x, y);
            // A raw material test, not `cell.is_empty()`. This function's own
            // question is "is there material here to destroy", not "is this
            // position available to use" -- and `is_empty()` answers the
            // second, treating a promoted liquid body's reserved container
            // cells as occupied even though they hold `material::EMPTY`.
            // Through `is_empty()` an explosion overlapping a lake's outline
            // spawned debris particles whose material was `EMPTY`: invisible
            // flying nothing, which then lands and writes itself into the
            // world as a cell. `render.rs` and `World::ignite_circle` already
            // made exactly this switch, each for its own version of the same
            // reason. Found by review.
            //
            // Bedrock is the world's own boundary material and never
            // destructible by anything, the same way it is never a target
            // for painting (`World::paint_circle`) or ignition.
            if cell.material == material::EMPTY || cell.material == material::BEDROCK {
                continue;
            }

            if dist2 > vaporize_r2 {
                let (vx, vy) = debris_velocity(world, x, y, cx, cy, strength);
                particles.spawn(x as f32, y as f32, vx, vy, cell.material, cell.shade);
            }
            let was_structural = matches!(world.materials.kind(cell.material), material::MaterialKind::Solid | material::MaterialKind::Plant);
            world.set(x, y, Cell::EMPTY);
            // M17: an explosion is exactly the kind of disturbance structural
            // checks exist for -- clearing a `Solid`/`Plant` cell (the
            // latter added by architecture item 9) may have just dropped
            // whatever it was propping up.
            if was_structural {
                world.schedule_structural_check_around(x, y);
            }
        }
    }

    // 2.5. A shockwave beyond the crater: loose material (`Powder`/`Liquid`)
    // just outside `radius` has a fading chance to be picked up and thrown
    // too, not just left to fall into the hole step 2 dug. Without this, an
    // explosion in the *middle* of a big sand pile reads as "a hole appears,
    // the surroundings quietly avalanche into it" — ordinary settling under
    // gravity is the only thing that ever moves a loose CA cell that wasn't
    // itself inside the blast radius, since the pressure impulse step 1
    // wrote only ever pushes free particles/gas, never settled grid
    // material. Restricted to loose material specifically (not `Solid`/
    // `Plant`, which shouldn't be uprooted by a shockwave that didn't even
    // clear them, and not already-`Empty`) — a blast can fling sand it
    // never touched, but it does not casually rip a wall out by the same
    // mechanism that would need to actually break it structurally first.
    let shockwave_r2 = r2 * SHOCKWAVE_RADIUS_MULTIPLIER * SHOCKWAVE_RADIUS_MULTIPLIER;
    let shockwave_radius = (radius as f32 * SHOCKWAVE_RADIUS_MULTIPLIER).round() as i32;
    for y in (cy - shockwave_radius)..=(cy + shockwave_radius) {
        for x in (cx - shockwave_radius)..=(cx + shockwave_radius) {
            let (dx, dy) = (x - cx, y - cy);
            let dist2 = (dx * dx + dy * dy) as f32;
            if dist2 <= r2 || dist2 > shockwave_r2 {
                continue; // already handled by step 2, or outside the shockwave's reach entirely
            }
            let cell = world.get(x, y);
            if !matches!(world.materials.kind(cell.material), material::MaterialKind::Powder | material::MaterialKind::Liquid) {
                continue;
            }
            let pickup_chance = shockwave_pickup_chance(radius, dist2.sqrt());
            if world.rng.chance(pickup_chance) {
                let (vx, vy) = debris_velocity(world, x, y, cx, cy, strength);
                particles.spawn(x as f32, y as f32, vx, vy, cell.material, cell.shade);
                world.set(x, y, Cell::EMPTY);
            }
        }
    }

    // 3. Ignite material just beyond the clearing radius — not within it,
    // since step 2 just cleared everything there to vacuum or debris, and a
    // fireball inside a hole has nothing left to burn. This is deliberately
    // *larger* than `radius`, reaching the scorched-but-intact ring around
    // the blast, which is what an explosion actually sets alight. Run after
    // clearing, against the final post-clearing world state, rather than
    // interleaved with it — the two no longer touch the same cells, so
    // ordering relative to step 2 has no effect other than being easier to
    // read as "destroy, then set the surroundings on fire."
    //
    // Known simplification: this reuses `World::ignite_circle`, the debug
    // force-ignite tool, which sets any material burning regardless of
    // `flammability` — a stone wall next to a blast currently gets the same
    // fire tint an oil pool would, rather than being immune the way its
    // `flammability: 0.0` says it should be. Visually this reads as "the
    // blast leaves the surroundings glowing hot" more than "stone catches
    // fire," which is not unreasonable for a first cut, but a version that
    // actually checks flammability (closer to `fire::try_ignite`'s
    // temperature-driven path, fed by the heat spike step 1 already wrote)
    // would be the more correct fix, not implemented here.
    let fireball_radius = radius + ((radius as f32) * FIREBALL_FRACTION).round() as i32;
    world.ignite_circle(cx, cy, fireball_radius.max(radius + 1));
}

/// Debris velocity from the local pressure gradient — not a naive radial
/// burst — so a blast throws material away from the centre and around
/// corners rather than in a perfect circle regardless of what is in the way.
///
/// The gradient is read from the field exactly as it stands the instant
/// after `add_pressure_impulse` runs, before the field has taken a single
/// `field::step` — the impulse has not yet propagated or reflected off
/// anything at this point, so what actually gives this its shape is checking
/// `field_is_blocked` at each neighbour and skipping a blocked one, rather
/// than reading its (still-ambient) pressure as if it were open ground. A
/// neighbour on the far side of a wall is excluded from the gradient the same
/// way the field's own `step_velocity` excludes it, just computed directly
/// here instead of waiting a frame for the field to do it.
/// Chance a loose cell at distance `dist` from the epicentre is picked up by
/// the shockwave, fading linearly from 1.0 at `radius` to 0.0 at
/// `radius * SHOCKWAVE_RADIUS_MULTIPLIER`. Deliberately built from the
/// *continuous* radius, not the rounded `shockwave_radius` the caller's loop
/// bounds use: an earlier version divided by `shockwave_radius - radius`
/// (rounded), so whenever `radius * SHOCKWAVE_RADIUS_MULTIPLIER` rounds
/// down, cells between the true and rounded edge still passed the caller's
/// `dist2 <= shockwave_r2` zone check but produced a negative chance instead
/// of fading to exactly zero — `Rng::chance` silently treats negative as
/// "never," so the annulus quietly narrowed below what the constant says it
/// should be.
fn shockwave_pickup_chance(radius: i32, dist: f32) -> f32 {
    // Clamped defensively: right at the outer edge, `dist` and the
    // continuous radius are equal in exact math but not always in float
    // math, so the unclamped formula can land a hair below zero there.
    // `Rng::chance` already treats negative as "never," but `chance` is a
    // probability and should read as one.
    (1.0 - (dist - radius as f32) / (radius as f32 * (SHOCKWAVE_RADIUS_MULTIPLIER - 1.0))).clamp(0.0, 1.0)
}

fn debris_velocity(world: &World, x: i32, y: i32, cx: i32, cy: i32, strength: f32) -> (f32, f32) {
    let sample = |dx: i32, dy: i32| -> Option<f32> {
        let (nx, ny) = (x + dx, y + dy);
        if world.field_is_blocked(nx, ny) {
            None
        } else {
            Some(world.field_at(nx, ny).pressure)
        }
    };

    let left = sample(-FIELD_SCALE, 0);
    let right = sample(FIELD_SCALE, 0);
    let up = sample(0, -FIELD_SCALE);
    let down = sample(0, FIELD_SCALE);

    // Missing (wall-blocked) sides simply do not contribute — treating a
    // blocked neighbour as "equal to here" would flatten the gradient right
    // where a wall should be steering it instead.
    let gx = match (left, right) {
        (Some(l), Some(r)) => l - r,
        (Some(l), None) => l, // only the open side pushes, away from it
        (None, Some(r)) => -r,
        (None, None) => 0.0,
    };
    let gy = match (up, down) {
        (Some(u), Some(d)) => u - d,
        (Some(u), None) => u,
        (None, Some(d)) => -d,
        (None, None) => 0.0,
    };

    let mag = (gx * gx + gy * gy).sqrt();
    let speed = strength * SPEED_PER_STRENGTH;

    let (vx, vy) = if mag > 0.01 {
        (gx / mag * speed, gy / mag * speed)
    } else {
        // No usable gradient (dead centre, or walled in on every side) —
        // fall back to a purely radial push away from the epicentre so a
        // symmetric position still gets thrown *somewhere* rather than
        // sitting motionless in an otherwise fully cleared blast radius.
        let (dx, dy) = ((x - cx) as f32, (y - cy) as f32);
        let d = (dx * dx + dy * dy).sqrt().max(1.0);
        (dx / d * speed, dy / d * speed)
    };

    // Position-keyed (not frame-keyed) so a given cell's jitter is stable —
    // matters here only for test determinism, not gameplay, since a cell is
    // cleared to `Cell::EMPTY` and never revisited the moment it's thrown.
    // See `DEBRIS_JITTER_STRENGTH`'s own doc for why this is scaled by
    // `speed`, not raw `strength`.
    let jx = (rng::jitter(x, y) - 0.5) * DEBRIS_JITTER_STRENGTH * speed;
    let jy = (rng::jitter(x + JITTER_AXIS_OFFSET, y) - 0.5) * DEBRIS_JITTER_STRENGTH * speed;
    (vx + jx, vy + jy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::material;

    fn test_world() -> World {
        let mut w = World::new(Rect::new(0, 0, 127, 127));
        for x in 0..128 {
            w.set(x, 127, Cell::new(material::STONE, 0));
        }
        w
    }

    #[test]
    fn an_explosion_clears_material_within_its_radius() {
        let mut w = test_world();
        for y in 30..50 {
            for x in 30..50 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 40, 40, 8, 150.0);

        assert!(w.get(40, 40).is_empty(), "the epicentre was not cleared");
    }

    #[test]
    fn an_explosion_leaves_bedrock_untouched() {
        let mut w = test_world();
        w.set(40, 40, Cell::new(material::BEDROCK, 0));
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 40, 40, 8, 150.0);
        assert_eq!(w.get(40, 40).material, material::BEDROCK, "bedrock was destroyed");
    }

    #[test]
    fn an_explosion_raises_pressure_and_temperature() {
        let mut w = test_world();
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 40, 40, 8, 150.0);

        assert!(w.field_at(40, 40).pressure.abs() > 1.0, "no pressure impulse");
        assert!(w.field_at(40, 40).temperature > 20.0, "no heat spike");
    }

    #[test]
    fn debris_velocity_varies_within_a_single_field_tile() {
        // Diagnosis this section fixed: `debris_velocity` samples the field
        // (via `World::field_at`, a coarse block lookup -- see its own doc)
        // at exactly +/- FIELD_SCALE, so every cell within roughly one field
        // tile read the exact same quantized pressure gradient and launched
        // with identical velocity, reading as a moving block rather than a
        // scatter of debris. An entirely open world (no walls to trip the
        // blocked-fallback path instead of the real gradient one) with a
        // real pressure impulse, so `x = 34` and `x = 35` (`y` held fixed)
        // land in the same coarse blocks for all four samples and would
        // read a bit-identical gradient before this section's jitter --
        // confirmed to fail (`vx1 == vx2 && vy1 == vy2` exactly) with
        // `DEBRIS_JITTER_STRENGTH` temporarily zeroed.
        let mut w = test_world();
        w.add_pressure_impulse(40, 40, 8, 200.0);
        let (vx1, vy1) = debris_velocity(&w, 34, 34, 40, 40, 200.0);
        let (vx2, vy2) = debris_velocity(&w, 35, 34, 40, 40, 200.0);
        assert!(
            (vx1 - vx2).abs() > 0.01 || (vy1 - vy2).abs() > 0.01,
            "adjacent cells reading the same coarse field block launched with identical velocity: \
             ({vx1}, {vy1}) vs ({vx2}, {vy2})"
        );
    }

    #[test]
    fn an_explosion_at_the_centre_throws_debris() {
        let mut w = test_world();
        for y in 30..50 {
            for x in 30..50 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 40, 40, 8, 150.0);

        assert!(!particles.is_empty(), "no debris was thrown at all");
        // At least some debris near the centre should be moving with real
        // speed, not sitting at zero velocity.
        let any_fast = particles.iter().any(|p| p.vx.abs() > 0.5 || p.vy.abs() > 0.5);
        assert!(any_fast, "debris was thrown with no meaningful velocity");
    }

    #[test]
    fn most_of_the_blast_radius_becomes_debris_not_vaporized() {
        // A dense fill spanning past the whole blast radius, so every
        // cleared cell had material to begin with (no early-continue on an
        // already-empty cell skewing the count). Checks that the *bulk* of
        // the affected area is thrown as debris, not just a lucky handful
        // near the epicentre -- the actual complaint the old `1.0 -
        // sqrt(dist / radius)` curve produced (most of a circle's area sits
        // in its outer band, where that curve's odds were already down to
        // single digits).
        let mut w = test_world();
        for y in 20..60 {
            for x in 20..60 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        let radius = 10;
        trigger(&mut w, &mut particles, 40, 40, radius, 150.0);

        let cleared = ((40 - radius)..=(40 + radius))
            .flat_map(|y| ((40 - radius)..=(40 + radius)).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let (dx, dy) = (x - 40, y - 40);
                dx * dx + dy * dy <= radius * radius
            })
            .filter(|&(x, y)| w.get(x, y).is_empty())
            .count();
        let debris_count = particles.iter().count();
        assert!(cleared > 0, "test setup: nothing was cleared at all");
        assert!(
            (debris_count as f32) > (cleared as f32) * 0.7,
            "most of the cleared blast radius should have become debris, not vaporized: \
             {debris_count} debris out of {cleared} cleared cells"
        );
    }

    #[test]
    fn a_shockwave_flings_loose_material_beyond_the_crater() {
        // The other half of "I want to see sand flying": an explosion in
        // the *middle* of a large sand pile should actively throw sand from
        // beyond the crater too, not just leave a hole for the surrounding
        // pile to quietly avalanche into under gravity -- gravity/settling
        // is the only thing that ever moves a loose CA cell the blast
        // radius itself never touched, since the field's own pressure
        // impulse only ever pushes free particles, never settled grid
        // material.
        let mut w = test_world();
        for y in 10..70 {
            for x in 10..70 {
                w.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        let radius = 8;
        trigger(&mut w, &mut particles, 40, 40, radius, 200.0);

        // Just past the crater's own edge, still well inside the shockwave
        // reach (`radius * SHOCKWAVE_RADIUS_MULTIPLIER`).
        let just_beyond = radius + 2;
        let cleared_beyond_crater = ((40 - just_beyond)..=(40 + just_beyond))
            .flat_map(|y| ((40 - just_beyond)..=(40 + just_beyond)).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let (dx, dy) = (x - 40, y - 40);
                let dist2 = dx * dx + dy * dy;
                dist2 > radius * radius && dist2 <= just_beyond * just_beyond
            })
            .filter(|&(x, y)| w.get(x, y).is_empty())
            .count();
        assert!(
            cleared_beyond_crater > 0,
            "no sand just beyond the crater was picked up by the shockwave at all"
        );
    }

    #[test]
    fn the_shockwave_does_not_uproot_solid_material_beyond_the_crater() {
        // The shockwave (step 2.5) is scoped to loose material specifically
        // -- a blast can fling sand it never directly touched, but it
        // should not casually rip out a stone wall by the same mechanism,
        // only by actually clearing it (step 2) or breaking it structurally.
        let mut w = test_world();
        for y in 10..70 {
            for x in 10..70 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        let radius = 8;
        trigger(&mut w, &mut particles, 40, 40, radius, 200.0);

        let shockwave_radius = (radius as f32 * SHOCKWAVE_RADIUS_MULTIPLIER).round() as i32;
        let untouched_beyond_crater = ((40 - shockwave_radius)..=(40 + shockwave_radius))
            .flat_map(|y| ((40 - shockwave_radius)..=(40 + shockwave_radius)).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let (dx, dy) = (x - 40, y - 40);
                let dist2 = dx * dx + dy * dy;
                dist2 > radius * radius && dist2 <= shockwave_radius * shockwave_radius
            })
            .all(|(x, y)| w.get(x, y).material == material::STONE);
        assert!(untouched_beyond_crater, "the shockwave uprooted stone that was never inside the crater");
    }

    #[test]
    fn shockwave_pickup_chance_never_goes_negative_across_the_whole_annulus() {
        // A rounding mismatch bug: an earlier version divided by the
        // *rounded* `shockwave_radius - radius` while the caller's loop
        // admitted cells against the *continuous* `radius *
        // SHOCKWAVE_RADIUS_MULTIPLIER`. Whenever the multiplier rounds the
        // outer radius down, cells between the true and rounded edge still
        // pass the zone check but produced a negative chance -- caught
        // concretely at radius 3 (3 * 1.8 = 5.4, rounds to 5): a cell at
        // (dx, dy) = (5, 2), dist ~= 5.385, is inside the true continuous
        // radius but the old formula gave `1.0 - (5.385 - 3.0) / (5 - 3)` =
        // -0.19. `Rng::chance` treats negative as "never," so this bug
        // wouldn't panic, it would just silently narrow the annulus.
        for radius in 1..30 {
            let shockwave_radius = (radius as f32 * SHOCKWAVE_RADIUS_MULTIPLIER).round() as i32;
            for dy in -shockwave_radius..=shockwave_radius {
                for dx in -shockwave_radius..=shockwave_radius {
                    let dist2 = (dx * dx + dy * dy) as f32;
                    if dist2 <= (radius * radius) as f32 || dist2 > (radius as f32 * SHOCKWAVE_RADIUS_MULTIPLIER).powi(2) {
                        continue; // outside the annulus this radius actually admits
                    }
                    let chance = shockwave_pickup_chance(radius, dist2.sqrt());
                    assert!(
                        (0.0..=1.0).contains(&chance),
                        "radius={radius} dx={dx} dy={dy} dist={:.3} produced out-of-range chance {chance}",
                        dist2.sqrt()
                    );
                }
            }
        }
    }

    #[test]
    fn debris_is_thrown_away_from_the_epicentre_not_toward_it() {
        let mut w = test_world();
        for y in 30..50 {
            for x in 30..50 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 40, 40, 8, 200.0);

        // For every fast-moving particle, its velocity should point broadly
        // away from (40, 40), not toward it — checked via the cosine of the
        // angle between (position - centre) and velocity, which should be
        // strongly positive on average.
        //
        // The per-particle bound is a tolerance, not a strict `> 0.0`:
        // `debris_velocity` reads a *pressure gradient*, which already
        // legitimately grazes close to perpendicular for some positions near
        // a filled square's corner even with `DEBRIS_JITTER_STRENGTH` at 0 —
        // measured directly (temporarily zeroing the constant) at cos ~=
        // 0.14 (~81.9 degrees) for this exact scene, a thin pre-existing
        // margin that has nothing to do with jitter. `DEBRIS_JITTER_STRENGTH`
        // (added this section) then spends some of that margin on purpose —
        // the whole point is to scatter debris rather than have it launch in
        // lockstep — so a small number of already-marginal cells can graze
        // a few degrees past perpendicular. `COS_TOLERANCE` allows that
        // without allowing what this test actually exists to catch: a
        // genuine sign error that sends debris *backward into* the blast
        // (a strongly negative cosine, not a graze).
        const COS_TOLERANCE: f32 = -0.2;
        let mut checked = 0;
        let mut cos_sum = 0.0;
        for p in particles.iter() {
            let (dx, dy) = (p.x - 40.0, p.y - 40.0);
            let dist = (dx * dx + dy * dy).sqrt();
            let speed = (p.vx * p.vx + p.vy * p.vy).sqrt();
            if dist < 0.5 || speed < 0.5 {
                continue; // too close to the centre or too slow to judge direction
            }
            let cos = (dx * p.vx + dy * p.vy) / (dist * speed);
            assert!(
                cos > COS_TOLERANCE,
                "debris at ({}, {}) moving ({}, {}) points strongly toward the epicentre, not away (cos = {cos})",
                p.x,
                p.y,
                p.vx,
                p.vy
            );
            cos_sum += cos;
            checked += 1;
        }
        assert!(checked > 0, "no particle was far/fast enough to check direction on");
        // The population as a whole must skew strongly outward -- a real
        // sign-flip bug would show up here as a mean well below this, not
        // just one grazing cell.
        assert!(
            cos_sum / checked as f32 > 0.5,
            "debris does not skew outward on average: mean cos = {}",
            cos_sum / checked as f32
        );
    }

    #[test]
    fn an_explosion_in_a_corridor_does_not_throw_debris_through_the_wall() {
        // A vertical wall with a narrow corridor opening below it — debris at
        // the opening should be pushed along the corridor, not straight
        // through solid stone to the other side.
        let mut w = test_world();
        for y in 0..60 {
            w.set(60, y, Cell::new(material::STONE, 0));
        }
        // A one-cell gap in the wall at y=60..64 for the corridor.
        for x in 55..65 {
            w.set(x, 70, Cell::new(material::STONE, 0)); // floor of the corridor
        }
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 50, 65, 6, 150.0);

        // No particle should end up with a large positive vx (rightward,
        // through the wall at x=60) while still left of the wall.
        for p in particles.iter() {
            if p.x < 60.0 {
                assert!(
                    p.vx < 5.0,
                    "debris at x={} got a strong rightward push toward/through the wall: vx={}",
                    p.x,
                    p.vx
                );
            }
        }
    }

    #[test]
    fn an_explosion_ignites_material_just_beyond_the_cleared_radius() {
        // Oil spans a much wider area than the blast will clear, so there is
        // intact, flammable material left in the ring the fireball is
        // supposed to reach.
        let mut w = test_world();
        for y in 10..70 {
            for x in 10..70 {
                w.set(x, y, Cell::new(material::OIL, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 40, 40, 8, 150.0);

        // The clearing radius (8) must be empty — nothing left to ignite
        // there, which is exactly the bug this test is a regression guard
        // for: an earlier version tried to ignite this same inner region
        // *before* clearing it, and the clearing step then silently erased
        // every cell it had just set on fire.
        let inner_clear = (36..=44).all(|y| (36..=44).all(|x| w.get(x, y).is_empty()));
        assert!(inner_clear, "the clearing radius was not actually cleared");

        // Something in the ring beyond it (out to the fireball radius, 8 +
        // round(8*0.5) = 12) should be burning.
        let ring_burning = (25..55).any(|y| (25..55).any(|x| w.get(x, y).is_burning()));
        assert!(ring_burning, "explosion did not ignite the intact ring around the blast");
    }

    #[test]
    fn a_zero_radius_explosion_does_not_panic() {
        let mut w = test_world();
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 40, 40, 0, 150.0);
        // Reaching this line without panicking is the assertion.
    }

    /// An explosion must not turn a promoted body's reserved container cells
    /// into flying nothing.
    ///
    /// `Cell::is_empty()` is managed-aware: a body's container cells hold
    /// `material::EMPTY` but report as *not* empty, because for the callers
    /// that motivated that behaviour the question is "is this position
    /// available to use". An explosion's question is the other one — "is
    /// there material here to destroy" — so routing it through `is_empty()`
    /// made it treat those cells as destructible and spawn debris particles
    /// carrying `material::EMPTY`, which then land and write themselves back
    /// into the world.
    ///
    /// Latent rather than live today: nothing in production promotes a body
    /// (`127e177`). Found by review.
    #[test]
    fn an_explosion_does_not_spawn_debris_made_of_nothing() {
        let mut w = test_world();
        let mut particles = ParticleSystem::new();

        // A reserved container cell: materially empty, but managed. This is
        // exactly the shape `LiquidBody` rasterizes around its own edges.
        let container = Cell::EMPTY.with_managed(true);
        for x in 60..70 {
            w.set_owned(x, 60, container);
        }
        assert!(!w.get(64, 60).is_empty(), "test setup: a container cell reads as not-empty");
        assert_eq!(w.get(64, 60).material, material::EMPTY, "test setup: but holds no material");

        trigger(&mut w, &mut particles, 64, 60, 12, 4.0);

        let nothing: usize = particles.iter().filter(|p| p.material == material::EMPTY).count();
        assert_eq!(nothing, 0, "{nothing} debris particles were spawned with no material at all");
    }
}
