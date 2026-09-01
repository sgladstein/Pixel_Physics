//! **The bed.** What a lab box is made of, and the one builder that makes it.
//!
//! `Reports/evolution-lab-design-guide-2026-08-30.md` Gate 1: *"There is no
//! such scene today"* — `filmstrip scene=colony` grows its plants out of
//! worldgen, `PlantScene` builds beds with no creatures, `creature_probe`
//! builds floors with no plants. This is the bed with both in it, and the
//! guide's reason for putting it here rather than in a harness is that **a
//! bed that is not the game's bed produces results that do not transfer**.
//! So the binary and the measuring harnesses must call *this*, never a
//! private copy of it.
//!
//! What the measurements already decided, and is therefore not a knob:
//!
//! - **A ceiling, not a sky.** Under a moving sun the field solves every tile
//!   in the world every frame; under a held light it does not (feasibility
//!   §2, §3b). This is simultaneously the fiction and the single largest
//!   performance decision, and it is free — a thing not to have.
//! - **...and the ceiling has to be drawn as one.** Empty sky is the most
//!   expensive branch on the screen — **27.4 ns/px against stone's 6.7** —
//!   and in a sealed box it is most of the screen. The box therefore
//!   declares itself an enclosure (`sim::enclosure::Enclosure`), and the
//!   renderer paints its air as a room. See `sky::Interior`.
//! - **Weather pinned clear.** Same reason. The air simulation still runs;
//!   what is removed is the thing that wakes every tile every frame.
//! - **Soil is required** — owner decision, 2026-08-30: *"We need soil.
//!   Plants grow roots into it and creatures need to dig into it and ideally
//!   create homes."* 40 → 240 rows costs **1.9x the frame**, so §2a's
//!   obligation is that something actually reaches the depth being paid for.
//!   `DEFAULT_SOIL_DEPTH` is that obligation discharged against a *measured*
//!   consumer — the colony's galleries, which were already digging to within
//!   five rows of the stone — and its value is the arithmetic that puts the
//!   floor exactly on top of the control bar, not a round number.
//! - **Partitions are the strongest single finding in the guide** (§2c):
//!   walling a fanned 2048-wide bed into 16 compartments took it from 4.1x to
//!   7.6x real time at a stand held to within 0.2%, and the same wall buys
//!   evolutionary isolation and a scoring move. One object, three payoffs.
//!
//! # Nothing is ever placed on a partition
//!
//! §2c records a scene error that changed an answer: a `walls=` sweep put its
//! single fan at `width / 2`, which is *also* where the partition goes at
//! every power-of-two compartment count, so the impulse straddled two
//! compartments and containment measured as absent rather than as present.
//! Founders, colonies and lamps are therefore placed **inside compartments**
//! rather than across the bed and then hoped to miss — `compartment_spans`
//! is the one function that knows where the walls are, and everything else
//! asks it. A founder on a wall is not a small error: it is a founder that
//! does not exist, because `plant_tree_species` refuses an occupied cell.

use crate::sim::cell::Cell;
use crate::sim::enclosure::Enclosure;
use crate::sim::field;
use crate::sim::material;
use crate::sim::weather::Pin;
use crate::sim::world::World;

/// A lab box, as data. `build` is the only way a lab world is made.
#[derive(Clone, Debug)]
pub struct LabBox {
    pub width: i32,
    pub height: i32,
    /// Rows of soil under the bed's surface.
    pub soil_depth: i32,
    /// The surface the founders are planted on, in world rows from the top.
    pub ground_y: i32,
    /// Sealed compartments, floor to ceiling. 1 is an open box.
    pub compartments: usize,
    /// Plant founders, spread across the bed.
    pub founders: usize,
    /// Which species the founders are. `herb` because it is the only shipped
    /// plant whose life cycle evolution can act on — generation 5 in 45,000
    /// frames, against `tree`'s generation 1 in 200,000.
    pub species: String,
    /// Ant colonies to found, spread across the bed.
    pub colonies: usize,
    /// **Beetles to release, spread across the bed the same way.**
    ///
    /// Zero by default, and that is not timidity: a predator is the one
    /// stocking choice that can empty a box, so a bed you did not ask to be
    /// hunted is not hunted. Raising it is the whole point of the knob.
    ///
    /// **What it is for is a paired comparison, not a garnish.** Measured
    /// 2026-08-31 on the outdoor world (`predation_probe mode=range`, twelve
    /// seeds): the survival advantage of a cell no beetle body fits into is
    /// about 2.8x *with no predator in the world at all*, and roughly 2.1x
    /// with nine — so predation is not what makes shelter valuable there,
    /// and the two shelter measures could not even agree on the sign of the
    /// beetle term. Whether that holds in a sealed bed, where the prey
    /// cannot walk away and the soil is deep enough to dig into, is a
    /// different question and this is the knob that asks it. Two chambers,
    /// same seed, this at 0 and at 4, is the experiment.
    ///
    /// **Gause's caveat applies and the box already has the answer to it.**
    /// A predator and its prey in one homogeneous sealed container is the
    /// classic extinction result; persistence needs spatial structure, which
    /// is exactly what `compartments` is. If a stocked bed simply empties,
    /// raise that before concluding anything about the predator.
    pub predators: usize,
    /// Columns between grow-light fixtures. A spacing rather than a count so
    /// that a 4096-wide bed is lit like a 512-wide one instead of being lit
    /// eight times more thinly, and so every compartment gets at least one
    /// fixture however many compartments there are.
    ///
    /// **64 rather than 128 since the fixtures became the light source.** A
    /// lamp's pool is its own width plus what `field::LIGHT_DECAY` bleeds
    /// sideways -- about 55 columns all told, measured -- so at 128 the bed
    /// was lit in four islands with dark ground between them and the founders
    /// stood in the dark half. At 64 over a 512 bed this places one fixture
    /// per founder station, from the same `spread` the founders use, and
    /// adjacent pools overlap the way `LAMP_REACH_FRACTION` always claimed
    /// they did. A working grow room is the default; making it *not* work by
    /// dragging a fixture off its bed is the mechanic.
    pub lamp_spacing: i32,
    pub seed: u64,
    /// **Walls the player dropped by hand**, as column numbers.
    ///
    /// Kept on the *spec* rather than only painted into the world, because
    /// every other bed knob takes effect on rebuild — and a rebuild that
    /// silently removed the walls you placed would make the two halves of the
    /// same idea fight each other.
    ///
    /// `compartments` remains the arithmetic, evenly-spaced version; these are
    /// added to it. Both go through [`LabBox::partition_columns`], which is
    /// still the one place wall positions are decided, so founders, colonies
    /// and lamps avoid a hand-placed wall exactly as they avoid a computed
    /// one.
    pub extra_walls: Vec<i32>,
}

/// **Rows of soil, and why this number and not a round one.**
///
/// Owner, 2026-08-30: *"we need a thicker layer of dirt, we don't need the
/// giant cement bottom (although a lot of that should become the
/// interface)."* At 40 rows the bed was 40 of soil against **120 of stone**,
/// and 64 of that stone was on screen above the control bar — more visible
/// cement than dirt, in a box whose whole subject is what lives in the dirt.
///
/// **The number is arithmetic, not a preference:** `ground_y` (160) + 96 +
/// `FLOOR_ROWS` (8) = **264**, which is the top of the control bar at the
/// two-row height the interface lane is building it to (`BAR_HEIGHT` 56 of
/// a 320-row window). The soil runs down to the last row the player can
/// see, the stone floor is the eight-row rim directly above the bar, and
/// every remaining row of the base is *behind* the interface — the owner's
/// "a lot of that should become the interface" taken literally rather than
/// approximately.
///
/// The two-row bar landed the same day and the floor is flush with it. It
/// was **not** flush against the one-row bar this was written on — that left
/// a 34-row rim of stone above the bar rather than 8 — and choosing 96
/// anyway is the deliberate direction to be wrong in: a rim is stone the
/// player can see, where a *buried* floor is soil the box pays
/// `update_soil_water` for and nobody can look at, which is exactly the
/// waste §2a names. If the bar grows a third row, that is the direction it
/// goes, and the guard below is what says so.
///
/// It is deliberately **not** written as a call into `ui::bar_top()`. The
/// bar is expected to keep growing, and a soil depth that moved when it did
/// would be a light, water and frame-cost change arriving as a UI commit
/// with nobody measuring it. The relationship is guarded instead, by
/// `the_visible_bed_is_mostly_soil_and_its_floor_clears_the_control_bar`.
///
/// # This was measured before it was changed, and the first measurement said not to
///
/// One paired run at 96 rows read plant cells 725 → 578, biggest 135 → 76
/// and roots 12 rows → 7 — a regression, in the direction nobody predicts.
/// **It was one seed.** Swept properly (`examples/labsoil`, 4 depths × 12
/// world seeds × 9,000 frames, paired per seed, equal *ticks* so the deep
/// arm's larger frame cost cannot masquerade as a smaller stand):
///
/// | rows | plant cells, median | vs 40 rows, paired per seed |
/// |---|---|---|
/// | 40 | 504 | — |
/// | 64 | 560 | 9 better / 3 worse, median ratio **1.117** |
/// | 96 | 553 | 7 better / 5 worse, median ratio **1.046** |
/// | 128 | 533 | 7 better / 5 worse, median ratio **1.057** |
///
/// Deeper soil does not cost the stand; per-seed ratios at 96 rows span
/// **0.81–1.57**, and the single run that condemned it sits at 0.80 — inside
/// its own arm's spread. Light is the reason it cannot: the sky walk
/// attenuates through what is *above* the bench, soil is below it, and the
/// light at the bench after warmup reads **0.447 in all 48 runs**, identical
/// to three decimals at every depth. Soil depth is not a light knob; the
/// **shell's** thickness is, and that one is real (see `CEILING`).
///
/// # What earns the depth is the ants, not the roots
///
/// §2a's standing obligation is that something reaches the depth being paid
/// for. The founder herb does not: over all 48 runs the deepest root cell in
/// any stand reached **13 rows**, median 7, exactly as the 40-row bed's own
/// note said. **Burrows do, and they were already hitting the floor.** Dug
/// void below the surface, at 9,000 frames:
///
/// | rows of soil | deepest dug void | deepest ant |
/// |---|---|---|
/// | 40 | 17, 17, 20, **35** | 11–16 |
/// | 96 | 15, 16, 44, **57** | 12–29 |
/// | 128 | 16, 17, 18, **44** | 10–18 |
///
/// At 40 rows the colony's deepest gallery was 35 rows into a 40-row bed —
/// **five rows off the stone**, which is a bed the ants had run out of. So
/// this is not depth bought for a mutant that may never arrive: it is depth
/// the shipped colony was already asking for, and `packedsoil` is what now
/// lets the gallery it digs stand.
///
/// Making it deeper still is not free and making it shallower is not either:
/// what the soil does not fill, the stone base does (see `FLOOR_ROWS`), and
/// a row of soil runs `update_soil_water` where a row of confined stone runs
/// nothing.
pub const DEFAULT_SOIL_DEPTH: i32 = 96;

impl Default for LabBox {
    fn default() -> Self {
        Self {
            width: 512,
            height: 320,
            soil_depth: DEFAULT_SOIL_DEPTH,
            ground_y: 160,
            compartments: 1,
            founders: 8,
            species: "herb".to_string(),
            colonies: 1,
            // **Zero, so the shipped bed is bit-identical to the one before
            // this field existed.** Every measurement anyone has taken in
            // this box was taken unhunted, and a default of 1 would silently
            // re-baseline all of them.
            predators: 0,
            lamp_spacing: 64,
            seed: 1,
            extra_walls: Vec::new(),
        }
    }
}

/// Smallest number of stone rows under the soil, so the bed has a floor to
/// sit on rather than falling out of the world — the scene error
/// `PlantScene` records having paid for twice.
///
/// A **minimum**, not the thickness: the base runs from under the soil to
/// the bottom of the world, so the box fills the frame it is drawn in. Left
/// at a fixed 8 it did not, and the shortfall was not cosmetic — everything
/// below the bed is space *outside* the box, which the interior draws as dug
/// earth, so a third of the screen was flat near-black with nothing in it
/// and no way to reach it.
///
/// **At the shipped depth the eight rows are the whole of the visible
/// floor**, and the padding below them has stopped being decoration: it
/// fills the rows the control bar covers. It still costs nothing — confined
/// stone runs no pass — and it is what keeps the bed correct if the bar is
/// ever moved, hidden or made shorter, which is the one change that would
/// otherwise expose a strip of void along the bottom of the screen.
const FLOOR_ROWS: i32 = 8;
/// Thickness of the side walls and the floor edges.
const SHELL: i32 = 4;
/// Thickness of the ceiling.
///
/// **It used to be a light knob, and deliberately is not one any more.**
/// `field.rs` casts sky light down each CA column through
/// `SKY_TRANSMISSION^(depth / FIELD_SCALE)`, so a four-row ceiling passed
/// **0.447** of the daylight above it and a seven-row one **0.245** — and
/// that was the crop's entire light budget. It was found the expensive way:
/// thickening the shell from 4 to 7 rows to make room for a recessed lamp
/// took the bench from 0.40 to **0.22** of `field::MAX_LIGHT` and the stand
/// at frame 3,600 from 474 plant cells to **286**, seed set 12 to **0**,
/// with nothing failing and no test going red.
///
/// The box now declares itself sunless (`World::set_sky_lighting`) and is lit
/// by its fixtures, so this number decides how solid the box *looks* and
/// nothing else. Two guards replace the one that made this constant
/// untouchable: `the_lamps_are_what_light_the_bed` (pull the fixtures and the
/// bed goes dark, where it used to be byte-identical) and
/// `a_fixture_does_not_shade_its_own_light` (pack the fixture's own block
/// with stone and the bench must not notice).
const CEILING: i32 = 4;
/// The fixture material. Named rather than numbered, like every other
/// material this scene places -- an id is a position in
/// `material::MATERIAL_FILES` and nothing in the world should depend on one.
const LAMP_MATERIAL: &str = "growlamp";
/// Half-width of a grow-light fixture, in columns.
///
/// **Never narrower than a light block, and that is a measurement rather
/// than a preference.** Light lives on the coarse field at one value per
/// `FIELD_SCALE` cells, so a fixture that fits *inside* one block emits from
/// exactly one block-column however you slide it, and dragging it changes how
/// bright the pool is without moving where it is. Measured on
/// `examples/lamp_probe.rs` at `FIELD_SCALE` 16 with a fixed 15-cell bar: the
/// bench's light centroid sits at 263.500 for **ten consecutive columns** of
/// drag, dimming from 0.562 to 0.412 of `MAX_LIGHT` as it goes, and then
/// jumps 4.9 cells at once. That is the control a player calls broken —
/// nothing happens, and then it lurches.
///
/// Widening the bar to a block either side closes it outright: the same sweep
/// moves the centroid at **every one of 32 columns**, minimum step 0.444, at
/// a peak pinned flat at 0.600. So the rule is the fixture spans at least two
/// block-columns whatever `FIELD_SCALE` is.
///
/// 7 at today's `FIELD_SCALE` of 8 — a 15-cell bar, unchanged, and the sweep
/// at 8 has no dead cell in it either. The expression only bites if the field
/// is coarsened, which is a live proposal.
const LAMP_HALF: i32 = if field::FIELD_SCALE - 1 > 7 { field::FIELD_SCALE - 1 } else { 7 };
/// Rows of the ceiling a fixture is recessed into. Recessed rather than
/// hung: a bar bolted under the ceiling has a span to support and would be
/// the one thing in the box that can fall on the crop, which is a mechanic
/// nobody asked for. Inset, it is exactly as solid as the ceiling.
const LAMP_ROWS: i32 = 4;
/// How far a fixture's pool of light reaches, as a fraction of the spacing
/// between fixtures. A little over half, so adjacent pools overlap and the
/// wall between two lamps is dimmer rather than dark.
const LAMP_REACH_FRACTION: f32 = 0.62;

impl LabBox {
    /// The frame the grow light is held at.
    ///
    /// **Measured, not assumed** — the sun's hump is a cosine over half the
    /// period and the phase belongs to `sky_light_amplitude`, so this picks
    /// the brightest frame rather than guessing a quarter or a half. Guessing
    /// wrong pins the light at midnight, which reads in a census as
    /// *"constant light shrinks the stand"*.
    pub fn noon() -> u64 {
        (0..field::DAY_NIGHT_PERIOD_FRAMES)
            .max_by(|a, b| field::sky_light_amplitude(*a).total_cmp(&field::sky_light_amplitude(*b)))
            .expect("the day has frames in it")
    }

    /// The row the ceiling's shell starts at.
    pub fn ceiling_y(&self) -> i32 {
        (self.ground_y - self.height / 2).max(0)
    }

    /// The first row of *air* under the ceiling — the top of the room.
    pub fn room_top(&self) -> i32 {
        self.ceiling_y() + CEILING
    }

    /// The row the bed's stone base ends at — the bottom of the world, or
    /// `FLOOR_ROWS` under the soil if the world is shallower than that.
    fn bed_bottom(&self) -> i32 {
        self.height.max(self.ground_y + self.soil_depth + FLOOR_ROWS)
    }

    /// Columns the partition walls stand in, left to right. Empty for an
    /// open box.
    ///
    /// The one place the walls' positions are decided. Everything that has
    /// to avoid them reads this rather than recomputing it — the guide's
    /// §2c scene error is what a second copy of this arithmetic costs.
    pub fn partition_columns(&self) -> Vec<i32> {
        let mut out: Vec<i32> = if self.compartments <= 1 {
            Vec::new()
        } else {
            (1..self.compartments).map(|k| (self.width * k as i32) / self.compartments as i32).collect()
        };
        // Hand-placed walls join the computed ones here, so everything that
        // avoids a wall — `compartment_spans`, and through it every founder,
        // colony and lamp — avoids these too without knowing they exist.
        out.extend(self.extra_walls.iter().copied().filter(|x| *x > SHELL && *x < self.width - SHELL));
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Paint one wall column into a world that is already running.
    ///
    /// **The verb has to deliver now, not on rebuild.** `CLAUDE.md`'s second
    /// law: a control whose effect you can only see by restarting is one the
    /// player is a spectator of. So the column goes into the live world *and*
    /// into the spec, and the two agree.
    ///
    /// It cuts whatever is in the way. That is the point rather than a
    /// side-effect — a wall through a stand is a population split in half,
    /// which is the thing isolation is for.
    pub fn paint_wall(&self, world: &mut World, x: i32) {
        let (ceiling, bottom) = (self.ceiling_y(), self.bed_bottom());
        for y in ceiling..bottom {
            world.set(x, y, Cell::new(material::STONE, 0));
        }
    }

    /// `bed_bottom` for tests, which need the span the wall occupies.
    #[cfg(test)]
    pub fn bed_bottom_for_test(&self) -> i32 {
        self.bed_bottom()
    }

    /// Take a wall column back out of a running world.
    ///
    /// Beside `paint_wall` rather than in `Lab`, so the two ends of the same
    /// geometry stay in one file: a remove that used a different span than
    /// the paint would leave a stub of stone at the top or the bottom.
    pub fn clear_wall(&self, world: &mut World, x: i32) {
        let (ceiling, bottom) = (self.ceiling_y(), self.bed_bottom());
        for y in ceiling..bottom {
            world.set(x, y, Cell::EMPTY);
        }
    }

    /// The nearest hand-placed wall to `x`, within `reach` columns.
    pub fn wall_near(&self, x: i32, reach: i32) -> Option<i32> {
        self.extra_walls.iter().copied().filter(|w| (w - x).abs() <= reach).min_by_key(|w| (w - x).abs())
    }

    /// The open interior of each compartment, as `[lo, hi)` column ranges
    /// that exclude the shell and every partition.
    ///
    /// A span is where things may be *placed*. Nothing here ever returns a
    /// wall column, which is what makes "a founder on a partition" not a
    /// case any caller has to think about.
    pub fn compartment_spans(&self) -> Vec<(i32, i32)> {
        let mut spans = Vec::with_capacity(self.compartments.max(1));
        let mut lo = SHELL;
        for w in self.partition_columns() {
            spans.push((lo, w));
            lo = w + 1;
        }
        spans.push((lo, self.width - SHELL));
        spans.retain(|(lo, hi)| hi > lo);
        spans
    }

    /// Spread `n` items evenly through the compartments, round-robin, and
    /// report the column each lands in.
    ///
    /// Round-robin rather than by area on purpose: with fewer founders than
    /// compartments, what matters is that as many compartments as possible
    /// are *founded*, because an empty compartment is not an isolated
    /// population, it is an absent one.
    fn spread(&self, n: usize) -> Vec<i32> {
        let spans = self.compartment_spans();
        if n == 0 || spans.is_empty() {
            return Vec::new();
        }
        // How many land in each compartment, before any is placed — a span
        // needs its own total to space its share evenly inside itself.
        let mut per = vec![0usize; spans.len()];
        for i in 0..n {
            per[i % spans.len()] += 1;
        }
        let mut out = Vec::with_capacity(n);
        for (&(lo, hi), &count) in spans.iter().zip(per.iter()) {
            for j in 0..count {
                let step = (hi - lo) as i64 * (j as i64 + 1) / (count as i64 + 1);
                out.push(lo + step as i32);
            }
        }
        out.sort_unstable();
        out
    }

    /// Columns the founders are planted at.
    pub fn founder_columns(&self) -> Vec<i32> {
        self.spread(self.founders)
    }

    /// Columns the ant colonies are founded at.
    pub fn colony_columns(&self) -> Vec<i32> {
        self.spread(self.colonies)
    }

    /// Columns the beetles are released at.
    ///
    /// **The same `spread` as everything else, and that matters here more
    /// than it does for founders.** A predator dropped on top of the colony
    /// it is meant to hunt is measuring its own placement rather than its
    /// behaviour: `predation_probe` has exactly that confound on record —
    /// two of its nine beetles start inside the nest band, so its near-nest
    /// numbers carry a caveat its far ones do not. Round-robin across
    /// compartments puts a predator in each isolated population instead,
    /// which is the placement the isolation experiment wants.
    pub fn predator_columns(&self) -> Vec<i32> {
        self.spread(self.predators)
    }

    /// Columns the grow-light fixtures are bolted at, and how far each one's
    /// pool of light reaches.
    ///
    /// Per compartment, at `lamp_spacing`, with a floor of one — a walled-off
    /// bed with no lamp in it is a dark compartment, which is a silent way to
    /// make one arm of an isolation experiment fail for a reason that has
    /// nothing to do with isolation.
    pub fn lamp_columns(&self) -> (Vec<i32>, i32) {
        let spacing = self.lamp_spacing.max(8);
        let usable: i32 = self.compartment_spans().iter().map(|(lo, hi)| hi - lo).sum();
        // **The same `spread` the founders get, not a placement of its own.**
        // Two independent even spacings across one bed interleave rather than
        // coincide: at eight of each, the old midpoint formula put every
        // fixture 3 to 25 columns off a founder, which did not matter while
        // the fixtures lit nothing and decides the stand now that they do.
        // One function means a plant station and a light station are the same
        // place by construction.
        let n = ((usable as f32 / spacing as f32).round() as usize).max(self.compartments.max(1));
        let reach = ((spacing as f32 * LAMP_REACH_FRACTION).round() as i32).max(4);
        (self.spread(n), reach)
    }

    /// The rows a fixture is recessed into.
    pub fn lamp_rows(&self) -> std::ops::Range<i32> {
        let ceiling = self.ceiling_y();
        (ceiling + CEILING - LAMP_ROWS)..(ceiling + CEILING)
    }

    /// Write one fixture's bar into the ceiling at `cx`, or take it out
    /// again, and say whether anything was written.
    ///
    /// **The one place a fixture's geometry is expressed**, so the builder
    /// and [`LabBox::move_lamp`] cannot drift apart -- a moved lamp that is
    /// one row shallower than a built one is a light difference nothing in
    /// the box would report.
    fn paint_lamp(&self, w: &mut World, cx: i32, on: bool) -> bool {
        let Some(lamp) = w.materials.id_of(LAMP_MATERIAL) else { return false };
        let walls = self.partition_columns();
        let mut wrote = false;
        for y in self.lamp_rows() {
            for x in (cx - LAMP_HALF)..=(cx + LAMP_HALF) {
                // A bar is centred in its compartment but is wider than a
                // narrow compartment, so at high wall counts its ends reach
                // the walls either side. Skipping them rather than clamping
                // the bar keeps the fixture the same size everywhere and
                // keeps every wall one unbroken column of stone — §2c's whole
                // point is that a wall is a wall.
                if x < SHELL || x >= self.width - SHELL || walls.contains(&x) {
                    continue;
                }
                if on {
                    w.set(x, y, Cell::new(lamp, ((x * 5 + y * 3) % 4) as u8));
                } else if w.get(x, y).material == lamp {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
                wrote = true;
            }
        }
        wrote
    }

    /// Columns the fixtures currently stand at, **read off the world** rather
    /// than off the spec.
    ///
    /// The spec says where the builder put them; this says where they are
    /// now, which is the only one of the two a player can change. Contiguous
    /// runs of fixture cells in the ceiling, reported by their centre.
    pub fn lamps_in(&self, world: &World) -> Vec<i32> {
        let Some(lamp) = world.materials.id_of(LAMP_MATERIAL) else { return Vec::new() };
        let y = self.lamp_rows().start;
        let mut out = Vec::new();
        let mut run: Option<(i32, i32)> = None;
        for x in 0..=self.width {
            let lit = x < self.width && world.get(x, y).material == lamp;
            match (&mut run, lit) {
                (None, true) => run = Some((x, x)),
                (Some(r), true) => r.1 = x,
                (Some(r), false) => {
                    out.push((r.0 + r.1) / 2);
                    run = None;
                }
                (None, false) => {}
            }
        }
        out
    }

    /// The fixture nearest `x`, if one is within half a bay of it — what a
    /// click on the ceiling should pick up.
    pub fn lamp_near(&self, world: &World, x: i32) -> Option<i32> {
        let reach = self.lamp_spacing.max(8) / 2;
        self.lamps_in(world).into_iter().filter(|c| (c - x).abs() <= reach).min_by_key(|c| (c - x).abs())
    }

    /// Take the fixture centred at `cx` out of the ceiling, leaving stone --
    /// the `lamps=0` control, and what a player uninstalling one does.
    pub fn remove_lamp(&self, world: &mut World, cx: i32) -> bool {
        let out = self.paint_lamp(world, cx, false);
        self.resync_enclosure(world);
        out
    }

    /// **Move the fixture centred at `from` so its centre lands at `to`.**
    ///
    /// This is the whole of the mechanic the owner asked for — *adjust plant
    /// growth by moving lights* — and it is one call: the light under the bed
    /// follows on the next field step, because `Material::beam` is gathered
    /// in `field::rebuild_blocked`'s existing scan and writing cells wakes
    /// the chunk that holds them.
    ///
    /// Returns `false` and changes nothing if `to` would put the bar outside
    /// the shell, so a drag can be attempted every frame and simply refuse at
    /// the wall. Sub-`FIELD_SCALE` moves are real: the block a fixture emits
    /// from is averaged across its CA columns, so a one-cell drag moves an
    /// eighth of a block's worth of light rather than nothing at all.
    pub fn move_lamp(&self, world: &mut World, from: i32, to: i32) -> bool {
        if to - LAMP_HALF < SHELL || to + LAMP_HALF >= self.width - SHELL {
            return false;
        }
        if !self.lamps_in(world).contains(&from) {
            return false;
        }
        self.paint_lamp(world, from, false);
        let moved = self.paint_lamp(world, to, true);
        self.resync_enclosure(world);
        moved
    }

    /// Point the room's declared fixture list at where the fixtures actually
    /// are.
    ///
    /// **The picture does not follow the physics on its own, and this is the
    /// join.** `Enclosure::lamps` is what the interior renderer paints its
    /// pools from, and the builder set it once; without this a moved fixture
    /// beams from its new column while the room stays lit under its old one —
    /// the same picture-says-one-thing-physics-says-another the whole change
    /// exists to close, reintroduced by the fix for it.
    fn resync_enclosure(&self, world: &mut World) {
        let lamps = self.lamps_in(world);
        let (_, reach) = self.lamp_columns();
        world.set_enclosure(Some(
            Enclosure::new(self.room_top(), self.ground_y).with_lamps(lamps, reach),
        ));
    }

    pub fn build(&self) -> World {
        self.build_counted().0
    }

    /// Build the box and say how much of what it asked for it got.
    ///
    /// `plant_tree_species` returns `false` for an occupied cell and for an
    /// unknown species, and it is the *only* thing that knows — so a bed that
    /// planted five of eight founders looks from the outside exactly like a
    /// bed whose other three are too small to see. `CLAUDE.md`: *"did it fire
    /// at all" needs a counter, not a picture*, and those two readings mean
    /// opposite things and call for opposite fixes.
    pub fn build_counted(&self) -> (World, Planted) {
        let mut w = World::new(crate::sim::chunk::Rect::new(0, 0, self.width - 1, self.height - 1));
        w.seed = self.seed;
        let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");
        let ceiling = self.ceiling_y();
        let bed_bottom = self.bed_bottom();

        // Soil, then the stone the bed stands on — which runs to the bottom
        // of the world, so the box fills the frame rather than floating over
        // a band of nothing. Confined stone: `structural` anchors a cell
        // walled in on every side outright, so this is a foundation and not
        // a slab waiting to come down.
        for x in 0..self.width {
            for y in self.ground_y..(self.ground_y + self.soil_depth) {
                w.set(x, y, Cell::new(soil, (crate::sim::rng::jitter(x, y) * 255.0) as u8)
                    .with_aux(material::SOIL_FIELD_CAPACITY));
            }
            for y in (self.ground_y + self.soil_depth)..bed_bottom {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }

        // The shell: side walls and a ceiling. The ceiling is the point — it
        // is what makes this a lab and not a field, and it is why the field
        // does not have to solve every tile every frame.
        for x in 0..self.width {
            for y in ceiling..(ceiling + CEILING) {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for y in ceiling..bed_bottom {
            for k in 0..SHELL {
                w.set(k, y, Cell::new(material::STONE, 0));
                w.set(self.width - 1 - k, y, Cell::new(material::STONE, 0));
            }
        }

        // Partitions, written after the bed so they cut through soil and air
        // alike — a real wall, of the same stone the floor is. Floor to
        // ceiling and one cell wide, which is enough to seal: `field.rs`
        // marks a whole `FIELD_SCALE` block blocked if *any* CA cell in it
        // is solid, so a single column stops the air as well as it stops a
        // creature. `partitions_seal_a_compartment` is the guard, and its
        // control is the same box at `compartments: 1`.
        for x in self.partition_columns() {
            for y in ceiling..bed_bottom {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }

        // **The grow lights, as objects, and now as the light.** Recessed
        // into the ceiling rather than painted on: a fixture is something the
        // player buys, places and moves.
        //
        // **What they used to be, measured: nothing.** Replacing every
        // fixture with plain stone — `labshot lamps=0`, which is the control
        // — left the light at the bench and the whole stand *byte-identical*
        // at every frame sampled. They were `crystal`, whose `Material::glow`
        // seeds the light channel of its own field block and then relies on
        // `LIGHT_DECAY` to spread it, which `field.rs` holds to "a handful of
        // blocks"; the bench is nineteen blocks below the ceiling. So the
        // crop lived on sky light through the shell while the picture said
        // grow lights.
        //
        // `growlamp` closes that: `Material::beam` rides the sun's own column
        // descent, so a fixture lights the bed under it and clear air passes
        // it undimmed. Together with the sunless declaration below, the
        // picture and the physics now agree about *where* the light comes
        // from as well as how much of it there is.
        let (lamps, lamp_reach) = self.lamp_columns();
        for &cx in &lamps {
            self.paint_lamp(&mut w, cx, true);
        }

        // **This is a room, and the renderer is to draw it as one.** Purely
        // a statement about the scene — no simulation pass reads it. Without
        // it the air in a sealed box takes `cell_colour`'s most expensive
        // branch to paint a dusk gradient, a horizon band and a star hash
        // over a laboratory (guide §2).
        w.set_enclosure(Some(
            Enclosure::new(self.room_top(), self.ground_y).with_lamps(lamps, lamp_reach),
        ));

        // **A grow light, not a sun** — and since 2026-08-30 that is a
        // statement about the physics and not only about the picture. The
        // design of record's §2 is "the lab has a ceiling, not a sky"; it had
        // both, and the sky was winning, passing 0.447 of full daylight
        // through four rows of stone to do the whole job the fixtures were
        // drawn doing. `set_sky_lighting(false)` is that sentence.
        //
        // The hold stays, and is not vestigial: it pins `sky_temperature` and
        // the renderer's own day tint, and it is what `noon_equivalent_light`
        // normalises the crop's readings by. Calm air.
        w.set_sky_lighting(false);
        w.set_sky_hold(Some(Self::noon()));
        w.set_weather_pin(Pin::Clear);

        let mut planted = 0usize;
        for x in self.founder_columns() {
            planted += usize::from(w.plant_tree_species(x, self.ground_y - 2, &self.species));
        }
        let mut ants = 0usize;
        for x in self.colony_columns() {
            ants += w.found_colony(x, self.ground_y - 2);
        }
        // **Predators last, so they are placed into a bed that already has
        // its prey and its plants in it.** `plant_creature_seed` refuses a
        // site its body does not fit, and a 2x2 beetle needs more clearance
        // than a two-cell ant -- so a refusal here is a real fact about the
        // bed rather than an ordering artifact, and `Planted::beetles` is
        // the only place it is visible.
        let mut beetles = 0usize;
        for x in self.predator_columns() {
            if let Some(site) = crate::sim::creature::plant_creature_seed(&mut w, x, self.ground_y - 2, "beetle") {
                w.schedule_active_site(site);
                beetles += 1;
            }
        }
        (w, Planted { asked: self.founders, planted, ants, beetles })
    }
}

/// What `build` managed to place, against what it was asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Planted {
    pub asked: usize,
    pub planted: usize,
    /// Ants actually placed across every colony. `found_colony` refuses a
    /// site with no footing and says so only through this number.
    pub ants: usize,
    /// Beetles actually released. Below `predators` means the bed refused a
    /// site — a 2x2 rigid body needs clearance a two-cell chain does not,
    /// so this can fall short where `ants` did not, and a predation result
    /// read off a bed with no predator in it is the null wearing a result's
    /// clothes.
    pub beetles: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Every cell reachable from `start` through non-solid cells, four ways.
    /// A partition seals iff this cannot leave the compartment it starts in.
    fn flood(world: &World, start: (i32, i32), bounds: (i32, i32, i32, i32)) -> HashSet<(i32, i32)> {
        let (x0, y0, x1, y1) = bounds;
        let mut seen = HashSet::new();
        let mut stack = vec![start];
        while let Some((x, y)) = stack.pop() {
            if x < x0 || x > x1 || y < y0 || y > y1 || !seen.insert((x, y)) {
                continue;
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < x0 || nx > x1 || ny < y0 || ny > y1 {
                    continue;
                }
                if world.get(nx, ny).material == material::EMPTY {
                    stack.push((nx, ny));
                }
            }
        }
        seen
    }

    #[test]
    fn partitions_seal_a_compartment_and_an_open_box_does_not() {
        // The guard and its own control, in one test: a wall that seals is
        // only evidence if the same flood *does* cross the box without one.
        let open = LabBox { compartments: 1, founders: 0, colonies: 0, ..LabBox::default() };
        let walled = LabBox { compartments: 4, ..open.clone() };
        let bounds = |b: &LabBox| (0, b.room_top(), b.width - 1, b.ground_y - 1);
        let start = |b: &LabBox| (SHELL + 4, b.room_top() + 2);

        let w = open.build();
        let reached = flood(&w, start(&open), bounds(&open));
        assert!(
            reached.contains(&(open.width - SHELL - 4, open.room_top() + 2)),
            "the control must cross an open box, or a sealed reading proves nothing"
        );

        let w = walled.build();
        let reached = flood(&w, start(&walled), bounds(&walled));
        for wall in walled.partition_columns() {
            assert!(
                !reached.contains(&(wall + 2, walled.room_top() + 2)),
                "air escaped past the partition at x={wall}; a wall with a gap measures as a weak effect, not as a broken wall"
            );
        }
    }

    #[test]
    fn partitions_seal_the_soil_as_well_as_the_air() {
        // A burrow under a partition is the same hole as a gap over it. Swept
        // to 32 compartments because that is where a wall gets narrower than
        // a lamp bar is wide, and the bar is written *after* the wall — at 4
        // compartments the two never meet, so a spot check would say nothing
        // about the case that can actually break.
        let stone = material::STONE;
        for compartments in [2usize, 4, 8, 16, 32] {
            let b = LabBox { compartments, founders: 0, colonies: 0, ..LabBox::default() };
            let w = b.build();
            for wall in b.partition_columns() {
                for y in b.ceiling_y()..b.bed_bottom() {
                    assert_eq!(
                        w.get(wall, y).material,
                        stone,
                        "the partition at x={wall} is not stone at y={y} ({compartments} compartments)"
                    );
                }
            }
        }
    }

    #[test]
    fn nothing_is_ever_placed_on_a_partition() {
        // §2c's recorded scene error: a fan at `width / 2` sits exactly on
        // the wall at every power-of-two compartment count, and containment
        // then measured as absent. Swept rather than spot-checked, because
        // the collision is a coincidence of two arithmetics and which counts
        // collide is not guessable.
        for compartments in 1..=16usize {
            for founders in [1usize, 2, 3, 5, 8, 16] {
                let b = LabBox { compartments, founders, colonies: founders, ..LabBox::default() };
                let walls: HashSet<i32> = b.partition_columns().into_iter().collect();
                for x in b.founder_columns() {
                    assert!(!walls.contains(&x), "founder on the partition at x={x} ({compartments} compartments, {founders} founders)");
                }
                for x in b.colony_columns() {
                    assert!(!walls.contains(&x), "colony on the partition at x={x} ({compartments} compartments, {founders} founders)");
                }
                for x in b.lamp_columns().0 {
                    assert!(!walls.contains(&x), "lamp on the partition at x={x} ({compartments} compartments)");
                }
            }
        }
    }

    #[test]
    fn every_compartment_gets_a_lamp() {
        // A dark compartment fails for a reason that has nothing to do with
        // the thing an isolation experiment is testing.
        for compartments in [1usize, 2, 4, 8, 16] {
            let b = LabBox { compartments, ..LabBox::default() };
            let (lamps, _) = b.lamp_columns();
            for (lo, hi) in b.compartment_spans() {
                assert!(
                    lamps.iter().any(|&x| x >= lo && x < hi),
                    "compartment [{lo},{hi}) has no grow light at {compartments} compartments"
                );
            }
        }
    }

    #[test]
    fn every_founder_asked_for_is_actually_planted() {
        // "Five of eight visible" is either germination or size, and those
        // mean opposite things. This settles the half that `build` owns.
        for compartments in [1usize, 2, 4, 8] {
            let b = LabBox { compartments, founders: 8, colonies: 0, ..LabBox::default() };
            let (_, planted) = b.build_counted();
            assert_eq!(planted.planted, planted.asked, "at {compartments} compartments");
        }
    }

    #[test]
    fn a_fixture_does_not_shade_its_own_light() {
        // **The half of "the shell is no longer a light knob" that can
        // actually fail.** The first version of this guard buried the box
        // under three more blocks of stone and asserted the bench did not
        // move — and it passed with the sun switched back *on*, because the
        // fixtures out-shine the leak either way, so it could not see the
        // fault it was named for. `CLAUDE.md`: if a guard does not go red
        // when you put the fault back it is blind, not weak, and the fix is
        // to replace it rather than widen its assertion.
        //
        // What decides whether shell thickness is a light knob is one line in
        // `field::apply_sky_to`: the descent attenuates *then* re-seeds the
        // beam, so a fixture emits from its face rather than through its
        // housing. Reverse that and a lamp set into four rows of stone loses
        // 55% of its output, and every extra row costs again. So: fill the
        // rest of the fixture's own light block with stone and require the
        // bench not to notice. Injecting the reversed order takes this arm to
        // roughly a fifth of the other, which is the sensitivity the buried
        // version never had.
        let b = LabBox::default();
        let bench_light = |encase: bool| {
            let mut w = b.build();
            if encase {
                // From under the bar to the bottom of the block it sits in.
                let block_end = (b.lamp_rows().start / field::FIELD_SCALE + 1) * field::FIELD_SCALE;
                for x in 0..b.width {
                    for y in b.lamp_rows().end..block_end {
                        w.set(x, y, Cell::new(material::STONE, 0));
                    }
                }
            }
            for _ in 0..240 {
                w.step_fields();
            }
            let cols = b.founder_columns();
            cols.iter().map(|&x| w.field_at(x, b.ground_y - 2).light).sum::<f32>()
                / cols.len().max(1) as f32
                / field::MAX_LIGHT
        };
        let bare = bench_light(false);
        let encased = bench_light(true);
        assert!(bare > 0.30, "the bench is at {bare:.3} of full light -- the fixtures are not delivering");
        assert!(
            (encased - bare).abs() < 0.01,
            "packing the fixture's own block with stone moved the bench {bare:.3} -> {encased:.3}; \
             the lamps are shining through their own housing, so how thick the shell is decides \
             how much light the crop gets again"
        );
    }

    #[test]
    fn the_lamps_are_what_light_the_bed() {
        // **The measurement that started this work, run the other way
        // round.** `labshot lamps=0` used to come back *byte-identical*: the
        // fixtures contributed nothing and the shell's leak was the whole
        // light budget. Pulling every fixture out must now put the bed in the
        // dark, and this is the only thing in the lane that would notice if
        // it stopped being true.
        //
        // The lit arm is the positive control the same assertion needs: a
        // bench that reads dark in both arms passes "dark without lamps" for
        // the wrong reason.
        let b = LabBox::default();
        let bench = |lamps: bool| {
            let mut w = b.build();
            if !lamps {
                for cx in b.lamps_in(&w) {
                    b.paint_lamp(&mut w, cx, false);
                }
            }
            for _ in 0..240 {
                w.step_fields();
            }
            let cols = b.founder_columns();
            cols.iter().map(|&x| w.field_at(x, b.ground_y - 2).light).sum::<f32>()
                / cols.len().max(1) as f32
                / field::MAX_LIGHT
        };
        let lit = bench(true);
        let dark = bench(false);
        assert!(lit > 0.30, "the fixtures are not lighting the bench ({lit:.3} of full light)");
        assert!(dark < 0.02, "the bed is still lit with every fixture removed ({dark:.3}) -- something other than the lamps is lighting it");
    }

    #[test]
    fn a_lamp_lights_the_bed_under_it_and_moving_it_moves_the_light() {
        // **The mechanic, as an assertion.** Owner, 2026-08-30: *it would be
        // fun to adjust plant growth by moving lights.* That is only true if
        // where a fixture sits decides what is bright, so: one fixture, one
        // reading under it and one a bay away, then move it and the two swap.
        //
        // A paired swap rather than two absolute thresholds, because the
        // claim is *the light followed the lamp* and a box that simply got
        // brighter or dimmer everywhere would satisfy thresholds.
        let b = LabBox { founders: 0, colonies: 0, lamp_spacing: 512, ..LabBox::default() };
        let mut w = b.build();
        let lamps = b.lamps_in(&w);
        assert_eq!(lamps.len(), 1, "this test wants exactly one fixture, got {lamps:?}");
        let (home, away) = (lamps[0], lamps[0] + 128);
        let settle = |w: &mut World| {
            for _ in 0..240 {
                w.step_fields();
            }
        };
        settle(&mut w);
        let under_before = w.field_at(home, b.ground_y - 2).light;
        let away_before = w.field_at(away, b.ground_y - 2).light;
        assert!(b.move_lamp(&mut w, home, away), "the fixture refused to move");
        settle(&mut w);
        let under_after = w.field_at(home, b.ground_y - 2).light;
        let away_after = w.field_at(away, b.ground_y - 2).light;
        assert!(
            under_before > away_before * 4.0,
            "the bed is not brighter under the fixture than a bay away ({under_before:.3} against {away_before:.3})"
        );
        assert!(
            away_after > under_after * 4.0,
            "moving the fixture did not move the light ({under_after:.3} under where it was, {away_after:.3} where it went)"
        );
    }

    #[test]
    fn the_crop_is_actually_lit_where_the_founders_stand() {
        // **The channel has a writer and a reader, and this checks the pair.**
        // A sealed box has a stone ceiling, and sky light is stopped by
        // solids — so "the box is lit" is a claim about the *field*, not
        // about the sky hold, and nothing else in the lane would notice if
        // it were false. `herb`'s `Germinate` needs `light_threshold: 0.1`,
        // so an unlit bed is a bed where nothing ever starts, which reads on
        // a contact sheet as "the founders are too small to see".
        //
        // Its own control is the row *under the floor*, which no light of
        // any kind should reach: without that, a reading of "lit everywhere"
        // would pass whether the field were working or stuck at a constant.
        let b = LabBox::default();
        let mut w = b.build();
        for _ in 0..240 {
            w.step_fields();
        }
        let threshold = 0.1 * field::MAX_LIGHT;
        for x in b.founder_columns() {
            let lit = w.field_at(x, b.ground_y - 2).light;
            assert!(
                lit >= threshold,
                "the bed is dark at x={x}: light {lit} against herb's germination threshold {threshold}"
            );
            // **And the shell is really in the way.** Bounded on both sides
            // because a one-sided "bright enough" passes just as happily on a
            // field pinned at its maximum everywhere, which is what a broken
            // sky walk looks like.
            //
            // The obvious control — "no light under the stone floor" — is
            // *wrong here*, and it was written and watched fail before this
            // was: solid field blocks carry the light arriving at them on
            // purpose (`dead-ends.md`, the occluder-light reversal: "the
            // light arriving at an occluder is what it intercepts, and a leaf
            // is an occluder"), so a reading inside rock is neither zero nor
            // meaningless. Sensitivity to the shell is
            // `the_ceiling_is_thin_enough_to_grow_under`'s job, and it is a
            // paired comparison rather than a bound.
            assert!(
                lit < 0.9 * field::MAX_LIGHT,
                "the bed at x={x} reads {lit} of {} — a sealed box under a stone ceiling cannot be at full daylight, so this is a constant rather than a measurement",
                field::MAX_LIGHT
            );
        }
    }

    #[test]
    fn the_box_declares_itself_a_room_and_says_where_its_lights_are() {
        let b = LabBox::default();
        let mut w = b.build();
        let e = w.enclosure().expect("a sealed box is an enclosure or it draws as sky").clone();
        assert_eq!(e.ceiling_y, b.room_top());
        assert_eq!(e.floor_y, b.ground_y);
        assert_eq!(e.lamps, b.lamp_columns().0);
        // ...and the fixtures are really in the ceiling, not just in the
        // table. A painted lamp over an empty ceiling is the channel with a
        // reader and no writer.
        let lamp = w.materials.id_of(LAMP_MATERIAL).expect("the fixture material is compiled in");
        for &x in &e.lamps {
            assert_eq!(w.get(x, b.room_top() - 1).material, lamp, "no fixture over the pool at x={x}");
        }
        // **And the table follows the fixture when it moves**, which is the
        // half a build-time check cannot see: `Enclosure::lamps` is set once
        // by the builder, so a moved lamp beaming from its new column while
        // the room stays lit under its old one is exactly the
        // picture-disagrees-with-physics defect this whole change closes,
        // reintroduced by the fix for it.
        let home = e.lamps[0];
        assert!(b.move_lamp(&mut w, home, home + 24));
        let moved = w.enclosure().expect("still a room").lamps.clone();
        assert!(!moved.contains(&home), "the room is still lit where the fixture used to be: {moved:?}");
        assert!(moved.contains(&(home + 24)), "the room is not lit where the fixture went: {moved:?}");
    }

    /// **The bed's floor has to clear the control bar, and the visible bed
    /// has to be mostly dirt.** Nothing else in the tree relates the bed's
    /// geometry to the interface's, and the owner's complaint was precisely
    /// a ratio he could see: 40 rows of soil under 90 visible rows of stone.
    ///
    /// **Two assertions rather than an equality, because the bar is moving.**
    /// It is one row today and two rows on the interface lane's branch — 30
    /// rows against 56 — so an equality would be green against whichever copy
    /// of `ui.rs` happened to be in the tree and red against the other, which
    /// is how a guard ends up asserting another lane's unlanded work. What
    /// does not depend on which bar is in the tree is: the floor must not be
    /// *under* the bar (soil the box simulates and nobody can see, §2a's own
    /// waste), and dirt must outweigh visible cement.
    ///
    /// If this fails, someone has to decide whether the soil or the stone
    /// gives up the rows, and re-run `examples/labsoil` if it is the soil.
    #[test]
    fn the_visible_bed_is_mostly_soil_and_its_floor_clears_the_control_bar() {
        let b = LabBox::default();
        let bar_top = crate::lab::ui::bar_top();
        let floor_bottom = b.ground_y + b.soil_depth + FLOOR_ROWS;
        assert!(
            floor_bottom <= bar_top,
            "the bed's floor ends at row {floor_bottom}, below the bar at {bar_top}: the box is \
             running `update_soil_water` over rows the interface covers. The bar has grown — \
             take the rows off the soil"
        );

        // The ratio the owner was actually looking at, off the built world
        // rather than off the constants: everything from the surface to the
        // bar, split into dirt and cement.
        let w = b.build();
        let soil = w.materials.id_of("soil").expect("soil is compiled in");
        let x = b.width / 4; // clear of the colony founded at the centre.
        let dirt = (b.ground_y..bar_top).filter(|&y| w.get(x, y).material == soil).count();
        let cement = (b.ground_y..bar_top).count() - dirt;
        assert!(
            dirt >= cement * 2,
            "{dirt} rows of dirt against {cement} of visible cement between the surface and the \
             bar. The owner's complaint was that this was 40 against 90"
        );
    }

    /// **The predator knob places predators, and zero places none.**
    ///
    /// The failure this is written against is the one `CLAUDE.md` calls a
    /// knob with a reader and no writer: a field that parses, stores,
    /// round-trips through `read_bed`/`write_bed` and draws a dial, while
    /// nothing downstream ever looks at it. Every readout stays plausible
    /// and the bed is simply never hunted -- and a predation result read off
    /// an unhunted bed is a null wearing a result's clothes.
    ///
    /// So this asserts the **far side of the call**: beetle cells standing
    /// in the built world, counted off the material table, rather than the
    /// `Planted` figure the placer reports about itself.
    #[test]
    fn the_predator_knob_puts_beetles_in_the_bed() {
        let beetle_cells = |w: &World, b: &LabBox| -> usize {
            let id = w.materials.id_of("beetle").expect("beetle material is compiled in");
            let mut n = 0;
            for y in 0..b.height {
                for x in 0..b.width {
                    if w.get(x, y).material == id {
                        n += 1;
                    }
                }
            }
            n
        };

        // No plants and no ants: what is varied is the predator and nothing
        // else, so a difference cannot be something the colony did.
        let none = LabBox { founders: 0, colonies: 0, predators: 0, ..LabBox::default() };
        let (w0, p0) = none.build_counted();
        assert_eq!(p0.beetles, 0, "a bed asked for no predators reported placing some");
        assert_eq!(beetle_cells(&w0, &none), 0, "a bed asked for no predators has beetle cells standing in it");

        let some = LabBox { predators: 4, ..none.clone() };
        let (w1, p1) = some.build_counted();
        assert!(p1.beetles > 0, "predators=4 placed nothing; the knob is stored and never read");
        // A beetle is a four-cell rigid body, so cells outnumber animals.
        // Asserting on cells rather than on the count is what makes this the
        // far side of the placer rather than the placer's own claim.
        let cells = beetle_cells(&w1, &some);
        assert!(cells >= p1.beetles, "reported {} beetles but only {cells} beetle cells stand in the world", p1.beetles);
    }
}
