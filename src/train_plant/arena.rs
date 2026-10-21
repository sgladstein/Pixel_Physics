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
    pub const ALL: [Station; 3] = [Station::BRIGHT, Station::DIM, Station::DRY];

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
}

impl Rung {
    pub const ALL: [Rung; 3] = [Rung::Establish, Rung::Grow, Rung::Life];

    pub fn name(self) -> &'static str {
        match self {
            Rung::Establish => "establish",
            Rung::Grow => "grow",
            Rung::Life => "life",
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
            Rung::Establish | Rung::Grow => (0..n).map(|i| Layout { seed: base + i as u64, offset: Offset::Under }).collect(),
        }
    }

    /// The rung's bed, in `station`.
    pub fn spec(self, station: Station) -> BedSpec {
        let d = BedSpec { station, ..BedSpec::default() };
        match self {
            Rung::Establish => BedSpec { width: 48, frames: 3_000, every: 100, ..d },
            Rung::Grow => BedSpec { width: 48, frames: 10_000, every: 250, ..d },
            Rung::Life => d,
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
}

impl Default for BedSpec {
    fn default() -> Self {
        Self { width: 96, frames: 40_000, every: 250, n_est: 20, mutation_off: true, station: Station::BRIGHT }
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
    b.founder_at = Some(vec![founder_column(&b, layout.offset)]);
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
    let mut w = bed.build();
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
    (w, ids[0])
}

/// Grow `bed` for `spec.frames` frames and census its founder.
pub fn grow(bed: &LabBox, spec: &BedSpec) -> Census {
    let (mut w, founder) = build(bed, spec);
    let species = w.organism(founder).map(|s| s.species).expect("the founder exists");
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    let mut c = Census::default();
    for f in 1..=spec.frames {
        crate::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, bed, None);
        if f % spec.every != 0 && f != spec.frames {
            continue;
        }
        let Some(st) = w.organism(founder) else { continue };
        if st.species != species {
            // The slot was freed and reused by another plant: the founder is gone.
            continue;
        }
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
        c.peak_height = c.peak_height.max(height_of(&w, founder, bed.ground_y));
        let (flowers, fruit) = organs_of(&w, founder);
        if flowers > 0 && c.first_flower.is_none() {
            c.first_flower = Some(f);
            c.shoot_at_first_flower = shoot;
        }
        c.fruited |= fruit > 0;
    }
    c.founder_alive = w.organism(founder).is_some_and(|s| s.species == species);
    if !c.founder_alive {
        c.end_shoot = 0;
    }
    c.seedlings_established = w
        .live_organism_ids()
        .into_iter()
        .filter(|&id| id != founder)
        .filter_map(|id| w.organism(id))
        .filter(|s| s.species == species && s.shoot_cells >= spec.n_est)
        .count() as u32;
    c.frames = spec.frames;
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

    #[test]
    fn a_bed_holds_exactly_its_founder_and_the_dials_are_off() {
        let spec = BedSpec::default();
        let bed = bed_for(None, "herb", Layout { seed: 3, offset: Offset::Quarter }, &spec);
        let (w, founder) = build(&bed, &spec);
        assert!(w.organism(founder).is_some());
        assert_eq!(w.founder_variant_chance, 0.0);
        assert_eq!(w.fate_mutation_chance, 0.0);
    }
}
