//! Application state: everything the sandbox does that is not windowing.
//!
//! Kept free of winit and pixels types so it stays testable without a GPU or a
//! display, and so the windowing layer can be replaced later without touching
//! any behaviour.

use crate::render::Renderer;
use crate::sim::chunk::Rect;
use crate::sim::material::{self, MaterialId, MaterialKind};
use crate::sim::update;
use crate::sim::world::World;
use crate::sim::Cell;

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
    pub renderer: Renderer,
    pub brush_radius: i32,
    /// Index into `paintable`, not a `MaterialId`, so cycling wraps cleanly.
    selected: usize,
    paintable: Vec<MaterialId>,
    pub paused: bool,
    /// Set by the step key while paused; consumed by the next `update`.
    pub step_once: bool,
    /// Feedback from the last material reload — the only place a typo in a
    /// `.ron` file shows up.
    pub message: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let mut world = World::new(Rect::new(0, 0, WIDTH as i32 - 1, HEIGHT as i32 - 1));

        // Load over the compiled-in set, so edits made before launch apply and
        // a broken assets directory still leaves a working engine.
        let message = match world.materials.reload(material::ASSET_DIR) {
            Ok(n) => Some(format!("{n} materials")),
            Err(e) => Some(format!("materials: {e}")),
        };

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
            renderer: Renderer::new(),
            brush_radius: 6,
            selected,
            paintable,
            paused: false,
            step_once: false,
            message,
        }
    }

    pub fn update(&mut self) {
        if self.paused && !self.step_once {
            return;
        }
        self.step_once = false;
        update::step(&mut self.world);
        // Its own phase, after the CA sweep, per the `entities → CA sweep →
        // rigid bodies → render` ordering the plan settled on: the field
        // reacts to whatever solids the sweep just placed rather than a frame
        // stale. No coupling back into CA cells yet — that starts in M14.
        self.world.step_fields();
    }

    pub fn draw(&self, frame: &mut [u8]) {
        self.renderer.draw(&self.world, frame, WIDTH, HEIGHT);
    }

    /// Re-read the material files. Ids are keyed by name, so material already
    /// in the world keeps its identity and simply starts behaving differently.
    pub fn reload_materials(&mut self) {
        self.message = match self.world.materials.reload(material::ASSET_DIR) {
            Ok(n) => Some(format!("reloaded {n} materials")),
            Err(e) => Some(format!("materials: {e}")),
        };
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
            "Pixel Physics — {:.0} fps — {} (brush {}) — chunks {}/{} awake{}{}",
            fps,
            self.selected_name(),
            self.brush_radius,
            self.world.active_chunk_count(),
            self.world.chunk_count(),
            if self.paused { " — PAUSED" } else { "" },
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
