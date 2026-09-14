# Lane E — the forest being eaten (round 36)

*Branch `claude/evolution-lab-forest-eaten`. Owns `src/sim/creature.rs` for
this fix — handed over from Lane D, so it is held in slices and landed fast.
Opened 2026-09-14.*

## What this is

The owner is losing his plants in the lab. He isolated the condition himself:
**a bed with no creatures keeps its plants, the same bed with creatures loses
them**, hunks falling off first and total collapse later. §Z23 is the
mechanism and it was filed, designed and never shipped.

**It is shipped.** §Z23 is CLOSED in the register with the full account.

## The answer, in one line

**Grazing a plant made the plant look like an attacker, and the ant
retaliated against the tree.** On his own bed **100% of every swing and every
cell the jaw took was at a plant**, and the jaw was removing **nine of every
ten cells** a colony took off a living plant — feeding on none of them,
because `wood`/`rootwood`/`grassroot` carry no `food_energy` at all.

## What shipped

Both owner rulings, 2026-09-14, in `src/sim/creature.rs`:

1. **Creatures do not attack plants, no exception.** `nearest_foe`'s target
   rule now skips any cell that is not `MaterialKind::Creature` — the
   predicate its own odds count one line below was already using. The
   reopening condition (**a plant that can damage an animal**) is in
   `Reports/dead-ends.md` `creatures:082`, with the entry, per the ruling.
2. **Eating a plant raises no alarm.** Both feeding call sites gated on the
   victim being a living animal. A corpse needs no clause — `kind: Powder`,
   no organism id.

`PIXEL_PHYSICS_PLANT_FOE=on` restores both together for measurement and
reproduces every pre-fix column **byte-identically**, so the A/B is one binary
rather than two builds.

## The sizing, which is the part the coordinator asked for and did not have

Told to prove the attack path is the dominant term rather than assume it.
**It is, on this bed** — and note that §Z22 measured the *druid garden* and
found it a tenth there, so this is not a contradiction of that finding, it is
a different bed.

`latecensus scenario=played_bed_longant`, three seeds, 40,000 frames,
`RAYON_NUM_THREADS=1`. **Cells taken off a LIVING plant** — the comparison
nobody could make before, because no counter held both halves in the same
unit:

| seed | by the mouth | by the jaw | jaw share |
|---|---|---|---|
| 1 | 112 | **2,782** | 96.1% |
| 2 | 317 | **2,761** | 89.7% |
| 3 | 364 | **5,071** | 93.3% |

Swings: 3,501 / 4,497 / 7,442, **all of them at plants**. The paired
`no_colony=1` control — the owner's own — is exact: every column 0.

**The standing plant census cannot answer this, and it was measured proving
it.** Over the same three seeds the paired standing count moved **−84,
+3,463 and +3,484** — one arm reading *more* plant with the colony on it. A
stock carries everything the bed did as well as the loss. Hence
`eaten_plant_cells`: the flow, in cells, at the line where the cell leaves the
world.

**After:** `attacks` 0 and `attack_plant_cells` 0 on every seed. That half is
exact and does not move.

## I RETRACT "plants up on every seed". Re-measured, it is not true.

The tree moved under me — `main` landed 21 commits including `Cell::organism_
id` widening u16 → u32 — and this repo's own rule is to **re-measure the
baseline in the same session rather than compare against a remembered
number**. Re-run paired on the merged tree, one binary, the ablation switch
the only difference:

| seed | plants, loop live | plants, fixed | no-colony control | ants, live → fixed |
|---|---|---|---|---|
| 1 | 243 | **275** | 270 | 198 → 137 |
| 2 | 161 | **97** | 268 | 127 → 224 |
| 3 | 190 | 187 | 200 | 2 → 8 |

**Up on one seed, down hard on one, flat on one — median −3.** The pre-merge
run read 173 → 206, 134 → 191, 60 → 158, three for three, and **that was a
sample from a wide distribution on a tree that no longer exists.** Three seeds
is not a sweep either way.

**The mechanism claim is untouched by this** and is what the round rests on:
100% of swings plant-directed, and the jaw taking **77–93%** of every cell
removed from a living plant, reproduced on the merged tree (seed 1: 3,839 jaw
cells against 659 by mouth; seed 2: 5,226 against 412). The *outcome* claim is
the one that was oversold.

**And the two columns together say what is actually going on, which is more
useful than the number I withdrew.** Where the colony does not grow, the stand
recovers to the unhunted control — seed 1 ends at **275 against a no-ant 270**,
i.e. a colony that stays its size now costs the bed essentially nothing in
plant count. Where the colony explodes, grazing replaces the jaw — seed 2's
ants nearly double, 127 → 224, and the stand falls with them. **The fix removes
the pure loss completely; what happens to the forest next depends on what the
colony does with the energy it is no longer wasting.**

**That is the constant re-derivation this repo warns about, now with evidence
rather than as a worry.** The birth bar and the starvation balance were
calibrated against a colony paying a bill that has gone. **Lane D's file and
Lane D's question** — flagged, not taken, and the seed-2 row is the case to
tune against.

## On "worse than yesterday"

Still **UNCONFIRMED**, and not written off. The nearest thing to an
explanation anyone has: **`longant` is roughly an order of magnitude worse
than `ant` at this** — 1,147 plant cells at 20,000 frames here against §Z23's
58–86 at 24,000 on the two-colony `ant` bed. His bed is the longant one. That
is a difference in *bed*, not a change over time, so it does not establish an
amplification; it does explain why the harness beds looked mild.

## Counters added (`CreatureStats`), each a near/far pair

`attacks_at_plants` / `attack_plant_cells` — **0 for ever now**, which is what
makes them the repair's standing guard rather than dead weight.
`eaten_plant_cells` — the mouth's half, the denominator.
`alarm_attack` / `alarm_eat_animal` / `alarm_eat_plant` — where the alarm
plane's writes come from. **`alarm_eat_plant` deliberately still counts the
suppressed cries**: a repair can remove the picture and leave the mechanism.

All appended to `latecensus`'s `SUMMARY` as its own `SUMMARY z23` row —
`main`'s fields untouched, per the contested-line convention.

## A guard re-derived, not weakened

`a_lone_grazer_cannot_farm_a_moss_lawn_forever` went red. It compared a moss
lawn against a wall of `litter` in **joules**, and the wall arm is a ceiling
on *mouthfuls*, not joules — the two larders are different foods (`litter` is
`food_class: -1.0` against the shipped neutral gut). It read the right way
round only while the lawn arm carried this bug.

One binary, the switch the only difference:

| arm | intake | eats | attacks | alarm-bites |
|---|---|---|---|---|
| wall, plant-a-foe **on** | 684 J | 37 | 0 | 0 |
| wall, plant-a-foe **off** | 684 J | 37 | 0 | 0 |
| lawn, plant-a-foe **on** | 456 J | 9 | 1 | 20 |
| lawn, plant-a-foe **off** | **912 J** | **18** | 0 | 20 |

**The wall arm is byte-identical** — painted litter carries no organism id, so
nothing in it can raise an alarm or be struck, and that is what makes it the
control that says the move is the lawn's. The lawn arm doubled off **one**
attack and twenty silenced alarms: the grazing ant had been spending half its
feeding opportunities on the fight verb.

Asserted on mouthfuls now (18 against 37), and the bar was put back tight and
**watched go red**. `dead-ends.md` `creatures:083`.

## Open — the owner's, asked as a card

**Should eating another *creature* raise an alarm, or should alarm mean only
"I was attacked"?** Card `20260914T211725703Z-094f70`, board `lab`. Shipped
today as *a living animal being bitten, whichever verb did it*.

**It is inert on his bed either way**: `alarm_eat_animal` is **0** over three
40,000-frame runs, because a lone colony has no stranger to eat. It starts to
matter the moment two colonies share a box, which is now the default. So the
answer can wait without blocking anything — but it should not be settled by
implementation.

## The routing collision, and what I did about it

The coordinator's poke reached me at ~21:25Z, after this lane had already
built and pushed both rulings. **Lane D shipped ruling 2 in PR #436 at 19:47Z
— eleven minutes before the poke handing `creature.rs` to this lane fired.**
Both lanes implemented it, **identically in semantics** and differently in
shape.

**D's shape is the one that stands.** Its `is_animal_cell(world, cell)` takes
a `Cell` rather than coordinates and leaves the organism-id test to the
caller; both feeding sites bind `bitten` before gating. Adopted verbatim
(`fddd586d`), so where the two branches meet the overlap is **textual rather
than a conflict** in the function both lanes are in — and extended to the two
readers ruling 1 needs, which makes four readers of one definition.

**What I did NOT do is strip my half**, and the reason is state rather than
pride: **#436 is `mergeable_state: dirty` and unmerged.** Dropping ruling 2
here would have left `main` with neither ruling if #436 stalls. Taking D's
shape gets the same outcome — D's version wins on merge — with no window in
which the fix is nowhere.

**D's own struck claim is the one this lane was asked to settle, and it is
settled.** #436's body says it does not claim every remaining swing is
animal-directed, because `xcol` and `killedA` are both *kill* counters.
`attacks_at_plants` and `attack_plant_cells` are the split-by-victim-kind
counters that answer it: before ruling 1, **100% of swings and 100% of cells
on the played bed were plant-directed**. Plant-directed swings did not merely
survive #436 — on this bed they were all of them.

## §Z26 — I disagree with the framing, and censused it rather than arguing

D filed §Z26 off the same moss-lawn move this lane hit independently. **The two
measurements agree byte-for-byte** — lawn 456 → 912 J, litter wall 684 → 684 —
which is two lanes, two harness routes, one number.

**The reading differs.** §Z26 reads the doubled yield as *"the moss pump is
live"* and `#[ignore]`s the guard pending a per-cell grazing cooldown. Two
things say that is a diet-quality artifact rather than a pump:

- **The wall arm is a ceiling on mouthfuls, not on joules.** `litter` is
  `food_class: -1.0` against the shipped neutral gut and moss is not, so the
  comparison was 37 cheap mouthfuls against 18 expensive ones. On mouthfuls
  the lawn is bounded, comfortably: **18 against 37**, and 9 against 37 before
  the repair.
- **The lawn is not being mined.** Censused rather than inferred — standing
  moss cells owned by a live organism, same run: **20 → 26 with the defect
  live, 20 → 22 with it fixed.** The lawn is net *producing* in both arms. The
  fixed arm ends smaller because the ant eats twice as much of the regrowth,
  which is what a renewable niche is.

**So the guard is re-derived and left ACTIVE here rather than `#[ignore]`d** —
which happens to satisfy §Z26's own stated acceptance test (*"un-ignoring it
is the acceptance test"*), though not by the route it expected.

**§Z26 is qualified, not closed, and that is deliberate.** What would
establish a pump — a standing lawn that falls, or mouthfuls exceeding the
wall's — neither does. What this does **not** establish is the thing the
test's name asks for: that the lawn is bounded over an *unbounded* horizon.
One seed, one scene, 1.1 idle lifetimes, and 20 → 22 is a small number. **The
cooldown remedy should not be built on the joule evidence**; the long-horizon
census is the thing still missing. That is D's bug and D's call.

## Two instrument checks after the fix, and one of them is a hand-off

**`conflict_arena control=selftest` — PASS, and it is the best specificity /
sensitivity pair in the round.** One binary, the switch the only difference:

| | plantbites | cells | eats | contests | fights | x-kills |
|---|---|---|---|---|---|---|
| strangers, plant-a-foe **on** | 54 | 32 | 505 | 33 | 18 | 9 |
| strangers, plant-a-foe **off** | **0** | **0** | **695** | 33 | 18 | 9 |
| one family, plant-a-foe **on** | 43 | 16 | 26 | 0 | 0 | 0 |
| one family, plant-a-foe **off** | **0** | **0** | **127** | 0 | 0 | 0 |

**Animal-against-animal fighting is byte-identical** — contests, fights,
displays, escalation and cross-colony kills all unmoved. That is the
sensitivity arm for ruling 1: the fight verb still works, it just no longer
finds a tree. And the **one family** arm is the owner's own control in
miniature: a bed with **no strangers in it at all** was taking 43 plant bites
and 16 cells. Feeding rose on both arms (26 → 127 and 505 → 695).

**`rivalry control=selftest` FAILS — and it is NOT this lane.** Byte-identical
failure with the switch on and off (`shipped` arm: `cross 1 attacks 1` against
expectations of 0; `wire-only`: `attacks 2` against 0), so it cannot be the
plant rule. The cause is visible on the line: the `shipped` arm reads
**`gap 1.744`, `between 100.00%`** — the two colonies are already mutual
strangers on the shipped default, because #423 authored a live `scent_spread`.
The selftest's expectations were written when the shipped default made
everyone kin, so the arm named `shipped` is asserting about a default that no
longer exists.

**This is Lane D's file and Lane D's question** — it is the same shape as the
brief's own warning that `spread=0` was not an off arm, and it means any
`rivalry` baseline quoted from before #423 is a measurement of a different
default. Flagged, not taken: `examples/rivalry.rs` is not this lane's, and
fixing an expectation is a decision about what the arm is for.

## Ruled out before starting, per the brief — nothing re-derived

`structural.rs` / `load.rs` / `rigid.rs` (zero commits in three days), plant
species files, rivalry (felled 161 vs 165, `plantkill` 0 both arms).

## Gates

clippy clean · `cargo test --lib` 1812 passed (post-merge) / 0 failed / 85 ignored ·
`--test worldgen --test determinism` 47 passed · `ascii` 31 scenes, 0 skipped
· `docscheck` clean.
