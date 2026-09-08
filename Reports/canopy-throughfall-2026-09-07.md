# Rain that never reached the ground: water on a canopy, and the drip that clears it

**2026-09-07.** Built and measured. The owner's report:

> *"water also pools on the top of our plants. it should drip through.
> what's the best way to fix that? I'm guessing it's probably similar
> solution to creatures"*

The guess was right and the precedent is closer than "similar": **the rule
already existed for powders and water was simply never wired into it.**

---

## 1. Why it pooled

`update_liquid` tries straight down, a long-range lateral descent, both
diagonals, then fill transfer. Every one of those needs a target it can
enter, and a plant cell cannot be displaced — so water that lands on a crown
has nowhere to go and sits.

`update_powder` has one step the liquid path does not:
`fall_through_organism`, which lets a cell tunnel down through living tissue
to the first real air below it. Its own doc already makes the argument:

> *"The 2D-slice rule… the world is one vertical slice of a 3D wood, a branch
> one cell wide is not a shelf spanning the tree's whole depth, and a leaf
> falling past it is the common outcome."*

That shipped for `litter`, `seed` and `windfall`, gated on
`Material::falls_through_organisms`. `deadwood` deliberately lacks the flag —
a snapped branch is chunky enough to genuinely hang up in a crown. Water was
never given it, and the liquid path would not have read it.

## 2. How bad it was

`scene=canopyrain` (new): nine grown trees on a wet bed, rain pinned rather
than waited for, censusing liquid whose cell *below* is living tissue.

Under continuous rain, three samples: **89–96% of every liquid cell in the
world was standing on living tissue**, at a mean of 55–70 rows above the
floor and as high as 107. Almost no rain reached the ground in a wood at all;
what did was what missed the canopy.

## 3. What shipped

`update::drip_through_organism` — the liquid half of
`fall_through_organism`, with one difference that is the whole design: it
passes water at a **rate** rather than straight through. The owner asked for
it to *drip*, and passing it instantly would empty a crown the moment the
rain stopped, which is the binary outcome the ethos rules out. The middle
here is a wood that keeps dripping after the sky clears.

`DRIP_PERIOD_FRAMES = 8`, staggered by column (`+ x`). The stagger is not
cosmetic: without it every drop in the world falls on the same frame and a
wood sheds in flat curtains rather than dripping. It is the frame number
rather than an RNG draw, which costs nothing and does not perturb the shared
stream for every cell downstream.

**It never displaces tissue** — the landing cell must be genuinely empty,
exactly as the powder version requires. That is what makes this far simpler
than the creature work that preceded it: a body has to *be* somewhere, and
displacing a plant cell cost the plant its anchor. Water has no such problem.

`water.ron` gains `falls_through_organisms`. `oil` deliberately does not,
which is what the specificity guard turns on.

## 4. Measured

Rate sweep, `scene=canopyrain soak=0` (raining throughout), three samples per
arm, all arms from **one binary** via `CANOPY_DRIP`:

| period | share of liquid on tissue | mean rows above floor |
|---|---|---|
| off | 95.7 / 93.0 / 89.3% | 70.4 / 61.7 / 55.5 |
| 24 | 86.7 / 91.9 / 86.9% | 46.1 / 53.8 / 57.3 |
| **8 (shipped)** | **64.5 / 69.0 / 60.6%** | **50.6 / 27.7 / 32.6** |
| 2 | 36.8 / 56.5 / 66.7% | 19.9 / 25.2 / 22.4 |

Monotonic in both columns. **Mean height is the clearer signal** — water
ending 20–30 rows up instead of 55–70 is water most of the way to the ground.

Drainage arm (`soak=1500`, then the sky clears), five samples:

| | frame 100 → 1,700 after the rain stops |
|---|---|
| off | 38 → 28 → 25 → 27 → **20**, mean height 65 → 54 |
| on | 26 → 15 → 11 → 11 → **9** |

The off arm plateaus; the on arm keeps falling.

**Frame cost: below the noise floor.** Three alternating paired runs of
`examples/ascii`, whole-run wall clock: off 118.0 / 120.4 / 121.9 s against
on 120.1 / 118.6 / 122.0 s. The arms interleave and "on" is faster in one
pair, which is what noise looks like. The gate is one branch on a `Cell` the
caller already holds (`below.organism_id() == 0`), so water that is not
sitting on tissue — nearly all water, nearly all the time — pays no lookup.

`examples/ascii` exits 0 with the drip on. Its output does differ, in the one
scene that has water and plants together; that is the change working, not a
regression, and there is no golden file to break.

## 5. Two metric traps this cost, both already in `CLAUDE.md`

**Counting cells when the quantity is volume.** The guard's first version
counted *cells* of each liquid and reported 128 below the canopy from five
placed above it. Not a conservation violation — a `Liquid` cell holds
continuous fill, so five full cells spread into dozens of part-full ones and
a cell count measures *spreading*. `update::liquid_fill` summed as volume is
the right instrument, and `CLAUDE.md`'s liquid metric trap says so in as many
words.

**Censusing a standing quantity in the state that manufactures it.** The
first paired run measured water-on-tissue *while it rained* and reported the
fix barely working — 85–95% → 76–88% — while the gate counters underneath
showed **532 successful drips**. Under continuous rain some water is always
on the canopy, so a crown that sheds perfectly and one that holds everything
read almost the same: the census was counting water *in transit* as water
*stuck*. The scene now has a soak-then-clear arm for exactly this, and the
rate sweep is run under sustained rain where the steady state is the point.

A third, smaller: `soak=0` originally pinned the sky clear immediately, so
the first rate sweep ran on a nearly dry canopy — its totals of one to five
cells were the tell.

## 6. What is not done

**Interception is not modelled.** A real crown *holds* a film of water and
some of it evaporates rather than reaching the ground; here every drop
eventually goes through. Adding a held film would connect to the plant water
economy that already exists, and would change constants tuned against
current behaviour, so it was deliberately left out of a change whose job was
to stop water hanging in the air.

**A third of drip attempts still find no air.** The scan gives up at the
first cell that is neither air nor organism, and at `ORGANISM_TUNNEL_REACH`
(16, bounded by `MAX_REACH`) inside continuous tissue. In a dense crown that
leaves a residue — 9 cells after the rain stops, against 20 with no drip at
all, still falling rather than plateauing. Letting a drop land *into* a
part-full liquid cell rather than requiring empty is the obvious next lever
and is untried.

**Stemflow is not modelled** — real water also runs down a trunk. Nothing
here does that.
