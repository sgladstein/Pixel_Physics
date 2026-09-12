# Lane P — room per ant, and the premise it was built on

*Round thirty, brief 3 of the late-game programme
([`../evolution-lab-late-game-design-2026-09-12.md`](../evolution-lab-late-game-design-2026-09-12.md)).
Built 2026-09-12. **The mechanism works and the diagnosis behind it does
not.** Read §2 before proposing anything about the dig gate again.*

## 1. What landed

At the nest, `BrainInput::Crowding` carries **room** rather than density: the
roofed void the nest patch holds over the ants in it, as
`target / (target + room)` so more room reads lower
(`World::NestRoom::occupancy`). Away from the nest it is the same 5x5 count it
always was, which is what `ant.ron`'s `(Crowding, Move, -0.3)` depends on.
**`ant.ron`'s wiring is unchanged** — units 5 and 6 still read the same slot at
the same weights; only what the slot means at the nest changed.

`World::step_nest_room` takes the census in `begin_step`, once per 256 frames,
on `step_nest_scents`' own idiom. Two dials on the lab's ANTS page and as env
switches: `PIXEL_PHYSICS_LAB_ROOM`, `PIXEL_PHYSICS_LAB_ROOM_TARGET`.

## 2. The premise is false on this bed — do not re-derive it

The brief, and `dead-ends.md`'s `(Crowding, Dig, 0.6)` entry under it, rest on
one claim: *the old input is pinned at its ceiling at the nest, so nothing the
colony digs can lower it* (median 1.000, p90 and max 1.000). **That figure was
measured on a different scene and it does not describe `played_bed`.**

`CreatureStats::at_nest_crowding` buckets what the gate actually read,
weighted by ant-ticks rather than by census stops. **Pooled over twelve seeds,
gate off, 300,000 frames each, n = 687,349 at-nest ticks:**

| bucket | 0.0–0.1 | 0.1–0.2 | 0.2–0.3 | 0.3–0.4 | 0.4–0.5 | 0.5–0.6 | 0.6–0.7 | 0.7–0.8 | 0.8–0.9 | 0.9–1.0 |
|---|---|---|---|---|---|---|---|---|---|---|
| ticks | **170,185** | 71,151 | 101,864 | 53,364 | 0 | 50,631 | 35,449 | 30,921 | 34,294 | 139,490 |

**Median bucket 0.3–0.4, a quarter of every read in the bottom tenth, a fifth
at the ceiling.** An ant at its own door is very often standing alone. The old
gate was never stuck, so there was nothing for a desaturated input to release.

## 2b. And the outcome is a coin flip — 12 seeds, paired

`latecensus scenario=played_bed frames=300000`, `RAYON_NUM_THREADS=1`, each
seed run twice from the same start with only `PIXEL_PHYSICS_LAB_ROOM` changed:

| | median | min | max | room arm lower on |
|---|---|---|---|---|
| cells dug, room / crowding | **1.12** | 0.14 | 5.19 | **6 of 12** |
| cemented spoil above ground | **1.01** | 0.20 | 3.50 | **6 of 12** |

Six of twelve each way is a coin flip, and the per-seed spread — a seventh of
the digging on one bed, five times it on another — dwarfs it. **This lane read
seed 3 alone first and got "+27% digs, +13% mound", which is the tidy
single-seed answer `CLAUDE.md` warns is evidence of an artifact.** Do not quote
a one-seed figure for anything on this bed.

**Consequence, and it is the finding:** an anthill that never stops growing is
not a colony asking the wrong question at the door. It is a colony that
outgrows its rooms faster than it can cut them. The lever is colony size —
brief 2's lifespan work — not the dig gate's input.

## 3. What the mechanism does do, which is worth keeping

The loop closes and reopens, cleanly, and the census reads it (seed 3, gate
on): **0.38 cells of room per ant at 20,000 frames** (occupancy 0.84, dig
hard), 4.25 by 60,000 (0.32, quiet), 10.03 by 120,000 as the colony thins
(0.17, off), back to 3.66 by 160,000 as it fills again (0.35). The input's
realised range at 100,000 frames is **0.187 to 1.000, median 0.555** against
an old input that this brief was told read 1.000 flat. Re-test condition (0)
of the `(Crowding, Dig, 0.6)` entry is discharged: the input leaves
saturation. It is the *rest* of that entry's reasoning that does not survive.

## 4. Two instrument corrections that cost real time

**`World::ground_datum` is built and wrong in the lab.** The hand-built bed
marks the whole box underground, so the datum reads **0 in all 512 columns**
and the sealed lid roofs the sky. The census read **8,544 cells of roofed void
against `latecensus`'s 26** — a 330x overcount that every unit test passed
through, because the test box has no lid at row 0 and no grow lamps. The
census freezes its own datum (`World::room_datum`). Anything else in this repo
reading `ground_datum` in a lab box has the same bug waiting.

**An incremental counter cannot track roofed void**, which is what the brief
asked for. Soil is a `Powder` so galleries collapse, and the spoil drop needs
`SPOIL_HEADROOM` of clear air so a pellet never lands in a roofed cell. Seed 1
logs 1,969 digs and stands at 306 roofed; a digs-minus-dumps counter reads
five to eighteen times the standing void.

## 5. Instruments this leaves behind

`NestRoom::room_per_ant` / `occupancy`; `CreatureStats::dig_rolls` (the "it
fired" half, paired with `digs`), `at_nest_ticks`, `at_nest_crowding`;
`census::Sample::mound_bare` / `mound_cols` — **bare columns on the mound's own
surface**, which `bare_in_band` cannot answer, because a seedling at the foot
of a heap marks the whole column vegetated while the slope above it is bare.
`latecensus control=selftest` asserts all of them against known geometry.
