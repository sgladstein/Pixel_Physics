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

/// Base debris speed at `strength = 100`, before the centre/edge falloff
/// below. Picked by eye against `App::spawn_burst`'s 3.0–6.0 range for a
/// visually comparable "thrown," not measured against anything physical.
const SPEED_PER_STRENGTH: f32 = 0.05;

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

    // 2. Clear the blast radius, converting to debris — with a chance that
    // falls off toward the edge, matching the plan's "free particles OR
    // vacuum" rather than making every single cleared cell a particle. A
    // point-blank hit at the centre reliably throws debris; the outer edge of
    // the same blast mostly just clears material without launching it, which
    // reads as the difference between "shattered" and "merely destroyed."
    let r2 = (radius * radius) as f32;
    for y in (cy - radius)..=(cy + radius) {
        for x in (cx - radius)..=(cx + radius) {
            let (dx, dy) = (x - cx, y - cy);
            let dist2 = (dx * dx + dy * dy) as f32;
            if dist2 > r2 {
                continue;
            }
            let cell = world.get(x, y);
            // Bedrock is the world's own boundary material and never
            // destructible by anything, the same way it is never a target
            // for painting (`World::paint_circle`) or ignition.
            if cell.is_empty() || cell.material == material::BEDROCK {
                continue;
            }

            let becomes_debris = world.rng.chance(1.0 - (dist2 / r2).sqrt());
            if becomes_debris {
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

    if mag > 0.01 {
        (gx / mag * speed, gy / mag * speed)
    } else {
        // No usable gradient (dead centre, or walled in on every side) —
        // fall back to a purely radial push away from the epicentre so a
        // symmetric position still gets thrown *somewhere* rather than
        // sitting motionless in an otherwise fully cleared blast radius.
        let (dx, dy) = ((x - cx) as f32, (y - cy) as f32);
        let d = (dx * dx + dy * dy).sqrt().max(1.0);
        (dx / d * speed, dy / d * speed)
    }
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
    fn debris_is_thrown_away_from_the_epicentre_not_toward_it() {
        let mut w = test_world();
        for y in 30..50 {
            for x in 30..50 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let mut particles = ParticleSystem::new();
        trigger(&mut w, &mut particles, 40, 40, 8, 200.0);

        // For every fast-moving particle, its velocity should point away
        // from (40, 40), not toward it — checked via the dot product of
        // (position - centre) and velocity, which should be positive for
        // outward motion.
        let mut checked = 0;
        for p in particles.iter() {
            let (dx, dy) = (p.x - 40.0, p.y - 40.0);
            let dist = (dx * dx + dy * dy).sqrt();
            let speed = (p.vx * p.vx + p.vy * p.vy).sqrt();
            if dist < 0.5 || speed < 0.5 {
                continue; // too close to the centre or too slow to judge direction
            }
            let dot = dx * p.vx + dy * p.vy;
            assert!(
                dot > 0.0,
                "debris at ({}, {}) moving ({}, {}) points toward the epicentre, not away",
                p.x,
                p.y,
                p.vx,
                p.vy
            );
            checked += 1;
        }
        assert!(checked > 0, "no particle was far/fast enough to check direction on");
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
}
