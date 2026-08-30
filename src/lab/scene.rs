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
//!   `DEFAULT_SOIL_DEPTH` is set from a measurement of where the founder
//!   species' roots actually stop, not from a round number.
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
/// §2a: 40 → 240 rows costs **1.9x the frame for a byte-identical stand**,
/// because herb's roots never reach past 40 — so a deep bed is 1.9x for
/// decoration and the obligation is that something reaches the depth being
/// paid for. Measured in *this* bed with `labshot roots=1`, 10,800 frames,
/// eight founders: the deepest root cell in the stand sits **26 rows** below
/// the surface, and the deepest of the whole run is what this is set from.
///
/// 48 is that measurement plus room for the two consumers that are not the
/// founder herb: a deeper-rooting mutant, which §2a wants to be a visible
/// evolutionary win rather than a purchase, and a burrow, which §2a calls
/// the stronger consumer because it needs no evolution to arrive. Both need
/// somewhere to go that the shipped herb is not already using.
///
/// It is deliberately **not** 80, which is what this scene shipped with:
/// that was 54 rows of soil nothing in the box has ever entered, at the
/// frame cost §2a measured and with nothing to show for it.
pub const DEFAULT_SOIL_DEPTH: i32 = 48;

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

/// Rows of stone under the soil, so the bed has a floor to sit on rather
/// than falling out of the world — the scene error `PlantScene` records
/// having paid for twice.
const FLOOR_ROWS: i32 = 8;
/// Thickness of the side walls and the floor edges.
const SHELL: i32 = 4;
/// Thickness of the ceiling, which is thicker than the walls **so that a
/// fixture can be recessed into it and still be a fixture**. At the wall's
/// four rows a lamp bar is two pixels tall at the very top of the frame,
/// which is not something anyone reads as a light.
const CEILING: i32 = 7;
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

    /// The row the bed's stone floor ends at.
    fn bed_bottom(&self) -> i32 {
        self.ground_y + self.soil_depth + FLOOR_ROWS
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

        // Soil, then the stone floor under it.
        for x in 0..self.width {
            for y in self.ground_y..(self.ground_y + self.soil_depth) {
                w.set(x, y, Cell::new(soil, (crate::sim::rng::jitter(x, y) * 255.0) as u8)
                    .with_aux(material::SOIL_FIELD_CAPACITY));
            }
            let floor = self.ground_y + self.soil_depth;
            for y in floor..(floor + FLOOR_ROWS) {
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

        // **The grow lights, as objects.** Recessed into the underside of the
        // ceiling rather than painted on: a fixture is something the player
        // will eventually move, switch and pay for, and `Material::glow`
        // means these seed the light channel the plants actually
        // photosynthesise from. `crystal` is the shipped glowing solid
        // (`glow: 1.8` against `field::MAX_LIGHT` 4.0) and reads at this
        // scale as a cold bar of light in the ceiling.
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
        }
        let buried = w.field_at(b.width / 2, b.ground_y + b.soil_depth + 4).light;
        assert!(
            buried < threshold,
            "light reached under the bed's stone floor ({buried}) — the reading above is a constant, not a measurement"
        );
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
}
