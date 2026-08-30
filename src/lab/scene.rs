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
    /// Columns between grow-light fixtures. A spacing rather than a count so
    /// that a 4096-wide bed is lit like a 512-wide one instead of being lit
    /// eight times more thinly, and so every compartment gets at least one
    /// fixture however many compartments there are.
    pub lamp_spacing: i32,
    pub seed: u64,
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
/// **Today's shipped bar is one row, 30 rows tall**, so until the second row
/// lands there is a 34-row rim of stone between the floor and the bar
/// instead of 8. That is the right way round to be wrong: a rim is stone
/// the player can see and a *buried* floor is soil the box pays
/// `update_soil_water` for and nobody can look at, which is exactly the
/// waste §2a names.
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
            lamp_spacing: 128,
            seed: 1,
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
/// **Four, and it is not a free choice: the shell's thickness is a light
/// knob.** `field.rs` casts sky light down each CA column and passes
/// `SKY_TRANSMISSION^(depth / FIELD_SCALE)` — `0.2^(depth/8)` — so a ceiling
/// four rows deep passes **0.447** of what falls on it and one seven rows
/// deep passes **0.245**. This was found the expensive way: a ceiling
/// thickened from 4 to 7 rows to make room for a recessed lamp took the
/// light at the bench from 0.40 to **0.22** of `field::MAX_LIGHT` and the
/// stand at frame 3,600 from 474 plant cells to **286**, seed set 12 to
/// **0**. Nothing failed, nothing germinated late and no test went red — the
/// crop simply grew less, which reads exactly like a species problem.
///
/// So the shell is as thin as it can be while still being a shell, and
/// thickening it is a decision about how much light the crop gets rather
/// than about how solid the box looks.
/// `the_ceiling_is_thin_enough_to_grow_under` is the guard.
const CEILING: i32 = 4;
/// Half-width of a grow-light fixture, in columns.
const LAMP_HALF: i32 = 7;
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
        if self.compartments <= 1 {
            return Vec::new();
        }
        (1..self.compartments)
            .map(|k| (self.width * k as i32) / self.compartments as i32)
            .collect()
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

    /// Columns the grow-light fixtures are bolted at, and how far each one's
    /// pool of light reaches.
    ///
    /// Per compartment, at `lamp_spacing`, with a floor of one — a walled-off
    /// bed with no lamp in it is a dark compartment, which is a silent way to
    /// make one arm of an isolation experiment fail for a reason that has
    /// nothing to do with isolation.
    pub fn lamp_columns(&self) -> (Vec<i32>, i32) {
        let spacing = self.lamp_spacing.max(8);
        let mut out = Vec::new();
        for (lo, hi) in self.compartment_spans() {
            let n = (((hi - lo) as f32 / spacing as f32).round() as i32).max(1);
            for j in 0..n {
                out.push(lo + (hi - lo) * (2 * j + 1) / (2 * n));
            }
        }
        out.sort_unstable();
        let reach = ((spacing as f32 * LAMP_REACH_FRACTION).round() as i32).max(4);
        (out, reach)
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

        // **The grow lights, as objects.** Recessed into the ceiling rather
        // than painted on: a fixture is something the player will eventually
        // move, switch and pay for. `crystal` is the shipped glowing solid
        // (`glow: 1.8` against `field::MAX_LIGHT` 4.0) and reads at this
        // scale as a cold bar of light in the ceiling.
        //
        // **What they are not, measured: the crop's light source.** Replacing
        // every fixture with plain stone — `labshot lamps=0`, which is the
        // control — leaves the light at the bench and the whole stand
        // *byte-identical* at every frame sampled. `Material::glow` seeds the
        // light channel of its own field block and then decays at
        // `LIGHT_DECAY` per step, which `field.rs` describes as reaching "a
        // handful of blocks"; the bench is about nineteen blocks below the
        // ceiling. So the crop lives on sky light coming through the shell,
        // and the fixtures are how the room *reads* the schedule rather than
        // how it is lit. Both are driven by the same held frame, so the
        // picture and the physics agree about how much light there is and
        // disagree only about where it comes from -- worth closing, and not
        // by making one lamp brighter.
        let (lamps, lamp_reach) = self.lamp_columns();
        let walls = self.partition_columns();
        if let Some(crystal) = w.materials.id_of("crystal") {
            for &cx in &lamps {
                for y in (ceiling + CEILING - LAMP_ROWS)..(ceiling + CEILING) {
                    for x in (cx - LAMP_HALF)..=(cx + LAMP_HALF) {
                        // A bar is centred in its compartment but is wider
                        // than a narrow compartment, so at high wall counts
                        // its ends reach the walls either side. Skipping
                        // them rather than clamping the bar keeps the
                        // fixture the same size everywhere and keeps every
                        // wall one unbroken column of stone — §2c's whole
                        // point is that a wall is a wall.
                        if x >= SHELL && x < self.width - SHELL && !walls.contains(&x) {
                            w.set(x, y, Cell::new(crystal, ((x * 5 + y * 3) % 4) as u8));
                        }
                    }
                }
            }
        }

        // **This is a room, and the renderer is to draw it as one.** Purely
        // a statement about the scene — no simulation pass reads it. Without
        // it the air in a sealed box takes `cell_colour`'s most expensive
        // branch to paint a dusk gradient, a horizon band and a star hash
        // over a laboratory (guide §2).
        w.set_enclosure(Some(
            Enclosure::new(self.room_top(), self.ground_y).with_lamps(lamps, lamp_reach),
        ));

        // A grow light, not a sun, and calm air.
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
        (w, Planted { asked: self.founders, planted, ants })
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
    fn the_ceiling_is_thin_enough_to_grow_under() {
        // **The shell's thickness is a light knob, and this is the only
        // thing that says so.** `field.rs` passes `0.2^(depth/8)` down a
        // column, so three extra rows of ceiling cost 45% of the light and
        // the only symptom is a smaller stand — no failure, no late
        // germination, nothing red. Measured on this build: 4 rows gives
        // 0.40 of `MAX_LIGHT` at the bench and 7 rows gives 0.22.
        //
        // Asserted as a *paired* comparison against a deliberately thicker
        // shell rather than against a remembered constant, so it survives
        // a retune of `SKY_TRANSMISSION` and still catches the thing it is
        // named for. The thick arm is the positive control: if it does not
        // come out dimmer, this test cannot see ceiling thickness at all.
        let thin = LabBox::default();
        let thick = LabBox { ground_y: thin.ground_y + 3, ..thin.clone() };

        let bench_light = |b: &LabBox, extra: i32| {
            let mut w = b.build();
            if extra > 0 {
                // Three more rows of stone under the ceiling, and nothing
                // else changed.
                for x in 0..b.width {
                    for y in b.room_top()..(b.room_top() + extra) {
                        w.set(x, y, Cell::new(material::STONE, 0));
                    }
                }
            }
            for _ in 0..240 {
                w.step_fields();
            }
            let cols = b.founder_columns();
            let sum: f32 = cols.iter().map(|&x| w.field_at(x, b.ground_y - 2).light).sum();
            sum / cols.len().max(1) as f32 / field::MAX_LIGHT
        };

        let open = bench_light(&thin, 0);
        let buried = bench_light(&thick, 3);
        assert!(buried < open * 0.8, "the control failed: three more rows of ceiling must measurably dim the bench ({buried:.3} against {open:.3})");
        assert!(
            open > 0.30,
            "the bench is at {open:.3} of full light -- the shell has been thickened, and the only symptom is a smaller stand"
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
        let w = b.build();
        let e = w.enclosure().expect("a sealed box is an enclosure or it draws as sky");
        assert_eq!(e.ceiling_y, b.room_top());
        assert_eq!(e.floor_y, b.ground_y);
        assert_eq!(e.lamps, b.lamp_columns().0);
        // ...and the fixtures are really in the ceiling, not just in the
        // table. A painted lamp over an empty ceiling is the channel with a
        // reader and no writer.
        let crystal = w.materials.id_of("crystal").expect("crystal is compiled in");
        for &x in &e.lamps {
            assert_eq!(w.get(x, b.room_top() - 1).material, crystal, "no fixture over the pool at x={x}");
        }
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
}
