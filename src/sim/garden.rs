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
//!    back to crumbs of the same worth at `WILT_CHANCE` a visit, so a garden
//!    whose roof falls in, or one the player digs open, dies back rather
//!    than vanishing -- and dying back makes no food.
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
//! inside the box round the cells the colony has dug (`garden_reach`), which
//! bounds the work at that box's area / `GARDEN_PERIOD` cell reads a frame
//! and needs no bookkeeping that could strand a site. It runs in every game
//! that steps `frame::step` (lab, outdoor, held: one physics); a world with
//! no colony pays nothing. The ant-training harness (`train::challenges`)
//! steps its own phases and never runs it.
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
    roofed_over(world, world.materials.id_of("fungus"), x, y)
}

/// [`roofed`] with the fungus id already looked up, for the scan, which
/// asks it of every candidate (the review of 2026-10-31 found a string-hash
/// lookup per call).
fn roofed_over(world: &World, fungus: Option<super::material::MaterialId>, x: i32, y: i32) -> bool {
    use super::material::MaterialKind;
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

/// **Where the garden can be: the box round every cell an ant has dug,
/// widened by `REACH_MARGIN`**, or `None` when nothing has been dug.
///
/// The first build scanned the whole world's stripe, and `frame::step` runs
/// in every game: the review of 2026-10-31 timed it at 0.013 ms a frame on
/// the 512x320 lab, 0.42 ms on the held world and **5.5 ms on the 8192x2560
/// outdoor world**, hidden in the active-sites lap. Every garden cell has to
/// be in or beside the dug nest anyway (the nest gate below), so the dug
/// cells bound it: a world with no colony pays one empty-set check, and a
/// colony pays for its own nest. Refreshed once per `GARDEN_PERIOD`, at the
/// top of the cycle, so one pass over `dug_cells` is shared by 64 frames.
fn garden_reach(world: &World) -> Option<super::chunk::Rect> {
    let mut it = world.dug_cells.iter();
    let &(x0, y0) = it.next()?;
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (x0, y0, x0, y0);
    for &(x, y) in it {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    let b = world.bounds()?;
    let r = super::chunk::Rect::new(
        (min_x - REACH_MARGIN).max(b.min_x),
        (min_y - REACH_MARGIN).max(b.min_y),
        (max_x + REACH_MARGIN).min(b.max_x),
        (max_y + REACH_MARGIN).min(b.max_y),
    );
    (r.min_x <= r.max_x && r.min_y <= r.max_y).then_some(r)
}

/// How far past the outermost dug cell the garden looks: one for "beside
/// the nest", and room for fungus a passing ant has dragged a few cells.
pub const REACH_MARGIN: i32 = 4;

/// One frame's stripe of the garden.
pub fn step(world: &mut World) {
    if !world.garden_on {
        return;
    }
    let Some(fungus) = world.materials.id_of("fungus") else { return };
    let phase = (world.frame % GARDEN_PERIOD) as i32;
    if phase == 0 || world.garden_reach.is_none() && !world.dug_cells.is_empty() {
        world.garden_reach = garden_reach(world);
    }
    let Some(b) = world.garden_reach else { return };
    let nest = world.materials.id_of("nest");
    let packed = world.materials.id_of("packedsoil");
    let crumbs = world.materials.id_of("crumbs");
    let litter = world.materials.id_of("litter");
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
            let mut dice = rng::stream(world.seed ^ GARDEN_SALT, x as u32 as u64, y as u32 as u64, world.frame);
            if is_fungus {
                // A bare fungus cell (aux 0) is priced at the material's own
                // 120 J (`food_value`'s fallback), so it ripens from there.
                let worth = if cell.aux() == 0 { START_WORTH } else { cell.aux() };
                if !roofed_over(world, Some(fungus), x, y) {
                    if dice.chance(WILT_CHANCE) {
                        world.garden_wilted += 1;
                        // **Wilts to crumbs of the same worth, not to
                        // litter.** Litter is priced by its material at
                        // 480 J whatever went in, so the first build turned
                        // a fresh 120 J cell into 480 J the moment it was
                        // uncovered, and a re-roofed heap came back as ripe
                        // fungus at once (review, 2026-10-31). Crumbs carry
                        // their worth in `aux`, so dying back makes nothing.
                        let shade = dice.below(4) as u8;
                        match (crumbs, litter) {
                            (Some(c), _) => world.set(x, y, Cell::new(c, shade).with_aux(worth)),
                            (None, Some(l)) => world.set(x, y, Cell::new(l, shade)),
                            _ => world.set(x, y, Cell::EMPTY),
                        }
                    }
                } else if worth < RIPE_WORTH {
                    // **Ripens from what it holds**: the first build lifted
                    // every cell to `START_WORTH` first, so a 1 J scrap an
                    // ant set down came back as 120 J and only the +4 was
                    // booked in `garden_ripened_j` (review, 2026-10-31).
                    let next = (worth + RIPEN_STEP).min(RIPE_WORTH);
                    world.garden_ripened_j += (next - worth) as u64;
                    let mut c = cell.with_aux(next);
                    c.shade = shade_for(next);
                    world.set(x, y, c);
                }
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
            // in or beside the dug nest. Checked before the roof, which is
            // the dearer test (up to `ROOF_REACH` reads).
            if !by_nest || !roofed_over(world, Some(fungus), x, y) {
                continue;
            }
            let chance = if by_fungus { SPREAD_CHANCE } else { START_CHANCE };
            if !dice.chance(chance) {
                continue;
            }
            // **Keeps what the fodder was worth**, never less than
            // `START_WORTH`: the first build also capped it at `RIPE_WORTH`,
            // so a 1,440 J flower went into the garden and came out 480 J
            // (review, 2026-10-31). The lift to 120 J is the garden's yield
            // and is booked in `garden_ripened_j` with the ripening.
            let value = crate::sim::creature::food_value(world, cell).round().clamp(0.0, u16::MAX as f32) as u16;
            let worth = value.max(START_WORTH);
            world.garden_ripened_j += (worth - value) as u64;
            if by_fungus {
                world.garden_spread += 1;
            } else {
                world.garden_started += 1;
            }
            world.set(x, y, Cell::new(fungus, shade_for(worth)).with_aux(worth));
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
        // Everything the garden added, booked: 60 leaves at 40 J in, 60 ripe cells out.
        assert_eq!(w.garden_ripened_j, worth - 60 * 40);
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
    fn a_garden_with_its_roof_taken_off_wilts_to_crumbs_of_the_same_worth() {
        let (mut w, leaf, fungus) = pile("packedsoil");
        run(&mut w, GARDEN_PERIOD * 400);
        assert_eq!(census(&w, fungus).0, 60);
        for x in 10..50 {
            w.set(x, 56, Cell::EMPTY);
        }
        assert!(!roofed(&w, 30, 59));
        run(&mut w, GARDEN_PERIOD * 200);
        let crumbs = w.materials.id_of("crumbs").unwrap();
        assert_eq!(census(&w, fungus).0, 0, "an open garden should have wilted");
        // Same cells, same joules: dying back makes no food.
        assert_eq!(census(&w, crumbs), (60, 60 * RIPE_WORTH as u64));
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
        // An ant packed it, so an ant dug there: the cell is in `dug_cells`,
        // which is what puts the strip inside `garden_reach` at all.
        w.dug_cells.insert((20, 62));
        run(&mut w, GARDEN_PERIOD * 400);
        let (grown, _) = census(&w, fungus);
        assert!(grown > 0, "the end on nest ground should have started");
        assert!(grown <= 6, "only cells beside the nest ground may turn, got {grown}");
        assert!(census(&w, leaf).0 >= 54);
    }

    /// **A living plant's leaves are never the garden's.** The review of
    /// 2026-10-31 deleted the organism guard and every other test here
    /// stayed green; this one is the guard's own.
    #[test]
    fn a_living_plants_leaves_never_turn() {
        let (mut w, leaf, fungus) = pile("packedsoil");
        let owner: crate::sim::cell::OrganismId = 7;
        for x in 20..40 {
            for y in 59..62 {
                w.set(x, y, Cell::new(leaf, 0).with_organism_id(owner));
            }
        }
        run(&mut w, GARDEN_PERIOD * 400);
        assert_eq!(census(&w, fungus).0, 0, "fungus grew on a living plant");
        assert_eq!(w.garden_started + w.garden_spread, 0);
    }

    /// **A scrap ripens from what it holds.** The first build lifted any
    /// fungus cell below 120 J straight to 120 J on its next visit.
    #[test]
    fn a_scrap_of_fungus_ripens_from_its_own_worth() {
        let (mut w, _, fungus) = pile("packedsoil");
        w.set(30, 61, Cell::new(fungus, 0).with_aux(1));
        let start = w.frame;
        while w.frame < start + GARDEN_PERIOD {
            step(&mut w);
            w.frame += 1;
        }
        assert_eq!(w.get(30, 61).aux(), 1 + RIPEN_STEP, "one visit should add one step");
    }

    /// **A flower keeps its worth.** It is 1,440 J; the first build capped
    /// every new fungus cell at 480 J.
    #[test]
    fn a_flower_goes_into_the_garden_at_its_own_worth() {
        let (mut w, leaf, fungus) = pile("packedsoil");
        let flower = w.materials.id_of("flower").unwrap();
        for x in 20..40 {
            for y in 59..62 {
                if w.get(x, y).material == leaf {
                    w.set(x, y, Cell::new(flower, 0));
                }
            }
        }
        run(&mut w, GARDEN_PERIOD * 400);
        let (n, worth) = census(&w, fungus);
        assert_eq!(n, 60);
        assert_eq!(worth, 60 * 1440, "flowers lost worth going into the garden");
    }

    /// **No colony, no scan.** The garden looks only inside the box round
    /// the dug cells, so the outdoor world pays nothing where no ant dug.
    #[test]
    fn the_garden_looks_only_round_the_dug_cells() {
        let (mut w, _, _) = pile("soil");
        step(&mut w);
        assert_eq!(w.garden_reach, None);
        w.dug_cells.insert((30, 40));
        w.dug_cells.insert((12, 50));
        assert_eq!(garden_reach(&w), Some(crate::sim::chunk::Rect::new(12 - REACH_MARGIN, 40 - REACH_MARGIN, 30 + REACH_MARGIN, 50 + REACH_MARGIN)));
    }
}
