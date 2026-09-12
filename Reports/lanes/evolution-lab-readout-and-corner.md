# Lane note — the box stops talking over the bed (round 30, R1)

*Two of the owner's own sentences, 2026-09-12: the cell readout should only
show up while LOOK is armed; the top-left corner should carry ticks and
nothing else. Both are view-only -- `src/lab/ui.rs` and `src/lab/time.rs`,
nothing in `src/sim/**`.*

## Task 1 — the hover readout follows LOOK

`paint_hover_cell` (the docked material/temp/moisture/organism box,
`ui.rs`) drew unconditionally from 2026-08-30. Gated on `self.tool ==
Tool::Look`. The **pinned** cell page (`self.inspect`, the click-to-open
CELL panel) is a separate mechanism and is untouched -- it stays open across
a tool switch because the player asked for it by clicking, where the hover
box was never asked for at all under another tool.

Guard: `the_hover_readout_only_shows_under_the_look_tool` (`ui.rs`) draws a
real `Lab` frame under LOOK and under six other tools at the same cursor
cell and reads one pixel of the readout's fixed dock position. Verified
sensitive by removing the gate and watching it fail (`Plant is armed and
the hover readout still drew`) before restoring the fix.

Also fixed: `Tool::Look`'s own help text claimed the readout was "ALWAYS
READ OUT TOP RIGHT, TOOL OR NO TOOL" — false on two counts even before this
change (it had moved to the left column months earlier). Now states the
LOOK-only behaviour and that a pinned page survives a tool switch.

## Task 2 — the corner cut to ticks

`TimeControl::readout()` ran up to six lines while running (RUNNING/ASKED,
GOT/AT-HZ, SIM-per-second, N-ticks-per-frame, the MOTION/FAST-FORWARD
crossover, FRAME) and four while paused. Cut to two: `FRAME {n}` (ticks)
and `MIN {}HZ`.

**What justified removing each line, not just that it looked busy:**
PAUSED/RUNNING and the ASK/GOT rate are already the bar's own speed readout
at the bottom of the screen, and both are also in the window title bar
(`Evolution Lab — paused/running X.Xx — N fps`) — removing them loses
nothing, only the duplicate. SIM-per-second and the ticks-per-frame/
MOTION-vs-FAST-FORWARD line are commentary derived from those same two
numbers, not a control with its own readout. Deleted `sim_per_second` and
its dedicated test along with the line that was its only caller — a
private fn with no other call site is dead code, not a spare part.

**One line kept, and it is a decision rather than an oversight:** `MIN
{}HZ`, the display floor `F` (`cycle_display_floor`) cycles. No other
readout of that value exists anywhere in the interface — not the bar, not
the HELP page, not the window title. Deleting it would strand the only way
to see what `F` currently has set, which is exactly the regression this
round's brief warned against. Flagged to the coordinator and here for lane
R2: the right long-term home for it is the master menu that round is
building, not this corner, but leaving it one line here until that page
exists is not the same failure as silently deleting it.

## What a later lane should not have to re-derive

**The top-left corner and the bar's speed readout say the same thing in two
places**, which is *why* four of the six removed lines were safe to cut —
check the bar and the window title before assuming a top-left line is the
only place a number lives. It usually is not. `MIN HZ` is the one number in
this readout that actually was sole-route, and the only way to know that
was to grep the rest of the interface for it, not to guess from how the
line reads.

## Verification

- `cargo test --lib`: 1,632 passed, 70 ignored, 0 failed, on the merged
  tree (this branch's one new test, minus the one deleted alongside
  `sim_per_second`).
- `cargo clippy --all-targets --release --locked -- -D warnings`: clean.
- `cargo run --release --example ascii`: 31 scenes, 0 skipped — unmoved.
- `bash scripts/acceptance.sh`, `scripts/worldgencheck.sh`,
  `scripts/docscheck.sh`: all clean.
- Review card `20260912T093446297Z-c001b8`, board `lab`: before/after of
  the corner and LOOK-armed/PLANT-armed of the hover readout, same bed and
  cursor cell (300,150) in both pairs.
