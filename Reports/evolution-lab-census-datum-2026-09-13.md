# The five dead chronicle columns: two causes, and neither of them is the nest band

*2026-09-13. Round 32, Lane B. Closes the defect
`Reports/evolution-lab-playtest-2026-09-13.md` §2 found in the owner's
560,000-frame log — `roofed`, `pit`, `pack<`, `mnd` and the nest band each a
single constant across all 56 census samples of a session with 2,982 ants and
356,688 digs in it. That report is in flight on PR #382, which is why it is
named here rather than linked.*

## The finding in one line

**§2's hypothesis — "the nest band is `0/0`, so everything scoped to it
measures an empty set; that is one fix, not five" — is wrong.** The band and
the four footprint columns fail for two unrelated reasons, neither of which
is downstream of the other, and **`pack^`, the one column §2 records as
working, was not working either.** It was counting the deep gallery lining as
mound.

## Cause A — the spec is not the world

`lab::census` took its geometry from the `LabBox` spec: `spec.ground_y` for
"the original surface", `spec.width`/`spec.height` for the rows and columns
to walk. A `LabBox` describes **the bed that will be built on the next
REBUILD**. `params::write_bed` moves it the instant a row on the parameters
page is nudged, and the running world is deliberately left alone until the
button is pressed — `raising_the_width_and_rebuilding_gives_a_wider_world`
asserts exactly that separation, on purpose.

The owner raised the box height during setup. `ground_y` **rides the height**
by design (`write_bed`'s `"height"` arm scales it, so a taller box is not a
bed sitting in the top quarter of it): 320 → 512 takes `ground_y` 160 → 256.
He did not rebuild after it. So for the whole session the census measured a
surface **96 rows below the one the world had** — `ground_y + soil_depth`,
which lands on the soil/stone boundary, down in the foundation.

Everything scoped to "below the original surface" is then a region of solid
stone:

| column | what it read | why |
|---|---|---|
| `roofed` | 0 | no void below a datum inside the stone base |
| `pit` | 0 | same |
| `pack<` | 0 | no worked soil below it either |
| `mnd` | 48 | the whole `MOUND_REACH` window above the false datum is solid bed, so the highest "mound" cell is always at exactly the window's top |
| `pack^` | **wrong, not dead** | worked soil in rows `[surface+48, surface+96)` — the *bottom half of the soil bed* — counted as standing above the surface |

Reconstructed in a test (`a_spec_whose_ground_has_drifted_from_the_world_
still_censuses_the_world`), the pre-fix census reads
**`(roofed 0, pit 0, pack< 0, pack^ 0, mnd 48)`** off a bed with a 9-cell
chamber, a 5-cell shaft, a lining cell and a spoil cell hand-placed in it —
the log's own five constants, from the log's own geometry.

**The repair is that there is one answer to "where was the ground", not
two.** `World::room_datum` is a per-column top-of-ground row frozen on the
first simulated frame, and `World::step_nest_room` — the engine's own nest
census — already reads it. `freeze_room_datum`'s doc records the last time
two rules for that one word drifted apart (422 cells of roofed void against
`lab::census`'s 289 for the same world). `lab::census` now reads the same
datum through `World::room_surface_at`, and walks `World::bounds` rather than
the spec's dimensions. `LabBox::ground_y` survives only as the fallback for a
world that has never been stepped, which is a harness and not a session.

## Cause B — a nest the player founds is not in any spec

`census::nest_columns` built the band from `LabBox::colony_columns` plus a
scenario's placements and timeline. Both are **specs**: they say what was
asked for at build time.

The owner's chronicle header reads **`FOUNDERS 0  COLONIES 0`** and the
session ran five long-ant colonies. He founded them by hand with the colony
key, which paints a nest patch and registers a site
(`creature::paint_nest_patch` → `World::register_nest_site`) and touches
neither spec. So `nest_cols` came back empty, every column fell outside the
band, and the band divided by an empty set in all 56 samples — the dead-zone
ratio the owner's own complaint ("they dig large chambers ... which creates an
area where plants don't grow") is measured by.

`nest_columns` now takes `&World` and unions `World::nest_sites` in. The spec
halves are kept: a scenario timeline can name a colony that has not landed
yet, and a site is only minted when its patch is painted. It is recomputed per
sample rather than once per run, because a colony can be founded at any frame
of a played session.

**This is independent of cause A.** The band reads `0/0` on a bed whose spec
and world agree perfectly, and the four footprint columns read their constants
on a bed with a band. Fixing either alone leaves the other dead — which is the
failure §2's single-cause reading would have shipped.

## What this costs the playtest report (`evolution-lab-playtest-2026-09-13.md`)

- **§2's "one fix, not five" is withdrawn**, and **`pack^` is not the control
  it is named as there.** The two other claims in §2 stand.
- **§4 — "the anthill does not exist until frame 360,000" — does not follow
  from that log.** It rests entirely on `pack^`, which under the drift was
  reading the bottom 48 rows of the soil bed. The 8,352 cells it reaches by
  560,000 frames are a real count of worked soil; what they are *not* is a
  count of mound. §4's consequence — that a nest-structure card must be taken
  at 400,000+ frames rather than 150,000 — is not established by this log and
  needs re-taking on a rebuilt chronicle.
- Everything in §1, §3, §5 and §6 is untouched: none of it reads a footprint
  column.

## The controls

Paired, both halves, per column — `CLAUDE.md`'s *a column that reads 0
everywhere passes any test that only checks it does not crash*. Each carves a
feature of known size, asserts the exact count, then removes the feature and
asserts the column returns to zero; `pit` and `roofed` are additionally
checked to *separate* (roofing a shaft's mouth moves its cells from one to the
other and back), because a hole open to the sky is not a room and a pair that
only ever moved together would be one column shipped twice.

Sensitivity, measured rather than argued, with the mechanism put back in each
case:

| fault restored | what goes red |
|---|---|
| the five columns pinned to the log's own readings `(0,0,0,0,48)` | all five column controls; the band control stays green |
| `surface_of`/`extents` reading the spec again | the drift control, at exactly `(0,0,0,0,48)` |
| `World::nest_sites` dropped from `nest_columns` | the band control only |

`examples/latecensus control=selftest` — which reconciles `lab::census`
against `World::step_nest_room` on one carved box — passes unchanged.
