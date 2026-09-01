//! **The evolution lab — the biosphere the second game is played in.**
//!
//! A sealed box of soil under a grow light, with plants and creatures living
//! in it, that the player can run at speed and read the state of. The design
//! of record is `Reports/evolution-lab-design-guide-2026-08-30.md`; this
//! module is its Gate 1 (*one hand-built box that runs plants and creatures
//! together*) and Gate 3 (*the two-phase loop, end to end*).
//!
//! **It is a second game, not a second engine.** Everything below the scene
//! is the shipped simulation, reached through `sim::frame::step` — the same
//! tick `src/main.rs` runs, in the same order, with nothing skipped. The
//! lab's frame time comes from what is *not in the box* rather than from what
//! is not in the binary: measured (feasibility §3c), a bed with no rock pays
//! 0.028 ms for the structural scheduler against 3.389 ms outdoors, blasts
//! 0.000, particles 0.000, the gnome 0.001. So there is nothing to strip and
//! stripping is what would make this a fork.
//!
//! Four files, and each is owned by one concern so that concurrent work on
//! them does not collide (`CLAUDE.md`, *working alongside another session*):
//!
//! | file | owns |
//! |---|---|
//! | `scene` | the box: geometry, soil, light, partitions, founders |
//! | `time`  | paused vs running, the speed dial, and what it actually achieved |
//! | `stats` | the census, and the page that draws it |
//! | `ui`    | the control bar along the bottom, its pages, and the mouse |
//! | `params`| which numbers the player can reach, and how they are written |
//! | this file | `Lab` — the state the four above are wired into, and the frame |
//!
//! **The viewport is `app::WIDTH` x `app::HEIGHT`**, deliberately the same
//! framebuffer the sandbox uses, so `render::Renderer` and `hud` need no
//! second set of geometry and a screenshot from either game is comparable
//! with one from the other.

pub mod batch;
pub mod params;
pub mod scene;
pub mod stats;
pub mod time;
pub mod ui;

use crate::render::Renderer;
use crate::sim::explosion::Blasts;
use crate::sim::frame;
use crate::sim::particle::ParticleSystem;
use crate::sim::player;
use crate::sim::world::World;

pub use crate::app::{HEIGHT, WIDTH};

/// How much a rack still is shrunk in each axis. Kept beside `Thumb` rather
/// than in `ui`, because the downscale happens here.
const THUMB_SHRINK: u32 = 4;

/// The whole lab: a world, the systems that live beside the cell grid, a view
/// on it, and the two things the player drives — time and what is on screen.
pub struct Lab {
    pub world: World,
    pub particles: ParticleSystem,
    pub blasts: Blasts,
    pub renderer: Renderer,
    /// Present so `frame::step` gets the same arguments the sandbox gives it.
    /// The lab never summons a gnome, and `player::step` is a no-op without
    /// one — measured at 0.001 ms/frame, which is the whole reason this is a
    /// dormant field rather than a phase the lab skips.
    player_tuning: player::Tuning,
    /// The speed dial and the two-phase loop. See `time`.
    pub time: time::TimeControl,
    /// The census and its page. See `stats`.
    pub stats: stats::Stats,
    /// The control bar along the bottom, the pages it opens, and the mouse.
    /// See `ui`.
    pub ui: ui::Ui,
    /// What the box was built from, kept so a reset rebuilds the same lab and
    /// so the stats page can say what bed these numbers are from.
    pub spec: scene::LabBox,
    /// The key list. On by default on a fresh lab and dismissed by any key,
    /// because there is no other way to discover a control here — the sandbox
    /// grew its bindings a key at a time in front of somebody who already knew
    /// them, and the lab has no such person.
    ///
    /// **It is no longer the only way**, which is what `ui` is for: the bar
    /// along the bottom shows every control as a button and prints its key
    /// under it, so the page is now a reference rather than the interface.
    pub show_help: bool,
    /// **A live paint stroke: the last world cell the brush was at, and
    /// whether it is erasing.**
    ///
    /// Held as a *world* cell, not a screen pixel, for `App::begin_drag`'s
    /// reason: a screen point held across a camera move names a different
    /// cell every frame, and the stroke would smear backwards as the view
    /// panned. `None` between strokes.
    stroke: Option<Stroke>,
    /// **The rest of the rack — every chamber that is not the one on screen.**
    ///
    /// The active chamber is *not* in here: its world, spec, stats, particles
    /// and blasts are the inline fields above, exactly as they were when the
    /// lab held one box. That is deliberate and it is the whole reason this
    /// change is cheap — `lab.world` still means "the box you are looking at"
    /// at all 117 call sites in the binary and in `examples/lab*.rs`, so a
    /// rack costs those files nothing and cannot collide with another lane
    /// editing them.
    ///
    /// **Invariant: `rack[active]` is `None` and every other entry is
    /// `Some`.** The hole is where the inline fields came from and where they
    /// go back on a switch, which is what keeps indices stable — a chamber
    /// keeps its number for its whole life, so a tab does not renumber itself
    /// when you look at a different box. `chamber_count`,
    /// `chamber_summaries` and `switch_to` are the only things that need to
    /// know, and `rack_invariant_holds` pins it.
    rack: Vec<Option<Chamber>>,
    /// Which rack slot the inline fields belong to.
    active: usize,
    /// The label of the box on screen, inline with the rest of its state.
    label: Option<String>,
    /// **Runs whose world was dropped for the memory budget.**
    ///
    /// Nothing is lost that cannot be recomputed: the spec reproduces the run
    /// exactly, which is what `tests/determinism.rs`'s lab bed asserts. They
    /// are listed on the rack page rather than silently discarded, because a
    /// batch that quietly returns fewer rows than it ran is a batch nobody
    /// can trust the count of.
    pub on_record: Vec<OnRecord>,
    /// A rack of copies running headless in the background, if one is.
    ///
    /// **`Option`, and only ever one.** Two batches at once would contend for
    /// the same cores and neither would finish sooner; the rack page refuses
    /// to start a second rather than queueing it, so "how long is left" stays
    /// a question with one answer.
    pub batch: Option<batch::Batch>,
    /// What the last batch asked for, kept so the page's dials hold their
    /// setting between runs rather than resetting to the default each time.
    pub batch_spec: batch::BatchSpec,
    /// The still of the box on screen, if one has been taken. Inline with the
    /// rest of the active chamber's state, for `world`'s reason.
    thumb: Option<Thumb>,
    /// Set when the thing on screen changed for a reason the dirty-rect skip
    /// cannot see — today, a chamber switch. Consumed by the next `draw`.
    ///
    /// `Renderer` carries the previous frame's rectangles, and those belong to
    /// the box we just left; without this the old chamber is painted under the
    /// new one wherever the new one happens to be settled.
    view_dirty: bool,
}

/// **A chamber that is not currently on screen.**
///
/// Everything here is per-box state, and each field is in this struct rather
/// than on `Lab` because leaving it shared produces a visible bleed on a
/// switch rather than a compile error:
///
/// | field | what leaks if it stays shared |
/// |---|---|
/// | `world`, `spec` | the obvious one |
/// | `stats` | one box's census drawn over another's bed |
/// | `history` | `Ui`'s population strip, same failure one level down |
/// | `particles`, `blasts` | a blast's debris following you into a box that never had one |
///
/// `Renderer` is deliberately *not* here — it is shared, because it is pure
/// CPU state (`render.rs`) and one per chamber would multiply its caches by
/// the rack. What it does carry is the previous frame's dirty rectangles, so
/// `switch_to` forces a full redraw instead; a switch that skips that paints
/// the old box under the new one.
pub struct Chamber {
    pub world: World,
    pub spec: scene::LabBox,
    pub stats: stats::Stats,
    pub particles: ParticleSystem,
    pub blasts: Blasts,
    /// The population strip `Ui` keeps for the bar. Parked with its box for
    /// the reason the table above gives.
    pub history: ui::History,
    /// The last still taken of this box, if any. See [`Thumb`].
    pub thumb: Option<Thumb>,
    /// What to call it on the rack page. `None` is "its number", which is
    /// what a chamber you made yourself gets; a batch names its own so a rack
    /// of fifty says where each row came from.
    pub label: Option<String>,
}

/// **A still of one chamber, for the rack page.**
///
/// Taken when a row is clicked and kept on the chamber, rather than rendered
/// per frame: `Renderer::draw` needs `&mut` where the page has it borrowed
/// shared, and more to the point a frozen box repainted sixty times a second
/// is the same picture sixty times.
///
/// **It is thrown away when the chamber runs.** A picture of a box as it was
/// four thousand ticks ago, sitting under a live census, is the stale side
/// table this repo keeps paying for — better no picture than a wrong one.
pub struct Thumb {
    pub w: u32,
    pub h: u32,
    /// RGBA, `w * h * 4` bytes.
    pub rgba: Vec<u8>,
    /// The chamber's frame when it was taken, so `Lab` can tell a picture
    /// that still matches its box from one that does not.
    pub frame: u64,
}

/// A finished run kept as numbers rather than as a world.
///
/// About 10 KB against a world's 2.5 MB, and it reproduces its run exactly —
/// the spec carries the seed, and a chamber built from it and run for the
/// same ticks is bit-identical. See `tests/determinism.rs`'s lab bed.
pub struct OnRecord {
    pub spec: scene::LabBox,
    pub census: stats::Census,
    pub label: String,
    /// Set while this record is being re-run back into a world.
    ///
    /// The row stays on the page rather than vanishing for the minute the
    /// rebuild takes: a row that disappears when you press its own button
    /// reads as having been thrown away.
    pub rebuilding: bool,
}

impl OnRecord {
    /// How many ticks this run did — **read off the census rather than stored
    /// beside it**, because `Census::frame` *is* the world frame the run
    /// ended on. A second copy of the number is a second thing that can be
    /// wrong, and a rebuild that ran a different length than the record it
    /// replaces would not be the same chamber.
    pub fn ticks(&self) -> u64 {
        self.census.frame
    }
}

/// What the rack page needs to know about batches. A snapshot, because the
/// real thing is behind a mutex on a worker thread and the page must never
/// wait on it.
#[derive(Clone, Copy, Debug, Default)]
pub struct BatchBar {
    pub copies: u32,
    pub frames: u64,
    /// `None` when no rack is running.
    pub progress: Option<batch::Progress>,
}

/// One row of the chamber menu: enough to compare two boxes without
/// unfreezing either.
///
/// **`frame` is in here on purpose.** A frozen chamber that is quietly still
/// ticking looks identical to a correctly frozen one in any screenshot, so
/// the number that says which it is has to be on the row —
/// `CLAUDE.md`'s *"did it fire at all" needs a counter, not a picture*.
pub struct ChamberSummary {
    pub index: usize,
    pub active: bool,
    /// **A box you can walk into, or one kept only as numbers.** Not a
    /// cosmetic distinction: an on-record row's world was dropped for the
    /// memory budget, so `ENTER` cannot open it. Saying which is which on the
    /// row is the difference between a rack that is honest about what it
    /// holds and one where some rows mysteriously do nothing.
    pub on_record: bool,
    /// Set while an on-record row is being re-run back into a world.
    pub rebuilding: bool,
    pub label: String,
    pub seed: u64,
    /// Simulated ticks this box has run. Frozen chambers hold still.
    pub frame: u64,
    /// `None` before the census has ever run — a box built and never stepped.
    pub census: Option<stats::Census>,
}

/// One in-progress brush stroke.
#[derive(Clone, Copy)]
struct Stroke {
    /// Where the brush was last applied, in world cells.
    last: (i32, i32),
    erase: bool,
}

impl Lab {
    /// Build the box `spec` describes and start it **stopped**. Nothing
    /// ticks until the player presses `SPACE` or asks for a speed.
    pub fn new(spec: scene::LabBox) -> Self {
        let mut world = spec.build();
        earth_toned_nest(&mut world);
        // **The shelf is read once, at start.** It is a directory, and a
        // `read_dir` per frame is a syscall storm to answer a question whose
        // answer only changes when a button is pressed -- the three that
        // change it reload it, and `RELOAD` covers a jar added from outside.
        // Read *here* rather than lazily on first open, so that a rack
        // carried over from a previous session is already in the count the
        // moment the box appears.
        let mut ui = ui::Ui::new();
        ui.reload_shelf();
        let spec_for_batch = spec.clone();
        Self {
            world,
            particles: ParticleSystem::new(),
            blasts: Blasts::new(),
            renderer: Renderer::new(),
            player_tuning: player::Tuning::default(),
            time: time::TimeControl::new(),
            stats: stats::Stats::new(),
            ui,
            spec,
            // Down only when something explicitly asks for it. The one caller
            // is a headless capture wanting to photograph what is *under* the
            // key list, which is otherwise unreachable without a keypress and
            // so unphotographable on a box with no keyboard.
            show_help: std::env::var("PIXEL_PHYSICS_LAB_HELP").as_deref() != Ok("0"),
            stroke: None,
            // One chamber, and it is the one on screen — so the rack is a
            // single hole. Every later chamber is pushed beside it.
            rack: vec![None],
            active: 0,
            label: None,
            on_record: Vec::new(),
            batch: None,
            batch_spec: batch::BatchSpec {
                base: spec_for_batch,
                replicates: 8,
                sweep: None,
                frames: 9_000,
                seed0: 1,
                // ~2.5 MB a chamber at 512x320, so this holds about a hundred
                // of them. Past it a run keeps its record and drops its world,
                // and the row says so.
                keep_bytes: 256 * 1024 * 1024,
            },
            thumb: None,
            view_dirty: false,
        }
    }

    /// Rebuild the box from the same spec, keeping the view and the dial.
    pub fn reset(&mut self) {
        // **The rules the player set survive the rebuild; the box does not.**
        // `spec.build()` returns a brand-new `World` at its defaults, so a
        // switch thrown on the parameters page would silently come back on
        // the next `REBUILD` -- and `REBUILD` is exactly what a player presses
        // after changing the bed, i.e. in the middle of the experiment the
        // switch was set for. Carried explicitly rather than by making the
        // builder take them, so the list of what is a *setting* rather than
        // part of the box is readable in one place.
        let plant_load_failure = self.world.plant_load_failure;
        self.world = self.spec.build();
        self.world.plant_load_failure = plant_load_failure;
        earth_toned_nest(&mut self.world);
        self.particles = ParticleSystem::new();
        self.blasts = Blasts::new();
        self.stats = stats::Stats::new();
    }

    // ------------------------------------------------------------ the rack

    /// How many chambers the facility holds, the one on screen included.
    pub fn chamber_count(&self) -> usize {
        self.rack.len()
    }

    /// Which chamber is on screen.
    pub fn active_chamber(&self) -> usize {
        self.active
    }

    /// **`rack[active]` is the hole, everything else is a box.**
    ///
    /// The one invariant this whole representation rests on, exposed so a
    /// guard can assert it rather than a comment claiming it.
    pub fn rack_invariant_holds(&self) -> bool {
        self.active < self.rack.len()
            && self.rack.iter().enumerate().all(|(i, slot)| slot.is_none() == (i == self.active))
    }

    /// One row per chamber, for the tabs and the menu, without unfreezing
    /// anything.
    ///
    /// **`frame` is on every row deliberately.** A frozen chamber that is
    /// quietly still ticking and one that is genuinely held look identical in
    /// any picture of the rack, so the number that separates them travels with
    /// the row — `CLAUDE.md`, *"did it fire at all" needs a counter, not a
    /// picture*.
    pub fn chamber_summaries(&self) -> Vec<ChamberSummary> {
        (0..self.rack.len())
            .map(|i| {
                let active = i == self.active;
                let (spec, stats, world) = match &self.rack[i] {
                    Some(ch) => (&ch.spec, &ch.stats, &ch.world),
                    // The hole: the box on screen lives in the inline fields.
                    None => (&self.spec, &self.stats, &self.world),
                };
                let label = match &self.rack[i] {
                    Some(ch) => ch.label.clone(),
                    None => self.label.clone(),
                };
                ChamberSummary {
                    index: i,
                    active,
                    on_record: false,
                    rebuilding: false,
                    label: label.unwrap_or_else(|| format!("{}", i + 1)),
                    seed: spec.seed,
                    frame: world.frame,
                    census: stats.census().cloned(),
                }
            })
            // Runs whose world was dropped for the budget come after the
            // chambers, still with their numbers. They are listed rather than
            // discarded because a batch that quietly returns fewer rows than
            // it ran is a batch nobody can trust the count of.
            .chain(self.on_record.iter().enumerate().map(|(k, r)| ChamberSummary {
                index: self.rack.len() + k,
                active: false,
                on_record: true,
                rebuilding: r.rebuilding,
                label: r.label.clone(),
                seed: r.spec.seed,
                frame: r.census.frame,
                census: Some(r.census.clone()),
            }))
            .collect()
    }

    /// Put chamber `i` on screen and park the one that was.
    ///
    /// **A swap, never a rebuild.** The outgoing box keeps its world, its
    /// census, its particles and its strip exactly as they were, so switching
    /// away and back is lossless and a frozen chamber resumes on the tick it
    /// stopped at — there is nothing to restore because nothing was thrown
    /// away. Freezing costs nothing at all: a `World` owns no threads and no
    /// timers, so one that is not stepped is simply not stepped, and its
    /// active-site heap and awake-chunk set are still standing when it comes
    /// back.
    ///
    /// Out of range, or already active, is a no-op rather than a panic: this
    /// is reached from a click on a tab, and a stale tab rectangle is an
    /// ordinary thing for a click to land on.
    pub fn switch_to(&mut self, i: usize) {
        // `i` can address an on-record row, which is past the rack and has no
        // world to swap in. A no-op, like any other stale click.
        if i == self.active || !self.rack.get(i).is_some_and(|slot| slot.is_some()) {
            return;
        }
        // Take the incoming box out first so every `replace` below has a real
        // value to put in place — `World` has no default and no placeholder,
        // which is what makes this a swap rather than a take-then-fill.
        let incoming = self.rack[i].take().expect("checked Some directly above");
        let outgoing = Chamber {
            world: std::mem::replace(&mut self.world, incoming.world),
            spec: std::mem::replace(&mut self.spec, incoming.spec),
            stats: std::mem::replace(&mut self.stats, incoming.stats),
            particles: std::mem::replace(&mut self.particles, incoming.particles),
            blasts: std::mem::replace(&mut self.blasts, incoming.blasts),
            history: std::mem::replace(&mut self.ui.history, incoming.history),
            // The outgoing box gets a fresh still on the way out -- this is
            // the one moment its picture is both wanted and free, because the
            // frame just drawn *is* that picture.
            thumb: std::mem::replace(&mut self.thumb, incoming.thumb),
            label: std::mem::replace(&mut self.label, incoming.label),
        };
        self.rack[self.active] = Some(outgoing);
        self.active = i;
        // A stroke belongs to the box it was started on, and a brush that
        // carried across would draw a line from wherever the cursor was in the
        // old chamber to wherever it is in this one.
        self.stroke = None;
        self.view_dirty = true;
    }

    /// Build a new chamber from `spec` and park it at the end of the rack.
    ///
    /// Returns its index. It is **not** switched to — adding a box and
    /// walking into it are two decisions, and a batch adds fifty.
    pub fn add_chamber(&mut self, spec: scene::LabBox) -> usize {
        let mut world = spec.build();
        earth_toned_nest(&mut world);
        self.rack.push(Some(Chamber {
            world,
            spec,
            stats: stats::Stats::new(),
            particles: ParticleSystem::new(),
            blasts: Blasts::new(),
            history: ui::History::default(),
            thumb: None,
            label: None,
        }));
        self.rack.len() - 1
    }

    /// Copy the box on screen — its recipe, not its contents.
    ///
    /// **A world cannot be copied and this is not a limitation being worked
    /// around, it is the right operation.** `World` is not `Clone`; `LabBox`
    /// is, and it carries the seed, so building from the spec reproduces the
    /// box exactly. The duplicate therefore starts at frame 0 rather than
    /// mid-life, which is what you want from "another one of these".
    ///
    /// **`reseed` is the whole difference between a copy and a replicate**,
    /// and getting it wrong is silent: at the same seed every draw in the
    /// engine is a pure function of `(world.seed, identity, position)`, so the
    /// duplicate is not a similar box, it is a **bit-identical** one, and a
    /// rack of them is one sample wearing many labels. Reseeding takes the
    /// next seed no chamber is using, so replicates cannot collide.
    pub fn duplicate_active(&mut self, reseed: bool) -> usize {
        let mut spec = self.spec.clone();
        if reseed {
            spec.seed = self.next_unused_seed();
        }
        self.add_chamber(spec)
    }

    /// One past the highest seed anywhere in the rack.
    ///
    /// Highest-plus-one rather than count-plus-one: chambers get closed, and a
    /// counter that reuses a freed number hands two replicates the same world.
    fn next_unused_seed(&self) -> u64 {
        let highest = (0..self.rack.len())
            .map(|i| match &self.rack[i] {
                Some(ch) => ch.spec.seed,
                None => self.spec.seed,
            })
            .max()
            .unwrap_or(0);
        highest.wrapping_add(1)
    }

    /// Start a rack of copies of the box on screen, running headless.
    ///
    /// Refuses while one is already running, for [`Lab::batch`]'s reason.
    /// Returns what the interface should say about it either way — a verb
    /// that silently does nothing is `CLAUDE.md`'s second law being broken.
    pub fn start_batch(&mut self) -> String {
        if self.batch.is_some() {
            return "A RACK IS ALREADY RUNNING -- STOP IT FIRST".to_string();
        }
        // The base is always the box on screen: "copies of *this*" is the
        // whole verb, and a batch of some remembered other chamber would be a
        // button that does something different from what it says.
        self.batch_spec.base = self.spec.clone();
        // Start past every seed the rack already holds, so a second batch
        // explores new worlds rather than re-running the ones on the bench.
        self.batch_spec.seed0 = self.next_unused_seed();
        let spec = self.batch_spec.clone();
        let runs = spec.runs();
        let n = runs.len();
        let frames = spec.frames;
        // **Copies of the box as it stands, not of its recipe.** The recipe
        // says `founders: 0` on the shipped opening, so a recipe-built copy of
        // a bed you planted by hand comes out empty -- which is exactly what
        // the owner hit. The world carries the plants, the ants, their
        // positions and their genomes; the seed change reaches only what
        // happens next.
        let template = batch::Start::Copy(Box::new(self.world.clone()));
        self.batch = Some(batch::Batch::start_runs_from(runs, frames, spec.keep_bytes, template));
        let alive = self.world.live_organism_count();
        format!("RUNNING {n} COPIES OF THIS BOX ({alive} ALIVE) FOR {frames} TICKS EACH")
    }

    /// **Re-run an on-record row back into a world you can walk into.**
    ///
    /// The record kept its spec, and a spec plus a tick count reproduces its
    /// run **exactly** — that is `tests/determinism.rs`'s lab bed being spent
    /// rather than a hope, and it is the whole reason dropping the world for
    /// the memory budget is affordable instead of lossy.
    ///
    /// It runs on the same background threads the batch uses, so the box on
    /// screen keeps working. Refused while a rack is running, for
    /// [`Lab::batch`]'s reason — and said out loud rather than ignored.
    pub fn rebuild_record(&mut self, i: usize) -> String {
        // The rack page numbers on-record rows after the chambers.
        let Some(k) = i.checked_sub(self.rack.len()) else {
            return "THAT ROW IS ALREADY A CHAMBER".to_string();
        };
        let Some(rec) = self.on_record.get(k) else {
            return "NO SUCH ROW".to_string();
        };
        if rec.rebuilding {
            return "ALREADY REBUILDING".to_string();
        }
        if self.batch.is_some() {
            return "A RACK IS RUNNING -- STOP IT FIRST".to_string();
        }
        let ticks = rec.ticks();
        let run = batch::PlannedRun {
            index: 0,
            setting_index: 0,
            setting: None,
            replicate: 0,
            spec: rec.spec.clone(),
        };
        self.on_record[k].rebuilding = true;
        // `u64::MAX` — the whole point of a rebuild is to get the world back,
        // so it must not be dropped by the same budget that dropped it once.
        // Recipe-built on purpose, unlike `start_batch`: this reproduces a
        // run that was *itself* recipe-built, and its spec plus its seed is
        // exactly what makes that reproduction exact.
        self.batch = Some(batch::Batch::start_runs(vec![run], ticks, u64::MAX));
        format!("REBUILDING {ticks} TICKS -- THE BOX ON SCREEN KEEPS RUNNING")
    }

    /// Ask a running rack to stop. Runs already finished keep their results.
    pub fn stop_batch(&mut self) -> String {
        match &self.batch {
            Some(b) => {
                b.cancel();
                "STOPPING -- FINISHED COPIES ARE KEPT".to_string()
            }
            None => "NO RACK IS RUNNING".to_string(),
        }
    }

    /// Adopt every run that has landed since the last call, and reap the
    /// worker once it is done.
    ///
    /// **Called from `advance`, and it never blocks.** `main.rs`'s
    /// `poll_loading` rule: a `join` in the frame loop freezes the window for
    /// the rest of the batch, which is the whole thing the background thread
    /// exists to avoid.
    fn poll_batch(&mut self) {
        // Drained into a local first, so the borrow of `self.batch` ends
        // before `adopt_chamber` needs `&mut self`.
        let Some(landed) = self.batch.as_ref().map(|b| b.drain()) else { return };
        for r in landed {
            let label = match r.setting {
                Some(v) => format!("BATCH {} @ {v:.0}", r.index + 1),
                None => format!("BATCH {}", r.index + 1),
            };
            match r.world {
                // Held: it becomes a chamber you can walk into now.
                Some(world) => {
                    // A landed run retires the on-record row it was rebuilt
                    // from, matched on the seed. Without this the rack shows
                    // the same run twice — once as a chamber and once as the
                    // record it came from — which is a rack whose count is
                    // wrong in the direction that looks like more work got
                    // done than did.
                    let seed = r.spec.seed;
                    self.on_record.retain(|rec| !(rec.rebuilding && rec.spec.seed == seed));
                    self.adopt_chamber(world, r.spec, r.census, r.history, label);
                }
                // On record only: the census is kept and the world is
                // rebuilt from the spec on demand, which is exact.
                None => self.on_record.push(OnRecord { spec: r.spec, census: r.census, label, rebuilding: false }),
            }
        }
        let Some(b) = &self.batch else { return };
        if b.is_finished() {
            let p = b.progress();
            let mut b = self.batch.take().expect("checked above");
            let ok = b.join();
            let note = if !ok {
                "THE RACK'S OWN THREAD FAILED".to_string()
            } else if p.failed > 0 {
                format!("RACK DONE -- {} FINISHED, {} FAILED TO BUILD", p.finished, p.failed)
            } else if p.cancelled {
                format!("RACK STOPPED -- {} COPIES KEPT", p.finished)
            } else {
                format!("RACK DONE -- {} COPIES", p.finished)
            };
            // Whatever happened, nothing is still rebuilding: the worker has
            // stopped. A row left saying REBUILDING after a cancel or a panic
            // is a button that can never be pressed again.
            for rec in &mut self.on_record {
                rec.rebuilding = false;
            }
            self.ui.say(note);
        }
    }

    /// Put an already-built world into the rack.
    ///
    /// The batch's counterpart to [`Lab::add_chamber`]: the world exists
    /// already and must **not** be rebuilt, because rebuilding would discard
    /// the run that was just paid for.
    pub fn adopt_chamber(
        &mut self,
        world: World,
        spec: scene::LabBox,
        census: stats::Census,
        history: Vec<stats::Sample>,
        label: String,
    ) -> usize {
        self.rack.push(Some(Chamber {
            world,
            spec,
            stats: stats::Stats::restored(census, history),
            particles: ParticleSystem::new(),
            blasts: Blasts::new(),
            history: ui::History::default(),
            thumb: None,
            label: Some(label),
        }));
        self.rack.len() - 1
    }

    /// Take a still of chamber `i`, unless the one it already has is current.
    ///
    /// **Costs a full-size render plus a box downscale, and is therefore done
    /// on a click rather than on a frame.** It also leaves `Renderer`'s
    /// dirty-rect state describing a *different* world, so `view_dirty` is set
    /// — without that the next frame of the box on screen would be an
    /// incremental update against the picture of another chamber.
    ///
    /// A chamber whose picture matches its current frame keeps it: a frozen
    /// box cannot have changed, which is the common case on this page.
    fn take_thumb(&mut self, i: usize) {
        let frame_now = match self.chamber_frame(i) {
            Some(f) => f,
            None => return,
        };
        if self.thumb_at(i).is_some_and(|t| t.frame == frame_now) {
            return;
        }
        let mut full = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        {
            let world = match self.rack.get(i) {
                Some(Some(ch)) => &ch.world,
                _ if i == self.active => &self.world,
                _ => return,
            };
            let particles = match self.rack.get(i) {
                Some(Some(ch)) => &ch.particles,
                _ => &self.particles,
            };
            // Forced full: an incremental draw of a world the renderer has
            // never seen would leave most of the buffer black.
            self.renderer.draw(world, particles, &Default::default(), &mut full, (WIDTH, HEIGHT), true);
        }
        let (tw, th) = (WIDTH / THUMB_SHRINK, HEIGHT / THUMB_SHRINK);
        let mut rgba = vec![0u8; (tw * th * 4) as usize];
        // A box mean, not a nearest sample. At a quarter scale nearest
        // throws away 15 of every 16 cells, and a bed of one-cell-wide stems
        // reads as an empty box -- which is precisely the picture this page
        // exists to avoid showing.
        let n = THUMB_SHRINK * THUMB_SHRINK;
        for y in 0..th {
            for x in 0..tw {
                let mut acc = [0u32; 3];
                for dy in 0..THUMB_SHRINK {
                    for dx in 0..THUMB_SHRINK {
                        let sx = x * THUMB_SHRINK + dx;
                        let sy = y * THUMB_SHRINK + dy;
                        let si = ((sy * WIDTH + sx) * 4) as usize;
                        for c in 0..3 {
                            acc[c] += full[si + c] as u32;
                        }
                    }
                }
                let di = ((y * tw + x) * 4) as usize;
                for c in 0..3 {
                    rgba[di + c] = (acc[c] / n) as u8;
                }
                rgba[di + 3] = 255;
            }
        }
        let thumb = Thumb { w: tw, h: th, rgba, frame: frame_now };
        match self.rack.get_mut(i) {
            Some(Some(ch)) => ch.thumb = Some(thumb),
            _ => self.thumb = Some(thumb),
        }
        // The renderer now describes another world. See the doc above.
        self.view_dirty = true;
    }

    fn chamber_frame(&self, i: usize) -> Option<u64> {
        match self.rack.get(i)? {
            Some(ch) => Some(ch.world.frame),
            None => Some(self.world.frame),
        }
    }

    fn thumb_at(&self, i: usize) -> Option<&Thumb> {
        match self.rack.get(i)? {
            Some(ch) => ch.thumb.as_ref(),
            None => self.thumb.as_ref(),
        }
    }

    /// **Throw away every chamber and record except the one on screen.**
    ///
    /// A rack of fifty is made by one click, so it needs to be unmade by one
    /// click too — clearing it a row at a time is fifty clicks and a verb
    /// nobody uses. The box on screen survives for `remove_chamber`'s reason:
    /// clearing must never also move you somewhere you did not ask to go.
    pub fn clear_rack(&mut self) -> String {
        let chambers = self.rack.len() - 1;
        let records = self.on_record.len();
        if chambers == 0 && records == 0 {
            return "NOTHING TO CLEAR -- THIS IS THE ONLY BOX".to_string();
        }
        // Keep only the hole the inline fields live in, and rebase `active`
        // onto it: the rack invariant is that exactly one slot is `None`.
        self.rack.retain(|slot| slot.is_none());
        self.active = 0;
        self.on_record.clear();
        format!("CLEARED {chambers} CHAMBERS AND {records} RECORDS -- THIS BOX KEPT")
    }

    /// Discard one on-record row. Returns whether it went.
    ///
    /// Separate from [`Lab::remove_chamber`] because the two address
    /// different things behind one row number: chambers first, records after
    /// them. A record being deletable at all is the owner's ask — a run you
    /// have read and finished with is clutter, and ~10 KB of it is still a
    /// row you have to scroll past.
    pub fn remove_record(&mut self, i: usize) -> bool {
        let Some(k) = i.checked_sub(self.rack.len()) else { return false };
        if k >= self.on_record.len() || self.on_record[k].rebuilding {
            return false;
        }
        self.on_record.remove(k);
        true
    }

    /// Close chamber `i`. Returns whether it went.
    ///
    /// **Refuses to close the box on screen.** Honouring that would mean
    /// promoting some other chamber into the inline fields behind the
    /// player's back, and "close this" and "and now you are looking at a
    /// different experiment" are two things one click should not do. The
    /// caller switches away first, which also means the only chamber can
    /// never be closed — correct, since a facility with no box in it has
    /// nothing to draw.
    pub fn remove_chamber(&mut self, i: usize) -> bool {
        if i == self.active || i >= self.rack.len() {
            return false;
        }
        self.rack.remove(i);
        // Closing a tab renumbers the ones after it, this one included.
        if i < self.active {
            self.active -= 1;
        }
        true
    }

    /// One simulated tick — the shipped sequence, nothing skipped.
    ///
    /// Private on purpose: everything outside goes through `advance`, because
    /// how many of these run per displayed frame is `time`'s decision and
    /// splitting that across two callers is how a speed dial stops being
    /// honest about what it achieved.
    fn tick(&mut self) {
        frame::step(
            &mut self.world,
            &mut self.particles,
            &mut self.blasts,
            player::PlayerInput::default(),
            &self.player_tuning,
        );
    }

    /// Run whatever this displayed frame's share of simulated time is, and
    /// report what actually happened.
    ///
    /// `elapsed` is real time since the last call. Everything about how that
    /// becomes a tick count — the requested multiplier, the wall-clock
    /// ceiling that keeps the window answering, whether the box is paused
    /// or running — is `time::TimeControl`'s, so that the readout on screen
    /// and the loop that produced it cannot disagree.
    pub fn advance(&mut self, elapsed: std::time::Duration) -> time::Advance {
        let plan = self.time.plan(elapsed);
        let started = std::time::Instant::now();
        let mut ran = 0u32;
        while ran < plan.ticks {
            self.tick();
            ran += 1;
            if started.elapsed() >= plan.budget {
                break;
            }
        }
        let advance = self.time.record(ran, started.elapsed());
        // Never blocks; see `poll_batch`.
        self.poll_batch();
        if ran > 0 {
            // **The picture of a box that has moved is a wrong picture.** Held
            // under a live census it is the stale side table this repo keeps
            // paying for, and no picture is better than one that disagrees
            // with the numbers beside it. Re-taken on the next click.
            self.thumb = None;
        }
        self.stats.observe(&self.world);
        // Sampled here rather than in `draw` so that a frame which drew
        // nothing still advances the series -- and gated on `World::frame`
        // inside, so the x-axis is simulated time and not the speed dial.
        self.ui.observe(&self.world);
        advance
    }

    /// Draw the world, then whatever the lab is showing over it.
    ///
    /// `fps` is the *window's* rate, which only the binary knows; it is passed
    /// in rather than measured here so that the number on the box page is the
    /// same one the title bar shows.
    pub fn draw(&mut self, frame_buf: &mut [u8], fps: f32) {
        // Anything drawn over the terrain has no footprint tracked between
        // frames, so the dirty-rect skip cannot know to erase last frame's.
        // Same rule, and the same reasoning, as `App::draw`'s. The bar is
        // permanent chrome and its hover highlight moves with the cursor, so
        // `ui::Ui::is_dirty` is in this expression for the same reason
        // `time::hud_is_dirty` is.
        let force_full = self.ui.cursor().is_some()
            || self.ui.is_dirty()
            || self.stats.showing()
            || self.time.hud_is_dirty()
            || self.show_help
            // A chamber switch. See `Lab::view_dirty`.
            || std::mem::take(&mut self.view_dirty);
        let touched = self.world.take_touched_chunks();
        self.renderer.draw(
            &self.world,
            &self.particles,
            &touched,
            frame_buf,
            (WIDTH, HEIGHT),
            force_full,
        );
        self.time.draw(frame_buf, &self.world);
        // The species chip's face and its explanation, read out of the world's
        // own table rather than written down here — a chip that named a
        // species while explaining a different one is the stale side table
        // `ui::Spec::note` was made owned to avoid.
        let (species, species_note) = self.selected_species();
        // The jar chip's face and its explanation, for the species chip's
        // reason and by the same route: read out of the loaded rack rather
        // than remembered here.
        let (jar, jar_note) = (self.ui.jar_face(), self.ui.jar_chip_note());
        // Built before the state that borrows it. One row per chamber, and
        // cheap: everything on a row is already computed — the census is the
        // one `stats` last took, never a fresh walk of a frozen box.
        let chambers = self.chamber_summaries();
        // Reached through the fields rather than through `thumb_at`, so the
        // borrow is of `rack`/`thumb` and not of the whole `Lab` — `ui.draw`
        // below needs `&mut self.ui`, and a method call here would hold all of
        // `self` for as long as `state` lives.
        let rack_thumb = self.ui.selected_chamber().and_then(|i| match self.rack.get(i) {
            Some(Some(ch)) => ch.thumb.as_ref(),
            Some(None) => self.thumb.as_ref(),
            None => None,
        });
        let state = ui::BarState {
            running: self.time.phase == time::Phase::Running,
            requested: self.time.requested,
            achieved: self.time.achieved,
            presets: &time::PRESETS,
            panel: self.ui.panel,
            stats: self.stats.showing(),
            help: self.show_help,
            tool: self.ui.tool(),
            species: &species,
            species_note: &species_note,
            brush: self.ui.brush(),
            overlay: self.renderer.field_overlay.label(),
            jar: &jar,
            jar_note: &jar_note,
            chambers: &chambers,
            rack_thumb,
            batch: BatchBar {
                copies: self.batch_spec.replicates,
                frames: self.batch_spec.frames,
                progress: self.batch.as_ref().map(|b| b.progress()),
            },
        };
        self.ui.draw(frame_buf, &self.world, &self.spec, &state, &self.renderer, fps);
        // The pages last, because they are modal: a page covers the box *and*
        // the bar, and a control you can see but not reach is worse than one
        // that is plainly behind a page.
        //
        // **One page at a time.** The key list and the biosphere page are both
        // full-height overlays, and drawn together they interleave into
        // something neither of them is -- caught by looking at a capture of
        // the real window, not by any test, since both draw exactly what they
        // were asked to. The key list wins because it is transient: it is up
        // on a fresh lab and gone on the next keypress, where the page is
        // where you live.
        //
        // `draw_at` rather than `draw`: the hover explanation is the half of
        // the page that makes it readable cold, so the page needs the cursor.
        // `ui::Ui` owns it now -- the bar hit-tests the same position on the
        // same frame -- so it is read back from there rather than passed down
        // a second path. One owner, because two positions that disagree by a
        // frame is exactly the class of bug the retained bar exists to avoid.
        if self.show_help {
            draw_help(frame_buf);
        } else {
            self.stats.draw_at(frame_buf, &self.world, self.ui.cursor());
        }
    }

    /// Where the mouse is, in framebuffer pixels — `None` when it has left the
    /// window. Drives the hover highlight and the hover explanations.
    pub fn set_cursor(&mut self, at: Option<(i32, i32)>) {
        self.ui.set_cursor(at);
        if at.is_none() {
            self.stroke = None;
        }
    }

    /// The left button went down at `(x, y)`.
    ///
    /// **A brush starts painting here, not on release.** Every other control
    /// on the bar fires on release so a press can be taken back; a brush
    /// cannot work that way, because what you are judging is the stroke you
    /// are laying down while you hold the button. The bar's own press
    /// semantics are untouched — this only fires when the press landed on the
    /// world and a painting tool is armed.
    pub fn press(&mut self, x: i32, y: i32) {
        self.ui.press(x, y);
        if self.show_help || self.ui.covers(x, y) || !self.ui.tool().is_brush() {
            return;
        }
        let at = self.renderer.screen_to_world(x, y);
        self.begin_stroke(at, false);
    }

    /// The right button went down at `(x, y)` — the eraser, whatever tool is
    /// armed. The sandbox's rule, and it is the one the owner asked for:
    /// *"left-click paints, right-click erases"*. Having erase on its own
    /// button rather than as a seventh tool is what makes the pair a single
    /// gesture instead of two modes.
    pub fn press_erase(&mut self, x: i32, y: i32) {
        if self.show_help || self.ui.covers(x, y) {
            return;
        }
        let at = self.renderer.screen_to_world(x, y);
        self.begin_stroke(at, true);
    }

    /// A button came up, or the pointer left. Ends whatever stroke was live.
    pub fn end_stroke(&mut self) {
        self.stroke = None;
    }

    /// The pointer moved to `(x, y)` while a button is held.
    ///
    /// Continues the live stroke as a **capsule** from the last cell to this
    /// one rather than a dab at each, so a fast drag leaves one continuous
    /// band instead of a row of blobs — `App::paint_stroke`'s reason, and the
    /// same call underneath.
    pub fn drag(&mut self, x: i32, y: i32) {
        let Some(stroke) = self.stroke else { return };
        let to = self.renderer.screen_to_world(x, y);
        if to == stroke.last {
            return;
        }
        self.paint_span(stroke.last, to, stroke.erase);
        self.stroke = Some(Stroke { last: to, ..stroke });
    }

    fn begin_stroke(&mut self, at: (i32, i32), erase: bool) {
        self.paint_span(at, at, erase);
        self.stroke = Some(Stroke { last: at, erase });
    }

    /// Lay down (or lift) one span of the brush.
    ///
    /// **Two materials, and the two `aux` conventions point opposite ways.**
    /// `CLAUDE.md`'s standing gotcha: on a `Liquid` `aux == 0` means *full*,
    /// on a `Powder` it means *dry*. So water is painted at `aux == 0` — which
    /// `update::liquid_fill` reads back as `LIQUID_FULL` — and soil is painted
    /// at `SOIL_FIELD_CAPACITY`, which is damp enough for a root and short of
    /// the saturation that makes it slump. Painting soil at the `Powder`
    /// default of 0 would lay down bone-dry ground nothing can grow in;
    /// painting water at `LIQUID_FULL` would be a cell holding *twice* what it
    /// should, which is how water gets manufactured out of nothing.
    fn paint_span(&mut self, from: (i32, i32), to: (i32, i32), erase: bool) {
        use crate::sim::material;
        let radius = self.ui.brush();
        let (id, aux) = if erase {
            (material::EMPTY, 0)
        } else {
            match self.ui.tool() {
                ui::Tool::Water => match self.world.materials.id_of("water") {
                    Some(id) => (id, 0),
                    None => return,
                },
                // **`aux` 0, and here that is not a convention call.**
                // `windfall` does not set `worth_in_aux`, so
                // `creature::food_value` reads its `food_energy` (960) off the
                // material whatever the cell carries — there is no stamp to
                // write and no second pass to make. Painting it at anything
                // else would be writing a number nothing reads.
                //
                // **An explicit arm rather than the catch-all below**, which
                // is the whole reason this is a `match` and not an `if`: the
                // `_` arm paints soil, so a new brush that forgot to declare
                // itself would silently lay down ground instead of food and
                // look, on screen, like a tool that simply missed.
                ui::Tool::Food => match self.world.materials.id_of("windfall") {
                    Some(id) => (id, 0),
                    None => return,
                },
                _ => match self.world.materials.id_of("soil") {
                    Some(id) => (id, material::SOIL_FIELD_CAPACITY),
                    None => return,
                },
            }
        };
        self.world.paint_capsule_as(from, to, radius, id, 1.0);
        // `paint_capsule_as` writes the cell with a palette shade and no
        // `aux`, so the moisture is a second pass over the same disc. Only
        // over cells holding the material at `aux == 0`, so a wide stroke
        // cannot re-wet ground it never touched.
        if aux != 0 {
            let r = radius.max(0);
            for y in (from.1.min(to.1) - r)..=(from.1.max(to.1) + r) {
                for x in (from.0.min(to.0) - r)..=(from.0.max(to.0) + r) {
                    let cell = self.world.get(x, y);
                    if cell.material == id && cell.aux() == 0 {
                        self.world.set(x, y, cell.with_aux(aux));
                    }
                }
            }
        }
    }

    /// **Do the armed verb at a world cell.** Gate 4's three, plus `LOOK`.
    fn use_tool(&mut self, at: (i32, i32)) {
        let (x, y) = at;
        match self.ui.tool() {
            ui::Tool::Look => self.ui.inspect(at),
            ui::Tool::Plant => self.plant_at(x, y),
            ui::Tool::Colony => {
                let (species, ants) = (self.spec.colony_species.clone(), self.spec.colony_ants);
                let placed = self.world.found_colony_of(x, y, &species, ants);
                // The count, not just the picture: an ant is two dark cells at
                // play zoom, and a colony that placed nothing looks exactly
                // like one you have not found yet.
                self.ui.say(match placed {
                    0 => "NO ROOM FOR A COLONY HERE".to_string(),
                    n => format!("COLONY OF {n} RELEASED"),
                });
            }
            ui::Tool::Cull => self.cull_at(x, y),
            ui::Tool::Release => self.release_at(x, y),
            ui::Tool::Wall => self.wall_at(x),
            // The brushes never arrive here: they paint from `press`, so a
            // release that also painted would double the last dab.
            ui::Tool::Soil | ui::Tool::Water | ui::Tool::Food => {}
        }
    }

    /// Drop a wall in column `x`, or take out the one already there.
    ///
    /// **A toggle rather than two tools**, because the undo for "I put that
    /// in the wrong place" has to be the same gesture as the mistake — a
    /// separate remove-wall tool is a second thing to find at the moment you
    /// already feel stupid.
    ///
    /// The wall goes into the live world **and** into the spec, so it is
    /// there now *and* still there after a rebuild. Every other bed knob
    /// takes effect only on rebuild; a wall that behaved that way would be a
    /// verb whose effect you can only see by restarting, which is
    /// `CLAUDE.md`'s second law being broken.
    fn wall_at(&mut self, x: i32) {
        // A little slack, because a one-cell column is not something a mouse
        // hits exactly and the near-miss should remove the wall you were
        // aiming at rather than build a second one beside it.
        const REACH: i32 = 3;
        if let Some(w) = self.spec.wall_near(x, REACH) {
            self.spec.extra_walls.retain(|c| *c != w);
            let spec = self.spec.clone();
            spec.clear_wall(&mut self.world, w);
            self.ui.say(format!("WALL AT {w} REMOVED"));
            return;
        }
        let computed = self.spec.compartments > 1 && self.spec.partition_columns().contains(&x);
        if computed {
            // A computed partition belongs to `compartments`, and pulling one
            // out from under that number would leave the two disagreeing.
            self.ui.say("THAT WALL COMES FROM THE COMPARTMENTS SETTING -- CHANGE IT ON THE PARAMS PAGE".to_string());
            return;
        }
        let spec = self.spec.clone();
        spec.paint_wall(&mut self.world, x);
        self.spec.extra_walls.push(x);
        let n = self.spec.partition_columns().len();
        self.ui.say(format!("WALL AT {x} -- {} COMPARTMENTS", n + 1));
    }

    /// Put one seed of the selected species in at `(x, y)`.
    ///
    /// **The click lands where you aimed; the seed lands where a seed can
    /// go.** `plant_tree_species` refuses an occupied cell, and a player
    /// aiming at soil is aiming at the ground rather than at the air one cell
    /// above it — so a click that lands *in* the ground walks up to the first
    /// empty cell above it. A seed is a `Powder` and falls the rest of the way
    /// on its own, so a click in mid-air is honest too.
    fn plant_at(&mut self, x: i32, y: i32) {
        let (name, _) = self.selected_species();
        let mut site = y;
        for _ in 0..MAX_PLANT_LIFT {
            if self.world.is_empty(x, site) {
                break;
            }
            site -= 1;
        }
        let lower = name.to_lowercase();
        if self.world.plant_tree_species(x, site, &lower) {
            self.ui.say(format!("PLANTED {name} AT {x},{site}"));
        } else {
            self.ui.say(format!("NO ROOM TO PLANT {name} HERE"));
        }
    }

    /// **Kill the organism under `(x, y)`, the way the engine already kills
    /// organisms.**
    ///
    /// Two paths, because the two kingdoms die differently and there is no
    /// shared one. A plant is marked **senescent**, which `plant::rot_remains`
    /// then carries out at the species' own `remains_half_life` — the owner's
    /// own ruling, and `CLAUDE.md`'s first law: the death is *graded* rather
    /// than a disappearance. A creature has no senescence path at all (nothing
    /// outside `plant.rs` reads the flag), so its energy is taken to zero and
    /// `creature::apply_creature_energy` writes a **corpse** on its next tick
    /// — a `Powder` that falls, rots, burns and can be eaten by whatever is
    /// still alive.
    ///
    /// Neither leaves a hole. That is the point: an outcome is a distribution,
    /// and a cull that deleted the cells would be the binary this project
    /// keeps ruling against.
    fn cull_at(&mut self, x: i32, y: i32) {
        let cell = self.world.get(x, y);
        let id = cell.organism_id();
        let Some(state) = self.world.organism_state(id) else {
            self.ui.say("NOTHING ALIVE HERE");
            return;
        };
        let species = self.world.species.get(state.species);
        let name = species.name.to_uppercase();
        let animal = species.creature.is_some();
        let cells = state.cells.len();
        if animal {
            if let Some(state) = self.world.organism_mut(id) {
                state.energy = 0.0;
            }
            self.ui.say(format!("CULLED {name} - DIES ON THE NEXT TICK"));
        } else {
            self.world.mark_organism_senescent(id);
            self.ui.say(format!("CULLED {name} - {cells} CELLS LEFT TO ROT"));
        }
    }

    /// The selected plantable species: its name in the bar's uppercase, and
    /// the line the chip's hover explanation shows.
    fn selected_species(&self) -> (String, String) {
        let Some(id) = self.ui.species_of(&self.world) else {
            return ("NONE".to_string(), "NO PLANTABLE SPECIES IS LOADED.".to_string());
        };
        let species = self.world.species.get(id);
        let name = species.name.to_uppercase();
        // Two real numbers off the species itself, not a description. The
        // design guide is explicit that planting must show what you are about
        // to plant *"or planting is a slot machine"*; a name alone is the
        // minimum, and these are the two that decide how it plays — how long a
        // seed keeps if it does not germinate, and how long its remains take
        // to go once it dies.
        let note = format!(
            "WHICH PLANT THE PLANTING TOOL PUTS IN. CLICK TO CYCLE. {name}: A SEED KEEPS {:.0} FRAMES BEFORE IT IS GONE, AND ITS REMAINS TAKE {:.0} FRAMES TO ROT DOWN ONCE IT DIES.",
            species.seed_half_life, species.remains_half_life
        );
        (name, note)
    }

    /// The left button came up at `(x, y)`. This is where a click turns into
    /// something happening.
    ///
    /// **Every verb here is one a key already had, plus one the keyboard could
    /// not offer at all** — pointing at a cell. That asymmetry is the whole
    /// argument for the mouse: a key can toggle a page, and nothing on the
    /// keyboard can say *that ant*.
    pub fn release(&mut self, x: i32, y: i32) {
        if self.stroke.take().is_some() {
            // The brush already did its work on press and on every drag; a
            // release that also fired the verb would double the last dab.
            self.ui.cancel_press();
            return;
        }
        match self.ui.release(x, y) {
            ui::Release::Fired(action) => self.act(action),
            ui::Release::Consumed => {}
            ui::Release::World => {
                // The key page is modal, so a click on the world dismisses it
                // rather than reaching through it — the same rule as any key.
                if self.show_help {
                    self.show_help = false;
                    return;
                }
                let at = self.renderer.screen_to_world(x, y);
                self.use_tool(at);
            }
        }
    }

    /// Do one of the bar's verbs.
    ///
    /// **The single place a control turns into a change**, so a button and its
    /// keyboard shortcut cannot drift: `bin/lab.rs`'s key handler routes
    /// through here too, and there is no second copy of "what SPACE does".
    pub fn act(&mut self, action: ui::Action) {
        match action {
            ui::Action::TogglePhase => self.time.toggle_phase(),
            ui::Action::Slower => self.time.slower(),
            ui::Action::Faster => self.time.faster(),
            ui::Action::Preset(i) => self.time.set_preset(i),
            // **One page at a time**, the rule this file already applies to
            // the key list against the biosphere page, extended to the bar's
            // three. The screen is 512x320, the biosphere page is a full-
            // height right-hand column and these open above the bar's own
            // right-hand group, so two of them drawn together interleave into
            // something neither of them is. Made exclusive in *state* rather
            // than only in the paint, so the latch on the bar tells the truth
            // about what is on screen.
            ui::Action::Panel(panel) => {
                self.ui.toggle_panel(panel);
                if self.ui.panel.is_some() && self.stats.showing() {
                    self.stats.toggle();
                }
            }
            ui::Action::Stats => {
                self.stats.toggle();
                if self.stats.showing() {
                    self.ui.panel = None;
                }
            }
            ui::Action::Tool(tool) => {
                self.ui.set_tool(tool);
                self.ui.say(format!("TOOL {}", self.ui.tool().label()));
            }
            ui::Action::NextSpecies => {
                self.ui.next_species();
                let (name, _) = self.selected_species();
                // ...and picking a species arms the tool that uses it. A chip
                // that changes what a *different* button will do, silently, is
                // the mode you forget you are in.
                //
                // **`arm_tool`, never `set_tool`** -- see its doc. `set_tool`
                // toggles, so cycling the species while `PLANT` was already
                // armed put the tool away, which is the owner's reported
                // *"suddenly plant is unselected and now the mouse is on
                // look"*.
                self.ui.arm_tool(ui::Tool::Plant);
                self.ui.say(format!("PLANTING {name}"));
            }
            ui::Action::Brush(delta) => {
                self.ui.adjust_brush(delta);
                self.ui.say(format!("BRUSH R{}", self.ui.brush()));
            }
            ui::Action::CycleOverlay => {
                self.renderer.cycle_field_overlay();
                self.ui.say(format!("OVERLAY {}", self.renderer.field_overlay.label()));
            }
            ui::Action::Help => self.show_help = !self.show_help,
            ui::Action::Reset => self.reset(),
            ui::Action::ParamGroup(i) => self.ui.set_param_group(i),
            ui::Action::ParamScroll(d) => self.ui.scroll_params(d),
            ui::Action::ParamSelect(i) => self.ui.select_param(i),
            ui::Action::ParamAdjust(i, sign) => self.adjust_param(i, sign),
            ui::Action::ParamSave => self.save_param(),
            // **The two verbs the bar gave up, now on the pages that already
            // know what they mean.** Owner, 2026-08-31: *"I feel like we don't
            // need the keep and free buttons... this will save some menu
            // space."* Neither is a new mechanism -- `keep_at` and
            // `release_at` are untouched -- what changed is where the thing
            // they act on is chosen.
            ui::Action::KeepInspected => match self.ui.inspecting() {
                // The cell page's own coordinate, so what is kept is what the
                // page is showing. It is re-read every frame, which is the
                // point: an ant that walked out from under the marker while
                // you were reading is not the ant on the page any more, and
                // keeping the cell rather than a snapshot is the honest
                // reading of "this one".
                Some((x, y)) => self.keep_at(x, y),
                None => self.ui.say("OPEN THE CELL PAGE ON SOMETHING ALIVE FIRST"),
            },
            ui::Action::ShelfPlace => {
                if self.ui.armed_jar().is_none() {
                    self.ui.say("NO JAR ARMED -- CLICK ONE IN THE RACK FIRST");
                } else {
                    // The rack is a page over the box, so leaving it open
                    // would arm a click at the thing covering what the click
                    // is for.
                    self.ui.close_panel();
                    self.ui.arm_tool(ui::Tool::Release);
                    let name = self.ui.armed_jar().map_or_else(String::new, |j| j.name.to_uppercase());
                    let dial = self.ui.brood_label();
                    self.ui.say(format!("CLICK IN THE BOX TO PLACE {name} -- {dial}"));
                }
            }
            ui::Action::ShelfSelect(i) => {
                self.ui.select_jar(i);
                // ...and picking a jar arms the tool that uses it, which is
                // exactly what `NextSpecies` does for the species chip. A
                // chip that changes what a *different* button will do,
                // silently, is the mode you forget you are in. `arm_tool` for
                // `NextSpecies`' reason: picking a second jar is not a request
                // to stop placing.
                self.ui.arm_tool(ui::Tool::Release);
                match self.ui.armed_jar() {
                    Some(jar) => {
                        let (name, dial) = (jar.name.to_uppercase(), self.ui.brood_label());
                        self.ui.say(format!("ARMED {name} -- {dial}"));
                    }
                    None => self.ui.say("THAT JAR IS NO LONGER ON THE SHELF"),
                }
            }
            ui::Action::Broods(delta) => {
                self.ui.adjust_broods(delta);
                let dial = self.ui.brood_label();
                self.ui.say(match self.ui.broods() {
                    0 => "DRIFT CLONE -- A RELEASE IS THAT EXACT INDIVIDUAL".to_string(),
                    1 => "DRIFT 1 BROOD -- AS DIFFERENT AS ITS OWN CHILD".to_string(),
                    _ => format!("DRIFT {dial}"),
                });
            }
            ui::Action::ShelfDrift => self.drift_jar(),
            ui::Action::ShelfDiscard => self.discard_jar(),
            ui::Action::ShelfPromote => self.promote_jar(),
            ui::Action::ShelfReload => {
                self.ui.reload_shelf();
                let n = self.ui.shelf().len();
                self.ui.say(format!("SHELF RELOADED -- {n} JAR(S)"));
            }
            ui::Action::BatchRun => {
                let said = self.start_batch();
                self.ui.say(said);
            }
            ui::Action::BatchStop => {
                let said = self.stop_batch();
                self.ui.say(said);
            }
            ui::Action::BatchCopies(d) => {
                // 1..=200. The floor is 1 rather than 0 because a rack of
                // nothing is a button that reports success and does nothing;
                // the ceiling is where ~2.5 MB a chamber meets the 256 MB
                // the budget holds.
                let next = (self.batch_spec.replicates as i32 + d).clamp(1, 200);
                self.batch_spec.replicates = next as u32;
            }
            ui::Action::BatchFrames(d) => {
                // Steps of 1,000, because the interesting range is 1,800 (the
                // first inherited plant) to ~45,000 (the fifth generation) and
                // stepping that in ones is not a control anybody would use.
                let next = (self.batch_spec.frames as i64 + d as i64 * 1_000).clamp(1_000, 200_000);
                self.batch_spec.frames = next as u64;
            }
            ui::Action::ChamberSelect(i) => {
                self.ui.select_chamber(i);
                // The picture is what a click on a row is *for*, so it is
                // taken here rather than lazily in `draw` — a page that shows
                // the row highlighted and the picture one frame later reads as
                // a stutter on every click.
                self.take_thumb(i);
            }
            ui::Action::ChamberAdd => {
                let i = self.duplicate_active(true);
                let seed = self.chamber_summaries()[i].seed;
                self.ui.select_chamber(i);
                self.ui.say(format!("CHAMBER {} ADDED -- SEED {seed}", i + 1));
            }
            ui::Action::ChamberRebuild(i) => {
                let said = self.rebuild_record(i);
                self.ui.say(said);
            }
            ui::Action::ChamberSort(c) => self.ui.sort_chambers(c),
            ui::Action::ChamberClear => {
                let said = self.clear_rack();
                self.ui.say(said);
            }
            ui::Action::ChamberClose(i) => {
                if self.remove_record(i) {
                    self.ui.say("RECORD DISCARDED".to_string());
                } else if self.remove_chamber(i) {
                    self.ui.say(format!("CHAMBER {} CLOSED", i + 1));
                } else {
                    // Says why rather than doing nothing. `CLAUDE.md`'s second
                    // law: a verb with no visible consequence is not finished.
                    self.ui.say("CANNOT CLOSE THE BOX YOU ARE IN -- ENTER ANOTHER FIRST".to_string());
                }
            }
            ui::Action::Chamber(i) => {
                if i == self.active {
                    // Say so rather than doing nothing. `CLAUDE.md`'s second
                    // law: a verb that produces no visible consequence is not
                    // finished, and clicking the tab you are already on is the
                    // commonest way to find that out.
                    self.ui.say(format!("ALREADY IN CHAMBER {}", i + 1));
                } else {
                    self.switch_to(i);
                    let frame = self.world.frame;
                    self.ui.say(format!("CHAMBER {} -- HELD AT FRAME {frame}", i + 1));
                }
            }
        }
    }

    // ------------------------------------------------------------ the shelf
    //
    // **Four verbs and one law: nothing here destroys a specimen except
    // `DISCARD`.** Keeping never overwrites (`specimen::save_to` refuses and
    // `next_free_name` picks the next stem), drifting writes a new jar and
    // leaves its parent standing, and releasing does not consume the jar. A
    // kept specimen is the one thing in this lab a player cannot regenerate
    // -- the box moves on and the individual dies -- so the only way to lose
    // one is to say so.

    /// **Put the genetics of whatever is under `(x, y)` in a jar.**
    ///
    /// Named after its species, numbered up if that name is taken, so the
    /// gesture is one click and the naming is not a dialogue box in a game
    /// that has none.
    fn keep_at(&mut self, x: i32, y: i32) {
        use crate::sim::specimen;
        let Some(id) = specimen::organism_at(&self.world, x, y) else {
            self.ui.say("NOTHING ALIVE HERE TO KEEP");
            return;
        };
        // The species name is the default stem, so a rack reads as
        // `herb`, `herb_2`, `ant`, `ant_2` -- which is what the player will
        // be looking for when they come back to it.
        let stem = self
            .world
            .organism_state(id)
            .map(|st| specimen::sanitise(&self.world.species.get(st.species).name))
            .unwrap_or_else(|| "jar".to_string());
        let name = specimen::next_free_name(specimen::shelf_dir(), &stem);
        let spec = match specimen::capture(&self.world, id, &name) {
            Ok(s) => s,
            Err(e) => {
                self.ui.say(e.say());
                return;
            }
        };
        let kingdom = spec.genetics.kingdom();
        let generation = spec.taken.generation;
        match specimen::save(&spec) {
            Ok(_) => {
                self.ui.reload_shelf();
                // **The count and the generation, not just "kept".** A jar
                // is a file the player cannot see, and generation is the one
                // number that says whether this is a founder or something
                // the box actually bred.
                let n = self.ui.shelf().len();
                self.ui.say(format!("KEPT {} -- {kingdom} G{generation} -- {n} ON THE SHELF", name.to_uppercase()));
            }
            Err(e) => self.ui.say(e.say()),
        }
    }

    /// **Put the armed jar back in the box at `(x, y)`**, drifted by the
    /// dial.
    ///
    /// A creature arrives alive; a plant arrives as a seed that still has to
    /// fall and germinate, which is the same deal `PLANT` offers and is why
    /// the notice says which happened.
    fn release_at(&mut self, x: i32, y: i32) {
        use crate::sim::specimen;
        let Some(jar) = self.ui.armed_jar().cloned() else {
            self.ui.say("NO JAR ARMED -- OPEN THE SHELF (G) AND CLICK ONE");
            return;
        };
        let broods = self.ui.broods();
        // A seed is a falling powder and a body needs its cells free, so a
        // click that lands *in* the ground walks up to the first empty cell
        // above it -- `plant_at`'s rule, for `plant_at`'s reason.
        let mut site = y;
        for _ in 0..MAX_PLANT_LIFT {
            if self.world.is_empty(x, site) {
                break;
            }
            site -= 1;
        }
        // **Its own stream, keyed on the frame and the release point.** The
        // dial's draws must not come out of a generator the world goes on
        // using -- `brain::mutate` takes a variable number of them, so the
        // sweep's own draws would shift by an amount that depends on how
        // many slots happened to mutate.
        let mut rng = crate::sim::rng::stream(self.world.seed, x as u64, site as u64, self.world.frame);
        match specimen::release(&mut self.world, &jar, x, site, broods, &mut rng) {
            Ok(out) => {
                let what = jar.genetics.kingdom();
                let name = jar.name.to_uppercase();
                // **The slots the dial actually moved, beside the picture.**
                // A clone and a four-brood release are two dark cells either
                // way at play zoom, and only the number says which one just
                // happened.
                self.ui.say(match out.moved {
                    0 => format!("RELEASED {name} -- AN EXACT {what}"),
                    n => format!("RELEASED {name} -- {} BROODS, {n} GENOME SLOTS MOVED", broods),
                });
            }
            Err(e) => self.ui.say(e.say()),
        }
    }

    /// **Breed the armed jar on the shelf**, without releasing it.
    ///
    /// The original stays. This is the verb that makes the rack a working
    /// set rather than an archive: a player can carry a line forward,
    /// compare two drifts of it side by side, and release whichever they
    /// prefer -- and the new jar records which one it came from and how far.
    fn drift_jar(&mut self) {
        use crate::sim::specimen;
        let Some(jar) = self.ui.armed_jar().cloned() else {
            self.ui.say("NOTHING ARMED -- CLICK A JAR FIRST");
            return;
        };
        let broods = self.ui.broods();
        let name = specimen::next_free_name(specimen::shelf_dir(), &jar.name);
        let mut rng = crate::sim::rng::stream(self.world.seed, jar.name.len() as u64, broods as u64, self.world.frame);
        let drifted = match specimen::drift(&self.world, &jar, broods, &name, &mut rng) {
            Ok(d) => d,
            Err(e) => {
                self.ui.say(e.say());
                return;
            }
        };
        match specimen::save(&drifted.specimen) {
            Ok(_) => {
                self.ui.reload_shelf();
                self.ui.say(format!(
                    "{} DRIFTED {broods} BROODS INTO {} -- {} GENOME SLOTS MOVED",
                    jar.name.to_uppercase(),
                    name.to_uppercase(),
                    drifted.moved
                ));
            }
            Err(e) => self.ui.say(e.say()),
        }
    }

    /// Take the armed jar off the shelf for good.
    fn discard_jar(&mut self) {
        use crate::sim::specimen;
        let Some(name) = self.ui.armed_jar().map(|j| j.name.clone()) else {
            self.ui.say("NOTHING ARMED -- CLICK A JAR FIRST");
            return;
        };
        match specimen::discard(&name) {
            Ok(()) => {
                self.ui.reload_shelf();
                self.ui.say(format!("DISCARDED {}", name.to_uppercase()));
            }
            Err(e) => self.ui.say(e.say()),
        }
    }

    /// **Write the armed jar out as a whole species file** — the way out of
    /// the lab and into the game.
    ///
    /// A jar is small because it leans on its species for everything that is
    /// not heritable; a species file is self-contained and is what the
    /// game's own loader reads. `species_export` owns the format and the
    /// refusal to overwrite a hand-authored file; this is the button.
    ///
    /// **Creatures only, and it says so rather than failing quietly.** A
    /// plant's species file carries a growth program that `species_export`
    /// has never written, so promoting one would produce a file that parses
    /// and grows the wrong plant.
    fn promote_jar(&mut self) {
        use crate::sim::{species_export, specimen};
        let Some(jar) = self.ui.armed_jar().cloned() else {
            self.ui.say("NOTHING ARMED -- CLICK A JAR FIRST");
            return;
        };
        let specimen::Genetics::Creature(g) = &jar.genetics else {
            self.ui.say("PROMOTING A PLANT IS NOT BUILT YET -- CREATURES ONLY");
            return;
        };
        let Some(parent_id) = self.world.species.id_of(&jar.species) else {
            self.ui.say(specimen::ShelfError::NoSuchSpecies(jar.species.clone()).say());
            return;
        };
        let genome = crate::sim::brain::genome_from_wiring(&g.instincts, &g.hidden, &g.outputs, &g.recurrence);
        let mut traits = [0.0; crate::sim::organism::CREATURE_TRAITS];
        for (dst, src) in traits.iter_mut().zip(g.traits.iter()) {
            *dst = *src;
        }
        let parent = self.world.species.get(parent_id);
        let def = match species_export::individual_as_species(parent, &genome, traits, &jar.name) {
            Ok(d) => d,
            Err(e) => {
                self.ui.say(format!("{e}").to_uppercase());
                return;
            }
        };
        match species_export::save(&def) {
            Ok(path) => {
                // **And what is still missing**, said at the moment it
                // matters. A species with no material of the same name
                // resolves to no body and hatches nothing, and what a new
                // creature looks like is the one thing E8 is explicit must
                // not be generated.
                let needs_material = self.world.materials.id_of(&jar.name).is_none();
                let where_ = path.file_name().map(|f| f.to_string_lossy().to_uppercase()).unwrap_or_default();
                self.ui.say(if needs_material {
                    format!("PROMOTED TO {where_} -- IT STILL NEEDS A MATERIAL OF THE SAME NAME TO HATCH")
                } else {
                    format!("PROMOTED TO {where_}")
                });
            }
            Err(e) => self.ui.say(format!("{e}").to_uppercase()),
        }
    }

    /// **Move one parameter by one of its own steps, and say what it did.**
    ///
    /// The list is rebuilt here rather than carried from the paint, `ui::Ui::
    /// page_params`' tradeoff — and `i` is an index into the page as it was
    /// drawn, which is the page a click was aimed at.
    ///
    /// Every branch of `params::write` is a live store the tick reads through,
    /// so an ordinary change is felt on the next tick with no reload. The one
    /// exception is the bed's own spec, which is what a rebuild is made from;
    /// the notice says so rather than leaving the player to wonder why nothing
    /// moved. `CLAUDE.md`'s second law: an event with no visible consequence
    /// is not finished, and half of these knobs are invisible on a **stopped**
    /// box, which is exactly when you set them.
    fn adjust_param(&mut self, index: usize, sign: i32) {
        let list = self.ui.page_params(&self.world, &self.spec);
        let Some(param) = list.get(index) else { return };
        if !param.writable() {
            self.ui.say("THIS ONE IS SHOWN, NOT CHANGED");
            return;
        }
        let target = param.tunable.stepped(sign);
        // The name as the panel prints it, not as the file spells it: the
        // notice sits under the row it is about, and `BIRTH_GRANT` beside a
        // row reading `BIRTH GRANT` reads as a different thing.
        let name = param.tunable.name.replace('_', " ").to_uppercase();
        let rebuild = params::needs_rebuild(&param.knob);
        let knob = param.knob.clone();
        // Cloned out before the borrow ends, so the write can take the world
        // mutably. A `Knob` is a small enum over a name and a species string.
        if !params::write(&mut self.world, &mut self.spec, &knob, target) {
            // A registered row whose writer declined is the "reader with no
            // writer" failure `CLAUDE.md` names, and it looks exactly like
            // working code from the outside -- so it is said out loud rather
            // than swallowed. `params::tests::every_writable_parameter_
            // actually_moves` is what keeps it from ever being seen.
            self.ui.say(format!("{name} HAS NO WRITER -- NOTHING CHANGED"));
            return;
        }
        self.ui.select_param(index);
        // Re-read rather than echoing `target`: the store may have rounded it
        // (every integral field does), and a readout that disagrees with the
        // world is worse than no readout.
        let shown = self
            .ui
            .page_params(&self.world, &self.spec)
            .get(index)
            .map_or_else(|| format!("{target:.3}"), |p| p.display());
        self.ui.say(if rebuild {
            format!("{name} = {shown} - TAKES EFFECT ON REBUILD")
        } else {
            format!("{name} = {shown}")
        });
    }

    /// Write the highlighted parameter back to its asset file.
    ///
    /// The refusals are as much of the feature as the writes — see
    /// `params::save`, which declines an ambiguous or non-numeric field rather
    /// than editing the wrong one.
    fn save_param(&mut self) {
        let Some(index) = self.ui.selected_param() else {
            self.ui.say("NOTHING PICKED - MOVE A ROW OR CLICK ITS NAME FIRST");
            return;
        };
        let list = self.ui.page_params(&self.world, &self.spec);
        let Some(param) = list.get(index) else { return };
        let message = match params::save(param) {
            Ok(ok) => ok,
            Err(e) => e.to_uppercase(),
        };
        self.ui.say(message);
    }
}

/// **Draw nest material as worked earth in the lab, not as pale stone.**
///
/// Owner, 2026-08-30: *"we don't need a visible line for where the colony is
/// placed."* `creature::found_colony` paints one row of `nest` across 53
/// columns at the surface, and `nest.ron`'s palette is a pale warm sand
/// (196,168,120) chosen to *"read clearly against soil"* — which outdoors is
/// the point and in a flat bed is a stripe drawn across the box.
///
/// **The colour carries no behaviour.** `AtNest` is a contact scan for the
/// material and nothing else, and an ant's route home is a pheromone gradient
/// that never asks where the nest is — `nest.ron` says both, at length. So
/// what the fix costs is exactly the player's ability to spot a colony at a
/// glance, and what it buys is that a founded colony no longer draws a line.
///
/// Three things this is deliberately *not*:
///
/// - not an edit to `creature.rs`. The nest patch is functional and narrower
///   than the ant band on purpose — *"home has to be a place, not everywhere,
///   or there is no gradient to walk up"* — and the foraging scene measured
///   414 deliveries at that ratio;
/// - not an edit to `nest.ron`, which would change the sandbox too, where a
///   findable nest is wanted;
/// - not a test in `render.rs`'s per-pixel path. A material comparison per
///   pixel is 163,840 of them a frame for a stripe; swapping the palette in
///   the lab's own `Materials` costs **nothing at draw time at all**, which is
///   what `Materials::get_mut` exists for.
///
/// The tones are `packedsoil`'s own reference-loam family, which is the
/// engine's existing answer to "ground an animal has worked" — the same
/// material an ant lines its galleries with. Worked ground drawn as worked
/// ground.
fn earth_toned_nest(world: &mut World) {
    const WORKED_EARTH: [[u8; 4]; 4] = [
        [48, 38, 32, 255],
        [56, 45, 38, 255],
        [42, 33, 28, 255],
        [62, 50, 42, 255],
    ];
    let Some(nest) = world.materials.id_of("nest") else { return };
    let def = world.materials.get_mut(nest);
    def.palette = WORKED_EARTH.to_vec();
    def.base_shades = WORKED_EARTH.len();
}

/// How far a planting click may walk **up** out of the ground to find an
/// empty cell to drop a seed in. Twelve rows: a click aimed at the surface
/// lands in soil, and a click aimed a hand's width into the bed should still
/// plant rather than silently refusing. Past that the player is aiming at
/// something buried and a seed is not what they meant.
const MAX_PLANT_LIFT: i32 = 12;

/// The key list, drawn over a dimmed screen.
///
/// Uppercase and punctuation-light on purpose: `hud`'s font is a hand-authored
/// 5x7 bitmap with a deliberately small glyph set, and a character it does not
/// have draws as a silent blank rather than as anything you would notice. That
/// gap has shipped three times in this repo, so every line here is checked
/// against `hud::has_glyph` by `every_help_line_is_drawable`.
const HELP: [&str; 30] = [
    "THE EVOLUTION LAB",
    "",
    "THE BOX STARTS EMPTY. YOU STOCK IT.",
    "EVERY CONTROL IS ALSO A BUTTON ON",
    "THE BAR ALONG THE BOTTOM.",
    "",
    "SPACE      STOP / RUN THE BOX",
    "UP DOWN    SPEED     1-7  PRESET",
    "",
    "Z X C V B N  LOOK PLANT COLONY",
    "             CULL SOIL WATER",
    "M ,          KEEP THIS ONE / PLACE A JAR",
    "CLICK      USE THE ARMED TOOL",
    "RIGHT      ERASE",
    ".          WHICH SPECIES TO PLANT",
    "[ ]        BRUSH NARROWER WIDER",
    "O L        FIELD / LIFE OVERLAY",
    "",
    "P          PARAMETERS -- THE NUMBERS",
    "           BEHIND THE VERBS",
    "G          THE SHELF -- KEPT GENETICS.",
    "           KEEP AND PLACE ARE BUTTONS NOW,",
    "           ON THE CELL PAGE AND THE RACK",
    "; \x27        DRIFT A RELEASE, IN BROODS",
    "K E        WALL / FOOD -- NO BUTTON, KEY ONLY",
    "F1 F2 F3 F4   PLANTS ANTS BOX RACK   TAB STATS",
    "SHIFT+1..5   SWITCH CHAMBER    ALL   THE WHOLE RACK",
    "F RATE   WASD PAN   - = ZOOM   R REBUILD",
    "?          THIS PAGE",
    "",
];

fn draw_help(frame: &mut [u8]) {
    let w = HELP.iter().map(|l| crate::hud::text_width(l)).max().unwrap_or(0);
    let (bw, bh) = (w + 24, HELP.len() as i32 * 10 + 20);
    let (x0, y0) = ((WIDTH as i32 - bw) / 2, (HEIGHT as i32 - bh) / 2);
    for y in y0..(y0 + bh) {
        for x in x0..(x0 + bw) {
            if x < 0 || y < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
                continue;
            }
            let i = ((y as u32 * WIDTH + x as u32) * 4) as usize;
            // Dim what is behind rather than covering it, so the box stays
            // visible under the page and the page reads as an overlay.
            for c in 0..3 {
                frame[i + c] /= 4;
            }
        }
    }
    for (i, line) in HELP.iter().enumerate() {
        let colour = if i == 0 { [235, 235, 200, 255] } else { [200, 200, 200, 255] };
        crate::hud::draw_text(frame, WIDTH, HEIGHT, x0 + 12, y0 + 12 + i as i32 * 10, line, colour);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------- the rack

    /// A small box that germinates, for the rack guards. Deliberately not
    /// `LabBox::default()`: these tests run hundreds of frames and the
    /// shipped bed is four times the area for no extra signal. `colonies: 0`
    /// because a colony eats founders (Gate 1 §3: five of eight by frame
    /// 66,000) and every guard below is about the *rack*, not about grazing.
    fn rack_bed(seed: u64) -> scene::LabBox {
        scene::LabBox {
            width: 256,
            height: 192,
            // Scaled together. `lab_resolution` records what happens when
            // they are not: at the default `ground_y` under a short box the
            // soil sits in the top quarter and the rest is void — a scene
            // error wearing a result.
            ground_y: 96,
            soil_depth: 48,
            founders: 4,
            colonies: 0,
            seed,
            ..scene::LabBox::default()
        }
    }

    /// An order-sensitive digest of the whole grid, the same shape as
    /// `tests/determinism.rs`'s and `frame.rs`'s: what these guards have to
    /// catch moves cells, so a census of counts would miss it.
    fn grid_hash(w: &World) -> u64 {
        fn fnv1a(h: u64, v: u64) -> u64 {
            (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
        }
        let b = w.bounds().expect("the lab bed sets bounds");
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                let c = w.get(x, y);
                h = fnv1a(h, c.material.0 as u64);
                h = fnv1a(h, c.aux() as u64);
                h = fnv1a(h, c.organism_id() as u64);
            }
        }
        // Organism state as well as the grid: a genotype difference reaches
        // the cells only once it has grown differently, and these guards must
        // be able to see it before then.
        h = fnv1a(h, w.live_organism_count() as u64);
        let (born, died) = w.organism_turnover();
        h = fnv1a(h, born);
        h = fnv1a(h, died);
        fnv1a(h, w.germinations)
    }

    /// Run `n` ticks of the chamber on screen.
    fn run(lab: &mut Lab, n: u32) {
        for _ in 0..n {
            lab.tick();
        }
    }

    /// **A copy carries what you planted, and the copies still differ.**
    ///
    /// The owner's report, on the merged code: *"I added some plants and ants
    /// to my chamber, hit F4, tried to run copies of the same room, but all
    /// of the copies were empty."* Exactly right. A copy was built from the
    /// chamber's **recipe**, and the shipped binary opens on `founders: 0,
    /// colonies: 0` — the box starts empty and you stock it — so everything
    /// planted by hand lived in the *world*, which the recipe has never heard
    /// of.
    ///
    /// The bed here is founded through the spec so the test can run headless,
    /// but the mechanism is the one that was broken: the batch is started
    /// from a spec with **no founders at all**, so any life in a copy can only
    /// have arrived by cloning the live world.
    ///
    /// Both halves again, because either alone is green on a broken build:
    /// the copies must **carry the stand**, and they must still **differ from
    /// each other** — a clone that also copied the seed would satisfy the
    /// first and make the rack one world wearing N labels.
    #[test]
    fn copies_carry_what_was_planted_and_still_diverge() {
        // **A colony, and that is load-bearing rather than incidental.**
        // Reseeding a clone changes only draws keyed on `world.seed`, and the
        // two halves of the biosphere read it very differently:
        //
        //   - **creatures read it every tick** (`creature.rs`'s
        //     `RNG_SLOT_MOVE`, `:458` and `:1695`), so ants diverge on the
        //     next step;
        //   - **plants read it only at seeding** (`plant.rs`'s
        //     `seed_genotype`, `:1178`; growth is keyed on `(organism, x, y,
        //     frame)` instead), so an established stand grows *identically*
        //     in every copy until something new germinates or breeds.
        //
        // That is worth knowing rather than working around: copy a settled
        // plant-only box and the copies hold still together until the next
        // generation starts. A plants-only bed here made this test fail on
        // its divergence half for a reason that was about the engine and not
        // about the batch.
        let mut lab = Lab::new(scene::LabBox { colonies: 1, ..rack_bed(1) });
        run(&mut lab, 400);
        let alive = lab.world.live_organism_count();
        assert!(alive > 0, "the bed never germinated, so this test cannot see the thing it is about");

        // **The recipe is emptied.** Anything alive in a copy now has to have
        // come from the world, which is the whole claim.
        lab.batch_spec.base = scene::LabBox { founders: 0, colonies: 0, ..rack_bed(1) };
        lab.batch_spec.replicates = 3;
        lab.batch_spec.frames = 600;
        lab.start_batch();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
        while lab.batch.is_some() && std::time::Instant::now() < deadline {
            lab.poll_batch();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(lab.batch.is_none(), "the batch never finished");

        let rows = lab.chamber_summaries();
        let landed: Vec<&ChamberSummary> = rows.iter().filter(|r| r.label.starts_with("BATCH")).collect();
        assert_eq!(landed.len(), 3, "three copies");
        for r in &landed {
            let c = r.census.as_ref().expect("a census");
            assert!(
                c.plants > 0,
                "a copy came out EMPTY -- it was built from the recipe (0 founders) instead of copied \
                 from the box, which is exactly the bug this test is named for"
            );
        }
        // Read on the animals, for the reason at the top: they are the half
        // that reads the seed every tick.
        let distinct: std::collections::HashSet<(usize, usize, usize)> = landed
            .iter()
            .filter_map(|r| r.census.as_ref())
            .map(|c| (c.animals, c.animal_cells, c.plant_cells))
            .collect();
        assert!(
            distinct.len() > 1,
            "the copies carry the stand but are identical -- the seed is not reaching them, so the \
             rack is one world wearing three labels. Got {distinct:?}"
        );
    }

    /// **REBUILD gives back the same box, not a similar one.**
    ///
    /// The claim the whole memory policy rests on: a run whose world was
    /// dropped keeps ~10 KB of record instead of 2.5 MB of world, and that is
    /// only affordable because the spec plus its tick count reproduces the run
    /// **exactly**. `tests/determinism.rs`'s lab bed asserts the engine half;
    /// this asserts that the rack actually spends it — that the row's spec,
    /// its seed and its length all survive being kept as a record.
    ///
    /// So the assertion is not "a chamber appeared": it is that the rebuilt
    /// chamber's census matches the record's, field for field. A rebuild that
    /// ran the right spec for the wrong number of ticks would pass the first
    /// and fail this.
    #[test]
    fn a_rebuilt_record_reproduces_its_run_exactly() {
        let mut lab = Lab::new(rack_bed(1));
        lab.batch_spec.replicates = 2;
        lab.batch_spec.frames = 600;
        // Nothing may be held, so both runs land as records rather than
        // chambers — which is the state this verb exists for.
        lab.batch_spec.keep_bytes = 0;
        lab.start_batch();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
        while lab.batch.is_some() && std::time::Instant::now() < deadline {
            lab.poll_batch();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(lab.on_record.len(), 2, "with no memory budget both runs must be records");
        let chambers_before = lab.chamber_count();
        let kept = lab.on_record[0].census.clone();
        assert!(kept.frame > 0, "a record of a run that never ran");

        // On-record rows are numbered after the chambers.
        let row = lab.rack.len();
        assert!(lab.rebuild_record(row).contains("REBUILDING"), "the verb must say it started");
        while lab.batch.is_some() && std::time::Instant::now() < deadline {
            lab.poll_batch();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        assert_eq!(lab.chamber_count(), chambers_before + 1, "the rebuilt row must become a chamber");
        assert_eq!(lab.on_record.len(), 1, "the record it was rebuilt from must retire, or the rack counts the run twice");
        let back = lab.chamber_summaries().into_iter().find(|r| r.index == chambers_before).expect("the new chamber");
        let got = back.census.expect("a rebuilt chamber arrives with its census");
        assert_eq!(
            (got.frame, got.plants, got.plant_cells, got.animals, got.seeds_borne, got.germinations),
            (kept.frame, kept.plants, kept.plant_cells, kept.animals, kept.seeds_borne, kept.germinations),
            "the rebuild is not the run it replaced -- the record's spec, seed or length did not survive being kept"
        );
        assert!(lab.on_record.iter().all(|r| !r.rebuilding), "a row left stuck saying REBUILDING can never be pressed again");
    }

    /// **A hand-placed wall is there now, survives a rebuild, and comes out
    /// with the same gesture that put it in.**
    ///
    /// Three claims, and the middle one is the reason the wall lives on the
    /// *spec* rather than only in the world: every other bed knob takes
    /// effect on rebuild, so a wall that a rebuild silently removed would put
    /// the two halves of the same idea in a fight.
    ///
    /// The last assertion is the one that makes it a wall rather than a
    /// decoration: `compartment_spans` — the function every founder, colony
    /// and lamp is placed through — must see it, or the box looks divided and
    /// is not.
    #[test]
    fn a_hand_placed_wall_holds_now_survives_a_rebuild_and_toggles_off() {
        let mut lab = Lab::new(rack_bed(1));
        let x = lab.spec.width / 3;
        let spans_before = lab.spec.compartment_spans().len();

        lab.ui.set_tool(ui::Tool::Wall);
        lab.wall_at(x);
        assert!(lab.spec.extra_walls.contains(&x), "the wall is not in the spec, so a rebuild would lose it");
        assert_eq!(
            lab.spec.compartment_spans().len(),
            spans_before + 1,
            "the placement machinery cannot see the wall -- founders and colonies would be spread straight across it"
        );
        // Standing in the world *now*, not on the next rebuild.
        let mid = (lab.spec.ceiling_y() + lab.spec.bed_bottom_for_test()) / 2;
        assert_eq!(lab.world.get(x, mid).material, crate::sim::material::STONE, "the wall was written to the spec and not to the box");

        // ...and it is still there after the rebuild every bed knob triggers.
        lab.reset();
        assert!(lab.spec.extra_walls.contains(&x), "the rebuild dropped the wall from the spec");
        assert_eq!(lab.world.get(x, mid).material, crate::sim::material::STONE, "the rebuild did not paint the wall back");

        // The same gesture takes it out: the undo for a misplaced wall has to
        // be the mistake repeated, not a second tool to go and find.
        lab.wall_at(x + 1);
        assert!(!lab.spec.extra_walls.contains(&x), "clicking near the wall did not remove it");
        assert_eq!(lab.spec.compartment_spans().len(), spans_before, "the span did not close back up");
        assert_ne!(lab.world.get(x, mid).material, crate::sim::material::STONE, "the stone is still standing in the box");
    }

    /// **A batch fills the rack, and the copies are different worlds.**
    ///
    /// The whole feature end to end: start it, poll it the way the frame loop
    /// does, and check what lands. The last assertion is the one that matters
    /// — chambers appearing is also true of a batch that ran the same world N
    /// times, which is the failure the runner's own control exists to catch
    /// and which must not be able to reach the rack either.
    #[test]
    fn a_batch_fills_the_rack_with_different_worlds() {
        let mut lab = Lab::new(rack_bed(1));
        lab.batch_spec.replicates = 3;
        lab.batch_spec.frames = 900;
        let before = lab.chamber_count();
        assert!(lab.start_batch().contains("RUNNING 3 COPIES"), "the verb must say what it started");
        assert!(lab.start_batch().contains("ALREADY RUNNING"), "a second rack must be refused, not queued");

        // Polled the way `advance` does it — never a blocking join.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
        while lab.batch.is_some() && std::time::Instant::now() < deadline {
            lab.poll_batch();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(lab.batch.is_none(), "the batch never finished inside three minutes");

        assert_eq!(lab.chamber_count(), before + 3, "three copies must land as three chambers");
        let rows = lab.chamber_summaries();
        let landed: Vec<&ChamberSummary> = rows.iter().skip(before).collect();
        assert!(landed.iter().all(|r| r.frame == 900), "an adopted chamber must carry the run it did, not a fresh world");
        assert!(
            landed.iter().all(|r| r.census.is_some()),
            "an adopted chamber must arrive with its census -- a rack row with no numbers is a run nobody can compare"
        );
        let distinct: std::collections::HashSet<(usize, usize)> =
            landed.iter().filter_map(|r| r.census.as_ref()).map(|c| (c.plants, c.plant_cells)).collect();
        assert!(
            distinct.len() > 1,
            "three copies came out identical -- the batch is one world wearing three labels. Got {distinct:?}"
        );
    }

    /// **A parked chamber does not advance, and the one on screen does.**
    ///
    /// Both arms in one test on purpose. The frozen arm alone is green for a
    /// lab whose `tick` has stopped working altogether — `CLAUDE.md`'s *green
    /// is the default state* — so the running arm is the positive control
    /// that says the instrument can move at all.
    #[test]
    fn a_parked_chamber_holds_still_while_the_one_on_screen_runs() {
        let mut lab = Lab::new(rack_bed(1));
        let parked = lab.add_chamber(rack_bed(2));
        let before = lab.chamber_summaries()[parked].frame;

        run(&mut lab, 40);

        let after = lab.chamber_summaries();
        assert!(after[lab.active_chamber()].frame >= 40, "the positive control: the box on screen must have run, got {}", after[lab.active_chamber()].frame);
        assert_eq!(after[parked].frame, before, "a parked chamber advanced while another was running");
    }

    /// **Switching away and back is lossless.**
    ///
    /// Paired with its own sensitivity half: the same digest must *change*
    /// when the box is stepped, or "unchanged after a round trip" is a claim
    /// about a blind hash rather than about the swap.
    #[test]
    fn switching_away_and_back_leaves_the_box_exactly_as_it_was() {
        let mut lab = Lab::new(rack_bed(1));
        let other = lab.add_chamber(rack_bed(2));
        run(&mut lab, 30);

        let before = grid_hash(&lab.world);
        lab.switch_to(other);
        run(&mut lab, 30);
        lab.switch_to(0);
        assert_eq!(grid_hash(&lab.world), before, "a round trip through another chamber changed the box");

        // The sensitivity half: this digest is not a constant.
        run(&mut lab, 30);
        assert_ne!(grid_hash(&lab.world), before, "the digest cannot see the box changing, so the assertion above proves nothing");
    }

    /// **The seed is what makes a copy a replicate, and this is both halves
    /// of that claim.**
    ///
    /// The whole premise of running a rack of copies rests on this. Every
    /// draw in the engine is a pure function of `(world.seed, identity,
    /// position)`, so a duplicate at the *same* seed is not a similar box, it
    /// is a bit-identical one — a rack of them is one sample wearing many
    /// labels, which is `CLAUDE.md`'s *3 populations wearing 24 logs*
    /// arriving in a new costume.
    ///
    /// So: same seed must be **equal**, reseeded must **differ**. Either
    /// assertion alone is green for a broken build — the first for a lab that
    /// ignores the seed entirely, the second for one whose duplicate shares
    /// nothing with its parent.
    #[test]
    fn a_reseeded_duplicate_diverges_and_an_unseeded_one_does_not() {
        const FRAMES: u32 = 600;

        let mut lab = Lab::new(rack_bed(7));
        let twin = lab.duplicate_active(false);
        let replicate = lab.duplicate_active(true);
        assert_ne!(
            lab.chamber_summaries()[replicate].seed,
            lab.chamber_summaries()[twin].seed,
            "reseeding handed the replicate the seed it was meant to differ by"
        );

        run(&mut lab, FRAMES);
        let parent = grid_hash(&lab.world);

        lab.switch_to(twin);
        run(&mut lab, FRAMES);
        assert_eq!(grid_hash(&lab.world), parent, "a duplicate at the same seed came out different — the engine is not reproducible and no comparison across a rack means anything");

        lab.switch_to(replicate);
        run(&mut lab, FRAMES);
        assert_ne!(grid_hash(&lab.world), parent, "a duplicate at a NEW seed came out identical — the seed is not reaching the copy, so a rack of replicates is one world wearing many labels");
    }

    /// The rack's one invariant, through every verb that reshapes it.
    #[test]
    fn the_rack_invariant_survives_add_switch_and_close() {
        let mut lab = Lab::new(rack_bed(1));
        assert!(lab.rack_invariant_holds(), "a fresh lab");
        let b = lab.add_chamber(rack_bed(2));
        let c = lab.add_chamber(rack_bed(3));
        assert!(lab.rack_invariant_holds(), "after adding");
        lab.switch_to(c);
        assert!(lab.rack_invariant_holds(), "after switching");
        assert_eq!(lab.active_chamber(), c);

        assert!(!lab.remove_chamber(lab.active_chamber()), "closing the box on screen must be refused");
        assert!(lab.remove_chamber(b), "closing a parked box");
        assert!(lab.rack_invariant_holds(), "after closing");
        assert_eq!(lab.chamber_count(), 2);
        // `b` sat before `c`, so closing it renumbers `c` down by one — the
        // way closing a tab does.
        assert_eq!(lab.active_chamber(), c - 1, "the active index did not follow its chamber past the closed one");
    }

    /// **A switch forces a full redraw.**
    ///
    /// `Renderer` carries the previous frame's dirty rectangles and they
    /// belong to the box just left; without this the old chamber is painted
    /// under the new one wherever the new one is settled. Nothing about that
    /// is a compile error and nothing in a census can see it, so the flag is
    /// asserted directly.
    #[test]
    fn a_switch_forces_a_full_redraw() {
        let mut lab = Lab::new(rack_bed(1));
        let other = lab.add_chamber(rack_bed(2));
        lab.view_dirty = false;
        lab.switch_to(other);
        assert!(lab.view_dirty, "a switch left the dirty-rect skip believing the screen still holds the old chamber");
        // A no-op switch must not claim the screen changed.
        lab.view_dirty = false;
        lab.switch_to(other);
        assert!(!lab.view_dirty, "switching to the chamber already on screen forced a redraw for nothing");
    }

    /// **`SPACE` stops the world, and this is the counter that says so.**
    ///
    /// Owner complaint, 2026-08-30: *"what does spacebar/tending mean. it
    /// isn't pausing anything."* An image cannot answer this — a stopped box
    /// and a settled one look identical in a still frame, which is exactly
    /// `CLAUDE.md`'s *"did it fire at all" needs a counter, not a picture*.
    /// `World::frame` is that counter.
    ///
    /// Both arms in one test, because the paused arm alone would be green for
    /// a lab whose `advance` had stopped working altogether.
    #[test]
    fn a_paused_lab_does_not_advance_the_world_and_a_running_one_does() {
        let mut lab = Lab::new(scene::LabBox {
            founders: 0,
            colonies: 0,
            ..scene::LabBox::default()
        });
        assert_eq!(lab.time.phase, time::Phase::Paused, "a fresh lab starts stopped");
        let at_rest = lab.world.frame;
        for _ in 0..120 {
            lab.advance(std::time::Duration::from_millis(16));
        }
        assert_eq!(lab.world.frame, at_rest, "a stopped box ran ticks");

        lab.act(ui::Action::TogglePhase);
        assert_eq!(lab.time.phase, time::Phase::Running);
        for _ in 0..120 {
            lab.advance(std::time::Duration::from_millis(16));
        }
        assert!(
            lab.world.frame > at_rest,
            "the positive control did not fire: the world stayed at frame {at_rest}"
        );

        // ...and `SPACE` again stops it where it stands.
        lab.act(ui::Action::TogglePhase);
        let stopped_at = lab.world.frame;
        for _ in 0..120 {
            lab.advance(std::time::Duration::from_millis(16));
        }
        assert_eq!(lab.world.frame, stopped_at, "stopping again did not stop it");
    }

    /// **An empty box is a state, not an error.**
    ///
    /// `bin/lab.rs` opens the game with no founders and no colony — owner
    /// request, 2026-08-30, *"the game should start with no plants or
    /// creatures. I add them."* Everything that reads a population now has to
    /// survive one of zero, and three of them divide by a count or index a
    /// sorted list. This runs an empty bed and paints every page over it.
    #[test]
    fn an_empty_box_runs_and_draws_every_page() {
        let mut lab = Lab::new(scene::LabBox {
            founders: 0,
            colonies: 0,
            ..scene::LabBox::default()
        });
        lab.show_help = false;
        assert_eq!(lab.world.live_organism_count(), 0, "the bed was not empty");
        lab.act(ui::Action::TogglePhase);
        for _ in 0..240 {
            lab.advance(std::time::Duration::from_millis(16));
        }
        assert!(lab.world.frame > 0, "the positive control did not fire");
        assert_eq!(lab.world.live_organism_count(), 0, "something grew in an empty bed");

        let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        // Every page, plus the census, plus a cursor so the hover paths run.
        lab.set_cursor(Some((40, 40)));
        for panel in [ui::Panel::Plants, ui::Panel::Ants, ui::Panel::Box, ui::Panel::Params, ui::Panel::Shelf] {
            lab.ui.panel = Some(panel);
            lab.draw(&mut frame, 60.0);
        }
        lab.ui.panel = None;
        lab.draw(&mut frame, 60.0);
    }

    /// **The bench: the first free cell above the bed's own soil.** Tests
    /// used to build a stone shelf of their own here, which is a scene that
    /// contradicts the box — `CLAUDE.md`'s *a scene that contradicts the code
    /// will look like a bug in the code*. A seed needs bare soil under it and
    /// a body needs its cells free, and the bed already has both.
    fn bench_cell(lab: &Lab) -> (i32, i32) {
        (lab.spec.width / 2 - 30, lab.spec.ground_y - 1)
    }

    /// **The shelf tests run one at a time**, because the directory override
    /// is an environment variable and an environment is per *process*, not
    /// per test. `cargo test` runs a module's tests on several threads in one
    /// process, so two shelf tests without this would each point the override
    /// at their own directory and then read each other's rack — a flake that
    /// depends on thread scheduling and would reproduce about as often as it
    /// did not.
    ///
    /// Poison is deliberately ignored: a panicking test leaves the lock
    /// poisoned, and turning one real failure into four cascading ones hides
    /// the real one.
    static SHELF_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// A private shelf directory, held for the caller's whole test, so a test
    /// that keeps a specimen does not write into the working tree.
    ///
    /// The pid is in the name because `/tmp` is shared between agents in this
    /// project's containers — one lane has already captured another lane's
    /// screenshot for exactly that reason
    /// (`Reports/lanes/evolution-lab-coordinator.md`).
    fn shelf_scratch(tag: &str) -> (std::path::PathBuf, std::sync::MutexGuard<'static, ()>) {
        let guard = SHELF_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("pixel_physics_lab_shelf_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch shelf");
        std::env::set_var(crate::sim::specimen::SHELF_DIR_ENV, &dir);
        (dir, guard)
    }

    /// **Keep an ant, free it, and check the released animal is the one that
    /// was kept** — through `Lab`'s own verbs rather than through
    /// `sim::specimen`, so the tool routing, the naming, the shelf write and
    /// the reload are all in the path.
    ///
    /// `specimen`'s own tests guard the genome round trip; this guards
    /// everything between a click and them, which is the half that has no
    /// other cover.
    #[test]
    fn keeping_an_ant_and_freeing_it_puts_the_same_animal_back() {
        let (dir, _shelf) = shelf_scratch("roundtrip");
        let mut lab = Lab::new(scene::LabBox { founders: 0, colonies: 0, ..scene::LabBox::default() });
        lab.show_help = false;

        // Somewhere to stand, then one ant on it.
        let (fx, fy) = bench_cell(&lab);
        crate::sim::creature::plant_creature_seed(&mut lab.world, fx + 10, fy, "ant").expect("an ant hatches");
        let id = lab.world.get(fx + 10, fy).organism_id();
        assert_ne!(id, 0, "the harness placed no ant, so there is nothing to keep");
        let genome = lab.world.organism(id).expect("live ant").genome.clone();

        lab.keep_at(fx + 10, fy);
        assert_eq!(lab.ui.shelf().len(), 1, "keeping wrote no jar: {:?}", lab.ui.notice_text());
        assert!(dir.join("ant.ron").exists(), "the jar was not written under the species name");

        // Arm it the way a player does -- by clicking the row -- and free it.
        lab.act(ui::Action::ShelfSelect(0));
        assert_eq!(lab.ui.tool(), ui::Tool::Release, "picking a jar did not arm the tool that uses it");
        lab.release_at(fx + 40, fy);

        let freed = lab.world.get(fx + 40, fy).organism_id();
        assert_ne!(freed, 0, "nothing was freed: {:?}", lab.ui.notice_text());
        assert_ne!(freed, id, "the release found the original rather than a new individual");
        let state = lab.world.organism(freed).expect("the freed ant");
        assert_eq!(state.genome, genome, "the freed ant is not carrying the kept genome");
        assert!(state.stocked, "the freed ant is not flagged as coming off the shelf");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **The dial is the graded middle, and it has to be reachable from the
    /// bar.** `CLAUDE.md`'s first law: an outcome is a distribution, not a
    /// binary. A shelf that could only produce *this exact animal* or nothing
    /// has the defect the rubble had.
    ///
    /// Positive and negative control in one test: at `CLONE` the freed genome
    /// must be identical, and at four broods it must not be.
    #[test]
    fn the_brood_dial_reaches_the_world_from_the_bar() {
        let (dir, _shelf) = shelf_scratch("dial");
        let mut lab = Lab::new(scene::LabBox { founders: 0, colonies: 0, ..scene::LabBox::default() });
        lab.show_help = false;
        let (fx, fy) = bench_cell(&lab);
        crate::sim::creature::plant_creature_seed(&mut lab.world, fx + 10, fy, "ant").expect("an ant hatches");
        let id = lab.world.get(fx + 10, fy).organism_id();
        let genome = lab.world.organism(id).expect("live ant").genome.clone();
        lab.keep_at(fx + 10, fy);
        lab.act(ui::Action::ShelfSelect(0));

        assert_eq!(lab.ui.broods(), 0, "the dial does not start at CLONE");
        lab.release_at(fx + 40, fy);
        let clone = lab.world.get(fx + 40, fy).organism_id();
        assert_eq!(lab.world.organism(clone).expect("freed").genome, genome, "a CLONE release is not a clone");

        for _ in 0..4 {
            lab.act(ui::Action::Broods(1));
        }
        assert_eq!(lab.ui.broods(), 4);
        lab.release_at(fx + 70, fy);
        let drifted = lab.world.get(fx + 70, fy).organism_id();
        assert_ne!(drifted, clone);
        assert_ne!(lab.world.organism(drifted).expect("freed").genome, genome, "four broods produced a bit-identical genome; the dial is not reaching the world");

        // ...and the dial cannot be turned past either end.
        for _ in 0..20 {
            lab.act(ui::Action::Broods(-1));
        }
        assert_eq!(lab.ui.broods(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **Every jar keeps its own name and nothing is ever overwritten.** A
    /// kept specimen is the one thing in this lab a player cannot regenerate,
    /// so two keeps of the same species must produce two jars.
    #[test]
    fn keeping_twice_makes_two_jars_and_drifting_makes_a_third() {
        let (dir, _shelf) = shelf_scratch("names");
        let mut lab = Lab::new(scene::LabBox { founders: 0, colonies: 0, ..scene::LabBox::default() });
        lab.show_help = false;
        let (fx, fy) = bench_cell(&lab);
        for dx in [10, 30] {
            crate::sim::creature::plant_creature_seed(&mut lab.world, fx + dx, fy, "ant").expect("an ant hatches");
            lab.keep_at(fx + dx, fy);
        }
        let names: Vec<String> = lab.ui.shelf().iter().map(|j| j.name.clone()).collect();
        assert_eq!(names, vec!["ant".to_string(), "ant_2".to_string()], "keeping twice did not number the second jar");

        lab.act(ui::Action::ShelfSelect(0));
        lab.act(ui::Action::Broods(1));
        lab.act(ui::Action::ShelfDrift);
        let names: Vec<String> = lab.ui.shelf().iter().map(|j| j.name.clone()).collect();
        assert_eq!(names.len(), 3, "drifting did not add a jar: {:?}", lab.ui.notice_text());
        // **The jar you drifted from stays armed.** Drifting writes a file
        // and reloads the rack, and the first version of the reload cleared
        // the selection -- so the next FREE refused, right after a button
        // that had visibly worked. Making siblings from one parent is the
        // common case, so the parent keeps the arm and the notice names the
        // child.
        assert_eq!(lab.ui.armed_jar().map(|j| j.name.as_str()), Some("ant"), "drifting disarmed the jar it bred from");
        // The drifted jar records where it came from, which is the shelf's
        // own pedigree and the only record of what the player selected for.
        let child = lab.ui.shelf().iter().find(|j| j.name == "ant_3").expect("the drifted jar");
        assert_eq!(child.taken.from_jar, Some(("ant".to_string(), 1)));

        // ...and DISCARD is the only thing that removes one.
        lab.act(ui::Action::ShelfSelect(0));
        lab.act(ui::Action::ShelfDiscard);
        assert_eq!(lab.ui.shelf().len(), 2, "discard did not remove the armed jar");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **A plant is keepable too, and that is the half `species_export` has
    /// never been able to do.** It comes back as a seed rather than as a
    /// grown plant, which is the same deal the `PLANT` tool offers.
    #[test]
    fn keeping_a_plant_and_freeing_it_sows_the_kept_genome() {
        let (dir, _shelf) = shelf_scratch("plant");
        let mut lab = Lab::new(scene::LabBox { founders: 0, colonies: 0, ..scene::LabBox::default() });
        lab.show_help = false;
        let (fx, fy) = bench_cell(&lab);
        assert!(lab.world.plant_tree_species(fx + 10, fy, "herb"), "the harness planted nothing");
        let id = lab.world.get(fx + 10, fy).organism_id();
        if let Some(st) = lab.world.organism_mut(id) {
            st.alleles = [1, 2, 1, 1, 1, 2];
        }
        let draws = lab.world.organism(id).expect("live plant").genotype_draws;

        lab.keep_at(fx + 10, fy);
        assert_eq!(lab.ui.shelf().len(), 1, "keeping a plant wrote no jar: {:?}", lab.ui.notice_text());
        assert!(dir.join("herb.ron").exists());
        lab.act(ui::Action::ShelfSelect(0));
        lab.release_at(fx + 40, fy);

        let sown = lab.world.get(fx + 40, fy).organism_id();
        assert_ne!(sown, 0, "nothing was sown: {:?}", lab.ui.notice_text());
        let state = lab.world.organism(sown).expect("the sown seed");
        assert_eq!(state.genotype_draws, draws, "the sown seed is not carrying the kept genome");
        assert_eq!(state.alleles, [1, 2, 1, 1, 1, 2], "the discrete loci did not survive the jar");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **Every refusal is sayable, and none of them half-places anything.**
    /// A verb that fails silently is the second law's own failure — the
    /// player pressed a button and the box did not change.
    #[test]
    fn the_shelf_verbs_all_say_what_happened_when_they_refuse() {
        let (dir, _shelf) = shelf_scratch("refuse");
        let mut lab = Lab::new(scene::LabBox { founders: 0, colonies: 0, ..scene::LabBox::default() });
        lab.show_help = false;
        let (fx, fy) = bench_cell(&lab);

        for (label, act) in [
            ("keep nothing", &(|l: &mut Lab, x: i32, y: i32| l.keep_at(x, y)) as &dyn Fn(&mut Lab, i32, i32)),
            ("free nothing armed", &|l: &mut Lab, x: i32, y: i32| l.release_at(x, y)),
        ] {
            act(&mut lab, fx, fy);
            let said = lab.ui.notice_text().unwrap_or_default();
            assert!(said.len() > 10, "{label} said nothing");
        }
        for action in [ui::Action::ShelfDrift, ui::Action::ShelfDiscard, ui::Action::ShelfPromote] {
            lab.act(action);
            let said = lab.ui.notice_text().unwrap_or_default();
            assert!(said.contains("NOTHING ARMED"), "{action:?} with an empty shelf said {said:?}");
        }
        assert_eq!(lab.ui.shelf().len(), 0, "a refusal wrote a jar");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **The parameters page, driven the way a player drives it: by clicking.**
    ///
    /// The positive control the bar already has (`every_button_answers_where_
    /// it_was_drawn`), extended to the one page whose rows are controls. Every
    /// `+` face is clicked at the middle of the rectangle it was *drawn* at —
    /// read back out of the retained layout, so this cannot pass against a
    /// hand-written pixel table — and the value is read back out of the
    /// registry afterwards.
    ///
    /// **A row that paints its figure correctly and refuses to move looks
    /// identical to a working one in any screenshot.** `params::tests::every_
    /// writable_parameter_actually_moves` guards the writers; this guards
    /// everything between the click and them: the hit test, the row index the
    /// action carries, and `Lab::act`'s routing.
    #[test]
    fn clicking_a_parameters_row_moves_it() {
        let mut lab = Lab::new(scene::LabBox::default());
        lab.show_help = false;
        let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        lab.act(ui::Action::Panel(ui::Panel::Params));
        // A frame has to be painted before anything on it can be clicked: the
        // page's rectangles are retained from the last paint, deliberately.
        lab.draw(&mut frame, 60.0);

        let mut moved = 0;
        for group in 0..params::GROUPS.len() {
            lab.act(ui::Action::ParamGroup(group));
            lab.draw(&mut frame, 60.0);
            for row in 0..lab.ui.page_params(&lab.world, &lab.spec).len() {
                let Some(r) = lab.ui.widget_rect(ui::Action::ParamAdjust(row, 1)) else { continue };
                let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
                assert_eq!(
                    lab.ui.hit(cx, cy),
                    Some(ui::Action::ParamAdjust(row, 1)),
                    "row {row} of page {group} answered for another widget"
                );
                let before = lab.ui.page_params(&lab.world, &lab.spec)[row].display();
                lab.press(cx, cy);
                lab.release(cx, cy);
                lab.draw(&mut frame, 60.0);
                let after = lab.ui.page_params(&lab.world, &lab.spec)[row].display();
                let param = &lab.ui.page_params(&lab.world, &lab.spec)[row];
                // A knob shipped at its own ceiling clamps rather than moves,
                // which is right -- `water.flow_rate` is 1000 of 1000.
                if param.tunable.value < param.tunable.max {
                    assert_ne!(before, after, "{}.{} did not move on a click", param.tunable.category, param.tunable.name);
                    moved += 1;
                }
            }
        }
        assert!(moved >= 30, "only {moved} rows were reachable by a click");
    }

    /// A click on the parameters page must not also reach the world behind it.
    /// The page is a big rectangle over the bed and the `LOOK` tool is armed
    /// by default, so without this every adjustment would move the inspector
    /// as well.
    #[test]
    fn a_click_on_the_parameters_page_does_not_reach_the_world() {
        let mut lab = Lab::new(scene::LabBox::default());
        lab.show_help = false;
        let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        lab.act(ui::Action::Panel(ui::Panel::Params));
        lab.draw(&mut frame, 60.0);
        let before = lab.ui.inspecting();
        // Somewhere on the page that is not one of its chips: the title bar.
        lab.press(20, ui::bar_top() - 120);
        lab.release(20, ui::bar_top() - 120);
        assert_eq!(lab.ui.inspecting(), before, "a click on the page moved the inspector");
    }

    /// An empty bed with the view at rest, and the screen position of one
    /// world cell on it. Every test below aims through `world_to_screen`
    /// rather than assuming screen and world coincide, which is only true
    /// while the camera has not moved.
    fn bench() -> Lab {
        let mut lab = Lab::new(scene::LabBox {
            founders: 0,
            colonies: 0,
            ..scene::LabBox::default()
        });
        lab.show_help = false;
        let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        // One painted frame, because a click is tested against the retained
        // bar and there is not one until something has been drawn.
        lab.draw(&mut frame, 60.0);
        lab
    }

    fn aim(lab: &Lab, wx: i32, wy: i32) -> (i32, i32) {
        lab.renderer.world_to_screen(wx, wy).expect("the bed is on screen")
    }

    /// Click a **world** cell, aimed through the renderer. Takes the cell
    /// rather than a screen point so a caller cannot borrow `lab` twice in
    /// one expression, and so no test can aim at a screen pixel that is only
    /// the right one while the camera has not moved.
    fn click_cell(lab: &mut Lab, wx: i32, wy: i32) {
        let at = aim(lab, wx, wy);
        lab.set_cursor(Some(at));
        lab.press(at.0, at.1);
        lab.release(at.0, at.1);
    }

    /// **Picking what a tool will use must not put the tool away**, which is
    /// the owner's playtest report: *"i will click plant, then change the type
    /// from grass to herb and suddenly plant is unselected and now the mouse
    /// is on look."*
    ///
    /// The fault is `Ui::set_tool`'s toggle reached through a chip, so the
    /// test asserts the *toggle is still there* on the second arm -- pressing
    /// `PLANT` twice must still land on `LOOK`. Without that half it would go
    /// green for a `set_tool` that had simply stopped toggling at all, which
    /// would break every tool button on the bar.
    #[test]
    fn cycling_the_species_leaves_the_plant_tool_armed() {
        let mut lab = bench();
        lab.act(ui::Action::Tool(ui::Tool::Plant));
        assert_eq!(lab.ui.tool(), ui::Tool::Plant);
        for _ in 0..4 {
            lab.act(ui::Action::NextSpecies);
            assert_eq!(
                lab.ui.tool(),
                ui::Tool::Plant,
                "cycling the species chip put the planting tool away"
            );
        }
        // ...and the toggle it must not have cost: the tool button itself.
        lab.act(ui::Action::Tool(ui::Tool::Plant));
        assert_eq!(lab.ui.tool(), ui::Tool::Look, "pressing an armed tool no longer puts it away");
    }

    /// **Gate 4's three verbs, each with the count that says it fired.**
    ///
    /// One test, because each of them looks identical to nothing happening in
    /// a still frame — a seed is one cell, an ant is two — and because the
    /// shared setup is most of the cost. `CLAUDE.md`'s rule: *"did it fire at
    /// all" needs a counter, not a picture*.
    ///
    /// The negative arms are the half that matters. Every one of these can
    /// return quietly: `plant_tree_species` refuses an occupied cell,
    /// `found_colony` places nobody where there is no surface, and a cull aimed
    /// at bare soil resolves to no organism. A test that only asserted the
    /// happy path would be green for a verb wired to the wrong button.
    #[test]
    fn the_three_verbs_change_the_world_and_say_so_when_they_cannot() {
        let mut lab = bench();
        let surface = lab.spec.ground_y;
        // **Well apart, and that is not tidiness.** `found_colony` paints a
        // band of `nest` along the surface and drops ants across a wider one
        // still; founding it on top of the seed overwrote the seed's only
        // cell, freed its slot, and the cull arm below then failed looking for
        // a plant that the *previous* arm had deleted.
        let x = 80;
        let colony_x = lab.spec.width - 112;

        lab.act(ui::Action::Tool(ui::Tool::Plant));
        assert_eq!(lab.ui.tool(), ui::Tool::Plant);
        let before = lab.world.live_organism_count();
        click_cell(&mut lab, x, surface);
        assert_eq!(
            lab.world.live_organism_count(),
            before + 1,
            "planting put nothing in the bed"
        );
        // ...and the negative: the same click into the stone base, which has
        // no empty cell within reach above it.
        let deep = lab.spec.ground_y + lab.spec.soil_depth + 40;
        let before = lab.world.live_organism_count();
        click_cell(&mut lab, x, deep);
        assert_eq!(
            lab.world.live_organism_count(),
            before,
            "planting into the stone base planted something"
        );

        lab.act(ui::Action::Tool(ui::Tool::Colony));
        let before = lab.world.live_creature_count();
        click_cell(&mut lab, colony_x, surface);
        assert!(
            lab.world.live_creature_count() > before,
            "founding a colony released nobody: {before} creatures either side"
        );

        // Cull, aimed at the plant put in above — found by walking the live
        // list rather than by remembering an id, so this cannot pass against
        // an organism that is no longer there.
        lab.act(ui::Action::Tool(ui::Tool::Cull));
        let plant = lab
            .world
            .live_organism_ids()
            .into_iter()
            .find(|id| {
                lab.world.organism_state(*id).is_some_and(|st| {
                    lab.world.species.get(st.species).creature.is_none() && !st.cells.is_empty()
                })
            })
            .expect("the planted seed is alive");
        let (cell_at, cells_before) = {
            let st = lab.world.organism_state(plant).expect("live");
            (*st.cells.keys().next().expect("a cell"), st.cells.len())
        };
        click_cell(&mut lab, cell_at.0, cell_at.1);
        let after = lab.world.organism_state(plant).expect("still tracked");
        assert!(after.senescent, "the cull did not mark the plant");
        // **The graded death.** `CLAUDE.md`'s first law: an outcome is a
        // distribution, not a binary. A cull that emptied the cell list would
        // pass any "is it dead" assertion and be exactly the disappearance the
        // owner ruled against.
        assert_eq!(
            after.cells.len(),
            cells_before,
            "the cull deleted the plant instead of leaving remains to rot"
        );

        // And a cull aimed at nothing says so rather than silently missing.
        let empty = lab.spec.ground_y - 40;
        assert!(lab.world.get(x, empty).organism_id() == 0, "test setup: that cell is bare");
        click_cell(&mut lab, x, empty);
    }

    /// **A culled animal becomes a corpse, and it takes a tick.**
    ///
    /// The other half of the graded death, and the half with no senescence
    /// path: nothing outside `plant.rs` reads the flag, so a creature cull
    /// takes its energy to zero and `creature::apply_creature_energy` writes
    /// the corpse on its next tick. The **stopped** box is asserted first,
    /// because that is the honest behaviour and it is also what would make a
    /// naive test green for a cull that did nothing at all.
    #[test]
    fn a_culled_animal_leaves_a_corpse_once_the_box_runs() {
        let mut lab = bench();
        let x = lab.spec.width / 2;
        let surface = lab.spec.ground_y;
        lab.act(ui::Action::Tool(ui::Tool::Colony));
        click_cell(&mut lab, x, surface);
        let alive = lab.world.live_creature_count();
        assert!(alive > 0, "test setup: the colony released nobody");

        let ant = lab
            .world
            .live_organism_ids()
            .into_iter()
            .find(|id| {
                lab.world
                    .organism_state(*id)
                    .is_some_and(|st| lab.world.species.get(st.species).creature.is_some())
            })
            .expect("an ant");
        let at = *lab.world.organism_state(ant).expect("live").cells.keys().next().expect("a cell");

        lab.act(ui::Action::Tool(ui::Tool::Cull));
        click_cell(&mut lab, at.0, at.1);
        assert_eq!(lab.world.organism_state(ant).map(|s| s.energy), Some(0.0));
        assert_eq!(
            lab.world.live_creature_count(),
            alive,
            "a stopped box killed something: nothing ticks while paused"
        );

        lab.act(ui::Action::TogglePhase);
        for _ in 0..120 {
            lab.advance(std::time::Duration::from_millis(16));
        }
        assert!(lab.world.frame > 0, "the positive control did not fire");
        assert!(
            lab.world.organism_state(ant).is_none(),
            "the culled ant is still alive after 120 frames"
        );
        let corpse = lab.world.materials.id_of("corpse").expect("corpse is compiled in");
        let bounds = lab.world.bounds().expect("bounds");
        let corpses: u32 = (bounds.min_y..=bounds.max_y)
            .flat_map(|y| (bounds.min_x..=bounds.max_x).map(move |x| (x, y)))
            .map(|(x, y)| u32::from(lab.world.get(x, y).material == corpse))
            .sum();
        assert!(corpses > 0, "the culled ant vanished instead of leaving meat");
    }

    /// **The two `aux` conventions point opposite ways, and getting either
    /// backwards manufactures water out of nothing.**
    ///
    /// `CLAUDE.md`'s standing gotcha, asserted in both directions rather than
    /// once: on a `Powder` `aux == 0` is *dry*, so soil painted at the default
    /// would be ground nothing can root in; on a `Liquid` `aux == 0` is
    /// *full*, so water painted at `LIQUID_FULL` would be a cell holding twice
    /// what it should. Each half would look completely correct on screen.
    #[test]
    fn the_soil_brush_lays_damp_ground_and_the_water_brush_lays_full_water() {
        use crate::sim::{material, update};
        let mut lab = bench();
        let (x, y) = (lab.spec.width / 2, lab.spec.ground_y - 30);
        let soil = lab.world.materials.id_of("soil").expect("soil");
        let water = lab.world.materials.id_of("water").expect("water");

        lab.act(ui::Action::Tool(ui::Tool::Soil));
        let at = aim(&lab, x, y);
        lab.set_cursor(Some(at));
        lab.press(at.0, at.1);
        lab.release(at.0, at.1);
        let cell = lab.world.get(x, y);
        assert_eq!(cell.material, soil, "the soil brush laid nothing down");
        assert_eq!(
            cell.aux(),
            material::SOIL_FIELD_CAPACITY,
            "soil painted at the powder default is bone dry"
        );

        let (wx, wy) = (x + 60, y);
        lab.act(ui::Action::Tool(ui::Tool::Water));
        let at = aim(&lab, wx, wy);
        lab.set_cursor(Some(at));
        lab.press(at.0, at.1);
        lab.release(at.0, at.1);
        let cell = lab.world.get(wx, wy);
        assert_eq!(cell.material, water, "the water brush laid nothing down");
        assert_eq!(
            update::liquid_fill(cell),
            material::LIQUID_FULL,
            "the water brush laid down a partly-drained cell"
        );
        // The inverse, which is the half that catches the convention being
        // flipped: a `Liquid` written at `LIQUID_FULL` in `aux` reads back as
        // full too, and holds twice what it should.
        assert_eq!(cell.aux(), 0, "water written at LIQUID_FULL manufactures water");

        // And the right button takes it back, wherever the tool is pointed.
        let before = lab.world.get(wx, wy).material;
        assert_ne!(before, material::EMPTY);
        lab.press_erase(at.0, at.1);
        lab.end_stroke();
        assert_eq!(lab.world.get(wx, wy).material, material::EMPTY, "the eraser left the cell");
    }

    /// **The food brush puts something edible on the ground, and the count is
    /// what says so.**
    ///
    /// `wiki/ants.md` records the two arms this verb exists to let a player
    /// run: food beside the nest breeds a colony thirteen generations deep,
    /// and the same colony foraging the sealed bed brings home four loads out
    /// of sixteen hundred pickups. Until this tool existed the box could not
    /// be put in the first state at all, so no measurement in it could
    /// separate *the foraging is broken* from *the economy is broken*.
    ///
    /// **Asserted as a census of food value, not as a picture and not as a
    /// material match alone.** A sheet of the bed looks the same whether this
    /// laid down fruit or dirt — the shelf's `FREE` bug was caught only by
    /// `organisms 89 -> 89` and by nothing on screen. What a forager cares
    /// about is what `creature::food_value` returns, so that is the quantity
    /// counted here.
    ///
    /// **The fault it was watched going red for** is the one the brush's own
    /// `match` is shaped to prevent: delete the `Tool::Food` arm from
    /// `paint_span` and the `_` arm catches it, so the brush lays **soil**.
    /// The material assertion fails and the census stays flat — which is
    /// exactly what a tool that silently painted ground would look like.
    #[test]
    fn the_food_brush_puts_something_edible_on_the_ground() {
        use crate::sim::creature;
        let mut lab = bench();
        let (x, y) = (lab.spec.width / 2, lab.spec.ground_y - 30);
        let windfall = lab.world.materials.id_of("windfall").expect("windfall is compiled in");

        // The whole bed's standing food value, before and after. A census
        // rather than one cell, so a brush that wrote a single cell and a
        // brush that wrote its whole disc are distinguishable.
        let edible = |lab: &Lab| -> f64 {
            let mut total = 0.0;
            for yy in 0..lab.spec.height {
                for xx in 0..lab.spec.width {
                    total += creature::food_value(&lab.world, lab.world.get(xx, yy)) as f64;
                }
            }
            total
        };
        let before = edible(&lab);

        lab.act(ui::Action::Tool(ui::Tool::Food));
        assert_eq!(lab.ui.tool(), ui::Tool::Food, "the food tool did not arm");
        let at = aim(&lab, x, y);
        lab.set_cursor(Some(at));
        lab.press(at.0, at.1);
        lab.release(at.0, at.1);

        let cell = lab.world.get(x, y);
        assert_eq!(cell.material, windfall, "the food brush laid down something that is not windfall");
        // The value is read off the material rather than a stamp -- windfall
        // does not set `worth_in_aux` -- so a cell painted at any `aux` is
        // worth the same, and that is the number a forager sees.
        assert!(
            creature::food_value(&lab.world, cell) > 0.0,
            "the cell the food brush painted is worth nothing to eat"
        );
        assert!(
            edible(&lab) > before,
            "the bed holds no more food than before the brush ran: {before} -> {}",
            edible(&lab)
        );

        // And it comes back out with the right button, like every other brush.
        lab.press_erase(at.0, at.1);
        lab.end_stroke();
        assert_eq!(lab.world.get(x, y).material, crate::sim::material::EMPTY, "the eraser left the food");
    }

    /// A drag lays down one continuous band, not a dab at each end.
    #[test]
    fn a_drag_paints_the_whole_span_it_swept() {
        let mut lab = bench();
        let (x, y) = (lab.spec.width / 2 - 40, lab.spec.ground_y - 30);
        let soil = lab.world.materials.id_of("soil").expect("soil");
        lab.act(ui::Action::Tool(ui::Tool::Soil));
        let from = aim(&lab, x, y);
        lab.set_cursor(Some(from));
        lab.press(from.0, from.1);
        // One jump, wider than the brush, so the two ends cannot touch.
        let to = aim(&lab, x + 60, y);
        lab.set_cursor(Some(to));
        lab.drag(to.0, to.1);
        lab.release(to.0, to.1);
        let midpoint = lab.world.get(x + 30, y);
        assert_eq!(
            midpoint.material, soil,
            "the drag left a gap: a stroke must be a capsule, not two dabs"
        );
    }

    /// **A founded colony must not draw a line across the bed.**
    ///
    /// Measured as contrast against the ground it sits on rather than as a
    /// colour equality: the claim is *"you cannot see a stripe"*, and a test
    /// that asserted three specific bytes would be green for any repaint and
    /// would go red for a legitimate retune of `packedsoil`.
    ///
    /// The sensitivity half is the shipped `nest.ron` palette, checked in the
    /// same test: a pale (196,168,120) against soil's (64,46,34) is a
    /// separation of 132, and the assertion below must be one this cannot
    /// pass — otherwise it is measuring nothing.
    #[test]
    fn a_founded_colony_does_not_draw_a_pale_line_across_the_bed() {
        use crate::sim::material::MaterialId;
        let lab = bench();
        let luma = |id: MaterialId| -> f32 {
            let p = &lab.world.materials.get(id).palette;
            p.iter()
                .map(|c| 0.299 * c[0] as f32 + 0.587 * c[1] as f32 + 0.114 * c[2] as f32)
                .sum::<f32>()
                / p.len().max(1) as f32
        };
        let nest = lab.world.materials.id_of("nest").expect("nest");
        let soil = lab.world.materials.id_of("soil").expect("soil");
        let gap = (luma(nest) - luma(soil)).abs();
        assert!(gap < 24.0, "nest stands {gap:.0} luma off the soil it is painted on");

        // The control: the shipped palette, which is what this replaces.
        let shipped = [[196u8, 168, 120, 255], [184, 156, 110, 255], [206, 180, 132, 255]];
        let shipped_luma = shipped
            .iter()
            .map(|c| 0.299 * c[0] as f32 + 0.587 * c[1] as f32 + 0.114 * c[2] as f32)
            .sum::<f32>()
            / shipped.len() as f32;
        assert!(
            (shipped_luma - luma(soil)).abs() > 24.0,
            "the control does not fail the assertion, so the assertion measures nothing"
        );
    }

    /// **The font cannot draw everything, and what it cannot draw it draws as
    /// nothing.** `[`/`]`, then `_`/`<`/`>`, then `;`/`'` have each shipped
    /// blank in this engine's UI for as long as they were bound. This is the
    /// cheap standing check, not a discipline.
    #[test]
    fn every_help_line_is_drawable() {
        for line in super::HELP {
            for c in line.chars() {
                assert!(crate::hud::has_glyph(c), "the HUD font has no glyph for {c:?} in {line:?}");
            }
        }
    }

    /// **...and the page it draws them on has to fit on the screen.**
    ///
    /// The same arithmetic `draw_help` uses, because the failure it catches is
    /// silent: `draw_text` clips, so a key list one line too long loses its
    /// last line and looks exactly like a key list that never had it. The
    /// window is 512x320 and the page is already within ten rows of the
    /// bottom, so this is the guard that says the next line added has to buy
    /// its place by taking one out.
    #[test]
    fn the_help_page_fits_on_the_screen() {
        let w = super::HELP.iter().map(|l| crate::hud::text_width(l)).max().unwrap_or(0);
        let (bw, bh) = (w + 24, super::HELP.len() as i32 * 10 + 20);
        assert!(bw <= WIDTH as i32, "the key list is {bw} px wide and the window is {WIDTH}");
        assert!(bh <= HEIGHT as i32, "the key list is {bh} px tall and the window is {HEIGHT}");
    }
}
