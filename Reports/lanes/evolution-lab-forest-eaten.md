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

**After:** `attacks` 0 and `attack_plant_cells` 0 on every seed. Plants
standing **173 → 206, 134 → 191, 60 → 158** against a no-colony control of
264 / 202 / 202.

**Reported with its cost, not as a clean win.** The colony grew too — ants
66 → 123, 81 → 146, 156 → 190; births 83 → 152 — because the jaw work it was
billed for bought nothing, and grazing rose with it (`eaten_plant_cells`
112 → 477 on seed 1). Plants still went up on every seed. **Whether a bigger,
better-fed colony is what he wants is a question for him**, and it is the
constant-re-derivation this repo warns about: the birth bar and the starvation
balance were calibrated against a colony paying a bill that has now gone.
**That is Lane D's file and Lane D's question** — flagged, not taken.

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

## Ruled out before starting, per the brief — nothing re-derived

`structural.rs` / `load.rs` / `rigid.rs` (zero commits in three days), plant
species files, rivalry (felled 161 vs 165, `plantkill` 0 both arms).

## Gates

clippy clean · `cargo test --lib` 1807 passed / 0 failed / 85 ignored ·
`--test worldgen --test determinism` 47 passed · `ascii` 31 scenes, 0 skipped
· `docscheck` clean.
