# Feasibility: plants, creatures, and destructible structures

Exploration of three directions against the engine as it actually stands, not
against a pixel engine in the abstract. Written after M3.

**Short version:** all three are feasible, none needs a rewrite, and they share
two pieces of infrastructure that do not exist yet. Building that shared part
first turns three hard features into three moderate ones. The order matters more
than any individual feature does.

---

## What the current architecture allows and forbids

Five facts drive everything below.

**`Cell` is exactly 4 bytes and full.** `material: u16`, `shade: u8`,
`flags: u8` — and only one flag bit is used, leaving **7 spare bits**. There is
no room for a growth stage, a temperature, an anchor distance, or a body id
without either spending those bits or widening the cell.

**The sweep deliberately does not visit cells that are not moving.** Dirty
rectangles exist precisely to skip settled material, and chunk sleeping is the
whole performance strategy — a still world costs nothing. But a plant that is
not moving still needs to grow, a structure that is not moving still needs its
supports checked, and a sleeping creature still needs to wake up. **Every one of
these three features needs the one thing the core system is built to avoid.**

**`MAX_REACH` (32) bounds how far any CA rule may read.** A rule that reads
further acts on cells that never wake it, and goes stale. This is not a
guideline — it is the bug that froze sand in mid-air and, separately, stopped
water levelling.

**There is no entity system.** Everything is cells. There is no notion of a
thing with a position, a velocity, or a state machine.

**The sweep is single-threaded and a full screen of moving material already
costs ~16 ms of a 16.6 ms budget.** Anything added here competes for a budget
that is already spent at peak.

---

## 1. Plant growth

**Verdict: highly feasible, lowest risk of the three, and the best first move.**

Growth *is* a cellular automaton, so this works with the grain of the engine
rather than against it. Nothing here fights the architecture.

### The one real problem

Grass in a settled chunk never gets swept, so it never grows. Waking chunks that
contain plants would defeat sleeping across any world with vegetation in it.

The fix is not to wake chunks — it is to notice that **plants only change at
their tips.** A tree trunk is inert; only the growing ends do anything. So keep
a small per-chunk list of *growth sites* rather than scanning for them. When a
site grows, it removes itself and adds its new tips. Cost is proportional to the
number of tips, which is tiny, and it is completely independent of how much
vegetation exists.

This "active site list" is the first piece of shared infrastructure, and
creatures and structural integrity both need the same thing.

### Where the state lives

A plant cell wants an age, a growth stage, a direction, and a distance from
root. That does not fit in 7 bits.

It does not have to live in the cell. Growth sites are sparse, so a side table
per chunk — position to plant data — is both cheaper and more flexible than
widening every cell in the world to serve the few that are growing. The cell
only has to say "I am vine"; the interesting state hangs off the site list.

### What is cheap to build

- **Moss and grass**: a cell adjacent to solid ground with space above spawns
  into a neighbouring empty cell with some probability, capped by distance from
  its source. Purely local, a handful of lines.
- **Vines**: carry a growth direction (3 flag bits gives 8 directions), extend
  into empty space, prefer to hug solids.
- **Trees**: an L-system stepped over time, with the instruction pointer in the
  side table. Branches push new sites. This is where genuinely varied results
  come from, and it is not much harder than vines.
- **Roots drinking water**: a root cell adjacent to water consumes it and
  credits an energy counter. This is the first mechanic that ties plants to the
  existing material physics, and it is nearly free.

### What is not cheap

**Growing toward light.** Needs a light field. Worth doing anyway — a coarse
grid at roughly 1/8 resolution with a diffusion pass serves plants, creature
perception, fire, *and* the M6 rendering work. Do it once as shared
infrastructure rather than as a plant feature.

**Burning.** Needs temperature, which needs the cell widening discussed below.

### Effort

Low to moderate. Grass and vines are days once the site list exists. Trees are
the interesting part and are bounded. Nothing here is research.

---

## 2. Creatures with basic behaviours

**Verdict: feasible, but it is a different kind of system, and the risk is
scope rather than difficulty.**

The first decision is the whole design: **are creatures cells, or are they
entities?** Three answers, and they are not equally good at the same things.

### Option A — creatures as cells

The creature *is* one or more cells with a special material and rules. A worm
head that burrows by swapping with sand.

- **For**: perfect physical integration, destructible for free, no new systems,
  fits the data-driven material model almost exactly — a `kind: Creature` with
  a named behaviour.
- **Against**: no memory, no multi-cell coherence, movement locked to one cell
  per frame, and nothing that deserves to be called AI.
- **Good for**: worms, slimes, spreading fungus, fire sprites.

### Option B — creatures as entities

A struct with a float position, a velocity, and a state machine, that queries
the grid for collision and writes back into it to dig or eat.

- **For**: real behaviour, pathfinding, sub-cell movement, animation.
- **Against**: an entirely new system; needs collision against a pixel world,
  which is the same problem as character physics; and it is not destructible
  without extra work.

### Option C — entity brain, cell body

The body is drawn into the grid as cells, so it is destructible and physically
real, while a controller decides where it wants to be and re-rasterizes each
frame.

- **For**: shoot a worm and it loses cells. Physical, destructible, *and*
  properly controllable.
- **Against**: needs the erase-transform-rasterize loop.
- **The important part**: that loop is *exactly* the rigid body pipeline from
  section 3. Option C is nearly free once rigid bodies exist, and quite
  expensive before.

**Recommendation: A now, C later, skip B.** B's costs mostly overlap with C's
without C's payoff.

### Things that will bite

**Ordering.** Entities writing into the grid *during* the CA sweep would break
the sweep's invariants — the bottom-up ordering and the moved flag both assume
nothing else is mutating cells. Entity updates must be their own phase, before
or after the sweep, never inside it. Fix the frame order early:

```
entities → CA sweep → rigid bodies → render
```

**Perception is cheap, and the reach limit does not apply.** `MAX_REACH` binds
CA rules because of how waking works. Entities are not part of the sweep and
can read anywhere in the world. Sampling a few cells ahead and below is enough
for walking, swimming, and fleeing.

**The fun is in material interaction, not in the AI.** A creature that dies out
of water, burrows only through loose powder, or eats sand and excretes it gets
enormous mileage out of systems that already exist. Behaviour trees do not.

### Effort

Option A is moderate and reuses the plant site list almost unchanged. Option C
inherits whatever section 3 costs.

---

## 3. Rigid and semi-rigid destructible structures

**Verdict: feasible, the largest of the three — and the most valuable insight
here is that most of the payoff does not require the expensive part.**

### The expensive path

Full rigid bodies, as in Noita: connected-component labelling on pixel clumps →
marching squares contour → Douglas-Peucker simplification → triangulation →
a physics solver → and each frame erase the old pixels, step, re-rasterize at
the new pose. This is genuinely weeks of work, and it is where most projects of
this kind stall.

Known traps, worth writing down now:

- **Rotation leaks.** A rotated body no longer aligns to the grid and leaves
  gaps sand pours through. Rasterize by inverse-mapping each destination pixel
  into body-local space rather than forward-mapping source pixels, and dilate
  slightly.
- **Bodies keep chunks awake.** A moving body rewrites cells every frame, which
  is correct but means a world full of debris never sleeps. Sleeping bodies in
  the solver must translate into not re-rasterizing.
- **Bodies fight multithreading.** Once the sweep is parallel, bodies writing
  arbitrary cells cannot run inside it. They need their own serial phase — which
  is another reason to fix the frame order before parallelising anything.

### The cheap path that gets most of the feel

**Structural integrity by distance-to-anchor.** Every solid cell stores how far
it is from something anchored — bedrock, the world edge, or a foundation. A cell
whose distance exceeds its material's tolerance is no longer supported and
breaks free, falling as loose material.

This gives you: buildings that collapse when you cut their supports, overhangs
that can only span so far, and materials that differ structurally — with **no
polygons, no solver, and no rasterization loop at all.** Debris falls as powder
rather than tumbling as a rigid slab, which is a real difference but a much
smaller one than the work involved.

It is also local and incremental. Distances propagate from neighbours; removing
a support only forces recomputation within the affected structure.

Best of all, it drops straight into the existing data-driven material model:

```ron
(
    name: "wood",
    kind: Solid,
    max_unsupported_span: 8,   // stone 3, steel 20
)
```

That is one field in a `.ron` file and a hot-reloadable tuning knob for the
entire structural feel of the game.

**Recommendation: build structural integrity first and rigid bodies later, as
separate projects.** Integrity is roughly a week and transforms building into a
real mechanic. Rigid bodies then upgrade collapsing rubble into tumbling chunks,
and can be deferred a long way without blocking anything else.

---

## The shared infrastructure

The reason ordering matters: three of the pieces below are needed by more than
one feature, and building them once is much cheaper than three times.

### 1. Widen `Cell` to 8 bytes

Everything wants per-cell state and 7 bits will not stretch. Proposal:

```
material: u16   shade: u8   flags: u8   aux: u32
```

Where `aux` is interpreted according to the material's kind — growth stage,
anchor distance, temperature, or owning body id. A tagged union by kind is not
elegant, but it is honest, it is what these engines do, and the alternative is
four parallel side tables.

Cost: a 2048×2048 world goes from 16 MB to 32 MB. Irrelevant.

This is also exactly what heat and fire need, so it is one decision covering
four features. **Do it before, not during, any of them.**

### 2. A per-chunk active-site list

Separate from dirty rectangles, which must keep meaning "something moved". This
one means "revisit this cell periodically even though nothing moved" — plant
tips, creature cells, integrity checks pending recomputation.

Small to build, and it is the difference between these features costing
proportional to the number of interesting cells versus proportional to the size
of the world. **This is the single highest-leverage thing on this page.**

### 3. Connected-component labelling

Needed by rigid bodies, by structural integrity, and arguably by creature
bodies. Write it once and well.

### 4. A coarse field grid

A downsampled grid with a diffusion pass, carrying light, heat, and moisture.
Plants grow toward light, creatures sense, fire spreads, and M6 rendering gets
its lighting from the same place.

### 5. Fixed frame phases

`entities → CA sweep → rigid bodies → render`. Cheap to establish now, painful
to retrofit once three systems are all writing cells at different times — and a
hard prerequisite for multithreading.

---

## Suggested order

Each step unlocks the next and delivers something visible on its own.

| # | Step | Size | Why here |
|---|---|---|---|
| 1 | Widen `Cell`, add `aux` | Small | Unblocks everything, including fire |
| 2 | Active-site list | Small | The enabling piece for 3 and 5 |
| 3 | Plants | Moderate | High payoff, low risk, proves the site list |
| 4 | Structural integrity | Moderate | Destructible building without a solver |
| 5 | Cell-based creatures | Moderate | Reuses the site list directly |
| 6 | Multithreading | Large | Budget is already spent at peak |
| 7 | Connected components + rigid bodies | Large | The expensive part, deferred as far as possible |
| 8 | Entity creatures on rigid bodies | Large | Nearly free once 7 exists |

The thing to resist is starting at 7. It is the most exciting item and the one
most likely to consume months without producing a playable result, while 3 and 4
between them deliver growing, burnable, buildable, collapsible worlds on top of
machinery that already works.

---

## Honest risks

- **The budget is already spent.** A full screen of moving material costs ~16 ms
  of 16.6. Plants and integrity checks are cheap per frame, but they are not
  free, and multithreading moves up the list the moment peak load becomes normal
  load.
- **`aux` as a tagged union is a compromise.** It will be tempting to overload
  it. Worth a written rule about which kind owns which interpretation.
- **Creature scope is unbounded.** Materials are a finite design space; "AI" is
  not. The ladder in section 2 exists to keep that honest.
- **Rigid bodies and threading genuinely conflict.** Not fatal, but it means
  step 6 must land before step 7, not after.
