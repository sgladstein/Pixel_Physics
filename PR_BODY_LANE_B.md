## Founding: the menu, where the colony lands, and what it leaves on the ground

Lane B of the held-world round. The three founding complaints from the owner's
2026-09-14 playtest, each reproduced before it was touched.

### 1. *"when I found ants there are little yellow bars that get placed onto the ground. I don't like this."*

**Reproduced as a number, not just a picture.** The nest patch — the worked
ground a colony walks home to — was laid as `i % 3 != 2` over 53 columns:
`examples/founding_shot`'s run histogram reads **36 columns in eighteen runs of
exactly two**. That is a barcode, and a barcode is what the complaint
describes. It also had no middle at all, which is `CLAUDE.md`'s first law —
*an outcome is a distribution, not a binary* — failing on a patch of ground.
And every cell took a literal `0` shade, so all 36 drew `nest.ron`'s
**lightest** colour while every other material in the world draws a random
byte "so bulk material has visible grain".

`nest_mask` replaces the comb with an unbroken core at the gnome's feet and a
fringe of single cells thinning to nothing at the rim. **36 columns in 18 runs
→ 25 in 15**; longest run **2 → 11**. Cells now take a position-keyed shade —
deliberately not `World::rng`, because a player-triggered draw reshuffles a
stream worldgen and every pass also draw from, and same-build replay is
required (`PLAN.md`).

**The drain rule it must not break is held by construction.**
`open-bugs-handoff.md` §T2: `nest` is an impermeable `Solid`, so an unbroken
patch stands under a film and the colony loses its front door. The property
that repair needs is that every nest column is within one of ground that
drinks; outside the core no run may exceed `DRAIN_PERIOD - 1`, and the fringe
clears that with a column to spare. The core is the single relaxation, so it
comes off `examples/nestdoor`'s own water pair rather than an argument —
three seeds, standing water on the patch against the same width of ordinary
ground beside it, at 10k/20k/30k frames:

| arm | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| run-bounded comb (`NEST_CORE=0`) | 1, 10, 15 | 2, 3, 1 | 1, 0, 0 |
| **shipped (`NEST_CORE=5`)** | 7, 11, 9 | 1, 4, 0 | 0, 4, 0 |
| unbroken (`NEST_DRAINS=off`) | 52, 63, 68 | 10, 9, 10 | 18, 1, 1 |

Ground beside the patch read 0–4 in every arm. `core=12` was measurably wetter
(20, 19, 52 on seed 1) and is not taken. The unbroken arm is the **positive
control** and reproduces §T2 outright, so the instrument is known to
discriminate rather than merely to pass.

**Three shapes were built and rejected first**, all reachable from the shipped
knobs: a linear taper anchored at the core's edge (~50% dense at the rim, so
the patch ended in a straight line of crumbs); a fringe bounded at
`drain_period - 1` (that is 2-on-1-off — a *shorter* barcode, and the fringe is
all the player sees, because the core is under his feet); a square taper from
the centre (25 → **13** columns, a door too small to be one).

**This one is judge-by-eye and is with the owner**, blind, three arms
(shipped / grain only / both) so the two halves of the change can be told
apart: card `20260914T203358857Z-2a28fa`. It also asks the question I could not
settle myself — whether the complaint is the *pattern* or the pale *colour*.
If it is the colour, the fix is `assets/materials/nest.ron`, which is shared
with the lab and whose own comment says the pale tan is deliberate, so I have
not touched it.

### 2. *"it should still happen right under or next to the druid."*

`colony_stations` decided every offset before it looked at the ground — a band
centred on the cursor with a station every `spacing` columns — and **dropped**
the ones that were not sites. So a stand whose middle is blocked seated nobody
near him and scattered the survivors over the band's full width.

It walks outward from the cursor a column at a time now, taking the first that
is a site and keeping a body's corridor between any two already claimed. The
nearest ground is claimed first, and distance is only ever paid where the near
ground refused. Nine columns of trunk through a stand: **6 of 8 seated → 8 of
8**, and the guard was watched red against the predecessor.

Order stays left to right — `founder_reserve` staggers the cohort's starting
energy by index, so reordering would move the lab for no reason anybody asked
for. No two entries share a column, so `sort_unstable` has no tie to break.

**`src/sim/creature.rs` is shared with the evolution lab**, so this was
measured there too, one binary, `PIXEL_PHYSICS_COLONY_BAND=1` as the paired
arm, `RAYON_NUM_THREADS=2` pinned because a counter downstream of the
checkerboard is not load-independent. `labnest founders=8 seeds=2`: **52
founders seated in both arms**; the die-off keeps its shape (52 → 13/16 by
frame 5,000 on the walk, 52 → 15/19 on the band); `roofed` at 9,000 is 7/19
against 15/22 and `digs` 503/373 against 433/468, with the two seeds swapping
which arm is higher — the bed's own spread, not the change. It is close to a
no-op on flat ground by construction, which is what keeps it safe there.

**`burrow_probe arms=colony` cannot see this change and should not be quoted
about it**: its colony arm builds through `World::plant_ant` in a loop, never
`found_colony_of`, and it proves it by coming back byte-identical across the
switch.

### 3. *"the found menu needs to be way improved."*

Three clauses; the middle one turned out to be two bugs.

- **Fully controlled by arrows, WASD or mouse.** A cursor over
  `founding::ROWS` — body, the three lineages, the founder count, and
  `FOUND`/`LEAVE` as rows of their own so the arrows have somewhere to arrive.
  Up/down walk it, left/right work whatever it is on, `ENTER` chooses. Every
  row carries a hit box from `hud::offer_layout`, **one definition read by the
  drawing and by the click** — a hit box that disagrees with the thing it is
  under is §R2 wearing a mouse. The two dials carry `<`/`>` ends so the
  pointer can *work* them rather than only select them, under the button bar's
  own press/release protocol (sliding off takes the press back). **Every old
  letter still works**; nothing was taken away to make room.
- **Choices stay set.** `founding::Memory`. They did not — `toggle_founding`
  built a fresh `Offer` on every open. **The same line was eating the
  reroll**: `commit_founding` called `reroll()` and *then* dropped the offer,
  so the three it had just drawn went out with it and the next open served
  attempt 0 again. "Committing is what costs you the other two" was the design
  from day one and had never once happened in the game.
- **The game pauses.** `Druid::time_stopped`, *derived* rather than written
  into `paused`: the alternative is saving and restoring the player's own
  pause across a modal, and a restore that misses one exit path unpauses a
  game they had deliberately stopped. Measured with its control in the same
  run — twenty updates move the clock by **0** with the screen open and by
  **20** with it shut.

Before/after card with the owner: `20260914T203509792Z-1d6a4f`.

### Guards, each watched red

`a_threshold_has_a_middle` (the barcode, `core=0`, is an **asserted** control —
the test fails if the control passes), `the_fringe_never_holds_a_run_longer_than_the_drain_bound`,
`a_colony_is_founded_at_his_feet` (red against the band: 6 of 8),
`every_row_of_the_founding_screen_can_be_clicked`, `the_screen_opens_on_what_was_left_set`
(with a control that the default does not already satisfy it),
`a_commit_is_still_a_commit_after_the_screen_closes`,
`every_row_of_the_menu_is_reachable_and_does_something`,
`a_stale_memory_is_clamped_rather_than_trusted`.

`the_legend_names_every_key_the_binary_binds` now sweeps all three on-screen
legends rather than `KEYS` alone. The claim it makes is *a player can find out
what this key does without reading the source*, and a key live only inside a
modal is discoverable on that modal's own footer; four `ARROWUP`-shaped rows in
the world's corner would be the opposite of what the legend exists for.

### New instrument

`examples/founding_shot` — drives the real `Druid` headless and writes the
picture with the counts beside it. Indexed in `Reports/instruments.md`. It
renders the **world alone**, which `bin/druid` cannot: the biosphere page and
the button bar cover exactly the ground a nest patch is painted on. Its
run-length histogram is the general form for any *"is this pattern regular"*
question, which no image metric answers.

### Paired arms, all in one binary

`PIXEL_PHYSICS_NEST_SHAPE=comb` (the 2026-09-12 patch, verbatim),
`PIXEL_PHYSICS_NEST_CORE=<n>`, `PIXEL_PHYSICS_COLONY_BAND=1`.

### Files, and one I took that the brief did not give me

Owned and changed: `src/sim/creature.rs`, `src/druid/founding.rs`, the `offer`
arm of `src/bin/druid.rs`. Also `src/druid/hud.rs` (the screen's draw and
layout), three small edits to `src/druid/mod.rs` (`toggle_founding`,
`commit_founding`, `update`'s pause gate, one field) — **nothing in the trail
functions** — and **~40 lines of `src/bin/druid.rs` outside the `offer` arm**:
one `Handler` field, the founding branch of `mouse_button`, two helpers, and
two lines in `CursorMoved`/`CursorLeft`. The mouse plumbing is the only place
item 3's "and/or mouse" can live and a lane cannot message its coordinator;
flagged here and in the lane note so Lane C expects the conflict in
`window_event`.

### Gates

`cargo clippy --all-targets --release --locked -- -D warnings`,
`cargo test --release` (full — `--lib` cannot reach `tests/*.rs`, and
`creature.rs` changes are what those catch), `bash scripts/docscheck.sh`
(wanted `readmetoc.py` after the README edit; rerun clean),
`python3 scripts/deadendindex.py --touching` (quiet).

Lane note: `Reports/lanes/druid-founding.md`.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01R9CaT2LnxTh8Khpn2VzELr
