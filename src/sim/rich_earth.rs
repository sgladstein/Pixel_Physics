//! **Rich earth: the colony's finished garden feeds the ground round it.**
//!
//! Built 2026-10-31 for the plants lane, from the ants-help-plants design
//! (<https://claude.ai/artifact/UT4mXFWbSiL92FzSB69JJW>), Scott's ask "how
//! ants can actually help the plants grow, instead of just getting them to
//! survive together". Real leaf-cutter ants are the inspiration: the
//! fertility of their nests sits in the spent garden and the refuse, and
//! plants that reach it grow better. Here:
//!
//! 1. **It is made where the ants eat the garden.** When an ant eats a
//!    `fungus` cell (the one bite that removes a cell, `creature.rs`'s
//!    `FOOD_SWALLOW` site), every nutrient-carrying neighbour --
//!    soil, packed soil, spoil (`plant::cell_carries_nutrient`) -- is topped
//!    up to `RICH_STOCK` root draws of richness, carried on the cell
//!    ([`Cell::richness`]). So the stock is bounded by what the colony
//!    actually ate, and the walls of a farmed room go dark.
//! 2. **It rides with the grain.** The richness is on the cell, so packed
//!    soil an ant later cuts out as spoil carries it to the heap, and a
//!    bank that slumps takes it down with it.
//! 3. **A root in it is fed, and fed past full** (`RichEarth::Bonus`, the
//!    default). A rich face reads as full soil, and a plant's income is
//!    multiplied by up to `1 + RICH_BONUS` (double) for the share of its nutrient
//!    faces that are rich (`OrganismState::rich_share`). "It can be rich"
//!    (Scott, 2026-10-31): on the played bed nutrient is almost never what
//!    limits a plant (income lost to it: grass 2.3%, herb 1.7%, shrub
//!    0.1%, tree 2.0%, scrambler 16%; one bed, `planttrace` seed 1), so a
//!    refill alone would be the change that moves nothing.
//!    `RichEarth::Refill` is that gentler arm, kept as a control.
//! 4. **Roots use it up.** Each root draw from a rich cell spends one unit
//!    of its stock instead of its nutrient, so a patch nobody keeps
//!    gardening goes back to plain soil.
//! 5. **It is seen.** Rich ground draws darker, in two steps
//!    (`render_step`), so a repaint is owed only when a cell crosses a step,
//!    and the dirty-rect skip keeps working.
//!
//! **What bounds the loop.** Litter -> fungus -> rich earth -> a lusher door
//! garden -> more litter is a positive loop (`dead-ends.md:636`, the growth
//! pump made from rotted soil). Richness is minted only from fungus eaten,
//! is spent by the roots that use it, and multiplies income by at most
//! `1 + RICH_BONUS`, so it bounds the pump's rate without closing it. The
//! paired test watches for runaway growth.
//!
//! Off with `PIXEL_PHYSICS_RICH_EARTH=off` (`World::rich_earth`);
//! `=refill` for the hunger-only arm.

use super::cell::Cell;
use super::world::World;

/// Which version of rich earth a world runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RichEarth {
    /// No richness is made, and none is read.
    Off,
    /// Rich ground reads as full soil, and nothing more: fixes hunger only.
    Refill,
    /// As `Refill`, plus an income bonus above full for the share of a
    /// plant's nutrient faces in rich ground.
    Bonus,
}

/// Root draws of richness a cell beside eaten fungus is topped up to. One
/// root face draws once an organism tick (45 frames), so a full cell feeds
/// one face for about 10,800 frames -- a lab day and a half.
pub const RICH_STOCK: u16 = 240;

/// The largest income bonus, for a plant with every nutrient face in rich
/// ground: income x (1 + `RICH_BONUS` x share) -- up to double.
///
/// **1.0, not the 0.3 the design proposed, and that was measured.** Paired
/// over played_bed seeds 1-12 with the colony (`latecensus mister=1`,
/// 2026-10-31; `Reports/plants-and-ants-2026-10-30.md` §8), shoot cells of
/// plants within 48 columns of a nest:
///
/// - **0.3 moved nothing**: 6-6 at 150k, 8-4 at 300k (p 0.39). The fed
///   plants do reach it -- 23-64% of their nutrient faces are rich -- so the
///   lever is connected; a ~13% income bump on plants the colony grazes
///   hardest is simply lost to the grazing.
/// - **1.0**: more door plants on 10 of 12 beds at 150k (p 0.039, Holm over
///   the two readings 0.077), level again by 300k (7-5). Guards: colony at
///   250k level (6-6), at 450k larger on 10 of 12 (median 457 against 306,
///   p 0.039, a guard reading, not a planned test); beds with 4+ kinds 10
///   against 10. Two bonus levels were tried, so read those p values as a
///   pattern, not a passed test.
pub const RICH_BONUS: f32 = 1.0;

/// `RICH_BONUS`, or `PIXEL_PHYSICS_RICH_BONUS` for a sweep.
pub fn rich_bonus() -> f32 {
    static B: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *B.get_or_init(|| {
        std::env::var("PIXEL_PHYSICS_RICH_BONUS").ok().and_then(|v| v.parse().ok()).unwrap_or(RICH_BONUS)
    })
}

/// `PIXEL_PHYSICS_RICH_EARTH=off|refill|bonus` sets the version a world
/// starts with; `bonus` by default.
pub fn rich_earth_default() -> RichEarth {
    static MODE: std::sync::OnceLock<RichEarth> = std::sync::OnceLock::new();
    *MODE.get_or_init(|| match std::env::var("PIXEL_PHYSICS_RICH_EARTH").as_deref() {
        Ok("off") => RichEarth::Off,
        Ok("refill") => RichEarth::Refill,
        _ => RichEarth::Bonus,
    })
}

/// **The draw step a cell of this richness is drawn at**: 0 plain, 1 half
/// spent or more, 2 freshly rich. Steps rather than a ramp so a root's
/// draw repaints the cell only twice over its life.
#[inline]
pub fn render_step(richness: u16) -> u8 {
    if richness == 0 {
        0
    } else if richness <= RICH_STOCK / 2 {
        1
    } else {
        2
    }
}

/// How far toward [`RICH_TARGET`] each draw step pulls the ground's own
/// colour (index 0 is plain ground, drawn as itself). Stops short of 1 so
/// the grain still shows.
pub const RICH_PULL: [f32; 3] = [0.0, 0.55, 0.85];
/// The deep red-brown rich ground is pulled toward. **Not near-black, and
/// that was looked at**: the first target, (26, 17, 12), drew rich nest
/// walls the same colour as the dug void beside them, (21, 16, 13) on the
/// played bed at 150k, so the one place richness is made was the one place
/// it could not be seen. A warmer, redder brown stays darker than loam
/// and the wet family without reading as a hole; the second try at
/// (62, 28, 12) and pulls 0.45 / 0.75 showed as a faint halo round the
/// nest at 4x, so both were pushed further.
pub const RICH_TARGET: [f32; 3] = [72.0, 30.0, 10.0];

/// **An ant ate the fungus at `(x, y)`**: top up every nutrient-carrying
/// neighbour to `RICH_STOCK`. Called after the fungus cell is cleared.
/// Returns how many cells it enriched (topped up from below the stock).
pub fn fungus_eaten(world: &mut World, x: i32, y: i32) -> u32 {
    if world.rich_earth == RichEarth::Off {
        return 0;
    }
    let mut made = 0;
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let (nx, ny) = (x + dx, y + dy);
            let cell = world.get(nx, ny);
            if !super::plant::cell_carries_nutrient(world, cell) || cell.is_burning() {
                continue;
            }
            if cell.richness() < RICH_STOCK {
                world.set_richness(nx, ny, RICH_STOCK);
                made += 1;
            }
        }
    }
    world.rich_cells_made += u64::from(made);
    made
}

/// **A root drew from `cell` at `(x, y)`.** Spends one unit of its
/// richness and returns true when the cell was rich (the draw is paid
/// from the richness, not the cell's nutrient); false leaves the cell to
/// the ordinary nutrient draw.
pub fn draw(world: &mut World, x: i32, y: i32, cell: Cell, amount: u8) -> bool {
    if world.rich_earth == RichEarth::Off {
        return false;
    }
    let rich = cell.richness();
    if rich == 0 {
        return false;
    }
    world.set_richness(x, y, rich.saturating_sub(u16::from(amount.max(1))));
    world.rich_draws += 1;
    true
}

/// Whether this ground reads as rich in this world.
#[inline]
pub fn is_rich(world: &World, cell: Cell) -> bool {
    world.rich_earth != RichEarth::Off && cell.richness() > 0
}

/// What a plant's share of rich faces does to its income: `1.0` unless the
/// world runs `Bonus`.
#[inline]
pub fn income_bonus(world: &World, rich_share: f32) -> f32 {
    if world.rich_earth == RichEarth::Bonus {
        1.0 + rich_bonus() * rich_share.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

/// Rich cells standing in the world now, a census for harnesses: a scan
/// of every chunk, so call it at sample time, never per frame.
pub fn census(world: &World) -> u64 {
    let mut n = 0u64;
    for chunk in world.chunks() {
        let origin = chunk.coord.origin();
        for ly in 0..super::chunk::CHUNK_SIZE {
            for lx in 0..super::chunk::CHUNK_SIZE {
                let cell = chunk.get_world(origin.0 + lx, origin.1 + ly);
                if cell.richness() > 0 && super::plant::cell_carries_nutrient(world, cell) {
                    n += 1;
                }
            }
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::material;

    /// A strip of packed soil (nest wall) with a fungus cell eaten in the
    /// middle of it, at (30, 30).
    fn wall(mode: RichEarth) -> World {
        let mut w = World::new(Rect::new(0, 0, 63, 63));
        let packed = w.materials.id_of("packedsoil").expect("packedsoil");
        for y in 28..=32 {
            for x in 26..=34 {
                w.set(x, y, Cell::new(packed, 0));
            }
        }
        w.set(30, 30, Cell::EMPTY);
        w.set(30, 31, Cell::new(material::STONE, 0));
        w.rich_earth = mode;
        w
    }

    #[test]
    fn eaten_fungus_enriches_the_ground_round_it_and_only_that() {
        let mut w = wall(RichEarth::Bonus);
        let made = fungus_eaten(&mut w, 30, 30);
        // Eight neighbours, one of them stone: seven enriched.
        assert_eq!(made, 7);
        assert_eq!(w.rich_cells_made, 7);
        assert_eq!(w.get(29, 29).richness(), RICH_STOCK);
        assert_eq!(w.get(30, 31).richness(), 0, "stone carries no nutrient and is never rich");
        assert_eq!(w.get(28, 30).richness(), 0, "two cells away is out of reach");
        assert_eq!(census(&w), 7);
        // A second meal beside it tops up, it does not stack.
        assert_eq!(fungus_eaten(&mut w, 30, 30), 0);
        assert_eq!(w.get(29, 29).richness(), RICH_STOCK);
    }

    #[test]
    fn rich_ground_reads_full_and_roots_spend_it_down_to_plain_soil() {
        let mut w = wall(RichEarth::Bonus);
        fungus_eaten(&mut w, 30, 30);
        assert_eq!(w.soil_nutrient_fraction(29, 29), 1.0);
        for _ in 0..RICH_STOCK {
            assert_eq!(w.draw_soil_nutrient(29, 29, 1), 1);
        }
        assert_eq!(w.rich_draws, u64::from(RICH_STOCK));
        assert_eq!(w.get(29, 29).richness(), 0, "spent");
        assert!(!is_rich(&w, w.get(29, 29)));
        // The next draw comes out of the cell's own nutrient, as before.
        let before = w.rich_draws;
        w.draw_soil_nutrient(29, 29, 1);
        assert_eq!(w.rich_draws, before);
    }

    #[test]
    fn the_switch_turns_rich_earth_off_and_refill_gives_no_bonus() {
        let mut off = wall(RichEarth::Off);
        assert_eq!(fungus_eaten(&mut off, 30, 30), 0);
        assert_eq!(census(&off), 0);
        assert_eq!(income_bonus(&off, 1.0), 1.0);
        let refill = wall(RichEarth::Refill);
        assert_eq!(income_bonus(&refill, 1.0), 1.0);
        let bonus = wall(RichEarth::Bonus);
        assert_eq!(income_bonus(&bonus, 1.0), 1.0 + rich_bonus());
        assert_eq!(income_bonus(&bonus, 0.5), 1.0 + rich_bonus() * 0.5);
    }

    #[test]
    fn richness_rides_with_the_cell_and_never_reads_on_a_burning_one() {
        let mut c = Cell::new(material::STONE, 0);
        c.set_richness(100);
        assert_eq!(c.richness(), 100);
        c.ignite(50);
        assert_eq!(c.richness(), 0);
        c.set_richness(7);
        assert_eq!(c.burn_remaining(), 50, "a burning cell's timer is not richness to write");
        c.extinguish();
        assert_eq!(c.richness(), 0);
    }

    #[test]
    fn render_steps_cover_the_stock() {
        assert_eq!(render_step(0), 0);
        assert_eq!(render_step(1), 1);
        assert_eq!(render_step(RICH_STOCK / 2), 1);
        assert_eq!(render_step(RICH_STOCK / 2 + 1), 2);
        assert_eq!(render_step(RICH_STOCK), 2);
    }
}
