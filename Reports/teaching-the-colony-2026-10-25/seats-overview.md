# Four more seats: guest, colony mind, enemy, lone scout (overview)

*2026-10-25. Game design research only: no code and no runs. The owner asked
for full design documents for the four further seats offered after the
[teaching plan](../teaching-the-colony-2026-10-25.md): the guest who lives in
the nest, the colony mind, the enemy, and the lone scout. Each was written by an
independent researcher from [`engine-brief.md`](engine-brief.md), the plan, and
the first sketch in [`perspectives.md`](perspectives.md); each was then checked
against the code and sources by a separate reviewer, and revised. The owner's
copy is a Claude Doc with one tab per seat.*

| Seat | Design | Review (must-fix / improve) |
|---|---|---|
| Guest | [seat-guest.md](seat-guest.md) | [3 / 9](seat-guest-critique.md) |
| Colony mind | [seat-colony-mind.md](seat-colony-mind.md) | [5 / 8](seat-colony-mind-critique.md) |
| Enemy | [seat-enemy.md](seat-enemy.md) | [4 / 9](seat-enemy-critique.md) |
| Lone scout | [seat-scout.md](seat-scout.md) | [5 / 7](seat-scout-critique.md) |

Every revised design ends its "Risks and open questions" with *What the review
changed*: each item fixed, or disputed and why.

## At a glance

No new seat beats the queen as the main game, but three of the four make it
better as a stage, a season or a graft. Every seat had 3 to 5 claims that were
wrong on today's engine; the designs are the corrected versions.

| Seat | Who you are | Teaches through | The verb that sells it | Works on today's ants | Best role |
|---|---|---|---|---|---|
| Queen | The founding mother | Who is born, what the young are fed, where brood lies | Choosing which line lays | Yes: selection by who lays acts on fixed weights | Main seat (teaching plan) |
| Keeper | The gardener gnome outside | The place: food, plants, water, light, carrying ants | Planting and baiting a garden | Little: a hand-laid trail pulled no workers | The lab's hand tools |
| Guest | A beetle living inside the nest | How the colony treats you: ignore, inspect, feed, carry, kill | Begging in the brood pile | No: mind ants sort strangers by colony number, not scent | A graft: the keeper as symbiont |
| Colony mind | The colony's drives, no body | Which of the colony's own signals win | Swelling the news a full forager brings home | Barely: only the alarm verbs | Late stage of queen, scout, mind |
| Enemy | Antlion, centipede or rival queen | Who survives you; alarm and corpses in the place | A strike that leaves a corpse and an alarm bloom | Partly: bites and alarm exist; no steerable predator | Seasons in the queen game; a sparring mode |
| Lone scout | One ordinary worker | Ant to ant: taste shared, a sister led by touch | A tandem run that can break and still help | No: the shipped pool zeroes the last-food memory | Middle stage of the arc, if leading can spread |

## How the seats combine

- **The life-cycle arc: queen, then scout, then colony mind.** Found the colony
  and choose who lays; continue as one of her workers and persuade by touch;
  late, become the colony's drives. The scout stage needs a led ant to lead
  others; the mind stage needs a way to send never-foraged ants without the
  starvation the recruit switch caused.
- **Switch sides: the enemy as seasons.** Score is the price of an ant: loads
  carried home per ant you eat, so a colony that hides scores badly. Late game,
  daughters of the toughest colonies refound the fallen ones.
- **The keeper as symbiont.** Give the gnome the guest's scent model; the
  colony's attitude to him becomes graded with no trust meter.
- **What does not combine well:** the colony mind alone (no body, no stake),
  and the parasite guest in the main colony (it feeds on it by design).

## Shared engine work

| Shared piece | Seats | State today |
|---|---|---|
| A steerable body that is not an ant | Guest, enemy | `sim::steer` drives only an ant |
| The memory layer (plan §7) | Scout, keeper, colony mind habits | Not built; shipped pool zeroes last-food memory |
| Sending never-foraged ants without starving them | Colony mind, scout | `PIXEL_PHYSICS_RECRUIT` off: 442 starved vs 298 and 323 |
| Reactions that read the scent model via body rules | Guest, keeper as symbiont, enemy raids | Scent and tolerance genes exist; minds judge by colony number |
| Choosing who lays | Queen, colony mind | Laying is each ant's own gene |

## Corrections the reviews found that affect other work

- **Trained lab ants can bite.** Every member of `assets/minds/std_pool.txt`
  has the Attack switch (slot 8) on; the comment at `creature/options.rs:395`
  saying trained pools hold Attack banned is stale (enemy seat, verified
  against `mind.rs:164`, `mind.rs:2575-2604`, `src/lab/training.rs:94-95`).
- **The link price has not yet made training shed links**
  (`creature.rs:11596-11613`: the training bed never feels the bill). The
  plan's §2 engine table is corrected in the same change.
- **Mind ants judge strangers by colony number, not scent**
  (`creature/options.rs:1059-1061`), and skip the hand-built `act()` with its
  stranger alarm (`creature.rs:8022`).
