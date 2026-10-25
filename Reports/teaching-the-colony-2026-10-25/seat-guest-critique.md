# Critique: guest.md (the guest seat)

Checked at `main` 1c8b7a3b; cites are `src/sim/`.

1. **BLOCKING: the mind judges by colony id, not scent.** The pitch says "kin is 'scent within my tolerance'". Step 0 says "Print the shipped pool's weights on the Feed verb".
   - **Wrong:** a mind ant's NESTMATE/STRANGER feature is `o.colony == here.colony` (`creature/options.rs:1059-1061`).
   - **Wrong:** Feed is offered only when `o.colony == here.colony`. Any other animal gets Attack instead (`options.rs:877-907`).
   - **So:** a guest from another colony is never a Feed option, whatever the weights or its disguise. No lab ant's mind can see the scent disguise at all. Scent reaches only body rules: nursing, the nest blend and dig company.
   - **Fix:** say this up front. Re-aim step 0 at the colony gate. Make the owner choose between two options: give the guest the host's colony id (the disguise then means nothing to the mind), or make the mind's kin sense read scent (the pool must be retrained).

2. **BLOCKING: the species gate makes "This one is family: Today? Yes" false.**
   - `is_living_kin_id` requires the same species unless `gut.crosses_kinds` is set (`creature.rs:11819`).
   - That switch is off for every shipped kind (`organism.rs:4282-4293`). The test `a_beetle_is_never_family_unless_its_kind_crosses_kinds` guards it (`creature.rs:49601`).
   - The doc's own fix ("the nurse must judge") makes this worse. The ant's gut has the switch off, so the guest is never fed.
   - The stranger alarm also ignores other kinds by default (`StrangerAlarm::ON`, `species: false`, `creature.rs:26837`).
   - **Fix:** decision 2 is a precondition for Beg, Rub and the whole family ladder, not an open question. Mark the family row "new: needs `kin_crosses_kinds` on ants, or a guest-only exception".

3. **BLOCKING: today's lab ants do not react to anything that touches them.**
   - Mind ants return into `mind::tick` (`creature.rs:8022`). That skips `act`, which holds the stranger-touch alarm (`creature.rs:20393`).
   - The display deposit the doc cites (`creature.rs:20661`) belongs to the old brain's Attack path.
   - Every trained pool bans Attack (`options.rs:395`). A mind's Eat takes only from its own crop (`options.rs:916`).
   - **So these are wrong as engine claims:** the 0:20 jerk [E], the 2:00 guard bite, and step 1's positive control ("authored far, it must be bitten or eaten"). That control will read zero.
   - **And these cannot happen without new aggression:** the scenes "forager bitten at her own door" and "civil split … turns on its drifted sisters". Note also that an ant at its own nest always rejoins it (`creature.rs:11389-11401`).
   - **Fix:** drop the [E] marks from the walkthrough. Change step 1's control to "ignored either way". Move every bite to step 4. Say that step 4 must also cover nestmates for those two scenes.

4. **IMPROVE: Soak has the same wrong-side gate the doc found for nursing.**
   - For a foreign nest, `blend_with_nest` judges by the *visitor's own* tolerance. A refusal blocks both directions (`creature.rs:11397-11406`). So the guest decides whether it may soak, and the gate is a binary threshold, not graded.
   - Rub cites `:11349`, which is the nest blend. The body-to-body blend is `blend_with_kin` (`:11459`), and it runs only on a colony-gated Feed (`creature/mind.rs:3020`).
   - That blend is symmetric. Each rub also pulls the *worker* toward the guest's odour at beta 0.1. A rubbed nurse becomes a little foreign to her sisters.
   - **Fix:** the host side should judge. State the threshold, and show the symmetric drift on the rubbed ant.

5. **IMPROVE: "Beg harder" and "Sing" have nothing to hook into.**
   - `nurse()` has no step where a nurse chooses whom to feed. Each larva, on its own tick, takes from its richest kin neighbour, capped at its own need (`brood.rs:1051-1120`). Presented hunger raises only that cap.
   - "Fed ahead of real larvae", "fed twice as often" and "nurses serve the singer first" all need a new rule where the nurse shares out her food. That is not "small".
   - "Register the guest as a brood item" is not small either. Brood is a powder-like stage cell, not an animal cell (`creature.rs:26906`).
   - "Go limp → carried inward" also needs brood carry to lift a multi-cell body. That carry is off and only re-piles brood; it never fetches from a door (`brood.rs:448-456`).

6. **IMPROVE: "the bloodline channel is real here, not hoped for" overclaims.**
   - One guest drains a few nurses in a colony of about 400. Tolerance drifts only 0.05 a birth (`creature.rs:11653`, `ant.ron:996`). The mind's Feed ignores tolerance anyway.
   - The "every ant is a person" and "enforcers" fantasy needs ants to differ. Spread grows roughly as 0.05·√generations around a radius of 1.0, so early on the ants are nearly uniform. Nobody "you have fooled before" would be visible.
   - **Fix:** relabel it a hypothesis, and run step 6 before the pitch leans on it. Print the per-ant tolerance spread from an existing run.

7. **IMPROVE: the pseudogyne claim is outdated.**
   - The doc says the leading explanation is neglected queen larvae [S, UK Beetles].
   - AntWiki now says *Lomechusa* is not the cause. The deformed forms are secretergates from labial gland disease. Tulloch found infested colonies with no pseudogynes. (https://www.antwiki.org/wiki/Labial_Gland_Disease ; https://antwiki.org/wiki/images/e/e5/Tulloch_1930_Pogonomyrmex.pdf)
   - **Fix:** mark it as a historic hypothesis, or cut the "Deformed young" scene.

8. **IMPROVE: "begging yields about six times more than eating brood" misreads the source.**
   - The roughly 6x is how many caterpillars one nest can support: cuckoo against predatory *Maculinea* (Thomas & Elmes 1998). It is not one animal's yield.
   - Thomas & Wardlaw 1992 does confirm the 354 workers (https://link.springer.com/article/10.1007/BF00317247).
   - **Fix:** score on how many guests the host can carry, which the "young × host standing" score already does.

9. **IMPROVE: two game references are wrong.**
   - "Rain World-style continuation across lives": Rain World has no playing-as-offspring. You respawn at a shelter and lose karma (https://en.wikipedia.org/wiki/Rain_World). Rogue Legacy or Crusader Kings is the right reference.
   - "Plague Inc. works because killing the host kills you" [K]: Plague Inc's goal *is* extinction. Its restraint is about the timing of spread. That is a weak model for "host still standing".

10. **IMPROVE: internal contradictions.**
    - The pitch says "No other seat gets a graded … reaction this cheaply". Then: "needs a new creature, a new food liquid and a reaction ladder before the first minute is playable". After items 1-3, the ladder is entirely hand-made body rules.
    - Hide: "your suspicion halo fades" implies a suspicion state, against "no meter … the ring is the only gauge".
    - Cry is marked "now (alarm deposit)". A guest emitting alarm at will is new; today alarm is written by a bite (`cry_alarm`) or a display.

11. **IMPROVE: the herd loop is mostly waiting, on an untested signal.**
    - Its only teaching verb, walking the herd, pays off "a minute or two later, if it follows at all". That depends on trail B, which pulled nobody in slice 0.
    - "Idle ants find a job" around the herd has no mechanism.
    - **Fix:** in the faked slice, script the tenders' pull *and* add an honest arm, today's engine with the herd moved, so the owner sees the gap. Run the worker-level trace of trail B before step 5.

12. **IMPROVE: two line numbers are off.** Tolerance drift is `creature.rs:11653`, not `ant.ron:991`. The jump is authored in `hopper.ron`, but it is `ant.ron`'s shipped jump.

**Verdict.** The beggar's minute-to-minute loop is good, and so is the gardener-as-symbiont graft. The faked slice is the right test. But the doc's central engine premise is wrong: today's lab ants judge guests by colony id and species, never by scent, and they do not react to strangers at all. So "the hardest part already exists" must become "the hardest part is new body rules plus an owner switch".
