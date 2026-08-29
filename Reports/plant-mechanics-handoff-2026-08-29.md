# Plant mechanics: handoff

**Status: handoff, written to be picked up cold.** The plan of record is
`Reports/tree-mechanics-plan-2026-08-29.md` — read its §0 and §10 first, then
come back here. This document is what happened *after* the plan was written:
what landed, six findings that change what the next agent should build, and
the traps that cost this session real time.

**Read `CLAUDE.md` first.** Every mistake below is one it warns about.

---

## 1. State, exactly

| | |
|---|---|
| branch | `claude/tree-physics-destruction-l6eocs` |
| PR #102 | **merged** (`e9d1730`) — the plan, the debris tiers, the colour work |
| unlanded | one commit, `2304871` "foliage stays on the branch it came down on", + a merge of `main`. **Gates were still running when this was written — check them before pushing.** |
| review cards | board `felling`; all three posted this session are answered. The last one's verdict is quoted in §3. |

**`main` moves fast.** Other lanes landed 36 commits in one stretch and
something every ~30 minutes all session. CI takes ~20 minutes, so **four
merge attempts were lost to conflicts appearing during the wait** before the
fifth held. The conflicts were always the same two, and both have a
mechanical resolution:

- `Reports/open-bugs-handoff.md`'s **generated index block** — never
  hand-merge it. Take either side whole, run `python3 scripts/bugindex.py`.
  The block is derived from the headings, so it is correct from either
  starting point. The file says so itself.
- `src/sim/material.rs`'s `MATERIAL_FILES` — both sides only ever *append*.
  Keep both, with `main`'s entries first: that list's own comment states the
  trunk-first tiebreak, and nothing in it is addressed by index.

**Auto-merge is disabled on the repository.** Enabling it (Settings → General
→ Pull Requests → Allow auto-merge) would remove this whole class of waste.
The owner has been told; it is not an agent's call.

---

## 2. What landed

**The plan** — `Reports/tree-mechanics-plan-2026-08-29.md`. One stress
number, two material properties: stiffness decides how far a thing bends,
strength decides when it snaps, and bending *relieves* the moment, which is
why grass bends and never breaks from the same arithmetic that fells a tree.
Its §10 tabulates twelve corrections three independent reviews and two owner
rulings made to the first draft.

**`examples/beam_probe.rs`** — the positive control, run *before* the
mechanism. It reports three section measures side by side because they
disagree on **52% and 61% of 1,946 woody cells**; §9 of the plan says why its
own calibration window is an artifact and must not be used to set a constant.

**Debris that reads as a felled tree.** `deadleaf`, the foliage piece tier
(severed leaves stop becoming powder); and colour preservation across every
tier change.

---

## 3. The six findings, most load-bearing first

### 3.1 The collapse was never a fall

**This reframes the whole package and it is the single most useful thing
learned today.**

Building severed foliage as a `Solid` — the obvious reading of the owner's
*"the leaves shouldn't turn to powder ever"* — left the tree **standing dead
in the shape it grew in**. The crown converts in mid-air, the first pieces to
land form a scaffold that nothing can ever fail (a piece tier is opted out of
the structural check), and no fall happens.

So what has always *looked* like a crown collapsing was **leaf powder running
downhill**. Nothing in the engine makes a severed piece travel sideways. Take
the grain away and there is no fall left.

That explains why the previous attempt (T1) kept measuring well and looking
wrong: 91%+ of woody mass promoting as coherent pieces, against a verdict of
*"It is still very clearly dust."* The pile was formed by flow, not falling.
**The fall is the critical path, not a polish item.**

### 3.2 The load model has no `topple` outcome — and the control proved it

`open-bugs-handoff.md` §Q asks one question before anything is tuned: *what
material is a standing needle made of?* For `scene=fell` it is `log`, and
`filmstrip`'s `log_pieces` census already reports 8–10 of 13–15 settled
pieces upright.

A source reading said: `load::bearing_moment` **is** a tipping test (a
no-tension Winkler bed reducing to "is the centre of mass within a sixth of
the contact width"), and it never runs on a log because the bearing clamp is
guarded on `capacity != i64::MAX` and `log` opts out. True, and the
conclusion drawn from it was **wrong**. Under the control (remove the guard,
re-run the identical cut):

| settled | baseline | clamp reaching `log` |
|---|---|---|
| lying / upright / square | 3 / 8 / 2 | **1 / 11 / 1** |
| `log` cells | 833 | 716 |
| `deadwood` cells | 553 | 645 |

**Giving logs the tipping test does not make them lie down. It crushes
them**, and leaves *more* of the rest standing — the pieces it condemned were
the ones with a footing, and the needles survived.

The load model's two verdicts are *holds* and *fails*, and failing means
`breaks_into`: convert where you stand. **The missing mechanism is not a
test, it is an outcome.** An unbalanced piece has to be re-promoted as a body
and allowed to rotate.

Consequence for scheduling: **rotation and the tipping test are one change,
and the tipping test is worthless before the rotation exists.**

### 3.3 Ask 1 is buckling, not bending

A balanced upright stem has a bending moment of ~0. Measured on the shipped
tree: the base reads **0.0–0.3** against a max of 7,171 — the *least*
stressed place in it. So the moment model cannot deliver "narrow base, heavy
top" at all.

The right physics is **buckling**: `L_c ∝ D^(2/3)` (Greenhill), and that
exponent is identical for a 2D slab and a 3D cylinder, so it sidesteps the
dimension question the rest of the model has to answer.

`organism_structural_tick`'s shipped rule is
`support > max_cantilever_reach · density` — a slenderness rule **with the
width term missing**. Multiply it by the section. Do **not** delete the field:
its `u16::MAX` default is foliage's opt-out, taken by omission in `leaf.ron`
and `grassblade.ron`, and the recorded cost of losing it is median leaves per
tree 1,376 → **1**, stand 31,731 cells → 7,171.

### 3.4 Do not accumulate load up `accumulate_support`'s parent array

It is a **spanning tree over what is, for a thickened trunk, a blob**, and
`dead-ends.md`:807 records a rule built on it taking a stand from 3,437 cells
to 704–2,569. `load.rs` deliberately uses a share-dividing DAG, and its own
doc says why: a spanning tree funnels a building's whole load down one leg —
*"a one-pixel red line through an otherwise green building."* On a tree that
is a red line down the middle of every trunk.

Port `load.rs`'s share division, not its formula alone.

### 3.5 Seed rotation from angular acceleration, not breaking torque

`α = Σmgd / Σmr²` about the break, both sums over cells `Failure::region`
already hands over. For a limb of length `L` breaking at one end this is
`3g/(2L)` — **inversely proportional to length**, so a bole turns about once
over a 50-cell fall and a twig tumbles, with no tuning constant.

Seeding from torque gives `spin ∝ m·d`, so the heaviest piece spins hardest —
backwards, and what `SPIN_PER_SPEED`'s own doc records being tuned away from.

Three things that are **not** free, against an earlier claim of "no new
machinery":

- `spin` is an **angle accumulator, not a rate**; a `spin_rate` has to be
  threaded through promotion.
- **There is no reverse quarter-turn.** `BodyCell::rotated()` is one
  handedness, documented as the single definition of the transform, so a
  signed direction needs three forward turns and a fit probe on each
  intermediate pose.
- `rotate_quarter` turns about the **body origin, not the centroid**. For a
  49x48 bole that is a ~50-cell teleport of the far end in one frame, and
  `rotation_fits` checks only the final footprint, not the swept path.

**And a correction to a number this session published:** "9 quarter-turns
asked, 0 refused" was used to argue the clearance gate is not the blocker.
Those 9 were all **in open air**. The turn that matters happens at the break,
inside the standing tree, which is exactly where clearance is scarce. The
measurement does not say what it was used to say.

### 3.6 Four failed attempts on a colour is the tell that there is no colour

`dead-ends.md` already carried three failures on `log`'s palette — a wide
spread that read as speckle, a grey that read as tissue dying, a warm brown
that vanished into litter. This session added a fourth (bark-outside,
timber-inside), which gave **every felled tree birch-grey bark whatever
species it was**.

The owner, after two wrong readings: *"The color should not change at all
(not sure how many times I have to say this) and most of the leaves should
stay on the branch"*, then *"and the trunks and branches shouldn't change
colour either."*

**The answer was that there is no right colour — nothing should change.**
`log` now carries `wood`'s palette entry for entry, `deadleaf` carries
`leaf`'s, and `settle` copies the cell's own shade across. Bit-identical.

This also restored something the re-roll was destroying unnoticed: a `wood`
band is the individual's **bark density**, chosen by its genome, and a `leaf`
band is its **species**. Re-rolling flattened a varied stand to one tone the
moment it hit the ground.

---

## 4. The code map you will need

Read this before §5; it is the set of things the next three stages touch and
none of it is guessable from the file names.

### 4a. How a piece comes off a plant today

```
plant::anchor_support            Dijkstra from the anchors outward, once per
  (src/sim/plant.rs)             organism per ORGANISM_TICK_INTERVAL (45
                                 frames). Writes OrganismCell::support.
                                 u16::MAX == "nothing I reach touches ground".
        |
        v
structural::organism_structural_tick   two questions off that one number:
  (src/sim/structural.rs:1155)         detached (== u16::MAX) and over_span
                                       (> max_cantilever_reach x density
                                        - supported_load / 4)
        |
        v
structural::detached_organism_piece    the 8-connected, same-organism, ALSO
                                       -detached run, sorted, never iterating
                                       a HashSet
        |
        v
rigid::fell_severed_tissue             (src/sim/rigid.rs:901) only WOODY cells
                                       may seed a fragment; foliage the flood
                                       reaches is carried down with it
        |
        v
rigid::take_fragment  ->  rigid::promote  ->  ChunkBody
  (:960, 8-connected           (:1030)        stepped serially by
   for woody material)                        rigid::step_chunk_bodies (:1835)
        |
        v
rigid::advance (:1859)   gravity, collision, and the spin accumulator
        |
        v
rigid::settle            writes cells back; MaterialDef::severs_into gives the
                         piece tier; the shade is now carried, not re-rolled
```

**`load::Failure`** (`src/sim/load.rs:2309`) is `{ at, mode, region }` — the
break point and every cell of the piece. Stage 5.1 needs exactly those two
and nothing more.

### 4b. Constants and fields that matter

| thing | where | value / note |
|---|---|---|
| `SPIN_PER_SPEED` | `rigid.rs:154` | 0.012; `spin` is an **angle accumulator**, quarter-turns |
| `MIN_BODY_CELLS` | `rigid.rs:116` | 8 — under this a fragment is grit, not a body |
| `MAX_BODY_CELLS` | `rigid.rs:137` | 400, global; leave it alone (plan §5) |
| `MIN_FRACTURE_CELLS` | `rigid.rs:127` | 6 — regions below it crumble; **24% of failed regions on a fell** |
| `GRAVITY` | `rigid.rs` | 0.15, needed for the `α` arithmetic |
| `ORGANISM_TICK_INTERVAL` | `plant.rs` | 45 frames — the cadence anything per-organism must ride |
| `SUPPORT_COST_STANDING/REACH/HANGING` | `plant.rs:3714/3722/3728` | 0 / 1 / 2 |
| `max_cantilever_reach` | `wood.ron` | 96, x wood density. **`u16::MAX` default is foliage's opt-out** |
| `OrganismCell` | `organism.rs:2611` | `support`, `q_peak`, `q_now`, `order`, `path_len`, `heading`, `carbon_conductance` |
| `MaterialDef::woody` / `clings_to_wood` / `anchors_organisms` / `severs_into` / `breaks_into` | `material.rs` | the per-material opt-ins this work uses |

### 4c. Commands

```
# the felling scene, settled, at fixed light -- THE card render
cargo run --release --example filmstrip -- scene=fell fell=7150 \
    start=8200 every=1 count=1 zoom=4 crop=215,135,110,80 daylight=1.0 out=x.png

# the same as an animation (gif needs its own review item)
cargo run --release --example filmstrip -- scene=fell fell=7150 \
    start=7120 every=12 count=48 zoom=4 crop=215,135,110,80 daylight=1.0 gif=1 out=x.gif

# the frame-cost gate -- quote mean AND worst, paired, same session
cargo run --release --example ascii -- scene=ants

# the stress model's own probe
cargo run --release --example beam_probe -- frames=7100 species=tree

cargo test --release --lib      # ~500-600s; run it in the background
cargo clippy --all-targets -- -D warnings
bash scripts/acceptance.sh      # gates `fell` among others
bash scripts/docscheck.sh       # after EVERY merge, unconditionally
python3 scripts/bugindex.py     # after any edit to open-bugs-handoff.md
```

**Always `cargo build --release --examples` before measuring**, with
`set -o pipefail` — a stale example binary prints plausible numbers and has a
newer mtime than the source. Materials are `include_str!`d, so editing a
`.ron` and re-running a prebuilt example measures nothing.

---

## 5. What to build next, in order

### 5.1 The fall — rotation and the tipping test, as one change

**BUILT, 2026-08-29 — `Reports/tree-fall-2026-08-29.md`.** Everything below
this paragraph is the brief it was built from, kept because its reasoning is
what the build followed, not because anything here is still to do. Three
corrections it earned, for anyone reading the brief rather than the report:

- **The reverse quarter turn is one turn, not three.** §3.5 costed it at
  "three forward turns and a fit probe on each intermediate pose";
  `Turn::Ccw` is the exact inverse permutation, equally exact on a grid.
- **The pivot must be *stored*, not re-derived.** Turning about the centroid
  is right and re-computing it each turn is not: the floored mean moves, the
  next turn pivots about the new place, and the piece walks a cell at a time.
- **The topple does nearly all the work and the seeded rate does little** —
  109 topples against 21 in-flight turns over twelve scenes. §3.2 is right
  that the two are one change; what it could not know is the split. A severed
  crown's pieces mostly sit *over* the cut, so `alpha` is small and they come
  down straight; what lays them over is the tipping test on landing.

Two things the build found that change what the next stage should expect:
`open-bugs-handoff.md` **§Z3**, a settled piece re-promoted about every five
frames for ever, present in the unchanged engine and corrupting any
cumulative per-event census on `scene=fell`; and the fact that
`filmstrip`'s `settled log pieces` — the census §5.1's own bars are phrased
in — **folds touching logs into one cluster and reports the pile's
orientation, not the pieces'**. Use `how pieces came to rest` instead.

**The critical path** (§3.1), and the thing that makes every other stage
judgeable, because until a piece travels the settled pile is a heap whatever
else is true.

**Two halves that must land together** (§3.2): a piece needs a reason to
topple *and* a way to express toppling that is not `breaks_into`.

*Sketch, not a specification:*

1. `ChunkBody` gains a **rate** (`spin_rate: f32`) beside the existing `spin`
   accumulator. `advance` adds the rate each frame instead of, or as well as,
   the speed term; the speed term does essentially nothing for tree pieces
   (§3.5's correction) and should probably be suppressed for them.
2. `promote` takes the seed. `α = Σ m·g·d / Σ m·r²` over `Failure::region`,
   with `r` and `d` measured from `Failure::at`. Mass is
   `MaterialDef::density`. For a uniform limb of length `L` this reduces to
   `3g/(2L)`, which is the sanity check to assert.
3. **A settled piece that is out of balance is re-promoted**, rather than
   converted. This is the outcome the model does not have. `bearing_moment`'s
   kern test is the right *predicate* — the fix is what it triggers.

*Three machinery gaps, all named in §3.5:* no rate field, no reverse
quarter-turn, and `rotate_quarter` turning about the origin rather than the
centroid.

*Bars:*
- settled pieces **lying vs upright** — today **3 lying / 8 upright / 2
  square** — and **quarter turns asked vs refused** printed beside it. Turns
  asked that has not moved means nothing fired, whatever the picture shows.
- the negative control: a piece that is *well* seated must not be re-promoted,
  or a settled pile will never stop moving and chunks will never sleep.
- frame cost: `ascii scene=ants` mean and worst, paired, same session.

*Instrument caveat:* `filmstrip`'s `log_pieces` census counts **8-connected
clusters of settled `log`**, so adjacent separate logs merge into one
"piece". It cannot tell one big log from several touching, and it moved from
"largest 447 at 49x48" to "largest 1,222 at 99x71" purely because the pile
packed tighter. Do not read it as a body count.

*Scene note:* `scene=fell` is `trees: 1`. There is no `scene=fellgrove`, so
nothing yet tests a tree falling *into* its neighbours — hang-up,
crash-through, a stripped neighbour. That is where the most cinematic
behaviour lives and it is unbuilt and unmeasured.

### 5.2 Stress, seen before it is used

`OrganismOverlay` (`src/render.rs:900`) currently cycles Off → CellType →
Resource → CanopyDensity → VeinConductance → SoilMoisture → FoodValue →
GutBias → Off, on `L`. Add `Stress` to `next()` and `label()`, and to
`filmstrip`'s `channel=` parser (`examples/filmstrip.rs:2942`).

**A full-replace ramp on a fixed dark→bright scale, never a blend into the
cell's own colour.** The recorded failure: a magnitude-scaled blend produced
a canopy-density sheet that read as blank, because the ramp was red, wood is
brown, and a mid-range value moved one colour byte from 139 to 155. The
obvious reading — "the mechanism is dead" — would send a fix at working code.

Compute on demand for the overlay, the way `App::draw_stress_overlay` already
does for rock (unbudgeted and uncached, because a readout must give the same
answer however busy the frame was). `beam_probe` is the quantitative pair and
already exists.

*Bar:* **not** "the field is non-flat" — that is already true and cannot
fail, which makes it a blind guard. The bar is that a hand-built cantilever
reads hottest **at its root**, and that the overlay and the probe agree
cell for cell.

### 5.3 Then bend, then break

Per the plan's §7, and in that order: bending is visible on every plant every
day, breaking only when something breaks.

**Grass first** — cheapest, most visible, and the owner's named case. A blade
is a few cells tall, so laying it over moves its tip two or three cells;
that is the whole blade, and it is why the "a bend is 0 or 1 cells at this
resolution" objection is true of a trunk and false here.

**Watch the standing-cost bar.** A canopy that never stops moving was
measured at **+8.0 ms/frame**, not from the maths but because it defeats the
dirty-rect render skip (a settled world redraws in 0.012 ms, a moving one in
11.9). A lean may only change when a plant ticks or a gust arrives. Quote the
**whole frame**, not the sub-phase: there is a case on record where removing
91% of a pass's work made the frame *slower*, because the cost relocated to
cache misses.

**Then break**, with the constant set from a seed sweep with headroom (plan
§2c), `powder_surcharge` replacing `supported_load` (the snow-on-a-branch
term, which is also ask 1's free demo scene — snow, sand and rubble already
fall and pile), and buckling per §3.3.

**And it needs an invalidation story it does not have.** The only
organism-side `schedule_structural_check` fires on a *distance* rise
(`plant.rs:3892`). Under a load criterion the thing that should re-open a
cell is a *load* rise — the crown grew, a rock landed, a gust blew. Growth
raises the moment and never raises `support`, and scheduling from the growth
path is separately forbidden (growth only adds material, and a `GrowingTip`
is expected to be transiently unsupported). So ask 1 currently has **no
trigger**.

**One line worth knowing:** `organism_structural_tick` gates on
`within_disturbance`, a constant `true` only at the default `F9` setting. At
LOCAL/TIGHT/NONE a self-weight failure with no nearby disturbance never
fires — ask 1 is switched off by a settings key.

---

## 5. Do not retry — measured this session

1. **Severed foliage as a `Solid`.** Scaffolds; leaves the tree standing dead
   in the shape it grew in (§3.1).
2. **`deadleaf` with `max_unsupported_span: 2`.** Fails the opposite way just
   as hard: settled box to **80% loose grain against a 76% baseline**,
   `deadwood` 557 → 941 as failing foliage takes the logs' support with it.
   Two settings of one knob, opposite failures — the knob is not the problem.
3. **Reinstating `bearing_moment` for `log`.** Crushes rather than topples
   (§3.2).
4. **Any new colour for a debris tier** (§3.6).

---

## 6. Traps that cost time here

- **A card that shows a day/night cycle cannot ask about colour.** Six frames
  spanning dawn to night differ in *ambient light*, and the owner read that as
  the material still changing. The confound the change existed to remove,
  reintroduced by the card. **Pin `daylight=1.0`.** The owner's own
  instruction: *"just a single side by side at noon. This would be better with
  an animated gif."*
- **A GIF needs its own review item.** `files` is the frame-sequence field, so
  a still and a gif in one item render as a 2-frame strip and the animation is
  lost. The tool says so; heed it.
- **Verdicts arrive on `review.py sync`, not on `get`.** A card read as
  unanswered for an hour was answered; the queue had not been pulled.
- **`git stash push <paths>` silently does nothing if the paths are already
  committed** — "No local changes to save", and the subsequent "before" render
  is actually the "after" arm. It produced identical numbers, which was the
  tell. Build a before-arm by editing the one line back, not by stashing.
- **Python regex over `rigid.rs` can hang** (catastrophic backtracking on
  `(?:///.*\n)*` against a 5,000-line file). Use line spans or plain string
  finds.
- **Inserting a function above another one steals its doc comment.**
  `settled_shade` went in between `settle`'s doc and `settle`, silently
  reattaching it. Caught by clippy's lazy-continuation lint and by nothing
  else.
- **The release suite now takes ~500–600s**, not 355s, so a 10-minute tool
  timeout is not enough. Run it in the background.

---

## 7. Standing owner rulings

Recorded verbatim because each one overturned a plan:

- *"I don't see why you should be changing the growth or the carbon budget
  systems directly. You are inventing physics and structural mechanics for
  them."* — **growth and the economy are out of scope.**
- *"It should work for all plants. The system needs to make grass bend and not
  break. It is ok if the grass gets a little smaller."* — **every plant
  participates.** (Note the tension with foliage's structural opt-out in §3.3:
  the opt-out is about the *cantilever* rule, and bending is a different
  mechanism. Do not resolve it by deleting the opt-out.)
- *"The leaves shouldn't turn to powder ever."*
- *"The color should not change at all."*
- *"most of the leaves should stay on the branch."*
- *"Definitely think about how to be performance efficient when implementing
  this."*
- On tree shape: the multi-stemmed sprawling forms **are** a real tree shape.
  An earlier claim that they were wrong was the author's aesthetics leaking
  into a measurement, and it was withdrawn.

---

## 8. Freshness

Written 2026-08-29 on `claude/tree-physics-destruction-l6eocs` at `723d940`,
0 behind `main`. Every figure was measured this session on this machine.
§4's bars are predictions; nothing in §4 has been built.
