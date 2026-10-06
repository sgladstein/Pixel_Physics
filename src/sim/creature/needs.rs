//! **The needs-and-jobs walk: a second way for an ant to decide.** Built
//! beside today's ant and off until a harness turns it on, so if it fails
//! nothing else is lost (owner, 2026-10-06). The design of record is the doc
//! "A needs-and-jobs ant: design for a second walk"
//! (<https://claude.ai/code/artifact/6f9b6730-a887-4697-8689-906afc9afb0a>).
//!
//! **The switch is [`World::needs`], set at a frame, never an environment
//! variable**: a variable is read once per process and so cannot hand a
//! running colony over at a frame, and a `Chooser` variant would enrol the
//! walk in the 69 `Chooser::` comparisons that already exist. `None` is
//! today's ant and every hook below is one `is_some` test.
//!
//! **A hook, not the split of `act` the design planned** (slice 0). The
//! engineering review was right that the brain's outputs reach about ten
//! places in the tick and that `act` interleaves its mechanics with the hand
//! rules that gate them. Splitting it would have moved code that seven live
//! branches were editing on the day this was built
//! (`branchcheck.sh --who-touched src/sim/creature.rs`, 2026-10-06), so the
//! walk instead **drives `act` itself**: it hands `act` the urges its drive
//! wants (1 for the verb it means, 0 for the ones it does not, the brain's
//! own for the verbs it does not own yet), and `act`'s mechanics and the
//! gates on where a load may lie or a cut may go run exactly as they do for
//! today's ant. That is the design's "haul and dig as jobs ... with their
//! own act gates" taken literally. What a need must never meet is a veto,
//! and the guard in this module's tests checks that by outcome.
//!
//! **The ten places the brain reaches, and what the walk does at each**
//! (review finding E1):
//!
//! | Site | Today | Under this walk |
//! |---|---|---|
//! | step or stay (`Move`) | the brain's roll | [`kinesis`] |
//! | forage drive and pace | lifts `Move` for a needed forager | overridden by [`kinesis`] |
//! | launch (`Impulse`) | the brain | unchanged: an ant authors no `Impulse` |
//! | tumble after a lost roll | the chooser pauses | unchanged: a lost roll is a pause |
//! | `act`'s six urges | the brain | [`decide`]: the drive's verbs, the brain's for share and bite |
//! | the step (persistence, turn, noise) | `chooser_step` | [`step`] |
//! | trail A and B | the brain | unchanged until the walk owns trail laying |
//! | synapse tax | the brain, which still runs | unchanged, so starvation compares like with like |
//! | trace row | `DecisionRow` | unchanged, plus the walk's own rows |
//! | speculative brain | every ant, every tick | unchanged: the brain still runs |
//!
//! **Pass-through** ([`NeedsMode::Passthrough`]) runs every hook and hands
//! today's value straight back. It is the control for the hooks themselves:
//! a colony switched to it at any frame must stay identical to one that never
//! was, frame for frame, map for map and ledger row for ledger row. **Checked
//! 2026-10-06** (`deeptrace`, 60k frames, the switch at 20k): seeds 1 and 2
//! shipped, and seed 1 under `NEEDS_FIRST`, `CARRY_HOME` and `NEST_REST`,
//! byte-identical in every map, ledger, colony, stats, hunger and event row
//! to the build before the hooks, switch off and pass-through alike. The
//! positive control: a build nudging any one of the three live hooks (the
//! urges halved, the step chance cut by a tenth, one extra draw before the
//! step) differs at the first map after the switch and at none before.
//!
//! **The no-veto guard** is this module's tests: an ant at 30% of its budget
//! in each of six places, holding each of three loads, in either role, must
//! eat, reach the open air or dig if sealed, before it starves. It reads what
//! happened in the scene, never a weight, and today's ant fails 16 of its 36
//! rows (`the_guard_on_todays_ant`).

use super::*;

/// **Which walk the ants use once [`World::needs`] is set.**
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeedsMode {
    /// Every hook runs and hands today's value back unchanged: the identity
    /// control for the hooks.
    Passthrough,
}

impl NeedsMode {
    /// `passthrough`, as a harness names it. Anything else is refused rather
    /// than read as a default, so a typo cannot run the control under another
    /// arm's label.
    pub fn parse(raw: &str) -> NeedsMode {
        match raw.trim() {
            "passthrough" => NeedsMode::Passthrough,
            other => panic!("needs={other:?}: the walk's modes are passthrough"),
        }
    }
}

/// **The walk and everything it remembers**, held on [`World::needs`] so
/// nothing of it lives on `OrganismState`: the shipped ant's state, and so
/// its random stream, cannot be touched by it.
#[derive(Clone, Debug)]
pub struct NeedsWalk {
    pub mode: NeedsMode,
    /// The frame the walk was switched on, for the switch-time burn-in.
    pub since: u64,
}

impl NeedsWalk {
    pub fn new(mode: NeedsMode, frame: u64) -> NeedsWalk {
        NeedsWalk { mode, since: frame }
    }
}

/// What [`decide`] settled for this tick's `act`.
pub(super) enum Decided {
    /// Run `act` with these urges.
    Act([f32; brain::BRAIN_OUTPUTS]),
}

/// **Before `act`: what the walk wants done with the jaws and the mouth.**
/// Pass-through hands the brain's urges back as they are.
pub(super) fn decide(
    world: &mut World,
    _x: i32,
    _y: i32,
    _organism: OrganismId,
    _def: &CreatureDef,
    _inputs: &[f32; brain::BRAIN_INPUTS],
    outputs: &[f32; brain::BRAIN_OUTPUTS],
) -> Decided {
    match world.needs.as_ref().map(|n| n.mode) {
        Some(NeedsMode::Passthrough) | None => Decided::Act(*outputs),
    }
}

/// **Step or stay**: the chance this decision steps at all. Pass-through
/// hands today's chance back.
pub(super) fn kinesis(
    world: &World,
    _organism: OrganismId,
    _def: &CreatureDef,
    _head: (i32, i32),
    _inputs: &[f32; brain::BRAIN_INPUTS],
    p_move: f32,
) -> f32 {
    match world.needs.as_ref().map(|n| n.mode) {
        Some(NeedsMode::Passthrough) | None => p_move,
    }
}

/// **The step**, once the roll said step. Pass-through takes today's step:
/// the chooser's, or `step_chain`'s with the chooser off.
#[allow(clippy::too_many_arguments)]
pub(super) fn step(
    world: &mut World,
    organism: OrganismId,
    heading: u8,
    outputs: &[f32; brain::BRAIN_OUTPUTS],
    def: &CreatureDef,
    draw: &mut rng::Rng,
    chooser: Chooser,
) -> bool {
    match world.needs.as_ref().map(|n| n.mode) {
        Some(NeedsMode::Passthrough) | None => {
            if chooser == Chooser::Off {
                step_chain(world, organism, heading, outputs, def, draw)
            } else {
                chooser_step(world, organism, heading, outputs, def, draw, chooser)
            }
        }
    }
}

/// **After the move**: what the walk remembers of this tick. Pass-through
/// remembers nothing.
pub(super) fn after(world: &mut World, _organism: OrganismId, _def: &CreatureDef, _moved: bool) {
    match world.needs.as_ref().map(|n| n.mode) {
        Some(NeedsMode::Passthrough) | None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;

    // --- the no-veto guard ----------------------------------------------------
    //
    // The design's *Measurement and acceptance* and the rule audit's §5 (2026-10-06):
    // an ant at 30% of its budget, in every place, holding every load, in either
    // role, must within a bounded number of frames have eaten, reached the open
    // air, or -- where nothing joins its pocket to the air -- be digging; and if it
    // let go of a load, the way out must still be open afterwards. **It asserts
    // what happened in the scene, never the sign of a weight**: the history's
    // vetoes were each a rule that let a weight stay positive while a gate
    // further down took the act away.

    /// The founding ground's top row: everything at or below it is ground.
    const GROUND: i32 = 40;

    /// How long a row has, in frames. An ant ticks about once in six frames
    /// and pays about a quarter of a joule a tick, so at 30% of the budget it
    /// has about 1,400 frames to live: a row that has met nothing by then has
    /// starved, and the bound only stops a row the ant survives.
    const BOUND: u64 = 3_000;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Place {
        /// Just inside the door, at the top of the founding shaft.
        Door,
        /// The founding chamber at the shaft's foot, 6-7 rows down.
        Chamber,
        /// A room 20-21 rows down, at the end of a shaft below the chamber.
        Deep,
        /// A tunnel through the spoil mound, open to the air at its west end.
        MoundTunnel,
        /// A closed pocket in the mound that nothing joins.
        MoundPocket,
        /// A closed pocket in the ground, 30 rows down, that nothing joins.
        Buried,
    }

    impl Place {
        const ALL: [Place; 6] = [
            Place::Door,
            Place::Chamber,
            Place::Deep,
            Place::MoundTunnel,
            Place::MoundPocket,
            Place::Buried,
        ];

        /// Where the ant is put down.
        fn at(self) -> (i32, i32) {
            match self {
                Place::Door => (61, 41),
                Place::Chamber => (58, 47),
                Place::Deep => (60, 61),
                Place::MoundTunnel => (40, 37),
                Place::MoundPocket => (49, 33),
                Place::Buried => (21, 71),
            }
        }

        /// Nothing joins it to the air, so digging is the need met.
        fn sealed(self) -> bool {
            matches!(self, Place::MoundPocket | Place::Buried)
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Load {
        Nothing,
        /// A pellet of dug soil in the jaws.
        Pellet,
        /// A cell of food picked up at home for the store, in the jaws.
        Food,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Role {
        Forager,
        /// Nest-bound for life, as one ant in four is today (`is_nest_bound`).
        NestWorker,
    }

    /// One row of the guard: what the ant did, and when.
    #[derive(Clone, Debug)]
    struct Row {
        place: Place,
        load: Load,
        role: Role,
        /// The first frame its crop held food or its energy rose.
        ate: Option<u64>,
        /// The first frame its head stood above the ground with nothing overhead.
        out: Option<u64>,
        /// The first frame it cut a cell, and how many it cut.
        dug: Option<u64>,
        digs: u64,
        died: Option<u64>,
        /// It let go of the load it started with.
        let_go: bool,
        /// An open way joined it to the open air at the start, and at the end.
        way_before: bool,
        way_after: bool,
        head: (i32, i32),
    }

    /// Cells a sealed ant must cut for its digging to count: one cut is a
    /// roll that came up, three is an ant working at the wall.
    const DIGGING: u64 = 3;

    impl Row {
        /// **When its need was met**, if before it died. Eating always counts.
        /// The open air counts unless it holds food, which a starving ant eats
        /// rather than carries (the design's hunger: *eats what it holds*).
        /// Digging counts only where nothing joined it to the air.
        fn met(&self) -> Option<u64> {
            let out = self.out.filter(|_| self.load != Load::Food);
            let dug = self.dug.filter(|_| self.place.sealed() && self.digs >= DIGGING);
            [self.ate, out, dug]
                .into_iter()
                .flatten()
                .min()
                .filter(|&f| self.died.is_none_or(|d| f < d))
        }

        /// Met, and no load it let go of shut a way that was open.
        fn green(&self) -> bool {
            self.met().is_some() && !(self.let_go && self.way_before && !self.way_after)
        }
    }

    /// **The guard's bed**: `mound_world`'s ground and mound (`creature.rs`'s
    /// tests), with the rest of a nest under them and food on the surface.
    /// Loose soil from row 40 over a stone floor at 92; a nest site at column
    /// 60 whose founding cut is a shaft on columns 60-61 down to row 45 and a
    /// chamber on rows 46-47; a second shaft from the chamber's floor down to
    /// a room on rows 60-61; a closed pocket in the ground on rows 70-71; a
    /// mound of packed soil on columns 30-55 over rows 26-39 with a tunnel on
    /// rows 35-37 open to the air at column 29 and a closed pocket on rows
    /// 31-33; and a heap of player food 25 columns east of the door.
    ///
    /// **Every open cell in the ground is lined with packed soil**, as the
    /// founding cut lines its own and a digging ant lines the gallery it cuts
    /// (`soil.ron`'s `packs_into`): a hole in loose soil is gone in five frames
    /// (`packedsoil.ron`), and the first draft of this bed lost its shaft,
    /// chamber and the mound pocket's one-row roof that way, burying the ant
    /// it was testing. The mound is fourteen rows tall so that no roof over
    /// its holes is a single row: a packed cell with air beneath it holds on
    /// three contacts, and a one-row roof has two. **Every room is two rows
    /// tall**, as the founding chamber is: an ant on the floor of a taller one
    /// can step only along the floor until it finds a wall (its usable
    /// headings were east and west alone), so a shaft in the middle of a
    /// three-row room's ceiling is out of reach, and the first draft's deep
    /// room starved every empty ant in it for that reason, which is the
    /// room's shape and not a rule.
    fn guard_bed() -> World {
        let mut w = World::new(Rect::new(0, 0, 119, 99));
        w.brood = Some(false);
        w.bud_at_nest = Some(false);
        let soil = w.materials.id_of("soil").expect("soil material");
        let packed = w.materials.id_of("packedsoil").expect("packed soil material");
        let food = w.materials.id_of("provisions").expect("provisions material");
        for x in 0..=119 {
            for y in GROUND..=99 {
                let stone = y >= 92 || x == 0 || x == 119;
                w.set(
                    x,
                    y,
                    if stone {
                        Cell::new(material::STONE, 0)
                    } else {
                        Cell::new(soil, 0)
                    },
                );
            }
        }
        w.register_nest_site(60, 38, 2);
        let cut = crate::sim::world::ShaftFootprint {
            x0: 60,
            x1: 61,
            top: 40,
            bottom: 45,
            mouth_bottom: 41,
            chamber_x0: 56,
            chamber_x1: 65,
            chamber_top: 46,
            chamber_bottom: 47,
            side: None,
        };
        w.nest_sites[0].shaft = Some(cut);
        let open: Vec<(i32, i32)> = (40..=59)
            .flat_map(|y| [(60, y), (61, y)])
            .chain((46..=47).flat_map(|y| (56..=65).map(move |x| (x, y))))
            .chain((60..=61).flat_map(|y| (54..=67).map(move |x| (x, y))))
            .chain((70..=71).flat_map(|y| (18..=24).map(move |x| (x, y))))
            .collect();
        for &(x, y) in &open {
            w.set(x, y, Cell::EMPTY);
        }
        for &(x, y) in &open {
            for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                if w.get(x + dx, y + dy).material == soil {
                    w.set(x + dx, y + dy, Cell::new(packed, 0));
                }
            }
        }
        for y in 26..GROUND {
            for x in 30..=55 {
                let tunnel = (35..=37).contains(&y) && x <= 45;
                let pocket = (31..=33).contains(&y) && (46..=52).contains(&x);
                w.set(
                    x,
                    y,
                    if tunnel || pocket {
                        Cell::EMPTY
                    } else {
                        Cell::new(packed, 0)
                    },
                );
            }
        }
        for y in 37..GROUND {
            for x in 84..=90 {
                w.set(x, y, Cell::new(food, 0));
            }
        }
        w.step_nest_dug();
        step_nest_rest(&mut w);
        w
    }

    /// **The guard's bed stands with nobody in it.** The first draft's did not:
    /// its shaft, chamber and the mound pocket's roof fell in within five
    /// frames and buried the ant each row was testing, so 22 of 36 rows read
    /// red for the ground, not for any rule. Every cell the bed opens must
    /// still be open after `BOUND` frames, and every place must be where an
    /// ant can be put down. Watched red with the packed lining taken out.
    #[test]
    fn the_guard_bed_stands() {
        let mut w = guard_bed();
        let open: Vec<(i32, i32)> = (0..=119)
            .flat_map(|x| (GROUND..=91).map(move |y| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == material::EMPTY)
            .collect();
        let mound: Vec<(i32, i32)> = (30..=55)
            .flat_map(|x| (26..GROUND).map(move |y| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == material::EMPTY)
            .collect();
        // Shaft 40, chamber 16 beside it, deep room 28, buried pocket 14; the
        // mound's tunnel 48 and pocket 21.
        assert_eq!(
            (open.len(), mound.len()),
            (98, 69),
            "test setup: the bed did not open the cells it names"
        );
        for _ in 0..BOUND {
            crate::sim::update::step(&mut w);
            w.step_active_sites();
        }
        let filled: Vec<(i32, i32)> = open
            .iter()
            .chain(&mound)
            .copied()
            .filter(|&(x, y)| w.get(x, y).material != material::EMPTY)
            .collect();
        assert!(
            filled.is_empty(),
            "{} of the bed's open cells filled in {BOUND} frames, first {:?}",
            filled.len(),
            filled.first()
        );
        for place in Place::ALL {
            let mut w = guard_bed();
            let (x, y) = place.at();
            assert!(
                plant_creature_seed(&mut w, x, y, "ant").is_some(),
                "no ant fits at {place:?} {:?}",
                place.at()
            );
            assert_eq!(
                open_to_the_air(&w, (x, y), w.get(x, y).organism_id()),
                !place.sealed(),
                "{place:?} is not as joined to the air as it is named"
            );
        }
    }

    /// One row: the bed, one ant at `place` at 30% of its budget holding
    /// `load` in `role`, run for [`BOUND`] frames under `walk`, with `setup`
    /// applied to the world first (a switch the arm runs under).
    fn guard_row(place: Place, load: Load, role: Role, walk: Option<NeedsMode>, setup: fn(&mut World)) -> Row {
        let mut w = guard_bed();
        setup(&mut w);
        let (x, y) = place.at();
        let site = plant_creature_seed(&mut w, x, y, "ant").expect("test setup: the ant does not fit");
        w.schedule_active_site(site);
        let a = w.get(x, y).organism_id();
        assert_ne!(a, 0, "test setup: no ant at {:?}", place.at());
        let soil = w.materials.id_of("soil").expect("soil material");
        let food = w.materials.id_of("provisions").expect("provisions material");
        let start = w
            .species
            .get(w.organism(a).expect("live").species)
            .creature
            .as_ref()
            .expect("a creature")
            .start_energy;
        let st = w.organism_mut(a).expect("live");
        st.energy = 0.3 * start;
        st.nest_bound_until = if role == Role::NestWorker { u64::MAX } else { 0 };
        st.spoil = match load {
            Load::Nothing => None,
            Load::Pellet => Some(crate::sim::organism::Spoil {
                cell: Cell::new(soil, 0),
                store: false,
            }),
            Load::Food => Some(crate::sim::organism::Spoil {
                cell: Cell::new(food, 0),
                store: true,
            }),
        };
        w.needs = walk.map(|mode| Box::new(NeedsWalk::new(mode, w.frame)));
        let digs = w.creature_stats.digs;
        let way_before = open_to_the_air(&w, (x, y), a);
        let mut row = Row {
            place,
            load,
            role,
            ate: None,
            out: None,
            dug: None,
            digs: 0,
            died: None,
            let_go: false,
            way_before,
            way_after: way_before,
            head: (x, y),
        };
        let mut energy = 0.3 * start;
        for f in 0..BOUND {
            crate::sim::update::step(&mut w);
            w.step_active_sites();
            let Some(st) = w.organism(a) else {
                row.died = Some(f);
                break;
            };
            row.head = st.chain[0];
            if row.ate.is_none() && (st.crop.is_some() || st.energy > energy + 1.0) {
                row.ate = Some(f);
            }
            energy = st.energy;
            let (hx, hy) = row.head;
            if row.out.is_none() && hy < GROUND && !under_cover(&w, hx, hy) {
                row.out = Some(f);
            }
            row.digs = w.creature_stats.digs - digs;
            if row.dug.is_none() && row.digs > 0 {
                row.dug = Some(f);
            }
            row.let_go |= load != Load::Nothing && st.spoil.is_none();
        }
        row.way_after = open_to_the_air(&w, row.head, a);
        row
    }

    /// Whether a 4-connected walk over empty cells (and the ant's own body)
    /// joins `from` to a cell above the ground with nothing overhead.
    fn open_to_the_air(w: &World, from: (i32, i32), a: OrganismId) -> bool {
        let passable = |x: i32, y: i32| {
            (0..=119).contains(&x) && (0..=99).contains(&y) && {
                let c = w.get(x, y);
                c.material == material::EMPTY || c.organism_id() == a
            }
        };
        let mut seen = std::collections::HashSet::from([from]);
        let mut todo = vec![from];
        while let Some((x, y)) = todo.pop() {
            if y < GROUND && !under_cover(w, x, y) {
                return true;
            }
            for n in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                if passable(n.0, n.1) && seen.insert(n) {
                    todo.push(n);
                }
            }
        }
        false
    }

    /// Every row of the guard under `walk`: six places, three loads, two roles.
    fn guard(walk: Option<NeedsMode>, setup: fn(&mut World)) -> Vec<Row> {
        Place::ALL
            .into_iter()
            .flat_map(|p| {
                [Load::Nothing, Load::Pellet, Load::Food]
                    .into_iter()
                    .map(move |l| (p, l))
            })
            .flat_map(|(p, l)| [Role::Forager, Role::NestWorker].into_iter().map(move |r| (p, l, r)))
            .map(|(p, l, r)| guard_row(p, l, r, walk, setup))
            .collect()
    }

    fn print_guard(rows: &[Row]) {
        let at = |f: Option<u64>| f.map_or("-".to_string(), |f| f.to_string());
        println!(
            "{:<12} {:<10} {:<11} {:>5} {:>5} {:>5} {:>4} {:>5} {:>7} {:>9} {:<10} verdict",
            "place", "load", "role", "ate", "out", "dug", "cuts", "died", "let go", "way b/a", "head"
        );
        for r in rows {
            println!(
                "{:<12} {:<10} {:<11} {:>5} {:>5} {:>5} {:>4} {:>5} {:>7} {:>9} {:<10} {}",
                format!("{:?}", r.place),
                format!("{:?}", r.load),
                format!("{:?}", r.role),
                at(r.ate),
                at(r.out),
                at(r.dug),
                r.digs,
                at(r.died),
                r.let_go,
                format!("{}/{}", u8::from(r.way_before), u8::from(r.way_after)),
                format!("{:?}", r.head),
                if r.green() { "green" } else { "RED" }
            );
        }
        println!(
            "{} of {} rows green",
            rows.iter().filter(|r| r.green()).count(),
            rows.len()
        );
    }

    /// **The guard on today's ant**, as a report: which places, loads and
    /// roles leave a hungry ant with no way to meet its need. Run with
    /// `cargo test --release --lib needs::tests::the_guard_on_todays_ant -- --ignored --nocapture`
    /// (about 2 s, debug or release).
    ///
    /// **Read 2026-10-06 on this branch (main 3ba1e7bd and switches that are
    /// off): 20 of 36 rows green, the same for foragers and nest workers but
    /// in two rows.** The
    /// 16 red, in four groups:
    /// - **Holding food, it starves** (door, chamber, deep room; 6 rows): it
    ///   puts its store load down, walks out and dies without eating.
    /// - **In the mound's tunnel, 11 cells from the air** (4 rows): nothing
    ///   pulls it out of the mound (`MOUND_OUT`'s `way` is off), and its walk
    ///   paces the tunnel's floor until it starves.
    /// - **In a pocket in the ground, lean, it never cuts** (4 rows): the lean
    ///   rule takes its dig roll, and `MOUND_OUT=dig` lets only the mound's
    ///   pockets off it.
    /// - **A forager in the mound's pocket stops after one cut** (2 rows); a
    ///   nest worker there cuts five.
    ///
    /// Under `NEEDS_FIRST` with every part on, 26 of 36: the buried pocket
    /// digs and the chamber's carrier eats its load
    /// ([`the_guard_under_needs_first`]).
    #[test]
    #[ignore = "a report, not a gate: prints the guard's table for today's ant"]
    fn the_guard_on_todays_ant() {
        print_guard(&guard(None, |_| {}));
    }

    /// **...and under `NEEDS_FIRST` with every part on**, the switch built to
    /// waive the blocks a hungry or shut-in ant meets: the rows it turns
    /// green are the guard seeing a block lifted.
    #[test]
    #[ignore = "a report, not a gate: prints the guard's table under NEEDS_FIRST"]
    fn the_guard_under_needs_first() {
        print_guard(&guard(None, |w| w.needs_first = Some(NeedsFirst::ON)));
    }
}
