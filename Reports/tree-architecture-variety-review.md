# Where tree variety actually comes from, and a review of the substrate under it

Independent review of `plant-substrate-v2`, requested against
`Reports/tree-architecture-implementation-plan.md`'s "where this branch
actually stands (2026-08-16)". Two halves, and they are not equally
weighted: a correctness pass on `FieldTile::occupancy` and the six stated
concerns, then the main question — **why jittering six scalars around one
growth model cannot produce botanical variety, and what would.**

Nothing on the "do not re-litigate" list is reopened. Where this report
touches one of those settled findings it is to *extend* it, and it says so.

Everything below is marked **measured** or **argued**. Measured means it was
run in this worktree this session; the harness invocation is given.

---

## 0. The findings, stated first

**On variety, which is the ask.** The literature's own answer to "what makes
two trees different kinds of thing" is not a parameter distribution. Hallé,
Oldeman & Tomlinson resolve the whole diversity of tree form onto
**four or five discrete axes** and get 23 models out of them; Prusinkiewicz
et al. reproduce all 23 from "relatively simple parametric variations" over
those same axes. The branch currently sits at **one corner of that space**
— monopodial, orthotropic, continuous, acrotonous-by-accident — and every
genome ever drawn has been a perturbation *within* that corner. The six
jittered scalars move size and proportion. They cannot move corner.

> **The single most valuable structural fact found in this review: the
> apex already dies every growth step.** `Grow` retires the parent cell to
> `MatureBody`/`DormantBud` and creates the frontier as *children*
> (`plant.rs:1077`, `:1143`). The engine is therefore *physically* sympodial
> already, and only *labelled* monopodial — the primary child inherits
> `order` and `heading` while the lateral gets `order+1` and a fresh
> heading. **Sympodial branching is a four-line change and one species
> flag**, and it is the largest silhouette lever available anywhere in this
> codebase per unit of work.

**On the substrate.** Three defects, each of which bears directly on a
stated concern rather than being incidental:

1. **`occupancy` is orientation-blind, and it is wrong in exactly the
   direction of the artifact being fought.** A block holding a horizontal
   1-cell leaf plate spanning all 8 columns and a block holding a vertical
   8-cell trunk both read `8/64 = 0.125` and both pass 90% of the light. The
   trunk answer is right; the plate answer is 4.5x too bright, and the flat
   canopy plate is precisely the thing `crowding_weight` was pushed to 12.0
   to suppress. Fix is free — count *columns hit*, not cells filled.
2. **The crowding cliff is a code artifact, not a balance point.** Crowding
   is subtracted from a score that is then filtered by `score > 0.0`
   (`plant.rs:952-959`). Past a threshold the term does not bias the choice,
   it **empties the candidate set** — the tip dead-ends, ages out on
   `ORGANISM_STALE_LIMIT`, and the tree strangles. That is the measured
   collapse at 20.0 (median tree 26 cells), and it is a property of
   subtract-then-filter, not of crown shyness. A multiplicative form has no
   cliff at any weight.
3. **Every economic quantity in the plant is proportional to a 20:1
   oscillator, and the harness samples it at an arbitrary phase.** Measured
   this session, same scene, same species, paired: **71 live `GrowingTip`s
   at frame 28,800 (noon) against 28 at frame 30,000 (night)** — 2.5x, on a
   stand that is 4% *smaller* at the noon sample. This is the deepest form
   of `PLAN.md` 0e and it is the mechanical explanation of the constant
   treadmill in §2.1.

---

## 1. `FieldTile::occupancy` — the invariant change (concern A)

### 1a. The change is right, and the reasoning behind it is right

An occluder should read the light arriving *at* it, because it is the thing
doing the intercepting. The previous rule — skip blocked blocks entirely —
made a leaf's own reading a function of lateral diffusion only, which is a
quantity with no physical relationship to what that leaf absorbs. The
commit message's account of this is correct and the rewritten guard test
(gradient preserved, occluder widened to nine blocks so diffusion cannot
fill the shadow from both sides) is the right test to have kept.

**Verified this session:** `cargo test --release --lib` — 390 passed, 0
failed, 1 ignored. `cargo clippy --all-targets -- -D warnings` — clean.
`plant_probe -- trees=8 frames=30000` reproduces the branch's stated
headline numbers exactly (mean 2,541 cells, 1,064 leaves, median stem 15,
thickest contiguous run 51, canopy top row 69).

### 1b. `occupancy` counts something other than its name — twice

The doc comment says "how full this block is". The code
(`field.rs:985-988`) counts `Solid | Plant`, plus out-of-bounds. Two
consequences, and the first is the important one.

**(i) It is orientation-blind, and a downward cast is not.** Occupancy is
`filled / 64` over the whole 8x8 block. For a *vertical* ray only the
horizontal extent of the occluder matters:

| block contents | `filled` | occupancy | transmission | should be |
|---|---|---|---|---|
| vertical 8-cell trunk, 1 column | 8 | 0.125 | 0.90 | 0.90 ✔ |
| horizontal 8-cell plate, 8 columns | 8 | 0.125 | 0.90 | **0.20** |
| solid rock | 64 | 1.000 | 0.20 | 0.20 ✔ |

The trunk case is correct by luck — for a vertical stick, cells-filled and
columns-hit happen to agree. The plate case is wrong by 4.5x, and a flat
horizontal canopy plate is the exact geometry this branch has fought three
separate times (`plant-species-authoring.md` §2). **The light model is
under-charging the artifact it is supposed to bound.**

The fix costs nothing and reuses the scan that already runs. In
`rebuild_blocked`'s `dy`/`dx` loop, replace the counter with a column
bitmask:

```rust
let mut cols: u8 = 0;            // instead of `let mut filled = 0u32;`
...
if matches!(kind, Solid | Plant) { blocked = true; cols |= 1 << dx; }
...
tile.set_occupancy_local(lx, ly, cols.count_ones(), FIELD_SCALE as u32);
```

Same loop, same reads, one `u8` of stack. `FIELD_SCALE` is 8, so the mask
fits exactly. **This is a claim that should be A/B'd on tree outcomes, not
on a depth profile** — per the plan's own §0f discipline. The prediction is
specific and falsifiable: self-shading gets teeth, `crowding_weight` no
longer has to carry the whole bounding job, and the usable band widens
downward from 12.0.

**(ii) Powders and liquids are transparent.** `soil.ron` is `kind: Powder`,
so soil contributes nothing to `blocked` *or* `occupancy`. `apply_sky` casts
`amplitude` straight down through the entire soil bed to the world floor.
With `NIGHT_LIGHT_FLOOR: 0.2` and `Germinate { light_threshold: 0.1 }`,
**a seed buried at any depth in soil passes the light gate at midnight.**
The plan records this gate as "unreachable"; it is the reverse — it is
unconditionally satisfied, everywhere, and has been degenerate the whole
time.

This is pre-existing rather than caused by the occupancy change, but the
change makes it worth fixing, because occupancy is now the natural place to
express it. Counting *any* non-`EMPTY` cell into the column mask (while
leaving `blocked` on `Solid | Plant` alone, since `blocked` means
"impassable to the pressure/velocity solve", not "opaque") gives soil and
water real opacity for one extra comparison. **Flag the interaction before
doing it:** it flips the germination gate from always-pass to never-pass
for a buried seed, and `common::PlantScene` drops its seeds onto the
surface, so check what `germinate` actually needs before changing it.

### 1c. The consumers you have not exercised

You asked specifically about heat, pressure and moisture. **They are not
affected**, and the reason is worth writing down so nobody re-checks it:

- `apply_sky` writes only `cell.light`. `occupancy` has exactly one reader
  (`field.rs:816`).
- `step_pressure` (`:1038`), `step_velocity` and `step_diffusion` all gate
  on `blocked`, which is unchanged in both definition and value.
- `apply_moisture_sources` deliberately ignores `blocked` and reads
  `moisture_source`, untouched.

So the blast radius is the light channel alone. Within it, the consumers
that *did* change behaviour and are worth a look:

| reader | effect of the change |
|---|---|
| `plant::ambient_light_above` | see §1d — the offset is now stale |
| `Germinate.light_threshold` | already degenerate (§1b(ii)); unchanged |
| `plant::shade_factor` (moss) | moss under a canopy is now less shaded; `moss_spreads_over_damp_stone…` still passes |
| `render.rs` `FieldOverlay` | a solid rock face now draws lit on its sky-facing block. Cosmetic, but it will look like a bug the first time somebody sees it |
| `fire.rs` light emission | uses `add_light`, independent |

### 1d. `ambient_light_above`'s one-block offset is now a stale workaround, and it costs one full block of self-shading

`ambient_light_above` (`plant.rs:522`) samples `y - FIELD_SCALE`. Its doc
justifies that offset explicitly:

> "…a plant cell reading `field_at` at its own exact position always lands
> inside a block its own material just made opaque, and reads a permanent
> `0.0` regardless of how bright the sky is one cell away."

**That is no longer true.** `apply_sky` now writes the arriving light into
occupied blocks precisely so a leaf can read its own position. The offset
survived the change, and what it now does is make every leaf read the light
one field block *above* itself — i.e. **before its own block's attenuation
is applied**. Every leaf in the plant systematically over-reads its income
by exactly one block of self-shading, which is the strongest term in the
feedback that is supposed to bound the plant.

This is `CLAUDE.md`'s "fixing a bug often exposes a constant that was
compensating for it", one level up: it exposed a whole *mechanism* that was
compensating for it. Deleting the offset is a one-character change and it
will move `LEAF_INCOME_PER_TICK` again — which §2.1 argues is the actual
disease.

### 1e. Two smaller things in the same area

- **`apply_sky` never darkens** (`if carried > cell.light`), by design, so
  the light channel falls only through `step_diffusion`'s averaging. At
  dusk the amplitude drops 20:1 in a few hundred frames while the deep
  field relaxes over ~3,300; the plan already records this as 0e. Worth
  noting the *sign*: the field lags bright, so night is systematically
  brighter than `sky_light_amplitude` says.
- **`if carried <= 0.0 { continue }`** inside the `ly` loop should be
  `break` out of the `cy` loop for the column — `continue` re-tests 63 more
  times per tile and every tile below. Unreachable today (`SKY_TRANSMISSION`
  is 0.2, never 0), so this is tidiness, not a bug.

---

## 2. The six concerns, answered

### 2.1 The constant treadmill — **not genuine coupling. It is an unnormalised currency.** (concern B1)

You asked to be told plainly if the answer is "they are genuinely coupled,
stop treating it as a smell". The honest answer is **partly, and the part
that is genuine is not the part that keeps costing you re-derivations.**

**What *is* genuinely coupled, and should stay coupled.** Income and girth
both consume the same quantity `Q` — intercepted light accumulated
basipetally. That is not an accident of this implementation; it is the
pipe model and the carbon economy sharing a variable *on purpose*, in every
model in `tree-procedural-prior-art.md`. Shinozaki's cross-section and
Palubicki's `v` are two consumers of one `Q`. Trying to make `pipe_ratio`
independent of the economy would be modelling a tree whose trunk width has
nothing to do with its crown, which is the thing the whole of Phase 3
existed to fix. **Leave that one alone.**

**What is *not* genuine.** All three constants are *dimensional conversion
factors denominated in raw units of a quantity whose scale is set
elsewhere*:

| constant | converts | scale set by |
|---|---|---|
| `LEAF_INCOME_PER_TICK` | summed field light → carbon | `MAX_LIGHT`, `leaf_cluster`, the sky model, the day/night phase |
| `pipe_ratio` | summed field light → cells of stem width | the same four |
| `crowding_weight` | `canopy_density` → growth score | `GROW_CANOPY_DEPOSIT`, `CANOPY_DENSITY_DECAY_PER_TICK`, tick cadence |

So a change to `field.rs` moves the numerator of the first two; a change to
`leaf_cluster` multiplies it by five; a change to tick cadence moves the
third. **None of those are changes to the plant's biology, and all three
invalidated the constants anyway.** That is the treadmill, and it is a
units problem, not a coupling problem.

**The evidence is in `tree.ron` itself.** The `SecondaryThicken` block
carries a seven-row sweep ending `per-stem 45 … <-- here`. The live value
is `110.0`. The sweep documents a `leaf_count` denominated in *leaves*; the
live quantity is `q_peak`, denominated in *summed light*, worth up to
`MAX_LIGHT = 4.0` per leaf. The table and the value are measuring different
things and the file no longer records where 110 came from. This is not
sloppiness — it is what happens when a constant's units are defined by four
other files.

**The fix is normalisation, and it is small.** Define one canonical unit:
the light one healthy leaf cluster intercepts in open sky at noon,
`L_node = MAX_LIGHT * leaf_cluster`. Then

```
income     = (intercepted / L_node) * income_per_node
pipe_ratio = q_peak / L_node                       // "nodes of foliage per cell of stem width"
```

`income_per_node` and the normalised `pipe_ratio` are now **invariant under
every change that has moved them**: the light model, `MAX_LIGHT`,
`leaf_cluster`, and the day/night convention. They also become *readable* —
the normalised pipe ratio is a Huber value, a number with a literature
range, instead of 110.

Do the same for crowding: `candidate_crowding` returns a mean of decayed
`canopy_density` values with no fixed scale, while every other term in the
score is `dot(dir, ref) * weight` with `|dot| <= 1`. Divide by
`GROW_CANOPY_DEPOSIT` and the term lands in `[0, 1]` like its siblings, and
`crowding_weight: 12.0` becomes a number comparable with
`upward_weight: 0.9` instead of one that has to be thirteen times larger to
mean the same thing.

**Acceptance test, and it is a good one:** after normalisation, changing
`leaf_cluster` from 5 to 1 should leave mean tree size roughly unchanged.
Today it changes it fivefold, which is recorded in
`LEAF_INCOME_PER_TICK`'s own doc as a thing that had to be absorbed by hand.

**Cost:** two divisions per organism tick. Free.

### 2.2 The crown-shyness cliff — **yes, it is doing more than one job, but the cliff is not why** (concern B2)

Two separate things are true and they have been conflated.

**The cliff is a code artifact.** `plant.rs:952-959`:

```rust
let score = dot(dir, heading) * continuation_weight
          + ... - density * crowding_weight;
if score > 0.0 { candidates.push(...); }
```

and then `if candidates.is_empty() { continue; }` — the tip fails, banks a
`stale_tick`, and retires permanently after `ORGANISM_STALE_LIMIT` (4). So
crowding is not a preference that saturates; **past a threshold it is a
prohibition that kills lineages.** The positive terms sum to at most
`continuation_weight + light_weight + wind_weight + upward_weight` ≈ 1.15
at the outer orders, so a `crowding_weight` of 20 needs only
`density > 0.058` to zero every direction at once. That is the measured
collapse to a median of 26 cells, and it is arithmetic, not ecology.

**A multiplicative form has no cliff at any weight:**

```rust
let score = (dot(dir, heading) * continuation_weight + ... )
          / (1.0 + density * crowding_weight);
```

Now crowding can be arbitrarily strong and still only ever *reorders*
candidates; a fully crowded tip takes the least-bad direction rather than
dying. The knob becomes monotone with no collapse to sit under, which is
worth having on its own terms — `plant-species-authoring.md` §2's rule
("make the bound graded") applied to a bound that is currently a step.

**And yes, it is doing three jobs**, which is the reason it had to be 12.0:

1. self-avoidance (crown porosity within one plant),
2. crown shyness (separation between plants),
3. **bounding total lateral canopy extent** — a job that belongs to
   self-shading and has been vacated, because §1b(i) makes a horizontal
   plate nearly transparent and §1d discards one more block of it.

Jobs 1 and 2 are legitimately one rule and the owner-blind design is right
for the reason `plant-species-authoring.md` §3 gives (a phytochrome cannot
tell whose leaf reflected the far-red). **Job 3 is the impostor.** Fix
§1b(i) and §1d and job 3 goes back where it belongs; then re-derive
`crowding_weight` a *third* time, which sounds like the treadmill but is
not — it is the first re-derivation where the quantity being opposed is
actually being measured.

### 2.3 One species, one scene — **the studies are sound; their generality is unmeasured, and one specific conclusion is at risk** (concern B3)

Both 1,024-genome studies are methodologically strong, and
`plant-species-authoring.md` §7's "sample the population, not the
parameter" is a genuinely good idea that I would not change. The exposure is
narrower than "which conclusions survive":

- **Robust to scene, argued:** `plastochron` strongest, `branch_chance`
  second, `turgor` reads as height. These are direct structural relations
  (leaf spacing → income; shoots → foliage; turgor → height) that do not
  route through the stand.
- **At risk, and the one I would re-run first:** `light_weight` measured
  inert. Your own queue item 2 already flags this, and §1b(i) makes it more
  urgent than it looks — on flat ground with a uniform sky and an
  under-attenuating canopy there is genuinely no gradient. **On a slope, or
  beside a clearing, there is one by construction.** Do not conclude
  phototropism is inert until it has been measured somewhere with a
  horizontal light gradient. It is currently held at variance 0.0, which is
  the right way to park it.
- **Untested and likely to move:** everything that depends on spacing being
  uniform. `crowding_weight` was derived against a regular 57-cell pitch.
  A clearing is a spacing discontinuity and is exactly where crown shyness
  should *stop* applying.

**Cheapest thing that buys the most:** add a `slope` and a `gap`
(one missing tree) variant to `common::PlantScene`. That is a scene
parameter, not a mechanism, and it turns three untested conclusions into
tested ones for an afternoon. It is also the only way to measure whether
the variety mechanisms in §3 do anything, since several of them
(plagiotropy, reiteration) are specifically responses to *asymmetric*
light.

Mixed ages and a second species are a bigger ask and I would defer both
until §3's discrete axes exist — a second species that is a parameter
perturbation of the first is not a second species, which is §3's whole
argument.

### 2.4 Determinism vs. variety — **position-keying is right, and there is a worse problem than the one you named** (concern B4)

**Today, `organism_id` is exactly what you think it is**, and slightly more
fragile. `push_organism` (`world.rs:487`) pops `free_organism_slots` first;
that list is never populated (there is no `free_organism` yet), so ids are
currently strictly monotonic in planting order. So genotype is a pure
function of *planting index in the world's whole history*.

That has a consequence worse than "the same trees every playthrough":
**it is not stable under any upstream edit.** Plant one extra sapling
anywhere, earlier, and every subsequent organism in the world shifts one
slot and the entire forest redraws its genotypes. Worldgen changes, player
planting, and organism death all renumber everything downstream of them.
The genotype of a tree is a property of the world's event history, not of
the tree.

Two more things to know before this is designed:

- **`decode_organism_id` wraps generation at 4 bits.** Once `free_organism`
  exists, a slot reused 16 times produces a *bit-identical* `organism_id`,
  and therefore an identical genotype. With 12 index bits, replanting churn
  in one area reuses nearby slots, so exact genotype repeats will be
  spatially clustered — visible in exactly the way random repeats would not
  be.
- **There is no world save/load in this codebase at all.** I checked: no
  `Serialize`, no bincode, no save path; `serde` appears only for species
  and tunables RON. So the question is not "is it stable across save/load",
  it is "what must save/load preserve". Answer, if genotype stays keyed on
  id: the `organisms` `Vec` *by index*, every slot's `generation`, and
  `free_organism_slots` **in order**. That is a strong constraint to place
  on a serialiser that does not exist yet, for no benefit.

**Recommendation: key the genotype on world position.** Use the seed's
germination coordinate (not the planting coordinate — a seed is a `Powder`
and rolls) hashed with the world seed:

```rust
pub fn genotype_at(world_seed: u64, gx: i32, gy: i32, salt: u64, variance: f32) -> f32
```

This gives you every property you want and costs one `i32` pair stored on
the organism at germination:

- **Stable under save/load by construction** — position is already
  serialisable, and a save that reproduces the grid reproduces the genotype
  with no extra invariant.
- **Stable under upstream edits** — planting a sapling elsewhere does not
  redraw the forest.
- **Same world, same trees** — which you *want*; that is determinism, and
  `PLAN.md` requires it.
- **Different world, different trees**, from the world seed alone.
- **Genuinely more varied in play**, because a replanted tree in a new spot
  is a new individual rather than "whatever index the counter is on".

The one thing it costs: two trees germinating at the same cell in a
long-lived world draw the same genotype. Mixing the world frame at
germination into the hash fixes that and breaks save/load stability again,
so I would not — a repeat in the same spot decades apart is not a visible
defect.

**But note what this does *not* fix**, and it is the point of §3: keying
differently changes *which* draw each tree gets, not *what a draw can be*.
Every tree is still the same architectural model. Position-keying is worth
doing because it makes save/load cheap; it is not a variety mechanism.

### 2.5 `q_peak` monotone forever — **no conflict. The biology already splits the two quantities you are trying to get from one** (concern B5)

Palubicki's rule is about **rendered branch width**, and it is right:
*"branch width is not decreased when leaves and branches are shed"*. A trunk
does not thin in autumn, and it does not thin when a limb breaks, because
the wood is still there. Keep `q_peak` monotone and keep it driving girth.

The mistake is asking the *same* scalar to be the vigour signal. The pipe
model's own refinement makes the split for you: Shinozaki's pipes above a
shed branch become **disused** — the stem stays as wide (heartwood is
retained mechanically) while the *conducting* cross-section shrinks. So:

| quantity | what it is | behaviour | drives |
|---|---|---|---|
| `q_peak` | all pipes ever built | monotone max | **girth** — never shrinks |
| `q_now` | pipes still supplied | instantaneous | **vigour** — bud break, allocation |
| `q_peak − q_now` | disused pipes | closes as foliage returns | **the damage signal** |

This resolves your worry directly. The deficit is **not** permanent: it is
`q_peak − q_now`, and it closes on its own the moment `q_now` recovers. A
plant is "damaged" exactly while it is carrying less foliage than it once
did, which is the correct meaning. The only permanent-deficit case is a
plant that genuinely cannot recover its former crown — which should
permanently behave like a damaged plant, because it is one.

**This costs zero extra storage.** `accumulate_support` already builds the
instantaneous `q` vector (`plant.rs:1719-1734`) and then throws it away
after the max-accumulate. Hand it to `break_buds` in the same tick — the
two run consecutively in `step_organisms` — and the damage signal exists
for the price of a function argument.

**And it fixes the known defect in `break_buds` without adding stock to the
numerator**, which is the trap that function's own doc documents (38,605
cells against 1,723). Drive the flush budget on
`max(q_now, α · q_peak)` rather than on stock: a plant that has lost
foliage mobilises toward what it *used to* support, a plant that never had
any does not, and the two are told apart by memory rather than by a
reserve. That is exactly the distinction the doc says it needs and could
not compute.

**Where the monotone mark is genuinely costing you** is queue item 5
(taper), and you have already identified it: anything a cell earned at any
instant it keeps forever. Note the sharper version — because light swings
20:1 over the day (§2.6), `q_peak` is a **noon-only** quantity latched at
the brightest instant of the brightest tick. `pipe_ratio: 110` is roughly
20x what it would be against a daily mean, and that factor is pure
day/night convention. If taper stays too uniform after 0e is fixed, the
remaining lever is to make the mark leaky on a *very* long time constant
(sapwood → heartwood conversion is a real process with a real rate), not to
abandon monotonicity.

### 2.6 2D foliage — **`leaf_cluster` is defensible; the framing is not, and there is a better one** (concern B6)

The argument in `tree.ron` is sound as far as it goes: at this cell size a
leaf spray genuinely is larger than the twig bearing it, and one green cell
per node renders as a bare skeleton with specks. `plant-species-authoring.md`
§5's separation — a cluster is a correction to *visual scale*, not to the
economy — is the right instinct, and holding `LEAF_INCOME_PER_TICK` per
*cell* so a node still earns one node's worth is the right implementation
of it.

**The better framing is that a 2D slice is not a cross-section of a 3D
tree; it is a projection of one.** That reframing is not cosmetic — it
changes what the right answer is:

- A **cross-section** through a crown cuts a few twigs and a lot of air.
  Scaling foliage up is compensating for a sampling artifact, and the
  compensation is arbitrary (why 5?).
- A **projection** of a crown onto a plane is what a silhouette *is*, and it
  is nearly opaque, because the leaves at every depth overlap. There is
  nothing to compensate for — the crown genuinely fills its outline.

Under the projection reading, `leaf_cluster: 5` is not a fudge factor, it is
an **optical depth**: how much foliage the third dimension contributes per
node that the 2D grid cannot store. That gives it a defensible value
(the ratio of crown volume to crown slice thickness — roughly the crown's
depth in cells, so 5 is low rather than high for a big tree) and, more
usefully, tells you where it belongs: **in the light model, not in the cell
count.** A `Leaf` cell should carry an occlusion weight > 1 in the occupancy
mask rather than being drawn five times.

That is a strictly better trade in this engine's terms:

| | five cells per node | one cell, occlusion weight 5 |
|---|---|---|
| foliage reads as a mass | yes | **no** — needs a render change |
| self-shading | five cells' worth | five cells' worth |
| cells in the world | 5x | 1x |
| `thicken`'s `can_widen` | five cells blocking sides | one |
| `accumulate_support` cost | O(5N) | O(N) |

The catch is the first row, and it is the one that matters to a
`CLAUDE.md`-shaped project: the picture. **So I would not change this now.**
The recommendation is to *name* it correctly — `leaf_cluster` is a render
and occlusion quantity, not an economic one — and to revisit it if crown
cell counts ever become a frame-cost problem, at which point the answer is
"draw a leaf cell fatter" rather than "spawn five".

---

## 3. Real growth variety — the main ask

### 3.0 The finding

> **Botanical variety is discrete, not continuous.** Hallé, Oldeman &
> Tomlinson enumerate **23** architectural models and claim they cover the
> diversity of trees in nature. The discriminating criteria are a handful of
> categorical choices: *type of growth* (rhythmic / continuous),
> *branching* (present / absent, terminal / lateral, monopodial /
> sympodial), *axis differentiation* (orthotropic / plagiotropic), and
> *position of sexuality* (terminal / lateral). Prusinkiewicz et al.
> reproduce all 23 from "relatively simple parametric variations" over
> those same axes.

Three of those four axes are vegetative and this engine could express all
three. The fourth (sexuality) is a flowering model and is out of scope.

**Why this is the right frame for the stated goal.** The field data agrees
on the ordering, too: a survey of crown-architecture variation across 342
species reports that shifts along environmental gradients reflect **both
species turnover and intraspecific plasticity, with intraspecific
plasticity of secondary importance**, and a study of crown architecture
against competitive position concludes that **developmental constraints
matter more than competitive position**. Discrete architecture first,
plasticity second, genotype third — which is the exact inverse of the
branch's current investment.

### 3.1 Monopodial vs sympodial — **the cheapest large lever in the codebase**

**What it is.** Monopodial: one apical meristem continues indefinitely and
laterals are subordinate (spruce, pine, most conifers — Rauh's and
Massart's models). Sympodial: the apex stops and is replaced by one or more
equivalent laterals, so the axis is built of stacked modules
(Leeuwenberg's model — frangipani, cassava, most large-leaved shrubs;
Champagnat's — arching and weeping habits; Troll's — much of the temperate
broadleaf flora, where the "trunk" is assembled from plagiotropic units
that straighten).

**Why it is nearly free here.** The engine already retires the apex on every
successful growth step:

```rust
// plant.rs:1077 — the parent, after growing
let self_type_after_grow = if cell_type == GrowingTip && leaf_due { DormantBud }
                           else if ... { MatureBody } ...
```

Monopodiality is not in the physics; it is in **two lines of labelling**.
The primary child gets `write_order(tx, ty, order)` (`:1110`) and the
parent's blended `heading` (`:1120`); the lateral gets
`order.saturating_add(1)` (`:1180`) and a fresh heading (`:1188`). A
sympodial species is one where, **on a branch event, the primary child is
treated as a lateral too** — both children take `order+1` and a fresh
heading, and neither inherits the axis.

**And `ByOrder`'s saturation makes it come out right for free.** `at()`
clamps at `BRANCH_ORDERS - 1`, so a sympodial plant runs to the last tier
within a few forks and stays there — every module thereafter governed by
identical parameters. **That is Leeuwenberg's model exactly**: a plant built
of equivalent repeated modules. The existing data structure produces the
correct botany without being asked to.

| cost | |
|---|---|
| per-cell state | **none** |
| species parameters | one flag, ideally `ByOrder<bool>` so the trunk can be monopodial and the crown sympodial (Scarrone's model — mango) |
| new passes | **none** |
| frame cost | **zero** — same branch, different constant |
| lines | ~4 in `Grow`, plus the `.ron` field |

**Silhouette impact: the highest available.** A sympodial crown is forking,
tiered and roughly equal-armed; a monopodial one has a dominant axis with
subordinate limbs. They are not two settings of one tree, they are two
kinds of object. **Rank 1.**

**Acceptance, and it needs a counter, not just a picture** (`CLAUDE.md`:
"did it fire at all"): print the count of branch events resolved
sympodially, next to the contact sheet. A sympodial run whose counter reads
zero is a monopodial tree that happened to fork.

### 3.2 Orthotropic vs plagiotropic — **the missing silhouette lever, and you are right that it is the biggest one you do not have**

**What it is.** Orthotropic axes grow vertically with radial symmetry;
plagiotropic axes grow horizontally with **bilateral** symmetry and spread
their foliage in a plane. The distinction is the single most visible thing
about a fir (orthotropic trunk, plagiotropic branch tiers — Massart's
model) versus a poplar (everything orthotropic — Rauh's). Roux's and Cook's
models are orthotropic trunks with continuous plagiotropic branching;
Attims' model is orthotropic throughout.

**What the engine does today.** `plant.rs:929-936`:

```rust
let gravity_or_water = if cell_type == RootTip { ...moisture... }
                       else { (0.0, -1.0) };   // up, for every shoot, every order
```

A single hardcoded reference direction, weighted by `upward_weight`
per order. Every axis in every species is orthotropic by construction. This
is also, I suspect, a large part of why **`upward_weight` measured inert
across 1,024 genomes** (a listed do-not-relitigate finding — I am extending
it, not disputing it): jittering the *magnitude* of a bias whose
*direction* is fixed and shared by every axis in the plant moves nothing,
because the term is the same for all candidates' competitors. A per-order
*direction* is a different quantity entirely and should be measured
separately before `upward_weight` is written off.

**The cheap implementation, and the data is already there.** A plagiotropic
axis needs to know which way "out" is — and `OrganismCell::heading` already
stores it, laid down when the lateral departs the trunk (`:1188`). So:

```rust
let reference = match tropism.at(order) {
    Orthotropic  => (0.0, -1.0),
    Plagiotropic => (heading.0.signum(), 0.0),   // straight out, the way this axis left
};
```

`heading` is already read two lines earlier for momentum. This costs one
`ByOrder<Tropism>` in the species file and **no per-cell state, no new
pass, and no extra reads**.

| cost | |
|---|---|
| per-cell state | **none** (reuses `heading`) |
| species parameters | one `ByOrder<Tropism>` enum list |
| new passes | none |
| frame cost | **zero** |

Two things to expect, and they are why this needs the §2.3 scenes:

- Plagiotropic branches spread horizontally, which **runs straight into
  `crowding_weight`** — a plane of foliage is dense by definition. Do §2.2's
  multiplicative form first or plagiotropic tiers will strangle themselves
  on the score filter.
- Plagiotropy in 2D is *more* visible than in 3D, not less: a real
  plagiotropic branch spreads its plane perpendicular to the viewer and
  reads as a tier. Here it reads as a horizontal line. That will look
  strongly architectural, and it will also look flat — worth a filmstrip
  before committing to a value.

**Rank 2**, behind sympody only because sympody is even cheaper and because
plagiotropy has a prerequisite.

### 3.3 Rhythmic vs continuous growth — **real, cheap, and it buys conifers**

**What it is.** Rhythmic growth produces discrete flushes separated by rest,
so laterals appear in **whorls** at tier boundaries and internodes vary in
length along the shoot. Continuous growth spreads nodes evenly. It is the
difference between a pine (obvious annual whorls, countable) and a
eucalypt.

**What the engine does.** `plastochron` is a fixed per-order interval and
`branch_chance` is a fixed per-order probability rolled independently at
every step. Growth is purely continuous, and — worth naming — **branching is
currently a Poisson process**, so the plant has no memory of where it last
branched. Whorls are impossible by construction.

**The implementation, and the counter already exists.** `lineage_step` is
carried on the `ActiveSite` and threaded through `Grow` (`:1007`) to drive
the plastochron. Gate branching on the same counter:

```rust
let in_flush = flush_period.at(order) == 0
            || (lineage_step % flush_period.at(order)) < flush_width.at(order);
if resource >= cost && in_flush && rng.chance(branch_chance) { ... }
```

`flush_period: [0]` is continuous growth and reproduces today's behaviour
exactly, which is the right default under `design-philosophy.md`.

One real cost, unlike §3.1 and §3.2: **a whorl needs several laterals from
one node**, and the branch block currently creates at most one
(`:1160-1198`). Making it a loop over `alt` is straightforward, but it is
the one place here that changes the per-tick work rather than a constant —
`whorl_count` laterals means `whorl_count` extra `growable` checks and
`World::set` calls on flush ticks only. On a stand this is small (branch
events are rare relative to growth steps) but it should be measured on
`examples/ascii.rs`'s worst-frame timing, not assumed.

| cost | |
|---|---|
| per-cell state | **none** (reuses `lineage_step`) |
| species parameters | two `ByOrder<u8>` (period, width), one `whorl_count` |
| new passes | none |
| frame cost | small and bounded; measure worst-frame |

**Rank 4.** High impact for one family of forms (conifers, and rhythmic
tropical models like Aubréville's and Massart's) rather than across the
board, and it has the only nonzero frame cost in the top five.

### 3.4 Acrotony / mesotony / basitony — **a single parameter that flips habit, and your understanding is right with one caveat**

**What it is.** Where along a shoot the buds preferentially break: acrotony
= distal, mesotony = middle, basitony = proximal.

**Your understanding, checked.** The review literature says exactly what you
think, and then adds a caveat worth having:

> "Acrotony or basitony are frequently considered as two fundamental
> phenomena underlying, respectively, the arborescent or bushy growth
> habit."

and immediately:

> "at the annual branch scale acrotony is observed on both trees and
> shrubs, suggesting the distinction between tree and shrub habits involves
> more complex factors than branching position alone."

**So: yes at plant scale, no at shoot scale.** What makes a shrub is not
that each annual shoot branches proximally — most do branch distally — it is
that over the *plant's* life the basal axes are the vigorous ones. That is
good news here, because plant scale is the scale this engine can read
cheaply and shoot scale is the one it cannot.

**The implementation, and the data already exists.** `break_buds` scores
each bud as `light / (1 + crowding)` (`:1792`) and takes the best. Add a
positional term against the organism's own collar, which
`OrganismState::collar_y` already stores and `accumulate_support` already
reads (`:1683`):

```rust
let elevation = (collar_y - y) as f32 / span;        // 0 at the collar, 1 at the top
let position = 1.0 + acrotony * (elevation - 0.5);   // >0 acrotonous, <0 basitonous
buds.push((x, y, light / (1.0 + crowding) * position));
```

`acrotony: 0.0` is today's behaviour. Positive builds a tree; negative
feeds the base and builds a thicket of competing basal axes — a shrub, from
one signed scalar.

| cost | |
|---|---|
| per-cell state | **none** (`collar_y` exists on `OrganismState`) |
| species parameters | one `f32` |
| new passes | **none** — one more term in a loop already running |
| frame cost | **zero** |

**Rank 3.** Not as transformative as sympody or plagiotropy on a *tree*, but
it is the cheapest thing in this report and it is the difference between
"tree" and "bush" as categories — which is precisely what
`plant-species-authoring.md` was written to enable.

**One interaction to watch:** basitony plus `thickening_survival: 0.2` will
fight. A basal bud is on the trunk, the trunk is thickened over many times,
so a basitonous species' preferred buds are exactly the ones the cambium
kills. That is *correct biology* (it is why real shrubs are multi-stemmed
from the start rather than sprouting from an old trunk) but it means
`thickening_survival` must become a species parameter that moves with
`acrotony`, or a basitonous species will read as a normal tree with a few
sprouts at the base.

### 3.5 Reiteration — **the mechanism you suspected, and it is as good as you hoped**

**What it is.** Oldeman's reiteration: a plant repeats its whole
architectural unit, at reduced scale, from a bud that would otherwise have
stayed dormant. The literature splits it two ways and **both splits map onto
things this engine can already compute**:

- **Complete vs partial** — the whole architectural unit, or part of the
  developmental sequence.
- **Adaptive vs traumatic** — *"adaptive reiteration is a response to an
  increase in resource levels, whereas traumatic reiteration is a response
  of a plant after it has been damaged and lost a major part of its
  structure."*
- **Sequential vs delayed** — and delayed traumatic reiteration is defined
  as *"branches that emerge after the release of dormancy of a latent
  epicormic bud in response to a defoliation or a branch loss."*

**`CellType::DormantBud` is already a latent epicormic bud.** It is
deposited by extension, buried by thickening, survives at
`thickening_survival`, and has one flush in it. That is the botanical
definition, implemented, by accident of following the metamer correctly.

**Reiteration is one line.** `break_buds` currently does:

```rust
write_order(world, bx, by, order.saturating_add(1));   // plant.rs:1833
```

A reiterated bud writes `0` instead. It restarts as a trunk — the sparsest
branching tier, the longest plastochron, the strongest upward bias, the
juvenile economy — inside a mature plant. **That is the architectural unit
repeating at reduced scale, exactly.**

**And it is the damage response, from §2.5's deficit.** One trigger, both
kinds of reiteration:

```rust
let deficit = q_peak - q_now;                    // free, §2.5
let surplus = q_now - q_expected_for(order);     // a bud in a light gap
if deficit > threshold * q_peak { reiterate }    // traumatic  — rebuild after a cut
if surplus > threshold * q_peak { reiterate }    // adaptive   — exploit a gap
```

**This is the combination you said you most wanted**, and it is real: the
same mechanism that makes a topped tree resprout a new crown makes a mature
tree accumulate crown complexity with age and makes an edge tree exploit a
clearing. Three visible behaviours, one branch.

| cost | |
|---|---|
| per-cell state | **none** — `q_peak` exists, `q_now` is already computed and discarded |
| species parameters | one threshold, optionally a `reiteration_scale` |
| new passes | **none** — both triggers live in `break_buds`, already running |
| frame cost | **zero** |

**Rank 5 on silhouette alone; rank 1 if damage response is counted**, and it
should be — `CLAUDE.md`'s ethos section is explicit that destruction which
produces no visible consequence is not finished, and queue item 3 is exactly
this. It sits at rank 5 in the table below only because §2.5's `q_now` split
is a genuine (if small) prerequisite.

**One real design problem to solve before building it.** `juvenile_size`
gates on *whole-organism* cell count, so a reiterated complex inside a
3,000-cell tree will not get juvenile treatment — it will be handed the
mature economy with order-0 parameters, which is the starving-seedling
failure `tree.ron` already records. A reiterated complex needs its own size,
which is the first thing in this report that costs storage. Cheapest
approximation: use the reiterated bud's own `q_peak` as a proxy for complex
size (it starts near zero and grows with the complex), which costs nothing
and is roughly the right shape. Measure it before assuming it works.

### 3.6 What drives variety within a species in the field — **and whether 35% is realistic**

**The ordering, from the field literature.** Species turnover dominates;
intraspecific plasticity is "of secondary importance"; and within
plasticity, developmental constraint outranks competitive position. Trees
under high competition show **narrower crowns, greater height and less
taper** for a given diameter, and the *degree* of that plasticity is itself
species-specific, tracking shade tolerance.

Read against this branch, that says three things:

1. **The investment is inverted.** Six jittered scalars target the
   third-ranked driver. §3.1–§3.5 target the first.
2. **Plasticity is the second-ranked driver and this engine is unusually
   good at it** — everything is a local response to a local field read, so
   competition-driven crown narrowing should emerge rather than be authored.
   It is worth *checking that it does*: the narrow-crown-under-competition
   response is a specific, published, measurable prediction, and it is
   exactly the kind of thing `plant_probe` could test with the §2.3 gap
   scene. If a tree beside a gap does not build a wider crown than one in
   the interior, plasticity is not working and no amount of genotype width
   will substitute.
3. **`light_weight` being inert is more suspicious in this light**, per
   §2.3.

**On the 35% figure.** Directionally right, in the wrong shape.

- **Magnitude: conservative.** Density-dependent mortality in crowded
  even-aged stands is the norm — the −3/2 self-thinning rule describes
  precisely this, and stands progress from "all trees have sufficient
  space, no mortality" through canopy closure to continuous
  competition-driven death. Real seedling-to-sapling attrition in natural
  regeneration runs far above 35%. So a self-thinning stand is a *correct*
  outcome and `plant-species-authoring.md` §7 is right to call it a real
  forestry result rather than a defect.
- **Spatial pattern: wrong, and this is the part that will read as fake.**
  The measured failures fell **on a regular pitch (median gap 2)**. Natural
  self-thinning is *clustered* — mortality concentrates where local crowding
  is highest and leaves gaps that later recruitment fills. A regular pitch
  of failures is the signature of a **regular pitch of planting** plus a
  threshold, not of competition. In a stand planted on a uniform grid, every
  tree has identical neighbours and the only thing separating winners from
  losers is the genotype draw and RNG — so the survivors alternate.
- **The fix is the scene, not the mechanism.** Jitter the planting
  positions. Then failures should cluster, and if they still fall on a
  pitch, *that* is a mechanism defect worth chasing.

---

## 4. Recommendations, ranked by silhouette impact per unit of complexity

Prerequisites are marked **[P]** — they buy no silhouette themselves but
unblock or de-risk what follows.

| # | change | silhouette | cost | state | pass |
|---|---|---|---|---|---|
| 1 | **Sympodial branching** (§3.1) — both children take `order+1` on a fork | **very high** — a different kind of crown | ~4 lines + `ByOrder<bool>` | none | none |
| 2 | **[P] Multiplicative crowding** (§2.2) — divide, don't subtract | none directly; **removes the cliff** | 1 line | none | none |
| 3 | **Plagiotropic axes** (§3.2) — per-order reference direction from `heading` | **very high** — tiers, planes, fir-vs-poplar | ~6 lines + `ByOrder<Tropism>` | none (reuses `heading`) | none |
| 4 | **Acrotony/basitony scalar** (§3.4) — one signed term in the bud score | **high** — flips tree ↔ shrub | ~3 lines + 1 `f32` | none (`collar_y` exists) | none |
| 5 | **[P] `q_now` beside `q_peak`** (§2.5) — stop discarding the instantaneous vector | none directly; **makes damage computable** | ~5 lines | none | none |
| 6 | **Reiteration** (§3.5) — flushed bud writes `order = 0` on a deficit/surplus trigger | **high**, and it is *also* the damage response | ~10 lines + 1 threshold | none (needs #5) | none |
| 7 | **[P] Column-mask occupancy** (§1b) — count columns hit, not cells filled | indirect: **makes canopy plates opaque** | ~3 lines | none | none |
| 8 | **[P] Drop `ambient_light_above`'s offset** (§1d) — stale workaround | indirect: **restores one block of self-shading** | 1 char | none | none |
| 9 | **[P] Normalise the currency** (§2.1) — `L_node`, and crowding into `[0,1]` | none directly; **ends the treadmill** | ~10 lines + re-derive 3 constants | none | none |
| 10 | **Rhythmic growth / whorls** (§3.3) — gate branching on `lineage_step` | **high for conifers** | ~15 lines + 3 params | none (reuses `lineage_step`) | **small extra work on flush ticks** |
| 11 | **[P] `slope` and `gap` scenes** (§2.3) | none; **makes 3, 6, and `light_weight` measurable** | scene params | — | — |
| 12 | **Position-keyed genotype** (§2.4) | none; **removes a save/load constraint** | ~10 lines | 1 `(i32, i32)` per organism | none |
| 13 | **Opaque powders/liquids** (§1b(ii)) — with the germination interaction handled | small; correctness | ~2 lines | none | none |

**Note what the state and pass columns say.** Ten of thirteen cost **no
per-cell state and no new pass**, and the one nonzero frame cost (#10) is
bounded to flush ticks. This is not a coincidence and it is the strongest
argument in this report: **the discrete architectural axes are cheap in this
engine specifically because `Grow` already retires its apex, already
carries `heading`, already carries `lineage_step`, and already has a
per-order parameter table.** The expensive-looking thing (23 architectural
models) is mostly already built; it is one corner of it that is wired up.

**A suggested order**, since several have prerequisites: 2 → 7 → 8 → 9
(the substrate pass, which will move `crowding_weight`,
`LEAF_INCOME_PER_TICK` and `pipe_ratio` **one last time, together, in
normalised units**) → 1 → 3 → 4 (three independent silhouette levers, each
judgeable by eye against a filmstrip) → 5 → 6 → 11 → 10.

Doing 9 *before* 1/3/4 rather than after is deliberate: the treadmill's cost
is that every mechanism change re-derives it, and 1/3/4 are three more
mechanism changes. Normalise once, then change mechanisms freely.

---

## 5. Measured vs argued

**Measured this session**, in `.claude/worktrees/plant-v2`:

- `cargo test --release --lib` — 390 passed, 0 failed, 1 ignored, 186 s.
- `cargo clippy --all-targets -- -D warnings` — clean.
- `plant_probe -- trees=8 frames=30000` — reproduces the branch's headline
  numbers exactly. 20,331 organism cells; 8,512 `Leaf`, 11,244
  `MatureBody`, **547 `DormantBud`, 28 `GrowingTip`**.
- **Paired phase comparison** — `plant_probe -- trees=8` at `frames=28800`
  (cycle phase 0.0, noon, `sky_light_amplitude` = `MAX_LIGHT` = 4.0) against
  `frames=30000` (phase 0.333, night, amplitude = `NIGHT_LIGHT_FLOOR` =
  0.2):

  | | 28,800 (noon) | 30,000 (night) |
  |---|---|---|
  | mean cells | 2,405 | 2,541 |
  | mean leaves | 1,000 | 1,064 |
  | **live `GrowingTip`** | **71** | **28** |
  | `DormantBud` | 530 | 547 |
  | thickest fused run | 51 | 51 |

  The noon sample is a *smaller* stand with **2.5x the live frontier**. That
  is the oscillator, not growth. `supportable = ⌊intercepted ·
  LEAF_INCOME_PER_TICK / cost⌋` = `⌊intercepted / 50⌋`, and `intercepted`
  scales with the amplitude, so the bud-break gate is ~20x looser at noon
  than at midnight.
- `SKY_TRANSMISSION = 0.2`, `MAX_LIGHT = 4.0`, `NIGHT_LIGHT_FLOOR = 0.2`,
  `FIELD_SCALE = 8`, `DAY_NIGHT_PERIOD_FRAMES = 3600` — read from source.
- `soil.ron` is `kind: Powder`; `rebuild_blocked` counts only
  `Solid | Plant` into `filled` — read from source.
- No world serialisation exists anywhere in `src/` — grepped.
- `free_organism_slots` is never populated (no `free_organism` exists), so
  `organism_id` is currently monotonic in planting order — read from source.

**Argued, not measured** — every recommendation in §4, the occupancy
orientation table in §1b (arithmetic from the source, not run), the claim
that the crowding cliff is caused by subtract-then-filter (the arithmetic is
checkable but the fix is untested), and everything in §3 about what a
mechanism *would* look like. **`CLAUDE.md`'s rule applies to all of it: a
mechanism that adds a discrete "this happened" event needs a counter printed
next to the picture.** Specifically — sympodial fork count, plagiotropic
axis count, reiteration count. A contact sheet cannot distinguish a
reiterated complex from a vigorous lateral at the zoom these are read at,
and this project has already been caught by exactly that once.

**Not investigated:** roots (queue item 4), λ (item 6), the juvenile/adult
axis (item 7) beyond noting §3.5's interaction with `juvenile_size`, and
the `worldgen` interaction generally.

---

## 6. Sources

Prior art already cited on this branch (Palubicki et al. 2009, Shinozaki's
pipe model, Takenaka, Bond et al.) is in
`Reports/tree-procedural-prior-art.md` and
`Reports/tree-extension-biology.md` and is not repeated here.

- Hallé, F., Oldeman, R.A.A. & Tomlinson, P.B. (1978), *Tropical Trees and
  Forests: An Architectural Analysis*, Springer — the 23 models.
  [record](https://www.scirp.org/reference/referencespapers?referenceid=1225154)
- Prusinkiewicz, P. et al., [*Characterization of architectural tree models
  using L-systems and Petri nets*](https://algorithmicbotany.org/papers/catm.tree2000.pdf)
  — the 23 models as parametric variation over a small set of axes. Same
  group as Palubicki, so the vocabulary already matches this branch's.
- Barthélémy, D. & Caraglio, Y. (2007), [*Plant Architecture: A Dynamic,
  Multilevel and Comprehensive Approach to Plant Form, Structure and
  Ontogeny*](https://academic.oup.com/aob/article-abstract/99/3/375/2464324),
  Annals of Botany 99:375–407 — the canonical modern review; architectural
  unit, reiteration, morphogenetic gradients.
- Millet, J., Bouchard, A. & Édelin, C. (1999), [*Relationship between
  architecture and successional status of trees in the temperate deciduous
  forest*](https://www.arboritecture.org/pdf_uploads/millet/relationship-between-architecture-and-successional-status-of-trees-in-the-temperate-deciduous-forest--millet-bouchard-and-edelin-1999.pdf)
  — the discriminating criteria, stated compactly.
- Costes, E. et al. (2014), [*Bud structure, position and fate generate
  various branching patterns along shoots of closely related Rosaceae
  species: a review*](https://pmc.ncbi.nlm.nih.gov/articles/PMC4251308/)
  — acrotony/mesotony/basitony, and the caveat that the tree/shrub link
  holds at plant scale rather than annual-shoot scale.
- [*Physiological and ecological implications of adaptive reiteration as a
  mechanism for crown maintenance and
  longevity*](https://pubmed.ncbi.nlm.nih.gov/17241987/), Tree Physiology
  27:455 — adaptive vs traumatic reiteration.
- [*Determinants of delayed traumatic tree reiteration
  growth*](https://www.sciencedirect.com/science/article/abs/pii/S1618866718307982)
  — delayed traumatic reiteration from latent epicormic buds.
- Sterck, F.J. & Bongers, F., [*Changes in crown architecture with tree
  height … developmental constraints or plastic response to the competition
  for light?*](https://www.sciencedirect.com/science/article/abs/pii/S0378112703004031)
  — developmental constraint outranks competitive position.
- [*The global spectrum of tree crown
  architecture*](https://www.nature.com/articles/s41467-025-60262-x), Nature
  Communications (2025) — species turnover dominates; intraspecific
  plasticity secondary.
- [*Tree allometry variation in response to intra- and inter-specific
  competitions*](https://link.springer.com/article/10.1007/s00468-018-1763-3)
  — narrower crowns, greater height, less taper under competition; the
  plasticity tracks shade tolerance.
- [*3/2 power law of self-thinning*](https://cfs.nrcan.gc.ca/terms/read/273),
  Natural Resources Canada — density-dependent mortality in crowded
  even-aged stands.

---

*Written 2026-08-16 against `29e8984`. Freshness: the measurements in §5
were taken on that commit; §1 and §2 are a review of code as of that commit
and will need re-reading if `field.rs` or `plant.rs` move.*
