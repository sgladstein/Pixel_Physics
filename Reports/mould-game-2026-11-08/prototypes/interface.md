# Mould: interface and graphics, round 1 (design looks, then build the picked one)

Read `common.md` first (its rules apply, except where this says otherwise).

**Why.** Scott (owner) on his first play: "The game needs better instruction of what you're supposed to do. What your actual options are. The interface could use a lot of work." He then asked for a dedicated high-effort pass on "the interface and graphics". The lesson + briefing card (PR #1052) fixed *instruction*; this is about *how it looks and reads*. The 8-demo playtest's top finding across all games: **preview what an action will do before you commit it** (see `/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/` playtest material if useful; the summary page is https://claude.ai/artifact/E8re9eqAAKbggnRJM8oEK9 — you may not be able to open it; don't block on it).

**Look first.** Build and run `cargo run --release --example mouldprobe -- lesson=DIR` and `look=DIR seeds=1` (read the probe's header for flags) and look at the stills before designing. Read `src/mould/render.rs` whole (it is the whole drawing + HUD: 5x7-ish bitmap font, a bottom strip of text lines, briefing card, lesson banner) and `src/bin/mould.rs` (window: 1024x726 at SCALE 4? check `render::frame_size`). Read `../toy-feel.md` (what reads at phone size, what doesn't: e.g. snip mark 1 px, lamp rings 1 px thin, all tubes same width) and `../theme.md` top section (the mould is a pet you keep; colour already *is* its health: yellow fed, grey starving — don't hide that behind meters).

**What to design** (each look must cover all of these):
1. **Tool bar**: four tools (lamp, oats, salt, snip) with an icon or glyph each, the key number, how many are left (oats 5 + regrow 1/15 s, salt 1, snips 3), and which is selected. Replaces the text line `[1 LAMP] 2 OATS 1 3 SALT 1 4 SNIP 2`.
2. **Status**: time/storm countdown, food joined x/y, mould size; make the storm countdown impossible to miss in its last 5 s.
3. **Cursor previews** (the testers' top ask): the lamp's reach ring, where oats will land and roughly what they'll pull, the salt stroke's remaining length while dragging, the snip line. Cheap and honest: preview only what the code knows.
4. **Briefing card, lesson banner, score card**: restyle to match. The score card should lead with an amount (Scott liked "arrived with N of M"-style amounts over stars; stars can stay beside it).
5. **The dish itself**: agar, food piles (fed ring), oats, salt, storm warning line and scar, tube look (consider width/brightness by how much it carries; `fed` in `mod.rs` is the BFS distance from food), the lamp's light. Keep fed=yellow / starving=grey legible — that is the game's best feedback.
6. Font: the current bitmap font is readable but crude; you may draw a better bitmap font or a 2x one. No external font files unless trivially embedded.

**Deliver tonight: 2 or 3 distinct looks, rendered by real code**, not painted mockups: e.g. a `Style` enum in a NEW file `src/mould/look.rs` (or `render_b.rs`), selected by `mouldprobe ... style=A|B|C` and by a key in the window (e.g. F2 cycles; name the active style on screen only in a debug corner). **Do not restructure `src/mould/render.rs` tonight**: other builds are editing it (a pulse along tubes, a snap flash, open wounds, per-piece starving are going into render.rs/mod.rs on the owner's branch). Add a minimal hook in render.rs (one call into your module) rather than rewriting it. Your looks will be rebased onto those changes later.

For each look, same scripted run, same frames: (a) lesson step 2 (oats), (b) mid-dish growing with lamp held, (c) storm warning, (d) score card. Make one contact sheet per look plus a per-frame set; and a short GIF of one look in motion if cheap. Put them in `/mnt/project-files/game-design/mould-game-brainstorm-2026-11-08/prototypes/interface/`. Name the files by a **shuffled letter** (A/B/C); keep the key (which letter = which idea) in `interface/key.md`, NOT in result.md, because the owner judges them blind. In result.md describe each letter's *feel* in one line without saying which you prefer, then a separate "designer's recommendation" section at the end.

Frame cost: quote mean/worst frame ms for each look vs current (mouldprobe or a small timing loop around `render::draw`). A look that costs more than ~2 ms extra worst-frame must say so.

Branch: `interface-proto` in your worktree. Commit; don't push; no PR. clippy -D warnings on what you touched; `cargo test --release --lib mould` green.

**Stop after the looks** (do not build the final one): the owner picks first. Your final report: lead with what the three looks are (by letter, one line each), frame cost, then branch/SHA/worktree path, then files.
