//! Cells to pixels.
//!
//! The simulation never writes colours. It stores a material id and a shade
//! index, and this module resolves those into RGBA at draw time. Keeping the
//! two apart is what lets M6 swap in a GPU pipeline with lighting and bloom
//! without touching a single movement rule.

use crate::sim::cell::AMBIENT_TEMPERATURE;
use crate::sim::chunk::CHUNK_SIZE;
use crate::sim::particle::ParticleSystem;
use crate::sim::rng;
use crate::sim::world::World;

/// Colour shown for positions outside the world.
const VOID: [u8; 4] = [12, 12, 16, 255];
const CHUNK_BORDER_ACTIVE: [u8; 4] = [80, 200, 120, 255];
const CHUNK_BORDER_SETTLED: [u8; 4] = [60, 60, 70, 255];

// --- M19 visual polish: cheap, CPU-side, no shader pipeline -----------------
//
// Tier 1/2 of the M19 execution plan (see `PLAN.md` and
// `research/m19-visual-polish.md`) — grain, heat glow and fake occlusion,
// the techniques the research found gave the most visible improvement for
// the least engine change. A GPU bloom/lighting pass (Tier 4) stays with
// M6's deferral, since it still needs a human watching it render; these
// don't — they're as self-verifiable by screenshot as M7/M15 were.

/// How much a cell's brightness varies from its neighbours', at most.
/// Sandspiel's whole "looks organic despite simple materials" reputation
/// traces to exactly this trick: reusing a per-cell value as a brightness
/// jitter instead of only as a palette index. Keyed on world position via
/// `rng::jitter` (already used for movement decisions that must be stable
/// across frames) rather than redrawn per frame, so a settled pile doesn't
/// visibly sparkle — the grain is a property of the position, like the
/// palette shade already is.
const JITTER_STRENGTH: f32 = 0.12;

/// How far above ambient a cell needs to be for `HEAT_GLOW_RANGE` to
/// saturate the warm-tint blend fully. Oil burns at 900C, so this is a
/// fraction of that — hot enough to mean something, not so high that only
/// active fire ever visibly registers.
const HEAT_GLOW_RANGE: f32 = 400.0;
/// The fire tint's own colour now shifts with `heat_ratio` — a bare ember
/// (`FIRE_TINT_LOW`) at the low end, up toward a bright yellow-white blaze
/// (`FIRE_TINT_HIGH`) at the top of `HEAT_GLOW_RANGE` — rather than one flat
/// orange regardless of intensity. A cooling coal and an actively raging
/// fire should not read as the identical colour just because both cross
/// `is_burning()`'s own threshold.
const FIRE_TINT_LOW: [f32; 3] = [180.0, 55.0, 20.0];
const FIRE_TINT_HIGH: [f32; 3] = [255.0, 210.0, 110.0];
/// Frames per flicker step for an actively burning cell — a real flame's
/// visible flicker rate is on the order of 10-15Hz, not a 60fps repaint, so
/// re-rolling every single frame would read as noise, not fire. `jitter3`
/// (position plus this coarse time bucket) holds the flicker steady within
/// one step and changes it deterministically at the next, with no per-cell
/// state to track.
const FLAME_FLICKER_PERIOD: u64 = 4;
/// How much the flicker can push the fire blend strength up or down, as a
/// fraction of it. `0.3` means anywhere from 70% to 130% of the blend
/// `heat_ratio` alone would produce — enough to visibly flicker, not enough
/// to make a hot cell ever read as merely warm.
const FLAME_FLICKER_STRENGTH: f32 = 0.3;

pub struct Renderer {
    /// World coordinate displayed at the top-left pixel. Fixed at the origin
    /// for M2; M10 moves it with the player.
    pub camera_x: i32,
    pub camera_y: i32,
    /// Draws chunk boundaries tinted by whether the chunk will be swept next
    /// frame. This is the primary way to confirm sleeping actually works.
    pub show_chunk_overlay: bool,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            camera_x: 0,
            camera_y: 0,
            show_chunk_overlay: false,
        }
    }

    pub fn draw(&self, world: &World, particles: &ParticleSystem, frame: &mut [u8], width: u32, height: u32) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let sx = (i % width as usize) as i32;
            let sy = (i / width as usize) as i32;
            let colour = self.cell_colour(world, sx + self.camera_x, sy + self.camera_y);
            pixel.copy_from_slice(&colour);
        }

        self.draw_particles(world, particles, frame, width, height);

        if self.show_chunk_overlay {
            self.draw_chunk_overlay(world, frame, width, height);
        }
    }

    /// Free particles are not CA cells, so the main per-pixel pass above
    /// never sees them — drawn here as a second, small pass instead. One
    /// pixel each, no interpolation between a particle's sub-cell position
    /// and the pixel grid; the CA cell it becomes on landing carries all the
    /// same visual weight bulk material has, so a free particle mid-flight
    /// not doing that too is not a loss worth the complexity of drawing it
    /// any richer.
    fn draw_particles(&self, world: &World, particles: &ParticleSystem, frame: &mut [u8], width: u32, height: u32) {
        for particle in particles.iter() {
            let sx = particle.x.round() as i32 - self.camera_x;
            let sy = particle.y.round() as i32 - self.camera_y;
            if sx < 0 || sy < 0 || sx >= width as i32 || sy >= height as i32 {
                continue;
            }
            let palette = &world.materials.get(particle.material).palette;
            let colour = palette[particle.shade as usize % palette.len()];
            let idx = (sy as usize * width as usize + sx as usize) * 4;
            frame[idx..idx + 4].copy_from_slice(&colour);
        }
    }

    fn cell_colour(&self, world: &World, x: i32, y: i32) -> [u8; 4] {
        if !world.in_bounds(x, y) {
            return VOID;
        }
        let cell = world.get(x, y);
        let palette = &world.materials.get(cell.material).palette;
        // Modulo keeps any shade value valid, so a palette can shrink on hot
        // reload in M3 without invalidating cells already in the world.
        let base = palette[cell.shade as usize % palette.len()];
        if cell.is_empty() {
            return base; // nothing to texture, glow or occlude in vacuum
        }
        let mut rgb = [base[0], base[1], base[2]];

        // Grain: a stable per-position brightness jitter — see the module
        // constant doc for why this is the Sandspiel trick, not dithering.
        // Integer math, not float: this runs on every visible non-empty
        // pixel every frame (rendering has no dirty-rect equivalent — it
        // redraws the whole screen regardless of what's settled), so it
        // has to earn its cost the same way M14 learned fire's per-cell
        // pass did at CA-sweep scale.
        let jitter_permille = ((rng::jitter(x, y) - 0.5) * 2000.0 * JITTER_STRENGTH) as i32;
        for c in &mut rgb {
            *c = (*c as i32 + (*c as i32 * jitter_permille) / 1000).clamp(0, 255) as u8;
        }

        // Heat glow: continuous with temperature, not just a flat tint for
        // `is_burning()` — a cell cooling down after a fire, or sitting next
        // to one, should visibly register as warm even once the flame
        // itself is out. Burning cells are floored at the same 0.6 blend
        // the flat version used, so active fire never looks weaker than it
        // did before; non-burning warmth caps at half that, since ambient
        // heat is not the same visual statement as active flame.
        //
        // Early exit for cells already at ambient and not burning — which
        // is nearly every cell, nearly always — for the same reason: the
        // full blend computation costs nothing turned off, and turned on
        // unconditionally it was measured to nearly triple this function's
        // cost across a full 512x320 stress scene even though almost every
        // one of those calls would have computed `t == 0.0` and changed
        // nothing.
        if cell.is_burning() || cell.temperature() != AMBIENT_TEMPERATURE {
            let heat_ratio = ((cell.temperature() as f32 - AMBIENT_TEMPERATURE as f32) / HEAT_GLOW_RANGE).clamp(0.0, 1.0);
            let mut t = if cell.is_burning() { heat_ratio.max(0.6) } else { heat_ratio * 0.5 };
            // Flicker only for actively burning cells — a passively cooling
            // ember shouldn't dance the way a live flame does.
            if cell.is_burning() {
                let bucket = (world.frame / FLAME_FLICKER_PERIOD) as i32;
                let flicker = 1.0 + (rng::jitter3(x, y, bucket) - 0.5) * 2.0 * FLAME_FLICKER_STRENGTH;
                t = (t * flicker).clamp(0.0, 1.0);
            }
            if t > 0.0 {
                let fire = [
                    FIRE_TINT_LOW[0] + (FIRE_TINT_HIGH[0] - FIRE_TINT_LOW[0]) * heat_ratio,
                    FIRE_TINT_LOW[1] + (FIRE_TINT_HIGH[1] - FIRE_TINT_LOW[1]) * heat_ratio,
                    FIRE_TINT_LOW[2] + (FIRE_TINT_HIGH[2] - FIRE_TINT_LOW[2]) * heat_ratio,
                ];
                for (c, fire) in rgb.iter_mut().zip(fire) {
                    *c = (*c as f32 + (fire - *c as f32) * t).round() as u8;
                }
            }
        }

        // Fake AO (darkening a cell by how enclosed it is) was tried here
        // too and measured to cost more than jitter and heat combined —
        // ~10ms on the 512x320 stress scene alone, from the 4 extra
        // `World::get` calls per pixel, on top of rendering's own per-pixel
        // lookup. Unlike the CA sweep, rendering has no dirty-rect skip: it
        // redraws every visible pixel every frame regardless of what
        // settled, so a densely-filled *static* world pays this cost
        // forever, not just as a stress-test edge case. Cutting rather than
        // shipping it over budget — see `PLAN.md`'s M19 section for the
        // real fix this needs (reusing a chunk's direct array access
        // instead of a `World::get` HashMap lookup per neighbour, the same
        // lesson M5's `ChunkView` already applied to the sweep) before it's
        // safe to turn back on.

        [rgb[0], rgb[1], rgb[2], 255]
    }

    fn draw_chunk_overlay(&self, world: &World, frame: &mut [u8], width: u32, height: u32) {
        for chunk in world.chunks() {
            let colour = if chunk.is_settled() {
                CHUNK_BORDER_SETTLED
            } else {
                CHUNK_BORDER_ACTIVE
            };
            let (ox, oy) = chunk.coord.origin();
            let sx = ox - self.camera_x;
            let sy = oy - self.camera_y;
            for i in 0..CHUNK_SIZE {
                put(frame, width, height, sx + i, sy, colour);
                put(frame, width, height, sx, sy + i, colour);
            }
        }
    }

    /// World position under a screen pixel. The inverse of the camera offset
    /// applied in `draw`, used to turn cursor position into a brush position.
    pub fn screen_to_world(&self, sx: i32, sy: i32) -> (i32, i32) {
        (sx + self.camera_x, sy + self.camera_y)
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

fn put(frame: &mut [u8], width: u32, height: u32, x: i32, y: i32, colour: [u8; 4]) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }
    let i = (y as usize * width as usize + x as usize) * 4;
    frame[i..i + 4].copy_from_slice(&colour);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::cell::Cell;
    use crate::sim::chunk::Rect;
    use crate::sim::material;

    #[test]
    fn draws_material_colours_and_void_outside_the_world() {
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        world.set(30, 30, Cell::new(material::SAND, 0));
        let renderer = Renderer::new();
        let particles = ParticleSystem::new();

        // A 128-wide framebuffer over a 64-wide world: the right half is void.
        let (w, h) = (128u32, 64u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        renderer.draw(&world, &particles, &mut frame, w, h);

        // Not hot, so only the position-jitter grain (see `JITTER_STRENGTH`)
        // should have moved this away from the flat palette colour.
        let sand = world.materials.get(material::SAND).palette[0];
        let jitter_permille = ((rng::jitter(30, 30) - 0.5) * 2000.0 * JITTER_STRENGTH) as i32;
        let expected = [
            (sand[0] as i32 + (sand[0] as i32 * jitter_permille) / 1000).clamp(0, 255) as u8,
            (sand[1] as i32 + (sand[1] as i32 * jitter_permille) / 1000).clamp(0, 255) as u8,
            (sand[2] as i32 + (sand[2] as i32 * jitter_permille) / 1000).clamp(0, 255) as u8,
            255,
        ];
        let idx = (30 * w as usize + 30) * 4;
        assert_eq!(&frame[idx..idx + 4], &expected);

        // Row 0, column 100 — past the right edge of the 64-wide world.
        let outside = 100 * 4;
        assert_eq!(&frame[outside..outside + 4], &VOID);
    }


    #[test]
    fn draws_a_free_particle_at_its_rounded_position() {
        let world = World::new(Rect::new(0, 0, 63, 63));
        let renderer = Renderer::new();
        let mut particles = ParticleSystem::new();
        particles.spawn(10.4, 20.3, 0.0, 0.0, material::SAND, 1);

        let (w, h) = (64u32, 64u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        renderer.draw(&world, &particles, &mut frame, w, h);

        let sand = world.materials.get(material::SAND).palette[1];
        let idx = (20 * w as usize + 10) * 4;
        assert_eq!(&frame[idx..idx + 4], &sand, "particle did not draw at its rounded position");
    }

    #[test]
    fn a_particle_outside_the_framebuffer_does_not_panic_or_wrap() {
        let world = World::new(Rect::new(0, 0, 63, 63));
        let renderer = Renderer::new();
        let mut particles = ParticleSystem::new();
        particles.spawn(-5.0, -5.0, 0.0, 0.0, material::SAND, 0);
        particles.spawn(1000.0, 1000.0, 0.0, 0.0, material::SAND, 0);

        let (w, h) = (64u32, 64u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        // Reaching this line without panicking is the assertion.
        renderer.draw(&world, &particles, &mut frame, w, h);
    }

    #[test]
    fn grain_varies_between_same_material_cells_at_different_positions() {
        // The whole point of keying jitter on position rather than shade
        // index (see `JITTER_STRENGTH`'s doc): even two cells with the
        // *same* palette shade should end up visibly different, which a
        // flat "just show the palette colour" renderer never produced.
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        world.set(0, 0, Cell::new(material::SAND, 0));
        world.set(10, 0, Cell::new(material::SAND, 0));
        let renderer = Renderer::new();
        let particles = ParticleSystem::new();
        let (w, h) = (64u32, 64u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        renderer.draw(&world, &particles, &mut frame, w, h);

        let a = &frame[0..4];
        let b_idx = 10 * 4;
        let b = &frame[b_idx..b_idx + 4];
        assert_ne!(a, b, "two same-shade cells at different positions rendered identically");
    }

    #[test]
    fn a_hot_non_burning_cell_looks_warmer_than_a_cool_one() {
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        world.set(0, 0, Cell::new(material::STONE, 0).with_temperature(20)); // ambient
        world.set(10, 0, Cell::new(material::STONE, 0).with_temperature(500)); // hot, not burning
        let renderer = Renderer::new();
        let particles = ParticleSystem::new();
        let (w, h) = (64u32, 64u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        renderer.draw(&world, &particles, &mut frame, w, h);

        let cool = &frame[0..4];
        let hot_idx = 10 * 4;
        let hot = &frame[hot_idx..hot_idx + 4];
        // Warmer means redder relative to blue -- a robust check regardless
        // of exactly how strong the jitter/AO terms happened to land, since
        // neither of those touches the red/blue balance the heat blend does.
        let warmth = |p: &[u8]| p[0] as i32 - p[2] as i32;
        assert!(
            warmth(hot) > warmth(cool),
            "a cell well above ambient should read warmer (redder) than one at ambient"
        );
    }

    #[test]
    fn a_burning_cells_colour_flickers_across_time_buckets() {
        // The user-facing complaint this fixes: fire used to be a static
        // blend keyed only on temperature, so a burning cell held one flat
        // colour for its whole burn -- no animation at all frame to frame.
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        let mut burning = Cell::new(material::OIL, 0);
        burning.ignite(9999);
        world.set(30, 30, burning);
        let renderer = Renderer::new();
        let particles = ParticleSystem::new();
        let (w, h) = (64u32, 64u32);
        let idx = (30 * w as usize + 30) * 4;

        // Enough distinct flicker buckets that at least one pair of frames
        // should disagree even allowing for `jitter3` occasionally landing
        // close to the same value twice by chance.
        let mut colours = std::collections::HashSet::new();
        for bucket in 0..10 {
            world.frame = bucket * FLAME_FLICKER_PERIOD;
            let mut frame = vec![0u8; (w * h * 4) as usize];
            renderer.draw(&world, &particles, &mut frame, w, h);
            colours.insert(frame[idx..idx + 4].to_vec());
        }
        assert!(colours.len() > 1, "a burning cell rendered identically across every time bucket -- no flicker at all");
    }

    #[test]
    fn a_merely_warm_non_burning_cell_shifts_hue_with_temperature_not_just_brightness() {
        // The other half of the animation fix: the fire tint itself used to
        // be one flat orange (`FIRE_TINT`) regardless of how hot a cell
        // read, so intensity only ever changed blend *strength*, never
        // colour. Comparing two different temperatures does not isolate
        // that claim -- a flat tint blended in harder by a rising
        // `heat_ratio` alone raises the green channel too, which is exactly
        // what the first version of this test got fooled by (confirmed by
        // temporarily flattening the tint back to a constant and watching
        // it still pass). Instead, pin the temperature at exactly the point
        // `heat_ratio` saturates to 1.0, where `fire == FIRE_TINT_HIGH`
        // algebraically regardless of blend strength `t`, and predict the
        // pixel by hand from the *old* flat tint. A real hue shift makes
        // the rendered pixel disagree with that flat-tint prediction; a
        // blend-strength-only change could never produce a disagreement
        // here, since `t` is the same number either way.
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        let temperature = AMBIENT_TEMPERATURE + HEAT_GLOW_RANGE as i16; // heat_ratio == 1.0 exactly
        world.set(10, 0, Cell::new(material::STONE, 0).with_temperature(temperature));
        let renderer = Renderer::new();
        let particles = ParticleSystem::new();
        let (w, h) = (64u32, 64u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        renderer.draw(&world, &particles, &mut frame, w, h);

        let hot_idx = 10 * 4;
        let actual = &frame[hot_idx..hot_idx + 4];

        // Reconstruct exactly what the renderer computed, up to the tint:
        // same base palette colour, same position grain, same blend
        // strength `t` -- everything the old flat-tint code also had.
        let base = world.materials.get(material::STONE).palette[0];
        let jitter_permille = ((rng::jitter(10, 0) - 0.5) * 2000.0 * 0.12) as i32;
        let mut grained = [base[0], base[1], base[2]];
        for c in &mut grained {
            *c = (*c as i32 + (*c as i32 * jitter_permille) / 1000).clamp(0, 255) as u8;
        }
        let t = 0.5_f32; // heat_ratio (1.0) * 0.5, non-burning
        let old_flat_fire = [255.0f32, 140.0, 40.0];
        let predicted_with_old_flat_tint: Vec<u8> = grained
            .iter()
            .zip(old_flat_fire)
            .map(|(c, fire)| (*c as f32 + (fire - *c as f32) * t).round() as u8)
            .collect();

        assert_ne!(
            &actual[0..3],
            predicted_with_old_flat_tint.as_slice(),
            "at saturation the rendered colour should match FIRE_TINT_HIGH exactly (not the old flat tint), \
             so it must differ from what a flat-tint implementation at the same blend strength would produce: \
             actual={actual:?}, old-flat-tint prediction={predicted_with_old_flat_tint:?}"
        );
        // And it should match what the real ramp predicts (fire ==
        // FIRE_TINT_HIGH exactly once heat_ratio saturates).
        let predicted_with_ramp: Vec<u8> = grained
            .iter()
            .zip(FIRE_TINT_HIGH)
            .map(|(c, fire)| (*c as f32 + (fire - *c as f32) * t).round() as u8)
            .collect();
        assert_eq!(
            &actual[0..3],
            predicted_with_ramp.as_slice(),
            "actual={actual:?}, ramp prediction={predicted_with_ramp:?}"
        );
    }

    #[test]
    fn screen_to_world_accounts_for_the_camera() {
        let mut r = Renderer::new();
        assert_eq!(r.screen_to_world(5, 7), (5, 7));
        r.camera_x = 100;
        r.camera_y = -20;
        assert_eq!(r.screen_to_world(5, 7), (105, -13));
    }

    #[test]
    fn the_overlay_distinguishes_active_from_settled_chunks() {
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        let mut renderer = Renderer::new();
        renderer.show_chunk_overlay = true;
        let particles = ParticleSystem::new();
        let (w, h) = (64u32, 64u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];

        // Freshly built chunks are dirty, so the border reads as active.
        renderer.draw(&world, &particles, &mut frame, w, h);
        assert_eq!(&frame[0..4], &CHUNK_BORDER_ACTIVE);

        // Once settled it dims.
        world.end_step();
        renderer.draw(&world, &particles, &mut frame, w, h);
        assert_eq!(&frame[0..4], &CHUNK_BORDER_SETTLED);
    }
}
