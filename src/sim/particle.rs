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
//! **Now couples to the M13 field** (`WIND_DRAG`), reversing this module's
//! original "only add a cross-system read when something concrete needs it"
//! position — something concrete turned up. The field's velocity channel had
//! no consumer that displaced anything at all, so an explosion's pressure
//! impulse propagated and reflected across the world while moving nothing;
//! `update_gas`'s wind bias was the first fix for that and this is the
//! second. The original judgement call was right when it was made and is
//! recorded here rather than deleted, because the reasoning ("only couple
//! when a caller needs it") still holds — it is the premise that changed,
//! not the rule.

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

/// How strongly a free particle is dragged toward the ambient field
/// velocity each step — see the call site for why this is a *relative*
/// drag rather than an added force. Small: debris should be nudged by a
/// blast's own outflow and by wind while it hangs, not steered by it.
const WIND_DRAG: f32 = 0.03;

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
    /// The source cell's `Cell::aux`, carried so a thrown cell lands worth
    /// what it was worth. Written back by `land` **only when the landing
    /// material declares `Material::worth_in_aux`** — see there for why the
    /// gate is on the flag and not on the value.
    ///
    /// Before this field existed, `land` wrote `Cell::new(material, shade)`
    /// and the stamp was simply dropped. Since S3 a `corpse` cell carries
    /// what it is worth to eat in `aux`, so a blasted corpse fell through to
    /// `corpse.ron`'s `food_energy` fallback: **1,020 to 120, an 8.5x silent
    /// loss** on the one material whose value is per-cell. Measured on the
    /// reproduction below at radius 20: 114 cells thrown, every one of them
    /// landing at 120 — 102,600 energy out of one blast, invisible to every
    /// existing guard (`max_standing_meat` is a `<=` bound, and biomass is
    /// asserted monotone non-increasing, so both pass on a loss).
    ///
    /// **`rigid.rs`'s `BodyCell` has the same shape and deliberately does
    /// *not* do this.** It only ever holds `Solid`/`Plant`, and its `aux = 0`
    /// is load-bearing: `aux` on those is the organism/cell-type packing, so
    /// carrying it would let a landing body silently re-attach to an
    /// organism it is no longer part of. The asymmetry is intentional; do
    /// not "fix" it by symmetry with this field.
    ///
    /// Note `Cell::aux` is a tagged union with conventions that point
    /// opposite ways — `0` means *full* on a `Liquid` and *dry* on a
    /// `Powder` — which is the other reason this is gated on the landing
    /// material rather than copied unconditionally.
    pub aux: u16,
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

    /// Throw a particle that has no source cell — a material and a shade
    /// chosen by a caller rather than lifted off the grid (the brush's
    /// debug burst). `aux` is 0: there is no stamp to carry.
    ///
    /// **A caller that *does* hold a `Cell` must use `spawn_from_cell`**, or
    /// it silently reprices whatever it threw. That is bug Z2, and the reason
    /// the cell-sourced path takes the `Cell` itself rather than an `aux`
    /// parameter it would be equally easy to forget.
    pub fn spawn(&mut self, x: f32, y: f32, vx: f32, vy: f32, material: MaterialId, shade: u8) {
        self.push((x, y), (vx, vy), material, shade, 0, 0);
    }

    /// Throw a cell that is being taken off the grid, carrying everything it
    /// was — material, shade and `Cell::aux` — plus a budget of loose cells
    /// it may punch through before it must land (`Particle::pierce`; 0 for
    /// ordinary debris, which lands on first contact).
    ///
    /// Takes the `Cell` rather than its parts precisely because the parts
    /// are what got dropped: every caller here already had the `Cell` in
    /// hand and passed two of its three fields.
    ///
    /// Position and velocity are pairs rather than four scalars purely to
    /// stay under clippy's argument-count limit, which the unpacked form
    /// trips.
    pub fn spawn_from_cell(&mut self, (x, y): (f32, f32), (vx, vy): (f32, f32), cell: super::cell::Cell, pierce: u8) {
        self.push((x, y), (vx, vy), cell.material, cell.shade, cell.aux(), pierce);
    }

    fn push(&mut self, (x, y): (f32, f32), (vx, vy): (f32, f32), material: MaterialId, shade: u8, aux: u16, pierce: u8) {
        let drag = ranged(&mut self.rng, MIN_DRAG, MAX_DRAG);
        let gravity_scale = ranged(&mut self.rng, MIN_GRAVITY_SCALE, MAX_GRAVITY_SCALE);
        self.particles.push(Particle {
            x,
            y,
            vx: vx.clamp(-MAX_SPEED_PER_AXIS, MAX_SPEED_PER_AXIS),
            vy: vy.clamp(-MAX_SPEED_PER_AXIS, MAX_SPEED_PER_AXIS),
            material,
            shade,
            aux,
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
            // Wind drag, reversing this module's own original "deliberately
            // does not touch the M13 field" decision now that the field has
            // a reason to push things (see `update_gas`).
            //
            // Written as drag toward the wind's own velocity, **not** as a
            // force added to the particle's. That matters more here than it
            // looks: field velocity around a fresh blast peaks near 86 while
            // `MAX_SPEED_PER_AXIS` clamps particles to 8, so any formula
            // that adds a fraction of the raw wind would swamp the launch
            // direction `debris_velocity` carefully computed — and once the
            // shock reflects, would send debris back into the crater, which
            // is precisely what `debris_is_thrown_away_from_the_epicentre_
            // not_toward_it` exists to catch. A relative-velocity drag is
            // self-limiting instead: fast debris moving with the blast feels
            // almost nothing, and only slow or stalled debris gets carried.
            // `World::field_at` directly, not `CellSurface::field_wind_at`:
            // this module holds a real `&mut World`, not a generic surface,
            // so there is nothing to gain from pulling the trait into scope.
            let wind = world.field_at(particle.x.round() as i32, particle.y.round() as i32);
            particle.vx += (wind.vx - particle.vx) * WIND_DRAG;
            particle.vy += (wind.vy - particle.vy) * WIND_DRAG;
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

/// The cell a landing particle becomes.
///
/// **`aux` rides along only when the landing material declares
/// `Material::worth_in_aux`.** Copying it unconditionally would be wrong in
/// both directions: `Cell::aux` is a tagged union whose conventions point
/// opposite ways (`0` is *full* on a `Liquid` and *dry* on a `Powder`), and
/// on `Solid`/`Plant` it is the organism packing — so a thrown wet soil
/// grain would land claiming to be food, and a thrown plant cell would land
/// claiming to belong to an organism. `worth_in_aux` is the one convention
/// where the number means "what this cell is worth to eat", and it is the
/// only one a free particle has any business preserving.
///
/// Gated on the **flag, not the value**: an unstamped corpse (`aux == 0`) is
/// a real case — `fire.rs`'s burnout writes one, deliberately — and it must
/// stay unstamped rather than be skipped into a fallback by a zero test.
/// `creature::food_value` is what turns 0 back into the material fallback,
/// and it should keep being the only place that decides that.
fn landed_cell(world: &World, particle: &Particle) -> super::cell::Cell {
    let cell = super::cell::Cell::new(particle.material, particle.shade);
    if world.materials.get(particle.material).worth_in_aux {
        return cell.with_aux(particle.aux);
    }
    // **The shipping behaviour since 2026-08-27.** `PARTICLE_AUX_MAX=0`
    // restores `Cell::new`'s zero, for measurement only -- the four-arm
    // ablation in `Reports/structural-support-model.md` §6.4 is what set this
    // default and is worth being able to re-run. **The sibling case is
    // `rigid::settle`, and neither alone is enough**: closing one leaves the
    // damaged region relaxing off the other, 23% and 1% against 99.5%
    // together.
    //
    // The doc above enumerates two `aux` conventions a free particle must
    // not carry and concludes it should carry none. On an inert `Solid`
    // there is a **third**, and dropping to `Cell::new`'s 0 does not decline
    // to make a claim there -- it makes the strongest one available:
    // `structural.rs` reads `aux == 0` as *at an anchor*. A thrown rock
    // therefore lands claiming to be bedrock-adjacent, and the massif around
    // it relaxes downhill off that.
    //
    // Trapped at the write seam (`World::report_false_anchor`): twelve
    // consecutive false anchors on the two frames after a radius-20 charge,
    // every one of them `empty aux 0 -> stone aux 0` beside `stone:2405`,
    // and every backtrace `ParticleSystem::step`.
    //
    // `u16::MAX` is the honest value and the cheap one -- "no known path,
    // earn one". `tick` relaxes it to `min(neighbour + step)` in a *single*
    // round, because an improvement is one step whatever its size, whereas
    // climbing out of a false 0 costs one round per unit of the field's
    // depth (~2,400 at the shipped size).
    let particle_aux_max = {
        use std::sync::OnceLock;
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var("PARTICLE_AUX_MAX").map(|v| v != "0").unwrap_or(true))
    };
    if particle_aux_max && super::structural::is_body_material(world, particle.material) {
        return cell.with_aux(u16::MAX);
    }
    cell
}

/// Write a landed particle into the grid **and tell the structural system it
/// arrived**.
///
/// `land` wrote through `World::set` and scheduled nothing, so a landed
/// `Solid` or `Plant` cell was invisible to `structural::tick` until some
/// unrelated disturbance happened to wake it. Whatever `landed_cell` chooses
/// to write then stands unexamined: `Cell::new`'s `aux 0` reads as
/// *bedrock-adjacent* for as long as nothing asks, and a `u16::MAX` reads as
/// *no path at all* for just as long. Neither is a value a cell should get to
/// keep without being asked, and `rigid::settle` -- the other way a piece of
/// the world comes to rest -- has scheduled around both its aimed-at and its
/// landed positions since the "stone hanging in open sky over a pond" bug.
///
/// **`_around`, not the cell alone**, for `settle`'s reason: the neighbours
/// have just gained a possible support, and the cell may have come to rest
/// somewhere it cannot hold itself.
///
/// **Gated on body material**, which is the whole cost story. A blast throws
/// far more sand, water and smoke than rock, and none of it participates in
/// the support field -- `schedule_structural_check` no-ops on those anyway,
/// but only after five heap pushes and a chunk lookup each. `is_body_material`
/// is a `Vec` index on a value already in hand.
fn place_landed(world: &mut World, x: i32, y: i32, cell: super::cell::Cell) {
    world.set(x, y, cell);
    if super::structural::is_body_material(world, cell.material) {
        world.schedule_structural_check_around(x, y);
    }
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
        place_landed(world, tx, ty, landed_cell(world, particle));
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
                    place_landed(world, nx, ny, landed_cell(world, particle));
                    return;
                }
            }
        }
    }
    // Genuinely nowhere to go — dropped, as before.
}

/// Upward speed a splash droplet leaves at, and how much of that it also
/// carries sideways at the outermost droplet of a burst.
///
/// Modest against `MAX_SPEED_PER_AXIS` (8): a droplet that clears a pool by
/// twenty cells is not a splash, it is an eruption, and the pool it left is
/// visibly short of a cell when it lands somewhere else entirely.
const SPLASH_UP: f32 = 2.2;
const SPLASH_OUT: f32 = 1.1;

/// How many droplets one splash site may throw. Each one is a whole cell
/// taken out of the pool, so this is a bound on how much water is in the
/// air at once as much as it is a look decision.
const SPLASH_DROPLETS: i32 = 3;

/// The reported strength at which a splash becomes a *crown* rather than a
/// single drop — see `CellSurface::report_splash`.
const SPLASH_CROWN_STRENGTH: f32 = 0.5;

/// Turn this frame's splash candidates (`World::splash_sites`) into
/// droplets, taking each droplet's water **out of the pool it came from**.
///
/// **The removal and the launch are in the same function, and that is the
/// whole design.** `land` writes a full cell wherever a droplet comes down,
/// so a droplet that was not debited somewhere is water manufactured. The
/// sweep therefore only reports sites (`CellSurface::report_splash`) and
/// never removes anything, which is what lets `examples/ascii.rs` and every
/// unit test step the world with no `ParticleSystem` at all and lose
/// nothing.
///
/// Every site is re-checked here rather than trusted, because a site is a
/// frame old by the time this runs and the cell it names may have drained,
/// frozen, been buried or moved: only a near-full liquid cell with air
/// directly above it is taken.
///
/// Run after the CA sweep and before `ParticleSystem::step`, the same
/// ordering and for the same reason as everything else in `App::update`.
pub fn throw_splashes(world: &mut World, particles: &mut ParticleSystem) {
    if world.splash_sites.is_empty() {
        return;
    }
    // Taken rather than borrowed: the loop needs `&mut World` to empty the
    // cells it takes. Same borrow shape as `ParticleSystem::step`'s drain.
    let sites = std::mem::take(&mut world.splash_sites);
    for (x, y, strength) in sites {
        // Droplets fan out from the site, one per column, so a burst reads
        // as a crown rather than as three grains stacked in one place --
        // **unless the event is small**, in which case it is one drop.
        // Three whole cells of water leaving a simmering pan every time a
        // bubble bursts is a fountain, not a simmer.
        let fan = if strength >= SPLASH_CROWN_STRENGTH { SPLASH_DROPLETS / 2 } else { 0 };
        for offset in -fan..=fan {
            let dx = x + offset;
            let cell = world.get(dx, y);
            if world.materials.kind(cell.material) != super::material::MaterialKind::Liquid {
                continue;
            }
            if super::update::liquid_fill(cell) < SPLASH_MIN_FILL {
                continue;
            }
            // **Air above, or nothing but the pool's own film.**
            //
            // The plain air test is what a crown wants -- a droplet needs
            // somewhere to go -- and on its own it refuses every site on a
            // *settled* pool, because a settled pool's top row is the
            // remainder of its volume and is never full. That is already
            // recorded as the reason
            // `a_body_entering_water_at_speed_reports_a_crown_and_one_sliding_in_does_not`
            // counts sites rather than droplets, and it made
            // `fire.rs`'s simmering-surface pops fire **zero** times on
            // `scene=simmer`: every site correctly reported, every one
            // declined.
            //
            // A part-filled cell of the same liquid is not a lid. Taking
            // the full cell under it conserves exactly as before -- a whole
            // cell out, a whole cell in when the droplet lands -- and the
            // film falls into the hole on the next sweep.
            if !world.in_bounds(dx, y - 1) {
                continue;
            }
            let cover = world.get(dx, y - 1);
            let filmed = cover.material == cell.material && super::update::liquid_fill(cover) < SPLASH_MIN_FILL;
            if cover.material != super::material::EMPTY && !filmed {
                continue;
            }
            // Debit first. If anything below this line fails, the cell is
            // already gone and the water is lost -- `spawn` cannot fail,
            // which is why the order is safe, and why it is worth saying
            // so here rather than leaving the next reader to check.
            world.set(dx, y, super::cell::Cell::EMPTY);
            world.splashes_thrown += 1;
            particles.spawn_from_cell(
                (dx as f32, (y - 1) as f32),
                (SPLASH_OUT * offset as f32 * strength, -SPLASH_UP * strength),
                cell,
                0,
            );
        }
    }
}

/// The fill a cell must hold before a droplet may be taken out of it --
/// full, for the reason `update::SPLASH_MIN_FILL` documents. The sweep
/// checks it when it reports a site and this checks it again when it acts a
/// frame later, and the two have to agree.
const SPLASH_MIN_FILL: u16 = super::material::LIQUID_FULL;

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

    /// Water in the whole world, in cell-equivalents — fill, not occupancy
    /// (`CLAUDE.md`'s metric traps), plus whatever is still in the air.
    ///
    /// **Droplets in flight have to be counted or the metric is a lie**: a
    /// splash takes a cell out of the grid and gives it back several frames
    /// later, so a grid-only census reads a genuine conservation as a loss
    /// on exactly the frames the mechanism is doing something.
    fn water_in_world(w: &World, ps: &ParticleSystem) -> f64 {
        let full = material::LIQUID_FULL as f64;
        let mut total = 0.0;
        for y in 0..64 {
            for x in 0..64 {
                let cell = w.get(x, y);
                if cell.material == material::WATER {
                    total += super::super::update::liquid_fill(cell) as f64 / full;
                }
            }
        }
        total + ps.iter().filter(|p| p.material == material::WATER).count() as f64
    }

    #[test]
    fn a_splash_takes_its_droplets_out_of_the_pool() {
        // **The conservation claim, as a paired comparison** (`CLAUDE.md`):
        // the same scene run twice, once with the splash sites drained and
        // once with them ignored, so everything the mechanism is *not*
        // about -- settling, the last of the fill shuffling sideways --
        // cancels. A single run against a remembered number would be
        // measuring the pool's own drift.
        //
        // The failure this exists for is manufacture, and it is a real
        // shape: `land` writes a *whole* cell wherever a droplet comes
        // down, so a droplet taken from a cell holding 900 comes back as
        // 1,000. Measured on `filmstrip scene=splash` before the fill gate
        // was tightened to full: 30,351.3 cell-equivalents at the start and
        // 30,363.3 at the end over 499 droplets.
        let run = |throw: bool| {
            let mut w = World::new(Rect::new(0, 0, 63, 63));
            for x in 0..64 {
                w.set(x, 63, super::super::cell::Cell::new(material::STONE, 0));
            }
            // Walls, or the pool drains off the sides and the census is
            // measuring the world's edges rather than the splash.
            for y in 40..63 {
                w.set(0, y, super::super::cell::Cell::new(material::STONE, 0));
                w.set(63, y, super::super::cell::Cell::new(material::STONE, 0));
            }
            for x in 1..63 {
                for y in 45..63 {
                    w.set(x, y, super::super::cell::Cell::new(material::WATER, 0));
                }
            }
            // Loose grains rather than a block: a splash site needs air
            // above the displaced water, which a solid slab never leaves.
            for x in 8..56 {
                for y in 10..40 {
                    if super::super::rng::jitter(x, y) < 0.06 {
                        w.set(x, y, super::super::cell::Cell::new(material::SAND, 0));
                    }
                }
            }
            let mut ps = ParticleSystem::new();
            let before = water_in_world(&w, &ps);
            for _ in 0..400 {
                crate::sim::parallel::step(&mut w);
                if throw {
                    throw_splashes(&mut w, &mut ps);
                }
                ps.step(&mut w);
            }
            (before, water_in_world(&w, &ps), w.splashes_thrown)
        };

        let (before, after, thrown) = run(true);
        let (control_before, control_after, control_thrown) = run(false);
        println!("splashing: {before:.2} -> {after:.2} over {thrown} droplets; control {control_before:.2} -> {control_after:.2}");
        assert_eq!(control_thrown, 0, "the control threw droplets; it is not a control");
        assert!(thrown > 0, "no droplet was ever thrown, so this asserts nothing about splashing");
        // Against the control, not against the starting value: the pool
        // settles a little either way, and the claim is that splashing does
        // not change how much water there is. A tenth of a cell per droplet
        // in either direction is far tighter than the whole-cell error the
        // gate exists to prevent.
        let per_droplet = (after - control_after).abs() / thrown as f64;
        assert!(
            per_droplet < 0.1,
            "splashing moved the tally by {per_droplet:.3} cells per droplet against the control ({after:.2} vs {control_after:.2} over {thrown})"
        );
    }

    #[test]
    fn a_splash_never_takes_a_cell_that_is_not_full() {
        // The gate on its own, at unit scale, because the integration test
        // above can only see the sum. A half-full cell must be left alone:
        // it would come back whole.
        let mut w = test_world();
        w.set(30, 40, super::super::cell::Cell::new(material::WATER, 0).with_aux(500));
        w.splash_sites.push((30, 40, 1.0));
        let mut ps = ParticleSystem::new();
        throw_splashes(&mut w, &mut ps);
        assert_eq!(w.get(30, 40).material, material::WATER, "a half-full cell was thrown as a whole droplet");
        assert_eq!(ps.len(), 0);
        assert_eq!(w.splashes_thrown, 0);

        // ...and the full one beside it is taken, so the refusal above is
        // the gate doing its job rather than the whole path being dead.
        w.set(30, 40, super::super::cell::Cell::new(material::WATER, 0));
        w.splash_sites.push((30, 40, 1.0));
        throw_splashes(&mut w, &mut ps);
        assert_eq!(w.get(30, 40).material, material::EMPTY, "a full cell at a free surface was not thrown");
        assert_eq!(ps.len(), 1);
        assert_eq!(w.splashes_thrown, 1);
    }

    /// **A corpse thrown by a blast must land worth what it was worth.**
    /// The reproduction for `Reports/open-bugs-handoff.md` bug Z2, driven
    /// through the *real* explosion path rather than through a hand-built
    /// particle, because the bug is a dropped field in the hand-off between
    /// `explosion::trigger` and `land`, and a hand-built particle would test
    /// the fix against its own literal.
    ///
    /// Why no existing guard sees this: `EnergyLedger::max_standing_meat` is
    /// a `<=` bound, so meat quietly going missing passes it, and
    /// `creature_biomass` is asserted monotone non-increasing, which a loss
    /// also satisfies. The census here is `creature::food_value` over the
    /// world -- the quantity an animal actually eats.
    #[test]
    fn a_blasted_corpse_lands_worth_what_it_was_worth() {
        use super::super::creature::food_value;

        // Big enough that thrown debris has somewhere to land inside the
        // world, and a solid floor so nothing simply falls out of it.
        let mut w = World::new(Rect::new(0, 0, 127, 127));
        for x in 0..128 {
            w.set(x, 127, super::super::cell::Cell::new(material::STONE, 0));
        }
        let corpse = w.materials.id_of("corpse").expect("corpse material");
        assert!(
            w.materials.get(corpse).worth_in_aux,
            "test setup: corpse must price itself per cell, or this measures nothing"
        );

        // A stamped corpse worth far more than `corpse.ron`'s fallback, so a
        // dropped stamp is unmistakable rather than a rounding difference.
        // 1,020 is bug Z2's own figure.
        const WORTH: u16 = 1_020;
        let fallback = w.materials.get(corpse).food_energy;
        assert!(
            (WORTH as f32) > fallback * 2.0,
            "test setup: the stamp ({WORTH}) has to be well clear of the {fallback} fallback or a dropped stamp is invisible"
        );

        // A slab of stamped corpse sitting on the floor, wide enough to
        // reach past the vaporize core (`Tuning::vaporize_fraction`) so a
        // real share of it is *thrown* rather than simply cleared.
        let (cx, cy, radius) = (64i32, 100i32, 20i32);
        for y in 96..104 {
            for x in 44..84 {
                w.set(x, y, super::super::cell::Cell::new(corpse, 0).with_aux(WORTH));
            }
        }

        let census = |w: &World| -> (f32, usize) {
            let (mut total, mut count) = (0.0, 0);
            for y in 0..128 {
                for x in 0..128 {
                    let c = w.get(x, y);
                    if c.material == corpse {
                        total += food_value(w, c);
                        count += 1;
                    }
                }
            }
            (total, count)
        };
        let (before, before_count) = census(&w);
        assert!(
            (before - WORTH as f32 * before_count as f32).abs() < 1.0,
            "test setup: the census does not see the stamps it was just given ({before} over {before_count} cells)"
        );

        let mut ps = ParticleSystem::new();
        super::super::explosion::trigger(&mut w, &mut ps, cx, cy, radius, 180.0);
        let thrown = ps.len();
        assert!(
            thrown > 0,
            "test setup: the blast threw no particles at all, so the hand-off this test is about never happened"
        );

        // Let every particle land.
        for _ in 0..600 {
            if ps.is_empty() {
                break;
            }
            ps.step(&mut w);
        }
        assert!(ps.is_empty(), "{} particles never landed in 600 frames", ps.len());

        let (after, standing) = census(&w);

        // **The bar is worth *per surviving cell*, not on the total.** A
        // blast is allowed to destroy corpse cells outright -- that is the
        // consumption path, and WP-6's `meat_lost` is what books it -- so
        // the total legitimately falls. What may never happen is a cell
        // coming back *cheaper than it went in*, which is exactly what
        // dropping `aux` does: 1,020 -> `corpse.ron`'s 120 fallback.
        assert!(
            standing > 0,
            "the blast left no corpse standing at all, so this cannot tell a fix from a hole"
        );
        let per_cell = after / standing as f32;
        assert!(
            (per_cell - WORTH as f32).abs() < 1.0,
            "a corpse stamped {WORTH} came back worth {per_cell:.1} per cell \
             ({standing} standing of {before_count}, {thrown} thrown): the particle dropped its aux"
        );
    }

    /// **The other half of bug Z2's fix: a thrown grain must not land
    /// carrying a stamp that means something else.**
    ///
    /// The guard for the *replacement* artifact, per `CLAUDE.md` — the risk
    /// a fix like this introduces is not "the stamp is dropped" but "the
    /// stamp is copied onto a material where `aux` means something else
    /// entirely". `Cell::aux` is a tagged union: on soil it is saturation on
    /// `SOIL_SATURATED`'s scale, so an unconditional copy makes every
    /// blasted grain land soaking wet.
    ///
    /// **This test is asserted on the raw `aux`, and the first version of it
    /// was vacuous for asserting through `creature::food_value` instead.**
    /// That function *already* gates on `worth_in_aux`, so it reports soil's
    /// flat `food_energy` whatever the stamp says — the assertion could not
    /// fail, and it duly passed with the gate in `landed_cell` deleted. It
    /// was measuring the gate it was written to guard, through a second copy
    /// of that same gate. `CLAUDE.md`: sanity-check a new metric against a
    /// case you know is broken, not only one you know is fine.
    ///
    /// Written as a pair with the corpse case above deliberately: the two
    /// fail in opposite directions, so a gate that is simply inverted passes
    /// one and fails the other rather than passing both.
    #[test]
    fn a_blasted_grain_does_not_land_carrying_its_moisture() {
        let mut w = World::new(Rect::new(0, 0, 127, 127));
        for x in 0..128 {
            w.set(x, 127, super::super::cell::Cell::new(material::STONE, 0));
        }
        let soil = w.materials.id_of("soil").expect("soil material");
        assert!(
            !w.materials.get(soil).worth_in_aux,
            "test setup: soil must NOT price itself in aux, or this is testing the corpse case again"
        );

        // Saturated to the top of the moisture scale — the largest wrong
        // number a landed grain could possibly carry.
        let (cx, cy, radius) = (64i32, 100i32, 20i32);
        let slab = |x: i32, y: i32| (44..84).contains(&x) && (96..104).contains(&y);
        for y in 96..104 {
            for x in 44..84 {
                w.set(x, y, super::super::cell::Cell::new(soil, 0).with_aux(material::SOIL_SATURATED));
            }
        }

        let mut ps = ParticleSystem::new();
        super::super::explosion::trigger(&mut w, &mut ps, cx, cy, radius, 180.0);
        let thrown = ps.len();
        assert!(thrown > 0, "test setup: the blast threw no grains, so nothing here is exercised");
        for _ in 0..600 {
            if ps.is_empty() {
                break;
            }
            ps.step(&mut w);
        }
        assert!(ps.is_empty(), "{} particles never landed in 600 frames", ps.len());

        // **Soil outside the original slab can only have got there by being
        // thrown**, which is what makes this a test of the landing write
        // rather than of the scene. Cells still inside the footprint are a
        // mix of untouched and landed and cannot distinguish the two.
        let mut landed = 0;
        for y in 0..128 {
            for x in 0..128 {
                let c = w.get(x, y);
                if c.material != soil || slab(x, y) {
                    continue;
                }
                landed += 1;
                assert_eq!(
                    c.aux(),
                    0,
                    "a grain thrown clear of the slab landed at ({x}, {y}) carrying aux {} — \
                     on soil that is saturation, not worth, and {thrown} grains were thrown",
                    c.aux()
                );
            }
        }
        assert!(
            landed > 0,
            "no grain landed outside the slab at all ({thrown} thrown), so this guard saw nothing"
        );
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

    /// **A landed rock has to be asked whether it is held up.**
    ///
    /// `land` wrote through `World::set` and scheduled nothing, so a landed
    /// `Solid` kept whatever `landed_cell` chose for its `aux` until some
    /// unrelated disturbance woke it -- and `Cell::new`'s `aux 0` reads as
    /// *bedrock-adjacent*. `Reports/structural-support-model.md` §6.6.
    ///
    /// Asserts the *effect* rather than the call: the scheduler has to be
    /// holding a site afterwards. Sand is the control that says the gate is
    /// on body material rather than on landing -- it is a `Powder`, it takes
    /// no part in the support field, and it must schedule nothing.
    #[test]
    fn a_landed_solid_schedules_a_structural_check_and_a_landed_powder_does_not() {
        for (material_id, expect_sites, what) in
            [(material::STONE, true, "stone"), (material::SAND, false, "sand")]
        {
            let mut w = test_world();
            let mut ps = ParticleSystem::new();
            let before = w.active_site_count();
            ps.spawn(30.0, 10.0, 0.0, 0.0, material_id, 2);
            for _ in 0..200 {
                ps.step(&mut w);
                if ps.is_empty() {
                    break;
                }
            }
            assert!(ps.is_empty(), "{what}: particle never landed after 200 frames");
            let landed = (0..63).any(|y| w.get(30, y).material == material_id);
            assert!(landed, "{what}: test setup -- the particle did not become a cell");

            let raised = w.active_site_count() > before;
            assert_eq!(
                raised, expect_sites,
                "{what}: landing raised sites = {raised}, expected {expect_sites} -- \
                 a landed body cell must be handed to structural::tick, and a landed \
                 powder must not pay for five heap pushes it cannot use"
            );
        }
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
