# Lane A — the held world's screen GUI, 2026-09-14

Coordinator: `session_01TngZpRY8LoqpFWHuXjUTTD`. Branch:
`claude/druid-screen-gui`, off `91567399` (PR #408's branch). Files owned:
`src/druid/hud.rs`, `src/druid/menu.rs`, `src/bin/druid.rs`, one line of
`src/lab/stats.rs`. Worked in the brief's own order: item 2's
proportionality first, item 3's bar, item 1's panel, item 2's deletions
last.

## What shipped

- **Item 2a/2c** — `druid::hud::mote_count_for_amount(amount)` replaces the
  flat `PER_DRAW = 26` with `6 + amount * 0.6`, rounded. Verified it reads
  (a headless absorb logged mote counts moving with the drawn amount)
  *before* touching the chevrons, per the brief's ordering. Then removed
  `Mark`/`marks()`/the chevron draw, `Readout`'s corner `CHARGE … NEAR YOU`
  row, and — the one licensed exception — `Druid::charged_animals` at
  `src/druid/mod.rs` (own commit, as asked).
- **Item 3** — a two-row button bar at the bottom of the screen, ported from
  `lab::ui`'s `Rect`/`Widget`/`Bar`/measured `layout` discipline. Thirteen
  actions have a button (label + dimmer key caption underneath); three
  continuous dials and the held movement keys stay keyboard-only.
  `Handler::act` in `src/bin/druid.rs` is the single dispatch point both the
  keyboard and the bar's clicks call, wrapping `druid::hud::Druid::act` for
  the pure game-state verbs. Mouse plumbing (`CursorMoved`/`MouseInput`/
  `CursorLeft`) added to `window_event`, mapped 1:1 via
  `pixels.window_pos_to_pixel` (no `to_logical` divide needed — see the
  brief, confirmed correct).
- **Item 1** — the biosphere page (`TAB`), `lab::stats::Stats` reused almost
  verbatim. Births/deaths/germinations needed no new plumbing
  (`World::creature_stats`, `World::germinations` already public).

## Where I could not do what the brief assumed, and why

**`Druid::act` is not the single dispatch point — `Handler::act` is, and
that is a structural finding, not a shortcut.** The brief's item 3 section
says to route both keys and clicks through `Druid::act`. I could not: the
button bar's own retained state (cursor, press-armed action, last frame's
laid-out `Bar`) and the biosphere page (`Stats`, with its own history and
census cache) both need to persist across frames, and neither can be a
field on `Druid` — `src/druid/mod.rs` is Lane B's file for the length of
this program and I have no licence to touch its struct beyond the one named
deletion. So that state lives on `Handler` in `src/bin/druid.rs` instead,
and `Handler::act` is the actual single place a control turns into a
change; `Druid::act` (a new `impl Druid` block added from `hud.rs` — legal
Rust, an inherent impl can live in any file in the same crate) is what it
calls for the verbs that need nothing from `Handler`. Two actions
(`ToggleOptions`, `FoundColony`) still need a line in `Handler::act` to
clear held movement keys before the modal steals the keyboard, and one
(`ToggleStats`) is handled entirely in `Handler::act` since `Druid` has no
`Stats` field to call into. This is exactly the shape the brief itself
predicted might be needed and it turned out to be needed.

**The stats page could not be threaded through `Interface`'s own repaint
comparison for the same reason** — `Interface::build` takes `&Druid`, and
adding a cursor parameter would mean editing its call site inside
`Druid::draw`, in `mod.rs`. So the bar and the stats page are painted as a
second pass from `Handler::frame`, after `Druid::draw` returns, and the
repaint decision below is built around that constraint rather than despite
it.

**One claim in the brief that turned out wrong, worth flagging plainly**:
the `stats.rs` note said *"the only lab-bound line in the whole panel is
`stats.rs:710`"* (the `bar_top()` clamp). That is true of the *drawing*
code, but `draw_at`'s own on-screen caption is literally the string
`"TAB CLOSE"` — a second, silent assumption that the caller's close key is
`TAB`. I did not have to fix it: I bound the biosphere page's own toggle to
`TAB` in the druid too (moving seed-kind cycling to `K` instead), so the
caption is simply true here as well, and it means a player who has touched
the lab gets the same reflex in this game. Worth recording because a
different key choice would have shipped a caption that lies, and nothing
would have caught it — there is no guard over that string's honesty, only
over its width and glyph coverage.

## The repaint decision, with the number

Read the brief's framing carefully: it worried about the *bar's* hover
needing quantisation the way the lab's does. In practice the bar itself
needed none of that — its outer plate is a fixed rectangle, unconditionally
fully repainted every drawn frame (`hud::draw_bar` fills the whole strip
before painting a single widget), so nothing under it can ever go stale
regardless of hover. Measured: **0.055 ms/frame**
(`PIXEL_PHYSICS_DRUID_BENCH_BAR=1`). Drawing it every frame costs nothing
worth avoiding, and it needed no participation in `Interface`'s `PartialEq`
at all.

**The actual hard case was the biosphere page, and it is a real bug I found
by reasoning about the design rather than by a test catching it.** Unlike
the bar, `Stats::rect`'s bottom edge tracks its own content — more
generations, a species starting or stopping to stand — so a naive
"repaint it every frame, unconditionally, like the bar" would leave stale
page pixels below a *shrinking* bottom edge on any frame the world's own
repaint was skipped underneath it. That is exactly the smear
`hud::Interface`'s `last_ui`/`ui_changed` comparison exists to prevent,
just aimed at a piece of screen that comparison does not reach. Measured
the actual cost that made "just always force a full repaint" unappealing:
the page's own paint is **1.11 ms/frame** (`PIXEL_PHYSICS_DRUID_BENCH_BAR=1`,
same run, same seed) against the bar's 0.055 ms — nearly twenty times as
much, and on top of whatever the world repaint itself costs.

So `Handler::frame` computes `(Stats::rect(world, floor), cursor)` before
drawing, compares it against last frame's, and passes that comparison as
`Druid::draw`'s existing `force_full` parameter — which was already there,
already public, and I did not have to touch `mod.rs` to use it with a
caller-supplied value instead of the hardcoded `false` `Handler` used
before. The cursor rides along in the same comparison for a second reason:
`Stats::draw_at_floor` also draws a hover-note popup wherever the cursor
sits over an explainable row, and that popup's position is not a function
of `Stats::rect` at all — it moves and vanishes with the cursor alone,
which is the same smear risk from a different source. I did not find an
exposed way to get the note's own rectangle without adding more surface to
`stats.rs`, so the coarse fix (repaint whenever the cursor changes at all
while the page is open) is what shipped; it costs more than necessary when
the player rests the mouse without moving it over an open page, but it is
correct, and correct was the bar I was measuring myself against here.

Net effect: while the biosphere page is closed, this game's original
dirty-rect premise is completely untouched — the bar's own paint is the
only added per-frame cost, at 0.055 ms. While the page is open, a full
world repaint fires only on the (roughly twice-a-second-at-1x) frames its
own content or the cursor actually changed, not on all sixty.

## What I did not build, and why

- **No icons, no hover-note popup, no pages on the bar.** The lab's
  `Widget` carries `icon`/`ratio`/`note`; none of the thirteen druid
  buttons needed a fill-strip readout or an icon, and a tooltip system
  beyond the caption itself is not what the owner asked for ("buttons …
  with subtle hotkey always visible" — the caption *is* the hotkey).
- **Three continuous dials stay keyboard-only** (place radius, carried
  circle reach, speed) and so do the four held movement keys — see
  `bar_specs`'s own doc for the reasoning, which is the lab's own
  `STOCK_LADDER` argument.
- **A true multi-setting A/B for the mote-count formula wasn't run as a
  sweep of many values** — I picked one steeper alternative
  (`floor 2, +1.0/energy`) against the shipped one (`floor 6, +0.6/energy`)
  and posted a blind A/B at a small pull (2.25 energy/animal, where the
  floor dominates and the two formulas diverge most): card
  `20260914T034825842Z-e892f1`, board `held-world`. Fire-and-forget per the
  house rule; verdict not yet in as of this note.

## Gates

`cargo clippy --all-targets --release --locked -- -D warnings`: clean.
`cargo test --lib --release`: 1,752 passed, 0 failed, 85 ignored (no
regressions; this branch adds tests, does not remove any). `cargo test`
(full, reaches `tests/*.rs`): run because `src/lab/stats.rs` is shared with
the lab and `examples/labstats.rs`; see the PR body for the result.
`bash scripts/docscheck.sh`: clean (pre-existing lane-note size warnings,
unrelated to this branch). `python3 scripts/deadendindex.py --touching`:
0 hits (a new UI surface, not a retried mechanism).

## Head

See `PR_BODY_LANE_A.md` on this branch and the PR itself (opened via the
GitHub MCP tools, since `mcp__github__get_me` is available in this
session) for the head SHA and the PR link.
