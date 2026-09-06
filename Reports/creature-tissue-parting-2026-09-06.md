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

## 5. What was deliberately not done, and what to do next

**Wood was not made passable**, and should not be: an ant walking through a
trunk is less realistic than one walking around it, and it would stop a tree
being an object.

**But that is where the measured mass is** — 69–96% of tissue blocking — so
the complaint is not fully answered and this is the honest statement of what
is left. The mechanism is not passability, it is that **an ant meeting a
trunk has no notion of following it round or climbing it deliberately.** All
three of its forward candidates are inside the trunk, so it tumbles to a
random new heading and tries again; it gets up the trunk eventually because
`Plant` is a foothold and a vertical heading works, but only by chance.

The next measurement to take is whether biasing the re-roll toward headings
that run *along* the obstruction — wall-following, one rule, no new
passability — closes that 69–96% without making wood soft. It is a
`tumble` change, not a `landing_is_placeable` one.
