//! **The bed a plant is scored in: the lab's own, with nothing else alive.**
//!
//! Built by `lab::scene::LabBox` and stepped by `lab::tick_world`, the same
//! builder and the same tick the game runs, so a plant trained here is
//! trained in the physics the lab plays (`LabBox::build_counted` sets
//! `plant_size_cadence`, `plant_bending` off and `growth_slowdown` 2, where a
//! default world has the opposite; the plan's review, F7). One founder, no
//! colony, no predators.
//!
//! **Where the founder stands is a layout, not a constant.** A single founder
//! would otherwise always land at the compartment midpoint, which sits under a
//! lamp whenever the lamp count is odd (review F6). So a layout is a world
//! seed and an offset from the lamp nearest the middle: under it, a quarter
//! of the way to the next, or half way. A lab bed is not mirror-symmetric
//! (soil and lamp textures, rain columns, the sweep order), so layouts are
//! stratified by light rather than mirrored.

use crate::lab::rain::Rain;
use crate::lab::scene::LabBox;
use crate::sim::cell::OrganismId;
use crate::sim::explosion::Blasts;
use crate::sim::organism::{self, CellType};
use crate::sim::particle::ParticleSystem;
use crate::sim::player;
use crate::sim::world::World;

/// Where the founder stands relative to the light.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Offset {
    /// Directly under a lamp.
    Under,
    /// A quarter of the way from one lamp to the next.
    Quarter,
    /// Half way between two lamps: the dimmest spot on the bench.
    Between,
}

impl Offset {
    pub const ALL: [Offset; 3] = [Offset::Under, Offset::Quarter, Offset::Between];

    pub fn name(self) -> &'static str {
        match self {
            Offset::Under => "under",
            Offset::Quarter => "quarter",
            Offset::Between => "between",
        }
    }
}

/// One bed: a world seed and where the founder stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Layout {
    pub seed: u64,
    pub offset: Offset,
}

/// `n` layouts from `base`, cycling through the three offsets so any three
/// consecutive layouts cover every light position.
pub fn layouts(base: u64, n: usize) -> Vec<Layout> {
    (0..n).map(|i| Layout { seed: base + i as u64, offset: Offset::ALL[i % Offset::ALL.len()] }).collect()
}

/// **A station: the fixed conditions an individual is grown in** (plan §5).
/// One plant is noisy, so a genome is grown as several individuals across a
/// set of stations, and a role is judged mostly in the stations its niche is
/// made of -- an understory candidate in dim ones, a colonist in open ones.
/// A station changes only the light and the water; the bed, its soil and the
/// founder's layout are the run's.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Station {
    pub name: &'static str,
    /// The grow lamps' beam, as a fraction of the lab's.
    pub lamp: f32,
    /// The mister.
    pub rain: Rain,
    /// The soil's water at the start, as a fraction of the way from wilting
    /// point to what the bed was built holding (field capacity). 1 leaves the
    /// bed as built.
    pub soil: f32,
}

impl Station {
    /// The lab as it ships: full lamps, light rain.
    pub const BRIGHT: Station = Station { name: "bright", lamp: 1.0, rain: Rain::Light, soil: 1.0 };
    /// Lamps at 0.4 of their beam, the dim arm `plant_m0 axis=shade` measured
    /// its shade genes in (and checked was dim, which M0's between-lamps arm
    /// was not).
    pub const DIM: Station = Station { name: "dim", lamp: 0.4, rain: Rain::Light, soil: 1.0 };
    /// Full lamps, no mister, and the soil a quarter of the way from wilting
    /// to field capacity -- just above the highest germination threshold of
    /// the four role species (shrub, 0.20).
    ///
    /// **Not yet a station that scores anything, and the reason is the
    /// engine's, not this constant's** (M2, 2026-10-21; `plantrain
    /// mode=rungs`). A young plant's growth does not respond to soil water
    /// above the germination threshold: starting soil at 1.0, 0.35, 0.25 and
    /// 0.15 grew the same herb on 8-12 layouts (end shoot p50 116, 110, 106
    /// against 102 under the mister), and at 0.12 and below nothing
    /// germinated. Water is a cliff at germination with no middle, so this
    /// station is kept as the instrument and left out of the default set
    /// until a dry plant grows less. No mister alone is not dry either: a bed
    /// is built at field capacity, and over 3,000 frames it read the same as
    /// the bright station on every species, to the cell.
    pub const DRY: Station = Station { name: "dry", lamp: 1.0, rain: Rain::Off, soil: 0.25 };
    /// Lamps at 0.2 of their beam: **canopy shade**, the light an
    /// understory plant gets under a closed crown (leaves under the tree in
    /// the roles bed read ~0.08 of full; here ~0.11). Where shade tolerance
    /// earns its keep: a lone shipped shrub dies here on 16 of 16 layouts,
    /// and one with `shade_tolerate` 0.7 lives on 5 (`plant_m0 axis=shade
    /// dimlamp=0.2`, 60,000 frames; `Reports/plant-community-2026-10-20/
    /// m2/shade-scald/`). At `DIM`'s 0.4 tolerance is neutral.
    pub const SHADE: Station = Station { name: "shade", lamp: 0.2, rain: Rain::Light, soil: 1.0 };
    pub const ALL: [Station; 4] = [Station::BRIGHT, Station::DIM, Station::DRY, Station::SHADE];

    pub fn named(name: &str) -> Option<Station> {
        Station::ALL.into_iter().find(|s| s.name == name)
    }
}

/// **A rung of the ladder** (plan §6.2): a bed size, a length, and the steps
/// of a plant's life it scores. Each is the cheapest arena its stage can be
/// decided in; the run is cut off once it is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rung {
    /// Germinate and establish: a pocket of soil 48 wide, 3,000 frames.
    /// The plan's 24 cannot hold the layouts: a bed that narrow has one
    /// lamp, and "half way to the next" lands on the far wall. Measured: 24
    /// and 40 wide built no founder, 48 planted at all three offsets.
    Establish,
    /// Juvenile growth rate: 48 wide, 10,000 frames.
    Grow,
    /// A whole life, alone: P1's bed, 96 wide, 40,000 frames.
    Life,
    /// **Hold ground** (plan §6.2 rung 4): a whole life 56 wide, beside one
    /// shipped plant of another role ([`BedSpec::neighbour`]), so a plant
    /// that takes everything is seen to. The trainer runs it once per
    /// neighbour in turn.
    Hold,
}

impl Rung {
    pub const ALL: [Rung; 4] = [Rung::Establish, Rung::Grow, Rung::Life, Rung::Hold];

    pub fn name(self) -> &'static str {
        match self {
            Rung::Establish => "establish",
            Rung::Grow => "grow",
            Rung::Life => "life",
            Rung::Hold => "hold",
        }
    }

    pub fn named(name: &str) -> Option<Rung> {
        Rung::ALL.into_iter().find(|r| r.name() == name)
    }

    /// The rung's `n` layouts from `base`. **The short rungs plant under a
    /// lamp only**: the light is the station's job, and in a narrow bed a
    /// seed between two lamps never sprouts at all (measured on shrub and
    /// herb, 3,000 frames: 4 of 4 between-lamp seeds still a seed, every
    /// under-lamp one grown), so a third of the beds would score nothing for
    /// every genome. A whole life keeps all three offsets, as P1 had it.
    pub fn layouts(self, base: u64, n: usize) -> Vec<Layout> {
        match self {
            Rung::Life => layouts(base, n),
            // `Hold` places its own pair about a lamp ([`Neighbour`]).
            Rung::Establish | Rung::Grow | Rung::Hold => (0..n).map(|i| Layout { seed: base + i as u64, offset: Offset::Under }).collect(),
        }
    }

    /// The rung's bed, in `station`.
    pub fn spec(self, station: Station) -> BedSpec {
        let d = BedSpec { station, ..BedSpec::default() };
        match self {
            Rung::Establish => BedSpec { width: 48, frames: 3_000, every: 100, ..d },
            Rung::Grow => BedSpec { width: 48, frames: 10_000, every: 250, ..d },
            Rung::Life => d,
            Rung::Hold => BedSpec { width: 56, ..d },
        }
    }
}

/// The bed's size, length and sampling. Every field is printed in the run's
/// settings line.
#[derive(Clone, Copy, Debug)]
pub struct BedSpec {
    pub width: i32,
    pub frames: u64,
    /// Frames between two censuses of the founder.
    pub every: u64,
    /// Shoot cells a seedling needs to count as established (a run constant,
    /// measured on the shipped species: never read from a child).
    pub n_est: u32,
    /// **Heredity in the bed, off while scoring** (plan §5.4): the founder is
    /// founded on its species file's alleles exactly, and its offspring
    /// inherit without mutation. The only variation is the one the trainer
    /// makes. `false` leaves every dial as the lab ships it.
    pub mutation_off: bool,
    /// The light and water the bed is grown under.
    pub station: Station,
    /// A shipped plant of another role, planted beside the founder (rung 4).
    pub neighbour: Option<Neighbour>,
}

/// The four roles' shipped species (plan §4.2), the neighbours rung 4 plants.
pub const ROLES: [&str; 4] = ["tree", "shrub", "herb", "creeper"];

/// **A neighbour for rung 4**: a shipped species, `gap` columns from the
/// founder, the two either side of one lamp so they start in the same light.
/// The layout's seed picks which side the founder takes. Heredity is off for
/// it as for the founder. **Not one plant under a lamp and the other off
/// it**: placed `gap` from a founder at a layout offset, the neighbour landed
/// between lamps on some layouts and never established (a shipped tree
/// beside the shrub scored 1.77 against ~2.5), so the bed measured where it
/// stood rather than whom it stood beside. **56 wide, not the plan's 64**:
/// 64 holds two lamps (columns 18 and 46), and the pair about the first put
/// one plant six columns from the wall, which failed on every layout that
/// put the founder there (tree: peak 1 cell on 2 of 4; herb 4-107 shoot
/// cells against 180-247 on the other side). 56 holds one lamp, at 28, and
/// both plants stand 18 columns from a wall.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Neighbour {
    pub species: &'static str,
    pub gap: i32,
}

impl Neighbour {
    /// 12 columns: inside each other's crown and root spread from the first
    /// third of a life, measured by eye on the shipped shrub and herb (64
    /// wide, 40,000 frames), and far enough that neither seed lands on the
    /// other's first cells.
    pub const GAP: i32 = 12;

    pub fn named(species: &str) -> Option<Neighbour> {
        ROLES.into_iter().find(|&r| r == species).map(|species| Neighbour { species, gap: Self::GAP })
    }

    /// The founder's column: `gap / 2` to one side of the lamp the bed is
    /// built around.
    pub fn founder_column(self, bed: &LabBox) -> i32 {
        let lamp = founder_column(bed, Offset::Under);
        if bed.seed.is_multiple_of(2) {
            lamp - self.gap / 2
        } else {
            lamp + (self.gap - self.gap / 2)
        }
    }

    /// The neighbour's column: the founder's, mirrored about that lamp.
    pub fn column(self, bed: &LabBox, founder: i32) -> i32 {
        2 * founder_column(bed, Offset::Under) - founder
    }
}

impl Default for BedSpec {
    fn default() -> Self {
        Self { width: 96, frames: 40_000, every: 250, n_est: 20, mutation_off: true, station: Station::BRIGHT, neighbour: None }
    }
}

/// The founder's column for `offset`, in a `LabBox` of the given width.
pub fn founder_column(spec: &LabBox, offset: Offset) -> i32 {
    let (lamps, _) = spec.lamp_columns();
    let mid = spec.width / 2;
    let Some(&lamp) = lamps.iter().min_by_key(|&&c| (c - mid).abs()) else {
        return mid;
    };
    let spacing = lamps.windows(2).map(|w| w[1] - w[0]).min().unwrap_or(spec.lamp_spacing.max(8));
    match offset {
        Offset::Under => lamp,
        Offset::Quarter => lamp + spacing / 4,
        Offset::Between => lamp + spacing / 2,
    }
}

/// The `LabBox` a child is scored in.
pub fn bed_for(species_ron: Option<&str>, name: &str, layout: Layout, spec: &BedSpec) -> LabBox {
    let mut b = LabBox {
        width: spec.width,
        founders: 1,
        species: name.to_string(),
        colonies: 0,
        predators: 0,
        seed: layout.seed,
        species_ron: species_ron.map(str::to_string),
        rain: spec.station.rain,
        ..LabBox::default()
    };
    let at = match spec.neighbour {
        Some(nb) => nb.founder_column(&b),
        None => founder_column(&b, layout.offset),
    };
    b.founder_at = Some(vec![at]);
    b
}

/// What one bed's founder did, from a census every `BedSpec::every` frames.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Census {
    /// The founder grew past its seed.
    pub germinated: bool,
    /// The most shoot cells the founder held at any census.
    pub peak_shoot: u32,
    /// The tallest the founder stood, in rows above the ground.
    pub peak_height: i32,
    /// The first census at which the founder held a flower.
    pub first_flower: Option<u64>,
    /// The founder's shoot cells at that census.
    pub shoot_at_first_flower: u32,
    /// The founder held a fruit at some census.
    pub fruited: bool,
    /// Seeds the founder has set (`OrganismState::seeds_set`), at its last
    /// census alive. The plan's step 5 is graded by this (§5), and rung 4
    /// paid nothing for it: a herb trained there set a quarter of the shipped
    /// herb's seed and died out of the mixed bed (2026-10-22,
    /// `Reports/plant-community-2026-10-20/m2/community/`).
    pub seeds_set: u32,
    /// Plants of the founder's species, other than the founder, holding at
    /// least `n_est` shoot cells at the end. In a one-species bed with
    /// heredity off these are all the founder's line.
    pub seedlings_established: u32,
    /// The first census at which the founder held `n_est` shoot cells.
    pub established_at: Option<u64>,
    /// The founder's shoot cells at the last census.
    pub end_shoot: u32,
    /// The founder is alive at the end.
    pub founder_alive: bool,
    /// Frames the bed ran.
    pub frames: u64,
    /// The rung-4 neighbour's own census, by the same rules, when the bed had
    /// one. Its `seedlings_established` counts its own species.
    pub neighbour: Option<Box<Census>>,
}

/// The founder's flower and fruit cells right now.
fn organs_of(w: &World, id: OrganismId) -> (u32, u32) {
    let Some(st) = w.organism(id) else { return (0, 0) };
    let (mut flowers, mut fruit) = (0, 0);
    for &(x, y) in st.cells.keys() {
        match organism::cell_type(w.get(x, y).aux()) {
            Some(CellType::Flower) => flowers += 1,
            Some(CellType::Fruit) => fruit += 1,
            _ => {}
        }
    }
    (flowers, fruit)
}

/// The founder's height above `ground_y` right now.
fn height_of(w: &World, id: OrganismId, ground_y: i32) -> i32 {
    w.organism(id).map_or(0, |st| st.cells.keys().map(|&(_, y)| ground_y - y).max().unwrap_or(0))
}

/// Scale the grow lamps' beam by `factor` and re-place every lamp cell, so
/// the light field picks the new beam up (`plant_m0`'s `dim_lamps`, which
/// measured that it does).
pub fn dim_lamps(w: &mut World, factor: f32, width: i32, height: i32) {
    let Some(lamp) = w.materials.id_of("growlamp") else {
        return;
    };
    if factor == 1.0 {
        return;
    }
    w.materials.get_mut(lamp).beam *= factor;
    for y in 0..height {
        for x in 0..width {
            let c = w.get(x, y);
            if c.material == lamp {
                w.set(x, y, crate::sim::cell::Cell::EMPTY);
                w.set(x, y, c);
            }
        }
    }
}

/// Set every soil cell's water to `fraction` of the way from wilting point to
/// what it holds now -- its own field capacity, as the bed was built, so a
/// textured bed keeps each soil's own scale.
pub fn dry_soil(w: &mut World, fraction: f32, width: i32, height: i32) {
    let Some(soil) = w.materials.id_of("soil") else {
        return;
    };
    let wilt = crate::sim::material::SOIL_WILTING_POINT as f32;
    for y in 0..height {
        for x in 0..width {
            let c = w.get(x, y);
            if c.material == soil {
                let held = c.aux() as f32;
                let to = wilt + fraction * (held - wilt).max(0.0);
                w.set(x, y, c.with_aux(to.round() as u16));
            }
        }
    }
}

/// Build the bed, find its founder, apply the heredity dials and the
/// station's light. The founder is the one organism the bed holds once it is
/// built (no colony, no predator, one founder).
pub fn build(bed: &LabBox, spec: &BedSpec) -> (World, OrganismId) {
    // Founded from the species file, never the lab's plant pool
    // (`LabBox::plant_pool`): a pool genome would overwrite the very
    // candidate this bed exists to score.
    let mut w = LabBox { plant_pool: false, ..bed.clone() }.build();
    dim_lamps(&mut w, spec.station.lamp, bed.width, bed.height);
    if spec.station.soil < 1.0 {
        dry_soil(&mut w, spec.station.soil, bed.width, bed.height);
    }
    if spec.mutation_off {
        w.fate_mutation_chance = 0.0;
        w.param_mutation_chance = 0.0;
        w.mutation_sigma = 0.0;
        w.founder_variant_chance = 0.0;
    }
    let ids = w.live_organism_ids();
    assert_eq!(ids.len(), 1, "a one-founder bed built {} organisms", ids.len());
    if let Some(nb) = spec.neighbour {
        let at = bed.founder_at.as_ref().and_then(|v| v.first().copied()).unwrap_or(bed.width / 2);
        let x = nb.column(bed, at);
        assert!(w.plant_tree_species(x, bed.ground_y - 2, nb.species), "the neighbour {} did not plant at column {x}", nb.species);
    }
    (w, ids[0])
}

/// The founder, or the neighbour, census as it runs.
struct Tracker {
    id: OrganismId,
    species: organism::SpeciesId,
    c: Census,
}

impl Tracker {
    fn new(w: &World, id: OrganismId) -> Self {
        let species = w.organism(id).map(|s| s.species).expect("the plant exists");
        Self { id, species, c: Census::default() }
    }

    fn alive<'a>(&self, w: &'a World) -> Option<&'a organism::OrganismState> {
        w.organism(self.id).filter(|st| st.species == self.species)
    }

    fn census(&mut self, w: &World, f: u64, spec: &BedSpec, ground_y: i32) {
        // A slot freed and reused by another plant means the plant is gone.
        let Some(st) = self.alive(w) else { return };
        let c = &mut self.c;
        let shoot = st.shoot_cells;
        // **More than the seed**: an unsprouted seed is one shoot cell, so
        // `shoot > 0` counted every seed that sat on the ground for the whole
        // run as germinated (measured: 12 of 12 "germinated" on a narrow bed
        // where four never grew a cell). Germination is the first root or
        // shoot past the seed.
        c.germinated |= st.cells.len() > 1;
        c.peak_shoot = c.peak_shoot.max(shoot);
        c.end_shoot = shoot;
        if shoot >= spec.n_est && c.established_at.is_none() {
            c.established_at = Some(f);
        }
        c.peak_height = c.peak_height.max(height_of(w, self.id, ground_y));
        let (flowers, fruit) = organs_of(w, self.id);
        if flowers > 0 && c.first_flower.is_none() {
            c.first_flower = Some(f);
            c.shoot_at_first_flower = shoot;
        }
        c.fruited |= fruit > 0;
        c.seeds_set = c.seeds_set.max(st.seeds_set);
    }

    fn finish(mut self, w: &World, spec: &BedSpec, others: &[OrganismId]) -> Census {
        self.c.founder_alive = self.alive(w).is_some();
        if !self.c.founder_alive {
            self.c.end_shoot = 0;
        }
        self.c.seedlings_established = w
            .live_organism_ids()
            .into_iter()
            .filter(|id| *id != self.id && !others.contains(id))
            .filter_map(|id| w.organism(id))
            .filter(|s| s.species == self.species && s.shoot_cells >= spec.n_est)
            .count() as u32;
        self.c.frames = spec.frames;
        self.c
    }
}

/// Grow `bed` for `spec.frames` frames and census its founder, and its
/// neighbour when it has one.
pub fn grow(bed: &LabBox, spec: &BedSpec) -> Census {
    let (mut w, founder) = build(bed, spec);
    let mut me = Tracker::new(&w, founder);
    let mut nb = w.live_organism_ids().into_iter().find(|&id| id != founder).map(|id| Tracker::new(&w, id));
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    for f in 1..=spec.frames {
        crate::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, bed, None);
        if f % spec.every != 0 && f != spec.frames {
            continue;
        }
        me.census(&w, f, spec, bed.ground_y);
        if let Some(t) = nb.as_mut() {
            t.census(&w, f, spec, bed.ground_y);
        }
    }
    let nb_id = nb.as_ref().map(|t| t.id);
    let neighbour = nb.map(|t| Box::new(t.finish(&w, spec, &[founder])));
    let mut c = me.finish(&w, spec, &nb_id.into_iter().collect::<Vec<_>>());
    c.neighbour = neighbour;
    c
}

/// **The whole bed after `spec.frames`, as one number**: every cell's
/// material, shade, aux and organism id, over the box. A census of the founder
/// is too coarse to say whether two species grow the same plant (measured: a
/// 1% leaf photosynthesis change left a 3,000-frame census identical), so a
/// round trip, or whether a gene moves anything at all, is asked of this.
/// **An unchanged hash is conclusive that nothing moved; a changed one says
/// only that something did** (`genome_reach`'s rule), since a bed is chaotic
/// in every draw.
pub fn grow_hash(bed: &LabBox, spec: &BedSpec) -> u64 {
    let (mut w, _) = build(bed, spec);
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    for _ in 0..spec.frames {
        crate::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, bed, None);
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for y in 0..bed.height {
        for x in 0..bed.width {
            let c = w.get(x, y);
            for v in [c.material.0 as u64, c.shade as u64, c.aux() as u64, c.organism_id() as u64] {
                h = (h ^ v).wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_offsets_put_the_founder_in_three_places_under_the_light() {
        let spec = BedSpec::default();
        let cols: Vec<i32> = Offset::ALL
            .iter()
            .map(|&o| bed_for(None, "herb", Layout { seed: 1, offset: o }, &spec).founder_at.expect("set")[0])
            .collect();
        assert!(cols[0] < cols[1] && cols[1] < cols[2], "{cols:?}");
        let b = bed_for(None, "herb", Layout { seed: 1, offset: Offset::Under }, &spec);
        assert!(b.lamp_columns().0.contains(&cols[0]), "under a lamp: {cols:?} against {:?}", b.lamp_columns().0);
    }

    /// **A station reaches the bed it names.** Dim scales the lamp's beam;
    /// dry sets the soil to its fraction of the way from wilting point; the
    /// short rungs plant under a lamp only. Each read off the built world,
    /// against the bright station built from the same layout.
    #[test]
    fn a_station_reaches_the_bed_and_the_short_rungs_plant_under_a_lamp() {
        let layout = Layout { seed: 3, offset: Offset::Under };
        let built = |station: Station| {
            let spec = Rung::Establish.spec(station);
            let bed = bed_for(None, "herb", layout, &spec);
            let (w, _) = build(&bed, &spec);
            let lamp = w.materials.id_of("growlamp").expect("the lab has lamps");
            let soil = w.materials.id_of("soil").expect("soil ships");
            let water: Vec<u16> = (0..bed.height).flat_map(|y| (0..bed.width).map(move |x| (x, y))).map(|(x, y)| w.get(x, y)).filter(|c| c.material == soil).map(|c| c.aux()).collect();
            (w.materials.get(lamp).beam, water, bed.rain)
        };
        let (beam, wet, rain) = built(Station::BRIGHT);
        let (dim_beam, _, _) = built(Station::DIM);
        let (_, dry, dry_rain) = built(Station::DRY);
        assert!((dim_beam - beam * Station::DIM.lamp).abs() < 1e-6, "dim: beam {dim_beam} against {beam}");
        assert_eq!((rain, dry_rain), (Rain::Light, Rain::Off));
        let mean = |v: &[u16]| v.iter().map(|&a| a as f32).sum::<f32>() / v.len().max(1) as f32;
        let wilt = crate::sim::material::SOIL_WILTING_POINT as f32;
        let want = wilt + Station::DRY.soil * (mean(&wet) - wilt);
        assert!((mean(&dry) - want).abs() < 2.0, "dry: soil water {} against {want}", mean(&dry));
        assert!(Rung::Establish.layouts(0, 6).iter().all(|l| l.offset == Offset::Under));
        assert_eq!(Rung::Life.layouts(0, 3).iter().map(|l| l.offset).collect::<Vec<_>>(), Offset::ALL.to_vec());
    }

    /// **A rung-4 bed plants its pair about one lamp, and censuses both.**
    /// The founder and the neighbour stand `gap` apart, mirrored about the
    /// lamp, on either side by the layout's seed; each grows past its seed,
    /// and the neighbour's census is its own species, not the founder's.
    #[test]
    fn a_hold_bed_plants_the_pair_about_a_lamp_and_censuses_both() {
        let nb = Neighbour::named("herb").expect("a role");
        let spec = BedSpec { frames: 2_500, neighbour: Some(nb), ..Rung::Hold.spec(Station::BRIGHT) };
        let mut sides = Vec::new();
        for seed in [4, 5] {
            let bed = bed_for(None, "shrub", Layout { seed, offset: Offset::Under }, &spec);
            let at = bed.founder_at.as_ref().expect("set")[0];
            let lamp = founder_column(&bed, Offset::Under);
            assert_eq!((nb.column(&bed, at) - at).abs(), nb.gap, "a gap apart");
            assert_eq!(at + nb.column(&bed, at), 2 * lamp, "mirrored about the lamp");
            sides.push(at < lamp);
            let (w, _) = build(&bed, &spec);
            assert_eq!(w.live_organism_ids().len(), 2, "founder and neighbour");
            let c = grow(&bed, &spec);
            let n = c.neighbour.as_deref().expect("the neighbour's census");
            assert!(c.germinated && n.germinated, "both grew past the seed: {c:?}");
        }
        assert_ne!(sides[0], sides[1], "the seed picks the founder's side");
        assert!(Rung::Hold.layouts(0, 4).iter().all(|l| l.offset == Offset::Under));
    }

    #[test]
    fn a_bed_holds_exactly_its_founder_and_the_dials_are_off() {
        let spec = BedSpec::default();
        let bed = LabBox { plant_pool: true, ..bed_for(None, "herb", Layout { seed: 3, offset: Offset::Quarter }, &spec) };
        let (w, founder) = build(&bed, &spec);
        assert!(w.organism(founder).is_some());
        assert_eq!(w.founder_variant_chance, 0.0);
        assert_eq!(w.fate_mutation_chance, 0.0);
        assert!(bed.plant_pool, "test setup: the box would have pooled the herb");
        assert!(w.plant_pool.is_none(), "the founder is the species file, not a pool genome");
    }
}
