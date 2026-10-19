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
}

impl Default for BedSpec {
    fn default() -> Self {
        Self { width: 96, frames: 40_000, every: 250, n_est: 20, mutation_off: true }
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

/// Build the bed, find its founder, apply the heredity dials. The founder is
/// the one organism the bed holds once it is built (no colony, no predator,
/// one founder).
pub fn build(bed: &LabBox, spec: &BedSpec) -> (World, OrganismId) {
    let mut w = bed.build();
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
        c.germinated |= shoot > 0;
        c.peak_shoot = c.peak_shoot.max(shoot);
        c.peak_height = c.peak_height.max(height_of(&w, founder, bed.ground_y));
        let (flowers, fruit) = organs_of(&w, founder);
        if flowers > 0 && c.first_flower.is_none() {
            c.first_flower = Some(f);
            c.shoot_at_first_flower = shoot;
        }
        c.fruited |= fruit > 0;
    }
    c.founder_alive = w.organism(founder).is_some_and(|s| s.species == species);
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
