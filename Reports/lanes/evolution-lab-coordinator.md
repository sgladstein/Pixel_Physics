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

## Round seven, 2026-09-03 — the frame cost re-measured, and the target has moved

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
