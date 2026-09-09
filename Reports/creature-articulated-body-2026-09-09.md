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
| `body_energy` | 480 | the meat stamp **and** `birth_cost = grant + body_energy × cells` |

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

**Two of the five `FateWhen` variants are meaningless to a body**, and this
has to be handled or it is a small silent waste. `Node`, `Flush` and `Ripe`
are plant lifecycle events; a body unfold reads `Grew` and (for a terminal
segment) `Stale`. `recondition_one` draws its `when` uniformly from all five,
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

## 7. Status, and what was built alongside this

**Built and shipped with this report:** the ant and the hopper as
articulated bodies from a heritable growth program, as the defaults. The
measured frame cost and the review card are recorded here.

<!-- BUILD RESULTS -->

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
