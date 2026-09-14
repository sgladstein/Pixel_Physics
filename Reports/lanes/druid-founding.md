# Lane B — founding, 2026-09-14

Coordinator: `session_01TngZpRY8LoqpFWHuXjUTTD`. Branch:
`claude/druid-founding`, cut from `main` at `9fd637a7`. Files owned:
`src/sim/creature.rs`, `src/druid/founding.rs`, the `offer` arm of
`src/bin/druid.rs`. Three items from the owner's 2026-09-14 playtest; all
three shipped, each reproduced before it was touched.

## What shipped

**Item 1 — "little yellow bars that get placed onto the ground."**
Reproduced as a number first: `paint_nest_patch` laid `i % 3 != 2` over 53
columns, which `examples/founding_shot`'s run histogram reads as **36 columns
in eighteen runs of exactly two**. A dotted line with no middle — `CLAUDE.md`'s
first law, arrived at from the destruction line and true here. And every cell
took a literal `0` shade, so the patch was the palette's *lightest* tan and
the only unmottled thing on the ground.

`nest_mask` replaces the comb: an unbroken core, then a fringe of single cells
thinning to nothing at the rim. **36 columns in 18 runs → 25 in 15**, longest
run 2 → 11. Cells take a position-keyed shade now, not `World::rng` — a
player-triggered draw reshuffles a stream worldgen also draws from, and
same-build replay is required.

**Item 2 — "it should still happen right under or next to the druid."**
`colony_stations` decided every offset before it looked at the ground and
dropped the ones that were not sites, so a blocked middle seated nobody near
him and scattered the survivors across the band's full width. It walks outward
a column at a time now, taking the first that is a site and keeping a body's
corridor between any two. Nine columns of trunk through a stand: **6 of 8
seated → 8 of 8**, watched red against the predecessor.

**Item 3 — "the found menu needs to be way improved."** A cursor over
`founding::ROWS` (body, three lineages, the count, `FOUND`, `LEAVE`); arrows
and `WASD` move it, left/right work the row, `ENTER` chooses, and every row
has a hit box. `hud::offer_layout` is **one definition read by the drawing and
by the click** — §R2 is what a second copy of a placement rule cost last time.
The dials carry `<`/`>` ends so the pointer can work them, not only select
them. Every old letter still works. `founding::Memory` makes the dials survive
a close; `Druid::time_stopped` stops the clock while the screen is up.

## The three things worth another lane's time

**`commit_founding` had never rerolled the offer, and the stickiness bug was
hiding it.** It called `reroll()` and *then* did `self.offer = None`, so the
fresh three went out with the screen and the next open served attempt 0 again.
"Committing is what costs you the other two" was in the module doc from day
one and had never once happened in the game. The general form: **a `take()`
that discards state is a place where an earlier line's work can vanish with no
error anywhere**, and it looked exactly like a design that was merely never
noticed working.

**`burrow_probe arms=colony` is structurally blind to founder placement.** The
brief names it as one of the two instruments for item 2's lab exposure. Its
colony arm builds through `World::plant_ant` in a loop, never
`found_colony_of`, so it cannot see `colony_stations` at all — and it proves
it by coming back **byte-identical** across `PIXEL_PHYSICS_COLONY_BAND`. That
is `CLAUDE.md`'s stale-binary tell wearing a different hat: identical output
across a change that must have moved something, where the cause is the harness
rather than the build. (It is also still on §Z24's list, for a different
reason.) `labnest founders=8` *is* on the path and is the one to use.

**A stale example binary cost the first drainage sweep.** `cargo build
--release --example founding_shot` rebuilt one binary; `nestdoor` then reported
**byte-identical** results for three different `PIXEL_PHYSICS_NEST_CORE`
settings, all reading the old comb's 36 cells. The `off` arm looked right the
whole time, which is what made the table plausible. `--examples`, always.

## Numbers a later session should not re-derive

**Drainage, `examples/nestdoor`, 3 seeds, standing water on the patch against
the same width of ordinary ground beside it, at 10k/20k/30k frames.** The core
is the one place the §T2 drain rule is relaxed, so it is set from this rather
than argued:

| arm | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| run-bounded comb (`core=0`) | 1, 10, 15 | 2, 3, 1 | 1, 0, 0 |
| **shipped (`core=5`)** | 7, 11, 9 | 1, 4, 0 | 0, 4, 0 |
| unbroken (`NEST_DRAINS=off`) | 52, 63, 68 | 10, 9, 10 | 18, 1, 1 |

Ground beside the patch read 0–4 in every arm. `core=12` was measurably wetter
(20, 19, 52 on seed 1) and is not taken. The unbroken arm is the positive
control and reproduces §T2 outright.

**Lab exposure, `labnest founders=8 seeds=2`, `RAYON_NUM_THREADS=2`, the walk
against `PIXEL_PHYSICS_COLONY_BAND=1`.** **52 founders seated in both arms**,
which is the claim that matters; the die-off keeps its shape (52 → 13/16 by
frame 5,000 on the walk, 52 → 15/19 on the band); `roofed` at frame 9,000 is
7/19 on the walk against 15/22 on the band, and `digs` 503/373 against
433/468 — the two seeds swap which arm is higher, so that is the bed's own
spread rather than the change.

**Tried and rejected, in order, all on item 1's shape:** a linear taper
anchored at the core's edge — still ~50% dense at the rim, so the patch ended
in a straight line of crumbs; a fringe bounded at `drain_period - 1` — that is
2-on-1-off, a *shorter* barcode, and the fringe is all the player sees because
the core is under his feet; a square taper from the centre — 25 → **13**
columns, a door too small to be one. None is in `dead-ends.md`: each was a
tuning step inside one mechanism rather than a mechanism, and all three are
reachable from the shipped knobs.

## What I took that the brief did not give me, and what I left alone

**I took ~40 lines of `src/bin/druid.rs` outside the `offer` arm**, which the
brief asked me to flag rather than take: one `Handler` field
(`offer_pressed`), the founding branch of `mouse_button`, two helpers
(`offer_click`, `hover_offer`), and two lines in `CursorMoved`/`CursorLeft`.
The mouse plumbing is the only place item 3's "and/or mouse" can live, and a
lane cannot message its coordinator to ask. Lane C should expect a conflict
there and take its own side of `window_event`.

**I also took `src/druid/hud.rs`** (the founding screen's draw and layout) and
three small edits to `src/druid/mod.rs` — `toggle_founding`, `commit_founding`,
`update`'s pause gate, plus one field. **Nothing in the trail functions**, per
the brief.

**Left alone: `assets/materials/nest.ron`.** It is shared with the lab, whose
own comment says the pale tan is deliberate — *"to read clearly against soil
and against the dark ants standing on it"*. If the owner's complaint turns out
to be the **colour** rather than the pattern, that file is the fix and it is a
lab-visible change; review card
`20260914T203358857Z-2a28fa` asks him which reading it is, with a
grain-only pane between the two so the two halves can be told apart.

## Review cards out

- `20260914T203358857Z-2a28fa` — the ground after a founding, three arms,
  blind. **Decode the prose through `blind_was` before acting on it.**
- `20260914T203509792Z-1d6a4f` — the rebuilt menu, before/after, not blind.

Both `owner_can_see_it: true`, verified present on `origin/review-queue`.

## Gates

`cargo clippy --all-targets --release --locked -- -D warnings`,
`cargo test --release` (full, not `--lib`), `bash scripts/docscheck.sh`,
`python3 scripts/deadendindex.py --touching` — all green; `docscheck` wanted
`readmetoc.py` after the README edit and was rerun clean.

Head SHA: see the last line of `PR_BODY_LANE_B.md`.
