//! **The fungus garden: a colony farms the plant matter it carries home.**
//!
//! Built 2026-10-30 for the plants-and-ants lane. Scott's repo review found
//! that plants and ants barely touch: an ant grazes a plant and that is the
//! whole relationship -- no farming of any kind, and a cut leaf worth 40 J,
//! so nothing gave a colony a reason to keep plants near home. This is the
//! farm, on fun-over-fidelity terms (real leaf-cutter ants are the
//! inspiration, not the bar):
//!
//! 1. **It starts.** A loose cell of plant matter (`Material::garden_fodder`:
//!    cut leaf, dead leaf, litter, grass, flowers, crumbs) lying **under a
//!    roof** next to worked nest ground (`nest`, `packedsoil`, or a cell an
//!    ant dug) has a small chance each visit to turn into `fungus`.
//! 2. **It spreads through the pile.** Fodder touching fungus turns much
//!    faster, so a stocked room goes over as a front rather than speckle by
//!    speckle -- but only inside the nest, like the start. No fodder, no
//!    spread: the garden grows only on what the ants bring it.
//! 3. **It ripens while it stays under its roof.** Each visit adds `RIPEN_STEP` to the
//!    cell's worth, up to `RIPE_WORTH` -- a 40 J leaf becomes a 480 J
//!    mouthful in about two lab days -- and the shade brightens with it, so
//!    the player sees a garden coming on without an overlay.
//! 4. **Open sky kills it, gradually.** Fungus with no roof over it wilts
//!    back to litter at `WILT_CHANCE` a visit, so a garden whose roof falls
//!    in, or one the player digs open, dies back rather than vanishing.
//!
//! **A roof, not a light reading, and that was measured.** The first build
//! asked for field light under a threshold, as real fungus wants the dark.
//! On the played bed (trained ant, seeds 1-2, 2026-10-30) no cell the colony
//! uses ever read dark: the field is coarse, so air 12-16 cells down a
//! shaft reads the block's light (about half of full, 2.1 of 4.0) and every
//! fodder cell in the nest band read 1.1-2.4. A threshold the nest can never
//! pass is a garden that never fires. And "is there ground over it" is the
//! rule a player can read off the screen: a crater floor is open, a room
//! is roofed, so the garden grows in the rooms the colony (or the player's
//! DIG tool) makes, which is the nest the game wants anyway.
//!
//! **Why a scan of its own and not the decay channel.** Decay sites are
//! scheduled when a chunk settles (`decay.rs`), and a nest with ants walking
//! through it is exactly the chunk that does not settle; a cut leaf has no
//! `decays_into` and never gets a site at all. So the garden visits a
//! stripe of rows each frame -- every row once per `GARDEN_PERIOD` frames --
//! which bounds the work at `width * height / GARDEN_PERIOD` cell reads a
//! frame (2,560 on the 512x320 lab) and needs no bookkeeping that could
//! strand a site.
//!
//! **Its own dice, keyed on where and when** (`rng::stream`), never
//! `World::rng`: a garden visit must not re-roll the rest of the world, for
//! `decay.rs`'s measured reason.
//!
//! Off with `PIXEL_PHYSICS_GARDEN=off` (`World::garden_on`), for the paired
//! arm; on by default.

use super::cell::Cell;
use super::rng;
use super::world::World;

/// Frames between visits to any one row.
pub const GARDEN_PERIOD: u64 = 64;
/// How far up a column `roofed` looks for ground. A room under a shallow
/// mound is roofed; a crater floor with the open box over it is not.
pub const ROOF_REACH: i32 = 24;
/// Per-visit chance that fodder beside worked nest ground starts a garden.
/// Small, so a single leaf dropped in a passage usually waits a while; a pile
/// gets there sooner because it has more cells rolling.
pub const START_CHANCE: f32 = 0.02;
/// Per-visit chance that fodder touching fungus turns.
pub const SPREAD_CHANCE: f32 = 0.25;
/// The least a fresh fungus cell is worth, whatever its fodder was: a cut
/// leaf is 40 J and a garden that started there should already be a meal.
pub const START_WORTH: u16 = 120;
/// What a cell ripens to.
pub const RIPE_WORTH: u16 = 480;
/// Joules a dark fungus cell gains per visit. 64-frame visits, so 120 -> 480
/// takes 90 visits, about 5,800 frames: a lab day and a half.
pub const RIPEN_STEP: u16 = 4;
/// Per-visit chance that fungus with no roof wilts to litter.
pub const WILT_CHANCE: f32 = 0.05;

/// Separates the garden's dice from every other `rng::stream` caller.
const GARDEN_SALT: u64 = 0x4761_7264_656E_5F31; // "Garden_1"

/// `PIXEL_PHYSICS_GARDEN=off` starts a world with the garden off.
pub fn garden_default() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| !matches!(std::env::var("PIXEL_PHYSICS_GARDEN").as_deref(), Ok("off")))
}

/// The palette index for a fungus cell of this worth: four steps from just
/// started to ripe.
pub fn shade_for(worth: u16) -> u8 {
    let span = (RIPE_WORTH - START_WORTH).max(1) as u32;
    let into = worth.saturating_sub(START_WORTH) as u32;
    ((into * 4 / (span + 1)).min(3)) as u8
}

/// **Whether there is ground over `(x, y)`**: walking up the column, the
/// first cell that is neither air, liquid, gas, a living thing, nor the
/// garden's own fodder or fungus is solid or powder ground within
/// `ROOF_REACH`. A pile does not roof itself, and a canopy is not a roof.
pub fn roofed(world: &World, x: i32, y: i32) -> bool {
    use super::material::MaterialKind;
    let fungus = world.materials.id_of("fungus");
    for dy in 1..=ROOF_REACH {
        let c = world.get(x, y - dy);
        if c.organism_id() != 0 || Some(c.material) == fungus || world.materials.get(c.material).garden_fodder {
            continue;
        }
        match world.materials.kind(c.material) {
            MaterialKind::Solid | MaterialKind::Powder => return true,
            _ => {}
        }
    }
    false
}

/// One frame's stripe of the garden.
pub fn step(world: &mut World) {
    if !world.garden_on {
        return;
    }
    let Some(fungus) = world.materials.id_of("fungus") else { return };
    let Some(b) = world.bounds() else { return };
    let nest = world.materials.id_of("nest");
    let packed = world.materials.id_of("packedsoil");
    let litter = world.materials.id_of("litter");
    let phase = (world.frame % GARDEN_PERIOD) as i32;
    let first = b.min_y + (phase - b.min_y).rem_euclid(GARDEN_PERIOD as i32);
    let mut y = first;
    while y <= b.max_y {
        for x in b.min_x..=b.max_x {
            let cell = world.get(x, y);
            if cell.organism_id() != 0 {
                continue;
            }
            let is_fungus = cell.material == fungus;
            if !is_fungus && !world.materials.get(cell.material).garden_fodder {
                continue;
            }
            let lit = !roofed(world, x, y);
            let mut dice = rng::stream(world.seed ^ GARDEN_SALT, x as u32 as u64, y as u32 as u64, world.frame);
            if is_fungus {
                if lit {
                    if dice.chance(WILT_CHANCE) {
                        world.garden_wilted += 1;
                        match litter {
                            Some(l) => world.set(x, y, Cell::new(l, 0)),
                            None => world.set(x, y, Cell::EMPTY),
                        }
                    }
                } else {
                    let worth = cell.aux().max(START_WORTH);
                    if worth < RIPE_WORTH {
                        let next = (worth + RIPEN_STEP).min(RIPE_WORTH);
                        world.garden_ripened_j += (next - worth) as u64;
                        let mut c = cell.with_aux(next);
                        c.shade = shade_for(next);
                        world.set(x, y, c);
                    }
                }
                continue;
            }
            if lit {
                continue;
            }
            let mut by_fungus = false;
            let mut by_nest = world.dug_cells.contains(&(x, y));
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let m = world.get(x + dx, y + dy).material;
                    by_fungus |= m == fungus;
                    by_nest |= Some(m) == nest || Some(m) == packed || world.dug_cells.contains(&(x + dx, y + dy));
                }
            }
            // **The nest, for spreading too.** The first build let fungus
            // spread through any roofed fodder it touched, and on the played
            // bed it crept out of the nest along the litter buried just under
            // the soil surface: 2,237 spreads by 100k on seed 2, a speckle of
            // ripe fungus along the whole bed and 300 cells nowhere near the
            // colony. A garden is the colony's, so every cell of it has to be
            // in or beside the dug nest.
            if !by_nest {
                continue;
            }
            let chance = if by_fungus { SPREAD_CHANCE } else { START_CHANCE };
            if !dice.chance(chance) {
                continue;
            }
            let worth = (crate::sim::creature::food_value(world, cell).round().clamp(0.0, u16::MAX as f32) as u16).clamp(START_WORTH, RIPE_WORTH);
            if by_fungus {
                world.garden_spread += 1;
            } else {
                world.garden_started += 1;
            }
            let mut c = Cell::new(fungus, shade_for(worth)).with_aux(worth);
            c.shade = shade_for(worth);
            world.set(x, y, c);
        }
        y += GARDEN_PERIOD as i32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::material;

    /// A floor of stone, a strip of `under` on it, a pile of cut leaf on the
    /// strip, and a stone roof two cells over the pile.
    /// On `packedsoil` the pile lies in a dug room (its cells are in
    /// `dug_cells`), as a pile the ants carried in does; on anything else it
    /// is a pile on undug ground.
    fn pile(under: &str) -> (World, crate::sim::material::MaterialId, crate::sim::material::MaterialId) {
        let mut w = World::new(Rect::new(0, 0, 63, 63));
        let dug = under == "packedsoil";
        let under = w.materials.id_of(under).expect("compiled in");
        let leaf = w.materials.id_of("leaf").expect("leaf");
        let fungus = w.materials.id_of("fungus").expect("fungus");
        for x in 0..64 {
            w.set(x, 63, Cell::new(material::STONE, 0));
        }
        for x in 20..40 {
            w.set(x, 62, Cell::new(under, 0));
            for y in 59..62 {
                w.set(x, y, Cell::new(leaf, 0));
                if dug {
                    w.dug_cells.insert((x, y));
                }
            }
        }
        for x in 10..50 {
            w.set(x, 56, Cell::new(material::STONE, 0));
        }
        w.garden_on = true;
        (w, leaf, fungus)
    }

    fn run(w: &mut World, frames: u64) {
        for _ in 0..frames {
            step(w);
            w.frame += 1;
        }
    }

    fn census(w: &World, id: crate::sim::material::MaterialId) -> (usize, u64) {
        let mut n = 0;
        let mut worth = 0u64;
        for y in 0..64 {
            for x in 0..64 {
                let c = w.get(x, y);
                if c.material == id {
                    n += 1;
                    worth += c.aux() as u64;
                }
            }
        }
        (n, worth)
    }

    #[test]
    fn a_leaf_pile_on_nest_ground_turns_to_fungus_and_ripens() {
        let (mut w, leaf, fungus) = pile("packedsoil");
        assert!(roofed(&w, 30, 61) && roofed(&w, 30, 59));
        run(&mut w, GARDEN_PERIOD * 400);
        let (left, _) = census(&w, leaf);
        let (grown, worth) = census(&w, fungus);
        assert_eq!(left, 0, "the whole pile should have gone over");
        assert_eq!(grown, 60);
        assert_eq!(worth, 60 * RIPE_WORTH as u64, "every cell ripe after 400 visits");
        assert!(w.garden_started > 0 && w.garden_spread > 0, "started {} spread {}", w.garden_started, w.garden_spread);
        // The front, not the speckle: most of the pile turned because fungus touched it.
        assert!(w.garden_spread > w.garden_started * 2, "started {} spread {}", w.garden_started, w.garden_spread);
        assert_eq!(w.garden_ripened_j, worth - (w.garden_started + w.garden_spread) * START_WORTH as u64);
    }

    #[test]
    fn a_leaf_pile_away_from_the_nest_never_starts() {
        let (mut w, leaf, fungus) = pile("soil");
        run(&mut w, GARDEN_PERIOD * 400);
        assert_eq!(census(&w, fungus).0, 0);
        assert_eq!(census(&w, leaf).0, 60);
    }

    #[test]
    fn the_switch_turns_the_garden_off() {
        let (mut w, leaf, fungus) = pile("packedsoil");
        w.garden_on = false;
        run(&mut w, GARDEN_PERIOD * 400);
        assert_eq!(census(&w, fungus).0, 0);
        assert_eq!(census(&w, leaf).0, 60);
    }

    #[test]
    fn the_shade_climbs_with_the_worth() {
        assert_eq!(shade_for(START_WORTH), 0);
        assert_eq!(shade_for(RIPE_WORTH), 3);
        let mut last = 0;
        for worth in START_WORTH..=RIPE_WORTH {
            let s = shade_for(worth);
            assert!(s >= last && s <= 3);
            last = s;
        }
    }

    #[test]
    fn a_garden_with_its_roof_taken_off_wilts_to_litter() {
        let (mut w, leaf, fungus) = pile("packedsoil");
        run(&mut w, GARDEN_PERIOD * 400);
        assert_eq!(census(&w, fungus).0, 60);
        for x in 10..50 {
            w.set(x, 56, Cell::EMPTY);
        }
        assert!(!roofed(&w, 30, 59));
        run(&mut w, GARDEN_PERIOD * 200);
        let litter = w.materials.id_of("litter").unwrap();
        assert_eq!(census(&w, fungus).0, 0, "an open garden should have wilted");
        assert_eq!(census(&w, litter).0, 60);
        assert_eq!(census(&w, leaf).0, 0);
        assert_eq!(w.garden_wilted, 60);
    }

    #[test]
    fn a_leaf_pile_with_no_roof_never_starts() {
        let (mut w, leaf, fungus) = pile("packedsoil");
        for x in 10..50 {
            w.set(x, 56, Cell::EMPTY);
        }
        run(&mut w, GARDEN_PERIOD * 400);
        assert_eq!(census(&w, fungus).0, 0);
        assert_eq!(census(&w, leaf).0, 60);
    }

    #[test]
    fn the_garden_does_not_spread_out_of_the_nest() {
        // A long roofed strip of leaf on plain soil, one end on nest ground:
        // the garden takes the end beside the nest and stops there.
        let (mut w, leaf, fungus) = pile("soil");
        let packed = w.materials.id_of("packedsoil").unwrap();
        w.set(20, 62, Cell::new(packed, 0));
        run(&mut w, GARDEN_PERIOD * 400);
        let (grown, _) = census(&w, fungus);
        assert!(grown > 0, "the end on nest ground should have started");
        assert!(grown <= 6, "only cells beside the nest ground may turn, got {grown}");
        assert!(census(&w, leaf).0 >= 54);
    }
}
