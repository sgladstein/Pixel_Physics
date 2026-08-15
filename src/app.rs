//! Application state: everything the sandbox does that is not windowing.
//!
//! Kept free of winit and pixels types so it stays testable without a GPU or a
//! display, and so the windowing layer can be replaced later without touching
//! any behaviour.

use crate::hud;
use crate::render::{self, Renderer};
use crate::sim::chunk::Rect;
use crate::sim::explosion;
use crate::sim::material::{self, MaterialId, MaterialKind};
use crate::sim::organism;
use crate::sim::parallel;
use crate::sim::particle::ParticleSystem;
use crate::sim::structural;
use crate::sim::world::World;
use crate::sim::Cell;
use crate::tunables::{self, Tunable, TunableGroup};

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
    /// Explosions currently expanding, plus their live tuning. A blast is
    /// no longer a single-frame event (`sim::explosion`'s own module doc has
    /// the measurements), so it needs somewhere to live between frames.
    pub blasts: explosion::Blasts,
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
    /// Which menu the tunables panel is showing. `PageUp`/`PageDown`.
    tunables_group: TunableGroup,
    /// The tunable currently pinned for live adjustment with the panel
    /// *closed* — `Enter` in the panel sets it and closes.
    ///
    /// The panel covers most of the screen, so anything judged by eye could
    /// not be judged while it was open: you adjusted blind, closed, looked,
    /// reopened, scrolled back. Pinning is the fix — a one-line readout in
    /// the corner and the same left/right keys, with the world fully
    /// visible. Stored by identity (group/category/name), never by value,
    /// so it stays correct across a hot-reload that rebuilds the list.
    pinned: Option<(TunableGroup, String, String)>,
    /// `K` — whether the current A/B experiment is on. See `toggle_experiment`.
    pub experiment: bool,
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
            blasts: explosion::Blasts::with_tuning(explosion::Tuning::load()),
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
            tunables_group: TunableGroup::Physics,
            pinned: None,
            experiment: false,
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
        self.all_tunables().into_iter().filter(|t| t.group == self.tunables_group).collect()
    }

    /// Every registered tunable from every source, ungrouped — materials
    /// plus the engine structs that are not materials at all.
    fn all_tunables(&self) -> Vec<Tunable> {
        let mut out = tunables::from_materials(&self.world.materials);
        out.extend(tunables::from_explosion(&self.blasts.tuning));
        out
    }

    /// `PageUp`/`PageDown` — switch which menu the panel shows. Resets the
    /// selection, since an index into one group means nothing in another.
    pub fn tunables_cycle_group(&mut self) {
        self.tunables_group = self.tunables_group.next();
        self.tunables_selected = 0;
    }

    /// `K` — flip whatever is being evaluated right now between its baseline
    /// and its candidate value, so an A/B comparison is one keypress instead
    /// of a scroll through a panel.
    ///
    /// **Deliberately rewritten whenever the question changes**, and the
    /// previous experiment deleted rather than accumulated — the whole point
    /// is that this one key always means "the thing I am looking at today".
    /// An experiment that turns out to matter graduates into a real tunable
    /// or a chosen default; one that does not is simply overwritten.
    ///
    /// Current experiment: **water's `fill_dimming`**, the long-standing
    /// 0.65 against 0.20. Motivation and measurements are in that field's
    /// own doc — a settled waterline's top row spans fill 286..1002, which
    /// at 0.65 draws across 54%..100% brightness and reads as a mottled band
    /// rather than a clean edge.
    pub fn toggle_experiment(&mut self) {
        self.experiment = !self.experiment;
        let value = if self.experiment { 0.20 } else { 0.65 };
        if let Some(id) = self.world.materials.id_of("water") {
            self.world.materials.get_mut(id).fill_dimming = value;
        }
        self.message = Some(format!("water fill_dimming {value:.2}"));
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
        self.apply_adjust(&t, sign);
    }

    /// Nudge `t` by one `step` in `sign`'s direction and write it back to
    /// whichever live store it came from. Split out of `tunables_adjust` so
    /// the pinned path (`adjust_pinned`) drives exactly the same code rather
    /// than a parallel copy of it.
    fn apply_adjust(&mut self, t: &Tunable, sign: i32) {
        let new_value = (t.value + sign as f32 * t.step).clamp(t.min, t.max);
        if t.group == TunableGroup::Explosion {
            tunables::apply_explosion(&mut self.blasts.tuning, &t.name, new_value);
            self.message = Some(format!("{}.{} = {new_value:.3}", t.category, t.name));
            return;
        }
        let Some(id) = self.world.materials.id_of(&t.category) else {
            return;
        };
        let m = self.world.materials.get_mut(id);
        match t.name.as_str() {
            "density" => m.density = new_value,
            "friction_angle" => m.friction_angle = new_value,
            "flammability" => m.flammability = new_value,
            "min_transfer" => m.min_transfer = new_value.max(0.0).round() as u16,
            "flow_rate" => m.flow_rate = new_value.max(0.0).round() as u16,
            "fill_dimming" => m.fill_dimming = new_value,
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
        self.message = Some(format!("{}.{} = {new_value:.3}", t.category, t.name));
    }

    /// `Enter` in the panel: remember the highlighted entry and close, so it
    /// can be swept with the world actually visible. See `App::pinned`.
    pub fn pin_selected(&mut self) {
        let Some(t) = self.tunables_list().into_iter().nth(self.tunables_selected) else {
            return;
        };
        self.message = Some(format!("pinned {}.{} -- LEFT/RIGHT to adjust, ESC to release", t.category, t.name));
        self.pinned = Some((t.group, t.category, t.name));
        self.show_tunables = false;
    }

    pub fn clear_pin(&mut self) {
        if self.pinned.take().is_some() {
            self.message = Some("released".into());
        }
    }

    pub fn has_pin(&self) -> bool {
        self.pinned.is_some()
    }

    /// The pinned entry, re-read live from whichever registry owns it, so
    /// the readout and the next adjustment both see the current value rather
    /// than whatever it was when it was pinned.
    fn pinned_tunable(&self) -> Option<Tunable> {
        let (group, category, name) = self.pinned.as_ref()?;
        self.all_tunables()
            .into_iter()
            .find(|t| t.group == *group && &t.category == category && &t.name == name)
    }

    /// Left/right with the panel closed.
    pub fn adjust_pinned(&mut self, sign: i32) {
        let Some(t) = self.pinned_tunable() else { return };
        self.apply_adjust(&t, sign);
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
        // Engine tunables have no material file to span-edit -- the whole
        // struct round-trips to its own generated file instead. See
        // `explosion::Tuning::save` for why a full re-serialization is fine
        // there and emphatically not fine for materials.
        if t.group == TunableGroup::Explosion {
            self.message = Some(match self.blasts.tuning.save() {
                Ok(()) => format!("saved {}", explosion::Tuning::ASSET_PATH),
                Err(e) => format!("{}: {e}", explosion::Tuning::ASSET_PATH),
            });
            return;
        }
        let path = tunables::material_file_path(material::ASSET_DIR, &t.category);
        let result = (|| -> Result<(), String> {
            let source = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let updated = tunables::write_field_value(&source, &t.name, t.value, t.integral)?;
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
        // M8 chunk bodies in the same slot and for the same reason: a body
        // spanning two same-parity chunks would write to both from separate
        // workers and break `parallel.rs`'s write-disjointness proof
        // (`Reports/coupling-research.md` §4 states this outright), so it
        // gets its own serial phase. Before active sites, so a structural
        // check this frame sees a landed chunk's cells already in the grid
        // rather than a frame-old hole where they used to be.
        crate::sim::rigid::step_chunk_bodies(&mut self.world);
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
        // Blasts before particles: a blast stage clears cells and spawns
        // debris, and that debris should get its first `particle::step`
        // against the cavity this stage just opened rather than waiting a
        // frame for it -- which is the whole reason staging helps debris
        // escape at all (`sim::explosion::Tuning::duration`).
        self.blasts.step(&mut self.world, &mut self.particles);
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
        let force_full = cursor.is_some() || self.show_palette || self.show_help || self.show_tunables || self.show_hover_inspector || self.pinned.is_some();
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
        // The help panel existed from the start and was invisible unless you
        // already knew the key -- which is the same as not existing. Hidden
        // only while help itself is open, where it would be redundant.
        if !self.show_help {
            hud::draw_text(frame, WIDTH, HEIGHT, WIDTH as i32 - 56, HEIGHT as i32 - 10, "? HELP", WHITE);
        }

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
        } else {
            // Only with the panel closed -- the whole point of pinning is
            // watching the world, and the panel already shows the value.
            self.draw_pinned_readout(frame);
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
    /// The live-tunables panel (`O`).
    ///
    /// Translucent rather than opaque, by request: the panel covers most of
    /// the screen, and a solid fill meant nothing being tuned could be seen
    /// while tuning it. The world still shows through at `PANEL_ALPHA`, the
    /// selected row gets a brighter bar behind it so it stays readable
    /// against whatever happens to be underneath, and a value bar shows each
    /// entry's position within its own min..max range — which is the one
    /// piece of information a bare number never gave: whether there is any
    /// headroom left in the direction you are pushing.
    fn draw_tunables_panel(&self, frame: &mut [u8]) {
        const PANEL: [u8; 4] = [10, 10, 16, 255];
        const PANEL_ALPHA: f32 = 0.78;
        /// The selected row is *lifted* out of the panel, not pushed further
        /// into it. Blending the panel colour over itself was the first
        /// attempt and it darkened the row instead of highlighting it --
        /// visible immediately in a rendered PNG, invisible in the code.
        const ROW_LIFT: [u8; 4] = [46, 58, 82, 255];
        const ROW_ALPHA: f32 = 0.85;
        const WHITE: [u8; 4] = [225, 228, 235, 255];
        const DIM: [u8; 4] = [140, 146, 158, 255];
        const SELECTED: [u8; 4] = [255, 220, 100, 255];
        const ACCENT: [u8; 4] = [90, 170, 240, 255];
        const BAR: [u8; 4] = [70, 96, 130, 255];

        let (left, top, right, bottom) = (20, 20, WIDTH as i32 - 20, HEIGHT as i32 - 20);
        for y in top..bottom {
            for x in left..right {
                render::blend(frame, WIDTH, HEIGHT, x, y, PANEL, PANEL_ALPHA);
            }
        }
        // A one-pixel border, so the panel still reads as a panel now that
        // its fill no longer fully hides what is behind it.
        for x in left..right {
            render::put(frame, WIDTH, HEIGHT, x, top, ACCENT);
            render::put(frame, WIDTH, HEIGHT, x, bottom - 1, ACCENT);
        }
        for y in top..bottom {
            render::put(frame, WIDTH, HEIGHT, left, y, ACCENT);
            render::put(frame, WIDTH, HEIGHT, right - 1, y, ACCENT);
        }

        let list = self.tunables_list();
        hud::draw_text(frame, WIDTH, HEIGHT, left + 8, top + 6, &format!("TUNABLES  [{}]", self.tunables_group.label()), SELECTED);
        hud::draw_text(
            frame,
            WIDTH,
            HEIGHT,
            left + 8,
            top + 16,
            "PGUP/PGDN MENU   UP/DOWN SELECT   LEFT/RIGHT ADJUST   ENTER PIN+CLOSE   S SAVE",
            DIM,
        );
        if list.is_empty() {
            hud::draw_text(frame, WIDTH, HEIGHT, left + 8, top + 30, "NOTHING REGISTERED", WHITE);
            return;
        }

        const ROW_HEIGHT: i32 = 10;
        const HEADER_HEIGHT: i32 = 30;
        // Reserved unconditionally, not only when `self.message` is
        // currently `Some` -- a live PNG check caught the list running
        // rows straight through this space when the reservation was
        // message-dependent: a save's resulting message would land on the
        // same pixels as the list's own last row, both unreadable. The
        // footer must stay put whether or not there's a message to put in
        // it, so the two can never collide.
        const FOOTER_HEIGHT: i32 = 16;
        let rows_top = top + HEADER_HEIGHT;
        let rows_bottom = bottom - FOOTER_HEIGHT;
        let visible_rows = ((rows_bottom - rows_top) / ROW_HEIGHT).max(1) as usize;
        // Centre the window on the selection, clamped so it never scrolls
        // past either end of the list.
        let half = visible_rows / 2;
        let first = self.tunables_selected.saturating_sub(half).min(list.len().saturating_sub(visible_rows));

        // Where the value bar lives, right-aligned inside the panel.
        let bar_w = 60;
        let bar_x = right - 8 - bar_w;

        for (row, (i, t)) in list.iter().enumerate().skip(first).take(visible_rows).enumerate() {
            let selected = i == self.tunables_selected;
            let y = rows_top + row as i32 * ROW_HEIGHT;
            if selected {
                for by in y - 1..y + ROW_HEIGHT - 1 {
                    for bx in left + 1..right - 1 {
                        render::blend(frame, WIDTH, HEIGHT, bx, by, ROW_LIFT, ROW_ALPHA);
                    }
                }
            }
            let colour = if selected { SELECTED } else { WHITE };
            let marker = if selected { ">" } else { " " };
            let line = format!("{marker} {}.{}", t.category, t.name);
            hud::draw_text(frame, WIDTH, HEIGHT, left + 8, y, &line, colour);
            hud::draw_text(frame, WIDTH, HEIGHT, bar_x - 52, y, &format!("{:>8.3}", t.value), colour);

            // Fill fraction within min..max. Guards a degenerate range
            // rather than dividing by zero -- nothing registers one today,
            // but a future entry with min == max would otherwise produce NaN
            // and a bar of unpredictable width.
            let span = t.max - t.min;
            let frac = if span > 0.0 { ((t.value - t.min) / span).clamp(0.0, 1.0) } else { 0.0 };
            let filled = (bar_w as f32 * frac).round() as i32;
            for bx in 0..bar_w {
                let c = if bx < filled {
                    if selected { SELECTED } else { ACCENT }
                } else {
                    BAR
                };
                // Three rows tall: a one-pixel bar was drawn first and read
                // as a dashed hairline at this resolution rather than as a
                // gauge -- caught by looking at the rendered panel.
                for by in 2..5 {
                    render::put(frame, WIDTH, HEIGHT, bar_x + bx, y + by, c);
                }
            }
        }

        if let Some(message) = &self.message {
            hud::draw_text(frame, WIDTH, HEIGHT, left + 8, bottom - 12, message, DIM);
        }
    }

    /// The pinned-tunable readout — one line, bottom-left, drawn only while
    /// something is pinned and the panel is closed. See `App::pinned`.
    fn draw_pinned_readout(&self, frame: &mut [u8]) {
        const BG: [u8; 4] = [10, 10, 16, 255];
        const SELECTED: [u8; 4] = [255, 220, 100, 255];
        let Some(t) = self.pinned_tunable() else { return };

        let line = format!("<{}.{}={:.3}>", t.category, t.name, t.value);
        let w = (line.chars().count() as i32) * 6 + 10;
        let (x, y) = (4, HEIGHT as i32 - 40);
        for by in y - 3..y + 11 {
            for bx in x - 3..x + w {
                render::blend(frame, WIDTH, HEIGHT, bx, by, BG, 0.72);
            }
        }
        hud::draw_text(frame, WIDTH, HEIGHT, x + 2, y, &line, SELECTED);

        // The same min..max bar the panel draws, so the readout carries the
        // one thing the number alone cannot: how much room is left.
        let span = t.max - t.min;
        let frac = if span > 0.0 { ((t.value - t.min) / span).clamp(0.0, 1.0) } else { 0.0 };
        let bar_w = w - 8;
        let filled = (bar_w as f32 * frac).round() as i32;
        for bx in 0..bar_w {
            let c = if bx < filled { SELECTED } else { [70, 96, 130, 255] };
            for by in 8..10 {
                render::put(frame, WIDTH, HEIGHT, x + 2 + bx, y + by, c);
            }
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
        const WHITE: [u8; 4] = [225, 228, 235, 255];
        const ACCENT: [u8; 4] = [90, 170, 240, 255];
        let (left, top, right, bottom) = (20, 20, WIDTH as i32 - 20, HEIGHT as i32 - 20);
        // Translucent, matching the tunables panel -- see its own doc.
        for y in top..bottom {
            for x in left..right {
                render::blend(frame, WIDTH, HEIGHT, x, y, BG, 0.88);
            }
        }
        for x in left..right {
            render::put(frame, WIDTH, HEIGHT, x, top, ACCENT);
            render::put(frame, WIDTH, HEIGHT, x, bottom - 1, ACCENT);
        }
        for y in top..bottom {
            render::put(frame, WIDTH, HEIGHT, left, y, ACCENT);
            render::put(frame, WIDTH, HEIGHT, right - 1, y, ACCENT);
        }
        let lines = [
            "LEFT CLICK PAINT    RIGHT CLICK ERASE",
            "Q E CYCLE MATERIAL    1-9 SELECT    [ ] BRUSH",
            "SPACE PAUSE    . STEP    R RESET    = - ZOOM",
            "",
            "F IGNITE    P BURST    X EXPLODE",
            "T PLANT TREE    M PLANT MOSS    W PLANT WORM",
            "",
            "TAB PALETTE    I INSPECTOR    V FIELD OVERLAY",
            "F1 CHUNK OVERLAY    G WATER GRAIN",
            "",
            "O TUNABLES  (PGUP PGDN MENU, ARROWS SELECT/ADJUST,",
            "             ENTER PIN AND CLOSE, S SAVE)",
            "  PINNED: LEFT/RIGHT ADJUST LIVE, ESC RELEASE",
            "K A/B EXPERIMENT    F5 RELOAD ASSETS",
            "",
            "? THIS HELP    ESC CLOSE",
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
        self.blasts.trigger(&mut self.world, &mut self.particles, x, y);
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
///
/// Public so headless tools can render and probe *this* terrain rather than
/// a hand-rolled approximation of it — `examples/filmstrip.rs` builds the
/// real thing to check that structural integrity leaves it standing, which
/// is a claim about the terrain players actually see, not about a replica.
pub fn build_terrain(world: &mut World) {
    build_terrain_only(world);
    // Terrain is structurally real from frame one, not exempt from checking
    // until something disturbs it -- see `compute_world_distances` for why
    // this is a direct converged pass rather than a scheduled one.
    structural::compute_world_distances(world);
}

/// Just the material placement, without the structural pass `build_terrain`
/// runs after it. Split out so `examples/ascii.rs` can time the two halves
/// separately and attribute the generation cost rather than just stating it.
pub fn build_terrain_only(world: &mut World) {
    let w = WIDTH as i32;
    let h = HEIGHT as i32;
    // Always present: `reload` only ever adds or updates, so the compiled-in
    // stone cannot be removed by editing the assets directory.
    let stone = world
        .materials
        .id_of("stone")
        .expect("stone is a compiled-in material");

    // The bottom two rows are literal bedrock, the world's structural
    // anchor (`structural.rs`) and the deepest of the six vertical zones
    // `Reports/worldgen-design.md` §2 defines. Nothing placed bedrock
    // anywhere before this, which meant the only anchor in the entire world
    // was the floor's bottom row happening to touch the out-of-bounds
    // sentinel -- true, but by accident rather than by construction, and
    // not something generated terrain could rely on.
    for x in 0..w {
        for y in (h - 2)..h {
            world.set(x, y, Cell::new(material::BEDROCK, 0).with_attached(true));
        }
    }
    for x in 0..w {
        for y in (h - 8)..(h - 2) {
            world.set(x, y, Cell::new(stone, (x % 4) as u8).with_attached(true));
        }
    }

    // 6 cells deep, which is deliberately more than stone's confinement
    // diameter (5): each ledge contains genuinely confined rock and so holds
    // itself up, with no support pillar and no exemption from checking. Thin
    // these below 5 and they will come down, which is the mechanic working
    // rather than a regression.
    let mut ledge = |x0: i32, x1: i32, y: i32| {
        for x in x0..x1 {
            for dy in 0..6 {
                world.set(x, y + dy, Cell::new(stone, (x % 4) as u8).with_attached(true));
            }
        }
    };
    ledge(0, 110, 200); // cut into the left wall
    ledge(402, w, 150); // cut into the right wall
    ledge(180, 320, 260);
    for y in 266..(h - 8) {
        for x in 244..256 {
            world.set(x, y, Cell::new(stone, (x % 4) as u8).with_attached(true)); // the middle platform's pillar
        }
    }

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
    fn generated_terrain_is_structurally_real_and_still_stands() {
        // `Reports/worldgen-design.md` §6b, "the structural-integrity
        // landmine": terrain used to be exempt from structural checking
        // entirely, keeping `aux = 0` -- "indistinguishable from
        // 'anchored'" -- so the whole world was structurally invalid while
        // reading as fine, and turning checks on would collapse it.
        // `build_terrain` now computes real distances at generation, so
        // this asserts both halves: the distances are genuinely computed
        // (not left at a default that merely looks anchored), and the
        // terrain nonetheless stands.
        let mut app = App::new();
        let stone = id(&app, "stone");
        let h = HEIGHT as i32;

        // The ledges are the interesting case. They reach bedrock only
        // through the wall they are cut into, and attachment buys them the
        // span to get there -- it does not anchor them outright, which is
        // what would make an undercut shelf unfallable.
        let ledge_probe = (60, 202);
        assert_eq!(app.world.get(ledge_probe.0, ledge_probe.1).material, stone, "test setup: expected a ledge here");
        assert!(
            app.world.get(ledge_probe.0, ledge_probe.1).aux() < u16::MAX,
            "the ledge never reached an anchor, so it is standing only because nothing has checked it yet"
        );
        // Attachment is what anchors it, and it has to be *stated* rather
        // than inferred -- a ledge that stands only because nothing has
        // checked it yet is section 6b's landmine, and reads identically
        // from the outside.
        assert!(
            app.world.get(ledge_probe.0, ledge_probe.1).attached(),
            "generated terrain must mark itself attached, or it is anchored only by never having been asked"
        );
        // Freshly brushed material is foreground and must NOT inherit it,
        // or everything the player builds becomes indestructible -- the
        // exact failure two inferred-support models both had.
        let mut probe = App::new();
        probe.world.paint_capsule((40, 40), (40, 40), 3, stone, 1.0);
        assert!(!probe.world.get(40, 40).attached(), "brushed stone must be foreground, not attached");

        for _ in 0..400 {
            app.update();
        }

        for y in (h - 8)..(h - 2) {
            assert_eq!(app.world.get(256, y).material, stone, "the stone floor crumbled at y={y}");
        }
        for y in (h - 2)..h {
            assert_eq!(app.world.get(256, y).material, material::BEDROCK, "the bedrock floor was disturbed at y={y}");
        }
        for &(x, y) in &[(60, 202), (460, 152), (250, 262)] {
            assert_eq!(app.world.get(x, y).material, stone, "a floating ledge crumbled at ({x}, {y})");
        }
        // Read from the registry rather than named, so retargeting stone's
        // `breaks_into` cannot quietly turn this into a check for a material
        // the engine no longer produces.
        let debris = app.world.materials.get(stone).breaks_into.expect("stone must define a breaks_into");
        let crumbled = (0..WIDTH as i32).any(|x| (0..h).any(|y| app.world.get(x, y).material == debris));
        assert!(!crumbled, "some terrain broke free despite nothing disturbing it");
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
        // Both layers of the floor, since it is no longer stone all the way
        // down -- the bottom two rows are literal bedrock, the anchor
        // `structural.rs` keys on.
        assert_eq!(
            app.world.get(10, HEIGHT as i32 - 1).material,
            material::BEDROCK,
            "reset lost the bedrock floor"
        );
        assert_eq!(
            app.world.get(10, HEIGHT as i32 - 3).material,
            stone,
            "reset lost the stone floor"
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
