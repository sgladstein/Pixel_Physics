//! Application state: everything the sandbox does that is not windowing.
//!
//! Kept free of winit and pixels types so it stays testable without a GPU or a
//! display, and so the windowing layer can be replaced later without touching
//! any behaviour.

use crate::hud;
use crate::render::{self, Renderer};
use crate::sim::chunk::Rect;
use crate::sim::explosion;
use crate::sim::load;
use crate::sim::material::{self, MaterialId, MaterialKind};
use crate::sim::organism;
use crate::sim::parallel;
use crate::sim::particle::ParticleSystem;
use crate::sim::player;
use crate::sim::structural;
use crate::sim::world::World;
use crate::tunables::{self, Tunable, TunableGroup};
use crate::worldgen::{self, WorldgenPresets};

/// Viewport resolution, in cells — the framebuffer `pixels` scales up, which
/// is what gives the chunky pixel look. **Not the size of the world.**
///
/// These two were one pair of constants doing both jobs, which was fine only
/// while the world happened to be exactly one screen. The camera
/// (`Renderer::follow`) is what separates them.
/// One row of the help overlay: a section heading, a key and what it does,
/// or a full-width note. See `App::help_columns`.
enum HelpRow {
    Head(&'static str),
    Key(&'static str, &'static str),
    Note(&'static str),
    Blank,
}

pub const WIDTH: u32 = 512;
pub const HEIGHT: u32 = 320;

/// Size of the world itself, in cells.
///
/// Wider than the viewport so there is somewhere to walk to, and taller than
/// it so there is somewhere to dig to. Worldgen keeps its regions
/// window-sized as this grows (`worldgen::region`), so a wider world is more
/// places rather than the same places stretched.
///
/// **4x linear on round 7's 2048x640, which is sixteen times the cells.**
/// The owner's decision, and not a tuning one: round 6's renders were
/// rejected because *"everything needs to be bigger, the whole world, the
/// caves. You cannot create good looking crystals or stalagmites and
/// stalactites that are only 1-2 pixels wide."* A feature only has a
/// silhouette, a taper and an interior if it is many cells across, and there
/// is no room for a many-cells-across cave in a world four screens wide --
/// so the world had to grow before anything in it could.
///
/// The cost was measured before the size was taken, not after
/// (`Reports/field-settling-2026-08.md`, and `examples/scale_probe.rs` is
/// the instrument): generation 6516 ms behind a loading screen, peak RSS
/// 358 MiB, the field 16.7 ms amortised over a full day/night cycle. The
/// one target missed -- 4 ms amortised -- is recorded as a gap in
/// `Reports/world-scale-handoff.md` rather than relabelled away.
///
/// **Not the number to change to test something at a smaller size.** Every
/// probe that builds a world takes its own size (`scale_probe size=WxH`,
/// `viewshot`, the `tests/worldgen.rs` suite at 512x320); this is what the
/// app ships.
pub const WORLD_WIDTH: u32 = 8192;
pub const WORLD_HEIGHT: u32 = 2560;

/// The pair above, for the constructors that take a size.
const SHIPPED_SIZE: (u32, u32) = (WORLD_WIDTH, WORLD_HEIGHT);

/// Fraction of the brush filled per application for loose material. Low enough
/// that a moving brush lays down separated grains, high enough that a stationary
/// one fills in within a handful of frames.
const STREAM_DENSITY: f32 = 0.3;

/// How hard a strike throws its debris, per cell of brush radius. Tuned so a
/// small brush chips and a large one hurls -- the brush size is already the
/// player's sense of "how big a hit is this", so it drives the force rather
/// than a second hidden number.
const STRIKE_FORCE_PER_RADIUS: f32 = 0.9;

/// The seed a fresh session starts on, so that launching the sandbox twice
/// gives the same world and a bug report has something to name.
const INITIAL_SEED: u64 = 0x5EED;

/// Build `world` from a named preset, falling back to the hand-authored
/// terrain.
///
/// The fallback covers both `worldgen::LEGACY` (which is deliberately not in
/// the presets file) and a name that no longer resolves because the file was
/// edited while the app was running — neither is worth failing over when
/// there is a working world one branch away.
fn build_world_with(world: &mut World, presets: &WorldgenPresets, preset: &str, seed: u64) {
    build_world_reporting(world, presets, preset, seed, &mut |_, _| {});
}

/// [`build_world_with`], announcing each generation stage as it starts.
fn build_world_reporting(
    world: &mut World,
    presets: &WorldgenPresets,
    preset: &str,
    seed: u64,
    progress: worldgen::Progress,
) {
    match presets.get(preset) {
        Some(params) => {
            worldgen::generate_reporting(world, worldgen::Spec::Generated { params, seed }, progress)
        }
        None => worldgen::generate_reporting(world, worldgen::Spec::Legacy, progress),
    }
}

/// The reference room `B` stamps, in cells. See `stamp_reference_room`.
///
/// **Set to the measured edge of what the structural model will hold, not
/// to a round number that looks nice.** A 200-cell span with 5- to
/// 17-cell walls stands untouched; 260 fails at every thickness tested
/// (`Reports/next-session-handoff.md` §2b). Stamping the *limit* is the
/// point — a room comfortably inside it would show that rooms work, which
/// nobody doubts, rather than whether the ceiling is in a sensible place.
///
/// 200 wide against the 512-wide viewport (`WIDTH`) is 39% of it, which is
/// the number the eye is actually being asked to judge —
/// `stamp_reference_room` places it at the cursor and it is judged by
/// looking at it on screen, not against the far larger world it is
/// stamped into.
const REFERENCE_ROOM_SPAN: i32 = 200;

/// Tall enough to stand a structure up rather than draw a lintel: the roof
/// has to be carried by walls doing real work, or the span is not being
/// tested. 160 is half the viewport's height (`HEIGHT`, 320) — the same
/// "judged on screen" reasoning as `REFERENCE_ROOM_SPAN` above, not a claim
/// about the world's own (now much taller) height.
const REFERENCE_ROOM_HEIGHT: i32 = 160;

pub struct App {
    pub world: World,
    pub particles: ParticleSystem,
    /// Explosions currently expanding, plus their live tuning. A blast is
    /// no longer a single-frame event (`sim::explosion`'s own module doc has
    /// the measurements), so it needs somewhere to live between frames.
    pub blasts: explosion::Blasts,
    /// M9 character feel, alongside `blasts.tuning` for the same reason:
    /// engine-struct tunables live on `App`, and the sim step receives
    /// them by reference. The character itself lives on `World::player`.
    pub player_tuning: player::Tuning,
    /// This tick's held/pressed movement intent, written by `main.rs`
    /// once per frame and consumed by `update` — the edge-triggered
    /// `jump_pressed` is cleared after the first simulated tick so a
    /// catch-up burst cannot fire one press several times.
    pub player_input: player::PlayerInput,
    /// Which `player::MOVEMENT_FEELS` / `WATER_FEELS` entry is live.
    /// Indices rather than copies of the numbers, so the status line can
    /// name the active one — a mode you cannot see is the failure this
    /// codebase keeps relearning, and a *feel* selector you cannot see is
    /// worse, since the whole point is reporting back which one won.
    pub movement_feel: usize,
    pub water_feel: usize,
    pub spoil_mode: usize,
    /// Index into `structural::CHAIN_MODES`; 0 is the shipped behaviour.
    pub chain_mode: usize,
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
    /// Files under `assets/` that differ from the committed tree, or `None`
    /// where git cannot answer (not installed, not a repository). Shown in
    /// the title bar so a value saved through the tunables panel cannot sit
    /// invisibly uncommitted across sessions — refreshed only on the events
    /// that can change it (startup, a tunables save, a reload), never per
    /// frame.
    pub assets_dirty: Option<usize>,
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
    /// Which gesture the mouse lays material with. `Z` cycles it.
    ///
    /// Reported from play: *"using a paint brush type tool to build is not
    /// satisfying."* A freehand round brush is a **drawing** tool -- right
    /// for sand, water and terrain, wrong for building, and for reasons
    /// that have nothing to do with the simulation: no straight lines, no
    /// right angles, no repeatable dimensions, and no sense of *placing*
    /// anything. Every structure comes out wobbly. Prior art is unanimous
    /// that building games use placement rather than painting
    /// (`Reports/prior-art-destruction.md`).
    ///
    /// A mode rather than a held modifier, deliberately. A modifier is
    /// invisible, and this project has already learned that an invisible
    /// rule is the failure mode of the whole genre; a mode can be named in
    /// the brush label and announced when it changes.
    pub tool: Tool,
    /// Where a drag-out gesture started, as a **world** cell, while the
    /// button is still held. `None` for the freehand brush, which paints as
    /// it goes rather than on release.
    ///
    /// This said "in screen space" long after `begin_drag` had been changed
    /// to store a world cell, and the stale line is very probably why
    /// `draw_hud` drew the preview rectangle from a world coordinate against
    /// a screen one. World space is the deliberate choice, for the reason
    /// `begin_drag` gives: a drag held across a camera move would otherwise
    /// re-anchor to a different cell than the one under the cursor when the
    /// button went down.
    pub drag_from: Option<(i32, i32)>,
    /// `N` — the stress overlay. Off by default: it repaints the world
    /// every frame it is up, which is the cost the dirty-rect skip exists
    /// to avoid, and that is only worth paying while someone is actually
    /// asking the question.
    pub show_stress: bool,
    /// A short-lived on-screen line, and the frame it stops being drawn on.
    ///
    /// `message` already existed but is only ever *read* from the window
    /// title bar and the tunables panel's own footer — neither of which is
    /// where anyone is looking mid-stroke. Reported from play about `B`
    /// specifically: the brush changes what it authors and nothing on the
    /// world itself says so, so which mode you are in has to be inferred
    /// from what happens after you have already painted. A mode change is
    /// exactly the case that wants a transient: it is worth saying loudly
    /// once and worth nothing a second later, unlike the persistent brush
    /// label beneath it.
    toast: Option<(String, u64)>,
    /// Every worldgen preset, reloaded whenever `assets/worldgen.ron` changes.
    pub worldgen: WorldgenPresets,
    /// Which preset a new world is built from. May be `worldgen::LEGACY`,
    /// which selects the hand-authored terrain instead of generating.
    pub worldgen_preset: String,
    /// The seed the current world was generated from. `F6` rolls a new one.
    ///
    /// On screen in the status line at all times, so that a world worth
    /// keeping can be written down and a screenshot always says which world
    /// it is. A generator whose output cannot be named is one whose bugs
    /// cannot be reported.
    pub worldgen_seed: u64,
    /// Seeds already visited, so `F8` can walk back to one that looked good.
    /// Rolling forward past a promising world and having no way back is the
    /// obvious failure of a single-key reroll.
    seed_history: Vec<u64>,
}

/// Which gesture the mouse lays material with. See `App::tool`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    /// Freehand, painting as the cursor moves. The original behaviour, and
    /// still the right one for sand, water and terrain.
    Brush,
    /// Drag out a filled rectangle. Walls, floors, columns.
    Rect,
    /// Drag out a **hollow** rectangle: four walls of the brush's width
    /// around an empty middle. The tool for anything you can go inside.
    ///
    /// Worth its own mode rather than being a modifier on `Rect`, because
    /// it is the one a player reaches for most once they are building
    /// rather than sculpting -- and because it gives `brush_radius` a
    /// second, better meaning here: wall thickness.
    Room,
    /// Drag out a straight run of the current brush width. Beams,
    /// diagonals, bracing.
    Line,
    /// The gnome's dig (M9 phase 2). Freehand like `Brush`, but the left
    /// button cuts rock at his arm's length toward the cursor instead of
    /// laying material down.
    ///
    /// **A mode, after proximity gating failed in play.** The first
    /// version had no tool of its own: while a gnome existed, a left
    /// click *within `dig_reach` of him* dug and anything further painted.
    /// Reported from the first playtest as "I cannot dig. The mouse
    /// either makes sand/material or erases it" — and correctly, because
    /// at zoom 1 that reach is a fourteen-*pixel* bullseye around what was,
    /// at the time, a 3x6 pixel character (he has since grown twice more,
    /// to 5x10 and now 7x14 — see `player::PLAYER_WIDTH`/`PLAYER_HEIGHT`),
    /// with no indication it is there and the ordinary brush as the
    /// failure mode. Nothing on screen said a second verb existed, so the
    /// verb effectively did not.
    ///
    /// The general lesson is one `CLAUDE.md` already states about size
    /// caps: a reach may bound *where* something happens and must never
    /// decide *whether* it happens. Reach now only clamps the bite along
    /// the aim ray; the tool decides the verb, is named in the persistent
    /// HUD label like every other tool, and is switched to automatically
    /// when a gnome is summoned.
    Dig,
}

/// Frames a `toast` stays up — two seconds at the sandbox's fixed 60 Hz.
/// Long enough to read a short line without looking away from the cursor,
/// short enough that it is gone before the next stroke finishes.
const TOAST_FRAMES: u64 = 120;

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

/// How many files under `assets/` differ from the committed tree, per
/// `git status --porcelain`, or `None` when git cannot answer (not
/// installed, not a repository, subprocess failure — all silently fine,
/// this is an affordance, not a dependency).
///
/// Exists because the tunables panel saves straight back into the `.ron`
/// files — right for iteration, and it quietly accumulates unrecorded
/// balance changes: a `smoke_fraction` saved mid-playtest sat invisible in
/// the working tree for a whole session before a review caught it, having
/// silently removed SMOKE's only producer. One subprocess per call, so
/// callers refresh it on the events that can change it, never per frame.
fn dirty_asset_count() -> Option<usize> {
    let out = std::process::Command::new("git").args(["status", "--porcelain", "--", "assets"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).lines().filter(|l| !l.trim().is_empty()).count())
}

impl App {
    /// A fully built world, generated on this thread.
    pub fn new() -> Self {
        Self::build(true, SHIPPED_SIZE, &mut |_, _| {})
    }

    /// A fully built world, announcing each generation stage as it starts —
    /// what the loading screen in `main.rs` runs on a worker thread.
    pub fn new_reporting(progress: worldgen::Progress) -> Self {
        Self::build(true, SHIPPED_SIZE, progress)
    }

    /// Everything except the world, which is left empty.
    ///
    /// The placeholder the window has something to hold while the real one is
    /// generated elsewhere. Cheap — the expensive half of startup is
    /// generation and the structural distance field, and this does neither.
    /// Deliberately a whole `App` rather than an `Option<App>` in the
    /// handler: the frame loop touches `self.app` in 85 places, and making
    /// every one of them fallible to express a state that lasts a few
    /// seconds at startup would be the tail wagging the dog. The real one
    /// replaces this wholesale when the worker finishes.
    pub fn new_pending() -> Self {
        Self::build(false, SHIPPED_SIZE, &mut |_, _| {})
    }

    /// [`App::new`] at a stated world size rather than the shipped one.
    ///
    /// **Exists for the test suite, and the reason is a number.** Generating
    /// the shipped 8192x2560 costs seconds and a third of a gigabyte of peak
    /// RSS, and thirty-odd tests in this file want *an app with a world in
    /// it* rather than the shipped world specifically -- the cheque came due
    /// the moment the world grew 16x in area, with `cargo test --lib` going
    /// from about a minute to over ten and several 358 MiB worlds coexisting
    /// across cargo's test threads.
    ///
    /// Not a way to make a test pass that would fail at the shipped size:
    /// the tests here already ask the world where its own edges are
    /// (`world_bottom`, `world_cells`), which is what makes them honest at
    /// any size, and `a_shipped_size_world_is_generated_and_at_rest` still
    /// builds the real thing so the size that ships is exercised somewhere.
    pub fn with_world_size(w: u32, h: u32) -> Self {
        Self::build(true, (w, h), &mut |_, _| {})
    }

    fn build(generate: bool, (w, h): (u32, u32), progress: worldgen::Progress) -> Self {
        let mut world = World::new(Rect::new(0, 0, w as i32 - 1, h as i32 - 1));

        // Load over the compiled-in set, so edits made before launch apply and
        // a broken assets directory still leaves a working engine. Species
        // reload alongside materials -- `Reports/organism-substrate-
        // design.md`'s own stated design ("hot-reloaded via the same notify
        // pattern MaterialRegistry already uses"), which an independent
        // review of the first version of this section caught as never
        // actually wired up: `SpeciesRegistry::reload` existed and was
        // tested, but nothing called it, so editing `assets/species/*.ron`
        // silently did nothing, unlike every material file.
        let mut message = reload_assets(&mut world);

        let (worldgen_presets, worldgen_error) = WorldgenPresets::load();
        // A broken `worldgen.ron` falls back to compiled-in defaults, which
        // still builds a world -- so without this the only symptom of a typo
        // would be that a tuning session silently stopped having any effect.
        if let Some(e) = worldgen_error {
            message = Some(match message {
                Some(m) => format!("{m}; {e}"),
                None => e,
            });
        }
        let worldgen_preset = worldgen_presets.default_name();
        let worldgen_seed = INITIAL_SEED;
        if generate {
            build_world_reporting(&mut world, &worldgen_presets, &worldgen_preset, worldgen_seed, progress);
        }

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
            player_tuning: player::Tuning::load(),
            player_input: player::PlayerInput::default(),
            movement_feel: 0,
            water_feel: 0,
            spoil_mode: 0,
            chain_mode: 0,
            renderer: Renderer::new(),
            brush_radius: 6,
            selected,
            paintable,
            paused: false,
            step_once: false,
            message,
            assets_dirty: dirty_asset_count(),
            show_hover_inspector: false,
            show_palette: false,
            show_help: false,
            show_tunables: false,
            tunables_selected: 0,
            tunables_group: TunableGroup::Physics,
            pinned: None,
            experiment: false,
            tool: Tool::Brush,
            drag_from: None,
            show_stress: false,
            toast: None,
            worldgen: worldgen_presets,
            worldgen_preset,
            worldgen_seed,
            seed_history: Vec::new(),
        }
    }

    /// `F6` — generate a fresh world from a new seed.
    ///
    /// The reroll key, and the reason the generator is judged by eye rather
    /// than argued about: a parameter change is worth nothing until it has
    /// been seen across a dozen seeds, and any workflow where that costs a
    /// recompile means it does not happen.
    pub fn next_seed(&mut self) {
        self.seed_history.push(self.worldgen_seed);
        // Advancing the seed through the same hash the generator uses, rather
        // than incrementing: adjacent integer seeds are perfectly fine inputs
        // to a hashed generator, but consecutive worlds looking related --
        // even once, even by coincidence -- would be read as the generator
        // being broken.
        self.worldgen_seed = worldgen::noise::hash(self.worldgen_seed, worldgen::noise::Purpose::Height, 0, 0);
        self.reset();
        self.announce_world();
    }

    /// `F8` — back to the previous seed.
    pub fn previous_seed(&mut self) {
        match self.seed_history.pop() {
            Some(seed) => {
                self.worldgen_seed = seed;
                self.reset();
                self.announce_world();
            }
            None => self.show_toast("NO PREVIOUS SEED"),
        }
    }

    /// `F7` — cycle worldgen preset, keeping the seed.
    ///
    /// Same seed on purpose: comparing two presets is only meaningful on the
    /// same underlying world, and a preset switch that also rerolled would
    /// confound every comparison it was reached for.
    pub fn cycle_preset(&mut self) {
        let order = self.worldgen.cycle_order();
        let at = order.iter().position(|n| *n == self.worldgen_preset).unwrap_or(0);
        self.worldgen_preset = order[(at + 1) % order.len()].clone();
        self.reset();
        self.announce_world();
    }

    /// Re-read `assets/worldgen.ron` and rebuild on the current seed.
    ///
    /// The tuning loop: edit a number, save, look. Reached from the file
    /// watcher, so no keypress is needed at all.
    pub fn reload_worldgen(&mut self) {
        let (presets, error) = WorldgenPresets::load();
        self.worldgen = presets;
        // A preset can be deleted out from under the current selection.
        if self.worldgen_preset != worldgen::LEGACY && self.worldgen.get(&self.worldgen_preset).is_none() {
            self.worldgen_preset = self.worldgen.default_name();
        }
        self.reset();
        self.assets_dirty = dirty_asset_count();
        match error {
            Some(e) => self.show_toast(e),
            None => self.announce_world(),
        }
    }

    /// Put the current seed and preset on screen.
    fn announce_world(&mut self) {
        let text = format!("{} — SEED {:#018X}", self.worldgen_preset.to_uppercase(), self.worldgen_seed);
        self.show_toast(text);
    }

    /// `F3` — cycle the gnome's movement feel. See `player::MovementFeel`
    /// for why this is a runtime selector rather than a decided number.
    ///
    /// Overwrites whatever the PLAYER tunables currently hold for the
    /// fields it owns, and says so: the two are the coarse and fine
    /// halves of the same job, and silently keeping a hand-swept value
    /// while announcing a preset name would make the label a lie.
    /// Cycle whether the gnome weaves through a stand of trees or draws
    /// over it. `,`.
    pub fn cycle_tree_depth(&mut self) {
        let mode = self.renderer.cycle_tree_depth();
        self.show_toast(format!("TREE DEPTH: {}", mode.label()));
    }

    pub fn cycle_movement_feel(&mut self) {
        self.movement_feel = (self.movement_feel + 1) % player::MOVEMENT_FEELS.len();
        let feel = &player::MOVEMENT_FEELS[self.movement_feel];
        feel.apply(&mut self.player_tuning);
        let line = format!(
            "JUMP FEEL: {} - {} (rise ~{:.0} CELLS)",
            feel.name,
            feel.note.to_uppercase(),
            feel.jump_cells()
        );
        self.show_toast(&line);
    }

    /// `F4` — cycle how water handles him. See `player::WaterFeel`.
    pub fn cycle_water_feel(&mut self) {
        self.water_feel = (self.water_feel + 1) % player::WATER_FEELS.len();
        let feel = &player::WATER_FEELS[self.water_feel];
        feel.apply(&mut self.player_tuning);
        let line = format!("WATER FEEL: {} - {}", feel.name, feel.note.to_uppercase());
        self.show_toast(&line);
    }

    /// `F2` — cycle what happens to mined rock. See `player::SpoilMode`
    /// for why this is a selector: the owner wants both "just remove it"
    /// and "make collecting it a mechanic", and those pull opposite ways.
    pub fn cycle_spoil_mode(&mut self) {
        self.spoil_mode = (self.spoil_mode + 1) % player::SPOIL_MODES.len();
        let mode = &player::SPOIL_MODES[self.spoil_mode];
        self.player_tuning.dig_yield = mode.dig_yield;
        let line = format!("SPOIL: {} - {}", mode.name, mode.note.to_uppercase());
        self.show_toast(&line);
    }

    /// `F9` — cycle how far damage is allowed to travel from what was
    /// actually hit. See `structural::ChainMode` for why this is a
    /// selector: the owner wants both "they chain too far and too much"
    /// and "collapse must be obvious and delayed", and those pull opposite
    /// ways.
    pub fn cycle_chain_mode(&mut self) {
        self.chain_mode = (self.chain_mode + 1) % crate::sim::structural::CHAIN_MODES.len();
        let mode = &crate::sim::structural::CHAIN_MODES[self.chain_mode];
        let previous = self.world.chain_reach;
        self.world.chain_reach = mode.reach;
        // **Tightening the setting also drops staged work it no longer
        // licenses.** The staged queue is ungated once a failure has been
        // judged and that is deliberate (`structural::advance_staged_
        // fractures`), so without this line the aftermath the player is
        // trying to stop keeps arriving at `FRACTURE_CELLS_PER_TICK` a tick
        // from a queue the new setting never sees -- and `F9` reads as
        // having done nothing, which is exactly the reported complaint.
        // Only on a tighten, so relaxing the setting can never resurrect
        // work that was already dropped.
        if mode.reach < previous {
            self.world.relicense_staged_fractures();
        }
        let line = format!("CHAINING: {} - {}", mode.name, mode.note.to_uppercase());
        self.show_toast(&line);
    }

    /// `Z` — cycle the build gesture. See `App::tool`.
    pub fn cycle_tool(&mut self) {
        self.tool = match self.tool {
            Tool::Brush => Tool::Rect,
            Tool::Rect => Tool::Room,
            Tool::Room => Tool::Line,
            // Dig sits in the cycle only while there is a gnome to do it,
            // rather than being a mode you can select and have do nothing.
            Tool::Line if self.world.player.is_some() => Tool::Dig,
            Tool::Line | Tool::Dig => Tool::Brush,
        };
        self.drag_from = None;
        self.show_toast(match self.tool {
            Tool::Brush => "TOOL: BRUSH - FREEHAND",
            Tool::Rect => "TOOL: RECTANGLE - DRAG OUT A SOLID BLOCK",
            Tool::Room => "TOOL: ROOM - HOLLOW, BRUSH SETS WALL THICKNESS",
            Tool::Line => "TOOL: LINE - DRAG OUT A BEAM",
            Tool::Dig => "TOOL: DIG - LMB CUTS AT THE GNOME'S ARM'S LENGTH",
        });
    }

    /// Begin a drag-out gesture, for the tools that commit on release.
    pub fn begin_drag(&mut self, screen_x: i32, screen_y: i32) {
        if self.tool != Tool::Brush {
            // Stored in **world** coordinates. Screen coordinates were fine
            // while the view could not move; with a camera following the
            // gnome, a drag begun before he walks would silently re-anchor to
            // a different world cell than the one under the cursor when the
            // button went down.
            self.drag_from = Some(self.renderer.screen_to_world(screen_x, screen_y));
        }
    }

    /// Finish a drag-out gesture, laying the shape down. A no-op for the
    /// freehand brush, which has already painted everything it is going to.
    pub fn end_drag(&mut self, screen_x: i32, screen_y: i32, erase: bool) {
        let Some(from) = self.drag_from.take() else { return };
        let m = if erase { material::EMPTY } else { self.selected_material() };
        let density = self.emission_density(m, erase);
        let a = from; // already world space -- see `begin_drag`
        let b = self.renderer.screen_to_world(screen_x, screen_y);
        match self.tool {
            // Filled by sweeping capsules row by row rather than by a
            // dedicated rectangle primitive: `paint_capsule_as` already
            // carries every rule that matters -- density, the
            // do-not-overwrite-solid guard, structural scheduling, the
            // background-must-join-background test and the converged
            // relaxation at the end -- and a second path into the grid
            // would have to reimplement all of it to stay consistent.
            Tool::Rect => {
                let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
                let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
                let step = (self.brush_radius).max(1);
                let mut y = y0;
                loop {
                    self.world.paint_capsule_as((x0, y), (x1, y), self.brush_radius, m, density);
                    if y >= y1 {
                        break;
                    }
                    y = (y + step).min(y1);
                }
            }
            // Four walls rather than a fill. Each is a capsule run, so the
            // corners are covered twice and join properly -- a room drawn
            // as four independent segments would leak at every corner,
            // which for a structure means the roof is not actually carried
            // by the walls.
            Tool::Room => self.paint_room(a, b, self.brush_radius, m, density),
            Tool::Line => {
                self.world.paint_capsule_as(a, b, self.brush_radius, m, density);
            }
            // Freehand tools commit as the cursor moves, not on release.
            Tool::Brush | Tool::Dig => {}
        }
    }

    /// Four walls around an empty middle, with `a` and `b` as opposite
    /// corners.
    ///
    /// Each wall is a capsule *run*, so the corners are covered twice and
    /// join properly — a room drawn as four independent segments leaks at
    /// every corner, which for a structure means the roof is not actually
    /// carried by the walls.
    ///
    /// Shared by the drag-out room tool and `stamp_reference_room` on
    /// purpose. A reference room that was not built by the same code as a
    /// player's room would be measuring a replica, which is the mistake
    /// `build_terrain` is public to avoid.
    fn paint_room(&mut self, a: (i32, i32), b: (i32, i32), r: i32, m: material::MaterialId, density: f32) {
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        for (from, to) in [((x0, y0), (x1, y0)), ((x0, y1), (x1, y1)), ((x0, y0), (x0, y1)), ((x1, y0), (x1, y1))] {
            self.world.paint_capsule_as(from, to, r, m, density);
        }
    }

    /// `B` — drop a **reference room** of a known size at the cursor, sitting
    /// on whatever ground is under it.
    ///
    /// This exists to answer a question that only the eye can answer and
    /// that no contact sheet has managed to: *is a room this size a
    /// reasonable thing to want to build?* The structural model's measured
    /// envelope is that a room holds its own roof up to about
    /// `REFERENCE_ROOM_SPAN` cells wide and fails above it
    /// (`Reports/next-session-handoff.md` §2b), and whether that is a
    /// generous allowance or a cramped one is a judgement about play, not
    /// about statics. Dragging one out by hand gives a different size every
    /// time and so cannot settle it.
    ///
    /// Wall thickness is the brush radius, deliberately, so the same key at
    /// different brush sizes walks the other axis of that envelope.
    ///
    /// **Always stone, ignoring the palette.** The app starts on sand, so
    /// keying to the selection would hand a first-time press a room made of
    /// powder — which answers no question about structure and looks like the
    /// feature is broken. A reference is a fixed known thing or it is not a
    /// reference; the ordinary room tool on `Z` is there for building out of
    /// whatever you like.
    pub fn stamp_reference_room(&mut self, screen_x: i32, screen_y: i32) {
        let Some(stone) = self.world.materials.id_of("stone") else { return };
        let (cx, cy) = self.renderer.screen_to_world(screen_x, screen_y);
        // Sit it on the ground rather than at the cursor's height: a room
        // floating in mid-air is a different structural question (and a
        // much easier one) than a room standing on something.
        // Bounded by the *world*, not the viewport: with a camera, the
        // bottom of the screen is not the bottom of the world, and this
        // walk would stop in mid-air wherever the view happened to end.
        // An unbounded world has no floor to find and no sides to refuse
        // against, so the room is always placeable there.
        let world = self.world.bounds().unwrap_or(Rect::new(i32::MIN / 2, i32::MIN / 2, i32::MAX / 2, i32::MAX / 2));
        let mut floor = cy;
        while floor < world.max_y && self.world.get(cx, floor + 1).material == material::EMPTY {
            floor += 1;
        }
        let half = REFERENCE_ROOM_SPAN / 2;
        let top = floor - REFERENCE_ROOM_HEIGHT;
        // **Refuse rather than build a smaller one.** Generated terrain
        // (M10) puts the surface anywhere, and on a hilltop there is not
        // 160 cells of headroom -- a room clipped by the top of the world
        // is not the reference size, and silently stamping one would be a
        // measuring stick that changes length depending where you stand.
        // Say so instead: the whole value of this key is that its answer
        // means the same thing every time.
        let margin = self.brush_radius;
        if top - margin < world.min_y || cx - half - margin < world.min_x || cx + half + margin > world.max_x {
            self.show_toast(format!(
                "NO ROOM FOR A {}x{} REFERENCE HERE - NEEDS CLEAR SKY AND SIDES",
                REFERENCE_ROOM_SPAN, REFERENCE_ROOM_HEIGHT
            ));
            return;
        }
        self.paint_room((cx - half, top), (cx + half, floor), self.brush_radius, stone, 1.0);
        self.show_toast(format!(
            "REFERENCE ROOM {}x{} STONE - WALLS {} THICK",
            REFERENCE_ROOM_SPAN,
            REFERENCE_ROOM_HEIGHT,
            self.brush_radius * 2 + 1
        ));
    }

    /// `N` — show or hide the structural stress overlay.
    ///
    /// The key is `N` because that is what Medieval Engineers binds its
    /// own stress view to, and `Reports/prior-art-destruction.md` is
    /// emphatic that legibility, not physics, is what every disliked
    /// stability system in this genre actually got wrong. There is no
    /// reason to make someone learn a different key for the same idea.
    pub fn toggle_stress_view(&mut self) {
        self.show_stress = !self.show_stress;
        self.show_toast(if self.show_stress { "STRESS VIEW ON - RED IS AT ITS LIMIT" } else { "STRESS VIEW OFF" });
    }

    /// Put `text` on screen for `TOAST_FRAMES`. See `App::toast`.
    /// `pub` because the binary is a separate crate from the library, and
    /// `main.rs`'s loading screen reports how long generation took through
    /// the same channel every other transient notice uses.
    pub fn show_toast(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), self.world.frame + TOAST_FRAMES));
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
        out.extend(tunables::from_player(&self.player_tuning));
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
        if t.group == TunableGroup::Player {
            tunables::apply_player(&mut self.player_tuning, &t.name, new_value);
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
            "dissipation" => m.dissipation = new_value,
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
            self.assets_dirty = dirty_asset_count();
            return;
        }
        if t.group == TunableGroup::Player {
            self.message = Some(match self.player_tuning.save() {
                Ok(()) => format!("saved {}", player::Tuning::ASSET_PATH),
                Err(e) => format!("{}: {e}", player::Tuning::ASSET_PATH),
            });
            self.assets_dirty = dirty_asset_count();
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
        self.assets_dirty = dirty_asset_count();
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
        // M9: the character in the same serial slot as the bodies, right
        // after them — so standing on a body that settled this frame sees
        // its cells already in the grid, not a frame-old gap. The
        // edge-triggered press is consumed here so that when `main.rs`'s
        // catch-up loop runs several ticks in one frame, one press means
        // one jump.
        player::step(&mut self.world, self.player_input, &self.player_tuning);
        self.player_input.jump_pressed = false;
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
        // Splashes between the two, for the same reason blasts come before
        // particles: the sweep reported these sites against this frame's
        // state, so they should be taken and thrown before the step that
        // moves everything, not left a frame stale. See
        // `particle::throw_splashes` -- this is the only place a splash
        // droplet's water is actually debited from the pool.
        crate::sim::particle::throw_splashes(&mut self.world, &mut self.particles);
        self.particles.step(&mut self.world);
        self.world.step_fields();
        // Beside the field step, and for the same reason: a coarse
        // environmental channel with its own cadence, decoupled from the CA
        // sweep. `step_pheromones` gates itself on `PHEROMONE_INTERVAL`, so
        // this is called every frame like its neighbour above.
        self.world.step_pheromones();
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
        let force_full = cursor.is_some()
            || self.show_palette
            || self.show_help
            || self.show_tunables
            || self.show_hover_inspector
            || self.pinned.is_some()
            // A toast paints over terrain and has no tracked footprint, so
            // the frame it expires on has to redraw or it stays burned in
            // over a settled world -- the exact reason every other overlay
            // is in this list.
            || self.active_toast().is_some()
            || self.show_stress
            || self.drag_from.is_some();
        // The view follows the gnome. Here rather than in `update` on purpose:
        // the camera is view state, so it belongs on the rendered frame rather
        // than inside the fixed-timestep catch-up loop, where it would run
        // several times per frame and could not affect determinism either way.
        if let Some(player) = &self.world.player {
            let target = player.center();
            let bounds = self.world.bounds();
            self.renderer.follow(target, (WIDTH, HEIGHT), bounds);
        }
        let touched = self.world.take_touched_chunks();
        self.renderer.draw(&self.world, &self.particles, &touched, frame, (WIDTH, HEIGHT), force_full);
        self.draw_hud(frame, cursor);
    }

    /// The toast text, if one is set and has not yet expired. Expiry is
    /// checked at draw time against `world.frame` rather than cleared by a
    /// per-frame tick, so this needs no update-phase wiring at all. It does
    /// mean a toast raised while *paused* stays up until the simulation
    /// runs again, which is the behaviour worth having: the point is to say
    /// which mode the brush is in, and paused is exactly when someone is
    /// setting up rather than watching a clock.
    fn active_toast(&self) -> Option<&str> {
        self.toast.as_ref().filter(|(_, until)| self.world.frame < *until).map(|(text, _)| text.as_str())
    }

    fn draw_hud(&self, frame: &mut [u8], cursor: Option<(i32, i32)>) {
        const WHITE: [u8; 4] = [255, 255, 255, 255];
        const YELLOW: [u8; 4] = [255, 240, 120, 255];
        /// The shake ring. Green rather than a second yellow, because the
        /// point is that two verbs live on one button and you can tell
        /// which you are about to use before you press it.
        const GREEN: [u8; 4] = [130, 240, 140, 255];

        // Brush label -- always on, bottom-left, the data `status()` above
        // already computes just shown persistently instead of only in the
        // window title bar.
        // Under the HUD text but over the terrain, so the readouts stay
        // legible against it.
        if self.show_stress {
            self.draw_stress_overlay(frame);
        }
        // The tool is named in the persistent label, not only announced
        // when it changes: a mode you cannot see is the failure this whole
        // subsystem keeps relearning.
        let label = match self.tool {
            Tool::Brush => format!("{} R{}", self.selected_name(), self.brush_radius),
            Tool::Rect => format!("{} R{} - RECT", self.selected_name(), self.brush_radius),
            Tool::Room => format!("{} R{} - ROOM", self.selected_name(), self.brush_radius),
            Tool::Line => format!("{} R{} - LINE", self.selected_name(), self.brush_radius),
            Tool::Dig => "GNOME DIG - LMB CUT, RMB ERASE, Z FOR THE BRUSH".to_string(),
        };
        // With no gnome, `WASD` scrolls (`pan_camera`) and the view is a
        // thing you move — so say where it is. Two jobs in one readout:
        // the cue that the view moves at all, and the *number*, which is
        // what makes a sighting nameable ("at VIEW 1600,400 there is a
        // seam") instead of "somewhere off to the right".
        //
        // Safe to draw as changing text for free, which is not obvious:
        // HUD text has no tracked footprint, so a readout that changes
        // needs the terrain under it repainted or the old digits stay
        // burned in over a settled world. The only thing that changes this
        // one is a pan, and a pan moves the camera, which forces the full
        // redraw that erases it (`Renderer::draw`'s `camera_moved`). It
        // costs nothing on a settled world for the same reason: nothing
        // moves it, so it never needs erasing.
        let label = match &self.world.player {
            Some(_) => label,
            None => format!("{label}   VIEW {},{}", self.renderer.camera_x, self.renderer.camera_y),
        };
        hud::draw_text(frame, WIDTH, HEIGHT, 4, HEIGHT as i32 - 10, &label, WHITE);
        // Directly above the persistent brush label, so a mode change reads
        // as a line about the brush rather than as an unrelated notice
        // somewhere else on screen.
        if let Some(toast) = self.active_toast() {
            hud::draw_text(frame, WIDTH, HEIGHT, 4, HEIGHT as i32 - 20, toast, YELLOW);
        }
        // The help panel existed from the start and was invisible unless you
        // already knew the key -- which is the same as not existing. Hidden
        // only while help itself is open, where it would be redundant.
        if !self.show_help {
            hud::draw_text(frame, WIDTH, HEIGHT, WIDTH as i32 - 56, HEIGHT as i32 - 10, "? HELP", WHITE);
        }

        // The shape being dragged out, so the player can see what they are
        // about to commit before they release.
        // `drag_from` is a **world** cell (see `begin_drag`) and `cursor` is a
        // screen pixel, so the anchor has to come back through the camera
        // before the two can be compared. Wrong ever since the camera existed,
        // and invisible until now: with no gnome the camera sat at the origin
        // at zoom 1, where the two spaces are the same numbers and this drew
        // in the right place for the wrong reason. Scrolling with no gnome is
        // what makes it reachable, which is why it surfaced here. `None` at
        // `zoom_out_stride > 1` means the anchor falls between sampled columns
        // and simply has no pixel this frame.
        let anchor = self.drag_from.and_then(|w| self.renderer.world_to_screen(w.0, w.1));
        if let (Some(from), Some(to)) = (anchor, cursor) {
            match self.tool {
                Tool::Rect | Tool::Room => {
                    let (x0, x1) = (from.0.min(to.0), from.0.max(to.0));
                    let (y0, y1) = (from.1.min(to.1), from.1.max(to.1));
                    for x in x0..=x1 {
                        render::put(frame, WIDTH, HEIGHT, x, y0, YELLOW);
                        render::put(frame, WIDTH, HEIGHT, x, y1, YELLOW);
                    }
                    for y in y0..=y1 {
                        render::put(frame, WIDTH, HEIGHT, x0, y, YELLOW);
                        render::put(frame, WIDTH, HEIGHT, x1, y, YELLOW);
                    }
                }
                Tool::Line => {
                    // A straight run of dots between the two ends. Enough
                    // to show what will be laid down; the real stroke is
                    // `paint_capsule_as`, which is a capsule of the
                    // current brush width rather than a hairline.
                    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
                    let steps = dx.abs().max(dy.abs()).max(1);
                    for i in 0..=steps {
                        let x = from.0 + dx * i / steps;
                        let y = from.1 + dy * i / steps;
                        render::put(frame, WIDTH, HEIGHT, x, y, YELLOW);
                    }
                }
                Tool::Brush | Tool::Dig => {}
            }
        }

        if let Some((sx, sy)) = cursor {
            // Under the dig tool the cursor ring moves to where the *bite*
            // will land, not where the mouse is.
            //
            // This is the other half of the fix for "I cannot dig". A
            // reach-limited verb aimed with a free cursor is invisible
            // without it: point past his arms and the cut lands somewhere
            // you did not click, point into a hillside and it lands on
            // the near face, and with a ring drawn under the mouse both
            // read as the game ignoring the click. Drawn from
            // `player::bite_point`, the same function `dig` aims with, so
            // the marker cannot drift from the cut. Sized to
            // `dig_radius`, so the ring also says how big a bite is.
            // Two verbs on one button, so the ring has to say which. A cut
            // is a yellow disc the size of the bite; a shake is a green
            // ring at the plant, sized to nothing in particular because
            // what it shakes is the plant, not a radius.
            let mut shaking = false;
            let dig_marker = match (self.tool, &self.world.player) {
                (Tool::Dig, Some(p)) => {
                    let aim = self.renderer.screen_to_world(sx, sy);
                    let (at, radius) = match player::shake_target(&self.world, p, aim, &self.player_tuning) {
                        Some(at) => {
                            shaking = true;
                            (at, 3)
                        }
                        None => (player::bite_point(&self.world, p, aim, &self.player_tuning), self.player_tuning.dig_radius as i32),
                    };
                    self.renderer.world_to_screen(at.0, at.1).map(|s| (s, radius))
                }
                _ => None,
            };
            let ((cx, cy), radius) = dig_marker.unwrap_or(((sx, sy), self.brush_radius));
            // Brush outline preview -- always on while the cursor is in the
            // window, scaled to match whatever `render.rs`'s own zoom is
            // doing so the ring actually matches the area a click would
            // paint, not the unscaled brush_radius regardless of zoom.
            let screen_radius = if self.renderer.zoom > 1 {
                radius * self.renderer.zoom
            } else {
                (radius / self.renderer.zoom_out_stride.max(1)).max(1)
            };
            let ring = match (dig_marker.is_some(), shaking) {
                (_, true) => GREEN,
                (true, false) => YELLOW,
                (false, false) => WHITE,
            };
            render::draw_circle_outline(frame, WIDTH, HEIGHT, cx, cy, screen_radius, ring);

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
    /// Paint every structurally interesting cell by how close it is to
    /// failing.
    ///
    /// # Why this exists at all
    ///
    /// `load::Load::stress()` is the exact ratio the failure criterion
    /// tests, and before Phase A nothing in the running game read it. The
    /// model decided whether a player's structure stood using quantities
    /// invisible on screen — `attached` multiplies capacity twelvefold and
    /// renders identically either way; `section` can differ 1,600x between
    /// two cells that draw the same. Every shipped stability system in this
    /// genre that players disliked failed on exactly that, and Rust ships a
    /// cruder model than ours that players accept because the number is on
    /// the hammer.
    ///
    /// # Why one cache for the whole screen
    ///
    /// `load::evaluate` walks a subtree, so a per-cell call would re-walk
    /// what the neighbour just walked and the pass would be quadratic. One
    /// shared `load::Cache` makes it O(region). The budget is deliberately
    /// unbounded here: this is a debug view answering a question the user
    /// asked *now*, and a half-drawn overlay would be worse than a slow
    /// one — the honest cost is that it defeats the dirty-rect skip while
    /// it is up, which is the same trade the animated grain documents and
    /// is why it is off by default.
    fn draw_stress_overlay(&self, frame: &mut [u8]) {
        let mut cache = load::Cache::default();
        let mut budget = u32::MAX;
        let (x0, y0) = self.renderer.screen_to_world(0, 0);
        let (x1, y1) = self.renderer.screen_to_world(WIDTH as i32, HEIGHT as i32);
        let zoom = self.renderer.zoom.max(1);
        for wy in y0..=y1 {
            for wx in x0..=x1 {
                let Some(l) = load::evaluate_with_cache(&self.world, wx, wy, &mut cache, &mut budget) else { continue };
                let Some((sx, sy)) = self.renderer.world_to_screen(wx, wy) else { continue };
                // Green at rest through amber to red at the limit, and
                // beyond 1.0 it stays saturated rather than wrapping --
                // "over its limit" is one state, not a gradient, and a cell
                // that is about to go should not fade back toward green.
                let ratio = l.stress().clamp(0.0, 1.0);
                let colour = [(40.0 + 215.0 * ratio) as u8, (220.0 * (1.0 - ratio)) as u8, 60, 255];
                for dy in 0..zoom {
                    for dx in 0..zoom {
                        render::blend(frame, WIDTH, HEIGHT, sx + dx, sy + dy, colour, 0.55);
                    }
                }
            }
        }
    }

    /// What the structural model thinks of the cell under the cursor.
    ///
    /// # Why this is not a nice-to-have
    ///
    /// `load::Load::stress()` is the ratio the failure criterion actually
    /// tests, it has always been computed, and until now **nothing in the
    /// running game read it** — its only consumer was `examples/filmstrip`.
    /// So the model decided whether a player's structure stood using two
    /// quantities that are invisible on screen: `attached`, a bit that
    /// multiplies capacity twelvefold and renders identically either way,
    /// and `section`, which can differ 1,600x between two cells that draw
    /// the same.
    ///
    /// That is precisely how every shipped stress system in this genre
    /// went wrong. `Reports/prior-art-destruction.md`: Medieval Engineers'
    /// complaints were "buttresses push into the legs" — *unpredictable*,
    /// not *too fragile* — while Rust ships a cruder physical model that
    /// players accept because the number is written on the hammer.
    ///
    /// Says why when there is nothing to show, rather than going blank:
    /// "no reading" covers both "solid rock that cannot fail" and "this
    /// cell is not part of the structural system at all", and confusing
    /// those wastes a session.
    fn structural_line(&self, wx: i32, wy: i32) -> String {
        if let Some(load) = load::evaluate(&self.world, wx, wy) {
            return format!(
                "STRESS {:.2} M{} T{}/{}{}{}",
                load.stress(),
                load.mass,
                load.torque,
                load.capacity,
                if load.supported { "" } else { " UNSUPPORTED" },
                if load.truncated { " PARTIAL" } else { "" },
            );
        }
        let cell = self.world.get(wx, wy);
        if !structural::is_body_material(&self.world, cell.material) {
            return "NOT STRUCTURAL".into();
        }
        if cell.aux() == 0 {
            return "ANCHORED".into();
        }
        format!("BULK D{} {}", cell.aux(), if cell.attached() { "BACKGROUND" } else { "FOREGROUND" })
    }

    fn draw_hover_inspector(&self, frame: &mut [u8], sx: i32, sy: i32, colour: [u8; 4]) {
        let (wx, wy) = self.renderer.screen_to_world(sx, sy);
        let cell = self.world.get(wx, wy);
        let material = self.world.materials.get(cell.material).display.clone();
        let field = self.world.field_at(wx, wy);
        let lines = [
            format!("{material} ({wx},{wy})"),
            format!("TEMP {}C{}", cell.temperature(), if cell.is_burning() { " BURNING" } else { "" }),
            format!("P{:.1} T{:.0} L{:.1} M{:.1}", field.pressure, field.temperature, field.light, field.moisture),
            self.structural_line(wx, wy),
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
    ///
    /// Kept as data rather than drawn inline so a test can read it. The
    /// panel is the only place several keys are documented and it drifts
    /// silently — the line describing the gnome's dig outlived the
    /// mechanism it described by two commits, still telling players to
    /// click *near him* long after proximity meant anything.
    /// Geometry of the help overlay. Named because the guard test reads the
    /// same numbers `draw_help` lays out with -- a test that re-derives them
    /// is testing a copy, and the copy is what goes stale.
    const HELP_MARGIN: i32 = 20;
    const HELP_PAD: i32 = 8;
    /// 9 rather than 10: the glyphs are 7 tall, and the extra row per line
    /// was what pushed the old flat list three lines past its own panel.
    const HELP_LINE: i32 = 9;
    const HELP_COL: i32 = 232;
    /// Pixels from a column's left edge to its description text.
    const HELP_KEY: i32 = 56;

    /// One row of the help overlay.
    ///
    /// Data rather than pre-formatted strings, so the key column can be
    /// aligned in pixels and the headings drawn in the accent colour. What
    /// this replaced was a flat `[&str; 32]` with each key jammed against
    /// its own label at whatever column the previous word ended in -- and
    /// it ran **three lines past the bottom of its panel**, so the last
    /// thing it drew off-screen was the line telling you which key closes
    /// it. `the_help_page_fits_inside_its_own_panel` now fails if that
    /// recurs.
    fn help_columns() -> ([HelpRow; 30], [HelpRow; 27]) {
        use HelpRow::{Blank, Head, Key, Note};
        (
            [
                Head("PAINT AND BUILD"),
                Key("LMB/RMB", "PAINT / ERASE"),
                Key("Q E", "CYCLE MATERIAL"),
                Key("1-9", "SELECT MATERIAL"),
                Key("[ ]", "BRUSH SIZE"),
                Key("Z", "TOOL: BRUSH/RECT/ROOM/LINE"),
                Key("B", "STAMP REFERENCE ROOM"),
                Key("TAB", "PALETTE"),
                Blank,
                Head("ACT ON IT"),
                Key("C", "STRIKE ROCK"),
                Key("H", "DIG (PRECISE CUT)"),
                Key("F", "IGNITE"),
                Key("P", "BURST"),
                Key("X", "EXPLODE"),
                Key("T M J", "PLANT TREE/MOSS/WORM"),
                Key("Y", "FOUND COLONY"),
                Blank,
                Head("THE WORLD"),
                Key("SPACE", "PAUSE"),
                Key(".", "STEP ONE FRAME"),
                Key("R", "RESET"),
                Key("= -", "ZOOM"),
                Key("F6 F7 F8", "NEW WORLD / PRESET / SEED"),
                Key("F5", "RELOAD ASSETS"),
                Blank,
                Head("TUNING"),
                Key("O", "TUNABLES PANEL"),
                Key("K", "A/B EXPERIMENT"),
                Key("/ ESC", "THIS HELP / CLOSE"),
            ],
            [
                Head("THE GNOME"),
                Key("U", "SUMMON / DISMISS"),
                Key("A D W", "RUN / JUMP"),
                Key("SHIFT", "HOLD ON IN A TREE"),
                Key("W S", "SWIM UP / DOWN"),
                Key("LMB", "GNOME DIG AT YELLOW RING"),
                Key("LMB", "SHAKE AT THE GREEN RING"),
                Key("F3", "JUMP FEEL"),
                Key("F4", "WATER FEEL"),
                Key("F2", "SPOIL MODE"),
                Key("F9", "CHAIN MODE"),
                Note("NO GNOME: A D W S SCROLL THE MAP"),
                Blank,
                Head("LOOK AT IT"),
                Key("I", "INSPECTOR"),
                Key("V", "FIELD OVERLAY"),
                Key("L", "ORGANISM OVERLAY"),
                Key("N", "STRESS VIEW"),
                Key("F1", "CHUNK OVERLAY"),
                Key("G", "WATER GRAIN"),
                Key("` TICK", "BUBBLES"),
                Key("\\", "GAS LOOKS"),
                Key(",", "TREES IN FRONT / BEHIND"),
                Key("; F10", "DEPTH LIGHT"),
                Key("0 F11", "REVEAL CAVES"),
                Key("F12", "SKY LIGHT"),
                Key("' QUOTE", "GLOW SHAPE"),
            ],
        )
    }

    fn draw_help(&self, frame: &mut [u8]) {
        const BG: [u8; 4] = [10, 10, 16, 255];
        const WHITE: [u8; 4] = [225, 228, 235, 255];
        const ACCENT: [u8; 4] = [90, 170, 240, 255];
        /// Keys sit a step below their own description, so the eye runs down
        /// the descriptions and only crosses to the key it needs.
        const KEYCAP: [u8; 4] = [150, 158, 175, 255];
        let m = Self::HELP_MARGIN;
        let (left, top, right, bottom) = (m, m, WIDTH as i32 - m, HEIGHT as i32 - m);
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
        let (col_a, col_b) = Self::help_columns();
        for (c, rows) in [&col_a[..], &col_b[..]].iter().enumerate() {
            let x = left + Self::HELP_PAD + c as i32 * Self::HELP_COL;
            for (i, row) in rows.iter().enumerate() {
                let y = top + Self::HELP_PAD + i as i32 * Self::HELP_LINE;
                match row {
                    HelpRow::Head(title) => {
                        hud::draw_text(frame, WIDTH, HEIGHT, x, y, title, ACCENT);
                    }
                    HelpRow::Key(key, what) => {
                        hud::draw_text(frame, WIDTH, HEIGHT, x, y, key, KEYCAP);
                        hud::draw_text(frame, WIDTH, HEIGHT, x + Self::HELP_KEY, y, what, WHITE);
                    }
                    HelpRow::Note(text) => {
                        hud::draw_text(frame, WIDTH, HEIGHT, x, y, text, KEYCAP);
                    }
                    HelpRow::Blank => {}
                }
            }
        }
    }

    /// Re-read the material and species files. Ids are keyed by name, so
    /// material/species already in the world keep their identity and simply
    /// start behaving differently.
    pub fn reload_materials(&mut self) {
        self.message = reload_assets(&mut self.world).map(|s| format!("reloaded {s}"));
        // The watcher lands here on any external edit to the asset files,
        // so the marker tracks hand-edits as well as panel saves.
        self.assets_dirty = dirty_asset_count();
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
        let p = self.renderer.screen_to_world(screen_x, screen_y);
        self.paint_stroke(p, p, erase);
    }

    /// Paint the area the brush swept between two screen positions, so a fast
    /// drag leaves one continuous stroke rather than a row of blobs.
    /// Both endpoints are **world** cells, not screen pixels. They were
    /// screen pixels while the view could not move; with a camera, the
    /// caller holds the previous point across frames, and a screen point
    /// held across a camera move names a different cell each frame — the
    /// stroke would smear backwards as the gnome walked.
    pub fn paint_stroke(&mut self, from: (i32, i32), to: (i32, i32), erase: bool) {
        // M9 phase 2: under `Tool::Dig`, the left button is the gnome's
        // dig rather than the brush — on his own cooldown, cutting the
        // near face along the aim, spoil shoved aside rather than deleted
        // (`player::dig`). The right button stays the eraser, which is
        // also the rescue if he digs himself somewhere impossible.
        //
        // Keyed on the *tool*, not on how close the cursor is to him.
        // See `Tool::Dig`'s own doc for the playtest that killed the
        // proximity version: a reach may bound where a verb lands and
        // must never decide whether it happens, or the verb is invisible.
        // Gated here rather than in `main.rs`'s `paint_now` for the
        // reason that function's own comment gives: one gate on the
        // operation, three call sites.
        if !erase && self.tool == Tool::Dig && self.world.player.is_some() {
            // One button, and what it does is decided by what he is
            // pointing at: rock is cut, a living plant is shaken. Same
            // reasoning as the tool gate itself -- adding a second key for
            // the second verb, on a keyboard with nothing left on it,
            // would make the verb invisible in exactly the way proximity
            // gating did. The ring says which one you will get before you
            // click.
            let shake_at = self
                .world
                .player
                .as_ref()
                .and_then(|p| player::shake_target(&self.world, p, to, &self.player_tuning));
            match shake_at {
                Some(at) => {
                    player::shake(&mut self.world, at, &self.player_tuning);
                }
                None => {
                    player::dig(&mut self.world, to, &self.player_tuning);
                }
            }
            return;
        }
        let m = if erase {
            material::EMPTY
        } else {
            self.selected_material()
        };
        let density = self.emission_density(m, erase);
        self.world
            .paint_capsule_as(from, to, self.brush_radius, m, density);
    }

    /// Screen pixel to world cell. Exposed so callers that hold a point
    /// *across frames* (a paint stroke, a drag) can store it in world
    /// space, which is the only space that still means the same thing
    /// after the camera has moved.
    pub fn to_world(&self, screen: (i32, i32)) -> (i32, i32) {
        self.renderer.screen_to_world(screen.0, screen.1)
    }

    /// Scroll the view — `WASD` with no gnome in the world. `dir` is -1/0/+1
    /// per axis and `seconds` is real elapsed time; `main.rs` turns held keys
    /// into both. The world keeps simulating underneath, since scrolling is a
    /// view change and `Space` is still the way to stop time.
    ///
    /// **Both gates live here, not at the call site**, for the reason
    /// `main.rs`'s `paint_now` already records about its own: a gate on the
    /// operation cannot be missed by a second caller added later, and a
    /// gate at one of two call sites will be.
    ///
    /// - A gnome owns the camera *and* those four keys while he exists, so
    ///   this is inert the moment one is summoned and `Renderer::follow`
    ///   takes the view back. That is the whole reason the feature needs no
    ///   mode toggle: the two readings of `WASD` are mutually exclusive by
    ///   construction rather than by a flag somebody has to keep in sync.
    /// - The tunables panel owns `S` while it is open (it saves), so
    ///   scrolling stands aside rather than doing both at once.
    ///
    /// Returns whether the view was free to move, so the caller can end a
    /// gesture that is being ignored rather than let its carried sub-cell
    /// remainder sit behind a closed gate — see `main.rs`'s `pan`.
    pub fn pan_camera(&mut self, dir: (i32, i32), seconds: f32) -> bool {
        if self.world.player.is_some() || self.show_tunables {
            return false;
        }
        let bounds = self.world.bounds();
        self.renderer.pan(dir, seconds, (WIDTH, HEIGHT), bounds);
        true
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

    /// Strike the rock under the cursor — the destruction *verb*.
    ///
    /// Scaled off the brush so the tool the player is already sizing is the
    /// tool that decides how hard they hit, rather than introducing a second
    /// invisible number to tune. See `rigid::strike`.
    pub fn strike(&mut self, screen_x: i32, screen_y: i32) {
        let (x, y) = self.renderer.screen_to_world(screen_x, screen_y);
        let force = self.brush_radius as f32 * STRIKE_FORCE_PER_RADIUS;
        crate::sim::rigid::strike(&mut self.world, x, y, self.brush_radius, force);
    }

    /// Cut rock away precisely under the cursor — the *mining* verb, as
    /// distinct from the eraser, which deletes matter and tells the
    /// structural model nothing. See `rigid::mine`.
    ///
    /// Spoil comes from `player_tuning.dig_yield`, the same number the
    /// gnome digs by and `F2` cycles. It used to take whatever fell out
    /// instead, so the sandbox verb and the character dug different holes
    /// in the same rock — `brush_radius`'s one-number-two-verbs bug in a
    /// second place.
    pub fn mine(&mut self, screen_x: i32, screen_y: i32) {
        let (x, y) = self.renderer.screen_to_world(screen_x, screen_y);
        crate::sim::rigid::mine(&mut self.world, x, y, self.brush_radius, self.player_tuning.dig_yield);
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

    /// Found an ant colony on the ground under a screen position — `Y`.
    /// See `World::found_colony`; a colony, not an ant, because fifty is
    /// roughly where ants start behaving like ants.
    pub fn found_colony(&mut self, screen_x: i32, screen_y: i32) {
        let (x, y) = self.renderer.screen_to_world(screen_x, screen_y);
        let placed = self.world.found_colony(x, y);
        // **Say what happened, including when nothing did.** Pressing this
        // over open sky used to place nothing and report nothing, which
        // reads exactly like the feature not existing.
        if placed == 0 {
            self.show_toast("no ground under the cursor - point at terrain and press Y");
        } else {
            self.show_toast(format!("colony founded: {placed} ants (V cycles the pheromone overlay)"));
        }
    }

    /// Summon the gnome at a screen position, or dismiss him if already
    /// present — `U`. Opt-in by design: the sandbox stays a pure tool
    /// until a character is asked for, and everything character-shaped
    /// (held-key movement, the PLAYER tunables) is inert without one.
    pub fn summon_player(&mut self, screen_x: i32, screen_y: i32) {
        if self.world.player.take().is_some() {
            // Dig without a digger is a mode that does nothing, so the
            // brush comes back with him gone.
            if self.tool == Tool::Dig {
                self.tool = Tool::Brush;
            }
            self.message = Some("gnome dismissed".into());
            // A **toast**, not just `message`: dismissing him hands the camera
            // back, so `WASD` changes meaning underneath you, and that is
            // exactly the kind of mode change that is invisible unless it is
            // announced where someone is looking. `message` reaches the window
            // title bar and the tunables panel footer only — its own doc says
            // so, and neither is where anyone's eyes are when they press `U`.
            self.show_toast("GNOME DISMISSED - WASD NOW SCROLLS THE VIEW");
            return;
        }
        let (x, y) = self.renderer.screen_to_world(screen_x, screen_y);
        self.world.player = Some(player::Player::at(x, y));
        // Switched into the dig tool on arrival rather than left for the
        // player to find. A verb nobody knows exists is a verb that does
        // not exist — see `Tool::Dig` — and arriving in it also makes the
        // persistent HUD label say what the mouse now does.
        self.tool = Tool::Dig;
        self.message =
            Some("gnome summoned — A/D run, W jump, LMB dig, F2/F3/F4 feel presets, U dismiss".into());
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
        build_world_with(&mut world, &self.worldgen, &self.worldgen_preset, self.worldgen_seed);
        self.world = world;
        // A rebuilt world is a fresh `World`, and `World::new` starts at
        // `i32::MAX` (SPREAD). The chain mode is a *player setting*, not a
        // property of the terrain, so it has to be re-applied here or every
        // `F6`/`F7`/`F8` and every worldgen hot-reload silently reverts it --
        // while `App::chain_mode` and the status line keep naming the mode the
        // player chose. Every A/B of chain modes that rerolled the seed between
        // arms was therefore comparing SPREAD against SPREAD, which is how
        // "LOCAL and TIGHT do not contain anything" came to be reported.
        self.world.chain_reach = crate::sim::structural::CHAIN_MODES[self.chain_mode].reach;
    }

    pub fn toggle_overlay(&mut self) {
        self.renderer.show_chunk_overlay = !self.renderer.show_chunk_overlay;
    }

    /// Status line for the window title — cheaper than rendering text, and
    /// enough to verify frame rate and sleeping at a glance.
    pub fn status(&self, fps: f32) -> String {
        format!(
            "Pixel Physics — {:.0} fps — {} (brush {}) — chunks {}/{} awake — {} {:#018X}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
            fps,
            self.selected_name(),
            self.brush_radius,
            self.world.active_chunk_count(),
            self.world.chunk_count(),
            // Always shown, never elided. A screenshot of a generated world
            // that does not say which world it is cannot be reproduced, and
            // this generator will be judged almost entirely from screenshots.
            self.worldgen_preset,
            self.worldgen_seed,
            if self.paused { " — PAUSED" } else { "" },
            // Only shown once it has been changed, so the ordinary status
            // line is untouched until someone is actually comparing modes.
            if self.renderer.grain == render::GrainMode::Position {
                String::new()
            } else {
                format!(" — grain {}", self.renderer.grain.label())
            },
            // Same rule again: silent at the default, named the moment it
            // is not, because the value of a look selector is being able to
            // say afterwards which one you liked.
            if self.renderer.bubbles == render::BubbleMode::default() {
                String::new()
            } else {
                format!(" — bubbles {}", self.renderer.bubbles.label())
            },
            if self.renderer.gas == render::GasMode::default() {
                String::new()
            } else {
                format!(" — gas {}", self.renderer.gas.label())
            },
            // Same rule again for the terrain depth light (`F10`), with the
            // roles flipped: the depth grade is the default, so the label
            // appears when someone has switched *back* to the flat look and
            // needs the screenshot to say so.
            if self.renderer.terrain_light == render::TerrainLight::default() {
                String::new()
            } else {
                format!(" — light {}", self.renderer.terrain_light.label())
            },
            // And the glow shape (`'`), same flipped rule as the depth light:
            // the near-field term is the default, so the label appears only
            // when someone has switched back to the 8-cell blocks to compare.
            if self.renderer.glow_shape == render::GlowShape::default() {
                String::new()
            } else {
                format!(" — glow {}", self.renderer.glow_shape.label())
            },
            // Sky light (`F12`), same rule as the rest: silent on the
            // default, named the moment someone switches away from it. Since
            // the verdicts made propagated /4 the default, the label now
            // appears when someone has gone *back* to the old depth fade to
            // compare — which is exactly when a screenshot needs to say so.
            if self.renderer.sky_light == render::SkyLight::default() {
                String::new()
            } else {
                format!(" — sky light {}", self.renderer.sky_light.label())
            },
            // The void reveal (`F11`) is a debug X-ray, so a screenshot with
            // it on must say so — magenta caves in a shared image with no
            // label would read as a rendering bug.
            if self.renderer.reveal_voids { " — VOIDS REVEALED" } else { "" },
            // Same rule as grain: silent until someone is actually
            // comparing, then named, because the whole value of a
            // selector is being able to report which one you liked.
            match (self.movement_feel, self.water_feel) {
                (0, 0) => String::new(),
                (m, 0) => format!(" — jump {}", player::MOVEMENT_FEELS[m].name),
                (0, w) => format!(" — water {}", player::WATER_FEELS[w].name),
                (m, w) => format!(
                    " — jump {} — water {}",
                    player::MOVEMENT_FEELS[m].name,
                    player::WATER_FEELS[w].name
                ),
            },
            if self.spoil_mode == 0 {
                String::new()
            } else {
                format!(" — spoil {}", player::SPOIL_MODES[self.spoil_mode].name)
            },
            // Same rule again: silent at the default, named the moment it
            // is not, so a screenshot of a stand can be reported as having
            // been taken in a particular mode.
            if self.renderer.tree_depth == render::TreeDepth::default() {
                String::new()
            } else {
                format!(" — trees {}", self.renderer.tree_depth.label())
            },
            // Same "only once turned on" rule as spoil: silent at the
            // default, named on screen the moment it is not, because a
            // destruction model behaving differently from the shipped one
            // must never be a thing the player has to remember.
            if self.chain_mode == 0 {
                String::new()
            } else {
                format!(" — chain {}", crate::sim::structural::CHAIN_MODES[self.chain_mode].name)
            },
            // The same rule again, and this one was **missing entirely**
            // until the overlay it names was mistaken for a bug. `V` cycles
            // Off -> Pressure -> Temperature -> Light -> Moisture ->
            // Pheromone A -> Pheromone B -> Off, and `FieldOverlay::Light`
            // is a pale cream blended at up to 75% over every pixel
            // *including solid rock* -- so a player who pressed `V` four
            // times got "a pale light effect spreading through rock" with
            // nothing on screen to say why, or that it was a debug channel
            // at all. Every other selector here already knew that the whole
            // value of a selector is being able to report which one you
            // liked.
            if self.renderer.field_overlay == render::FieldOverlay::Off {
                String::new()
            } else {
                format!(" — field {}", self.renderer.field_overlay.label())
            },
            // Same "only once turned on" rule. Worth showing at all because
            // the tint is subtle on a sparse tree and "is this channel on,
            // or is it on and reading zero everywhere?" is exactly the
            // question the overlay exists to answer unambiguously.
            if self.renderer.organism_overlay == render::OrganismOverlay::Off {
                String::new()
            } else {
                format!(" — organism {}", self.renderer.organism_overlay.label())
            },
            // Uncommitted asset edits, so a value saved mid-playtest is a
            // glance rather than an audit. Silent when git can't answer.
            match self.assets_dirty {
                Some(n) if n > 0 => format!(" — ASSETS EDITED ({n})"),
                _ => String::new(),
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
    worldgen::generate(world, worldgen::Spec::Legacy);
}

/// Just the material placement, without the structural pass `build_terrain`
/// runs after it. Split out so `examples/ascii.rs` can time the two halves
/// separately and attribute the generation cost rather than just stating it.
pub fn build_terrain_only(world: &mut World) {
    worldgen::generate_only(world, worldgen::Spec::Legacy);
}

#[cfg(test)]
mod tests {

    /// The old flat help list ran three lines past the bottom of its own
    /// panel, so the line naming the key that closes it was drawn
    /// off-screen. Reads the same geometry constants `draw_help` lays out
    /// with, so it cannot drift from what is actually rendered.
    #[test]
    fn the_help_page_fits_inside_its_own_panel() {
        let (a, b) = App::help_columns();
        let m = App::HELP_MARGIN;
        let (left, top, right, bottom) = (m, m, WIDTH as i32 - m, HEIGHT as i32 - m);
        for (c, rows) in [&a[..], &b[..]].iter().enumerate() {
            let x = left + App::HELP_PAD + c as i32 * App::HELP_COL;
            // A column may not spill into the next one, nor past the border.
            let limit = if c == 0 { x + App::HELP_COL - 4 } else { right - App::HELP_PAD };
            for (i, row) in rows.iter().enumerate() {
                let y = top + App::HELP_PAD + i as i32 * App::HELP_LINE;
                assert!(
                    y + hud::GLYPH_HEIGHT <= bottom - 2,
                    "column {c} row {i} is drawn at y={y}, past the panel bottom {bottom}"
                );
                let (start, text) = match row {
                    HelpRow::Head(s) | HelpRow::Note(s) => (x, *s),
                    HelpRow::Key(k, w) => {
                        assert!(
                            x + hud::text_width(k) <= x + App::HELP_KEY - 2,
                            "key {k:?} overruns the key column into its own description"
                        );
                        (x + App::HELP_KEY, *w)
                    }
                    HelpRow::Blank => continue,
                };
                assert!(
                    start + hud::text_width(text) <= limit,
                    "column {c} row {i} ({text:?}) reaches {}, past its limit {limit}",
                    start + hud::text_width(text)
                );
            }
        }
    }

    /// The font renders anything it lacks as a blank gap rather than a
    /// mystery box, so a key the page names in punctuation can silently
    /// list itself as nothing at all -- which `;` and `'` did for as long
    /// as they were bound. Catches the whole class rather than those two.
    #[test]
    fn the_help_page_only_uses_glyphs_the_font_has() {
        let (a, b) = App::help_columns();
        for rows in [&a[..], &b[..]] {
            for row in rows {
                let texts: [&str; 2] = match row {
                    HelpRow::Head(s) | HelpRow::Note(s) => [s, ""],
                    HelpRow::Key(k, w) => [k, w],
                    HelpRow::Blank => continue,
                };
                for text in texts {
                    for ch in text.chars() {
                        assert!(
                            hud::has_glyph(ch),
                            "the help page prints {ch:?} (in {text:?}), which the font draws as a blank gap"
                        );
                    }
                }
            }
        }
    }

    /// Not a guard -- a way to look at the page, since it is judged by eye.
    /// Writes only when `HELP_PNG` names a path.
    #[test]
    fn dump_the_help_page_when_asked() {
        let Ok(out) = std::env::var("HELP_PNG") else { return };
        let app = App::new_pending();
        let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        for px in frame.chunks_exact_mut(4) {
            px.copy_from_slice(&[40, 44, 52, 255]);
        }
        app.draw_help(&mut frame);
        image::save_buffer(&out, &frame, WIDTH, HEIGHT, image::ColorType::Rgba8).unwrap();
        eprintln!("wrote {out}");
    }
    use super::*;
    // Only the tests build cells directly now that the terrain moved to
    // `worldgen`.
    use crate::sim::Cell;

    fn id(app: &App, name: &str) -> MaterialId {
        app.world.materials.id_of(name).expect(name)
    }

    /// An app on the hand-authored terrain.
    ///
    /// `App::new` builds a *generated* world now, so any test written against
    /// the sandbox terrain's exact coordinates — its ledges, its eight-row
    /// floor — has to ask for that terrain by name. Kept pointed at the real
    /// layout rather than relaxed into "some stone somewhere": these tests
    /// are about a known shape standing up, and a version that passed against
    /// any terrain would have stopped testing the thing it was written for.
    /// The bottom row of the **world**, which since the world grew past the
    /// viewport is no longer `HEIGHT - 1`. Tests that name the floor's exact
    /// rows have to ask the world where its floor is.
    fn world_bottom(app: &App) -> i32 {
        app.world.bounds().expect("the app's world is bounded").max_y
    }

    /// Every world cell. The scans below used `0..WIDTH x 0..HEIGHT`, which
    /// was the whole world only while the world was exactly one screen; on
    /// the world this ships at now — sixteen screens wide and eight deep —
    /// they would have quietly searched the top-left corner and reported
    /// "nothing anywhere" about a sixteenth of an eighth of it (1/128).
    fn world_cells(app: &App) -> impl Iterator<Item = (i32, i32)> {
        let b = app.world.bounds().expect("the app's world is bounded");
        (b.min_x..=b.max_x).flat_map(move |x| (b.min_y..=b.max_y).map(move |y| (x, y)))
    }

    /// The size the tests in this file build at, and why it is not the
    /// shipped one.
    ///
    /// **A cost, measured, not a preference.** The shipped world is
    /// 8192x2560 and takes 9.0 s to generate on this machine at 359 MiB of
    /// peak RSS (`examples/scale_probe.rs`, the instrument). Thirty-odd
    /// tests here build one, and cargo runs them on several threads at once,
    /// so `cargo test --lib` went from about a minute to past ten and held
    /// several third-of-a-gigabyte worlds simultaneously. At 2048x640 the
    /// same build is ~0.46 s.
    ///
    /// **What it must not become is a smaller question.** Nothing here is
    /// asserted against a coordinate this size supplies: the helpers ask the
    /// world for its own edges (`world_bottom`, `world_cells`), which is what
    /// already made these tests honest when the world outgrew the viewport.
    /// The size that ships is still exercised --
    /// `a_shipped_size_world_is_generated_and_at_rest` builds the real thing
    /// -- so a bug that only appears at 8192x2560 has somewhere to show up.
    ///
    /// 2048x640 rather than something smaller because it is the size round 7
    /// shipped: every test in this file passed against it, so choosing it
    /// changes what these tests measure by exactly nothing.
    pub(super) const TEST_WORLD: (u32, u32) = (2048, 640);

    pub(super) fn test_app() -> App {
        App::build(true, TEST_WORLD, &mut |_, _| {})
    }

    fn legacy_app() -> App {
        let mut app = test_app();
        app.worldgen_preset = worldgen::LEGACY.to_string();
        app.reset();
        app
    }

    /// The world the app actually ships generates, arrives at rest, and has
    /// rock in it.
    ///
    /// The one test here that pays for the shipped 8192x2560 (9.0 s, 359 MiB
    /// peak). It exists because every other test in this file now builds at
    /// [`TEST_WORLD`], and a suite where nothing ever constructs the shipped
    /// size is a suite that cannot see a bug that only appears there -- and
    /// this engine has shipped exactly that shape of miss before, most
    /// recently a "below the world floor" water table that stopped being
    /// below the world floor two size changes ago and was asserted against a
    /// literal the whole time.
    ///
    /// Asserts the two properties that are genuinely size-dependent: the
    /// generator fills a world this large (a pass that silently declines on
    /// a bigger world writes nothing, and `every_pass_writes_something` runs
    /// at 512x320), and the result is at rest (the at-rest guarantee is
    /// about placement, and placement reads the world's own dimensions).
    #[test]
    fn a_shipped_size_world_is_generated_and_at_rest() {
        let mut app = App::new();
        let b = app.world.bounds().expect("the app's world is bounded");
        assert_eq!((b.max_x + 1, b.max_y + 1), (WORLD_WIDTH as i32, WORLD_HEIGHT as i32));

        // Sampled rather than censused: a full scan here is 21M `get`s, and
        // the question is "is there a world", not "how much of it".
        let solid = (b.min_y..=b.max_y)
            .step_by(32)
            .flat_map(|y| (b.min_x..=b.max_x).step_by(32).map(move |x| (x, y)))
            .filter(|&(x, y)| app.world.get(x, y).material != material::EMPTY)
            .count();
        assert!(solid > 1000, "a {}x{} world has only {solid} sampled cells in it", WORLD_WIDTH, WORLD_HEIGHT);

        // A **census**, not a chunk count. `CLAUDE.md`: a failure count is
        // not a damage count, and here the same distinction bites the other
        // way -- an *awake* chunk has only been scheduled for a sweep that
        // will confirm it is still, which is not the same as a cell having
        // moved. The first version of this asserted `active_chunk_count() ==
        // 0` within 20 frames and failed, and the world was at rest the whole
        // time: 5120 chunks simply take longer to walk down to sleeping than
        // 320 did (traced below: 4936 awake on frame 1, 35 by frame 60, and
        // nothing moving in any of them).
        //
        // What is actually worth asserting at this size is that the terrain
        // does not avalanche, and mineral cells only ever *leave* when
        // something ran -- plants add cells and never remove rock, so this
        // survives the life pass being on, which a positional snapshot would
        // not.
        let mineral = |app: &App| {
            let mut n = 0usize;
            for y in b.min_y..=b.max_y {
                for x in b.min_x..=b.max_x {
                    let m = app.world.get(x, y).material;
                    if m != material::EMPTY && matches!(app.world.materials.get(m).kind, MaterialKind::Solid | MaterialKind::Powder) {
                        n += 1;
                    }
                }
            }
            n
        };
        let before = mineral(&app);

        let total = app.world.chunk_count();
        let mut trace = Vec::new();
        for _ in 0..60 {
            app.update();
            trace.push(app.world.active_chunk_count());
        }
        let after = mineral(&app);
        println!("awake chunks of {total} over 60 frames: {trace:?}");
        println!("mineral cells: {before} -> {after}");

        // A bar with headroom rather than equality: a generated world has
        // real powder in it, and a handful of grains finding a lower seat is
        // the ordinary case the at-rest suite already characterises exactly
        // (`tests/worldgen.rs::generated_terrain_is_already_at_rest`, which
        // asserts *zero* over 120 frames at 512x320 with life off). What
        // this catches is the class that only appears at size: a slope that
        // holds at 640 rows and runs at 2560.
        let lost = before.saturating_sub(after);
        assert!(
            lost * 1000 < before,
            "the shipped {WORLD_WIDTH}x{WORLD_HEIGHT} world lost {lost} of {before} mineral cells \
             in 60 frames -- it is avalanching, not settling"
        );
        assert!(trace[59] < trace[0] / 10, "awake chunks are not draining: {trace:?}");
    }

    /// Step until every chunk is asleep, asserting that it happens quickly.
    ///
    /// Two frames used to be enough anywhere, because the starting world was
    /// stone and bedrock — solids, which have no movement rule to run. A
    /// generated world contains real powder (soil blankets, scree, buried
    /// lenses), and powder cannot be known to be at rest until it has been
    /// examined, so settling now takes a few frames rather than exactly one.
    /// The claim worth testing is unchanged and is still asserted here: the
    /// world goes quiet promptly and does not keep waking itself.
    fn settle(app: &mut App) {
        for frame in 0..20 {
            app.update();
            if app.world.active_chunk_count() == 0 {
                return;
            }
            assert!(frame < 19, "world never went quiet");
        }
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
        let app = test_app();
        assert_eq!(app.selected_material(), id(&app, "sand"));
        assert_eq!(app.selected_name(), "Sand");
    }

    #[test]
    fn materials_load_from_the_assets_directory() {
        let app = test_app();
        // Every shipped material should be offered in the picker.
        for name in ["stone", "sand", "gravel", "ash", "water", "oil", "smoke"] {
            let m = id(&app, name);
            assert!(app.paintable.contains(&m), "{name} is not paintable");
        }
    }

    /// The guard for the M9 bug that reached a playtest: the dig existed,
    /// had seven passing unit tests, and **could not be performed with
    /// the mouse**, because the only route to it was a click within
    /// `dig_reach` world cells of a character a few pixels tall. Every
    /// test called `player::dig` directly and so proved nothing about the
    /// path a player actually uses.
    ///
    /// So this one goes through `paint_stroke`, the function the mouse
    /// reaches, from a cursor position deliberately far from him.
    #[test]
    fn a_click_far_from_the_gnome_still_digs_rather_than_painting() {
        let mut app = test_app();
        let stone = id(&app, "stone");
        // A wall to his right, and open ground to stand on.
        for y in 40..70 {
            for x in 0..120 {
                app.world.set(x, y, Cell::EMPTY);
            }
        }
        for x in 0..120 {
            app.world.set(x, 70, Cell::new(stone, 0).with_attached(true));
        }
        for y in 40..70 {
            for x in 45..120 {
                app.world.set(x, y, Cell::new(stone, 0).with_attached(true));
            }
        }
        app.summon_player(30, 64);
        assert_eq!(app.tool, Tool::Dig, "summoning must arrive in the dig tool, or nothing announces the verb");

        let count_stone = |app: &App| {
            (40..70).map(|y| (0..120).filter(|&x| app.world.get(x, y).material == stone).count()).sum::<usize>()
        };
        let before = count_stone(&app);
        // Well past `dig_reach` and past the wall's near face: the case
        // the shipped version silently painted on. The bite is clamped
        // back onto the face, which is inside his reach, so rock goes.
        app.paint_stroke((110, 64), (110, 64), false);
        let after = count_stone(&app);
        assert!(after < before, "a far click should have cut rock: {before} -> {after}");

        // And the brush is still reachable: `Z` off the dig tool paints.
        app.cycle_tool();
        assert_ne!(app.tool, Tool::Dig, "Z must get the player out of the dig tool");
    }

    /// A click on a living plant must reach the *shake*, through the
    /// function the mouse actually calls.
    ///
    /// Written because the shake shipped without it. Every test of that
    /// verb called `player::shake_target`/`player::shake` directly, which
    /// is precisely the hole the test above records for the dig — "a test
    /// called `player::dig` directly and so proved nothing about the path
    /// a player actually uses" — reproduced one verb later.
    ///
    /// The discriminator is **dislodged material**, because it is the one
    /// effect only a shake has and the only one with no RNG in it. Shed
    /// leaves are a weighted roll and seed needs a grown tree; a dig
    /// cannot touch organism cells at all, so "the wood is still there"
    /// would pass whichever verb fired.
    #[test]
    fn a_click_on_a_tree_shakes_it_rather_than_cutting_or_painting() {
        let mut app = test_app();
        let stone = id(&app, "stone");
        let wood = id(&app, "wood");
        let sand = id(&app, "sand");
        for y in 40..70 {
            for x in 0..120 {
                app.world.set(x, y, Cell::EMPTY);
            }
        }
        for x in 0..120 {
            app.world.set(x, 70, Cell::new(stone, 0).with_attached(true));
        }
        // A living trunk within `shake_reach`, with open air either side of
        // it so the dislodge has somewhere to drop from.
        let species = app.world.species.id_of("tree").expect("tree is compiled in");
        let organism = app.world.push_organism(species);
        let aux = crate::sim::organism::pack_cell_type(crate::sim::organism::CellType::MatureBody);
        for y in 55..70 {
            app.world.set(40, y, Cell::new(wood, 0).with_organism_id(organism).with_aux(aux));
        }
        // One grain resting on the top of it.
        app.world.set(40, 54, Cell::new(sand, 0));

        app.summon_player(30, 64);
        assert_eq!(app.tool, Tool::Dig, "summoning must arrive in the dig tool");

        app.paint_stroke((60, 64), (60, 64), false);

        assert_ne!(
            app.world.get(40, 54).material,
            sand,
            "the grain resting on the trunk should have been shaken off it"
        );
        assert_eq!(
            app.world.get(40, 60).material,
            wood,
            "and the trunk itself is not something a click may cut or paint over"
        );
    }

    /// The help panel is the only place several keys are documented, and
    /// it drifts silently: the line describing the gnome's dig outlived
    /// the mechanism it described by two commits, still telling players
    /// to click *near him* long after proximity stopped meaning
    /// anything. Nothing failed, because nothing checks prose.
    ///
    /// This checks the part that can be checked mechanically — that the
    /// keys the panel advertises for the character are the keys `main.rs`
    /// binds — and is the reason to update the panel in the same change
    /// as a rebind rather than afterwards.
    #[test]
    fn the_help_panel_names_the_keys_the_gnome_actually_uses() {
        let (a, b) = App::help_columns();
        let help = [&a[..], &b[..]]
            .iter()
            .flat_map(|rows| rows.iter())
            .map(|row| match row {
                HelpRow::Head(s) | HelpRow::Note(s) => (*s).to_string(),
                HelpRow::Key(k, w) => format!("{k} {w}"),
                HelpRow::Blank => String::new(),
            })
            .collect::<Vec<_>>()
            .join("\n");
        for key in ["U SUMMON", "F3 JUMP FEEL", "F4 WATER FEEL", "F2 SPOIL", "GNOME DIG", "SCROLL THE MAP"] {
            assert!(help.contains(key), "help panel no longer mentions {key:?}");
        }
        assert!(
            !help.contains("CLICK NEAR THE GNOME"),
            "the proximity-gated dig is gone; the help must not still describe it"
        );
    }

    #[test]
    fn the_view_is_the_players_only_while_no_gnome_is_summoned() {
        // The whole design in one assertion: the two readings of `WASD` are
        // mutually exclusive by construction, not by two call sites both
        // remembering. It has to be by construction — `App::draw` re-centres
        // on a gnome every frame, so a pan that *did* work while one existed
        // would be visibly yanked back on the next frame, and a control that
        // fights back is worse than one that is not there.
        let mut app = test_app();
        let before = (app.renderer.camera_x, app.renderer.camera_y);
        assert!(app.pan_camera((1, 0), 0.5), "no gnome: the view must be free to move");
        assert_ne!(
            (app.renderer.camera_x, app.renderer.camera_y), before,
            "no gnome: WASD must scroll the view"
        );

        app.summon_player(100, 100);
        // Load-bearing: `summon_player` can leave `player` as `None`, and
        // without this the half below passes because nothing was summoned —
        // the "check the scene still contains the situation you think it
        // does" trap.
        assert!(app.world.player.is_some(), "the summon must have landed or the rest of this proves nothing");
        let his = (app.renderer.camera_x, app.renderer.camera_y);
        assert!(!app.pan_camera((1, 0), 0.5), "with a gnome, the view is his");
        assert_eq!(
            (app.renderer.camera_x, app.renderer.camera_y), his,
            "with a gnome, a pan must not move the camera"
        );
    }

    #[test]
    fn the_tunables_panel_keeps_its_own_keys_while_it_is_open() {
        // `S` saves a tunable while the panel is open. Without this gate it
        // would both save and scroll on the same keypress.
        let mut app = test_app();
        app.show_tunables = true;
        let before = (app.renderer.camera_x, app.renderer.camera_y);
        assert!(!app.pan_camera((0, 1), 0.5), "the panel owns S while it is open");
        assert_eq!((app.renderer.camera_x, app.renderer.camera_y), before, "the panel is open; the view must hold still");

        // And gives them back on close -- otherwise this passes against a
        // `pan_camera` that never works at all.
        app.show_tunables = false;
        assert!(app.pan_camera((0, 1), 0.5), "closing the panel must hand the keys back");
        assert_ne!((app.renderer.camera_x, app.renderer.camera_y), before, "closing the panel must hand the view back");
    }

    #[test]
    fn reloading_materials_keeps_the_current_selection() {
        let mut app = test_app();
        app.select_material(2);
        let before = app.selected_material();
        app.reload_materials();
        assert_eq!(app.selected_material(), before);
    }

    #[test]
    fn the_status_line_shows_uncommitted_asset_edits_and_stays_quiet_otherwise() {
        // The field is set directly rather than through `dirty_asset_count`
        // — what the working tree actually contains while the suite runs is
        // not this test's to assert. What is: a real count reaches the eye,
        // and both "clean" and "git couldn't answer" stay silent.
        let mut app = test_app();
        app.assets_dirty = Some(2);
        assert!(app.status(60.0).contains("ASSETS EDITED (2)"), "a dirty asset count must be visible in the title");
        app.assets_dirty = Some(0);
        assert!(!app.status(60.0).contains("ASSETS EDITED"), "a clean tree must not add noise to the title");
        app.assets_dirty = None;
        assert!(!app.status(60.0).contains("ASSETS EDITED"), "git being unavailable is silence, not a warning");
    }

    /// The field overlay was the one selector on the status line that never
    /// said its own name, and `FieldOverlay::Light` is a pale cream blended
    /// at up to 75% over *every* pixel including solid rock. A player who
    /// pressed `V` four times had no way to find out why the world had
    /// gone pale, and the report that came back — "a pale light effect
    /// spreads through rock" — is a literal description of it.
    #[test]
    fn the_status_line_names_the_field_overlay_it_is_showing() {
        let mut app = App::new();
        assert!(
            !app.status(60.0).contains("field"),
            "the default line must stay quiet, the same as grain, spoil, chain and organism do"
        );
        app.renderer.field_overlay = render::FieldOverlay::Light;
        assert!(
            app.status(60.0).contains("field LIGHT"),
            "a selected field overlay has to name itself: {}",
            app.status(60.0)
        );
    }

    #[test]
    fn reloading_materials_wakes_the_world() {
        // Changed friction or dispersion can unstick material that had settled,
        // so everything must be re-examined.
        //
        // On the lifeless legacy terrain, deliberately. The default generated
        // world plants trees and moss (worldgen's life pass), and since the
        // light model became a per-column sky cast those seeds germinate
        // within a frame or two of generation and start growing -- a living
        // world is never globally quiet, by design, so "settle, then assert
        // the reload woke things" needs a world whose only activity is the
        // reload's doing.
        let mut app = legacy_app();
        settle(&mut app);
        app.reload_materials();
        assert!(app.world.active_chunk_count() > 0, "reload left the world asleep");
    }

    #[test]
    fn cycling_materials_wraps_in_both_directions() {
        let mut app = test_app();
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
        let mut app = test_app();
        app.select_material(1);
        assert_eq!(app.selected_material(), app.paintable[0]);
        let before = app.selected_material();
        app.select_material(0);
        app.select_material(99);
        assert_eq!(app.selected_material(), before);
    }

    #[test]
    fn brush_radius_is_clamped() {
        let mut app = test_app();
        app.adjust_brush(-1000);
        assert_eq!(app.brush_radius, 1);
        app.adjust_brush(1000);
        assert_eq!(app.brush_radius, 64);
    }

    #[test]
    fn static_terrain_settles_and_stays_settled() {
        // The *static* terrain -- the lifeless legacy layout. The generated
        // default carries living flora (worldgen's life pass germinates
        // under the column-cast light within frames), and a growing plant
        // wakes its chunks on every organism tick, so "stays settled" is a
        // claim only a world without life can make. The mineral fraction of
        // generated worlds arriving at rest is tests/worldgen.rs's job.
        let mut app = legacy_app();
        // Dirty regions are double buffered, so the terrain writes made
        // before the first frame are only promoted into the swept region by
        // the end of it -- the second frame is the earliest that can examine
        // the terrain and conclude it does not move. Generated worlds carry
        // powder as well as stone and take a few frames more; `settle`
        // asserts it still happens promptly.
        settle(&mut app);

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
        let mut app = legacy_app();
        let stone = id(&app, "stone");
        let h = world_bottom(&app) + 1;

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
        // Freshly brushed material is now laid down *intact* as well.
        // Undamaged material is held, which is what a construction is until
        // something happens to it (`Reports/building-rethink.md`). It is a
        // capacity multiplier and not an exemption, so this is not the
        // "everything the player builds is indestructible" failure two
        // inferred-support models had: damage revokes it, and every
        // destructive verb already does.
        // Legacy terrain too, so (40, 40) is open sky: painting into
        // generated rock would assert that pre-existing terrain is attached,
        // which is a different claim that happens to pass.
        let mut probe = legacy_app();
        probe.world.paint_capsule((40, 40), (40, 40), 3, stone, 1.0);
        assert!(probe.world.get(40, 40).attached(), "brushed stone should be laid down intact");

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
        let crumbled = world_cells(&app).any(|(x, y)| app.world.get(x, y).material == debris);
        assert!(!crumbled, "some terrain broke free despite nothing disturbing it");
    }

    #[test]
    fn the_room_tool_leaves_an_interior_you_could_walk_into() {
        // Reported from play: "right now we are just building a solid
        // block, but we will want interiors of an external structure ...
        // like you can go inside." Asserts both halves, because either
        // alone passes against a broken tool: walls all round, and a middle
        // that is genuinely empty. A hollow rectangle that leaked at a
        // corner would also fail the wall checks, which is the failure mode
        // worth catching -- for a structure, a corner gap means the roof is
        // not actually carried by the walls.
        let mut app = test_app();
        let stone = id(&app, "stone");
        app.select_material(app.paintable.iter().position(|&m| m == stone).unwrap() + 1);
        app.cycle_tool(); // Rect
        app.cycle_tool(); // Room
        assert_eq!(app.tool, Tool::Room);

        let (ax, ay, bx, by) = (60, 50, 200, 140);
        app.begin_drag(ax, ay);
        app.end_drag(bx, by, false);

        let (wx0, wy0) = app.renderer.screen_to_world(ax, ay);
        let (wx1, wy1) = app.renderer.screen_to_world(bx, by);
        let (cx, cy) = ((wx0 + wx1) / 2, (wy0 + wy1) / 2);
        let stone_near = |x: i32, y: i32| {
            (-3..=3).flat_map(|dy| (-3..=3).map(move |dx| (dx, dy))).any(|(dx, dy)| app.world.get(x + dx, y + dy).material == stone)
        };

        assert!(stone_near(cx, wy0), "the room has no top wall");
        assert!(stone_near(cx, wy1), "the room has no bottom wall");
        assert!(stone_near(wx0, cy), "the room has no left wall");
        assert!(stone_near(wx1, cy), "the room has no right wall");
        // The interior is the whole point: a fill would pass every wall
        // check above and still be a solid block.
        assert!(!stone_near(cx, cy), "the room is solid -- there is nothing to go inside");
    }

    #[test]
    fn the_reference_room_is_stone_and_stands_on_the_ground() {
        // Three claims, and each one is a way the key could look like it
        // worked while answering nothing.
        //
        // It must be **stone**: the app starts on sand, so a version keyed
        // to the palette would hand a first press a powder room, which
        // reads as a broken feature rather than as a structure to judge.
        //
        // It must **sit on the ground**: a room dropped in mid-air is a
        // much easier structural question than one standing on something,
        // so stamping at the cursor's own height would quietly measure the
        // wrong thing.
        //
        // And it must be **the size it claims**, because the whole point of
        // a reference is that its dimensions are known -- this is the
        // measured edge of the model's envelope, not a round number.
        let mut app = test_app();
        let stone = id(&app, "stone");
        let sand = id(&app, "sand");
        assert_eq!(app.selected_material(), sand, "test setup: the app is expected to start on sand");

        // On the `flat` preset, which is the structural test bed and exists
        // precisely because a 160-tall room needs 200 rows of sky.
        //
        // This used to run on the default preset and passed on a margin of
        // about five cells: only the very deepest columns of `rolling` had
        // the headroom, and the day standing water landed it filled exactly
        // those hollows and the test could not find a column at all. It was
        // never really testing the default world -- it was testing whichever
        // world happened to have one deep enough spot -- so pointing it at
        // the preset built for the job is what it always meant.
        app.worldgen_preset = "flat".to_string();
        app.reset();

        // The surface height is still not something this test may assume --
        // an earlier version hardcoded the middle of the screen and broke the
        // day worldgen became the default, which is the right kind of
        // breakage but not what this test is about. Find a column with the
        // headroom the room actually needs.
        let half = REFERENCE_ROOM_SPAN / 2;
        let need = REFERENCE_ROOM_HEIGHT + app.brush_radius;
        let bounds = app.world.bounds().expect("bounded world");
        let ground_at =
            |app: &App, x: i32| (bounds.min_y..=bounds.max_y).find(|&y| app.world.get(x, y).material != material::EMPTY);
        let cx = (half + app.brush_radius..bounds.max_x - half - app.brush_radius)
            .find(|&x| ground_at(&app, x).is_some_and(|g| g > need))
            .expect("no column in the generated world has room for a reference room");
        let ground = ground_at(&app, cx).expect("ground at the chosen column");

        let (sx, sy) = (cx, 0);
        app.stamp_reference_room(sx, sy);
        let stone_near = |x: i32, y: i32| {
            (-4..=4).flat_map(|dy| (-4..=4).map(move |dx| (dx, dy))).any(|(dx, dy)| app.world.get(x + dx, y + dy).material == stone)
        };

        let mid = ground - 1 - REFERENCE_ROOM_HEIGHT / 2;
        assert!(stone_near(cx - half, mid), "no left wall at the stated span");
        assert!(stone_near(cx + half, mid), "no right wall at the stated span");
        // The roof is the assertion that carries the "always stone" claim
        // as well as the height one: it sits 160 cells above the ground in
        // open air, so stone there cannot have come from the terrain and
        // can only have been stamped. A version keyed to the palette would
        // have put sand there and this would fail.
        //
        // Deliberately *not* a world-wide sand count, which is what the
        // first attempt used and which fails for an honest reason -- the
        // walls displace sand the generator placed, so the count legitimately
        // drops. A metric that moves when the feature is working is worse
        // than no metric.
        assert!(stone_near(cx, ground - 1 - REFERENCE_ROOM_HEIGHT), "no roof at the stated height, or it was not built from stone");
        assert!(!stone_near(cx, mid), "the reference room is solid, not a room");
    }

    #[test]
    fn a_reference_room_with_nowhere_to_stand_is_refused_rather_than_shrunk() {
        // The other half of "the size it claims". Generated terrain puts
        // the surface anywhere, so a hilltop or a world edge can leave less
        // than the room needs -- and a measuring stick that quietly comes
        // out shorter when there is less space is worse than no measuring
        // stick. Asserts the refusal changes *nothing*, not merely that it
        // does not crash.
        let mut app = test_app();
        let stone = id(&app, "stone");
        let count_stone =
            |app: &App| world_cells(app).filter(|&(x, y)| app.world.get(x, y).material == stone).count();

        let before = count_stone(&app);
        // Hard against the left edge: the room's own span cannot fit
        // beside it whatever the terrain is doing.
        app.stamp_reference_room(1, 0);
        assert_eq!(count_stone(&app), before, "a reference room was stamped where its full span does not fit");
        assert!(
            app.toast.as_ref().is_some_and(|(m, _)| m.contains("NO ROOM")),
            "the refusal was silent -- a key that does nothing reads as broken"
        );
    }

    #[test]
    fn the_flat_preset_is_somewhere_a_reference_room_can_actually_be_stamped() {
        // The two halves of the test bed, joined. `tests/worldgen.rs` says
        // the `flat` preset has 200 rows of sky; `stamp_reference_room`
        // refuses when it has less than the room needs. Neither knows about
        // the other, so nothing until now said the combination works -- and
        // "the preset is fine" plus "the key is fine" adding up to a key
        // that refuses everywhere on the preset built for it is exactly the
        // shape of failure this repo keeps shipping.
        let mut app = test_app();
        let stone = id(&app, "stone");
        let count_stone = |app: &App| {
            world_cells(app).filter(|&(x, y)| app.world.get(x, y).material == stone).count()
        };

        // Cycle to `flat` the way F7 does rather than reaching past the
        // key, so a preset renamed or dropped from the cycle fails here.
        let mut guard = 0;
        while app.worldgen_preset != "flat" {
            app.cycle_preset();
            guard += 1;
            assert!(guard < 32, "`flat` is not in the preset cycle -- F7 cannot reach the structural test bed");
        }

        let before = count_stone(&app);
        app.stamp_reference_room(WIDTH as i32 / 2, 0);
        assert!(
            count_stone(&app) > before,
            "B refused on the one preset built for it: {}",
            app.toast.as_ref().map(|(m, _)| m.as_str()).unwrap_or("no toast")
        );
    }

    #[test]
    fn the_stress_view_paints_loaded_rock_and_leaves_empty_space_alone() {
        // Two assertions, because either alone passes against a broken
        // overlay: one that paints nothing would satisfy "the sky is
        // untouched", and one that paints the whole screen would satisfy
        // "the rock changed colour".
        //
        // Also writes the frame out, because `CLAUDE.md` is emphatic that
        // an assertion about pixels is not the same as having looked at
        // them -- and a colour ramp is exactly the kind of thing that
        // passes a numeric test while being unreadable on screen.
        let mut app = test_app();
        let stone = id(&app, "stone");
        // A cantilever off the left cliff: something with a real stress
        // gradient along it rather than a uniform blob.
        for x in 0..90 {
            for y in 120..170 {
                app.world.set(x, y, Cell::new(stone, 0).with_attached(true));
            }
        }
        for x in 90..250 {
            for y in 150..162 {
                app.world.set(x, y, Cell::new(stone, 0));
            }
        }
        crate::sim::structural::compute_world_distances(&mut app.world);

        let mut plain = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        app.draw(&mut plain, None);
        app.toggle_stress_view();
        assert!(app.show_stress);
        let mut tinted = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        app.draw(&mut tinted, None);

        let at = |buf: &[u8], x: i32, y: i32| {
            let i = ((y * WIDTH as i32 + x) * 4) as usize;
            [buf[i], buf[i + 1], buf[i + 2]]
        };
        // The far end of the cantilever carries the most, so if anything is
        // tinted it is this.
        assert_ne!(at(&tinted, 120, 155), at(&plain, 120, 155), "loaded rock was not tinted by the stress view");
        // Open sky well above the structure has no structural cell at all.
        assert_eq!(at(&tinted, 300, 40), at(&plain, 300, 40), "the stress view painted empty space");

        let dir = std::env::temp_dir().join("pixel-physics-stress-view");
        std::fs::create_dir_all(&dir).ok();
        image::save_buffer(dir.join("stress.png"), &tinted, WIDTH, HEIGHT, image::ColorType::Rgba8).ok();
    }

    #[test]
    fn the_rectangle_tool_lays_down_a_filled_rectangle() {
        // Reported from play: "using a paint brush type tool to build is
        // not satisfying." A freehand round brush cannot make a straight
        // wall, and every structure came out wobbly. Asserts the corners
        // and the middle are filled and that the gesture commits on
        // *release*, not on press -- pressing alone must not leave a blob
        // at the corner.
        let mut app = test_app();
        let stone = id(&app, "stone");
        app.select_material(app.paintable.iter().position(|&m| m == stone).unwrap() + 1);
        app.cycle_tool();
        assert_eq!(app.tool, Tool::Rect);

        let (ax, ay) = (60, 60);
        let (bx, by) = (140, 110);
        app.begin_drag(ax, ay);
        let (wx0, wy0) = app.renderer.screen_to_world(ax, ay);
        let (wx1, wy1) = app.renderer.screen_to_world(bx, by);
        assert_ne!(app.world.get(wx0, wy0).material, stone, "pressing alone must not paint anything");

        app.end_drag(bx, by, false);

        // Sampled rather than tested cell by cell: painting rolls a
        // per-cell density, so no single cell is guaranteed.
        let filled = |cx: i32, cy: i32| {
            (-2..=2).flat_map(|dy| (-2..=2).map(move |dx| (dx, dy))).any(|(dx, dy)| app.world.get(cx + dx, cy + dy).material == stone)
        };
        assert!(filled(wx0, wy0), "the rectangle's first corner is empty");
        assert!(filled(wx1, wy1), "the rectangle's opposite corner is empty");
        assert!(filled((wx0 + wx1) / 2, (wy0 + wy1) / 2), "the rectangle's middle is empty -- it drew an outline, not a wall");
        // And the interior really is continuous, which is the whole point
        // of the tool: a row of capsules that missed each other would leave
        // gaps between them.
        for cy in (wy0 + 2)..(wy1 - 2) {
            assert!(filled((wx0 + wx1) / 2, cy), "gap in the rectangle at y={cy} -- the row sweep does not overlap");
        }
        assert!(app.drag_from.is_none(), "the gesture should be finished after release");
    }

    #[test]
    fn painting_adds_material_and_erasing_removes_it() {
        let mut app = test_app();
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
        let mut app = test_app();
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
        let mut app = test_app();
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
        let mut app = test_app();
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
        let mut app = test_app();
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
        let mut app = test_app();
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
        // Legacy terrain: the assertions below name the floor's exact rows.
        let mut app = legacy_app();
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
            app.world.get(10, world_bottom(&app)).material,
            material::BEDROCK,
            "reset lost the bedrock floor"
        );
        assert_eq!(
            app.world.get(10, world_bottom(&app) - 2).material,
            stone,
            "reset lost the stone floor"
        );
        // Reset must not throw away materials loaded from disk.
        assert!(app.world.materials.id_of("gravel").is_some());
    }

    /// **The `F9` setting must survive a world rebuild, because the title
    /// bar goes on claiming it does.**
    ///
    /// `reset` builds a brand-new `World` and `World::new` starts at
    /// `i32::MAX` (SPREAD), while `App::chain_mode` -- which is what the
    /// status line names -- is untouched. `reset` is reached from `F6`
    /// (next seed), `F7` (cycle preset), `F8` (previous seed) and the
    /// worldgen file watcher, which is every way a player gets a fresh world
    /// to test a chain mode in. So every A/B that rerolled the world between
    /// arms was comparing SPREAD with SPREAD while the title bar said
    /// LOCAL -- which is how "LOCAL and TIGHT do not contain anything" came
    /// to be reported.
    #[test]
    fn a_rebuilt_world_keeps_the_chain_mode_the_title_bar_is_claiming() {
        let mut app = App::new();
        app.cycle_chain_mode();
        let chosen = &crate::sim::structural::CHAIN_MODES[app.chain_mode];
        assert_ne!(chosen.reach, i32::MAX, "test setup: one cycle must leave the default, or this proves nothing");
        assert_eq!(app.world.chain_reach, chosen.reach, "test setup: F9 never reached the world in the first place");

        app.reset();

        assert_eq!(
            app.world.chain_reach,
            crate::sim::structural::CHAIN_MODES[app.chain_mode].reach,
            "the world came back at reach {} while the status line still says {}",
            app.world.chain_reach,
            crate::sim::structural::CHAIN_MODES[app.chain_mode].name
        );
    }

    /// **Tightening the setting has to reach the work already in flight.**
    ///
    /// `World::staged_fractures` is ungated by design once a failure has
    /// been judged, so a collapse mid-flight goes on arriving whatever `F9`
    /// now says -- which is exactly the "switching to NONE does nothing"
    /// half of the complaint. The queue here stands in for one mid-collapse;
    /// nothing has been disturbed, so at any reach but SPREAD the licence
    /// covers none of it.
    #[test]
    fn tightening_the_chain_mode_drops_staged_work_the_new_setting_cannot_license() {
        let mut app = App::new();
        app.world.staged_fractures.push_back(crate::sim::structural::StagedFracture {
            region: vec![(100, 100), (101, 100)],
            at: (100, 100),
            next_frame: app.world.frame + 5,
        });

        // **Cycle to SPREAD first, then tighten off it.** This read
        // `app.cycle_chain_mode(); // SPREAD -> LOCAL: a tighten`, which
        // depended on SPREAD being the mode a fresh `App` starts in. It is
        // `TIGHT` now, and one cycle off TIGHT is a *loosen* -- which
        // deliberately does not relicense, so the case failed while
        // testing nothing about tightening. Driven by name rather than by
        // index so it survives the list being reordered again.
        while crate::sim::structural::CHAIN_MODES[app.chain_mode].name != "SPREAD" {
            app.cycle_chain_mode();
        }
        let before = app.world.chain_reach;
        app.cycle_chain_mode();
        assert!(
            app.world.chain_reach < before,
            "test setup: cycling off SPREAD must be a tighten, got {} after {before}",
            app.world.chain_reach
        );

        assert!(
            app.world.staged_fractures.is_empty(),
            "F9 tightened to {} and {} staged fracture(s) survived it -- the aftermath the player is trying to stop keeps arriving from a queue the new setting never sees",
            crate::sim::structural::CHAIN_MODES[app.chain_mode].name,
            app.world.staged_fractures.len()
        );
    }

    #[test]
    fn sand_dropped_on_the_floor_settles_and_the_world_sleeps_again() {
        // Legacy terrain for the same reason as the two settling tests
        // above: this is a claim about *sand* coming to rest, and the
        // generated default's living flora keeps its own chunks awake
        // indefinitely, which would fail the assertion for a reason that
        // has nothing to do with sand.
        let mut app = legacy_app();
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

#[cfg(test)]
mod loading_path {
    use super::tests::{test_app, TEST_WORLD};
    use super::*;

    /// `App` can cross a thread boundary.
    ///
    /// Not a curiosity: `main.rs` generates the world on a worker and moves
    /// the finished `App` back, so an `Rc` or a raw pointer added to any
    /// field would break the loading screen. The failure would be a compile
    /// error in the binary rather than the library, which is a long way from
    /// the field that caused it — this puts it next to the cause.
    #[test]
    fn app_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<App>();
    }

    /// Generating with a progress callback builds the *same world* as
    /// generating without one.
    ///
    /// The loading screen would be worth nothing if it quietly produced a
    /// different world from the one every test and every seed screenshot is
    /// taken against, and `PLAN.md` requires same-build determinism. Compared
    /// by checksum over every cell rather than by spot checks, for the reason
    /// the round-7 field work kept relearning: a world that is subtly
    /// different but still plausible passes every property assertion.
    #[test]
    fn reporting_progress_does_not_change_the_world() {
        fn checksum(app: &App) -> u64 {
            let mut acc: u64 = 0xcbf2_9ce4_8422_2325;
            let bounds = app.world.bounds().expect("the app world has bounds");
            for y in bounds.min_y..=bounds.max_y {
                for x in bounds.min_x..=bounds.max_x {
                    let c = app.world.get(x, y);
                    for byte in [c.material.0 as u64, c.aux() as u64, c.shade as u64] {
                        acc ^= byte;
                        acc = acc.wrapping_mul(0x100_0000_01b3);
                    }
                }
            }
            acc
        }

        let quiet = test_app();
        let mut stages: Vec<&'static str> = Vec::new();
        let reported = App::build(true, TEST_WORLD, &mut |_, name| stages.push(name));

        assert!(!stages.is_empty(), "vacuous: nothing reported a stage");
        assert_eq!(checksum(&quiet), checksum(&reported), "the reported build produced a different world");
    }

    /// The placeholder is empty, and cheap because it is.
    #[test]
    fn a_pending_app_has_not_generated_anything() {
        let pending = App::build(false, TEST_WORLD, &mut |_, _| {});
        let bounds = pending.world.bounds().expect("the app world has bounds");
        let solid = (bounds.min_y..=bounds.max_y)
            .step_by(16)
            .flat_map(|y| (bounds.min_x..=bounds.max_x).step_by(16).map(move |x| (x, y)))
            .filter(|&(x, y)| pending.world.get(x, y).material != crate::sim::material::EMPTY)
            .count();
        assert_eq!(solid, 0, "the placeholder world already has {solid} cells in it");
    }
}
