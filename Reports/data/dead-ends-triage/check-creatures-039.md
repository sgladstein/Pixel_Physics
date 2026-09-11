# Check: `creatures:039` — the lateral pheromone sensors, and the flier that arrived

**Verdict: DEMOTED. The condition is met in the letter and not in the substance.**
Verified 2026-09-11, **corrected the same day** — the first version of this file
called it "EXPIRED, condition met" and that conclusion was wrong. What follows
keeps the original checks, which stand, and adds the one that overturns them.

## The entry

`dead-ends.md:1001`, on `src/sim/brain.rs BrainInput::PheroAAlong`:

> The Jones/Physarum lateral sensor pair (`PheroALateral`/`PheroBLateral` at full
> sensor offset) does not work for surface-walking creatures in a side-view
> world: both sensors land in open air, measured with `examples/creature_probe.rs`
> at exactly **0.000** over a cell holding A=27. Replaced by the along-heading
> gradient scalar (run-and-tumble chemotaxis). The lateral inputs are kept but
> unwired.
>
> *Re-test when:* **Correct for anything moving in open space (a flier, a
> swimmer) — rewire and re-test the laterals then.** Removing the slots is
> forbidden by the positional genome law.

The rejection is correct and was correctly scoped: the null was produced by the
*world* (a walker's lateral offsets are in air), not by the mechanism, and the
author said exactly what would reopen it.

## What is true now

| claim | check | result |
|---|---|---|
| a flier exists | `assets/species/flitter.ron` | **exists**, and is airborne — 11 `Impulse` references, including `(Bias, Impulse, 2.0)` and `(BloomNear, Impulse, 1.0)` |
| the slots survive | `src/sim/brain.rs:515,517` | `PheroALateral = 2`, `PheroBLateral = 4` still present, as the positional genome law requires |
| the laterals are still unwired | every `.ron` under `assets/species/` | **no species carries a lateral weight.** Ten files match the names and **every match is a comment** — `// (PheroALateral, Turn, ...) -- now via hidden units 0 and 1` |
| flitter wires them | `assets/species/flitter.ron` | **zero** lateral or along-heading pheromone wires of any kind |

## The check that overturns it

`flitter` is a flier with **no pheromone economy at all**. Counting real wires
(not comments) in the species files: **`flitter` 0, `ant` 4.** It neither lays a
trail nor follows one, and its own file records the removal as deliberate —
*"`emit_cost_in_moves` is gone, not merely zeroed"*, and *"there is nothing
upstream left to drive `Move`/`EmitA` from"*.

So wiring a lateral **pheromone** sensor onto the one species that moves in open
space would sense a plane that species never writes to and never reads. The
entry asked for *"anything moving in open space"* as a proxy for *a creature
whose lateral offsets are not in dead air*; `flitter` satisfies the proxy and
not the thing it was standing for.

**This is the register's own "a scene that contradicts the code will look like a
bug in the code" gotcha, one level up**: the precondition was checked against
the *existence* of a flier rather than against whether the flier contains the
situation under test. Two of the three original checks were fine and the third —
the one that would have caught this — was never asked.

## What still stands

The sensors really are computed and really are unread: `creature.rs:3691` fills
`inputs[lateral_slot] = r - l` every tick, and no species carries a genome weight
on either slot, so the value is produced and discarded. The slots survive. That
half of the entry is accurate and unchanged.

## Why this one matters beyond itself

It is the register's own exemplar shape, arriving twice. The `hopper` case is
already recorded elsewhere: the jump verb worked from 2026-08-29 with no species
wired to use it, `forage_probe` read **0 launches**, and one `Bias -> Impulse`
wire took it to **275**. This is the same failure one layer up — a mechanism that
was correctly parked against a named condition, the condition quietly being met
by work on another line, and nothing connecting the two.

Nothing in the repo closes that loop. `dead-ends.md` is grepped by *area* before
work starts, so a creature-line session adding a flier has no reason to read a
brain-input entry, and the entry has no way to notice a new species file.

## What would actually reopen it

Not a flier — **a trail-using creature whose lateral offsets are not in dead
air.** Either a flier wired into the pheromone economy, or the swimmer the entry
also names. Until one exists the entry holds, and the right label is `DEAD` on
its stated condition rather than `EXPIRED`.

If such a species does appear, the check is cheap and has both arms: run
`creature_probe` on it over a cell with a known non-zero A. **Alive** if
`PheroALateral`/`PheroBLateral` read non-zero — the 0.000 was geometric and
open-space motion removes the geometry. **Still dead** if they read 0.000 anyway,
meaning the offsets miss for a reason the entry never identified. The positive
control is the known-non-zero cell itself, so a zero can never be read as "the
sensor works and there was nothing there".
