//! **Every individual in the box, as rows you can sort and click.**
//!
//! The lab could already tell you what the *box* was doing (the PLANTS and
//! ANTS pages) and what *one individual* was, if you could find it (the cell
//! page, `params::specimen_sections`). It had nothing in between: no way to
//! enumerate the population, so the only route to an individual was spotting
//! it on screen. The design guide measured what that costs -- *"an ant is two
//! dark cells at play zoom, findable only because it moves"* -- and a dead one
//! has stopped moving, so the thing you most want to look at is the thing you
//! cannot find.
//!
//! **This module is the data and not the page**, for the reason
//! `params::specimen_sections`' grouping lives beside the numbers rather than
//! in the painter: which individuals exist, and how they compare, is a claim
//! about the world. A harness needs it without a framebuffer, and the sort
//! order is something a test should be able to assert.

use crate::sim::organism::SpeciesId;
use crate::sim::world::World;

/// **Which individual**, across frames.
///
/// The handle alone is not an identity -- `encode_organism_id` gives the slot
/// 12 bits and the generation 4, so a handle comes back after 16 turns of a
/// slot. `Reports/dead-ends.md` records the general shape this is avoiding:
/// *any selection stored as a position into a list a neighbouring verb
/// rewrites has this bug, and re-finding by identity is the fix* -- and a
/// roster is rewritten by every birth and every death.
///
/// `born_frame` is the term that makes it unique; see that field's own doc for
/// why it is per-world rather than rack-wide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Individual {
    pub id: u16,
    pub born_frame: u64,
}

impl Individual {
    /// The individual this names, or `None` if it is gone.
    ///
    /// **Both halves are checked.** A handle that resolves is not enough: it
    /// resolves to *whatever is in that slot now*, which after a reuse is a
    /// different animal wearing the same number. The frame is what tells them
    /// apart, and a mismatch is a death rather than a lookup failure.
    pub fn resolve(self, world: &World) -> Option<&crate::sim::organism::OrganismState> {
        world.organism(self.id).filter(|s| s.born_frame == self.born_frame)
    }

    /// Whether this individual is still alive in `world`.
    pub fn alive(self, world: &World) -> bool {
        self.resolve(world).is_some()
    }
}

/// Which table. Plants and animals are the same type in the engine
/// (`OrganismState`), split only by whether the species carries a
/// `CreatureDef` -- so the split is the caller's, and this is the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kingdom {
    Plants,
    Creatures,
}

/// **How an individual is doing, in one word.**
///
/// A column of numbers does not answer *"which of these is in trouble"* at a
/// glance, and that is the question a roster is opened to ask. Ordered worst
/// first so a sort on this column puts the dying at the top.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RowState {
    /// Dead and rotting. A culled plant reads this way and keeps its cells
    /// until they rot, which is why a cull is graded rather than a deletion.
    Senescent,
    /// Failing to pay its upkeep, with the starvation clock running.
    Starving,
    /// An animal poor enough that its next charge could kill it.
    Hungry,
    /// An animal carrying food.
    Carrying,
    /// **An animal deep in an excursion** -- further from the last place it
    /// touched home than `FAR_FROM_HOME`, in cells.
    ///
    /// **This row used to be `LOST`, and the state it named no longer
    /// exists.** It read `since_nest >= nest_memory`: past that window the
    /// `recency` multiplier scaling channel-A deposit was exactly zero, so
    /// the ant had no gradient home and "lost" was a fact about the engine.
    /// 2026-09-02 deleted both halves -- `nest_memory` is gone, and homing
    /// is now three authored weights on a self-recurrent hidden unit
    /// (`creature.rs`'s deposit block, and
    /// `Reports/creature-genome-flexibility-2026-09-02.md` §2b). That curve
    /// is hyperbolic and **never reaches zero**, and its decay rate is each
    /// individual's own genome rather than a species constant. So there is
    /// no threshold in Rust at which an animal's way home has gone, and a
    /// roster column claiming otherwise would be re-inventing the species
    /// constant main deleted.
    ///
    /// What survives is honest and is a different question: *how deep is the
    /// excursion in progress*. `forage_max` answers it in **cells**, and it
    /// re-anchors at every nest contact, so -- unlike `since_nest`, whose
    /// resets were 136-of-142 loitering -- standing on the nest cannot
    /// manufacture one. See `OrganismState::forage_anchor`.
    ///
    /// **Gated on the species declaring a nest.** `CreatureDef::nest` is
    /// optional since the same change; a species without one never
    /// re-anchors, so its `forage_max` is just distance from where it
    /// hatched and would climb to FAR and stay there for ever. An animal
    /// with no home cannot be far from it.
    ///
    /// **The first version of this row was `HOME`, on `since_nest <
    /// nest_memory`, and it was vacuous**: the ant's window is 3,000 ticks
    /// and a nine-hundred-frame bed had every animal at 182, so the column
    /// said HOME fifty-two times. Same defect as the hunger floor below it,
    /// found the same way -- by looking at the drawn table. That is the
    /// reason `FAR_FROM_HOME` is set from a measured distribution rather
    /// than picked.
    Far,
    Ok,
}

impl RowState {
    pub fn label(self) -> &'static str {
        match self {
            RowState::Senescent => "ROTTING",
            RowState::Starving => "STARVING",
            RowState::Hungry => "HUNGRY",
            RowState::Carrying => "LADEN",
            RowState::Far => "FAR",
            RowState::Ok => "OK",
        }
    }

    /// Whether this is a state worth being told about -- what the `IN
    /// TROUBLE` filter keeps.
    ///
    /// **`Far` is deliberately not in this set, and it is the one that
    /// changed.** Its predecessor `Lost` was: past `nest_memory` an ant had
    /// no gradient home, which is a failure. `Far` measures excursion depth
    /// instead, and a deep excursion is an ant doing its job -- a colony
    /// whose animals never went far would be the thing worth reporting. See
    /// `RowState::Far`.
    pub fn is_trouble(self) -> bool {
        matches!(self, RowState::Senescent | RowState::Starving | RowState::Hungry)
    }
}

/// **An animal below this share of the bank it started life with is
/// "hungry".**
///
/// A fraction of the species' own `start_energy` rather than an absolute
/// number, because a 36-cell creature and a 2-cell ant have banks two orders
/// of magnitude apart and one threshold cannot serve both. This is a readout,
/// not a rule: nothing in the simulation branches on it.
///
/// **`start_energy`, and the first version used `body_energy`, which put
/// every ant in the box on the same word.** The ant's `body_energy` is 480
/// (what its corpse is worth as meat) against a `start_energy` of 200 (what
/// it is given to live on), so a floor scaled off the first read 336 against
/// banks sitting around 150 and the column said HUNGRY down all fifty-two
/// rows. A state that never varies is not a state -- it is a constant with a
/// column to itself -- and the only thing that showed it was reading the
/// rendered table.
const HUNGRY_FRACTION: f32 = 0.35;

/// **An animal whose current excursion has reached this many cells from the
/// last place it touched home is `FAR`.** Chebyshev cells, read off
/// `OrganismState::forage_max`.
///
/// **Set from the measured distribution, because the two row states written
/// before it were both set by eye and both came out vacuous** -- `HOME` said
/// HOME fifty-two times, and the first hunger floor said HUNGRY down all
/// fifty-two rows. `what_the_bed_ranges` is the readout, on the shipped bed
/// with one colony:
///
/// | frame | p10 | p50 | p90 | max | rows reading FAR |
/// |---|---|---|---|---|---|
/// | 900 | 0 | 13 | 26 | 38 | 3 of 52 |
/// | 2,000 | 2 | 16 | 37 | 51 | 12 of 52 |
/// | 4,000 | 5 | 23 | 48 | 64 | 9 of 51 |
/// | 9,000 | 12 | 32 | 61 | 75 | 8 of 16 |
///
/// 30 sits just above the settled early bed's p90 (26 at frame 900), so it
/// marks the top of the range rather than the middle of it, and it stays a
/// minority as the distribution walks right. **It never reads zero rows and
/// never reads all of them at any tile**, which is the only property the two
/// failures above lacked.
///
/// **It climbs through a run and that is the measure working, not drifting.**
/// `forage_max` is the deepest point of the excursion *in progress* and
/// re-anchors only on nest contact, so an animal that stops going home keeps
/// climbing. That is the thing worth seeing.
const FAR_FROM_HOME: u16 = 30;

/// One individual, as the table draws it.
///
/// Everything here is read off `OrganismState` in one pass. Deliberately a
/// flat struct of already-computed values rather than a borrow of the state:
/// the page sorts it, and sorting a list of borrows against a world it is
/// still holding is a fight with the borrow checker that buys nothing.
#[derive(Clone, Debug)]
pub struct RosterRow {
    pub who: Individual,
    pub species: SpeciesId,
    /// Where to point the marker: a creature's head, or the centre of a
    /// plant's cells.
    pub at: (i32, i32),
    /// The bounding box of everything it owns, for the whole-body marker.
    pub bounds: (i32, i32, i32, i32),
    pub cells: u32,
    pub generation: u16,
    pub lineage: u32,
    pub born_frame: u64,
    /// A creature's energy bank; a plant's water status, 0..1.
    pub energy: f32,
    /// A creature's crop, in cells; a plant's shoot count.
    pub carrying: u32,
    /// **What it has actually produced** -- a plant's seeds set, an animal's
    /// young. It is the fitness column, in the only sense the box measures.
    pub score: u32,
    pub state: RowState,
}

/// Which column the table is sorted on. Positional, and the page's column
/// list is asserted against it, so an inserted column breaks the build rather
/// than silently sorting on its neighbour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    /// The order the engine hands them out. Carries no opinion, which is why
    /// it is the default: a box with six plants is read as it stands.
    Slot,
    Species,
    Cells,
    Energy,
    Carrying,
    Score,
    Generation,
    Lineage,
    Age,
    State,
}

/// What the table is showing. `All` is the default; the rest are one click.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    /// Only the individuals whose `RowState` is worth being told about.
    Trouble,
    /// Only one founding line. Set by selecting a row and pressing the chip,
    /// so it is always a line that exists.
    Lineage(u32),
}

impl Filter {
    pub fn label(self) -> String {
        match self {
            Filter::All => "ALL".to_string(),
            Filter::Trouble => "IN TROUBLE".to_string(),
            Filter::Lineage(l) => format!("LINE {l}"),
        }
    }
}

/// **Every live individual of one kingdom, sorted and filtered.**
///
/// Walks `World::live_organism_ids` -- the organism registry, never a scan for
/// head cells. `Reports/open-bugs-handoff.md` §R3: a creature chain above two
/// cells overwrites its own `CellType::Head` marking, so a head scan reports
/// an empty world over a living population. The registry closes its books
/// exactly and is the only enumeration that does.
///
/// Rebuilt every frame the page is open and never retained, which is the
/// tradeoff `Ui::page_params` states: a retained list is a list that
/// disagrees with the world, and the world here is tens of organisms.
pub fn rows(world: &World, kingdom: Kingdom, sort: SortKey, desc: bool, filter: Filter) -> Vec<RosterRow> {
    let mut out: Vec<RosterRow> = Vec::new();
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        let def = world.species.get(state.species);
        let creature = def.creature.is_some();
        if creature != (kingdom == Kingdom::Creatures) {
            continue;
        }
        let row = row_of(world, id, state);
        let keep = match filter {
            Filter::All => true,
            Filter::Trouble => row.state.is_trouble(),
            Filter::Lineage(l) => row.lineage == l,
        };
        if keep {
            out.push(row);
        }
    }
    sort_rows(&mut out, sort, desc);
    out
}

/// **The one cell that stands for a whole individual.**
///
/// A creature's head, because that is the cell a player would point at and
/// call "the ant". A plant's *lowest* cell -- the collar, where it meets the
/// ground -- rather than the centre of its bounding box, for two reasons: the
/// collar is stable while the crown grows, where the centre climbs as the
/// plant does, and the centre of a leaning tree's box is often not a cell the
/// tree owns at all.
///
/// **Shared, because two callers disagreeing about it is visible.** The
/// roster's notice said `PINNED HERB AT 226,154` (the box centre) while the
/// cell page it opened read `AT 228,159` (the collar), from the same click on
/// the same plant.
pub fn anchor_of(state: &crate::sim::organism::OrganismState) -> Option<(i32, i32)> {
    state.chain.first().copied().or_else(|| state.cells.keys().copied().max_by_key(|&(_, y)| y))
}

/// One row, from one organism.
fn row_of(world: &World, id: u16, state: &crate::sim::organism::OrganismState) -> RosterRow {
    let def = world.species.get(state.species);
    let creature = def.creature.as_ref();

    // **The bounding box comes from `cells`, which is the membership the
    // engine maintains at the `World::set` seam** -- so it is every cell the
    // organism owns, including the ones a stale `chain` would miss. An empty
    // set is possible for one tick between allocation and the first cell, and
    // is reported at the origin rather than panicking on an empty fold.
    let mut bounds = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for &(x, y) in state.cells.keys() {
        bounds.0 = bounds.0.min(x);
        bounds.1 = bounds.1.min(y);
        bounds.2 = bounds.2.max(x);
        bounds.3 = bounds.3.max(y);
    }
    if bounds.0 > bounds.2 {
        bounds = (0, 0, 0, 0);
    }
    let at = anchor_of(state).unwrap_or(((bounds.0 + bounds.2) / 2, (bounds.1 + bounds.3) / 2));

    let state_word = if state.senescent {
        RowState::Senescent
    } else if let Some(c) = creature {
        let bank = state.energy;
        let floor = c.start_energy * HUNGRY_FRACTION;
        if bank < floor {
            RowState::Hungry
        } else if !c.nest.is_empty() && state.forage_max >= FAR_FROM_HOME {
            RowState::Far
        } else if state.crop.is_some() {
            RowState::Carrying
        } else {
            RowState::Ok
        }
    } else if state.starving_ticks > 0 {
        RowState::Starving
    } else {
        RowState::Ok
    };

    RosterRow {
        who: Individual { id, born_frame: state.born_frame },
        species: state.species,
        at,
        bounds,
        cells: state.cells.len() as u32,
        generation: state.generation,
        lineage: state.lineage,
        born_frame: state.born_frame,
        energy: if creature.is_some() { state.energy } else { state.water_status },
        carrying: if creature.is_some() {
            state.crop.as_ref().map_or(0, |c| c.cells as u32)
        } else {
            state.shoot_cells
        },
        // A plant counts seeds, an animal counts young: the same question
        // asked of two kingdoms that answer it with different organs.
        score: if creature.is_some() { state.life.offspring } else { state.seeds_set },
        state: state_word,
    }
}

/// Sort in place.
///
/// **The comparator breaks ties explicitly, on the identity.** `CLAUDE.md`'s
/// `sort_unstable` gotcha is that an unstable sort's tie order is not a
/// function of the comparator alone -- it depends on the element type, so two
/// sorts asking identical questions can order equal elements differently. A
/// roster ties constantly (every founder is generation 0, every ant of one
/// species is the same size), so without this the rows under the cursor
/// reshuffle between frames and a click lands on a different animal than the
/// one it was aimed at.
fn sort_rows(rows: &mut [RosterRow], sort: SortKey, desc: bool) {
    rows.sort_by(|a, b| compare(a, b, sort, desc));
}

/// The roster's order, as one function.
///
/// **Named and `pub` so a guard can assert the property rather than the
/// output.** The obvious test -- sort the same list eight times and check the
/// answer does not move -- is **blind**, and that was measured rather than
/// assumed: it stays green with the tie-break deleted, because a sort is
/// deterministic within one build whatever its comparator says. The hazard
/// `CLAUDE.md` records is that the tie order changes when the *element type*
/// or the toolchain's small-sort strategy changes, which nothing inside one
/// build can observe.
///
/// What is checkable is the property that makes the hazard harmless: this is
/// a **total** order, returning `Equal` only for a row against itself. Then
/// no two distinct rows are ever tied, so no sort implementation has a choice
/// to make.
pub fn compare(a: &RosterRow, b: &RosterRow, sort: SortKey, desc: bool) -> std::cmp::Ordering {
    {
        let ord = match sort {
            SortKey::Slot => std::cmp::Ordering::Equal,
            SortKey::Species => a.species.0.cmp(&b.species.0),
            SortKey::Cells => a.cells.cmp(&b.cells),
            SortKey::Energy => a.energy.total_cmp(&b.energy),
            SortKey::Carrying => a.carrying.cmp(&b.carrying),
            SortKey::Score => a.score.cmp(&b.score),
            SortKey::Generation => a.generation.cmp(&b.generation),
            SortKey::Lineage => a.lineage.cmp(&b.lineage),
            // Older first when ascending: `born_frame` counts up, age counts
            // down, so the two are reversed and the column is AGE.
            SortKey::Age => b.born_frame.cmp(&a.born_frame),
            SortKey::State => a.state.cmp(&b.state),
        };
        // The tie-break is the whole point -- see the doc above.
        let ord = ord.then(a.who.id.cmp(&b.who.id)).then(a.who.born_frame.cmp(&b.who.born_frame));
        if desc { ord.reverse() } else { ord }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::{scene, Lab};

    /// **What excursion depths the bed actually produces** — the readout
    /// `FAR_FROM_HOME` is set from, and the one that has twice caught a row
    /// state that never varies.
    ///
    /// A readout rather than an assertion, for the reason the two vacuous
    /// versions before it were both found by eye: the failure mode here is
    /// not "wrong number", it is "same word down all fifty-two rows", and
    /// that is a property of the distribution rather than of any one row.
    /// `cargo test --release --lib -- --ignored --nocapture what_the_bed_ranges`
    #[test]
    #[ignore = "a readout, not an assertion -- cargo test -- --ignored --nocapture what_the_bed_ranges"]
    fn what_the_bed_ranges() {
        let mut lab = Lab::new(scene::LabBox { colonies: 1, founders: 8, ..scene::LabBox::default() });
        for tile in [900u32, 2000, 4000, 9000] {
            while lab.world.frame < tile as u64 {
                lab.tick();
            }
            let rows = rows(&lab.world, Kingdom::Creatures, SortKey::Slot, false, Filter::All);
            let mut depths: Vec<u16> = rows
                .iter()
                .filter_map(|r| lab.world.organism(r.who.id).map(|st| st.forage_max))
                .collect();
            depths.sort_unstable();
            let q = |p: f64| -> u16 {
                if depths.is_empty() {
                    return 0;
                }
                depths[((depths.len() - 1) as f64 * p) as usize]
            };
            let mut states: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
            for r in &rows {
                *states.entry(r.state.label()).or_default() += 1;
            }
            println!(
                "frame {tile:5}  animals {:3}  forage_max p10 {:4} p50 {:4} p90 {:4} max {:4}   states {states:?}",
                rows.len(),
                q(0.10),
                q(0.50),
                q(0.90),
                depths.last().copied().unwrap_or(0),
            );
        }
    }
}
