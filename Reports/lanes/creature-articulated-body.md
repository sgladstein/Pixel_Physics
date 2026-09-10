# Lane note: articulated creature bodies

Branch `claude/creature-evolution-engine-67lhjp`. Report of record:
[`Reports/creature-articulated-body-2026-09-09.md`](../creature-articulated-body-2026-09-09.md).
Written for the lab coordinator; everything substantive is in the report's §7.

## Standing: NOT landable

**Clippy in release is green. `ascii` is red and the bodies do not walk**, so
there is no pull request and there is nothing to post to the review queue.
`ascii` fails with *"the colony has gone sessile: 0 round trips of 8+ cells"*
against an expected 23, at 172 moves and 9,586 blocked.

Measured, `creature_scale mode=walk`, one seed, with a six-cell plain chain as
the paired control so **length is controlled for**:

| body | cells | `flat` | `rolling` |
|---|---|---|---|
| `ant_long`, `Chain(6)` | 6 | 2.5% | 12.4% |
| ant, articulated | 7 | 43.9% | 96.8% |
| hopper, articulated | 8 | 74.6% | 91.9% |

`filmstrip scene=colony` founds **4 ants of 52 asked** against 28 viable
sites, so placement fails as well as movement. §7c lists three hypotheses
that moved nothing, so they are not retried; §7d names the ablation that
should come next instead of a fourth guess.

## File ownership

My footprint against `origin/main`, complete. Nothing here is in the ecology
round's list except the three lines it already accounted for:

| file | lines |
|---|---|
| `src/sim/creature.rs` | 1,353 |
| `src/sim/organism.rs` | 377 |
| `assets/species/ant.ron`, `hopper.ron` | 185, 144 |
| `examples/ascii.rs` | 74 |
| `src/render.rs` | 25 |
| **`assets/materials/corpse.ron`** | 15 |
| `src/lab/mod.rs` | 13 |
| `src/lab/stats.rs` | 12 |
| `assets/materials/ant.ron`, `hopper.ron` | 10, 5 |
| `src/sim/world.rs` | 5 |
| `examples/fate_viability.rs` | 3 |

**One file to flag: `assets/materials/corpse.ron`.** Not on the ecology
round's list, but it is a shared fallback and it was carrying a live
accounting bug. Its `food_energy` stayed at the whole-cell 480 while the
articulated bodies re-priced flesh per cell, and `food_value` falls back to
the material whenever `aux` is 0 — which `fire.rs`'s generic burnout always
writes. A burnt seven-cell ant was worth 3,360 against the 960 its flesh cost
to build: **3.5x energy creation inside a ledger `EnergyLedger` asserts
closed**. Now 120, the smallest real stamp, which is what its own comment had
already been rewritten to claim. If the ecology round adds a species whose
material burns into `corpse`, that number is the one to keep honest.

## Answers to the ingestion question

Asked for one site each; there are **two** ingestion sites, not one, and the
clearing and the credit are in different functions. That matters for a
one-call hook.

- **A mouthful taken into the crop** — `act` (`creature.rs:5011`), the cell is
  cleared at **`creature.rs:5329`** (`world.set(fxx, fyy, Cell::EMPTY)`). The
  joules are *not* credited there: the crop digests later in `creature_tick`
  (`creature.rs:2903`), which books `harvested_plant` / `harvested_corpse` at
  **`creature.rs:3358`–`3362`**. So the cell and its material are in hand at
  5329 and gone by the time the energy lands — **5329 is where an eaten fruit
  can still leave its seed**, and the only place that knows what was eaten.
- **The second path, easy to miss** — brood provisioning, which clears a cell
  and credits immediately in the same block: **`creature.rs:1489`** clears,
  **`:1493`–`1495`** credits. A hook only at 5329 will not see food taken this
  way.
- **Where a carried load ends its trip** — `act` again: it is **dropped back
  into the world as a whole cell**, `world.set(dx, dy, unit.into_cell(world))`
  at **`creature.rs:5470`**, with `drops` at `:5475` and `deliveries` at
  `:5477` when `at_nest`. Not eaten and not converted at the nest — eating is
  the separate digestion path above. So a delivered windfall is a real cell
  standing on the ground again, which is the natural place for a seed to
  survive a trip.
