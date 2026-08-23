# Springs in generated worlds

*Written 2026-08-22, when the pass landed; updated the same day with the
review verdict. Status: shipped. The sink half of the verdict is done; the
source half is a recorded dead end with an untried route out.*

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

**Correction, 2026-08-23: this pass invalidated that measurement, and then
the harness stopped saying so.** Two faults, found together.

*The control arm stopped being a control.* `canyon` ships `spring_flow: 5.0`,
so once **this** pass landed, the world the river-cost scene builds contains a
generated spring — running in the "spring OFF" arm too. Re-measured, the
control reported **`awake chunks max 2`**, not the 0 the paragraph above rests
on, and the pass's water surfaced in the ledger as `unaccounted -369347` on
2,000,000 emitted: 18%, against the harness's own printed criterion that a
large residual means it is lying. A pass invalidating its own baseline is
exactly the shape of thing that goes unnoticed, because both arms move
together and the *difference* still looks plausible. The scene now builds both
arms with `spring_flow: 0.0` and the control is back to **0 awake chunks**.

*And the scene still drained at the world's lowest column.* The paragraph
below — "The drain has to be in the plunge pool, and there has to be more than
one" — names this scene, by number, as the case that proves it, and then only
the worldgen pass was fixed. Measured at the shipped size: drain in column
6531 against an outlet at 4501, **2030 columns away, `drained 0` after 1400
frames**, every one of 2,000,000 emitted fill units still standing. So the
"standing bill" it printed was a *filling bath*, not a steady state.
`viewshot`'s `spring=` branch had the identical global read, which matters
because `spring::MAX_TOTAL_SPAN`'s doc names that harness as the instrument
for re-pricing the flow budget as spans grow — the tool for pricing a wider
waterfall was measuring a bath too.

The rule now lives once, in `passes::spring_drains`, and all three callers use
it. The harness's hand-rolled drain also took the topmost water in a 60-row
window *up from the basin floor*, which is the construction this pass
explicitly rejected — it takes water as fast as it lands, so no pool ever
stands. It drains the drain cell only, like `spring::step`.

**Re-measured, same session, same machine, at the shipped 8192x2560:**

| | before | after |
|---|---|---|
| drain distance from outlet | 2030 columns | **8** |
| `drained` of 2,000,000 emitted | **0** | 919,176 |
| unaccounted | −369,347 (−18%) | +115,808 (+5.8%) |
| awake chunks, **control** arm | 2 of 5120 | **0** |
| mean, spring off | — | 8.921 ms |
| mean, spring on | — | 11.945 ms |
| **standing bill** | 1.734 ms/frame | **3.025 ms/frame** |

**The bill is nearly double what the scene used to report, and that is the
point of fixing it.** Both faults pushed the same way: a bath that never
drains stops costing anything once the pool stops spreading, and a control
with a live spring in it already carries part of the cost being subtracted.
At **3.025 ms/frame** a single spring is past the pre-registered 2.0 ms bar
and inside the ~3.5 ms wind-revert class — the number this scene exists to
gate, so it should be read before any change widens a fall or adds one. The
1.734 ms figure should not be quoted again.

The residual sign flipped, which is the useful tell: **negative** before
(more water in the world than the harness emitted — the pass's own spring),
**positive** now (water leaving to evaporation and infiltration, the two
sinks the harness's message already names). At 512x320 it was positive both
times, ~374k, which is that same legitimate loss and not this bug.

Steady state is reached partway through rather than at frame 0: the pool has
to fill to the drain's height before anything leaves, so `standing delta`
965,016 is the standing pool, not a leak.

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

## The card came back: "it comes from nowhere and goes nowhere"

> *"It is too thin, but i think the biggest issue is that it looks like it
> comes from nowhere and goes nowhere. spring should originate in depressions
> so they fill up and spill out into a waterfall. So it comes from a pool.
> Ideally it should also end in a pool. Not sure how we do that because the
> water in should equal the water out so we don't flood the world, but how do
> you get it to pool at the bottom first?"*

**The sink half shipped** (`f5f3b19`). The answer to his question is the
drain's *height*, not its rate: `spring::step` takes only from a drain cell
that currently holds a liquid, so a drain above the waterline is inert.
Nothing leaves until the pool has risen to it, and then it takes at most
`DRAIN_FILL` per frame -- which equals `EMIT_FILL`, so one drain balances one
emission column and the pool settles *at the drain's height* with the
throughput passing through. Conservation was never in tension with a standing
pool; the outlet just had to be at the lip instead of on the floor. Measured
over 500 frames, water standing in the fall's own 128-column bucket: seed 7
421 -> 630, seed 42 293 -> 761.

**The source half is now cut rather than found.** As a *search* it is a dead
end, recorded in `dead-ends.md`.
Finding a basin that spills over a given cliff requires that cliff's lip to be
the basin's lowest exit, and that landform does not occur here: a cliff edge
is a local high point, so the ground behind it rises. Requiring it honestly
placed **zero springs across four presets and six seeds**. About half of all
rims do have a hollow within 120 columns behind them (canyon 49/92, rolling
33/70, terraced 29/63, median 12 rows deep, ~50 wide) -- but behind
intervening high ground, so a spring put in one fills a pond that never moves
toward the cliff. `probe_p1_is_there_a_pool_behind_the_cliff` is the census.

**So the pass excavates one.** The measurement that killed the search is what
says cutting works: because the ground behind a rim never drops below the lip,
it stands *at or above* it -- a median 107 columns on `canyon`, 120 on
`rolling` and `terraced` -- and that shelf is exactly the back wall a cut
basin needs. `springs` walks the shelf, refuses anything that is not ordinary
ground, cuts a tapered bowl clearing each column from its top down (so nothing
is left overhanging and the carve is structurally safe by construction), and
fills it to the lip. The pool has two ways out, the cliff at distance 0 and
the far end of the shelf a hundred columns away, and it reaches the cliff
first. That is the whole mechanism.

**The basin has to sit in level ground, and that was learned the hard way.**
The first shelf test was "ground at or above the lip", which admits ground
*well* above it -- and since the cut clears each column from the sky down to
the bowl floor, a basin sited on rising ground is a sheer trench gouged
through a hillside. Shown one, the owner: *"a weird cut through a sharp piece
of stone."* Requiring the ground within `SPRING_BASIN_RIM` = 8 rows of the lip
for the basin's whole width fixes it, and costs nothing -- it *improves*
placement, because a narrower basin in flat ground qualifies where a wide one
on a slope did not.

Placement with the carve: **canyon 1.0 springs per world**, rolling 0.8,
terraced 0.8, wetland 0. Three canyon seeds emit 4.5M fill units and drain
~4.0M of it with **zero** throttling, so the pool never drowns its own
outlet.

Still open too: the fall is thin. Some of that was the drains taking water as
fast as it landed, which the sink fix addresses; the rest is that `render.rs`
dims a liquid toward black by *fill*.

## Explicitly deferred, at the owner's request

**Cliff faces are where springs go for now, and the owner has said a wider
distribution should be explored later** — the decision was "let's start with
cliff faces for testing but record that we want to explore a larger
distribution later". This is that record. Candidate sources not implemented:
valley-floor seeps, cave-mouth resurgences, and springs keyed to the
`hardness_field` contact between a permeable and an impermeable bed, which is
the one with real geology behind it.
