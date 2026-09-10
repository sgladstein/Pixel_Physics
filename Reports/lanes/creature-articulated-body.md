# Lane note: articulated creature bodies

Branch `claude/creature-evolution-engine-67lhjp`. Report of record:
[`Reports/creature-articulated-body-2026-09-09.md`](../creature-articulated-body-2026-09-09.md).
Written for the lab coordinator; everything substantive is in the report's §7.

## Standing: lateral rule built and measured; PR open, awaiting review/merge

**§7f's lateral rule is built, on `claude/creature-lateral-tuck-r26` (cut from
this branch), measured against its own table, and merged with `main`.** §7f(7)
carries the full write-up: the ant's blocked rate falls 43.9%/96.8% ->
1.9%/22.1% on flat/rolling (chain control 2.5%/12.4%), the hopper's
74.6%/91.9% -> 6.8%/15.4% (beating its own laterals-off ablation on both
presets), `ascii`'s colony-forage round trips are back at 32 (was 0, bar 6),
and `cargo test --lib` is 1,537 passed / 0 failed / 65 ignored. Two
pre-existing scenes needed their premise re-established rather than the
mechanism touched (`CLAUDE.md`'s "a scene that contradicts the code" —
same shape as the §9 guard below), and one further pre-existing, unrelated
`ascii` failure was uncovered and left for whoever owns the body-shape line
next (§7f(7) has it). `filmstrip scene=colony` founding is unchanged at 4/52
— that is a placement-time check the movement rule never reaches, not a
regression.

Superseded by that build: the paragraph below described the state before
`claude/creature-lateral-tuck-r26` existed and is kept for the record.

**§7f of the report is a complete build brief for the lateral rule** — the
rule itself, the sites to change with line numbers and what must not change,
the tests including the two that do not exist yet, the measurement to re-run,
and the card to post. A build lane picks it up from this branch. Handed over
rather than built here because a 2-3 hour rule build is a build, and the
owner's standing split is design on Opus and builds on Sonnet.

**The cause is settled** (§7e, coordinator's ablation, verified on this
branch): the laterals are the whole of it and the segmented spine is
exonerated — a `Segmented` body with zero laterals is byte-identical to
`Chain(6)` on every counter, both presets. The design in §1–§6 stands. Only
the movement rule is wrong.

**The red guard in §9 stays red by decision**, and §7f(6) says why: it is not
caused by this body plan, it must not be fixed by pinning `ant.ron` back, and
it must not be fixed by moving the bar.

## Before the spec was written: why it is not landable

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
that moved nothing, so they are not retried; §7d named the ablation, and
§7e (branch `claude/creature-bodies-ablation-r26`, stacked on this one) ran
it: **laterals are the cause, not the segmented spine.**
`PIXEL_PHYSICS_BODY_LATERALS=0` collapses the ant to 2.0%/14.8%
(flat/rolling), *at or below* the `Chain(6)` control, and the hopper to
5.5%/40.7% — most of the way down from 74.6%/91.9%. A second ablation
(`Segmented`, 6 spine cells, 0 laterals, vs `ant_long`'s own `Chain(6)`) came
back **byte-identical** on every counter, on both presets, which exonerates
the `Segmented` movement code path outright: it costs nothing beyond a
`Chain` at matched length. What is left is a hopper-specific residual on
`rolling` (40.7% against a length-matched ~13-14%) that is not the body plan
and is not explained by spine length or by the hop verb's own counters
either — narrowed, not fixed; no code changed on the shipped bodies. Still
**NOT landable** as shipped (laterals stay on by default), but the next fix
now has a target: the lateral placement/collision rule, not the spine.

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
