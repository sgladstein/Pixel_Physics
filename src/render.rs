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

/// Which M13 field channel `Renderer::field_overlay` tints the screen by,
/// cycled by `V` — parallel to `F1`'s existing chunk overlay, for seeing
/// the coarse field grid instead of chunk activity.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FieldOverlay {
    #[default]
    Off,
    Pressure,
    Temperature,
    Light,
    Moisture,
}

impl FieldOverlay {
    fn next(self) -> Self {
        match self {
            FieldOverlay::Off => FieldOverlay::Pressure,
            FieldOverlay::Pressure => FieldOverlay::Temperature,
            FieldOverlay::Temperature => FieldOverlay::Light,
            FieldOverlay::Light => FieldOverlay::Moisture,
            FieldOverlay::Moisture => FieldOverlay::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FieldOverlay::Off => "OFF",
            FieldOverlay::Pressure => "PRESSURE",
            FieldOverlay::Temperature => "TEMPERATURE",
            FieldOverlay::Light => "LIGHT",
            FieldOverlay::Moisture => "MOISTURE",
        }
    }
}

/// Display-only normalization ranges for the field overlay — not the
/// field's own internal clamp bounds (`field.rs`'s `MAX_TEMPERATURE`/
/// `MAX_LIGHT`/`MAX_MOISTURE` are private to that module, the same
/// "documented assumption about a scale this module can't reach"
/// `creature.rs`'s own `WORM_MOISTURE_SATURATION` already accepts for the
/// identical reason). Chosen to make the *common* range of each channel
/// legible, not to bound it exactly — a value past the range end still
/// renders, just clamped to the most saturated colour rather than blowing
/// out past it.
const PRESSURE_OVERLAY_RANGE: f32 = 30.0;
const TEMPERATURE_OVERLAY_MAX: f32 = 900.0;
const LIGHT_OVERLAY_MAX: f32 = 4.0;
const MOISTURE_OVERLAY_MAX: f32 = 4.0;

/// Clamp bounds for `Renderer::zoom` — magnifying, screen pixels per world
/// cell. `8` is arbitrary (untuned against anything but "still recognizably
/// the sandbox, not a single giant cell filling the window" at the engine's
/// 512x320 simulation resolution).
const MAX_ZOOM: i32 = 8;
/// Clamp bounds for `Renderer::zoom_out_stride` — minifying, world cells
/// per screen pixel. `4` keeps a stride-sampled view still readable as
/// "the same kind of picture, zoomed out" rather than aliasing into noise.
const MAX_ZOOM_OUT_STRIDE: i32 = 4;

pub struct Renderer {
    /// World coordinate displayed at the top-left pixel. Fixed at the origin
    /// for M2; M10 moves it with the player.
    pub camera_x: i32,
    pub camera_y: i32,
    /// Draws chunk boundaries tinted by whether the chunk will be swept next
    /// frame. This is the primary way to confirm sleeping actually works.
    pub show_chunk_overlay: bool,
    /// Screen pixels per world cell, 1-`MAX_ZOOM`, nearest-neighbour
    /// magnify. `1` is the original unscaled mapping M2 shipped with.
    /// Mutually exclusive with `zoom_out_stride > 1` in practice —
    /// `adjust_zoom` never lets both climb above 1 at once, though nothing
    /// below is structurally prevented from being called with both set;
    /// it would just be a confusing scale to be in, not an unsafe one.
    pub zoom: i32,
    /// World cells sampled per screen pixel, 1-`MAX_ZOOM_OUT_STRIDE` — the
    /// reverse direction, seeing more of the world at once at the cost of
    /// skipping cells between samples rather than averaging them (a proper
    /// minify filter is not worth it for a debug/overview zoom level).
    pub zoom_out_stride: i32,
    /// Tints every pixel by an M13 field channel instead of (blended over)
    /// the ordinary cell colour — `V` cycles it. `Off` by default, so it
    /// costs nothing (no extra `World::field_at` calls) unless a player
    /// opts in, the same "toggled debug overlay, zero cost when off" shape
    /// `show_chunk_overlay` already uses.
    pub field_overlay: FieldOverlay,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            camera_x: 0,
            camera_y: 0,
            show_chunk_overlay: false,
            zoom: 1,
            zoom_out_stride: 1,
            field_overlay: FieldOverlay::Off,
        }
    }

    pub fn cycle_field_overlay(&mut self) {
        self.field_overlay = self.field_overlay.next();
    }

    /// `delta > 0` zooms in a step, `delta < 0` zooms out a step — `=`/`-`
    /// in the live app. The two fields form one continuous scale rather
    /// than being independently adjustable: zooming in past `zoom_out_
    /// stride > 1` counts down the stride back to 1 before `zoom` itself
    /// starts climbing, and symmetrically the other way, so the key pair
    /// reads as a single "more/less zoom" control, not two separate ones a
    /// player has to understand are different mechanisms.
    pub fn adjust_zoom(&mut self, delta: i32) {
        if delta > 0 {
            if self.zoom_out_stride > 1 {
                self.zoom_out_stride -= 1;
            } else {
                self.zoom = (self.zoom + 1).min(MAX_ZOOM);
            }
        } else if delta < 0 {
            if self.zoom > 1 {
                self.zoom -= 1;
            } else {
                self.zoom_out_stride = (self.zoom_out_stride + 1).min(MAX_ZOOM_OUT_STRIDE);
            }
        }
    }

    pub fn draw(&self, world: &World, particles: &ParticleSystem, frame: &mut [u8], width: u32, height: u32) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let sx = (i % width as usize) as i32;
            let sy = (i / width as usize) as i32;
            let (wx, wy) = self.screen_to_world(sx, sy);
            let colour = self.cell_colour(world, wx, wy);
            pixel.copy_from_slice(&colour);
        }

        self.draw_particles(world, particles, frame, width, height);

        if self.show_chunk_overlay {
            self.draw_chunk_overlay(world, frame, width, height);
        }
    }

    /// Free particles are not CA cells, so the main per-pixel pass above
    /// never sees them — drawn here as a second, small pass instead. At
    /// `zoom > 1` each particle fills the same `zoom`x`zoom` screen block
    /// its position would round to for a CA cell, rather than staying a
    /// single screen pixel while everything around it magnifies — the
    /// otherwise-jarring case being debris that visually shrinks to a dot
    /// against its own newly-oversized surroundings the instant zoom goes
    /// up. No interpolation between a particle's sub-cell position and the
    /// pixel grid beyond that; the CA cell it becomes on landing carries
    /// all the same visual weight bulk material has, so a free particle
    /// mid-flight not doing that too is not a loss worth more complexity.
    fn draw_particles(&self, world: &World, particles: &ParticleSystem, frame: &mut [u8], width: u32, height: u32) {
        for particle in particles.iter() {
            let Some((sx, sy)) = self.world_to_screen(particle.x.round() as i32, particle.y.round() as i32) else {
                continue;
            };
            let palette = &world.materials.get(particle.material).palette;
            let colour = palette[particle.shade as usize % palette.len()];
            let block = self.zoom.max(1);
            for dy in 0..block {
                for dx in 0..block {
                    put(frame, width, height, sx + dx, sy + dy, colour);
                }
            }
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
            // Still route through the field overlay below (a field reading
            // exists over empty space same as anywhere else -- pressure and
            // temperature very much propagate through vacuum) rather than
            // returning here the way the pre-overlay code always did.
            return self.apply_field_overlay(world, x, y, base);
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

        self.apply_field_overlay(world, x, y, [rgb[0], rgb[1], rgb[2], 255])
    }

    /// Blends `base` toward a colour ramp keyed on the currently-selected
    /// field channel — a no-op returning `base` unchanged when the overlay
    /// is off, so every caller can route through this unconditionally
    /// rather than each needing its own `if field_overlay != Off` guard.
    ///
    /// Blend strength scales with `magnitude` (0..1, how far the reading
    /// sits from that channel's own ambient/baseline value) up to
    /// `MAX_BLEND` at full saturation — **not** a flat blend regardless of
    /// value. An earlier version used a flat blend specifically to avoid
    /// washing out low readings, but every channel's ramp colour at
    /// `magnitude == 0` is some fixed, saturated colour (`Off` aside), not
    /// `base` itself — a flat blend therefore tints *every* unaffected
    /// pixel toward that fixed colour regardless of whether the channel is
    /// actually elevated there, which independent review caught
    /// concretely for pressure (every ambient cell, i.e. nearly the whole
    /// visible world nearly all the time, blended 60% toward white). Real
    /// elevation still reads clearly under magnitude-scaling — a fully
    /// saturated reading still reaches `MAX_BLEND` — it just no longer
    /// paints the entire screen for a channel currently near zero
    /// everywhere.
    fn apply_field_overlay(&self, world: &World, x: i32, y: i32, base: [u8; 4]) -> [u8; 4] {
        let (ramp, magnitude) = match self.field_overlay {
            FieldOverlay::Off => return base,
            FieldOverlay::Pressure => {
                let f = world.field_at(x, y);
                // Signed: red for positive (compression), blue for negative
                // (rarefaction) -- `magnitude` (not the ramp colour itself)
                // is what fades this to `base` at zero, so both directions
                // share one ramp pair rather than needing a third "zero"
                // colour of their own.
                let t = (f.pressure / PRESSURE_OVERLAY_RANGE).clamp(-1.0, 1.0);
                let colour = if t >= 0.0 { [255.0, 60.0, 60.0] } else { [60.0, 90.0, 255.0] };
                (colour, t.abs())
            }
            FieldOverlay::Temperature => {
                let f = world.field_at(x, y);
                let t = ((f.temperature - AMBIENT_TEMPERATURE as f32) / TEMPERATURE_OVERLAY_MAX).clamp(0.0, 1.0);
                ([255.0, 140.0, 40.0], t)
            }
            FieldOverlay::Light => {
                let f = world.field_at(x, y);
                let t = (f.light / LIGHT_OVERLAY_MAX).clamp(0.0, 1.0);
                ([255.0, 255.0, 190.0], t)
            }
            FieldOverlay::Moisture => {
                let f = world.field_at(x, y);
                let t = (f.moisture / MOISTURE_OVERLAY_MAX).clamp(0.0, 1.0);
                ([60.0, 140.0, 255.0], t)
            }
        };
        const MAX_BLEND: f32 = 0.75;
        let blend = magnitude.clamp(0.0, 1.0) * MAX_BLEND;
        let mut out = base;
        for (c, r) in out.iter_mut().take(3).zip(ramp) {
            *c = (*c as f32 + (r - *c as f32) * blend).round().clamp(0.0, 255.0) as u8;
        }
        out
    }

    fn draw_chunk_overlay(&self, world: &World, frame: &mut [u8], width: u32, height: u32) {
        for chunk in world.chunks() {
            let colour = if chunk.is_settled() {
                CHUNK_BORDER_SETTLED
            } else {
                CHUNK_BORDER_ACTIVE
            };
            let (ox, oy) = chunk.coord.origin();
            for i in 0..CHUNK_SIZE {
                // Each world-space border point maps through zoom/stride
                // individually rather than scaling the whole chunk-sized
                // loop bound -- simpler than special-casing the two zoom
                // directions here, at the cost of a thin (not `zoom`-wide)
                // line when magnified, an accepted minor look rather than
                // added complexity for a debug overlay.
                if let Some((sx, sy)) = self.world_to_screen(ox + i, oy) {
                    put(frame, width, height, sx, sy, colour);
                }
                if let Some((sx, sy)) = self.world_to_screen(ox, oy + i) {
                    put(frame, width, height, sx, sy, colour);
                }
            }
        }
    }

    /// World position under a screen pixel — the camera offset plus the
    /// zoom/stride scale `draw`'s own per-pixel loop applies, used to turn
    /// cursor position into a brush position (so painting lands on the
    /// cell actually under the cursor at any zoom level, not the
    /// unscaled-1:1 cell it would be at `zoom == 1`).
    ///
    /// `div_euclid`, not `/`: a screen position left of the camera (a
    /// negative `sx`, reachable once panning exists) must floor toward
    /// negative infinity the same way `ChunkCoord::containing` already
    /// establishes is required — truncating division would fold screen
    /// column -1 onto the same world cell as column 0.
    pub fn screen_to_world(&self, sx: i32, sy: i32) -> (i32, i32) {
        if self.zoom > 1 {
            (self.camera_x + sx.div_euclid(self.zoom), self.camera_y + sy.div_euclid(self.zoom))
        } else {
            (self.camera_x + sx * self.zoom_out_stride, self.camera_y + sy * self.zoom_out_stride)
        }
    }

    /// Inverse of `screen_to_world`, for placing something drawn in world
    /// space (a particle, a chunk border) onto the screen. `None` when the
    /// position falls between two stride-sampled columns/rows at
    /// `zoom_out_stride > 1` and so has no single screen pixel of its own —
    /// distinct from simply being off-screen, which callers already clip
    /// against separately via `put`'s own bounds check.
    fn world_to_screen(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        if self.zoom > 1 {
            Some(((x - self.camera_x) * self.zoom, (y - self.camera_y) * self.zoom))
        } else if self.zoom_out_stride > 1 {
            let (dx, dy) = (x - self.camera_x, y - self.camera_y);
            if dx.rem_euclid(self.zoom_out_stride) != 0 || dy.rem_euclid(self.zoom_out_stride) != 0 {
                return None;
            }
            Some((dx.div_euclid(self.zoom_out_stride), dy.div_euclid(self.zoom_out_stride)))
        } else {
            Some((x - self.camera_x, y - self.camera_y))
        }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn put(frame: &mut [u8], width: u32, height: u32, x: i32, y: i32, colour: [u8; 4]) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }
    let i = (y as usize * width as usize + x as usize) * 4;
    frame[i..i + 4].copy_from_slice(&colour);
}

/// Midpoint circle algorithm, eight-way symmetry — the brush outline
/// preview (§9 of `PLAN.md`'s UI-improvement pass), and reusable for
/// anything else that ever wants a cheap outline (a future explosion
/// radius preview, say). `radius <= 0` draws nothing rather than a
/// degenerate single pixel, since a zero-radius brush is a real, valid
/// brush size and a dot at the cursor would misleadingly suggest a
/// one-cell circle instead.
pub(crate) fn draw_circle_outline(frame: &mut [u8], width: u32, height: u32, cx: i32, cy: i32, radius: i32, colour: [u8; 4]) {
    if radius <= 0 {
        return;
    }
    let mut x = radius;
    let mut y = 0;
    let mut err = 0;
    while x >= y {
        for (dx, dy) in [(x, y), (y, x), (-y, x), (-x, y), (-x, -y), (-y, -x), (y, -x), (x, -y)] {
            put(frame, width, height, cx + dx, cy + dy, colour);
        }
        y += 1;
        err += 1 + 2 * y;
        if 2 * (err - x) + 1 > 0 {
            x -= 1;
            err += 1 - 2 * x;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::cell::Cell;
    use crate::sim::chunk::Rect;
    use crate::sim::material;

    #[test]
    fn circle_outline_lights_the_ring_but_not_the_centre() {
        let (w, h) = (32u32, 32u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        let white = [255, 255, 255, 255];
        draw_circle_outline(&mut frame, w, h, 16, 16, 10, white);

        let px = |x: i32, y: i32| -> [u8; 4] {
            let i = ((y as u32 * w + x as u32) * 4) as usize;
            frame[i..i + 4].try_into().unwrap()
        };
        // The four axis-aligned points at exactly `radius` away are always
        // lit for any radius under the midpoint algorithm's own symmetry.
        assert_eq!(px(26, 16), white, "east point of the ring should be lit");
        assert_eq!(px(6, 16), white, "west point of the ring should be lit");
        assert_eq!(px(16, 26), white, "south point of the ring should be lit");
        assert_eq!(px(16, 6), white, "north point of the ring should be lit");
        // An outline, not a filled disc.
        assert_ne!(px(16, 16), white, "the centre should not be lit by an outline");
    }

    #[test]
    fn a_zero_or_negative_radius_circle_draws_nothing() {
        let (w, h) = (16u32, 16u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        draw_circle_outline(&mut frame, w, h, 8, 8, 0, [255, 255, 255, 255]);
        draw_circle_outline(&mut frame, w, h, 8, 8, -5, [255, 255, 255, 255]);
        assert!(frame.iter().all(|&b| b == 0), "a zero/negative radius should draw nothing, not a stray pixel");
    }

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
    fn field_overlay_off_matches_the_pre_overlay_render_exactly() {
        // The overlay must be a genuine no-op when off, not just
        // visually negligible -- `apply_field_overlay` is unconditionally
        // in the call path now (both the empty- and non-empty-cell
        // branches route through it). Compares against a `cell_colour`
        // computed by hand the pre-overlay way (grain + heat glow, nothing
        // else) rather than `Off` against `Off`, which would trivially
        // pass regardless of whether the refactor preserved anything.
        let mut world = World::new(Rect::new(0, 0, 31, 31));
        world.set(10, 10, Cell::new(material::STONE, 0));
        world.add_pressure_impulse(5, 5, 3, 20.0); // field nonzero nearby, still must not show through when off
        let renderer = Renderer::new();
        let base = world.materials.get(material::STONE).palette[0];
        let jitter_permille = ((rng::jitter(10, 10) - 0.5) * 2000.0 * JITTER_STRENGTH) as i32;
        let expected = [
            (base[0] as i32 + (base[0] as i32 * jitter_permille) / 1000).clamp(0, 255) as u8,
            (base[1] as i32 + (base[1] as i32 * jitter_permille) / 1000).clamp(0, 255) as u8,
            (base[2] as i32 + (base[2] as i32 * jitter_permille) / 1000).clamp(0, 255) as u8,
            255,
        ];
        assert_eq!(renderer.cell_colour(&world, 10, 10), expected);
    }

    #[test]
    fn field_overlay_leaves_an_unaffected_cell_unchanged_even_when_on() {
        // The exact property an independent review caught broken in an
        // earlier version: a flat blend regardless of reading magnitude
        // tinted *every* pixel toward a fixed colour, including cells with
        // a perfectly ordinary ambient reading, not just ones near a real
        // disturbance. A cell far from the one active impulse below,
        // for every channel, must render identically to the overlay
        // being off -- confirmed to fail (for Pressure specifically) with
        // the blend strength reverted to a flat constant.
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        world.set(50, 50, Cell::new(material::STONE, 0));
        world.add_pressure_impulse(5, 5, 3, 40.0); // disturbance confined to a far corner
        let mut renderer = Renderer::new();
        let off = renderer.cell_colour(&world, 50, 50);
        for overlay in [FieldOverlay::Pressure, FieldOverlay::Temperature, FieldOverlay::Light, FieldOverlay::Moisture] {
            renderer.field_overlay = overlay;
            assert_eq!(renderer.cell_colour(&world, 50, 50), off, "{overlay:?} tinted an unaffected cell far from any real disturbance");
        }
    }

    #[test]
    fn pressure_overlay_tints_a_cell_near_a_real_impulse() {
        let mut world = World::new(Rect::new(0, 0, 31, 31));
        world.add_pressure_impulse(15, 15, 5, 40.0);
        let particles = ParticleSystem::new();
        let (w, h) = (32u32, 32u32);

        let mut renderer = Renderer::new();
        let mut off = vec![0u8; (w * h * 4) as usize];
        renderer.draw(&world, &particles, &mut off, w, h);

        renderer.field_overlay = FieldOverlay::Pressure;
        let mut on = vec![0u8; (w * h * 4) as usize];
        renderer.draw(&world, &particles, &mut on, w, h);

        assert_ne!(off, on, "the pressure overlay should visibly change the render over a real pressure impulse");
    }

    #[test]
    fn cycle_field_overlay_visits_every_channel_and_returns_to_off() {
        let mut r = Renderer::new();
        assert_eq!(r.field_overlay, FieldOverlay::Off);
        let mut seen = vec![r.field_overlay];
        for _ in 0..4 {
            r.cycle_field_overlay();
            seen.push(r.field_overlay);
        }
        assert_eq!(
            seen,
            vec![FieldOverlay::Off, FieldOverlay::Pressure, FieldOverlay::Temperature, FieldOverlay::Light, FieldOverlay::Moisture]
        );
        r.cycle_field_overlay();
        assert_eq!(r.field_overlay, FieldOverlay::Off, "cycling should wrap back to Off, not stop at Moisture");
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
    fn zoomed_in_screen_to_world_maps_a_zoom_wide_block_to_one_world_cell() {
        let mut r = Renderer::new();
        r.zoom = 4;
        // Screen columns 0-3 should all land on world column 0; column 4
        // starts the next world cell.
        assert_eq!(r.screen_to_world(0, 0), (0, 0));
        assert_eq!(r.screen_to_world(3, 0), (0, 0));
        assert_eq!(r.screen_to_world(4, 0), (1, 0));
    }

    #[test]
    fn zoomed_out_screen_to_world_skips_by_the_stride() {
        let mut r = Renderer::new();
        r.zoom_out_stride = 3;
        assert_eq!(r.screen_to_world(0, 0), (0, 0));
        assert_eq!(r.screen_to_world(1, 0), (3, 0));
        assert_eq!(r.screen_to_world(2, 0), (6, 0));
    }

    #[test]
    fn screen_to_world_floors_toward_negative_infinity_at_zoom() {
        // Mirrors ChunkCoord::containing's own div_euclid reasoning: a
        // screen position left of the camera must not fold onto the same
        // world cell as one to its right of it.
        let mut r = Renderer::new();
        r.zoom = 2;
        assert_eq!(r.screen_to_world(-1, 0), (-1, 0));
        assert_eq!(r.screen_to_world(-2, 0), (-1, 0));
        assert_eq!(r.screen_to_world(-3, 0), (-2, 0));
    }

    #[test]
    fn adjust_zoom_forms_one_continuous_scale_across_both_fields() {
        let mut r = Renderer::new();
        assert_eq!((r.zoom, r.zoom_out_stride), (1, 1));

        // Zooming out first counts up zoom_out_stride, zoom staying at 1.
        r.adjust_zoom(-1);
        assert_eq!((r.zoom, r.zoom_out_stride), (1, 2));
        r.adjust_zoom(-1);
        assert_eq!((r.zoom, r.zoom_out_stride), (1, 3));

        // Zooming back in counts zoom_out_stride back down to 1 before
        // zoom itself ever climbs above 1.
        r.adjust_zoom(1);
        assert_eq!((r.zoom, r.zoom_out_stride), (1, 2));
        r.adjust_zoom(1);
        assert_eq!((r.zoom, r.zoom_out_stride), (1, 1));
        r.adjust_zoom(1);
        assert_eq!((r.zoom, r.zoom_out_stride), (2, 1));
    }

    #[test]
    fn zoom_and_zoom_out_stride_clamp_at_their_own_bounds() {
        let mut r = Renderer::new();
        for _ in 0..20 {
            r.adjust_zoom(1);
        }
        assert_eq!(r.zoom, MAX_ZOOM, "zoom should clamp rather than grow unbounded");

        let mut r = Renderer::new();
        for _ in 0..20 {
            r.adjust_zoom(-1);
        }
        assert_eq!(r.zoom_out_stride, MAX_ZOOM_OUT_STRIDE, "zoom_out_stride should clamp rather than grow unbounded");
    }

    #[test]
    fn drawing_at_zoom_two_fills_a_two_by_two_block_per_world_cell() {
        let mut world = World::new(Rect::new(0, 0, 15, 15));
        world.set(2, 2, Cell::new(material::STONE, 0));
        let mut r = Renderer::new();
        r.zoom = 2;
        let particles = ParticleSystem::new();
        let (w, h) = (32u32, 32u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        r.draw(&world, &particles, &mut frame, w, h);

        let px = |x: i32, y: i32| -> [u8; 4] {
            let i = ((y as u32 * w + x as u32) * 4) as usize;
            frame[i..i + 4].try_into().unwrap()
        };
        // World cell (2, 2) should occupy screen block (4..6, 4..6).
        let stone_colour = px(4, 4);
        assert_eq!(px(5, 4), stone_colour);
        assert_eq!(px(4, 5), stone_colour);
        assert_eq!(px(5, 5), stone_colour);
        // One column/row past the block should already be back to void
        // (world cell (3, 2), still empty).
        assert_ne!(px(6, 4), stone_colour);
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
