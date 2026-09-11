# Check: `creatures:039` — the lateral pheromone sensors, and the flier that arrived

**Verdict: EXPIRED, condition met, no re-test recorded.** Verified 2026-09-11.

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

So the author's own precondition arrived — with `flitter`, and before that
`hopper` — and the sensors it was written for have never been rewired.

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

## The check to run next

Cheap, and it has both arms:

```
cargo build --release --examples          # set -o pipefail; read ${PIPESTATUS[0]}
./target/release/examples/creature_probe species=flitter      # baseline
```

**Alive** if `PheroALateral`/`PheroBLateral` read non-zero for a flying creature
over a trail cell — the 0.000 was geometric and flight removes the geometry.
**Still dead** if they read 0.000 anyway, which would mean the offsets miss for a
reason the entry did not identify and the along-heading scalar remains the only
route.

Run the positive control in the same pass: a cell with a known non-zero A, so a
zero cannot be read as "the sensor works and there was nothing there".
