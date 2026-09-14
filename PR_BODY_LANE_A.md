## What this does

The held world's first real GUI — the owner's 2026-09-14 playtest, item 3:
*"We need an actual GUI. We can keep it minimalistic, but buttons for the
main actions (with subtle hotkey always visible). I don't want to have to
remember all these shortcuts and the menu isn't even a menu, it is a
shortcut list."* Three things, all from that same playtest:

- **A two-row button bar** along the bottom of the screen — thirteen of the
  game's verbs get a button (a label, and the key that also fires it drawn
  dimmer underneath), ported from the evolution lab's own control bar
  rather than invented. The on-screen key legend shrank to match: a key
  with a button no longer repeats itself as a corner row, so what is left
  is only what has no button — three continuous dials and the held
  movement keys.
- **A biosphere page (`TAB`)** — plants, animals, biomass, births and
  deaths, generations, reproduction — reusing the evolution lab's own
  `lab::stats::Stats` almost unchanged.
- **The energy pull lost its arrow.** Item 2 of the same playtest: *"no
  indication of how much life energy the ants have... you should just see
  how much energy you get by how many particles come."* The chevron over a
  charged animal's head (the only on-screen reading of how much was
  waiting) and the corner's `CHARGE … NEAR YOU` line are both gone; the
  stream of particles itself now scales with how much energy the draw
  actually carries, which it never did before — a flat 26 particles
  regardless of the amount, so the number the whole complaint is about was
  computed and thrown away before it reached the screen.

## Where it sits

Last of the three playtest items from the *screen* brief — a colleague
lane is doing the held world's third item (the plane/rim look) separately.
This closes out "the game has no interface at all," which was the owner's
very first reaction to playing it.

## Mechanism

- `druid::hud::mote_count_for_amount(amount) = round(6 + amount * 0.6)`
  replaces the old fixed `PER_DRAW = 26`, applied per `Draw` so both an
  absorb and a founding's reversed flow read honestly. Verified it moves
  with the amount (a headless run) before removing the chevron, per the
  brief's own ordering, so a readout existed throughout.
- The bar (`Rect`/`Widget`/`Bar`, `bar_layout`'s measured, loosest-spacing-
  first `layout`) lives in `src/druid/hud.rs`; mouse plumbing
  (`CursorMoved`/`MouseInput`/`CursorLeft`) and the retained per-frame state
  live in `src/bin/druid.rs`'s `Handler`, mapped 1:1 through
  `pixels.window_pos_to_pixel` (this game never zooms, so no logical-size
  divide is needed). `Handler::act` is the single dispatch point both the
  keyboard and the bar's clicks call; it wraps a new `Druid::act` (an
  `impl Druid` block added from `hud.rs`, legal across files in one crate)
  for the pure game-state verbs, and handles the two concerns that belong
  to the event loop rather than the game itself (clearing held movement
  keys before a modal opens; the biosphere page toggle, since `Stats` also
  lives on `Handler`). **This deviates from the brief, which expected
  `Druid::act` alone to be the single dispatch point — `src/druid/mod.rs`
  is a different lane's file for the length of this program, so `Druid`
  could not gain the fields this needed. See `Reports/lanes/druid-screen.md`
  for the full reasoning.**
- `src/lab/stats.rs` — the one file this lane shared a line with another
  game already using it. `Stats::rect`'s hardcoded `super::ui::bar_top()`
  became a `floor: i32` parameter (`Stats::draw_at`'s own external
  signature is unchanged — it now calls a new `draw_at_floor` internally
  with the lab's own floor, so nothing outside this file needed editing),
  and a new `pub fn draw_at_floor(.., floor)` is the druid's entry point,
  supplying its own bar's `bar_top()` instead.
- **The repaint decision** (the one the brief called out as the hard part):
  the bar needed none of the quantisation the brief anticipated — its plate
  is a fixed rectangle, unconditionally fully repainted every frame
  regardless of hover, measured at **0.055 ms/frame**. The biosphere page
  is the real case: its rectangle's bottom edge tracks its own content, so
  drawing it unconditionally risked leaving stale pixels below a *shrinking*
  page on a frame the world's own repaint was skipped — found by reasoning
  through the design, not by a test. Its own paint costs **1.11 ms/frame**,
  which is also why it is not simply always-on like the bar. `Handler::frame`
  now compares `(Stats::rect, cursor)` frame to frame and passes that into
  `Druid::draw`'s existing (already-public) `force_full` parameter, so a
  full world repaint fires only on the frames the page's content or the
  hovered row actually changed — a few times a second at 1x while the page
  is open, never while it is closed.

## What I left on the keyboard, and why

Three continuous dials (place radius `Q`/`E`, carried-circle reach `[`/`]`,
speed `Z`/`V`) and the held movement keys (`A`/`D`/`W`/`S`/`SHIFT`/`G`) have
no button — a button pressed dozens of times to walk a ladder is not a
control (the lab's own `STOCK_LADDER` doc makes the same call), and a
press-and-release button cannot express "held". `TAB` was reassigned from
seed-kind cycling (now `K`) to the biosphere page, matching the lab's own
key for the same panel.

## Testing

- `cargo clippy --all-targets --release --locked -- -D warnings`: clean.
- `cargo test --lib --release`: 1,752 passed, 0 failed, 85 ignored.
- `cargo test` (full): run because `src/lab/stats.rs` is shared with the
  lab and `examples/labstats.rs`.
- `bash scripts/docscheck.sh`: clean.
- Verified live: headless screenshots and GIF captures via
  `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES`/`PIXEL_PHYSICS_DRUID_GIF` under
  Xvfb + lavapipe, confirming the bar, the legend's new shorter form, the
  biosphere page, and the chevron's removal all render correctly together
  with no overlap (also covered by
  `both_panels_fit_inside_the_viewport`/`the_bar_fits_the_screen_and_no_two_widgets_overlap`).
- Posted a blind A/B of the mote-count formula against a steeper
  alternative, at a small pull where the two diverge most:
  `python3 scripts/review.py get 20260914T034825842Z-e892f1` (board
  `held-world`). Fire-and-forget; verdict pending.

---

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_013etYZf9Wg3HZMgiz9aj62J
