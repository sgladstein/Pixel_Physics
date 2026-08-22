# Open bugs handoff

Rewritten at the end of the session that landed `15b2e51` … `ad1e227`.
Everything here was measured, not reasoned — where something is a guess it
says so, and where a plausible idea was measured and found wrong it is
recorded with its numbers so it is not tried twice.

Read `CLAUDE.md` first; it holds the method these bugs keep re-teaching.

---

## Open

### 0. ~~A melting `Powder` manufactures water~~ — **FIXED**

**Resolution (2026-08-20):** fixed in `fire::transform`'s aux table, with
the conservation test this section asked for
(`weather.rs`'s `a_thaw_does_not_manufacture_water`, written first and
confirmed red at **123.4%** before a line of fix went in).

The fix is *not* the plain density scale this section proposed, and the
difference was found by measuring the version this section described.
`fire::melt_fill` splits on whether the pair is **reciprocal** — whether
the liquid's own `cools_into` names the phase that is melting:

- **Snow** (nothing freezes water into snow; the sky makes it) melts at its
  own density, 0.3, so 1,700 flakes are ~510 cells of meltwater. That is
  the arm this section was about and it is the whole of the flood.
- **Ice** (water froze it, and `freeze_min_fill` refused to freeze anything
  but a near-full cell) comes back **full**, which is what it took.

**Why the density scale is wrong for ice, measured rather than argued.** A
`Solid` carries no fill, so `1000 -> ice -> 920` loses 8% of every cell
that freezes full — nearly all of them — and `scene=coldsnap` cycles its
pond surface roughly ten times in one front (froze 2,608, melted 4,671
against a 60-cell pond). It compounds: the pond read 1,200 cell-equivalents
at the cut and 1,050 by frame 361, and two rows of drop took the ice
sheet's end cells out from beside their shore anchor — **2 unconfined
overloads (133 cells) and 4 unsupported**, a visible wedge of sheet
slumping into the water, on the one acceptance case whose bar is that
nothing is dismantled. Returning it full closes that loop exactly, and
tiles 0-2 of the acceptance run are then numerically identical to the
pre-fix ones: only the snow half moved.

Measured, `filmstrip scene=coldsnap ... count=6`, standing water at frame
1080: **3,231.9 cell-equivalents before, 1,789.9 after** — the hillside
flood goes from a three-cell-deep band to a one-cell film, judged at
`crop=300,234,80,14 zoom=10`. `max_unconfined=0` is **OK**, with the same
failure signature as before the fix (4 overloaded, 845 cells, all
confined).

Residual, recorded rather than tuned away: a cell that froze *at* the gate
(fill 900) rather than full gives back 11% more than it took, so a whole
storm's freeze/thaw reads ~104% on the census. The ceiling is structural —
`LIQUID_FULL - freeze_min_fill` — and closing it means paying a matching
loss on the far commoner full cell, which is the worse trade above. The
guard bar is 110% against 103.9% measured.

Two tests moved with the fix and neither was a rubber-stamp:
`fire.rs`'s melt test asserted `aux == 0` (i.e. *full*) for every melt —
that assertion **was** the bug — and is now split into a density case and a
reciprocal round-trip case. `weather.rs`'s
`a_snowstorm_leaves_no_snow_floating_on_open_water` counted a proxy that
moved for a reason that was not the artifact (5 -> 11 columns, while the
pond went on freezing); it is now
`a_snowstorm_leaves_no_snow_raft_insulating_the_pond` and asserts the two
things a raft would actually do.

### 0b. ~~`scene=lavapour`'s pond simmers forever~~ — **CLOSED: the "eternal loop" had a literal heater in it**

The fix is one line of content: `rubble.ron` gains `heat_conductivity:
0.1`, per README's own rule that every `breaks_into` target needs one
(stone breaks into rubble, and a crush inside a quench delta crushes *hot*
stone). With it, the scene that boiled ~5.5 cells a frame at frame 18,000
with 7/40 chunks awake now stops boiling at 1,862 events total and the
world is **fully asleep (0/40) by frame 4,500**.

The hunt is worth keeping because three attractive wrong answers were
measured on the way, and each would have shipped without the per-material
spatial census filmstrip now prints (`>=100C in <material>: n cells, mean,
box`) — added for this hunt, kept as instrumentation:

1. **It looked like a thermodynamic pump** — molten 0, burning 0, yet the
   ≥100°C population held steady. The finite-inventory control
   (`fire.rs`'s `a_finite_heat_inventory_stops_boiling_and_the_world_
   sleeps`) ruled that out: the loop manufactures nothing in general.
2. **The census found a trapped steam pocket** (~1,400 cells at a mean of
   163°C sealed inside the quench delta), so **direct-contact
   condensation** was built — a `steam + water -> water + water` reaction,
   in two thermal variants. Both made the system *worse* (exchange rule:
   every collapse minted a ≥100° boiler, ~35 events/frame; mean rule,
   added as `ReactionDef::mixes_heat`, kept: lossless churn at ~34
   boils/frame with the warm population growing). Two variants failing
   the same way meant the approach, not the tuning — reverted, machinery
   kept with a test.
3. **A latent-heat cost on boiling** (steam born 40° cooler) was
   implemented on the "lossless loop needs a sink" theory. It worked —
   and the isolation control showed the rubble fix alone sleeps *sooner
   without it* (0/40 vs 2/40 at frame 4,500), so it was reverted per
   keep-each-fix-minimal. The idea is recorded at its site in `fire.rs`.

What the census finally showed, in one line: `>=100C in rubble: 90 cells,
mean 302C` — **byte-identical across 4,500 frames**. Hot rubble with zero
conductivity can never cool, and 90 permanent 300° radiators inside the
cavity re-warmed everything the other rules drained. The general lesson,
now also in `fire.rs`: any material that can *inherit* a temperature —
through `transform`, burnout, a crush, or a reaction — and has zero
`heat_conductivity` is a permanent radiator, and the next "eternal" heat
loop should be checked for one before any thermodynamics is redesigned.

Ruled out by measurement, in order:

- **The cooling model is not the cause** — the pin-era scene had the same
  simmer, worse (~25 boil/condense pairs a frame), on top of 195 cells of
  permanently molten lava.
- **The boil/condense loop does not manufacture heat in general.**
  `fire.rs`'s `a_finite_heat_inventory_stops_boiling_and_the_world_sleeps`
  is the control: a sealed basin with one 700°C stone row boils 30 cells,
  stops, and sleeps before frame 2,000.
- **A pond alone terminates.** `scene=boil` reads flat at frame 8,000:
  boiled 228,297 vs condensed 228,296, zero cells ≥100°C, awake 4/40 and
  draining. Its long tail had a real source the census exposed — 33 cells
  *still burning* at frame 4,000, fire creeping through the slick far past
  any single cell's 180-frame duration.

Verify it stayed closed with the same command that found it:
`cargo run --release --example filmstrip -- scene=lavapour start=1500
every=1500 count=4` — the standing census under each tile should show
zero cells ≥100°C from the first tile and the run fully asleep.

### 1. ~~Whiskers on a spreading front~~ — **CLOSED in the movement rule, and the render-side successor is falsified with it**

One-cell-tall sheets of water with open air above *and* below, drawing as a
comb of detached horizontal ledges along a spreading front. Reported from
live play. Distinct from the row banding that was fixed — that was a fill
deficit *inside* the body; this is the shape of its *edge*.

**Fixed by `LIQUID_SETTLE_DROP`** (`update.rs`): a `find_lateral_descent`
move now continues down to where the cell comes to *rest*, at most two extra
rows, instead of landing one row down in a column that is open by
construction. The bar
`a_spreading_front_does_not_shed_a_comb_of_detached_ledges` is 40 against a
measured **0**, where the artifact stood at 277.

This entry previously said the honest fix was probably not in the movement
rule at all but in how a one-cell sheet is *drawn*. **That reading has now
been measured and it is wrong**, and the numbers below are here so nobody
spends a session on it. Diagnosis harness: `examples/film_probe.rs`, and
`filmstrip scene=shelf`.

**Verified by disabling the fix and re-running, not by trusting the bar.**
`LIQUID_SETTLE_DROP: i32 = 0` reproduces the pre-fix engine exactly, and the
paired comparison is unambiguous by eye as well as by number — the fringe of
detached dashes along the front is gone. Set it to 0, rebuild (`include_str!`
is not involved but the constant is compiled in), and shoot the same crop
twice:

```
cargo run --release --example filmstrip -- scene=fall  start=150 every=50 count=4 cols=2 zoom=3 crop=200,230,180,70
cargo run --release --example filmstrip -- scene=pour  start=300 every=100 count=4 cols=2 zoom=3 crop=150,230,180,70
cargo run --release --example filmstrip -- scene=shelf start=110 every=8  count=4 cols=2 zoom=3 crop=140,170,220,60
cargo run --release --example film_probe -- scene=fall frames=400
```

| scene | comb cells, peak / mean / % of frames present | with the fix disabled |
|---|---|---|
| `fall`, 400 frames | 6 / 0.0 / **0.2%** | 276 / 88.1 / 72% |
| `pour`, 600 frames | 11 / 0.2 / 2% | 307 / 124.2 / 100% |
| `waterbed`, 400 frames | 0 / 0 / 0% | — |
| `shelf`, 400 frames (new; water onto an unwalled ledge) | 32 / 7.2 / 37% | 232 / 102.9 / 72% |

*Comb cells* means cells in a horizontal run of six or more films, per
frame — the same quantity the bar counts. `pour` run out to 1500 frames
settles to **zero** films and zero comb cells: whatever a future film
treatment does, it cannot touch a resting pool, because a resting pool has
no films at all.

**The metric that matters, and the third way this bug has been measured
wrong.** `CLAUDE.md` already records two: a raw film count counts every
falling droplet, and film *creation* blamed the straight-down fall for 76%.
The obvious correction — count films that **persist at the same cell** —
is worse than either, because it reads **exactly zero on a world where the
comb is unmistakable**. With the fix disabled, `fall` holds 247 comb cells
and **not one cell survives three frames** (lifetime p50 1, max 2); no
*row* holds a comb for more than 2 consecutive frames either. The comb
**travels**: the front advances a diagonal step per frame and every tooth
is a new cell. Anything keyed by position sees a shower of droplets. Use
the per-frame snapshot, and treat "standing" as a property of the pattern,
not of a cell.

**Why the render-side treatment is dead.** Every candidate keyed on fill —
draw a sub-threshold film as a partial row, dim it toward the sky — needs a
population of near-empty films to act on, and there is none. Of all film
cells seen with the fix disabled, on `fall`: **67.2% are completely full,
86.1% are at 80% or more, and 3.7% are below 40%**. On `pour`, the same
shape: 68.5% full, 89.2% at 80% or more, 3.1% below 40%. The comb is not a
rendering of thin water; it is full cells of water genuinely sitting in air,
drawn correctly. A fill-keyed treatment
would have addressed under 4% of the artifact — and, being a per-material
`fill_dimming`, would have hit the resting waterline instead, which is
exactly why `water.ron` sets `fill_dimming: 0.0` (see that field's doc:
a settled top row spans fill 286..1002 and dims into a mottled band).

The other render candidate, merging a film's pixel into the surface below,
fails on geometry rather than fill: with the fix disabled only **46.8%** of
comb cells sit within one row of anything, 22% hang 4–9 rows up. It reaches
under half the artifact and misreports where the water is for the rest.

Two more things ruled out with numbers. It is **not** chunk decomposition:
3.0% of comb cells on `fall` and 3.5% on `pour` lie on a horizontal
chunk-seam row, against the ~3% a uniform scatter gives. And **evaporation
is irrelevant to it** — films die by moving, in one or two frames, orders of
magnitude before evaporation could reach them; the `evaporate` scene
produces zero films across 600 frames.

Also **not** the VOF flotsam-and-jetsam the liquid research reports
diagnose. Their fix is a three-cell height function for partial-fill
droplets orphaned by interface reconstruction; measured here, the drained
basin strands 54 cells while producing **zero** films, and the films
elsewhere are mostly *full* cells, as the table above now quantifies.

**Three earlier candidates, measured and rejected** — kept because a revert
keeps the knowledge, and because two of them are still tempting:

| tried | result |
|---|---|
| Disable `find_lateral_descent` | −75% whiskers, and water reads as sand again — the original bug |
| Land the mover at `(tx, y)`, fall next frame | whiskers 2540 → 1635, but enclosed holes 289 → **1040** |
| Shrink `LIQUID_LATERAL_REACH` | pure trade against levelling, no path to zero: 24/12/6/3 → whiskers 290/175/151/119, levelling 343/557/1017/1661 frames |

**What is left open, and it is small.** A shelf pour — water onto a short
unwalled ledge, spreading with open air under most of its length — still
sheds a residual comb: worst 38 cells in the bare `parallel::step` loop, 32
under the probe's fuller step, against `fall`'s 0. It is barred at 80
(`a_shelf_pour_does_not_shed_a_comb_either`), and that bar exists because
the `fall` bar sits at 40 — **the geometry that sheds the most was sitting
under an untested bar**, `CLAUDE.md`'s "check that a guard's inputs actually
vary what it guards" in its liquid costume. 81.5% of the residue sits one
row above a surface, and its films are the only substantially partial ones
in the engine (19% below 40% fill against 2.5% on `fall`) — so if a
fill-keyed render treatment is ever built, *this* is the population it would
act on, and `filmstrip scene=shelf` is where to judge it. At 32 cells in a
falling curtain, where a one-cell horizontal streak reads as spray rather
than as a ledge, it does not look worth the frame cost: the renderer has no
dirty-rect equivalent, and distinguishing a comb tooth from a droplet needs
the run length, i.e. a neighbour scan on every liquid pixel every frame.

### 1l. Boiling never puts a bubble *in* the water

**Reported from play, measured, and deliberately not fixed** — it is a
mechanism rather than a constant, and the session it surfaced in was
wrapping up.

Reported about a heat source under a pool: *"I see bubbles form at the
bottom, rise to the top and pop, causing surface bubbles"*, and separately
that the drawn bubbles *"still read as animations instead of real
physics"*. Both are the same complaint and both are right.

`examples/filmstrip.rs`'s plume census now counts **steam with water
directly above it** — a bubble, by definition. Nothing counted it before,
and nothing could have seen this: a plume standing over a pond and a pond
full of rising bubbles give the same `steam` total, and at the zoom a
contact sheet is read at they look the same too.

Six tiles each, at `start=100 every=150`:

| scene | submerged steam | steam cells at peak |
|---|---|---|
| `lavapour` | **0** for the whole run | 104 |
| `lavadrop` | 0, 3, 0, 0, 0 | 496 |
| `simmer` | 5 on the first tile, then 0 | 11 |

So the engine essentially never puts gas inside a liquid. Boiling happens
where the hot face meets the water and the steam leaves upward from
there; nothing forms at a floor and travels up through a column of water.

**This is why the drawn bubbles read as animation: they are.** `render.rs`
computes them from position, frame and cell temperature, with no writes
back into the world — keyed to water that genuinely is near boiling, but a
mark on the screen. They have been standing in for a mechanism that does
not exist, which is a defensible thing to do only while everyone knows it.

Where to start, and what to check first:

- Find out *why* a boil at the bottom of a pool does not leave a steam
  cell in the water. Two candidates, and they want different fixes: either
  `fire.rs` will not boil a cell that has no free face (so only the
  interface ever converts), or the gas-through-liquid swap moves the new
  steam to the surface within the frame it is created. The census above
  cannot tell these apart; a counter at the conversion site can.
- **Check `steam.ron`'s `cooling_point` of 45 before blaming the boil.** A
  bubble rising through a pool at 120 degrees is thermally stable, so
  condensation is not what is removing them — the earlier reading that it
  was is recorded as wrong in this session's transcript.
- Ask the size question before building: at roughly 1.8 cm to the cell, a
  real boiling bubble is **sub-cell**, so a physical bubble in this engine
  is always at least an order of magnitude too big. That does not make the
  mechanism wrong — a stream of one-cell bubbles leaving a hot floor is
  exactly what the report asks for — but it does mean the drawn overlay
  probably survives alongside it rather than being replaced by it.

Related and still open: the coverage step in a freezing pond (*"it seems
to slowly grow and then jumps to fully frozen"*). Ruled out as the
day/night lighting alias in the contact sheet, which was a real harness
bug but not this. Ruled out as ice thickening, which now follows Stefan's
law. What is left is **lateral** spread across bare water under snowfall,
where a landing flake chills nine columns at once — and a snowy night
really does ice a surface over faster, so it is not obvious this is wrong.
Measured across the step: freezing goes +40 cells in one window to +246 in
the next.

### 1b. `diffuse_heat` does not conserve heat, and a hot cell is an amplifier

**Found while braking a boil-off, measured, and deliberately not fixed** —
it is the hottest loop in the engine and the right answer is the owner's
call, not a 3 a.m. rewrite.

`fire::diffuse_heat` relaxes each cell toward the average of its four
neighbours using **its own** `heat_conductivity`, and nothing debits the
cell it took the heat from. Five separate ways that breaks conservation,
in rough order of how much they matter:

1. **Asymmetric conductivity.** Cell A's step uses `k_A`, cell B's uses
   `k_B`, computed independently. Water (0.08) pulls forty times harder off
   a lava cell than lava (0.002) pushes into it, and lava is never charged
   for the difference. `lava.ron` states this as *intended* — "it does NOT
   throttle how fast lava heats other things" — which is a fine statement
   about responsiveness and an accidental one about energy.
2. **Air is an infinite reservoir at ambient.** `Cell::EMPTY` reads
   `AMBIENT_TEMPERATURE` and is never written, so every empty neighbour
   donates and absorbs without limit.
3. **The minimum-progress nudge** (`here + raw_delta.signum()`) invents or
   destroys up to half a degree per cell per visit whenever the physical
   step rounds to nothing.
4. **`i16` rounding**, every visit, every cell.
5. **Sequential in-place writes**, so a neighbour visited later in the
   sweep sees the post-update value and sweep order changes the result.

What it costs, measured: `scene=simmer`'s hearth of 336 cells at 900°C
holds about 547 boils' worth of stored heat and boiled **1,941** cells —
3.5x its own inventory — while terminating perfectly happily, which is why
every existing guard was green. `fire::LATENT_HEAT_DEGREES` now charges
boiling to its source, which bounds the one consequence that was visible;
it does not fix the underlying non-conservation, and anything else that
reads temperature is still downstream of an amplifier.

There is no total-heat invariant, ledger or test anywhere in the tree. The
nearest thing is `a_finite_heat_inventory_stops_boiling_and_the_world_
sleeps`, which asserts termination and not a quantity — so it cannot catch
an energy budget change, and equally will not block one.
`boiling_stops_where_the_stored_heat_runs_out` is the first guard that
bounds a quantity, and it only covers boiling.

### 1c. A rigid body loses about a tenth of its cells when it lands

Pre-existing, unrelated to water, and found while fixing the *underwater*
case of the same code path.

`rigid::settle` writes a body's cells back into the grid: into the target
if it is empty, else into the nearest empty cell within `DISPLACE_SEARCH`
(4 rings), else **dropped**. A body that comes to rest overlapping the
floor — which rotation and the fractional origin make ordinary — loses
whatever part of it sits deeper than four cells.

Measured on a 40x2 stone raft dropped in plain air onto bedrock: **80 cells
in, 72 out**. Underwater it used to be far worse (9 out of 80, because a
submerged body has water in every cell and no empty cell within reach of
any of them); a swap arm now takes that to 64 and it is guarded at 20 lost.
The remaining ~10% is the general case and is untouched.

**A fix was written and withdrawn**, and the reason is worth having: a
last-resort walk straight up the column to the first empty cell made the
air case lossless (80/80) and cost `scene=ligament` **18.1 ms → 86.6 ms**
against a 60 ms bar, byte-identical failure counts either side, because the
ligament's one failure settles ~4,400 cells in a single frame and every one
of them paid a walk up the whole world. It also put stone in the sky over a
pond, because the first empty cell above a submerged body is above the
waterline — and `settle` scheduled its structural checks around where each
cell was *aimed*, so a cell relocated that far was never checked where it
actually landed and hung there forever. Any replacement has to be O(1) in
the common case and cost nothing on a scene with no liquid in it.

### 1d. A large lava lake never finishes solidifying

`filmstrip scene=lavalake` — a 21,492-cell walled basin open to the sky.

Before `rubble.ron`'s density was corrected, the lake could never finish at
all: broken crust floated on the melt, lidded the surface, and `froze`
flatlined at 5,224 from frame 6,000 onward while overload failures climbed
without bound (188 → 3,205 by frame 10,000). That much is fixed — the crust
founders and sinks and `froze` reaches 11,976 by frame 10,000.

It still does not *finish*. Run to 60,000 frames it stalls at **9,551
molten cells from frame 30,000**, with 12 of 40 chunks awake for the rest of
the run and a worst frame of 122 ms. A molten core sealed inside its own
crust has no path to lose heat, which is arguably right and is certainly
expensive: a large enough lava body is a permanent tax on the frame.

### 1h. ~~Falling rock grinds itself to powder in deep water~~ — **FIXED: the footprint is reserved and the fluid is exchanged, not searched for**

Reported twice — *"they don't look like chunks when they fall, they are
still mostly dust when they sink"*, then *"chunks of rock hit the water and
then start disintegrating into grit instead of tumbling down as rock
chunks."* Both are the same bug and it is closed.

| `scene=rockdrop`, a 600-cell slab | before | after |
|---|---|---|
| what is left of the slab | `rock -600, rubble +572` | **`rock -178, rubble +127`** |
| chunk mass minted on the way down | 2,515 cells | **885** |
| water ledger (`water + sky`) | 32,850.6 | 32,847.5 |
| worst frame | 26 ms | 39 ms |

2,515 cells out of a 600-cell slab was four passes over the same rock. It is
1.5 now, and 422 of the 600 are still stone when it stops.

The unit fixture is starker because nothing else is going on in it: a
160-cell raft sinking through `pond_world` arrives as **151 stone and 4
rubble**, against **0 stone and 160 rubble** on a worktree at the parent
commit.

**The fix, and it is the shape this section already named.** A rigid body
moving by an integer offset vacates exactly as many cells as it enters, so
the fluid in front can be *paired* with the space behind by construction
rather than searched for:

- A body takes its footprint as `reserved()` — materially empty and
  `FLAG_MANAGED` — so water can no longer pour into the space it is
  standing in. **Only a body with liquid under it**
  (`falling_towards_liquid`, within `LIQUID_LOOKAHEAD`), or one that meets
  liquid later; see the third cost below for why that gate is not optional.
- `exchange_with_fluid` replaces `make_way_behind`. It is not a search and
  cannot fail: `clear_or_displaceable` records the liquid rather than
  shoving it, and the exchange walks back along `-motion` to the cell the
  body is giving up in that column. The look the old walk was reaching for
  — what is in front ends up behind — is preserved, and there is no
  displacement-failure path left to stall a body into being re-broken.
- `restamp_footprint` moves the reservation with the body each substep, and
  `settle` releases it *before* writing the body back — which is what
  stops `nearest_free`/`surface_above` handing displaced water a footprint
  cell the same loop then overwrites. That was the 1,821-cell loss.

**Powder is deliberately untouched.** `displace`'s ring search still shoves
grains exactly as before: every dry scene in the engine is tuned against
that behaviour and the reported bug is water.

**Three things this cost, all of them found by looking rather than by
measuring:**

1. **920 cell-equivalents**, from `exchange_with_fluid` writing a swap into
   a vacating cell that already held water — `restamp_footprint` declines
   to stamp a cell that holds something, so a cell the body walked over
   without clearing is still in the footprint and still wet. Fixed by
   filtering `vacating` to materially-empty cells.
2. **Wedge-shaped air pockets standing permanently inside the pond**, from
   `rotate_quarter` moving the body and not its reservation. A reserved
   cell is not `is_empty`, so no water ever closes over it. **The ledger
   was perfectly balanced throughout** — nothing was lost, so no
   conservation guard could have caught it, and only the contact sheet
   showed it. `rotate_reserved` now carries the reservation through a turn.
3. **A 5.5x frame-cost regression on a scene with no water in it**, which
   is the one worth reading twice. Holding the footprint costs a dry scene
   nothing to *maintain* — and changes its outcome. With the space it
   stands in closed, a body stops shedding cells to collisions on landing,
   so more of it arrives intact, so the load model is handed a **bigger
   connected region** to judge, and its cost is superlinear in region size.
   On `scene=strike`: the same two failures went from 503 to 1,372 cells
   and the worst frame from **20 ms to 118 ms**, against a 60 ms budget.
   `PROBE_NO_LOAD=1` put 111 of those 118 ms in the load model, and
   `MAX_LOAD_CELLS_PER_FRAME` does not bound it — 20,000 and 40,000 measure
   the same 118 ms — so most of that work is not charged to the budget at
   all. **That is a real defect in the budget and it is still there**; see
   §1j.

   Gating the reservation on `falling_towards_liquid` sidesteps it and
   leaves every dry scene byte-identical (`strike` reads `rock +106, rubble
   +27` either side, at 19 ms), which is also the honest scope: in air the
   reservation keeps nothing out. **Gating it on *contact* instead is too
   late** and was measured before it was accepted: bodies shed cells into
   each other's unreserved footprints during the fall, and `rockdrop` kept
   242 of 600 rather than 422. Hence a lookahead rather than a touch.

   What remains on a water scene is bodies simply *living longer* — they
   now sink thirty rows instead of stalling after three frames — at 26 to
   39 ms on `rockdrop`. `set_owned` rather than `set` for the reservation
   writes took 35.5 to 31.3 ms by skipping the `demote_body_at` lookup
   `World::set` does on every overwrite of a managed cell.

The 3 cell-equivalent difference in the scene ledger is **not** the body
path: instrumenting `step_chunk_bodies` to print any frame in which
`water_equivalents` moved reported nothing at all across the whole run, and
the unit guard below holds a full sink to under one cell. It is the rest of
the frame reacting to a different world — 422 cells of the slab now stand
on the floor as rock rather than lying there as powder.

Guards: `a_slab_that_sinks_arrives_as_rock_rather_than_as_powder` (151
stone / 4 rubble here, 0 / 160 at the parent commit),
`a_body_sinking_through_a_pond_conserves_the_water_it_displaces` (ledger
**plus the bank**, or it measures evaporation — that mistake read as a
113.7-cell leak on a fixture with no body in it),
`a_body_leaves_no_reservation_behind_when_it_settles`, and
`a_piece_with_no_water_under_it_never_takes_a_footprint`, which is the
guard for cost 3 and has a paired positive so it cannot pass by the
mechanism being dead. Each red-checked against its own fix only.

### 1k. A splash droplet loses about 1% of a cell somewhere

Small, measured, cause not found. Worth writing down because the path now
fires constantly rather than once per boulder.

`scene=simmer`, paired against the identical run with
`fire::SIMMER_SPLASH_CHANCE` at zero, which holds the ledger at **exactly
4054.0** at every sample:

| droplets thrown | ledger | shortfall |
|---|---|---|
| 0 | 4054.0 | — |
| 59 | 4053.4 | 0.6 |
| 173 | 4052.3 | 1.7 |
| 248 | 4051.5 | 2.5 |
| 465 | 4049.4 | 4.6 |

That is **~0.01 cell-equivalents per droplet** — a constant fraction of
each droplet rather than a whole droplet lost one time in a hundred, which
is the shape that matters for guessing at it. It is stable: once the pan
cools and the droplets stop, the ledger stops moving.

Ruled out by measurement: `particle::land` dropping a particle for want of
anywhere to go (instrumented, **zero** occurrences over the whole run).
`throw_splashes` debits a full cell and `land` writes a full cell, so the
whole-cell accounting is right on its face.

Not chased further because at the shipped rate it is 0.04% of a pan over
4,000 frames and it stops with the heat. It would matter for a permanent
heat source under water — a lava vent under a lake — so measure it there
before assuming it is negligible in general.

### 1j. `MAX_LOAD_CELLS_PER_FRAME` does not bound the load model's frame cost

Found while fixing §1h, measured, not fixed.

The budget is decremented once per cell of `is_supported`'s BFS and nowhere
else, so `chain_reaches_anchor`'s walks, `subtree_sum`, and the repeated
`evaluate_within` along `failing_along_support_chain` are all free of it.
On a `scene=strike` variant that handed the model a 1,372-cell region, the
worst frame measured **118 ms at a budget of 20,000 and 118.7 ms at
40,000** — identical, so the cap was not what stopped it — while 8,000 gave
64.8 ms. `PROBE_NO_LOAD=1` puts 111 of the 118 ms inside the model.

So the constant reads like a frame-cost bound and is not one: it bounds
*one* of the walks. Either charge the other walks to the same budget (they
are memoized per frame, so the accounting is cheap) or rename it to what it
actually caps. Do not raise it as a fix for anything until that is settled —
raising it from 12,000 to 20,000 earlier this session bought nothing on this
scene, by the measurement above.

### 1i. The rigid-body rotation probe is vacuous, and a body can turn through a wall

Found while fixing §1h, not fixed with it — the fix changes how every
tumbling piece in the engine behaves and wants its own measurement.

`advance` guards a quarter-turn with *"only turn if the turned shape
actually fits. Otherwise a body wedged in a gap would rotate straight
through the wall beside it, which is the one way this transform can
cheat."* The guard never fires. It builds a rotated `probe` at the body's
own position and calls `blocked_axis(world, &probe, probe.x, probe.y, …)`,
which skips every cell whose target equals its current position — and with
`ox == probe.x.round()`, `tx == ox + cell.dx` *is* `cell_position(cell)`
for every cell. The loop body never runs and the probe always returns
`None`.

To fix it, `blocked_axis` needs the **pre-turn** footprint to compare
against rather than deriving one from a single integer offset;
`rotate_reserved` already computes exactly that pair of sets and is the
natural place to hang it. Expect it to change tumbling on every dry scene,
so measure `worked`, `ligament` and `strike` before and after.

### (was) 1h. Falling rock grinds itself to powder in deep water — three coupled defects

Reported from play: *"they don't look like chunks when they fall, they are
still mostly dust when they sink."* True, and every counter said otherwise.
**Diagnosed and measured in full; not fixed.** Read this before touching
`rigid.rs`'s liquid path.

`scene=rockdrop`, a 600-cell slab into an open pool:

| | |
|---|---|
| mass promoted as chunks, cumulative | **2,515 cells** |
| mass shattered to rubble | 424 |
| chunk share by mass | 85% |
| stone left at the end | **0** (`rock -600, rubble +572`) |

2,515 cells out of a 600-cell slab is **four passes over the same rock**.
The pieces are real and they are re-broken until nothing is left. That is
why "85% chunks" and "it's all dust" are both true, and why
`what came off:` had to be added — `size_buckets` measures the *region*
and peak-bodies counts *events*, and a player watches neither.

**The loop.** A body cannot displace deep water → it stalls → it is
re-rasterized into the grid → the load model judges it unsupported → it
fractures again, one rung smaller → repeat. Measured: only **2,834 of
10,849** displacement attempts succeed.

**Why displacement fails.** Printing a failing walk:

```text
WAYFAIL back=(0,-1) motion=(0,1) reach=11 trail=empty*,empty*,water,water,…
```

`*` marks a cell the body is about to re-occupy, and the third entry is
**water inside this body's own footprint**. A promoted body's cells are
written `Cell::EMPTY`, so the space it is standing in reads as free to the
CA sweep and to every other body: water and rubble pour into it, and with
two dozen bodies in flight they fill each other.

**What was tried, and why it is not in the tree.** `FLAG_MANAGED` is
exactly the reservation this needs — `Cell::is_empty` is managed-aware, so
one flag closes the footprint to the sweep, to `try_move` and to other
bodies at once, and `demote_body_at` is a no-op for it since `body_index`
only holds liquid bodies. Built, and it works: bodies stay whole and reach
the floor. **It also loses 1,821 cell-equivalents of water** on
`scene=rockdrop` (ledger 32,850 → 31,029), because `settle`'s relocation
targets (`nearest_free`, `surface_above`) hand back footprint cells the
same loop then overwrites with body material. Holding the reservation up
through the fill and releasing it cell by cell — plus `surface_above`
asking `is_empty` rather than the raw material test — narrows it and does
not close it (30,641). Reverted rather than shipped: trading "rock grinds
to dust" for "water vanishes" is not a trade.

**Three defects, and they have to be fixed together:**

1. A body's footprint is not reserved from anything.
2. `settle` can relocate a displaced occupant into a cell it is about to
   fill (this is §1c's ~10% landing loss, seen from the other side).
3. A body that stalls is fractured again rather than left alone or helped
   down, so (1) turns into a grinder rather than a pause.

The right shape for (1)+(2) is probably a body-level exchange: a rigid body
moving by an integer offset vacates exactly as many cells as it enters, so
the displaced fluid can be paired with the vacated cells by construction
instead of searched for. `make_way_behind` is a per-cell approximation of
that and cannot see the pairing.

### 1e-ter. ~~A boulder that never leaves the sky~~ — **FIXED; the fourth version of one predicate**

Reported from play as *"the boulder just stops and gets stuck in the middle
of the water"*, and it was worse than that: on `scene=rockdrop` the
600-cell slab **never fell at all**, still airborne at frame 400 with only
~100 cells ever promoted away.

`rests_on_ground` grants an anchor to any `Solid` with `Powder` below it.
Three versions have now tried to qualify that from the grain's own
neighbourhood:

1. `Powder => true`. A single swallowed grain floated a 90-cell raft on
   `scene=lavadrop`'s pond.
2. `grain_is_footing` as an **enclosure** test — body material on all four
   faces. Catches one grain; **two adjacent grains defeat it**, each being
   the other's non-body neighbour. This is what shipped and what the
   boulder was standing on.
3. Now: walk **down** the column of loose material and ask what is under
   all of it. Bedrock, out of bounds, attached material or a pile deeper
   than `GRAIN_FOOTING_PROBE` is a footing; unattached rock, air or liquid
   is not.

The first two are the same mistake and `CLAUDE.md` names it: *"which object
does this rule evaluate?"*. "Rests on loose ground" is a claim about a
**piece**, and a grain's neighbourhood cannot tell a grain the piece stands
on from a grain the piece has swallowed — they look identical up close.
Version 3 reads a different quantity instead of a better-tuned version of
the same one.

**Given up knowingly:** a slab on rubble on a *player-built* (unattached)
platform now reads as unsupported. Solid-on-solid is untouched; only a
granular layer sandwiched inside player structure loses. Reading the stored
`aux` would cover it and is circular here, since the distance under a
swallowed grain is 0 *because of this rule*.

**Two fixtures were wrong and passed anyway**, both putting a grain in
mid-air and asserting it bore weight. Both are corrected, and the two-grain
case is now asserted explicitly rather than left to follow from the
one-grain case.

### 1e-bis. ~~Slabs of rock hanging over a solidifying lava lake~~ — **FIXED, and the cause was the frame budget**

Kept because the *shape* of it will recur. `scene=lavalake` at frame 6,000
held **497 cells reaching no anchor, in 171 clusters, the largest a 96-cell
plate in open air** — plainly visible in the contact sheet, and three times
the pre-quench-crust baseline of 151 in 112 clusters, largest 8.

Not a verdict. `is_supported` answers "supported" from an unfinished search
by design, so a `MAX_LOAD_CELLS_PER_FRAME` emptied *inside*
`failing_region` produced a `None` the chain walk read as `Holds` — and
`Holds` is what retires a cell from the scheduler for good. The control was
one line: the same run with the budget at 2,000,000 read 164 in 112
clusters, largest 8.

Two fixes, and the split matters:

- `failing_along_support_chain` now checks the budget **after** each
  `failing_region` as well as before it, so a walk that spends the last of
  it defers instead of retiring the check. 497 → 422 on its own.
- `MAX_LOAD_CELLS_PER_FRAME` 12,000 → 20,000, which is where the plates
  actually stop: largest 75 / 26 / 2 / 12 at 12k / 16k / 20k / 24k. Costs
  `scene=worked` 9.13 → 11.64 ms and `scene=ligament` nothing at all.

**The totals are chaotic in the budget and the plate size is not** — 24k
reads *worse* than 20k on the total and better than 12k on the plate. Judge
this scene on the plate.

Tried and reverted: deferring a starved check to the next frame rather than
`STRUCTURAL_TICK_INTERVAL`. Right in principle, bought 422 → 379 with the
largest plate going the wrong way (75 → 99), and cost `scene=strike` 12.52 →
14.55 ms. The queue is bound by how much work a frame can do, not by when a
check is allowed to retry.

### 1e. One cell in a lava pour is still left hanging, and the route is unknown

`filmstrip scene=lavapour` settles at **one** stone cell at (303,250),
alone in open air, from frame 1,200 to the end of the run. Down from 31
(and `scene=lavadrop` from 23 to none), after the two causes below were
found and fixed:

- `region_has_free_face` read `EMPTY` and a lighter `Liquid` and said no to
  every `Gas`, so a quench cell in its own steam was recorded as a confined
  `Unsupported` failure — which the caller answers by leaving the cell
  standing and rescheduling nothing.
- `is_resting_on_ground` roots a chain on a `Powder` beneath, and the grain
  can leave without scheduling anything. See `GROUNDED_RECHECK_INTERVAL`.

The survivor is **the same shape and a third route**: `filmstrip`'s
`poke=303,250,1200` drops it on the next check, which proves it is a cell
nothing ever asked again rather than a cell asked and refused. What
scheduled — or failed to schedule — its last check is not known.

Worth chasing only if the count comes back up. One pixel in a 400-frame
pour is below what anyone can see, and the tools to find it are now in the
tree: the `hanging` census prints cluster positions and what each cluster
touches, `poke=` separates "never asked" from "asked and refused", and a
temporary `PP_TRACE=x,y` `eprintln!` in `structural::tick` (what found both
causes above) prints every tick, verdict and confinement decision for one
cell.

### 1f. A pond with rock in it never stops shuffling fill

`filmstrip scene=lavadrop` used to settle to **0 of 40 chunks awake** by
frame 160 and stay there. Since the quench crust started surviving as rock
rather than dissolving into rubble, it sits at **4-5 of 40 awake at frame
12,000** — and at frame 6,000 on `scene=lavapour` too.

Nothing is happening. Five consecutive frames at 12,000 report identical
material counts, identical phase-change totals, and a water total identical
to 0.1 of a cell. It is liquid moving fill between cells around the
submerged rock without changing anything, which is `CLAUDE.md`'s own
example of a cost buying nothing: *"a pool that is visually flat but still
shuffling fill for another quarter of an hour"*.

**Structural is ruled out by control**, not by argument: with
`structural::tick` stubbed to return immediately the pond is still awake
(6/40), and turning off `GROUNDED_RECHECK_INTERVAL` does not settle it
either. The awake set is the pond and its floor. Cost measured on the
scene's worst frame: none — 8.47 ms against 8.70 baseline on lavadrop, 7.96
against 8.37 on lavapour, minimum of three interleaved runs each. What it
costs is the dirty-rect render skip over ~12% of the screen, permanently,
after any lava-into-water event.

Likely the same root as §4 (levelling is O(width²)) meeting an obstacle
field. Nobody has looked at whether the fill differences are converging
slowly or oscillating; that is the first thing to measure.

### 1g. `scene=lavapour` leaves one 3-cell raft that a poke does not drop

Eight lone hanging cells and one 3-cell group afloat at frame 6,000, up
from one lone cell before the quench-crust change and against 40 hanging on
`scene=lavalake` before it (now 0).

Worth a note because it is **a different shape from §1e**: `poke=305,247`
schedules the check and the group stays, so this one is asked and refused,
not never asked. The lead is its `aux` — 1903, a finite anchor distance for
a three-cell group that touches nothing but air and water, so something is
handing it a support chain that cannot exist.

The `afloat:` census in `filmstrip` is what to watch: unlike `hanging:` it
consults no support rule, so it still sees a piece the model has convinced
itself is fine.

### 2. Sand-into-water displacement

Unchanged from the previous handoff and still the design gap it was.
`abffff2` is **kept** — the decision was made explicitly with numbers:

| metric | before `abffff2` | now |
|---|---|---|
| water rise | **29 rows/frame** | **1 row/frame** |
| sand/water/sand stripes | 41 | **1379** |
| sand cells with air beneath | 86 | 115 |

Water crossing 29 rows in one frame is a gross physics violation; the
striping it traded for is ugly. **Option 1 from the old list (sideways-
preferring displacement) was implemented as a mass-conserving 3-cycle and
measured: it does nothing** — stripes 1379 → 1370, stall unchanged, and it
*regressed* water rise to 2 rows/frame. Reverted, not committed. It cannot
work as specified: inside a pool there is no free-or-lighter cell beside the
mover, so the sideways path only opens where the blob is already at a free
surface, which is where striping was never the problem.

The striping follows from two individually-correct premises — displaced
material moves at most one row per frame, and displacement is a straight
vertical swap — so no local `try_move` tweak can remove it. Remaining
options: let an unsupported refused mover fall (fixes the 115 floating cells
only), move a coherent body *as a body* (`rigid.rs` — the only thing that
removes the premise), or accept it.

### 3. Scheduler under-enforces `max_active_tips` — **FIXED, and the tripwire earned its keep**

**Resolution (2026-08-17):** the tripwire fired exactly as this section
predicted it would — the session multiplicative crowding stopped crowded
tips from dying, simultaneous tips finally approached the cap, and the
under-enforcement measured 19 against 14. Fixed by the route this section
also predicted: `organism_active_tip_count` counts the organism's own
cell list (Decision 2's sidecar, maintained at the `World::set` seam under
both drivers) instead of scanning the schedule heap, so in-flight
dispatch is no longer invisible. That took the overshoot to 16, and the
remainder was a second gate nobody had needed before: `break_buds`
creates frontier too and never checked the cap — `supportable` is now
throttled by `max_active_tips`, one gate for both creators. The tripwire
test asserts the cap holds through 8,000 frames and passes.

The original finding, kept because its reasoning about *why it could not
bite yet* was correct and is the reason the tripwire existed at all:

### (was) Scheduler under-enforces `max_active_tips` (a tree bug) — measured, and it cannot bite yet

Review finding. `scheduler::step` pops the entire due batch into `due_sites`
*before* dispatching any of it, so `world.active_sites` does not hold the
batch while `plant::tick` runs. `organism_active_tip_count` counts only the
heap, so it cannot see any tip in the current batch — and when a tree's tips
all come due on the same frame, which is the normal case, the count it
returns is far too low and `Behavior::Grow`'s cap (`max_active_tips`, 14 and
10 in `tree.ron`) is under-enforced. **The reading is correct.**

**Now reproduced properly, and the answer is that the cap is unreachable.**
The previous attempt "grew no tips at all (`plant_tree` on a soil floor with
no field step)" — germination is light-gated, so a run that never steps the
field never germinates and can only ever report zero. With fields stepped
(`plant.rs`'s `a_trees_simultaneous_tip_count_stays_within_its_species_cap`,
8,000 frames), the **peak simultaneous `GrowingTip` count for one tree is
1**.

Not "under the cap" — one. Tip retirement converts a `GrowingTip` to
`MatureBody` in the same tick it grows, with the child carrying the frontier
forward, so a lineage holds exactly one live tip and branching only briefly
makes it two. `max_active_tips: 14` was sized for the pre-retirement system
where tips persisted; against the current one it has nothing to do.

So the bug is **real as read and unreachable as built**: a cap that is never
approached cannot be exceeded, however badly it is checked. Deliberately
*not* fixed on that basis — the fix (dispatch-one-at-a-time, which changes
the cap's meaning and risks a tip producing a due-now tip in the same frame;
or making the in-flight batch visible to the count) costs more than the
defect currently does.

**What changes that:** `Reports/plant-substrate-v2-design.md`'s bud break
(retrofit step 9) exists specifically to let a mature tree open new
frontiers, and is the first thing that would push simultaneous tips toward
the cap. The reproduction above is kept as a tripwire and should start doing
real work exactly then. Decision 2's sidecar also fixes it structurally for
free — `organism_active_tip_count` becomes a count over the organism's own
cell list rather than a scan of the schedule (design doc §3e), which has no
in-flight-batch blind spot at all.

### 4. Levelling is O(width²)

Not a bug so much as a known cost, quantified here because the previous
handoff's numbers were read before convergence and were wrong:

| frame | 1024-wide pool tilt | wall clock |
|---|---|---|
| 8,000 | 29 cells | 2¼ min |
| 40,000 | 3 cells | 11 min |
| 70,000 | 1 cell, asleep | 19 min |

It **does** converge flat and **does** sleep — there is no limit cycle, and
the earlier "residual tilt" figures were mid-convergence readings. A 512
world (the sandbox's own width) is ~4x faster: near-flat around 2 minutes.
The real cost is CPU, not appearance: the visible defect is gone early and
what persists is chunks awake doing invisible fill shuffling.

This is what the heightfield bodies exist to fix (O(width) instead), and
they are blocked on the promotion gap below.

### 4b. ~~A cell alone in the air drops its column's skyline~~ — **CLOSED, by removing the inference entirely**

Logged and closed in the same session. It was the tail of "shade under a
tree is way too intense": the skyline was the topmost non-empty cell, so
anything in the air above a column made everything below it draw as the
inside of a cave.

Fixed by not inferring it. `World::sky_surface` records the top of the
ground once, on the world's first frame, and nothing revises it —
`Reports/underground-definition.md` has the reasoning and the numbers.

**What is worth carrying forward is why every inferred version failed**, and
it is a case of `CLAUDE.md`'s "when a rule must tell apart two things that
can look identical, state the difference as data". Four shapes have to be
distinguished — a hill, a shaft someone dug, a roof someone built, and a
grain in mid-air — and from the world as it stands they are the same
arrangement of cells. Measured on the last inferred version, which took the
topmost cell and then repaired any column with higher ground within six
either side:

| shape | verdict |
|---|---|
| one floating cell | 20 rows of cave under it |
| plank 1 to 51 wide | identical to the floating cell |
| shaft ≤ 12 wide | tunnel (correct) |
| shaft ≥ 13 wide | open daylight 35 rows into the mountain |

No reach setting fixes that: the repair rule had a width threshold in one
direction and no rule at all in the other, and mining is the activity that
walks a shape across exactly that threshold. The difference between "I dug
this" and "this is a hill" is *history*, not geometry, and history has to be
stored.

### 5. Automatic promotion — blocker removed, still not ready

`promote_liquid_body` is called **only from tests**, so `liquid.rs` — the
pipe solver, the seam, ~1000 lines — never runs in play and every bug in it
is latent.

**The documented blocker is now fixed.** `127e177` reverted automatic
promotion because "the persistent-flux solver has no mechanism to drive an
internally-level body to expand into open floor space beside it", and
`edge_with_room` is that mechanism (`95c917f`, `68371d7`). A promoted body
that can still spill no longer sleeps through it and sheds its edge column
back to the CA, which is what §6c always said outflow should be.

**But promotion is still not worth turning on**, measured on the exact
scene the revert names — the 100-column block from
`a_wide_deep_water_column_levels_out_instead_of_only_eroding_at_the_edges`,
promoted deliberately at frame 0:

| | spread at 6000 |
|---|---|
| before the fix | **106, frozen from frame 10** |
| after the fix | 57–68, still moving |
| no promotion at all (plain CA) | **128** |

So the freeze is genuinely gone — the body sheds steadily, 100 columns and
4.9M fill down to 50 and 2.45M — and it still ends up *worse* than leaving
the water to the CA. Shedding one column per `DEMOTE_COOLDOWN_FRAMES` is
simply slower than the CA spreading it directly.

### 6. The heightfield does not deliver the speed it was built for

**Measured, and it inverts the premise the whole subsystem rests on.**
Report r2 §5's argument for the heightfield is a *speed* one — "levels a
pool in **O(width)** rather than the current O(width²)". Levelling time to
the 2% flatness bar, on a walled basin with water spanning every column
(the shape most favourable to the body — it never has to spread, only
redistribute):

| columns | CA | promoted body | ratio |
|---|---|---|---|
| 50 | 77 | 204 | 2.6x slower |
| 100 | 307 | 742 | 2.4x |
| 200 | 1,323 | 2,421 | 1.8x |
| 400 | 5,659 | 6,864 | 1.2x |

The CA quadruples per doubling — O(width²), as documented. **So does the
body** (3.6x, 3.3x, 2.8x per doubling). It is not O(width). The ratio is
closing, so a crossover presumably exists somewhere past 400 columns, but
the sandbox's world is 512 wide and the heightfield never wins on speed
inside it.

The persistent-flux solver was supposed to avoid exactly this — §7a's
"flux must be persistent state, **or you have rebuilt diffusion**". The
measurement says diffusion is what it behaves like. Whether the flux is
not persisting, or a clamp is throttling the wave, is unknown and is the
thing to look at first.

**What the body does measurably win at is accuracy, not speed**: it
finishes at a flatness of **1** where the CA leaves **11**, because
`terminal_snap` solves the exact analytic equilibrium. That is a real
property and worth something — it is just not the property the subsystem
was justified by.

Before spending anything more here, settle what the heightfield is *for*.
If the answer is exactness, it is much cheaper to reach that another way.
If it is speed, the flux solver needs diagnosing against §7a first, and
nothing downstream (promotion criteria, the trigger, the deferred B-8/B-2/
B-6/B-7 bars) is worth building until it delivers.

Two bugs found while measuring this are fixed: a body shed down to one
column stranded its fill instead of handing it back (`94a0c12`), and
`edge_with_room` always picked the left edge, so a body spread in one
direction only (`68371d7`).

The promotion *criteria* question — promote only once contained, since §4a
already argues quiescence is the wrong gate — is now moot until the above
is settled.

---

## Closed this session

- **Chunk-seam cliffs** (powders) and **terracing** (liquids), both from the
  chunk-by-chunk sweep order. `FLAG_UNDERCUT`. The previous handoff's
  leading hypothesis (seam cells never getting `flowing()`) was **measured
  false**.
- **Dark lines on horizontal chunk seams.** Fixed by sweeping chunk rows
  bottom-first (`pass_key`) rather than by penalising the crossing cell —
  two attempts at the latter were reverted, because they replace the tear
  with a *throttle* at the same seam (2236 and 1948 summed row-fill deficit
  against 988 for correct ordering).
- **Chunks awake but never swept.** `is_settled` now answers from
  `sweep_region`.
- **Four of five review findings**: liquids scanning through a promoted
  body's cells; explosions spawning debris made of `material::EMPTY`;
  `try_extend` freezing CA water it did not claim; `absorb_liquid`
  destroying fill at a body's edge. The fifth is §3 above.

`particle::step`'s landing check was flagged by the same review and
**deliberately left alone** — the reasoning is recorded in place.

---

## Awaiting a decision

Five `GrainMode` variants are prototyped behind a runtime switch, default
unchanged, with GIFs generated for comparison (`examples/filmstrip.rs`,
`grain=`). They address the report that a pool reads as *static* in the
middle while its edges move — the grain is keyed on world position, so water
flows through a pattern nailed to the screen.

Worth knowing before choosing: a settled pool changes 431 cells per step
with **zero occupancy changes**. Its interior genuinely does not move. So
`Cell` grain makes moving water *read* as moving, which it currently does
not, but nothing can animate an interior that is standing still — `Muted`
and `Animated` are the variants aimed at that half, and `Animated` is the
only one that costs the dirty-rect render skip.

---

## ~~Open~~ **CLOSED** — the three the polarity review raised (M18 plant v2)

All three are now fixed, each with a guard verified to fail against the old
code. Kept here rather than deleted, because what they have in common is
worth more than any one of them: **all three were invisible to the suite
for the same reason — nothing tallies held water, and nothing walked the
frontier cell types.** A new test that covers either of those covers a
whole class.

| finding | fixed in | guard |
|---|---|---|
| allometry gate permanently retiring roots | `ab39721` | `a_root_tip_that_ages_out_retires_instead_of_becoming_a_phantom` |
| `Grow` into soil destroying stored water | (next commit) | `a_root_growing_into_soil_displaces_its_water_rather_than_destroying_it` |
| capillary exchange over-filling a neighbour | `13bce0a` | `capillary_flow_never_pushes_a_neighbour_past_its_own_capacity` |

Two of them turned out differently from the review's framing, and the
difference is recorded at each site:

- The root bug was **not** fixable by marking the "not now" gates as
  `found_candidate`, which is what the framing suggests. That breaks
  `a_tree_eventually_stops_growing` immediately — the staleness counter is
  the only thing that makes growth terminate. The real defect was that
  ageing out had no landing site for `RootTip`.
- The capillary bug needed a **second water-holding material to be
  testable at all**. With equal capacities the drier cell is by definition
  below its own limit, so the clamp can never bind. The guard writes a
  `tightsoil` into a temp dir and loads it additively.

The original descriptions follow, since the reproductions are still the
cheapest way back into each area.

### 1. `MAX_ROOT_FRACTION` feeds the staleness counter, permanently retiring roots

`plant.rs`'s allometry gate `continue`s without setting `found_candidate`,
so a *transient* root:shoot ratio counts as a failed tick. After
`STALE_LIMIT` blocked ticks the `RootTip` stops rescheduling — and
`organism_upkeep` skips frontier cell types, so nothing ever visits it
again. It loses `Absorb`/`Transpire` permanently while still counting
toward `root_cells`, which ratchets the very ratio that blocked it.

The gate is meant to say "not now", which is the "temporary shortfall"
framing `Divide`'s own resource gate uses — that path sets
`found_candidate` and this one does not. Suspect this first if roots look
like they stop drinking on a mature tree.

### 2. `Grow` into soil destroys the soil's stored water

Growing a root into a penetrable soil cell overwrites the cell wholesale,
replacing its `aux` — which for a `Powder` is moisture — with cell-type
bits. In the `forest` scene each root cell silently deletes
`SOIL_FIELD_CAPACITY` (620) units; a 100-cell root system loses roughly 62
water cells' worth. No conservation tally covers held water, which is why
nothing noticed.

Note this interacts with the still-open `water_capacity` item below: any
liquid-conservation test taught about held water will start failing here.

### 3. Capillary exchange can push a neighbour above its own capacity

`update.rs`'s capillary step bounds the transfer by *this* cell's
`water_capacity` and writes `there + moved` without checking the
neighbour's. Latent today because `water_capacity` is opt-in and only
`soil` has it, so every exchange is soil-to-soil with equal capacity. It
goes live the moment a second water-holding powder exists with a different
capacity — which is exactly what widening `water_capacity` to sand would
do.
