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
    /// **An animal that has not touched the nest inside its own memory
    /// window**, so the `AtNest` sense it steers home by has decayed to zero.
    ///
    /// The threshold is `nest_memory` itself rather than a fraction of it,
    /// because that constant is exactly the window: `creature.rs` computes
    /// the sense as `1 - since_nest / nest_memory`, so past it there is no
    /// signal left. `specimen_sections` already says the same thing in words
    /// -- *"a number that only ever climbs is an ant that is lost"*.
    ///
    /// **The first version of this row was `HOME`, on `since_nest <
    /// nest_memory`, and it was vacuous**: the ant's window is 3,000 ticks
    /// and a nine-hundred-frame bed had every animal at 182, so the column
    /// said HOME fifty-two times. Same defect as the hunger floor below it,
    /// found the same way -- by looking at the drawn table.
    Lost,
    Ok,
}

impl RowState {
    pub fn label(self) -> &'static str {
        match self {
            RowState::Senescent => "ROTTING",
            RowState::Starving => "STARVING",
            RowState::Hungry => "HUNGRY",
            RowState::Carrying => "LADEN",
            RowState::Lost => "LOST",
            RowState::Ok => "OK",
        }
    }

    /// Whether this is a state worth being told about -- what the `IN
    /// TROUBLE` filter keeps.
    pub fn is_trouble(self) -> bool {
        matches!(self, RowState::Senescent | RowState::Starving | RowState::Hungry | RowState::Lost)
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
        } else if state.since_nest >= c.nest_memory {
            RowState::Lost
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
