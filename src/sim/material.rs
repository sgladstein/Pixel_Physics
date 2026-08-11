//! Material definitions, loaded from `assets/materials/*.ron`.
//!
//! Behaviour comes from `kind` plus numeric parameters, so adding a material
//! never means adding a branch to the update loop. The files are the single
//! source of truth: they are embedded in the binary as defaults, and re-read
//! from disk on hot reload.
//!
//! Only vacuum and the out-of-bounds wall are built in, because the engine
//! itself refers to them.

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

use super::rng;

/// Index into `MaterialRegistry`. Stored in every cell, so it must stay small.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct MaterialId(pub u16);

pub const EMPTY: MaterialId = MaterialId(0);
pub const BEDROCK: MaterialId = MaterialId(1);

/// Well-known ids for the shipped materials.
///
/// These are stable because `builtin` always runs first and assigns ids in
/// `EMBEDDED` order, and `reload` only ever updates a material in place or
/// appends a new one — it never reassigns. They exist for the convenience of
/// engine code and tests; anything data-driven should use [`MaterialRegistry::id_of`]
/// instead. `well_known_ids_match_their_names` guards the correspondence.
pub const STONE: MaterialId = MaterialId(2);
pub const SAND: MaterialId = MaterialId(3);
pub const GRAVEL: MaterialId = MaterialId(4);
pub const ASH: MaterialId = MaterialId(5);
pub const WATER: MaterialId = MaterialId(6);
pub const OIL: MaterialId = MaterialId(7);
pub const SMOKE: MaterialId = MaterialId(8);

/// Determines which movement rule a cell obeys. Everything else is parameters.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
pub enum MaterialKind {
    /// Vacuum. Anything may move into it.
    Empty,
    /// Never moves and is never displaced.
    Solid,
    /// Falls, and piles up at its angle of repose. Sand, gravel, ash.
    Powder,
    /// Falls, and spreads sideways to find its level. Water, oil.
    Liquid,
    /// Rises, and spreads to fill space. Smoke, steam.
    Gas,
}

impl MaterialKind {
    /// Whether another material can displace this one by swapping with it.
    /// Solids anchor the world; empty space is moved into rather than swapped.
    pub fn is_displaceable(self) -> bool {
        matches!(self, MaterialKind::Liquid | MaterialKind::Gas)
    }

    /// Whether the update loop needs to consider this cell at all.
    pub fn is_mobile(self) -> bool {
        matches!(
            self,
            MaterialKind::Powder | MaterialKind::Liquid | MaterialKind::Gas
        )
    }
}

/// One `.ron` file. Everything optional has a sensible default so a file only
/// has to state what makes its material distinctive.
#[derive(Deserialize)]
pub struct MaterialDef {
    pub name: String,
    /// Shown in the picker. Left out, the name is title-cased — an `Option`
    /// here would force every file to write `display: Some("...")`.
    #[serde(default)]
    pub display: String,
    pub kind: MaterialKind,
    pub density: f32,
    /// Angle of repose in degrees, for powders. 45 is the steepest a pile can
    /// hold; lower values spread flatter. Ignored by other kinds.
    #[serde(default = "default_friction_angle")]
    pub friction_angle: f32,
    /// How far a liquid or gas may travel sideways in one step.
    #[serde(default)]
    pub dispersion: u8,
    pub colors: Vec<[u8; 3]>,

    // --- M14: heat, combustion, phase change and reactions -----------------
    //
    // All temperatures are Celsius, the same unit `Cell::temperature` and
    // `FieldCell::temperature` already use — one unit throughout, no
    // conversion anywhere a temperature crosses a boundary.
    /// Chance to ignite, checked once per burning neighbour per step. 0 means
    /// never catches fire. Kept in the same 0..1 probability space as
    /// `Rng::chance` rather than TPT's 0..1000 integer scale — nothing else
    /// in this engine uses an integer probability, and matching the rest of
    /// the codebase mattered more here than matching the reference exactly.
    #[serde(default)]
    pub flammability: f32,
    /// Temperature at which this material can catch fire on its own, without
    /// a burning neighbour — contact with lava, standing in a fire's heat.
    /// Defaults effectively to "never" so a material that only defines
    /// `flammability` still needs a neighbour actually on fire to ignite it,
    /// rather than spontaneously combusting the moment any warm cell touches it.
    #[serde(default = "default_never")]
    pub ignition_temperature: f32,
    /// Temperature this material radiates while it itself is burning. Feeds
    /// both its own cell temperature and the M13 ambient field once M14 wires
    /// the coupling in.
    #[serde(default = "default_never")]
    pub burn_temperature: f32,
    /// How long this material burns for, in frames, once ignited. Stored as
    /// the countdown in `Cell::ignite`.
    #[serde(default)]
    pub burn_duration: u16,
    /// Per-cell heat diffusion rate to CA neighbours. Must stay under the 2D
    /// explicit-diffusion stability bound (Fourier number ≤ 0.25) — this is
    /// the CA-resolution equivalent of `field::HEAT_DIFFUSION_RATE`, and
    /// needs the same margin the field grid keeps, not more, since the fine
    /// grid has no spare stability budget to give up.
    #[serde(default = "default_heat_conductivity")]
    pub heat_conductivity: f32,
    #[serde(default = "default_never")]
    pub melting_point: f32,
    /// What this material becomes above `melting_point`, or empty for "does
    /// not melt" — same `String`-not-`Option<String>` choice as `display`
    /// above, for the same reason: RON requires an explicit `Some(...)`
    /// wrapper for a present `Option` value (`melts_into: "ash"` alone is a
    /// parse error, `Some("ash")` is required), which is friction with no
    /// payoff here. An empty string is not a valid material name regardless,
    /// so it is an unambiguous "unset". `melting_point` and `melts_into` stay
    /// independent fields precisely so a `.ron` file can set one without the
    /// other while iterating on content without it silently doing something.
    #[serde(default)]
    pub melts_into: String,
    #[serde(default = "default_never")]
    pub boiling_point: f32,
    #[serde(default)]
    pub boils_into: String,
    /// Pairwise reactions with a specific other material — water quenching
    /// lava into stone and steam, that kind of thing. Order matters: `self`
    /// becomes `produces.0`, `with` becomes `produces.1`.
    #[serde(default)]
    pub reactions: Vec<ReactionDef>,
}

fn default_friction_angle() -> f32 {
    45.0
}

/// Effectively unreachable under normal gameplay temperatures, so a threshold
/// left unset in a `.ron` file never fires rather than firing at 0.0 — the
/// field default a bare `f32` would otherwise get, which would make every
/// material "ignite" and "melt" the instant it's created.
fn default_never() -> f32 {
    f32::INFINITY
}

/// A moderate default within the stability margin, so a material that
/// defines flammability without also tuning conductivity by hand still
/// conducts heat plausibly rather than being a perfect insulator by accident.
fn default_heat_conductivity() -> f32 {
    0.15
}

#[derive(Deserialize, Clone)]
pub struct ReactionDef {
    pub with: String,
    pub produces: (String, String),
    #[serde(default = "default_reaction_chance")]
    pub chance: f32,
}

fn default_reaction_chance() -> f32 {
    1.0
}

pub struct Material {
    pub name: String,
    pub display: String,
    pub kind: MaterialKind,
    /// Drives displacement: a denser material sinks through a lighter one.
    pub density: f32,
    pub friction_angle: f32,
    /// Derived from `friction_angle`. How far a grain looks along a slope for
    /// somewhere to fall before it settles — see `roll_reach`.
    roll_reach_base: f32,
    pub dispersion: u8,
    /// Per-cell colour variation. A cell picks one entry when it is created and
    /// keeps it, which gives bulk material visible grain instead of a flat slab.
    pub palette: Vec<[u8; 4]>,

    pub flammability: f32,
    pub ignition_temperature: f32,
    pub burn_temperature: f32,
    pub burn_duration: u16,
    pub heat_conductivity: f32,
    pub melting_point: f32,
    pub boiling_point: f32,

    // Names as written in the `.ron` file (empty = unset), kept so
    // `MaterialRegistry::resolve_references` can look them up once every
    // material in the registry is known. Resolution can't happen inside
    // `From<MaterialDef>` because a material may reference one that hasn't
    // been parsed yet in this same batch, or one from an earlier load this
    // reload never touches — either way, `Material::from` alone never has
    // the full set to check against.
    melts_into_name: String,
    boils_into_name: String,
    reactions_raw: Vec<ReactionDef>,

    /// Resolved by `resolve_references`. Unset (or naming something that
    /// doesn't exist) both read as "this doesn't happen" — a dangling
    /// reference is a quiet no-op, not a hard error, so a typo during content
    /// iteration costs a missing effect rather than breaking the whole
    /// registry load the way a malformed `.ron` file does.
    pub melts_into: Option<MaterialId>,
    pub boils_into: Option<MaterialId>,
    pub reactions: Vec<Reaction>,
}

/// A resolved pairwise reaction: `self` becomes `becomes`, the other material
/// (`with`) becomes `other_becomes`.
#[derive(Clone, Copy)]
pub struct Reaction {
    pub with: MaterialId,
    pub becomes: MaterialId,
    pub other_becomes: MaterialId,
    pub chance: f32,
}

impl Material {
    /// How many cells a grain at this position may roll along the surface.
    ///
    /// On a slope that drops one cell every `w` columns, the nearest place a
    /// surface grain could fall is `w + 1` columns away — the first column past
    /// the end of the row below it. So a grain keeps rolling while `reach > w`,
    /// and the pile comes to rest at `w = reach`, a slope of `1 / reach`.
    /// Inverting gives `reach = 1 / tan(angle)`.
    ///
    /// Because reach is a whole number of cells it can only express certain
    /// angles exactly — 1 gives 45 degrees, 2 gives 26.6, 3 gives 18.4. The
    /// fractional part is spent by giving *some positions* the longer reach, so
    /// a 34 degree material rolls one cell across about half the world and not
    /// at all across the rest. That averages to the right angle and leaves the
    /// surface pleasantly irregular rather than a perfectly straight wedge.
    ///
    /// Keyed on position rather than drawn fresh each call, so that a grain
    /// which cannot roll now can never roll from here. Re-rolling per call
    /// would let a chunk fall asleep on a frame the dice said no, freezing
    /// grains that should have kept moving.
    pub fn roll_reach_at(&self, x: i32, y: i32) -> i32 {
        let base = self.roll_reach_base.floor();
        let extra = if rng::jitter(x, y) < self.roll_reach_base - base {
            1
        } else {
            0
        };
        base as i32 + extra
    }
}

impl From<MaterialDef> for Material {
    fn from(def: MaterialDef) -> Self {
        let angle = def.friction_angle.clamp(1.0, 89.0);
        // A pile rests at slope `1 / reach`, so the reach for a target angle is
        // `1 / tan(angle)`. Capped at `MAX_REACH`, which is how far the sweep
        // region is widened — reading beyond it would leave grains stale — so
        // the flattest pile the engine can express is `atan(1 / MAX_REACH)`.
        let roll_reach_base =
            (1.0 / angle.to_radians().tan()).clamp(0.0, super::chunk::MAX_REACH as f32);

        Self {
            display: if def.display.is_empty() {
                title_case(&def.name)
            } else {
                def.display
            },
            name: def.name,
            kind: def.kind,
            density: def.density,
            friction_angle: angle,
            roll_reach_base,
            dispersion: def.dispersion,
            palette: def
                .colors
                .iter()
                .map(|c| [c[0], c[1], c[2], 255])
                .collect(),

            flammability: def.flammability,
            ignition_temperature: def.ignition_temperature,
            burn_temperature: def.burn_temperature,
            burn_duration: def.burn_duration,
            heat_conductivity: def.heat_conductivity,
            melting_point: def.melting_point,
            boiling_point: def.boiling_point,
            melts_into_name: def.melts_into,
            boils_into_name: def.boils_into,
            reactions_raw: def.reactions,
            // Left unresolved until `resolve_references` runs.
            melts_into: None,
            boils_into: None,
            reactions: Vec::new(),
        }
    }
}

fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The material files, compiled in so the engine always has a working set even
/// when run without its assets directory beside it. Hot reload re-reads the
/// same files from disk.
const EMBEDDED: &[&str] = &[
    include_str!("../../assets/materials/stone.ron"),
    include_str!("../../assets/materials/sand.ron"),
    include_str!("../../assets/materials/gravel.ron"),
    include_str!("../../assets/materials/ash.ron"),
    include_str!("../../assets/materials/water.ron"),
    include_str!("../../assets/materials/oil.ron"),
    include_str!("../../assets/materials/smoke.ron"),
];

/// Where the loader looks for material files, relative to the working directory.
pub const ASSET_DIR: &str = "assets/materials";

#[derive(Debug)]
pub enum MaterialError {
    Io(std::io::Error),
    /// Which file failed, and why. The path matters: on hot reload this is the
    /// only feedback for a typo.
    Parse { file: String, error: String },
}

impl fmt::Display for MaterialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MaterialError::Io(e) => write!(f, "reading materials: {e}"),
            MaterialError::Parse { file, error } => write!(f, "{file}: {error}"),
        }
    }
}

impl std::error::Error for MaterialError {}

pub struct MaterialRegistry {
    materials: Vec<Material>,
    by_name: HashMap<String, MaterialId>,
}

impl MaterialRegistry {
    /// Vacuum and the out-of-bounds wall, which are engine concepts rather than
    /// content, so they are not loadable and always hold ids 0 and 1.
    fn base() -> Self {
        let mut reg = Self {
            materials: Vec::new(),
            by_name: HashMap::new(),
        };
        reg.insert(Material::from(MaterialDef {
            name: "empty".into(),
            display: String::new(),
            kind: MaterialKind::Empty,
            density: 0.0,
            friction_angle: 45.0,
            dispersion: 0,
            colors: vec![[0, 0, 0]],
            flammability: 0.0,
            ignition_temperature: f32::INFINITY,
            burn_temperature: f32::INFINITY,
            burn_duration: 0,
            heat_conductivity: 0.0,
            melting_point: f32::INFINITY,
            melts_into: String::new(),
            boiling_point: f32::INFINITY,
            boils_into: String::new(),
            reactions: Vec::new(),
        }));
        reg.insert(Material::from(MaterialDef {
            name: "bedrock".into(),
            display: String::new(),
            kind: MaterialKind::Solid,
            density: f32::INFINITY,
            friction_angle: 45.0,
            dispersion: 0,
            colors: vec![[20, 20, 24]],
            flammability: 0.0,
            ignition_temperature: f32::INFINITY,
            burn_temperature: f32::INFINITY,
            burn_duration: 0,
            heat_conductivity: 0.0,
            melting_point: f32::INFINITY,
            melts_into: String::new(),
            boiling_point: f32::INFINITY,
            boils_into: String::new(),
            reactions: Vec::new(),
        }));
        reg
    }

    /// The compiled-in material set. Always succeeds.
    pub fn builtin() -> Self {
        let mut reg = Self::base();
        for (i, source) in EMBEDDED.iter().enumerate() {
            match ron::from_str::<MaterialDef>(source) {
                Ok(def) => reg.upsert(def),
                // Embedded files are validated by a test, so this is
                // unreachable in practice.
                Err(e) => panic!("embedded material {i} is malformed: {e}"),
            }
        }
        reg.resolve_references();
        reg
    }

    /// Read every `.ron` in `dir`, falling back to the compiled-in set on
    /// failure so a missing or broken assets directory cannot stop the engine.
    pub fn load(dir: impl AsRef<Path>) -> (Self, Option<MaterialError>) {
        let mut reg = Self::builtin();
        match reg.reload(dir) {
            Ok(_) => (reg, None),
            Err(e) => (MaterialRegistry::builtin(), Some(e)),
        }
    }

    /// Re-read `dir` over the current set, returning how many materials were
    /// applied.
    ///
    /// Ids are keyed by name and never reassigned, so cells already in the
    /// world keep their material across a reload. Editing a file changes how
    /// existing cells behave and look, which is the point; renaming one adds a
    /// material rather than changing an existing one.
    pub fn reload(&mut self, dir: impl AsRef<Path>) -> Result<usize, MaterialError> {
        let mut paths: Vec<_> = std::fs::read_dir(dir.as_ref())
            .map_err(MaterialError::Io)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "ron"))
            .collect();
        // Sorted so ids are assigned deterministically on a first load.
        paths.sort();

        let mut defs = Vec::new();
        for path in &paths {
            let source = std::fs::read_to_string(path).map_err(MaterialError::Io)?;
            // Plenty of Windows editors save UTF-8 with a byte order mark, and
            // RON rejects it as a stray character at 1:1. Silently tolerating
            // it beats making everyone discover that the hard way.
            let source = source.strip_prefix('\u{feff}').unwrap_or(&source);
            let def = ron::from_str::<MaterialDef>(source).map_err(|e| MaterialError::Parse {
                file: path.file_name().unwrap_or_default().to_string_lossy().into(),
                error: e.to_string(),
            })?;
            defs.push(def);
        }

        // Everything parsed, so nothing is half-applied on a typo.
        let count = defs.len();
        for def in defs {
            self.upsert(def);
        }
        // Re-resolve the whole registry, not just what this call touched: a
        // material reloaded here might newly satisfy a reference some other,
        // untouched material has been carrying unresolved since it was
        // loaded — melting_point/melts_into can be added to two files in
        // either order and the game should not care which.
        self.resolve_references();
        Ok(count)
    }

    /// Look up every material's `melts_into`/`boils_into`/`reactions` name
    /// references against the registry as it now stands, and fill in the
    /// resolved `MaterialId` fields. Call after every batch of upserts —
    /// never partway through one, since a material later in the batch might
    /// be what an earlier one references.
    fn resolve_references(&mut self) {
        // Snapshotted rather than looked up live during the loop below,
        // since resolving needs `&self.by_name` at the same time the loop
        // mutably borrows `self.materials` — cheap, a few dozen entries.
        let by_name = self.by_name.clone();
        let resolve = |name: &str| by_name.get(name).copied();

        let resolve_if_set = |name: &str| if name.is_empty() { None } else { resolve(name) };

        for material in &mut self.materials {
            material.melts_into = resolve_if_set(&material.melts_into_name);
            material.boils_into = resolve_if_set(&material.boils_into_name);
            material.reactions = material
                .reactions_raw
                .iter()
                .filter_map(|r| {
                    Some(Reaction {
                        with: resolve(&r.with)?,
                        becomes: resolve(&r.produces.0)?,
                        other_becomes: resolve(&r.produces.1)?,
                        chance: r.chance,
                    })
                })
                .collect();
        }
    }

    /// Replace the material of the same name in place, or append a new one.
    fn upsert(&mut self, def: MaterialDef) {
        let material = Material::from(def);
        match self.by_name.get(&material.name) {
            Some(id) => self.materials[id.0 as usize] = material,
            None => self.insert(material),
        }
    }

    fn insert(&mut self, material: Material) {
        let id = MaterialId(self.materials.len() as u16);
        self.by_name.insert(material.name.clone(), id);
        self.materials.push(material);
    }

    #[inline]
    pub fn get(&self, id: MaterialId) -> &Material {
        // Ids only originate from this registry, and are never reassigned.
        &self.materials[id.0 as usize]
    }

    #[inline]
    pub fn kind(&self, id: MaterialId) -> MaterialKind {
        self.get(id).kind
    }

    #[inline]
    pub fn density(&self, id: MaterialId) -> f32 {
        self.get(id).density
    }

    /// Look up by the `name` field in the material's file.
    pub fn id_of(&self, name: &str) -> Option<MaterialId> {
        self.by_name.get(name).copied()
    }

    pub fn len(&self) -> usize {
        self.materials.len()
    }

    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }

    /// Materials offered in the paint picker: everything except the internal
    /// vacuum and out-of-bounds sentinel.
    pub fn paintable(&self) -> Vec<MaterialId> {
        (0..self.materials.len() as u16)
            .map(MaterialId)
            .filter(|id| *id != EMPTY && *id != BEDROCK)
            .collect()
    }
}

impl Default for MaterialRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Panics if any shipped material file is malformed, which is what lets
    /// `builtin` be infallible.
    #[test]
    fn every_embedded_material_parses() {
        let reg = MaterialRegistry::builtin();
        assert_eq!(reg.len(), EMBEDDED.len() + 2, "a material failed to load");
    }

    #[test]
    fn the_files_on_disk_match_the_embedded_set() {
        // Catches a file added to assets/ but not to EMBEDDED, which would
        // silently work from source and break for a shipped binary.
        let mut reg = MaterialRegistry::builtin();
        let before = reg.len();
        let count = reg
            .reload(ASSET_DIR)
            .expect("assets/materials should load from the crate root");
        assert_eq!(count, EMBEDDED.len(), "assets/ and EMBEDDED disagree");
        assert_eq!(reg.len(), before, "loading from disk introduced new ids");
    }

    #[test]
    fn internal_materials_hold_the_ids_the_engine_assumes() {
        let reg = MaterialRegistry::builtin();
        assert_eq!(reg.id_of("empty"), Some(EMPTY));
        assert_eq!(reg.id_of("bedrock"), Some(BEDROCK));
    }

    #[test]
    fn expected_materials_are_present() {
        let reg = MaterialRegistry::builtin();
        for name in ["stone", "sand", "gravel", "ash", "water", "oil", "smoke"] {
            assert!(reg.id_of(name).is_some(), "{name} is missing");
        }
    }

    #[test]
    fn well_known_ids_match_their_names() {
        // Guards the convenience constants against a reordering of EMBEDDED,
        // which would otherwise silently point engine code at the wrong material.
        let mut reg = MaterialRegistry::builtin();
        let expected = [
            ("stone", STONE),
            ("sand", SAND),
            ("gravel", GRAVEL),
            ("ash", ASH),
            ("water", WATER),
            ("oil", OIL),
            ("smoke", SMOKE),
        ];
        for (name, id) in expected {
            assert_eq!(reg.id_of(name), Some(id), "{name} has the wrong id");
        }
        // And they must survive a reload from disk, since ids are keyed by name.
        reg.reload(ASSET_DIR).unwrap();
        for (name, id) in expected {
            assert_eq!(reg.id_of(name), Some(id), "{name} moved after a reload");
        }
    }

    #[test]
    fn every_material_has_at_least_one_colour() {
        let reg = MaterialRegistry::builtin();
        for i in 0..reg.len() as u16 {
            let m = reg.get(MaterialId(i));
            assert!(!m.palette.is_empty(), "{} has an empty palette", m.name);
        }
    }

    #[test]
    fn densities_order_the_way_the_rules_assume() {
        let reg = MaterialRegistry::builtin();
        let d = |n: &str| reg.density(reg.id_of(n).unwrap());
        // Sand sinks through water, oil floats on it, smoke rises through both.
        assert!(d("sand") > d("water"));
        assert!(d("water") > d("oil"));
        assert!(d("oil") > d("smoke"));
    }

    #[test]
    fn reload_keeps_ids_stable_so_existing_cells_survive() {
        let mut reg = MaterialRegistry::builtin();
        let sand_before = reg.id_of("sand").unwrap();
        reg.reload(ASSET_DIR).unwrap();
        assert_eq!(reg.id_of("sand"), Some(sand_before));
    }

    #[test]
    fn a_missing_directory_falls_back_to_the_embedded_set() {
        let (reg, err) = MaterialRegistry::load("no/such/directory");
        assert!(err.is_some(), "a missing directory should be reported");
        assert!(reg.id_of("sand").is_some(), "fallback set is unusable");
    }

    #[test]
    fn roll_reach_follows_the_angle_of_repose() {
        let reg = MaterialRegistry::builtin();

        // 45 degrees is the steepest a pile can hold. Reach 1 is the floor: a
        // downhill one column away would already have been taken by the
        // diagonal fall, so reach 1 never actually rolls anything.
        let gravel = reg.get(GRAVEL);
        for x in 0..100 {
            assert_eq!(gravel.roll_reach_at(x, 0), 1, "gravel reach changed at x = {x}");
        }

        // Shallower materials roll further. Averaged over positions, because
        // the fractional part of the reach is spent across the world rather
        // than over time.
        let mean = |m: &Material| {
            let mut total = 0;
            for y in 0..40 {
                for x in 0..40 {
                    total += m.roll_reach_at(x, y);
                }
            }
            total as f32 / 1600.0
        };
        let sand = mean(reg.get(SAND));
        let ash = mean(reg.get(ASH));
        assert!(
            sand > 1.0 && sand < 2.0,
            "sand reach {sand} should sit between 1 and 2"
        );
        assert!(ash > sand, "ash ({ash}) should roll further than sand ({sand})");
    }

    #[test]
    fn roll_reach_is_stable_for_a_position() {
        // The property that keeps sleeping safe: asking twice must agree, or a
        // chunk could settle on a "no" and freeze a grain that could move.
        let reg = MaterialRegistry::builtin();
        let sand = reg.get(SAND);
        for (x, y) in [(0, 0), (13, 7), (-5, 91), (1000, -1000)] {
            assert_eq!(sand.roll_reach_at(x, y), sand.roll_reach_at(x, y));
        }
    }

    #[test]
    fn a_byte_order_mark_does_not_break_a_file() {
        // Windows editors add one routinely, and RON rejects it at 1:1.
        let dir = std::env::temp_dir().join("pixel-physics-bom-material");
        std::fs::create_dir_all(&dir).unwrap();
        let body = "(name: \"bomtest\", kind: Powder, density: 1.0, colors: [(1, 2, 3)])";
        std::fs::write(dir.join("bomtest.ron"), format!("\u{feff}{body}")).unwrap();

        let mut reg = MaterialRegistry::builtin();
        reg.reload(&dir).expect("a leading BOM should be tolerated");
        assert!(reg.id_of("bomtest").is_some());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_malformed_file_is_reported_with_its_name() {
        let dir = std::env::temp_dir().join("pixel-physics-bad-material");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("broken.ron"), "(name: \"x\", this is not ron").unwrap();

        let mut reg = MaterialRegistry::builtin();
        let err = reg.reload(&dir).expect_err("malformed file should fail");
        match err {
            MaterialError::Parse { file, .. } => assert_eq!(file, "broken.ron"),
            other => panic!("expected a parse error, got {other:?}"),
        }
        // The good set must survive a bad reload untouched.
        assert!(reg.id_of("sand").is_some());

        std::fs::remove_dir_all(&dir).ok();
    }
}
