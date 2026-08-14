//! Free particles: off-grid debris with continuous position and velocity,
//! for explosions and splashes.
//!
//! A CA cell moves at most one cell per frame, by rule, not by an actual
//! velocity — that is what dirty rectangles and the moved-flag discipline are
//! built around, and it is a deliberate simplification the whole engine rests
//! on. It also means the CA grid cannot represent debris on a real ballistic
//! arc: something thrown by an explosion needs to leave the ground fast and
//! come down along a curve, not crawl outward one cell at a time. Free
//! particles are a completely separate system for exactly that case — a
//! `Vec<Particle>` with float position and velocity, gravity, and a landing
//! check against the CA grid that converts a particle back into a normal cell
//! the moment it would overlap something solid. Noita does the same split for
//! the same reason.
//!
//! Deliberately does **not** touch the M13 field (no wind drag, no coupling)
//! — that is a plausible enhancement, not something either of `particle.rs`'s
//! two callers (M15 explosion debris, general splash effects) need to exist
//! at all. Keeping the two systems uncoupled for now is the same judgement
//! call M14 made about not coupling every visited CA cell to the field: only
//! add a cross-system read when something concrete needs it.

use super::material::MaterialId;
use super::rng::Rng;
use super::world::World;

/// Cells per frame per frame. Chosen so a particle's fall visually resembles
/// the CA grid's roughly-constant-rate fall in the first few frames, then
/// keeps accelerating beyond it — the point of a real velocity is a proper
/// arc, not matching the grid's rate forever.
const GRAVITY: f32 = 0.15;

/// A speed cap, not a physical limit: without one, a large one-frame velocity
/// (a close explosion) could jump a particle clean over a thin wall in a
/// single step before substepping ever gets a chance to check for it.
const MAX_SPEED_PER_AXIS: f32 = 8.0;

/// Per-particle horizontal drag, drawn once at spawn and held for the
/// particle's whole flight — see `Particle::drag`'s own doc.
const MIN_DRAG: f32 = 0.985;
const MAX_DRAG: f32 = 1.0;

/// Per-particle gravity multiplier, drawn once at spawn — see
/// `Particle::gravity_scale`'s own doc.
const MIN_GRAVITY_SCALE: f32 = 0.9;
const MAX_GRAVITY_SCALE: f32 = 1.1;

/// Fraction of its speed a particle keeps for each cell of loose material it
/// punches through (`Particle::pierce`). Below 1.0 so debris decelerates
/// inside cover and comes to rest rather than crossing any thickness of it
/// for free — that falloff is what keeps a deeply buried charge from
/// throwing material to the surface as freely as a shallow one, which is the
/// distinction the mechanic exists to restore rather than erase.
const PIERCE_SPEED_RETENTION: f32 = 0.82;

/// How far `land` will search outward for an empty cell when a particle
/// comes to rest embedded in material — see its own comment. Small: this is
/// a last resort for a piercing particle, not a placement solver.
const NEAREST_EMPTY_SEARCH: i32 = 4;

pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub material: MaterialId,
    pub shade: u8,
    /// Multiplies `vx` every step, in `MIN_DRAG..MAX_DRAG` (`ranged`'s upper
    /// bound is exclusive; irrelevant at this granularity). Drawn once at
    /// spawn and held for life, not redrawn per frame — the same "stable
    /// decision, not reconsidered every tick" shape `Chunk::rng`'s own doc
    /// argues for, here so two particles launched with identical velocity
    /// (the common case: several cells in the same field tile read a
    /// near-identical pressure gradient, see `explosion.rs`'s own doc) don't
    /// keep tracing identical arcs forever and reading as a single moving
    /// block instead of a scatter.
    pub drag: f32,
    /// Multiplies the `GRAVITY` added to `vy` every step, in
    /// `MIN_GRAVITY_SCALE..MAX_GRAVITY_SCALE` (exclusive upper bound, same
    /// as `drag`). Same reasoning as `drag`,
    /// for the vertical axis: without this, particles that start identical
    /// keep falling at exactly the same rate and never visually separate.
    pub gravity_scale: f32,
    /// How many cells of *loose* material (`Powder`/`Liquid`) this particle
    /// may still punch through before it has to come to rest. Zero for
    /// ordinary debris, which lands on the first thing it touches.
    ///
    /// This exists because of a measured, structural gap: nothing in this
    /// engine can move through material. A CA cell only ever moves into
    /// empty space, and a free particle lands the instant its next substep
    /// is occupied — so an explosion buried more than a few cells deep has
    /// nowhere to throw anything. Measured on a flat sand bed at radius 20:
    /// cells thrown clear of the blast zone were 2 at 2 cells of cover, 33
    /// at 15, and **exactly 0** at 30 and beyond; in water, 0 at every depth
    /// including 2. The material that *did* move rose with depth (69 → 686
    /// cells) but all of it was collapse into the hole, not ejecta. The
    /// owner's report of the same thing: "you have to be so close to the
    /// edge to actually get material to blast around. it just doesn't happen
    /// if you're not really close."
    ///
    /// Piercing is deliberately restricted to loose material — a blast can
    /// throw grit out through sand or water, but debris does not tunnel
    /// through a stone wall, which is the same line `explosion.rs`'s
    /// shockwave step already draws for the same reason.
    pub pierce: u8,
}

/// Uniform in `min..max` (exclusive upper bound, inherited from `Rng::
/// below`'s own `0..n`). Not a method on `Rng` itself: this crate's `Rng`
/// deliberately stays a minimal integer/bool primitive (see its own module
/// doc), and the only two callers needing a ranged float are both here.
fn ranged(rng: &mut Rng, min: f32, max: f32) -> f32 {
    min + (rng.below(10_000) as f32 / 10_000.0) * (max - min)
}

/// Owns every free particle currently in flight. Lives alongside `World`
/// rather than inside it — particles are not part of the CA grid's own state,
/// and keeping them separate is what makes "does the field grid, or the CA
/// grid, need to know particles exist" a question with an easy default answer
/// of no.
pub struct ParticleSystem {
    particles: Vec<Particle>,
    /// This system's own RNG stream, used only to draw each new particle's
    /// `drag`/`gravity_scale` at spawn — the same "owns its own stream
    /// rather than sharing `World::rng`" shape `Chunk::rng` already uses,
    /// for the same reason: nothing here was ever required to be
    /// reproducible (see `rng.rs`'s own module doc), so there is no benefit
    /// to threading `&mut World` (or `&mut Rng`) through every `spawn`
    /// call site — `app.rs`'s `spawn_burst`, `render.rs`'s tests,
    /// `explosion.rs` — just to reach a shared generator.
    rng: Rng,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            rng: Rng::default(),
        }
    }

    pub fn spawn(&mut self, x: f32, y: f32, vx: f32, vy: f32, material: MaterialId, shade: u8) {
        self.spawn_piercing((x, y), (vx, vy), material, shade, 0);
    }

    /// `spawn`, plus a budget of loose cells the particle may punch through
    /// before it must land — see `Particle::pierce`. Kept as a separate
    /// entry point rather than a seventh parameter on `spawn`, because every
    /// caller other than `explosion::trigger` wants the plain
    /// land-on-first-contact behaviour and should not have to say so.
    ///
    /// Position and velocity are taken as pairs rather than four scalars
    /// purely to stay under clippy's argument-count limit, which the
    /// unpacked form trips.
    pub fn spawn_piercing(&mut self, (x, y): (f32, f32), (vx, vy): (f32, f32), material: MaterialId, shade: u8, pierce: u8) {
        let drag = ranged(&mut self.rng, MIN_DRAG, MAX_DRAG);
        let gravity_scale = ranged(&mut self.rng, MIN_GRAVITY_SCALE, MAX_GRAVITY_SCALE);
        self.particles.push(Particle {
            x,
            y,
            vx: vx.clamp(-MAX_SPEED_PER_AXIS, MAX_SPEED_PER_AXIS),
            vy: vy.clamp(-MAX_SPEED_PER_AXIS, MAX_SPEED_PER_AXIS),
            material,
            shade,
            drag,
            gravity_scale,
            pierce,
        });
    }

    pub fn len(&self) -> usize {
        self.particles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// For rendering: free particles are not CA cells, so nothing already
    /// iterates them — the renderer needs its own pass over this.
    pub fn iter(&self) -> impl Iterator<Item = &Particle> {
        self.particles.iter()
    }

    /// Advance every particle by one frame: apply gravity, move along the
    /// resulting velocity in sub-cell steps (so a fast particle cannot tunnel
    /// through a thin wall between one frame's position and the next), and
    /// convert to a normal CA cell the instant a step would land on
    /// non-empty ground.
    ///
    /// Run after the CA sweep, not before — see `App::update`'s ordering.
    /// Landing checks read `world.is_empty`, which should reflect this
    /// frame's fully-settled movement, not last frame's, or a particle could
    /// land inside material that has since moved out from under it.
    pub fn step(&mut self, world: &mut World) {
        // Drained and rebuilt rather than filtered in place: a landed
        // particle needs to write into `world` (`&mut`) while the loop is
        // simultaneously iterating `self.particles` — same shape of borrow
        // conflict this codebase has hit before (`MaterialRegistry::
        // resolve_references`, `fire::try_react`), same fix.
        let mut still_flying = Vec::with_capacity(self.particles.len());

        for mut particle in self.particles.drain(..) {
            particle.vy += GRAVITY * particle.gravity_scale;
            particle.vx *= particle.drag;
            particle.vx = particle.vx.clamp(-MAX_SPEED_PER_AXIS, MAX_SPEED_PER_AXIS);
            particle.vy = particle.vy.clamp(-MAX_SPEED_PER_AXIS, MAX_SPEED_PER_AXIS);

            if let Some(landing) = advance_and_check_landing(world, &mut particle) {
                land(world, &particle, landing);
                // Consumed — do not push back into `still_flying`.
            } else {
                still_flying.push(particle);
            }
        }

        self.particles = still_flying;
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Steps `particle`'s position forward by its own velocity, in increments of
/// at most one cell, stopping at the first cell that is not empty. Mutates
/// `particle.x`/`y` in place for every substep that stays clear, and returns
/// the world-cell coordinate to land at if one was found.
///
/// Substepping at a maximum of one cell per increment is what prevents
/// tunnelling: a particle moving 8 cells in a single unclamped step could
/// clear a one-cell-thick wall entirely between the frame it was on one side
/// and the frame it was found on the other, without a single sample ever
/// landing inside the wall to catch it.
///
/// Takes `&mut Particle`, not `&Particle` — an earlier version took a shared
/// reference and updated local `x`/`y` shadow variables that were never
/// written back, so a particle's recorded position never advanced on any
/// frame that did not end in a landing. Every test in this module failed the
/// same way (a particle "falling" that never actually moved), which is what
/// caught it — the bug was in what the function *did*, not in any of them.
fn advance_and_check_landing(world: &mut World, particle: &mut Particle) -> Option<(i32, i32)> {
    let (vx, vy) = (particle.vx, particle.vy);
    let distance = (vx * vx + vy * vy).sqrt();
    if distance <= 0.0 {
        return None;
    }
    let steps = distance.ceil().max(1.0) as i32;
    let (step_x, step_y) = (vx / steps as f32, vy / steps as f32);

    for _ in 0..steps {
        let (next_x, next_y) = (particle.x + step_x, particle.y + step_y);
        let (cell_x, cell_y) = (next_x.round() as i32, next_y.round() as i32);
        // Punch through loose material rather than landing on it, while a
        // pierce budget remains — see `Particle::pierce` for the measured
        // reason this exists at all.
        //
        // The particle carries its own grain straight through and writes
        // nothing to the CA grid on the way. An *exchange* version was
        // tried first — deposit the carried grain in the cell being left,
        // pick up the loose cell being entered — on the reasoning that it
        // conserves mass locally and drags a visible channel of material
        // outward. It does both, and it looks wrong: depositing at every
        // step riddles the pile with a trail of displaced grains and
        // scattered holes, which reads as static/corruption rather than
        // motion, and it transports material only one cell per exchange no
        // matter how far the particle actually flies — a bucket brigade,
        // not ejecta. Measured: it tripled "material disturbed" while
        // leaving "material thrown clear" at zero, which is the wrong one
        // of the two to move. Carrying the grain is also strictly cheaper
        // (no CA writes per pierced cell).
        //
        // Mass is still conserved: the grain stays on the particle until it
        // lands, and `land`'s nearest-empty search below guarantees a
        // particle that runs out of pierce while embedded still has
        // somewhere legal to come to rest.
        if particle.pierce > 0 && world.in_bounds(cell_x, cell_y) {
            let entering = world.get(cell_x, cell_y).material;
            let loose = matches!(
                world.materials.kind(entering),
                super::material::MaterialKind::Powder | super::material::MaterialKind::Liquid
            );
            if loose {
                particle.pierce -= 1;
                // Punching through costs speed, so debris slows to a stop
                // inside deep cover instead of crossing any thickness of it
                // for free — that falloff with depth is the point.
                particle.vx *= PIERCE_SPEED_RETENTION;
                particle.vy *= PIERCE_SPEED_RETENTION;
                particle.x = next_x;
                particle.y = next_y;
                continue;
            }
        }
        // `world.is_empty` deliberately, and deliberately *not* changed to a
        // raw material test the way `explosion::trigger` was. The question
        // here is "is this position available to land in", which is precisely
        // the question `Cell::is_empty()`'s managed-awareness exists to
        // answer: a promoted body's reserved container cell is not available,
        // and a particle converting itself into a CA cell there would disturb
        // the body. A review flagged this alongside the explosion case as an
        // "invisible wall along a body's outline", and that description is
        // accurate -- those cells draw as empty -- but blocking is the safer
        // of the two behaviours and the alternative demotes bodies from a
        // stray grain. Left as-is on purpose; revisit if bodies ever promote
        // in production and the outline is actually visible in play.
        if !world.in_bounds(cell_x, cell_y) || !world.is_empty(cell_x, cell_y) {
            return Some((cell_x, cell_y));
        }
        particle.x = next_x;
        particle.y = next_y;
    }
    None
}

fn land(world: &mut World, particle: &Particle, at: (i32, i32)) {
    // Land one cell short if the target itself is occupied or out of bounds —
    // otherwise the particle would overwrite whatever it just collided with,
    // rather than coming to rest against it the way falling CA material does.
    let (tx, ty) = if world.in_bounds(at.0, at.1) && world.is_empty(at.0, at.1) {
        at
    } else {
        (particle.x.round() as i32, particle.y.round() as i32)
    };
    if world.in_bounds(tx, ty) && world.is_empty(tx, ty) {
        world.set(tx, ty, super::cell::Cell::new(particle.material, particle.shade));
        return;
    }
    // Neither position is available. Before this, the particle was simply
    // dropped here — acceptable when the only way to reach this branch was
    // spawning already overlapping something, but not once `Particle::
    // pierce` exists: a particle that runs out of pierce mid-way through a
    // sand pile is embedded by construction, both positions are occupied,
    // and dropping it would silently delete a grain on *every* deeply
    // buried blast rather than in a rare edge case. Search outward for
    // somewhere legal to come to rest instead.
    //
    // Bounded deliberately, and small: this is a last resort for a particle
    // that is already inside material, not a general placement solver, and
    // an unbounded search would scan the whole world for the genuinely
    // hopeless case (a particle inside a solid block with no gap at all).
    // Ring by ring, so the grain surfaces at the nearest opening rather than
    // teleporting to whichever cell happens to come first in scan order.
    for ring in 1..=NEAREST_EMPTY_SEARCH {
        for dy in -ring..=ring {
            for dx in -ring..=ring {
                if dx.abs() != ring && dy.abs() != ring {
                    continue; // interior of this ring was covered by a previous one
                }
                let (nx, ny) = (tx + dx, ty + dy);
                if world.in_bounds(nx, ny) && world.is_empty(nx, ny) {
                    world.set(nx, ny, super::cell::Cell::new(particle.material, particle.shade));
                    return;
                }
            }
        }
    }
    // Genuinely nowhere to go — dropped, as before.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::material;
    use crate::sim::world::World;

    fn test_world() -> World {
        let mut w = World::new(Rect::new(0, 0, 63, 63));
        for x in 0..64 {
            w.set(x, 63, super::super::cell::Cell::new(material::STONE, 0));
        }
        w
    }

    #[test]
    fn a_particle_falls_under_gravity() {
        let mut w = test_world();
        let mut ps = ParticleSystem::new();
        ps.spawn(30.0, 10.0, 0.0, 0.0, material::SAND, 0);

        let start_y = ps.iter().next().unwrap().y;
        ps.step(&mut w);
        let after_y = ps.iter().next().unwrap().y;
        assert!(after_y > start_y, "particle did not fall: {start_y} -> {after_y}");
    }

    #[test]
    fn a_particle_lands_and_becomes_a_ca_cell() {
        let mut w = test_world();
        let mut ps = ParticleSystem::new();
        ps.spawn(30.0, 10.0, 0.0, 0.0, material::SAND, 2);

        for _ in 0..200 {
            ps.step(&mut w);
            if ps.is_empty() {
                break;
            }
        }
        assert!(ps.is_empty(), "particle never landed after 200 frames");

        // It must have become a real CA sand cell somewhere above the floor,
        // not vanished.
        let landed = (0..63).any(|y| w.get(30, y).material == material::SAND);
        assert!(landed, "particle disappeared instead of becoming a CA cell");
    }

    #[test]
    fn a_fast_particle_does_not_tunnel_through_a_thin_wall() {
        let mut w = test_world();
        // A one-cell-thick horizontal wall the particle must cross.
        for x in 0..64 {
            w.set(x, 40, super::super::cell::Cell::new(material::STONE, 0));
        }
        let mut ps = ParticleSystem::new();
        // A large downward velocity in one shot — without substepping this
        // could clear the wall in a single step.
        ps.spawn(30.0, 10.0, 0.0, 25.0, material::SAND, 0);

        for _ in 0..10 {
            ps.step(&mut w);
        }
        assert!(ps.is_empty(), "particle should have landed on the wall by now");
        // Landed at or above the wall (y <= 40), not below it.
        let landed_above_or_on_wall = (0..=40).any(|y| w.get(30, y).material == material::SAND);
        assert!(landed_above_or_on_wall, "particle tunnelled through the wall");
    }

    #[test]
    fn a_particle_at_rest_stays_put_until_gravity_moves_it() {
        // Zero velocity is a degenerate case for the distance/steps math in
        // advance_and_check_landing (division by a zero step count) — must
        // not panic, and must not silently teleport.
        let mut w = test_world();
        let mut ps = ParticleSystem::new();
        ps.spawn(30.0, 10.0, 0.0, 0.0, material::SAND, 0);
        ps.step(&mut w);
        assert_eq!(ps.len(), 1, "a stationary particle should not vanish on its first step");
    }

    #[test]
    fn velocity_is_clamped_on_spawn_and_while_flying() {
        let mut w = test_world();
        let mut ps = ParticleSystem::new();
        ps.spawn(30.0, 10.0, 1000.0, -1000.0, material::SAND, 0);
        let v = {
            let p = ps.iter().next().unwrap();
            (p.vx, p.vy)
        };
        assert!(v.0.abs() <= MAX_SPEED_PER_AXIS);
        assert!(v.1.abs() <= MAX_SPEED_PER_AXIS);
        ps.step(&mut w);
        // Still clamped after gravity is added in step().
        let still_clamped = ps.iter().next().map(|p| p.vy.abs() <= MAX_SPEED_PER_AXIS).unwrap_or(true);
        assert!(still_clamped);
    }

    #[test]
    fn particles_spawned_with_identical_velocity_diverge_over_time() {
        // Before this section, every particle shared one flat `GRAVITY`
        // constant with no per-particle variation, so two particles launched
        // identically kept tracing exactly identical arcs forever -- the
        // "lockstep falling" `drag`/`gravity_scale` exist to break. `vx: 0.0`
        // deliberately, so this isolates `gravity_scale`'s effect on the
        // vertical fall from `drag`'s (which has nothing to act on when `vx`
        // never leaves zero).
        let mut w = test_world();
        let mut ps = ParticleSystem::new();
        ps.spawn(10.0, 5.0, 0.0, -1.0, material::SAND, 0);
        ps.spawn(20.0, 5.0, 0.0, -1.0, material::SAND, 0);

        for _ in 0..30 {
            ps.step(&mut w);
        }

        let ys: Vec<f32> = ps.iter().map(|p| p.y).collect();
        assert_eq!(ys.len(), 2, "expected both particles still flying after 30 frames");
        assert!(
            (ys[0] - ys[1]).abs() > 0.01,
            "two particles spawned with identical velocity fell by the exact same amount: {ys:?}"
        );
    }

    #[test]
    fn landing_on_an_occupied_target_does_not_overwrite_it() {
        let mut w = test_world();
        w.set(30, 20, super::super::cell::Cell::new(material::STONE, 0));
        let mut ps = ParticleSystem::new();
        // Spawned one cell above an obstacle, moving straight down slowly so
        // it lands exactly at the boundary.
        ps.spawn(30.0, 19.0, 0.0, 0.5, material::SAND, 0);

        for _ in 0..20 {
            ps.step(&mut w);
            if ps.is_empty() {
                break;
            }
        }
        assert_eq!(w.get(30, 20).material, material::STONE, "the obstacle was overwritten");
    }

    #[test]
    fn many_particles_conserve_material_count() {
        let mut w = test_world();
        let mut ps = ParticleSystem::new();
        for i in 0..30 {
            ps.spawn(10.0 + i as f32, 5.0, (i % 5) as f32 - 2.0, 0.0, material::SAND, 0);
        }
        for _ in 0..300 {
            ps.step(&mut w);
        }
        assert!(ps.is_empty(), "particles left in flight after 300 frames");

        let landed = (0..64).flat_map(|y| (0..64).map(move |x| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == material::SAND)
            .count();
        assert_eq!(landed, 30, "expected 30 landed sand cells, found {landed}");
    }
}
