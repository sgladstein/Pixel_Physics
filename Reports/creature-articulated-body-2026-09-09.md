# A creature's form as a heritable growth program: feasibility, cost, and the one new rule

Answers the design question the creature line has been circling since
2026-09-02: **can a creature's body come out of a genome instead of a
species file, and if so, what does it cost?** The direction is the owner's,
2026-09-09 — *an engine that evolves form, not a library of hand-authored
animals* — and the proposal it names is specific: form from a heritable
growth program on the **plant fate machinery**, cell types that carry
**roles**, bodies as **chains of rigid segments, longer not wider, six to
ten cells, two wide at most**, with width, upkeep and the bite giving shape
consequences, and the caste channel able to provision it.

**The brief asked for a "no" if the plant machinery is unsuitable. It is not
a no.** What follows is why, what the honest limits are, and the four
numbers the re-derivation turns on.

Read alongside `Reports/creature-appearance-design.md` §1–§7 (extent is the
only appearance lever, and it is not reachable by evolution today),
`Reports/creature-shape-reachability-2026-09-02.md` (width sets mobility as a
step at 3 cells; shape at constant extent moves nothing), and
`Reports/creature-genome-flexibility-2026-09-02.md` §12–§13 (the body-plan
contract, and why growing a creature makes a worm or a brick). **Nothing here
re-measures any of those.** Extent is the lever, width ≤2 keeps mobility,
shape at constant size moves nothing: those are taken as settled and this
report is built on top of them.

## 0. The answer, in four numbers

1. **The heritable slot already exists and is already populated for every
   animal.** `World::push_organism` (`world.rs:4015`) seeds *every* organism —
   creature included — with a `FateGenome` built from its species' fate table,
   at allocation, with the comment *"Every organism carries its own production
   rule from the instant it exists."* An ant has had a body genome since the
   day the plant line landed one; it has been **empty**, because `ant.ron`
   authors no `fates` table. The heritable body costs **zero new bytes of
   per-organism state**.
2. **The encoding has room.** `CellType` is four bits in both places it is
   stored — `PackedFate::pack` and `Cell::aux` (`organism.rs:6818`,
   `aux & 0b1111`) — so **16 addressable types, 10 used, 6 free**. The
   proposal takes three (`Leg`, `Gut`, `Armour`), leaving three. `MAX_FATES`
   is 16 production rules; a six-segment body with two regions needs four.
3. **The economy re-derives on one factor.** Every size-linked term in the
   creature economy is a field of the form `*_per_cell`, and there are
   exactly **four** of them: `idle_cost_per_cell`, `move_cost_per_cell`,
   `exposure_cost_per_cell`, `body_energy`. Going from a 2-cell ant to a
   7-cell one and dividing all four by 3.5 holds the whole-animal idle bill,
   the whole-animal move bill, the birth stamp **and** the meat an animal is
   worth, all four exactly. See §5.
4. **One new movement rule.** `body_after_step` (`creature.rs:6683`) is, by
   its own doc, *"the two body plans differ **only here**"*. The articulated
   plan is a third arm in that one function. Everything else in this design is
   reuse.

**The verdict: build it.** The limits are in §6 and one of them is
load-bearing rather than a defect.

## 1. What the fate machinery already is, read as body-development machinery

The plant line built a production system and nobody has noticed it is
species-agnostic. Stated as what it provides, rather than as what plants do
with it:

| it provides | where | what a body wants it for |
|---|---|---|
| a rule `(owner, when, after n) → (becomes, child, lateral)` | `organism::Fate` | *this segment becomes X, the next one is Y, and it is two wide* |
| 16 rules in a fixed array, one `u32` each | `FateGenome`, `PackedFate` | a bounded body program |
| four mutation operators, weighted 60/15/15/10 | `FateGenome::mutate` | retarget a cell type, move a region boundary, add a rule, drop one |
| first-match-wins lookup, order load-bearing | `FateGenome::fate` | a special rule listed above the general one |
| a determinacy count `after_metamers` | `Fate::after_metamers` | **regions**: "from segment 3 onward, be a gut" |
| inheritance and mutation at reproduction | `plant::bear_seed_at` (`plant.rs:2618`, `:2731`) | the pattern to copy into `try_bud`, four lines |
| a decoder that reads a corrupt pattern as *no rule* | `PackedFate::unpack` | a mutated body genome cannot alias onto a wrong variant |
| serialisation both ways | `to_table` / `from_table`, `specimen.rs:445` | a jarred animal keeps its shape |
| a **measured viability record** | `Reports/plant-fate-viability-2026-08-28.md` §2 | 40 effective mutations: `becomes` and `lateral` took 34 with no sterile plant; `child` killed 5 of 6 |

That last row is the one worth pausing on, because it transfers with its sign
reversed. In a plant, `child` is what continues the axis, and a mutation that
removes it sterilises the plant — five deaths in six. **In a creature it ends
the body one segment early**, and a one-segment animal is still an animal
with a head. The fate machinery's most dangerous operator is its safest on a
body, because the unfold has a floor (the head) and a cap (§2) where a plant's
axis has neither.

## 2. The growth program

A body is one axis. The unfold is a loop with no RNG in it:

```text
current  = Head
metamers = 0
loop:
    rule = genome.fate(current, Grew, metamers)      // first match wins
    if no rule            -> stop
    push Segment { cell: rule.becomes, lateral: rule.lateral }
    if rule.child is None -> stop                    // the axis is finished
    current   = rule.child
    metamers += 1
    if metamers == CAP    -> stop
if the body came out empty -> one Head segment
```

Three things fall out of it that are worth stating separately, because each
is a piece of the design rather than an implementation detail.

**Regions come free from `after_metamers`.** A rule listed above the ordinary
one with `after_metamers: Some(3)` fires from the fourth segment on. That is
exactly *head, thorax, abdomen* — an axial body with distinct regions — and it
is the same first-match-wins ordering the authored plant species already
depend on. Nothing new is needed for it and `recondition_one` already mutates
it along a ladder (`None, 2, 4, 8, 16, 32`), which is a lineage moving where
its abdomen starts.

**`lateral` is the width bit, and one level of it is all there is.** A rule's
`lateral` makes its segment two cells instead of one. The unfold does not
recurse into it, so a lateral is a single cell, never a limb with segments of
its own. §6 says why that limit is load-bearing.

**The floor and the cap are what make a mutated body safe.** A genome that
mutates to nonsense produces a one-cell animal, not a crash and not a
zero-cell organism; a genome that mutates to a runaway produces `CAP`
segments, not an unbounded one. Both are one line. `PackedFate::unpack`
already returns `None` for a bit pattern that does not decode, so the failure
mode of a corrupt rule is *this rule does not exist*, which the loop reads as
"stop".

### 2a. Two bodies, written out, to show the encoding is sufficient

These are worked examples rather than the shipped tables — §7 records what
actually shipped — and their point is that a body with regions, a width and a
terminal cap takes **four rules** out of sixteen, with no new field anywhere.

**An ant: 5 segments, 7 cells.** Rules in genome order, first match wins:

| # | owner | when | after | becomes | child | lateral |
|---|---|---|---|---|---|---|
| A | `Head` | `Grew` | — | `Head` | `Segment` | — |
| B | `Segment` | `Grew` | 4 | `Segment` | — | — |
| C | `Segment` | `Grew` | 3 | `Gut` | `Segment` | — |
| D | `Segment` | `Grew` | — | `Segment` | `Segment` | `Leg` |

Unfolds to `Head` · `Segment+Leg` · `Segment+Leg` · `Gut` · `Segment` — one
head, three plain spine cells, two legs, one gut. B is the tail cap and has to
be listed above C and D, which is the same ordering discipline the authored
plant species already depend on.

**A hopper: 6 segments, 7 cells, longer and thinner.**

| # | owner | when | after | becomes | child | lateral |
|---|---|---|---|---|---|---|
| A | `Head` | `Grew` | — | `Head` | `Leg` | — |
| B | `Leg` | `Grew` | — | `Leg` | `Segment` | `Leg` |
| C | `Segment` | `Grew` | 5 | `Segment` | — | — |
| D | `Segment` | `Grew` | — | `Segment` | `Segment` | — |

Unfolds to `Head` · `Leg+Leg` · four plain segments — the same cell count as
the ant in a visibly different animal: one wide place near the front and a
long thin tail behind it, against the ant's two wide places in the middle.
**They are not the same animal in two colours**, which is the owner's
objection this whole line exists to answer, and the difference is four rows of
data rather than a second body plan in Rust.

What a mutation does to either is legible in the same table: `retarget` moves
one cell type (a leg becomes a gut), `recondition` moves a region boundary
along the `None, 2, 4, 8, 16, 32` ladder (the abdomen starts sooner) or
retimes a rule, `insert` adds a region, `delete` removes one and the body
gets simpler.

## 3. Roles: what a cell type is allowed to mean

The four roles the direction names each map onto a resolver that already
exists in `creature.rs` and already takes a trait vector:

| cell type | existing resolver | what more of it does |
|---|---|---|
| `Head` | `sight_range_of` | sees further |
| `Leg` | `tick_interval_of` | shorter interval — a faster animal |
| `Gut` | `crop_capacity_of` | carries more per trip |
| `Armour` | `armour_of` | resists the bite |

**A role reads the *fraction* of the live body, never the count**, and this is
the single most important line in the section. With a raw count every role
improves monotonically with body size, so size becomes a lever that only ever
pays — and an unpriced lever ratchets to its maximum and expresses nothing,
which is precisely the degenerate codomain that took plant reproduction to
zero (`CLAUDE.md`, *Fixing a bug often exposes a constant that was
compensating for it*). Read as a fraction, the resolvers are **scale-free**:
a bigger animal buys no advantage, only a finer split.

That is what turns a size gene from a ratchet into an economy, and the
arithmetic is worth writing out because it is the argument
`creature-shape-reachability-2026-09-02.md` §0 explicitly could not find.

**Why §11e's cancellation does not apply here.** That report withdrew
"scale `crop_capacity` with body cells" with an exact argument: at capacity
`k·b`, cost per delivered cell is `(idle·T + move·(1+k)·D)/k` and **`b`
cancels**; drop the always-fills assumption and it becomes linear in `b`,
i.e. strictly worse. That argument is about **size**. This proposal does not
scale a payoff with size — it scales it with **allocation at fixed size**.
With `b` cells split `g` gut, `l` legs, `a` armour, `h` head:

- upkeep is `idle·b`, **constant in the split**;
- delivery per unit time rises with `g/b` and with `l/b`;
- `g + l + a + h = b`.

So the lineage solves a constrained allocation, and it has an interior
optimum for the ordinary reason: it cannot spend the same cell twice. Nothing
about it is a subsidy for being large.

**And size itself is left with exactly one payoff, which is the one §11e said
survives.** Cost is linear in `b` on all four fields of §5. The roles are
scale-free by construction. What a longer body still buys is **survivability
under severing** — more segments means a bite takes a smaller fraction — and
that benefit saturates once there are enough segments to survive a bite. A
linear cost against a saturating benefit is an interior optimum, which is the
first time body size has had one in this engine.

**The requirement this puts on whoever writes the resolvers**, stated so it
can be checked in one line each: every role curve must be **concave in its
fraction**. A linear one leaves the allocation indifferent; a convex one puts
the optimum at a corner and every animal becomes all gut or all legs. The
existing `ratio_factor` family (`1/(1+t)` on one side, `1−t` on the other) is
already the right shape and is what these should be built on rather than a
fifth curve.

## 4. The one new piece of code

`body_after_step` gets a third arm. The rule, complete:

- The **spine** follows the chain rule that already ships — the new spine is
  `[head] ++ old_spine[..m−1]`. That is the existing `Chain` arm, unchanged,
  and it is why an articulated body inherits the chain's mobility rather than
  the rigid body's: measured, a chain is blocked ~6% of its moves and any
  footprint ≥3 wide 47–58%.
- A segment's **lateral** cell sits at its spine cell **+ (0, −1)** — directly
  above, in world space, with no facing or rotation term.

That is the whole rule and it has three properties that were argued for at
length elsewhere and here come out for free:

- **It bends.** Each segment walks the path of the segment ahead, so the body
  follows the head's history rather than translating as a fixed shape. That
  bend is what `creature-shape-reachability-2026-09-02.md` §3's correction
  names as *"plausibly the visual difference between 'a chain' and 'an
  animal'"* and as the one thing a still render of a monolithic proxy cannot
  show. The owner's verdict on that card said the same thing from the other
  side — *"decent starts, depends on how they look in action"*.
- **It cannot be built too wide to walk.** A segment is one cell or two. The
  ≥3-wide bucket that measures at 47–58% blocked is **unrepresentable**, not
  merely discouraged. This is the same move `Reports/creature-genome-
  flexibility-2026-09-02.md` §13c makes for connectivity — *"disconnected and
  self-overlapping bodies are unrepresentable rather than rejected"* — applied
  to mobility.
- **It has both shipped plans as its ends.** All-one-cell segments is
  `Chain(n)`; one segment is a rigid body of one or two cells. It is the
  generalisation, not a third plan beside them.

The lateral sits *above* the spine rather than below or perpendicular for a
plain reason: a walking animal has air above it and ground below it, so an
upward lateral is almost never the cell that refuses a step, and the whole
mobility argument turns on not refusing steps. It is also the cheapest rule
that is deterministic and conserves cell count, which `relocate_chain`'s
`debug_assert` requires — *"a body relocates cell for cell; a length change
here is a lost or invented cell"*.

**And it un-holds severing.** `creature.rs:641` currently routes a `Rigid`
body around the flood fill with an explicit comment saying the hold ends when
a body's live cells become state rather than a re-derived template. A
segmented body's cells *are* state, so the general 8-connected rule applies to
it, and a bite that takes a segment's spine cell detaches everything behind
it — a piece, at a joint. That is §11d's severing arriving with a natural cut
line, and it is what §3's "size buys survivability" is actually made of.

## 5. What it costs

### 5a. Per-organism state and the genome: nothing

`OrganismState::fates` is allocated for every organism today. The brain
genome is untouched: the body program does not live in it, so no
`mutation_rate` is re-derived, no synapse slot count moves, and no existing
animal's RNG stream shifts on that account.

**One new stream is needed and it is a genuine constraint rather than a
formality.** `try_bud` mutates the child's brain *after* `place_creature` has
returned, on the child's own handle. The **body is stamped inside
`place_creature`**, so a body mutation cannot be drawn there — it has to be
drawn from the parent's handle before placement, on a new RNG slot. A new
slot shifts no existing draw, so this costs nothing except being got right;
got wrong, every child is born with its parent's body and the whole channel
is inert, which is the failure this repo files as *a writer and a reader both
present, and the wire between them missing*.

### 5b. The economy: four fields, one factor

This is the part a reader should check rather than take. Every term in the
creature economy that reads a cell count is a `*_per_cell` field:

| field | ant today | read by |
|---|---|---|
| `idle_cost_per_cell` | 0.05 | `idle = idle_cost_per_cell × live_body_cells` |
| `move_cost_per_cell` | 0.125 | every move, and through it digging, emitting and the launch |
| `exposure_cost_per_cell` | 0 | the shelter charge |
| `body_energy` | 480 | the meat stamp **and** `birth_cost = grant + body_energy × cells` — **and pinned to `food_energy` in the matching material file** |

Going 2 cells → 7 and dividing all four by 3.5:

| quantity | 2 cells at 480 | 7 cells at 137 |
|---|---|---|
| whole-animal idle per tick | 0.10 | 0.10 |
| whole-animal move charge | 0.25 | 0.25 |
| birth stamp | 960 J | 959 J |
| meat in one whole animal | 960 J | 959 J |
| **meat in one bite** | **480 J** | **137 J** |

**Every whole-animal quantity is invariant and only the per-bite value
moves**, because the birth stamp and the meat value are the *same* product
`body_energy × cells`. That is not luck; it is what makes the re-derivation a
single division rather than a negotiation, and it is the reason this change
can be judged on appearance without also being a metabolic change nobody can
read apart from it — `CLAUDE.md`'s shared-budget rule, satisfied rather than
argued with.

**One correction to that, and it is the reason this re-derivation was
identified in 2026-08-30 and deliberately not done: `body_energy` is one
number wearing two names.** `assets/materials/{ant,hopper,corpse}.ron` each
carry `food_energy: 480.0`, matching the species files' `body_energy: 480.0`,
and `EnergyLedger`'s identity is asserted closed against that equality. Cut
one without the other and a predator eats a cell of flesh for more than the
flesh cost to build — energy creation inside a ledger that is checked. So the
re-derivation is **four species fields and their matching material
`food_energy`, moved in one change**. `dead-ends.md` records this being seen
and left alone because `assets/materials/**` was another lane's; the
mechanism was never the obstacle and the ownership was. It is one lane's now.

**And `start_energy` is deliberately not scaled.** The same register carries
it as a separate identified-and-not-built entry: with the burn per cell and
the tank flat, an `n`-cell animal's starvation horizon is `1/n` of a two-cell
one. That is an argument for *not leaving the per-cell rates alone*, which is
exactly what §5b does — divide the rates, hold the tank, and the horizon comes
out unchanged. Scaling the tank as well would move it the other way and make
two reallocations inseparable.

The one quantity that genuinely moves is the last row, and it moves in the
direction the ethos asks for: a bite off a seven-segment animal takes a
seventh of it instead of half of it. A binary becomes a distribution.

**Left alone, these four fields would be a 3.5x metabolic change riding on an
appearance change.** That is the whole of why they are named here: the birth
stamp in particular is *already* the term the creature economy is tightest
against (`birth_cost_of`'s own comment records the shipped ant being unable to
reach a 960 J stamp at any grant, before `TRAIT_REPRODUCE_AT` shipped), and
tripling it silently would read as "articulated bodies stopped the colony
breeding".

### 5c. Frame cost

The per-step work that scales with body cells is `relocate_chain` (one `get`
and two `set`s per cell) and two `contains` scans that are quadratic in the
cell count. At 7 cells that is 21 grid operations and 49 comparisons against 6
and 4 — a 3.5x on a path that runs once per creature tick, and the ant's
`tick_interval` is 6 frames. Fifty-two ants at 7 cells is on the order of
**180 grid writes per frame against 50**, in a world whose sweep visits
163,840 cells.

The analytic answer is therefore *invisible*, and this project's standing rule
is not to take that on faith — a cost that is invisible today because the
world is small still counts, and `examples/ascii` is the number to quote. The
measured worst-frame comparison is in §7, taken paired and alternating against
a binary built from this tree before the change, on the same machine in the
same session, because a timing number is only as trustworthy as the box was
quiet.

The two things that could make it *not* invisible, named so they are checked
rather than assumed: a body spanning more chunks keeps more of them awake, and
a larger dirty rectangle costs the render skip. Both are bounded by the body
being at most 8 segments and contiguous.

### 5d. Code

Roughly 230 lines across four files, of which the genuinely new logic is the
unfold (~35) and the movement arm (~25). The rest is a `BodyPlan` variant and
its `offsets`/`len`, three `CellType` variants, a `CREATURE_CELL_TYPES` draw
set, four resolver terms, and the four-line inheritance in `try_bud`.

## 6. What it does not buy, and the one limit that is load-bearing

**It is not growth.** The plant machinery runs its production rules over
*time*, driven by carbon, light and space. This runs the same rules **once, at
hatch, with no environment in them**. What is reused is the encoding, its
mutation operators, its inheritance and its serialisation — not the
developmental process. An animal that grows through instars is not what this
builds, and anyone reading "on the plant fate machinery" as "creatures grow
like plants" will be wrong.

**A body is an axis with single-cell appendages, not a tree.** The unfold does
not recurse into `lateral`, so a limb with segments of its own is
unrepresentable. **This is the limit that is load-bearing rather than a
defect**: recursion is exactly what would produce bodies ≥3 cells wide, and
those measure at 47–58% blocked against a chain's 6%. The shapes the encoding
can reach are exactly the shapes the mobility measurement says can walk. A
more expressive encoding would spend its extra expressiveness on animals that
cannot move.

**Three of the five `FateWhen` variants are meaningless to a body**, and this
has to be handled or it is a small silent waste. `Node`, `Flush` and `Ripe`
are plant lifecycle events; a body unfold reads the other two — `Grew`, and
`Stale` for a terminal segment. `recondition_one` draws its `when` uniformly from all five,
so about 4.5% of all fate mutations on a creature genome would produce a rule
that can never fire. The fix is the same one-line shape as the cell-type draw
set: select the draw set from the rule's **owner** type. Left unfixed it is
not a bug, it is a mutation rate quietly lower than the one on the page —
which is worse, because it reads as a tuning result.

**`lateral` means two different things depending on who owns the rule** — a
branch in a plant, a second cell in a creature. Mechanically fine, and a real
readability cost; it belongs in the doc comment on the field rather than in a
second field nobody would keep in sync.

**It does not make appearance evolvable, only shape.**
`creature-appearance-design.md` §7 names two things an individual cannot own:
extent and palette. This gives a lineage **extent** — the thing that report
measures as the only lever that works. **Palette is untouched**: a material is
still resolved by species name, one palette per species, and a lineage still
cannot recolour itself. Half of §7's constraint stands after this change and
should not be reported as closed. What does open is the door §7 names: the
cell type is now a *role*, so shade-by-cell-type has something to say, and a
countershaded body needs a top and a bottom, which a segmented body has and a
two-cell chain does not.

**It does not answer whether these bodies read as animals.** That verdict is
the owner's and it is on a moving sequence, not on a metric and not on a
still — the condition the owner attached to the six-silhouette card on
2026-09-03. §7 is where it goes.

## 7. Status: built, measured, and **it does not walk**

**The build is on the branch and the bodies are wrong.** The ant and the
hopper are expressed through the growth program exactly as §2a describes —
5 segments and 7 cells, 7 segments and 8 cells, from four heritable rules
each — and an animal built that way is **blocked on 44% to 97% of its
attempted moves**. The colony does not forage. This section is the
measurement, because the measurement is the deliverable now.

### 7a. The number, with its paired control

`examples/creature_scale mode=walk`, one seed, 24 attempted placements, 4,000
frames. The control is `ant_long` — `Chain(6)`, six cells, a plain chain — so
**body length is controlled for** and what is left is the body plan.

| body | cells | `preset=flat` | `preset=rolling` |
|---|---|---|---|
| `ant_long`, plain chain | 6 | **2.5%** | **12.4%** |
| ant, articulated | 7 | **43.9%** | **96.8%** |
| hopper, articulated | 8 | **74.6%** | **91.9%** |

`examples/ascii` fails on it outright, which is how it was found:

```
the colony has gone sessile: 0 round trips of 8+ cells
(measured 23 here on 2026-08-29 at f96c08d, 24 at ba6fc98; was 98 on 2026-08-23),
deepest excursion 7 cells, reach profile [10, 1, 1, 0, 0, 0, 0, 0]
```

with **172 moves against 9,586 blocked** over 12,000 frames. And on
`filmstrip scene=colony`, **4 ants founded of 28 viable sites of 52 asked** —
so placement is failing too: the site predicate says a site is good and the
body does not fit in it.

### 7b. One real defect found and fixed, which was not the cause

The lateral cell was placed at world-space `(sx, sy − 1)`, always. A spine
acquires a vertical link on any upward step, and from then on that lateral
sits exactly on the segment ahead — so `landing_is_placeable` refuses every
candidate, correctly, and the animal can neither move nor clear the kink.
A deadlock rather than a tax.

Replaced by a **perpendicular** rule: the lateral sits orthogonal to the
direction of the segment ahead, on the first of the two sides that is not
already spine, preferring up then left. It reduces to the old rule for a
straight body, so a horizontal animal is laid out exactly as before.

It is guarded by the right property — *the laterals add no collision the
spine does not already have*, over all eight headings, with a bare-spine
paired control inside the test and a positive control asserting the scene can
tell the two rules apart. An earlier version of that guard **characterised
the defect** instead, asserting the collision happened, and went red the
moment the rule was corrected; that is the right way round and it is recorded
because the temptation is to keep such a test passing.

**And it moved the number barely at all**: ant on flat 51.6% → 43.9%, ant on
rolling 96.3% → 96.8%. So the defect was real, the fix is right, and it is
**not** what makes these bodies immobile.

### 7c. Three hypotheses, all wrong, recorded so they are not retried

Each was reverted, and each is recorded because a change that moves nothing
is evidence about the condition it keyed on, not a neutral outcome.

| hypothesis | what happened |
|---|---|
| the whole body had become a jaw, so capping the non-mouth fight reach at the length it was authorised for | breach frames **101 and 109, byte for byte identical**. The reach is not binding |
| attackers spaced below the engine's own `COLONY_ANT_SPACING.max(body_span * 2)` were gridlocking | 109 → **107** |
| the lateral rule (§7b) | 51.6% → 43.9% flat, 96.3% → 96.8% rolling |

Two further things ruled out by reading rather than by running: `composition_
mix` is `1.0 + GAIN × (frac − baseline)`, exactly 1.0 at the baseline, so an
animal with none of a role is unmoved by it; and **no code branches on
`BodyPlan::Chain` in the movement path**, so a segmented body is not falling
into a rigid-body path by accident.

**The lateral count is anti-correlated with the damage**, which is the
strongest clue left and the reason the remaining suspicion is not "laterals":
the hopper has **one** lateral and is blocked *more* than the ant's two
(74.6% against 43.9% on flat). What the hopper has more of is **spine
length** — 7 segments against 5 — while a plain chain of 6 is at 2.5%.

### 7d. What is actually owed next

The unexplained quantity is why a segmented spine of `n` costs so much more
than a plain chain of `n`, when the spine rule *is* the chain rule and the
only other cells are one per widened segment. The next step is not another
hypothesis: it is **an ablation inside one binary** — an env switch that
places a `Segmented` body's laterals or does not, changing nothing else, so
both arms come from the same build and one run separates them. That is this
project's own prescription for a confound, and it would have saved the three
rows in §7c.

**Not measured, deliberately, because there is nothing yet worth measuring:**
the frame cost, and the moving sequence for the review queue. A contact sheet
of four immobile ants is not a question worth an owner's time, and the ethos
this project is built on says a mechanic that is right on paper and dull in
the hand has failed. This one is not even right on paper yet.

### 7e. The ablation (2026-09-10) — laterals are the cause, the spine is exonerated

**§7d's own prescription, built and run: `PIXEL_PHYSICS_BODY_LATERALS=0|1`,**
read once through a `OnceLock` (`creature::body_laterals_enabled`,
`creature.rs:181`), gating only whether `organism::grow_body` keeps or drops
`rule.lateral` while it walks a `FateGenome` (`organism.rs:1736` — `laterals`
is now a parameter, not an env read, so a test can drive both arms in one
process without fighting the once-per-process cache a `OnceLock` needs; the
env var is read once at the one production call site,
`creature::place_creature`). `1` (default) is today's shipped behaviour;
`0` walks the identical rules at the identical `metamers`, stopping at the
identical `cap` — same spine, same `CellType` per segment, same segment
count — and just never pushes a lateral cell. `organism::tests::
body_laterals_switch_strips_only_the_lateral_field` guards exactly that
(spine untouched, laterals gone).

**Table 1 — the switch, both shipped bodies, both presets, against the
`ant_long` `Chain(6)` control, one seed (7), 24 placements, 4,000 frames,
`RAYON_NUM_THREADS=4`:**

| body | laterals | cells | `flat` blocked | `rolling` blocked |
|---|---|---|---|---|
| `ant_long`, `Chain(6)` | n/a | 6 | **2.5%** | **12.4%** |
| ant, articulated | on (shipped) | 7 | 43.9% | 96.8% |
| ant, articulated | **off** | 5 | **2.0%** | **14.8%** |
| hopper, articulated | on (shipped) | 8 | 74.6% | 91.9% |
| hopper, articulated | **off** | 7 | **5.5%** | **40.7%** |

**The switch is connected** — `body_cells` (read off the world, not off
`BodyPlan`) moves with it on every articulated row and holds flat on the
`Chain(6)` control, which never reads it — and the 43.9%/96.8%/74.6%/91.9%
shipped-arm figures reproduce §7a's own numbers exactly, byte for byte, on
this build.

**For the ant, laterals are the whole story.** Stripping them collapses
blocked-on-flat from 43.9% to 2.0% — *below* the plain 6-cell chain's 2.5% —
and blocked-on-rolling from 96.8% to 14.8%, essentially the chain's 12.4%.
Two lateral cells, out of seven, were responsible for the entire pathology;
the bare 5-cell spine walks at least as well as a 6-cell chain.

**For the hopper, laterals explain nearly all of it and leave a residual.**
One lateral cell, out of eight, was responsible for blocked-on-flat falling
74.6% → 5.5% and blocked-on-rolling falling 91.9% → 40.7% — but 40.7% is
still 3.3x the chain control's 12.4%, where the ant's own bare spine landed
within noise of it. That gap is real and is not explained by §7d's own
leading suspicion, which the second ablation below rules out directly.

**Table 2 — the second ablation: does the `Segmented` code path itself cost
anything, independent of laterals, at a length the plain chain control
already covers?** `creature_scale`'s `body=segmented` override (new this
change) takes `ant_long`'s own registry entry — same material, same economy,
same name — and replaces its authored `Chain(6)` with a `Segmented` body of
the identical length and zero laterals: same head, five plain `Segment`
cells, nothing else touched.

| body | code path | cells | `flat` blocked | `rolling` blocked |
|---|---|---|---|---|
| `ant_long` | `Chain(6)` | 6 | 2.5% | 12.4% |
| `ant_long` | `Segmented`, 6 spine cells, 0 laterals | 6 | **2.5%** | **12.4%** |

**Byte-identical — placed, alive, ticks, moves, blocked, falls and digs all
match exactly, on both presets.** This is not a coincidence to be suspicious
of (`CLAUDE.md`'s stale-binary tell is *identical output across a change
that must have moved something*; here the two arms are proven, by the code's
own construction, to compute the same thing). `segmented_body_after_step`
(`creature.rs:7093`) rebuilds the spine with `chain_follow` over the
group-0 cells and then, per segment, appends a lateral only `if g == 2`
(`creature.rs:7117`); with every group at 1 — which is
exactly what `laterals=false` or an unauthored-lateral literal body
produces — the loop degenerates to pushing `(sx, sy)` for every segment and
nothing else, which *is* `chain_follow`. **The `Segmented` movement code
path is not the suspect.** §7d's own framing ("why does a segmented spine of
`n` cost so much more than a plain chain of `n`") had the wrong noun: it is
not the spine *code*, which this proves costs nothing extra; it was always
the laterals.

**A third, narrower control, run once the second ablation came back flat:**
is the hopper's remaining residual pure spine *length* (7 cells against the
control's 6), with no `Segmented` code in it at all? `body=chain5`/`chain7`
re-lays `ant_long`'s own `Chain(n)` at a different `n`:

| body | `flat` blocked | `rolling` blocked |
|---|---|---|
| `ant_long`, `Chain(5)` | 7.5% | 12.6% |
| `ant_long`, `Chain(6)` (control) | 2.5% | 12.4% |
| `ant_long`, `Chain(7)` | 4.8% | 13.7% |
| hopper, laterals off (7 cells) | 5.5% | 40.7% |

Length alone is noisy and non-monotonic at one seed — `Chain(5)` blocks
*more* than `Chain(6)` on flat — but it stays within about 2 points of the
control on both presets. `Chain(7)` **does** land close to the hopper's own
flat figure (4.8% against 5.5%) but nowhere near its rolling one (13.7%
against 40.7%). **Spine length does not explain the rolling residual
either.**

**What was checked and ruled out for the residual, by reading rather than by
a fourth run.** The hopper is the one shipped body that fires
`BrainOutput::Impulse` (`creature.rs:3175`), so a hop-selection-bias
hypothesis was live: a launch is decided *before* `step_chain` runs and, on
success, touches neither `moves` nor `moves_blocked` at all (`creature.rs:
3191`, the `act` dispatch — "Deliberately not `moved`"), so if
hopping preferentially removed *easy* ticks from the walking sample on rough
ground, the remaining `step_chain` calls would skew toward the ground the
verb could not clear, inflating blocked% with no change to the body at all.
The counters this build added to `creature_scale`'s own printout
(`impulses`, `flight_moves`) say this is backwards: launches are **common on
`flat`** (1,490 impulses, 7,212 flight cells, against 3,904 walking moves for
the laterals-off hopper) and **rare on `rolling`** (264 impulses, 825 flight
cells, against 861 walking moves) — the opposite of what the hypothesis
needs, since the residual is the rolling figure. Ruled out by the numbers
already in hand, not retried as a guess.

**Where this leaves it.** The switch, the spine code and (within a factor of
~2) spine length are all exonerated by direct measurement; laterals are
confirmed, mechanistically and numerically, as the dominant cause on both
shipped bodies. What remains is a hopper-specific residual on rolling
terrain (40.7% against a chain-length-matched ~13-14%) that is not the body
plan — it is somewhere among the hopper's *other* authored differences from
a plain chain (its own economy, jaw, dig thresholds, or the hop verb's
interaction with uneven ground in a way the impulse/flight counters above do
not by themselves explain). **No fix is built here.** The one candidate
fix — relaxing `lateral_for`'s placement rule (`creature.rs:7159`) or
`landing_is_placeable_through_tissue`'s per-cell requirement
(`creature.rs:7040`) so a lateral's own collision does not veto an otherwise
clear step — is a real design change to what a 2-wide body is allowed to
do to the world around it, not a bounded, obviously-correct patch, and it is
exactly the kind of change `CLAUDE.md` asks be scoped and budgeted rather
than started under an ablation's cost fork. That is the next report's
problem, not this one's.

## 8. What this deliberately leaves for later

- **Palette.** Per-individual colour is the other half of
  `creature-appearance-design.md` §7 and nothing here touches it.
- **Shade by cell type.** §7 of that report calls it *"the smallest change
  that opens any of this"* and one line at the stamp seam. It becomes worth
  having exactly when a body has a top and a bottom, which is now — but it is
  an appearance change and belongs on its own card.
- **The caste channel provisioning a body.** `expressed_traits` already moves
  a child's traits by what its parent's `Provision` handed it; a body program
  that read the same number would let one genome express a worker and a
  soldier. Every piece of that exists; the wire does not, and it should be
  built after there is pressure that pays for a soldier.
- **Crossover.** `FateGenome` is a flat, bounded, ordered array — the easiest
  thing in this engine to cross. Arc C2 already has the scissors on the shelf.
- **The played bed's plant mix, ruled by the owner 2026-09-09: grass + herb +
  shrub.** Recorded here because it is an owner ruling that would otherwise
  live only in a message. It should be encoded once, as
  `assets/lab_scenarios/played_bed.ron` — about **12 grass, 6 herb, 4 shrub**
  over the 512 columns, with the colony founded on the **timeline near frame
  6,000** rather than in `placements`, which is the difference between the bed
  the owner plays and the eight-herb frame-0 default every headless figure
  before 2026-09-09 was taken on. **Deliberately not written here.** Nothing
  in this report or its build needs a played-bed run, the scenario belongs to
  whichever lane does need one, and a scenario file authored by a lane that
  never runs it is a file nobody has checked. Whoever writes it, both lanes
  use it, and no played-bed figure should be quoted from the default bed
  again.
- **The species export writes the *species'* body, not the individual's**, and
  this is the seam that closes the other half of E5. `specimen::save`
  (`specimen.rs:445`) already stores `state.fates.to_table()`, so a **jar**
  carries an evolved shape; `species_export::individual_as_species`
  (`species_export.rs:177`) takes `parent.fates()` — the registry's species
  table — so a **`.ron` written from a live animal** carries the ancestral one.
  The two disagree, and the export is the one the owner's *"evolve creatures
  and add the good ones to the game"* workflow runs through. It is one field
  threaded from `organism_as_species` through to that line. Deliberately not
  done here: the body has to be heritable before exporting it means anything,
  and it is now.

## 9. One guard is red on the merge, and what isolated it

**`sim::creature::tests::a_swarm_gets_through_what_one_mouth_cannot` fails on
this branch.** It is the only failure: `cargo test --lib` reads **1,532
passed, 1 failed, 65 ignored**. It was green before the merge and it is green
on `main`. Recorded here rather than fixed, because what it caught is a
decision the creature line owns and not a defect in this build.

**What it says:**

```
the swarm's median breach must still come far sooner than the lone attacker's:
median frame 101 alone against 109 for eight, over 8 seeds
```

**What isolated it, in one experiment.** Holding this branch's body and
swapping only `assets/species/ant.ron` back to its pre-merge contents — my
body, my wiring — the test **passes**. With the merged file — my body, `main`'s
wiring from PR #291 — it fails. So the cause is neither side alone: it is the
articulated body against the rewired brain, and **the merge that produced it
had zero conflicts in that file.** `CLAUDE.md` warns that a conflict count
predicts whether a merge is laborious and cannot predict whether it is wrong;
this is that, with a number.

**What is actually happening, and it is not that the swarm got worse.** The
scene's premise is *a plate no single ant mouth can open*, and the lone arm
used to time out at the 900-frame budget — the test's own comment records
900 against 484. It now breaches at **101**. One ant has become sufficient,
so eight buy nothing, and the guard reads that as the swarm being slow. The
guard is doing its job: **the scene has stopped containing the situation the
test is named for**, which is `CLAUDE.md`'s *a scene that contradicts the code
will look like a bug in the code*.

**Two repairs were tried and both are recorded as moving nothing**, because a
change that moves nothing is evidence about the condition it keyed on:

- **Capping the non-mouth fight reach.** `adjacent_food_counted` scans the
  whole body's 8-neighbourhood and, past the head, accepts any living non-self
  organism — a concession authored for a `Chain(2)` ant, where "the body" was
  one cell. At seven segments that reads as a distributed jaw, so capping it
  at the reach it was authorised for looked like the answer. Result: **101 and
  109, byte for byte identical.** Reverted. The reach is not what is binding.
- **Widening the attackers' spacing to the engine's own rule.** The scene
  spaces eight attackers 6 apart while `found_colony` uses
  `COLONY_ANT_SPACING.max(body_span * 2)` = 10 for a 5-cell spine, and a line
  of ants shoulder to shoulder gridlocks rather than converging. Result: 109
  → **107**. Reverted.

**What it is not.** Not the composition mix: `composition_mix` is
`1.0 + GAIN * (frac − baseline)`, exactly 1.0 at the baseline, so an animal
with no armour cells — the beetle — is unmoved by it. Not the cost
re-derivation, which holds the whole-animal bill by construction.

**What the fix is, and why it is not made here.** The honest repair is to
restore the premise — an armour value one mouth genuinely cannot open under
the *merged* wiring — and then re-check that the swarm still beats it. That is
re-establishing a scene, not weakening a bar, and the distinction has to be
demonstrated rather than asserted: the repaired guard must be watched failing
with the mechanism broken, or it is a bar tuned until green. That is a
measurement and a fight-balance judgement belonging to whoever owns PR #291's
wiring, not something to settle inside an appearance change.

**Do not "fix" it by pinning `ant.ron` back.** The pre-merge file is another
lane's landed work reverted, which is the stale-file failure `CLAUDE.md`
records: it looks like a modification and is really a revert of an upstream
commit, and nobody recognises it as theirs.
