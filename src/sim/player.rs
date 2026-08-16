//! M9 — the playable character: a gnome who runs, jumps, and (phase 2
//! onward) digs, plants and throws.
//!
//! **Off-grid, like `rigid::ChunkBody`, and a ghost.** The character writes
//! no cells and is invisible to the CA sweep, which is the whole
//! performance story: an idle gnome on a settled world wakes zero chunks
//! and costs the dirty-rect render skip nothing. The trade is that sand
//! and bodies fall *through* the player's rectangle; the depenetration
//! pass below is the corrective, and `buried` is what it reports when
//! there is no way out. Deliberately not a `ChunkBody`: bodies settle
//! back into the grid after three stalled frames, rotate, and shove
//! powder aside, and a character must do none of those — the overlap
//! left after removing all three is a plain AABB sweep, written here
//! rather than factored out of `rigid.rs` where generalising the collider
//! could perturb settled collapse behaviour.
//!
//! Stepped in `App::update`'s serial body slot (right after
//! `rigid::step_chunk_bodies`), for the same write-disjointness reason
//! that phase exists at all. Everything here is input-driven — no RNG, no
//! wall clock — so a run replays from a `PlayerInput` sequence alone,
//! which is what the determinism requirement means for an entity that
//! only exists when a player summons it.

use super::material::MaterialKind;
use super::world::World;

/// Character extent in cells. 3x6 on a 512x320 world reads gnome-scale
/// (a worm is 1 cell, trees are tens), and fits through the 9-cell bore a
/// radius-4 `rigid::mine` carves — the tunnel size phase 2 digs.
pub const PLAYER_WIDTH: i32 = 3;
pub const PLAYER_HEIGHT: i32 = 6;

/// How far the depenetration pass will push to free an invaded rectangle
/// before giving up and declaring the player buried. Small on purpose: a
/// large push is a teleport, and popping through a thin ceiling reads far
/// worse than being stuck under it.
const DEPENETRATE_REACH: i32 = 4;

/// Everything about how the character *feels*, live-tunable under the
/// panel's PLAYER group and persisted to `assets/player.ron`. The same
/// shape as `explosion::Tuning` and for the same reason: these numbers
/// are judged by playing, not by argument, so they must be sweepable with
/// the world visible.
///
/// `#[serde(default)]` so a file written by an older build still loads.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Tuning {
    /// Per-tick downward acceleration. Defaults to `rigid`/`particle`'s
    /// own 0.15 — the gnome falling at a different rate than the debris
    /// beside him would read as a bug even if neither number is wrong.
    pub gravity: f32,
    /// Horizontal acceleration per tick while a run key is held.
    pub run_accel: f32,
    /// Horizontal speed cap, cells per tick. 1.3 crosses the 512-cell
    /// world in ~6.5 seconds.
    pub run_max: f32,
    /// Subtractive slowdown per tick when grounded with no run key held —
    /// stops from full speed in ~5 ticks. Airborne motion keeps its speed,
    /// which is what makes a jump arc feel committed.
    pub ground_decel: f32,
    /// Multiplier on `run_accel` while airborne.
    pub air_control: f32,
    /// Upward velocity a jump starts with. Against gravity 0.15 this is
    /// v²/2g ≈ 13 cells of rise, apex around 13 ticks.
    pub jump_impulse: f32,
    /// Terminal fall speed. Below `rigid`'s 6.0/axis so landings stay
    /// controllable; the substepped sweep would be correct at any value.
    pub fall_clamp: f32,
    /// Ticks after walking off an edge during which a jump still fires.
    pub coyote_frames: u8,
    /// Ticks a jump press is remembered while airborne, so landing within
    /// the buffer jumps immediately.
    pub jump_buffer_frames: u8,
    /// Tallest ledge, in cells, walked up without jumping. 2 rather than
    /// 1 because `rigid::mine` leaves rubble and worldgen terrain is
    /// rough — a 1-cell step-up feels sticky on exactly the ground this
    /// game produces.
    pub step_up: u8,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            gravity: 0.15,
            run_accel: 0.13,
            run_max: 1.3,
            ground_decel: 0.25,
            air_control: 0.5,
            jump_impulse: 2.0,
            fall_clamp: 4.0,
            coyote_frames: 6,
            jump_buffer_frames: 4,
            step_up: 2,
        }
    }
}

impl Tuning {
    /// Where the panel persists these, beside the other asset files.
    pub const ASSET_PATH: &'static str = "assets/player.ron";

    /// Load from `ASSET_PATH`, defaults when absent — absent is the
    /// normal case for a fresh checkout, same as `explosion::Tuning`.
    pub fn load() -> Self {
        std::fs::read_to_string(Self::ASSET_PATH)
            .ok()
            .and_then(|text| ron::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Full re-serialization, like `explosion::Tuning::save` and unlike
    /// the material files' careful span-edit: this file is generated, has
    /// no comments to lose, and each field's reasoning lives on the
    /// struct itself.
    pub fn save(&self) -> Result<(), String> {
        let pretty = ron::ser::PrettyConfig::new().struct_names(false);
        let text = ron::ser::to_string_pretty(self, pretty).map_err(|e| e.to_string())?;
        std::fs::write(Self::ASSET_PATH, text).map_err(|e| e.to_string())
    }
}

/// One tick's worth of intent, assembled by the windowing layer from held
/// keys and handed through `App::update`. This is the replay unit: same
/// build, same world, same `PlayerInput` sequence — same run.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerInput {
    pub left: bool,
    pub right: bool,
    pub jump_held: bool,
    /// Edge-triggered: true only on the tick after the key went down.
    /// `App::update` clears it after the first simulated tick of a frame,
    /// so a catch-up burst of ticks can't multi-fire one press.
    pub jump_pressed: bool,
    /// Reserved: crouch on ground, swim-down in water (phase 3).
    pub down: bool,
    /// Cursor in world coordinates, for the phase-2 dig aim. Plumbed now
    /// so the input path doesn't need reworking then.
    pub aim: Option<(i32, i32)>,
}

/// The character. Position is the rectangle's top-left corner in world
/// space, fractional like `ChunkBody`'s origin — the integer rectangle
/// (`rect_origin`) is what collides and draws.
#[derive(Clone, Debug)]
pub struct Player {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    /// A blocking cell sits in the row under the feet.
    pub grounded: bool,
    /// The depenetration pass found no way to free the rectangle: the
    /// gnome is entombed. Movement and jumping are dead until something
    /// changes — phase 2's dig is the escape verb.
    pub buried: bool,
    coyote: u8,
    jump_buffer: u8,
    /// Last tick's `jump_held`, for the release edge that halves an
    /// ascending `vy` — the variable-height jump.
    jump_was_held: bool,
}

impl Player {
    /// Spawn with the rectangle centred on `(x, y)`.
    pub fn at(x: i32, y: i32) -> Self {
        Self {
            x: (x - PLAYER_WIDTH / 2) as f32,
            y: (y - PLAYER_HEIGHT / 2) as f32,
            vx: 0.0,
            vy: 0.0,
            grounded: false,
            buried: false,
            coyote: 0,
            jump_buffer: 0,
            jump_was_held: false,
        }
    }

    /// Top-left of the occupied cell rectangle.
    pub fn rect_origin(&self) -> (i32, i32) {
        (self.x.round() as i32, self.y.round() as i32)
    }

    /// Inclusive world-space bounds, for the renderer's dirty rect.
    pub fn bounds(&self) -> (i32, i32, i32, i32) {
        let (x, y) = self.rect_origin();
        (x, y, x + PLAYER_WIDTH - 1, y + PLAYER_HEIGHT - 1)
    }
}

/// Whether the cell at `(x, y)` stops the character. Powder deliberately
/// blocks in phase 1: it means the gnome stands *on* sand piles and
/// climbs them by step-up along the angle of repose, which is
/// correct-feeling and free — wading and sinking are phase 3. Liquid and
/// gas pass through (he sinks; swimming is phase 3 too). Raw material
/// kind rather than `is_empty`, so a managed liquid body's container
/// cells (materially empty) read as passable space, which is what they
/// look like.
fn blocks(world: &World, x: i32, y: i32) -> bool {
    if !world.in_bounds(x, y) {
        return true; // OUT_OF_BOUNDS is solid: world-edge walls for free
    }
    matches!(
        world.materials.kind(world.get(x, y).material),
        MaterialKind::Solid | MaterialKind::Powder | MaterialKind::Plant | MaterialKind::Creature
    )
}

/// Whether the whole `PLAYER_WIDTH` x `PLAYER_HEIGHT` rectangle with
/// top-left `(x, y)` is free of blocking cells. 18 reads; the sweep calls
/// this a handful of times per tick, which is noise next to one chunk.
fn rect_clear(world: &World, x: i32, y: i32) -> bool {
    for dy in 0..PLAYER_HEIGHT {
        for dx in 0..PLAYER_WIDTH {
            if blocks(world, x + dx, y + dy) {
                return false;
            }
        }
    }
    true
}

/// One simulation tick. Runs in `App::update`'s serial phase; reads the
/// grid, never writes it (the ghost contract).
pub fn step(world: &mut World, input: PlayerInput, tuning: &Tuning) {
    let Some(mut p) = world.player.take() else {
        return;
    };

    // Free an invaded rectangle first, so this tick's movement starts
    // from a legal position: sand that fell into us, a body that settled
    // on us. Shortest clear push wins; up is tried first at each distance
    // because material arrives from above, and "on top of the pile" is
    // the right place to end up.
    depenetrate(world, &mut p);

    if p.buried {
        // Entombed: no movement, no jump, velocities dead. Coyote and the
        // jump buffer still tick down so nothing fires the instant the
        // gnome is freed.
        p.vx = 0.0;
        p.vy = 0.0;
        p.coyote = p.coyote.saturating_sub(1);
        p.jump_buffer = p.jump_buffer.saturating_sub(1);
        p.jump_was_held = input.jump_held;
        world.player = Some(p);
        return;
    }

    // --- intent to velocity ---
    let accel = if p.grounded { tuning.run_accel } else { tuning.run_accel * tuning.air_control };
    match (input.left, input.right) {
        (true, false) => p.vx -= accel,
        (false, true) => p.vx += accel,
        // No input (or both, cancelling): ground friction bleeds speed
        // off; airborne speed is kept so arcs stay committed.
        _ if p.grounded => {
            let drop = tuning.ground_decel.min(p.vx.abs());
            p.vx -= drop * p.vx.signum();
        }
        _ => {}
    }
    p.vx = p.vx.clamp(-tuning.run_max, tuning.run_max);

    if input.jump_pressed {
        p.jump_buffer = tuning.jump_buffer_frames;
    } else {
        p.jump_buffer = p.jump_buffer.saturating_sub(1);
    }
    if p.grounded {
        p.coyote = tuning.coyote_frames;
    } else {
        p.coyote = p.coyote.saturating_sub(1);
    }
    if p.jump_buffer > 0 && p.coyote > 0 {
        p.vy = -tuning.jump_impulse;
        p.jump_buffer = 0;
        p.coyote = 0;
    }
    // Variable height: releasing the key on the way up halves the rise,
    // once, on the release edge.
    if p.jump_was_held && !input.jump_held && p.vy < 0.0 {
        p.vy *= 0.5;
    }
    p.jump_was_held = input.jump_held;

    p.vy = (p.vy + tuning.gravity).min(tuning.fall_clamp);

    // --- the sweep: substepped at <= 1 cell, X (with step-up) then Y ---
    // Same anti-tunnelling shape as `rigid::advance` and for the same
    // reason: a fast fall must not cross a thin floor between samples.
    let distance = (p.vx * p.vx + p.vy * p.vy).sqrt();
    let steps = distance.ceil().max(1.0) as i32;
    let (step_x, step_y) = (p.vx / steps as f32, p.vy / steps as f32);
    for _ in 0..steps {
        if step_x != 0.0 {
            let next_x = p.x + step_x;
            let (nxi, nyi) = (next_x.round() as i32, p.y.round() as i32);
            if rect_clear(world, nxi, nyi) {
                p.x = next_x;
            } else {
                // Step-up: try the same horizontal move lifted by up to
                // `step_up` whole cells. Grounded only — the mid-air
                // version is a climb, not a step.
                let mut climbed = false;
                if p.grounded {
                    for lift in 1..=tuning.step_up as i32 {
                        if rect_clear(world, nxi, nyi - lift) {
                            p.x = next_x;
                            p.y -= lift as f32;
                            climbed = true;
                            break;
                        }
                    }
                }
                if !climbed {
                    p.vx = 0.0;
                }
            }
        }
        if step_y != 0.0 {
            let next_y = p.y + step_y;
            let (nxi, nyi) = (p.x.round() as i32, next_y.round() as i32);
            if rect_clear(world, nxi, nyi) {
                p.y = next_y;
            } else {
                // Landing or head bonk: the vertical axis dies, the
                // horizontal one keeps whatever it had — same per-axis
                // reasoning as `rigid::advance`.
                p.vy = 0.0;
            }
        }
    }

    // Grounded: any blocker in the row directly under the feet.
    let (xi, yi) = p.rect_origin();
    p.grounded = (0..PLAYER_WIDTH).any(|dx| blocks(world, xi + dx, yi + PLAYER_HEIGHT));

    world.player = Some(p);
}

/// Push an invaded rectangle to the nearest clear position within
/// `DEPENETRATE_REACH`, or mark the player buried. Up is preferred at
/// each distance (see `step`'s call-site comment), then sideways, then
/// down — down last because being squeezed downward through a floor gap
/// is the least expected outcome of being landed on.
fn depenetrate(world: &World, p: &mut Player) {
    let (xi, yi) = p.rect_origin();
    if rect_clear(world, xi, yi) {
        p.buried = false;
        return;
    }
    for d in 1..=DEPENETRATE_REACH {
        for (dx, dy) in [(0, -d), (-d, 0), (d, 0), (0, d)] {
            if rect_clear(world, xi + dx, yi + dy) {
                p.x += dx as f32;
                p.y += dy as f32;
                p.buried = false;
                // Whatever momentum the player had is spent shoving free.
                p.vx = 0.0;
                p.vy = 0.0;
                return;
            }
        }
    }
    p.buried = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::cell::Cell;
    use crate::sim::chunk::Rect;
    use crate::sim::material;

    /// A 128x96 world with a solid stone floor across the bottom 8 rows.
    fn world_with_floor() -> World {
        let mut world = World::new(Rect::new(0, 0, 127, 95));
        for y in 88..=95 {
            for x in 0..=127 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        world
    }

    fn tick(world: &mut World, input: PlayerInput) {
        step(world, input, &Tuning::default());
    }

    #[test]
    fn falls_under_gravity_and_lands_on_the_floor() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(64, 40));
        for _ in 0..300 {
            tick(&mut world, PlayerInput::default());
        }
        let p = world.player.as_ref().unwrap();
        assert!(p.grounded, "should have landed by now");
        let (_, _, _, feet) = p.bounds();
        assert_eq!(feet, 87, "feet should rest directly on the floor at y=88");
        assert_eq!(p.vy, 0.0, "vertical speed dies on landing");
    }

    #[test]
    fn runs_right_and_stops_when_the_key_lifts() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(20, 84));
        for _ in 0..60 {
            tick(&mut world, PlayerInput { right: true, ..Default::default() });
        }
        let moved_to = world.player.as_ref().unwrap().x;
        assert!(moved_to > 40.0, "a second of running should cover real ground, got x={moved_to}");
        for _ in 0..30 {
            tick(&mut world, PlayerInput::default());
        }
        let p = world.player.as_ref().unwrap();
        assert_eq!(p.vx, 0.0, "ground friction should stop him within a few ticks");
        assert!(p.x - moved_to < 8.0, "he should not coast far after release");
    }

    #[test]
    fn jump_rises_roughly_thirteen_cells_and_returns() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(64, 84));
        // Settle onto the floor first so coyote/grounded are real.
        for _ in 0..20 {
            tick(&mut world, PlayerInput::default());
        }
        let rest_y = world.player.as_ref().unwrap().y;
        tick(&mut world, PlayerInput { jump_pressed: true, jump_held: true, ..Default::default() });
        let mut apex = rest_y;
        for _ in 0..120 {
            tick(&mut world, PlayerInput { jump_held: true, ..Default::default() });
            apex = apex.min(world.player.as_ref().unwrap().y);
        }
        let rise = rest_y - apex;
        assert!((10.0..=16.0).contains(&rise), "expected a 10-16 cell jump, got {rise:.1}");
        let p = world.player.as_ref().unwrap();
        assert!(p.grounded, "should be back on the floor");
    }

    #[test]
    fn releasing_jump_early_cuts_the_height() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(64, 84));
        for _ in 0..20 {
            tick(&mut world, PlayerInput::default());
        }
        let rest_y = world.player.as_ref().unwrap().y;
        tick(&mut world, PlayerInput { jump_pressed: true, jump_held: true, ..Default::default() });
        // Hold for three ticks, then release.
        for _ in 0..3 {
            tick(&mut world, PlayerInput { jump_held: true, ..Default::default() });
        }
        let mut apex = rest_y;
        for _ in 0..120 {
            tick(&mut world, PlayerInput::default());
            apex = apex.min(world.player.as_ref().unwrap().y);
        }
        let rise = rest_y - apex;
        assert!(rise < 10.0, "a tapped jump should rise well short of a held one, got {rise:.1}");
        assert!(rise >= 2.0, "but it should still leave the ground, got {rise:.1}");
    }

    #[test]
    fn steps_up_a_two_cell_ledge_but_not_a_four_cell_wall() {
        let mut world = world_with_floor();
        // A 2-cell-high ledge ahead, then further along a 4-cell wall.
        for x in 70..=127 {
            for y in 86..88 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for x in 100..=127 {
            for y in 82..86 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        world.player = Some(Player::at(50, 84));
        for _ in 0..200 {
            tick(&mut world, PlayerInput { right: true, ..Default::default() });
        }
        let p = world.player.as_ref().unwrap();
        let (x, _) = p.rect_origin();
        assert!(x >= 70, "should have climbed the 2-cell ledge, stuck at x={x}");
        assert!(x < 100, "should be stopped by the 4-cell wall, got past to x={x}");
    }

    #[test]
    fn the_world_edge_is_a_wall() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(3, 84));
        for _ in 0..120 {
            tick(&mut world, PlayerInput { left: true, ..Default::default() });
        }
        let (x, _) = world.player.as_ref().unwrap().rect_origin();
        assert_eq!(x, 0, "should be pressed against the left edge, not through it");
    }

    #[test]
    fn sand_falling_into_the_rect_pushes_him_out_and_entombment_sets_buried() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(64, 84));
        for _ in 0..20 {
            tick(&mut world, PlayerInput::default());
        }
        // One sand cell lands inside the rectangle: pushed free, not buried.
        let (xi, yi) = world.player.as_ref().unwrap().rect_origin();
        world.set(xi + 1, yi + PLAYER_HEIGHT - 1, Cell::new(material::SAND, 0));
        tick(&mut world, PlayerInput::default());
        let p = world.player.as_ref().unwrap();
        assert!(!p.buried, "one intruding cell should be escapable");
        let (nx, ny) = p.rect_origin();
        assert!(rect_clear(&world, nx, ny), "the rect should be clear after depenetration");

        // Entomb him completely: buried, and motionless.
        let (xi, yi) = world.player.as_ref().unwrap().rect_origin();
        for dy in -(DEPENETRATE_REACH + 1)..(PLAYER_HEIGHT + DEPENETRATE_REACH + 1) {
            for dx in -(DEPENETRATE_REACH + 1)..(PLAYER_WIDTH + DEPENETRATE_REACH + 1) {
                world.set(xi + dx, yi + dy, Cell::new(material::SAND, 0));
            }
        }
        tick(&mut world, PlayerInput { jump_pressed: true, jump_held: true, ..Default::default() });
        let p = world.player.as_ref().unwrap();
        assert!(p.buried, "fully enclosed should read as buried");
        assert_eq!((p.vx, p.vy), (0.0, 0.0), "no movement while buried");
    }

    #[test]
    fn coyote_allows_a_late_jump_off_a_ledge() {
        let mut world = world_with_floor();
        // Floor only under the left half; a cliff edge at x=64.
        for y in 88..=95 {
            for x in 64..=127 {
                world.set(x, y, Cell::EMPTY);
            }
        }
        world.player = Some(Player::at(56, 84));
        for _ in 0..20 {
            tick(&mut world, PlayerInput::default());
        }
        // Run off the edge, then jump 3 ticks after leaving the ground.
        let mut off_ground_ticks = 0;
        for _ in 0..200 {
            let jump = off_ground_ticks == 3;
            let p = world.player.as_ref().unwrap();
            if !p.grounded {
                off_ground_ticks += 1;
            }
            tick(
                &mut world,
                PlayerInput { right: true, jump_pressed: jump, jump_held: jump, ..Default::default() },
            );
            if world.player.as_ref().unwrap().vy < -1.0 {
                return; // the late jump fired: rising fast — pass
            }
        }
        panic!("a jump 3 ticks after walking off the ledge should still fire (coyote)");
    }
}
