//! The hand-authored sandbox terrain: a floor and a couple of ledges.
//!
//! Moved here verbatim from `app::build_terrain_only` when worldgen became a
//! module. Kept rather than replaced by an equivalent preset, because several
//! things are written against *these* coordinates and would silently start
//! testing something else: `examples/filmstrip.rs`'s `mine` scene erases a
//! fixed rectangle out of the left ledge, and the app's own tests probe known
//! cells. A generated preset tuned to look similar would not be the same
//! world, and the difference would show up as a confusing test failure rather
//! than as an obviously different picture.
//!
//! It is also the control: the question "is generated terrain actually
//! better?" needs the thing it replaced to still be one keypress away.

use crate::sim::material;
use crate::sim::world::World;
use crate::sim::Cell;

/// Build the hand-authored terrain into `world`.
///
/// Dimensions come from the world's own bounds rather than `app::WIDTH` /
/// `app::HEIGHT`, so that `sim` and `worldgen` stay free of any dependency on
/// the sandbox binary's constants. For the fixed world those are the same
/// numbers this code has always used.
pub fn build(world: &mut World) {
    let bounds = world.bounds().expect("legacy terrain needs a bounded world");
    let w = bounds.max_x + 1;
    let h = bounds.max_y + 1;
    // Always present: `reload` only ever adds or updates, so the compiled-in
    // stone cannot be removed by editing the assets directory.
    let stone = world
        .materials
        .id_of("stone")
        .expect("stone is a compiled-in material");

    // The bottom two rows are literal bedrock, the world's structural
    // anchor (`structural.rs`) and the deepest of the six vertical zones
    // `Reports/worldgen-design.md` §2 defines. Nothing placed bedrock
    // anywhere before this, which meant the only anchor in the entire world
    // was the floor's bottom row happening to touch the out-of-bounds
    // sentinel -- true, but by accident rather than by construction, and
    // not something generated terrain could rely on.
    for x in 0..w {
        for y in (h - 2)..h {
            world.set(x, y, Cell::new(material::BEDROCK, 0).with_attached(true));
        }
    }
    for x in 0..w {
        for y in (h - 8)..(h - 2) {
            world.set(x, y, Cell::new(stone, (x % 4) as u8).with_attached(true));
        }
    }

    // 6 cells deep, which is deliberately more than stone's confinement
    // diameter (5): each ledge contains genuinely confined rock and so holds
    // itself up, with no support pillar and no exemption from checking. Thin
    // these below 5 and they will come down, which is the mechanic working
    // rather than a regression.
    let mut ledge = |x0: i32, x1: i32, y: i32| {
        for x in x0..x1 {
            for dy in 0..6 {
                world.set(x, y + dy, Cell::new(stone, (x % 4) as u8).with_attached(true));
            }
        }
    };
    ledge(0, 110, 200); // cut into the left wall
    ledge(402, w, 150); // cut into the right wall
    ledge(180, 320, 260);
    for y in 266..(h - 8) {
        for x in 244..256 {
            world.set(x, y, Cell::new(stone, (x % 4) as u8).with_attached(true)); // the middle platform's pillar
        }
    }

    // The world is left dirty on purpose. The first sweep examines the terrain,
    // finds that none of it moves, and settles from the second frame onward.
}
