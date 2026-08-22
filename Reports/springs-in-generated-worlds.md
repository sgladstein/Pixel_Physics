# Springs in generated worlds

*Written 2026-08-22, when the pass landed. Status: shipped; the visual
quality is with the owner (review card `20260822T180743652Z-e79163`).*

## Why there were no rivers

`src/sim/spring.rs` has been a finished mechanism for a long time — springs,
drains, a throttle that chokes on a dammed outlet, a flow budget, a ledger,
and tests that assert the *stops* first. It is wired into both drivers, so it
runs every frame. What never existed was the pass that puts one in a world.
Its own module header said so; every caller of `World::add_spring` was a unit
test or `examples/viewshot.rs spring=`.

So a generated world got ponds, soil moisture and rain, and **no flowing water
at all**. Every waterfall anyone had been shown was a harness placing a spring
by hand. The owner asked why they had never seen a river in a world; that is
the answer, and this is the pass.

## What was measured before anything was built

The standing bill of one spring at the shipped 8192x2560, from `ascii`'s
river-cost scene — the instrument the rivers track was opened with:

| | mean ms/frame | awake chunks |
|---|---|---|
| spring OFF | 7.135 | 0 of 5120 |
| spring ON | 9.780 | 7 of 5120 |
| **standing bill** | **+2.645** | |

Over the 2.0 ms bar the harness prints, under the ~3.5 ms wind-revert class.
It is a permanent cost: the world never sleeps again while a spring runs.

**A warning about how not to measure this.** A first attempt used
`viewshot`'s paired settle timing and got **+7.94 ms**, with a span sweep
(1 → +8.18, 3 → +10.35, 5 → +7.68) that looked like proof the cost was the
lost field early-out and independent of size. All of it was noise: at
`settle=400` the run sits inside the opening wind gale that
`examples/field_cost.rs` documents, which drives field activity in *both*
arms. The purpose-built scene runs 1400 frames and reports 0 awake chunks
with the spring off, which is what says the early-out is not the story.

## Placement: three models, two of them wrong

The pass reuses `passes::cliff_edges`, the same candidate data `brows` and
`talus` consume. **The pass owns all geometric validity**, because
`World::add_spring` validates nothing about position — `add_spring(-9999,
-9999, 1)` returns `true`, and an outlet seated in rock is not an error
anywhere downstream. It is a spring that emits nothing for the life of the
world and reports it only as a climbing `throttled` count.

1. **Outlet at `table_y`** — the literal reading of "the aquifer daylights on
   the face", and what the design called for. It cannot work: `ponds` runs
   first and fills every cell where the ground has dropped below the table,
   so **the table's exposure surface is the pond surface**. Every candidate
   that reached the check was rejected for an occupied outlet — 26 of 26 on
   one seed, 1:1 on the rest. A spring seated there is a drowned spring.
2. **Outlet a fixed depth under the plan's `surface_y`** — fails because the
   face is *rough*. `talus` has piled scree on it, `brows` has hung a lip off
   it, and rock country now stands pillars on it, so a run of columns the
   plan calls clear is occupied in the built world: 331 of 339 faces.
3. **Outlet at the rim's real top in the finished world**, hung just past the
   rim on the falling side — `viewshot`'s hand-placed rule, and the one that
   runs. These are therefore **perched springs**, which `worldgen-design.md`
   §7 names directly as what a water table that is a *field* rather than one
   global level buys. The table decides *whether* (`table_y < h`, and it must
   be below the local ground), not *where*.

A fourth thing was removed rather than added: an early gate required the
table to lie between the rim's ground and the foot of the face. It rejected
65-92% of every preset's candidates and **all** of `canyon`'s, which ships
`table_offset: 70` and keeps its table below even its valley floors. Canyon
has the best waterfall faces in the game; a gate that switched it off
entirely was measuring the wrong thing once the outlet became perched.

## Two smaller findings, both measured

**The scan needs a seed-dependent origin.** Taking the first qualifying
candidate in x order put every world's waterfall in its first thousand
columns — measured across six canyon seeds at x = 1, 392, 409, 505, 873 and
1035, in a world 8192 wide. Rotating the scan start costs nothing and, unlike
a sparse acceptance draw, spends none of the scarce candidates: that was
tried and cut placement from 1.0 springs per world to 0.2.

**The drain has to be in the plunge pool, and there has to be more than one.**
`viewshot` drains at the *world's* lowest column, a global read that also does
not work — in the river-cost scene it lands 2030 columns from the outlet and
reports `drained 0` after 1400 frames. Reading the plan's surface instead of
the built world puts the drain inside talus, same result. And a single reach
is not enough: seed 7 emitted 4.2M fill units into one and still returned
`drained 0`. Drains at nested reaches (`MAX_FALL / 3` and
`SPRING_DRAIN_REACH`) fixed it. Drains are free — they only ever remove work
— so there is no budget on them the way there is on springs.

## What ships

| preset | springs per world (6 seeds) |
|---|---|
| canyon | 0.8 |
| rolling | 0.8 |
| terraced | 0.5 |
| wetland | 0.0 |
| arid, flat | 0.0 (`spring_flow: 0.0`) |

Wetland's zero is correct, not a gap: it offers **8** cliff candidates in a
whole world against 400-1400 for the others, and 7 of those 8 sit at or below
the water table. A waterlogged, level preset should have marsh, not falls.

Over three canyon seeds, 900 frames each: emitted 2.6-4.5M fill units,
**drained 90-98% of it**. The water arrives and leaves, which is what makes it
a river rather than a rising bath (`PLAN.md`: "a real source ... and a real
sink").

## Open: it does not yet look like a waterfall

The numbers are right and the picture is not. The fall hugs the rock face and
renders nearly black, because `render.rs` dims a liquid toward black by
*fill* and the drains take the water almost as fast as it lands, so no cell is
ever near full — 421 water cells standing after 500 frames. The first lever to
try is moving the near drain further from the foot so a pool actually stands
under the fall. With the owner on the card above.

## Explicitly deferred, at the owner's request

**Cliff faces are where springs go for now, and the owner has said a wider
distribution should be explored later** — the decision was "let's start with
cliff faces for testing but record that we want to explore a larger
distribution later". This is that record. Candidate sources not implemented:
valley-floor seeps, cave-mouth resurgences, and springs keyed to the
`hardness_field` contact between a permeable and an impermeable bed, which is
the one with real geology behind it.
