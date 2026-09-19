# Lane note — nest biology research

*Branch `claude/nest-biology-research`, cut from `main` at `c061a245`.
Docs only: `src/`, `assets/`, `examples/` and every test are untouched, as
the brief required — `claude/laughing-davinci-f6lu1r` is live in
`src/sim/creature.rs`, `src/sim/world.rs` and `assets/species/*.ron`.*

## What was asked, and what was delivered

Four areas, all four wanted: chamber architecture and depth, granaries and
food storage, digging regulation and division of labour, nest microclimate.
Delivered as [`Reports/nest-biology-2026-09-19.md`](../nest-biology-2026-09-19.md)
with its index line in `Reports/README.md` in the same commit, plus this
note.

Every section ends in decisions, numbered `D<section>.<n>` and collected in
the report's §9 as one table a build session can work from. Twelve
decisions, four of which are **no** — no CO₂ field, no ventilation, no
fungus garden, no worker age.

## The three findings that should change a build

1. **A colony-wide scalar cannot produce nest architecture.** The
   literature's regulating quantity is worker density *at the excavation
   face*; `NestRoom::occupancy` is one number every ant at the door reads
   alike. That predicts `(Crowding, Dig, 0.6)`'s null exactly — it moved
   digging and never moved **buds (0 vs 0, 0 vs 5, 6 vs 5)** — so the null
   is evidence about the *reading*, not about density-dependent digging.
   The fix is a **local** room reading beside the global one, and the
   harness that reads it (`circ`/`inradius`/`buds`) was deliberately kept
   by that dead-end entry.
2. **The world is one to two orders of magnitude too shallow for the nests
   this material describes** — and the repo has **never stated a
   metres-per-cell convention**, so the factor is uncertain by two. This
   gates every other decision in the report and it is a one-paragraph
   call, not a measurement.
3. **The microclimate substrate already exists and nothing reads it.**
   `FieldCell` carries `temperature`, depth-graded `sky_temperature` (so
   the diurnal oscillator is separable *exactly*, per `CLAUDE.md`'s
   divide-out rule) and `moisture`. Every species wires `(TempAboveAmb,
   Turn, -0.8)` and stops. This is the cheapest high-return item in the
   report: a consumer, not a field.

## Corrections to the brief, verified in the tree

- **"Today that reach is ±2 rows, chosen as a placeholder" is not the
  tree.** `adjacent_nest` on `main` (`src/sim/creature.rs:7242`) is still
  the 8-neighbour *material* test. The `|hy − site.surface| ≤ 2` form is
  the **proposal** in `nest-design-2026-09-14.md` §13, which is a ruling
  and not code. So the brief's true statement — the geometry of *am I
  home* is the geometry of *should I dig* — holds today with that geometry
  being **one cell of contact**.
- **`ROOM_TARGET_DEFAULT = 2.0` is derived, not chosen**, and the report
  says so rather than proposing to replace it with a literature figure:
  its doc derives it from `latecensus`'s ~220 net dug cells against colony
  peaks of 116 and 495 ants. A biological per-capita number would be a
  different world's answer, and I could not find one I would stake a build
  on in any case (report §6 item 2 says so in those words).
- **`(MoistureGrad, Dig, -0.55)` is not a moisture weight.** Its own
  function doc measures it: curvature moves it 1.006–1.012x at every span
  while twenty rows of depth move it **1.91x** — *"it is a depth signal"*.
  So there is already an unlabelled depth term in the dig and its sign is
  backwards for nest-building. Any depth work must name it in its budget
  (`CLAUDE.md`'s *a term in a weighted sum is not an independent knob*,
  arriving before the change rather than after).

## Standards held to

- **No URL is given for any paper**, and the report's header says why: I
  can state author/year/journal confidently and cannot state DOIs with the
  same confidence, and a plausible dead link is worse than a citation a
  reader has to search for. Names are given in a searchable form in §8.
- **Every claim is marked [measured] / [repeated] / [general]**, with
  [general] meaning *my own synthesis, no source* — stated in those words,
  per the brief.
- **§6 is the disagreement section**: six places the literature is
  unsettled or the number is one species in one study, including the
  template-vs-emergent argument, which is the one that actually changes
  what gets built.
- **No harness is proposed.** §7 routes all four open questions to
  existing instruments (`latecensus`, `larder_probe`, `burrow_probe`);
  three are a column added to one, and the fourth is a decision rather
  than a measurement.

## What a build session should do first

`D2.3` (write down metres-per-cell) and `Q1` (band `latecensus`'s footprint
by depth). §0 finding 3 — that the nest has no vertical profile — is
currently an argument from the code and not from a census, and Q1 is what
turns it into one. `Q3` matters as much and is uncomfortable: `burrow_probe`
already reports a hand-carved chamber gone in **30 frames** in `soil`, and
`labnest`'s standing roofed-void figure did not reproduce on 2026-09-14
(230–260 recorded, 45–58 re-run). **Anything in the granary or microclimate
sections that treats a chamber as a persistent place is downstream of a
number that is not currently trustworthy.**

## Head SHA, PR and state — for the coordinator

**PR [#468](https://github.com/sgladstein/Pixel_Physics/pull/468), green and
ready to merge.** A lane does not merge its own PR here (`CLAUDE.md`: an
independent session merges its own, a coordinator merges its lanes'), and a
woken lane has no messaging tools — so this paragraph is the hand-off.

- **`ba125249`** is the CI-verified head: all nine jobs of the
  `pull_request` run (35409342680) **success**, including `cargo test`
  release and debug, `cargo clippy`, `cargo run --example ascii`, the
  structural acceptance cases, worldgen pass interference, `docscheck` and
  `branches`. The commit carrying the report, its `Reports/README.md` index
  line and this note is `96cb18d8`.
- **The `cancelled` jobs on that SHA are not a failure.** CI gates
  `claude/**` on push *and* on `pull_request`, and the concurrency group
  cancels the older run when the PR opens. That is why the PR reads
  `mergeable_state: unstable` rather than `clean` — a non-required check on
  the SHA is not `success` because it was superseded. **There is no merge
  conflict** (`unstable`, not `dirty`) and the branch is 0 behind `main`.
- **No review threads, no `Claude Approvals` check** in this repository.
- This stamp is one commit on top of `ba125249`; it touches only this file,
  so the gates above stand.

**Nothing is waiting on this lane.** The only thing left is the merge.
