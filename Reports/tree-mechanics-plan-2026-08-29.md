# Plants that carry their own weight: bending, breaking, and falling over

**Status: plan. One change landed** (`3bdf674`, a landed piece is bark
outside and timber where it broke — §7 stage 0). Rewritten 2026-08-29 after
three independent reviews and two owner rulings; §10 records what changed
and why, because the first draft got two things wrong that are worth not
repeating.

**The brief, verbatim** (owner, 2026-08-29):

> "You are in charge of making the trees (and all plants) have real
> structure and physics. [...] 1) trees with weak/narrow base and heavy top
> will crack and fall over, 2) if a rock or other large objects hit the tree
> it will break branches or even the whole tree, 3) you can chop a tree
> down, 4) wind or storms can knock off branches or the whole tree, 5) a
> branch that grows way too far laterally without being strong enough will
> bend and eventually break [...] In our previous attempt, the tree mostly
> broke turned into dust and collapsed into a pile instead of breaking into
> realistic branches or falling over. This should be realistic mechanics and
> physics as much as possible."

**Two rulings since**, both of which reshaped this document:

> *"I don't see why you should be changing the growth or the carbon budget
> systems directly. You are inventing physics and structural mechanics for
> them."* — growth and the economy are **out of scope**.

> *"It should work for all plants. The system needs to make grass bend and
> not break. It is ok if the grass gets a little smaller."* — **every plant
> participates**, and bending is not a tree feature.

And one verdict on the current build, card `20260829T034016675Z-5a9b07`:

> *"Everything. Why does it change colors? why does it look like a pile of
> dust next to the base of the trunk turned tan filled with more brown dust.
> **The leaves shouldn't turn to powder ever.** They can stay on the branch
> if that is easier (but if they do fall off, i feel like they should stay as
> larger leaf pieces). It should look like a bunch of branches are broken on
> the ground. It also needs to be realistic for if i hit the base of the
> tree, the whole think falls over (or due to a weak trunk). If it gets
> slammed by a rock it will explode more like this picture. We need to be
> realistic!"*

**Read first:** `CLAUDE.md` (the ethos section is the acceptance standard),
then `Reports/physical-trees-t1-implementation.md` §4c–§4f — the previous
attempt's owner verdicts, which are the bar this has to clear.

---

## 0. The summary

**One stress number, two material properties, five verbs.**

1. **Stress.** For every plant cell: how hard is it being bent?
   `stress = moment / (k · section²)`, with the moment accumulated over the
   plant's own topology and the section read across the load path. Identical
   arithmetic for moss, grass, a bramble and an oak. §2.

2. **Stiffness — how far it bends under that stress.** Grass: very low, so a
   blade lies right over. Wood: high, so a limb barely moves until it is in
   trouble. **And bending relieves stress**, because a blade lying over has
   almost no leverage left on its own base. That is why grass never breaks,
   and it falls out of the model rather than being special-cased: a flexible
   thing sheds the load by moving, a stiff one has to carry it. §3.

3. **Strength — when it snaps.** Set from a measured seed sweep with
   headroom, never from first principles: one cell is not one centimetre, and
   a branch drawn one cell wide is a slice through a branch with depth
   (`wiki/plants.md` says so about leaves falling past branches). So nothing
   falls over on a calm day, which is correct. **What fells a plant is a load
   that arrives** — a gust, a rock, an axe, snow. Self-weight is the baseline
   the event lands on. §2c.

4. **Bending is what makes "narrow base, heavy top" legible, and bending is
   also ask 5.** The owner used the same phrase for both. A tree whose crown
   has outrun its trunk leans further under the same gust than the squat one
   beside it, and leaning is visible before breaking is. That is the graded
   outcome the ethos asks for, arriving on every plant in the world rather
   than on the one being felled.

5. **Buckling, not bending, is what fails a narrow base** — and the shipped
   rule is already 90% of it. A balanced upright stem has a bending moment of
   ~zero; measured on the shipped tree, the base is the *least* stressed place
   in it (§2b). A column under its own weight fails by buckling at a height
   `∝ width^(2/3)`, an exponent that is **identical whether a stem is read as
   a flat slab or as a slice through a cylinder** — so it sidesteps the
   dimension question entirely. `organism_structural_tick` already runs
   `support > max_cantilever_reach · density`, which is a slenderness rule
   **with the width term missing**. One multiplication, not a new system. §4.

6. **The picture is the gate, and it is not downstream work.** 87% of a
   felled tree's mass already comes off as coherent chunks and the settled
   pile still reads as sand. Two causes were found by looking rather than by
   measuring, and one is fixed: a landed piece's colour was re-rolled per
   cell and flipped from bark to pale timber the instant it stopped moving
   (`3bdf674`). The other is the owner's own ruling — **leaves must never
   become a powder**. §6.

7. **The fall.** A severed piece is given no rotation at birth, so the
   biggest pieces land standing on end. The seed is **not** the breaking
   torque (that would spin the heaviest pieces hardest, backwards): it is the
   angular acceleration about the break, `α = Σmgd / Σmr²`, which for a limb
   of length `L` is `3g/2L` — **inversely proportional to length**, so a bole
   turns once and a twig tumbles, with no tuning constant. §5.

**Order: 0 make it legible → 1 see the stress → 2 bend → 3 break → 4 the
fall → 5 the verbs.** Bending comes before breaking because it is visible on
every plant every day, and breaking is only visible when something breaks.

---

## 1. What is wrong today, from source

`structural::organism_structural_tick` decides a plant cell's fate with
`support > max_cantilever_reach · wood_density − supported_load/4`, where
`support` is `plant::anchor_support`'s weighted distance to the nearest
anchor (Dijkstra out from the anchors, costs `standing 0 / reach 1 /
hanging 2`).

Three defects, and they are not the same defect:

- **Section width appears nowhere.** A one-cell twig and an eighteen-cell
  bole have identical strength.
- **The plant's own mass is invisible to its own failure rule.**
  `supported_load` counts only `Powder | Liquid` sitting on top of tissue
  within a small radius, so foreign weight matters and the crown does not.
- **There is no bend.** The outcome is binary: standing, or converted.

Measured on `scene=fell` at frame 7,100: *"furthest finite 72 of wood's
96"*, 4 cells severed in 7,100 frames. The rule sits at 75% of its threshold
on a healthy tree and cannot tell a twig from a trunk.

**The criterion half of this is one the repo already rejected — for rock.**
`dead-ends.md`, unconditionally: *"Reach/distance-to-anchor as the failure
criterion is backwards… Replaced by `load > capacity` on accumulated bending
moment."* That replacement shipped as `load.rs`, whose `evaluate_within`
returns `None` on `organism_id() != 0`.

**But `max_cantilever_reach` is not the field to delete**, and the first
draft of this plan was wrong to say so — twice over:

- Its `u16::MAX` default **is** foliage's opt-out, taken by omission in
  `leaf.ron` and `grassblade.ron`. `leaf.ron:126` records the cost of losing
  it: median leaves per tree 1,376 → **1**, and the stand from 31,731 cells
  to 7,171. A companion entry (`dead-ends.md:811`) records a grass sward
  deleted by excluding foliage the wrong way.
- The report the draft cited (`fracture-mechanics-design.md` §3.7) names
  **`max_unsupported_span`**, a deliberately different field on a different
  scale, and that instruction is already discharged.

So the field survives, reinterpreted: it is the **slenderness** rule, and
what it is missing is the width term. §4.

---

## 2. Stress

### 2a. The quantity

```text
M(c)  = Σ density(i)          over what c carries
Sx(c) = Σ density(i) · x(i)
moment(c) = Sx(c) − x_c · M(c)      (signed; the criterion takes |·|)
stress(c) = |moment(c)| / (k · section(c)²)
```

`density` is `MaterialDef::density` — leaf 0.25 against wood 0.9 — so 48% of
a tree's cells are 20% of its mass. **Signed, not absolute**, because §5's
rotation needs the direction and storing it costs nothing.

`section` is the run of tissue across the load path. Two corrections the
reviews forced, both already known to this repo in another place:

- **A fork must not read as the widest section in the tree.** `stem_run`'s
  own doc records the identical defect from the pipe-model side — *"53% of
  occupied rows containing more than one separate run… worst case 23 cells
  across 9 runs read as one 23-wide stem."* `OrganismCell::order` already
  exists and `thicken` already propagates it, so restricting the walk to
  same-order tissue is available and is where a limb should tear out.
- **Which reading, slab or cylinder, must be written down.** A slab (`t = 1`)
  gives `Z ∝ D²`; a slice through a cylinder gives `Z ∝ D³`. The engine
  counts cells for mass, which is the slab reading, and `load.rs` is
  quadratic. **Take the slab and say so**, or someone later "fixes" the
  exponent and every constant moves.

### 2b. The forest is a DAG, not a spanning tree — and this is the deepest finding

The first draft proposed accumulating `M`/`Sx` up the parent array
`plant::accumulate_support` already builds. **That is a recorded dead end for
exactly this use.** `dead-ends.md:807`:

> *"`accumulate_support` walks a spanning tree over what is, for a thickened
> trunk, a **blob** rather than a tree graph, so `q_now == 0` means 'not on
> the arbitrary path this tick's walk took' — true of most of a trunk's
> girth, and the rule was reading a traversal artifact as a biological
> fact."* Re-test when: *"only if computed over the true topology rather than
> a spanning tree."*

The measured cost when a rule was built on it: the stand went from 3,437
cells to 704–2,569, with 4–6 of 8 founders establishing against 8 of 8.

`load.rs` does not use a spanning tree and says why in `dependants`' doc: a
tree picks one route to the ground and sends the whole load down it —
*"visible in the stress view as a one-pixel red line through an otherwise
green building: a table's whole span loaded its left leg while the right leg
carried nothing."* It floods to greater-distance neighbours and divides each
hand-off by `support_count`, which is a **flow over a DAG**.

So a 15-cell bole under a spanning tree gives one spine cell the whole
crown's moment and fourteen neighbours nothing. **A one-pixel red line down a
trunk.** Port the share division; do not port the spanning tree.

### 2c. Why the strength constant is fitted, not derived

Measured with `examples/beam_probe.rs` on the shipped `scene=fell` tree
(§9 records what that instrument does and does not measure reliably):
stress spans 17,888× between median and max, peaks 20–50 rows above the
anchor plate, and **the base of the tree reads 0.0–0.3** — the least
stressed place in it, because the crown is balanced over it.

Two conclusions, one of which the first draft got wrong:

- **Right:** there is no first-principles strength for a cell of wood. The
  constant is set by sweeping the population and leaving headroom, so nothing
  falls on a calm day and events do the felling.
- **Wrong, and corrected:** the draft concluded from this that today's trees
  are mechanically impossible and that growth must change. Both the section
  measure and the missing clamps make that a statement about the instrument.
  §9.

---

## 3. Bend

**The owner's phrase for ask 5 and for grass is the same phrase**, which is
the tell that it is one mechanism.

- `stress` is already continuous. A material gains a **stiffness**; the
  plant's deflection is `stress / stiffness`, evaluated at organism-tick
  cadence and at a gust.
- Grass: low stiffness, so a blade lies over. A blade is a few cells tall, so
  laying it over moves its tip two or three cells — **not subtle, the whole
  blade.** The "a bend is 0 or 1 cells at this resolution" objection is true
  of a thick trunk and false exactly where the owner is pointing.
- Wood: high stiffness, so a limb barely moves until it is near failure, and
  then sags visibly before it goes. That is ask 5's *"bend and eventually
  break"* and it is also the telegraph the player currently has none of.
- **Bending relieves the moment**, because a leaning stem's lever arm
  shortens. This is why grass does not break, and it is why the two
  behaviours need one mechanism rather than two.

**The cost, and the dead end next door.** Sim-side sway — a per-frame
integrator over every cell of every plant — is a recorded do-not-retry with
four independently fatal counts, and a canopy that never stops moving was
measured at **+8.0 ms/frame** against half a 60 Hz budget. The distinction
this relies on: a stress-driven lean changes only when the plant ticks or a
gust arrives, which is the *"discrete, gust-local lean"* already costed at
~0.11 ms/frame amortised in `physical-trees-design` §3.4. **That distinction
is a claim until it is measured**, and stage 2 begins by measuring it.

Open, and to be settled by rendering both rather than arguing: whether the
lean moves cells (collided pose follows) or is a render-side offset (drawn
pose only). Grass wants the first and is cheap; a tree limb wants the second
and is not.

---

## 4. Break

Two failure modes, and they are different physics. Conflating them is why
ask 1 was unreachable in the first draft.

**Bending** — `|moment| > k · section²`. This is asks 2, 3 and 5: a limb
carrying too much too far out, a section weakened by an axe, a limb hit.

**Buckling** — a slender column under its own weight. `L_c ∝ D^(2/3)`
(Greenhill), and the exponent is the same for a 2D slab and a 3D cylinder,
so this term does not depend on the reading §2a has to choose. `D ∝ L^(3/2)`
is McMahon's elastic similarity, which is the taper real trees show.

**This is ask 1, and the shipped rule is already most of it.**
`support > max_cantilever_reach · density` is a slenderness rule whose width
term is a constant. Multiply it by the section and it becomes the rule that
fails a narrow base under a heavy top — and the field stops being vestigial
instead of being deleted, which also keeps foliage's opt-out (§1).

**What must be replaced, not dropped.** `supported_load` — *"weight piled on
a branch shortens how far it can reach"* — is the snow-on-a-branch term, and
the first draft deleted it silently. `load::powder_surcharge` is the
ready-made replacement (loose material standing on a cell, capped at
`POWDER_SURCHARGE_CAP = 12`). It is also **ask 1's demo scene**: snow, sand
and rubble already fall and pile, so a snow-loaded crown snapping its own
trunk is ask 1 with no new verb.

**And the criterion needs an invalidation story it does not have.** The only
organism-side `schedule_structural_check` fires on a *distance* rise
(`plant.rs:3892`). Under a load criterion the thing that should re-open a
cell is a *load* rise — the crown grew, a rock landed, a gust blew. Growth
raises the moment and never raises `support`, so ask 1 has **no trigger**
today. Scheduling from the growth path is separately forbidden
(`CLAUDE.md`'s amputation gotcha), so this needs its own answer.

**One line worth knowing:** `organism_structural_tick` gates on
`within_disturbance`, which is a constant `true` only at the default `F9`
setting. At LOCAL/TIGHT/NONE a self-weight failure with no nearby
disturbance never fires — ask 1 is switched off by a settings key.

---

## 5. The fall

Measured on one fell that promoted 60+ bodies: **9 quarter-turns asked, 0
refused**, and 8 of 14 settled pieces standing on end. `spin` accrues from
fall speed at 0.012/cell, so a piece dropping onto its own pile never
completes a turn.

**The seed is angular acceleration, not torque.** `α = Σmgd / Σmr²` about
the break, both sums over cells `Failure::region` already hands over. Mass
cancels: for a limb of length `L` breaking at one end, `α = 3g/(2L)`.

| piece | spin over a ~26-frame, 50-cell fall |
|---|---|
| 52-cell bole | **0.93 quarter-turns** — upright to lying, exactly once |
| 8-cell limb | 6 quarter-turns — tumbling |

**No tuning constant.** Seeding from torque instead gives `spin ∝ m·d`, so
the heaviest piece spins hardest — which is what `SPIN_PER_SPEED`'s doc
records being tuned away from.

**Three things that are not free, against the first draft's "no new
machinery":**

- there is no rate field — `spin` is the accumulator, and a `spin_rate` has
  to be threaded through promotion;
- **there is no reverse rotation.** `BodyCell::rotated()` is one handedness,
  documented as the single definition of the transform, so "sign from which
  side the centroid lies" cannot be honoured without three forward turns and
  a fit probe on each intermediate pose;
- `rotate_quarter` turns about the **body origin**, not the centroid. For a
  49x48 bole that is a ~50-cell teleport of the far end in one frame, and
  `rotation_fits` checks only the final footprint, not the swept path.

**And spin may not be sufficient.** `open-bugs-handoff.md` §Q is OPEN and is
the owner's own words — *"the long skinny vertical pieces should fall
over"* — recording that the landed-body support model has **no slenderness
ratio, no tipping moment and no bearing width**, so an upright bole is stable
*by the model*. §Q also says the decisive measurement has not been taken and
*"do not tune either system"* until it is. **Stage 4 starts there, not with
the spin change.**

---

## 6. The picture

**Not a downstream stage.** 87% of a felled tree's mass already comes off as
coherent chunks and the pile reads as sand, so every card from every later
stage would be judged through a filter that turns pieces into grit — which is
exactly how the previous attempt failed with 91% of woody mass promoting.

Three sources of the dust, not one:

| source | size, one fell | status |
|---|---|---|
| foliage → `litter`, a `Powder` | 992 cells | **owner ruling: never a powder** |
| sub-`MIN_FRACTURE_CELLS` regions → `deadwood`, a `Powder` | 557 cells | open — 24% of failed regions |
| coherent `log` that was illegible | 868 cells, 15 pieces | **fixed, `3bdf674`** |

The third was per-cell shade re-rolling plus a bark→timber colour flip at
landing, and the owner named it unprompted (*"Why does it change colors?"*).
Fixed as bark-on-the-surface, timber-in-the-interior, on a byte-identical
pile. **The honest reading of that A/B: the tan is gone, the mass reads as
wood, and it still does not read as branches on the ground.**

The first is next and it is an instruction, not a design question: *"The
leaves shouldn't turn to powder ever. They can stay on the branch if that is
easier (but if they do fall off, i feel like they should stay as larger leaf
pieces)."*

---

## 7. The stages

Every stage names what it can and cannot demonstrate, on the card, because
the previous attempt lost two review rounds to cards inviting judgements the
stage could not earn.

**Stage 0 — legibility.** *Partly landed (`3bdf674`).* Remaining: the leaf
tier, and `deadwood` off `soil`'s hue (they overlap almost exactly —
(64,43,26)–(96,66,40) against (56,40,30)–(82,60,44)).
*Bar:* a blind A/B on a **byte-identical settled pile**, which `3bdf674`
established is achievable by preserving the rng draw count.

**Stage 1 — see the stress.** **LANDED 2026-08-29.**
`plant::stress_field`, `OrganismOverlay::Stress` on `L`, and
`filmstrip channel=bend`. Nothing reads the number to decide anything, which
is the point: it exists before the rules that use it.
*Bar, met:* a hand-built cantilever reads hottest at its root and falls
monotonically to zero at its free tip; the ramp's brightest cell is the
field's hottest cell; a free tip lands on the ramp floor rather than a third
of the way up it. Measured on the shipped tree: 3,164 cells, median 1.64,
peak **15,316 at the base of the trunk**, 8% at exactly zero.

Three corrections the build forced, all worth carrying:

- **`SUPPORT_COST_STANDING` is zero, so `support` alone cannot order a
  trunk.** Standing on something is free in `anchor_support` — the field
  answers "can this reach the ground", and a vertical stack always can — so
  every cell of an upright stem reads distance 0 and a crown's load has
  nowhere to flow. Measured before the fix: a crown hung entirely off one
  side produced a moment of exactly **0** at the base. Ties are broken by
  height instead, so at equal distance a cell hands its load to the
  neighbours beneath it. The rank stays strict, so the flow still cannot
  cycle.
- **`channel=stress` was already taken** by `load::evaluate`'s *rock* stress.
  The plant channel is `channel=bend`; two quantities under one name is how a
  reader judges the wrong picture.
- **"Which cells is the load leaving through" is not `support == 0`**, for
  the same reason as the first point, and `CellStress::grounded` exists
  because the conservation guard could not be written without it — summing
  `carried` over distance-zero cells counts a trunk once per storey, measured
  134.5 against a true 22.5.

**Stage 2 — bend.** **LANDED 2026-08-29.** `MaterialDef::stiffness`,
`plant::bend_under_load`, the wind's own term in `stress_field`, and
`BEND=off` as the control. Two materials opt in: `grassblade` at 2.0 and
`leaf` at 1000, each fitted from *its own* measured moment distribution.
*Cost bar, met:* `ascii scene=foraging`, three alternating pairs of one
binary — mean **3.27–3.41 ms off against 3.72–3.98 ms on, +0.46 ms/frame**,
against the +8.0 ms/frame sway figure. The worst frame is not separable from
noise and is not quoted: `mean × frames` is 48,000 ms against a ~40 ms worst,
so nothing pins it.

*Bar, not met, and the reason is the finding:* the GIF of a stand in a
pinned gust does not exist, because **the mechanism is clearance-bound
rather than force-bound and this world has nothing blade-shaped in it.**
Three measurements, each of which changes what stage 3 should aim at:

- **Grass here is not blades.** The whole `scene=grove species=grass` stand
  is eight tufts of about twenty cells, five rows tall. §3's argument —
  "laying a blade over moves its tip two or three cells, not subtle" — is
  sound and has nothing to act on. Measured: 3 successful bends across the
  entire stand.
- **More force buys refusals, not movement.** Tripling `WIND_DRAG` took
  cells wanting to lean from 8 to 13 and refusals from 318 to **651**, while
  successes went 3 → 4. The constant is fitted at 1.0 on that evidence, and
  raising it is not the lever.
- **A dense crown cannot bend at one-cell granularity.** Foliage swings
  as a whole cross-section into empty space, and inside a crown there is
  none: 16,673 cross-sections blocked against 28 successes on
  `scene=fell`. What a real crown does instead is move *together*, which is
  the trunk bending — and the trunk is exactly what cannot bend until
  something stops it (below).

**Wood is deliberately still rigid, and this is not caution.** A bend
relieves its moment by shortening a **horizontal** lever; a trunk's wind
moment is a **vertical** one that a one-cell lean does not shorten. So a
trunk given a stiffness leans again every tick for as long as the wind
blows, with nothing in the model to stop it. What stops a real trunk is that
it breaks. **The wind load itself is already on wood** — `stress_field` puts
the gust's torque into every cell — so stage 3 reads a moment that has wind
in it and needs no further wind work to fell a tree.

Two corrections the build forced:

- **The local air velocity is not the wind.** The obvious read is
  `field_at_bilinear`'s `vx`, which is what `wind_lean_dir` uses for growth.
  Over every cell of living tissue in a grass stand it measures **`vx` from
  -0.040 to +0.077, median 0.000** — the momentum solve holds
  no-penetration at the ground and plants grow at the ground. A term built
  on it is a reader with no writer. What the rest of the engine means by
  surface wind is `weather::at().wind` times the column's `exposure`, and
  that is what this reads.
- **A stiffness must be fitted against its own material's moments, not the
  stand's.** Trunks dominate a pooled distribution and foliage sits two
  orders of magnitude below it; `filmstrip`'s moment line is now split per
  material for exactly this reason.

**Stage 3 — break.** **LANDED 2026-08-30.** `MaterialDef::strength`,
`plant::break_under_load`, `structural::snap_organism_cell`, and `BREAK=off`
as the control. Only `wood` opts in.

*Bars, met:* a hand-built over-reaching branch breaks **at its root, not its
tip**; a snow-loaded crown snaps its trunk, paired against a bare crown that
holds. *Cost:* nil — `ascii scene=foraging`, three alternating pairs, mean
3.61–3.84 ms off against 3.44–3.81 on, the arms overlapping.

**The third bar was withdrawn by the owner and the reason matters more than
the bar did.** It read *"an intact healthy stand loses zero cells over 12,000
frames"*, and it was met — zero across three seeds. It was still wrong:
> *"a tree that grows poorly then that it's a heavy top should be able to
> break on its own... eventually wind or self weight should break a poorly
> grown tree"*

A stand where nothing ever breaks is the binary outcome this project's ethos
forbids, and the bar as written selects for it. Replaced by: **a healthy
stand survives**, a badly-proportioned minority does not.

**How the constant was got wrong, because the shape of the error is the
transferable part.** Fitted first from the *stand maximum* — 60,000, above
every reading of a 15-sample sweep. That statistic is one cell out of eight
thousand, and it belongs to the single worst-grown individual in the wood:
**exactly the tree that ought to fail**. So the constant was set above the
whole failure mode. Nothing broke, and — the owner's own question, which the
measurement had not been asked — *wind could not break anything either*,
because the ceiling it was fitted against already had a gust in it. A number
can be measured, headroomed and defensible and still be fitted to the wrong
population.

The fit that works is the **per-plant** peak, pooled across eight seeds,
71 plants:

| p50 | p75 | p90 | p95 | max |
|---|---|---|---|---|
| 6,735 | 10,094 | 21,032 | 22,925 | 31,160 |

Set at **21,000** — p90. The owner asked for p75 on a selection argument,
*"I expect plants to evolve to be stronger and the damage rate will drop over
time"*, and **the stand does not survive p75 long enough to select**. One
seed, 160,000 frames, against a `BREAK=off` control on the same seed:

| frames | control | p90 (shipped) | p75 |
|---|---|---|---|
| 20,000 | 17,336 | 17,476 | 16,898 |
| 60,000 | 19,194 | 17,085 | 8,790 |
| 100,000 | 18,583 | 15,858 | 7,763 |
| 160,000 | **19,123** | **14,859** | **4,057** |
| snaps | — | 16 | 278 |
| plants | 9 | 7 | 5 |

At p75 the wood loses 79% of itself and the stand falls from nine plants to
five. The control holds flat, so that is the rule and not the lifecycle — and
**a population culled faster than it breeds crashes before selection can
operate**, so the owner's own mechanism is an argument *for* the gentler
constant. At p90 the stand holds at ~78% of the control while still losing
sixteen limbs, and the per-plant peak drifts down (p90 15,306 → 10,446) —
consistent with selection on `pipe_ratio`, though culling the
worst-proportioned trees produces the same trend and separating the two needs
the genotype tracked across generations.

**And the pressure is aimed at shape, not at density.** Scaling `strength`
by `organism::wood_density` was built and removed:
> *"I care more about mechanics impacting growth patterns so I don't want
> density dominating this evolutionary pressure."*

Density is a *discrete* allele worth a straight 1.8x on the threshold, so one
mutation buys the whole escape and shape never changes. With it absent the
only ways out are a smaller moment or a bigger section — and section is the
steeper gradient anyway, since `stress` is `|moment| / section²` and doubling
girth *quarters* it. Girth is `pipe_ratio` at genome slot 4, whose
`genotype_variance` is **0.7, the widest of any trait in `tree.ron`'s
tuple**. Density keeps its job on the span rule.

Three findings the build forced:

- **The trigger problem in §4 dissolved rather than being solved.** That
  section worries that growth raises the moment and never raises `support`,
  so a load criterion has no trigger. True of the *reactive per-cell* path;
  `break_under_load` runs at organism cadence over the whole stress field, so
  a load rise is picked up by construction. Scheduling is still needed for
  the **consequence** — the detached limb coming down — which
  `snap_organism_cell` handles by recording a disturbance and fanning out.
- **A snap is its own disturbance, which is what answers §4's last line.**
  `organism_structural_tick` refuses anything outside `within_disturbance`,
  so a limb snapped by wind in a quiet wood would hang at every `chain_reach`
  setting but the default. A snap is an event, so it reports itself as one,
  with the wound it does: a single cell.
- **`supported_load` is not replaced.** §4 asks for it because it assumed the
  new criterion would *supersede* `effective_span`; breaking is added
  alongside, so the span rule and its snow term are untouched. What ask 1
  needed instead was for snow to raise the **moment**, which is
  `plant::surcharge_mass` — the same idea in the plant's own mass units,
  sharing `POWDER_SURCHARGE_CAP` so the two rules never disagree about how
  far up a column they look.

**Stage 4 — the fall.** §Q's measurement first; then `α`-seeded rotation,
rotation about the centroid, and the tipping test §Q says is missing.
*Bar:* settled pieces lying vs upright (today 2 lying / 9 upright / 4
square) **and** quarter turns asked vs refused, printed together. Turns
asked that has not moved means nothing fired.

**Stage 5 — the verbs.** Chop with a **directional notch** (cut one side,
it goes that way — the verb ask 3 is missing, and nearly free once the moment
exists); impact, with a visible mark at the contact; wind, with the four
rungs censused **by rung** and something visible at the gust; and the **root
plate** for an uproot, so a snap and an uproot are not the same picture.
`OrganismState::anchor_cells` already enumerates the plate.

---

## 8. What this is not doing

Each a recorded do-not-retry or a measured kill: a constraint solver
(category error at cell scale); free-angle grid rotation (twice); a
mass-spring tree; sim-side per-frame sway (four fatal counts); a steady wind
field term (3.55 ms/frame, reverted four times); a physically-derived debris
distribution; per-material `MAX_BODY_CELLS`; **growth and the carbon economy**
(owner ruling, §0).

---

## 9. What the instrument does and does not measure

`examples/beam_probe.rs` produced §2c's numbers and **its section measure is
not the one §2a proposes.** It takes a binary horizontal/vertical run
perpendicular to an arbitrary 8-BFS parent link; where that parent arrives
diagonally the walk runs *lengthwise down the stem*, which is the failure
`plant.rs:6086` records as having happened three times already. It also
computes a narrowest-chord alternative and discards it, and defines a
`cross_section_axis` version it never calls. It applies none of `load.rs`'s
clamps (the vertical kern clamp, `arch_span`, `bearing_moment`, the
section-share block).

**So §2c's `k` window is an artifact and must not be used to set a
constant.** What survives is the qualitative shape: the quantity
discriminates, and it reads ~0 at the base of a balanced stem — which is §4's
whole argument and does not depend on the section at all.

---

## 10. What changed from the first draft, and why

| the draft said | corrected to | who caught it |
|---|---|---|
| growth sizes wood on mechanical demand | growth is out of scope | owner |
| grass and foliage opt out | every plant participates; grass bends | owner |
| the leaf tier is a maybe, last | never a powder, and it is next | owner |
| accumulate `M`/`Sx` up `accumulate_support`'s forest | that is a spanning tree over a blob, a recorded dead end; port `load.rs`'s share division | mechanics review, prosecutor |
| delete `max_cantilever_reach` | it is foliage's opt-out and it is the slenderness rule; multiply it by the section | prosecutor, mechanics review |
| ask 1 comes from the bending moment | the base reads ~0; ask 1 is buckling | mechanics review |
| seed rotation from breaking torque | from angular acceleration; torque spins the heaviest pieces hardest | mechanics review |
| the fit probe is not the blocker (9 asked, 0 refused) | those 9 were in open air; §Q says landed pieces have no tipping test at all | prosecutor |
| stage 1 is inert and byte-identical | re-rooting the walk changes `q_peak`/`q_now`, which are the maintenance bill | prosecutor |
| the picture is stage 6 | it gates every other card | player-feel review |
| chop needs nothing new | it is missing its verb — a directional notch | player-feel review |
| a bend is impossible at 1 px/cell | true of a trunk, false of a blade of grass | owner, player-feel review |

---

## 11. Freshness

Written 2026-08-29 on `claude/tree-physics-destruction-l6eocs` at `3bdf674`,
current with `main`. §1's source readings and §2c's, §5's and §6's
measurements were taken this session on this machine; §9 states which of them
the instrument can carry. Nothing in §3 or §4 has been built, rendered or
judged.
