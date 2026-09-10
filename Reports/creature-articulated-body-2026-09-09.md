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

### 7f. The lateral rule, specified — a build lane's brief

**This is a specification, not a diagnosis.** §7e settled the cause; what
follows is the whole of what has to be built, and it is deliberately written
so a build lane can work from it without re-deriving anything above.

#### (1) The rule

**The spine decides the move. A lateral never gets a vote.**

1. Compute the spine landing exactly as today: `chain_follow` over the
   segment spines. **Placeability of the step is decided on the spine
   alone**, by the same predicate a plain `Chain` uses. This is the whole
   invariant, and everything else is bookkeeping under it.
2. Then place each widened segment's lateral, in segment order:
   - its **authored side** (the perpendicular `lateral_for` already prefers)
     if that cell is placeable;
   - else the **other side**;
   - else it **tucks** — not placed this step, and that segment reads as one
     cell wide.
3. A tucked lateral **re-emerges** on any later step where a side is free.
   Nothing remembers that it was tucked; the rule is evaluated fresh each
   step, so re-emergence is automatic and needs no timer or flag.
4. **Width 2 stays the representable maximum.** Nothing here widens a body.

An animal squeezing through a gap therefore *looks* like an animal squeezing:
it narrows, passes, and fills out again. That is the ethos' first law — an
outcome is a distribution, not a binary — applied to a body instead of to
rubble, and it is why tucking is the design rather than a fallback.

**What a tucked lateral costs and keeps:**

- **Cell type: kept.** The role comes from the *authored* segment
  (`def.body`), never from the live cell, so a re-emerged lateral is the same
  `Leg`/`Gut`/`Armour` it was.
- **Colour: must be re-derived deterministically, and the key matters.**
  `place_creature` draws shade from a stream keyed on the **walk-order
  index**, and a tuck shifts every later index — so re-keying on walk order
  would recolour half the animal whenever one segment folds. **Key the draw
  on (segment index, is-lateral) instead**, and a cell that tucks and
  re-emerges comes back the colour it left.
- **Per-cell scalars: dropped.** The `OrganismCell` entry for a tucked
  position goes with it. Creature body cells are not believed to carry
  scalars that matter — unlike a plant's carbon, which is exactly the trap
  `relocate_chain`'s `Parted` machinery exists for — but **the build lane
  must check that before relying on it**, not inherit the belief from this
  sentence.
- **Upkeep and roles: genuinely smaller while tucked.** `live_body_cells`
  reads `state.chain.len()`, and the role resolvers read fractions of the
  live body, so a folded animal burns a little less and carries a little less
  armour. That is correct rather than a side effect, and it should be stated
  in a comment so nobody "fixes" it.

#### (2) What to change, and what must not

| site | change |
|---|---|
| `lateral_for` (`creature.rs:7159`) | returns a *choice* — authored side, other side, or none — instead of always a position. The perpendicular preference and its tie-order stay exactly as they are |
| `segmented_body_after_step` (`creature.rs:7093`) | needs `&World` to test placeability, and returns the landing **and the live widths** together. The two must be written as one value; a landing whose grouping is computed separately is how they come to disagree |
| `body_after_step` (`creature.rs:7201`) and its **six** call sites (`:6000`, `:6091`, `:6149`, `:6602`, `:6853`, `:6974`) | thread the world through; callers that only want cells take them off the returned pair |
| `relocate_chain` (`creature.rs:7369`) | its `debug_assert_eq!(from.len(), to.len())` (`:7380`) is the contract that tucking breaks. **The correspondence becomes keyed by (segment, role), not positional** — with a tuck in the middle, index *k* of `to` is not index *k* of `from`, and a positional `zip` would hand a spine cell a lateral's identity. Clear what is dropped, write what is new |
| `OrganismState::segment_groups` | becomes the **live** widths, rewritten with `chain` every step. Authored widths stay in `def.body` and are the only source for re-emergence |

**Must not change:** the spine rule (`chain_follow`); `is_rigid()` staying
false for `Segmented`; the head-only foothold rule; the duplicate-position
check in `landing_is_placeable_through_tissue` (`creature.rs:7040`) — a
landing still may not occupy one cell twice, and tucking is what makes that
satisfiable rather than a reason to relax it. And **do not touch
`assets/species/ant.ron` or `hopper.ron`**: the bodies are right, the
movement rule is not.

#### (3) The tests

- **The property guard already exists and must go green unchanged**:
  `a_lateral_adds_no_collision_the_spine_does_not_already_have`. It carries
  its own bare-spine control and a positive control asserting the scene can
  tell the two rules apart. Do not weaken either.
- **A body with both sides blocked still steps.** Build the case, assert the
  step happens and the segment comes out one wide. This is the invariant in
  (1); if only one test survives review, it is this one.
- **A tucked lateral re-emerges.** Step into the pinch, step out, assert the
  cell count returns and the colour and role are the ones it left with.
- **Put each fault back and watch it go red** before citing any of them. A
  guard over emergent behaviour that has only ever been seen green is the
  failure this repository has paid for most often, and §7c is three entries
  of it from this branch alone.

#### (4) The measurement

Re-run **exactly** §7e's table, from **one binary**, laterals on, under the
new rule, and put the rows beside §7e's. Same seed, same instrument, same two
presets, with `ant_long` still the control. The bar is the laterals-off
column, not the chain: the rule works when laterals-on lands on top of
laterals-off.

Then `ascii` must be green — round trips back near 23, not merely non-zero —
and `filmstrip scene=colony` must found far more than 4 of 52. Quote the
whole-frame figure from `ascii scene=foraging`'s `worst`/`mean` line, paired
and alternating against a pre-change binary on a quiet box, and run the
pinning check before quoting any worst.

#### (5) The card

`filmstrip gif=1`, never a still, animal count in the `meta`. The question is
**"do these read as animals rather than chains?"** — the owner's own
number-one issue, and the condition attached to the 2026-09-03 verdict that
started this line. Post it before calling the ant done.

#### (6) The red guard in §9 — the decision

**Leave it red. Do not tune it green.** It is not caused by this body plan,
and that is measured rather than assumed: swapping only `ant.ron` back to its
pre-merge contents makes it pass with this body in place. The scene's premise
— a plate no single mouth can open — died because the wiring that landed in
PR #291 made one ant persistent enough to get through alone, which is an
improvement in the ant and a problem for the scene.

Two things follow. It must **not** be repaired by pinning `ant.ron` back:
that reverts another lane's landed work while looking like an edit. And it
must not be repaired by moving the bar until it passes: the honest fix is to
re-establish the premise under the merged wiring and then demonstrate the
repaired guard still fails with the mechanism broken. That is a fight-balance
judgement belonging to whoever owns #291's wiring, and the pull request
should say so rather than carry a green it did not earn.

#### (7) Built and measured (2026-09-10, branch `claude/creature-lateral-tuck-r26`)

**The rule as specified, no deviation.** `lateral_for` now returns
`Option<(i32, i32)>` — authored side, other side, or `None` — and
`segmented_body_after_step` places a lateral only when its own candidate is
placeable, dropping it (not the step) otherwise. Placeability of the spine's
own landing is unconditional, exactly as before. `OrganismState::
segment_groups` is now the *live* width, rewritten with `chain` every step;
a new `segment_authored` (this individual's own grown shape, re-derived from
`FateGenome` — pure, cheap, called once a tick) is the stable reference for
whether a segment may *attempt* a lateral at all, so a tucked segment is
offered its lateral again on every later step rather than staying tucked
forever. `relocate_chain` finds the from/to correspondence by (segment,
role) rather than by flat index, since a tuck can make the two lengths
disagree; a re-emerging lateral is minted fresh (material and shade read
off the still-alive spine, `CellType` from `segment_authored`, shade keyed
on `(segment, is_lateral)` so a lateral that tucks and re-emerges
repeatedly always comes back the same colour). `body_after_step`'s three
slices (`chain`/`groups`/`authored`) and `relocate_chain`'s two position
lists are bundled into `BodyShape`/`BodySide` to stay under Clippy's
`too_many_arguments` lint (no prior use of `#[allow]` for it anywhere in
the tree, so bundling rather than suppressing).

**Table 1, re-run exactly as §7e's, laterals on under the new rule** (one
binary, `creature_scale mode=walk`, seed 7, 24 placements, 4,000 frames,
`RAYON_NUM_THREADS=4`, private `TMPDIR`):

| body | laterals | cells | `flat` blocked | `rolling` blocked |
|---|---|---|---|---|
| `ant_long`, `Chain(6)` | n/a | 6 | 2.5% | 12.4% |
| ant, articulated | old rule (shipped, §7e) | 7 | 43.9% | 96.8% |
| ant, articulated | **new rule (this build)** | 7 | **1.9%** | **22.1%** |
| ant, articulated | off (§7e, for reference) | 5 | 2.0% | 14.8% |
| hopper, articulated | old rule (shipped, §7e) | 8 | 74.6% | 91.9% |
| hopper, articulated | **new rule (this build)** | 8 | **6.8%** | **15.4%** |
| hopper, articulated | off (§7e, for reference) | 7 | 5.5% | 40.7% |

The `ant_long` control is byte-identical to §7e's own row, run fresh on this
binary — the change touches nothing a plain `Chain` reads. **The ant lands
on or ahead of the laterals-off floor on `flat` (1.9% against 2.0%, and
within a point of the bare chain's 2.5%) and well inside it on `rolling`
(22.1%, against the old rule's 96.8% and the chain's 12.4% — a 4.4x
reduction from where §7e left it, though not fully to chain parity: some
residual cost from carrying a lateral that must sometimes tuck is expected
and is not what §7f asked to eliminate).** The hopper is the more
unambiguous result: the new rule beats its own *laterals-off* ablation on
both presets (6.8%/15.4% against 5.5%/40.7%), because tucking pays the
mobility cost only on the ticks it is actually needed rather than every
tick, which is exactly what §7f predicted a graded rule would buy over a
blanket one.

**`cargo run --release --example ascii`**: the colony-forage sessile guard
— `st.forage_trips >= 6` — now reads **32 round trips**, comfortably above
both the bar and the pre-articulated-body baseline of 23 (deepest excursion
and reach profile also printed, unchanged in shape). It was **0** before
this build, which is the specific number this task was asked to fix, and it
is fixed. Running further, `ascii` reaches a *later* scene for the first
time since the articulated body landed — "ants excavating a chamber out of
soil" — and fails a **different, pre-existing** assertion there
(`roofed > 0`, dated 2026-09-05, older than the articulated body): the
6-ant colony in that scene digs 96 cells and leaves no roofed void, with 5
of 6 ants dying. Checked, not assumed: the identical scene with
`PIXEL_PHYSICS_BODY_LATERALS=0` (bare 5-cell spine, no width-2 segment
anywhere) passes cleanly — `roofed 14`, 1 death — so the failure tracks
body **width**, not the lateral placement rule this build changes; it
reproduces whether a lateral is blocked (the old rule) or tucked (this
one), because ascii could never reach this scene under the old rule to
tell the two apart. Left unfixed here: a 2-wide body's effect on
burrow/lining geometry is a different mechanism than §7f's movement rule,
and building a fix for it would be exactly the "second rule of your own
invention" the build brief's cost fork warns against. Flagged for whoever
owns the body-shape or burrow-lining line next.

**`filmstrip scene=colony` founding**: unchanged at **4 ants founded of 52
asked, 28 viable sites** — the same as before this build, and expected to
be: `place_creature` refuses a site whenever any cell of the body's full
*authored* footprint is not empty, which is evaluated once at founding
time, before the animal has taken a single step. Nothing about the lateral
tuck rule reaches that check — tucking is a property of *movement*, not of
the one-shot placement test — so a fix here was never implied by §7f and
none was attempted. The founding shortfall against the old 2-cell ant's
footprint is the same, separate defect the lane note already named
("placement fails as well as movement").

**Tests**: `a_lateral_adds_no_collision_the_spine_does_not_already_have`
goes green unchanged, its own bare-spine positive control intact.
`a_body_with_both_sides_blocked_still_steps` and
`a_tucked_lateral_re_emerges` are new — both watched red against a
reversion to the old always-place rule before being trusted green.
`a_lateral_free_segmented_body_is_byte_identical_to_a_chain` guards the
cost fork's hard invariant (zero-lateral `Segmented` == `Chain`) across six
headings. `cargo test --lib`: **1,537 passed, 0 failed, 65 ignored** — the
only prior failure besides §9's own guard was two scenes whose premise the
tuck rule's own correct behaviour broke
(`digging_costs_exactly_its_authored_price_per_cell` assumed a constant
body-cell count, which tucking legitimately varies — fixed by pricing
against `ant_long`'s fixed `Chain(6)` rather than the shipped ant;
`feeding_and_digging_are_separate_genes`'s single-soil-cell obstacle used
to force a dig only because the old body could barely move at all — fixed
by sealing the scene into a tunnel the animal cannot route around). Both
are `CLAUDE.md`'s "a scene that contradicts the code will look like a bug
in the code," not defects in the rule.

`cargo clippy --all-targets --release --locked -- -D warnings`: clean.

**§9's guard, re-checked on this branch before the `main` merge**: still
red, same shape as §7e/§7f(6) recorded — a lone attacker now breaches
faster than the report's own numbers (this build's own measurement, one
seed sweep, is in the pull request), consistent with mobility continuing
to improve rather than with anything about *this* change reversing. Left
red per §7f(6)'s own decision; not touched.

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

## 10. The founding repair (2026-09-10, lane K, branch `claude/creature-lateral-tuck-r26`)

**Founding now decides on the spine alone, exactly like a step.** §7f's
movement rule (§7f(1)) was never wired into `place_creature`: founding kept
its own, older check -- refuse the site if *any* cell of the body's full
*authored* footprint is not empty, evaluated once before the animal has ever
taken a step. A spine that fit perfectly still lost the whole site to one
blocked lateral cell, which is why `filmstrip scene=colony` founded 4 of 52
even after §7f landed (§7f(7) recorded this as expected and out of scope for
that build). This closes it: `place_creature` computes the spine's straight-
line footprint (the same formula `BodyPlan::offsets` uses for a fresh body),
refuses the site only if the spine itself does not fit, and then places each
widened segment's lateral by calling **the movement rule's own `lateral_for`**
-- authored side, other side, or tuck -- rather than a second, independently
maintained placement rule. `segment_groups` is seeded from what actually got
placed, so a founder born with a tucked lateral is not a corrupted body: it
re-emerges on its first step exactly as a lateral tucked mid-walk does
(§7f(1)), because `segmented_body_after_step` reads `segment_authored`, not
`segment_groups`, to decide whether a segment may *attempt* a lateral.

**The numbers, both scenes, one binary each, the `PIXEL_PHYSICS_BODY_LATERALS=0`
arm as the control this task's brief asked for:**

| scene | before this build | after (this fix) | laterals=0 control |
|---|---|---|---|
| `filmstrip scene=colony` seed=1 (52 asked, 28 "viable" by the scene's own probe) | **4** founded | **5** founded | **5** founded |
| `labshot scenario=played_bed frames=6000,6100` seed=1 (52 asked) | **11** animals at frame 6,000 (steady at 6,100) | **12** at frame 6,000 (steady at 6,100) | **12** at frame 6,000 (steady at 6,100) |

**The fix lands exactly on the control, on both scenes, not merely closer to
it.** That is the bar this task set (*"close to what a bare spine
founds"*), and it is met precisely rather than approximately -- the founding
predicate and the movement predicate now agree completely, where before this
build they disagreed by exactly the lateral cells a naive full-footprint scan
adds over a spine-only one.

**Why the absolute numbers are still small, and why that is not this task's
defect to fix.** Both scenes' `laterals=0` control -- a bare, un-widened
5-cell spine, which can never lose a site to a lateral because it has none --
*also* only founds 5 of 52 and 12 of 52. That is definitive: whatever is
capping these two scenes at roughly a tenth of the ask, it is not body width,
because the width-free body hits the identical ceiling. `filmstrip`'s own
comment already names one open, separate cause (`open-bugs-handoff.md` §R2,
the `colony_ant_site` probe disagreeing with `found_colony` about what counts
as ground -- the "viable" column above is that probe's count, not evidence of
what a real body can use); `played_bed.ron`'s own header comment names a
second (grown vegetation and its seed litter occupying the surface by frame
6,000, and colony spacing rules eating sites a bare terrain would not). Both
are the "placement fails as well as movement" defect the PR body already
flagged as separate from §7f's own scope, and this task's mandate was the
width/lateral half of that sentence -- which is now closed, verified by
parity with the correct control on two independent scenes -- not the whole
sentence.

**Test, watched red before being trusted.** `a_founding_site_with_a_blocked_
lateral_still_founds_tucked` (`src/sim/creature.rs`, beside `found_colony_
never_puts_an_ant_on_water`): an open-floor control founds with at least one
segment at width 2 (the positive control, so the obstructed arm below can
actually tell tucked from never-widened); a second world boxes a level ant's
lateral top and bottom -- the floor it already stands on for "down", a
ceiling one cell above the spine for "up", the exact cell `lateral_for`'s
"authored side" candidate lands on for a level body -- and asserts the site
still founds, every segment reading one cell wide, physically fewer cells
than the open control. Watched red against the pre-fix `place_creature`: the
boxed-in world's `spawn()` call panics (`place_creature` returns `None`,
`.expect` fires) under the old full-footprint rule, and passes clean once
the fix lands. `cargo test --lib`: this test plus the whole suite at 1,562
passed / 1 failed (§9's own guard, unrelated and unchanged) / 69 ignored.

## 11. Task B: the swarm guard's cause, isolated further -- length over width, and rescaling the plate does not restore the premise

**§9 already isolated the cause to "the articulated body against the rewired
brain" without separating body length from body width.** This section does
that separation, with the two `#[ignore]`d diagnostic probes added beside the
guard (`probe_lone_attacker_breach_frame_by_body`,
`probe_plate_calibration` -- not gates, run by hand with the env vars their
own doc comments name) as the instrument, per this task's own instruction to
diagnose rather than guess.

**Length, not width, drives the lone attacker's speed.** The lone arm's
median breach frame, one binary, eight seeds, `BUDGET=900` unless noted:

| body | median | per-seed frames |
|---|---|---|
| shipped ant, 7 cells (laterals on) | **101** | 19, 44, 52, 60, 101, 123, never, never |
| shipped ant, 5-cell spine (`PIXEL_PHYSICS_BODY_LATERALS=0`, no width at all) | **112** | 48, 56, 60, 109, 112, 130, 154, 241 |

Both land in the same order of magnitude, and the width-off arm has **zero**
900-frame timeouts against the width-on arm's two -- if anything the width-
free body is the *more* reliable of the two, not the safer-for-the-beetle
one. Per this task's own test: the 5-cell spine also breaches at ~101, so the
cause is body **length**, not width. The mechanism is not new to this
branch: `adjacent_food_counted`'s whole-body scan (dated 2026-09-06, "scanned
the head alone until" that date, its own doc explicit that this is *"the
thing this exists for is an attacker clamped onto a flank, which the head
cannot turn to face"*) lets any body cell -- not only the head -- find and
bite a hostile organism. A longer body, wide or not, is more often touching
its target somewhere along its length on any given tick, so it locks onto and
starts working a beetle sooner and more reliably than the old 2-cell ant did.
This is length acting through a rule that already shipped before the
articulated body, not an artifact of this branch's own lateral tuck rule or
of width.

**`ant_long` (`Chain(6)`, no `fates` table) is *not* a clean length-only
control, and its result is recorded here so nobody trusts it blind.** Same
lone-arm harness, `ant_long` in place of `ant`: median **900**, six of eight
seeds timing out (`392, 504, 900, 900, 900, 900, 900, 900`) -- read alone
this says "a plain six-cell chain is nearly harmless," which would flatly
contradict the 5-cell-spine result two paragraphs up. It is confounded on at
least two axes checked directly in its own file: `start_energy: 900.0`
against the shipped ant's `200.0` (`ant_long.ron` predates the four-price
locked-field audit and was never re-tuned against it), and `force_fraction`
never authored at all (defaults to 0, so the species pays nothing to carry
its jaw -- a cost difference, not a force one, but evidence the file was
never brought current for combat). More likely still: `ant_long.ron`'s own
header says *"Everything else in this file is `ant.ron` unchanged"*, which
was true when it was written and has not been true since the Attack/Alarm
instinct append §9 itself says moved the shipped seed's swarm ratio from
5.3x to 1.9x -- `ant_long` almost certainly carries the *pre*-append
instincts. **Dead end, recorded so it is not retried**: `ant_long` answers
"what does a stale, uncombat-tuned species do here," not "what does body
length alone do here." The 5-cell-spine ablation above is the clean control;
this one is not.

**Rescaling the plate -- this task's other named branch -- was tried and does
not restore the premise; it makes the swarm's relative position *worse* as
the plate gets tougher.** `probe_plate_calibration` swept `TRAIT_ARMOUR`
(the world's own shipped `trait_reach` ceiling, `TRAIT_REACH_MAX = 8.0`, left
untouched -- only the allele moves, never the dial's own top), lone and swarm
arms, eight seeds, `BUDGET=3600` so a slower breach is not miscounted as
"never":

| armour allele | lone median | swarm median | swarm/lone ratio |
|---|---|---|---|
| 1 (shipped bed) | 101 | 77 (the guard's own live number, post-fix) | **0.76** |
| 4 | 153 | 182 | **1.19** |
| 8 (the trait's own ceiling) | 239 | 336 | **1.41** |

At the shipped armour the swarm is still faster than one ant, just not by the
guard's required 1.5x margin (77 against a floor of ~67). Raising the plate
does not close that gap -- it **reverses** it, and by a growing amount: at
armour 8 the swarm takes 41% *longer* than one attacker, the opposite of "the
swarm gets through what one mouth cannot." Per `CLAUDE.md`'s own convention,
a lever that makes the target metric worse in the direction it was raised for
is not a tuning problem, it is the wrong lever, and it is recorded here as a
dead end for the fight-balance owner rather than as this task's own fix.

**Why: crowding, measured directly, and the same shape as §7e/§7f's own
lateral-cost finding, now hitting the *swarm* rather than the *lone*
attacker.** `probe_plate_calibration`'s per-seed `moves`/`blocked` at armour
4 (a fight long enough for the effect to accumulate):

| seed | lone blocked/moves | swarm blocked/moves |
|---|---|---|
| 1 | 1/4 = 25% | 29/77 = **38%** |
| 2 | 2/11 = 18% | 14/62 = **23%** |
| 3 | 3/12 = 25% | 16/56 = **29%** |
| 4 | 1/10 = 10% | 10/27 = **37%** |
| 6 | 3/7 = 43% | 49/84 = **58%** |
| 7 | 6/14 = 43% | 60/116 = **52%** |

Every one of six seeds shows the swarm's own blocked-move fraction above the
lone attacker's, by 1.2x to 3.7x. Eight width-2 bodies converging on one
target interfere with **each other's** movement measurably more than one
body does on its own, and that cost is paid every tick the fight is still
running -- which is exactly why raising the armour (making the fight run
longer) makes the swarm's relative disadvantage worse rather than better: a
longer fight is more ticks for the crowding cost to accumulate against, on
top of whatever the plate itself is costing.

**So the cause is neither of this task's two clean buckets, and the honest
account is a compound one.** Body length (through a pre-existing, unrelated
rule) makes one attacker faster than the guard's premise assumed; body width
(through crowding among the eight, not through anything about the lone
arm) makes the swarm slower relative to that one attacker than the premise
assumed; and the two compound rather than cancel as the fight gets harder.
This is not an accounting line to fix (the whole-body scan is deliberate,
dated, and unrelated to this branch; the crowding is real physical
interference, not a miscount) and it is not a plate value away from being
fixed (measured across a 8x range of the trait's own ceiling, and the wrong
direction). **Left exactly as §7f(6)/§9 already decided: red, untouched,**
now with the length/width split and the plate sweep on record so neither is
retried blind. `sim::creature::tests::a_swarm_gets_through_what_one_mouth_
cannot` is not deleted, not `#[ignore]`d, and not weakened.

## 12. Task C: the chamber assertion is a real consequence of body width -- diagnosed, and left red

**`examples/ascii.rs`'s "ants excavating a chamber out of soil" scene
(`nest_dig_scene`, its `roofed > 0` assertion dated 2026-09-05, older than
the articulated body) fails on this branch and is unaffected by anything in
this task's own change -- §7f(7) already found and recorded this before task
A or B were touched.** Diagnosed here with a windowed instrument (per-1,000-
frame dug/void/roofed/moves/blocked, every death's frame, cause and last
known position, and a final row-by-row void census), built as a temporary
scene in `examples/ascii.rs` for this investigation and removed again
afterward -- it is not part of the committed tree; the numbers below are what
it measured.

**The scene digs an unroofed crater at the top of the bank regardless of
body width -- this part is a pre-existing, width-independent quirk, not the
cause.** Row-by-row void at the final frame (`depth` counted down from the
bank's own top row), both arms:

| depth | width on (shipped) | width off (`LATERALS=0`) |
|---|---|---|
| 0 | 57 | 56 |
| 1 | 51 | 51 |
| 2 | 45 | 47 |
| 3 | 42 | 41 |
| 4 | 39 | 37 |
| 6 | 29 | 29 |
| 8 | 18 | 17 |
| 11 | 8 | 3 |
| 12 | 3 | -- |
| 27-29 | **0** | **4, 6, 4** |

The two arms carve a near-identical crater open to the bank's own top surface
across the first ~12 rows -- a colony quarrying the open face rather than
tunnelling into it, exactly the shape `CLAUDE.md`'s own metric-traps section
already names ("standing void is not dug void... a pit is standing void").
Every cell in that crater is unroofed by construction (nothing solid stands
between it and the sky), and both arms dig it the same way, so **this is not
what body width breaks.**

**What width breaks is reaching the *second* gallery, deep by the stone
floor, that the width-off colony finds and the width-on one never does.**
The width-off arm alone digs down to depth 27-29 (4-6 void cells per row,
tucked under nearly thirty rows of intact soil) -- a genuine tunnel with an
intact roof, which is exactly what `roofed 14` at the final frame is reading.
The width-on colony never reaches anywhere near that depth in the full 8,000
frames; its `roofed` count is **0** at every window from frame 1,000 onward,
not merely low.

**Why it never gets there: the same crowding cost task B found, now
starving a colony instead of merely slowing a swarm.** Over the identical
8,000 frames, one binary each:

| | width on (shipped) | width off (`LATERALS=0`) |
|---|---|---|
| digs, final | 96 | **133** (+39%) |
| moves blocked / moves, frame 7,000 | 878/1,396 = **63%** | 760/1,406 = 54% |
| deaths | **5 of 6** | 1 of 6 |
| death cause, every one | STARVED | STARVED |

Every death in both arms is `DeathCause::Starved`, not a cave-in or a bite --
several of the width-on colony's dead were last seen well outside the bank
entirely (e.g. `(0,111)`, `(17,105)`, `(20,104)`), consistent with a colony
spending an outsized share of its ticks blocked or wandering rather than
digging or feeding, exactly the pattern of "burns energy without progress"
task C's own hypothesis (3) named. The width-2 body, self-interfering in its
own dug gallery the way eight width-2 attackers interfere with each other
around one beetle, digs 28% fewer cells and starves five times as many ants
in the same budget -- and never lives long enough, as a colony, to reach the
depth where a stable roof would read as `roofed > 0` at all.

**This is a real, geometric-and-behavioural consequence of width, not a
one-line accounting bug or a placement predicate -- so per this task's own
branch, it is written up rather than patched, and the assertion stays red.**
Nothing here counts a body cell as fill (the void/roofed instrument used
above is the scene's own, unmodified), and nothing here is a placement
defect (task A's fix is orthogonal -- it changes whether a *site* founds, not
how a *standing* colony digs). A 2-wide body cannot dig a roofed chamber the
way a 1-wide one does, in this scene, because it cannot survive long enough
to reach one: fixing that is a burrow-geometry or body-shape design decision
belonging to whoever owns that line next (the PR body already named this
seam), not a repair this task's mandate covers. `nest_dig_scene`'s `assert!
(roofed > 0, ...)` is left exactly as written; `cargo run --release --example
ascii` still panics there, and every scene before it in the catalogue is
green.

## 13. The mechanics of a long body in terrain (2026-09-10, branch `claude/creature-mobility-r27`)

**The owner's ruling that opened this task, verbatim:** *"You are examining a
lot how many survive and how much they dig, etc. You are focusing on these
downstream statistics, but this seems more like a mechanical issue that these
larger ants get stuck or cannot move easily in more complicated terrain.
Let's fix that."* So there are no survival, dig or birth numbers below. The
question is mechanical, and it has a mechanical answer.

**The answer in one sentence: a body longer than two cells cannot turn
round, and almost every time one of them is stuck, that is why.** Not width
-- a one-wide six-cell chain and the two-wide articulated ant are stuck
*identically* (87.1% of steps refused each, in the tunnel scene below).
Length alone.

### 13a. The instrument: a blocked-step classifier

`moves_blocked` says an animal did not move. It cannot say whether a wall
stopped it, its own body stopped it, or there was nothing to stand on -- and
those want three different fixes, which is why a blocked *fraction* has
never been able to aim one. `creature::BlockedWhy` is the fixed set of
reasons, derived from the only two predicates that can refuse a move
(`landing_is_placeable_through_tissue` and `body_has_foothold`) rather than
from a guess about terrain:

| reason | what refused the step |
|---|---|
| `head_solid` | the head's target is solid world material |
| `head_tissue` | ...is living plant tissue a body may not part (a bole) |
| `head_edge` | ...is outside the world |
| **`head_on_self`** | ...is one of this body's **own** cells, and that cell is not the one vacating this tick |
| `body_fold` | two non-head landing cells want one position |
| `body_blocked` | a non-head landing cell is refused by the world |
| `no_foothold` | the landing is legal and there is nothing to hold on to |

`classify_step` runs the walk's own predicates over the landing
`body_after_step` actually produces -- deliberately not a second, cheaper
model of what refuses a move, because a classifier that disagrees with the
code it classifies is worse than none: its histogram still looks like an
answer.

**Two counters carry the diagnosis, and the histogram alone does not.** The
walk scores only three candidates, so a histogram over those three cannot
tell "facing the wrong way" from "stuck". `boxed_ticks` counts blocked ticks
on which **none of the eight headings** can be walked; `boxed_self_ticks`
counts those where at least one heading is refused by nothing but this
body's own cells. The first says the animal is stuck rather than merely
turned around; the second says a shorter animal in the identical cell would
not have been.

**Controls, both directions, before any of it was trusted.**
*Specificity*: `body_blocked` is 0 in every run in this section (every
non-head cell of a `chain_follow` landing is a cell the body already
occupies, so nothing there can fire -- a non-zero count would mean a body
plan placing a cell it never checked), `body_fold` is 0 everywhere (the §7f
tuck rule is doing its job; laterals are not causing collisions), and the
two-cell ant reads `boxed 0` and `head_on_self 0` on **all four scenes**, by
construction: its only body cell is its tail, and the tail vacates on the
same tick. *Sensitivity*: the tunnel scene below, where the same counter
reads 99.7%.

`PIXEL_PHYSICS_BLOCKED_CENSUS=1` turns it on. Off by default, so nothing in
the shipped game pays for the extra eight-way scan it adds to a tick that
has already given up.

**Two scenes were added to `creature_scale` because no generated preset
contains the terrain the complaint is about.** `preset=tunnel` is solid rock
with one-cell-wide passages cut through it -- a dead-end tunnel, a passage
with two right-angle bends, and a vertical shaft, with half the animals
started *inside* the passages rather than left to find them (hoping a
wanderer finds a tunnel is how a scene ends up measuring wandering).
`preset=chamber` is `ascii`'s own `nest_dig_scene` geometry, where the
passages are the ones the animals cut themselves and are therefore exactly
as wide as the body that made them.

**One defect found in the harness on the way, and it invalidated an
ablation that already existed.** `body=chainN` wrote `def.body` and nothing
else -- but `place_creature` grows the body from the species' `fates` table
whenever it has one and only falls back to `def.body` when it does not, and
`ant.ron` and `hopper.ron` both author one. So the override was **silently
ignored** on exactly the two species anyone would use it on.
`SpeciesRegistry::set_fates`'s own doc names this trap in as many words;
this call site is the second to walk into it. Fixed by clearing the table
with the override, and the tell that catches it is the one `CLAUDE.md`
names: `body_cells=` in the harness's own row must move when the override
does, and it did not. **Note what this costs the `chain6` arm below**:
clearing the fate table also removes the body's role differentiation, so
`chain6` is a control for *length* and is not a control for anything the
economy reads.

### 13b. The table

`creature_scale mode=walk`, seed 7, 4,000 frames, `RAYON_NUM_THREADS=4`,
one binary, `PIXEL_PHYSICS_BLOCKED_CENSUS=1`. `boxed` and `boxed_self` are
percentages **of blocked ticks**; the two causes are percentages of the
per-candidate histogram.

| scene | body | cells | blocked | boxed | boxed_self | top cause | 2nd cause |
|---|---|---|---|---|---|---|---|
| `flat` | two-cell ant | 2 | **0.6%** | **0%** | **0%** | head_solid 95.0 | head_edge 5.0 |
| `flat` | 6-cell chain | 6 | 1.8% | 61.0% | 61.0% | head_solid 43.3 | head_edge 25.1 |
| `flat` | 5-cell spine | 5 | 2.0% | 65.0% | 65.0% | head_solid 44.6 | **head_on_self 21.2** |
| `flat` | 2-wide articulated | 7 | 1.9% | 40.7% | 40.7% | head_solid 66.7 | **head_on_self 11.4** |
| `rolling` | two-cell ant | 2 | **4.0%** | **0%** | **0%** | head_solid 95.8 | head_tissue 4.2 |
| `rolling` | 6-cell chain | 6 | 10.6% | 80.8% | 80.8% | head_solid 57.5 | **head_on_self 26.7** |
| `rolling` | 5-cell spine | 5 | 14.8% | 87.9% | 87.9% | head_solid 52.2 | **head_on_self 29.3** |
| `rolling` | 2-wide articulated | 7 | 22.1% | 91.1% | 91.1% | head_solid 65.0 | **head_on_self 21.0** |
| `tunnel` | two-cell ant | 2 | **5.1%** | **0%** | **0%** | head_solid 100.0 | -- |
| `tunnel` | 6-cell chain | 6 | **87.1%** | 99.5% | 99.5% | head_solid 86.7 | **head_on_self 13.3** |
| `tunnel` | 5-cell spine | 5 | **84.4%** | 99.6% | 99.6% | head_solid 87.5 | **head_on_self 12.5** |
| `tunnel` | 2-wide articulated | 7 | **87.1%** | 99.7% | 99.7% | head_solid 86.8 | **head_on_self 13.2** |
| `chamber` | two-cell ant | 2 | **8.6%** | **0%** | **0%** | head_solid 100.0 | -- |
| `chamber` | 6-cell chain | 6 | 26.4% | 86.3% | 86.3% | head_solid 77.3 | **head_on_self 17.6** |
| `chamber` | 5-cell spine | 5 | 21.5% | 73.6% | 73.6% | head_solid 70.5 | **head_on_self 16.2** |
| `chamber` | 2-wide articulated | 7 | 23.0% | 88.2% | 88.2% | head_solid 61.7 | **head_on_self 17.5** |

**`boxed_self` equals `boxed` to the last count in all sixteen rows.** Every
single tick on which a long body has nowhere to go is a tick on which at
least one direction is refused by its own cells and nothing else. That is
not a tendency, it is an identity, and it is the finding.

**Width costs nothing here.** In the tunnel the one-wide six-cell chain and
the two-wide seven-cell ant are refused on 87.1% of steps *each*. §12 read
63% against 54% on the chamber scene and attributed it to width; on this
instrument the same scene reads 23.0% (two-wide) against 26.4% (one-wide
six-cell) -- the wide body is the **better** of the two. Whatever §12
measured, body width is not what stops a long animal in a tunnel.

**Flicker.** Segment widths change on 0.14 of committed moves on `flat` and
`tunnel`, 0.36 on `rolling`, 0.57 on `chamber`. Per *tick*, which is what an
eye sees, the stuck tunnel body changes width **0.0065 times a tick** -- one
change every 154 ticks. So the tuck rule is not strobing, and the owner's
"stuck and just flashing" is not lateral tucking; it wants its own
reproduction on the colony scene before anything is built for it.

### 13c. The mechanical diagnosis, in plain words

**What a two-cell ant can do that a five-cell one cannot: reverse.**

A body follows its head. When the head steps, every cell behind it moves
into the cell the one ahead of it just left, and the landing may not put two
cells in one place. Work through which of its own cells a head may therefore
legally land on, and the answer is exactly one: **the tail**, because the
tail is the only cell nothing else is moving into. Every other own-cell is
still occupied by the segment behind it at the moment the head would arrive.

For a two-cell ant the tail is the head's own neighbour. Reversing is one
ordinary step, always available, and that is why it reads as an animal that
turns on the spot. For a five-cell body the tail is four cells away and
cannot be reached in a single step -- so **no** own-cell is ever a legal
landing, and a long body has no way to go backwards at all.

On open ground this barely shows: there is almost always somewhere else to
go, and the cost is the odd wasted tick (`flat`, 1.9% of steps). It becomes
the whole story the moment the animal is somewhere it can only leave the way
it came in -- a dead-end tunnel, a bend it has overshot, a gallery it dug
itself. There, "go back" is the only move, and it is the one move the rule
forbids. The animal stays until it starves. That is the owner's report,
exactly: *stuck in more complicated terrain*.

Sorted by what each is:

- **Geometry, and no rule can change it.** A long body cannot turn round
  *inside* a one-wide tunnel by walking, and real ants cannot either. It
  cannot step straight up a face while its spine is horizontal. It cannot
  cross a gap. These are properties of being long in a grid.
- **The rule's own choice, and this is the whole of the problem.** That the
  only way to reverse is to *walk* backwards head-first, which the
  duplicate-position rule then forbids. Nothing about the world requires
  that. The body already occupies a legal set of cells; there is no physical
  reason it may not simply face the other way.
- **Not a cause at all, measured rather than assumed.** Width
  (`body_fold` 0, and the one-wide chain no better than the two-wide ant);
  the lateral tuck rule (§7f) (`body_fold` 0 on every scene); foliage
  (`head_tissue` at most 4.2%, and 0 in both underground scenes); footing
  (`no_foothold` never above 19% and 0 in the tunnel).

### 13d. The rule: turn the body round where it stands

**This is the owner's own proposal, relayed mid-task and adopted:** *"cannot
we just allow the creature to flip its whole body, so no U-turn is needed?"*
It is the right shape for the reason above -- the problem is not that the
animal cannot get anywhere, it is that it cannot get anywhere *facing the
way it must go*.

**The rule.** When an animal is refused in all eight headings, the segment
order reverses end for end. **No cell moves.** There is no landing to place,
no duplicate to avoid and no U-turn geometry to satisfy; the body already
stands in a legal set of cells and it goes on standing in exactly those
cells. The head is now the end that was the tail, and the next step is an
ordinary forward step from it.

Priced honestly, since the coordinator asked for each of these by name:

- **Roles.** `relocate_chain` carries cell contents by segment, so the
  head's own cell travels to what was the tail's position: the animal comes
  out **mirrored** -- the mouth is now at the far end. The alternative
  (roles stay on their cells, the animal walks backwards) is a bigger change
  and buys nothing the economy can see: **every role resolver reads a
  *fraction* of the live body**, and reversing an order changes no multiset,
  so no cell count, no armour share and no gut share moves either way. The
  choice is therefore purely what reads right, which is why it went to the
  owner as a card rather than being decided here.
- **A lateral's authored side: the question does not arise.** There is no
  stored side to mirror. `lateral_for` re-derives the side from world-space
  up-then-left on every step, so a flipped body simply re-lays its laterals
  through the same call the walk uses, and the §7f tuck rule applies
  unchanged.
- **Facing.** `heading` reverses with the body, `(heading + 4) % 8`.
- **Per-segment scalars.** None survive an ordinary step either --
  `World::set`'s `reindex_organism_cell` seam gives every relocated cell a
  fresh sidecar -- so a flip smuggles nothing back (§7f established this;
  re-checked here rather than inherited).

**When it fires.** Only when `is_boxed` -- refused in all eight headings --
and only if the reversed body can then walk, tested by the same `is_boxed`
on the far side. An animal that merely faces the wrong way is not stuck and
`tumble` is the whole remedy; a flip that does not open a direction would be
a strobe, since nothing moved. Gating on *boxed* rather than *blocked* is
also what keeps the verb off the hot path: it runs on 1.9% of ticks on open
ground.

**Measured against walking backwards, one binary, `PIXEL_PHYSICS_REVERSE=
off|flip|back`.** The `back` arm is the honest alternative: the tail leads
into a free cell and every segment inherits the position of the one behind
it -- `chain_follow` in the other direction, roles staying on their cells.

Blocked fraction, same scenes, same seed:

| scene | two-cell ant (the bar) | 2-wide, off | 2-wide, **flip** | 2-wide, back | 6-chain, off | 6-chain, **flip** | 6-chain, back |
|---|---|---|---|---|---|---|---|
| `flat` | 0.6% | 1.9% | **2.3%** | 2.7% | 1.8% | **1.5%** | 2.6% |
| `rolling` | 4.0% | 22.1% | **7.1%** | 14.3% | 10.6% | **5.9%** | 11.4% |
| `tunnel` | 5.1% | 87.1% | **7.6%** | 49.0% | 87.1% | **11.0%** | 58.4% |
| `chamber` | 8.6% | 23.0% | **11.5%** | 18.0% | 26.4% | **13.3%** | 14.2% |

**The flip wins on every scene, and the bar is met.** The brief's bar was
"the two-wide body within a few points of the two-cell ant's on every
scene": 2.3 against 0.6, 7.1 against 4.0, 7.6 against 5.1, 11.5 against 8.6.
Steps *taken* in the tunnel go 388 to 3,004. No deaths in any arm.

**`flat` gets very slightly worse (1.9% to 2.3%) and that is arithmetic, not
a regression**: a flip is booked as a blocked tick, because the animal did
not get anywhere, and 34 flips on that scene are almost exactly the 28-tick
difference.

**Backing out is worse everywhere and the reason is structural.** It fires
constantly and resolves nothing -- 809 reversals in the tunnel against the
flip's 136, and 49.0% blocked against 7.6%. A backward step does not change
which end the head is; the animal is boxed again on the next tick and backs
out again, and when it finally clears the mouth the head is the last thing
out, facing back down the tunnel it just left. The flip terminates because
it changes the *state* the boxing depends on; backing out only changes the
position, so it has to be repeated until the geometry happens to relent.

**One number in the first run of this comparison was about the gate and not
about the rule, and is recorded so it is not re-derived.** Gating both arms
on the flip's own delivery test ("the reversed body must be able to walk")
measured backing out at **8 reversals against 2,371 refusals** -- which
reads as "backing out never works" and is a fact about the gate: a single
backward step cannot unbox a head, so the test rejected every one of them.
The table above uses the right gate for each rule.

### 13e. What is built, and what a build lane still owes

**Built on this branch, and default-off.** `PIXEL_PHYSICS_REVERSE` is unset
in the shipped tree, so behaviour is unchanged until someone turns it on;
the switch exists so both arms live in one binary. The pieces are
`ReverseRule`, `spines_of`, `lay_out_along`, `flipped_body`,
`backed_out_body`, `is_boxed`, and the gate in `step_chain`'s blocked branch
(`src/sim/creature.rs`), with `CreatureStats::reversals` /
`reversals_refused` as the "did it fire" pair.

**Why it is not on by default, and this is the whole of what is owed.** One
paired `ascii` run, off against flip:

| | off | flip |
|---|---|---|
| colony foraging: moves | 6,590 | 8,227 |
| ...blocked | 357 | 315 |
| ...**round trips** | **32** | **19** |
| ...**deliveries** | **23** | **0** |
| ...deepest excursion | 74 | 119 |
| chamber: moves | 1,400 | 1,951 |
| ...blocked | 1,000 | 206 |
| ...digs | 96 | 77 |
| ...**roofed void** | **0** | **0** |

Mobility improves exactly as the table in 13d says. **Foraging changes, and
not for the better**: an ant that can turn round ranges further and stops
coming home. That is `CLAUDE.md`'s standing warning in its usual costume --
the trail-following constants were calibrated against a colony that could
not reverse, and a correct mechanism at inherited constants is a regression.
**Re-deriving them is part of the work, not scope creep.** It is one paired
run per arm and outcomes here have enormous spread, so the first job is to
confirm it reproduces before tuning anything.

**And the chamber's `roofed > 0` is not delivered.** Blocked falls from
1,000 to 206 and the colony still leaves no roofed void: the flip fixes
getting *stuck*, not what the colony chooses to dig. §12's crater -- a
colony quarrying the open face rather than tunnelling into it, which both
width arms did identically -- is untouched by anything here and remains
whoever owns burrow geometry's to answer.

**A defect this work introduced and fixed, worth carrying because the class
outlives it.** The first build of the flip deleted the bodies of plain
`Chain` animals outright -- `deaths: Killed 13` on the tunnel's `Chain(6)`
arm. `relocate_chain` falls back to one-cell-per-segment only when **both**
sides' group lists are empty; handing it an empty `from` and a `vec![1; n]`
`to` makes its carry loop zip an empty list against a full one, carry
nothing, clear the old body and write none of it back. The
`debug_assert_eq!` that names this exact failure is compiled out in release,
which is where every measurement in this section is taken -- and the
"improvement" it produced looked wonderful: blocked 87.1% to 3.4%, because
an animal that no longer exists is never blocked. `CLAUDE.md`'s *a cost that
vanishes may be work that vanished*, and what caught it was printing deaths
by cause beside the mobility numbers rather than `alive=` alone.

**The order of work.**

1. **Settle the picture.** Card `20260910T193810051Z-9f00a9` (board `lab`)
   asks the owner whether the mirrored turn reads as an animal turning round.
   If it does not, the alternative is roles-stay-on-cells (the animal walks
   backwards, mouth trailing) -- same trigger, same numbers, a different
   `relocate_chain` correspondence.
2. **Re-derive foraging against a colony that can reverse**, and only then
   turn `PIXEL_PHYSICS_REVERSE=flip` into the default. The gate is
   `ascii`'s own `forage_trips` and `deliveries`, back at or above the
   pre-articulated-body baseline (23 deliveries, 32 trips).
3. **Then the frame cost**, paired and alternating, quoting the whole-frame
   figure from `ascii scene=foraging`'s `worst`/`mean` line and running the
   pinning check before quoting any worst. The verb runs on ~2% of ticks and
   costs one eight-way scan when it does; that is the claim to check.

**What must not change**, in the same form §7f(2) used:

| | |
|---|---|
| `chain_follow` | the spine rule is untouched; a flip is not a step |
| `is_rigid()` | stays false for `Segmented`; a rigid body does not flip |
| the duplicate-position check | **unrelaxed**, and the flip is what makes it satisfiable: a reversed body occupies the cells it already occupied |
| `a_lateral_free_segmented_body_is_byte_identical_to_a_chain` | must stay green -- the flip is written for `Chain` and `Segmented` alike, off the same `spines_of` |
| `assets/species/*.ron` | untouched; this is a movement rule, not a body |

**The tests**, both watched red before being trusted:
`a_long_body_is_boxed_in_a_dead_end_where_a_two_cell_body_is_not` (the
diagnosis as a property, with the two-cell body in the identical cell as its
control) and
`flipping_a_body_reverses_its_order_and_keeps_the_empty_groups_convention`
(which goes red against the deleted-body defect above).
`cargo test --lib -- sim::creature::`: **164 passed, 1 failed, 10 ignored**
-- the one failure is §9's own guard, red before this branch and left red per
§7f(6). `cargo clippy --all-targets --release --locked -- -D warnings`:
clean, with the three lints it raised answered by simplifying signatures
rather than by `#[allow]`, per §7f(7)'s precedent.

### 13f. Founding is a separate question, and stays separate

A site is still refused unless the spine's straight-line footprint fits,
evaluated once before the animal has taken a step (§10 made that check agree
with the movement rule; it did not remove it). That is why a colony seats 5
of 52 and a played bed 12 of 52 -- and §10's own control settles that width
is not the cause, because a bare spine hits the identical ceiling.

**The proposal, one paragraph and deliberately not built here.** Found along
the surface *contour* rather than along a straight line: take the spine's
cells from the ground profile under the head, the way a chain lies when it
walks over rough ground, instead of demanding `n` empty cells in a row at
one height. A straight-line footprint is a shape a walking body only has on
flat ground, so on any real terrain the founder is being asked to fit
somewhere it would never stand anyway. The alternative -- a founder that
digs itself in -- is a bigger change and needs the movement rule above
first, since a founder digging into a bank is precisely the animal that then
has to reverse out of its own hole.

### 13h. Founding along the surface (2026-09-10, lane G, branch `claude/creature-founding-r27`)

**Built as proposed.** `founding_spine_walk` replaces the straight-line spine
with a per-segment walk behind the previous cell, tried in this order: the
flat cell if it has a foothold (`head_has_foothold`, the same ground-contact
test a step's own footing check uses -- "along the surface"); the up-diagonal
then the down-diagonal, each also requiring a foothold ("the diagonals" --
climbing a rise or curling down a ledge); and finally the flat cell again with
no foothold requirement at all, the original rule kept as the last resort so
a segment over open ground is never refused *more* often than before. The
walk **is** the viability check now -- a site is refused the moment any
segment finds nothing placeable in any tier, in place of the old "all `n`
cells empty in a row" scan. Laterals are unchanged: still `lateral_for`, over
whatever spine the walk lays, exactly as the movement rule computes one for a
live step.

**Verified byte-identical to the old straight line wherever the old line was
ever asked to work at all** -- tier 1 requires the flat cell to have a
foothold, which on any bed the old rule already founded on (flat ground under
every segment) it always does, so the walk never reaches tiers 2 or 3 there.
Direct comparison on `a_swarm_gets_through_what_one_mouth_cannot`'s own scene
(a 100-column stone floor) gave the identical chain under both the pre-fix
inline straight line and `founding_spine_walk`: `[(96,119), (95,119),
(95,118), (94,119), (94,118), (93,119), (92,119)]`, groups `[1,2,2,1,1]`.
That test's own failure (median frame 60 alone against 79 for eight, over 8
seeds) reproduces identically on the unmodified base branch and is unrelated
to this change -- confirmed by running it alone against both binaries before
touching anything.

**The measured counts, one binary each arm, `RAYON_NUM_THREADS=4`.** Three
bodies on two scenes: the articulated ant with the contour lay (this build),
the same species with the straight lay (this build's Segmented arm reverted
to the old formula, nothing else), and `ancestor` -- the shipped `Chain(2)`
body (`assets/species/ancestor.ron`), which never reaches
`founding_spine_walk` at all and is therefore the control for "how many sites
exist for *any* body" on this same terrain, independent of this build.

`labshot scenario=played_bed frames=6100,30000` (the bed merged from
`origin/main`, with its thicket and tree; `colony_species=` swaps who the
timeline's `Colony` event founds, added to `labshot`/`filmstrip` this build):

| seed | straight lay (before) | contour lay (after) | `ancestor` (two-cell) |
|---|---|---|---|
| 1, frame 6,100 | 11 | **13** | 29 |
| 2, frame 6,100 | 13 | **15** | 26 |
| 3, frame 6,100 | 13 | **14** | 19 |
| 1, frame 30,000 | 23 | 12 | 41 |
| 2, frame 30,000 | 31 | 28 | 60 |
| 3, frame 30,000 | 37 | 39 | 32 |

(Frame 30,000 moves with births and deaths on top of the founding count, not
with founding alone -- read the 6,100 row for what this build changed. Seed
1's 30,000 figure for the contour arm reads lower than its own 6,100 figure,
which is deaths outpacing births on that one seed rather than anything about
founding; included for completeness and not leaned on.)

`filmstrip scene=colony` (generated wetland, seed 1, `origin/main`'s wetland
preset, matching §10's own scene exactly):

| arm | founded of 28 viable sites (of 52 asked) |
|---|---|
| straight lay (before) | 5 -- matches §10's own recorded number exactly |
| contour lay (after) | **9** |
| `ancestor` (two-cell) | 18 |

**The contour lay is a real, repeatable, attributable gain -- nearly doubling
the bare-terrain count (5 -> 9) and adding 1-2 seats on every played-bed seed
(11/13/13 -> 13/15/14) -- and it does not reach the two-cell ant's count on
either scene.** The bar this task set was "within a few of the two-cell
ant"; on `played_bed` the gap closes from 15-16 down to 5-16 depending on
seed, and on the bare colony scene it closes from 13 (of 28) down to 9 --
better, and still short.

**What still refuses a site, so far as the numbers say without a per-cell
classifier: mostly not body length.** The `ancestor` control is the tell --
a genuinely two-cell body, spine-only, laid exactly the way it always was,
still only seats 18 of 28 "viable" columns on the bare scene and 19-29 of 52
asked on the played bed. `colony_ant_site`'s own "viable" count is a
single-column probe (is there a floor cell and one empty cell above it) that
neither body actually has to honour once `colony_stations`' spacing --
derived from `body_span`, which widens for a longer authored footprint -- has
moved the candidate columns to match a wider corridor. So part of the
remaining gap between `ancestor` (18-29) and the contour ant (9-15) is
consistent with §10's own reading: the played bed's grown vegetation, its
seed litter, and a spacing corridor sized for a longer body **are already
present in the `ancestor` arm's own shortfall against 28/52**, and are not
this task's defect to close. The narrower gap this build owns is between the
straight lay and the contour lay at the *same* spacing and the *same* bed --
5->9 and 11-13->13-15 -- which is the whole of what `founding_spine_walk`
can move, and it moved it in the right direction on every seed measured, on
both scenes, never fewer.

**No placement observed to overlap bodies or interpenetrate.** The
`founding_laterals_match_lateral_for` guard (below) and the x-monotonic
construction of `founding_spine_walk` (every candidate a tier considers
shares `prev.x + back.x`, so no two spine cells can ever land on the same
cell) rule this out by construction rather than by sampling; the review card
below is the eyes-on check `CLAUDE.md` asks for on top of that.

**Four tests, watched red first** (`src/sim/creature.rs`, beside
`a_founding_site_with_a_blocked_lateral_still_founds_tucked`):
`a_two_segment_founder_walks_exactly_as_the_old_straight_lay_did` (direct
equality against the old formula on flat ground);
`a_five_segment_body_founds_curled_over_a_step` (a two-cell-wide flat under
the head then a one-row drop; asserts the walk descends onto the lower level
rather than hanging over the drop in a straight line -- watched red against a
build with tiers 1/2 disabled: it fails exactly there, spine
`[(50,119)..(46,119)]`, flat all the way over the step, while the other three
new tests stay green under the same disabled build, since they test
reduction and refusal rather than the curl itself);
`a_single_free_cell_refuses_a_multi_segment_founder` (a site with one open
cell refuses a five-segment body); `founding_laterals_match_lateral_for`
(reconstructs the spine from a founded ant's own `(chain, segment_groups)`
and asserts each widened segment's lateral equals `lateral_for` recomputed
against the *pre-spawn* world -- caught by watching its own first draft red:
recomputing against the post-spawn world sees the organism's own
already-placed lateral cell as occupied and reports a tuck that never
happened).

`cargo test --lib -- sim::creature::`: **170 passed, 1 failed (pre-existing,
unrelated -- see above), 10 ignored**. `cargo clippy --all-targets --release
--locked -- -D warnings`: clean.

**Review card**: `20260910T225153157Z-7f9bac` (board `lab`) -- the played
bed's nest band at frame 6,100, seed 1, 13 ants founded of 52, asking whether
the founded bodies lie flat and separate on the soil. Posted, not yet
answered as of this writing; `python3 scripts/review.py inbox` picks up the
verdict.
