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

## Round 2 — the seven digging-signal questions (2026-09-19, later)

Answered as [`Reports/nest-biology-digging-signals-2026-09-19.md`](../nest-biology-digging-signals-2026-09-19.md),
with its index line in `Reports/README.md` in the same commit. PR #468 having
merged, this is a **second PR off the same branch** — the merged PR cannot
track new work.

**The four answers that change a build:**

1. **Contents trigger widening.** Workers excavate *only tunnels and no
   chambers* unless brood or fungus is relocated to a spot (Römer & Roces,
   *PLOS ONE* 2014). So the owner's *"nests have no purpose"* is the
   documented mechanism, not just a design gap; eggs would supply the
   **stimulus** a widening rule reads, not merely a reason; and a
   contents-free colony should build **tunnels**, where the coordinator's box
   builds an all-chamber lens. **The lens is failing in the opposite
   direction to the biology** — there is no tunnel mode to end.
2. **The density cue is collision rate and it gates REST, not dig**
   (*J. R. Soc. Interface* 2023). And most of the regulation is **not sensed
   at all**: it is the falling chance of *encountering the face* as space
   grows (Bruce et al. 2019). **That is why three interventions on the
   `Crowding` reading moved nothing** — they improved a measurement of a
   quantity biology does not measure.
3. **Tunnel direction is gravity plus the angle of repose (~40°)** — down
   from the surface, *up* when started mid-medium (*PNAS* 2021). Both already
   free here. `Persist` is the knob they happen to have; it is third at best.
4. **The digging pheromone is dead on evidence, not cost.** Tested directly,
   *Acromyrmex lundi*, fresh face against one aged an hour: **null**
   (Pielström & Roces 2015). The coordinator's withdrawal was right. Meanwhile
   **surface curvature** independently emerges in the termite construction
   literature as the cue guiding early building — corroborating their own 2.3x
   roofed-chamber result better than anything in the first report.

**Conceded: the first report's finding #5 was wrong.** `moisture_gradient`
returns `sqrt(gx²+gy²)` — an unsigned magnitude — so "a depth weight with the
wrong sign" was never coherent, and the term is **inert** rather than
inverted. I had cited the function's doc-comment prose (*"it is a depth
signal"*) instead of reading the four lines under it. The transferable form,
now in the report: **a doc comment's summary of what a channel measures is a
claim to check, not a measurement to cite** — and this repo's doc comments
being unusually good is what made the check feel unnecessary.

**Method limitation stated up front in the report rather than buried**: every
journal domain is blocked by this container's egress proxy, so search-tagged
claims rest on search-result summaries, not papers. §9 names the five to read
before they become code. The report also flags a content-farm page whose
invented "seasonal pheromone blends" are precisely the mechanism that was
nearly built.

## Round 3 — the build plan (2026-09-19, owner asked directly)

The owner asked whether the nest these reports describe is possible here, and
for the steps. Answered as
[`Reports/nest-build-plan-2026-09-19.md`](../nest-build-plan-2026-09-19.md).

**Yes, and cheaper than either research report implied**, because four things
both treated as needing construction already exist — verified in the source,
not inferred:

1. **The dig target is already directional.** `DIRS[heading]`, one cell, **no
   target selection at all.** So "forward" is `heading` and `Turn` is live.
   **This also explains why both dig-*target* dead ends failed across their
   whole ranges**: they weighted *whether* to dig, never *where*, because
   where was never a choice. Neither entry needs reopening.
2. **A dug void already stays open.** `line_burrow` packs all 8 neighbours
   into `self_supporting` `packedsoil`. **`burrow_probe`'s "gallery gone in 5
   frames" is a hand-carved void**, not an ant-dug one — so the gate that
   could have killed the whole plan is already passed.
3. **`Persist` is unwired in every species**, and its doc calls it heading
   maintenance.
4. **Contents exist without eggs** — the dig verb explicitly refuses to take
   a `live_seed` as spoil, so a set-down seed is a persistent object.

**Missing: one sense.** Nothing in the dig decision is oriented to gravity,
depth, or an existing tunnel's axis.

**Correction to my own round-2 ranking.** I ranked `Persist` third and called
it "the knob you happen to have". With a heading-directed dig that was
wrong-headed: `Persist` is the **straightness** half and a gravity-biased
`Turn` is the **direction** half. Complementary, not competing — round 2
presented them as alternatives. The coordinator's instinct beat my ranking.

**Stage 2 is priced honestly rather than waved at**: an input column is **24
live slots**, so `mutation_rate` is re-derived in every species file in the
same change, `brain::mutate`'s draw sequence moves (so births are not
comparable across it even at a re-derived rate), and every `creature_space`
baseline is void. Finite and nameable, which is the test for whether it is
scoped. The cheaper repurposing of the inert `MoistureGrad` writer is
**rejected** — it is also wired to `Drop` at a measured 2.94x, so it trades a
finite cost for the `phototropism_dir` shape.

**Still docs only.** Stages 1–3 land in `src/sim/creature.rs` and
`assets/species/*.ron`, measured at **56 landings in seven days, the most
recent six hours before writing**. The plan is safe to write down while
another lane is live in those files; the code is not.

## PR, head and state — for the coordinator

**PR #468 merged** (the first report, §§0–11). Its content is on `main`:
report 1,324 lines, index line, this note.

**Round 2 is a second PR off this branch**, because a merged PR cannot carry
new work. Read the head off the PR rather than off this paragraph — it has
gone stale three times already. A lane does not merge its own PR here
(`CLAUDE.md`), and a woken lane has no messaging tools, so this note is the
whole return path.

**Nothing is waiting on this lane.**
