# Shared piece: a player-driven ant in the lab (keys, camera follow, HUD)

Owner: **queen** seat. State: **BUILT, PR #904** (2026-10-25). A keyboard-held *ant* with a mind takes the hand-built path (all five slice-0 hooks); other species with minds still go to mind::tick (enemy's steer-mind).
Needed by: queen (the queen's body), scout (an ordinary worker's body). Guest/enemy need a *non-ant* body: that is a different piece (not claimed here).

## What exists
`src/sim/steer.rs` (slice 0, 2026-10-11): `steer::mark(world, id, input)` makes one ant player-driven; `SteerInput` carries dir + E eat, Q drop, F share, G dig, B bite, L lay, T trail. Five hooks in `creature.rs`. **Not wired to the lab binary** (no keys, no camera follow). Marking also applies the queen breeding rule to her colony and removes old age: the scout seat will need an option to mark *without* the queen rule.

## Interface (proposal)
- `lab::demo::steer_keys`: in demo mode, arrow keys steer the marked ant (WASD stays pan), verbs on their slice-0 letters where they don't collide with lab tools, camera follows the marked body (toggle).
- `steer::mark_with(world, id, input, SteerRole::{Queen, Worker})`: Worker = no queen breeding rule, normal age (scout asks; to confirm).
- A ring/marker draw on the steered body.

## Agreed with scout (2026-10-25 08:20Z)
- `SteerRole::Worker`: **no queen breeding rule, but still no old age** (owner's ruling is about the player's body). Hunger, bites, drowning still kill.
- Public read: `steer::steered_id(world) -> Option<OrganismId>` and `steer::is_steered(world, id) -> bool`; the seat's own module reads the body's position each tick. Follow/lead (tandem) is scout-owned, in scout's own files.

## Key layer in demo mode (proposal)
With `demo=` set and a body marked, a body key layer reads keys **before** the lab's own handler:
- **Arrows steer** (the speed dial moves to `PageUp`/`PageDown`; digits 1-7 still pick a preset). `WASD` stays pan.
- Slice-0 verbs keep their letters: `E` eat, `Q` drop, `F` share/feed, `G` dig, `B` bite, `L` lay, `T` trail.
- **Seat keys reserved for each seat's own verbs: `Y`, `U`, `P`, `J`, `K`, `R`** (R's rebuild is disabled in demo mode; rebuild from the menu). Queen uses some of these, scout others; seats don't collide because only one demo runs at a time.
- `H` toggles the seat's help card; `Tab` toggles camera-follow.
- Lab tools stay on the mouse bar.
