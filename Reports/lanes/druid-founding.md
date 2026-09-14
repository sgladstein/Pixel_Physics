# Lane B — founding, 2026-09-14

Coordinator: `session_01TngZpRY8LoqpFWHuXjUTTD`. Branch:
`claude/druid-founding`, cut from `main` at `9fd637a7`. Files owned:
`src/sim/creature.rs`, `src/druid/founding.rs`, the `offer` arm of
`src/bin/druid.rs`. Three items from the owner's 2026-09-14 playtest; all
three shipped, each reproduced before it was touched.

## What shipped

**Item 1, second pass — the colour is gone.** The first pass changed only the
patch's *shape* and came back rated **1 of 5**: *"None. There should be no
color. If we have to have this, it should be invisible."* The shape work below
stands (he accepts the drains are load-bearing); what he rejected is that a
player can see it.

**And it is `lab::earth_toned_nest` ported, not a second answer** — the
coordinator caught me scoping a fresh one. The lab solved this same complaint
from the same owner on 2026-08-30 and its doc already rules out the three
tempting fixes, including the one I had shipped an hour earlier: **an edit to
`nest.ron`, which would change the sandbox too, where a findable nest is
wanted.** That edit is reverted. `druid::ground_toned_nest` swaps the palette
on this game's own `Materials` at world construction, and `paint_nest_patch`
hands each new cell the **shade byte of the cell it replaced**. The halves are
worthless apart: a ground-toned palette with a fresh shade draws a *different*
soil, an inherited shade into a pale palette draws pale. Guards:
`a_founding_leaves_no_colour_on_the_ground` (druid) and
`a_painted_threshold_keeps_the_grounds_own_shade` (creature).

**The tones did not transfer, and that was worth checking.** The lab installs
`packedsoil`'s single worked-earth family because a lab bed is packedsoil.
This world's surface is `soil`, which ships **three** four-tone families that
`worldgen::passes::soil_shade` picks between per region, so a fixed family is
right in one part of the map and wrong in the next. `founding_shot` censuses
the ground the patch actually paints over: **soil in 25 of 25 cells**. So the
whole of soil's palette is installed, read off the registry rather than
written down — a copied table goes stale silently here, because a wrong tone
is a faint stripe and not a crash.

**Measured as a frame, per the coordinator's own prescription**, by
`examples/founding_shot invisible=1`: two framebuffers of **one** world — the
patch painted, the ground put back cell for cell, drawn again — counting the
pixels that differ over the patch, with the rest of the frame as the control.

- **The control earned its place on the first run**: 1,023 pixels differed
  *off* the patch, which is the carried quickening's animated haze — two draws
  of one unchanged world are not identical while it is on. Without the control
  that noise is the size of the whole question and would have sat inside the
  answer. Switched off for both arms; the control is 0 now.
- Clean, the patch reads **19 of 25 cells differing, worst channel 12 of
  255** — against roughly **130** for pale tan on dark loam before. The 6 that
  match are behind the gnome's own sprite, drawn in both arms.
- **The tidy story was wrong and I nearly published it.** The standing
  hypothesis was the moisture darkening (`cell_colour` gates it on
  `water_capacity`, which only `soil.ron` opts in to, so a nest cell draws
  dry). Split by the water the replaced ground held, the differing cells hold
  **506..531** and the matching ones **523..526** — overlapping ranges, so the
  split does not carry it. Reported as unexplained rather than as confirmed.

**The residue is not reachable from the engine side, and both routes out are
recorded dead ends.** Giving `nest` a `water_capacity` is one already in the
register (on a `Solid`, `Cell::aux` is the structural anchor distance, so the
field is painted as dampness). Making the door a *flag on soil* — which would
be invisible by construction and needs no render change at all — re-creates
the other: `soil` is a `Powder`, so `player::footing` goes `Hard` to `Soft`
and the gnome wades through the bottom of a nest wall
(`a_nest_still_stops_him`); it also touches **32 sites outside
`src/sim/creature.rs`**, in `lab/mod.rs`, `plant.rs`, `player.rs` and a dozen
instruments. **The remaining route is render-side**, in `cell_colour`, which is
`src/render.rs` — Lane A's file, with **PR #437 open on it**. Not taken. It is
a handful of lines and the coordinator can sequence it after #437 if the 12 is
judged to matter; by eye at play zoom it does not (see the card below).

**Item 1, first pass — "little yellow bars that get placed onto the ground."**
Reproduced as a number first: `paint_nest_patch` laid `i % 3 != 2` over 53
columns, which `examples/founding_shot`'s run histogram reads as **36 columns
in eighteen runs of exactly two** — a dotted line with no middle.
`nest_mask` replaces the comb with an unbroken core and a fringe of single
cells thinning to nothing at the rim: **36 columns in 18 runs → 25 in 15**,
longest run 2 → 11.

**Item 2 — "it should still happen right under or next to the druid."**
`colony_stations` walks outward from the stand a column at a time now, taking
the nearest viable ground and keeping a body's corridor between any two,
instead of stamping a fixed band and dropping the offsets that were not sites.
Nine columns of trunk through a stand: **6 of 8 seated → 8 of 8**, watched red
against the predecessor.

**Item 3 — "the found menu needs to be way improved."** A cursor over
`founding::ROWS`; arrows, `WASD` and the pointer all reach every row, `ENTER`
chooses, `founding::Memory` makes the dials survive a close, and
`Druid::time_stopped` stops the clock while the screen is up (0 ticks open
against 20 shut, with the control in the same run). **Approved by the owner.**

`PR_BODY_LANE_B.md` on this branch carries the full write-up of all three;
what is below is only what a *later session* cannot reconstruct from it.

## Handed back, not taken: the standing water on the door

The coordinator's instruction was to write this down and **not** chase it, so
this is the record and nothing on this branch acts on it.

**What was measured**, `examples/nestdoor`, the lab's played bed, seed 1/2/3,
standing free-liquid cells over the nest patch against the same width of
ordinary ground beside it, sampled at 10k/20k/30k frames with the colony
founded at 6,000:

| arm | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| unbroken patch (`PIXEL_PHYSICS_NEST_DRAINS=off`) | 52, 63, 68 | 10, 9, 10 | 18, 1, 1 |
| ground beside it, same width | 0, 1, 4 | 2, 13, 1 | 5, 1, 0 |
| shipped (broken, `NEST_CORE=5`) | 7, 11, 9 | 1, 4, 0 | 0, 4, 0 |

**The mechanism I believe causes it**, and it is `open-bugs-handoff.md` §T2's
own: `nest` is a plain `Solid` with **no `water_capacity`**, so an unbroken
patch is the only impermeable strip on the surface of a misted bed. Every
other ground cell around it drinks; the door does not, and a film one cell
deep stands on it. An ant cannot step into liquid
(`landing_is_placeable_through_tissue` wants `World::is_empty`), so that film
is a **wall**: `adjacent_nest` goes false for the whole colony at once and
`AtNest`, `nest_visits` and `deliveries` freeze on the same frame. What keeps
the film standing across an unbroken patch is *distance* — the middle of 53
columns is 26 from ground that drinks, and a one-cell film has almost no head
to spread on.

**What the drains buy is not dryness, and that is the part worth saying out
loud**: no part of a misted bed is dry. They buy that the door **stops being
the wettest strip on the bed** — the shipped arm sits at the neighbouring
ground's own level, the unbroken one sits an order of magnitude above it.
§T2 stays open on the residue, and it is right that it does.

**Three things a lane picking this up should know before measuring anything.**
`deliveries` is the noisiest column in the file (154–980 across six seeds of
an unedited ant, per `instruments.md`) and moved *against* the water on a
single seed here — do not read it at n=1. `burrow_probe arms=colony` cannot
see founder placement at all (below). And two of the obvious fixes are already
in `dead-ends.md`: a `water_capacity` on `nest` while it is a `Solid`, and
making it a `Powder` to allow one.

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

**A stale example binary cost the first drainage sweep.** `--example
founding_shot` rebuilt one binary; `nestdoor` then reported **byte-identical**
results for three different `PIXEL_PHYSICS_NEST_CORE` settings, all reading the
old comb's 36 cells, while the `off` arm looked right throughout — which is
what made the table plausible. `--examples`, always.

## Numbers a later session should not re-derive

**Drainage.** The table is in *Handed back* above; the reading for this branch
is that the core is the one place the §T2 drain rule is relaxed, so it is set
from `nestdoor` rather than argued. The run-bounded comb (`core=0`) reads
1,10,15 / 2,3,1 / 1,0,0 across the three seeds against the shipped `core=5`'s
7,11,9 / 1,4,0 / 0,4,0 — indistinguishable, both at the neighbouring ground's
own level. `core=12` was measurably wetter (20, 19, 52 on seed 1) and is not
taken.

**Lab exposure, `labnest founders=8 seeds=2`, `RAYON_NUM_THREADS=2`, the walk
against `PIXEL_PHYSICS_COLONY_BAND=1`.** **52 founders seated in both arms**,
which is the claim that matters; the die-off keeps its shape (52 → 13/16 by
frame 5,000 on the walk, 52 → 15/19 on the band); `roofed` at frame 9,000 is
7/19 on the walk against 15/22 on the band, and `digs` 503/373 against
433/468 — the two seeds swap which arm is higher, so that is the bed's own
spread rather than the change.

**Tried and rejected on item 1's shape:** a linear taper anchored at the core's
edge (~50% dense at the rim, so the patch ended in a straight line of crumbs);
a fringe bounded at `drain_period - 1` (2-on-1-off — a *shorter* barcode, and
the fringe is all the player sees because the core is under his feet); a square
taper from the centre (25 → **13** columns, a door too small to be one). None
is in `dead-ends.md`: each was a tuning step inside one mechanism rather than a
mechanism, and all three are reachable from the shipped knobs.

## Files: what I took beyond the brief, and what I left alone

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

**Left alone after all: `assets/materials/nest.ron`.** I edited it, the
coordinator pointed at `lab::earth_toned_nest`, and the edit is reverted — the
sandbox wants a findable nest and that file is how it gets one. The palette
now moves per game, on each game's own `Materials`.

**One consequence that does still reach the other two games**, flagged rather
than assumed: the inherited shade lives in shared `paint_nest_patch`, so a
nest cell in the lab and the sandbox now draws a varied entry of its own
palette instead of always the first. Both gain grain where they had a flat
tone; neither changes colour. **Follow-up not taken**: `ground_toned_nest` and
`lab::earth_toned_nest` want to be one helper, and a cross-game refactor is
not worth doing inside a playtest item.

## Review cards

- `20260914T203509792Z-1d6a4f` — the rebuilt menu, before/after, not blind.
  **Answered 2026-09-14 21:26Z: the owner chose "after"**, no comment. Item 3
  is approved as it stands.
- `20260914T203358857Z-2a28fa` — the ground after a founding, three arms
  (shipped / grain only / both), **blind**, still open. **Decode the prose
  through `blind_was` before acting on it** — on a blind card the stored
  `choice_label` and the owner's sentence are in different namespaces, and
  this repo has already misread one verdict for that reason. The third clause
  of its question is the one that matters for what happens next: if the answer
  is *the colour*, the fix is `assets/materials/nest.ron` and it is a
  lab-visible change, so it wants its own decision rather than being folded
  into this branch.

Both `owner_can_see_it: true`, verified present on `origin/review-queue` by
fetching the ref rather than trusting the local one (a stale
`origin/review-queue` said "not on remote" about a card that was).

## Gates

`clippy --all-targets --release --locked -D warnings`, `cargo test --release`
(full, not `--lib`), `docscheck.sh`, `deadendindex.py --touching`. `docscheck`
wanted `readmetoc.py` after each README edit and `deadendindex.py` after the
register write-backs; both rerun clean. Head SHA in `PR_BODY_LANE_B.md`.
