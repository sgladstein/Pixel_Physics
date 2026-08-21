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
