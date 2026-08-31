//! **The biosphere page — what the box is doing, in numbers.**
//!
//! §8.9 of the design guide is the uncomfortable one this answers:
//! *"What does the player watch during a Running phase?"* Measured by
//! `creature_look` and `motion_look`, an ant is **two dark cells at play
//! zoom, findable only because it moves** — and a dead one has stopped
//! moving, so it is unfindable by the very channel that finds a live one. A
//! phase whose whole content is *watch evolution happen* has a legibility
//! problem this repo has already measured and not solved, and a page of
//! numbers beside the box is the cheapest half of the answer.
//!
//! **Read the count next to the picture, never the picture alone** —
//! `CLAUDE.md`'s standing rule, learned when a collapse rendered as coherent
//! falling slabs was read as *"chunks are working"* while the harness's own
//! body count was zero for the whole run. The same trap is live here: a box
//! full of green that is not reproducing looks exactly like a box full of
//! green that is.
//!
//! ## What this is not
//!
//! It is **not** `App`'s colony panel with plants bolted on. That page
//! (`SHIFT+Y`, `Reports/lanes/creature-lane-g.md`) is the model for the
//! *shape* — distributions rather than means, rates rather than totals,
//! every row able to explain itself — and its reasoning is reused wherever
//! it transfers. Four things do not transfer, and each is a decision made
//! here rather than inherited:
//!
//! 1. **A lab is a biosphere, not a colony.** The commonest organism in this
//!    box is a plant, and the plants are what breed. Every headline is
//!    two-sided.
//! 2. **The history is the experiment, so it is kept whether the page is
//!    open or not.** The colony panel samples only while open, which is free
//!    and right for a sandbox you glance at. Here a Running phase *is* the
//!    content: a player who fast-forwards 45,000 frames with the page shut
//!    and then opens it must not be shown an empty strip. Costed below.
//! 3. **The strip covers the whole run, not the last few seconds.** It
//!    decimates instead of scrolling — see [`Stats::observe`].
//! 4. **The sample interval is in *world* frames, never displayed ones.**
//!    One displayed frame is 1 tick at 1x and ~256 at the top of the dial
//!    (`time::PRESETS`), so a per-frame sample would draw a strip whose
//!    x-axis is the speed dial.
//!
//! ## The oscillator
//!
//! `CLAUDE.md`: *a designed oscillator must be divided out of every number
//! it reaches.* The lab removes one of them by construction — `scene`'s box
//! holds the sky at noon and pins the weather clear under a stone ceiling,
//! so there is no day and no rain to alias into a census. That is worth
//! stating rather than assuming, because it is the reason this page's rate
//! window is not the colony panel's day-length one: measured on the standard
//! bed (see `examples/labstats.rs --control=steady`), nothing here rides a
//! 3,600-frame period. The window is set by what a *reader* needs instead,
//! and every row that quotes a rate also quotes the span it was measured
//! over, so a short window can never be mistaken for a long one.

use crate::sim::organism;
use crate::sim::world::World;

/// Frames between censuses, before any decimation. At 1x this is a refresh
/// twice a second; at 256x the dial outruns it and the page samples once per
/// displayed frame, which is the honest thing — the interval is a *minimum*
/// spacing in world time, not a promise of one.
pub const SAMPLE_INTERVAL: u64 = 30;

/// Samples the strip holds. When it fills, [`Stats::observe`] throws away
/// every other one and doubles the interval, so the strip always spans the
/// **whole run** at declining resolution rather than scrolling a fixed
/// window off the left.
///
/// This is the one place this page deliberately departs from the colony
/// panel's ring, and the reason is the dial: a Running phase at 256x puts
/// 45,000 world frames — the herb's measured five generations — through in
/// about twelve seconds of watching. A 3,840-frame window would show 8% of
/// that and call it the population trend.
pub const HISTORY: usize = 160;

/// The window rates are differenced over, in world frames.
///
/// **Not a day, and that is deliberate.** The colony panel's window is one
/// `field::DAY_NIGHT_PERIOD_FRAMES` because every rate an outdoor colony
/// produces rides the light. The lab holds its light (`scene::LabBox::noon`)
/// so that cycle is gone, and a window sized to a cycle that is not running
/// only makes the page slower to react. 1,800 frames is half a nominal day
/// and about 30 seconds at 1x. Every row that quotes a rate also prints the
/// **span it actually got**, which is what makes this a display choice
/// rather than a claim.
pub const RATE_WINDOW: u64 = 1_800;

/// Frames between refreshes of the standing-food set — ten censuses.
///
/// **The census is two costs, not one, and only one of them scales with the
/// stand.** Everything the page draws is `O(live organisms)` with an `O(1)`
/// read each, except the set of materials standing in the box, which needs
/// one `World::get` per *cell*. Measured on a settled bed by `labstats
/// control=cost`:
///
/// | | organisms | cells | one census |
/// |---|---|---|---|
/// | standard bed, 10,800 frames | 53 | 774 | **0.020 ms** |
/// | 48 founders, 45,000 frames | 102 | 2,205 | **0.327 ms** |
///
/// That is ~150 ns per living cell, and the guide warns the lab will reach
/// **1,812–2,503 live organisms** — where the same walk would be several
/// milliseconds *in one frame*, and at the top of `time`'s dial a census
/// falls on every displayed frame. So the material set, which changes only
/// when a new **kind** of tissue first appears anywhere in the box, is
/// refreshed on its own slower clock and cached between times. The populations
/// and the biomass are not: those are the numbers a player watches move.
pub const STANDING_INTERVAL: u64 = 300;

/// Generation buckets on the histogram: 0, 1, 2, 3, 4, 5, 6, 7+.
///
/// **Eight, and the last one is open-ended.** A histogram that dropped
/// everything past its axis would report a population that had got *further*
/// than the page can draw as a population that had vanished — the failure
/// `CLAUDE.md` calls a number that is arithmetically correct and about the
/// wrong thing.
pub const GEN_BUCKETS: usize = 8;

/// A distribution reduced to the three numbers that fit on one row.
///
/// **Three, not a mean**, and that is the whole reason this type exists —
/// `CLAUDE.md`'s first law, and the colony panel's `Spread` for the same
/// reason one level down. A stand of eight mature herbs and a stand of two
/// hundred seedlings can hold the same mean size and are not the same stand.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Spread {
    pub low: f32,
    pub mid: f32,
    pub high: f32,
}

impl Spread {
    /// `values` is sorted in place; `mid` is the lower median, which is what
    /// an integer-indexed order statistic gives and is honest for an even
    /// population rather than inventing a value nobody holds.
    pub fn of(values: &mut [f32]) -> Option<Self> {
        if values.is_empty() {
            return None;
        }
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Some(Self { low: values[0], mid: values[values.len() / 2], high: values[values.len() - 1] })
    }
}

/// One point on the strips, and the counter block the rates are differenced
/// from.
///
/// The counters are stored **beside** the populations rather than
/// differenced as they are taken, because the window they are read over is
/// decided at draw time and changes as the ring decimates.
#[derive(Clone, Copy, Default, Debug)]
pub struct Sample {
    pub frame: u64,
    pub plants: u32,
    pub animals: u32,
    pub plant_cells: u32,
    pub animal_cells: u32,
    /// `World::germinations` — every seed that became a growing shoot.
    pub germinations: u64,
    /// `World::fate_mutation_rolls` — every seed *borne by a parent*, i.e.
    /// every plant birth that had heredity in it. See [`Census::seeds_borne`].
    pub seeds_borne: u64,
    pub births: u64,
    pub deaths: u64,
}

/// **What one birth needs, and what this animal can ever hold.**
///
/// PR #162's finding, put on screen: what decides whether a species breeds
/// is one number, `ceiling - bar`, and the shipped ant sits far below zero.
/// A player looking at `BORN 0` cannot tell *my colony is failing* from *my
/// colony structurally cannot reproduce*, and those want opposite responses.
#[derive(Clone, Copy, Debug)]
pub struct BreedMargin {
    /// **Net joules banked per decision while fed** — what digesting the best
    /// mouthful this gut can draw pays, less what standing there costs.
    ///
    /// **This replaced a ceiling, and the change is not cosmetic.** The old
    /// row read `hunger_fraction * start_energy + one mouthful`: an animal
    /// stopped eating once comfortable, so its bank had a hard roof and a
    /// colony under it could not breed at any amount of food. With a crop
    /// there is no roof — an animal digests at a rate and the bank is limited
    /// by supply and by time instead. So the question a player needs answered
    /// stops being "can it ever get there" and becomes **"how long does it
    /// take"**, which is a number they can act on.
    ///
    /// Excludes the synapse tax, which depends on how many connections are
    /// live this tick and is small beside the two terms here; the figure is
    /// therefore mildly optimistic and is a bound rather than a prediction.
    pub gain_per_tick: f32,
    /// `creature::reproduce_at` — the bank a **founder** of this species
    /// must reach to bud.
    ///
    /// **The species' ancestral bar, not the population's, and that is a
    /// choice rather than an oversight.** `TRAIT_REPRODUCE_AT` made this a
    /// gene, so once anything has bred there is no single number here: an
    /// eager lineage buds at `birth_cost + 1` and a patient one at twice
    /// the authored threshold. The row this feeds answers *"what does this
    /// animal need"* for a player looking at a box, and the honest answer
    /// to that is the ancestral value the founders were placed with. The
    /// spread across the living population is a different question and
    /// wants an allele histogram, which is `genome_drift`'s shape and not a
    /// row on this page.
    pub bar: f32,
    /// The best single mouthful, priced over the **material table**.
    pub best_mouthful: f32,
}

impl BreedMargin {
    /// Decisions of uninterrupted feeding to reach the bar from empty.
    /// `None` when the animal cannot out-eat its own metabolism, which is the
    /// case the old negative margin was really about.
    pub fn ticks_to_bar(&self) -> Option<f32> {
        (self.gain_per_tick > 0.0).then(|| self.bar / self.gain_per_tick)
    }
}

/// Everything the page draws that has to be counted rather than read off a
/// counter — one walk of the live organism table, cached between censuses.
///
/// **Both kingdoms, and split at every line.** The colony panel's census
/// discards every plant in the world on purpose (a panel that counted a
/// forest as population would be the "ask what your number counts" failure
/// in its purest form). This page is the other decision: the box *is* a
/// biosphere, so the split is carried rather than one side dropped.
/// `Default` is the all-zero census — a box nothing has happened in.
///
/// Derived rather than hand-written so a field added later starts at zero
/// instead of silently keeping whatever a hand-written impl last knew about.
/// It exists for tests that need a census-shaped value without a world to
/// take one from; nothing in the running game constructs one this way, and
/// `take_census` remains the only path that produces a *real* one.
#[derive(Clone, Debug, Default)]
pub struct Census {
    pub frame: u64,
    pub plants: usize,
    pub animals: usize,
    /// Living cells, split. **This is also the frame-time dial** — the guide
    /// is explicit that *"how many growth beds is your lab running IS the
    /// frame-time dial"*, so the budget belongs on the page the player reads
    /// rather than in a settings menu.
    pub plant_cells: usize,
    pub animal_cells: usize,
    /// Plants that are dead and rotting — `OrganismState::senescent`.
    ///
    /// On the page because of `CLAUDE.md`'s first law: a plant that is either
    /// thriving or gone has the same defect the rubble did, and the engine's
    /// answer is that death is *graded*, carried out by `rot_remains` at the
    /// species half-life. A page with no row for it would report the graded
    /// death as a live plant right up until the last cell went.
    pub senescent: usize,
    /// Cells per living plant, as an order statistic.
    pub plant_size: Spread,
    /// Seeds set by plants **that are alive now**. Not cumulative: it drops
    /// when a plant dies, which is why the cumulative pair below exists.
    pub seeds_standing: u32,
    /// Every seed borne by a parent, cumulatively — `World::
    /// fate_mutation_rolls`, which `plant::bear_seed_at` increments once per
    /// inherited birth immediately before the fate-mutation draw.
    ///
    /// **Borrowed from a counter named for something else, deliberately and
    /// with the risk stated.** It is the only monotone world-level count of
    /// *plant births* that exists; `seeds_standing` is a sum over the living
    /// and therefore falls, and `germinations` counts the far side of the
    /// seed's life rather than the near one. Cross-checked against
    /// `seeds_standing` in `examples/labstats.rs`, which is what would catch
    /// the increment moving.
    pub seeds_borne: u64,
    /// Every germination — `World::germinations`. The far-side effect
    /// counter for `seeds_borne`, which is exactly the pairing `CLAUDE.md`
    /// asks for: a seed set and a seed that became a plant are different
    /// events and only both together say the loop closed.
    pub germinations: u64,
    /// Deepest lineage reached, each kingdom. **The number the whole game is
    /// about**, and the one nobody could see.
    pub plant_generation: u16,
    pub animal_generation: u16,
    /// How the living population distributes over generations, both kingdoms
    /// together, last bucket open-ended.
    pub generations: [u32; GEN_BUCKETS],
    /// Distinct founder lines still going, and the share held by the largest.
    pub lineages: usize,
    pub top_lineage: f32,
    /// The dominant animal species' name and its economy, when there is one.
    pub animal_species: Option<String>,
    pub animal_energy: Spread,
    pub animal_hungry: usize,
    pub breed: Option<BreedMargin>,
    /// `World::organism_slot_usage`, `organism_slot_high_water` and
    /// `organisms_refused`. **A population that has silently hit the ceiling
    /// and one that has stabilised look identical**, and only the refusal
    /// count separates them.
    pub slots_used: usize,
    pub slots_high_water: usize,
    pub slots_ceiling: usize,
    pub refused: u64,
}

impl Census {
    /// Living cells of both kinds — the biomass headline, and the lab's
    /// performance budget in the same number.
    pub fn biomass(&self) -> usize {
        self.plant_cells + self.animal_cells
    }
}

/// What one drawn row of the page *is*. Each variant knows its own height
/// and nothing else does — the page builds its whole list before it paints
/// anything so that it can size its own border to what it has to say, which
/// is the colony panel's construction and its reasoning transfers whole.
enum Body {
    Text(String, [u8; 4]),
    /// Breathing space between sections.
    Gap,
    /// A population strip: `(series, colour, label)`.
    Strip(Series, [u8; 4], &'static str),
    /// The generation histogram.
    Generations,
    /// A 0..1 gauge with a label to its right.
    Gauge(f32, String, [u8; 4]),
}

/// Which series a strip draws. Two, because the two kingdoms have wildly
/// different scales — 2,500 herbs against 50 ants — and one shared axis
/// would draw the colony as a flat line on the floor whatever it did.
#[derive(Clone, Copy)]
enum Series {
    Plants,
    Animals,
}

/// One drawn row, and what it means.
///
/// **The note is not decoration**, and it is here for the reason the colony
/// panel's is: the page is dense and every row is compressed to fit 5x7
/// glyphs, so a note says what the row means **and carries the numbers that
/// did not fit**. Reachable today through [`Stats::draw_at`]; see that
/// function for the one line in `lab::mod` that turns it on.
struct Row {
    body: Body,
    note: String,
}

impl Row {
    fn text(text: impl Into<String>, colour: [u8; 4], note: impl Into<String>) -> Self {
        Self { body: Body::Text(text.into(), colour), note: note.into() }
    }
    fn gap() -> Self {
        Self { body: Body::Gap, note: String::new() }
    }
    fn height(&self) -> i32 {
        match self.body {
            Body::Text(..) => LINE,
            Body::Gap => 5,
            Body::Strip(..) => 20,
            // 26 rather than the histogram's own 15: the axis labels sit
            // *under* the bars and a row that measured only the bars would
            // let `0 1 2 3 4 5 6 7+` overprint the next row's text.
            Body::Generations => 26,
            Body::Gauge(..) => LINE + 3,
        }
    }
}

/// The page's left edge, top edge and width. **Its height is not here**,
/// because the page is sized to whatever it has to say: the refusal row and
/// the breeding margin appear only when they have something to report.
///
/// Drawn down the **right**, unlike the colony panel, because `lab::time`
/// already owns the top-left corner — the phase, the dial and the frame
/// number are painted at `(4, 4)` and `(4, 14)`, and a page starting there
/// would sit on top of the one readout that says whether the box is even
/// running.
const RECT: (i32, i32, i32) = (super::WIDTH as i32 - 272, 10, 262);
/// Pixels between one drawn row's top and the next.
const LINE: i32 = crate::hud::GLYPH_HEIGHT + 2;
/// The strip between the title and the first row.
const HEADER: i32 = 23;

const WHITE: [u8; 4] = [225, 228, 235, 255];
const DIM: [u8; 4] = [140, 146, 158, 255];
const FAINT: [u8; 4] = [96, 102, 116, 255];
const HEADING: [u8; 4] = [150, 200, 160, 255];
/// The plant series, and anything that reads as "fine" at a glance.
const GREEN: [u8; 4] = [120, 200, 130, 255];
/// The animal series. Blue rather than a second green, because the whole
/// point of the two strips is that they are two populations.
const BLUE: [u8; 4] = [110, 175, 235, 255];
/// **Not red**: on this page amber means *worth knowing*, not *alarm*. A
/// page that alarms at the ordinary teaches its reader to ignore it, and in
/// a lab a hungry forager and a stand of annuals dying back are both
/// ordinary.
const AMBER: [u8; 4] = [220, 170, 90, 255];
/// Separators, axes, and the empty half of a gauge.
const RULE: [u8; 4] = [40, 52, 72, 255];

/// The census, the run's history, and the page that draws them.
pub struct Stats {
    show: bool,
    census: Option<Census>,
    history: Vec<Sample>,
    /// World frames between samples. Doubles each time `history` fills.
    interval: u64,
    /// The distinct materials of the living cells in the box, and the frame
    /// that was last read at. See [`STANDING_INTERVAL`] for why this is not
    /// refreshed every census.
    standing: Vec<crate::sim::material::MaterialId>,
    standing_at: Option<u64>,
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}

impl Stats {
    pub fn new() -> Self {
        Self {
            show: true,
            census: None,
            history: Vec::new(),
            interval: SAMPLE_INTERVAL,
            standing: Vec::new(),
            standing_at: None,
        }
    }

    /// **A `Stats` holding a run that happened somewhere else.**
    ///
    /// The batch runs its chambers on a worker thread with their own `Stats`;
    /// when one is adopted into the rack its page must show the run that
    /// actually happened rather than an empty strip that starts from the
    /// moment you walked in.
    ///
    /// `interval` is restored from the history rather than reset, because the
    /// ring **decimates**: a long run's samples are hundreds of frames apart
    /// by the end, and a `Stats` that carried them at the fresh interval
    /// would decimate again immediately and halve a series it was handed
    /// intact.
    pub fn restored(census: Census, history: Vec<Sample>) -> Self {
        let interval = match history.as_slice() {
            [.., a, b] => (b.frame - a.frame).max(SAMPLE_INTERVAL),
            _ => SAMPLE_INTERVAL,
        };
        Self {
            standing_at: Some(census.frame),
            census: Some(census),
            history,
            interval,
            ..Self::new()
        }
    }

    pub fn showing(&self) -> bool {
        self.show
    }

    pub fn toggle(&mut self) {
        self.show = !self.show;
    }

    /// The current census, for a harness that wants the numbers without a
    /// framebuffer.
    pub fn census(&self) -> Option<&Census> {
        self.census.as_ref()
    }

    /// The whole run so far, oldest first.
    pub fn history(&self) -> &[Sample] {
        &self.history
    }

    /// Called once per displayed frame, after the ticks have run.
    ///
    /// **Sampling is keyed on `world.frame`, never on being called**, which
    /// is what makes the strip a picture of simulated time rather than of the
    /// speed dial: one call is one tick at 1x and up to 256 at the top of the
    /// ladder.
    ///
    /// **It runs whether or not the page is showing.** That is the opposite
    /// of the colony panel's choice and it is deliberate — a Running phase is
    /// the lab's whole content, and a player who fast-forwards an experiment
    /// with the page shut must not be handed an empty strip when they open it
    /// to see what happened. The price is one census per `interval` world
    /// frames; `examples/labstats.rs --control=cost` reports it.
    pub fn observe(&mut self, world: &World) {
        let frame = world.frame;
        let due = match self.history.last() {
            None => true,
            Some(last) => frame >= last.frame.saturating_add(self.interval),
        };
        if !due && self.census.is_some() {
            return;
        }
        let refresh = self.standing_at.is_none_or(|at| frame >= at.saturating_add(STANDING_INTERVAL));
        if refresh {
            self.standing_at = Some(frame);
        }
        let census = take_census(world, &mut self.standing, refresh);
        if due {
            // **Decimate rather than scroll.** Dropping the oldest sample
            // would make the strip a moving window on the last few seconds,
            // which is exactly the thing a fast-forwarded experiment cannot
            // be read from. Halving the resolution instead keeps frame 0 on
            // the left for the whole run.
            if self.history.len() >= HISTORY {
                let mut keep: Vec<Sample> = self.history.iter().copied().step_by(2).collect();
                // The newest sample is the one the eye is on; keep it even
                // when the parity drops it, or the strip's right-hand end
                // jumps backwards every time the ring halves.
                if let Some(last) = self.history.last() {
                    if keep.last().map(|s| s.frame) != Some(last.frame) {
                        keep.push(*last);
                    }
                }
                self.history = keep;
                self.interval = self.interval.saturating_mul(2);
            }
            self.history.push(Sample {
                frame,
                plants: census.plants as u32,
                animals: census.animals as u32,
                plant_cells: census.plant_cells as u32,
                animal_cells: census.animal_cells as u32,
                germinations: census.germinations,
                seeds_borne: census.seeds_borne,
                births: world.creature_stats.births,
                deaths: world.creature_stats.deaths,
            });
        }
        self.census = Some(census);
    }

    pub fn draw(&self, frame: &mut [u8], world: &World) {
        self.draw_at(frame, world, None);
    }

    /// The page, with the row under `cursor` explaining itself beside it.
    ///
    /// **This is what `Lab::draw` calls**, passing the cursor it already
    /// holds for the renderer. [`Stats::draw`] is the cursor-less form, kept
    /// for harnesses (`examples/labstats.rs`) that render the page with no
    /// pointer in the world.
    pub fn draw_at(&self, frame: &mut [u8], world: &World, cursor: Option<(i32, i32)>) {
        if !self.show {
            return;
        }
        const PANEL: [u8; 4] = [10, 10, 16, 255];
        const PANEL_ALPHA: f32 = 0.82;
        const TITLE: [u8; 4] = [255, 220, 100, 255];
        const ACCENT: [u8; 4] = [90, 170, 240, 255];
        let (w, h) = (super::WIDTH, super::HEIGHT);

        let rows = self.rows(world);
        let (left, top, right, bottom) = self.rect(world);

        for y in top..bottom {
            for x in left..right {
                crate::render::blend(frame, w, h, x, y, PANEL, PANEL_ALPHA);
            }
        }
        for x in left..right {
            crate::render::put(frame, w, h, x, top, ACCENT);
            crate::render::put(frame, w, h, x, bottom - 1, ACCENT);
        }
        for y in top..bottom {
            crate::render::put(frame, w, h, left, y, ACCENT);
            crate::render::put(frame, w, h, right - 1, y, ACCENT);
        }

        let pad = left + 8;
        text(frame, pad, top + 6, "BIOSPHERE", TITLE);
        let close = "TAB CLOSE";
        text(frame, right - 8 - crate::hud::text_width(close), top + 6, close, FAINT);
        for x in left + 1..right - 1 {
            crate::render::put(frame, w, h, x, top + 17, RULE);
        }

        let mut y = top + HEADER;
        let mut hovered: Option<&str> = None;
        for row in &rows {
            // The row under the cursor, found while painting rather than by a
            // second pass over the same arithmetic — two copies of a layout is
            // how a marker and the thing it marks come to disagree.
            if let Some((cx, cy)) = cursor {
                if !row.note.is_empty()
                    && (left..right).contains(&cx)
                    && (y..y + row.height()).contains(&cy)
                {
                    hovered = Some(&row.note);
                }
            }
            match &row.body {
                Body::Gap => {}
                Body::Text(s, colour) => text(frame, pad, y, s, *colour),
                Body::Strip(series, colour, label) => {
                    self.draw_strip(frame, pad, y, *series, *colour, label)
                }
                Body::Generations => {
                    if let Some(census) = &self.census {
                        draw_generations(frame, census, pad, y);
                    }
                }
                Body::Gauge(fill, label, colour) => {
                    draw_gauge(frame, pad, y + 4, 108, *fill, *colour);
                    text(frame, pad + 116, y + 3, label, FAINT);
                }
            }
            y += row.height();
        }
        if let (Some(note), Some(at)) = (hovered, cursor) {
            draw_note(frame, note, at);
        }
    }

    /// The box the page will occupy, sized to what it currently has to say.
    ///
    /// One function so the border, the paint and
    /// `the_biosphere_page_stays_inside_its_own_border` all measure the same
    /// rectangle — a test against a hand-copied literal is a test of the
    /// literal.
    fn rect(&self, world: &World) -> (i32, i32, i32, i32) {
        let (left, top, width) = RECT;
        let content: i32 = self.rows(world).iter().map(Row::height).sum();
        // **Stops at the bar, not at the window.** Clamping to `HEIGHT - 6`
        // ran the page's last rows *under* the control bar, and stats draws
        // after it, so the page painted over the buttons -- a control you can
        // see but cannot reach, which is worse than one plainly behind a
        // page. Found by the interface lane looking at a capture; no test saw
        // it, because both surfaces drew exactly what they were asked to.
        (left, top, left + width, (top + HEADER + content + 6).min(super::ui::bar_top() - 6))
    }

    /// The oldest sample within [`RATE_WINDOW`] frames of now, the newest
    /// sample, and the span between them — or `None` until two readings
    /// exist.
    ///
    /// **The span is returned rather than assumed**, because the ring
    /// decimates: after the first halving the samples are 60 frames apart,
    /// after the fourth 480, and a rate quoted against a nominal window that
    /// the samples cannot actually resolve is a number about the wrong thing.
    fn rate_window(&self) -> Option<(f64, &Sample, &Sample)> {
        let last = self.history.last()?;
        let cutoff = last.frame.saturating_sub(RATE_WINDOW);
        // The newest sample at or before the cutoff, so the window is at
        // least `RATE_WINDOW` and never silently shorter.
        let first = self
            .history
            .iter()
            .rev()
            .find(|s| s.frame <= cutoff)
            .unwrap_or_else(|| self.history.first().expect("last() succeeded"));
        let span = last.frame.checked_sub(first.frame)?;
        (span > 0).then_some((span as f64, first, last))
    }

    /// Everything the page says, in order, before any of it is painted.
    ///
    /// Split out so the page can measure itself, so a hover can find the row
    /// under the cursor, and so a test — and `labstats`' text dump — can read
    /// what it would say without a framebuffer. That last one is not a
    /// convenience: `Reports/lanes/creature-lane-g.md` records an hour spent
    /// hunting a simulation bug that was a `3` one glyph wide in a downscaled
    /// screenshot. **Look at the picture for what and where; read the text
    /// for how much.**
    fn rows(&self, world: &World) -> Vec<Row> {
        let mut rows = Vec::new();
        let Some(census) = &self.census else {
            rows.push(Row::text(
                "NOTHING CENSUSED YET",
                WHITE,
                "THE PAGE TAKES ITS FIRST READING ONE TICK AFTER THE BOX STARTS RUNNING.",
            ));
            return rows;
        };

        // --- is anything alive, and is it going up or down -----------------
        if census.plants == 0 && census.animals == 0 {
            rows.push(Row::text(
                "THE BOX IS EMPTY",
                AMBER,
                "NO PLANT AND NO ANIMAL IS ALIVE IN THIS BOX. EVERY FIGURE BELOW WOULD BE A ZERO ABOUT NOTHING, SO THEY ARE NOT DRAWN.",
            ));
            rows.push(Row::gap());
            rows.push(Row::text(format!("SEEDS BORNE {}", census.seeds_borne), DIM, ""));
            rows.push(Row::text(format!("SPROUTED    {}", census.germinations), DIM, ""));
            rows.push(Row::text(
                format!("ANIMALS BORN {}  DIED {}", world.creature_stats.births, world.creature_stats.deaths),
                DIM,
                "TOTALS SINCE THE BOX WAS BUILT. THEY STAND EVEN WHEN NOTHING IS LEFT ALIVE, WHICH IS HOW YOU TELL A BOX THAT NEVER STARTED FROM ONE THAT DIED.",
            ));
            return rows;
        }

        let (word, colour) = self.trend_word();
        rows.push(Row::text(
            format!("PLANTS {}  ANIMALS {}{}", census.plants, census.animals, word),
            colour,
            format!(
                "EVERY LIVING PLANT AND EVERY LIVING ANIMAL IN THE BOX. THE WORD IS THE TREND OF THE TWO STRIPS BELOW OVER {} READINGS SPANNING {} FRAMES.",
                self.history.len(),
                self.span()
            ),
        ));
        rows.push(Row {
            body: Body::Strip(Series::Plants, GREEN, "PLANTS"),
            note: format!(
                "EVERY LIVING PLANT, FROM THE FIRST FRAME OF THE RUN TO NOW -- NOT A MOVING WINDOW. READINGS ARE {} FRAMES APART AND THE STRIP HALVES ITS OWN DETAIL WHEN IT FILLS, SO IT ALWAYS SHOWS THE WHOLE EXPERIMENT.",
                self.interval
            ),
        });
        rows.push(Row {
            body: Body::Strip(Series::Animals, BLUE, "ANIMALS"),
            note: "EVERY LIVING ANIMAL, ON ITS OWN AXIS. TWO STRIPS RATHER THAN ONE BECAUSE A BOX HOLDS THOUSANDS OF PLANTS AND TENS OF ANIMALS, AND A SHARED AXIS WOULD DRAW THE COLONY FLAT ON THE FLOOR WHATEVER IT DID."
                .to_string(),
        });
        rows.push(Row::text(
            format!("BIOMASS {} CELLS  {} PLANT", census.biomass(), census.plant_cells),
            WHITE,
            format!(
                "LIVING CELLS: {} PLANT AND {} ANIMAL. THIS IS ALSO WHAT THE BOX COSTS TO RUN -- HOW MUCH IS GROWING IN HERE IS THE SPEED DIAL, NOT A SETTING SOMEWHERE ELSE.",
                census.plant_cells, census.animal_cells
            ),
        ));
        rows.push(Row::text(
            format!(
                "PLANT SIZE {:.0} / {:.0} / {:.0} CELLS",
                census.plant_size.low, census.plant_size.mid, census.plant_size.high
            ),
            DIM,
            "THE SMALLEST PLANT, THE MIDDLE ONE AND THE BIGGEST. EIGHT GROWN HERBS AND TWO HUNDRED SEEDLINGS CAN AVERAGE THE SAME AND ARE NOT THE SAME STAND.",
        ));
        if census.senescent > 0 {
            rows.push(Row::text(
                format!("DYING BACK {}", census.senescent),
                AMBER,
                "PLANTS THAT ARE DEAD AND STILL ROTTING WHERE THEY STAND. THEY ARE NOT COUNTED AS LIVING ABOVE. A HERB DIES BACK AFTER IT FRUITS, SO SOME OF THIS IS THE ORDINARY END OF AN ANNUAL RATHER THAN TROUBLE.",
            ));
        }
        rows.push(Row::gap());

        // --- did anything actually happen ----------------------------------
        // **The discrete events, and the whole value of this page.** A box
        // full of green that is not reproducing looks exactly like one that
        // is (`CLAUDE.md`: "did it fire at all" needs a counter, not a
        // picture).
        rows.push(Row::text(
            "REPRODUCTION",
            HEADING,
            "THE THINGS THAT EITHER HAPPEN OR DO NOT. A PICTURE OF THE BOX CANNOT SHOW ANY OF THEM.",
        ));
        rows.push(Row::text(
            format!("SEEDS BORNE {}   SPROUTED {}", census.seeds_borne, census.germinations),
            if census.germinations > 0 { GREEN } else { DIM },
            format!(
                "TOTALS SINCE THE BOX WAS BUILT. BORNE IS SEEDS A PARENT PLANT PAID FOR; SPROUTED IS SEEDS THAT BECAME A NEW PLANT. BOTH ARE NEEDED -- SEEDS THAT NEVER SPROUT AND A STAND THAT NEVER SETS SEED BOTH LOOK LIKE A GREEN BOX. {} SEEDS ARE STANDING TO THE CREDIT OF PLANTS ALIVE NOW.",
                census.seeds_standing
            ),
        ));
        rows.push(Row::text(
            format!("ANIMALS BORN {}   DIED {}", world.creature_stats.births, world.creature_stats.deaths),
            if world.creature_stats.births > 0 { GREEN } else { DIM },
            format!(
                "ANIMALS AN ANIMAL PAID FOR OUT OF ITS OWN BODY, AND ANIMALS THAT HAVE DIED. {} WERE PLACED BY HAND OR BY THE BOX ITSELF AND ARE NOT BIRTHS.",
                world.creature_stats.spawned
            ),
        ));
        rows.push(match self.rate_window() {
            Some((span, first, last)) => {
                let per_k = |a: u64, b: u64| (b.saturating_sub(a)) as f64 * 1000.0 / span;
                Row::text(
                    format!(
                        "PER 1K  SPROUT {:.1}  BORN {:.1}  DIED {:.1}",
                        per_k(first.germinations, last.germinations),
                        per_k(first.births, last.births),
                        per_k(first.deaths, last.deaths)
                    ),
                    DIM,
                    format!(
                        "OVER THE LAST {span:.0} FRAMES: {} SPROUTED, {} ANIMALS BORN, {} DIED. A TOTAL THAT ONLY CLIMBS CANNOT SAY WHETHER ANYTHING IS HAPPENING NOW; THIS CAN.",
                        last.germinations.saturating_sub(first.germinations),
                        last.births.saturating_sub(first.births),
                        last.deaths.saturating_sub(first.deaths)
                    ),
                )
            }
            None => Row::text(
                "PER 1K  MEASURING...",
                FAINT,
                format!("A RATE NEEDS TWO READINGS {RATE_WINDOW} FRAMES APART. RUN THE BOX AND THIS FILLS IN."),
            ),
        });
        // **The two silent refusals**, shown only when they have happened. A
        // birth the terrain refused and one the engine's address space
        // refused are both invisible in `births`, and both read as "nothing
        // is breeding" -- the same reading as a population too poor to try,
        // wanting the opposite fix.
        if world.creature_stats.births_denied_no_space > 0 || census.refused > 0 {
            rows.push(Row::text(
                format!("REFUSED  NO ROOM {}  NO SLOT {}", world.creature_stats.births_denied_no_space, census.refused),
                AMBER,
                "SOMETHING COULD AFFORD A CHILD AND DID NOT GET ONE. NO ROOM MEANS THERE WAS NOWHERE BESIDE THE PARENT TO PUT A BODY. NO SLOT MEANS THE BOX IS FULL -- SEE THE LIVING THINGS GAUGE BELOW.",
            ));
        }
        // **The margin, and why zero births can be the correct answer.**
        // PR #162: what decides a birth is `ceiling - bar`, the shipped ant
        // sits far under it, and every negative margin gave exactly zero
        // births across twelve seeds. Without this row a player reads BORN 0
        // as a colony failing, when it is a colony that structurally cannot
        // reproduce -- and those want opposite responses.
        if let (Some(breed), Some(name)) = (census.breed, census.animal_species.as_ref()) {
            let ticks = breed.ticks_to_bar();
            rows.push(Row::text(
                match ticks {
                    Some(t) => format!("{name} NEEDS {t:.0} STEPS OF FEEDING PER CHILD"),
                    None => format!("{name} CANNOT OUT-EAT ITS OWN UPKEEP"),
                },
                if ticks.is_some() { GREEN } else { AMBER },
                format!(
                    "AN ANT DIGESTS WHAT IT CARRIES AS IT WALKS, SO ITS STORE HAS NO CEILING -- WHAT LIMITS IT IS TIME AND SUPPLY. ON THE BEST MOUTHFUL IN THE BOX ({:.0}) IT NETS {:+.2} PER STEP AFTER UPKEEP, AND A CHILD COSTS {:.0}. THAT IS WHY BORN 0 CAN MEAN TWO OPPOSITE THINGS: A COLONY THAT NEEDS LONGER, OR ONE THAT LOSES GROUND EVERY STEP AND NEVER GETS THERE. THE MOUTHFUL IS PRICED OVER EVERY MATERIAL STANDING IN THE BOX, SO IT IS THE BEST CASE.",
                    breed.best_mouthful, breed.gain_per_tick, breed.bar
                ),
            ));
        }
        rows.push(Row::gap());

        // --- what the whole game is about ------------------------------------
        rows.push(Row::text(
            "GENERATIONS",
            HEADING,
            "HOW MANY ANCESTORS DEEP THE POPULATION HAS GOT. THIS IS THE THING THE LAB IS FOR, AND IT IS THE NUMBER NOTHING USED TO SHOW.",
        ));
        rows.push(Row::text(
            format!("DEEPEST  PLANT {}   ANIMAL {}", census.plant_generation, census.animal_generation),
            if census.plant_generation > 0 || census.animal_generation > 0 { GREEN } else { DIM },
            "0 MEANS NOTHING OF THAT KIND HAS BRED: EVERY ONE ALIVE IS SOMETHING YOU OR THE BOX PUT THERE. 1 MEANS ONE ROUND OF INHERITANCE HAS HAPPENED, AND SELECTION HAS SOMETHING TO WORK ON.",
        ));
        rows.push(Row {
            body: Body::Generations,
            note: format!(
                "HOW THE LIVING POPULATION SPLITS BY GENERATION, 0 ON THE LEFT AND {}+ ON THE RIGHT. A TALL FIRST BAR AND NOTHING ELSE IS A POPULATION THAT HAS NOT BRED. THE SHAPE MOVING RIGHTWARD IS EVOLUTION HAPPENING.",
                GEN_BUCKETS - 1
            ),
        });
        rows.push(Row::text(
            format!("LINES {}   BIGGEST {:.0}%", census.lineages, census.top_lineage * 100.0),
            DIM,
            "HOW MANY SEPARATE FOUNDING FAMILIES ARE STILL GOING, AND WHAT SHARE OF EVERYTHING ALIVE THE BIGGEST OF THEM HOLDS. A POPULATION DOWN TO ONE LINE HAS CONVERGED, WHATEVER ITS INDIVIDUALS LOOK LIKE.",
        ));
        rows.push(Row::gap());

        // --- the ceiling -------------------------------------------------------
        // The guide warns the lab will reach it: herb already runs 1,812-2,503
        // live organisms with births outrunning deaths at 45,000 frames.
        let fill = census.slots_used as f32 / census.slots_ceiling.max(1) as f32;
        // **The gauge needs a name.** Rendered, it was a bar and `66 OF 4095`
        // with nothing saying what was being counted -- the one row on the
        // page a reader could not decode from the page itself.
        rows.push(Row::text(
            "ROOM IN THE BOX",
            HEADING,
            "THE BOX CAN HOLD A FIXED NUMBER OF LIVING THINGS AT ONCE, PLANTS AND ANIMALS TOGETHER, AND A SEED WAITING IN THE GROUND IS ONE OF THEM.",
        ));
        rows.push(Row {
            body: Body::Gauge(
                fill.clamp(0.0, 1.0),
                format!("{} OF {}", census.slots_used, census.slots_ceiling),
                if census.refused > 0 || fill > 0.9 { AMBER } else { GREEN },
            ),
            note: format!(
                "LIVING THINGS AGAINST THE BOX'S HARD LIMIT. {} SLOTS HAVE BEEN USED AT ONCE AT THE MOST, OUT OF {}; {} ARE LIVE RIGHT NOW AND {} BIRTHS HAVE BEEN TURNED AWAY FOR WANT OF A SLOT. A POPULATION THAT HAS QUIETLY HIT THE LIMIT AND ONE THAT HAS SETTLED LOOK IDENTICAL -- THE REFUSED COUNT IS THE ONLY THING THAT TELLS THEM APART.",
                census.slots_high_water,
                census.slots_ceiling,
                census.plants + census.animals,
                census.refused
            ),
        });
        if let Some(name) = &census.animal_species {
            // **The richest animal against the bar is the number that
            // moves**, and it belongs on the row rather than in a note: the
            // margin above says the bar is unreachable in principle, and this
            // says how far the best-fed animal in the box has actually got.
            // A player can watch one and not the other.
            rows.push(Row::text(
                format!(
                    "{name} HUNGRY {} OF {}   BEST {:.0}",
                    census.animal_hungry, census.animals, census.animal_energy.high
                ),
                if census.animal_hungry * 2 > census.animals { AMBER } else { DIM },
                format!(
                    "ANIMALS BELOW THEIR OWN HUNGER LINE -- UNDER IT THEY EAT WHAT THEY FIND, OVER IT THEY CARRY IT HOME. HUNGRY IS THE NORMAL STATE OF A FORAGER. STORES RUN {:.0} / {:.0} / {:.0}, AGAINST THE {} A CHILD COSTS.",
                    census.animal_energy.low,
                    census.animal_energy.mid,
                    census.animal_energy.high,
                    census.breed.map_or("--".to_string(), |b| format!("{:.0}", b.bar))
                ),
            ));
        }
        rows
    }

    /// Frames spanned by the history so far.
    fn span(&self) -> u64 {
        match (self.history.first(), self.history.last()) {
            (Some(a), Some(b)) => b.frame.saturating_sub(a.frame),
            _ => 0,
        }
    }

    /// The headline's one judgement word: the sign of the two strips
    /// together, said in words for a reader who has not looked at them.
    ///
    /// **The combined sign, not a health score** — nothing here knows what a
    /// healthy biosphere is. It is amber when either kingdom is losing
    /// ground, because in a lab either one going down is the thing to look
    /// at.
    fn trend_word(&self) -> (&'static str, [u8; 4]) {
        let (Some(a), Some(b)) = (self.history.first(), self.history.last()) else {
            return ("", WHITE);
        };
        if a.frame >= b.frame {
            return ("", WHITE);
        }
        let up = b.plants > a.plants || b.animals > a.animals;
        let down = b.plants < a.plants || b.animals < a.animals;
        match (up, down) {
            (true, true) => ("  MIXED", AMBER),
            (true, false) => ("  GROWING", GREEN),
            (false, true) => ("  SHRINKING", AMBER),
            (false, false) => ("  STEADY", WHITE),
        }
    }

    /// One population strip.
    ///
    /// **It starts empty and fills as you watch**, said in words rather than
    /// left as a blank box, because an empty chart reads as a dead population
    /// and here it means the opposite.
    ///
    /// Drawn against **frame, not sample index**, which the colony panel's
    /// does not need to be: this ring decimates and the dial changes, so
    /// samples are not evenly spaced in world time and one column per sample
    /// would stretch whichever stretch of the run happened to be sampled
    /// densely. Consecutive points are joined so a sparse early run still
    /// reads as a line.
    fn draw_strip(
        &self,
        frame: &mut [u8],
        x: i32,
        y: i32,
        series: Series,
        colour: [u8; 4],
        label: &str,
    ) {
        const UNDER: [u8; 4] = [26, 34, 44, 255];
        let (w, h) = (super::WIDTH, super::HEIGHT);
        let height = 16;
        let width = 168;
        for px in x..x + width {
            crate::render::put(frame, w, h, px, y + height, RULE);
        }
        let value = |s: &Sample| match series {
            Series::Plants => s.plants,
            Series::Animals => s.animals,
        };
        if self.history.len() < 2 {
            text(frame, x, y + height - 12, "TRACKING FROM NOW", FAINT);
            return;
        }
        let peak = self.history.iter().map(value).max().unwrap_or(1).max(1);
        // A quarter of headroom above the peak, so a flat line sits
        // somewhere you can see it is flat rather than pinned to the top.
        let axis = (peak * 5 / 4).max(peak + 1);
        let first = self.history.first().expect("len >= 2").frame;
        let last = self.history.last().expect("len >= 2").frame;
        let span = last.saturating_sub(first).max(1);
        let column = |s: &Sample| {
            x + ((s.frame.saturating_sub(first) as u128 * (width - 1) as u128) / span as u128) as i32
        };
        let bar = |s: &Sample| (value(s) as i64 * height as i64 / axis as i64) as i32;

        let mut previous: Option<(i32, i32)> = None;
        for sample in &self.history {
            let (px, ph) = (column(sample), bar(sample));
            for dy in 0..ph {
                crate::render::put(frame, w, h, px, y + height - 1 - dy, UNDER);
            }
            crate::render::put(frame, w, h, px, y + height - 1 - ph.min(height - 1), colour);
            // Join to the previous point: with a decimated ring the columns
            // are not adjacent, and isolated dots do not read as a line.
            if let Some((qx, qh)) = previous {
                for cx in (qx + 1)..px {
                    let t = (cx - qx) as f32 / (px - qx).max(1) as f32;
                    let ch = qh + ((ph - qh) as f32 * t).round() as i32;
                    for dy in 0..ch {
                        crate::render::put(frame, w, h, cx, y + height - 1 - dy, UNDER);
                    }
                    crate::render::put(frame, w, h, cx, y + height - 1 - ch.min(height - 1), colour);
                }
            }
            previous = Some((px, ph));
        }
        text(frame, x + width + 6, y + 1, label, FAINT);
        // **`MAX 43` rather than `43`.** Read off the rendered page, a bare
        // number beside a strip whose headline says `PLANTS 41` looks like a
        // second population count and invites the reader to reconcile two
        // figures that are not the same quantity.
        text(frame, x + width + 6, y + height - 7, &format!("MAX {peak}"), colour);
    }
}

/// One walk of the live organism table, reduced to what the page draws.
///
/// **Both kingdoms in one walk**, because the alternative is two walks that
/// can disagree about what was alive. Cost is `O(live organisms)` with an
/// `O(1)` read per organism — `HashMap::len` for the cell count, plain field
/// reads for everything else — and is *not* a grid scan; the 163,840-cell
/// scan `creature_probe` uses is the thing this is written to avoid.
fn take_census(
    world: &World,
    standing: &mut Vec<crate::sim::material::MaterialId>,
    refresh_standing: bool,
) -> Census {
    let ids = world.live_organism_ids();
    let mut census = Census {
        frame: world.frame,
        plants: 0,
        animals: 0,
        plant_cells: 0,
        animal_cells: 0,
        senescent: 0,
        plant_size: Spread::default(),
        seeds_standing: 0,
        seeds_borne: world.fate_mutation_rolls,
        germinations: world.germinations,
        plant_generation: 0,
        animal_generation: 0,
        generations: [0; GEN_BUCKETS],
        lineages: 0,
        top_lineage: 0.0,
        animal_species: None,
        animal_energy: Spread::default(),
        animal_hungry: 0,
        breed: None,
        slots_used: 0,
        slots_high_water: 0,
        slots_ceiling: 0,
        refused: world.organisms_refused(),
    };
    let (slots_used, _) = world.organism_slot_usage();
    let (high_water, ceiling) = world.organism_slot_high_water();
    census.slots_used = slots_used;
    census.slots_high_water = high_water;
    census.slots_ceiling = ceiling;

    let mut plant_sizes: Vec<f32> = Vec::new();
    // Grouped by species so the animal economy below is a species' property
    // rather than an average over two different animals.
    let mut animals_by_species: Vec<(organism::SpeciesId, Vec<f32>)> = Vec::new();
    let mut lineages: Vec<(u32, usize)> = Vec::new();
    let mut alive = 0usize;
    // **What is standing here that something could eat.** Distinct materials
    // only, deduped by linear scan because the answer is a handful of entries
    // (leaf, wood, seed, flower, ant) and a set would cost more to build than
    // it saves. See `breed_margin` for why the mouthful is priced on this
    // rather than on the whole material table, and `STANDING_INTERVAL` for why
    // it is not rebuilt every census.
    if refresh_standing {
        standing.clear();
    }

    for id in &ids {
        let Some(state) = world.organism(*id) else { continue };
        let cells = state.cells.len();
        let creature = world.species.get(state.species).creature.is_some();
        // A senescent plant is dead and rotting: counted as neither living
        // population nor living biomass, but not silently dropped either --
        // it gets its own row, because a graded death that is invisible reads
        // as a plant that is still fine right up until the last cell goes.
        if state.senescent {
            census.senescent += 1;
            continue;
        }
        alive += 1;
        if refresh_standing {
            for (x, y) in state.cells.keys() {
                let m = world.get(*x, *y).material;
                if !standing.contains(&m) {
                    standing.push(m);
                }
            }
        }
        let bucket = (state.generation as usize).min(GEN_BUCKETS - 1);
        census.generations[bucket] += 1;
        match lineages.iter_mut().find(|(l, _)| *l == state.lineage) {
            Some((_, n)) => *n += 1,
            None => lineages.push((state.lineage, 1)),
        }
        if creature {
            census.animals += 1;
            census.animal_cells += cells;
            census.animal_generation = census.animal_generation.max(state.generation);
            match animals_by_species.iter_mut().find(|(sp, _)| *sp == state.species) {
                Some((_, energies)) => energies.push(state.energy),
                None => animals_by_species.push((state.species, vec![state.energy])),
            }
        } else {
            census.plants += 1;
            census.plant_cells += cells;
            census.plant_generation = census.plant_generation.max(state.generation);
            census.seeds_standing = census.seeds_standing.saturating_add(state.seeds_set);
            plant_sizes.push(cells as f32);
        }
    }

    census.plant_size = Spread::of(&mut plant_sizes).unwrap_or_default();
    census.lineages = lineages.len();
    let top = lineages.iter().map(|(_, n)| *n).max().unwrap_or(0);
    census.top_lineage = if alive == 0 { 0.0 } else { top as f32 / alive as f32 };

    // The dominant animal species: one page cannot show two hunger lines or
    // two birth costs, and naming which one it is beats averaging them.
    animals_by_species.sort_by_key(|(_, energies)| std::cmp::Reverse(energies.len()));
    if let Some((species, energies)) = animals_by_species.first_mut() {
        let entry = world.species.get(*species);
        census.animal_species = Some(entry.name.to_uppercase());
        if let Some(def) = entry.creature.as_ref() {
            // **Poorer than a newborn**, since the hunger gate this used to
            // read no longer exists. See `app.rs`'s `lean_line`: with a crop
            // nothing compares a bank against a threshold, but "how many of my
            // animals are struggling" is still the question a player asks, and
            // below what this species hands a child is the derived answer.
            let line = crate::sim::creature::birth_grant(def, &def.traits);
            census.animal_hungry = energies.iter().filter(|e| **e < line).count();
            census.breed = breed_margin(world, def, standing);
        }
        census.animal_energy = Spread::of(energies).unwrap_or_default();
    }
    census
}

/// **What a birth needs against what this animal can ever hold.**
///
/// The arithmetic is `examples/stamp_probe.rs`', reproduced rather than
/// re-derived: the ceiling is `hunger_fraction * start_energy` — the line the
/// brain's own `hungry` tests, above which it carries food home instead of
/// eating it — plus one best mouthful, because an animal exactly at the line
/// takes one more bite before it stops.
///
/// **Priced on the food standing in *this box*, not on the material table**,
/// and that is the choice that makes this row worth having rather than a
/// constant nobody can act on. Measured, `examples/labstats.rs`:
///
/// | bed | best mouthful | ceiling | margin |
/// |---|---|---|---|
/// | `control=ants` — a colony, nothing planted | 120 (its own flesh) | 220 | **−880** |
/// | the standard lab bed at 27,000 frames | 360 (a `flower`) | 460 | **−640** |
///
/// The first is PR #162's own number, reproduced. The second is the row doing
/// the job a table-priced one could not: **the herb stand has flowered, and
/// the page says so through the margin.** `stamp_probe`'s comment for the same
/// correction — a table-priced bound "can rule out and can never rule in".
///
/// **And it says something the deadlock report wants.** A flower is 1,440 at
/// food class −1; the neutral gut takes 360 of it because
/// `creature::diet_yield` squares the mismatch. A gut drifted to −1 would draw
/// the whole 1,440 and clear the 1,100 bar outright — so in *this* bed the
/// deadlock is one mutation wide, which is exactly the shape
/// `Reports/lanes/evolution-lab-coordinator.md` was hoping for and could not
/// see, because that finding was measured on worldgen worlds where no flower
/// stands.
///
/// `standing` is the distinct materials of every living organism's cells,
/// collected by the census's own walk and cached on [`STANDING_INTERVAL`]'s
/// slower clock. It therefore misses litter and corpses lying on the floor —
/// reading those needs the 163,840-cell grid scan this page exists not to pay
/// for — and a colony's own flesh *is* in it, which is what makes an ant-only
/// bed read −880.
fn breed_margin(
    world: &World,
    def: &organism::CreatureDef,
    standing: &[crate::sim::material::MaterialId],
) -> Option<BreedMargin> {
    use crate::sim::cell::Cell;
    use crate::sim::creature;
    let bar = creature::reproduce_at(def)?;
    let gut = def.traits[organism::TRAIT_GUT_BIAS];
    let best = standing
        .iter()
        .map(|id| {
            // **A corpse carries its worth in `aux`, so an unstamped probe
            // cell prices the whole carrion half at nothing.** Exactly
            // `stamp_probe::yield_of`'s construction, reproduced so the two
            // numbers are the same number — a page quoting a different
            // ceiling from the harness that established the finding would be
            // worse than one quoting none.
            let aux = if world.materials.get(*id).worth_in_aux {
                def.body_energy.round().clamp(0.0, 65535.0) as u16
            } else {
                0
            };
            creature::diet_yield(world, Cell::new(*id, 0).with_aux(aux), gut)
        })
        .fold(0.0f32, f32::max);
    // The best *conversion* this gut can achieve, which is what a rate turns
    // face value into. Read over the same standing materials as `best`, so the
    // two cannot describe different worlds.
    let best_quality = standing
        .iter()
        .map(|id| creature::diet_quality(world, *id, gut))
        .fold(0.0f32, f32::max);
    let upkeep = def.idle_cost_per_cell * def.body.len() as f32;
    Some(BreedMargin { gain_per_tick: def.digest_rate * best_quality - upkeep, bar, best_mouthful: best })
}

/// The generation histogram — how the living population splits by how many
/// ancestors deep it is.
///
/// **A shape, not the deepest number alone.** One plant at generation 4 in a
/// stand of two thousand founders and a stand that has entirely turned over
/// read the same in `DEEPEST`, and they are the difference between a lucky
/// seed and a population that is evolving.
fn draw_generations(frame: &mut [u8], census: &Census, x: i32, y: i32) {
    let (w, h) = (super::WIDTH, super::HEIGHT);
    let height = 15;
    let bar = 14;
    let gap = 2;
    let tallest = census.generations.iter().copied().max().unwrap_or(0).max(1);
    for (i, count) in census.generations.iter().enumerate() {
        let bx = x + i as i32 * (bar + gap);
        // Generation 0 is "planted, never bred" and is the state the box
        // starts in; everything past it is inheritance that has happened.
        let colour = if i == 0 { FAINT } else { GREEN };
        let ph = if *count == 0 { 0 } else { ((*count as i64 * height as i64 / tallest as i64) as i32).max(1) };
        for dy in 0..ph {
            for dx in 0..bar {
                crate::render::put(frame, w, h, bx + dx, y + height - 1 - dy, colour);
            }
        }
        for dx in 0..bar {
            crate::render::put(frame, w, h, bx + dx, y + height, RULE);
        }
    }
    text(frame, x, y + height + 2, "0  1  2  3  4  5  6  7+", FAINT);
}

/// A filled bar, `fill` in 0..1. Used for the organism ceiling, where the
/// *headroom* is the point and a percentage buries it.
fn draw_gauge(frame: &mut [u8], x: i32, y: i32, width: i32, fill: f32, on: [u8; 4]) {
    let (w, h) = (super::WIDTH, super::HEIGHT);
    let filled = (width as f32 * fill).round() as i32;
    for dx in 0..width {
        for dy in 0..5 {
            let colour = if dx < filled { on } else { RULE };
            crate::render::put(frame, w, h, x + dx, y + dy, colour);
        }
    }
}

/// The explanation for the row under the cursor.
///
/// Placed **beside the page, not beside the cursor** — a box that follows the
/// pointer covers the row it is explaining, so you read the explanation
/// having lost the thing it is about. The page is on the right, so the note
/// opens to its **left**, which is the mirror of the colony panel's choice
/// and the same reasoning.
fn draw_note(frame: &mut [u8], note: &str, (_, cy): (i32, i32)) {
    const BG: [u8; 4] = [16, 20, 30, 255];
    const ALPHA: f32 = 0.92;
    let (w, h) = (super::WIDTH, super::HEIGHT);
    let (panel_left, _, _) = RECT;
    let width = panel_left - 12;
    let inner = width - 12;
    let columns = (inner / (crate::hud::GLYPH_WIDTH + 1)).max(8) as usize;
    let lines = wrap_words(note, columns);
    let x = 6;
    let height = lines.len() as i32 * LINE + 9;
    // Top-aligned with the row, then pulled back on screen. `max(10)` rather
    // than clamping to 0 so it never sits under the top edge.
    let y = (cy - 4).min(super::HEIGHT as i32 - 10 - height).max(10);
    for py in y..y + height {
        for px in x..x + width {
            crate::render::blend(frame, w, h, px, py, BG, ALPHA);
        }
    }
    for px in x..x + width {
        crate::render::put(frame, w, h, px, y, HEADING);
        crate::render::put(frame, w, h, px, y + height - 1, HEADING);
    }
    for py in y..y + height {
        crate::render::put(frame, w, h, x, py, HEADING);
        crate::render::put(frame, w, h, x + width - 1, py, HEADING);
    }
    for (i, line) in lines.iter().enumerate() {
        text(frame, x + 6, y + 5 + i as i32 * LINE, line, WHITE);
    }
}

/// Break `text` into lines of at most `columns` characters, on spaces.
///
/// A word longer than the column count is left to overrun rather than split:
/// every one on this page is a number, and a number cut in half across two
/// lines is worse than a line that runs a little wide.
fn wrap_words(text: &str, columns: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + word.chars().count() <= columns => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// Every string this page draws goes through here.
///
/// The font is a partial set and renders anything it lacks as a **blank
/// gap**, not a visible box (`hud.rs`, which records that shipping three
/// times). Nearly every string here is composed at run time out of species
/// names and formatted numbers, so a test over literals cannot see them; a
/// `debug_assert` checks whatever the page actually built, in every test that
/// draws it.
fn text(frame: &mut [u8], x: i32, y: i32, s: &str, colour: [u8; 4]) {
    debug_assert!(
        s.chars().all(crate::hud::has_glyph),
        "the biosphere page prints {s:?}, which the font would draw as a blank gap"
    );
    crate::hud::draw_text(frame, super::WIDTH, super::HEIGHT, x, y, s, colour);
}

/// **The page as text**, one line per row, for a harness.
///
/// `Reports/lanes/creature-lane-g.md` records an hour spent hunting a
/// simulation bug that turned out to be a `3` one glyph wide in a downscaled
/// screenshot: the panel had said `PICKED 30`. Look at the picture for *what
/// and where*; read this for *how much*.
pub fn dump(stats: &Stats, world: &World) -> Vec<String> {
    stats
        .rows(world)
        .iter()
        .map(|row| match &row.body {
            Body::Text(s, _) => s.clone(),
            Body::Gap => String::new(),
            Body::Strip(Series::Plants, ..) => "[strip: plants]".to_string(),
            Body::Strip(Series::Animals, ..) => "[strip: animals]".to_string(),
            Body::Generations => format!(
                "[generations: {:?}]",
                stats.census.as_ref().map(|c| c.generations).unwrap_or_default()
            ),
            Body::Gauge(fill, label, _) => format!("[gauge {fill:.3}] {label}"),
        })
        .collect()
}

/// Every note the page would show, for a test that has to drive them through
/// the glyph assert — they are built at run time, so nothing else can see a
/// character the font would draw as a blank gap.
pub fn notes(stats: &Stats, world: &World) -> Vec<String> {
    stats.rows(world).iter().map(|row| row.note.clone()).filter(|n| !n.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::scene::LabBox;

    /// A small bed, built by the same `scene::LabBox::build` the game uses —
    /// **not a private copy of it.** `scene`'s own module doc records why: a
    /// bed that is not the game's bed produces results that do not transfer.
    ///
    /// **256 wide rather than the smallest box that compiles**, and that is a
    /// scene decision rather than a taste one: `creature::found_colony` lays
    /// 52 ants at 4-cell spacing, so the band is **204 cells** and placement
    /// off the edge of the world fails *silently*. A narrower bed would seat
    /// a handful of ants or none, and `animals > 0` would then be a test of
    /// the bed rather than of the census — `CLAUDE.md`'s *a scene that
    /// contradicts the code will look like a bug in the code*.
    fn bed(founders: usize, colonies: usize) -> World {
        LabBox {
            width: 256,
            height: 128,
            soil_depth: 24,
            ground_y: 64,
            founders,
            colonies,
            ..LabBox::default()
        }
        .build()
    }

    fn censused(world: &World) -> Stats {
        let mut stats = Stats::new();
        stats.observe(world);
        stats
    }

    fn blank_frame() -> Vec<u8> {
        vec![0u8; (super::super::WIDTH * super::super::HEIGHT * 4) as usize]
    }

    /// **Both kingdoms, and each one's absence proved as well as its
    /// presence.** The negative half alone passes for a census that counts
    /// nothing, which is the blind-guard shape `CLAUDE.md` names; the
    /// positive half alone passes for one that cannot tell a plant from an
    /// ant, which is the failure the colony panel's census guards against
    /// from the other side.
    #[test]
    fn the_census_counts_plants_and_animals_apart() {
        let plants_only = censused(&bed(4, 0));
        let c = plants_only.census().expect("a census");
        assert!(c.plants > 0, "four founders were planted, census says {}", c.plants);
        assert!(c.plant_cells > 0, "a planted herb owns cells");
        assert_eq!(c.animals, 0, "no colony was founded");
        assert_eq!(c.animal_cells, 0);
        assert!(c.breed.is_none(), "no animal species, so no birth economy to price");

        let ants_only = censused(&bed(0, 1));
        let c = ants_only.census().expect("a census");
        assert!(c.animals > 0, "a colony was founded, census says {}", c.animals);
        assert!(c.animal_cells > 0);
        assert_eq!(c.plants, 0, "nothing was planted");
        assert_eq!(c.plant_cells, 0);
        assert!(c.breed.is_some(), "an ant has a birth cost and a hunger ceiling");
        assert!(c.animal_species.is_some());
    }

    /// The specificity half: nothing in the box, nothing on the page.
    #[test]
    fn an_empty_box_says_so_rather_than_drawing_plausible_zeroes() {
        let world = bed(0, 0);
        let stats = censused(&world);
        let c = stats.census().expect("a census");
        assert_eq!((c.plants, c.animals, c.biomass()), (0, 0, 0));
        assert_eq!(c.generations.iter().sum::<u32>(), 0);
        let rows = dump(&stats, &world);
        assert_eq!(rows.first().map(String::as_str), Some("THE BOX IS EMPTY"));
        assert!(
            !rows.iter().any(|r| r.starts_with("PLANTS ")),
            "an empty box must not draw a headline of zeroes: {rows:?}"
        );
    }

    /// **The sensitivity half of the population count.** A count that is
    /// correct about a settled box may still be a constant; this puts the
    /// fault in — half the stand killed — and watches the number move.
    #[test]
    fn killing_half_the_stand_moves_the_living_count_and_the_dying_back_row() {
        let mut world = bed(6, 0);
        let before = censused(&world);
        let live = before.census().expect("a census").plants;
        assert!(live >= 2, "the control needs a stand to halve, got {live}");

        let ids = world.live_organism_ids();
        let mut killed = 0;
        for id in ids.iter().take(ids.len() / 2) {
            if world.mark_organism_senescent(*id) {
                killed += 1;
            }
        }
        assert!(killed > 0, "the disturbance seam did nothing");

        let after = censused(&world);
        let c = after.census().expect("a census");
        assert_eq!(c.plants, live - killed, "a senescent plant is not living population");
        assert_eq!(c.senescent, killed, "and it is not silently dropped either");
        assert!(dump(&after, &world).iter().any(|r| r.starts_with("DYING BACK")));
    }

    /// **The strip is a picture of simulated time, not of the speed dial.**
    /// One displayed frame is one tick at 1x and up to 256 at the top of the
    /// ladder, so a sample per call would draw the dial.
    #[test]
    fn the_strip_samples_world_frames_and_not_calls() {
        let world = bed(2, 0);
        let mut stats = Stats::new();
        for _ in 0..200 {
            stats.observe(&world);
        }
        assert_eq!(stats.history().len(), 1, "the world never advanced, so there is one reading");
    }

    /// **The ring decimates rather than scrolls**, so the strip always spans
    /// the whole run. Asserted from both ends: the oldest sample must still
    /// be the run's first, and the interval must have widened.
    #[test]
    fn the_history_keeps_the_start_of_the_run() {
        let mut stats = Stats::new();
        let mut world = bed(0, 0);
        for _ in 0..(HISTORY as u64 * 4) {
            world.frame += SAMPLE_INTERVAL;
            stats.observe(&world);
        }
        assert_eq!(stats.history().first().map(|s| s.frame), Some(SAMPLE_INTERVAL));
        assert!(stats.history().len() <= HISTORY, "the ring is bounded: {}", stats.history().len());
        assert!(stats.interval > SAMPLE_INTERVAL, "the interval widened as it decimated");
        // A push is only due every `interval` frames, so step past the
        // widened interval before asserting the newest reading is the
        // newest -- otherwise this passes or fails on where the loop above
        // happened to stop.
        world.frame += stats.interval;
        stats.observe(&world);
        let last = stats.history().last().expect("samples").frame;
        assert_eq!(last, world.frame, "the newest reading survives every halving");
        assert_eq!(
            stats.history().first().map(|s| s.frame),
            Some(SAMPLE_INTERVAL),
            "and the run's first reading is still on the left"
        );
    }

    /// A rate is a difference over a window and there is no window yet.
    #[test]
    fn rates_need_a_window_before_they_report_anything() {
        let world = bed(2, 0);
        let stats = censused(&world);
        assert!(stats.rate_window().is_none());
        assert!(dump(&stats, &world).iter().any(|r| r.contains("MEASURING")));
    }

    /// **The generation histogram is a shape, not the deepest number.** One
    /// organism at generation 4 among two thousand founders and a population
    /// that has entirely turned over read the same in `DEEPEST`.
    #[test]
    fn a_bred_organism_lands_past_the_first_generation_bucket() {
        let mut world = bed(4, 0);
        let flat = censused(&world);
        let c = flat.census().expect("a census");
        assert_eq!(c.plant_generation, 0, "nothing planted has an ancestor");
        assert_eq!(c.generations[1..].iter().sum::<u32>(), 0, "{:?}", c.generations);

        let id = *world.live_organism_ids().first().expect("a founder");
        world.organism_mut(id).expect("live").generation = 3;
        let bred = censused(&world);
        let c = bred.census().expect("a census");
        assert_eq!(c.plant_generation, 3);
        assert_eq!(c.generations[3], 1, "{:?}", c.generations);
        assert!(dump(&bred, &world).iter().any(|r| r.contains("PLANT 3")));
    }

    /// **The shipped ant cannot pay for a child, and the page says by how
    /// much.** The number, not just its sign: a margin that happened to be
    /// negative for an unrelated reason would pass a sign-only assertion.
    #[test]
    fn the_page_prices_the_ants_birth_and_finds_it_short() {
        let world = bed(0, 1);
        let stats = censused(&world);
        let breed = stats.census().expect("a census").breed.expect("an ant has an economy");
        assert!(breed.bar > 0.0, "a bar to reach");
        assert!(breed.best_mouthful > 0.0, "some material in the game is edible");
        // **The claim changed shape with the model.** There is no ceiling to
        // sit under any more, so what this asserts is that the page reports a
        // *rate* and a time rather than a verdict: an ant that can out-eat its
        // upkeep gets a number of steps, one that cannot gets the amber row.
        assert!(
            breed.gain_per_tick.is_finite(),
            "the page must price a feeding rate, got {}",
            breed.gain_per_tick
        );
        let rows = dump(&stats, &world);
        assert!(
            rows.iter().any(|r| r.contains("STEPS OF FEEDING") || r.contains("CANNOT OUT-EAT")),
            "the birth-economy row must say either how long a child takes or that it never arrives"
        );
    }

    /// **The page stays inside its own border, and it draws something.**
    /// The containment half alone passes for a page that paints nothing,
    /// which is the blind-guard shape `CLAUDE.md` says to put the fault back
    /// for — hence the lit-pixel floor.
    #[test]
    fn the_page_stays_inside_its_own_border() {
        let world = bed(4, 1);
        let stats = censused(&world);
        let mut frame = blank_frame();
        stats.draw(&mut frame, &world);

        let (left, top, right, bottom) = stats.rect(&world);
        let content: i32 = stats.rows(&world).iter().map(Row::height).sum();
        assert_eq!(bottom, (top + HEADER + content + 6).min(super::super::ui::bar_top() - 6));
        let mut lit = 0;
        let mut ink = 0;
        for y in 0..super::super::HEIGHT as i32 {
            for x in 0..super::super::WIDTH as i32 {
                let i = ((y as u32 * super::super::WIDTH + x as u32) * 4) as usize;
                if frame[i..i + 4].iter().any(|b| *b != 0) {
                    lit += 1;
                    assert!(
                        (left..right).contains(&x) && (top..bottom).contains(&y),
                        "the page lit ({x}, {y}), outside ({left}, {top})..({right}, {bottom})"
                    );
                }
                // **Ink, not lit pixels.** The panel plate is blended over
                // black and comes out around 8 per channel, and the rules and
                // axes at 40-72, so `lit` counts ~65,000 pixels for a page
                // that has painted its own background and nothing else --
                // which is precisely the blind guard `CLAUDE.md` says to put
                // the fault back for. Every colour the page writes text and
                // bars in clears 90 in some channel.
                if frame[i..i + 3].iter().any(|b| *b > 80) {
                    ink += 1;
                }
            }
        }
        assert!(lit > 2_000, "the page drew only {lit} pixels, which a blank page would also pass");
        // The bar is set with headroom under the measured value rather than
        // on it (`CLAUDE.md`). Printed so the next reader can re-derive it
        // rather than trust this comment: `cargo test --lib
        // the_page_stays_inside_its_own_border -- --nocapture`.
        println!("the page on this bed: {lit} lit pixels, {ink} of them ink");
        assert!(ink > 2_500, "the page drew {ink} pixels of text and bars over a {lit}-pixel plate");
    }

    /// A shut page paints nothing at all, and an open one paints. Both
    /// halves, because a shut-only check passes for a page that never draws.
    #[test]
    fn a_shut_page_draws_nothing_and_an_open_one_does() {
        let world = bed(4, 1);
        let mut stats = censused(&world);
        let mut open = blank_frame();
        stats.draw(&mut open, &world);
        assert!(open.iter().any(|b| *b != 0));

        stats.toggle();
        let mut shut = blank_frame();
        stats.draw(&mut shut, &world);
        assert!(shut.iter().all(|b| *b == 0), "a shut page painted something");
    }

    /// Hovering a row opens its note; hovering off the page does not. **Both
    /// directions** — a check that only looks for the note appearing passes
    /// for a page that draws a box wherever the cursor is. It also drives
    /// every note through `text`'s glyph assert, which is the only thing that
    /// can see a character the font would draw as a blank gap, since the
    /// notes are built at run time.
    #[test]
    fn hovering_a_row_explains_it_and_hovering_off_the_page_does_not() {
        let world = bed(4, 1);
        let stats = censused(&world);
        let (left, top, ..) = stats.rect(&world);

        let mut plain = blank_frame();
        stats.draw_at(&mut plain, &world, None);
        let mut hovered = blank_frame();
        stats.draw_at(&mut hovered, &world, Some((left + 20, top + HEADER + 2)));
        assert_ne!(plain, hovered, "hovering the first row explained nothing");

        // Left of the page entirely: no row is under the cursor.
        let mut outside = blank_frame();
        stats.draw_at(&mut outside, &world, Some((2, top + HEADER + 2)));
        assert_eq!(plain, outside, "a cursor off the page opened a note anyway");
    }

    /// Every string the page builds is drawable. Most are composed at run
    /// time out of species names and formatted numbers, so a test over
    /// literals cannot see them; this drives the real ones through the
    /// `debug_assert` in `text`.
    #[test]
    fn every_string_the_page_builds_has_a_glyph_for_each_character() {
        for (founders, colonies) in [(0, 0), (4, 0), (0, 1), (4, 1)] {
            let world = bed(founders, colonies);
            let stats = censused(&world);
            for s in dump(&stats, &world).iter().chain(notes(&stats, &world).iter()) {
                assert!(
                    s.chars().all(crate::hud::has_glyph),
                    "({founders}, {colonies}) builds {s:?}, which the font would draw as a blank gap"
                );
            }
            let mut frame = blank_frame();
            stats.draw_at(&mut frame, &world, Some((super::super::WIDTH as i32 - 200, 40)));
        }
    }
}
