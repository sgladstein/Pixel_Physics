//! **Is the colony starving because the bed's food is gone, or because it
//! never gets to it?** — the separator `open-bugs-handoff.md` §Z6 asks for.
//!
//! §Z6 censused nine 300,000-frame runs and found every shipped bed's colony
//! dead of starvation with the stand still standing, and said in as many
//! words that its own census could not tell the two apart:
//!
//! > Whether the colony *overgrazes* (eats the stand faster than it regrows
//! > while it lives) or *cannot reach* what is there ... are different bugs
//! > with different fixes, and this census cannot tell them apart.
//!
//! The three instruments it named cannot either, and the reason is the
//! scene rather than the counters: `forage_probe` builds a hand-made bank
//! with a food pile at a fixed gap, `stamp_probe` runs outdoor terrain, and
//! `labnest`'s columns are about the *nest* — it reports `roofed`, `packed`
//! and `buried` and has no food census at all. Nothing measures the shipped
//! bed's larder.
//!
//! # What separates them, and why it is a *cumulative* mask
//!
//! A standing count of food cannot answer this. Food is standing in *both*
//! worlds — an overgrazed bed still holds wood, and an unreachable larder is
//! by definition untouched — so `edible` on its own is the same number for
//! opposite findings. What differs is **whether the colony was ever there**:
//!
//! * **Overgrazing** — the ants walk the bed, eat what they find, and the
//!   standing food falls below the unfed control. `visited` covers the bed;
//!   `edible` is far under `colonies=0`.
//! * **Cannot reach** — the standing food is at or near the unfed control's
//!   level, and it is standing in columns no ant ever entered, or at heights
//!   no ant ever climbed to.
//!
//! So the column that decides it is `unvisited` — edible cells standing in a
//! column the colony has *never* occupied over the whole run — beside
//! `eaten` and beside the same bed at `colonies=0`.
//!
//! **The mask is over columns, not cells, and that is deliberate.** An ant
//! reaches into its head's 8-neighbourhood from the surface it walks on, and
//! the surface drifts as litter piles; a cell-exact mask would score a leaf
//! one row above a track as unreached, which is a statement about the
//! bookkeeping and not about the animal. A column mask is the *generous*
//! reading — it credits the colony with everything in any column it ever
//! walked — so a large `unvisited` is a finding that had to survive the
//! reading most likely to erase it.
//!
//! # Controls, in the binary
//!
//! * `colonies=0` — the unfed stand. `eaten` must be 0 and `edible` is the
//!   ceiling every fed run is read against. This is the specificity half.
//! * `handout=N` — a food cell dropped on the colony's doorstep every N
//!   frames, `windfall_probe`'s positive control. Supply without travel: if
//!   the colony lives with it and dies without it, the failure is delivery.
//! * `control=selftest` — the sensitivity half, and the one this file would
//!   be untrustworthy without. It plants a known count of known food at a
//!   known height and a known distance and asserts every band reports it,
//!   because a census reading zero because nothing is there and one reading
//!   zero because it is blind are the same line of output.
//!
//! ```text
//! cargo run --release --example labforage -- frames=300000 seed=1
//! cargo run --release --example labforage -- frames=300000 seed=1 colonies=0
//! cargo run --release --example labforage -- frames=300000 founders=256 colonies=3
//! cargo run --release --example labforage -- control=selftest
//! ```

use pixel_physics::lab::scenario::{Placement, Scenario};
use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::brain;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::creature::{diet_yield, EAT_YIELD_THRESHOLD};
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::field;
use pixel_physics::sim::frame;
use pixel_physics::sim::material::MaterialId;
use pixel_physics::sim::organism::TRAIT_GUT_BIAS;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::pheromone::Channel;
use pixel_physics::sim::plant;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// `hidden=<Input>:<unit>:<weight>[,...]` -- see the call site, which is
/// where the reason it must run before founding lives.
fn hidden_rider() -> Vec<(brain::BrainInput, usize, f32)> {
    let Some(spec) = arg::<String>("hidden") else { return Vec::new() };
    spec.split(',')
        .map(|entry| {
            let bits: Vec<&str> = entry.split(':').collect();
            assert_eq!(bits.len(), 3, "hidden entry {entry:?} wants Input:unit:weight, e.g. hidden=PheroBAlong:2:6.0");
            let input = brain::INPUTS
                .iter()
                .copied()
                .find(|i| brain::INPUT_NAMES[*i as usize].eq_ignore_ascii_case(bits[0]))
                .unwrap_or_else(|| panic!("unknown input {:?}; known: {:?}", bits[0], brain::INPUT_NAMES));
            let unit: usize = bits[1].parse().unwrap_or_else(|_| panic!("hidden unit {:?} does not parse", bits[1]));
            assert!(unit < brain::BRAIN_HIDDEN, "hidden unit {unit} is past BRAIN_HIDDEN ({})", brain::BRAIN_HIDDEN);
            let w: f32 = bits[2].parse().unwrap_or_else(|_| panic!("hidden weight {:?} does not parse", bits[2]));
            (input, unit, w)
        })
        .collect()
}

/// `wire=<Input>:<Output>:<weight>[,...]` -- the input-to-**output** half of
/// the rider above, `labstats`' and `creature_arena`'s own syntax, added here
/// for P2's control arm.
///
/// **It exists because the alternative is the `include_str!` trap.** P2 needs
/// the shipped hopper raced at `(Bias, Impulse, 2.0)` against the flitter,
/// and `hopper.ron` ships 0.5. A species *file* copy carrying the other
/// weight is not "the same binary two arms": species files are
/// `include_str!`-embedded, so the arm is a different build, and this repo
/// has three bit-identical sweeps on record from exactly that. `instruments.
/// md` names this case by name -- *"The hopper's entire reason to exist is a
/// single instinct row, `(Bias, Impulse, 2.0)`, and there was no way to move
/// it without editing `hopper.ron` and rebuilding between arms"* -- and the
/// fix it records is this knob, which `labstats` and `creature_arena`
/// already carry. This is the third harness, deliberately with the same
/// spelling so the three race one set of numbers rather than three
/// transcriptions.
fn wire_rider() -> Vec<(brain::BrainInput, brain::BrainOutput, f32)> {
    let Some(spec) = arg::<String>("wire") else { return Vec::new() };
    spec.split(',')
        .map(|entry| {
            let bits: Vec<&str> = entry.split(':').collect();
            assert_eq!(bits.len(), 3, "wire entry {entry:?} wants Input:Output:weight, e.g. wire=Bias:Impulse:2.0");
            let input = brain::INPUTS
                .iter()
                .copied()
                .find(|i| brain::INPUT_NAMES[*i as usize].eq_ignore_ascii_case(bits[0]))
                .unwrap_or_else(|| panic!("unknown input {:?}; known: {:?}", bits[0], brain::INPUT_NAMES));
            let output = brain::OUTPUTS
                .iter()
                .copied()
                .find(|o| brain::OUTPUT_NAMES[*o as usize].eq_ignore_ascii_case(bits[1]))
                .unwrap_or_else(|| panic!("unknown output {:?}; known: {:?}", bits[1], brain::OUTPUT_NAMES));
            let w: f32 = bits[2].parse().unwrap_or_else(|_| panic!("wire weight {:?} does not parse", bits[2]));
            (input, output, w)
        })
        .collect()
}

/// The gut bias off a live founder, never off the species table -- the run
/// has to be measuring the gut it says it is. `0.0` (neutral) before any
/// ant exists to read one off, which only happens between the bed being
/// built and `ants_at` arriving when `ants_at > 0`.
fn ant_gut_bias(world: &World) -> f32 {
    world
        .live_organism_ids()
        .iter()
        .filter_map(|id| world.organism(*id))
        .find(|s| world.species.get(s.species).creature.is_some())
        .map(|s| s.traits[TRAIT_GUT_BIAS])
        .unwrap_or(0.0)
}

/// How far above the soil surface a cell still counts as **on the floor**.
///
/// Same value and same reasoning as `windfall_probe`'s: an ant is a two-cell
/// chain standing on the ground reaching into its head's 8-neighbourhood, so
/// a couple of rows above the surface is food it takes without climbing, and
/// litter drifts the walking surface upward over a run. Generous on purpose —
/// the finding this harness exists to make is about food out of reach, and a
/// generous floor band makes that finding harder to get rather than easier.
const FLOOR_BAND: i32 = 3;

/// Above this many rows off the soil, food is up a stem rather than on the
/// ground. Between the two bands is what an ant reaches by climbing a little.
const LOW_BAND: i32 = 16;

/// **The pile census** -- round 29, and the instrument the record lacked.
///
/// The owner's playtest report, 2026-09-11, verbatim: *"long ants getting
/// stuck. Not all of them but it happens regularly. It seems like they get
/// stuck in a big group/pile of long ants."* Nothing already in this file
/// can see that. `moves_blocked` says an animal did not move;
/// `boxed_ticks` (§13a) says it had nowhere to go; **neither says whether
/// what it had nowhere to go *past* was rock or its own colony**, and
/// those want opposite fixes -- §13's flip for the first, and something
/// else entirely for the second.
///
/// `creature::head_block` splits one animal's eight headings by what
/// refused each; this accumulates that over the run. Three readings come
/// out of it and they are deliberately separate:
///
/// * **how many animals are boxed at all**, split terrain-only /
///   creature-only / both -- the state;
/// * **the largest connected clump of body-boxed animals** at each stop
///   (`creature::piles_of`) -- the owner's *"big group/pile"*, as a number;
/// * **how long one animal stays body-boxed**, in consecutive stops -- the
///   *"stuck"*, which a per-stop count cannot show. A colony where a
///   different animal is momentarily jammed at every stop and one where
///   the same six have not moved all run read identically on a standing
///   count and are opposite findings.
///
/// **`probe=x,y;x,y;...` -- what stands at a named world cell, stop after
/// stop.** Round 29's second card came back with the owner's finger on
/// three fixed points in the frame: *"this the most prominent thing that
/// shows no movement in both images"*. An aggregate cannot answer that.
/// `Piles` says how many animals are wedged and for how long; it cannot
/// say what is standing at the cell the owner pointed at, and *that* is
/// the question -- a long body waiting on a nestmate, a one-cell body that
/// cannot flip, a corpse, or a plant that was never going to move.
///
/// So this reads the cell directly and names its occupant: the material,
/// and where that cell belongs to a live creature, the owning animal's
/// species, body length, generation, cargo and `head_block` split, plus
/// **whether its head cell has moved since the previous probe stop** --
/// which is the owner's own criterion and not one any counter in this file
/// already reports.
///
/// `CLAUDE.md`'s *ask what your number counts when nothing is wrong*: the
/// positive control is a cell over open air, which must read `empty` and
/// never name an animal, and a cell inside a plant, which must name the
/// material and report no animal at all. Both are in the run's own output
/// whenever the three probed cells are not all occupied.
struct CellProbe {
    cells: Vec<(i32, i32)>,
    every: u64,
    from: u64,
    to: u64,
    /// Per probed cell, the last (organism id, head cell) seen there, so a
    /// line can say `moved` or `STILL` rather than leaving the reader to
    /// diff two coordinates by eye.
    last: Vec<Option<(u16, (i32, i32))>>,
    /// Per probed cell: stops at which it held a creature, and stops at
    /// which that creature's head had not moved since the previous stop.
    occupied: Vec<u64>,
    unmoved: Vec<u64>,
    /// Distinct organism ids ever seen at the cell -- the cheap tell for
    /// "one animal stands here for ever" against "the crowd keeps
    /// rearranging itself and there is always somebody here".
    seen: Vec<std::collections::BTreeSet<u16>>,
    /// Probe stops taken, so the first one can print a full baseline and
    /// the rest print only what changed.
    stops: u64,
    /// **The half a standing census cannot give: is the animal *trying*?**
    /// `LifeCounters::moves` and `moves_blocked` are the animal's own
    /// committed steps and its refused ones, so their change across the
    /// window separates "wedged and shoving" (blocked climbs, moves does
    /// not) from "not asking at all" (neither climbs) -- and those are
    /// different bugs in different files. `CLAUDE.md`'s pairing of an
    /// "it fired" counter with an effect counter, per animal.
    ///
    /// First and last reading per probed cell, each `(id, moves,
    /// moves_blocked, energy)`; compared in `report` only where the id is
    /// the same at both ends, since two animals' counters do not subtract.
    first_seen: Vec<Option<(u16, u32, u32, f32)>>,
    last_seen: Vec<Option<(u16, u32, u32, f32)>>,
    /// **The positive control on the pair above.** `moves` and
    /// `moves_blocked` both flat is the finding; it is also exactly what a
    /// *dead or unticked* animal would read, and those are different
    /// answers. `bites`/`digs`/`deliveries` come off the same `LifeCounters`
    /// and are incremented by `act`, which runs *before* the move section,
    /// so any of them climbing proves the animal's tick is running and it
    /// is doing work -- it simply is not asking to walk.
    /// `(id, bites, digs, deliveries)`, first and last.
    first_act: Vec<Option<(u16, u32, u32, u32)>>,
    last_act: Vec<Option<(u16, u32, u32, u32)>>,
}

impl CellProbe {
    fn new() -> Option<Self> {
        let spec: String = arg("probe")?;
        let cells: Vec<(i32, i32)> = spec
            .split(';')
            .filter_map(|p| {
                let mut it = p.split(',');
                let x = it.next()?.trim().parse().ok()?;
                let y = it.next()?.trim().parse().ok()?;
                Some((x, y))
            })
            .collect();
        if cells.is_empty() {
            eprintln!("labforage: probe={spec} parsed no cells -- expected x,y;x,y");
            return None;
        }
        // **`probebox=N` widens each probed cell to a (2N+1)-square block.**
        // A marker on a review card is a *click*, so it carries a cell or
        // two of aim error, and a probe of the single nearest cell answers
        // a question the owner did not quite ask. The block is the honest
        // reading; the summary below names the cells in it that held an
        // animal, so the aim error shows up as neighbours rather than
        // hiding as a miss.
        let box_r: i32 = arg("probebox").unwrap_or(0);
        let cells: Vec<(i32, i32)> = if box_r <= 0 {
            cells
        } else {
            let mut out = Vec::new();
            for &(x, y) in &cells {
                for dy in -box_r..=box_r {
                    for dx in -box_r..=box_r {
                        out.push((x + dx, y + dy));
                    }
                }
            }
            out
        };
        let n = cells.len();
        Some(Self {
            cells,
            every: arg("probeevery").unwrap_or(60).max(1),
            from: arg("probefrom").unwrap_or(0),
            to: arg("probeto").unwrap_or(u64::MAX),
            last: vec![None; n],
            occupied: vec![0; n],
            unmoved: vec![0; n],
            seen: vec![std::collections::BTreeSet::new(); n],
            stops: 0,
            first_seen: vec![None; n],
            last_seen: vec![None; n],
            first_act: vec![None; n],
            last_act: vec![None; n],
        })
    }

    fn due(&self, f: u64) -> bool {
        f >= self.from && f <= self.to && f.is_multiple_of(self.every)
    }

    fn sample(&mut self, world: &World, f: u64) {
        for i in 0..self.cells.len() {
            let (x, y) = self.cells[i];
            let cell = world.get(x, y);
            let mat = world.materials.get(cell.material).name.clone();
            let id = cell.organism_id();
            let state = (id != 0).then(|| world.organism(id)).flatten();
            let Some(st) = state else {
                // Not a live animal's cell. Printed on the first stop and
                // whenever it *becomes* empty: an empty or plant cell at a
                // spot the owner marked is itself the answer, and it is the
                // control on the creature reading below.
                if self.last[i].is_some() || self.stops == 0 {
                    println!("  probe f{f} ({x},{y}): {mat} -- no live creature (organism_id {id})");
                }
                self.last[i] = None;
                continue;
            };
            let head = st.chain.first().copied().unwrap_or((x, y));
            let moved = match self.last[i] {
                Some((prev_id, prev_head)) => prev_id != id || prev_head != head,
                None => true,
            };
            self.occupied[i] += 1;
            if !moved {
                self.unmoved[i] += 1;
            }
            self.seen[i].insert(id);
            let reading = (id, st.life.moves, st.life.moves_blocked, st.energy);
            if self.first_act[i].is_none() {
                self.first_act[i] = Some((id, st.life.bites, st.life.digs, st.life.deliveries));
            }
            self.last_act[i] = Some((id, st.life.bites, st.life.digs, st.life.deliveries));
            if self.first_seen[i].is_none() {
                self.first_seen[i] = Some(reading);
            }
            self.last_seen[i] = Some(reading);
            let block = pixel_physics::sim::creature::head_block(world, id);
            let (open, by_c, by_o, kin) = block
                .map(|b| (b.open, b.by_creature, b.by_other, b.kin_would_open))
                .unwrap_or((0, 0, 0, 0));
            // **Printed on the first stop and on every change, not every
            // stop.** A block of 75 cells at a 60-frame cadence is 3,750
            // lines of mostly-identical text, and the owner's question is
            // *"does this move"* -- which is exactly the change. The stop
            // counts in `report` carry the "and it never did" half, so
            // nothing is lost by not repeating an unchanged line.
            if !moved && self.stops > 0 {
                self.last[i] = Some((id, head));
                continue;
            }
            println!(
                "  probe f{f} ({x},{y}): {mat} org {id} {} cells{} gen {} {} head ({},{}) {} | open {open} by_creature {by_c} by_other {by_o} kin_would_open {kin}{}",
                st.chain.len(),
                pixel_physics::sim::creature::authored_body_cells(world, id)
                    .map(|a| format!(" (genome {a})"))
                    .unwrap_or_default(),
                st.generation,
                world.species.get(st.species).name,
                head.0,
                head.1,
                if st.crop.is_some() { "LADEN" } else { "empty" },
                if moved { "" } else { "  STILL" },
            );
            println!(
                "         ...energy {:.1} since_nest {} traffic_deferred {} heading {} moves {} blocked {} bites {} digs {} deliveries {}{}{}{}{}",
                st.energy,
                st.since_nest,
                st.traffic_deferred,
                st.heading,
                st.life.moves,
                st.life.moves_blocked,
                st.life.bites,
                st.life.digs,
                st.life.deliveries,
                // The three states that would each explain a standing animal
                // without the brain being involved at all, so a reader does
                // not have to take the deduction on trust.
                if st.crossing.is_some() { " CROSSING" } else { "" },
                if st.flight.is_some() { " IN-FLIGHT" } else { "" },
                if st.senescent { " SENESCENT" } else { "" },
                if st.spoil.is_some() { " CARRYING-SPOIL" } else { "" },
            );
            self.last[i] = Some((id, head));
        }
        self.stops += 1;
    }

    fn report(&self) {
        println!(
            "\n  probe summary -- {} stops every {} frames over {}..{}",
            self.stops, self.every, self.from, self.to
        );
        for (i, &(x, y)) in self.cells.iter().enumerate() {
            if self.occupied[i] == 0 {
                continue;
            }
            let trying = match (self.first_seen[i], self.last_seen[i]) {
                (Some((a_id, a_m, a_b, a_e)), Some((b_id, b_m, b_b, b_e))) if a_id == b_id => format!(
                    " | org {a_id} over the window: moves {a_m}->{b_m} (+{}), blocked {a_b}->{b_b} (+{}), energy {a_e:.1}->{b_e:.1}",
                    b_m.saturating_sub(a_m),
                    b_b.saturating_sub(a_b)
                ),
                _ => String::new(),
            };
            let doing = match (self.first_act[i], self.last_act[i]) {
                (Some((a_id, a_bi, a_d, a_de)), Some((b_id, b_bi, b_d, b_de))) if a_id == b_id => format!(
                    "; while it stood there: bites +{}, digs +{}, deliveries +{}",
                    b_bi.saturating_sub(a_bi),
                    b_d.saturating_sub(a_d),
                    b_de.saturating_sub(a_de)
                ),
                _ => String::new(),
            };
            println!(
                "    ({x},{y}): creature at {}/{} stops, head unmoved at {} of them, {} distinct animal(s): {:?}{trying}{doing}",
                self.occupied[i],
                self.stops,
                self.unmoved[i],
                self.seen[i].len(),
                self.seen[i]
            );
        }
        let never: Vec<(i32, i32)> = self
            .cells
            .iter()
            .enumerate()
            .filter(|(i, _)| self.occupied[*i] == 0)
            .map(|(_, &c)| c)
            .collect();
        println!("    {} of {} probed cell(s) never held a live creature", never.len(), self.cells.len());
    }
}

/// A stuck *duration* is in **sample stops**, not frames, and the summary
/// prints the frame conversion beside it: an animal is only looked at when
/// the sweep stops, so a streak of 3 at `sample=900` means "still stuck
/// 1,800 frames later", not "stuck for 3 frames".
#[derive(Default)]
struct Piles {
    stops: u64,
    /// Per-animal readings summed over stops -- a rate's denominator, not
    /// a head count.
    animal_stops: u64,
    boxed: u64,
    terrain_only: u64,
    creature_only: u64,
    both: u64,
    body_boxed: u64,
    /// ...split by what the animal *is*, which turned out to matter more
    /// than anything else the census reports. A body of three cells or
    /// more is the one §13c is about -- it cannot reverse by walking. One
    /// or two cells is a newborn that has not grown its body yet (or the
    /// shipped two-cell ant), and for it the flip is a **no-op**: reversing
    /// a list of one changes nothing, so the delivery test refuses it every
    /// tick for ever. Two populations, two different reasons, and a single
    /// `body_boxed` total cannot tell them apart.
    body_boxed_long: u64,
    body_boxed_short: u64,
    /// ...and, of the short ones, **which of the two ways they got that
    /// way** -- the fact round 30 needs and a standing count cannot give.
    /// `short_by_genome` is a body whose own `FateGenome` unfolds to one or
    /// two cells: a short morph, which a colony thirty generations deep can
    /// evolve, since the fates table is heritable. `short_by_loss` was born
    /// long and lost cells to a bite, a dig or a rock. Different bugs, for
    /// different lanes. `creature::authored_body_cells` is the reading.
    short_by_genome: u64,
    short_by_loss: u64,
    /// The deepest generation seen among the short body-boxed animals --
    /// the tell for the morph reading, since an evolved body plan cannot
    /// appear in generation 0.
    short_max_generation: u16,
    /// ...and how many of the body-boxed were carrying, which is the other
    /// fork: a laden animal's flip is withheld by the traffic gate, an
    /// empty-handed one's is attempted and refused.
    body_boxed_laden: u64,
    /// Stops at which some clump of body-boxed animals reached 3.
    pile_stops: u64,
    /// The largest clump seen all run, and where and when, so a card can
    /// be cropped on it rather than on a guess.
    largest: usize,
    largest_at: (u64, i32, i32),
    /// Live streak per organism id, and the completed ones as a histogram
    /// of length -> count.
    live: std::collections::HashMap<u16, (u32, bool)>,
    hist: std::collections::BTreeMap<u32, u64>,
    /// The same histogram over bodies of three cells or more alone.
    hist_long: std::collections::BTreeMap<u32, u64>,
    max_streak: u32,
    max_streak_long: u32,
    /// The largest clump counting only bodies of three cells or more.
    largest_long: usize,
    /// **Round 30's starting facts about the short bodies, and nothing
    /// more.** This branch does not touch what produces them; the split
    /// above says *how many* and these say *what they are*, which is the
    /// question a fix would have to open with.
    ///
    /// `genome_sum` / `chain_sum` are summed over short body-boxed
    /// readings, so their ratio is "cells the genome asks for" against
    /// "cells the animal has" -- the cells-lost figure, as a mean rather
    /// than a count of animals. `first_age_*` is the animal's age in frames
    /// the first time it is ever seen body-boxed, which says whether a
    /// short body is wedged from birth or gets that way later; the same
    /// pair is kept for long bodies as the paired control, because an age
    /// with nothing to compare it against says nothing.
    short_genome_sum: u64,
    short_chain_sum: u64,
    short_first_age_sum: u64,
    short_first_age_n: u64,
    long_first_age_sum: u64,
    long_first_age_n: u64,
    /// Ids already counted into `first_age_*`, so each animal contributes
    /// its *first* boxed tick once rather than every stop of its streak.
    first_boxed_seen: std::collections::HashSet<u16>,
    /// **The population the pile census cannot see, and the one the owner's
    /// own markers landed on.** `body_boxed` requires `open == 0`; an animal
    /// with somewhere to go is excluded by construction, however long it
    /// stands there. Round 29's second card came back with three markers on
    /// three full-length long ants reading `open` 3, 1 and 1 -- none of them
    /// in any column of this census, in either arm.
    ///
    /// `idle_with_room` counts per-animal readings where the animal had at
    /// least one legal heading and its head had not moved since the previous
    /// stop; `idle_with_room_long` is the same over bodies of three cells or
    /// more, which is what the owner was looking at. `moving` is the paired
    /// denominator -- readings where the head *did* move -- so the pair is a
    /// rate rather than a count that grows with the colony.
    ///
    /// `CLAUDE.md`'s *ask what your number counts when nothing is wrong*: on
    /// a bed whose animals are all walking this reads 0, and the two-cell ant
    /// control is the specificity check the same way it is for `body_boxed`.
    idle_with_room: u64,
    idle_with_room_long: u64,
    moving: u64,
    /// Head cell per animal at the previous stop, so "did not move" is a
    /// measurement rather than an inference from `moves_blocked` -- which is
    /// exactly the counter that stays flat for this population.
    heads: std::collections::HashMap<u16, (i32, i32)>,
    /// **The rate above is not the finding, and its own control says so.**
    /// Measured 2026-09-12 on the shipped two-cell ant (`played_bed`, seed
    /// 3, 30,400 frames): `idle_with_room` **4,543 of 6,040 readings, 75%**,
    /// against 74-76% for the long ant on the same scene shape. An ant that
    /// is not walking this instant is an ordinary ant in both species, and a
    /// rate cannot tell that apart from the thing the owner reported --
    /// `CLAUDE.md`'s *ask what your number counts when nothing is wrong*,
    /// caught by running the control before trusting the number.
    ///
    /// What separates them is **duration**: a two-cell ant resting across
    /// one stop is invisible, and a seven-cell body holding one cell for a
    /// whole 2,400-frame window is what a player points at. So this is the
    /// same streak machinery `hist`/`max_streak` runs over `body_boxed`, run
    /// instead over "had somewhere to go and did not go", for bodies of
    /// three cells or more.
    idle_live: std::collections::HashMap<u16, u32>,
    idle_hist: std::collections::BTreeMap<u32, u64>,
    idle_max_streak: u32,
    /// **The same streak over *every* body size, because the 3+-cell one
    /// cannot be controlled.** The shipped two-cell ant is the specificity
    /// control for everything else in this census, and for the pair above it
    /// is **vacuous**: a two-cell body never satisfies `chain.len() >= 3`,
    /// so the control reads 0 by construction whether or not a shipped ant
    /// stands still for minutes. `CLAUDE.md`'s *a change that moves nothing
    /// is different evidence from one that moves a little* -- an
    /// always-zero control is not a control. These three are the same
    /// measure with the body-length gate removed, so `played_bed` produces a
    /// real number and the long ant's tail has something to be long
    /// *against*.
    idle_live_any: std::collections::HashMap<u16, u32>,
    idle_hist_any: std::collections::BTreeMap<u32, u64>,
    idle_max_streak_any: u32,
}

impl Piles {
    /// One stop. Returns the largest clump at this instant, for the
    /// per-stop line.
    /// One stop, with `follow` asking for a line about the animal that
    /// has been body-boxed longest -- the "follow one stuck animal" half.
    /// An aggregate cannot say *why* a pile holds; only one animal's own
    /// eight headings, its cargo and its facing can, and those are the
    /// three things the two candidate mechanisms differ on.
    fn sample(&mut self, world: &World, f: u64, follow: bool) -> usize {
        self.stops += 1;
        let mut next_heads: std::collections::HashMap<u16, (i32, i32)> = std::collections::HashMap::new();
        let mut next_idle: std::collections::HashMap<u16, u32> = std::collections::HashMap::new();
        let mut next_idle_any: std::collections::HashMap<u16, u32> = std::collections::HashMap::new();
        let mut members: Vec<u16> = Vec::new();
        let mut reads: Vec<pixel_physics::sim::creature::HeadBlock> = Vec::new();
        let mut longs: Vec<bool> = Vec::new();
        for id in world.live_organism_ids() {
            // `None` for anything that is not a creature -- every plant in
            // the bed, and the great majority of the organism table.
            let Some(b) = pixel_physics::sim::creature::head_block(world, id) else { continue };
            self.animal_stops += 1;
            if b.boxed() {
                self.boxed += 1;
                if b.by_creature == 0 {
                    self.terrain_only += 1;
                } else if b.by_other == 0 {
                    self.creature_only += 1;
                } else {
                    self.both += 1;
                }
            }
            // **Every animal, not only the boxed ones** -- the whole point
            // is that this population is not boxed. Read before the
            // `body_boxed` branch below so the two are independent.
            let head_now = world.organism(id).and_then(|st| st.chain.first().copied());
            if let Some(h) = head_now {
                let stood = self.heads.get(&id) == Some(&h);
                let long_body = world.organism(id).is_some_and(|st| st.chain.len() >= 3);
                if stood {
                    if b.open > 0 {
                        self.idle_with_room += 1;
                        let any = self.idle_live_any.get(&id).copied().unwrap_or(0) + 1;
                        self.idle_max_streak_any = self.idle_max_streak_any.max(any);
                        next_idle_any.insert(id, any);
                        if long_body {
                            self.idle_with_room_long += 1;
                            let n = self.idle_live.get(&id).copied().unwrap_or(0) + 1;
                            self.idle_max_streak = self.idle_max_streak.max(n);
                            next_idle.insert(id, n);
                        }
                    }
                } else {
                    self.moving += 1;
                }
                next_heads.insert(id, h);
            }
            if b.body_boxed() {
                self.body_boxed += 1;
                let long = world.organism(id).is_some_and(|st| st.chain.len() >= 3);
                if long {
                    self.body_boxed_long += 1;
                } else {
                    self.body_boxed_short += 1;
                    let authored = pixel_physics::sim::creature::authored_body_cells(world, id).unwrap_or(0);
                    if authored <= 2 {
                        self.short_by_genome += 1;
                    } else {
                        self.short_by_loss += 1;
                    }
                    if let Some(st) = world.organism(id) {
                        self.short_max_generation = self.short_max_generation.max(st.generation);
                        self.short_genome_sum += authored as u64;
                        self.short_chain_sum += st.chain.len() as u64;
                    }
                }
                // **Age at the *first* boxed tick, once per animal.** Kept
                // for both populations: the short bodies are the question
                // and the long ones are the control.
                if self.first_boxed_seen.insert(id) {
                    if let Some(st) = world.organism(id) {
                        let age = f.saturating_sub(st.born_frame);
                        if long {
                            self.long_first_age_sum += age;
                            self.long_first_age_n += 1;
                        } else {
                            self.short_first_age_sum += age;
                            self.short_first_age_n += 1;
                        }
                    }
                }
                if world.organism(id).is_some_and(|st| st.crop.is_some()) {
                    self.body_boxed_laden += 1;
                }
                members.push(id);
                longs.push(long);
                reads.push(b);
            }
        }
        // **Streaks close when an animal stops being body-boxed**, which
        // includes it dying: a dead handle is simply absent from
        // `members`, and the streak it was on is a real completed streak
        // rather than a censored one. Ids are reused when a slot is freed,
        // so a very long streak on a busy bed could in principle be two
        // animals' -- noted rather than defended against, because the
        // alternative is a second identity scheme for a diagnostic.
        let mut next: std::collections::HashMap<u16, (u32, bool)> = std::collections::HashMap::new();
        for (i, &id) in members.iter().enumerate() {
            let n = self.live.get(&id).map(|&(n, _)| n).unwrap_or(0) + 1;
            let long = longs[i];
            self.max_streak = self.max_streak.max(n);
            if long {
                self.max_streak_long = self.max_streak_long.max(n);
            }
            next.insert(id, (n, long));
        }
        for (id, &(n, long)) in &self.live {
            if !next.contains_key(id) {
                *self.hist.entry(n).or_default() += 1;
                if long {
                    *self.hist_long.entry(n).or_default() += 1;
                }
            }
        }
        self.live = next;
        // Swapped wholesale, so an animal that died has no stale head and
        // its id being reused cannot read as "stood still".
        self.heads = next_heads;
        // A streak closes the moment the animal moves, dies, or drops below
        // three cells -- all three are simply an id absent from `next_idle`,
        // and all three are real ends of standing still.
        for (id, &n) in &self.idle_live {
            if !next_idle.contains_key(id) {
                *self.idle_hist.entry(n).or_default() += 1;
            }
        }
        self.idle_live = next_idle;
        for (id, &n) in &self.idle_live_any {
            if !next_idle_any.contains_key(id) {
                *self.idle_hist_any.entry(n).or_default() += 1;
            }
        }
        self.idle_live_any = next_idle_any;

        // **One animal, named** -- the longest live streak at this stop,
        // with everything the two candidate mechanisms differ on: whether
        // it is carrying (the laden deferral never expires), and whether
        // the *other* end of it is free (a flip that cannot deliver is
        // refused and the animal tumbles for ever). `CLAUDE.md`: an image
        // says what and where, a counter says whether it fired, and only
        // a named individual says why.
        if follow {
            if let Some((&id, &(n, _))) = self.live.iter().max_by_key(|(id, (n, _))| (*n, std::cmp::Reverse(**id))) {
                if let Some(st) = world.organism(id) {
                    let (hx, hy) = st.chain.first().copied().unwrap_or((0, 0));
                    let (tx, ty) = st.chain.last().copied().unwrap_or((0, 0));
                    let b = members.iter().position(|&m| m == id).map(|i| reads[i]).unwrap_or_default();
                    println!(
                        "  stuck f{f}: org {id} head ({hx},{hy}) tail ({tx},{ty}) heading {} cells {} | {n} stop(s) body-boxed | {} | open {} by_creature {} by_other {} kin_would_open {}",
                        st.heading,
                        st.chain.len(),
                        if st.crop.is_some() { "LADEN (the flip is deferred while a nestmate is in the way)" } else { "empty-handed (the flip is allowed to fire)" },
                        b.open,
                        b.by_creature,
                        b.by_other,
                        b.kin_would_open
                    );
                }
            }
        }

        let piles = pixel_physics::sim::creature::piles_of(world, &members);
        let largest = piles.first().map(Vec::len).unwrap_or(0);
        let long_members: Vec<u16> = members.iter().zip(&longs).filter(|(_, &l)| l).map(|(&id, _)| id).collect();
        self.largest_long = self.largest_long.max(pixel_physics::sim::creature::piles_of(world, &long_members).first().map(Vec::len).unwrap_or(0));
        if largest >= 3 {
            self.pile_stops += 1;
        }
        if largest > self.largest {
            let head = piles[0]
                .iter()
                .filter_map(|&id| world.organism(id).and_then(|s| s.chain.first().copied()))
                .next()
                .unwrap_or((0, 0));
            self.largest = largest;
            self.largest_at = (f, head.0, head.1);
        }
        largest
    }

    /// Fold the still-running streaks into the histogram, so the tail is
    /// the run's and not "the streaks that happened to end".
    fn close(&mut self) {
        let live: Vec<(u32, bool)> = self.live.values().copied().collect();
        for (n, long) in live {
            *self.hist.entry(n).or_default() += 1;
            if long {
                *self.hist_long.entry(n).or_default() += 1;
            }
        }
        self.live.clear();
    }

    fn streak_total(&self) -> u64 {
        self.hist.values().sum()
    }

    /// The median completed streak, in stops. 0 when nothing was ever
    /// body-boxed -- which is the two-cell ant's expected reading and is
    /// why it is a number rather than a `None`.
    fn median_streak(&self) -> u32 {
        Self::median_of(&self.hist)
    }

    fn median_streak_long(&self) -> u32 {
        Self::median_of(&self.hist_long)
    }

    /// The 90th percentile of the *idle-with-room* streak histogram, in
    /// sample stops. An order statistic rather than a mean, per `CLAUDE.md`:
    /// this distribution is the long tail, and a mean over a population that
    /// is 75% one-stop rests says nothing about the animal a player is
    /// pointing at.
    fn idle_streak_p90(&self) -> u32 {
        Self::p90_of(&self.idle_hist)
    }

    /// The control's own p90 -- see `idle_live_any`.
    fn idle_streak_p90_any(&self) -> u32 {
        Self::p90_of(&self.idle_hist_any)
    }

    fn p90_of(hist: &std::collections::BTreeMap<u32, u64>) -> u32 {
        let total: u64 = hist.values().sum();
        if total == 0 {
            return 0;
        }
        let mut seen = 0u64;
        for (&len, &n) in hist {
            seen += n;
            if seen * 10 >= total * 9 {
                return len;
            }
        }
        hist.keys().next_back().copied().unwrap_or(0)
    }

    fn median_of(hist: &std::collections::BTreeMap<u32, u64>) -> u32 {
        let total: u64 = hist.values().sum();
        if total == 0 {
            return 0;
        }
        let mut seen = 0u64;
        for (&len, &n) in hist {
            seen += n;
            if seen * 2 >= total {
                return len;
            }
        }
        0
    }

    fn print(&self, sample_every: u64, st: &pixel_physics::sim::world::CreatureStats) {
        println!(
            "  pile census: {} stops, {} animal-readings | boxed {} ({:.1}%) = terrain {} + creature {} + both {} | body-boxed {} ({:.1}%)",
            self.stops,
            self.animal_stops,
            self.boxed,
            100.0 * self.boxed as f64 / self.animal_stops.max(1) as f64,
            self.terrain_only,
            self.creature_only,
            self.both,
            self.body_boxed,
            100.0 * self.body_boxed as f64 / self.animal_stops.max(1) as f64
        );
        println!(
            "    ...of those body-boxed: {} are bodies of 3+ cells (the ones §13c is about) and {} are 1-2 cells (for which a flip is a no-op) | {} were carrying",
            self.body_boxed_long, self.body_boxed_short, self.body_boxed_laden
        );
        println!(
            "    ...and of the 1-2 cell ones: {} were BORN short (their own fate genome unfolds to 1-2 cells -- an evolved morph) and {} were born long and LOST cells | deepest generation among them {}",
            self.short_by_genome, self.short_by_loss, self.short_max_generation
        );
        println!(
            "    largest pile all run: {} animals at frame {} near ({},{}) ({} counting 3+-cell bodies alone) | stops holding a pile of 3+: {} of {}",
            self.largest, self.largest_at.0, self.largest_at.1, self.largest_at.2, self.largest_long, self.pile_stops, self.stops
        );
        println!(
            "    stuck-duration histogram, consecutive stops body-boxed ({} frames per stop); {} streaks, median {}, max {}:",
            sample_every,
            self.streak_total(),
            self.median_streak(),
            self.max_streak
        );
        if self.hist.is_empty() {
            println!("      (none -- no animal was ever boxed by another animal's body)");
        }
        for (&len, &n) in &self.hist {
            println!(
                "      {len:>4} stop(s) = {:>8} frames: {n} (of which {} are 3+-cell bodies)",
                len as u64 * sample_every,
                self.hist_long.get(&len).copied().unwrap_or(0)
            );
        }
        println!(
            "    ...3+-cell bodies alone: {} streaks, median {}, max {}",
            self.hist_long.values().sum::<u64>(),
            self.median_streak_long(),
            self.max_streak_long
        );
        // **Why a boxed animal did not get out** -- the reversal rule's own
        // three exits, printed here rather than left in the SUMMARY soup
        // because they are the diagnosis and the census above is only the
        // symptom. `deferred` is the laden traffic gate (§13g) declining to
        // flip; `refused` is a flip attempted and rejected because the
        // reversed body would be boxed too -- both ends walled in, which on
        // a pile is the common case; `reversals` is the verb working.
        println!(
            "    reversal exits: fired {} (carrying {}, at nest {}) | refused {} (the far end was boxed too) | traffic-deferred {} (laden, a nestmate in the way) | blocked ticks {}",
            st.reversals, st.reversals_carrying, st.reversals_at_nest, st.reversals_refused, st.reversals_traffic_deferred, st.moves_blocked
        );
    }
}

/// Distance bands from the nearest nest column, in cells. `larder_probe`'s
/// shape: a quantity present in the world and a quantity concentrated where
/// the animals are are different findings, and only a banded census separates
/// them.
const DIST_BANDS: [i32; 4] = [16, 48, 128, i32::MAX];

#[derive(Default, Clone, Copy)]
struct Sample {
    /// Standing cells this gut would eat, by the mouth's own predicate.
    edible: usize,
    /// ...of those, within `FLOOR_BAND` rows of the soil surface.
    floor: usize,
    /// ...between `FLOOR_BAND` and `LOW_BAND` — reachable by a short climb.
    low: usize,
    /// ...above `LOW_BAND` — up a stem.
    aloft: usize,
    /// ...standing in a column no ant has occupied at any point in the run.
    unvisited: usize,
    /// Face value of every edible cell standing, in joules, at this gut.
    worth: f64,
    /// Edible cells by distance band from the nearest nest column.
    by_dist: [usize; DIST_BANDS.len()],
    ants: usize,
    /// Deepest row above the soil any ant head is standing at.
    ant_high: i32,
    /// **Living animals that have produced at least one child** -- the
    /// breeders, in the sense `creature::try_bud`'s suppression uses.
    ///
    /// This is the "did it fire at all" counter for a breeding regime, and
    /// nothing else in the run can stand in for it. Under `queen` it must
    /// sit at one per colony; under `individual` it climbs with the
    /// population. A regime that reads as working from the birth count
    /// alone, with this flat at zero, never suppressed anything -- the
    /// collapse that was read as "chunks are working" while the body count
    /// was zero for the whole run.
    breeders: usize,
    /// Deepest generation any animal has reached, ever -- sterile workers
    /// included.
    gen: u16,
    /// Deepest generation of an animal that has ITSELF reproduced: the
    /// chain a genome actually travels.
    ///
    /// Printed beside `gen` rather than instead of it, because the two
    /// diverging is the whole tell. Under queen-only breeding `gen` counts
    /// workers that are genetic dead ends and reads one step deeper than
    /// the line; if `gen` climbs while this sits still, the arm is
    /// manufacturing dead ends and its generation count is answering a
    /// different question from the one asked.
    bgen: u16,
    /// **Live plant organisms** -- `world.live_organism_ids()` filtered to
    /// species with no `creature` component, the plant-side twin of `ants`.
    /// M2's brief (`Reports/lanes/evolution-lab-ecology-measure-2.md`) asks
    /// "does the thicket grow at all", and a stand that never establishes
    /// answers that before any fruit or bite count does.
    plants: usize,
    /// **Standing `windfall` cells, raw** -- counted off `cell.material`
    /// *before* the `diet_yield`/`EAT_YIELD_THRESHOLD` gate `edible` is
    /// filtered through, so this is a physical count of fallen fruit on the
    /// ground regardless of whether the founders' own gut would eat it. The
    /// loop that fills `edible` already visits every cell; this rides along
    /// rather than re-sweeping the grid.
    windfall: usize,
    /// **Standing `flower`/`fruit` cells, raw** -- the rebloom brief's own
    /// "the effect" pair beside `World::flowers_rebloomed`'s "it fired"
    /// (`Reports/evolution-lab-pollinator-design-2026-09-10.md`'s rebloom
    /// extension). A physical material census, same shape as `windfall`
    /// above and for the same reason: `edible` is gated on the founders'
    /// gut and would go to zero at a pure-carrion bias while the bed is
    /// visibly still in flower.
    standing_flowers: usize,
    standing_fruit: usize,
}

/// **Round 28's garden-loop instrument** — the fruit → animal → nest →
/// seedling brief's hypothesis (b): does windfall land somewhere the
/// colony's own foot traffic never reaches? `Reports/lanes/evolution-lab-
/// garden-loop.md`.
///
/// Accumulated across every stop and never cleared, the same convention
/// `visited` uses — so `windfall_heat`/`ant_heat` are a whole-run exposure,
/// not one frame's snapshot, and the two can be read against each other
/// column by column without a second grid sweep: `census`'s own loop
/// already visits every cell once per stop, so the windfall side rides
/// along with it instead of duplicating it (`mark_visited` gets the ant
/// side the same way, since it already walks every ant's chain once a
/// frame). One `Garden` per run, not per stop.
struct Garden {
    /// Per-column: how many stops found a standing `windfall` cell there.
    windfall_heat: Vec<u32>,
    /// Per-column: how many frames found any part of an ant's body there
    /// -- `mark_visited`'s own boolean turned into a count over the same
    /// loop, so this is exposure-time, not a distinct census.
    ant_heat: Vec<u32>,
    /// Standing windfall by height band, same thresholds `Sample::floor`/
    /// `low`/`aloft` use, but un-gated by diet -- a windfall census, not
    /// an edibility one.
    windfall_floor: u64,
    windfall_low: u64,
    windfall_aloft: u64,
    /// Standing windfall by distance from the nearest nest column, same
    /// bands `Sample::by_dist` uses.
    windfall_by_dist: [u64; DIST_BANDS.len()],
    /// **A light-based "under canopy" rather than the played bed's four
    /// scrambler columns hardcoded** — `plant::ambient_light_above` below
    /// half of open-sky noon-equivalent (`field::MAX_LIGHT`), so the same
    /// instrument reads any bed rather than only this one. Counts of
    /// stop-samples, not of distinct fruit (a fruit standing through ten
    /// stops is counted ten times, same as `windfall_heat`).
    windfall_shaded: u64,
    windfall_open: u64,
}

impl Garden {
    fn new(width: usize) -> Self {
        Garden {
            windfall_heat: vec![0; width],
            ant_heat: vec![0; width],
            windfall_floor: 0,
            windfall_low: 0,
            windfall_aloft: 0,
            windfall_by_dist: [0; DIST_BANDS.len()],
            windfall_shaded: 0,
            windfall_open: 0,
        }
    }
}

/// **What is standing here that this gut would eat, and where.**
///
/// Priced through `creature::diet_yield` and gated on
/// `EAT_YIELD_THRESHOLD` — the mouth's own predicate, called rather than
/// re-derived, because a readout that decides for itself what counts as food
/// is the standing house failure (`food_value`'s own doc records the eat verb
/// and the overlay disagreeing about what a cell is worth).
///
/// Swept over the grid rather than over the organism registry, unlike
/// `windfall_probe`'s: loose litter, corpses, fallen leaves and spoil are not
/// organism-owned and are exactly the food a walking ant meets. The sweep is
/// 512x320 per sample and the default interval is 900 frames.
///
/// Eight material-id/behaviour parameters is over clippy's bare threshold;
/// bundling the three raw material censuses (`windfall`/`flower`/`fruit`)
/// into a struct for this one caller would cost a name at every call site for
/// no reader this function has, so the lint is silenced rather than the
/// signature contorted -- the same call `src/worldgen/cave.rs` and
/// `src/sky.rs` already make for functions with several independent
/// per-call parameters.
#[allow(clippy::too_many_arguments)]
fn census(
    world: &World,
    spec: &LabBox,
    gut: f32,
    visited: &[bool],
    nest_cols: &[i32],
    windfall_id: Option<MaterialId>,
    flower_id: Option<MaterialId>,
    fruit_id: Option<MaterialId>,
    garden: &mut Garden,
) -> Sample {
    let mut s = Sample {
        ant_high: i32::MIN,
        gen: world.deepest_animal_generation,
        bgen: world.deepest_breeder_generation,
        ..Sample::default()
    };
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_some() {
            s.ants += 1;
            if state.children > 0 {
                s.breeders += 1;
            }
            if let Some(&(_, hy)) = state.chain.first() {
                s.ant_high = s.ant_high.max(spec.ground_y - hy);
            }
        } else {
            s.plants += 1;
        }
    }
    if s.ant_high == i32::MIN {
        s.ant_high = 0;
    }
    for y in 0..spec.height {
        for x in 0..spec.width {
            let cell = world.get(x, y);
            if windfall_id.is_some_and(|wid| cell.material == wid) {
                s.windfall += 1;
                // **Round 28's garden-loop instrument, riding this same
                // sweep rather than a second one.** `Reports/lanes/
                // evolution-lab-garden-loop.md` hypothesis (b): where does
                // fruit actually land, against where the colony actually
                // walks (`garden.ant_heat`, filled by `mark_visited`)?
                garden.windfall_heat[x.clamp(0, spec.width - 1) as usize] += 1;
                let wf_above = spec.ground_y - y;
                if wf_above <= FLOOR_BAND {
                    garden.windfall_floor += 1;
                } else if wf_above <= LOW_BAND {
                    garden.windfall_low += 1;
                } else {
                    garden.windfall_aloft += 1;
                }
                let wf_d = nest_cols.iter().map(|c| (c - x).abs()).min().unwrap_or(i32::MAX);
                for (b, &edge) in DIST_BANDS.iter().enumerate() {
                    if wf_d <= edge {
                        garden.windfall_by_dist[b] += 1;
                        break;
                    }
                }
                if plant::ambient_light_above(world, x, y) < field::MAX_LIGHT / 2.0 {
                    garden.windfall_shaded += 1;
                } else {
                    garden.windfall_open += 1;
                }
            }
            if flower_id.is_some_and(|fid| cell.material == fid) {
                s.standing_flowers += 1;
            }
            if fruit_id.is_some_and(|fid| cell.material == fid) {
                s.standing_fruit += 1;
            }
            let yielded = diet_yield(world, cell, gut);
            if yielded <= EAT_YIELD_THRESHOLD {
                continue;
            }
            // A living ant is not larder. The colony does not eat its own
            // (`is_living_kin`), and counting standing ants as food would put
            // the answer inside the question — a bed whose colony is dying
            // would read as a bed getting richer.
            if world.organism(cell.organism_id()).is_some_and(|st| world.species.get(st.species).creature.is_some()) {
                continue;
            }
            s.edible += 1;
            s.worth += yielded as f64;
            let above = spec.ground_y - y;
            if above <= FLOOR_BAND {
                s.floor += 1;
            } else if above <= LOW_BAND {
                s.low += 1;
            } else {
                s.aloft += 1;
            }
            if !visited[x.clamp(0, spec.width - 1) as usize] {
                s.unvisited += 1;
            }
            let d = nest_cols.iter().map(|c| (c - x).abs()).min().unwrap_or(i32::MAX);
            for (b, &edge) in DIST_BANDS.iter().enumerate() {
                if d <= edge {
                    s.by_dist[b] += 1;
                    break;
                }
            }
        }
    }
    s
}

/// **`no_colony=1` — the colony-removed control the rebloom brief's own
/// paired reading needs**, without touching a scenario file. A scenario's
/// `Colony`/`Colonies` timeline entries are unconditional
/// (`lab::scenario::apply_colony` reads nothing off `LabBox::colonies`), so
/// there is no flag on the scenario side to suppress arrival with; this
/// clears every creature organism's cells the instant they land instead,
/// which is public-API-only (no `pub(crate)` reached) and leaves the bed
/// itself, including whatever the colony would have eaten, untouched.
///
/// Cells rather than the organism slot: `World::free_organism` is
/// `pub(crate)` and not reachable from an example, but an organism whose
/// cell list has gone empty is reclaimed by the very next `step_organisms`
/// pass on its own (`push_organism`'s own doc on the reclaim rule) -- so
/// clearing the grid is both sufficient and the only public lever.
fn strip_colony(world: &mut World) -> usize {
    let mut cleared = 0usize;
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_none() {
            continue;
        }
        let positions: Vec<(i32, i32)> = state.cells.keys().copied().collect();
        for (x, y) in positions {
            world.set(x, y, Cell::EMPTY);
        }
        cleared += 1;
    }
    cleared
}

/// Every column any ant head has stood in, ever. Cumulative — the mask is
/// never cleared, so `unvisited` is a claim about the whole run rather than
/// about this frame.
///
/// **`heat` rides the same loop** (round 28's garden-loop instrument,
/// `Garden::ant_heat`) rather than a second walk of `live_organism_ids`:
/// every column any part of any ant's body occupies this frame gets one
/// count, so a column's total is exposure-time in frames, comparable
/// column-by-column against `census`'s `windfall_heat` at the stop
/// cadence. Called every frame (not gated by `sample_every`), so this is a
/// *finer* census than "sampled every N frames" asks for, not a coarser
/// one.
fn mark_visited(world: &World, visited: &mut [bool], heat: &mut [u32], width: i32) {
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_none() {
            continue;
        }
        for &(x, _) in &state.chain {
            if (0..width).contains(&x) {
                visited[x as usize] = true;
                heat[x as usize] += 1;
            }
        }
    }
}

fn main() {
    let control: String = arg("control").unwrap_or_else(|| "run".to_string());
    let frames: u64 = arg("frames").unwrap_or(300_000);
    let sample_every: u64 = arg("sample").unwrap_or(900);
    let handout: u64 = arg("handout").unwrap_or(0);
    // **`no_colony=1` -- the colony-removed control, on a scenario or off
    // one alike.** See `strip_colony`'s own doc for why this is a
    // post-arrival sweep rather than a flag on the founding call: a
    // scenario's own timeline places its colony unconditionally.
    let no_colony: bool = arg::<u32>("no_colony").unwrap_or(0) != 0;
    // **Found the colony at frame `ants_at` instead of at frame 0** --
    // `labshot.rs`'s own knob and the same owner framing, 2026-09-09: a bed
    // grown first and stocked later is how the game is actually played. 0
    // (the default) keeps this file's existing behaviour byte-for-byte --
    // founding happens before the loop, exactly as it always has.
    let ants_at: u64 = arg("ants_at").unwrap_or(0);
    // **`scenario=<name>` builds the whole bed from a saved scenario**, the
    // way `labshot` already does, and for the reason the played bed forced:
    // the flags below spread ONE species evenly, and the owner's bed is a
    // mix laid out by column. `lab::scenario` already places arbitrary
    // species at arbitrary columns and already founds colonies on a
    // timeline, so a species mix wanted a data file rather than another
    // knob here. A bad name refuses at load rather than running the default
    // bed under the wrong label -- the "an unknown argument is silently
    // ignored" shape `CLAUDE.md` names.
    let scenario: Option<Scenario> = arg::<String>("scenario").map(|n| {
        let mut sc = Scenario::load(&n).unwrap_or_else(|e| {
            eprintln!("scenario {n}: {e}");
            std::process::exit(2);
        });
        // **`seed=` overrides the scenario's own bed seed, and it has to be
        // done HERE, on the scenario, not on the `spec` below.**
        //
        // `Scenario::build` reads `self.bed`, so a seed applied only to the
        // local `spec` reaches the census and nothing else: the world is
        // built at the file's pinned seed every time. Caught by three seeds
        // returning a **byte-identical** sample row -- `CLAUDE.md`'s
        // "identical output across a change that must have moved something",
        // and it was one command away from turning an eighteen-run sweep
        // into three runs reported six times.
        if let Some(sd) = arg::<u64>("seed") {
            sc.bed.seed = sd;
        }
        // **`creature=<species>` overrides who a scenario's `Colony`/
        // `Colonies` placements found, the same shape `seed=` above is and
        // for the same reason: a scenario's `.ron` bakes one species into
        // its timeline (`played_bed.ron`'s `Colony(species: "ant", ...)`),
        // so racing a second species on the owner's own played bed needs an
        // override here rather than a second copy of the file --
        // `Reports/evolution-lab-pollinator-design-2026-09-10.md` P1's
        // three-arm measurement is exactly this: the played bed, once with
        // its own ant and once with a bloom-wired species, same seed, same
        // everything else.
        if let Some(species) = arg::<String>("creature") {
            sc.bed.colony_species = species.clone();
            for p in sc.placements.iter_mut().chain(sc.timeline.iter_mut().map(|e| &mut e.what)) {
                match p {
                    Placement::Colony { species: s, .. } | Placement::Colonies { species: s, .. } => *s = species.clone(),
                    _ => {}
                }
            }
        }
        sc
    });
    let spec = match &scenario {
        // The scenario's own bed, so `ground_y`, `width` and the soil the
        // census reads are the ones its placements were authored against --
        // **except the seed, which `seed=` still overrides.**
        //
        // Without that override a scenario pins its own `bed.seed` and every
        // run of a six-seed sweep is the same world six times. It announces
        // itself as three seeds reporting an identical founder count, which
        // is `CLAUDE.md`'s "identical outputs across settings mean the knob
        // was never connected" -- caught here by exactly that tell, one
        // command before an eighteen-run sweep would have been six copies of
        // three runs.
        Some(s) => s.bed.clone(),
        None => LabBox {
        width: arg("width").unwrap_or(512),
        height: arg("height").unwrap_or(320),
        // `labstats`' default, so a run here and a run there are the same bed.
        soil_depth: arg("soil").unwrap_or(80),
        founders: arg("founders").unwrap_or(8),
        colonies: arg("colonies").unwrap_or(1),
        compartments: arg("walls").unwrap_or(1),
        seed: arg("seed").unwrap_or(1),
        // `plant=<species>`: every figure this harness has produced was on
        // the default eight herbs; the owner's played bed is whatever the
        // PLANT chip offers, and a canopy is a different larder from a herb.
            species: arg::<String>("plant").unwrap_or_else(|| LabBox::default().species),
            ..LabBox::default()
        },
    };
    if control == "selftest" {
        return selftest(spec);
    }
    // Echo the parameters. A knob nobody can see the value of is a knob
    // nobody can tell is disconnected -- `plant_probe`'s 3.5-hour lesson.
    println!(
        "labforage: frames={frames} sample={sample_every} founders={} of {} colonies={} walls={} soil={} seed={} colony={} handout={handout} ants_at={ants_at} no_colony={no_colony}{}",
        spec.founders, spec.species, spec.colonies, spec.compartments, spec.soil_depth, spec.seed, spec.colony_species,
        scenario.as_ref().map(|s| format!(" scenario={} ({})", s.name, s.question)).unwrap_or_default()
    );
    // **Round 29 B1: the float's three switches, echoed with the rest.** A
    // knob nobody can see the value of is a knob nobody can tell is
    // disconnected -- `plant_probe`'s 3.5-hour lesson, and `flight_speed` in
    // particular ships as a *selector* (0.25/0.5/1.0) precisely because
    // which of them reads as a bee is a judge-by-eye question, so a log that
    // does not name the active one cannot be read at all. Taken from
    // `creature::flight_speed()` rather than from the env directly, so the
    // line reports what the simulation resolved and not what was typed.
    println!(
        "labforage: flight -- fly={} flight_speed={} land_afloat={}",
        if std::env::var("FLY").as_deref() == Ok("0") { "off (ballistic hop, main's arm)" } else { "on" },
        pixel_physics::sim::creature::flight_speed(),
        if std::env::var("LAND_AFLOAT").as_deref() == Ok("0") { "off (bug Z9 put back)" } else { "on" }
    );

    // Built bare and founded afterwards, for `windfall_probe`'s reason: a
    // species-level write after the founders are standing reaches nobody,
    // because `place_creature` copies the traits at placement. At the
    // default `ants_at=0` the founding happens right here, same as always;
    // at `ants_at>0` it is deferred to that frame, in the loop below.
    let (mut world, planted) = match &scenario {
        Some(s) => {
            if ants_at > 0 {
                println!("  ants_at={ants_at} ignored -- a scenario's own timeline decides colony placement");
            }
            let (w, p, sp) = s.build();
            println!(
                "  scenario {}: {} cells, {} plants, {} animals, {} settings applied",
                s.name, sp.cells, sp.plants, sp.animals, sp.settings
            );
            (w, p)
        }
        None => {
            let bare = LabBox { colonies: 0, ..spec.clone() };
            bare.build_counted()
        }
    };
    // **`hidden=<Input>:<unit>:<weight>[,...]` -- input-to-hidden weights set
    // on the colony species before a single ant is placed**, so a brain
    // change can be raced on the *played* bed without editing a `.ron` and
    // rebuilding between arms (the `include_str!` trap, which has produced
    // three bit-identical "sweeps" in this repo).
    //
    // **Before founding, and that is the whole reason it sits here rather
    // than beside the other knobs**: `place_creature` copies the genome at
    // placement, so the same write after the founders are standing reaches
    // nobody -- the identical trap the comment above this block records for
    // traits. `creature_arena` carries the same rider with the same syntax
    // and `trailfollow.rs` prints the string, so the three harnesses race
    // one set of numbers rather than three transcriptions of it.
    // **`bdecay=` / `adecay=` -- how fast a trail plane forgets**, against
    // `pheromone::DECAY_RHO`'s shipped 0.03 for both. The arm §Z7 needs:
    // once the ant can read channel B, the colony converges on patches it has
    // already eaten, and a trail that outlives its patch is that failure
    // exactly. Faster decay is the lever a real colony uses against it.
    for (key, channel) in [("adecay", Channel::A), ("bdecay", Channel::B)] {
        if let Some(rho) = arg::<f32>(key) {
            world.pheromones.set_channel_rho(channel, rho);
            println!("  {key}= {rho} (shipped {})", pixel_physics::sim::pheromone::DECAY_RHO);
        }
    }
    // **Same block, same reason, same refusal.** See `wire_rider`'s own doc:
    // before founding, because `place_creature` copies the genome at
    // placement -- and it asserts that the write actually moved a slot,
    // because an arm that matched nothing is the control wearing a label,
    // which reads as a clean null rather than as a broken run.
    let wires = wire_rider();
    if !wires.is_empty() {
        let sid = world.species.id_of(&spec.colony_species).expect("the colony species is compiled in");
        let mut genome = world.species.get(sid).genome.clone();
        let mut moved = 0;
        for &(input, output, w) in &wires {
            let i = brain::io_slot(input, output);
            if genome[i] != w {
                genome[i] = w;
                moved += 1;
            }
        }
        assert!(moved > 0, "wire= matched no slot the species did not already carry; this arm is the control wearing a label");
        world.species.set_genome(sid, genome);
        println!(
            "  wire= set {moved} of {} input->output weights on {}: {}",
            wires.len(),
            spec.colony_species,
            wires
                .iter()
                .map(|&(i, o, w)| format!("{}:{}:{w}", brain::INPUT_NAMES[i as usize], brain::OUTPUT_NAMES[o as usize]))
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    let hidden = hidden_rider();
    if !hidden.is_empty() {
        let sid = world.species.id_of(&spec.colony_species).expect("the colony species is compiled in");
        let mut genome = world.species.get(sid).genome.clone();
        let mut moved = 0;
        for &(input, unit, w) in &hidden {
            let i = brain::ih_slot(input, unit);
            if genome[i] != w {
                genome[i] = w;
                moved += 1;
            }
        }
        assert!(moved > 0, "hidden= matched no slot the species did not already carry; this arm is the control wearing a label");
        world.species.set_genome(sid, genome);
        println!(
            "  hidden= set {moved} of {} input->hidden weights on {}: {}",
            hidden.len(),
            spec.colony_species,
            hidden.iter().map(|&(i, u, w)| format!("{}:{u}:{w}", brain::INPUT_NAMES[i as usize])).collect::<Vec<_>>().join(",")
        );
    }
    // **Where the nests are, which the distance bands are measured from.**
    // A scenario sets `colonies: 0` and founds on its timeline instead, so
    // `colony_columns()` is empty for one and the whole `d<16 / d<48 /
    // d<128 / far` split would silently collapse into `far` -- a census
    // that still prints four columns and means none of them. Read the
    // scenario's own `Colony` entries instead, from placements and timeline
    // alike, since either may carry them.
    let nest_cols: Vec<i32> = match &scenario {
        Some(s) => {
            let mut v: Vec<i32> = s
                .placements
                .iter()
                .chain(s.timeline.iter().map(|e| &e.what))
                .filter_map(|p| match p {
                    Placement::Colony { x, .. } => Some(*x),
                    _ => None,
                })
                .collect();
            v.sort_unstable();
            v.dedup();
            v
        }
        None => spec.colony_columns(),
    };
    let mut ants_placed = 0usize;
    let mut gut = 0.0f32;
    if scenario.is_none() && ants_at == 0 {
        for &x in &nest_cols {
            ants_placed += world.found_colony(x, spec.ground_y - 2);
        }
        gut = ant_gut_bias(&world);
        if no_colony {
            let cleared = strip_colony(&mut world);
            println!("  no_colony=1: cleared {cleared} colony/colonies right back off the bed");
            ants_placed = 0;
        }
    }
    println!(
        "  bed: {} of {} founders planted, {}",
        planted.planted,
        planted.asked,
        match (&scenario, ants_at) {
            (Some(s), _) => format!("colonies arrive on {}'s timeline, nests at {nest_cols:?}", s.name),
            (None, 0) => format!("{ants_placed} ants in {} colony/colonies at {nest_cols:?}, founder gut_bias {gut}", spec.colonies),
            (None, _) => format!("ants founded later at frame {ants_at}, {} colony/colonies staged at {nest_cols:?}", spec.colonies),
        }
    );
    if scenario.is_none() && ants_at > frames {
        // The same "an unknown argument is silently ignored" shape
        // `CLAUDE.md` names, with a frame number standing in for the flag:
        // a founding frame past the run's own length would otherwise never
        // fire and the run would read as a colony that starved instantly.
        println!("  WARNING: ants_at={ants_at} is past frames={frames} -- the colony is never founded");
    }
    println!(
        "  bands: floor <= {FLOOR_BAND} rows above soil, low <= {LOW_BAND}, aloft above that | distance bands {DIST_BANDS:?}\n"
    );

    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    let mut visited = vec![false; spec.width as usize];
    // Round 28's garden-loop instrument -- see `Garden`'s own doc.
    let mut garden = Garden::new(spec.width as usize);
    let windfall_id = world.materials.id_of("windfall");
    let flower_id = world.materials.id_of("flower");
    let fruit_id = world.materials.id_of("fruit");
    let mut handed_out = 0u64;
    let mut first: Option<Sample> = None;
    let mut last = Sample::default();
    // See `trace=`'s own block in the loop for what these four do. The scan
    // is over the whole bed once per traced frame, which is why it is off
    // unless asked for and why `traceevery` defaults to a round number
    // rather than to 1.
    let trace_species: Option<String> = arg::<String>("trace");
    let trace_every: u64 = arg("traceevery").unwrap_or(50).max(1);
    let trace_from: u64 = arg("tracefrom").unwrap_or(0);
    let trace_to: u64 = arg("traceto").unwrap_or(u64::MAX);
    // See the census in the loop below for why this is sampled at 10 frames.
    let mut head_max: std::collections::BTreeMap<String, i32> = std::collections::BTreeMap::new();
    let mut peak_edible = 0usize;
    // Round 29's pile census -- see `Piles`. Always on: it is one
    // eight-way scan per animal per *stop*, against a census that already
    // walks every cell of the bed at the same stop.
    let mut piles = Piles::default();
    // `pilefollow=1` prints one line per stop about the animal that has
    // been body-boxed longest. Off by default: on a bed with a thousand
    // animals it is one line every stop whether or not anything is stuck.
    let pile_follow: bool = arg::<u32>("pilefollow").unwrap_or(0) != 0;
    // Round 29's second card: the owner marked three fixed points that do
    // not move in *either* arm. `CellProbe` is what names their occupants.
    let mut probe = CellProbe::new();

    println!(
        "{:>7} {:>5} {:>6} {:>7} {:>10} {:>6} {:>6} {:>6} {:>9} | {:>5} {:>5} {:>5} {:>5} | {:>4} {:>5} {:>5} {:>6} | {:>4} {:>4} {:>4} | {:>5} {:>8} {:>5}",
        "frame", "ants", "plnts", "edible", "worth(J)", "floor", "low", "aloft", "unvisited",
        "d<16", "d<48", "d<128", "far", "high", "eats", "born", "died",
        "brdr", "gen", "bgen", "fvis", "necJ", "bseen"
    );
    for f in 0..=frames {
        // **Founding, deferred to here when `ants_at > 0`.** Checked before
        // `mark_visited`/`census` below so the frame it lands on already
        // sees the colony rather than the empty bed it replaced.
        if scenario.is_none() && ants_at > 0 && f == ants_at {
            for &x in &nest_cols {
                ants_placed += world.found_colony(x, spec.ground_y - 2);
            }
            gut = ant_gut_bias(&world);
            if no_colony {
                let cleared = strip_colony(&mut world);
                println!("  ants_at {ants_at}: no_colony=1, cleared {cleared} colony/colonies right back off the bed\n");
                ants_placed = 0;
            } else {
                println!("  ants_at {ants_at}: founded {ants_placed} ants at {nest_cols:?}, founder gut_bias {gut}\n");
            }
        }
        // **The scenario's timeline, before the census on the same frame**,
        // so the frame a colony lands on already reports it rather than the
        // empty bed it replaced -- the same ordering `ants_at` above needs
        // and for the same reason.
        if let Some(sc) = &scenario {
            let arrived = pixel_physics::lab::scenario::tick_timeline(sc, &mut world, &spec);
            if arrived.animals > 0 {
                ants_placed += arrived.animals;
                // **Read the founders' gut the frame they arrive, not
                // before.** `ant_gut_bias` over an empty bed is 0.0, which
                // is a real gut value (a pure-carrion ant), so a gut left
                // at its initialiser is indistinguishable from a measured
                // one -- a null wearing a measurement.
                gut = ant_gut_bias(&world);
                if no_colony {
                    // **The colony-removed control, on a scenario's own
                    // timeline.** `apply_colony`/`apply_colonies` read
                    // nothing off `LabBox::colonies`, so the timeline places
                    // its colony regardless -- this clears it the instant it
                    // lands rather than suppressing the placement, which
                    // needs no edit to the scenario file at all.
                    let cleared = strip_colony(&mut world);
                    println!("  frame {f}: no_colony=1, cleared {cleared} colony/colonies right back off the bed\n");
                    ants_placed -= arrived.animals;
                } else {
                    println!("  frame {f}: {} animal(s) arrived on the timeline, founder gut_bias {gut}\n", arrived.animals);
                }
            }
        }
        // **`trace=<species> [traceevery=N] [tracefrom=F] [traceto=T]` --
        // follow ONE animal and print what it is doing.** Round 28's own
        // question, and it is not one a counter can answer: the flitter's
        // `flower_visits` reads 3-20 over 120,000 frames with the eye firing
        // constantly, and "turns toward the flower and overshoots", "lands
        // beside it and does not bite", "never gets within a cell of one"
        // and "sits on a leaf" are four different repairs that produce the
        // same small number.
        //
        // The first living animal of the species by organism id, the same
        // rule `labgif`'s `follow=` picks its subject by, so a trace and a
        // card of the same run are of the same animal. One line per
        // `traceevery` frames: where its head is, how high, whether it is
        // in the air, where the nearest flower is and whether that flower
        // is actually paying, whether it is close enough to drink, and the
        // running visit count. **`visits` is the effect column** -- the
        // other five say what the animal did and only that one says whether
        // it fed.
        if let Some(sp_name) = &trace_species {
            if f >= trace_from && f <= trace_to && f % trace_every == 0 {
                if let Some(sid) = world.species.id_of(sp_name) {
                    let subject = world
                        .live_organism_ids()
                        .into_iter()
                        .find(|id| world.organism(*id).is_some_and(|st| st.species == sid));
                    if let Some(id) = subject {
                        if let Some(st) = world.organism(id) {
                            let (hx, hy) = st.chain.first().copied().unwrap_or((0, 0));
                            let aloft = st.flight.is_some();
                            let energy = st.energy;
                            // Nearest flower cell in the whole bed, by
                            // Chebyshev -- the same distance the 8-ring the
                            // mouth uses is a radius of, so "d 1" reads as
                            // "could drink right now" with no arithmetic.
                            let mut best: Option<(i32, i32, i32, bool)> = None;
                            for y in 0..spec.height {
                                for x in 0..spec.width {
                                    let c = world.get(x, y);
                                    if c.organism_id() == 0
                                        || pixel_physics::sim::organism::cell_type(c.aux())
                                            != Some(pixel_physics::sim::organism::CellType::Flower)
                                    {
                                        continue;
                                    }
                                    let d = (x - hx).abs().max((y - hy).abs());
                                    if best.is_none_or(|(bd, _, _, _)| d < bd) {
                                        best = Some((d, x, y, pixel_physics::sim::plant::nectar_available(&world, x, y)));
                                    }
                                }
                            }
                            let visits = world.flower_visits_by_species.get(&sid.0).copied().unwrap_or(0);
                            match best {
                                Some((d, fx, fy, wet)) => println!(
                                    "  trace {sp_name} f{f}: head ({hx},{hy}) {} rows up, {} | nearest flower ({fx},{fy}) d {d} {} | adjacent {} | energy {energy:.0} | visits {visits}",
                                    spec.ground_y - hy,
                                    if aloft { "IN THE AIR" } else { "standing" },
                                    if wet { "PAYING" } else { "dry" },
                                    if d <= 1 { "YES" } else { "no" }
                                ),
                                None => println!(
                                    "  trace {sp_name} f{f}: head ({hx},{hy}) {} rows up, {} | no flower standing in the bed | energy {energy:.0} | visits {visits}",
                                    spec.ground_y - hy,
                                    if aloft { "IN THE AIR" } else { "standing" }
                                ),
                            }
                        }
                    } else {
                        println!("  trace {sp_name} f{f}: no living {sp_name} left");
                    }
                }
            }
        }
        mark_visited(&world, &mut visited, &mut garden.ant_heat, spec.width);
        // **How high any animal of each species has ever got, in rows above
        // the soil surface** -- P2's reach census, and the one number that
        // separates the two readings of a zero `flower_visits`. The flower
        // this bed's animals live on stands ~22 rows up its own stem, and
        // `dead-ends.md`'s hopper entry is built on exactly this figure
        // (*"the wired seed 3 colony's highest head reached 21 rows above
        // the soil"*), so it is stated in the same units on purpose: a
        // successor that reads 30 here and still never feeds has a
        // different problem from one that reads 13.
        //
        // **Every 10 frames, not every sample.** A hop lasts tens of frames
        // and `sample_every` is tens of thousands, so a per-sample reading
        // would photograph whatever happened to be in the air at six
        // instants -- the max of a sparse sample of a transient, which is
        // not a maximum at all. Ten frames is inside the shortest arc and
        // walks a few dozen live organisms; the cost does not show against
        // the sweep.
        if f % 10 == 0 {
            for id in world.live_organism_ids() {
                let Some(state) = world.organism(id) else { continue };
                if world.species.get(state.species).creature.is_none() {
                    continue;
                }
                let Some(&(_, hy)) = state.chain.first() else { continue };
                let rows = spec.ground_y - hy;
                if rows > 0 {
                    let e = head_max.entry(world.species.get(state.species).name.clone()).or_insert(0);
                    *e = (*e).max(rows);
                }
            }
        }
        // **Its own cadence, not `sample_every`'s.** The question is
        // whether a named cell's occupant moves, and the owner's own window
        // was 3,000 frames -- a 900-frame stop cannot see inside that.
        if let Some(p) = probe.as_mut() {
            if p.due(f) {
                p.sample(&world, f);
            }
        }
        if f % sample_every == 0 {
            let s = census(&world, &spec, gut, &visited, &nest_cols, windfall_id, flower_id, fruit_id, &mut garden);
            // **The pile, at this stop.** Printed on its own line rather
            // than as a column of the table above, because it is only ever
            // non-trivial on a handful of stops and a column of zeros is
            // how a finding gets skimmed past. `largest` is the owner's
            // *"big group/pile"* as a number; the durations are the
            // *"stuck"*, and only the summary can carry those.
            let largest = piles.sample(&world, f, pile_follow);
            if largest >= 3 {
                println!("  pile f{f}: largest clump of body-boxed animals = {largest}");
            }
            peak_edible = peak_edible.max(s.edible);
            if first.is_none() {
                first = Some(s);
            }
            last = s;
            let st = world.creature_stats;
            // One line per sample and every column on it, so the whole run is
            // one greppable block rather than a shape that has to be reread.
            println!(
                "{f:>7} {:>5} {:>6} {:>7} {:>10.0} {:>6} {:>6} {:>6} {:>9} | {:>5} {:>5} {:>5} {:>5} | {:>4} {:>5} {:>5} {:>6} | {:>4} {:>4} {:>4} | {:>5} {:>8.0} {:>5} | wfall={} flwr={} frt={} rblm={} rblk={}",
                s.ants, s.plants, s.edible, s.worth, s.floor, s.low, s.aloft, s.unvisited,
                s.by_dist[0], s.by_dist[1], s.by_dist[2], s.by_dist[3],
                s.ant_high, st.eats, st.births, st.deaths,
                s.breeders, s.gen, s.bgen,
                // **B1' (nectar, in two currencies)** --
                // `Reports/evolution-lab-pollinator-design-2026-09-10.md`
                // §3.1. `fvis` is the sensitivity counter (every reach of an
                // owned flower, paid or not); `necJ` is the cumulative
                // joules actually paid, the effect half -- their pairing is
                // the positive control the design's own brief names: at
                // `nectar_refill: 0.0`, `necJ` must stay flat at 0 across
                // the whole run while `fvis` keeps climbing. Divide `necJ`
                // by `f / 1000.0` for "joules paid per 1,000 frames" at any
                // sampled frame.
                //
                // **P1's own pair, beside it**: `bseen` is `bloom_seen`, the
                // bloom sense's own "did it fire at all" -- an eyed animal
                // that read `BloomNear > 0` this tick, cumulative over the
                // whole run so far. Read it against `fvis`: a bloom sense
                // that fires while `fvis` stays flat found flowers and did
                // not reach one; a `fvis` that climbs with `bseen` at 0 is a
                // bite that arrived by chance, not by sight.
                world.flower_visits, world.nectar_paid, world.creature_stats.bloom_seen, s.windfall,
                // **The rebloom extension, 2026-09-10** (this PR). `flwr`/
                // `frt` are the standing counts -- the effect a picture would
                // show, read at every stop rather than only at the run's end.
                // `rblm` is `World::flowers_rebloomed`, cumulative -- the "did
                // it fire at all" counter, since a bed that merely takes
                // longer to run dry would read identically on `flwr` alone
                // until this run is long enough to tell the two apart.
                // `rblk` is the reproductive-account refusal
                // (`organ_ripening_blocked`), the same counter the rest of
                // the organ pipeline uses -- a rebloom that keeps the flower
                // count up by exploding this instead is not the fix.
                s.standing_flowers, s.standing_fruit, world.flowers_rebloomed, world.organ_ripening_blocked
            );
        }
        if handout > 0 && f > 0 && f % handout == 0 {
            if let Some(wid) = windfall_id {
                let x = nest_cols[(handed_out as usize) % nest_cols.len().max(1)];
                for dy in 1..=6 {
                    let y = spec.ground_y - dy;
                    if world.get(x, y).material == pixel_physics::sim::material::EMPTY {
                        world.set(x, y, Cell::new(wid, 0));
                        handed_out += 1;
                        break;
                    }
                }
            }
        }
        if f < frames {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        }
    }

    let st = world.creature_stats;
    let l = &world.energy_ledger;
    let cols = visited.iter().filter(|v| **v).count();
    let burn = l.metabolized + l.moved + l.synapse_tax;
    println!("\n  the colony's reach over {frames} frames:");
    println!(
        "    columns ever walked   {cols} of {} ({:.0}% of the bed's width)",
        spec.width,
        100.0 * cols as f64 / spec.width as f64
    );
    println!("    highest ant head      {} rows above the soil at the last sample", last.ant_high);
    println!(
        "    edible standing now   {} cells ({:.0} J), peak over the run {peak_edible}",
        last.edible, last.worth
    );
    println!(
        "    ...of which unvisited {} cells ({:.0}% of what is standing) -- food in a column no ant ever entered",
        last.unvisited,
        if last.edible > 0 { 100.0 * last.unvisited as f64 / last.edible as f64 } else { 0.0 }
    );
    println!("    ...by height          floor {} low {} aloft {}", last.floor, last.low, last.aloft);
    // **M2's own two questions, both live counts rather than cumulative.**
    // `plants` is the stand itself (did the thicket establish); `windfall`
    // is standing fallen fruit on the ground right now, unfiltered by
    // whether the founders' gut would eat it -- the physical quantity a
    // bite needs present, not the diet-priced `edible` figure above.
    println!(
        "  standing now: plants {} | windfall (fallen fruit) {} cells, cumulative fruit_dropped {}",
        last.plants, last.windfall, world.fruit_dropped
    );
    println!(
        "\n  what the colony took: eats {} pickups {} | harvested plant {:.0} J corpse {:.0} J against burn {:.0} J",
        st.eats, st.pickups, l.harvested_plant, l.harvested_corpse, burn
    );
    println!("  animals: born {} died {} alive {} | handouts placed {handed_out}", st.births, st.deaths, last.ants);
    // **`deliveries` is the round trip, and it is the number a trail is
    // *for*.** `cols` says how far the colony ranged and `eats` says what it
    // put in its own mouth; only this says a laden ant got back to the nest,
    // which is what central-place foraging means and what the homing half of
    // the pheromone circuit exists to produce. It counts only for a species
    // that authors a `nest` -- `creature.rs`'s `adjacent_nest` returns
    // `false` for ever without one, so a run on `ancestor` reads 0 here by
    // construction rather than by failure.
    println!("  round trips: deliveries {} nest visits {}", st.deliveries, st.nest_visits);
    // **A2 -- the seed rides home.** `Reports/evolution-lab-ecology-design-
    // 2026-09-10.md` §2.6/§2.7, Brief A2. `nest_cols` is already computed
    // above (scenario `Colony` entries or `LabBox::colony_columns`) for the
    // food-distance bands, so the headline reuses it rather than asking the
    // engine to have an opinion about where a nest is -- see
    // `World::pip_germination_x`'s own doc for why that split is
    // deliberate.
    const NEAR_NEST_REACH: i32 = 32;
    let plants_from_pip_near_nest =
        world.pip_germination_x.iter().filter(|&&x| nest_cols.iter().any(|&c| (c - x).abs() <= NEAR_NEST_REACH)).count();
    // **Round 29's seed-where-eaten lane** -- `Reports/lanes/evolution-lab-
    // seed-where-eaten.md`, the owner's 2026-09-11 rule: "where should the
    // seed drop when a creature picks up food -- it should drop where it
    // is eaten, not immediately." This is the distribution the rule is
    // actually asking about: for each pip digestion released
    // (`World::pip_digestion_release_x`), the eating animal's column
    // distance from the nearest nest, bucketed into the same `DIST_BANDS`
    // every other distance reading in this file already uses rather than
    // a bespoke scale -- a colony that finishes its food before it gets
    // home should read differently here than one that always carries a
    // meal all the way in first.
    let mut digestion_release_by_dist = [0u64; DIST_BANDS.len()];
    for &x in &world.pip_digestion_release_x {
        let d = nest_cols.iter().map(|c| (c - x).abs()).min().unwrap_or(i32::MAX);
        for (b, &edge) in DIST_BANDS.iter().enumerate() {
            if d <= edge {
                digestion_release_by_dist[b] += 1;
                break;
            }
        }
    }
    println!(
        "  A2, eaten -- where the eater stood when digestion released the seed, by distance from the nearest nest {DIST_BANDS:?}: {digestion_release_by_dist:?} (n={})",
        world.pip_digestion_release_x.len()
    );
    // Median, not mean: `seed_transit_frames` is exactly the kind of
    // long-tailed sample (one lucky seed dropped a step from the bite, one
    // unlucky one carried the length of the bed) a mean would let the tail
    // dominate. `CLAUDE.md`'s own worked cases are about the same shape.
    let seed_transit_median = {
        let mut v = world.seed_transit_frames.clone();
        v.sort_unstable();
        v.get(v.len() / 2).copied()
    };
    println!(
        "  A2 -- the seed rides home: seeds_carried {} seeds_delivered {} | plants_from_pip {} of which within {NEAR_NEST_REACH} cols of a nest {plants_from_pip_near_nest} | \
         median carry {} frames over {} completed deliveries (herb.seed_half_life is 14,000; this becomes a live knob only if the median nears four figures)",
        world.seeds_carried,
        world.seeds_delivered,
        world.plants_from_pip,
        seed_transit_median.map_or_else(|| "n/a".to_string(), |m| m.to_string()),
        world.seed_transit_frames.len()
    );
    // **P2's own roll-ups (the flitter, Brief P2).** Three per-species
    // lines, because every one of them reads a single number on a bed that
    // holds two animals and the whole question is which animal it belongs
    // to. `CLAUDE.md`'s effect-counter rule is why they come in pairs here:
    // `impulses` says the verb fired and `impulses_refused` says how much of
    // that firing was into thin air (a launch called while already airborne
    // is refused, `creature::launch`), so the number that means anything is
    // the difference, printed as `real_launches` rather than left to be
    // subtracted by eye.
    let mut alive_by: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    for g in world.live_creature_groups() {
        *alive_by.entry(world.species.get(g.species).name.clone()).or_insert(0) += g.alive;
    }
    let fmt_alive = alive_by.iter().map(|(k, v)| format!("{k}:{v}")).collect::<Vec<_>>().join(",");
    let fmt_visits = world
        .flower_visits_by_species
        .iter()
        .map(|(sp, n)| format!("{}:{n}", world.species.get(pixel_physics::sim::organism::SpeciesId(*sp)).name))
        .collect::<Vec<_>>()
        .join(",");
    // **The effect half, and it points the other way.** A visit is an animal
    // drinking and the flower surviving; a bite is the flower coming off. The
    // pair is the design's own §2.2 instrument, and a nectar-only species must
    // read 0 here while an ordinary one still moves -- which is the positive
    // control that says this is a fact about the mouth and not a blind row.
    let fmt_bitten = world
        .flowers_bitten_by_species
        .iter()
        .map(|(sp, n)| format!("{}:{n}", world.species.get(pixel_physics::sim::organism::SpeciesId(*sp)).name))
        .collect::<Vec<_>>()
        .join(",");
    // The death-cause histogram, per species -- the cost fork's own
    // deliverable ("report the death-cause histogram and stop") and the only
    // place `starved aloft` can be told from ordinary starvation for ONE of
    // two animals in a bed. `World::group_deaths` is already split by
    // `(species, colony)`; this rolls the colonies up.
    let mut deaths_by: std::collections::BTreeMap<String, [u64; pixel_physics::sim::organism::DEATH_CAUSES]> =
        std::collections::BTreeMap::new();
    for g in &world.group_deaths {
        let row = deaths_by
            .entry(world.species.get(g.species).name.clone())
            .or_insert([0; pixel_physics::sim::organism::DEATH_CAUSES]);
        for (i, n) in g.by_cause.iter().enumerate() {
            row[i] += n;
        }
    }
    let fmt_deaths = deaths_by
        .iter()
        .map(|(name, row)| {
            let causes = pixel_physics::sim::organism::DEATH_CAUSE_LIST
                .iter()
                .enumerate()
                .filter(|(i, _)| row[*i] > 0)
                .map(|(i, c)| format!("{}:{}", c.label().replace(' ', "_"), row[i]))
                .collect::<Vec<_>>()
                .join("/");
            format!("{name}[{}]", if causes.is_empty() { "none".to_string() } else { causes })
        })
        .collect::<Vec<_>>()
        .join(",");
    let starved_aloft = world.deaths_by_cause[pixel_physics::sim::organism::DeathCause::StarvedInFlight.index()];

    // **Round 28 -- the garden-loop instrument.** `Reports/lanes/
    // evolution-lab-garden-loop.md`. Two questions the A2 line above
    // cannot answer: given a pip *was* delivered (or stood at the bite
    // site), does its drop cell actually meet the species' own Germinate
    // thresholds (hypothesis a), and does windfall land somewhere the
    // colony's own foot traffic reaches at all (hypothesis b)?
    let pip_checks_n = world.pip_checks.len();
    let pip_checks_delivered = world.pip_checks.iter().filter(|c| c.delivered).count();
    let pip_checks_resting = world.pip_checks.iter().filter(|c| c.resting).count();
    let pip_checks_light_ok = world.pip_checks.iter().filter(|c| c.light >= c.light_threshold).count();
    let pip_checks_water_ok = world.pip_checks.iter().filter(|c| c.soil_water >= c.soil_water_threshold).count();
    let pip_checks_ready = world
        .pip_checks
        .iter()
        .filter(|c| c.resting && c.light >= c.light_threshold && c.soil_water >= c.soil_water_threshold)
        .count();
    println!(
        "\n  round 28 -- garden loop, hypothesis (a): first Germinate check per pip \
         ({pip_checks_n} pips checked, {pip_checks_delivered} rode home/A2, {} stood at the bite/A1):",
        pip_checks_n - pip_checks_delivered
    );
    println!(
        "    resting {pip_checks_resting}/{pip_checks_n} | light>=threshold {pip_checks_light_ok}/{pip_checks_n} | \
         soil_water>=threshold {pip_checks_water_ok}/{pip_checks_n} | all three (would germinate now) {pip_checks_ready}/{pip_checks_n}"
    );
    for c in &world.pip_checks {
        println!(
            "      frame {:>7} x={:>4} y={:>4} {} resting={:<5} light {:>5.2}/{:<5.2} ({}) water {:>4.2}/{:<4.2} ({}) overburden={}",
            c.frame,
            c.x,
            c.y,
            if c.delivered { "A2" } else { "A1" },
            c.resting,
            c.light,
            c.light_threshold,
            if c.light >= c.light_threshold { "OK " } else { "LOW" },
            c.soil_water,
            c.soil_water_threshold,
            if c.soil_water >= c.soil_water_threshold { "OK " } else { "DRY" },
            c.overburden
        );
    }
    let pip_rot_x = &world.pip_rot_x;
    let pip_eaten_x = &world.pip_eaten_x;
    println!(
        "  round 28 -- pip exits: seeds_spilled {} = seeds_carried {} (rode home) + {} (stood at the bite) | \
         plants_from_pip {} + pips_rotted {} (at {pip_rot_x:?}) + pips_eaten {} (at {pip_eaten_x:?}) should not exceed seeds_spilled, the rest still standing | \
         dig_diverted_seed {} (garden-fix: the dig verb routed around a live seed instead of clearing it)",
        world.seeds_spilled,
        world.seeds_carried,
        world.seeds_spilled.saturating_sub(world.seeds_carried),
        world.plants_from_pip,
        world.pips_rotted,
        world.pips_eaten,
        world.dig_diverted_seed
    );

    // Hypothesis (b): where fruit lands against where the colony walks.
    let total_wf_heat: u64 = garden.windfall_heat.iter().map(|&v| u64::from(v)).sum();
    let wf_dead_zone: u64 =
        garden.windfall_heat.iter().zip(garden.ant_heat.iter()).filter(|&(_, &ah)| ah == 0).map(|(&wh, _)| u64::from(wh)).sum();
    let wf_dead_zone_pct = if total_wf_heat > 0 { 100.0 * wf_dead_zone as f64 / total_wf_heat as f64 } else { 0.0 };
    println!(
        "\n  round 28 -- garden loop, hypothesis (b): windfall landing vs the colony's own foot traffic:"
    );
    println!(
        "    standing-windfall height bands: floor {} low {} aloft {} | distance bands {:?} | \
         under canopy (<{:.1} noon-equiv light) {} of {} column-stops",
        garden.windfall_floor,
        garden.windfall_low,
        garden.windfall_aloft,
        garden.windfall_by_dist,
        field::MAX_LIGHT / 2.0,
        garden.windfall_shaded,
        garden.windfall_shaded + garden.windfall_open
    );
    println!(
        "    windfall column-stops {total_wf_heat}, of which {wf_dead_zone} ({wf_dead_zone_pct:.0}%) sit in a column \
         the colony's own heat map never touched all run (ant_heat==0 there)"
    );
    const HEAT_BUCKET: usize = 32;
    let nbuckets = (spec.width as usize).div_ceil(HEAT_BUCKET);
    let mut wf_buckets = vec![0u64; nbuckets];
    let mut ant_buckets = vec![0u64; nbuckets];
    for (i, (&wh, &ah)) in garden.windfall_heat.iter().zip(garden.ant_heat.iter()).enumerate() {
        wf_buckets[i / HEAT_BUCKET] += u64::from(wh);
        ant_buckets[i / HEAT_BUCKET] += u64::from(ah);
    }
    println!("    by {HEAT_BUCKET}-column band, col_start: windfall column-stops / ant body-frames (nest at {nest_cols:?}):");
    for b in 0..nbuckets {
        println!("      {:>4}: {:>7} / {:>10}", b * HEAT_BUCKET, wf_buckets[b], ant_buckets[b]);
    }

    // **The seed bank as a standing count, round 29 Brief 1.** `plants`
    // above is plants **plus** the waiting bank -- the late-game design's §3
    // records that as a live mislabel in every earlier reading of this file
    // -- so the bank has to be counted separately to say whether it drained.
    // A waiting seed is a one-cell organism whose cell reads `CellType::Seed`
    // (the same test `latecensus::is_waiting_seed` uses), plus any seed
    // currently riding in a crop, which owns no cell at all while it rides.
    let seed_bank_n = world
        .live_organism_ids()
        .into_iter()
        .filter(|&id| {
            world.organism(id).is_some_and(|st| {
                world.species.get(st.species).creature.is_none()
                    && (world.is_carried_seed(id)
                        || (st.cells.len() == 1
                            && st.cells.keys().next().is_some_and(|&(x, y)| {
                                pixel_physics::sim::organism::cell_type(world.get(x, y).aux())
                                    == Some(pixel_physics::sim::organism::CellType::Seed)
                            })))
            })
        })
        .count();

    piles.close();
    piles.print(sample_every, &world.creature_stats);

    println!(
        "SUMMARY seed={} founders={} colonies={} frames={frames} handout={handout} cols={cols} plants={} windfall={} fruit_dropped={} edible={} unvisited={} floor={} aloft={} \
         peak_edible={peak_edible} eats={} born={} died={} alive={} intake={:.0} burn={:.0} shares={} shared_j={:.0} moves={} deliveries={} nest_visits={} \
         regime={} breeders={} gen={} bgen={} windfall_bitten={} seeds_spilled={} plants_from_pip={} pips_rotted={} pips_eaten={} \
         windfall_bitten_ownerless={} seeds_carried={} seeds_delivered={} plants_from_pip_near_nest={} seed_transit_median={} lookup={} visits={} \
         flower_visits={} nectar_paid={:.0} nectar_j_per_1000f={:.2} organs_built={} bloom_seen={} \
         standing_flowers={} standing_fruit={} flowers_rebloomed={} organ_ripening_blocked={} organ_ripening_paid={} \
         pip_checks={pip_checks_n} pip_checks_delivered={pip_checks_delivered} pip_checks_resting={pip_checks_resting} \
         pip_checks_light_ok={pip_checks_light_ok} pip_checks_water_ok={pip_checks_water_ok} pip_checks_ready={pip_checks_ready} \
         windfall_col_stops={total_wf_heat} windfall_dead_zone_stops={wf_dead_zone} windfall_dead_zone_pct={wf_dead_zone_pct:.0} \
         windfall_floor={} windfall_low={} windfall_aloft={} windfall_shaded={} windfall_open={} dig_diverted_seed={} \
         launch_attempts={} real_launches={} impulses_refused={} refused_pct={:.0} starved_aloft={} flight_frames={} \
         flower_visits_by={} flowers_bitten_by={} alive_by={} deaths_by={} head_max_rows={} \
         pips_set_on_soil={} pips_set_on_nest={} \
         fly_ticks={} fly_frames={} fly_turns={} fly_j={:.1} landed_afloat={} \
         moves_per_launch={:.2} frames_per_launch={:.0} fly_share={:.0} flight_speed={} \
         pips_released_by_digestion={} fruit_dropped_with_seed={} digestion_release_by_dist={digestion_release_by_dist:?} \
         nest_blends={} share_blends={} nest_sites={} nest_gap_max={:.4} \
         bare_seeds_spared={} bare_seeds_carried={} seed_bank={seed_bank_n} \
         pile_stops={} pile_animal_reads={} pile_boxed={} pile_boxed_terrain={} pile_boxed_creature={} pile_boxed_both={} \
         pile_body_boxed={} pile_largest={} pile_stops_with_3={} pile_streaks={} pile_streak_median={} pile_streak_max={} \
         pile_body_boxed_long={} pile_body_boxed_short={} pile_body_boxed_laden={} pile_largest_long={} \
         pile_streaks_long={} pile_streak_median_long={} pile_streak_max_long={} \
         pile_short_by_genome={} pile_short_by_loss={} pile_short_max_gen={} \
         pile_short_genome_cells={} pile_short_have_cells={} pile_short_first_boxed_age={} pile_long_first_boxed_age={} \
         idle_with_room={} idle_with_room_long={} moving={} \
         idle_streaks_long={} idle_streak_max_long={} idle_streak_p90_long={} \
         idle_streaks_any={} idle_streak_max_any={} idle_streak_p90_any={}",
        spec.seed, spec.founders, spec.colonies, last.plants, last.windfall, world.fruit_dropped, last.edible, last.unvisited, last.floor, last.aloft,
        st.eats, st.births, st.deaths, last.ants, l.harvested_plant + l.harvested_corpse, burn, st.shares, st.shared_j, st.moves,
        st.deliveries, st.nest_visits,
        std::env::var("PIXEL_PHYSICS_BREEDING").unwrap_or_else(|_| "individual".to_string()),
        last.breeders, world.deepest_animal_generation, world.deepest_breeder_generation,
        // **M2's own counter** -- every bite that reached an owned windfall
        // cell and was about to roll for survival, counted *before* the
        // roll in `plant::seed_survives_bite`. The true bite rate, rather
        // than an estimate backed out of `seeds_spilled / seed_gut_survival`
        // (which is silent at `seed_gut_survival: 0.0`). See
        // `World::windfall_bitten`.
        world.windfall_bitten,
        // **A1's counters, both halves** -- "it fired" (`seeds_spilled`) and
        // the three exits that sum to it (`plants_from_pip`, `pips_rotted`,
        // `pips_eaten`), plus whatever is still standing as a `pip`.
        // `Reports/evolution-lab-ecology-design-2026-09-10.md` §2.6.
        world.seeds_spilled, world.plants_from_pip, world.pips_rotted, world.pips_eaten,
        // **The measure lane's finding, 2026-09-10** -- a bite met a
        // `windfall` with no organism owning it, so nothing above could
        // roll. High against a low `seeds_spilled`: read this before
        // retuning `seed_gut_survival`. See `World::
        // windfall_bitten_ownerless`.
        world.windfall_bitten_ownerless,
        // **A2's own counters** -- "it fired" (`seeds_carried`), "it worked"
        // (`seeds_delivered`), the headline (`plants_from_pip_near_nest`),
        // and the transit-cost check, all computed just above.
        // `Reports/evolution-lab-ecology-design-2026-09-10.md` §2.6/§2.7.
        world.seeds_carried,
        world.seeds_delivered,
        plants_from_pip_near_nest,
        seed_transit_median.map_or_else(|| "n/a".to_string(), |m| m.to_string()),
        // **`visits` is the whole point of the breeder-index change, and
        // `lookup` says which arm produced it.** Organisms the breeding rule
        // had to look at over the run: the full-slot scan walks every
        // organism in the world, the per-colony index walks that colony's
        // breeders. Both arms count the same way, so the pair is a ratio
        // rather than two numbers.
        //
        // Printed beside `born`/`alive` deliberately: the index changes only
        // HOW an answer is computed, so at one seed and one regime every
        // other field on this line must be identical between the arms. If
        // `born` moves, the index changed behaviour and is wrong -- a
        // whole-run equivalence check that no unit test can match.
        if std::env::var("PIXEL_PHYSICS_BREEDER_INDEX").as_deref() == Ok("scan") { "scan" } else { "index" },
        st.breeder_scan_visits,
        // **B1' (nectar, in two currencies)**
        // (`Reports/evolution-lab-pollinator-design-2026-09-10.md` §3.1,
        // Brief B1'). `flower_visits` is the sensitivity counter, `nectar_
        // paid` the effect (raw joules the plant side handed out, not the
        // gut-filtered credit an animal actually banked -- see `World::
        // nectar_paid`'s own doc for why the two differ). `nectar_j_per_
        // 1000f` is the design's own "joules paid per 1,000 frames" read at
        // this run's own length; re-read it from the per-sample table
        // above for the frame-40,000 figure the brief asks for on a longer
        // run. `organs_built` is B1's own guard: nectar must not starve
        // fruit, so this must not fall against a `nectar_yield: 0` ablation
        // of the same seed.
        world.flower_visits, world.nectar_paid,
        if frames > 0 { world.nectar_paid / (frames as f64 / 1000.0) } else { 0.0 },
        world.organs_built,
        // **P1 (the bloom sense)** -- the "it fired" half beside B1's
        // "it worked" half above: an eyed animal reading `BloomNear > 0`,
        // cumulative over the run. See `World::CreatureStats::bloom_seen`.
        world.creature_stats.bloom_seen,
        // **The rebloom extension, 2026-09-10** (this PR,
        // `Reports/evolution-lab-pollinator-design-2026-09-10.md`'s rebloom
        // brief). `standing_flowers`/`standing_fruit` are the last sample's
        // raw material census -- the effect a picture of the bed would show.
        // `flowers_rebloomed` is `World::flowers_rebloomed`, the "did it
        // fire at all" counter: a bed that merely ran the ordinary
        // once-per-axis route slower would still move `standing_flowers`,
        // and only this says the axes are actually being reused.
        // `organ_ripening_blocked`/`organ_ripening_paid` are the whole
        // organ pipeline's shared refusal/success pair (fruit-set,
        // seed-drop and rebloom all count against the same two lines) --
        // read against the pre-rebloom baseline for whether keeping the
        // flower count up also exploded the refusal rate.
        last.standing_flowers, last.standing_fruit, world.flowers_rebloomed,
        world.organ_ripening_blocked, world.organ_ripening_paid,
        // Round 28's garden-loop instrument, appended rather than woven in
        // -- `labforage`'s SUMMARY line is contested by every lane
        // (`Reports/evolution-lab-round-27-2026-09-10.md`'s own environment
        // note): keep main's fields and append the branch's.
        garden.windfall_floor, garden.windfall_low, garden.windfall_aloft, garden.windfall_shaded, garden.windfall_open,
        // Round 28 garden-fix: the dig verb's own "it fired" counter for
        // routing around a live seed instead of clearing it as spoil --
        // see `World::dig_diverted_seed`'s own doc.
        world.dig_diverted_seed,
        // **P2 (the flitter)** -- the hop's own pair and the two
        // per-species splits.
        //
        // **`CreatureStats::impulses` is already the real launches**, not
        // the attempts: `creature::launch` increments it only on the branch
        // that puts a body in the air and increments `impulses_refused` on
        // the branch that returns false. So the design report's `launches`
        // column is the SUM of the two, and it is printed here as
        // `launch_attempts` rather than left to be reconstructed -- a first
        // draft of this line printed `impulses - refused` as the real
        // launches and read **0 real launches** on an arm whose animals were
        // visibly hopping, the counter-means-what-you-assumed failure
        // `CLAUDE.md` opens its measurement section with.
        //
        // `refused_pct` is printed beside the raw counts because the raw
        // counts are not comparable between arms of different population:
        // an arm with twenty times the animals asks for the verb twenty
        // times as often. The design's own prediction is about the share
        // (*"at 2.0, 60% of every launch is one"*). `starved_aloft` is the
        // verb's own bill (`DeathCause::StarvedInFlight`).
        world.creature_stats.impulses + world.creature_stats.impulses_refused,
        world.creature_stats.impulses,
        world.creature_stats.impulses_refused,
        {
            let asked = world.creature_stats.impulses + world.creature_stats.impulses_refused;
            if asked > 0 { 100.0 * world.creature_stats.impulses_refused as f64 / asked as f64 } else { 0.0 }
        },
        starved_aloft,
        world.creature_stats.flight_frames,
        if fmt_visits.is_empty() { "none".to_string() } else { fmt_visits },
        if fmt_bitten.is_empty() { "none".to_string() } else { fmt_bitten },
        if fmt_alive.is_empty() { "none".to_string() } else { fmt_alive },
        if fmt_deaths.is_empty() { "none".to_string() } else { fmt_deaths },
        // **Rows above the soil, the highest any animal of that species
        // reached at any sampled frame.** Read it against 22, the height of
        // the flower: a `flower_visits` of zero beside a `head_max_rows`
        // short of that is a REACH failure and nothing else, and beside one
        // well past it is a failure of the mouth, the menu or the refill --
        // opposite fixes, and the count alone cannot tell them apart.
        {
            let v = head_max.iter().map(|(k, r)| format!("{k}:{r}")).collect::<Vec<_>>().join(",");
            if v.is_empty() { "none".to_string() } else { v }
        },
        // Round 28's garden-midden build (this lane, appended per the same
        // "keep main's fields, append the branch's" convention the line
        // above already follows): where a delivered pip's final ground
        // stood, read the same two-part test `Behavior::Germinate` runs.
        // `World::pips_set_on_soil`/`pips_set_on_nest`'s own docs.
        world.pips_set_on_soil, world.pips_set_on_nest,
        // **Round 29 B1's own counters** (`Reports/evolution-lab-flight-
        // design-2026-09-11.md` §7), appended rather than woven in, per the
        // same "keep main's fields, append the branch's" convention the
        // block above already follows.
        //
        // **The pair, in the order `CLAUDE.md` asks them to be read.**
        // `fly_ticks` is "it was asked at all" -- brain evaluations made
        // aloft, which is **0 on `main` by construction**, since an airborne
        // animal there did not read the world or evaluate its brain.
        // `fly_frames` is the effect from the far side of the call: airborne
        // frames on which `BrainOutput::Fly` was actually holding the body
        // up. High ticks against zero frames is a wiring problem; zero ticks
        // is no species having priced flight, or nothing having left the
        // ground, and `real_launches` above says which.
        world.creature_stats.fly_ticks,
        world.creature_stats.fly_frames,
        // **`Turn`'s own effect counter, and it exists because of R4.** On
        // the ground both outer candidates lose at every `Turn` value on
        // level footing, so "the weight is authored and the animal is
        // steering" is a false inference this engine has already made once.
        // This counts octant rotations actually applied to a velocity.
        world.creature_stats.fly_turns,
        world.creature_stats.fly_energy,
        // **§Z9's exit firing**: bodies put down because they were
        // weightless and not flying -- standing on water rather than
        // hanging over it. Read beside `deaths_by`'s `STARVED ALOFT` share,
        // which is the number the bug is about; `LAND_AFLOAT=0` puts the
        // defect back and this goes to 0.
        world.creature_stats.landed_afloat,
        // **Walking steps per real launch -- the design's own headline, and
        // it reads 0.60 on `main`.** At that rate the animal turns about
        // once per two hops and each hop carries it ~27 uncontrolled cells,
        // so a flower nine cells away is a 3x overshoot rather than a near
        // miss.
        if world.creature_stats.impulses > 0 { st.moves as f64 / world.creature_stats.impulses as f64 } else { 0.0 },
        // **Frames per launch, read against the 22-frame ballistic arc** for
        // a `Chain(2)` body at launch speed 2.0. Anything well above 22 is
        // airborne time that is not an arc -- §Z9 at the scale the register
        // does not carry -- and `landed_afloat` beside it says whether the
        // fix is what closed it.
        if world.creature_stats.impulses > 0 { world.creature_stats.flight_frames as f64 / world.creature_stats.impulses as f64 } else { 0.0 },
        // The share of airborne frames the verb was paying for, in percent.
        if world.creature_stats.flight_frames > 0 {
            100.0 * world.creature_stats.fly_frames as f64 / world.creature_stats.flight_frames as f64
        } else {
            0.0
        },
        if std::env::var("FLY").as_deref() == Ok("0") { "off".to_string() } else { pixel_physics::sim::creature::flight_speed().to_string() },
        // **Round 29's seed-where-eaten lane's own counters, appended per
        // the same "keep main's fields, append the branch's" convention
        // every block above follows.** `pips_released_by_digestion` is
        // this build's own eaten exit's "it fired"; `fruit_dropped_with_
        // seed` is the uneaten exit's; `seeds_delivered` above (shared
        // with A2) is the union of both plus the pre-existing drop path.
        // The histogram computed just above is the where-eaten
        // distribution the owner's rule is about.
        world.pips_released_by_digestion,
        world.fruit_dropped_with_seed,
        // **Appended at the end, keeping `main`'s fields first and in place**
        // -- this line is contested by every lane and a reordering breaks
        // everyone's parser. The pair is `CLAUDE.md`'s "it fired" counter and
        // its far side: `nest_blends` says cohesion ran, `nest_gap_max` says
        // what it produced.
        world.creature_stats.nest_blends,
        world.creature_stats.share_blends,
        world.nest_sites.len(),
        world.nest_scent_gaps().iter().map(|(_, _, d)| *d).fold(0.0f32, f32::max),
        // **Round 29's seed-cargo lane's counters, appended after the
        // cohesion lane's by that same convention** -- both pairs landed in
        // this line on the same day, and the merge kept main's in place
        // rather than interleaving them. `bare_seeds_spared` is the plant
        // side's "it fired" (a bare-seed bite rolled `seed_gut_survival` and
        // won); `bare_seeds_carried` is the "it worked" from the far side of
        // the call (that survivor became a `Crop::passenger` rather than
        // standing where it was bitten). Both read 0 with
        // `PIXEL_PHYSICS_SEED_CARGO=0`, which is the kill switch's control.
        world.bare_seeds_spared,
        world.bare_seeds_carried,
        // **Round 29's pile census, appended after main's fields** -- see
        // `Piles`. `pile_largest` is the owner's *"big group/pile"* and
        // `pile_streak_median`/`pile_streak_max` are the *"stuck"*: a
        // colony where a different animal is briefly jammed at every stop
        // and one where the same six have not moved all run are identical
        // on `pile_body_boxed` alone and are opposite findings.
        piles.stops,
        piles.animal_stops,
        piles.boxed,
        piles.terrain_only,
        piles.creature_only,
        piles.both,
        piles.body_boxed,
        piles.largest,
        piles.pile_stops,
        piles.streak_total(),
        piles.median_streak(),
        piles.max_streak,
        piles.body_boxed_long,
        piles.body_boxed_short,
        piles.body_boxed_laden,
        piles.largest_long,
        piles.hist_long.values().sum::<u64>(),
        piles.median_streak_long(),
        piles.max_streak_long,
        // **Round 29's second finding, and the fact round 30 starts from.**
        // See `Piles::short_by_genome`.
        piles.short_by_genome,
        piles.short_by_loss,
        piles.short_max_generation,
        // Round 30's starting facts -- see `Piles`' own doc on the four.
        // Printed as means so the two body populations are comparable at a
        // glance; the raw sums are one multiplication away if needed.
        piles.short_genome_sum,
        piles.short_chain_sum,
        piles.short_first_age_sum.checked_div(piles.short_first_age_n).unwrap_or(0),
        piles.long_first_age_sum.checked_div(piles.long_first_age_n).unwrap_or(0),
        // **The population `body_boxed` excludes by construction** -- see
        // `Piles::idle_with_room`. Read `idle_with_room_long` against
        // `moving`: that ratio is the owner's *"standing still"* for the
        // bodies he was looking at, and no other column in this line
        // contains it.
        piles.idle_with_room,
        piles.idle_with_room_long,
        piles.moving,
        // **The duration half, which is the finding the rate above is not.**
        // `idle_streak_max_long` is in sample stops; multiply by `sample=`
        // for frames. p90 rather than the mean, per `CLAUDE.md` on order
        // statistics over chaotic outcomes.
        piles.idle_hist.values().sum::<u64>(),
        piles.idle_max_streak,
        piles.idle_streak_p90(),
        // ...and the same with the body-length gate removed, so the shipped
        // two-cell ant can act as a control at all. See `idle_live_any`.
        piles.idle_hist_any.values().sum::<u64>(),
        piles.idle_max_streak_any,
        piles.idle_streak_p90_any()
    );
    if let Some(p) = probe.as_ref() {
        p.report();
    }
}

/// **The sensitivity half.** A census reading zero because there is nothing
/// there and one reading zero because it is blind print the same line, and
/// `CLAUDE.md`'s standing remedy is to construct the case whose answer is
/// known and watch the instrument report it.
///
/// Plants windfall (960 J face, well clear of the threshold at any gut near
/// neutral) at a known height and a known distance from the nest and asserts
/// every band moves: the total, the height band, the distance band and the
/// visited mask. Each assert is a band that could otherwise be silently
/// always-zero -- which is exactly what `aloft` and `unvisited` would look
/// like on a bed where the finding is real.
fn selftest(spec: LabBox) {
    let bare = LabBox { colonies: 0, founders: 0, ..spec.clone() };
    let (mut world, _) = bare.build_counted();
    let nest_cols = spec.colony_columns();
    let nest = nest_cols[0];
    let wid = world.materials.id_of("windfall").expect("windfall material");
    let visited_none = vec![false; spec.width as usize];
    let mut visited_all = vec![true; spec.width as usize];

    let mut garden0 = Garden::new(spec.width as usize);
    let base = census(&world, &spec, 0.0, &visited_none, &nest_cols, Some(wid), None, None, &mut garden0);
    println!("labforage selftest: empty bed reads edible {} (must be 0)", base.edible);
    assert_eq!(base.edible, 0, "an unplanted bed is not food; the census is counting something it should not");
    assert_eq!(base.windfall, 0, "an unplanted bed has no fallen fruit either; the raw windfall count is counting something it should not");
    // Round 28's garden-loop instrument, same shape: nothing planted means
    // nothing in any of `Garden`'s windfall bands either.
    assert_eq!(garden0.windfall_heat.iter().sum::<u32>(), 0, "an unplanted bed must not heat any column");
    assert_eq!(garden0.windfall_floor + garden0.windfall_low + garden0.windfall_aloft, 0, "an unplanted bed has no windfall to band by height");

    // One cell on the floor beside the nest, one 40 rows up and 200 columns
    // away. The two differ in every band the run's finding turns on.
    let near = (nest + 2, spec.ground_y - 1);
    let far_x = (nest + 200).min(spec.width - 6);
    let far = (far_x, spec.ground_y - 40);
    world.set(near.0, near.1, Cell::new(wid, 0));
    world.set(far.0, far.1, Cell::new(wid, 0));

    let mut garden1 = Garden::new(spec.width as usize);
    let s = census(&world, &spec, 0.0, &visited_none, &nest_cols, Some(wid), None, None, &mut garden1);
    println!(
        "  planted 2 cells (one at the nest on the floor, one {} columns out and 40 rows up): \
         edible {} floor {} low {} aloft {} unvisited {} by_dist {:?} worth {:.0} J windfall {}",
        far.0 - nest, s.edible, s.floor, s.low, s.aloft, s.unvisited, s.by_dist, s.worth, s.windfall
    );
    assert_eq!(s.edible, 2, "the census must see both planted cells");
    assert_eq!(s.floor, 1, "the floor band must see the cell on the floor and only it");
    assert_eq!(s.aloft, 1, "the aloft band must see the cell 40 rows up -- otherwise `aloft 0` means nothing");
    assert_eq!(s.unvisited, 2, "with no column visited, every cell must read unvisited");
    assert!(s.by_dist[0] >= 1, "the near band must see the cell 2 columns from the nest");
    assert!(s.by_dist[2] >= 1 || s.by_dist[3] >= 1, "the far cell must land in a far distance band, not the near one");
    assert!(s.worth > 0.0, "food priced at zero is not food; diet_yield is not reaching the census");
    assert_eq!(s.windfall, 2, "the raw windfall count must see both planted cells regardless of the gut -- it is a material census, not a diet_yield one");
    // **Round 28's garden-loop instrument, the same positive control
    // applied to `Garden`.** Two known cells at known columns, heights and
    // distances must move every one of its bands, or a zero on the played
    // bed would be indistinguishable from blind.
    println!(
        "  garden: windfall_heat[near]={} windfall_heat[far]={} floor {} low {} aloft {} by_dist {:?} shaded {} open {}",
        garden1.windfall_heat[near.0 as usize], garden1.windfall_heat[far.0 as usize],
        garden1.windfall_floor, garden1.windfall_low, garden1.windfall_aloft, garden1.windfall_by_dist,
        garden1.windfall_shaded, garden1.windfall_open
    );
    assert_eq!(garden1.windfall_heat[near.0 as usize], 1, "the near cell's own column must heat by exactly one stop's worth");
    assert_eq!(garden1.windfall_heat[far.0 as usize], 1, "the far cell's own column must heat too, independently of the near one");
    assert_eq!(garden1.windfall_floor, 1, "garden's height band must agree with Sample's: the floor cell is the only floor windfall");
    assert_eq!(garden1.windfall_aloft, 1, "garden's height band must agree with Sample's: the far cell is the only aloft windfall");
    assert_eq!(garden1.windfall_low, 0, "neither planted cell sits in the low band");
    assert!(garden1.windfall_by_dist[0] >= 1, "garden's distance band must agree with Sample's: the near cell is within d<16");
    assert!(
        garden1.windfall_by_dist[2] >= 1 || garden1.windfall_by_dist[3] >= 1,
        "garden's distance band must agree with Sample's: the far cell lands in a far band"
    );
    assert_eq!(
        garden1.windfall_shaded + garden1.windfall_open,
        2,
        "every standing windfall cell must land in exactly one of shaded/open -- a light read that silently drops a cell would show up as a sum under 2"
    );

    // ...and the mask has to be able to go the other way, or `unvisited`
    // would be a constant wearing a measurement's clothes.
    visited_all[..].fill(true);
    let mut garden2 = Garden::new(spec.width as usize);
    let s2 = census(&world, &spec, 0.0, &visited_all, &nest_cols, Some(wid), None, None, &mut garden2);
    println!("  same bed with every column marked visited: unvisited {} (must be 0)", s2.unvisited);
    assert_eq!(s2.unvisited, 0, "the visited mask does not reach the census");
    assert_eq!(s2.edible, 2, "the mask must not change what is counted as food");

    // A gut that cannot digest plants must stop seeing them -- the predicate
    // is the mouth's, so this is the check that the census asks the mouth.
    let mut garden3 = Garden::new(spec.width as usize);
    let s3 = census(&world, &spec, 1.0, &visited_none, &nest_cols, Some(wid), None, None, &mut garden3);
    println!("  same bed read at a pure-flesh gut (bias +1.0): edible {} (must be 0)", s3.edible);
    assert_eq!(s3.edible, 0, "a carnivore's census must not count plants; the gut is not reaching diet_yield");
    assert_eq!(s3.windfall, 2, "the raw windfall count must NOT depend on the gut -- unlike edible, it is a material census");
    assert_eq!(
        garden3.windfall_heat.iter().sum::<u32>(),
        2,
        "garden's windfall census must NOT depend on the gut either, same reasoning as s3.windfall"
    );

    println!("labforage selftest: PASS -- every band moves for a case whose answer is known");
}
