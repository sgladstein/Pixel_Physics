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

/// How far past the bore a dig may throw its spoil, and how far a *buried*
/// dig may throw it. Constants rather than tunables on purpose: neither is
/// a feel knob the panel should sweep — the first is the shove distance
/// `rigid::DISPLACE_SEARCH` already fixes at 4 for bodies, and the second
/// is "far enough to reach the surface of a pile that could plausibly have
/// buried you", which is a reachability question, not a taste one. See
/// `dig` for why they differ.
const SPOIL_THROW: i32 = 4;
const BURIED_THROW: i32 = 16;

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
    /// How far from the gnome's centre a dig can land, in cells. The dig
    /// point is clamped onto this circle along the aim ray, so clicking
    /// across the map digs at arm's length toward the cursor rather than
    /// doing nothing.
    pub dig_reach: u8,
    /// Radius of one dig bite. 4 bores a 9-cell hole — two overlapping
    /// bites make a tunnel a 3x6 gnome walks through upright.
    pub dig_radius: u8,
    /// Ticks between bites while the button is held. 8 is ~7 bites a
    /// second: fast enough to feel like digging, slow enough that each
    /// bite's crack/impulse feedback reads individually.
    pub dig_cooldown: u8,
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
            dig_reach: 14,
            dig_radius: 4,
            dig_cooldown: 8,
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
    /// Ticks until the next dig bite may land. Sim state rather than UI
    /// state, so a replayed input sequence digs on the same ticks.
    dig_cooldown: u8,
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
            dig_cooldown: 0,
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

    /// Centre of the occupied rectangle — where reach is measured from.
    pub fn center(&self) -> (i32, i32) {
        let (x, y) = self.rect_origin();
        (x + PLAYER_WIDTH / 2, y + PLAYER_HEIGHT / 2)
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
        p.dig_cooldown = p.dig_cooldown.saturating_sub(1);
        p.jump_was_held = input.jump_held;
        world.player = Some(p);
        return;
    }
    p.dig_cooldown = p.dig_cooldown.saturating_sub(1);

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

/// One dig bite toward `aim` — the phase-2 verb. A no-op without a
/// summoned player or while the cooldown is running. Two steps, and both
/// matter:
///
/// - **`rigid::mine`** at the bite point: cracks, detachment, structural
///   scheduling, a pressure impulse — everything that makes a cut *felt*.
///   It converts rock to rubble in place, though, so on its own a dig
///   loosens a bore without opening one.
/// - **Displacement** then shoves the loose material (the fresh rubble,
///   sand, water) out of the bite to the nearest free cells beyond it —
///   the same shove-don't-delete contract `rigid::displace` keeps for
///   bodies, so digging conserves every cell. Spoil surfaces at the bore
///   mouth or along the tunnel behind, which is where dug material should
///   end up. In a sealed pocket with nowhere free, material stays put and
///   the dig genuinely cannot advance — a full pocket is full.
///
/// While `buried`, the bite auto-aims at the gnome's own centre whatever
/// `aim` says: the M9 "buried and dig out" escape. The displacement is
/// what actually frees him — burying material is loose by nature, and
/// shoving it out of the rectangle gives the depenetration pass somewhere
/// to stand him up.
pub fn dig(world: &mut World, aim: (i32, i32), tuning: &Tuning) -> Option<Bite> {
    let mut p = world.player.take()?;
    let bite = if p.dig_cooldown == 0 {
        p.dig_cooldown = tuning.dig_cooldown;
        let (cx, cy) = p.center();
        // Buried, the bite auto-aims *above his head* rather than at his
        // own centre, and the difference is the whole escape. Centred on
        // himself, the disc reaches as far below his feet as above his
        // hat: on `scene=bury` that mined the stone floor he was standing
        // on into rubble, threw it clear, and dropped him into the hole —
        // he ended eight cells *lower* than he started, sealed inside the
        // floor, with the pile still on top. Aiming high puts the bore
        // between him and the surface, which is both the direction a
        // buried digger actually works and the direction `depenetrate`
        // already prefers, so the two pull the same way and he climbs out
        // a few cells per bite.
        let (_, y0, _, _) = p.bounds();
        let at = if p.buried {
            (cx, y0 - 1)
        } else {
            face_toward(world, (cx, cy), aim, tuning.dig_reach as i32)
        };
        let radius = tuning.dig_radius as i32;
        crate::sim::rigid::mine(world, at.0, at.1, radius);
        // How far spoil may be thrown, and the two cases genuinely differ.
        //
        // A bite at a rock face only ever needs to shove material a cell or
        // two: it is standing in the open space it came in through, so the
        // near ring is free and a long search would never be reached
        // anyway. Digging *out of a burial* is the opposite problem. The
        // rectangle is full, every one of its cells has to find a home
        // before `depenetrate` can stand him up, and under a dumped pile
        // the nearest free cell is the pile's own surface. Measured on
        // `scene=bury`: at the short reach a buried gnome dug 34 bites and
        // moved zero cells — the pile had no opening within four cells of
        // the bore, so the escape simply did not exist, which is not what
        // M9 asks for ("buried by a sand dump and dig out"). The long
        // reach is the case where throwing material to the surface is
        // *also* what a digger actually does, so it costs nothing in
        // plausibility. It is still bounded: bury him deep enough and the
        // surface is out of range and he stays under, which is the right
        // answer rather than a missing feature.
        let search = if p.buried { radius + BURIED_THROW } else { radius + SPOIL_THROW };
        let displaced = displace_disc(world, &p, at.0, at.1, radius, search);
        Some(Bite { at, displaced })
    } else {
        None
    };
    world.player = Some(p);
    bite
}

/// What one bite actually did. Returned rather than kept private because
/// of the failure `CLAUDE.md` records under "did it fire at all needs a
/// counter": a bore full of loose rubble and a bore the dig never touched
/// are the same picture, and only a count separates them. The filmstrip
/// scenes print these next to the tile; `App` discards them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bite {
    /// Where the bite landed after reach-clamping — not where the cursor
    /// was, which is the whole point of the clamp.
    pub at: (i32, i32),
    /// Loose cells shoved clear of the bore. Zero with a non-zero bite is
    /// meaningful, not a bug: it is what a dig into open air looks like.
    pub displaced: usize,
}

/// Where a bite aimed at `aim` actually lands: the **first blocking cell
/// along the ray** from `from`, stopping at `reach`.
///
/// This started as a plain clamp of the cursor onto the circle of
/// `reach`, and that was wrong in a way worth recording, because it looks
/// right and passes an obvious test. Clicking deep inside a massif put
/// the bite *behind* the rock face, carving a sealed pocket several cells
/// in — and a sealed pocket has nowhere to put its own spoil, so the
/// stone turned to rubble in place and stayed there. On screen that is a
/// dig that does nothing: rubble and stone are near enough the same grey
/// (deliberately — see `assets/materials/rubble.ron`) that the player
/// gets no feedback at all. Digging the near face instead is what a
/// pickaxe does, keeps the cut where the player is looking, and
/// guarantees the bore always has the open space the digger came in
/// through on its far side, which is where spoil goes.
///
/// Aiming into open air within reach digs where the cursor is — the bite
/// finds nothing to displace and costs nothing, which is the right
/// outcome for a swing at nothing.
fn face_toward(world: &World, from: (i32, i32), aim: (i32, i32), reach: i32) -> (i32, i32) {
    let (dx, dy) = ((aim.0 - from.0) as f32, (aim.1 - from.1) as f32);
    let dist = (dx * dx + dy * dy).sqrt();
    if dist == 0.0 {
        return from;
    }
    let limit = dist.min(reach as f32);
    // Sampled at whole-cell steps along the ray, like the movement
    // sweep's substepping and for the same reason: a diagonal aim must
    // not step past a one-cell-thick wall between samples.
    let steps = limit.ceil() as i32;
    let (sx, sy) = (dx / dist, dy / dist);
    let mut last = from;
    for i in 1..=steps {
        let t = (i as f32).min(limit);
        let cell = (from.0 + (sx * t).round() as i32, from.1 + (sy * t).round() as i32);
        if blocks(world, cell.0, cell.1) {
            return cell;
        }
        last = cell;
    }
    last
}

/// Shove every loose cell in the dig disc to the nearest empty cell
/// outside it, and report how many moved — ring by ring, the same
/// nearest-opening reasoning as `rigid::displace`, rewritten here because
/// that one is private to a body's own occupancy set. The search reaches
/// a few cells past the disc so spoil lands just beyond the bore rather
/// than teleporting; a cell with nowhere to go stays where it is (never
/// deleted), which is what makes a sealed pocket genuinely undiggable
/// rather than a slow leak of material out of the world.
///
/// Only `Powder` and `Liquid` move. Solids do not, because `mine` has
/// already had its say about those — anything of its it could break is
/// rubble by the time this runs, and what is left is bedrock or another
/// material with no `breaks_into`, both of which should stop a dig.
/// `Plant` is likewise left alone: shoving one cell of an organism
/// somewhere else would tear it out of its own structure, so a root
/// currently stops a tunnel. That is a known gap, not a decision — phase
/// 4 owns planting and is where cutting through one belongs.
fn displace_disc(world: &mut World, p: &Player, cx: i32, cy: i32, radius: i32, search: i32) -> usize {
    let mut moved = 0;
    // The gnome's own cells first, then everything else.
    //
    // Not cosmetic ordering: openings run out. A bite into a tight spot
    // finds somewhere for the first several cells and nowhere for the
    // rest, so whichever cells are visited first are the ones that
    // actually move. Scanning the disc top-left to bottom-right spent
    // that budget on the rock above his head and left him sealed in —
    // a buried gnome dug for 300 ticks and never got out. His own
    // rectangle is the whole point of the bite while buried, and is
    // where the material is loosest when it is not, so it goes first.
    for pass in [true, false] {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy > radius * radius {
                    continue;
                }
                let (x, y) = (cx + dx, cy + dy);
                if inside_player(p, x, y) != pass {
                    continue;
                }
                let kind = world.materials.kind(world.get(x, y).material);
                if !matches!(kind, MaterialKind::Powder | MaterialKind::Liquid) {
                    continue;
                }
                'rings: for ring in (radius + 1).max(1)..=search {
                    for (rx, ry) in ring_offsets(ring) {
                        let (nx, ny) = (cx + rx, cy + ry);
                        if inside_player(p, nx, ny) {
                            continue; // not back into the gnome
                        }
                        if world.in_bounds(nx, ny) && world.is_empty(nx, ny) {
                            let moving = world.get(x, y);
                            world.set(nx, ny, moving);
                            world.set(x, y, super::cell::Cell::EMPTY);
                            moved += 1;
                            break 'rings;
                        }
                    }
                }
            }
        }
    }
    moved
}

/// The cells exactly `ring` away in Chebyshev distance, top row first,
/// then the two sides descending, then the bottom row.
///
/// The order is the feel, not an implementation detail: first match wins,
/// so preferring the top row means spoil surfaces *above* the bore where
/// a digger would throw it, and only falls back to sideways and behind
/// when up is solid. The perimeter is walked directly rather than by
/// scanning the full square and skipping its interior, which is what this
/// did first — the same order and result, roughly a tenth of the reads,
/// and this runs inside a doubly-nested loop over the whole disc.
fn ring_offsets(ring: i32) -> impl Iterator<Item = (i32, i32)> {
    let top = (-ring..=ring).map(move |rx| (rx, -ring));
    let sides = (-ring + 1..ring).flat_map(move |ry| [(-ring, ry), (ring, ry)]);
    let bottom = (-ring..=ring).map(move |rx| (rx, ring));
    top.chain(sides).chain(bottom)
}

fn inside_player(p: &Player, x: i32, y: i32) -> bool {
    let (x0, y0, x1, y1) = p.bounds();
    x >= x0 && x <= x1 && y >= y0 && y <= y1
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

    /// Every cell holding material, by the raw predicate rather than
    /// `is_empty` — the question here is "is there material here", which
    /// is exactly the distinction `CLAUDE.md` records against the
    /// managed-aware version.
    fn occupied_cells(world: &World) -> usize {
        let (w, h) = (128, 96);
        (0..h).map(|y| (0..w).filter(|&x| world.get(x, y).material != material::EMPTY).count()).sum()
    }

    /// Stone across the right half, from the surface down — a cliff face
    /// to tunnel into, with the gnome standing on the floor beside it.
    fn world_with_cliff() -> World {
        let mut world = world_with_floor();
        for y in 60..88 {
            for x in 70..=127 {
                world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        world
    }

    #[test]
    fn a_bite_opens_a_bore_and_loses_no_material() {
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        let before = occupied_cells(&world);
        let bite = dig(&mut world, (76, 84), &Tuning::default()).expect("a fresh gnome digs immediately");
        assert_eq!(bite.at.0, 70, "the bite lands on the near face, not behind it at the cursor");
        assert!(bite.displaced > 0, "biting into solid stone should shove spoil clear");
        // The bore is actually open, not merely loosened: `mine` alone
        // turns stone into rubble in place and would leave this full.
        let open = (-2..=2).filter(|dy| world.get(71, 84 + dy).material == material::EMPTY).count();
        assert!(open >= 4, "the bite should leave a hole, found {open} empty cells through its middle");
        assert_eq!(before, occupied_cells(&world), "digging must move material, never delete it");
    }

    #[test]
    fn a_bite_stops_at_the_first_face_rather_than_carving_a_sealed_pocket() {
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        // Aimed at the far side of the massif, well past the face and
        // still inside reach. The old clamp-to-reach rule bit here and
        // left a pocket of rubble buried in the rock.
        let bite = dig(&mut world, (79, 84), &Tuning::default()).expect("digs");
        assert_eq!(bite.at.0, 70, "expected the face at x=70, bit at {:?}", bite.at);
        assert!(
            world.get(79, 84).material == material::STONE,
            "the rock behind the face must be untouched, found {:?}",
            world.materials.get(world.get(79, 84).material).name
        );
    }

    #[test]
    fn the_cooldown_rate_limits_held_digging() {
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        let tuning = Tuning::default();
        assert!(dig(&mut world, (76, 84), &tuning).is_some(), "first bite fires");
        assert!(dig(&mut world, (76, 84), &tuning).is_none(), "a held button must not bite every frame");
        for _ in 0..tuning.dig_cooldown {
            tick(&mut world, PlayerInput::default());
        }
        assert!(dig(&mut world, (76, 84), &tuning).is_some(), "the cooldown should expire on its own ticks");
    }

    #[test]
    fn a_cursor_across_the_map_still_digs_the_near_face() {
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        let bite = dig(&mut world, (127, 84), &Tuning::default()).expect("a far click still digs");
        assert_eq!(bite.at.0, 70, "a click across the map digs the wall in front of him");
    }

    #[test]
    fn aimed_at_open_sky_the_bite_stops_at_reach() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(64, 84));
        let tuning = Tuning::default();
        // Straight up into empty sky: nothing blocks, so the ray runs out
        // at reach rather than following the cursor to the top of the world.
        let bite = dig(&mut world, (64, 0), &tuning).expect("digs");
        let (cx, cy) = world.player.as_ref().unwrap().center();
        let d = (((bite.at.0 - cx).pow(2) + (bite.at.1 - cy).pow(2)) as f32).sqrt();
        assert!(
            (d - tuning.dig_reach as f32).abs() <= 1.0,
            "expected a bite at reach ({}), landed {d:.1} away",
            tuning.dig_reach
        );
        assert_eq!(bite.displaced, 0, "a swing at empty air moves nothing");
    }

    #[test]
    fn a_buried_gnome_digs_himself_out() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(64, 84));
        for _ in 0..20 {
            tick(&mut world, PlayerInput::default());
        }
        // Entomb him in sand, as a dumped pile would.
        let (xi, yi) = world.player.as_ref().unwrap().rect_origin();
        for dy in -(DEPENETRATE_REACH + 2)..(PLAYER_HEIGHT + DEPENETRATE_REACH + 2) {
            for dx in -(DEPENETRATE_REACH + 2)..(PLAYER_WIDTH + DEPENETRATE_REACH + 2) {
                if yi + dy < 88 {
                    world.set(xi + dx, yi + dy, Cell::new(material::SAND, 0));
                }
            }
        }
        tick(&mut world, PlayerInput::default());
        assert!(world.player.as_ref().unwrap().buried, "the pile should bury him");
        let before = occupied_cells(&world);

        // Dig with the cursor pointed somewhere useless: buried aims at
        // himself regardless, which is the whole escape.
        let tuning = Tuning::default();
        let mut bites = 0;
        for _ in 0..300 {
            if dig(&mut world, (0, 0), &tuning).is_some() {
                bites += 1;
            }
            tick(&mut world, PlayerInput::default());
            if !world.player.as_ref().unwrap().buried {
                assert!(bites > 0, "he got out without ever digging — the test is not testing the dig");
                assert_eq!(before, occupied_cells(&world), "digging out must not delete the pile");
                return;
            }
        }
        panic!("still buried after 300 ticks of digging");
    }

    #[test]
    fn spoil_never_lands_inside_the_gnome() {
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        let tuning = Tuning::default();
        for _ in 0..40 {
            dig(&mut world, (74, 84), &tuning);
            let p = world.player.as_ref().unwrap();
            let (x0, y0, x1, y1) = p.bounds();
            for y in y0..=y1 {
                for x in x0..=x1 {
                    assert!(
                        world.get(x, y).material == material::EMPTY,
                        "spoil was shoved into the gnome's own rectangle at ({x}, {y})"
                    );
                }
            }
            tick(&mut world, PlayerInput::default());
        }
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
