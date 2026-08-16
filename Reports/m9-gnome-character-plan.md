# The Gnome — a playable character (M9)

## Context

The owner wants an actual character in the world: a gnome who runs, jumps, digs caves/tunnels/mines, grows plants, and eventually throws or places explosives. `PLAN.md:232` already specs this as **M9 — Character physics** ("player as a kinematic body with sand-aware movement: walking on debris, being buried, swimming in liquid"), gated on M8's collision work — which now exists (`rigid.rs`'s `blocked_axis`/`displace`/`settle`). There is no player code anywhere today; the closest analogs are `rigid::ChunkBody` (off-grid debris bodies) and the destruction verbs `mine`/`strike`/`Blasts::trigger_with`, all fully built.

Owner decisions (asked and answered):
- **Summoned by key** (opt-in; sandbox stays pure debug-tool by default)
- **WASD movement**, rebinding debug verbs worm `W→J` and mine `D→H`
- **Planting before explosives** (phase 4 vs 5) — fits the nature-gnome identity

Five phases, each independently playable and judged by playing (per CLAUDE.md ethos: satisfying > correct; verify live, not by tests alone).

## Core architecture (applies to all phases)

**A dedicated `Player` struct in new `src/sim/player.rs`, stored on `World`, off-grid, ghost-first.**

- **Not a `ChunkBody`** — bodies settle after 3 stalled frames, spin, and burrow through powder; a character must never settle, never rotate, and should stand *on* sand. Don't factor a shared collider out of `rigid.rs` either: the player is an axis-aligned rect whose sweep samples only its leading edge — a ~40-line dedicated AABB sweep in `player.rs` is cheaper than generalizing `blocked_axis` and can't perturb settled collapse behavior.
- **Size 3 wide × 6 tall** cells; position as f32 `x, y` (feet-center) + `vx, vy`, fractional accumulation like `ChunkBody`.
- **State**: `pub player: Option<Player>` on `World` next to `chunk_bodies` (world.rs:105). Plain struct, no HashMap (determinism convention). Input arrives at App and is passed in per tick as a `PlayerInput` value, keeping the sim step a pure function of (world, input) — the replay unit for determinism.
- **Ghost-first occupancy**: the player writes no grid cells. CA sand falls through the player's rect; bodies ignore it. This preserves the core perf property — an idle player wakes zero chunks. Overlap is fixed reactively per tick (depenetration, below). Noted follow-up only if play demands: a player-AABB check in `blocked_axis` (cheap); never temporary Creature cells (wakes chunks), never a CA-side check (hot loop).
- **Step phase**: `player::step(&mut world, input)` in `App::update` (app.rs:753) immediately after `rigid::step_chunk_bodies` (app.rs:781) and before `step_active_sites` — the serial body slot, for the write-disjointness reason documented at that call site.
- **Rendering**: clone the `draw_chunk_bodies` mechanism (render.rs:587): `draw_player` paints a hardcoded 3×6 colored-cell sprite table (pointed hat, face, boots — real sprite later) on top of the cell pass; union the player's current rect + a `last_player_rect` into the dirty region alongside `body.bounds()`/`last_body_rects` (render.rs:457–474). **Skip the union entirely when the player hasn't moved a whole pixel** — this keeps the §11 dirty-rect skip alive with an idle gnome on a settled world (explicitly verified in phase 1).
- **Tunables**: new `TunableGroup::Player` in `src/tunables.rs` backed by `player::Tuning` with `assets/player.ron` — straight copy of the `explosion::Tuning` pattern (explosion.rs:194–217), including the `App::save_tunable` branch. Live feel-tuning under `O` is the repo's established method.

## Phase 1 — Entity, run, jump, collision

**Goal**: summon a gnome (`U` at cursor; press again to dismiss), run and jump around existing terrain, feel good.

**Files**: create `src/sim/player.rs`, `assets/player.ron`; modify `src/sim/mod.rs`, `src/sim/world.rs`, `src/app.rs`, `src/main.rs`, `src/render.rs`, `src/tunables.rs`.

**Movement tunables** (defaults vs `GRAVITY = 0.15` at 60 Hz): `run_accel 0.13`, `run_max 1.3`, `ground_decel 0.25`, `air_control 0.5`, `jump_impulse 2.0` (→ ~13-cell jump, apex ~13 frames), `fall_clamp 4.0`, `coyote_frames 6`, `jump_buffer_frames 4`, `step_up 2` (2 not 1 — `mine()` leaves rubble; rough terrain is the norm). Variable jump: on release while `vy < 0`, halve `vy`.

**Collision (deliberately simple)**: `Solid | Plant | Creature | Powder` block — powder-as-solid means the gnome walks on sand piles and climbs them via step-up; wading is phase 3. `Liquid | Gas | Empty` pass through. Sweep substeps ≤1 cell, per-axis, X-with-step-up then Y. Grounded = any blocker in the row below the feet. Use `world.is_empty` (managed-aware = "is this position available") for clearance; OUT_OF_BOUNDS reads solid = free world-edge walls. **Depenetration**: if the rect gets invaded (sand fell in, body settled on us), push out along the shortest clear axis up to 4 cells; no clear push → `buried` flag (movement zeroed; phase 2's dig is the escape).

**Input plumbing** (`src/main.rs`): extend the KeyboardInput arm (main.rs:433) to deliver `Released` too; route A/D/W/S into a `HeldKeys` struct on `Handler` (the `painting`/`erasing` booleans are the precedent), keep `Pressed && !repeat` routing to `key()` for everything else; record edge-triggered `jump_pressed` for the buffer. **Rebinds in the same commit**: worm `W→J`, mine `D→H` (both free); `S` needs no rebind (save is gated on `show_tunables`, main.rs:346). Update the help panel (`/`). Per tick, assemble `PlayerInput { left, right, jump_held, jump_pressed, down, aim: Option<(i32,i32)> }` (aim = cursor via `renderer.screen_to_world`) and pass through `App::update`.

**Verify**: run/jump onto a `stamp_reference_room` roof (live-tune in the PLAYER group if 13 cells doesn't clear it). `PIXEL_PHYSICS_CAPTURE_SEQUENCE` a run-jump-land arc; judge shape and landing. **Frame cost, quoted both ways**: settled world + idle gnome must match no-gnome baseline (dirty-rect skip survives, F1 shows no awake chunks); running gnome redraws only its ~5×8 rect. Dump sand on him: falls through (expected), depenetration lands him on top or sets `buried` — note how it reads for phase 3.

## Phase 2 — Dig, aimed at the cursor

**Goal**: carve gnome-scale tunnels; dig out when buried.

- Cursor-aimed, reach-limited — matches every existing verb. While the gnome exists, holding LMB with the Brush tool **digs instead of paints** when the cursor is within reach (gate in the `painting`-held path; `about_to_wait` already re-fires while held, giving held-to-dig for free). Fallback if that fights the painting code: add `Tool::Dig` to the `Tool` cycle (app.rs:186).
- Clamp the dig point to `dig_reach` (default 14, tunable) along the character→cursor ray; call `rigid::mine(world, dx, dy, dig_radius)` (rigid.rs:805) every `dig_cooldown` frames while held. Defaults: radius 4 (9-cell bore — two bites make a comfortable tunnel for a 3×6 gnome), cooldown 8 (~7 digs/s). `mine` already does the satisfying part: rubble not vacuum, cracks, detachment, impulse.
- When `buried`, digging auto-aims at the blocking cells — the M9 "buried and dig out" verify line.
- **Measure-first flag**: `mine` schedules structural checks but never calls `structural::relax_region` (only the paint path does, world.rs:1440). Tunnel 60 cells under an overhang and watch when the ceiling reacts; only if it reads wrong, add a `relax_region` cadence — and measure its frame cost first.
- Verify rubble underfoot explicitly: step_up 2 + powder-as-solid should let him walk out of his own spoil.

**Verify**: tunnel through the cliff preset and walk through; bury him with a sand dump and dig out (capture-sequence it — the M9 money shot); quote frame cost while held-digging.

## Phase 3 — Sand-aware refinement: burial and swimming (M9 proper)

- **Powder wading**: ≤2 powder rows overlapping the feet → don't depenetrate; slow horizontal movement (`wade_slowdown` ~0.4×), sink 1–2 cells into fresh piles. Fully enclosed → `buried` (dig only).
- **Swimming**: head cells Liquid → buoyancy (`gravity × -0.3` net), velocity damping ~0.9/frame, W = upward stroke (impulse ~0.8, cooldown ~10), S = swim down. Stroke at the surface + coyote hops him out.
- **Standing on bodies** (M9: "stands on a tumbling rigid body"): grounded/step checks also test `world.chunk_bodies` cell positions near the feet — bodies are few, cost negligible. Bodies still don't know about the player (ghost holds).
- New tunables: `wade_slowdown`, `buoyancy`, `stroke_impulse`, `swim_damp`.

**Verify**: the M9 line verbatim — buried by a sand dump and dig out; swims (filmstrip a dive-and-surface); stands on a chunk struck off with `C` while it tumbles. Idle frame cost unchanged.

## Phase 4 — Planting

While the gnome exists, `T` (tree) and `M` (moss) clamp to `dig_reach` and only fire within it — same `plant_tree`/`plant_moss` calls (app.rs:1452, 1458), now proximity-cast. Without a gnome, unchanged (sandbox tools intact). Optional cheap flavor: small impulse + one-frame kneel offset on cast. The organism scheduler does the growing; no new systems.

**Verify**: dig a cave, plant moss on its walls and a tree at the mouth, watch them grow — pure play.

## Phase 5 — Explosives

**Thrown bomb first** — zero World plumbing: `Blasts` lives on `App` (app.rs:79), so a `Vec<Bomb>` on App (ballistic pos/vel entity cloned from `Particle`'s advance-and-land shape, particle.rs:349) stepped right before `self.blasts.step` calls `blasts.trigger_with(world, particles, x, y, radius, strength)` (explosion.rs:268) on landing or fuse expiry (~90 frames; drawn as a flashing 2×2 via the body-rect dirty mechanism). Bind `Y` = hurl toward cursor, speed scaled by distance like `spawn_burst`'s arc (app.rs:1398).

**Placed charge second**: a `blast_powder` material in `assets/materials/` with `ignition_temperature` set (fire.rs drives ignition purely from .ron), whose ignition pushes onto a new `world.pending_detonations: Vec<(i32,i32)>`; `App::update` drains it into `trigger_with` after `parallel::step`. Deterministic (Vec, sweep order), and the right long-term hook — chain reactions, powder-trail fuses, dig-into-a-pocket accidents all fall out. The gnome places a charge via a reach-clamped stamp.

**Verify**: throw a bomb into your own tunnel mouth; capture the collapse. Chain two placed charges with a powder trail lit by `F`.

## Risks / opens

- **Ghost trade-off (accepted)**: sand/bodies pass through the gnome; depenetration corrects. Escalation path noted above; decide from play, not in advance.
- **Crush feel**: depenetration could pop him through a thin ceiling — cap push at 4 cells, prefer `buried` over teleporting; judge in play.
- **relax_region during tunneling** — measure-first (phase 2).
- **Determinism**: input-driven, no wall-clock, `world.rng` only if randomness is ever needed; `PlayerInput` per tick is the replay unit. Extend/check `tests/determinism.rs` coverage once the player exists.
- **Camera fixed** — fine at 512×320; M10 explicitly out of scope (render.rs:239's comment stays true: "M10 moves it with the player").
- **Key debt**: `W→J`, `D→H` rebinds + help-panel update must land in the same commit as held-key tracking.
- **Wiki**: a shipped character is player-visible behavior — add a `wiki/` page (what the gnome does, controls) in the phase-1 change per the CLAUDE.md convention.
