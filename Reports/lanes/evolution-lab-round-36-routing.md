# Round 36 — routing, after the regression took the lead

*Coordinator `session_01Evmt6MKMGFbk4zA5rC7B3y`, 2026-09-14 19:58Z. **This file
exists because there is no delivery signal for a poke.** Every lane was sent the
part of this that concerns it; a poke that silently failed leaves this standing.
If you are a round-36 lane, read the row for your letter and believe this file
over your spawn brief where they differ — it is newer.*

## The change

**A live regression took the lead, ahead of everything in
[`../evolution-lab-round-36-brief-2026-09-14.md`](../evolution-lab-round-36-brief-2026-09-14.md).**
The owner is losing plants in his lab. He isolated it himself: **a bed with no
creatures keeps its plants; the same bed with creatures loses them**, hunks
falling off first and total collapse later.

**The loop, established — do not re-derive it.** An ant grazes a plant cell
(legitimate foraging). Because the victim is an **organism**, the eat path calls
`cry_alarm` at the bite site — both call sites in the swallow branch of
`src/sim/creature.rs`, ~7177 and ~7217. **Alarm is the only wired route to
attacking**, and all nine armed species author exactly `(Alarm, Attack, 2.0)`.
`nearest_foe` targets *"any living non-kin organism, and not only an animal"* —
its own comment — so it finds **the plant being grazed**. That bite has **no
diet gate**, so it takes wood.

**Wood is not food and never was**: `wood`/`rootwood`/`grassroot` carry no
`food_energy` and no `food_class`, so `food_value` and `diet_yield` are both 0.
**Only the attack path can remove a wood cell, and it feeds nobody — pure loss,
billed to the jaw.**

**The owner's positive control beats any harness run so far:** he sees alarm
signals in a **single-colony box containing only trees**. Nothing there can
attack anything, so the alarm is coming from eating.

## Two owner rulings, 2026-09-14

1. **Creatures do not attack plants. No exception.** He asked to be argued with
   and the argument was not made: the code's defence — an animal cornered by
   something it cannot digest must still be able to hit it — is sound in
   principle and **has no instance**, because nothing in this engine lets a
   plant harm an animal. **The condition that would reopen it is a plant that
   can damage an animal**, and it belongs in `dead-ends.md` with the entry, not
   in a code comment.
2. **Eating a plant does not raise an alarm.** **Open, and his to answer:**
   should eating another *creature* raise one, or should alarm mean only *"I was
   attacked"*? The coordinator's recommendation, **not a ruling**: fire when a
   **living animal** is bitten, whichever verb did it — from the victim's side
   being eaten *is* being attacked — which excludes plants and corpses and keeps
   the recruit signal the colony needs. **Put it to him before building it; do
   not settle it by implementation.**

## Who owns what now

| lane | session | branch | owns |
|---|---|---|---|
| **E** (leads) | `session_01N4VRVeKGH5g2GWcJMWHec7` | `claude/evolution-lab-forest-eaten` | **`src/sim/creature.rs`** for this fix — the `cry_alarm` / `nearest_foe` / attack-target path |
| A | `session_01Rh2VKAF8sv8gHzX7SM7sgo` | `claude/evolution-lab-food-readable` | `src/render.rs`, `src/food_road.rs`, `src/lab/ui.rs`, `examples/foodroad.rs`, `examples/labui.rs`, `examples/colonybooks.rs` |
| B | `session_01FraVGU28Uh85SdVh5ophdR` | `claude/evolution-lab-ant-cost-census` | `examples/antcost.rs`, new `examples/*`; **reads** `src/sim/*`, writes none of it |
| C | `session_01M86KmRgxhQos7HzYXcpEh6` | `claude/evolution-lab-pheromones` | `src/sim/pheromone.rs`, `src/sim/brain.rs`, new `examples/*` |
| D | `session_01DWkhUx9DWMZaVWaXUVLyQC` | `claude/evolution-lab-rivalry-economy` | `assets/species/*.ron` and the economy constants — **no longer the creature.rs alarm/foe path** |

**§Z23 is no longer Lane D's footnote.** It was never a contained follow-up job.
**C and D unchanged otherwise:** if C needs a species weight moved, it goes
through D.

## What every lane must do about its own numbers

**Record the head SHA with every measurement, and say which side of E's fix it
sits on.** Both rulings change what the creature line does, so a number taken
against today's engine may not be comparable to one taken after E lands. Cheap
now, unrecoverable later.

## Sizing, which is NOT yet established

The coordinator has the mechanism and **not** proof it is the dominant term. A
rivalry A/B moved **plants felled 161 → 165** with **`plantkill` 0 in both
arms** — so do not assume the attack path is all of it. What *did* move is
**plants eaten, 5,333 → 9,690 joules**: that is the **mouth**, not the jaw, and
it is adjacent to Lane D's economy work rather than E's. **If the attack path
turns out to be a small share, the rest is grazing pressure and that is a
different finding** — say so rather than declaring victory on the fix.

**Ruled out already, so nobody spends a night on it:** `structural.rs`,
`load.rs` and `rigid.rs` have **zero commits in three days**; no plant species
file changed in the window; the default one-colony bed is **byte-identical**
across yesterday-evening vs now (58 plants, 4,292 plant cells, dying back 1),
which also makes `labstats` a valid two-engine instrument; the two-colony bed
moved 44 → 40 plants and 3,390 → 3,341 biomass, inside that bed's spread, with
rivalry live (6 and 2 cross-colony kills against zero).

**"Worse than yesterday" is UNCONFIRMED rather than false** — it did not
reproduce on any harness bed. His bed has something the harness beds do not, and
the honest position is that **the loop above is old and something may have
amplified it**.
