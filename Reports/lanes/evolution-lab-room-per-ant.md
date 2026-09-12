# Lane P — room per ant, and the premise it was built on

*Round thirty, brief 3 of the late-game programme
([`../evolution-lab-late-game-design-2026-09-12.md`](../evolution-lab-late-game-design-2026-09-12.md)).
Built 2026-09-12. **The mechanism works and the diagnosis behind it does
not.** Read §2 before proposing anything about the dig gate again.*

## 1. What landed

At the nest, `BrainInput::Crowding` carries **room** rather than density: the
roofed void the nest patch holds over the ants in it, as
`target / (target + room)` so more room reads lower (`NestRoom::occupancy`).
Away from the nest it is the same 5x5 count, which is what `(Crowding, Move,
-0.3)` depends on. **`ant.ron`'s wiring is unchanged** — only what the slot
means at the nest changed.

`World::step_nest_room` takes the census in `begin_step`, once per 256 frames,
on `step_nest_scents`' own idiom, and is itself gated on the dial. Two dials on
the lab's ANTS page and as env switches: `PIXEL_PHYSICS_LAB_ROOM`,
`PIXEL_PHYSICS_LAB_ROOM_TARGET`.

**It ships off**, departing from *ship new behaviours on by default*, and
that is the finding rather than a preference: on costs **+4.0% of a frame**
(paired, alternating, quiet box; the census alone, on a byte-identical
trajectory, is +1.7%) and builds a *bigger* mound on 9 of 12 seeds. A default
is the one form in which a behaviour cannot be declined. The owner has the
numbers and the switch (card `20260912T115506982Z-6ae8e2`).

## 2. The premise is false on this bed — do not re-derive it

The brief, and `dead-ends.md`'s `(Crowding, Dig, 0.6)` entry under it, rest on
one claim: *the old input is pinned at its ceiling at the nest, so nothing the
colony digs can lower it* (median 1.000, p90 and max 1.000). **That figure was
measured on a different scene and it does not describe `played_bed`.**

`CreatureStats::at_nest_crowding` buckets what the gate actually read,
weighted by ant-ticks rather than by census stops. **Pooled over twelve seeds,
gate off, 300,000 frames each, n = 835,536 at-nest ticks:**

| bucket | 0.0–0.1 | 0.1–0.2 | 0.2–0.3 | 0.3–0.4 | 0.4–0.5 | 0.5–0.6 | 0.6–0.7 | 0.7–0.8 | 0.8–0.9 | 0.9–1.0 |
|---|---|---|---|---|---|---|---|---|---|---|
| ticks | **271,185** | 85,255 | 128,946 | 60,817 | 0 | 54,015 | 36,760 | 33,166 | 30,344 | 135,048 |

**Median bucket 0.2–0.3, a third of every read in the bottom tenth, a sixth at
the ceiling.** It held on the pre-#343 baseline too (median 0.3–0.4, bottom
tenth 24.8%, n = 687,349), which is the only reason to trust it: two different
worlds, same answer. An ant at its own door is very often standing alone. The old
gate was never stuck, so there was nothing for a desaturated input to release.

## 2b. And the outcome is a coin flip — 12 seeds, paired

`latecensus scenario=played_bed frames=300000`, `RAYON_NUM_THREADS=1`, each
seed run twice with only `PIXEL_PHYSICS_LAB_ROOM` changed:

| | median | min | max | room arm lower on |
|---|---|---|---|---|
| cells dug, room / crowding | **1.48** | 0.14 | 111.8 | 4 of 12 |
| cemented spoil above ground | **1.32** | 0.04 | 74.2 | 3 of 12 |

**It leaves a bigger mound on nine of twelve seeds** — a sign test puts that at
p ≈ 0.15, so not a result, but certainly not the reduction it was built for.
The per-seed spread is the real finding: 0.04x to 74x on one metric, over one
knob, on twelve beds of the same scenario.

**Two things this table cost, both worth carrying.** This lane read **seed 3
alone** first and got a tidy "+27% digs, +13% mound" — the single-seed artifact
`CLAUDE.md` names, and the same shape that sank the `(Crowding, Dig, 0.6)`
build. And the **same 12-seed sweep on the pre-#343 baseline read a clean coin
flip** (median 1.12 and 1.01, 6 of 12 each way); the round-trip fix landing
under it moved absolute digging by an order of magnitude on some seeds and
moved the comparison with it. A sweep is a measurement of one trunk, not of a
mechanism.

**Consequence, and it is the finding:** an anthill that never stops growing is
not a colony asking the wrong question at the door. It is a colony that
outgrows its rooms faster than it can cut them. The lever is colony size —
brief 2's lifespan work — not the dig gate's input.

## 3. What the mechanism does do, which is worth keeping

The loop closes and reopens (seed 3, gate on): **0.38 cells of room per ant at
20,000 frames** (the gate reads 0.84, dig hard), 4.25 by 60,000 (0.32, quiet),
10.03 by 120,000 as the colony thins, back to 3.66 by 160,000 as it fills
again. The input's realised range is **0.187 to 1.000, median 0.555** against
an old input this brief was told read 1.000 flat, so re-test condition (0) of
the `(Crowding, Dig, 0.6)` entry is discharged. It is the *rest* of that
entry's reasoning that does not survive.

## 4. Two instrument corrections that cost real time

**`World::ground_datum` is built and wrong in the lab.** The hand-built bed
marks the whole box underground, so the datum reads **0 in all 512 columns**
and the sealed lid roofs the sky: the census read **8,544 cells of roofed void
against `latecensus`'s 26**, a 330x overcount every unit test passed through,
because the test box has no lid at row 0 and no grow lamps. This census freezes
its own datum. Anything else reading `ground_datum` in a lab box has the same
bug waiting.

**An incremental counter cannot track roofed void**, which is what the brief
asked for: soil is a `Powder` so galleries collapse, and the spoil drop needs
`SPOIL_HEADROOM` of clear air so a pellet never lands in a roofed cell. A
digs-minus-dumps counter reads five to eighteen times the standing void.

## 5. Instruments this leaves behind

`NestRoom::room_per_ant` / `occupancy`; `CreatureStats::dig_rolls` (the "it
fired" half, paired with `digs`), `at_nest_ticks`, `at_nest_crowding`;
`census::Sample::mound_bare` / `mound_cols` — **bare columns on the mound's own
surface**, which `bare_in_band` cannot answer, because a seedling at the foot
of a heap marks the whole column vegetated while the slope above it is bare.
`latecensus control=selftest` asserts each against known geometry; `labshot`
grew `scale=` and prints the nest's room beside the picture.
