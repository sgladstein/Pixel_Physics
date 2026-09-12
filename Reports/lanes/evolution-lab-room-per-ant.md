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

**It ships on, and that is the owner's eye overruling this lane's counters.**
The counters say off: on costs **+4.0% of a frame** (paired, alternating,
quiet box; the census alone, on a byte-identical trajectory, is +1.7%) and
roughly doubles both the digging and the mound. It shipped off on that. Then
the owner judged a blind A/B of the two arms at 300,000 frames on today's trunk
(card `20260912T134505685Z-b518ab`): ***"A is bad. B is good"***, and
`blind_was` `[1, 0]` puts the room arm in pane B. **The arm that measures worse
on mound size is the one that reads better as an anthill.** That is this repo's
own order of evidence, and the counters stay in the record rather than being
explained away — they are why this is a dial.

`PIXEL_PHYSICS_LAB_ROOM=off` is byte-identical to `main` **through 300,000
frames**, not merely through 20,000: a determinism control shorter than a
mechanism's onset proves nothing, and this bed's arms have run identical for
100,000 frames before splitting.

## 2. The premise is false on this bed — do not re-derive it

The brief, and `dead-ends.md`'s `(Crowding, Dig, 0.6)` entry under it, rest on
one claim: *the old input is pinned at its ceiling at the nest, so nothing the
colony digs can lower it* (median 1.000, p90 and max 1.000). **That figure was
measured on a different scene and it does not describe `played_bed`.**

`CreatureStats::at_nest_crowding` buckets what the gate actually read,
weighted by ant-ticks rather than by census stops. **Pooled over twelve seeds,
gate off, 300,000 frames each, n = 442,788 at-nest ticks:**

| bucket | 0.0–0.1 | 0.1–0.2 | 0.2–0.3 | 0.3–0.4 | 0.4–0.5 | 0.5–0.6 | 0.6–0.7 | 0.7–0.8 | 0.8–0.9 | 0.9–1.0 |
|---|---|---|---|---|---|---|---|---|---|---|
| ticks | **166,212** | 51,232 | 73,228 | 29,108 | 0 | 27,493 | 18,074 | 14,610 | 14,638 | 48,193 |

**Median bucket 0.2–0.3, 37.5% of every read in the bottom tenth, 10.9% at the
ceiling.** **The exact zero in bucket 4 is a positive control on the probe, not
a flaw in it** — `CROWDING_SCALE` is 8.0 and the count is an integer, so the
input can only take `k/8`, and `[0.4, 0.5)` is the one decile eighths cannot
reach. A miswired probe would not have put its hole in precisely that bucket.
It also means "median bucket" here is a median over nine reachable values
rather than over a continuum. It held on all three trunks this lane measured — bottom tenth 24.8%
before #343 and 32.5% before #354 — which is the only reason to trust it:
three different worlds, same answer. An ant at its own door is very often standing alone. The old
gate was never stuck, so there was nothing for a desaturated input to release.

## 2b. And the outcome is a coin flip — 12 seeds, paired

`latecensus scenario=played_bed frames=300000`, `RAYON_NUM_THREADS=1`, each
seed run twice with only `PIXEL_PHYSICS_LAB_ROOM` changed:

| | median | min | max | room arm lower on |
|---|---|---|---|---|
| cells dug, room / crowding | **1.90** | 0.83 | 138.7 | **1 of 12** |
| cemented spoil above ground | **2.16** | 0.02 | 2690 | 3 of 12 |

**It digs about twice as much and leaves about twice the mound.** Eleven of
twelve seeds digging more is p ≈ 0.006 by a sign test — a result, and the
opposite of the one the build was commissioned for.

**The sweep was run on three trunks and only the last is quotable.** Before
#343 it read a clean coin flip (median 1.12 and 1.01, 6 of 12 each way); before
#354 a lean (1.48 and 1.32, 9 of 12 mounds bigger); on today's trunk it is
one-directional. Round 29's ant work moved absolute digging by two orders of
magnitude on some seeds and moved the comparison with it. **A sweep measures
one trunk, not a mechanism** — a lane measuring across a round of landings must
re-take it after each, not average them. This lane also read **seed 3 alone**
first and got a tidy "+27% digs, +13% mound", the single-seed artifact
`CLAUDE.md` names.

**Consequence:** an anthill that never stops growing is not a colony asking the
wrong question at the door. It is a colony that outgrows its rooms faster than
it can cut them. The lever is colony size — brief 2's lifespan work — not the
dig gate's input.

## 2c. The brief asked the wrong question, and the right one is survival

**Re-read from the same 12 paired runs, after the owner picked the room arm by
eye and the coordinator asked what he might actually have been looking at.**
Colony still alive at frame 300,000:

| | alive at 300,000 |
|---|---|
| crowding at the door (today) | **0 of 12** |
| room at the door | **5 of 12** — 427, 222, 181, 178 and 46 ants |

Every control colony is extinct. Five discordant pairs, all one way, is
**p ≈ 0.03** one-sided by McNemar. **The gate's value was never the mound.**

That also rescues the owner's verdict from the objection against it. He was
shown one bed where the control had died and the room arm had not, so his
"A is bad, B is good" could have been one bed's luck — and it is not: it is
5 of 12 against 0 of 12 across the sweep.

**Two honesties on top of it.** The bed pays for a living colony: where the
room arm survives, the seed bank and the standing stand are *lower*, and
starvation deaths are much higher because there are ants alive to starve.
Pooled, neither seed bank nor plant count moves reliably (7 of 12 each way).
**And the mechanism is unmeasured** — whether a colony that stops digging when
it has room spends the saving on foraging is a plausible story and nothing
here tested it. That is the question the next lane should take, and it is a
better one than the brief's.

## 3. What the mechanism does do, which is worth keeping

The loop closes and reopens (seed 3, gate on): **0.38 cells of room per ant at
20,000 frames** (the gate reads 0.84, dig hard), 4.25 by 60,000 (0.32, quiet),
10.03 by 120,000 as the colony thins, back to 3.66 by 160,000 as it fills
again. The input's realised range is **0.187 to 1.000, median 0.555** against
an old input this brief was told read 1.000 flat, so re-test condition (0) of
the `(Crowding, Dig, 0.6)` entry is discharged. It is the *rest* of that
entry's reasoning that does not survive.

## 4. Two instrument corrections that cost real time

**`World::ground_datum` is built and wrong in the lab — filed as §Z17.** The hand-built bed
marks the whole box underground, so the datum reads **0 in all 512 columns**
and the sealed lid roofs the sky: the census read **8,544 cells of roofed void
against `latecensus`'s 26**, a 330x overcount every unit test passed through,
because the test box has no lid at row 0 and no grow lamps. This census freezes
its own datum. Anything else reading `ground_datum` in a lab box has the same
bug waiting. The owner's own unprompted report of spoil standing in open sky,
seen on both arms and therefore not this build's, is **§Z18**.

**The two roofed rules are now reconciled and were not.** `World::step_nest_room`
and `lab::census::census` each decide what counts as roofed; two definitions of
one word drift silently. They had: seed 3 at 300,000 frames read **422 against
289**, because the room datum was frozen lazily at the colony's arrival rather
than at the bed's construction, so litter risen above the original surface
counted as below it. Frozen in `begin_step` instead, they agree exactly on the
selftest box and to a **median 0.97** on the real bed over 12 seeds.
`latecensus control=selftest` now asserts the equality.

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
