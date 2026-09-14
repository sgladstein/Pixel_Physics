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

#### ...and then the owner rated that 1 of 5, so the colour went too

Card `20260914T203358857Z-2a28fa` came back **1 of 5**: *"None. There should be
no color. If we have to have this, it should be invisible."* He accepts the
drains are load-bearing; what he rejects is that a player can see them. The
shape work above stands and this is on top of it.

`nest.ron` now carries `soil.ron`'s palette **entry for entry and family for
family**, and `paint_nest_patch` hands each new cell the **shade byte of the
cell it replaced**. `cell_colour` resolves a cell as `palette[shade % len]`, so
the pair reproduces the exact tone the patch covered — same family, same tone,
same grain. The two halves are worthless apart, which is why one guard holds
both: a matching palette with a fresh shade draws a *different* soil, and an
inherited shade into the old tan palette draws tan. It also removes a question
rather than answering one — there is no draw left in `paint_nest_patch` at all,
so founding cannot disturb `World::rng`.

**"Invisible" is a claim about the rendered frame and is measured as one.**
`examples/founding_shot invisible=1` takes two framebuffers of **one** world —
the patch painted, the ground put back cell for cell, drawn again — and counts
the pixels that differ over the patch, with the rest of the frame as the
control. A material-level assertion that the cells share a palette entry would
be the readout being a function of the thing it debugs.

- **The control earned its place on the first run.** It read **1,023 pixels**
  differing *off* the patch: two draws of one unchanged world are not identical
  while the carried quickening's animated haze is on. That is noise the size of
  the whole question, and without the control it would have sat inside the
  answer. Switched off for both arms; the control is **0** now.
- Clean, the patch reads **19 of 25 cells differing, worst channel 12 of 255**,
  against roughly **130** for pale tan on dark loam before. The 6 that match
  are behind the gnome's own sprite and drawn in both arms.
- **The tidy story was wrong and is reported as wrong.** The standing
  hypothesis was the moisture darkening — `cell_colour` gates it on
  `water_capacity`, which only `soil.ron` opts in to, so a nest cell draws dry.
  Split by the water the replaced ground held, the differing cells hold
  **506..531** and the matching ones **523..526**. Overlapping ranges: the
  split does not carry it, so the residue is recorded as unexplained rather
  than as confirmed.

**The last 12 is not reachable from the engine side, and both routes out are
recorded dead ends.** A `water_capacity` on `nest` is already in the register
(on a `Solid`, `Cell::aux` is the structural anchor distance, so the field
would be painted as dampness). Making the door a **flag on soil** — invisible
by construction, no render change at all — re-creates the other: `soil` is a
`Powder`, so `player::footing` goes `Hard` to `Soft` and the gnome wades
through the bottom of a nest wall (`a_nest_still_stops_him`); it also touches
**32 sites outside `src/sim/creature.rs`**, in `lab/mod.rs`, `plant.rs`,
`player.rs` and a dozen instruments. The remaining route is render-side, in
`cell_colour` — `src/render.rs`, Lane A's file, **PR #437 open on it**. Not
taken. It is a handful of lines and can be sequenced after #437 if the 12 is
judged to matter; by eye at play zoom it does not.

**One cost, flagged rather than assumed**: `nest.ron` is shared with the
evolution lab, whose own comment said the pale tan was deliberate — *"to read
clearly against soil and against the dark ants standing on it"*. That is a real
want, and it belongs to a diagnostic box rather than to the game with the
complaint. **The lab loses a visual cue here** and can have it back as an
overlay.

Card out on the result: `20260914T215649928Z-91968a`, blind, before/after.

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

Before/after card with the owner: `20260914T203509792Z-1d6a4f` — **answered,
the owner chose "after"**.

### Guards, each watched red

`a_threshold_has_a_middle` (the barcode, `core=0`, is an **asserted** control —
the test fails if the control passes), `the_fringe_never_holds_a_run_longer_than_the_drain_bound`,
`a_colony_is_founded_at_his_feet` (red against the band: 6 of 8),
`the_nest_draws_in_the_grounds_own_colours` (**its own control fired on the
first run** — `matted_bed` lays every cell at shade 0, so an inherited byte and
a hard-coded `0` were the same picture and the guard could not discriminate;
the bed is given a grain now),
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

### Not taken: the standing water on the door

The owner's verdict also said *"this should be dug into may more"* about the
film §T2 measures. On the coordinator's instruction that is **not** this lane
and nothing here acts on it. The measurement, the mechanism I believe causes
it, and three traps for whoever picks it up are written down in
`Reports/lanes/druid-founding.md` under *Handed back*.

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
