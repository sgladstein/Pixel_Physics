# Walking through plants: what actually stops an ant, and what was done about it

**2026-09-06.** Built and measured. The owner's ask, in his own words:

> *"I want to revisit making it so creatures can walk through plants. I know
> there are complications but it just feel unrealistic that plants are solid
> walls to ants/creature."*

This report is mostly the measurement, because the measurement **moved the
answer**: the wall an ant meets in a wood is not foliage, it is trunk, and
that is not what anyone including this author expected going in.

---

## 1. The state before, and the asymmetry nobody had noticed

Two facts about living plant tissue were already true and had never been put
side by side:

- **`Plant` is a foothold.** `step_chain`'s support test is 8-neighbour and
  includes `MaterialKind::Plant`, with a source comment saying in as many
  words that ants climb walls and ceilings. An ant can stand on a tree.
- **`Plant` is not enterable.** `landing_is_placeable` required
  `World::is_empty`, so every cell of living tissue was a wall to a body.

Meanwhile the *gnome* has walked through living plants since M16, by owner
ruling, and `wood.ron` says so in its own header:

> *"Walk-through and climbable, while it is alive. A growing tree is scenery
> you move past and go up, not a wall you stop against — the gnome used to
> wedge against trunks with no way round and no way through."*

`Material::climbable` is read only by `player.rs`. So the rule the owner
had already made for the player had simply never been extended to anything
else, and the complaint is that asymmetry showing.

`wiki/ants.md` had already recorded the *cost* of it without naming it as a
defect — *"a column with a trunk standing in it has nowhere for an ant to
stand… expect somewhere between a third and half of a colony to be lost to
water and trees on rough, wooded, wet ground."*

## 2. What the wall is actually made of

`CreatureStats::blocked_by_plant` / `blocked_tissue_freed` /
`blocked_tissue_freed_any` were added at the one site that refuses a step,
plus `World::blocked_tissue_by_material` for the breakdown. `filmstrip`
prints all four.

`scene=colony genome=authored`, 12,000 frames, four world seeds:

| seed | moves | blocked | of moves | tissue among blockers | wood share of that |
|---|---|---|---|---|---|
| 1 | 9,341 | 2,241 | 24.0% | 1,423 (63.5%) | **96%** |
| 2 | 8,026 | 831 | 10.4% | 205 (24.7%) | **80%** |
| 7 | 9,498 | 677 | 7.1% | 123 (18.2%) | **78%** |
| 12 | 9,396 | 917 | 9.8% | 281 (30.6%) | **69%** |

Two things to read off it.

**Living tissue is between a fifth and two thirds of every blocked step**,
which says the complaint is real and is not a matter of taste.

**And 69–96% of it is `wood`.** Leaf ran 3–24%, grass 0–13%, `rootwood`
about 1%. The intuition this work started from — that ants are being stopped
by *foliage*, or by roots underground — is wrong in both directions.

Two controls, both cheap and both necessary:

- **Specificity.** `scene=hunt` is a stone floor with no plants in it: tissue
  **0** of 35 blocked ticks. The counter is quiet when nothing is wrong.
- **The identity.** `tissue` and `freed-if-any-tissue` are **equal on every
  seed**. Whenever tissue was among the obstructions, tissue was the *whole*
  obstruction for at least one candidate — plants are not merely present
  alongside rock, they are the thing in the way.

### 2a. Pricing grass was measured, and it is not the fix

`grassblade` and `grassroot` author no `penetration_resistance` and so sat at
the 100.0 default — a meadow priced like heartwood. Fixing that is correct on
its own terms and **it barely moves this number**: `freed-if-soft` went
77 → 86, 26 → 65, 37 → 37, 76 → 92 on the four seeds, against a tissue total
of 1,423 / 205 / 123 / 281. The data gap was real and it was not the story.

## 3. What was built

**Soft living tissue is passable; a body that enters it holds it and puts it
back.** `organism::Parted` is a lifted tissue cell — material *and*
`OrganismCell` scalars — and `relocate_chain` lifts what the new body
position covers and restores what it no longer covers, so the foliage closes
behind the animal. `creature_dies` restores the lot before the corpse is
written.

Three gates decide "soft", and none of them is a material name:

- `MaterialKind::Plant`, so this is tissue and never rock, clutter or flesh;
- `organism_id() != 0` — it must be **alive**, the same gate `climbable` is
  read behind, so a `wood` wall someone painted stays as solid as it was;
- `penetration_resistance <= dig_force`, the pattern roots and `act`'s dig
  gate already use. `leaf`/`flower`/`moss` are 0.1, `fruit` 0.2, `grassblade`
  now 0.1; `wood` and `rootwood` author none and stay at the 100.0 default.

**The justification for foliage and not wood is a resolution argument, not a
biological one.** A bush is air with leaves in it; the grid cannot hold the
air, so foliage draws solid when it physically is not, and this is the
correction for that artifact. A trunk really is solid, so it stays solid.
An ant walks *on* a tree.

Because the force is the dig allele, a lineage that evolves a stronger bite
also gets through thicker growth — no new gene.

## 4. That it fired, and that it took only what it should

`scene=hedge` (new): seven grown `shrub`, twelve ants pinned to meat guts so
the hedge is scenery rather than lunch, both arms from **one binary** via
`TISSUE_PARTING=0`.

| | foliage solid | foliage parts |
|---|---|---|
| moves | 4,016 | 4,321 |
| blocked | **666** (16.6%) | **180** (4.2%) |
| tissue among blockers | 628 (94.3%) | 125 (69.4%) |
| `freed-if-soft` | 459 (68.9%) | **0** |
| what the tissue was | wood 53%, **leaf 43%**, rootwood 4% | wood 82%, rootwood 18%, **leaf 0** |

`freed-if-soft` falling to **0** is the check worth having: there is no
blocked tick left that softening tissue would fix, so the mechanism took
exactly its own class and nothing else. Leaf leaves the blocker list
entirely. 4.2% is essentially the bare-stone rate (4.7% on `hunt`) — in a
hedge an ant now moves about as freely as in the open.

**`examples/ascii` is byte-identical across the change**, all 1,153 lines.
Its creature scenes are hand-built on stone and sand with no living tissue
beside the animals, so the gated suite cannot see this — which is a
statement about the gate, not evidence about the mechanism. The evidence is
the table above and `an_ant_walking_through_foliage_leaves_it_intact`.

That guard censuses leaf **count and carbon** across a hedge crossed by six
ants, and was watched failing for both of its faults: with the restore
disabled the hedge loses all 160 cells; with only the scalar restore
disabled the cells come back and their carbon does not (80 → 18). Its first
run failed a third way — the ants never moved, because the loop advanced the
scheduler without advancing `frame`, so nothing was ever due. The positive
control caught a blind test on its first outing.

## 5. The trunk half, and the mechanism that solved it

§5 of the first draft of this report said wood should stay solid, on the
grounds that an ant walking through a trunk is less realistic than one
walking round it. **The owner overturned that and was right:**

> *"in a 3D world, an ant can walk around the trunk of a tree. In this 2D
> world, creatures are getting stuck if they get surrounded by branches or
> between two plants."*

A side-view grid has no depth axis, so a trunk a real animal steps around is
an unbroken wall here. That is the same argument that justifies parting
foliage — a correction for something the grid cannot represent — one step
further. The consistency argument reinforces it: `Material::climbable` is
authored on `wood` and `rootwood` already, the gnome has walked through
living trunks since M16 on the owner's own ruling, and only the creature line
never read the flag.

### 5a. Letting a body occupy wood was built, and it kills plants

The obvious mechanism — reuse `Parted` for wood — works beautifully for
movement and is not shippable. On `scene=colony` seed 1 it took tissue blocks
from 776 to **17**, blocked steps 18.5% to 5.0%, and moves 7,471 to 11,422.
It also ends with the lab bed empty:
`lab::tests::copies_carry_what_was_planted_and_still_diverge` finishes at
`plant_cells 0` in all three copies with it on, passes with it off, and is
green on `main`.

**The mechanism is grid-resolved ownership.** `plant::is_structural_anchor`
opens `if cell.organism_id() != organism_id { return false }` against the
grid. A parted cell holds the animal, so it stops counting as an anchor, and
a seedling whose single base stem an ant is standing in becomes an unanchored
plant. Foliage never showed it because a leaf is never an anchor. The repair
is not that one line — several per-organism passes resolve a plant's own
cells through the grid and each would need to answer *"this is still mine, an
animal is merely standing in it"*.

One repair from that attempt was measured and is kept: a parted cell stays in
its plant's `cells` list, so the connectivity graph is not cut while a body
stands in it. Without it, seed 1 severed 7,845 cells against 1,773 and
snapped 28 against 10; with it, snapped is **2**. `PART_KEEP_GRAPH=0` is the
control.

### 5b. What shipped instead: crossing, not occupying

The owner's design, and it dissolves the problem rather than patching it:

> *"if an ant tries to go through a trunk, they basically just teleport to
> the other side with a delay long enough for however thick the trunk is, so
> they don't actually overlap with the cells ever."*

`organism::Crossing`. A body refused by living woody tissue looks along its
heading for the first place it could stand; if there is one it waits
`thickness x tick_interval` frames and appears there, charged the energy of
the walk it replaced. **It never enters the wood**, so there is nothing to
displace, nothing to restore, no hole in the connectivity graph and no anchor
to lose — the whole class of failure in §5a stops existing.

It is also graded for free. A one-cell stem is a blink and a bole is a long
wait, so the outcome has a middle without a constant tuned to give it one,
and the delay *is* the depth axis expressed in time.

`scene=colony genome=authored` seed 1, 12,000 frames, both arms from one
binary via `CROSS_TRUNK=0`:

| | wood solid | crossing |
|---|---|---|
| blocked steps | 15.7% (1,446) | **7.3% (735)** |
| tissue blocking | 656 | **216** |
| what still blocks | wood 88% / rootwood 12% | wood 65% / rootwood 35% |
| crossings | — | **243 begun, 243 completed, 0 abandoned** |
| limb severing | 3,987 | **3,770** |

Plants are very slightly *better* off, which is the check that matters: the
mechanism costs them nothing. The 216 blocks that remain are encounters with
no far side to come out on — inside a crown, or emerging into open air with
no foothold — which is the rule declining rather than a gap.

`grassblade` and `grassroot` gained `climbable` in the same change. They
authored none, so a meadow was priced as a wall, and once trunks could be got
round `grassblade` was **35% of everything still blocking an ant**.

### 5c. What it covers, and the one thing it does not

Both mechanisms sit in the shared `step_chain`/`relocate_chain` path and gate
on material data rather than species names, so every walking species has them
— `ant`, `ant_long`, `ancestor` (Chain) and `beetle`, `ant_block`,
`ant_block_shaded`, `ant_wide`, `chitin_pale` (Rigid) — and so will any
species or climbable plant material added later.

**The worm has neither, deliberately.** It runs on `worm_tick`/`move_cost`, a
separate path predating the chain creatures, where `MaterialKind::Plant`
returns `None`. It also moves by *overwriting* the cell it enters, so making
tissue enterable there would have it eat roots rather than pass them — §5a's
failure in a different costume. Teaching the worm path parting or crossing
properly is the open item.

## 6. Two method notes this work paid for

**An alarm raised from one seed.** "Foliage parting raises limb severing
4.8x" was read off seed 1 alone (1,773 against 8,546). Seed 2 puts the three
arms at 8,347 / 9,715 / 9,811 — the seed dominates, not the arm — and
mouthfuls eaten do not move either. `CLAUDE.md` says outcomes here are
chaotic in the seed and a six-seed sample is not a sweep; this was a
one-seed sample used to raise an alarm about merged code.

**A guard whose own census asked the wrong question.** The hedge guard
counted leaf cells in the grid — but a leaf an animal is standing in is
*deliberately* absent from the grid, so three ants living in the hedge read
as six destroyed leaves with `eats`, `digs` and `deaths` all zero. It counts
held cells too now. This is "ask what your number counts when nothing is
wrong" in the form where the wrong number belongs to the guard.
