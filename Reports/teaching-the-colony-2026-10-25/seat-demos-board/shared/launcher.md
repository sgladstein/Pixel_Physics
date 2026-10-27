# Shared piece: the common demo launcher

Owner: **queen** seat (thread "Build the queen seat demo", branch claude/project-thread-epio3i).
State: **BUILT, PR #904** (head 7858f8ff, branch claude/project-thread-epio3i), at the merge desk 2026-10-25 09:45Z. Scout and colony mind asks folded in. Comments welcome: message the queen thread or write in your own status file.

## Interface (proposal)
- Launch: `cargo run --release --bin lab -- demo=<seat>` (also `PIXEL_PHYSICS_LAB_DEMO=<seat>`).
- `demo=<seat>` does two things, nothing else:
  1. loads the lab scenario `assets/lab_scenarios/demo_<seat>.ron` (the existing `scenario=` machinery: box, placements, timeline, parameter knobs). **Each seat owns its own .ron.**
  2. sets `Lab::demo: Option<DemoSeat>` so a seat's code can route keys and draw its own HUD. `DemoSeat` is a plain enum (Queen, Keeper, Guest, Mind, Enemy, Scout); each seat adds its own arms to the match in its own module (`src/lab/demo/<seat>.rs`), so seats don't edit each other's files.
- Off by default: with no `demo=`, the lab is byte-identical (the enum is `None`, every hook a branch not taken).
- An on-screen line names the demo and its keys (help card on `H`).

## Why a scenario and not a new binary
Every demo then gets the lab's time dial, pages, overlays, screenshot hooks (`PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES`, `PIXEL_PHYSICS_LAB_CLICK`) and the plain-speak ant pages for free, and stays one physics.

## Build
The launcher itself is ~50 lines in `src/bin/lab.rs` + `src/lab/mod.rs` + `src/lab/demo/mod.rs`. The queen thread builds it first, lands it early as its own small PR (so others can stack on main), and posts the merged SHA here.

## As built (PR #904)
- `cargo run --release --bin lab -- demo=<seat>` (or `PIXEL_PHYSICS_LAB_DEMO`). Seat names: queen keeper guest mind enemy scout (`colony-mind` also parses).
- `Lab::start_demo_with(seat, args)`; `Lab::demo: Option<demo::Demo>`; `Demo { seat, held, follow, args: BTreeMap<String,String>, mouse_held, queen: Option<..> }`. **Add one field for your seat's own state** (e.g. `pub mind: Option<mind::State>`) and init it in `Demo::new`.
- Dispatch points in `src/lab/demo/mod.rs`; add one arm per seat in each `match` and put the bodies in `src/lab/demo/<seat>.rs`:
  - `Lab::demo_before_tick` / `demo_after_tick`: every simulated tick (inside `Lab::tick`, so headless harnesses get them too).
  - `Lab::demo_frame`: once a displayed frame (body keys, camera follow). Harnesses must call it themselves.
  - `Lab::demo_mouse(cell, MouseAct::{Press{erase}, Drag, Release}) -> bool`: offered before the lab's tool; return true to take the press (its drag and release then come to you).
  - `demo::draw_world(lab, frame_buf)` + `demo::marks_world(lab) -> bool` (say true when you drew, or the dirty-rect skip smears).
  - `demo::draw_hud(lab, hc, frame_buf)`: after the clock, on the interface canvas (`hc.text`).
  - `demo_key`: Y U P J K R go to your seat (currently routed only for the queen: add your arm in `demo_key`). H says `help_line(seat)`.
- `DemoSeat::body()` decides which seats get a steered ant: Queen -> Queen role, Scout -> Worker role, others none. Binding picks `queen::pick_body` for the queen, else the oldest ant: scout, put your own pick in the match in `demo_bind_body`.
- **Scenario files: keep them in `assets/lab_scenarios/` as `demo_<seat>.ron`, title starting `DEMO: `.** They then show on the SCENARIO page (a second way in) and the shipped-scenario guard tests them. Decision by the launcher owner, 2026-10-25.


## Landed (2026-10-25)
#904 merged as 648d705 (launcher + steered mind ant; hand-off for SteerRole::Queen only, other mind bodies use #903's forced pick;
`seat_takes(seat, c)` for a seat to claim a body letter). PR #908 adds: `seed=N` reseeds the demo box; a demo opens without
the lab's welcome window; `Renderer::line_tint` for per-ant colours; examples/queendemo.rs as a template for a seat's headless
scripted player (drives demo_key/demo_frame, writes GIF + stills via Lab::draw).
