//! Application state: everything the sandbox does that is not windowing.
//!
//! Kept free of winit and pixels types so it stays testable without a GPU or a
//! display, and so the windowing layer can be replaced later without touching
//! any behaviour.

use crate::hud;
use crate::render::{self, Renderer};
use crate::sim::chunk::Rect;
use crate::sim::material::{self, MaterialId, MaterialKind};
use crate::sim::organism;
use crate::sim::parallel;
use crate::sim::particle::ParticleSystem;
use crate::sim::world::World;
use crate::sim::Cell;
use crate::tunables::{self, Tunable};

/// Simulation resolution, in cells. The window is larger; `pixels` scales the
/// framebuffer up, which is what gives the chunky pixel look.
pub const WIDTH: u32 = 512;
pub const HEIGHT: u32 = 320;

/// Fraction of the brush filled per application for loose material. Low enough
/// that a moving brush lays down separated grains, high enough that a stationary
/// one fills in within a handful of frames.
const STREAM_DENSITY: f32 = 0.3;

pub struct App {
    pub world: World,
    pub particles: ParticleSystem,
    pub renderer: Renderer,
    pub brush_radius: i32,
    /// Index into `paintable`, not a `MaterialId`, so cycling wraps cleanly.
    selected: usize,
    paintable: Vec<MaterialId>,
    pub paused: bool,
    /// Set by the step key while paused; consumed by the next `update`.
    pub step_once: bool,
    /// Feedback from the last material/species reload — the only place a
    /// typo in a `.ron` file shows up.
    pub message: Option<String>,
    /// `I` — material/temperature/field readout for whatever's under the
    /// cursor. Off by default (§9 of `PLAN.md`'s UI-improvement pass).
    pub show_hover_inspector: bool,
    /// `Tab` — the material picker as an on-screen swatch row, not just
    /// something inferred from the number keys/status line.
    pub show_palette: bool,
    /// `/` (displayed as `?`) — every control, for a player who hasn't
    /// read `README.md`'s table.
    pub show_help: bool,
    /// `O` — the live-tunables panel (§10 of `PLAN.md`'s UI-improvement
    /// pass). Not `P` — already `spawn_burst`.
    pub show_tunables: bool,
    /// Index into a freshly-rebuilt `tunables::from_materials` list every
    /// time the panel draws or an adjustment/save is applied — there is no
    /// persistent `Vec<Tunable>` on `App` to keep in sync with material
    /// hot-reload, deliberately; see `tunables_list`'s own doc.
    tunables_selected: usize,
}

/// Re-read both the material and species directories over the current
/// registries, returning a combined status message. Shared by `App::new`
/// (the initial load) and `App::reload_materials` (F5 / the file watcher)
/// so the two can never drift into reloading one but not the other.
fn reload_assets(world: &mut World) -> Option<String> {
    let materials = world.materials.reload(material::ASSET_DIR);
    let species = world.species.reload(organism::ASSET_DIR);
    match (materials, species) {
        (Ok(m), Ok(s)) => Some(format!("{m} materials, {s} species")),
        (Ok(m), Err(e)) => Some(format!("{m} materials; species: {e}")),
        (Err(e), Ok(s)) => Some(format!("materials: {e}; {s} species")),
        (Err(me), Err(se)) => Some(format!("materials: {me}; species: {se}")),
    }
}

impl App {
    pub fn new() -> Self {
        let mut world = World::new(Rect::new(0, 0, WIDTH as i32 - 1, HEIGHT as i32 - 1));

        // Load over the compiled-in set, so edits made before launch apply and
        // a broken assets directory still leaves a working engine. Species
        // reload alongside materials -- `Reports/organism-substrate-
        // design.md`'s own stated design ("hot-reloaded via the same notify
        // pattern MaterialRegistry already uses"), which an independent
        // review of the first version of this section caught as never
        // actually wired up: `SpeciesRegistry::reload` existed and was
        // tested, but nothing called it, so editing `assets/species/*.ron`
        // silently did nothing, unlike every material file.
        let message = reload_assets(&mut world);

        build_terrain(&mut world);
        let paintable = world.materials.paintable();
        // Start on sand: the material that most obviously shows whether the
        // simulation is behaving.
        let selected = world
            .materials
            .id_of("sand")
            .and_then(|id| paintable.iter().position(|p| *p == id))
            .unwrap_or(0);

        Self {
            world,
            particles: ParticleSystem::new(),
            renderer: Renderer::new(),
            brush_radius: 6,
            selected,
            paintable,
            paused: false,
            step_once: false,
            message,
            show_hover_inspector: false,
            show_palette: false,
            show_help: false,
            show_tunables: false,
            tunables_selected: 0,
        }
    }

    pub fn toggle_hover_inspector(&mut self) {
        self.show_hover_inspector = !self.show_hover_inspector;
    }

    pub fn toggle_palette(&mut self) {
        self.show_palette = !self.show_palette;
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Resets the selection to the top of the list on every open — a
    /// stale index from a previous session (materials since hot-reloaded,
    /// possibly with a different count) would otherwise point at a
    /// different entry than whatever it last pointed at, silently.
    pub fn toggle_tunables(&mut self) {
        self.show_tunables = !self.show_tunables;
        if self.show_tunables {
            self.tunables_selected = 0;
        }
    }

    /// Freshly rebuilt every call, not cached on `App` — materials are
    /// cheap to enumerate (a few dozen entries) and this sidesteps keeping
    /// a persistent list in sync with hot-reload (`F5`) entirely, the same
    /// tradeoff `tunables.rs`'s own module doc explains.
    fn tunables_list(&self) -> Vec<Tunable> {
        tunables::from_materials(&self.world.materials)
    }

    pub fn tunables_move(&mut self, delta: i32) {
        let len = self.tunables_list().len();
        if len == 0 {
            return;
        }
        let next = self.tunables_selected as i32 + delta;
        self.tunables_selected = next.rem_euclid(len as i32) as usize;
    }

    /// Adjusts the selected tunable's live value by `sign` (-1 or 1) times
    /// its own registered `step`, applied directly to the in-memory
    /// `MaterialRegistry` — felt immediately (the next frame's friction/
    /// flammability/etc. reads the new value), not deferred to a save.
    /// Session-only until `tunables_save` (`Enter`) writes it back to disk.
    pub fn tunables_adjust(&mut self, sign: i32) {
        let Some(t) = self.tunables_list().into_iter().nth(self.tunables_selected) else {
            return;
        };
        let Some(id) = self.world.materials.id_of(&t.category) else {
            return;
        };
        let new_value = (t.value + sign as f32 * t.step).clamp(t.min, t.max);
        let m = self.world.materials.get_mut(id);
        match t.name.as_str() {
            "density" => m.density = new_value,
            "friction_angle" => m.friction_angle = new_value,
            "flammability" => m.flammability = new_value,
            "heat_conductivity" => m.heat_conductivity = new_value,
            "ignition_temperature" => m.ignition_temperature = new_value,
            "burn_temperature" => m.burn_temperature = new_value,
            "melting_point" => m.melting_point = new_value,
            "boiling_point" => m.boiling_point = new_value,
            // `tunables::from_materials` is the only source of entries, so
            // every `t.name` it can produce is handled above -- this is a
            // defensive floor against the two lists drifting apart, not a
            // reachable case today.
            _ => {}
        }
    }

    /// Write the selected tunable's current live value back to its
    /// `.ron` file — a targeted span-edit (`tunables::write_field_value`),
    /// never a full `ron::ser` re-serialization, which would silently
    /// destroy every comment in the file. Verifies the edited text still
    /// parses *before* ever touching disk; on failure, aborts and reports
    /// rather than writing a broken file.
    ///
    /// Known, accepted cosmetic tradeoff (an independent review flagged
    /// this): the disk write below is itself picked up by `main.rs`'s file
    /// watcher a couple hundred milliseconds later, which calls
    /// `reload_materials` and overwrites this method's own "saved X.Y = Z"
    /// confirmation with a generic "reloaded N materials" one. Harmless —
    /// the reload just re-reads the exact value already live in memory —
    /// and not worth a cross-module suppression flag to prevent one
    /// message from briefly outliving another.
    pub fn save_tunable(&mut self) {
        let Some(t) = self.tunables_list().into_iter().nth(self.tunables_selected) else {
            return;
        };
        let path = tunables::material_file_path(material::ASSET_DIR, &t.category);
        let result = (|| -> Result<(), String> {
            let source = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let updated = tunables::write_field_value(&source, &t.name, t.value)?;
            ron::from_str::<material::MaterialDef>(&updated).map_err(|e| format!("edit would corrupt the file: {e}"))?;
            std::fs::write(&path, updated).map_err(|e| e.to_string())?;
            Ok(())
        })();
        self.message = Some(match result {
            Ok(()) => format!("saved {}.{} = {}", t.category, t.name, t.value),
            Err(e) => format!("{}: {e}", path.display()),
        });
    }

    pub fn update(&mut self) {
        if self.paused && !self.step_once {
            return;
        }
        self.step_once = false;
        parallel::step(&mut self.world);
        // Liquid heightfield bodies (`Reports/liquid-heightfield-design.md`
        // §8a) after the sweep -- the sweep is what produces this frame's
        // absorptions once a later step adds them -- and before active
        // sites, so `plant::Absorb` reading an adjacent liquid cell sees
        // this frame's settled body state, the same reasoning the comment
        // below already gives for active sites running after the sweep.
        // Its own serial phase, not inside `parallel::step`, for the reason
        // that design doc section states: a body spanning two same-parity
        // active chunks writing its own columns from both workers would
        // violate the write-disjointness proof `parallel.rs`'s module doc
        // rests on. A no-op today -- step 1 of that design's build order
        // gives every promoted body no solver, so there is nothing yet for
        // this phase to do; wired in now so later steps land here rather
        // than needing frame-order surgery.
        self.world.step_liquid_bodies();
        // M16 active sites after the CA sweep too, for the same reason as
        // particles below: a root deciding whether to drink an adjacent
        // water cell needs this frame's settled position, not last frame's.
        self.world.step_active_sites();
        // Particles after the CA sweep, not before: a landing check needs
        // this frame's fully-settled CA state, not last frame's, or a
        // particle could land inside material that has since moved out from
        // under it. Field after that — order between the two does not
        // currently matter, since particles do not read or write the field,
        // but keeping the CA-derived phases grouped together here is easier
        // to reason about than interleaving them.
        self.particles.step(&mut self.world);
        self.world.step_fields();
    }

    /// `cursor`, when present, is a screen position (framebuffer pixels,
    /// already accounting for window scaling — the same space `main.rs`'s
    /// own `self.cursor` already tracks) — used for the brush outline
    /// preview and the hover inspector, both of which need to know where
    /// the cursor actually is on screen, not just that painting is
    /// currently happening somewhere.
    ///
    /// `&mut self`, unlike nearly every other read-only method here —
    /// `Renderer::draw`'s own §11 dirty-rect skip needs to remember the
    /// zoom/stride it last drew at, to know when the frame buffer it's
    /// about to partially reuse was actually built at a different scale.
    pub fn draw(&mut self, frame: &mut [u8], cursor: Option<(i32, i32)>) {
        // Anything `draw_hud` is about to paint over this frame's terrain
        // -- a panel, the hover inspector, or the brush outline that
        // follows the cursor -- has no footprint tracked from one frame to
        // the next, so `Renderer::draw` has no way to know an old one needs
        // erasing. Forcing a full redraw whenever any of this is showing is
        // the simple, always-correct alternative to tracking that footprint
        // separately; see `Renderer::draw`'s own doc for the full reasoning.
        let force_full = cursor.is_some() || self.show_palette || self.show_help || self.show_tunables || self.show_hover_inspector;
        let touched = self.world.take_touched_chunks();
        self.renderer.draw(&self.world, &self.particles, &touched, frame, (WIDTH, HEIGHT), force_full);
        self.draw_hud(frame, cursor);
    }

    fn draw_hud(&self, frame: &mut [u8], cursor: Option<(i32, i32)>) {
        const WHITE: [u8; 4] = [255, 255, 255, 255];
        const YELLOW: [u8; 4] = [255, 240, 120, 255];

        // Brush label -- always on, bottom-left, the data `status()` above
        // already computes just shown persistently instead of only in the
        // window title bar.
        let label = format!("{} R{}", self.selected_name(), self.brush_radius);
        hud::draw_text(frame, WIDTH, HEIGHT, 4, HEIGHT as i32 - 10, &label, WHITE);

        if let Some((sx, sy)) = cursor {
            // Brush outline preview -- always on while the cursor is in the
            // window, scaled to match whatever `render.rs`'s own zoom is
            // doing so the ring actually matches the area a click would
            // paint, not the unscaled brush_radius regardless of zoom.
            let screen_radius = if self.renderer.zoom > 1 {
                self.brush_radius * self.renderer.zoom
            } else {
                (self.brush_radius / self.renderer.zoom_out_stride.max(1)).max(1)
            };
            render::draw_circle_outline(frame, WIDTH, HEIGHT, sx, sy, screen_radius, WHITE);

            if self.show_hover_inspector {
                self.draw_hover_inspector(frame, sx, sy, YELLOW);
            }
        }

        if self.show_palette {
            self.draw_palette(frame);
        }

        if self.show_help {
            self.draw_help(frame);
        }

        if self.show_tunables {
            self.draw_tunables_panel(frame);
        }
    }

    /// A scrollable list of every registered `Tunable`, grouped by
    /// category (material name) in the order `tunables_list` produces
    /// them, current selection highlighted and shown with its live value
    /// plus a `[SAVED]`/error tag via `self.message` (the same feedback
    /// channel a material reload already uses, reused rather than adding
    /// a second one). Scrolls to keep the selection on screen once the
    /// list is taller than the panel — a fixed window of rows centred
    /// around `tunables_selected` rather than a real scrollbar.
    fn draw_tunables_panel(&self, frame: &mut [u8]) {
        const BG: [u8; 4] = [10, 10, 16, 255];
        const WHITE: [u8; 4] = [235, 235, 235, 255];
        const SELECTED: [u8; 4] = [255, 220, 100, 255];
        let (left, top, right, bottom) = (20, 20, WIDTH as i32 - 20, HEIGHT as i32 - 20);
        for y in top..bottom {
            for x in left..right {
                render::put(frame, WIDTH, HEIGHT, x, y, BG);
            }
        }

        let list = self.tunables_list();
        hud::draw_text(frame, WIDTH, HEIGHT, left + 8, top + 6, "TUNABLES  UP DOWN SELECT  LEFT RIGHT ADJUST  ENTER SAVE  ESC CLOSE", WHITE);
        if list.is_empty() {
            hud::draw_text(frame, WIDTH, HEIGHT, left + 8, top + 20, "NOTHING REGISTERED", WHITE);
            return;
        }

        const ROW_HEIGHT: i32 = 10;
        // Reserved unconditionally, not only when `self.message` is
        // currently `Some` -- a live PNG check caught the list running
        // rows straight through this space when the reservation was
        // message-dependent: a save's resulting message would land on the
        // same pixels as the list's own last row, both unreadable. The
        // footer must stay put whether or not there's a message to put in
        // it, so the two can never collide.
        const FOOTER_HEIGHT: i32 = 16;
        let rows_bottom = bottom - FOOTER_HEIGHT;
        let visible_rows = ((rows_bottom - (top + 20)) / ROW_HEIGHT).max(1) as usize;
        // Centre the window on the selection, clamped so it never scrolls
        // past either end of the list.
        let half = visible_rows / 2;
        let first = self.tunables_selected.saturating_sub(half).min(list.len().saturating_sub(visible_rows));

        for (row, (i, t)) in list.iter().enumerate().skip(first).take(visible_rows).enumerate() {
            let selected = i == self.tunables_selected;
            let colour = if selected { SELECTED } else { WHITE };
            let marker = if selected { ">" } else { " " };
            let line = format!("{marker} {}.{} = {:.3}", t.category, t.name, t.value);
            hud::draw_text(frame, WIDTH, HEIGHT, left + 8, top + 20 + row as i32 * ROW_HEIGHT, &line, colour);
        }

        if let Some(message) = &self.message {
            hud::draw_text(frame, WIDTH, HEIGHT, left + 8, bottom - 12, message, WHITE);
        }
    }

    /// Material name, position, temperature/burning state, and every M13
    /// field channel at the world position under the cursor — every
    /// existing data path `I` toggles into view rather than needing a
    /// debugger to inspect.
    fn draw_hover_inspector(&self, frame: &mut [u8], sx: i32, sy: i32, colour: [u8; 4]) {
        let (wx, wy) = self.renderer.screen_to_world(sx, sy);
        let cell = self.world.get(wx, wy);
        let material = self.world.materials.get(cell.material).display.clone();
        let field = self.world.field_at(wx, wy);
        let lines = [
            format!("{material} ({wx},{wy})"),
            format!("TEMP {}C{}", cell.temperature(), if cell.is_burning() { " BURNING" } else { "" }),
            format!("P{:.1} T{:.0} L{:.1} M{:.1}", field.pressure, field.temperature, field.light, field.moisture),
        ];
        for (i, line) in lines.iter().enumerate() {
            hud::draw_text(frame, WIDTH, HEIGHT, 4, 4 + i as i32 * 9, line, colour);
        }
    }

    /// Swatch row along the bottom for every paintable material — `Tab` —
    /// the material picker made visible instead of only inferred from the
    /// number keys or the status line's own material name.
    fn draw_palette(&self, frame: &mut [u8]) {
        const SWATCH: i32 = 7;
        const GAP: i32 = 2;
        let y = HEIGHT as i32 - 24;
        for (i, &id) in self.paintable.iter().enumerate() {
            let x = 4 + i as i32 * (SWATCH + GAP);
            let colour = self.world.materials.get(id).palette[0];
            for dy in 0..SWATCH {
                for dx in 0..SWATCH {
                    render::put(frame, WIDTH, HEIGHT, x + dx, y + dy, colour);
                }
            }
            if i == self.selected {
                let border = [255, 255, 255, 255];
                for dx in -1..=SWATCH {
                    render::put(frame, WIDTH, HEIGHT, x + dx, y - 1, border);
                    render::put(frame, WIDTH, HEIGHT, x + dx, y + SWATCH, border);
                }
                for dy in -1..=SWATCH {
                    render::put(frame, WIDTH, HEIGHT, x - 1, y + dy, border);
                    render::put(frame, WIDTH, HEIGHT, x + SWATCH, y + dy, border);
                }
            }
        }
    }

    /// Every control from `README.md`'s own table, including everything
    /// this UI-improvement pass added — `/` (labelled `?` in-app, since
    /// `/` itself has no glyph reserved for it and `?` is what a player
    /// actually looks for).
    fn draw_help(&self, frame: &mut [u8]) {
        const BG: [u8; 4] = [10, 10, 16, 255];
        const WHITE: [u8; 4] = [235, 235, 235, 255];
        let (left, top, right, bottom) = (20, 20, WIDTH as i32 - 20, HEIGHT as i32 - 20);
        for y in top..bottom {
            for x in left..right {
                render::put(frame, WIDTH, HEIGHT, x, y, BG);
            }
        }
        let lines = [
            "SPACE PAUSE    . STEP    R RESET",
            "F1 CHUNK OVERLAY    F5 RELOAD",
            "LEFT CLICK PAINT    RIGHT CLICK ERASE",
            "[ ] BRUSH SIZE    = - ZOOM",
            "Q E CYCLE MATERIAL    1-9 SELECT",
            "F IGNITE    P BURST    X EXPLODE",
            "T PLANT TREE    M PLANT MOSS    W PLANT WORM",
            "I HOVER INSPECTOR    V FIELD OVERLAY",
            "TAB MATERIAL PALETTE    ? THIS HELP",
            "G WATER GRAIN",
        ];
        for (i, line) in lines.iter().enumerate() {
            hud::draw_text(frame, WIDTH, HEIGHT, left + 8, top + 8 + i as i32 * 10, line, WHITE);
        }
    }

    /// Re-read the material and species files. Ids are keyed by name, so
    /// material/species already in the world keep their identity and simply
    /// start behaving differently.
    pub fn reload_materials(&mut self) {
        self.message = reload_assets(&mut self.world).map(|s| format!("reloaded {s}"));
        // A new material file adds an id, so the picker has to be rebuilt.
        let current = self.selected_material();
        self.paintable = self.world.materials.paintable();
        self.selected = self
            .paintable
            .iter()
            .position(|p| *p == current)
            .unwrap_or(0);
        // Friction and dispersion may have changed, so nothing at rest can be
        // assumed to still be at rest.
        self.world.wake_all();
        // A reload can change which conditional fields (ignition_temperature
        // et al.) are registered per material, shifting every later
        // flattened `tunables_list` index -- an independent review pointed
        // out that leaving a stale `tunables_selected` in place could land
        // a subsequent save on a field the player never selected. Reset to
        // the top, same as `toggle_tunables` already does on open, rather
        // than trying to track identity across a rebuild neither list nor
        // registry otherwise needs.
        self.tunables_selected = 0;
    }

    pub fn selected_material(&self) -> MaterialId {
        self.paintable[self.selected]
    }

    pub fn selected_name(&self) -> &str {
        &self.world.materials.get(self.selected_material()).display
    }

    pub fn cycle_material(&mut self, delta: i32) {
        let n = self.paintable.len() as i32;
        self.selected = (((self.selected as i32 + delta) % n + n) % n) as usize;
    }

    /// Select by number key. `n` is 1-based to match the key labels; out of
    /// range presses are ignored.
    pub fn select_material(&mut self, n: usize) {
        if n >= 1 && n <= self.paintable.len() {
            self.selected = n - 1;
        }
    }

    pub fn adjust_brush(&mut self, delta: i32) {
        self.brush_radius = (self.brush_radius + delta).clamp(1, 64);
    }

    /// Paint at a screen position. `erase` swaps the brush for vacuum.
    pub fn paint(&mut self, screen_x: i32, screen_y: i32, erase: bool) {
        self.paint_stroke((screen_x, screen_y), (screen_x, screen_y), erase);
    }

    /// Paint the area the brush swept between two screen positions, so a fast
    /// drag leaves one continuous stroke rather than a row of blobs.
    pub fn paint_stroke(&mut self, from: (i32, i32), to: (i32, i32), erase: bool) {
        let m = if erase {
            material::EMPTY
        } else {
            self.selected_material()
        };
        let from = self.renderer.screen_to_world(from.0, from.1);
        let to = self.renderer.screen_to_world(to.0, to.1);
        let density = self.emission_density(m, erase);
        self.world
            .paint_capsule(from, to, self.brush_radius, m, density);
    }

    /// Force-ignite the brush area at a screen position. See
    /// `World::ignite_circle` for why this exists as a debug tool ahead of
    /// M15's more physical ignition sources.
    pub fn ignite(&mut self, screen_x: i32, screen_y: i32) {
        let (x, y) = self.renderer.screen_to_world(screen_x, screen_y);
        self.world.ignite_circle(x, y, self.brush_radius);
    }

    /// Throw a small burst of the selected material as free particles from a
    /// screen position — a debug tool for M7 the same way `ignite` is for
    /// M14. `explode` below is the real, physically-grounded reason
    /// `ParticleSystem::spawn` exists; this stays as a cheaper, more
    /// predictable way to sanity-check particle rendering/physics on their
    /// own, without a field impulse or CA destruction mixed in.
    pub fn spawn_burst(&mut self, screen_x: i32, screen_y: i32) {
        let (x, y) = self.renderer.screen_to_world(screen_x, screen_y);
        let material = self.selected_material();
        let shades = self.world.materials.get(material).palette.len().max(1) as u32;
        const COUNT: i32 = 24;
        for i in 0..COUNT {
            // Spread across an upward-biased arc rather than a full circle —
            // reads as "thrown," which is the case this exists to demo,
            // rather than "leaking outward in every direction at once."
            let angle = -std::f32::consts::FRAC_PI_2
                + (i as f32 / COUNT as f32 - 0.5) * std::f32::consts::PI;
            let speed = 3.0 + self.world.rng.below(30) as f32 / 10.0;
            let shade = self.world.rng.below(shades) as u8;
            self.particles.spawn(
                x as f32,
                y as f32,
                angle.cos() * speed,
                angle.sin() * speed,
                material,
                shade,
            );
        }
    }

    /// Trigger an explosion at a screen position, using the brush radius and
    /// a fixed strength. See `sim::explosion::trigger` for what this actually
    /// does — a pressure impulse and heat spike into the field, plus a
    /// radius of cells converted to thrown debris or vacuum.
    pub fn explode(&mut self, screen_x: i32, screen_y: i32) {
        let (x, y) = self.renderer.screen_to_world(screen_x, screen_y);
        const STRENGTH: f32 = 180.0;
        crate::sim::explosion::trigger(&mut self.world, &mut self.particles, x, y, self.brush_radius, STRENGTH);
    }

    /// Plant a tree seed at a screen position — M16 debug tool. See
    /// `World::plant_tree` for what actually grows from it.
    pub fn plant_tree(&mut self, screen_x: i32, screen_y: i32) {
        let (x, y) = self.renderer.screen_to_world(screen_x, screen_y);
        self.world.plant_tree(x, y);
    }

    /// Plant a moss seed at a screen position — M16 debug tool.
    pub fn plant_moss(&mut self, screen_x: i32, screen_y: i32) {
        let (x, y) = self.renderer.screen_to_world(screen_x, screen_y);
        self.world.plant_moss_seed(x, y);
    }

    /// Plant a worm at a screen position — M18 debug tool.
    pub fn plant_worm(&mut self, screen_x: i32, screen_y: i32) {
        let (x, y) = self.renderer.screen_to_world(screen_x, screen_y);
        self.world.plant_worm(x, y);
    }

    /// How much of the brush to fill per application.
    ///
    /// Loose material is emitted as a scatter so a held brush looks like a
    /// stream pouring rather than a solid slab appearing under the cursor —
    /// grains separate as they fall, which is the look sand actually has.
    /// Building with solids and erasing both want a crisp, fully filled brush.
    fn emission_density(&self, m: MaterialId, erase: bool) -> f32 {
        if erase || self.world.materials.kind(m) == MaterialKind::Solid {
            1.0
        } else {
            STREAM_DENSITY
        }
    }

    pub fn reset(&mut self) {
        let bounds = self.world.bounds().expect("fixed world always has bounds");
        let mut world = World::new(bounds);
        // Carry the loaded materials across rather than dropping back to the
        // compiled-in set.
        std::mem::swap(&mut world.materials, &mut self.world.materials);
        build_terrain(&mut world);
        self.world = world;
    }

    pub fn toggle_overlay(&mut self) {
        self.renderer.show_chunk_overlay = !self.renderer.show_chunk_overlay;
    }

    /// Status line for the window title — cheaper than rendering text, and
    /// enough to verify frame rate and sleeping at a glance.
    pub fn status(&self, fps: f32) -> String {
        format!(
            "Pixel Physics — {:.0} fps — {} (brush {}) — chunks {}/{} awake{}{}{}",
            fps,
            self.selected_name(),
            self.brush_radius,
            self.world.active_chunk_count(),
            self.world.chunk_count(),
            if self.paused { " — PAUSED" } else { "" },
            // Only shown once it has been changed, so the ordinary status
            // line is untouched until someone is actually comparing modes.
            if self.renderer.grain == render::GrainMode::Position {
                String::new()
            } else {
                format!(" — grain {}", self.renderer.grain.label())
            },
            match &self.message {
                Some(m) => format!(" — {m}"),
                None => String::new(),
            },
        )
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// A floor and a couple of ledges, so there is something for material to
/// interact with on startup instead of an empty box.
fn build_terrain(world: &mut World) {
    let w = WIDTH as i32;
    let h = HEIGHT as i32;
    // Always present: `reload` only ever adds or updates, so the compiled-in
    // stone cannot be removed by editing the assets directory.
    let stone = world
        .materials
        .id_of("stone")
        .expect("stone is a compiled-in material");

    for x in 0..w {
        for y in (h - 8)..h {
            world.set(x, y, Cell::new(stone, (x % 4) as u8));
        }
    }

    let mut ledge = |x0: i32, x1: i32, y: i32| {
        for x in x0..x1 {
            for dy in 0..6 {
                world.set(x, y + dy, Cell::new(stone, (x % 4) as u8));
            }
        }
    };
    ledge(60, 200, 200);
    ledge(300, 440, 150);
    ledge(180, 320, 260);

    // The world is left dirty on purpose. The first sweep examines the terrain,
    // finds that none of it moves, and settles from the second frame onward.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(app: &App, name: &str) -> MaterialId {
        app.world.materials.id_of(name).expect(name)
    }

    /// Cells of `id` inside a square window around a point.
    fn count_near(app: &App, cx: i32, cy: i32, r: i32, want: MaterialId) -> usize {
        let mut n = 0;
        for y in (cy - r)..=(cy + r) {
            for x in (cx - r)..=(cx + r) {
                if app.world.get(x, y).material == want {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn starts_on_sand() {
        let app = App::new();
        assert_eq!(app.selected_material(), id(&app, "sand"));
        assert_eq!(app.selected_name(), "Sand");
    }

    #[test]
    fn materials_load_from_the_assets_directory() {
        let app = App::new();
        // Every shipped material should be offered in the picker.
        for name in ["stone", "sand", "gravel", "ash", "water", "oil", "smoke"] {
            let m = id(&app, name);
            assert!(app.paintable.contains(&m), "{name} is not paintable");
        }
    }

    #[test]
    fn reloading_materials_keeps_the_current_selection() {
        let mut app = App::new();
        app.select_material(2);
        let before = app.selected_material();
        app.reload_materials();
        assert_eq!(app.selected_material(), before);
    }

    #[test]
    fn reloading_materials_wakes_the_world() {
        // Changed friction or dispersion can unstick material that had settled,
        // so everything must be re-examined.
        let mut app = App::new();
        app.update();
        app.update();
        assert_eq!(app.world.active_chunk_count(), 0);
        app.reload_materials();
        assert!(app.world.active_chunk_count() > 0, "reload left the world asleep");
    }

    #[test]
    fn cycling_materials_wraps_in_both_directions() {
        let mut app = App::new();
        let n = app.paintable.len();
        let first = app.selected_material();
        for _ in 0..n {
            app.cycle_material(1);
        }
        assert_eq!(app.selected_material(), first, "forward cycle did not wrap");
        for _ in 0..n {
            app.cycle_material(-1);
        }
        assert_eq!(app.selected_material(), first, "backward cycle did not wrap");
        // A single backward step from the *first* entry must land on the last.
        app.select_material(1);
        app.cycle_material(-1);
        assert_eq!(app.selected_material(), app.paintable[n - 1]);
    }

    #[test]
    fn number_keys_select_and_ignore_out_of_range() {
        let mut app = App::new();
        app.select_material(1);
        assert_eq!(app.selected_material(), app.paintable[0]);
        let before = app.selected_material();
        app.select_material(0);
        app.select_material(99);
        assert_eq!(app.selected_material(), before);
    }

    #[test]
    fn brush_radius_is_clamped() {
        let mut app = App::new();
        app.adjust_brush(-1000);
        assert_eq!(app.brush_radius, 1);
        app.adjust_brush(1000);
        assert_eq!(app.brush_radius, 64);
    }

    #[test]
    fn static_terrain_settles_and_stays_settled() {
        let mut app = App::new();
        // Two sweeps, because dirty regions are double buffered: the terrain
        // writes made before the first frame are only promoted into the swept
        // region by the end of it, so the second frame is the one that examines
        // the stone and concludes it does not move.
        app.update();
        app.update();
        assert_eq!(
            app.world.active_chunk_count(),
            0,
            "static starting terrain kept chunks awake"
        );

        for _ in 0..10 {
            app.update();
            assert_eq!(app.world.active_chunk_count(), 0, "a settled chunk woke itself");
        }
    }

    #[test]
    fn painting_adds_material_and_erasing_removes_it() {
        let mut app = App::new();
        let sand = id(&app, "sand");
        app.paint(100, 50, false);
        assert!(count_near(&app, 100, 50, 8, sand) > 0, "brush laid down nothing");
        app.paint(100, 50, true);
        assert_eq!(count_near(&app, 100, 50, 8, sand), 0, "eraser left material behind");
    }

    #[test]
    fn a_held_brush_fills_in_within_a_few_frames() {
        // Emitting a scatter must not make the brush feel unresponsive: holding
        // still should still produce a solid mass quickly.
        let mut app = App::new();
        let sand = id(&app, "sand");
        app.brush_radius = 4;
        for _ in 0..12 {
            app.paint(100, 50, false);
        }
        let filled = count_near(&app, 100, 50, 1, sand);
        assert!(filled >= 8, "held brush filled only {filled} of 9 central cells");
    }

    #[test]
    fn a_dragged_brush_lays_down_a_sparse_trail() {
        // The point of the scatter: a single fast drag emits grains, not a slab.
        let mut app = App::new();
        let sand = id(&app, "sand");
        app.brush_radius = 3;
        app.paint_stroke((20, 20), (120, 20), false);

        let painted = (20..=120)
            .filter(|&x| app.world.get(x, 20).material == sand)
            .count();
        assert!(painted > 10, "stroke laid down almost nothing ({painted})");
        assert!(painted < 90, "stroke was a solid slab, not a stream ({painted})");
    }

    #[test]
    fn a_solid_brush_is_not_scattered() {
        // Building terrain needs a crisp, fully filled brush.
        let mut app = App::new();
        let stone = id(&app, "stone");
        app.brush_radius = 3;
        app.select_material(1);
        assert_eq!(app.selected_material(), stone);
        app.paint_stroke((20, 20), (120, 20), false);
        for x in 20..=120 {
            assert_eq!(
                app.world.get(x, 20).material,
                stone,
                "gap in a solid stroke at x = {x}"
            );
        }
    }

    #[test]
    fn the_eraser_sweeps_the_whole_drag_path() {
        let mut app = App::new();
        app.brush_radius = 3;
        app.select_material(1);
        app.paint_stroke((20, 20), (120, 20), false);
        app.paint_stroke((20, 20), (120, 20), true);
        for x in 20..=120 {
            assert!(app.world.get(x, 20).is_empty(), "eraser left material at x = {x}");
        }
    }

    #[test]
    fn pausing_freezes_the_simulation() {
        let mut app = App::new();
        app.paint(100, 20, false);
        app.paused = true;
        let before = app.world.get(100, 20);
        app.update();
        assert_eq!(app.world.get(100, 20), before, "paused world advanced");

        app.step_once = true;
        app.update();
        assert!(!app.step_once, "step flag was not consumed");
    }

    #[test]
    fn reset_clears_painted_material_but_keeps_terrain_and_materials() {
        let mut app = App::new();
        let sand = id(&app, "sand");
        let stone = id(&app, "stone");
        app.paint(100, 20, false);
        assert!(count_near(&app, 100, 20, 8, sand) > 0);

        app.reset();
        assert_eq!(
            count_near(&app, 100, 20, 8, sand),
            0,
            "reset left painted material behind"
        );
        assert_eq!(
            app.world.get(10, HEIGHT as i32 - 1).material,
            stone,
            "reset lost the floor"
        );
        // Reset must not throw away materials loaded from disk.
        assert!(app.world.materials.id_of("gravel").is_some());
    }

    #[test]
    fn sand_dropped_on_the_floor_settles_and_the_world_sleeps_again() {
        let mut app = App::new();
        app.paint(256, 20, false);
        for _ in 0..3000 {
            app.update();
        }
        assert_eq!(
            app.world.active_chunk_count(),
            0,
            "world never settled after painting"
        );
    }
}
