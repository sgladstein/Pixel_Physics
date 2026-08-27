# Physical trees: sway, impact breakage, and a tree that falls over

**Status: design; §8's T1 stage has since been built** — a felled tree comes
down as pieces (`bbbd789`), and a hand tool cuts living wood (`43adeb6`,
`rigid::is_tool_target`). The rest is not built. **No engine code shipped
with this document when it was written.** The prototype quoted throughout was written on a scratch branch,
measured, and deleted; it is reproducible from §3.2 and §5.3 in about forty
lines and it is not proposed as shipping code.

**Audience:** whoever picks up lane S past package S1. Read `CLAUDE.md`
first — this assumes its ethos section as the acceptance standard — then
`Reports/design-philosophy.md` §0a, `Reports/felling-blockers.md` (its §1
premise is superseded; its §2 is not, and this report replaces it), and
`Reports/open-bugs-handoff.md` §D1.

**The brief, verbatim.** Review card `20260823T092247531Z-a33d82`, board
`felling`, answering the first felling GIF:

> "It reads as a tree disintegrating into dust. I am wondering if we should
> take a step back and plan something more ambitious. Eventually I would
> want trees to be physical in the world, be able to sway in the wind, have
> branches break off if a rock falls on it. We need a more real physical and
> partially rigid modeling. Thoughts? Don't just start doing this. we need
> to think about it."

---

## 0. Summary, stated first

1. **The dust is one wrong ladder, and the fix is measured, not argued.**
   A real fell severs 2,648 cells and promotes **45 of them** (1.7%) as
   pieces; the other 2,733 become single `deadwood` grains. Routed through a
   wood-scale fragment ladder instead of the rock one, the *identical* cut
   promotes **1,547 cells (58%)** as 39 pieces spanning 8 to 300+ cells, at
   no measurable frame cost. §5 and the A/B sheets in §5.4.

2. **The single highest-leverage line in this whole document is a
   neighbourhood.** `rigid::take_fragment` floods `NEIGHBOURS_4`; `Grow`
   places organism cells at eight. So every diagonal in a crown is a
   fragment boundary. Measured on the same cut: 4-connected gives 45 pieces
   averaging 23 cells; 8-connected gives 20 pieces averaging 57, with two
   over 256. This is `CLAUDE.md`'s own "a traversal must use the same
   neighbourhood the writer used", in a place nobody had looked. §5.2.

3. **Render-side sway is not free, and the integrator's hypothesis that it
   costs nothing is wrong by a factor of four.** Measured on a grown 8-tree
   stand at the shipped 512x320 viewport: a canopy that never stops moving
   costs **+8.0 ms/frame**, against a settled cost of 0.012 ms and a full
   redraw of 11.9 ms. That is 2.3x the ~3.5 ms "wind-revert class" kill bar
   `ascii` already prints, on half the 60 Hz budget. It is not the shear
   maths — it is that a grown canopy covers 60–66% of the screen and repaint
   costs ~76 ns/pixel. §3.

4. **Sway is still worth having, in exactly one form.** At one pixel per
   cell the smallest visible lean is a whole cell, so there is no such thing
   as subtle sway here; and this engine's wind is already *gusts*, not a
   breeze, because a steady field term is a measured do-not-retry. Those two
   facts point at the same design: a **discrete, gust-local lean** — the
   crowns inside a passing squall jerk downwind and spring back. Amortised
   ~0.11 ms/frame, with a +3.5 ms spike on two frames in twenty-six. §3.4.

5. **A tree falls over without any new rotation, and it must, because free
   rotation is a recorded do-not-retry twice over.** Cut the severed trunk
   into a handful of segments, promote each as its own body, and give
   segment *k* a horizontal velocity proportional to its height above the
   stump. The stack shears; the crown travels sideways while the butt stays
   near the cut. That is what toppling looks like at this resolution and it
   needs nothing `ChunkBody` does not already have. §6.2.

6. **`spring.rs` cannot serve breakable joints and nothing here asks it
   to.** It is hydrology — springs and drains, water crossing the plane of
   the world. There is no mechanical spring in this engine, and §7.6 says
   why adding one is the sim-side sway cost with extra steps.

7. **What `BodyCell` is missing is two bytes.** An organism id, so a landed
   tree piece rots as deadwood instead of re-rasterizing as inert `wood`
   (which is what the prototype does, and it is wrong), so the census can
   see plant mass in flight, and so `promote` can decline to schedule a
   structural check on tissue that is already leaving. `Cell` does not grow;
   there is no side table; there is no determinism trap. §5.5.

**Recommended order: T1 pieces → T2 topple → T3 impact → T4 sway → T5
torque.** Sway is fourth, not first, because it is the one item whose cost
is a *standing bill* rather than an event, and the three ahead of it answer
the complaint that was actually made. §8.

---

## 1. What the owner is reacting to, measured

`filmstrip scene=fell fell=7150 start=7150 every=39 count=4`, on
`claude/s1-felling-instrument` at `6905a49`:

| | |
|---|---|
| cells severed by the support check | **2,648** |
| became single `deadwood` cells | **2,733** (with the axe's own chips) |
| promoted as coherent pieces | **45 cells, in 4 bodies** — 1.7% |
| largest piece | 16–31 cells |
| peak bodies in flight at once | 4 |

`deadwood` is a `Powder` with `friction_angle: 38.0`. Two thousand seven
hundred grains of it do exactly what the engine's granular rules say they
must: they pile at their angle of repose. The final frame of `scene=fell` is
**a cone of sawdust**, and that is not a figure of speech — it is the same
shape `ascii`'s "angle of repose" scene draws on purpose.

So the complaint is precise and it is not about the *cut*. The cut works:
S1 measured living tissue 2,906 → 409, both drivers agreeing. What fails is
everything downstream of `structural::organism_structural_tick`'s last line,
which converts **one cell** and reschedules its four neighbours. There is no
piece anywhere in the pipeline, so no piece can come out of it.

This is `design-philosophy.md` §0a's second named failure — "a uniform
dissolve to powder" — arriving verbatim, in a subsystem that was warned
about it in writing.

---

## 2. The constraints, hoisted to the front because they kill options

Four, all load-bearing. Every option below is priced against them, and where
one bites, it is named.

**2a. Same-build determinism is required** (`PLAN.md`, reversed with the
owner; `emergent-world-architecture.md` §8). Nothing in this document needs
a constraint solver, which is the main thing this constraint usually kills.
Where it does bite: any new flood over a severed region must iterate a
sorted list, not a `HashSet`. `fracture_with_impulse` already sorts its
seeds (`remaining.sort_unstable()`); the 8-connected component walk §5.2
proposes must do the same. Called out because it is the exact shape of
issue #7's live violation.

**2b. Bodies step serially, outside the checkerboard**
(`coupling-research.md` §4). A body spanning two same-parity chunks would
write to both and break `parallel.rs`'s write-disjointness proof. A tree
fall promotes tens of bodies at once — the prototype peaked at 22 in flight
— and every one of them steps in `rigid::step_chunk_bodies`'s serial phase.
This is where they must stay. Measured consequence in §5.4: on this scene it
cost nothing, and that is one scene, not a licence.

**2c. Chunk sleeping is the performance model.** A trunk holds no active
sites, so a forest sleeps. Measured on the river-cost scene at 512x320: a
world with **0** awake chunks runs at 1.29 ms mean, and one with **13** awake
runs at 2.87 ms — a standing bill of +1.58 ms against a pre-registered 2.0 ms
bar. Any proposal that keeps canopy chunks awake is paying that bill on every
forested scene forever. §3.5 is where this kills sim-side sway.

**2d. The dirty-rect render skip is worth 11.9 ms.** Measured on a grown
8-tree stand at 512x320: full redraw **11.948 ms mean / 14.272 ms worst**;
the same stand settled, with the skip working, **0.012 ms mean**. That is
the number to quote at anything that proposes to keep a canopy dirty. §3 is
entirely about this.

One more that is not a constraint but a fact about the medium, and it
reshapes §3 completely:

**2e. One cell is one pixel, so displacement is quantised to whole cells.**
There is no sub-pixel sway. A lean is zero cells or one cell; below that it
does not exist, and at 60 Hz a one-cell oscillation is shimmer, not motion.
Every "gentle continuous sway" design in this space is really asking for a
discrete lean and does not know it.

---

## 3. SWAY

### 3.1 The hypothesis, stated fairly

From the integrator, and worth stating in its strongest form: *per-column
offset from the wind field times height-above-anchor; zero physics, zero
wake cost.* The sim never learns the tree moved; the renderer samples the
grid at a sheared coordinate; chunks stay asleep; `anchor_support` stays
valid; nothing schedules a structural check.

Every clause of that is true except the cost, and the cost is not in the
part the hypothesis is about.

### 3.2 What it actually costs

The shear maths is free. **The repaint is not.** A cell whose drawn position
moves must be repainted, and so must the cell it vacated, and the renderer's
unit of repaint is a rect.

Measured with a throwaway probe on `PlantScene { trees: 8 }` at 512x320
(the shipped viewport, `app::WIDTH`/`HEIGHT`), grown 20,000 frames through
the real `App::update` phase order and then let go quiet — **28,421 plant
cells, 14,939 wood and 10,136 leaf**, canopy bounding box x 14..493,
y 75..233:

| what the renderer is asked to repaint | mean | worst |
|---|---|---|
| A. everything, every frame (`force_full`) | **11.948 ms** | 14.272 ms |
| B. nothing — settled, skip working | **0.012 ms** | 0.026 ms |
| C. every chunk holding a plant cell, every frame | **7.997 ms** | 9.302 ms |
| D. the union rect the renderer would really build | **8.475 ms** | — |
| E. one gust-width band (78 cells) over the canopy rows | **3.08–3.87 ms** | — |

**C is the sway hypothesis, priced: +8.0 ms/frame.** Against `ascii`'s own
recorded kill bar for a standing cost — *"kill bar: wind-revert class ~3.5
ms; pre-registered 2.0 ms"*, printed by the river-cost scene — that is 2.3x
the number that got the steady wind field reverted, and it is half the 16.6
ms a 60 Hz frame has.

Three things make it robust rather than an artifact of how it was measured:

- **It is not chunk granularity.** Per-tree bounding boxes come to
  **108,538 px of 163,840 — 66.2% of the screen**, *worse* than the 60.0%
  the 24 canopy chunks cover, because grown crowns interleave and their
  boxes overlap. A multi-rect renderer would repaint more, not less.
- **The renderer unions anyway.** `render.rs` folds every body rect into
  **one** dirty rect (`dirty = Some(d.union(*r))`). Row D is that union, and
  it is the same 8.5 ms.
- **Cost is linear in pixels at ~76 ns each** (11.948 ms / 163,840 px), and
  every arrangement above lands on that line. Nothing about how the dirty
  region is described changes what it costs to fill.

### 3.3 The three cheaper shapes, and what each buys

**S-a. Reduce the cadence** — repaint the canopy every 4th frame. Amortises
to 2.0 ms and does not move the *worst* frame at all, which is the number
`CLAUDE.md` says to quote. Rejected as a headline fix; kept as a modifier.

**S-b. A cell-granular repaint list.** Repaint only the cells that moved and
the cells they vacated, not a rect around them. Bounded below by
2 x (cells swayed) x 76 ns, and realistically worse because scattered writes
lose the linear-scan cache behaviour the 76 ns figure was measured with.
Foliage only: 10,136 leaf cells → 20,272 px → **≥1.5 ms**, probably 3–4.
This is the only version that could scale to a forest, and it is **a change
to the renderer, not to trees** — no such path exists today. Recorded as the
thing that would have to be built if §3.4 turns out not to be enough.

**S-c. Sway only where the wind actually is.** Row E: one gust-width band
costs 3.1–3.9 ms — still at the kill bar *if repainted every frame*.

### 3.4 Recommendation: a discrete, gust-local lean

Three separate facts converge on one design, and the convergence is the
argument.

1. **§2e**: at one pixel per cell the smallest visible sway is one whole
   cell. A continuous oscillator has nothing to express below that.
2. **This engine's wind is already gusts.** A steady field forcing term was
   built, measured at a permanent **3.55 ms/frame on every scene**, and
   reverted; `dead-ends.md` carries it four times over and `weather::gust`'s
   own doc is written around it. What exists is a bounded dipole impulse,
   `GUST_RADIUS = 26`, fired every `GUST_INTERVAL = 26` frames while
   `|wind| > GUST_THRESHOLD = 0.45`. Measured over 288,000 frames at the
   default seed, that threshold is exceeded **41.6% of the time**.
3. **A gust is local.** Its reach is ~78 cells with the dipole's lead — row
   E's band, not the whole stand.

So: when a gust passes, the crowns inside it lean one to three cells
downwind, held for the gust's duration, and spring back. The repaint
happens **on the frames the lean changes**, which for a discrete lean is two
per gust rather than sixty.

Priced from row E: 2 x ~3.5 ms per 26 frames = **0.27 ms/frame while windy,
0.11 ms/frame amortised over the 41.6% duty cycle.** The worst frame moves
by +3.5 ms on two frames in twenty-six, and that must be said out loud
rather than hidden in a mean — it is a real spike and it is what the stage's
cost bar is set against.

Two riders:

- **Sway must not feed growth.** `organism::wind_lean_dir` already leans
  growth off the field, direction-only at fixed magnitude, and
  magnitude-scaling it is a recorded dead end (a blast shockwave dominated
  the growth formula outright). A sway that wrote back into the lean would
  rediscover that. Sway is a *display* of wind, never a second input to it.
- **The lean must be a pure function of `(seed, frame, position)`.**
  `weather::at` already is; the field is not necessarily converged at any
  given frame. Deriving the lean from `weather::at` rather than from
  `field_at_bilinear` keeps it deterministic and keeps it working on a
  scene whose field is asleep.

### 3.5 Sim-side movement: dead on arrival, and here is where it bites

The brief asks for this to be evaluated rather than dismissed. It is
dismissed, on four independent counts, any one of which is fatal.

1. **It wakes the forest permanently.** Plant cells never move in the CA
   sweep — `wood.ron` says so in its first paragraph, and a trunk holding no
   active sites is *why* a forest sleeps. Moving them wakes every canopy
   chunk: **24 of 40** in the measured stand. The nearest measured analogue
   (§2c) puts 13 awake chunks at +1.58 ms/frame standing. This is the
   constraint `CLAUDE.md` names as hard, not a tiebreaker.
2. **It invalidates the support field every frame.** `plant::anchor_support`
   is a per-organism Dijkstra over the organism's whole cell list, run once
   per `ORGANISM_TICK_INTERVAL` (45 frames). Geometry that changes every
   frame means running it every frame: **45x the current rate, over 28,421
   cells** in the measured stand.
3. **It fires the amputation landmine thousands of times a second.**
   `anchor_support` schedules a structural check whenever a cell's support
   distance *rises* — and a leaning tree raises distances by construction.
   `CLAUDE.md`'s own gotcha: a structural check scheduled mid-organism
   converts crown to deadwood; the measured precedent is a stand going from
   20,213 living cells to 772 from **one** such check.
4. **It buys nothing the render version does not.** The player cannot tell
   whether the leaf that moved moved in the grid. Nothing reads a swaying
   tree's cell positions except the renderer.

Verdict: **do not build sim-side sway.** If a later mechanic genuinely needs
a tree's *physical* position to move — a trunk you can push over by hand,
say — that is the topple pipeline in §6, which is an event, not a standing
cost.

---

## 4. IMPACT BREAKAGE

*A rock lands on a branch; the branch comes off as a piece.*

### 4.1 What already exists, and what the gap actually is

**The load half is half-built, and the half that is missing is the half the
owner asked for.** `structural::organism_structural_tick` computes

```text
effective_span = max_cantilever_reach x wood_density - supported_load / LOAD_PER_SPAN_UNIT
```

so a branch with weight piled on it breaks at a shorter reach than a bare
one. That is real and it is the right shape. But `supported_load` counts
**cells standing in the grid above the branch** — and a falling rock is a
`ChunkBody`, lifted *out* of the grid for the whole of its flight. It
contributes exactly zero until it has already landed and come to rest.

So today a rock can only break a branch by *sitting* on it. There is no path
from "was hit hard" to "broke", which is `design-philosophy.md` §0a's third
named failure — no verb behind the effect — arriving for a second subsystem.

**The impulse is already computed and already thrown away.** `ChunkBody`
records `peak_speed` precisely because the velocity at rest has been damped
by the collisions that stopped it. `rigid::settle` writes
`LANDING_PRESSURE x sqrt(cells) x speed` into the field. That product *is*
the delivered energy of the impact, and nothing structural reads it. This is
the same shape as `explosion::probe_confinement` computing `RayResult.cost`
and discarding it (§D3), and the same shape as
`prior-art-destruction.md` §3.5's "drive `size_bias` from delivered surplus,
not brush radius."

### 4.2 The pipeline, in five parts

1. **Deliver the impact as a transient load.** At the contact cells, add a
   decaying term to `supported_load` — sized from the landing impulse
   already computed, decaying over a few `STRUCTURAL_TICK_INTERVAL`s. A
   heavy rock at speed shortens the effective span sharply for a moment; a
   pebble does not. This is `prior-art-destruction.md` §1.5's Chaos "break
   damage propagation" in miniature: damage arrives at an event, is spent,
   and leaves nothing standing to keep correct.
   - **Where to put the state.** Not on `Cell` (`Cell::flags` is full, 8/8).
     Not in a position-keyed side table (`plant-substrate-v2-design.md` §3f
     rejected exactly that: a determinism trap and a leak shape). It belongs
     on `OrganismCell`, beside `support`, which is a maintained per-cell
     sidecar that already exists and is already walked once per organism
     tick.
2. **Fail the section, not the cell.** The failing branch cell is the
   *neck*; what should come off is everything it was holding. The support
   Dijkstra already answers this and the answer is free: once the neck is
   gone, `anchor_support` marks every cell downstream `u16::MAX`. Take the
   8-connected component of `u16::MAX` cells as the region. §5.2.
3. **Break the region on a wood ladder.** §5.
4. **Carry organism identity through promotion.** §5.5.
5. **Leave the rest of the tree standing.** The severed limb's neighbours
   are rescheduled, `anchor_support` re-runs, and cells still reaching a
   root keep their distance. This already works — it is what S1 measured
   when a stump of 409 cells survived a fell.

### 4.3 What `MAX_BODY_CELLS` means for a tree, and the recommendation is to leave it alone

`MAX_BODY_CELLS = 400` is global, and the temptation is to make it
per-material so a tree can fly as one piece. **Don't.**

- A 2,400-cell crown wants to be six to ten pieces, not one. One indivisible
  slab is `design-philosophy.md` §0a's *first* named failure — the
  all-or-nothing outcome — approached from the opposite side. The mockup
  capped fragments at 512 and produced two pieces over 256, which is a bole
  and a major limb; nothing in the sheet argues for more.
- The cap must bound *work*, never gate *whether* something happens
  (`CLAUDE.md`, written twice, and `dead-ends.md` §414 records the third
  instance). `fracture_failing_region`'s own doc records what happened last
  time a size cap sat on the decision: the bigger the collapse, the more
  certain it dissolved.
- 400 vs 512 is not a difference the eye is judging. Say so rather than
  inventing a knob nobody can set from evidence.

What *does* need to change is the ladder's **floor**, not its ceiling. §5.1.

---

## 5. THE FRAGMENT LADDER, and the prototype that settles it

### 5.1 Why the rock ladder cannot produce a log

`rigid::fracture_with_impulse` draws each fragment's target as

```text
target = 1 << (1 + rng.below(fragment_rungs) + size_bias)
```

`wood.ron` sets no `fragment_rungs`, so it takes the default 5 → targets
uniform over {2, 4, 8, 16, 32}; `size_bias` caps at 2, so the very best a
blow can do is {8 … 128}. Anything under `MIN_BODY_CELLS = 8` becomes rubble
rather than a body. **Two of the five rungs are below the promotion floor
before shape is even considered**, which is why a fell promotes 1.7% of what
it severs.

The shape of the ladder is right — log-uniform over the exponent is the
heavy-tailed distribution fragmentation actually has, and
`prior-art-destruction.md` §2.4 confirms it is the mainstream answer. What
is wrong is where it starts. A tree wants the **base moved**, not more
rungs.

**Recommendation: add `fragment_floor` beside `fragment_rungs`**, the
exponent the ladder starts at, defaulting to 1 so every existing `.ron`
behaves exactly as it does today. `wood.ron` sets 5 → targets
{32, 64, 128, 256, 400}. That is `design-philosophy.md` §2a's own test for
when a constant becomes data, and it is precisely the axis
`prior-art-destruction.md` §2.4 says Red Faction authors per material.

### 5.2 The neighbourhood, which is the biggest single lever here

`take_fragment` floods `NEIGHBOURS_4`. `Grow` places organism cells at
**eight**. So a crown, which is mostly diagonal twigs, is cut at every
diagonal before the size ladder gets a say.

Measured on one cut, wood ladder held constant, only the flood changed:

| flood | bodies | cells promoted | size distribution |
|---|---|---|---|
| 4-connected | 45 | 1,051 | 23x(8–15), 15x(16–31), 4x(32–63), 2x(64–127), 1x(256+) |
| 8-connected | 20 | 1,146 | 8x(8–15), 5x(16–31), 3x(32–63), 2x(64–127), **2x(256+)** |

Fewer pieces, larger pieces, and *more* mass surviving as pieces rather than
falling through to grit. Mean fragment size 23 → 57 cells.

**It must be 8-connected only for organism material.** Rock is 4-connected
on purpose and the reason is documented and tested:
`label_component`'s `diagonal_only_contact_does_not_connect_two_components`
— two blobs touching at a corner are not one physical body. The rule is not
"eight is better", it is `CLAUDE.md`'s "a traversal must use the same
neighbourhood the writer used", and the writer differs by material. Gate it
where the data already is: a `MaterialDef` field tested at the call site,
never an `id_of("wood")` string hash on a per-cell path.

The same defect is one function away in `structural.rs`:
`schedule_organism_neighbours` also walks `NEIGHBOURS_4`, so a cascade
through a crown skips diagonally-attached tissue. Filed in §9, not fixed.

### 5.3 Leaves are not wood and must not be on the ladder

A crown is roughly a third foliage by cell count (10,136 of 28,421 in the
measured stand). `leaf.ron` is a different material with different physics
on four numbers, and it should come off as **scatter**, not as a slab: down
the existing `shatter_to_rubble` path to `litter`, which the decay and ant
layers already read.

That gives the three-tier outcome the ethos asks for and which
`prior-art-destruction.md` §2.4 says every shipped game stacks rather than
derives: **a log or two, a spread of branch-scale pieces, and a scatter of
leaf litter.** Three mechanisms, one per size class, no single physical
model pretending to produce a distribution.

### 5.4 The prototype, and what it shows

Written on a scratch branch, run, and deleted. `organism_structural_tick`,
on a detached cell, takes the 8-connected `u16::MAX` component instead of
the one cell, splits leaf from wood, sends leaf to `shatter_to_rubble` and
wood through a ladder of {32 … 512} with an 8-connected flood.

Identical scene, identical seed, identical cut —
`filmstrip scene=fell fell=7150 start=7150 every=39 count=4`, both arms
severing exactly **2,648 cells**:

| | today | prototype |
|---|---|---|
| severed by the support check | 2,648 | 2,648 |
| promoted as pieces | **45 cells / 4 bodies (1.7%)** | **1,547 cells / 39 bodies (58%)** |
| size distribution | 3x(8–15), 1x(16–31) | 17x(8–15), 9x(16–31), 5x(32–63), 4x(64–127), 2x(128–255), **2x(256+)** |
| became `deadwood` grains | 2,733 | 1,912 |
| peak bodies in flight | 4 | **22** |
| worst frame (whole run) | 56.66 ms | 53.30 ms |

**The frame cost is nil**, and the reason is worth stating so nobody quotes
the number wrong: both runs' worst frame lands at frame 6,838, which is
*before* the cut at 7,150 — it is growth cost, identical on both arms
because the arms are byte-identical until the axe swings. The piece path
added nothing measurable to the worst frame **on this scene**. One scene is
not a licence, and T1's cost bar in §8 asks for `ascii` and `seedsweep.sh`.

**Posted for judgement**, blind, board `felling`, card
`20260823T155425084Z-9a7951` — twenty scrubbable frames per side, both arms
severing the same 2,648 cells, with the counts above in each item's `meta`.

**What the sheets show.** Today: crown → brown cloud → a smooth cone, which
is the angle-of-repose pile `deadwood`'s `friction_angle: 38.0` guarantees.
Prototype: the crown holds recognisable branch structure for several tiles,
a large angular mass slides clear, and the final silhouette is lumpy and
irregular rather than conical.

**And it is honest about not being finished.** The prototype still sends
1,907 cells to grit — the leaves by design, plus wood fragments that fell
below `MIN_BODY_CELLS` after larger neighbours were claimed — so the fallen
tree still ends as a mound with pieces in it rather than as a log lying on
the ground. That residue is not a tuning failure. It is §5.5.

### 5.5 What `BodyCell` is missing, and why the landed log is the wrong material

`rigid::settle` writes each landed cell as `Cell::new(cell.material,
cell.shade)` — no organism id, no aux, deliberately unattached. So a
promoted tree piece lands as **inert `wood`**: it does not rot, it does not
feed the litter layer, the felling census cannot see it, and `decay.rs` will
never touch it. That is what the prototype produces and it is wrong.

**`BodyCell` needs an organism id.** The struct is
`{dx: i32, dy: i32, material: MaterialId, shade: u8}`; a `u16` is two bytes
into existing padding. This is *not* growing `Cell` (whose flags are full at
8/8, `load-model-handoff.md`), and it is not a position-keyed side table
(rejected, `dead-ends.md` §572). It buys three things at once:

- `settle` can write the piece as dead tissue rather than live wood;
- the census can report plant mass in flight — S1's own line already reads
  *"bodies carrying plant material 0 of 0 body cells"* and that zero is
  exactly the number T1 has to move;
- `promote` can decline `schedule_structural_check_around` for organism
  cells that are already leaving, which is the amputation landmine firing
  from inside the fall (`felling-blockers.md` §3 step 4 predicted this).

**And `deadwood` is the wrong material for a log.** It is a `Powder`. A
powder piles at its friction angle; a log lies where it fell. Recommend a
new `log` material — `kind: Solid`, `breaks_into: "deadwood"`, flammable and
decayable, climbable — as the *piece* tier, with `deadwood` remaining the
*grit* tier. That is `design-philosophy.md` §2a's test met squarely: the
physics genuinely differ on numbers that already exist (it does not flow, it
can be stood on, it burns and rots on its own schedule). Note the
`include_str!` gotcha — a new material needs a rebuild, not an F5, and a
sweep that edits the `.ron` and re-runs a prebuilt example measures nothing.

---

## 6. FELLING AND TOPPLE — the "partially rigid" core

### 6.1 The constraint that decides the whole shape

**Free (arbitrary-angle) rotation of a body's grid pose is a recorded
do-not-retry, twice.** `dead-ends.md` carries it for `ChunkBody`
(*"a cell grid cannot represent a slab at 37 degrees without resampling, and
resampling a rotating body is where the classic re-rasterization leaks and
holes come from"*) and again for creature bodies (D1: *"rotation on a grid
is unsolved"*). Quarter turns are exact — an offset maps to an offset with
no interpolation — so a tumbling chunk can never gain or lose a cell.

So the answer to "rotation from torque, not from speed" cannot be "give
`ChunkBody` a real angle and integrate it". It has to be either a fall that
needs no rotation at all (v1) or a **split between the drawn pose and the
collided pose** (v2). Both are below.

What is genuinely wrong with today's rotation, and `felling-blockers.md` §2
called it correctly: `spin` accrues from *speed* (`SPIN_PER_SPEED`), so a
just-cut trunk with no speed accumulates no spin and falls flat; and a
quarter-turn is gated on the rotated shape fitting, so a 30-cell trunk needs
30 cells of side clearance before it may turn at all.

### 6.2 v1 — the segment shear. Cheapest thing that reads as a tree falling

**Recommended as T2, and it needs no new rotation whatsoever.**

A single `ChunkBody` has one `(vx, vy)`. One velocity cannot lean. But
*several stacked bodies* can, and the lean falls out for free:

1. Cut the severed trunk-and-crown into **N segments along its own length**,
   N sized so each segment lands in the 100–400 cell band (so N ≈ 3–6 for a
   2,400-cell tree). The cut lines are the fragment ladder's, restricted to
   run across the bole rather than anywhere.
2. Promote each segment as its own body.
3. Give segment *k* a horizontal velocity `vx = omega x h_k`, where `h_k` is
   the segment's centroid height above the stump and `omega` is an angular
   rate derived once from the whole piece's overturning moment about the
   cut. Sign from which side of the stump the piece's centroid lies — the
   same question `load.rs` already answers for rock.
4. **Turn `spin` off for these bodies.** A quarter-turn snap of a 300-cell
   bole mid-fall is not a topple, it is a glitch; and because it is gated on
   clearance it would mostly refuse and occasionally fire, which is the
   worst of both. v1 suppresses it explicitly rather than leaving it to
   chance.

The stack shears: the crown travels sideways while the butt stays near the
cut, the segments separate as they go, and the tree comes apart as it falls
— which is what felled trees do. **At 1 px/cell a shear and a rotation are
very hard to tell apart**, which is the whole reason this is worth trying
before v2.

Three artifacts to state up front rather than discover:

- **Bodies pass through each other.** `clear_or_displaceable`'s doc is
  explicit that this is deliberate and that changing it "is a separate
  change with its own consequences for a collapse with two dozen pieces in
  flight". Adjacent segments will interpenetrate at the joints. At 3–6
  segments the seam is a few cells; **measure it on the GIF before deciding
  it needs joints.** Do not pre-emptively build collision for it.
- **`settle` loses ~10% of a body's cells** (`open-bugs-handoff.md` §1c:
  80 cells in, 72 out on a 40x2 raft in plain air). A 2,400-cell tree loses
  ~240 cells at the moment it lands — the log arrives visibly thinner than
  it left. This is pre-existing and has a *withdrawn* fix on record (it cost
  `scene=ligament` 18.1 → 86.6 ms against a 60 ms bar). T2 makes it more
  visible, which is a reason to schedule it, not to bundle it. It goes on
  T2's acceptance list as a counter, not a fix.
- **A falling body does not know it is a tree.** It will drop through a
  neighbouring canopy without touching it, because `clear_or_displaceable`
  treats `Plant` as a real obstruction only when it is *in the grid* — which
  it is, so in fact a falling trunk will be *stopped* by the neighbour's
  foliage. Whether that reads as caught-in-the-canopy or as stuck-in-mid-air
  is a GIF question. Flagged, not designed.

### 6.3 v2 — torque about a hinge, and exactly what it asks of `ChunkBody`

Only if T2's verdict is "it falls, but it slides rather than turns."

- `ChunkBody` gains a stored **hinge point**, a visual angle `theta: f32`
  and an angular rate `omega: f32`, integrated from `torque = M x arm` about
  the hinge — the same two sums `load.rs` already computes and the same
  quantity that decided the piece should fall in the first place.
- **The collision and rasterization pose stays quarter-turn exact.** `theta`
  drives *rendering only*; the body snaps its grid pose at quarter-turn
  boundaries exactly as today. This is the same separation as render-side
  sway in §3, and it is the only way to get a continuous-looking rotation
  without reintroducing the leak the dead-end record forbids.
- **Its honest limit is thin pieces**, and this must be measured before it
  is promised. The drawn and collided poses disagree by up to ±45°, which
  for a 3-cell-thick trunk is a couple of cells and for a 40-cell crown is
  not. A tree that visibly passes through the ground before snapping is
  worse than one that shears.
- Cost is **unmeasured**. `draw_chunk_bodies` draws at `cell_position` — an
  integer offset — so a rotated draw is a new path, and a rotating body's
  dirty rect is its rotated bound, which grows. Neither has been priced and
  this report will not pretend otherwise.

### 6.4 What happens to the organism

- **The stump keeps the identity.** S1 measured 409 cells standing after a
  fell (shoot 1, root 406). It keeps its `organism_id`, its root system and
  its sidecar, and it is exactly the population lane P's resprout work
  (P5/D4) has to come out of. Nothing here needs building — it already
  happens.
- **The fallen part leaves the organism, for free.** `promote` writes
  `Cell::EMPTY` over each cell it lifts, and `World::set` already reindexes
  the organism's cell list, so the cells drop out of `OrganismState::cells`
  today. The only thing missing is that the *body* forgets what it was,
  which is §5.5.
- **A landed log rots.** With `BodyCell.organism_id` and a `log` material,
  `settle` writes dead tissue, `decay.rs` picks it up on its existing damp
  and dry paths, and it becomes `litter` on the schedule the ant and
  litter layers already read. No new subsystem: the fallen tree joins the
  ecology that is already running.
- **Deadwood standing in the crown is a separate thing and stays.**
  `break_free` remains the right outcome for a *twig* that loses its anchor
  — one cell, one grain. What changes is that it stops being the outcome for
  a two-thousand-cell crown.

---

## 7. What we are NOT doing, and why

1. **Free rotation of a body's grid pose.** Recorded do-not-retry twice
   (`dead-ends.md`, `ChunkBody::spin`'s own doc, creature decision D1). v2
   splits drawn from collided pose instead; the grid pose stays exact.
2. **A steady wind field term.** Measured at a permanent **3.55 ms/frame on
   every scene in the engine** with six field tests failing, reverted, and
   recorded four separate times. §3.4's gust-local lean is not a quieter
   version of it — it writes nothing to the field at all.
3. **`rapier2d` or any constraint solver.** Its `enhanced-determinism`
   feature cannot combine with `parallel` (`coupling-research.md` §0.2)
   against a determinism requirement `PLAN.md` reversed to *required*. And
   what a falling tree needs is gravity, a fit test and a settle rule.
4. **Sim-side sway.** §3.5, four independent fatal counts.
5. **Per-material `MAX_BODY_CELLS`.** §4.3. The cap must bound work, never
   gate whether something happens, and 400-vs-512 is not what the eye is
   judging.
6. **A mass-spring tree.** `spring.rs` is hydrology — springs and drains,
   water arriving from behind the plane of the world — and there is no
   mechanical spring anywhere in this engine. Adding one means a per-frame
   integrator over every cell of every tree, which is §3.5's standing bill
   with a solver on top, for a visual result §2e says is quantised to whole
   cells anyway.
7. **Making bodies collide with each other.** Wanted eventually, out of
   scope here: `clear_or_displaceable`'s doc says it changes every collapse
   with two dozen pieces in flight, and T1 alone puts 22 in flight.
8. **Fixing the ~10% landing loss (§1c).** Pre-existing, with a withdrawn
   fix on record that cost 18.1 → 86.6 ms against a 60 ms bar. T2 makes it
   more visible; that schedules it, it does not bundle it.
9. **A physically-derived debris distribution.** Nobody does this
   (`prior-art-destruction.md` §2.4) and it is not a gap. Three authored
   tiers — log, branch-piece, leaf scatter — is the mainstream answer and
   the one the ethos asks for.

---

## 8. The staged path

Every stage is judged by a GIF on the review queue with its discrete event
count in the card's `meta` — because a collapse has already been read as
"chunks are working" from a picture whose body count was zero for the whole
run.

**T1 — the severed piece comes down as pieces.** The dust fix.
`fragment_floor` on `MaterialDef`; 8-connected flood for organism material;
leaves off the ladder to litter; `BodyCell.organism_id`; the `log` material.
- *Judged by:* the §5.4 A/B, re-rendered from shipped code.
- *Bar:* promoted share of severed mass **≥ 50%** (prototype 58%, today
  1.7%), at least one piece ≥ 256 cells, `crumbled to grit` reported beside
  it, and `bodies carrying plant material` no longer 0.
- *Cost bar:* `ascii`'s organism scene mean within noise of `main`
  **re-measured in the same session** (today: worst 55.5 ms, mean 3.579 ms
  over 12,000 frames with 76 live organisms); `seedsweep.sh` unchanged,
  which it should be exactly, since the ladder is gated on material.

**T2 — the tree falls over.** Segments, shear velocity, `spin` suppressed.
- *Bar:* the crown's centroid ends **≥ one tree-height** horizontally from
  the stump. Counters printed beside the GIF: segments promoted, peak bodies
  in flight, and **cells lost in `settle`** (§1c, made visible).

**T3 — a rock breaks a branch off.** The delivered-impulse term, feeding
T1's consequence pipeline.
- *Build the instrument first.* There is no scene that drops a rock on a
  branch. `scene=limbstrike` before the mechanism — S1's own lesson, and
  `felling-blockers.md` §3 step 0 before that.
- *Bar:* the limb comes off as ≥ 1 body; the trunk survives; and **the same
  rock dropped on the trunk does not fell the tree** — the negative case,
  which is the one that tells a graded outcome from a delete button.

**T4 — sway.** Gust-local discrete lean, render-only, off `weather::at`.
- *Judged by:* a GIF in a pinned windy epoch.
- *Bar:* worst render frame over a **grown** stand, measured the way §3.2
  measures it. Hard gate at the pre-registered **+2.0 ms** standing bill;
  kill at the **3.5 ms** wind-revert class. The +3.5 ms two-frames-in-26
  spike is expected and must be reported, not averaged away.

**T5 — torque rotation (v2).** Only if T2's verdict is "slides, doesn't
turn". Costs are unmeasured; T5 begins by measuring them.

**T6 — wind-throw.** A storm knocks a tree over: root anchorage against the
overturning moment, and slenderness as an independent failure mode. §11.
- *Build the instrument first.* There is no scene that puts a grown stand in
  a pinned storm, and `weather::at` is a pure function of `(seed, frame)`, so
  a windy epoch has to be *found* rather than asked for. `scene=windthrow`
  before the mechanism — S1's lesson, and T3's already.
- *Bar:* the four rungs of §11.3 are **separately visible in one sweep** —
  uprooted, snapped, delimbed, shed-only — with the count of each printed
  beside the GIF. A run in which every failure is the same rung has not
  cleared this bar whatever its total, because a graded outcome is the whole
  point of the stage and an all-or-nothing one is the failure
  `design-philosophy.md` §0a names first.
- *Cost bar:* **zero standing cost.** The evaluation runs only for organisms
  a gust actually overlaps, at organism-tick cadence, off quantities
  `anchor_support` already produced. `ascii`'s organism scene mean within
  noise of `main`, re-measured in the same session.

**T6 is placed after T2 and not before it** because it is a *trigger* on the
fall machinery, not a fall of its own: until a severed tree comes down as
pieces (T1) and travels sideways as it goes (T2), a wind-throw would show the
player the same cone of sawdust the owner already rejected, from a new cause.
Its economy half is a different matter and does not wait — §11.6.

**Why sway is fourth.** It is the only item whose cost is a *standing bill*
rather than an event, it is the only one that needs a renderer change to
scale, and it is not what the owner's card was about. T1–T3 answer the
complaint that was actually made.

---

## 9. Findings filed rather than fixed

This was a design session; none of these were touched.

1. **`rigid::take_fragment` floods `NEIGHBOURS_4` while `Grow` places at
   eight.** Measured effect in §5.2: mean fragment size 23 → 57 cells and
   two pieces over 256 where there was one. The highest-leverage line in
   this report.
2. **`structural::schedule_organism_neighbours` walks `NEIGHBOURS_4` too**,
   so a cascade through a crown skips diagonally-attached tissue. Same
   cause, one function away, unmeasured.
3. **`organism.rs`'s `CellType::RootTip` doc is stale in its correction.**
   It says *"It does not do that yet, and this comment used to say it did"*
   about anchoring on roots — but `plant::is_structural_anchor` now keys on
   `CellType::RootTip` (and on root tissue touching wet powder), so the
   original claim is true again and the correction is not.
   **`Reports/felling-blockers.md` §4's "one correction to make regardless"
   is discharged by the plant-line merge**, and both places should say so.
4. **`felling-blockers.md` §2's two redesigns are superseded by this
   document**, and its §1 premise was already superseded by
   `open-bugs-handoff.md` §0d. Its §3 ordering still holds.
5. **`filmstrip` prints no census in `gif=1` mode**, so a GIF cannot carry
   its own counts and the house rule ("put the discrete event count in the
   card's `meta`") needs a second, non-`gif` run at the same span to source
   them. Cheap to fix and it is a footgun: the summary line
   `peak chunk bodies in flight at once` prints **0** on a gif run that in
   fact peaked at 22.
6. **`rigid::loosen_shell` still declines organism cells** — S1 left it
   deliberately, and it is T1's decision now: it governs whether a blast rim
   promotes wood into bodies, which is the same ladder question.

---

## 10. Freshness

Written 2026-08-23 against `main` at `95f0a0d`, with the felling instrument
read from `claude/s1-felling-instrument` at `6905a49` (PR #21, open). Every
figure in §1, §3.2 and §5.4 was measured in that session on this machine;
the render figures are at 512x320, which is the shipped viewport
(`app::WIDTH` / `app::HEIGHT`), not a stand-in. Re-measure any baseline in
the same session before reporting a regression against it — the worst-frame
figure for `ascii`'s organism scene has been observed at 49.7, 55.5 and 63.3
ms across three sessions on unchanged code, while its mean held at
3.17–3.58.

**Re-measured after merging `main`**, which moved 35 commits during this
session and touched `src/render.rs` — so the §3.2 figures were taken on a
tree nobody else had, and a baseline taken 35 behind is not a baseline. On
the merged tree, same stand to the cell (28,421 plant cells, 14,939 wood /
10,136 leaf, 24 of 40 chunks): full redraw **11.838 ms** mean, settled
**0.012 ms**, canopy dirty every frame **8.220 ms** mean / 9.218 worst, one
gust band **3.07–3.90 ms**. Every headline holds; the table in §3.2 keeps the
pre-merge run's numbers because that is the run the argument was built on,
and the two agree inside the spread.

---

## 11. Addendum, 2026-08-23 — wind-throw: roots as anchorage, slenderness as the second failure mode

**Design, nothing built.** Added by the plant-program integrator after this
report merged (PR #23), from a brief the owner gave in conversation rather
than on a card. The three scheduling and scope calls it raised were put to
the owner the same evening and **all three are decided** — §11.8 records them
in their decided form.

### 11.0 The brief, verbatim

> "One thing that I forgot to mention when we explored physical tree design
> [...] I also want roots to play a role in stabalizing the tree. if you
> have a weak root system, the tree can be knocked over in a wind storm or
> damaged. Additionally if the tree is very top heavy with a skinny trunk
> (regardless of root) that could cause the tree to fall over in a storm.
> Obviously there should also be costs to having large root systems and
> large trunks so we don't just push everything to large roots and trunks,
> but this could result in biomes with few storms to have smaller roots and
> thinner trunks because they are not needed. Thoughts?"

### 11.1 Why this closes a hole rather than adding a feature

**Root investment currently has a price and no benefit.** Lane P is about to
ship the owner's root-blob directive — a root cell not touching soil earns
nothing and costs something (`plant-implementation-split-2026-08-23.md` §4,
and card `20260823T163504317Z-3cef7b`). Taken alone that is a monotone lever:
a quantity with a cost and no counterweight has exactly one optimum, the
minimum, and an economy that is working correctly will find it and hold every
plant there. The visible result is not "roots got sensible", it is *one root
morphology everywhere* — the same complaint the owner already made twice
about the three architectural levers (§*Ask which pixels a lever moves* in
`CLAUDE.md`, and `plant-appearance-design.md`).

Anchorage is the counterweight. It is what makes root allocation a genuine
trade rather than a tax, and it is the reason this belongs *near* the root
economy rather than five stages downstream of it. §11.6.

**And it is the first destructive verb the world owns.** T1, T2 and T3 all
originate with the player — an axe, a dropped rock. Wind-throw and W2's
grassfire would be the two things that take a tree apart while the player is
standing somewhere else watching it happen. `design-philosophy.md` §0a's
third named failure is "no verb behind the effect"; this is the verb that
does not need a hand on it.

### 11.2 What already exists, which is most of it

| the quantity | where it already lives |
|---|---|
| **what counts as an anchor** | `plant::is_structural_anchor` — root tissue (a `reinforces_powder` material, or a `CellType::RootTip`) adjacent to a water-holding `Powder`, or *any* cell adjacent to a `Solid` |
| **how many anchors, and how wide they spread** | free from the walk `plant::anchor_support` already runs once per `ORGANISM_TICK_INTERVAL` (45 frames) — it enumerates the anchor set to seed its Dijkstra and currently throws the set away |
| **the overturning test** | `load.rs::bearing_moment`, and its reduction to "is the centroid inside the middle third" — the same question already answered for rock, against a base width |
| **stem width as a function of crown mass** | `thicken` / `pipe_ratio` (Shinozaki), gated on `leaf_count / stem_width > pipe_ratio` |
| **the storm** | `weather::gust` — a bounded dipole impulse, `GUST_RADIUS = 26`, fired every `GUST_INTERVAL = 26` frames while `\|wind\| > GUST_THRESHOLD = 0.45`, which is 41.6% of frames at the default seed |
| **the fall** | T2's segment shear. Wind-throw needs a *trigger*, not new falling machinery |

Two consequences worth stating plainly, because they change how much work
this is:

- **"How well rooted is this tree" is not a new subsystem.** It is a number
  recoverable from a walk that already runs, at no additional traversal.
- **"Top-heavy with a skinny trunk" is not something to author.** `thicken`
  already ties width to the leaf mass above it, so a slender tree is what
  happens when the crown flushes faster than the stem thickens — a shaded
  tree racing for height, or a good year following a bad one. Slenderness
  (height above the anchor plate ÷ stem width at the base) is one division
  away from quantities that exist. It should be *read*, never assigned.

### 11.3 The design call: two causes must produce two different-looking outcomes

The owner named two independent failure modes. If both render as "the tree
falls over", half of what was asked for is invisible on screen — and a binary
fall is `design-philosophy.md` §0a's all-or-nothing failure arriving in a
third subsystem. Grade it, and let the *cause* pick the rung:

| condition | outcome | what is on the ground afterwards |
|---|---|---|
| anchorage loses to the moment | **uproot** — the whole plant tips as one piece, the root plate lifting with it | a crater of disturbed soil, the butt in the air, no stump |
| well anchored, too slender | **snap** — the stem fails at its most-loaded section, well above the ground | a *rooted stump*, the top down as pieces |
| marginal | **limb off** — T3's delivered-impulse pipeline, driven by wind instead of a rock | one branch |
| ordinary gust | leaf and twig shed | `litter`, which the decay, ant and fire layers already read |

The distinguishing detail is the last column, not the physics: **the stump
tells the player which of the two things happened**, hours of play later.
That is legible feedback surviving past the event, which is what the ethos
asks for and what a fall animation alone cannot give.

The bottom rung is worth more than it looks. It fires in ordinary weather at
a 41.6% duty cycle, so the mechanic is something the player feels constantly
rather than once a season — and it feeds a litter layer that already has
consumers.

### 11.4 Do not add a third cost

The brief asks for costs on large roots and large trunks so the economy does
not push everything to both. **Those costs already exist and a third would be
double-charging:**

- roots: the root-blob economy P2 is re-deriving (interior root tissue earns
  nothing and respires);
- trunk: thickening is carbon that did not become leaf, and `pipe_ratio`
  already ties width to the crown it serves.

So the work here is *not* "add a price for anchorage". It is: make sure those
two prices are real, then add the benefit that makes paying them rational in
some places and wasteful in others. A separate anchorage cost would be a
knob nobody can set from evidence — `design-philosophy.md` §2a's test,
failed.

### 11.5 What did not exist, and the biome outcome depended on it

> **Discharged 2026-08-24 by package W4** (`claude/w4-wind-geography`, merged).
> This section is kept as written because the reasoning it records is what W4
> was briefed against and is still the constraint on anything that touches
> wind; the two sentences that are no longer true are corrected in place
> below. **Terrain-derived exposure exists on `main`.**

**Wind had no geography.** `weather::at(seed, frame)` takes no position:
`wind` is `channel(seed, frame, 3) * 2.0 - 1.0`, one value for the entire
world. Gusts fire *at* a location, and before W4 their strength was global
and time-only.

**Two corrections to what this section originally claimed**, both from
measurement made after it was written:

- It said flatly that there is no sheltered spot in this world. **Too
  strong.** The *field* already carried locality even though the driver did
  not: horizontal wind sampled across a sward at three instants read spreads
  of 0.0000 / 0.0387 / 0.0000, the middle one a `weather::gust` dipole
  sitting in the field. What was missing was *persistent* shelter, because
  nothing positional fed the driver — a hollow was calmer than a ridge only
  for as long as a gust happened to be somewhere else, which is not
  something a tree can grow differently in response to.
- It said "a sheltered valley grows slender trees and an exposed ridge grows
  squat ones" **cannot emerge today**. **After W4 it can.** Shelter is now
  persistent and terrain-shaped, so there is something for the biome
  divergence to vary against. Whether it *does* emerge is a separate
  question and is not yet measured — W3's two-patch divergence instrument
  (`examples/divergence.rs`, merged) is the thing that would answer it, and
  needs one arm on its `Axis` to be pointed at wind.

The obvious fix is a spatial wind field, and that is precisely the shape of
the term this engine already reverted: a steady global wind measured at a
permanent 3.55 ms/frame and recorded as a dead end four separate times (§7.2).
**The distinction that makes a cheaper version legal:** the reverted term
*wrote* pressure into the field every frame on every scene. What is wanted
here is a term that is *read*, by a handful of organisms, on the frames a
gust fires. Nothing is written and nothing is kept.

Cheapest shape that does the job: **exposure derived from terrain at gust
time** — open fetch upwind, and height above the local ground. No new field,
no new storage, no standing cost, and a pure function of terrain and
direction, so it stays deterministic.

**Built by W4, and one deliberate difference from what this paragraph
proposed.** This section suggested querying "only for the organisms a 26-cell
gust actually overlaps"; W4 queries once per gust at the strike column and
scales the whole dipole, which is strictly cheaper — one query per gust
rather than one per overlapped organism. The per-organism form is what T6
will want, reading the term at a tree's own position, and
`exposure(world, x, y, wind)` is already the point form that supports it.

Two things T6 inherits and must not re-derive:

- **The world is now globally calmer by design.** `MIN_GUST_SHARE` floors the
  scale and exposure tops out at 1.0, so the term can only ever *subtract*
  pressure — centring on neutral would let a ridge deliver ~1.5x the old
  impulse, and adding pressure is the exact failure that killed the reverted
  steady-wind term. Measured at 62.6% of the old total delivered strength
  over 517 gusts. Any wind-throw threshold tuned against pre-W4 gust
  strengths is tuned against a world that no longer exists.
- **The sampling trap.** `|wind| > GUST_THRESHOLD` holds 41.6% of the time
  and gusts fire one frame in 26, so most frames have no gust to be sheltered
  from at all. Sampling at an arbitrary frame can read flat and look like a
  dead mechanism, or read a gust's own dipole and credit it to terrain. W4's
  flat-preset control (mean 0.500, spread 0.000) is what separates those two,
  and T6 needs its own equivalent.

### 11.6 How the divergence actually arrives — plasticity before selection

Two mechanisms can produce "biomes with few storms grow thinner trees", and
the report recommends the second one first.

**Selection.** Heritable root and stem allocation, plus differential
survival, over generations. This is the answer the plant genome exists for
and it is where this should end up. It cannot go first: the axis it would
act on does not currently vary. The owner's own cards
(`3cef7b`, `6825a2`, `a227da`) record that the root-blob endpoint erased the
slot-5 root axis — selection cannot act on a lever whose settings all produce
the same plant, and lane B's root-differentiation re-renders are already
queued behind P2 for exactly this reason.

**Plasticity.** A tree that is repeatedly shaken puts carbon into root and
stem instead of height. This is real (thigmomorphogenesis: wind-exposed
trees grow shorter, more tapered, and more heavily rooted), it is **one
directive inside the economy P2 is already re-deriving**, and it produces the
divergence *within a single plant's lifetime*. That last point is what
decides the order: a playtest can see it, which means it can be judged by eye
on a card, which is this project's only real acceptance channel. Selection
needs generations, a working reproduction schedule, and a heritable axis that
varies — three things that are not all true yet.

**W3's two-patch divergence instrument is already the right measurement** and
is already queued: same founders, windy patch against sheltered patch,
scored on root:shoot and on slenderness. It needs §11.5's exposure to exist
before it can be pointed at wind, and nothing else.

#### 11.6a Can plasticity *derive itself* from selection? — asked by the owner, answered from source

The owner's question, 2026-08-23: *"I would love to plasticity derive itself
from selection/evolution, but I don't know if our model has the complexity
for that."*

**The model has the complexity.** Read at `4018aee`, the evolutionary
machinery is complete and is not a sketch:

- `OrganismState::genotype_draws: [f32; GENOTYPE_TRAITS]` — continuous, and
  `set_seed` copies the parent's array into the child with **independent
  per-trait jitter** (`MUTATION_SIGMA`), clamped to `[-1, 1]`. Its comment is
  explicit about why the traits drift separately: "two offspring of one
  parent can differ on branching and agree on height, which is what lets a
  population explore corners of the trait space rather than sliding along one
  diagonal."
- `alleles: [u8; DISCRETE_LOCI]` — six discrete loci, inherited whole and
  mutated by *jumping* rather than drifting, so a morph holds together
  between excursions.
- `seed_genotype` **declines to redraw an inherited genome** (`if
  ...s.inherited { return; }`), which is the difference between a population
  that breeds and one that inherits.
- `generation`, `seeds_set`, `endowment`, and cumulative `organisms_born` /
  `organisms_died` all already exist.

So **a reaction norm is not new machinery.** "How strongly does this
individual redirect carbon when it is shaken" is one more continuous trait,
read through the same `genotype(world, organism_id, slot, variance)` call
`pipe_ratio` already uses at slot 4. Selection then acts on the slope, not
just the intercept — which is exactly what the owner asked for, and it costs
one slot and one multiply.

**Two things stand between that and a result, and neither is complexity.**

1. **There is no free slot.** `GENOTYPE_TRAITS = 9` and every index is spoken
   for — 1 growth rate, 2 plastochron, 3 turgor, 4 `pipe_ratio`, 5 upward
   weight, 6 root allocation, 7 stomatal reserve, 8 penetration force, plus a
   computed `bc_slot` for branch chance. The slot ceiling is **lane P's
   current package (P3)**, so the gate on a heritable reaction norm is
   already being worked, by accident rather than by plan.
2. **Generational throughput is unmeasured, and this project already wrote
   down that it matters.** `plant-evolution-design.md` §5, quoted verbatim in
   `world.rs`'s doc for these counters: *"the count of inherited-genome
   establishments per run is the plant equivalent of births-per-generation,
   and if it reads ~0 at 30k frames, every evolution claim at that horizon is
   about founders."* `organisms_born` / `organisms_died` exist precisely to
   answer that and the number has not been read. **Read it before promising
   anything selection-derived** — it is one printout, not a study, and it
   decides whether selection is a mechanism or a rounding error at play
   horizons.

**The recommendation, which is not a fork.** Build the plastic response as a
**heritable reaction norm from the start**, with its slot's founding
distribution drawn wide. Plasticity then works from frame one — that is the
visible mechanic, judgeable on a card, no generations required — and
selection acts on the same number for free as soon as turnover supports it.
There is no version of this where "do plasticity now, evolution later" means
building twice; the only cost of doing it right is one genome slot, which is
blocked on a fix already in flight.

The honest caveat: **selection moving that slot has to be *demonstrated*, not
assumed.** Windthrow helps more than it looks — it is a selective *death*,
which is the strongest kind of pressure and the one this model has been short
of — but a claimed divergence needs the born/died counters, the two-patch
instrument, and an order statistic over seeds. `CLAUDE.md` records a 3.5-hour
megastudy that turned out to be three populations wearing twenty-four logs;
this is exactly the shape of study that fails that way.

### 11.7 Traps, filed before anyone builds

1. **The wind-throw decision is a whole-*plant* judgement, never a per-cell
   structural check.** This is the one that would turn a good mechanic into a
   catastrophe. A structural check fired mid-organism amputates it — the
   measured precedent is a stand going from 20,213 living cells to 772 from a
   *single* check (`CLAUDE.md`, the structural-check amputation gotcha). The
   rule here evaluates a moment about a root plate; the quantities it needs
   (a crown centroid, an anchor half-width, a tipping moment) are defined for
   a whole plant and undefined for a cell. This is `CLAUDE.md`'s "which
   object does this rule evaluate — a cell, a section, or a whole piece?",
   and it has already been missed twice in this repo by sessions that had the
   paragraph in front of them.
2. **No standing cost.** Evaluate only organisms a gust overlaps, at
   organism-tick cadence, off the anchor set `anchor_support` already built.
   A per-frame stability pass over every tree is §3.5's bill with a different
   name on it.
3. **Sway must not feed this.** T4's rider generalises: sway is a *display*
   of wind and never a second input to it. A lean that fed the overturning
   moment would rediscover the dead end where a blast shockwave dominated the
   growth formula outright.
4. **Do not tune it on one storm or one seed.** Wind-throw is chaotic in the
   seed by construction — which tree is marginal reshuffles on any legitimate
   change. `seedsweep.sh`, run to rest, gated on an order statistic, built
   *before* the model changes. `CLAUDE.md` records two model changes that
   were green on all eight acceptance cases and ate fifty times more world
   than the bug they fixed.
5. **Count the rungs, do not average them.** "How many trees fell" is the
   metric trap this report already names twice: a failure count is not a
   damage count, and a mean over events is not the size of the pieces. The
   quantity that answers §11.3 is the *census by rung*.

### 11.8 The three calls, decided by the owner 2026-08-23

Put to the owner as open questions and answered the same evening. Recorded
here in their decided form so nothing is re-litigated.

1. **The economy half moves into P2 now.** *"agreed to pull the benefit into
   P2 brief."* Anchorage-as-benefit is carried by lane P's next package
   rather than waiting behind lane S's T2, because P2 is where the root
   *cost* lands and a cost shipped without its counterweight is what produces
   the minimal-root monoculture of §11.1. The two halves are now in different
   lanes on purpose: lane P owns *what roots buy*, lane S owns *the storm that
   collects*.
2. **Exposure is in scope and is its own package.** *"start a new session to
   fix the wind geography."* Dispatched as **W4 — wind geography**, briefed
   against §11.5: terrain-derived exposure, read-only, one consumer
   (`weather::gust`), instrument before mechanism. The distinction from the
   reverted steady-wind term is the whole brief and is restated in §11.5.
3. **Plasticity is built as a heritable reaction norm**, so that selection
   can act on it rather than being an alternative to it — the owner's
   preference, and §11.6a establishes from source that the model already has
   the machinery. Gated on the genome slot ceiling, which is lane P's current
   package.

### 11.9 Freshness

Written 2026-08-23 against `main` at `4018aee` (this report's own merge). No
figure in this section was measured; every number quoted is read from source
at that commit — `plant::is_structural_anchor` and `anchor_support`
(`src/sim/plant.rs`), `bearing_moment` (`src/sim/load.rs`), `thicken` and
`pipe_ratio` (`src/sim/plant.rs`), and `weather::at` / `weather::gust` with
`GUST_THRESHOLD`, `GUST_INTERVAL` and `GUST_RADIUS` (`src/sim/weather.rs`).
The 41.6% duty cycle is quoted from §3.4 of this report, not re-measured.
**Nothing here has been rendered or judged by eye**, which for a mechanic
whose whole acceptance is §11.3's four visible rungs means every claim in it
is a prediction.
