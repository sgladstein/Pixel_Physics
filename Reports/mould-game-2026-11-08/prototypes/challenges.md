# Mould: the first three challenge dishes + dish picker (build for real)

Read `common.md` first (its rules apply). Then `../near-term-challenges.md` sections: "What the model allows", dishes 1 (Behind the Wall), 3 (Beat the Ruler), 6 (Running Dry), "The shared code", "The dish select screen". That write-up is the design; follow it unless measurement says otherwise, and say where you departed.

**This is not a prototype: it ships to main** (via the thread owner). Owner's standing rule: features default on; the lesson and the free seeded dishes must keep working.

What the base already has (your worktree is at the owner's branch head): the lesson (`src/mould/lesson.rs`, seed 0, the default), briefing card (`View.brief`, H), and tonight's changes: per-piece food (`Params::pieces`, on: each piece of mould eats only the food it's joined to; `Walker::food`), open wounds from snips (`Params::wound` 4 s, `Ground::Wound`), joins flashing (`Dish::snaps`), a pulse along fed tube (render only). Read `src/mould/mod.rs`'s module doc and these before designing.

Build, in order:
1. `src/mould/level.rs`: the `Level` struct (trimmed to what these three dishes need; leave room for the rest) and `Dish::from_level`. Hand-placed piles, pre-laid salt (via the existing stroke dab or direct), per-level tool stocks, per-pile food, level-timed arrivals (the existing `pending`), win rule `JoinAll{by}` / `UnderRuler{bars}` / `AliveAtBell{bars}` scored in `phase_tick` (or a level hook). Storms: keep the existing one storm unless a dish says otherwise; your call per dish, say why.
2. The three dishes. **Ship bar per dish, measured with `mouldprobe level=N`: do nothing 0 stars, random at most 1, an intended scripted hand 3** (write the intended hand per dish in `src/mould/play.rs` or the probe). Set every bar from the run with headroom; print the table. Run 5+ seeds where anything is random (players' rng), the dish layout itself is fixed.
3. The after-dish "why" line on the score card (one line, world words, from the failed condition).
4. Dish picker: M opens it; the lesson, the three dishes, and FREE DISH (today's seeded dishes, N cycles them). Unlock: next dish opens on 1+ star of the one before. Save best stars to `mould-progress.txt` beside the executable (missing file = fresh; never panic on a bad file). Keep it to the existing `panel`/`text` helpers in `render.rs`: an interface redesign is being done separately tonight in another worktree (`src/mould/look.rs` likely), so keep your drawing additions small and self-contained (a function or two), not restyling.
5. Briefing card per dish: its goal sentence and that dish's tools.

Measure and show: per dish, a still of the start, a still of the intended hand's win, the probe table (stars per hand per seed), and a GIF of the intended hand if cheap. Deliver to `prototypes/challenges/` with `result.md`. Quote frame mean/worst on one dish.

Tests: add a test per dish that the do-nothing hand scores 0 (fast enough for CI: if a full dish is slow in debug, mark the heavy one `#[ignore]` with a note and keep a cheap structural test). `cargo test --release --lib mould`, clippy -D warnings on lib + `--bin mould` + `--example mouldprobe`, `bash scripts/docscheck.sh` clean. Update `Reports/mould-probe-2026-11-07.md` with a short section (what the dishes are, the ship-bar table), and the `Reports/instruments.md` mouldprobe row for `level=`.

Branch `challenges` in your worktree; commit; don't push. Final reply: lead with whether each dish meets the ship bar (the table), then branch/SHA/worktree path, files.
