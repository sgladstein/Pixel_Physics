# Build the lone scout seat demo: lane hand-off (2026-10-27)

**State.** Done and on main: PR #912, merged as 700531a on 2026-10-25. It added:
- `src/sim/tandem.rs` (tandem runs)
- `src/lab/demo/scout.rs` (the seat)
- `assets/lab_scenarios/demo_scout.ron` (the box)
- `examples/scoutdemo.rs` (a scripted player, a GIF maker and the frame-cost control)
- the write-up in `Reports/teaching-the-colony-2026-10-25/scout-demo/`, with `results.md` as the summary.

Run it with `cargo run --release --bin lab -- demo=scout`.

**PR #946 merged 2026-10-27 as 6a58114** (docs only). It landed the step-0 census probe source and the first-look page, both of which lived only outside the repo.

**Owner decision pending.** The question, asked on 2026-10-25 on the first-look page and in the thread: "When she stops, does it read as her checking the way, or as the game stalling?" Pauses are 1 in 50 of her steps, each lasting 60-90 frames, after a 12-frame tell. My recommendation is to keep them while she is learning landmarks but halve the clock rate (1 in 100) if the owner says stalling. The knob is `tandem::Knobs::pause_chance`.

**Open to-dos.** None are in progress. Possible next steps, from `results.md` "Open":
- The cascade is small: 0-2 runs a game at c = 0.25 a trip.
- The recall pull is scripted at gain 6. The keeper seat's plan for this is a heritable "how much to listen" rider, so that selection tunes it.
- Arm B (they follow you) leads 2-3x as many sisters and wins every raw count. Which design feels better is the owner's call, and still open.

**Keeper's later changes to recall (after #912).** `prune` keeps sleeping lessons, and `SAME_PLACE` went from 6 to 12. They make scout's per-trip lesson renewal partly redundant but harmless. Nobody has re-run scout's 4-seed sweep since.

**Files outside the repo.** All are covered by #946, except the two first-look GIFs. Those are not kept (4.4 MB each) and regenerate from the command in `results.md`. The scratchpad sweep logs in `target/scout/` are reproducible and not worth keeping.

**Jobs running.** None on the Google Cloud VM or anywhere else.

**Branches.** None with unmerged work (`claude/project-thread-x1ms83` is fully merged).

**Working-practice lessons.** These are in `results.md` "Found on the way" and the README "Lone scout status" section:
- Measure tandem contact to the leader's body, not her head.
- A steered worker must go through `mind::tick`.
- A timeline heap drop buries any ant standing where it lands.
- Set `Lab::show_help = false` before headless captures.
- "Did not step this tick" is not the same as "blocked".
