//! **The queen seat's demo** (`demo=queen`): the player is the founding
//! queen. The design is `Reports/teaching-the-colony-2026-10-25/queen.md`
//! and the teaching plan's sections 3-6; the demo's own plan lands beside
//! them. This file grows with that plan; at the launcher stage it only says
//! which ant is hers.

use crate::sim::cell::OrganismId;
use crate::sim::world::World;

use super::super::Lab;

/// The queen seat's own keys, as the help line prints them. Empty until
/// the seat's verbs land.
pub const SEAT_KEYS: &str = "";

/// The queen seat's running state.
#[derive(Clone, Debug, Default)]
pub struct QueenDemo {}

/// **Which ant is the queen**: the oldest ant in the box. The demo's
/// scenario lands one founder alone, so this is her.
pub fn pick_body(world: &World) -> Option<OrganismId> {
    super::oldest_ant(world)
}

/// One of the seat keys (`Y U P J K R`) was pressed.
pub fn seat_key(_lab: &mut Lab, _key: char) {}

/// Once a displayed frame, after the body has its keys.
pub fn frame(_lab: &mut Lab) {}

/// Before each simulated tick.
pub fn before_tick(_lab: &mut Lab) {}

/// After each simulated tick.
pub fn after_tick(_lab: &mut Lab) {}

/// The pointer over the world; `true` if the queen seat took it.
pub fn mouse(_lab: &mut Lab, _cell: (i32, i32), _act: super::MouseAct) -> bool {
    false
}

/// Marks over the world.
pub fn draw_world(_lab: &Lab, _frame_buf: &mut [u8]) {}

/// Whether [`draw_world`] drew anything this frame.
pub fn marks_world(_lab: &Lab) -> bool {
    false
}

/// The queen's panel.
pub fn draw_hud(_lab: &Lab, _hc: crate::render::Hud, _frame_buf: &mut [u8]) {}
