//! **A saved starting box with a question written on it.**
//!
//! `Reports/lab-behaviour-scenarios-2026-09-06.md` §4 item 1: five of the
//! eight beds that report designs cannot be expressed as a bare `LabBox` at
//! all -- a food heap on a schedule, a raised bank, a species released at a
//! particular column -- because everything the player paints by hand today
//! is not saved. `scene::LabBox` persists and `params::Dials` persists
//! (`LabBox::save`/`load_saved`, `Dials::load_saved`); what a player places
//! after the box is built does not, so a bed like that can be played once
//! and never replicated. This closes that gap: a `Scenario` is a `LabBox`
//! plus what gets placed once at build time, what gets placed on a
//! schedule while the box runs, and which parameters-page knobs it turns
//! before either.
//!
//! Nothing here costs a frame that would not otherwise be spent. Build-time
//! placements happen once, inside `build()`; the running schedule is a
//! `Vec` scan gated on the timeline being non-empty, so a scenario with no
//! timeline costs the tick nothing beyond the `is_empty` check, and one
//! with a short timeline (every shipped scenario has a handful of entries)
//! costs one linear scan of it. See [`Scenario::due`]'s own doc for the
//! measured shape of that cost.
//!
//! # Why placements find their own ground rather than trusting the spec
//!
//! A build-time placement could read `spec.ground_y` the way
//! `LabBox::build_counted` reads it for founders and colonies -- and early
//! drafts of this module did exactly that. It is wrong the moment a
//! scenario raises the terrain: `the_bank.ron` fills a rectangle of soil
//! sixteen rows above `ground_y` and then asks for a beetle "on top", and a
//! beetle placed at `ground_y - 2` would land *inside* that fill, not on
//! it. So every placement that needs "the surface at column x" asks
//! `creature::colony_surface` for it, the same call `found_colony_of`
//! itself uses, cursored from the top of the room rather than from
//! `ground_y` -- which is what makes it see a bank, a trench, or bare rock
//! equally, instead of only the bed it was authored against.

use crate::sim::cell::Cell;
use crate::sim::creature::{self, colony_surface};
use crate::sim::material;
use crate::sim::rng;
use crate::sim::world::World;

use super::params::{self, Knob, Param};
use super::scene::{self, LabBox};

/// Where the shipped scenarios live. Tracked, shipped content, like
/// `assets/species/` -- **not** gitignored the way `LabBox::ASSET_PATH`
/// (one player's current box) and the specimen shelf are. A scenario is
/// authored content the box loads, not session state.
pub const ASSET_DIR: &str = "assets/lab_scenarios";
/// Override for [`ASSET_DIR`], same shape as `specimen::SHELF_DIR_ENV` --
/// a test or a second lab in this container points it elsewhere instead of
/// the shared, tracked directory.
pub const ASSET_DIR_ENV: &str = "PIXEL_PHYSICS_LAB_SCENARIOS";

/// A saved starting box with a question written on it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Scenario {
    /// The file stem this was loaded from. Not read back out of the file
    /// path -- carried in the file itself, so a scenario handed to
    /// `labshot`/`labbatch` as an in-memory value (a future harness
    /// composing one) still knows its own name.
    pub name: String,
    /// Short, uppercase, `<= 20` chars -- the page prints it as a row
    /// label and the bar has no room for a caption that wraps.
    pub title: String,
    /// One sentence: the question this bed asks. The page's hover note.
    pub question: String,
    pub bed: LabBox,
    #[serde(default)]
    pub settings: Vec<Setting>,
    /// Applied once, right after the bed is built -- and again on every
    /// rebuild, because a rebuild is exactly the moment a player expects
    /// the scenario back (`Lab::reset`'s own doc).
    #[serde(default)]
    pub placements: Vec<Placement>,
    /// Applied while the box runs, on the schedule each [`Event`] states.
    #[serde(default)]
    pub timeline: Vec<Event>,
    /// Frames the design says to read this bed at. `0` means unstated --
    /// the report's own horizon rule (nothing under 24,000 frames means
    /// anything in this box) applies, but this scenario is not making a
    /// claim about which multiple of it.
    #[serde(default)]
    pub horizon: u64,
}

/// One parameters-page knob, addressed the way the page labels it: the row's
/// `Tunable::category` and `Tunable::name`, which is also how
/// [`resolve_setting`] looks it up.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Setting {
    pub subject: String,
    pub field: String,
    pub value: f32,
}

/// One entry of a scenario's running schedule.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Event {
    /// First frame this fires on. Must be `>= 1` -- see [`Scenario::due`]'s
    /// timing note; `load` refuses `0` and says why.
    pub at: u64,
    /// `0` is once; otherwise this repeats every `every` frames after `at`.
    #[serde(default)]
    pub every: u64,
    /// `0` is for ever; otherwise the last frame this may fire on.
    #[serde(default)]
    pub until: u64,
    pub what: Placement,
}

/// Something a scenario puts in the box, either once at build time or on a
/// running schedule.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Placement {
    /// Fill a rectangle (world coordinates, `y` from the top like
    /// `LabBox::ground_y`) with a material by name.
    ///
    /// `"soil"` is laid down the way `LabBox::build_counted` lays down the
    /// bed's own soil -- `SOIL_FIELD_CAPACITY` aux, jittered shade -- so a
    /// filled bank is damp enough for a root exactly like the floor it
    /// joins. `"water"` is full (a `Liquid`'s `aux == 0`). Anything else is
    /// stamped at `aux == 0` with a palette shade chosen the way
    /// `World::paint_capsule_as` chooses one, so a filled rectangle of a
    /// multi-family material is not a block of one confetti-free shade.
    Fill { material: String, x: i32, y: i32, w: i32, h: i32 },
    /// Clear a rectangle to [`Cell::EMPTY`].
    Clear { x: i32, y: i32, w: i32, h: i32 },
    /// Pile `cells` cells of a material on the surface at column `x`:
    /// find the surface with [`colony_surface`] and stack upward in a
    /// widening pyramid (row 0: `x`; row 1: `x-1..=x+1`; ...) until `cells`
    /// are placed. A `Powder` settles on its own from there.
    Heap { material: String, x: i32, cells: i32 },
    /// `cells` single cells of a material, spaced evenly along the surface
    /// between `x0` and `x1` inclusive.
    Scatter { material: String, x0: i32, x1: i32, cells: i32 },
    /// One plant of a species at column `x`, on the surface -- mirrors
    /// `Lab::plant_at`: lift up to `MAX_PLANT_LIFT` rows looking for an
    /// empty cell.
    Plant { species: String, x: i32 },
    /// A colony of `count` animals of a species at column `x`, founded the
    /// way `LabBox::build_counted` founds one (`World::found_colony_of`,
    /// two rows above the detected surface).
    Colony { species: String, x: i32, count: i32 },
    /// One animal, no nest, at column `x` -- mirrors `Lab::stock_one`:
    /// `creature::plant_creature_seed` with the lift loop, then
    /// `schedule_active_site`.
    Animal { species: String, x: i32 },
    /// `colonies` colonies of `ants` animals each, spread the way
    /// [`LabBox::colony_columns`] spreads them for the bed itself rather
    /// than at a hand-picked `x` -- so a scenario keeps founding one colony
    /// per compartment under a `compartments` sweep instead of dropping
    /// every colony into whichever room a fixed column happens to fall in.
    Colonies { species: String, colonies: usize, ants: i32 },
    /// `count` predators released round-robin across the bed's
    /// compartments, the same spread [`LabBox::predator_columns`] gives
    /// `build_counted`'s own beetles.
    Predators { species: String, count: usize },
}

/// **"Did it fire at all" needs a counter, not a picture** (`CLAUDE.md`).
/// What [`Scenario::apply`], [`apply_placements`] and [`apply_settings`]
/// actually managed to place, against a scene that may refuse a site
/// exactly the way `LabBox::build_counted`'s own [`scene::Planted`] does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Placed {
    pub cells: usize,
    pub plants: usize,
    pub animals: usize,
    pub settings: usize,
}

impl Scenario {
    pub fn dir() -> std::path::PathBuf {
        std::env::var(ASSET_DIR_ENV).map(std::path::PathBuf::from).unwrap_or_else(|_| ASSET_DIR.into())
    }

    /// Load one scenario by name (the file stem) and validate it.
    ///
    /// Validation is what keeps a bad file a CI failure rather than a
    /// player's click: every [`Setting`] must resolve to exactly one
    /// parameters-page knob, every material and species name in every
    /// placement must exist in the compiled-in registries, and no
    /// [`Event`] may fire at frame 0 (build-time placements already are
    /// frame 0 -- see [`Scenario::due`]).
    pub fn load(name: &str) -> Result<Scenario, String> {
        let path = Self::dir().join(format!("{name}.ron"));
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let scenario: Scenario = ron::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        scenario.validate().map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(scenario)
    }

    /// Every `*.ron` in [`Scenario::dir`] that parses and validates, sorted
    /// by name. A file that fails either check is skipped with its error on
    /// stderr -- never a panic, because one bad hand-edit in the directory
    /// must not take the whole page down.
    pub fn list() -> Vec<Scenario> {
        let dir = Self::dir();
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(&dir) else { return out };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("ron") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
            match Self::load(stem) {
                Ok(s) => out.push(s),
                Err(e) => eprintln!("scenario {stem} failed to load: {e}"),
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// A throwaway world reaching only the compiled-in registries, for
    /// [`Scenario::validate`] to check names against before any bed of this
    /// scenario's own shape has been built. Species and materials load the
    /// same way regardless of world size (`World::new` populates both from
    /// `include_str!`'d assets), so a two-cell world is enough and is never
    /// stepped.
    fn registry_world() -> World {
        World::new(crate::sim::chunk::Rect::new(0, 0, 1, 1))
    }

    fn validate(&self) -> Result<(), String> {
        for e in &self.timeline {
            if e.at == 0 {
                return Err(
                    "an event at frame 0 is refused -- build-time placements are already frame 0 \
                     (Scenario::apply runs before the first tick), and tick_timeline only ever sees \
                     a frame after frame::step has run, so an 'at: 0' event could never fire"
                        .to_string(),
                );
            }
        }
        let world = Self::registry_world();
        for p in self.placements.iter().chain(self.timeline.iter().map(|e| &e.what)) {
            check_placement_names(&world, p)?;
        }
        let plant = world.species.id_of(&self.bed.species);
        let registry = params::registry(&world, &self.bed, plant);
        for s in &self.settings {
            resolve_setting(&registry, &s.subject, &s.field)?;
        }
        Ok(())
    }

    /// Settings first, then placements, in file order -- a setting like the
    /// hunting ground's ant `sight_range` has to be live before the founder
    /// that reads it is placed, and both must land before the box's first
    /// tick sees either.
    pub fn apply(&self, world: &mut World, spec: &mut LabBox) -> Placed {
        let mut placed = Placed::default();
        match apply_settings(world, spec, &self.settings) {
            Ok(n) => placed.settings = n,
            // `load` already validated every setting resolves and writes,
            // so reaching this arm means the world this scenario is being
            // applied to is not the one it was validated against (a
            // species reloaded out from under it, say). Reported rather
            // than panicked, so a bad reload degrades to "the setting did
            // not take" instead of taking the box down.
            Err(e) => eprintln!("scenario '{}': setting failed at apply time: {e}", self.name),
        }
        let p = apply_placements(world, &self.placements, spec);
        placed.cells = p.cells;
        placed.plants = p.plants;
        placed.animals = p.animals;
        placed
    }

    /// Build the bed and apply this scenario to it in one call -- what
    /// `labshot scenario=` and a batch's `Start::Fresh` arm both want.
    pub fn build(&self) -> (World, scene::Planted, Placed) {
        let (mut world, planted) = self.bed.build_counted();
        let mut spec = self.bed.clone();
        let placed = self.apply(&mut world, &mut spec);
        (world, planted, placed)
    }

    /// The placements due on `frame`: `at == frame`, or repeating
    /// (`every > 0 && frame >= at && (frame - at).is_multiple_of(every)`),
    /// and within `until` (`0` is for ever).
    ///
    /// **Cheap, and this is why it can run every tick**: a scenario with no
    /// timeline returns before touching `self.timeline` at all, and one
    /// with entries is a single linear scan doing integer arithmetic per
    /// entry. No allocation happens unless something is actually due.
    pub fn due(&self, frame: u64) -> Vec<Placement> {
        if self.timeline.is_empty() {
            return Vec::new();
        }
        self.timeline
            .iter()
            .filter(|e| {
                let fires = e.at == frame || (e.every > 0 && frame >= e.at && (frame - e.at).is_multiple_of(e.every));
                fires && (e.until == 0 || frame <= e.until)
            })
            .map(|e| e.what.clone())
            .collect()
    }
}

fn check_placement_names(world: &World, p: &Placement) -> Result<(), String> {
    match p {
        Placement::Fill { material, .. } | Placement::Heap { material, .. } | Placement::Scatter { material, .. } => {
            if world.materials.id_of(material).is_none() {
                return Err(format!("unknown material {material:?}"));
            }
        }
        Placement::Clear { .. } => {}
        Placement::Plant { species, .. }
        | Placement::Colony { species, .. }
        | Placement::Animal { species, .. }
        | Placement::Colonies { species, .. }
        | Placement::Predators { species, .. } => {
            if world.species.id_of(species).is_none() {
                return Err(format!("unknown species {species:?}"));
            }
        }
    }
    Ok(())
}

/// Resolve `(subject, field)` against the parameters page's own list, the
/// way the page itself labels a row: `Tunable::category` and
/// `Tunable::name`.
///
/// **There can be two matches for one name**, and it is not a bug in either
/// row: a species carries both an authored field (`Knob::Creature`) and a
/// heritable trait slot drifting from it (`Knob::CreatureTrait`), and both
/// are registered under the same `(species, field)` pair when the field
/// name and the trait name coincide -- `sight_range` is exactly this on the
/// ant. A scenario naming `ant.sight_range` means the species' own eye, the
/// number every founder starts from, not a drift offset from it -- so a
/// species-level match is preferred over a trait slot whenever both appear.
/// Any other multiplicity, or none at all, is refused rather than guessed
/// at.
fn resolve_setting<'a>(registry: &'a [Param], subject: &str, field: &str) -> Result<&'a Param, String> {
    let matches: Vec<&Param> = registry.iter().filter(|p| p.tunable.category == subject && p.tunable.name == field).collect();
    match matches.len() {
        0 => Err(format!("no parameter named {subject}.{field}")),
        1 => Ok(matches[0]),
        _ => {
            let preferred: Vec<&Param> = matches.iter().copied().filter(|p| !matches!(p.knob, Knob::CreatureTrait { .. })).collect();
            match preferred.len() {
                1 => Ok(preferred[0]),
                n => Err(format!(
                    "{subject}.{field} is ambiguous: {} parameters match it, and preferring the species field \
                     over a heritable trait slot leaves {n}",
                    matches.len()
                )),
            }
        }
    }
}

/// Write every [`Setting`] through to the world (or the spec, for a
/// `Knob::Bed` row), resolving each against the parameters page's own
/// current list -- so a setting always means what the page currently shows
/// it meaning, not a cached idea of where it lives.
pub fn apply_settings(world: &mut World, spec: &mut LabBox, settings: &[Setting]) -> Result<usize, String> {
    let mut applied = 0;
    for s in settings {
        let plant = world.species.id_of(&spec.species);
        let registry = params::registry(world, spec, plant);
        let knob = resolve_setting(&registry, &s.subject, &s.field)?.knob.clone();
        if !params::write(world, spec, &knob, s.value) {
            return Err(format!("{}.{} refused {}", s.subject, s.field, s.value));
        }
        applied += 1;
    }
    Ok(applied)
}

/// Apply every [`Placement`] to a running world, in order. `spec` is the
/// bed this scenario was built from -- only [`Placement::Colonies`] and
/// [`Placement::Predators`] read it, to spread across compartments the way
/// `LabBox::build_counted` itself does.
pub fn apply_placements(world: &mut World, placements: &[Placement], spec: &LabBox) -> Placed {
    let mut placed = Placed::default();
    for p in placements {
        match p {
            Placement::Fill { material, x, y, w, h } => placed.cells += apply_fill(world, material, *x, *y, *w, *h),
            Placement::Clear { x, y, w, h } => placed.cells += apply_clear(world, *x, *y, *w, *h),
            Placement::Heap { material, x, cells } => placed.cells += apply_heap(world, material, *x, *cells),
            Placement::Scatter { material, x0, x1, cells } => placed.cells += apply_scatter(world, material, *x0, *x1, *cells),
            Placement::Plant { species, x } => placed.plants += apply_plant(world, species, *x),
            Placement::Colony { species, x, count } => placed.animals += apply_colony(world, species, *x, *count),
            Placement::Animal { species, x } => placed.animals += apply_animal(world, species, *x),
            Placement::Colonies { species, colonies, ants } => placed.animals += apply_colonies(world, spec, species, *colonies, *ants),
            Placement::Predators { species, count } => placed.animals += apply_predators(world, spec, species, *count),
        }
    }
    placed
}

/// Apply whatever this scenario's timeline says is due on `world.frame`.
///
/// **Called immediately after `frame::step` returns.** `frame::step` (via
/// `World::begin_step`) increments `frame` first thing, so by the time this
/// runs, `world.frame` is the tick that just completed -- an `Event` with
/// `at: N` therefore lands in the world after its Nth tick, which is the
/// earliest frame it could possibly appear in. Build-time placements are
/// frame 0, before any tick has run at all, which is why `Scenario::load`
/// refuses `at: 0`: nothing this function does could ever be reached at it.
pub fn tick_timeline(scenario: &Scenario, world: &mut World, spec: &LabBox) -> Placed {
    let due = scenario.due(world.frame);
    apply_placements(world, &due, spec)
}

/// The top of the room, read off the world's own [`crate::sim::enclosure::
/// Enclosure`] rather than passed in -- every lab world declares one
/// (`LabBox::build_counted`'s last act), so this needs no `&LabBox` and
/// therefore works on a bed a `Fill` has since reshaped, not only the one it
/// was authored against. `0` for a world with no enclosure at all, which is
/// not a lab world this ever runs against but must not panic on.
fn room_top(world: &World) -> i32 {
    world.enclosure().map(|e| e.ceiling_y).unwrap_or(0)
}

/// The colour a hand-authored placement should carry when it is not
/// `"soil"` -- a full random byte whose low bits pick the palette entry and
/// whose high bits are the per-cell grain entropy `render::GrainMode::Cell`
/// keys on, exactly [`World::paint_capsule_as`]'s own derivation. Kept in
/// step with that function rather than re-derived independently: two
/// copies of "how a shade is drawn" is how one of them goes stale first.
fn random_shade(world: &mut World, material: crate::sim::material::MaterialId) -> u8 {
    let m = world.materials.get(material);
    let entries = m.palette.len().max(1) as u32;
    let base = m.base_shades.max(1) as u32;
    (world.rng.below(base) + entries * world.rng.below(256 / entries.max(1))) as u8
}

fn apply_fill(world: &mut World, material: &str, x: i32, y: i32, w: i32, h: i32) -> usize {
    let Some(id) = world.materials.id_of(material) else { return 0 };
    let mut placed = 0;
    if material == "soil" {
        // Mirrors `LabBox::build_counted`'s own soil loop exactly, jitter
        // included, so a filled bank is damp enough for a root the same
        // way the floor it joins is -- a fill stamped at `aux == 0` (dry)
        // would be a strip of ground nothing can grow in beside soil that
        // can.
        for cx in x..x + w {
            for cy in y..y + h {
                if !world.in_bounds(cx, cy) {
                    continue;
                }
                world.set(cx, cy, Cell::new(id, (rng::jitter(cx, cy) * 255.0) as u8).with_aux(material::SOIL_FIELD_CAPACITY));
                placed += 1;
            }
        }
        return placed;
    }
    // Water and everything else: `aux == 0` (which is *full* for a
    // `Liquid` -- the opposite convention `Powder` uses, `CLAUDE.md`'s
    // standing gotcha) and a palette shade drawn the way the brush draws
    // one, so a filled rectangle of a multi-family material is not one
    // flat shade repeated.
    for cx in x..x + w {
        for cy in y..y + h {
            if !world.in_bounds(cx, cy) {
                continue;
            }
            let shade = random_shade(world, id);
            world.set(cx, cy, Cell::new(id, shade));
            placed += 1;
        }
    }
    placed
}

fn apply_clear(world: &mut World, x: i32, y: i32, w: i32, h: i32) -> usize {
    let mut placed = 0;
    for cx in x..x + w {
        for cy in y..y + h {
            if !world.in_bounds(cx, cy) {
                continue;
            }
            world.set(cx, cy, Cell::EMPTY);
            placed += 1;
        }
    }
    placed
}

fn apply_heap(world: &mut World, material: &str, x: i32, cells: i32) -> usize {
    let Some(id) = world.materials.id_of(material) else { return 0 };
    if cells <= 0 {
        return 0;
    }
    let top = room_top(world);
    let Some(surface) = colony_surface(world, x, top) else { return 0 };
    let mut placed = 0i32;
    let mut row = 0i32;
    // A widening pyramid, stacked upward from the surface: row 0 is the
    // single column `x`, row 1 is `x-1..=x+1`, and so on. Guaranteed to
    // terminate -- every row places at least one cell -- so no cap is
    // needed beyond the `cells` count itself.
    while placed < cells {
        let y = surface - 1 - row;
        for cx in (x - row)..=(x + row) {
            if placed >= cells {
                break;
            }
            if world.in_bounds(cx, y) {
                let shade = random_shade(world, id);
                world.set(cx, y, Cell::new(id, shade));
            }
            // Counted whether or not the column was in bounds, matching
            // `cells` to "how much of the pile the author asked for" --
            // an off-bed heap is a scene error to notice, not one to
            // silently make bigger by retrying rows for ever.
            placed += 1;
        }
        row += 1;
    }
    placed as usize
}

fn apply_scatter(world: &mut World, material: &str, x0: i32, x1: i32, cells: i32) -> usize {
    let Some(id) = world.materials.id_of(material) else { return 0 };
    if cells <= 0 {
        return 0;
    }
    let top = room_top(world);
    let mut placed = 0;
    for i in 0..cells {
        let x = if cells == 1 { x0 } else { x0 + (x1 - x0) * i / (cells - 1) };
        let Some(surface) = colony_surface(world, x, top) else { continue };
        let shade = random_shade(world, id);
        world.set(x, surface - 1, Cell::new(id, shade));
        placed += 1;
    }
    placed
}

fn apply_plant(world: &mut World, species: &str, x: i32) -> usize {
    let top = room_top(world);
    let Some(surface) = colony_surface(world, x, top) else { return 0 };
    // Mirrors `Lab::plant_at`: start at the ground and lift until an empty
    // cell is found, or give up after `MAX_PLANT_LIFT` rows.
    let mut site = surface;
    for _ in 0..super::MAX_PLANT_LIFT {
        if world.is_empty(x, site) {
            break;
        }
        site -= 1;
    }
    usize::from(world.plant_tree_species(x, site, &species.to_lowercase()))
}

fn apply_colony(world: &mut World, species: &str, x: i32, count: i32) -> usize {
    let top = room_top(world);
    let Some(surface) = colony_surface(world, x, top) else { return 0 };
    // The same two rows of clearance `LabBox::build_counted` founds a
    // colony with, off whatever surface this column actually has rather
    // than off the bed's own `ground_y` -- see this module's own doc for
    // why that distinction matters the moment terrain is not flat.
    world.found_colony_of(x, surface - 2, species, count)
}

fn apply_animal(world: &mut World, species: &str, x: i32) -> usize {
    let top = room_top(world);
    let Some(surface) = colony_surface(world, x, top) else { return 0 };
    // Mirrors `Lab::stock_one`: try at the surface, lifting up to
    // `MAX_PLANT_LIFT` rows if the body does not fit.
    let mut site = surface;
    for _ in 0..super::MAX_PLANT_LIFT {
        if let Some(s) = creature::plant_creature_seed(world, x, site, species) {
            world.schedule_active_site(s);
            return 1;
        }
        site -= 1;
    }
    0
}

/// One colony at every column [`LabBox::colony_columns`] gives `colonies`
/// compartments -- built by handing that method a copy of `spec` with only
/// `colonies` overridden, which is cheaper than re-deriving the spread by
/// hand and cannot drift from what `build_counted` itself would place at
/// this bed's own compartment layout.
fn apply_colonies(world: &mut World, spec: &LabBox, species: &str, colonies: usize, ants: i32) -> usize {
    let spread = LabBox { colonies, ..spec.clone() };
    spread.colony_columns().into_iter().map(|x| apply_colony(world, species, x, ants)).sum()
}

/// `count` predators, one per column of [`LabBox::predator_columns`] at that
/// count -- the same round-robin-across-compartments spread
/// `build_counted` releases its own beetles at.
fn apply_predators(world: &mut World, spec: &LabBox, species: &str, count: usize) -> usize {
    let spread = LabBox { predators: count, ..spec.clone() };
    spread.predator_columns().into_iter().map(|x| apply_animal(world, species, x)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A private scenario directory for the whole test module, same
    /// reasoning as `scene::tests`' `BED_LOCK`: [`ASSET_DIR_ENV`] resolves
    /// through process-global state and `cargo test` runs in parallel.
    static DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn scratch_dir(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("pixel_physics_lab_scenarios_{tag}_{}", std::process::id()))
    }

    fn write_scenario(dir: &std::path::Path, name: &str, body: &Scenario) {
        std::fs::create_dir_all(dir).expect("scratch dir");
        let pretty = ron::ser::PrettyConfig::new().struct_names(false);
        let text = ron::ser::to_string_pretty(body, pretty).expect("scenario serialises");
        std::fs::write(dir.join(format!("{name}.ron")), text).expect("write scenario");
    }

    fn tiny_bed() -> LabBox {
        LabBox { width: 256, height: 192, ground_y: 96, soil_depth: 48, founders: 0, colonies: 0, ..LabBox::default() }
    }

    #[test]
    fn every_shipped_scenario_loads_builds_and_places_what_it_says() {
        // The real, tracked directory -- this is the CI guard the shipped
        // files depend on, not a scratch-dir round trip. Still takes the
        // lock: `ASSET_DIR_ENV` is process-global, and without this a
        // concurrently-running test that points it at a scratch directory
        // (`cargo test` runs test functions on separate threads) makes this
        // one read that scratch directory instead of the real one --
        // measured, not hypothetical: this is exactly what happened before
        // the lock was added here.
        let _guard = DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let scenarios = Scenario::list();
        assert!(!scenarios.is_empty(), "no scenario in {} parsed -- assets/lab_scenarios is either empty or every file is broken", Scenario::dir().display());
        for s in &scenarios {
            let (_world, _planted, placed) = s.build();
            eprintln!(
                "{}: cells={} plants={} animals={} settings={}",
                s.name, placed.cells, placed.plants, placed.animals, placed.settings
            );
            let wants_cells = s.placements.iter().any(|p| matches!(p, Placement::Fill { .. } | Placement::Heap { .. } | Placement::Scatter { .. }));
            let wants_plants = s.placements.iter().any(|p| matches!(p, Placement::Plant { .. }));
            let wants_animals = s.placements.iter().any(|p| matches!(p, Placement::Colony { .. } | Placement::Animal { .. }));
            if wants_cells {
                assert!(placed.cells > 0, "{}: a Fill/Heap/Scatter placement set zero cells", s.name);
            }
            if wants_plants {
                assert!(placed.plants > 0, "{}: a Plant placement germinated nothing", s.name);
            }
            if wants_animals {
                assert!(placed.animals > 0, "{}: a Colony/Animal placement placed nothing", s.name);
            }
            assert_eq!(placed.settings, s.settings.len(), "{}: not every declared setting resolved", s.name);
        }
    }

    /// The guard for the head start item 1 built: a scenario that delays a
    /// colony or a predator onto the timeline must actually have zero
    /// creatures before that arrival and more than zero on the tick it
    /// fires, and for the two scenarios this protects plants for
    /// (`gauses_jar`, `two_larders`) a plant must still be standing when it
    /// does.
    ///
    /// **Checked once per scenario at its earliest such arrival**, not once
    /// per event: a scenario whose timeline places creatures twice
    /// (`gauses_jar`'s beetles follow its ants) cannot read zero before the
    /// *second* arrival, because the first one is already standing there --
    /// that is the feature working, not a gap in the guard. Every later
    /// arrival is still checked for the weaker property that actually holds
    /// generally: the count rises on the frame it is due.
    #[test]
    fn a_timeline_creature_arrival_fires_after_a_head_start() {
        let _guard = DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let scenarios = Scenario::list();
        let mut checked = 0;
        for s in &scenarios {
            let mut arrivals: Vec<u64> = s
                .timeline
                .iter()
                .filter(|e| matches!(e.what, Placement::Colony { .. } | Placement::Colonies { .. } | Placement::Animal { .. } | Placement::Predators { .. }))
                .map(|e| e.at)
                .collect();
            arrivals.sort_unstable();
            arrivals.dedup();
            let Some(&last) = arrivals.last() else { continue };
            checked += 1;
            let first = arrivals[0];
            let mut lab = super::super::Lab::new(s.bed.clone());
            lab.scenario = Some(s.clone());
            lab.reset();
            assert_eq!(lab.world.live_creature_count(), 0, "{}: a creature exists at build time even though its first timeline arrival is at frame {first}", s.name);
            let mut before = lab.world.live_creature_count();
            while lab.world.frame < last {
                lab.tick_for_harness();
                let after = lab.world.live_creature_count();
                if arrivals.contains(&lab.world.frame) {
                    if lab.world.frame == first {
                        assert_eq!(before, 0, "{}: a creature already existed the tick before its first timeline arrival (frame {first})", s.name);
                    }
                    assert!(after > before, "{}: live_creature_count did not rise on its arrival frame {} ({before} -> {after})", s.name, lab.world.frame);
                    if matches!(s.name.as_str(), "gauses_jar" | "two_larders") {
                        let plants = lab.world.live_organism_count() - after;
                        assert!(plants > 0, "{}: no plant was still standing at frame {} for the head start to have protected", s.name, lab.world.frame);
                    }
                }
                before = after;
            }
        }
        assert!(checked > 0, "no shipped scenario has a Colony/Colonies/Animal/Predators event on its timeline -- this guard is not exercising anything");
    }

    #[test]
    fn an_unknown_setting_material_or_species_refuses_to_load() {
        let _guard = DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = scratch_dir("bad_names");
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var(ASSET_DIR_ENV, &dir);

        let bad_setting = Scenario {
            name: "bad_setting".into(),
            title: "BAD SETTING".into(),
            question: "does a nonexistent knob refuse".into(),
            bed: tiny_bed(),
            settings: vec![Setting { subject: "ant".into(), field: "not_a_real_field".into(), value: 1.0 }],
            placements: Vec::new(),
            timeline: Vec::new(),
            horizon: 0,
        };
        write_scenario(&dir, "bad_setting", &bad_setting);
        assert!(Scenario::load("bad_setting").is_err(), "an unknown setting field must refuse to load");

        let bad_material = Scenario {
            name: "bad_material".into(),
            title: "BAD MATERIAL".into(),
            question: "does a nonexistent material refuse".into(),
            bed: tiny_bed(),
            settings: Vec::new(),
            placements: vec![Placement::Heap { material: "unobtainium".into(), x: 40, cells: 4 }],
            timeline: Vec::new(),
            horizon: 0,
        };
        write_scenario(&dir, "bad_material", &bad_material);
        assert!(Scenario::load("bad_material").is_err(), "an unknown material must refuse to load");

        let bad_species = Scenario {
            name: "bad_species".into(),
            title: "BAD SPECIES".into(),
            question: "does a nonexistent species refuse".into(),
            bed: tiny_bed(),
            settings: Vec::new(),
            placements: vec![Placement::Animal { species: "dragon".into(), x: 40 }],
            timeline: Vec::new(),
            horizon: 0,
        };
        write_scenario(&dir, "bad_species", &bad_species);
        assert!(Scenario::load("bad_species").is_err(), "an unknown species must refuse to load");

        let _ = std::fs::remove_dir_all(&dir);
        std::env::remove_var(ASSET_DIR_ENV);
    }

    #[test]
    fn an_event_at_frame_zero_refuses_to_load() {
        let _guard = DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = scratch_dir("frame_zero");
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var(ASSET_DIR_ENV, &dir);

        let s = Scenario {
            name: "frame_zero".into(),
            title: "FRAME ZERO".into(),
            question: "does an at:0 event refuse".into(),
            bed: tiny_bed(),
            settings: Vec::new(),
            placements: Vec::new(),
            timeline: vec![Event { at: 0, every: 0, until: 0, what: Placement::Heap { material: "windfall".into(), x: 40, cells: 4 } }],
            horizon: 0,
        };
        write_scenario(&dir, "frame_zero", &s);
        assert!(Scenario::load("frame_zero").is_err(), "an event at frame 0 must refuse to load");

        let _ = std::fs::remove_dir_all(&dir);
        std::env::remove_var(ASSET_DIR_ENV);
    }

    #[test]
    fn the_timeline_fires_on_its_frame_and_not_before() {
        let s = Scenario {
            name: "timeline_probe".into(),
            title: "TIMELINE PROBE".into(),
            question: "does an event fire exactly on its own schedule".into(),
            bed: tiny_bed(),
            settings: Vec::new(),
            placements: Vec::new(),
            timeline: vec![
                Event { at: 5, every: 0, until: 0, what: Placement::Heap { material: "windfall".into(), x: 40, cells: 1 } },
                Event { at: 4, every: 3, until: 10, what: Placement::Heap { material: "windfall".into(), x: 60, cells: 1 } },
            ],
            horizon: 0,
        };
        let mut lab = super::super::Lab::new(s.bed.clone());
        // The row each heap lands on, fixed **before** anything is placed.
        // Re-deriving it from `colony_surface` after a placement would find
        // the just-placed windfall itself as the new "surface" -- a Powder
        // is not passable either -- and read one row too high from then on.
        let top = room_top(&lab.world);
        let y40 = colony_surface(&lab.world, 40, top).expect("a surface exists") - 1;
        let y60 = colony_surface(&lab.world, 60, top).expect("a surface exists") - 1;
        let windfall = lab.world.materials.id_of("windfall").expect("windfall is compiled in");
        lab.scenario = Some(s.clone());
        // Build-time placements only -- there are none here -- then drive
        // the clock by hand and watch exactly which frames the timeline
        // fires on.
        let mut once_fired = Vec::new();
        let mut repeat_fired = Vec::new();
        for _ in 0..12 {
            lab.tick_for_harness();
            if lab.world.get(40, y40).material == windfall {
                once_fired.push(lab.world.frame);
                // Consumed so the next frame's read is not seeing a stale
                // pile from an earlier tick.
                lab.world.set(40, y40, Cell::EMPTY);
            }
            if lab.world.get(60, y60).material == windfall {
                repeat_fired.push(lab.world.frame);
                lab.world.set(60, y60, Cell::EMPTY);
            }
        }
        assert_eq!(once_fired, vec![5], "the one-shot event fired on {once_fired:?}, not exactly frame 5");
        assert_eq!(repeat_fired, vec![4, 7, 10], "the repeating event fired on {repeat_fired:?}, not 4,7,10 (until=10 excludes 13)");
    }

    #[test]
    fn a_scenario_survives_rebuild() {
        let s = Scenario {
            name: "rebuild_probe".into(),
            title: "REBUILD PROBE".into(),
            question: "do placements and settings come back after REBUILD".into(),
            bed: LabBox { compartments: 1, founders: 0, colonies: 0, ..tiny_bed() },
            settings: vec![Setting { subject: "ant".into(), field: "sight_range".into(), value: 24.0 }],
            placements: vec![Placement::Colony { species: "ant".into(), x: 60, count: 8 }],
            timeline: Vec::new(),
            horizon: 0,
        };
        let mut lab = super::super::Lab::new(s.bed.clone());
        lab.scenario = Some(s);
        lab.reset();
        for _ in 0..50 {
            lab.tick_for_harness();
        }
        lab.reset();
        assert!(lab.world.live_organism_count() > 0, "the colony did not come back after rebuild");
        let sight = lab
            .world
            .species
            .id_of("ant")
            .and_then(|id| lab.world.species.get(id).creature.clone())
            .map(|d| d.sight_range)
            .expect("the ant has a creature block");
        assert_eq!(sight, 24, "the sight_range setting did not survive a rebuild");
    }

    #[test]
    fn the_box_page_reaches_the_scenarios_page_and_a_row_loads_one() {
        let _guard = DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = scratch_dir("reach_page");
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var(ASSET_DIR_ENV, &dir);
        let s = Scenario {
            name: "reach_page".into(),
            title: "REACH PAGE".into(),
            question: "does a row on the scenarios page load its scenario".into(),
            bed: LabBox { compartments: 1, founders: 0, colonies: 0, ..tiny_bed() },
            settings: Vec::new(),
            placements: vec![Placement::Colony { species: "ant".into(), x: 60, count: 6 }],
            timeline: Vec::new(),
            horizon: 12_345,
        };
        write_scenario(&dir, "reach_page", &s);

        let mut lab = super::super::Lab::new(LabBox::default());
        lab.ui.reload_shelf();
        lab.act(super::super::ui::Action::Panel(super::super::ui::Panel::Box));
        lab.act(super::super::ui::Action::Panel(super::super::ui::Panel::Scenarios));
        assert_eq!(lab.ui.panel, Some(super::super::ui::Panel::Scenarios), "the box page's SCENARIOS row did not open the scenarios page");
        let i = lab.ui.scenarios().iter().position(|sc| sc.name == "reach_page").expect("the scratch scenario is listed");
        lab.act(super::super::ui::Action::ScenarioLoad(i));
        assert_eq!(lab.ui.panel, None, "loading a scenario must close the page it was opened from");
        assert!(lab.world.live_organism_count() > 0, "the row's scenario was not actually loaded");
        assert_eq!(lab.batch_spec.frames, 12_345, "the row's horizon did not reach the batch dial");

        let _ = std::fs::remove_dir_all(&dir);
        std::env::remove_var(ASSET_DIR_ENV);
    }

    #[test]
    fn a_batch_run_carries_the_scenario() {
        // A material no default bed contains, so its presence in the
        // finished world can only be the scenario's own placement.
        let s = Scenario {
            name: "batch_probe".into(),
            title: "BATCH PROBE".into(),
            question: "does a batch run apply its scenario's placements".into(),
            bed: LabBox { compartments: 1, founders: 0, colonies: 0, ..tiny_bed() },
            settings: Vec::new(),
            placements: vec![Placement::Fill { material: "water".into(), x: 40, y: 90, w: 4, h: 2 }],
            timeline: Vec::new(),
            horizon: 0,
        };
        // `frames: 0` -- the point is that the placement reaches the
        // batch's world at all, not that a `Liquid` in open air survives a
        // few ticks of gravity in exactly the rectangle it was filled in.
        let spec = super::super::batch::BatchSpec {
            base: s.bed.clone(),
            replicates: 2,
            sweep: None,
            frames: 0,
            seed0: 1,
            keep_bytes: u64::MAX,
            scenario: Some(s.clone()),
        };
        let runs = spec.runs();
        let batch = super::super::batch::Batch::start_runs(runs, spec.frames, spec.keep_bytes);
        let mut landed = 0;
        while landed < 2 {
            for r in batch.drain() {
                let world = r.world.expect("kept for a small batch");
                let water = world.materials.id_of("water").expect("water is compiled in");
                let found = (40..44).any(|x| (90..92).any(|y| world.get(x, y).material == water));
                assert!(found, "the scenario's Fill(water) placement did not reach the batch's world");
                landed += 1;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // And a sweep over `compartments` on the same scenario still varies
        // the spec, exactly as it does with no scenario attached.
        let swept = super::super::batch::BatchSpec {
            sweep: Some(super::super::batch::Sweep { field: "compartments".into(), values: vec![1.0, 4.0] }),
            replicates: 1,
            ..spec
        };
        let plans = swept.runs();
        assert_eq!(plans.len(), 2, "one replicate at each of two swept values");
        assert_ne!(plans[0].spec.compartments, plans[1].spec.compartments, "the sweep did not vary compartments with a scenario attached");
    }

    #[test]
    fn the_hunting_ground_ant_can_see_a_beetle() {
        // The scenario's own claim, checked against the real sense rather
        // than a hand-rolled distance test: `creature::sighted` plus
        // `sight_range_of` is exactly what `sense()` calls to fill
        // `BrainInput::PreyNear`, so reproducing its one-line formula here
        // is reading the production pathway, not a proxy for it.
        // Locked only for the load: `ASSET_DIR_ENV` is process-global, and a
        // concurrently-running test that points it at a scratch directory
        // would otherwise make this read from there instead of the real
        // `assets/lab_scenarios`. Released immediately after -- the 300-tick
        // simulation below touches no shared state.
        let s = {
            let _guard = DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            Scenario::load("hunting_ground").expect("the shipped hunting_ground scenario loads")
        };
        let (mut world, _planted, _placed) = s.build();
        let ant_species = world.species.id_of("ant").expect("ant is compiled in");
        let Some(def) = world.species.get(ant_species).creature.clone() else {
            panic!("the ant has no creature block");
        };
        // Run enough ticks that a forager and a patrolling beetle actually
        // cross paths at least once -- a fresh-frame read could catch every
        // ant still standing where it was founded.
        let tuning = crate::sim::player::Tuning::default();
        let mut particles = crate::sim::particle::ParticleSystem::new();
        let mut blasts = crate::sim::explosion::Blasts::new();
        let mut best_prey_near = 0.0f32;
        for _ in 0..300 {
            crate::sim::frame::step(&mut world, &mut particles, &mut blasts, crate::sim::player::PlayerInput::default(), &tuning);
            for id in world.live_organism_ids() {
                let Some(state) = world.organism(id) else { continue };
                if state.species != ant_species {
                    continue;
                }
                let reach = creature::sight_range_of(&def, &state.traits);
                let Some(&(hx, hy)) = state.chain.first() else { continue };
                let (seen, _reads) = creature::sighted(&world, hx, hy, id, &def);
                if let Some(prey) = seen.prey {
                    if reach > 0 {
                        best_prey_near = best_prey_near.max((1.0 - prey.dist / reach as f32).clamp(0.0, 1.0));
                    }
                }
            }
            if best_prey_near > 0.0 {
                break;
            }
        }
        assert!(
            best_prey_near > 0.0,
            "no ant ever read a non-zero PreyNear in this scenario -- the setting or the placements are not doing what the file claims"
        );
    }

}
