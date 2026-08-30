//! **The shell a world is inside, when it is a room rather than a country.**
//!
//! Pure geometry — where the ceiling is, where the bench is, and where the
//! grow lights are bolted. It carries no colours: what an interior *looks
//! like* belongs to [`crate::sky::Interior`], exactly as what open air looks
//! like belongs to `sky::Sky`, and that split is the only thing keeping `sim`
//! from depending on the renderer.
//!
//! # Why this is on the world rather than on the renderer
//!
//! `Renderer::draw` takes `&World` and nothing else, so a scene that declares
//! itself an interior is drawn as one by *every* caller — the lab binary,
//! `labshot`, a test — with no wiring at any of them. That is the same route
//! the held grow light already takes (`World::sky_frame`, read by
//! `Renderer::draw` rather than passed to it). The alternative, a flag on the
//! `Renderer`, needs every call site changed and draws a lab as open country
//! wherever one is missed — and the outdoor game has one of those call sites
//! per binary.
//!
//! # Why the lab needs it at all
//!
//! `Reports/evolution-lab-design-guide-2026-08-30.md` §2: empty sky is
//! **27.4 ns/px against stone's 6.7**, so the air in a sealed box is the most
//! expensive thing on the screen and it is drawing the one picture the lab
//! does not want — a dusk gradient with a horizon band and a star hash, over
//! a room. The guide's line is *"whatever fills the air above the soil must
//! not draw as sky"*, and this is what says so.

/// Where the walls, the bench and the lamps are. Set on a world by whatever
/// built the room; read by the renderer to decide what its empty space is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Enclosure {
    /// First row of *air* under the ceiling — the top of the room, not the
    /// top of the shell.
    pub ceiling_y: i32,
    /// The row things stand on: the top of the soil. Air below this is dug
    /// space — a burrow — and draws as earth rather than as wall.
    pub floor_y: i32,
    /// Column centres of the grow-light fixtures, left to right. Empty is a
    /// legal, meaningful state: an unlit room.
    pub lamps: Vec<i32>,
    /// How far to either side of a fixture its pool of light reaches, in
    /// columns. Separate from the fixture's own width because a lamp is a
    /// small object throwing a wide pool.
    pub lamp_reach: i32,
}

impl Enclosure {
    /// The room's air, top to bottom, with no lamps in it.
    pub fn new(ceiling_y: i32, floor_y: i32) -> Self {
        Self { ceiling_y, floor_y, lamps: Vec::new(), lamp_reach: 0 }
    }

    /// Bolt fixtures in at these columns, each throwing a pool `reach` wide
    /// to either side.
    pub fn with_lamps(mut self, lamps: Vec<i32>, reach: i32) -> Self {
        self.lamps = lamps;
        self.lamp_reach = reach.max(1);
        self
    }

    /// How strongly a lamp lights this column, `0.0..=1.0`.
    ///
    /// A cosine shoulder rather than a linear one: a linear falloff leaves a
    /// visible crease where two pools meet, which reads as a seam in the wall
    /// rather than as two lights.
    pub fn lamp_weight(&self, x: i32) -> f32 {
        if self.lamps.is_empty() {
            return 0.0;
        }
        let reach = self.lamp_reach.max(1) as f32;
        let nearest = self.lamps.iter().map(|l| (l - x).abs()).min().unwrap_or(i32::MAX) as f32;
        if nearest >= reach {
            return 0.0;
        }
        let t = nearest / reach;
        (1.0 - t * t) * (1.0 - t * t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_room_with_no_lamps_is_unlit_everywhere() {
        let e = Enclosure::new(4, 160);
        for x in [-50, 0, 7, 256, 4096] {
            assert_eq!(e.lamp_weight(x), 0.0, "an unlit room must have no pool of light at {x}");
        }
    }

    #[test]
    fn a_lamps_pool_is_brightest_under_it_and_gone_at_its_reach() {
        let e = Enclosure::new(4, 160).with_lamps(vec![100], 20);
        assert_eq!(e.lamp_weight(100), 1.0);
        assert!(e.lamp_weight(110) > 0.0 && e.lamp_weight(110) < 1.0);
        assert_eq!(e.lamp_weight(120), 0.0, "the pool must end at its reach, or every column is lit");
        assert_eq!(e.lamp_weight(80), e.lamp_weight(120), "and it must be symmetric");
    }

    #[test]
    fn two_lamps_light_the_column_between_them_from_the_nearer_one() {
        let e = Enclosure::new(4, 160).with_lamps(vec![100, 140], 30);
        // Nearest wins, so the trough between two overlapping pools sits at
        // the midpoint and is the same from either side.
        assert_eq!(e.lamp_weight(115), e.lamp_weight(125));
        assert!(e.lamp_weight(120) < e.lamp_weight(105), "the midpoint must be the dimmest place between two lamps");
        assert!(e.lamp_weight(120) > 0.0, "...but still lit, or the pools do not join");
    }
}
