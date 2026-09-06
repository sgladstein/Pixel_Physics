# The evolution lab — coordinator note

*Brief, owner, 2026-08-30: build the first part of the new game direction —
the biosphere the shipped plants and ants run in, with the speed-up
function, and stats on screen. Design of record:
`Reports/evolution-lab-design-guide-2026-08-30.md` (PR #158, unmerged).
Branch `claude/evolution-lab-game-interface-9vf0fl`, PR #166.*

**Read this before picking the lab up.** It says who owns which file, what
the owner has already decided, and what is deliberately not being built yet.

## Round one, compressed

Superseded, but two things still bind. `sim::frame::step` is the tick
sequence shared by both binaries; its guard
`frame_step_matches_the_sequence_app_update_ran_before_extraction` holds a
hash taken from the other side of the extraction, so **if it goes red either
a phase moved deliberately (re-take the number and say what moved) or a phase
was added to one binary's loop and not to `frame::step`** — the failure the
module exists to prevent. And **round one's file-ownership table is no longer
in force**: PR #170 landed all of it and later rounds have edited every one of
those files. Re-derive ownership from the open PR list, never from here. The
first bed's census is in
[`../evolution-lab-gate-1-2026-08-30.md`](../evolution-lab-gate-1-2026-08-30.md).

## Round two of the program: what the four lanes settled, 2026-08-30

All four landed. PR #170. What each one *overturned* matters more than what
it built, so that is what is recorded here; the builds are in the lane notes
and in `Reports/evolution-lab-gate-1-2026-08-30.md`.

**Gate 0 is a reach problem, not an economy one — and this is the round's
most important result.** The margin arithmetic above is right: a `-1.0` gut
draws a whole 1,440 flower against a 1,040 bar. Run rather than computed, it
gives margin **+500** and **zero births over 48,000 frames**, richest bank
**568** — the *leaf* ceiling. Flowers and fruit stand **22-40 rows up a
stem**, and `windfall`, the only ground-level form, never exceeds **one
standing cell**. The ants walk the floor beneath food they cannot climb to.
The gut buys survival (3 → 9 ants), never breeding. This is #162's own
caveat coming true — *a fruit has to be found, and the failure case is
foraging rather than economy* — and **the next measurement is a counter on
fruit → windfall, which is small.**

**The frame cost does not follow biomass.** Correlation **+0.03** against
biomass, **+0.92** with the field's solve set, **+0.93** with awake chunks.
The design guide's *"cost follows living biomass"* and its Gate 3 corollary
*"a mature box is the expensive one"* are **backwards in this bed**: the
multiplier **rises** through a session, **9.0x fresh to 17.8x settled**,
because the box quiets as it fills.

**The draw dominates the tick five to one** — 4.78 ms against 0.94 — and
**2.8 ms of that is `sky_light`, running in a box that has a ceiling.**
That is the largest single thing left for the speed dial and it is not in
the simulation at all. **Next optimisation, and it is not where anyone was
looking.**

**Partitions: containment reproduces, the speed-up does not.** `solved/f`
falls 39.4 → 25.1 (−36%) over 1 → 16 compartments, exactly §2c's mechanism,
but the frame goes 1.69 → 1.41 → **1.92 ms**, non-monotone, because the
field is only 54% of a 1.5 ms tick at 512 wide. §2c's 7.6x was measured on a
**fanned 2048-wide** bed. Keep partitions for isolation and scoring; **do
not budget a speed-up at lab scale.**

**The founders were being eaten, and a ceiling was helping.** 8 of 8 plant
and germinate. A colony takes the stand from 55 plants to 19 and then goes
extinct itself (52 founded, 52 dead, 0 born, gone by frame 66,000). And
separately, thickening the ceiling 4 → 7 rows to seat a lamp cost **45% of
the light on the bench and half the stand** — 468 plant cells to 286, 12
seeds to 0 — with **no gate going red**. `field.rs` passes light down a
column as `0.2^(depth/8)`, so shell thickness is a light knob nobody knew
they were turning.

**The grow lamps are not what lights the crop.** `labshot lamps=0` replaces
every fixture with stone and the stand is byte-identical. The glow decays
over a handful of field blocks; the bench is nineteen below. The room
*reads* the schedule, the shell *passes* the light. Recorded rather than
brightened away.

**The dial's crossover is 12 ticks per displayed frame**, so `M* = 12·D/60`:
**12x at 60 Hz, 4x at 20 Hz**. The falling-grain arithmetic overstates it
**~14x** because nothing in a sealed box is in free fall. Under **0.16%** of
the box changes even at the top of the dial.

### What to do next, in the order the evidence supports

1. **The fruit → windfall counter.** Small, and it unblocks the whole
   creature half of the game.
2. **`sky_light` in an enclosed world.** 2.8 ms of a 4.78 ms draw, bought by
   a check the room already knows how to answer.
3. ~~**Gate 2 — does selection have teeth in *this* bed.**~~ **Run, and
   passed, 2026-09-02** — `examples/creature_arena.rs`, on branch
   `claude/pr-213-report-impl-dxp7lw`. The zeroed-brain arm loses on 4 seeds
   of 4 and a random genome loses on 5 of 6, against an identical-arms
   control sitting at the null. **The line every reader of this note needs**:
   the bed only discriminates *past the founding grant*. At `labbatch`'s
   9,000-frame horizon the zeroed brain **wins**, 65.8% on 4 seeds of 4,
   because an ant's grant is 12,000 frames of doing nothing and starving is
   not yet possible inside the window — so any creature result taken at 9,000
   frames in this bed is a reading of who was given more, not who earned
   more. Design: `Reports/creature-genome-flexibility-2026-09-02.md` §9.
4. **Gate 4's two verbs**, cull and partition, which the premise most
   depends on and which have no engine support.

## Round three, 2026-08-30 evening — read this first

### The owner's standing direction, which reframes the programme

> *"Your goals are not tweaking and optimizing evolution now. **Give me the
> tools, data, access to the parameters that need to be tweaked and I do that
> testing myself in the game. That is the game.** If I have access to food,
> water, can cull, can create plants, and creatures, I can figure it out."*

And: *"You don't need to tweak these things. The world starts with nothing,
but the user can add plants, creatures, water, food, soil, etc."*

**So: stop balancing, start exposing.** A default that looks wrong is
something to **register and report**, never to tune.

### Landed

**#176** the lab becomes drivable — two-row mouse bar (look, plant, colony,
cull, soil, water, species, brush, overlays), `SPACE` genuinely stops the
world, the box opens empty, a graded cull, hover readout, thicker dirt with
the cement slab gone, no nest stripe. **#174** ants breed — the blocker was
satiety, not the birth economy. **#178** `labnest`, the playtest report
reproduced.

### Three findings — detail in [`../evolution-lab-gui-physics-2026-08-30.md`](../evolution-lab-gui-physics-2026-08-30.md) §6

1. **Roots steer by air humidity, not soil water**, found by the owner from a
   screenshot. Hydrotropism reads the coarse *field* channel, which **does
   not diffuse inside solid ground** — so below the surface it has no
   gradient, which is why roots stop at 13 rows. Germination gates on it too.
   **The fifth coarse-field occurrence.** Diffusion is double-buffered and is
   **not** at fault — do not re-investigate. Lane:
   `claude/roots-drink-soil-not-air`.
2. **The tunnels were never collapsing** — `overcap` is 0 in every frame of
   both arms, so my one-unit-margin hypothesis is measured wrong. Real:
   **ants 52 -> 12 between frames 4,000 and 5,000**, digging stopping with
   the colony. That run had no food; **re-run now #174 has landed**.
3. **The dirt-depth regression does not reproduce**, and the light steer I
   gave that lane was wrong — bench light is flat at 0.447 across 48 runs and
   the figure that started the hunt is *downstream* of the stand. Deep soil
   is earned by **the ants**: roots max at 13 rows, galleries already at 35
   of a 40-row bed.

### Open PRs — land one at a time, merging `main` into each first

#175 and #180 both touch `field.rs`; #177 and #179 both touch `creature.rs`.
`wiki/ants.md`'s freshness note has now conflicted on **three** branches;
expect it, resolve keep-both.

| PR | what |
|---|---|
| #177 | The ants **do** tunnel — one connected gallery, unlit rather than undug. **See the block below** |
| #179 | `cell_scale` reaches the living. **Approved by eye** |
| #180 | Lamps light the bed; moving one moves what grows. **Approved by eye** |
| #181 | `FIELD_SCALE` 8 -> 16. **Approved by eye** |
| #175 | Half the draw, picture unchanged |
| #182 | This handoff |

### Verdicts from the 21:30 batch — one BLOCKS a PR

- **#177's lighting fix is REJECTED.** He chose **the shipped lighting**,
  *"the current light model is much better"*. **The finding stands and the
  fix does not.** Land the measurement and counters alone, or put the
  lighting behind a switch that is off. Making a standing gallery legible
  needs a different answer.
- **Parameters panel: rated 5, *"this is a great next step"***. Priority.
- **Double cell density confirmed twice** — he chose 1024x640 with light per
  16, and rated the same-physical-size card **5, *"Yes"***. **#179 and #181
  together are the direction**, not an experiment.
- **The 36-cell creature is not there yet, and the critique is shape.**
  *"Shape. It is a perfect cube. Are there perfect cube creatures in our
  world?"* and *"both are smudges but A is closer"*. **More cells did not by
  itself produce an animal** — `plant-appearance-design.md`'s lesson arriving
  on the creature line. Read it before trying again.

### Two gaps the owner will hit — the first is now half closed

**There is no save** — a parameter he changes and cannot keep is a toy. And
**species parameters are not in `tunables.rs` at all**: `dig_force`,
`hunger_fraction`, `gut_bias`, `reproduce_threshold`, `mutation_rate` are
compiled in via `include_str!` and unreachable at runtime. That is the
largest single gap between the lab and *"I can figure it out myself"*.

## Round four, 2026-08-31 — the specimen shelf

*Brief: "save genetics of creatures and animals, clone them or mutate."
Branch `claude/evolution-lab-genetics-gw4cv3`. **Everything below is
compressed; the account is
[`../evolution-lab-genetics-2026-08-31.md`](../evolution-lab-genetics-2026-08-31.md),
and the shipped behaviour is README's "Specimen shelf status".***

`KEEP` (`M`) jars the genetics of anything alive, plant or animal; the rack
behind the bar's jar chip (`G`) is what you kept; `FREE` (`,`) puts the armed
jar back, at a dial counted in **broods** — one brood is the engine's own
per-birth mutation applied once, so `mutation_rate` on the parameters page is
what a brood is worth and nothing new was calibrated. Jars are files under
`assets/shelf/` (gitignored; `PIXEL_PHYSICS_SHELF_DIR` overrides).

**What it overturned** — the part a later session cannot reconstruct:

- **"No save" was two gaps and only one was a parameter.** A box that throws
  away the one good forager it produced is the same defect and was on nobody's
  list. The parameter half is untouched and is still the larger.
- **An experiment in this bed could not be repeated, and nobody had said so.**
  A founder's genome is keyed on `(world seed, germination coordinate)`, so two
  runs of "the same" experiment start from different plants. **This is what
  Gate 2 was missing**: a null there is a statement about the world rather than
  the genome, and separating those needs the genome held while the world
  changes. Run Gate 2 now rather than later.
- **`Origin::Founder` reads the *species* genome**, so nothing could put a
  *specific* animal in the world except `Origin::Bud`, which needs a live
  parent and charges it. `Origin::Stock` is the third origin; a release books
  as a spawn, never a birth.
- **The bar is full.** Row 1 was already at exactly its own width; row 0's 76 px
  of slack is now gone, and both rows fit only at the tightest `SPACINGS`, 1 px
  spare. **The next lab control cannot just be added** — it needs a third row,
  a page, or a removal. Three attempts and the measurement are in
  `dead-ends.md`.

**Next:** `CROSS` (breed two jars — D4 caged the brain's topology on one shared
scaffold *precisely* so crossover is possible, and there is still no verb);
then Gate 2, now that it can have a control arm; then `PROMOTE` for plants,
which refuses today because `individual_as_species` copies the parent
*species'* fates rather than the individual's; then the parameter half.

## Round five, 2026-09-01 — a playtest round

*Four things the owner hit in play: zoom, selection, stocking, the plant
count. Branch `claude/evolution-lab-fixes-dzo88x`. **The account is
[`../evolution-lab-playtest-round-2026-09-01.md`](../evolution-lab-playtest-round-2026-09-01.md)**;
the shipped behaviour is README's "Lab hand-verbs status". Four findings from
it bind on whatever comes next:*

- **A verb can be complete, tested, documented and not wired in.** A released
  *animal* had never taken a tick — the shelf's own tests all place and then
  inspect, and **none ran a frame**.
- **Anything in `render.rs` bounded by "the world is huge" is a defect waiting
  in the lab.** Two were in one function.
- **A knob that is not an `f32` has no editor**, because the parameters page
  moves numbers. Three animals shipped and one could be placed.
- **The bar being full has a fourth answer** round four did not list: a cell
  that already exists and means nothing under the armed tool.

**One loose end, filed rather than fixed:** the census sees a live plant
organism with **zero cells**, one at a time, intermittently. One in ~450, not
the seed bank, and nobody has looked at what produces it.

## Round six, 2026-09-01 — the lab's own frame, measured

*Owner's two questions: is 60 Hz the limitation, and how do we run faster.
Branch `claude/evolution-lab-speed-performance-lshi7b`. **The account is
[`../evolution-lab-frame-cost-2026-09-01.md`](../evolution-lab-frame-cost-2026-09-01.md)**
and the shipped behaviour is README's "Lab speed-dial status". Only what a
later session cannot reconstruct is here.*

- **60 Hz was never required and is not the limitation.** The rate has been
  decoupled since Gate 3; it merely defaulted to 60 and never moved on its
  own. The whole 60 → 10 Hz ladder is **1.2x** in the real loop (2.03x →
  2.47x achieved), because a tick is 7.3 ms against a 4.7 ms draw. **Gate 3's
  "roughly triples" is retired.** Shipped anyway, as the owner asked:
  `time::AUTO_DISPLAY` takes the rate from the dial and `F` now sets the
  **minimum framerate** floor under it.
- **Round two's "the draw dominates the tick five to one" was measured on a
  young bed and has reversed.** On a grown one it is 6.4 ms of tick against
  4.7 ms of draw. **Which half dominates is a function of how full the box
  is**, so neither figure transfers without the stand's cell count beside it.
- **63% of the tick is soil moisture, and none of it is plant code.** The box
  is 0.006 ms empty and 7.03 ms with eight plants; `active_sites` is 0.28 ms
  of that. 410 of the 447 cells that change per tick are soil wetness, each
  marks a chunk dirty, and a dirty chunk buys the CA sweep **and** the field,
  because `field::step` is gated on `active_chunk_count()`. Ablated, the tick
  is **6.39 → 2.39 ms** and the multiplier **2.6x → 6.9x**, with a *larger*
  stand. This is the mechanism behind round two's "+0.92 with the solve set,
  +0.03 with biomass".
- **The sweep cannot be narrowed for free, and this blocks a whole class of
  optimisation.** Its random draws are consumed per *visited* cell, so any
  tighter region shifts the RNG stream and moves every pile in the world. Per-
  row dirty spans measure a real 1.19x and ship **off**
  (`PIXEL_PHYSICS_SWEEP=rows`); the unlock is a positional RNG, not a better
  region. In `dead-ends.md` with that condition.
- **`examples/labperf`** is the instrument: what the sweep is asked for
  against what it finds, and a per-phase grid diff naming which phase wrote
  the cells that keep the box awake.

**Moisture moved, 2026-09-02 (report §8).** Its own dirty channel and its own
pass: **tick 6.42 -> 3.81 ms, awake chunks 20.4 -> 8.3, the dial 2.6x -> 4.4x**,
and the stand **9.5% larger**, which is what says speed-up rather than
subtraction. `PIXEL_PHYSICS_MOISTURE=sweep` is the control. Two things it
overturned:

- **A phase written into `frame::step` does not reach the engine.** There are
  **155 call sites** that drive the world by calling a CA driver directly, and
  a frame phase is invisible to all of them -- three went red with nothing
  wrong in the code. It lives inside `parallel::step` and `update::step` now,
  where weather and spring already are and for the same stated reason. **Any
  future per-tick work has this choice and should not re-derive it.**
- **A chunk-local prefilter over the moisture region is *slower*** (1.37 ms
  against 1.23), because **88% of that region is soil**. The premise was
  assumed. What the same counter points at is real: ~300 ns per soil cell,
  nearly all of it `World::get`'s `HashMap` probe, so the next step is a
  `ChunkView` over the moisture channel -- worth about another 1.0 ms of 3.81.

**Next:** that `ChunkView`; then the field's all-or-nothing early-out (1.25 ms
of 3.81, still gated on *any* chunk being awake anywhere); then the positional
RNG, which unlocks the region work.

## Round seven, 2026-09-01 — tracking individuals

*Brief, owner: a table of all plants and all creatures you can click through
with stats and a highlight; a life history for one individual; a genome summary
in plain language; brainstorm the rest. Branch
`claude/evolution-lab-tracking-iacu4z`. Owner's scoping answers: all three
history tiers, dead individuals stay listed with a cause, order is tables →
plain speech → history, and all four brainstormed extras (FOLLOW, sortable
columns, side-by-side comparison, a lineage overlay).*

**Landed so far: the rosters.** README's *"Roster status"* is the shipped
behaviour. What it **overturned**, which is the part a later session cannot
reconstruct:

- **`Body::Head`'s hit target was built and thrown away.** `paint_page` has
  always collected clickable headings into a `taps` vec, and the one caller
  drawing a `Panel` through it passed `&mut Vec::new()`. Nothing had noticed
  because no generic page had a clickable heading yet. Anything added to the
  PLANTS / ANTS / BOX pages as a `Row::head` before this fix was a button
  nobody could press.
- **The bar has run out of spacings, and this is the number to carry
  forward.** At the start of this round `pad=2 gap=1` fitted with 2 px spare on
  row 0. **After merging round five, it does not fit at all** — row 0 overruns
  by 4 px — and the only rung left is `pad=1 gap=1`, where **both rows sit at
  exactly 508 of 508 with zero slack**. `layout` has nothing tighter to try.
  So the bar is no longer "nearly full": the *next* widget on it, of any width,
  fails `the_bar_fits_the_screen_and_no_two_widgets_overlap` outright.
  **The pattern that works is a `Body::Head` row on a page that already has a
  chip** — that is how both rosters are reached and it cost no painter code.
  Round four's list of answers (a third row, a page, a removal) now has that
  fourth entry, and it is the cheap one.
- **A page with no bar chip has no way out, and nothing else here has that
  shape.** Every other page is closed by pressing the chip that opened it; the
  roster needed a `BACK` button of its own. The harness found it twice by
  panicking, which is `labui` doing exactly its job.
- **`OrganismState::born_frame` now exists**, stamped at `World::push_organism`.
  The reason is identity: a handle is a 12-bit slot plus a 4-bit generation and
  **is reused**, so a pin keyed on the handle alone follows a stranger after
  sixteen turns of one slot. It doubles as `AGE`. **PR 3 was going to add this;
  it is in already**, and the life record's counters hang off the same field.
- **Two roster columns were vacuous and only the rendered table showed it.**
  `HUNGRY` for all 52 ants (floor scaled off `body_energy` 480, what a corpse
  is worth, against `start_energy` 200, what it lives on) and `HOME` for all 52
  (`nest_memory` is a 3,000-tick sense window, not a place). Both are
  `CLAUDE.md`'s *ask what your number counts when nothing is wrong*, in a
  column.
- **A guard was blind and had to be replaced, not widened.** *"Sort the same
  list eight times and check the order holds"* stays green with the tie-break
  deleted **and** `sort_unstable_by`, because a sort is deterministic inside
  one build. The replacement asserts `roster::compare` is a **total** order —
  `Equal` only against itself — so no implementation has a tie to make a choice
  about. **Any future sort in this game wants that form**, not the repeat form.
- **The two tables must not share a sort.** A sort is a *column index*, and
  index 1 is SEED on one table and BANK on the other. Shared, the harness
  reported a click pinning ant 41 where ant 11 was expected.

**Round five's zero-cell plant shows up here.** Its loose end -- *"the census
sees a live plant organism with zero cells, one at a time, intermittently"* --
is a row on the roster with `CELLS 0`, and `roster::anchor_of` has to survive
it: a plant with no cells has no lowest cell and no bounding box, so the
marker falls back to the origin rather than panicking on an empty fold. That
is a guard against the symptom and not a fix for the cause, which nobody has
looked at.

### Measured on this branch, and reusable

- `labstats frames=90000` on the shipped bed: **3,099 seeds borne, 279
  sprouted, 15 animals born, 64 dead**. So a run log recording every seed-set
  is **90% seed-set** and drowns; recording only an individual's *first* seed
  brings it to ~640 events per 90k frames, which a 2,048 cap covers for about
  290,000 frames. **This changed the run log's design before it was written.**
- **`eats` cannot be closed against anything and must not be mirrored
  per-individual.** Measured on `ascii`'s colony: `eats 283 / pickups 265` at
  2,000 frames and `1142 / 1017` at 12,000. It is incremented at two sites for
  one food cell (`creature.rs:2702` into the crop, `:1948` a cell absorbed) and
  the two comments disagree about what it means. Close a per-individual bite
  count against **`pickups`**, which has one site. Filed as its own finding.
- Frame-cost baseline for the next three PRs, `RAYON_NUM_THREADS=4`, `ascii`
  colony: **mean 3.98–4.30 ms over 12,000 frames**, worst ~52–57 ms and **not
  pinned by the aggregate** (`mean x frames` is ~51,600 ms), so quote the mean.

### The genome in words — landed, and what it overturned

README's *"Plain-speech status"* is the shipped behaviour. Three findings a
later session cannot reconstruct:

- **The two pheromone channels have no meaning in the engine, and assuming one
  gets it backwards.** `brain.rs` describes A and B as two anonymous planes;
  what a channel *means* is decided entirely by which weights emit onto it. The
  first phrasebook hard-coded *"A is the food trail"* — and on the shipped ant
  **every ant lays A all the time** (`Bias -> EmitA`), which pools it round the
  nest, while **only a laden ant lays B** (`Carrying -> EmitB`), which marks
  the way back from food. So A is the *home* scent and B is the *food* route,
  the opposite way round. The label is now derived from the individual's own
  emissions. **Anything else that reads a pheromone channel should derive its
  meaning the same way** rather than writing it down.
- **All of the ant's conditional behaviour is in the hidden layer, and none of
  it is in the direct one.** Its twelve direct weights are unconditional; the
  foraging loop is twelve hidden weights pairing a `Bias` offset against a
  `Carrying` weight so the unit switches with the load. A reader that walks
  only `Wiring::instincts` — which is the obvious thing to do, and what the
  first version did — sees an ant that lays two scents and never follows
  either. The pairs are push-pull (same condition wired twice with opposite
  signs on sensor *and* output), so they collapse to one behaviour, not two.
- **A fold group larger than the whole budget could never be opened**, in
  `inspect_rows`. The rule was `cost <= room` for every group including the
  chosen one, so clicking such a heading did nothing for ever. Latent since the
  fold landed; nothing had hit it while the largest group was eleven rows
  against fourteen.

Also: `TRAIT_REPRODUCE_AT` had been **missing from `specimen_sections` since
that group landed** — two of three body traits printed, and a page listing two
numbers looks exactly like a page whose subject has two.

**The phrase column is a hard constraint.** `page_rect` sizes the cell page to
its widest row and clamps it onto the screen, so a long sentence widens the
page and slides it over the roster — a thirty-character phrase hid three of
eight columns. `PHRASE_COLUMNS = 26`, swept by
`every_phrase_fits_the_column`. **Any future text on this page inherits that.**

### The life record, tier one — landed

README's *"Life record status"* is the shipped behaviour. Three things worth
carrying:

- **§B2 now has an organism-level counter, and it cost one boolean.** A plant
  the support check fells whole leaves the world with `senescent == false` and
  **no cause at all** -- `plant.rs`'s senescence rule is guarded on
  `!cells.is_empty()`, and a whole-plant felling empties the list -- so it fell
  through to slot reclamation indistinguishable from an organism allocated and
  never given a cell. §B2 has only ever had *cell-level* numbers (654 living
  cells severed per run) and could not say **how many plants**. Classifying
  "no cells, no cause" at `free_organism` as `FELLED` is that count.
- **`World::seeds_set` did not exist**, and two readers had worked around it
  without saying so in the same place: `lab::ui` walks the live list *because
  there was no counter*, and `lab::stats`' `seeds_borne` is
  `fate_mutation_rolls`, a proxy that only moves when the fate roll fires. The
  only cumulative seed figure in the engine was an estimate that fell whenever
  a bearer died.
- **A guard was written and deleted rather than shipped.** The dig mirror must
  sit outside the `spoil_kept()` block; a test named for that trap **cannot go
  red**, because `spoil_kept` caches in a `OnceLock` and defaults to true, so
  under `cargo test` both placements pass. Blind, not weak -- deleted, with the
  reasoning moved to a comment at the call site. **The same applies to any
  future guard over a `OnceLock`-cached env switch in this engine.**

The closing identity is `sum over the living + World::dead_life == the world
total`, and its **vacuity checks fired on the first run**: 4,000 frames of the
colony bed produce no death at all, so the identity held over two zeroes and
would have passed with the roll-up deleted.

### Next, in order
2. **The life record.** Tier 1's identity field is already in. Watch the three
   traps found in review: the `digs` mirror must stay **outside** the
   `spoil_kept()` gate at `creature.rs:2921`; `World::seeds_set` does not exist
   and `stats.rs`'s `seeds_borne` is a proxy (`fate_mutation_rolls`); and a
   §B2-felled plant reaches `free_organism` with **`senescent == false` and no
   death record at all**, so `!senescent` at that seam is the organism-level
   counter §B2 has never had.
3. **`Lab::advance` observes after its tick loop**, so at 256x and 1024x the
   bar's population strip samples once per *drawn* frame — and `ui::History::
   Sample` carries no frame, so its x-axis is the call cadence. `stats::Sample`
   carries one and only loses resolution. Both `observe` calls belong in
   `Lab::tick`; three lines, and the watch ring depends on it.

### The merge, 2026-09-05 — what the brain rewrite cost this branch

*117 behind, `BxF` 1638. The reconciliation was real work, not conflict
resolution, and it produced one finding worth more than the merge.*

- **The page was right and the ant was broken.** `plainspeak`'s guard went red
  asserting `ALWAYS LAYS HOME SCENT`, which it should have — that weight was
  deliberately deleted. But the sentence that replaced it did not appear
  either, and chasing *that absence* reached `open-bugs-handoff.md` **§Z5**:
  the ant's homing odometer charges at `0.0005` against a `W_EPS` of `0.01`,
  so `eval_brain` skips the wire and **no ant has laid channel-A pheromone
  since 2026-09-02**. Measured through `eval_brain` on the shipped genome —
  `h4` flat at 0.000000 through 400 ticks standing *on* the nest. Both
  species that author an odometer have it, and they are the only sub-`W_EPS`
  authored weights in the tree. **Do not "fix" it by raising the weight**: the
  output weight and bias were fitted against the old charge rate.
- **A page that reads a genome back in English is a reader for channels
  nothing else reads.** That is why this surfaced here and not on the creature
  line: the design note verified the odometer's curve by simulating the
  recurrence directly, which is right about the arithmetic and never saw the
  evaluator. `CLAUDE.md`'s *ask what your number counts*, with the instrument
  named as a **simulation**.
- **`LOST` could not survive and was not renameable.** `nest_memory` is gone
  and homing is now per-individual genome, so no Rust threshold can say an
  animal's way home has gone. Rebuilt as `FAR` on `forage_max` — spatial,
  re-anchored at every nest contact, threshold 30 from a measured
  distribution — and taken **out** of the `IN TROUBLE` filter, because a deep
  excursion is a forager working. README's "Roster status" carries it.
- **A sampling guard wearing an exhaustive claim.** `every_phrase_fits_the_
  column` samples 24 random genomes and sees only each one's top five drives;
  `generic`'s doc claimed it proved the width "over every combination". Main's
  two new slots made `SURFACECURVATURE > DROPSPOIL` 28 characters against a
  26-character budget, and neither guard nor page noticed. Replaced with a
  walk of `INPUTS` x `OUTPUTS`.
- **The graveyard landed, and the rendered table caught three things no
  number did.** Every grave read `AGE 9.8K` -- the column is `world.frame -
  born_frame`, which for the dead is time since *birth*. The `labui` tile
  waited on `graveyard.len() >= victims.len()`, already satisfied by four
  plants an earlier tile had killed, so it waited 0 ticks and rendered an
  empty animals graveyard holding somebody else's plants -- the counter was
  right and about the wrong thing. And its filter-reset loop re-used a click
  position from a different page's layout, so the click missed, the filter
  never moved and the loop never ended; it read as a *slow* harness for
  fifteen minutes, not a hung one. **All three were visible in the picture
  and in nothing else**, which is this file's own method arriving on the lab
  line.
- **A cull takes 3,403 ticks for an animal and 3,632 for a plant to reach the
  graveyard** (`roster::how_long_a_cull_takes`), because `rot_remains` carries
  the corpse out at the species half-life. Any harness waiting on a death
  needs sizing from that; two now are.
- **`World::seeds_set` and `World::seeds_borne` were the same counter, built
  twice.** Both sides added a world-level cumulative seed count at the same
  site in `bear_seed_at`. Item 2 above told this branch that `World::seeds_set`
  did not exist, so it built one; main built `seeds_borne` meanwhile. Kept
  main's, and note the branch's had **a writer and no reader** — `ui.rs` still
  walks the live list.

### Keep-and-clear, 2026-09-05 — and three defects only the picture showed

*Owner's ask, verbatim: "there should be options to cull plants/creatures from
the menu or select the ones you want to save and cull all others." The lists
were already there — `F1`/`F2` then `LIST EVERY PLANT`/`LIST EVERY ANIMAL` —
the verbs were not.*

- **`SPARE` on the pinned row, `CULL REST n` on the table's header.** Four
  decisions in it, all recorded in README's "Roster status": it works with no
  pin, it ignores the filter on purpose, it excludes already-rotting rows from
  its count so the face says what the press will take, and the keep list is
  **not** pruned when its members die.
- **A population count cannot verify a cull, and the harness printed one for
  an hour before anyone noticed.** *16 animals → 10* is equally consistent
  with the cull having missed and ten having been born, because the box breeds
  while the bodies rot. The tile now names individuals: *3 of 3 spared still
  alive, 0 of 12 doomed still alive*. This is `CLAUDE.md`'s *ask what your
  number counts* with the instrument named as a **census**, and the tidiness
  tell was there too — 16 → 10 is a clean-looking halving.
- **The keep mark shipped as a blank gap.** The 5x7 font has no `*` — the
  **fourth** time that omission has been found here, and the first the glyph
  guard could not have caught, because the mark was a `format!` inside
  `paint_roster` rather than a string the guard walks. Now `SPARED_MARK`, and
  the guard walks it. The params guard that used to lean on `*` having *no*
  glyph to catch markdown emphasis now asserts on `**` directly, which is what
  it was always trying to say; `lab::time`'s `the_glyph_check_can_fail` moved
  its control to `~`, which its own message told it to do.
- **The cell page was overpainting the roster's header, and the first fix was
  blind.** The cell page is drawn second and slides left when it will not fit
  beside the table, so a wide one ate the right-hand end of `BACK` — for as
  long as both pages have existed. Clamping the header against `inspect_box`
  passed its own test and changed nothing on screen: `inspect_box` is **last
  frame's** rectangle and the frame that clips is the frame the `WORDS` group
  opens and the page suddenly widens. The bound has to be *the widest the page
  can ever be*, derived from `plainspeak::PHRASE_COLUMNS` — whose own guard
  already said why, in a message reading *"it will widen the cell page and
  slide it over the roster"*.
- **Pinning a row moved the cell page's cell and not the individual it
  follows.** `follow_inspected` then dragged the page back to whatever had
  last been clicked in the world, one frame later, for as long as the pin was
  held: row highlighted, marker on the ant, `FOLLOW` walking the camera after
  it, page reading a plant across the bed. `inspect_at` takes the organism
  now. Nothing in 1,397 tests saw any of these three; all three came off one
  cropped contact sheet.
- **`labui`'s close-then-open navigation is a fixed point, not a sequence.**
  Three blocks hand-rolled *close whatever is open, click the cover, click the
  list*, and the third panicked — because the block before it left the roster
  open where the earlier ones left it shut, and `Action::Panel` toggles: the
  roster's way out is its cover's chip, so pressing the cover from the roster
  lands on the cover and the next press closes it. `open_list` loops towards
  the list instead and is correct from any starting state.

## Round eight, 2026-09-03 — the frame cost re-measured, and the target has moved

*Owner: "performance has gotten a lot worse." It has, it reproduces, and it is
**not** a regression in the speed work — the account is
[`../evolution-lab-frame-cost-2026-09-01.md`](../evolution-lab-frame-cost-2026-09-01.md)
§9. Three things bind on whatever comes next:*

- **The default bed reports the regression as a 1.3x improvement.** `bin/lab.rs`
  opens *empty* and the owner paints the population in, so the 8-founder bed
  every lab measurement in this repo uses is not the bed being played. Measure
  on **`founders=128 colonies=1`, 12,000 frames**; that is where it shows.
- **Every phase is unchanged except `active_sites`, which is 4.5x** (0.49 ->
  2.22 ms). The CA sweep, the field and the moisture channel all held across 81
  commits. Inside it, the scheduler is 0.3 ms of the 1.7 ms growth and
  **`plant::step_organisms` is the other 1.4** -- once per organism, never
  optimised, ~11x.
- **Two terms multiply and both are real: 2.7x more organisms (441 -> 1,174),
  each 1.7x dearer.** The average organism is *smaller* (7.2 cells against
  9.1), so the per-organism rise is new work rather than more cells. This is the
  germination fix and seed dispersal doing what they were built to do; the
  optimisation target has simply moved off the sweep and onto the plants.

**Also in play now:** organism slots at **1,239 of 4,095** on a medium bed at
12,000 frames and still climbing.

**Then the owner named the regime, and it sharpened all of the above (§10).**
*"Previously 10-30x with plant structural damage off; now I max out at 4x"* and
*"the toggle doesn't change anything right away but once a plant grows and
collapses, the collapse destroys performance."* Three results:

- **`plant_load_failure` off used to buy nothing (3.95 vs 3.95 ms) and now buys
  1.32x**, halving `active_sites`. Over half that phase's new cost is collapse
  work on living tissue -- the thing the owner turns off by hand.
- **The big-plant box no longer self-limits.** `species=tree founders=16
  colonies=0`, 32,000 frames: the old stand peaks at 6,100 cells / 93 organisms
  and *falls* to 2,961 / 33, dial recovering to **6.5x**; the new one climbs to
  **27,013 / 646 and is still climbing**, dial pinned at **2.5x from frame
  8,000 onward**. Find what used to make it stop.
- **The median frame barely moved (2.07 -> 2.25 ms); the mean went 2.57 ->
  6.78.** Two-thirds of all time is now in frames above the median, and the
  dial reports the mean -- so a box whose typical frame is fine reads as
  permanently slow. **Chase the tail, not the phase table.**

**Corrected, and then narrowed by the owner (§11).** *"I would prefer to solve
it with plant structural damage off first."* Two results:

- **"The new box never self-limits" was wrong** -- an extrapolation from a run
  that stopped before the turn. At 64,000 frames it does turn over; the brake
  engages ~4x later and at ~4.5x the population. *A cascade censused before it
  settles*, arriving on a population curve.
- **With damage OFF, nothing got dearer per cell.** `active_sites` 1.77 -> 4.19
  ms while `ca_sweep` and `field` hold, and **per plant cell it is 0.159 ->
  0.151 us -- unchanged.** The box just grows 2.5x more plant. `active_sites`
  was already the largest phase on the old build in this regime (49%) and is
  now **70%**, and `plant::step_organisms` has never been optimised. **That is
  the target, and it is a pure win**: no behaviour change, no seed sweep, no
  owner verdict.

## Round nine, 2026-09-04 — the parameter half of "no save," closed

*Owner: "I need parameters in the evolution lab game to persist when I close
the game and reopen it even if I recompile ideally." Branch
`claude/evolution-lab-persistence-tv32nw`. Full account in README's "Lab
parameters status".*

Round three's "no save" turned out to be two gaps, and only the save *action*
had ever been built — nowhere for it to be read back from.

- **The lab never reloaded `assets/materials`/`assets/species` at startup.**
  `params::save` already wrote a real, parsed `.ron` (found already on
  `main`, not built this round); a save from a previous session was
  invisible to the next one until the binary was rebuilt, which is why it
  looked like it survived a recompile and never a plain restart.
  `bin/lab.rs` now reloads both directories, then rebuilds the box a second
  time on top of the fresh registry before applying anything saved — the
  founders a saved bed places have to be seeded from the reloaded data too,
  not only their offspring. **Confirmed live, not just by test**: a saved
  `founders=40 colonies=3 colony_ants=52` bed opened at exactly **156
  animals** — 3×52 — at frame 0, paused, before simulation could have
  produced that number by any other path.
- **`LabBox` (the bed spec) and the eight `Rule`/`Heredity` `World` dials had
  no file at all**, and it was worse than a restart: `Lab::reset` was
  already carrying the three damage switches across an in-session `REBUILD`
  by hand and silently dropping all five heredity numbers on every one —
  nothing had caught it because nothing else read them back either. Both now
  round-trip through their own file (`scene::LabBox::save`/`load_saved`,
  `params::Dials::save`/`load_saved`, one small struct apiece), the same
  `S`-to-save gesture every other row already used, and `reset` now carries
  all eight dials through `Dials` instead of three by hand.
- **Gitignored** (`assets/lab_bed.ron`, `assets/lab_dials.ron`), on the
  specimen shelf's own reasoning rather than `player.ron`/`explosion.ron`'s:
  this is one player's current box, not authored content the whole game
  ships.

**Deliberately not touched:** `Knob::Heredity`'s own doc already argues
against a species-file heredity field (it would reintroduce the
bit-identical-sweep problem `fate_mutation_chance`'s doc records). The new
file is a separate, gitignored, runtime-only state file rather than a
species field, so that argument still holds and was not re-litigated.

## Round ten, 2026-09-06 — the soil layer

*Owner, from play: "when ants dig, the holes often fill up with loose soil"
and "over time it seems like the soil is disappearing." Branch
`claude/evolution-lab-soil-layer-tjkyv4`. Shipped behaviour is README's **Lab
soil status**; the cost is `open-bugs-handoff.md` **§W4**. Only what a later
session cannot reconstruct is here.*

- **Round three's "the tunnels were never collapsing — `overcap` is 0 in every
  frame of both arms" is overturned, and the reason is the scene.** `labnest`
  defaults to `founders: 0`, and a bed with no plants in it is a bed in which
  **nothing moves the water**. At `founders=8` the same harness reproduces the
  report outright: `overcap` 0 → **34,779 of 48,000 cells**, lining **234 →
  40** with the colony still digging. An instrument's default scene is an
  input like any other.
- **The un-pack line was `SOIL_FIELD_CAPACITY`, which is where drained ground
  *rests*, and the lab's bed is built exactly on it.** So every gallery in the
  box dissolved as soon as anything was planted. `packedsoil.ron`'s own
  justification — *"above field capacity the pore space is full"* — is right
  about the physics and named the wrong constant; field capacity is by
  definition where the drainable pore space has already emptied.
  `material::SOIL_WATERLOGGED` now, sited in a measured gap (the bed is
  bimodal: 32,936 damp, 1,476 saturated, **367 anywhere between**).
- **`corpse` was the only dead-organic material with no `decays_into`**, so a
  body plugged the gallery it died in for the whole run and locked its matter
  out of a sealed box (104 of 104 pool cells, 100%). The `dead-ends.md` entry
  that recorded the silence as deliberate is about the **energy ledger**, not
  about carrion — read as the latter it kept a real bug alive for months.
- **The bed drains through *root turnover*, and a single-cohort census cannot
  see it.** A root borrows a bed cell and used to return 5% of it. `labmass`
  with no cull reads −1,893 with 1,756 standing as roots (residual 137, looks
  fine); the same bed culled every 4,000 frames reads **−950 with 282
  standing — 842 cells destroyed**. Any future claim about this box's mass
  needs the turnover arm.
- **The moisture of a returned cell is most of its effect, which nobody would
  have predicted.** Writing the cell dry, drawing its water off neighbours
  greedily, and levelling the neighbourhood span **418 / 459 / 411** shed cells
  on the `reinforces_powder` guard; only *minting* it back at field capacity
  recovers the number (198), and that cannot ship. In `dead-ends.md`.
- **Both thresholds were wrong because the rule was, and the owner had already
  said so.** Landed at `SOIL_WATERLOGGED`, then switched off entirely the same
  day on the owner's *"the entire ground in the evolution lab should be able to
  dig tunnels/chambers"* -- which restates design guide **§2b**, already on the
  record since 2026-08-30: *"a roof that falls in is what was declined."* **The
  measurement that reversed it is the transferable part**: a count of cells
  past a threshold is a projection, and profiling the bed *by depth* showed the
  waterlogged cells are not scattered beside roots but a **water table standing
  on the stone floor** -- 94 cells to 1,559 over 12,000 frames, monotone, while
  every band above held flat. A rising permanent dead zone is not the graded
  hazard both thresholds were argued as. Filed as **§W5**; lining in the lab
  bed at frame 12,000 went **40 -> 182 -> 354** across the three builds.
- **The lab's colony still cannot be photographed digging.** The wall census
  moves 40 → 182 and `roofed` void does not, because the colony starves 52 →
  17 ants by frame 12,000 and `digs` plateaus. That die-off has been in
  `labnest`'s output since it was written and is the next thing in the way of
  the nest being a *place* on screen. It is a food problem, not a soil one.

## Round eleven, 2026-09-06 — scenarios

`Reports/lab-behaviour-scenarios-2026-09-06.md` §4 item 1: a saved bed plus
build-time placements, a running timeline and parameter settings
(`src/lab/scenario.rs`), the one engine piece that report's whole approach
was blocked on. Reached from the BOX page's new `SCENARIOS` row, `scenario=`
at startup and on `labbatch`/`labshot`. Nine shipped
(`assets/lab_scenarios/`), one per S1–S6 plus three paired controls. **One
gap the third control found:** `mutation_rate` is the *wiring* rate; the
nine traits mutate under `CreatureDef::trait_variance`, which is an array
and not a page knob, so no scenario can switch trait mutation off yet and
a clonal arm cannot be built from a `Setting`. **What it
overturned:** nothing standing — `reset()` already carried the rule and
heredity dials across a rebuild (Round nine), and a scenario rides the same
seam rather than a new one. Full account in README's own "Lab scenarios
status".

## Round twelve, 2026-09-06 — groups: who is who, and who is family

*Owner, after the first ant-and-beetle equilibrium ("the most fun I have had
in the game yet"): could not tell the species apart without the gut overlay;
could only graph the total; asked how groups are defined, whether two clicks
are two colonies, whether ant colonies ever fight, how friend is told from
foe, and whether an ant is always an ant. Branch
`claude/evolution-lab-creatures-v57wnd`. **The design of record is
[`../creature-groups-and-combat-design-2026-09-06.md`](../creature-groups-and-combat-design-2026-09-06.md)**;
the shipped half is README's "Creature groups status". This session took
over the creature line from `session_01DQxG4SzcwrAeNC9tZTfvMo` (PR #255,
fight mechanics) by poke-trigger; its handoff file is owed on its branch.*

**What the questions turned out to mean, which no reader should have to
re-derive:**

- **Nothing in the engine knew a click had happened.** `found_colony_of`
  placed animals and painted a nest; the animals carried a `lineage` (one
  per founder, so fifty-two per colony) and a `species`, and that was the
  whole of identity. Two clicks were one population. Now `OrganismState::
  colony` is claimed **once per gesture by the first animal that fits**, the
  rest join, children inherit — a founding that tries fifty stations claims
  one number, so `ANT 3` is the third thing the player put down.
- **"Foe" does not exist and never did.** `is_living_kin` is one predicate
  (same species) read at the mouth, the eye and the kin sense; everything
  not kin is either digestible or furniture. The owner's "equilibrium" was
  predator–prey, not war, because no second ant colony could exist. The
  `colony rivalry` dial (parameters, ANTS page, **off**) narrows kin to the
  colony; on, a hungry ant eats a stranger as it eats a beetle and the kin
  sense stops pulling colonies together. **It adds no verb and no private
  scent** — both colonies still write the same two pheromone planes — and
  the report says so rather than pretending otherwise.
- **An ant is always an ant, by construction**: `state.species` is written
  once. Speciation is the report's §3 — a heritable scent signature and
  tolerance replacing the species bit — and it is designed, not built.
- **The colour is a replace, not a tint**, because `GUT_TINT_*`'s 45% pull
  lost a blind A/B and every subtle recolour here has read as blank. Species
  and colony modes plus `Off` (the outdoor game's, byte-identical) behind one
  selector; the graph line under each group is the same colour through one
  function, `render::group_colour`.

**Rivalry measured the same evening, three seeds x 24,000 frames**: off,
zero ant-on-ant kills anywhere; on, 3–9 kills per side per run, both
directions, against 35–42 starvations per side. Predation, not war — the
report's §2 table. `World::group_deaths` (deaths by cause and kills by
attacking group, booked at the bite) is the readout, and `labstats
rivalry=1` prints it.

**The fight session answered inside its handoff and landed PR #263 itself**
(`creature-fight-handoff-2026-09-06.md`): the bite scan now covers the
whole body (a chain creature could not bite what was eating its back — head
adjacent to the beetle 60 times, beetle's head to the ant zero), and gnawing
is priced per jaw closure. Two of its findings bind here. **Ant-vs-ant is
binary**: ant flesh resists 0.25 and the armour trait tops out at 2x, so
0.5 against a bite of 1.0 is one-shot at every point on the axis — the
rivalry dial's fights cannot be graded until the armour reach is settled
for ant and beetle together (raise the material's resistance, or widen the
armour slot's clamp). And **body-fighting between colonies switches itself
on with rivalry**, because `is_living_kin` sits under #263's living-non-self
gate; the `ascii` deposition gate (1.03x, bar 0.9) must be re-run across
the dial if any `ascii` scene ever holds two colonies.

**The threat sense landed the same night** (`ThreatNear`/`ThreatBearing`,
slots 22–23, `is_visible_prey` with the roles swapped so threat is mutual
where diets overlap; `mutation_rate` re-derived to 3.18/584). Nothing
shipped is wired to it and the ant is blind, so **no picture changes and
no owner verdict was owed**; what changed is reach. The next measurement is
`creature_arena --arm=ablate input=ThreatNear` on a bed with eyed ants and
breeding beetles, which needs the fight session's encounter findings first.

**Two things that bind on what comes next:** the report's order (§6) puts
`ThreatNear` before anything on the prey side, because *a bed cannot select
for what the animal cannot perceive* and the prey has no sense for the
predator; and it budgets the kin-sense re-calibration inside the signature
work, not after it (`CLAUDE.md`'s shared-budget rule — `KinNear`'s authored
weights were fitted against a kin that was every ant).

## Round thirteen, 2026-09-06 — kin is a distance, and the castes verdict

*Owner: "once creatures evolve, is an ant always an ant?" and can a colony
naturally differentiate into workers and soldiers? Branch
`claude/evolution-lab-signature-castes-gfm0r2` (Fable, tags `creature-line`
+ `scent-signature`), alongside the Opus session on armour reach, alarm and
`Attack`. **The record is
[`../creature-signature-and-castes-2026-09-06.md`](../creature-signature-and-castes-2026-09-06.md)**;
the shipped half is README's "Creature groups status", second block.*

**Landed:** four heritable slots (`TRAIT_SCENT_A/B/C`, `TRAIT_TOLERANCE`),
`is_living_kin` as *the other's scent within my tolerance* with the colony
label nowhere in it, a per-colony founding offset (`scent_spread`), a
per-birth drift (`scent_drift`), a species-gate dial (`kin_crosses_kinds`,
off), and `World::regroup_by_scent`, which lets the colony label follow the
scent and names a drifted cluster `ANT 1b`. `colony_rivalry` retired.

**What the work overturned, which a later session cannot reconstruct:**

- **"Re-derive `live_creature_groups` by clustering signatures" is the
  wrong shape at the shipped dials, and the reason is placement identity.**
  A census that clusters scent folds two clicks into one group the moment
  spread is zero, because they *are* one point — and PR #255's whole
  deliverable was that two clicks are two lines. So the label stays the
  census and the *label follows the scent*: `regroup_by_scent` splits a
  label whose animals are no longer mutually family and mints the wanderers
  a child label. Splits only, never merges. Adoption is visible in the kill
  tally and the kin sense, not yet as a relabelling.
- **A signature that is only inherited and mutated has no cohesion.** Under
  drift a colony's own cloud spreads exactly as fast as two colonies part
  (both are independent random walks from one point), so what holds a
  colony together is its tolerance exceeding its cloud, not anything about
  being a colony. Real ants share a *gestalt* odour mixed by contact. If the
  owner's picture is colonies dissolving rather than splitting, the next
  mechanism is a contact-blended worn scent beside the heritable one — not
  a tuning of drift.
- **The four new slots consume no birth draw at the shipped dials**, because
  `try_bud`'s loop only draws for `width > 0` and their width is
  `scent_drift = 0`. That is what makes the default bed byte-identical
  rather than merely equivalent; a slot whose variance had been authored at
  0.15 like the others would have shifted every later draw in the jar's
  brood loop.
- **Castes cannot arrive the way real ants get them, and do not need to.**
  There is no queen and no brood; every ant buds a copy. The first design
  here (a threat-driven soldier morph with authored slots) was **ruled out
  by the owner the same evening** as hard-coding what a caste *is*: which
  sense a parent answers, which slots move and which way, and that the
  axis is soldier-versus-worker at all. The design of record is now
  plasticity and nothing named — a `Provision` brain output the parent
  evaluates at budding, a heritable per-slot plasticity block in the
  genome, one line in `traits_of`, a `Made` input, priced by the existing
  levies — under which a soldier is something a lineage finds. Genetic
  polymorphism needs nothing built and is the control. Deferred behind
  `Attack` because a provisioned body that cannot defend is a number.

**Measured** (report §1e; every run `RAYON_NUM_THREADS=4`, paired against
`main` `15ed3d6c` built clean in its own worktree): at the shipped dials
`ascii` is **0 lines different** across its 1,121 non-timing lines
(deposition 1.36x on 237 drops, digit for digit) and the two-colony
`labstats` census is identical on three seeds; `arm=ablate input=KinNear`
on `main` is a **null** (median 49.1%, 4 seeds below 50%, 1 above, 1 tied),
so there is no fitted kin-sense advantage to lose; the narrow end
(`rivalry=1` = `tolerance=-1 spread=1`) kills both ways on every seed
(12/6, 8/10, 7/6 against 0/0 at the defaults); and a tolerant colony beside
an intolerant one loses **14, 14 and 15 ants** to it over three seeds while
killing **none** of it — the raid and the adoption are the same asymmetry.
**Speciation fires on the shipped bed**: at `tolerance=-0.5 drift=0.3` all
three seeds name a split-off group inside 48,000 frames (`ANT 1b` at
32,793 / 36,504 / `ANT 2b` at 23,028; a second split on two seeds), and
almost every kill before the split is a colony eating its own drifted
cousins. At the authored radius of 1.0 two drift rates were byte-identical
over the horizon, and the positive control (radius 0.1, drift 1.0) bit on
the first birth and named nothing because the bed starves a colony under
the three-animal floor first — the bed, not the drift, bounds what can be
seen. **Card `20260906T165111553Z-b2d7cb`** (board `lab`) puts the split
in front of the owner with the two §7 decisions as its question.

**For the Opus session:** `is_living_kin(world, cell, gut)` keeps its
signature; `Gut` lost `colony`/`rivalry` and gained `scent`/`tolerance_sq`/
`crosses_kinds`; `World::colony_rivalry` no longer exists (`labstats
rivalry=1` is an alias for `tolerance=-1 spread=1`). `armour_at` reads
`st.traits` directly and will want `expressed` traits the day plasticity
lands — flagged, not changed. **Agreed order, by poke-trigger both ways
(16:22 UTC):** its armour-reach PR lands first, this branch merges `main`
and lands after. Its diff meets this one in four small places it named —
`gut_of`'s `bite:` line gains a `world.trait_reach` argument, the
`TRAIT_ROWS` loop's span becomes `allele_bound(slot, reach)` (1.0 for every
scent-side slot, so no change here), `specimen::drift`'s clamp reads the
same bound, and `Dials` gains `trait_reach` with a *named* serde default.
Two of its findings bind here and were already met: alleles are drawn in
two places (`try_bud` and `specimen::drift`; `creature::trait_width` serves
both), and a new `Dials` field wants a named default unless it ships at
zero (`scent_drift` and `scent_spread` are species fields and ship at 0).
Its third — `World::plant_ant` claims a fresh colony per call, so a scene
that places ants in a loop is N colonies — matters to any bed read through
`regroup_by_scent`: at the shipped dials every label is one point and the
pass is a no-op, so nothing moved, but a harness that turns the dials on
over a `plant_ant` loop is measuring fifty colonies.

## Deliberately not being built yet

The score and the economy — the guide's Gate 5. **Gate 2, does selection have
teeth in *this* bed, has still never been run**, and `selection_arena`'s whole
finding is that a null there is a statement about the world rather than about
the genome. Until it passes, every evolution result measured in this bed is
unvalidated.

## Environment notes that cost time here

- **The container suspends between tool calls**, so a backgrounded job makes
  no progress while the session is idle. `cargo test --lib` does not fit in
  one foreground call. **Let CI be the gate on the full suite** — it runs on
  branch pushes as well as pull requests.
- **Every push cancels the in-flight suite and restarts a ~19-minute clock**,
  so batch commits. A run whose jobs all read "cancelled" two seconds in is
  the concurrency group superseding a push run with a pull_request run, not a
  failure.
- `rust-toolchain.toml` pins 1.98 and CI has a build cache, so plain
  `cargo clippy --all-targets --release --locked -- -D warnings` matches CI.
- **The lab window captures with no display** —
  `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=N` under `xvfb-run` with lavapipe.
  `labshot` renders the *world* and shows no interface; `examples/labui.rs`
  renders the bar headlessly and scripts clicks.
- **`/tmp` is shared between agents in this container** and the screenshot
  hook writes `$TMPDIR/pixel_physics_lab.png` — one lane captured another
  lane's frame. **Set a private `TMPDIR`.**
- **`review.py inbox --mark-seen` marked all 199 cards seen**, not just the
  caller's. Use `get <id>` rather than trusting an empty inbox.
- **Several agents in one container makes every timing untrustworthy** — two
  byte-identical `ascii` runs have disagreed 2.42x here. Prefer counters, but
  a counter is only load-independent at **fixed parallelism** (610 digs idle,
  278 loaded, same binary): pin `RAYON_NUM_THREADS` or compare arms inside
  one run. Now in `CLAUDE.md`.
- **Cloud lanes cannot be messaged.** `SendMessage` does not resolve a
  `create_session` child, so a brief cannot be narrowed once it is running.
  Write briefs that degrade well: say what to land first and to report the
  rest.
