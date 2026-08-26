# Open bugs handoff

Rewritten at the end of the session that landed `15b2e51` … `ad1e227`.
Everything here was measured, not reasoned — where something is a guess it
says so, and where a plausible idea was measured and found wrong it is
recorded with its numbers so it is not tried twice.

Read `CLAUDE.md` first; it holds the method these bugs keep re-teaching.

---

<!-- BEGIN GENERATED INDEX -- regenerate with scripts/bugindex.py -->

**39 open, 78 bugs** (plus 18 landing-note items,
marked `note`). Generated from the headings by
`scripts/bugindex.py` -- a bug's verdict is written into its own heading, so
this is derived, never maintained by hand. Entries are never moved when they
close (the file is co-owned and reordering it conflicts with every open
branch), so **this table, not the `## Open` heading, is what says whether a
bug is live.** Jump by line number.

**Merge conflict in this block?** Do not hand-merge it. Take either side
whole, then run `python3 scripts/bugindex.py` -- the table is derived from
the headings, so the regenerated block is correct from either starting
point.

| § | Status | Line | What it is |
|---|---|---|---|
| 0-z | **OPEN** | 130 | Leaves are the only channel a plant has, and four separate "bugs" are all that one fact |
| 0-a | closed | 187 | Dark bands under overhangs, objects and open-cast digs (render) |
| C1 | **OPEN** | 251 | A forest-floor bank is a wall the gnome has no way over |
| D1 | closed | 288 | The brush and fire license nothing, so a burnt trunk leaves its crown in the air |
| D2 | **OPEN** | 445 | A room's collapse arrives at frame ~350 where it used to arrive at ~150 |
| D3 | **OPEN** | 468 | Near-surface blasts do not throw chunks into the air |
| D4 | **OPEN** | 501 | At a bounded reach a collapse can stop part way and leave a slab in open air |
| 0 | **OPEN** | 538 | Roofed water: ponds fills both sides of an overhang (worldgen) |
| 0b | closed | 556 | The deep massif reads as television static, and it is a per-cell palette dither (worldgen) |
| 0c | closed | 678 | Cave light is quantised to 8-cell squares (render) |
| 0d | **OPEN** | 752 | The organism support search asks the wrong question |
| 0e | closed | 794 | A decay site does not follow its cell |
| NEW | closed | 857 | Plants grow nothing on generated terrain |
| U | closed | 1017 | Water stress makes a tree BIGGER |
| V | closed | 1062 | A tree with no seedlings under it never stops growing |
| Z | decided | 1115 | The stand still reads as one mass |
| Z2 | closed | 1249 | A free particle drops Cell::aux, so a blast under-prices a corpse |
| Y | closed | 1367 | The gnome cannot get through the wood |
| X | decided | 1518 | A desert with no desert plants |
| -- | historic | 1567 | X (original). A desert with no desert plants |
| W | decided | 1644 | The water-cycle branch and this one are two halves of one mechanic |
| A | **OPEN** | 1718 | The slot-1 root spread has collapsed |
| B | closed | 1998 | anchor_support runs over creature organisms, unguarded |
| C | closed | 2085 | grass and creeper root branching is running a retired model |
| D | closed | 2134 | Two smaller things the merge exposed |
| E | closed | 2166 | A test scene can outlive the economy it was written for |
| F | **OPEN** | 2184 | Cross-line seams neither branch's tests exercise |
| P1 | **OPEN** | 2255 | The water book, the root-tip counter, and what they said about §A and §U |
| P3 | **OPEN** | 2542 | The generation loop |
| V3 | **OPEN** | 2738 | Die-back's shed tissue feeds a pile that grows up through the canopy |
| V2 | closed | 2808 | A tree cannot die of drought |
| P2 | **OPEN** | 2949 | The economy re-derivation |
| G | **OPEN** | 3164 | Grassfire arrives with a standing negative verdict |
| -- | historic | 3227 | G (original). Grassfire arrives with a standing negative verdict |
| 0f | closed | 3249 | A melting Powder manufactures water |
| 0g | closed | 3304 | scene=lavapour's pond simmers forever |
| 0h | **OPEN** | 3366 | Lens-stress at 2048x640 puts gravel and water in motion, with no cave anywhere (worldgen) |
| 0i | **OPEN** | 3410 | Terrace risers are inert: erosion deletes them at any nonzero world_age (worldgen) |
| 1 | closed | 3450 | Whiskers on a spreading front |
| 1l | **OPEN** | 3567 | Boiling never puts a bubble *in* the water |
| 1m | **OPEN** | 3631 | Damp-soil evaporation barely runs, and the humidity shadow that would switch it off is al... |
| 1b | **OPEN** | 3702 | diffuse_heat does not conserve heat, and a hot cell is an amplifier |
| 1c | **OPEN** | 3744 | A rigid body loses about a tenth of its cells when it lands |
| 1d | **OPEN** | 3773 | A large lava lake never finishes solidifying |
| 1h | closed | 3789 | Falling rock grinds itself to powder in deep water |
| 1k | **OPEN** | 3895 | A splash droplet loses about 1% of a cell somewhere |
| 1j | **OPEN** | 3927 | MAX_LOAD_CELLS_PER_FRAME does not bound the load model's frame cost |
| 1i | closed | 3959 | The rigid-body rotation probe is vacuous, and a body can turn through a wall |
| -- | historic | 3971 | (was) 1h. Falling rock grinds itself to powder in deep water |
| 1e-ter | closed | 4038 | A boulder that never leaves the sky |
| 1e-bis | closed | 4078 | Slabs of rock hanging over a solidifying lava lake |
| 1e | **OPEN** | 4111 | One cell in a lava pour is still left hanging, and the route is unknown |
| 1f | **OPEN** | 4138 | A pond with rock in it never stops shuffling fill |
| 1g | **OPEN** | 4165 | scene=lavapour leaves one 3-cell raft that a poke does not drop |
| 2 | **OPEN** | 4181 | Sand-into-water displacement |
| 3 | closed | 4208 | Scheduler under-enforces max_active_tips |
| -- | historic | 4226 | (was) Scheduler under-enforces max_active_tips (a tree bug) |
| 4 | **OPEN** | 4266 | Levelling is O(width²) |
| 4b | closed | 4286 | A cell alone in the air drops its column's skyline |
| 5 | **OPEN** | 4319 | Automatic promotion |
| 6 | **OPEN** | 4348 | The heightfield does not deliver the speed it was built for |
| H | closed | 4399 | ascii's ants moisture-gradient scene asserts a gradient the scene no longer has |
| H2 | closed | 4517 | The ascii colony has gone sessile |
| H3 | closed | 4579 | Both worldgen at-rest tests are red on main, and both are water |
| I | closed | 4702 | The disturbance-extent guard inverts once rubble stops anchoring |
| J | **OPEN** | 4767 | A blocked substep still vents the smoke it was only *probing* |
| Q | **OPEN** | 4797 | Settled debris stands in one-cell vertical needles that never topple |
| P | **OPEN** | 4852 | scene=worldcrack is not deterministic, so seedsweep.sh cannot compare two models on a cha... |
| K | closed | 4985 | try_step's rotation-fit probe compares every cell against itself |
| N | **OPEN** | 5067 | Decayed litter makes soil that does not match the soil around it, and roots will not ente... |
| O | **OPEN** | 5135 | Litter rots into soil that never leaves, so the floor rises all run |
| M | closed | 5192 | Two gating worldgen tests are red, and both are the same thing: generated water never com... |
| R | **OPEN** | 5402 | filmstrip scene=colony panics at its own default seed, and degrades badly at others |
| L | closed | 5481 | The colony has gone sessile: 98 round trips became 2 |
| R2 | **OPEN** | 5613 | An ant put down on open water stands on the surface for ever, and found_colony puts them ... |
| S | **OPEN** | 5675 | Every destructive verb but the brush leaves the structural scheduler pinned at its cap fo... |
| S2 | **OPEN** | 6382 | The brush's anchor rule destroys structures the other two rules leave standing |
| -- | closed | 6571 | The plant model bounds height and does not bound width FIXED |
| 1 | note | 6662 | MAX_ROOT_FRACTION feeds the staleness counter, permanently retiring roots |
| 2 | note | 6676 | Grow into soil destroys the soil's stored water |
| 3 | note | 6688 | Capillary exchange can push a neighbour above its own capacity |
| W1a | note | 6706 | creeper.ron's root tips still run the superseded in-tick branch path |
| W1b | note | 6727 | A material-counting guard cannot see a species |
| W1c | note | 6740 | generated_terrain_is_already_at_rest went red on main |
| T1a | note | 6874 | load::grain_is_footing reads *attachment* where it means *supported* |
| T1b | note | 6952 | The structural opt-out did not hold against bearing |
| T1d | note | 6963 | acceptance.sh's lavadrop sits close enough to its frame budget to flake, and is over it o... |
| T1e | note | 6997 | "The pieces hit the ground and turn to dust" was not settle, and the measurement says so |
| T1f | note | 7051 | The felled pile is 74% powder because the tree is 56% leaves. The piece ladder cannot fix... |
| T1g | note | 7105 | A "refixed" claim went out over a settled state that had barely moved |
| T1c | note | 7134 | §1c's settle loss is now a counter |
| -- | note | 7151 | What landed |
| -- | note | 7174 | Do not re-derive these |
| -- | note | 7202 | Measurements that contradict something written |
| -- | note | 7222 | Open |
| -- | note | 7257 | Unmerged at close, and one of it is a fix main needs anyway |

<!-- END GENERATED INDEX -->

## Open

### 0-z. **Leaves are the only channel a plant has, and four separate "bugs" are all that one fact** — OPEN, structural

Filed 2026-08-24 by the plant-program integrator. **Nothing here is a new
measurement.** Every number below was reported by a different lane, each as an
unrelated defect in its own area, and none of those lanes could see the others.
Assembled, they are one design property with four symptoms — which is why
fixing them one at a time has not worked and will not.

**The property.** A plant's leaves are the sole interface between it and every
system that acts on it. Economy, water, mortality and visible mass all read
foliage and nothing else. So a plant that loses its leaves does not become a
struggling plant; it drops out of the simulation's reach almost entirely, while
still standing there.

| the symptom, as reported | the lane that found it |
|---|---|
| Die-back is inert on a compact stump — a tree that cannot pay its bills loses nothing, because the only consequence of a deficit is shedding exposed tissue and a stump has none. | P2 §7 |
| Transpirational demand is summed over `Leaf \| GrowingTip` **only**, so `settle_water` returns desiccation through its `else { 0.0 }` branch at zero demand, and `drought_death` — a parameter on `Photosynthesize` — is unreachable. **A foliage-free tree is immune to drought.** | integrator, chasing the owner's pushback |
| A felled tree's pile reads as dust because `leaf` is **1,660 of 2,940 cells — 56%** — and every one becomes a `Powder`. 91%+ of *woody* mass promotes to pieces and it is not enough. | T1, filed as T1f |
| Grass has **no leaf stage at all**, and P2's superlinear maintenance made each grass plant ~20% smaller with plant *counts* unchanged within ±1. | W3, re-measured across #40 |

**The owner's judgement, which is what makes this a bug rather than a
curiosity** (2026-08-24): *"but economics should be able to call us tree death
right. if a tree doesn't get watered, it will eventually die."* Today it does
not, and the escape route is perverse: **a tree escapes dying of thirst by
starving first.** Losing its leaves is what makes it safe.

**Why the per-symptom fixes failed.** Each lane correctly diagnosed its own
half and reached for the lever in front of it — tune die-back, tune the
fragment ladder, tune maintenance. All four levers read foliage, so none of
them can act on the state that has none. This is the shape `CLAUDE.md` calls
*two fixes failing the same way means the approach is wrong, not the tuning*,
seen across four lanes instead of two attempts.

**Ranked fixes** (the integrator's, given to P2 2026-08-24; P2 was building #1
when this was filed — check whether it landed before starting):

1. **Sustained unpayable deficit kills outright.** A counter of consecutive
   ticks with an unpayable deficit; past a threshold the organism dies. Closes
   the economy and water holes together, needs no new channel, and is what the
   owner actually asked for.
2. **Non-foliage maintenance demand** — wood and roots draw a trickle, so
   demand never reaches zero and desiccation means something on a bare stump.
   Does not on its own guarantee death; it is what makes #1's threshold
   defensible.
3. **A wood/root cavitation path.** Most physical, most work, a later lever.

Separately, and not fixed by any of the above: **T1f's 56% is a *visual*
symptom of the same property** and wants a dead-leaf tier that is not a
`Powder`, or a less leaf-heavy species. Do not expect #1–#3 to change what a
felled tree looks like.

**The generalisable lesson, for whoever reads this next.** Each lane's finding
was correct, well-measured, and useless alone. When several packages report
unrelated inertness in the same subsystem, **ask what quantity they all read**
before tuning any of them — the shared input is the bug.

### 0-a. Dark bands under overhangs, objects and open-cast digs (render) — **CLOSED, all three**

Reported from play as *"dark bands under any overhangs or objects or when
I'm mining"*, with the guess that it is either the frozen background
baseline or a lighting shadow. It is the baseline. Full measurement and the
options in `Reports/dark-bands-diagnosis.md`; the short form:

`World::sky_surface` asks *"is there anything `Solid` or `Powder` above me
**in this column**, as of frame one"*, which cannot tell a cave roof from a
cliff brow, a hillside from a rock suspended in mid-air at genesis, or rock
you removed from rock that was never there. `background_at` then fades that
air to `UNDERGROUND` over 24 rows and saturates.

Measured with `examples/underground_probe.rs` (open air that is
flood-reachable from the sky yet answers `!is_outdoors`): **156–408 cells
per 2048x640 world** across seeds 1–6, in 20–50 cell patches on cliff
shoulders — small, and each one a hard-edged patch of darker sky. A 64-wide
open-cast pit takes it to **1,363 cells, 436 of them at full `UNDERGROUND`**,
in one 1,207-cell region.

Ruled out by measurement: the depth grade (`light=flat` leaves the pit
exactly as black — all of it is the empty-cell cave fade); the skyline going
stale as the world settles (156 cells at 1, 60, 600 and 3,000 frames, while
the open-air denominator did move, so the null is real).

The `water` board's *"dark vertical band through the pond"* card
(`20260822T225340455Z-ad69f8`) is the same bug seen through the other
consumer: `scene=rockdrop` reproduces it at **frame 0 with zero bodies in
flight**, because the slab is present when the surface freezes.

**Fixed for the overhang and object cases** by storing the genesis void per
*cell* instead of per column (`World::freeze_underground_map`) — which is
`dead-ends.md` §977's *"revisit only by storing more history, never by
inferring"*, not a return to inference. Rescues 149/156, 406/408 and 192/197
of the false-cave cells on seeds 1–3; the remainder were `Solid` or `Powder`
at genesis and are air now, so they stay dark by the same rule that keeps a
dug shaft a tunnel. Costs +0.3–0.7 ms on a ~11.5 ms full redraw, measured
interleaved against a worktree at the parent commit.

**The open-cast dig is fixed too**, by propagation rather than a better
boolean — sky light seeded only where a cell was outdoors at genesis and
spread at Terraria's 0.91 per air cell / 0.56 per solid over a 4-cell block
grid, on `F12` with /4 the default. A pit is bright at its rim and dark at
the floor; a shaft still goes dark at any width, because the seeding refuses
it, not because of any threshold. `Reports/sky-light-design.md` has the
measurements, including why `field.rs`'s own light channel could *not* drive
it (it hands a block-aligned 8-wide shaft full daylight 100 cells down) and
why a stored per-pixel field was tested and rejected.

**The second residual is fixed as well**: rock under an overhang was
over-darkened because the depth came from the per-column skyline, which a
brow sets. `World::ground_datum` — the top of the lowest run of cells the sky
cannot reach — replaces it as the shading datum.

**One thing changed underneath all of it:** the terrain depth grade is **off
by default** now, on a playtest (*"no question grade off is better"*). So the
`ground_datum` fix renders nothing unless someone presses `F10`. It is still
correct and still guarded, with the guard forcing the mode explicitly so it
cannot pass vacuously.

**The `D` entries are the destruction/blasting group**, from the explosion-in-
stone branch. Numbered apart because `0`, `0b`, `0c` and `0d` below are
worldgen's and were here first.

### C1. A forest-floor bank is a wall the gnome has no way over

Found while fixing the scattered-grain half of the same symptom (that half
is fixed; see `Tuning::shoulder_grains`). At the worst of six `scene=wood`
start windows the gnome stops for good at x=59 with `grounded=true`,
`wading=true`, `lift_limit=4` and **no `Footing::Hard` cell anywhere in the
rect he is trying to enter** — the blocker is loose soil in the forest
floor, five cells abreast at chest height.

That is the wade model meeting terrain it cannot express a way over.
`wade_rows` lets powder reach the knee and no higher, and `step_up` mounts
a *ledge* — it asks `rect_free` at a lifted position, which a tall powder
face fails at every lift. So he can neither wade through a bank nor climb
onto it, and a forest floor that piles above knee height is terminal
wherever it spans his width.

**Measured, so the gap is visible rather than argued.** Cells travelled in
the 600 ticks after he sets off, over six start frames, at the shipped
`shoulder_grains: 4`: 357, 50, 161, 358, 264, 134. Acceptance case 8 gates
200 at `frame0=0` (357, green); case 8b gates 40 at the worst window
(`frame0=3600`, 50), and the gap between 40 and 200 is this bug.

Not attempted, and each wants measuring before it is believed:

- **Let him mount powder.** Treat a powder surface as steppable, so
  step-up can climb a bank the way it climbs rubble. Closest to what a
  player expects; the risk is that it also lets him walk up the face of a
  drift, which is the thing `wade_rows` exists to stop.
- **Displace the grains.** He has `displace_disc` for digging already, but
  `player::step` is documented as reading the grid and never writing it —
  the ghost contract — so this is an architectural change, not a tweak.
- **Ask whether the floor should pile this deep at all.** The banks are new
  with main's litter and forest-floor work; the model may be right and the
  world wrong. Cheapest to check first.

---

### D1. The brush and fire license nothing, so a burnt trunk leaves its crown in the air — **FIXED**

`World::record_disturbance` has exactly three production callers —
`rigid::mine_swept`, `rigid::strike` and `explosion.rs`. The paint brush
(`World::paint_*`) and fire burnout record nothing. Since `39d0978` the
organism support path is gated on `within_disturbance`, so at LOCAL, TIGHT
and NONE, erasing or burning out a trunk leaves the crown standing as living
wood. **Rock has had the identical hole for longer** — erasing a pillar with
the brush at LOCAL leaves the roof floating — so this is a pre-existing gap
that `39d0978` newly exposed for a second material class, not a regression.

Also worth knowing: `rigid::strike` and `rigid::mine_swept` both `continue`
on `organism_id() != 0`, so the pick and the chisel cannot damage a tree at
all. **The explosion is the only tree-damaging verb that records a
disturbance**, which means LOCAL and TIGHT degenerate to NONE for vegetation
under every other verb.

*Fix shape:* give the brush and the fire burnout a `record_disturbance` with
an extent. That repairs rock's brush inertness at the same time. Deliberately
not done on the explosion branch — it is a change to two unrelated verbs.

**Done, on the branch that made `TIGHT` the default `chain_reach`** — which
is what forced it: with SPREAD default, `within_disturbance` returns `true`
on its first line and none of this was reachable, so the gap could sit here
indefinitely. Three verbs now record, not two:

- `World::paint_capsule`, per structural cell it writes, extent 0.
- `fire.rs`'s burnout, at the `was_structural` fan-out, extent 0.
- `fire.rs`'s `transform`, wherever a phase change crosses the structural
  boundary — the case neither this entry nor its fix shape named. Lava
  quenching to crust over open water mints a solid nothing has touched, and
  under a leash it minted and then never came apart.

The last two run inside the sweep with no `&mut World`, so this needed a
`CellSurface::record_disturbance` that `ChunkView` queues and `run_pass`
replays, the same shape as `schedule_active_site`.

Two sizing consequences, both from this entry's own premise that the world
is not a player: a burning wood writes a disturbance per burnt-out cell, so
`record_disturbance` now **coalesces spatially** at `chain_reach / 2`
(widening the kept record's extent to the larger of the two), and
`MAX_DISTURBANCES` is 16 -> 64. Without that, a fire evicts the player's own
dig within a frame and the licence tracks whatever burned most recently —
destroying exactly the delayed cave-in `chain_window`'s ten seconds exist
for.

**Still open from this entry, and untouched:** `rigid::strike` and
`rigid::mine_swept` still `continue` on `organism_id() != 0`, so the pick
and the chisel cannot damage a tree at all, and the explosion remains the
only tree-damaging verb. That half is a change to the dig verbs, not to
what records a disturbance.

**The dig-verb half, LANDED 2026-08-23, lane S package S1
(`claude/s1-felling-instrument`)** — the "still open, untouched" paragraph
directly above is what this closes. Written after that branch merged the
playtest-defaults line; the two were built in parallel and the licence half
above is that line's, not this one's.

*What was actually wrong, and it is not what the top of this entry says.*
The `organism_id() != 0` tests this entry names were **not load-bearing**.
Removing them changed nothing at all, measured: four `strike` blows across a
26-cell bole took **0 cells** and left every counter at zero.
`rigid::is_body_material` — the predicate one line earlier in the same
condition — is `MaterialKind::Solid` alone, and `wood` is `Plant`, so no
organism cell ever reached the organism test. Two gates, one visible in this
report and one not, and only the invisible one mattered. `CLAUDE.md`'s "a
change that moves *nothing* is different evidence from one that moves a
little", read the right way round.

The fix is `rigid::is_tool_target`, a second predicate (`Solid | Plant`,
still excluding bedrock) used by `strike` and `mine_swept` only.
`is_body_material` keeps its meaning for `label_component` and
`trace_contours`, which answer "what piece of *rock* is this" for the M8 body
pipeline — widening it there would change what a component is on every scene
in the engine to fix two verbs. Guards: `rigid.rs`'s `tool_target_tests`,
confirmed to fail against the pre-fix predicate.

*Measured, `scene=fell fell=6000` (new instrument, at SPREAD):* six bites
sever the bole, the axe itself takes 134 cells of living tissue and throws 6
bodies (67 cells), then `plant::anchor_support` declares the crown unreached
and **2,360 cells** are severed by the support check. Standing living tissue
2,906 → 409 (roots and stump). Both drivers agree: 2,360 parallel, 2,363
serial. Before the branch the identical cut left the crown standing and
*growing* — 2,823 → 2,911 over the next 210 frames, with every counter in
`FailureCounts` at zero. **These numbers predate `TIGHT` becoming the
default and must be re-read at the shipped setting** before anything is
concluded from them.

*New instrument.* `filmstrip scene=fell` (one tree, fixed trunk x, room to
fall), `fell=frame[,radius[,force]]` (chop through the subject's own thinnest
bole row, wherever it is — seed- and age-independent, and the knob lane P's
resprout work wants), `chop=x,y,r,force,frame` (a hand-aimed `strike`),
`min_severed=N` (the acceptance bar), and a three-line felling census under
every tile: standing tissue split shoot/root, where the bole is and what a
cut through it costs, detached-and-still-standing cells, the furthest finite
support distance, deadwood and litter, and how many body cells are plant
material. `FailureCounts::severed_organism_cells` is the new "did it fire"
counter — nothing else in that struct moves when a crown comes down, so
`min_failing_cells` reads zero through a run that dismantles a whole tree.
`scripts/acceptance.sh`'s `fell` case gates it.

**The owner's verdict on the result, and it redirects the line.** The GIF
went out as review card `20260823T092247531Z-a33d82` (board `felling`);
the answer was *"It reads as a tree disintegrating into dust. I am wondering
if we should take a step back and plan something more ambitious. Eventually I
would want trees to be physical in the world, be able to sway in the wind,
have branches break off if a rock falls on it. We need a more real physical
and partially rigid modeling."*

So **D3 as scoped (fix the fragment ladder) is on hold** pending a design
round on partially-rigid trees. What the instrument found that bears on that
design, recorded here because it is measurement and not opinion:

1. **The engine has two representations of matter and neither is partially
   rigid** — a cell welded to the grid (infinitely stiff, no pose) or a
   `ChunkBody` (free, no attachment, no hinge). `BodyCell` is
   `{dx, dy, material, shade}`; identity is lost at promotion. The dust is
   that gap, not a separate defect: the only available transition is
   welded → gone, and `break_free` takes it one cell at a time.
2. **A skeleton is already computed every organism tick and nothing reads it
   as pose** — `plant::anchor_support` (Dijkstra from the root anchors) and
   `plant::accumulate_support` (basipetal parent ordering). Both answer only
   yes/no support questions.
3. **`ChunkBody` cannot express a hinge, and it is a redesign not a
   constant** — `spin` accrues from *speed*, so a just-cut trunk has none;
   rotation is quarter-turn snaps gated on the turned shape fitting.
   `felling-blockers.md` §2 said this before the instrument existed and the
   instrument confirms it.
4. **Half of "a rock lands on a branch" already exists** —
   `structural::supported_load` already counts material resting on organism
   tissue and shortens the allowable span. What is missing is that the
   failure emits powder instead of a limb.

**Also left:** `rigid::loosen_shell` still declines organism cells (the third
of the three skips at the top of this entry), so a blast rim throws no wood.
Left deliberately — it is the same promote-an-organism-cell decision the
design round owns.

*Measured across `F9` after merging the playtest-defaults line, and it took a
harness bug out with it.* `scene=fell fell=6000`, cells severed by the
support check: **SPREAD 2,360 / LOCAL 2,333 / TIGHT 1,108 / NONE 0** (standing
tissue 407 / 445 / 1,712 / 2,836). At TIGHT half the crown comes down and the
top stays in the air; at NONE the cut trunk holds its whole canopy. Both are
the leash doing what it says, and both are now in `wiki/plants.md`.

The first attempt at that table read **byte-identical at all three settings**,
which is `CLAUDE.md`'s own tell that a knob was never connected — and it was
not. `filmstrip`'s `build()` applied `chain_reach=`, `confine=`, `arch=`,
`share=`, `joints=` and `bands=` to a world that **five scenes then throw
away**: `grove`, `wood`, `climb`, `shake` and `fell` all construct through
`common::PlantScene` and `return` its world. Every one of those knobs was
silently inert on all five. Fixed by splitting `build_scene` out and
re-applying the settings to the world that is actually returned — idempotent
for the scenes that already worked. **Lane P and lane W should know**: any
`grove`/`wood`/`climb`/`shake` measurement that varied one of those six
arguments before this commit varied nothing.

### D2. A room's collapse arrives at frame ~350 where it used to arrive at ~150

`c089aa2` reshaped what a failing region is (boundary erosion, fragments
separating along fissures). On `scene=room wall=5 dig=3` the ceiling's
collapse merged from **thirty-seven separate failures into one** paced
failure of 1,903 cells. Measured against `origin/main`, roofed void as a
percentage of what was there at the cut:

```text
  frame        2     200     400     800
  main      100%     20%     22%     22%
  branch    100%     24%     18%     18%
```

The roof comes down on both, and slightly further on the branch by frame 400
— so the outcome is equivalent or better and `acceptance.sh`'s `roomcut` bar
was moved from an event count to `max_cave=40` accordingly. **What is not
settled is the timing.** The owner has separately complained about breakage
arriving late, and this is a collapse taking a bit over twice as long to
arrive. It needs a playtest verdict, not another metric: if it reads as
sluggish in the hand, the lever is `FRACTURE_CELLS_PER_TICK` and the staging
interval, not the region shaping.

### D3. Near-surface blasts do not throw chunks into the air

Reported from play: *"explosions in particles and explosions deep in the rock
are close to satisfying, but explosions near the surface of rock should blast
chunks into the air and it doesn't."* Diagnosed and deliberately not built.

The plumbing exists: `ChunkBody` has signed `vx/vy`, `rigid::advance`
integrates gravity as a plain `+=` with no falling-only assumption, and
`rigid::promote` computes a real radial velocity. Four reasons it cannot do
this today, none of them a tuning value:

1. **Magnitude.** At the crater rim `|v| = (180 * 0.06) / ~21 ~= 0.51`
   cells/frame; against `GRAVITY = 0.15` that is **0.87 cells of rise**. The
   same blast's particles launch at 8-9 cells/frame, ~16x faster — which is
   why the ejecta plume that reads well is entirely grit.
2. **Direction is radial from the epicentre only** — no free-surface normal.
3. **The one late chunk-producing step is aimed the wrong way on purpose**:
   `explosion::calve` uses `-(strength * CALVE_FORCE)`, throwing the rim
   *into* the hole.
4. **Depth is not an input to any impulse.** `probe_confinement` computes
   `RayResult.cost` — the resistance-weighted distance to air — and
   **discards it**. There is no burden measurement anywhere.

*Fix shape, worked but unbuilt:* keep `RayResult.cost` per sector as the
burden; grade the outcome three ways on it (deep -> camouflet unchanged,
shallow -> flood the cone between the charge and its nearest free surface and
hand it to `fracture_with_impulse` with a **positive** force along the
free-surface normal, zero -> surface burst that mostly vents); and make the
magnitude an order of magnitude larger than the rim's — 2-4 cells/frame buys
13-53 cells of rise against `MAX_SPEED_PER_AXIS` of 6.0. `Reports/
explosion-stone-review.md` §4 already defers "explicit spall" by name.


### D4. At a bounded reach a collapse can stop part way and leave a slab in open air

`39d0978` clips a failing region to the licence, so at LOCAL and TIGHT a
failure eats only the part of itself the leash covers. On `scene=ligament` at
TIGHT that means **383 of the overhang's 4,400 cells come down** — the part
inside the 33x33 box around the neck — and **4,017 stay standing, as a slab
with air under it**, because the clip removed the middle of the connection
and refused the rest.

**This is a decision, not a bug**, and both halves of it are already written
down. `wiki/structural-collapse.md` states the consequence in the player's
language ("at the tighter settings a collapse can now stop part way and leave
rock standing that is holding nothing up"). The older promise it replaced —
*"nothing stops half way and leaves rock hanging in the air, and no setting
anywhere makes the rest of it safe"* — is what
`a_paced_remainder_falls_even_when_the_disturbance_cannot_reach_it` asserts,
and the two cannot both be true at a bounded reach. That test is `#[ignore]`d
with the full account in its own doc comment rather than edited, per
`CLAUDE.md`'s "a revert keeps the knowledge": the reproduction is exact, and
whichever way this is settled it is the scene that shows it.

**Nothing is unguarded, only undecided.** The property that test was *named*
for — the staged queue is work, never re-judged — is pinned by
`a_paced_remainder_falls_even_after_its_licence_has_gone`, in a form the clip
does not make vacuous.

**The open question, in the words of the commit that created it:** *is a
4,000-cell slab left hanging in open air at TIGHT better or worse than the
unleashed cascade it replaces? It is not obviously better, and it is the one
outcome the load model has spent four support models avoiding.*

It needs a playtest verdict at LOCAL/TIGHT, not another metric. Note that
SPREAD — the shipped default, and what acceptance and CI run — is untouched:
`clip_region_to_licence` returns the region unchanged at `i32::MAX`. Full
record: `Reports/explosion-stone-review.md` §17, and the test's doc comment
at `src/sim/structural.rs`.

### 0. Roofed water: `ponds` fills both sides of an overhang (worldgen)

`ponds` fills any hollow that reaches the open surface, and an overhang
(`brows` lip, or now an erosion-shaped shelf) over a flooded hollow can
leave water standing both above and below a rock shelf — water buried
under stone that the guards `generated_water_is_full_and_never_inside_
the_ground` / `every_solid_is_anchored_and_no_liquid_carries_a_stale_
fill` only catch at their 1 hardcoded seed each, so they pass by luck.
Present at `world_age 0` on a majority of seeds for several presets —
pre-existing, surfaced (not caused) by round 4's age flip; the full
measurement is finding **R4-3** in
`Reports/worldgen-implementation-tasks-2026-08.md`. Two narrow `brows`
guards shipped in round 4 close the paths that broke the structural
suite; the pattern itself needs a `ponds`-focused session with a real
seed sweep, not another guard clause. Do not widen the two named tests'
seed lists as a "fix" — they would go red on the standing defect.


### 0b. The deep massif reads as television static, and it is a per-cell palette dither (worldgen) — **FIXED**

> **Fixed 2026-08-21**, along the direction this entry names.
> `palette_family` now compares against fBm on the same `Purpose::Palette`
> stream instead of a per-cell `noise::unit` draw, so the boundary wanders
> because the field does. Measured, canyon seed 1, deep-rock crop, paired
> in one tree: **luma MAD 5.612 -> 2.216 (-61%), chroma MAD 1.775 -> 0.318
> (-82%)**.
>
> `FAMILY_DITHER_WAVELENGTH` = 40, chosen by eye from a three-point sweep:
> 14 reads as camouflage blotches, 96 as a bare curve, 40 as a coastline.
>
> **`FAMILY_DITHER_CONTRAST` is the part that would have been missed.**
> `noise::unit` is uniform on 0..1; a normalised three-octave fBm spans
> roughly 0.30..0.60, so thresholds tuned for the tails of a uniform draw
> stopped firing and `wetland` seed 1 came out with every rock cell in one
> family. Caught by `a_varied_world_uses_more_than_one_rock_family`, not by
> the author. Re-deriving the constants that read a changed quantity is
> part of the fix.
>
> **Still open, deliberately:** `strata_shade`'s separate "12% of cells jump
> a tone" rule, which this entry asks to be re-judged in the same pass. It
> is the same shape at much smaller amplitude -- brightness only, no hue --
> and now reads as rock texture rather than noise. Left rather than change
> two things at once; it is a one-line follow-up if the owner disagrees.
>
> **Measurement note, because it cost three invalid readings:** piping a
> render into `grep -q` closes the pipe on the first match and can kill the
> producer before it writes its PNG, leaving the previous run's file on
> disk. That produced a byte-identical image across three wavelengths --
> which reads exactly like "the knob was never connected" -- and a
> cross-worktree baseline no clean build could reproduce. Redirect, never
> pipe, and prefer a paired `git stash` comparison inside one tree.

The original entry, kept for the diagnosis and the mis-attribution:


Every cave render at 4x zoom shows the surrounding rock as full-contrast
salt-and-pepper speckle — louder than any cave feature in the frame, and
directly against the two things a cave picture is composed on (darkness
preserved; rock with grain and *flow* rather than noise). Images:
`Reports/img/cave-anatomy/`.

**Attributed wrong the first time, and the wrong attribution is the
useful part.** The obvious suspect was `render.rs`'s `JITTER_STRENGTH
0.12` — a per-pixel proportional brightness jitter applied at full
strength to deep rock. It was measured by setting the new
`DEEP_GRAIN_FLOOR` to **zero** (grain entirely off below the depth
ramp) and re-rendering the same crop. The picture barely moved.
`examples/pixel_stat` apportions it (canyon s1, deep-rock crop):

| | luma MAD | chroma MAD |
|---|---|---|
| shipped | 3.017 | 1.374 |
| grain floor 1/3 (now shipped) | 2.325 | — |
| grain **off** at depth | 2.090 | 1.301 |

So the render grain is **31% of the luma speckle and 5% of the chroma
speckle**. Sixty-nine per cent of it survives with the grain switched
off. On `rolling` seed 7 the chroma MAD (3.43) is *larger* than the luma
MAD (2.34) — the speckle there is predominantly a **hue** dither, which
the grain cannot produce at all (it scales all three channels by one
factor).

**The mechanism is `passes::palette_family`** (`src/worldgen/passes.rs`):
it draws `u = noise::unit(seed, Purpose::Palette, x, y)` **per cell** and
compares it against a family probability. Wherever that probability is
mid-range — which is most of the world, by design — the result is a
per-cell Bernoulli dither between two palette families that differ by
~40 brightness points *and* a large hue shift (neutral grey `128,128,132`
against warm sandstone `168,146,112`). At play scale that is confetti,
not a boundary.

**It is doing exactly what it was built to do**, which is why no test
sees it: the round-1 comment calls this "the dither band" and records
that the aridity ramps were deliberately *widened* to make it broader,
because a narrow ramp gave "solid blocks of one family" with the
families interleaving over only a few columns. The intent — a meandering
boundary between differently-coloured countries — is right. The
implementation puts the meander in white noise per cell instead of in
the field, so what should be a wandering coastline is dithered surf
everywhere.

**Direction, not yet built**: decide the family from a *continuous*
field — threshold the existing `PaletteField` fBm (plus the smoothstep
on `Character`) against a smooth spatial value rather than against a
fresh per-cell white-noise draw — so the boundary meanders because the
field does. If an interleave at the boundary is still wanted, an ordered
or blue-noise dither confined to a narrow band around the threshold
gives it without spraying the interior. Note `strata_shade`'s separate
"12% of cells jump a tone" rule is the same shape at smaller amplitude
(brightness only, inside one family) and should be re-judged in the same
pass.

**Owner's verdict, 2026-08-21, on a blind A/B of the grain grade**: *"I
see no difference. The problem is the big sharp squares that look like
giant white gray pixels."* Two things follow. The grain grade was
**reverted** — it measured a real 23% cut in luma speckle and bought
nothing anyone can see, which is the outcome the card was posted to test
(`DEEP_GRAIN_FLOOR`, reverted in the same session it landed; do not
re-attempt it as a standalone change). And the deep-rock texture
complaint is now **two** defects, not one: this per-cell palette dither,
*and* the light field's 8-cell quantisation (see 0c below), which is what
"giant white gray pixels" names. Fix 0c first — it was picked out by
name, unprompted, on a card that was not about it.

**Owned by the worldgen data track** (`passes.rs`), which is why this is
recorded rather than fixed: round 5 is mid-flight in that file. Do not
race it. **Scheduled: round 6, immediately after round 5 merges**
(owner's ruling, 2026-08-20), so the cave strips get judged twice — once
with the static and once without — and the round-5 bars are not measured
against a moving palette. `DEEP_GRAIN_FLOOR` shipped anyway on the render side — a
measured 23% cut for nothing, skip-safe — but it is **not** the fix and
must not be reported as one.

**Sanity note for whoever picks this up**: `pixel_stat` reports mean
absolute deviation from the 3x3 neighbourhood mean, not variance, so a
smooth large-scale gradient (a strata band, the depth ramp) scores near
zero and only per-pixel departure counts. Check it against a region you
know is clean before trusting it about one you don't.


### 0c. Cave light is quantised to 8-cell squares (render) — **FIXED**

> **Fixed 2026-08-21** by the near-field glow term this entry asks for, in
> `render.rs`'s `rebuild_near_glow`/`near_glow_at`. Every cell with
> `Material::glow > 0` splats a squared-linear falloff over
> `NEAR_GLOW_RADIUS` (14 cells) into a per-chunk, per-cell buffer;
> `glow_at` returns `coarse.max(near)`, gated on the coarse field being
> non-zero so the term inherits the field's blocking rather than shining
> through rock. Shipped behind `GlowShape` (`'` in the app,
> `glow=field|near` in `viewshot`) with the **new** behaviour as default,
> since this was a reported bug rather than an open question of taste.
>
> The cost trigger took a correction worth keeping: keying the rebuild on
> `glow_unsettled` rebuilt on every draw (9 in 9, measured), because the
> day/night cycle means a tile with any sky in it never settles. The splat
> depends on world cells only, so the trigger is `touched`.
> `a_settled_glow_does_not_rebuild_its_halo_every_frame` guards it.
>
> **Residual, deliberately not chased:** beyond the radius the coarse
> field's own blocks are still faintly legible on a large halo. They are
> much dimmer there; growing the radius until they leave the screen would
> cost the whole point of a *short*-range term.
>
> Note for 0b, which is still open: with the light blocks gone, what is
> left to look at in a cave is the palette static. The two were named
> together and only one of them is done.

The original entry, kept because the diagnosis is the load-bearing part:


`FIELD_SCALE = 8`: the light channel holds one value per 8x8 cells and
`field_at_bilinear` smooths between those. So a glow's smallest possible
feature is **8 cells**, its gradient is smeared over ~16, and it aligns
to the field lattice rather than to the emitter. A 1-2 cell crystal
therefore lights a **rectangle** of rock, offset to one side, with hard
vertical edges.

Named independently by the owner on two different cards: *"The
rectangular lighting looks bad"* and — on a card about something else
entirely — *"the problem is the big sharp squares that look like giant
white gray pixels"*. That is the strongest signal in the session: it was
volunteered against the question being asked.

**An earlier note in this repo called this "glow halo block-edge
softening, low priority" and was aimed at the wrong thing.** The halo is
not too hard-edged for want of smoothing; it is too *coarse* to have a
shape at all. Smoothing a 16-cell-wide blob harder makes it a vaguer
16-cell-wide blob.

**Do not fix by raising `FIELD_SCALE`** — the field is deliberately
coarse because pressure and light are low-frequency, and a finer grid is
64x the work for detail nothing else reads. The fix is a **short-range
term computed from the emitting cells themselves**, evaluated only in
chunks that contain one (`Renderer::glow_tiles` already gates exactly
this), with the coarse field left to carry the far falloff. That reads
neighbour cells, so it inherits landmine §7.22 — touched-chunk screen
rects must widen by one cell or it ships a stale-pixel class — and it is
not free; price it before building it.

**Also reverted here, so it is not retried**: an emissive-core term
(`EMISSIVE_RESTORE`) that drew a cell with `Material::glow > 0` at its
own unlit palette brightness. It gave the crystal a bright core and the
owner chose *against* it in a blind A/B, correctly: crystal's four tones
are luma ~205/224/240/250, all in the top fifth of the range, so pulling
them toward full brightness **collapses them into one white** and removes
the only facet variation the object had. Their words on the pane they
preferred: *"mostly the texture on the crystal."* A brightness lift for
an emitter has to preserve the tone spread, not compress it — and
crystal's spread is too narrow and too pale to survive one. See the
cave beauty review's round-5 verdict for the general rule (a shape needs
coherent shading; this codebase assigns per-cell random tone almost
everywhere).


### 0d. The organism support search asks the wrong question — see `Reports/felling-blockers.md`

Not new, but newly written up. `structural::organism_is_supported` anchors
on `MaterialKind::Solid` (soil is a `Powder`, so it anchors nothing) and
searches outward from the cell under test bounded by
`max_unsupported_span`, so it answers "am I within 8 hops of stone" rather
than "can I reach a root". Any structural check fired mid-crown therefore
amputates the tree — measured at 772 cells against 20,213 (`plant.rs`'s
`shed_stranded_leaves`).

**Superseded by the plant-line merge, 2026-08-22 — read this before
acting on the paragraph below.** This entry and `felling-blockers.md` were
written on a trunk that did not yet have `plant-substrate-v2`. That line
**replaced the mechanism they are about**: `structural::organism_is_supported`
no longer exists as a function anywhere in the tree (only as references in
comments), and what decides whether a plant cell stays up is now
`plant::anchor_support` plus `OrganismCell::support` — a Dijkstra run
*from the anchors outward*, once per organism tick, with **no span budget
to run out of** and an eight-connected walk matching `Grow`.

That is the same shape of fix `felling-blockers.md` §1 asks for. Both
defects it names are addressed by construction rather than by tuning: the
search no longer starts at the cell under test, so a check fired mid-crown
does not amputate, and a diagonal branch is not read as disconnected.

Two cautions against over-reading that. **It does not follow that felling
is unblocked** — `felling-blockers.md` lists other items, and it says two
of them are redesigns wearing the costume of constants; the whole report
wants re-reading against the merged tree rather than assuming one change
cleared it. And the claim below that "every organism path deliberately
schedules no check" is **no longer true**: `anchor_support` schedules one
whenever a cell's distance rises. That is safe for the reason §B records
(creature cells are discarded at `is_body_material`, and the plant path is
the one this mechanism was built for), but it means the latency argument
below has expired along with the mechanism it protected.

It is latent rather than live only because every organism path
deliberately schedules no check: growth, germination, abscission and
`player::shake` all say so in place. It goes live the moment anything
does, and it is the blocker under felling. The fix, the cost, and the six
paths that would trigger it are in `Reports/felling-blockers.md`.

### 0e. ~~A decay site does not follow its cell~~ — **FIXED 2026-08-21**

*(Was §0. Renumbered when this file merged with the trunk, which had
independently taken §0 for the roofed-water bug above. Four source comments
cite it and were updated in the same change.)*

Kept because the *reasoning* is reusable, not because the bug is open.

**Was:** a scheduled `ActiveKind::Decay` site is a bare coordinate;
`CellSurface::move_cell` touches no scheduler state; `decay::tick`
unschedules on a material mismatch, which is also what "the cell fell out of
this coordinate" looks like. So anything that moved before its first check
(200 frames) was immortal. Live for ash (fire makes it where the fuel just
burned away, so it usually falls) and total for litter (shed in a canopy,
falls every time).

**Fixed by changing *when* a site is scheduled, not by making sites follow
cells.** Decay sites are now created at the **awake→settled transition** in
`World::end_step`, riding the chunk scan `recompute_reach` was already doing
there. That is not a workaround for the strand — it is what the rule always
meant. Weathering happens to matter that has come to rest, so settling *is*
the event, and a cell that moves afterwards simply gets a fresh site when it
stops. Bounded (one chunk), rare (chunks settle once and stay settled), and
no hot-path cost.

Two riders it needed:

- `Material::decays_into` / `decay_reseeds`, so the scan gates on a `Vec`
  index at a site that already holds the `Cell` (ash → soil and litter →
  soil are both data now; ash keeps the reseed roll, litter does not).
- The dedup index extended from `StructuralCheck` to `Decay`. Without it a
  drift that is disturbed and re-settles stacks a site per settle, and since
  each rolls `DECAY_CHANCE_*` independently the decay rate would become a
  function of how often the ground was walked on — a correctness problem,
  not a performance one.

**The four candidates it was chosen over**, kept so they are not re-derived:

| candidate | why it lost |
|---|---|
| Re-schedule from `move_cell` | Its own comments call it the hottest path in the engine, and a falling cell moves every frame — it would push a site per frame of fall, each 200 frames out. |
| Have `tick` search for the cell | Bounded scan, fragile, wrong the moment two cells swap which one it finds. |
| Per-cell age in `aux` | **Cannot work.** Something must tick the age and the CA sweep skips settled chunks — a settled litter layer is exactly when decay must run and exactly when the sweep is not visiting. This is *why* the scheduler exists. Also `aux` already carries two opposite conventions. |
| Slow global sweep for decayable material | Trades a per-cell schedule for scanning the world; wrong direction with M10 streaming coming. |

**Guard:** `decay::tests::ash_that_falls_before_its_first_check_still_decays`
(was `#[ignore]`d as the reproduction, now passes and stays as the guard),
plus `litter_rots_away_instead_of_accumulating_forever` and
`a_world_where_nothing_sheds_holds_exactly_no_litter`.

**Measured after:** paired against the pre-change commit, same machine,
minutes apart — worst frame **240.60 ms vs 257.74 ms** on a settled tree
grove, i.e. no regression. Pending decay sites went **105 → 12,056**, which
is the mechanism working rather than leaking: every settled litter cell holds
one deduped site, and the count converges (8,424 → 11,671 → 12,056), so that
is a standing forest floor at equilibrium between leaf fall and decay.

**Still open, and it is cosmetic:** litter's palette was authored close to
soil's on purpose ("reads as texture, not a second canopy lying down"), and
on the close-up that looks like a mistake — twelve thousand cells of it and
it barely separates from the ground. Posted to the review queue; if it does
not read, the fix is the palette, not the mechanism.

### NEW. ~~Plants grow nothing on generated terrain~~ — **FIXED 2026-08-22**

**FIXED.** `examples/ascii` passes (exit 0), with the foraging scene showing
**739 deliveries by frame 12,000** where it showed zero. The fix is the
aridity-shaped soil baseline in the worldgen moisture pass. Measured against
a control with the baseline disabled (same build, same seed, frame 10,800):

| preset | living tissue, before -> after | decay events |
|---|---|---|
| wetland | 4,950 -> 24,160 | 113 -> 554 |
| rolling | **12 -> 14,166** | 0 -> 260 |
| canyon | **7 -> 6,064** | 0 -> 58 |

Rolling's control total of **12** living cells is exactly its `life_scatter`
count of 12, and canyon's **7** is exactly its 7: every seed the generator
scattered was still sitting there ungerminated after 10,800 frames. Those
biomes were not sparse before — they were **inert**. Judge-by-eye card for
the result posted 2026-08-22 (`plants` board, "Four biomes, after the
worldgen soil baseline").

**Historical, from when this was open:** it read as follows.

**~~This is the one finding here that should stop a merge to `main`.~~** It was
found by the second integration (bringing `origin/main` at `98ac541`, 144
commits, onto the plant lines), and it fails `examples/ascii`, which CI
runs.

`ascii`'s *"ants: the foraging loop"* scene plants **six trees** on
worldgen terrain, warms up 2,400 frames, and counts `leaf` cells as the
ants' food (`ascii.rs:1393`, `food_left`). Measured:

| | `origin/main` | merged |
|---|---|---|
| leaf cells at frame 2,000 | **2,140** | **0** |
| leaf cells at frame 12,000 | **3,087** | **0** |
| ant pickups / deliveries | 712 / 683 | **2 / 0** |
| live organisms | 75 → 75 | 75 → 63 |

The assertion that fires is the scene's own: *"no ant completed the loop."*
The ants are not broken — **there is nothing to forage.** Looking at the
frame confirms it: no trees are visible at all, only nest, stone and ants.

**What has been ruled out, each by a controlled run:**

- *The creature guard in `step_organisms`* — disabled it, output was
  **bit-identical** (moves 8371, pickups 1, digs 195). Not the cause, and
  this doubles as confirmation that the guard really is inert for creatures.
- *The plant-line slot reclamation* — disabled it; organism count returns
  to 75, and the ant numbers are again **bit-identical**. It explains the
  63 and nothing else.
- *`leaf.ron`'s expanded palette* — swapped main's file in; no change.
- *Worldgen divergence* — the spawn frame is **byte-identical** between the
  two branches, so the same world, the same six trees, the same food site.
- *The merge resolutions* — `pheromone.rs` and `field.rs` are byte-identical
  to main's; `creature.rs` differs only in a `#[cfg(test)]` scene.

**The mechanism, stated with its evidence level.** `main` has **no
`absorb_water` at all** — plants there run on one currency and do not need
soil moisture. The plant line makes water a real second currency. And
`worldgen::passes` wets soil **only within two cells of water**
(`passes.rs:3146-3170`: distance 0/1/2 get capacity/fringe/fringe, and
everything else hits `continue`). So a tree planted on generated ground
away from water sits in soil at `aux == 0`, which is *dry*, and a root in
dry soil has no income.

That chain is read from the code and matches every measurement above, but
**the last link is not directly instrumented**: nobody has printed soil
moisture at those six tree positions, and germination itself also reads
moisture, so "they germinated and then starved" and "they never germinated"
are not yet told apart. That is one probe.

**This is the third instance of one class**, and the class is now the
important thing rather than any single case. §E was a hand-written test bed
at `Cell::new(soil, 0)`; the same session then fixed that bed's missing
floor; this is the same defect in **worldgen**, which is not a scene anyone
can dampen by hand. **When a merge introduces a new currency, every place
that creates the substrate it is drawn from becomes a scene that may no
longer supply it** — including the procedural ones.

**THE OWNER HAS DECIDED THE DIRECTION, 2026-08-22, and it inverts the fix.**
Stated directly:

> plants should only grow where it's wet. rain and weather should allow that
> to happen everywhere. maybe some plants slow down where and when it's
> drier. if it's not wet at the time the seeds should sit there and wait
> until it is rain and then the soil gets wet and then they germinate. we
> could always build a scene that is ideal for plant growth and stable for
> comparisons

So **dry ground refusing plants is correct and must be kept.** Do not
baseline all soil wet — that was considered and rejected. The defect is not
that the ground is dry; it is that **seeds germinate anyway and then
starve** instead of waiting.

**The good news, verified: the dormancy machinery already exists and
works.** `Behavior::Germinate`'s not-ready path sets `found_candidate =
true` and reschedules with `stale_ticks` reset, and `is_frontier` includes
`Seed`, so the retirement branches are unreachable and
`ORGANISM_STALE_LIMIT` never applies. **A seed already waits forever.** The
only thing wrong is what the predicate reads: `world.field_at(x,y).moisture`
— the field channel at the seed's own cell, which the in-code comment
correctly measured as useless. The repair is to read the **soil the seed is
resting on**, via `update::plant_available_fraction` on the already-fetched
`below` cell (`pub(crate)`, already called from `plant.rs` in three places).
Suggested threshold 0.25 — strictly above 0.0, well under field capacity,
and under every existing test bed. The RON field wants renaming: its unit
changes from the field's 0..4 scale to a 0..1 fraction.

**Three traps, each verified in code, each of which would let the mechanic
be built exactly right and still not work:**

1. **`update::soil_moisture` has no material check**, and on a `Liquid`
   `aux` is *fill*, where 0 means FULL on the same 1000 scale as
   `SOIL_SATURATED`. A seed floats (density 0.6 against water's 1.0) and
   `resting` accepts any non-empty cell, so a seed on full water would read
   **bone dry** and one on half-drained water would read well-watered. Gate
   on `water_capacity > 0` first, as `plant.rs` and `update.rs` already do
   elsewhere.
2. **Rain cannot wet the soil under a resting seed.** `weather::step`'s soak
   loop starts at the topmost non-empty cell of the column and `break`s at
   the first cell with `water_capacity == 0` — **and the resting seed is
   that cell**, since `seed.ron` declares no capacity. Zero soak reaches its
   own column; only lateral capillary flow can, and that does nothing until
   the gradient exceeds 380. **This is the same defect as F1** (litter and
   grassblade block soak for the identical reason), which makes it a class
   rather than two bugs: *anything that rests on soil and declares no
   `water_capacity` shadows the ground beneath it from rain.*
3. **The failing scene cannot rain at all.** `weather::step` runs only from
   the CA drivers, and `ascii`'s forage scene grows its six trees in a
   2,400-frame warmup that calls only `step_active_sites` + `step_fields`.
   No CA in that window means no rain, no infiltration and no capillary
   flow, so those seeds cannot germinate there under **any** wait-for-rain
   predicate. The scene needs fixing alongside the mechanic.

**What "rain wets everywhere" needs to become true.** Measured: soil aux has
three sources and exactly one sink (root uptake) — there is **no
soil-to-air drying at all**. So wet is an absorbing state, and without a
drying sink "slow down where and when it's drier" is a transient that never
returns, and the grassfire steer (§G, *"moisture vs dryness should play a
role"*) would have nothing to vary. A drying sink is the missing half of
the owner's model.

**And a knob that already exists for "only where it's wet".** `aridity` is
per-column, varies smoothly within a world, ships per preset (wetland 0.08
→ arid 0.92), and is **already read three lines below the soil-moisture
pass** to decide where trees get planted. An aridity-scaled soil baseline
would make the same number decide both *where a tree is planted* and
*whether it can drink* — which closes this bug class structurally rather
than by picking a constant. Note the collision it must resolve in the same
change: the "damp" gates for moss, decay and fire trigger at soil `aux >
75`, while plants get nothing below 180, so any baseline that feeds a root
already reads damp to everything else (decay 25x, moss up to 175x).

**Not fixed here, deliberately.** The candidate fixes are a worldgen change
(wet soil more widely, or by climate rather than by distance-to-water), a
plant-economy change (let a root draw from something other than adjacent
soil moisture), or accepting dry ground as real and giving worldgen a
reason to place trees where water is. Those are three different games, and
picking between them is a design decision, not a merge resolution.

### U. Water stress makes a tree BIGGER — **DOES NOT REPRODUCE over 8 seeds, 2026-08-23; the missing penalty it names is real and measured. See §P1.**

> **2026-08-23, P1.** Swept over 8 seeds on this entry's own bed, drought grew
> a *smaller* plant on 5 of 8 and less wood on 6 of 8, and the means go the
> right way (2,102 cells against 2,423; 1,146 wood against 1,362). The
> 982-vs-734 below is one sample from a distribution that straddles. What
> **is** confirmed is the mechanism this entry guessed at: the
> `break_root_tips` exit for "thirsty, sites available, cannot afford it"
> reads **zero in every arm measured**. The penalty is missing; the outcome
> it was blamed for is not there.

Measured while trying to write a replacement guard for §V, on one bed over
12,000 frames with only the soil moisture differing:

| | nearly dry (aux 310) | field capacity (620) |
|---|---|---|
| total cells | **982** | 734 |
| wood cells | **428** | 299 |

**Both are the wrong way round.** Real drought reduces total biomass, and
reduces secondary growth in particular — narrow rings in dry years is the
entire basis of dendrochronology. What genuinely rises under water stress
is the root:shoot **ratio**, and it rises because shoots suffer *more*, not
because roots gain in absolute terms. Here every quantity goes up when
water is short.

An earlier note in this file waved this through as "exactly as a real plant
raises its root:shoot ratio under drought". That was too charitable and is
corrected here: the ratio shift is real, the absolute increase is not.

**Likely mechanism, unproven.** `break_root_tips` is gated on
`water_status < 0.95`, so water stress *triggers* root re-initiation — but
the stress does not appear to throttle the carbon that pays for it. Scarcity
buys extra tissue at no cost: a compensation response with the penalty
missing. If that is right, the fix is that `water_status` should scale what
the plant can *afford* as well as what it decides to build, and the two are
currently decoupled.

**Why it is logged rather than chased.** It surfaced inside another
investigation and is not a merge regression — it is a property of the plant
economy the plant lines brought, and it needs its own pass with a probe on
carbon income under stress. Deliberately not tuned: §A is already a live
warning about re-deriving plant constants without a seed sweep, and this
touches the same `water_status` path.

### V. ~~A tree with no seedlings under it never stops growing~~ — **ACCEPTED AND RETIRED, 2026-08-22, by owner decision.**

`a_tree_eventually_stops_growing` was passing after the worldgen work and
fails once seeds wait for water: the subject reaches **1,718 cells and is
still growing at 120,000 frames**, where the recorded plateau is ~565.
Isolated by control — neutralising the germination gate alone makes it pass,
and the run takes half as long (72 s against 146 s), which is the extra
growth showing up as work.

**The mechanism, stated as the hypothesis it is.** The bed is at field
capacity, so the *subject* germinates either way. What changes is its
offspring: `Behavior::Reproduce` recruits a stand indefinitely, and a
mature tree draws the soil around it down toward the wilting point. Once it
does, its own seedlings cannot clear the 0.25 threshold, so they sit as
dormant seeds instead of becoming competitors — and the parent, now
uncontested, keeps growing.

**Which means the test's premise may never have been true.** Its name says
the tree *"exhausts its resource economy"*, but if what actually bounded it
was competition from its own offspring, it was measuring crowding and
calling it economy. That is this file's recurring shape: a guard that passes
for a reason other than the one it is named for.

**The owner accepted it:** a solitary, well-watered tree growing without
bound is correct, and a mature tree drying the ground and suppressing its
own seedlings is what a real stand does. The claim is retired rather than
tuned, and `a_tree_eventually_stops_growing` is gone.

**No replacement guard shipped, and that is the interesting part.** Two
were written and both had their premise falsified by the first run:

- *"A tree grows less on less water"* is **false as stated**. Over 12,000
  frames on the same bed, the thirsty arm grew **982 cells against the
  watered arm's 734.** Not noise: `break_root_tips` is gated on
  `water_status < 0.95`, so a water-stressed plant re-initiates root tips
  and invests in roots — exactly as a real plant raises its root:shoot
  ratio under drought. The same mechanism §A is about.
- Counting **wood alone inverts it a second way** (299 watered against 428
  thirsty), because a well-watered plant spends a larger share on foliage.

So **growth here is not monotone in water**, and any future guard must say
which quantity it means — shoot mass, total mass, or time-to-plateau — and
be measured before it is asserted. `plant_tree_on_ground_with_moisture`
exists so that comparison is one argument away when someone has a premise
worth testing. The full account sits where the test was, in
`plant.rs`'s test module.

**Unverified:** the competition mechanism is inferred from the control plus
the population count (22 live organisms at failure, which includes dormant
seeds and so cannot separate "fewer competitors" from "more waiting seeds").
The measurement that would settle it is a count of *established* offspring —
organisms past the seed stage — in both configurations.

### Z. The stand still reads as one mass — **JUDGED 2026-08-22. Two verdicts, and a metric that lied.**

Two cards, both answered by the owner, and together they settle a question
this session got wrong twice in opposite directions.

**Card 1 — the merged stand judged against the absolute standard** (one
stand, frame 28,800 at noon, "how many separate trees can you count? eight
were planted"):

> **"No. Everything has merged together into a big mass. I cannot identify
> individual trees."**

**Card 2 — a blind A/B, merged against `plant-substrate-v2` alone:**

> *"In A everything has merged together, In B two of the trees have merged
> and 2 are more seperate. Big improvement but not a full solve"* — with
> the merged stand confirmed as the better side.

**Both are true and they are not in conflict: the merge improved a bad
situation that is still bad.** The delta is positive; the absolute is a
fail. That is precisely why `tree-architecture-research.md` §7d says to
judge against a clear bole and a foliage crown rather than against the
previous frame — and this session demonstrated the trap in both directions,
first raising a false alarm from an A/B and then retracting too far from
the owner's "not wildly different".

**The metric lesson, which is the reusable part.** The absolute-standard
card reported *"crown shyness is working"* on the strength of one number:
the **widest unbroken run of plant cells above ground was 39 cells against
a 56-cell founder spacing**, i.e. no row is continuous across two crowns.
That number was correct and the conclusion drawn from it was wrong. Crowns
interleave with one- and two-cell gaps: every row breaks, and the eye still
reads one mass. **A contiguous-run metric measures whether crowns *touch*;
it cannot measure whether they are *distinguishable*.** Anyone reaching for
it again should know it has been believed once and overturned by looking.

What would actually measure the claim is unsolved here. Candidates worth
trying before trusting any of them: count connected components of foliage
at the field's resolution rather than the cell's; or measure the width of
the *sky gaps* between founders rather than the runs of plant; or simply
accept that this one is judged by eye and post the card.

**Still open**, and now with an owner verdict behind it rather than a
suspicion: the stand does not read as separate trees. The bole findings in
§Y (bottom crown band 60 where a clear-boled tree reads 0, foliage centre
58, foliage share 27% and falling with age) are the measured shape behind
it.

---

**C4's metrics were built, calibrated against the owner's eye, and FAILED.
§Z is cards-only. — 2026-08-23, P1**

Both candidates this entry names were built in `examples/plant_probe.rs`:
connected canopy components at the field's 8x8 resolution (reported as
*fusion*, the largest component's share of canopy blocks), and the sky-gap
census. Three stands were rendered at frame 28,800 and put to the owner
**with the founder counts withheld**, asking only "how many separate trees
can you count?" — cards `20260823T092919055Z-ac816a` and `...-87b3f5`,
answered identically, so the reading is stable.

| founders | spacing | raw gaps (widths) | gaps + 1 | >=8-cell gaps + 1 | fusion | **owner counted** |
|---|---|---|---|---|---|---|
| 8 | 56 | 1 (`[1]`) | 2 | 1 | 99% | **2** |
| 4 | 102 | 1 (`[4]`) | 2 | 1 | **100%** | **4** |
| 3 | 128 | 2 (`[1, 32]`) | 3 | 2 | 38% | **3** |
| 2 | 170 | 1 (`[13]`) | 2 | 2 | 58% | not carded |

**The 4-founder stand settles it.** The owner counts *all four*. Fusion
reads **100%** — the strongest possible "one mass" — and the gap census
finds a single 4-cell gap where the eye finds three separations. The claim
P1 made before asking, that fusion "splits cleanly and in one place", is
**false**: the split it draws puts a stand read as four distinct trees on
the fused side.

**Why the column census misses it was guessed at, and the guess was wrong
— this is the retraction.** This entry first argued that no column census
can ever work: a gap is a fully empty column, the crowns at 102 cells apart
touch, so the eye must be reading trunk position and crown outline, cues
carried by the shape of the occupancy rather than by any empty column. Card
`20260823T150917441Z-d236fd` put the question to the owner directly, and he
answered: **"The gaps of sky. The two on the left are starting to merge but
still read separate. The two on the right are clearly separated with no
touching. I think the piles of soil are making it hard to read."**

So the cue **is** sky, the structural-limit argument above is **withdrawn**,
and a column census is not doomed — it is **looking at the wrong rows**. Two
of his three separations are clean, no-touching sky, and the census still
reported a single 4-cell gap across the whole stand. Something is occupying
those columns in rows the eye does not read as canopy, and he names the
suspect himself: the piles of soil. A census that kills a gap on any
occupied cell anywhere in the column is answering *"is this column clear
from the ground to the sky"*, where the eye is asking *"is there sky between
these two crowns"*. Those are different questions and only one of them was
carded.

**Two thresholds, both invented to explain away a reading I doubted, both
strictly harmful.** First a quarter of the founder spacing, which scored two
obviously separate trees 170 apart at zero (a quarter of 170 is 42). Then an
absolute 8 cells, which scores **0 of 3** against the owner where the
*unthresholded* count scores **2 of 3** — because the 1-cell gap at 8
founders that I discarded as noise is exactly the separation behind the
owner's answer of "2". Raw gaps + 1 gives 2, 2, 3 against 2, 4, 3.

**What survives, and it is not nothing.** The negative result this entry
already recorded is confirmed hard: `thickest contiguous run` reads **36 to
51 across the whole spacing range** and is *highest* on the stand the owner
counts as 2 of 8. It cannot tell a fused stand from a separate one in either
direction. And the component count must never be read alone — it exceeds the
founder count on a widely spaced stand, because a sparse crown breaks into
separate blocks.

So: **§Z is judged by eye and by card.** The numbers stay in `plant_probe`
as description, labelled as having failed calibration, so nobody spends the
round trip discovering this again.

**The next experiment, now that the card is answered, and it is the cheap
one.** Restrict the sky-gap census to a **canopy band** — the rows the
foliage mass actually occupies — instead of the full column, and re-score
the same three stands against the same three answers (2, 4, 3). That is a
*row* restriction, not another gap-width threshold; both thresholds tried
here were strictly harmful and a third would be the same mistake in a new
costume. The falsifiable prediction: the 4-founder stand gains at least the
two separations the owner calls clean and no-touching. If a band-restricted
census still reads one gap there, the mounds are not the occluder and the
trunk/outline reading comes back into play — but measured this time, not
argued.

**Not started, deliberately.** P1 closed with §Z cards-only and the card
arrived after it had landed; this entry records the answer so the next
session starts from the owner's own words rather than from the argument
withdrawn above. Until the band census is run and scored against those three
numbers, §Z stays judged by eye and by card.

### Z2. A free particle drops `Cell::aux`, so a blast under-prices a corpse — **REPRODUCED AND FIXED, 2026-08-23**

**Renamed from §Z to §Z2 on 2026-08-24**, its 13 inbound references repointed
(`src/sim/world.rs`, `explosion.rs`, `particle.rs`, `creature-review-2026-08.md`,
`creature-implementation-handoff-2026-08.md`). It collided with the older §Z
*("the stand still reads as one mass")*, and the two were resolving to each
other across reports — `creature-review-2026-08.md` meant this bug by §Z while
`plant-project-review-2026-08-23.md` meant that one. Renamed by recency per
`CLAUDE.md`'s own §Q→§R precedent: the first claimant keeps the letter.

> **Closed by WP-5 of the creature handoff.** Reproduced first, then fixed,
> then broken deliberately in both directions to prove the guards bite.
>
> **The reproduction, which this entry said had never been done.** A slab of
> `corpse` stamped 1,020 per cell, blasted at radius 20 through the real
> `explosion::trigger` path (`sim::particle::tests::
> a_blasted_corpse_lands_worth_what_it_was_worth`): **114 cells thrown, and
> every one landed worth 120.** The census over the survivors read **254.3
> per cell against 1,020** — arithmetic that resolves exactly to "the 20
> cells never thrown kept their stamp and all 114 thrown ones lost it".
> **102,600 energy destroyed by one blast**, and the estimate in this entry
> was right on the nose: 8.5x, on the one material whose value is per-cell.
>
> **The fix.** `Particle` gains `aux: u16`, taken from the source cell at
> spawn and written back by `land` **only when the landing material declares
> `Material::worth_in_aux`** — gated on the *flag*, not on the value, because
> an unstamped corpse (`aux == 0`) is a real case that `fire.rs`'s burnout
> writes deliberately, and `creature::food_value` should stay the only place
> that turns a 0 back into the material fallback.
>
> The cell-sourced entry point takes the **`Cell` itself** (`spawn_from_cell`)
> rather than an `aux` parameter: every caller that had this bug already held
> the `Cell` and passed two of its three fields, so a parameter would have
> been exactly as easy to forget again. Three callers source from a live
> cell — `explosion.rs:1639` and `:1826`, plus the splash path at
> `particle.rs`'s `throw_splashes`, which this entry did not list. The
> brush's debug burst (`app.rs::spawn_burst`) has no source cell and keeps
> the plain `spawn`.
>
> **`rigid.rs`'s `BodyCell` is left alone, as this entry says it should be**,
> and the reason is now recorded in `Particle::aux`'s own doc comment so the
> asymmetry is not "fixed" by symmetry later: a body only ever holds
> `Solid`/`Plant`, where `aux` is the organism packing, so carrying it would
> let a landing body silently re-attach.
>
> **Both guards, and both were made to fail.** The corpse case above, and its
> opposite — `a_blasted_grain_does_not_land_carrying_its_moisture`, because
> the artifact a fix like this *introduces* is over-copying: on soil `aux` is
> saturation on `SOIL_SATURATED`'s scale, so an unconditional copy lands
> every blasted grain soaking wet. Deleting the gate fires the moisture
> guard (a grain landed carrying `aux` 1000); reverting the whole fix fires
> the corpse guard (254.3 against 1,020); with both in place the pair is
> green.
>
> **The first version of that second guard was vacuous and is worth
> recording.** It asserted through `creature::food_value`, which *already*
> gates on `worth_in_aux` — so it reported soil's flat `food_energy`
> whatever the stamp said, could not fail, and duly passed with the gate
> deleted. It was measuring the gate it existed to guard, through a second
> copy of that gate. It only showed up because the fix was deliberately
> broken; on green alone it would have shipped. Rewritten to assert the raw
> `aux` on grains that landed *outside* the original slab footprint, which
> are the only ones that can have been thrown.
>
> **Nothing on screen changed, and that is the sharpest thing about this
> bug.** A corpse's *shade* is baked in at death by `creature_dies`
> (`creature.rs:1907`, a ramp over the animal's `start_energy`) and rides on
> `Particle::shade`, which was always carried. Only `aux` was dropped. So a
> blasted corpse landed **still drawn pale, as a fresh kill, while being
> worth 120** — the picture said rich and the number said carrion, and the
> picture was the one a person would have checked. `CLAUDE.md`'s division of
> labour, in the flesh: an image tells you *what* and *where*, and only a
> census tells you *how much*. No review card was posted for this fix,
> because there is nothing to look at; the evidence is the census.
>
> Determinism pair green on both drivers — a new field on `Particle` does
> not perturb replay. `ParticleSystem::step` runs once per frame from
> `App::update`, outside the CA sweep, so this path is driver-independent by
> construction rather than by test.
>
> **The other half landed with it (WP-6).** This preserves the worth of a
> corpse the blast *throws*; `EnergyLedger::meat_lost` now books the one it
> *consumes*, along with fire and the brush, so `max_standing_meat` is a
> real bound rather than a hope. The two were built together because they
> are two halves of one branch — booking a *throw* would charge for meat
> merely in flight and put the bound below the truth, and the guard
> `world.rs::a_destroyed_corpse_is_booked_rather_than_forgotten` asserts
> exactly that by carrying the in-flight term explicitly. The bar in this
> section's own test is worth *per surviving cell* rather than on the sum
> for the same reason: the total is allowed to fall, and what may not happen
> is a cell coming back cheaper.

**The original entry, kept as the record of what was inferred and what it
cost:**
Found by inspection during the `creatures-m18` merge review, **not created by
it**. `Particle` carries `material` and `shade` but not `aux`, and landing
writes `Cell::new(particle.material, particle.shade)`. Since S3, a `corpse`
cell carries what it is worth to eat in `aux` (`Material::worth_in_aux`), so a
corpse thrown by an explosion lands unstamped and falls through to
`corpse.ron`'s `food_energy` fallback: **a corpse worth 1,020 becomes worth
120, an 8.5x silent loss** on the one material whose value is per-cell.

**No existing guard can see it.** `EnergyLedger::max_standing_meat` is a `<=`
bound, so meat quietly going missing passes it, and `creature_biomass` is
asserted monotone non-increasing, which a loss also satisfies.

**Why it is listed now rather than fixed now.** The gap predates the merge —
`explosion.rs` was already throwing material at the merge base — but main is
the branch that made blasts actually throw debris, so the merge is what makes
it reachable in play. It has **not** been reproduced: nobody has measured how
often a corpse is inside a blast radius, and that is the first step.

The fix, when it is wanted: carry `aux: u16` on `Particle` and write it back
only when the landing material has `worth_in_aux`, or a wet soil grain will
land claiming to be food. `rigid.rs`'s `BodyCell` has the same shape and is
**not** a bug: it only ever holds `Solid`/`Plant`, and its `aux = 0` is
deliberate so a landing body does not silently re-attach.

### Y. ~~The gnome cannot get through the wood~~ — **FIXED 2026-08-23: one grain of soil was a wall**

> **RESOLVED, and the litter attribution below was a correlate rather than
> the cause.** `wood` now travels **357** cells against its bar of 200, on
> the merged build. The mechanism, found by instrumenting the rejection
> rather than reasoning about it:
>
> `rect_free` vetoed **any** powder above the wade line — a claim about
> walking into a drift, applied per cell. At the stuck frame the gnome was
> `grounded`, `lift_limit=4`, step-up working, and the rect he was trying
> to enter held **exactly one blocker**: a single `soil` cell at
> (108,194). Step-up could not clear it either, because lifting slides the
> offender *down* his body toward the wade rows, so a grain at `dy` wants a
> lift of `chest - dy` — this one sat at `dy` 5 wanting 5, against a
> `step_up` of 4. One row lower and nobody would ever have seen it.
>
> **This is why the two measurements below disagreed.** `litter.ron` has
> `decays_into: "soil"`, so shed foliage rots into a `Powder` and leaves
> loose grains scattered through and under the canopy. Disabling
> `shed_to_litter` bought 118 cells because it removed the *source* of
> those grains; `Material::insubstantial` bought exactly 0 because litter
> was never the blocker — the soil it rots into is. Both numbers were
> right and the attribution between them was not, and the entry's closing
> guess (tree architecture) was wrong.
>
> Fixed by counting powder **per row** instead of vetoing per rect
> (`Tuning::shoulder_grains`, 4 from a sweep over six start windows,
> confirmed by a blind A/B). A scatter is one or two cells in each of
> several rows; a drift's face is whole courses across his width, and still
> stops him at every setting the panel offers.
>
> **The bar was sound and is untouched.** What replaced the quarantine is
> case `8b`, which runs the worst-grown stand and gates 40 against a
> measured 50 — see bug C1 for the residual, which is a different mechanism
> and still open.


> **UPDATE 2026-08-23, measured on the `creatures-m18` merge: the 34 below
> no longer reproduces, and the litter attribution under it is now wrong.**
>
> Measured on `origin/main` (5515071) before touching anything, and again
> after the merge and after the port that adds `Material::insubstantial`:
>
> | | travelled | bar |
> |---|---|---|
> | `origin/main`, baseline this session | **98** | 200 |
> | + creatures-m18 merge (litter walks down, rots faster) | **98** | 200 |
> | + `insubstantial` (the gnome runs through litter) | **98** | 200 |
>
> Three separate builds, one number. So:
>
> - **The 34 is stale.** Main's own plant work moved it to 98 at some point
>   between this entry being written and 5515071, without anyone re-running
>   the case. Anything downstream that quotes 34 — including this entry's
>   own table, and the doc on `Material::insubstantial` as ijdlnp wrote it —
>   is quoting a world that no longer exists.
> - **`insubstantial` bought exactly 0 cells**, and that zero is recorded
>   rather than hidden. It was ported on the owner's direct instruction
>   ("make it so the gnome can run through leaf litter as if it was
>   nothing"), which is a gameplay-feel call and stands on its own; it is
>   simply not what this case measures.
> - **The residual 102 cells are not litter.** Litter is now 8x less hung
>   up (3,825 → 466 cells resting on plant tissue) and 81% of everything
>   shed rots away, and the number did not move at all. The remaining
>   attribution is tree architecture, as the section below already
>   suspected.
>
> Still open, still red against its bar of 200. No longer blocking on the
> ecology line.

`scripts/acceptance.sh`'s `wood` case fails on the merged branch:

```
gnome: at (43, 189), wading, travelled 34 cells, 36/98 cells behind foliage
FAIL: expected the gnome to cover at least 200 cells, he covered 34
```

**Attribution, because it took a controlled look.** `wood` is a case
**`main` added** — the suite had 16 cases at the first integration and 17
now, and `scene=wood` is absent from `9d3176c`'s `acceptance.sh` and present
in `origin/main`'s. It came in with the second integration and fails with
the plant lines merged. It is **not** caused by the Phase 1 worldgen work:
`scene=wood` builds from `common::PlantScene`, which is hand-built at
`SOIL_FIELD_CAPACITY` and never calls worldgen. Verified deterministic —
two runs, identical to the cell.

**A reporting error of mine, corrected here.** I reported acceptance as
green after the second integration. It was not: I read `tail -2` of a file
the suite was still writing and never saw the verdict line. The case has
been failing since `f424f98` with these exact numbers.

**Not "trees are walls" — that was checked and is false.** `Footing::Climb`
and `Material::climbable`/`fall_drag` are present and identical in
`origin/main` and here, so living plants are already walk-through and
climbable. The gnome-tree work is in.

**It is litter, and the split is measured.** Disabling `shed_to_litter`
(shed leaves vanish, as they did before the ecology line) and re-running:

| | travelled | reached |
|---|---|---|
| as shipped | **34** | x = 43 |
| litter disabled | **152** | x = 161 |
| bar | 200 | |

So **litter accounts for 118 of the 166-cell shortfall.** He reports
`wading` in both runs — that state means *overlapping loose powder*, not
water, and `wade_slowdown` cuts his horizontal speed for every cell of it.
The forest floor is now deep enough in shed leaves to bog him down. Note
`wade_rows = 4` is the point where wading stops and *stuck* begins, so this
has a cliff in it, not just a slope.

**A residual 48 cells is not litter**, and is the likelier home of the
shape argument below: even with litter off he reaches 152 against a bar of
200. `PlantScene` plants its first founder at **x = 56**, so the as-shipped
gnome stops **thirteen cells short of the first trunk** — blocked by
ground-level spread rather than by the trunk itself.

That is the same defect the absolute-standard review measured from the
other end. Judged against a clear bole and a foliage crown, the merged
stand scored **bottom crown band = 60** where a clear-boled tree reads
**0** — foliage running all the way to the ground — and a foliage centre of
58, which is a mound rather than a crown. **A tree with a clear bole leaves
a gap at ground level to walk under. A tree whose foliage reaches the
ground is a hedge.** The shape measurement and the gameplay failure are one
fact, and this case is what it costs.

Worth noting the case's own comment records the same class from the other
direction: it exists because a gnome *"travelled 0 cells and spent the run
BURIED, having been entombed by a crown that grew over the spot he was
standing on."*

**OWNER'S CALL, 2026-08-22: not the plant merge's to fix.** *"If the gnome
is just sinking a little into powders we can either remove that effect or
the player can jump out. Either way it doesn't seem like your
responsibility to fix."* So this is handed to whoever owns the player: the
options named are dropping the wading slowdown for shallow powder, or
giving the gnome a way out of it. Note the plant side is not blameless —
it is the ecology line's litter he is wading in — but the *response* lives
in `player.rs`, and `wade_rows`/`wade_slowdown` are its knobs.

**Two things that are true at once, and worth keeping for whoever takes
it.** The bar
(`min_travelled=200`) was calibrated on `main`'s trees, which are a
different shape — so it is partly the water-branch problem in miniature, a
constant measured against a world that no longer exists. But *"the gnome
gets through a wood"* is a gameplay property, not a calibration detail, and
if he cannot, that is a regression whoever set the number. **Not fixed
here**: the fix is the missing bole, which is the tree-architecture
programme, not a merge repair.

### X. A desert with no desert plants — **DECISION CARD WITH THE OWNER, 2026-08-23 (W2). Still: do not "fix" this by watering deserts.**

**The three levers are now costed against the code rather than estimated,
and two of the three costs on this page have changed.** Full working in
`Reports/grassfire-and-the-desert-2026-08-23.md` part two; the card is on
the owner's review queue. Nothing is implemented and nothing should be until
it comes back.

- **(a) sand gets a `water_capacity`.** The prerequisite this page names —
  *teach the liquid tallies about held water first* — **is already paid**:
  `weather::water_equivalents` counts held water under `MaterialKind::Powder
  if m.water_capacity > 0`, keyed on the field and not on a material name,
  so a second water-holding powder joins the ledger automatically. The
  conservation guards need re-running, not re-writing. The cost that *is*
  real is arithmetic and is not small: `update::plant_available_fraction`
  measures a cell against `SOIL_WILTING_POINT` (180) as an **absolute aux
  value**, not as a fraction of that material's own capacity — so a *small*
  capacity does nothing at all. At 150, a saturated sand cell is still under
  the wilting point, a plant gets exactly zero, and the world has bought an
  infiltration cost over every sand cell in it. **The threshold is 180
  before a plant gets one drop.** Also not desert-only: every beach and dune
  starts absorbing, darkening, and (as of W2) refusing to carry fire.
- **(b) roots reach the water table.** **There is no water table in the
  desert to reach.** `assets/worldgen.ron`'s `arid` preset sets
  `table_offset: 4000.0` — four thousand cells below the datum, off the
  bottom of the world, deliberately; `params.rs` names `arid` and `flat` as
  the two presets that put it past the world floor, and
  `tests/worldgen.rs`'s `the_dry_presets_keep_their_table_below_the_world_
  floor` guards it. So this lever begins by deciding to break that guard on
  purpose: the decision inside it is a *worldgen* one first — does the
  desert get a table at all — and only then a root-reach one. Give it one
  and the existing terms land
  it of the order of 90–100 cells down, which is Arc B4's taproot niche:
  **these two decide together.** Second-order: the aquifer-daylighting pass
  is switched *off* for `arid` by that same zero, not absent, so springs and
  seeps in a canyon wall come with it.
- **(c) stored rain.** Rain already falls on the desert and runs off, which
  is a flash flood and is correct. The lever is really *let a storm leave a
  decaying pulse of drinkable water behind*, and the engine already has the
  shape of the storage (`FieldTile::moisture_floor`, an authored lower bound
  evaporation may not cross, written once by worldgen for the aquifer). The
  cost is that **nothing in `assets/species/` can use it** — every plant
  here is a perennial that accumulates, and a desert annual is its own
  package. Largest of the three, and the one that buys the most distinct
  behaviour.

The rest of this entry stands unchanged and is still the reasoning the card
is built on.

### X (original). A desert with no desert plants — **DESIGN DIRECTION, 2026-08-22. Do not "fix" this by watering deserts.**

**CORRECTED 2026-08-22: the stated mechanism below was wrong, and the
correction changes what a fix would have to be.** This section originally
said an arid column lands near **50** against a wilting point of **180**, so
plant-available water is zero. The number is right and the *reason* is not.
Measured on the generator: arid lays a blanket of **7,411 cells** and the
moisture pass writes **0 cells** into it — because in arid country that
blanket is **sand, not soil**. `is_sandy` is `aridity > SAND_ARIDITY` (0.62,
`column.rs:78,92`) and arid's per-column aridity runs ~0.92, so essentially
every column is sandy; and **`soil.ron` is the only material in the whole
asset directory that declares a `water_capacity` at all**, so sand's is 0.

**Why that matters more than a factor of fifty.** An arid column is not dry
soil that a thirstier species could drink from — it is ground with **no
water-holding capacity whatever**. So the well-shaped fix recorded below
(make the wilting point a species trait) **would do nothing at all for the
desert**, which is the one biome it was proposed for: no wilting point,
however low, extracts water from a material whose capacity is zero. It
remains worth doing for the *gradient* between wetland and canyon, where the
ground really is soil at differing wetness. But a desert plant needs one of
three other things instead — sand given a small capacity, a root that
reaches the water table, or water stored from rain — and choosing between
those is the actual design question. **The tree being unable to live there
is still correct; the lever named below is simply not the one that opens the
niche.**

The worldgen soil baseline now scales `0 -> SOIL_FIELD_CAPACITY` by
`1 - aridity`, so an arid column lands near **50** against a wilting point
of **180** — plant-available water of **exactly zero**. Arid country is
genuinely dead, and for the tree that is correct: a tree should not grow in
a desert.

**But the owner's stated direction is that there should be different plants
for different biomes, including plants that can live in a desert** — and
as the engine stands **that is impossible by construction**, which is the
part worth writing down:

- `material::SOIL_WILTING_POINT` is a **single global constant**
  (`material.rs:67`), and
- `update::plant_available_fraction(cell)` takes only a `Cell`
  (`update.rs:991`) — **there is no species in scope**, so every plant in
  the world has the identical drought floor.

**State this precisely, because the loose version is wrong.** Species *can*
already differ in how they **cope** with shortage: `stomatal_reserve` sets
how early an individual closes its stomata and hoards, `drought_death` sets
how readily it sheds, and storage scales with root mass
(`water_capacity_of`). What no species can differ in is how much water it
can **get** from a given soil — that is the wilting point, and it is one
number for the whole world.

So a would-be desert plant can be given a huge reserve and no shedding, and
it will extract **not one extra drop** from dry ground; it will simply die
more slowly on the same zero income. And extraction is exactly the trait
that makes a xerophyte one: a cactus's trick is reaching water others
cannot, not enduring having none. In real biology the permanent wilting
point is species-specific — that is most of what being a desert plant
*means* — so this constant is modelling as universal a thing that should be
a trait.

**The change is small and well-shaped, which is why it is worth recording
rather than doing hastily.** One function signature gains a floor
parameter, three call sites in `plant.rs` pass it (all three already have
organism/species context: `absorb_water` at `:384`, and the two
root-scoring sites at `:3068` and `:3605`), and species files gain a
drought-tolerance field. `life_scatter` already thins placement by
`aridity`, so worldgen already knows which biome it is in — it just has
only `tree` and moss to place.

**The aridity baseline is the prerequisite for this, not an obstacle to
it.** Before it, soil was either bone dry or wet: a binary with no gradient
to adapt *along*. There is now a continuum from ~50 in arid country to ~570
in wetland, which is exactly what makes a lower wilting point worth having
— it buys a species somewhere to live that a tree cannot. **The dead desert
is the niche, not the bug.**

### W. The water-cycle branch and this one are two halves of one mechanic — **SEQUENCING DECIDED, 2026-08-22**

`origin/claude/water-phase-changes-ki6g8c` (tip `dcbdf7f`) is not adjacent
work. It builds **the half of the owner's model this branch records as
missing**: soil-to-air drying, with `SOIL_DRY_FLOOR =
material::SOIL_WILTING_POINT` and the rationale *"drying to zero would be
claiming that sunshine can do what a plant cannot"* — plus a **conserved
atmosphere** (`World::atmospheric_bank`, `spend_atmosphere`,
`storm_supply`). Their floor and this branch's zero-point are the same
number, reached independently.

**Land THIS branch first. The reason is not convenience.** Every constant
the water branch shipped — its `SOIL_SOAK_PER_DROP`, `STORM_RESERVE`,
`SOIL_DRY_PER_CHECK`, and its measured 0.76 supply equilibrium — was
measured in a world whose plants **had no `absorb_water`**, because that is
what trunk looked like when they forked. If water lands first, this branch
silently invalidates all of it. If this lands first, those constants get
re-derived against the consumer that actually exists. Note also that their
conservation tests run on **plantless** worlds, so they will pass while the
invariant is false in play.

Two further reasons: this branch is 0 behind trunk and can land now, while
theirs owes a 160-commit rebase regardless — and 11 of the 15 files a
dry-run merge conflicts on are *main-vs-water*, not plant-vs-water. And
sequential merges onto trunk beat a direct cross-merge, which would force
one person to resolve main-vs-water and plant-vs-water at once with no CI
midpoint.

**The good news, and it is substantial.** Because the drying floor is
exactly the plant zero-point, the pair is a **two-sided attractor**: bare
soil parks at exactly 180, and one rain strike (+10 at full intensity) puts
it at 190, which is immediately 2.3% plant-available. Once-wet soil then
oscillates just above the wilting point — a trickle after every shower,
nothing between. That is "some plants slow down where and when it is drier"
delivered as a **dimmer rather than a kill switch**, almost for free.

**Four things the merge must handle, in rough priority:**

1. **A resting seed shadows its own soil from BOTH rain and drying.** Their
   `is_damp_soil_surface` requires the cell above to be empty; their soak
   loop breaks at the first cell with no `water_capacity` — and a seed is
   that cell in its own column. So the planned "germinate when the soil
   below is wet" predicate would make a seed on dry ground **wait forever
   by construction**: rain physically cannot reach the one cell it polls.
   Fix before that predicate lands — give `seed.ron` a small
   `water_capacity`, or let the soak pass through a one-cell zero-capacity
   occupant. Same class as F1.
2. **`FLAG_MANAGED` goes live in production.** `Cell::is_empty` is
   byte-identical on both branches; what changes is that `rigid.rs` now
   reserves a promoted body's footprint with `Cell::EMPTY.with_managed(true)`.
   This branch's plant code was written under the explicit assumption that
   nothing promotes in production, so two deliberate raw-`EMPTY` checks
   become real: `growable()` lets roots grow **into** a floating body's
   reservation (and rigid has no demotion path), and Germinate's `resting`
   test reads a managed cell as "nothing holding this up", so a seed landing
   on one never germinates and never falls.
3. **Plant consumption is a one-way exit from the conserved cycle.** Soil
   and bank balance 1:1, but root uptake moves water into a pool the ledger
   cannot see, `transpire` vents to a non-conserved channel, and
   `absorb_water`'s Liquid arm destroys whole cells (F3). So the sky thins
   as the forest grows — *the forest that rain built drinks the sky dry*,
   over tens of thousands of frames. The cheap fix is to credit
   transpiration back to the bank.
4. **`transpire` has no wilting floor.** Bare soil can never go below 180;
   *rooted* soil can, all the way to 0, because transpiration subtracts
   without the check `absorb_water` makes. Expect dead halos around
   water-stressed stands that only bank-charged rain can heal.

**One thing the merge does NOT change:** the blocking ascii failure above.
That scene structurally cannot rain (its warmup runs no CA), so it is zero
leaves before and zero leaves after. The 10x soak cut does not touch it —
but it does make the intended fix roughly ten times more expensive: crossing
the wilting point from bone-dry goes from ~2 strikes to ~18.

### A. The slot-1 root spread has collapsed — **OPEN. Four explanations tried; the third was measured wrong too, and the lever now measures as dead.**

> **2026-08-23, P1: the third explanation is FALSIFIED by the counter this
> entry asked for, the guard has been recalibrated and un-quarantined, and
> the bug is still open. Read §P1 below before adding a fifth explanation** —
> `break_root_tips` fires around a hundred times per run in both arms, so
> nothing built on the amplifier being shut can be right.

> **2026-08-23, from the `creatures-m18` merge: this test flips with litter
> volume, and has still never been seed-swept.** Three measurements in one
> session, same machine, same build settings:
>
> | tree | draw -1 | draw +1 | spread | vs 10% bar |
> |---|---|---|---|---|
> | `origin/main` 5515071 (baseline) | 294 | 318 | 8.2% | **red** |
> | + creatures-m18 merge | — | — | — | *green* |
> | + `LITTER_FALL_REACH` 64 -> 512 | 354 | 378 | 6.8% | **red** |
>
> The sign never changes and neither does the failure mode (**superseded:
> the 2026-08-23 re-sweep above measures the sign flipping, 0.90 -> 1.035**);
> only the margin
> moves, and it moves by a couple of points either side of the bar as the
> volume of litter on the floor changes. **The green in the middle row is not
> a fix and must not be read as one** — it is one sample from a distribution
> straddling the bar, which is the exact shape `CLAUDE.md` warns about when a
> bar is set near a measured value.
>
> What this adds to the section below: the lever is not merely weak, it is
> weak enough that an unrelated change to ground cover moves it across the
> acceptance threshold. Any future attempt on this bug should **sweep seeds
> and report an order statistic** before believing either a red or a green.

> **2026-08-23, re-swept on `main` with litter in the world (the sweep §A
> asked for and had never had). The lever still measures as dead — and the
> claim below that "the sign never changes" does not survive.**
>
> `print_root_branch_slot_seed_sweep`, 8 seeds, both draws, 12,000 frames,
> one machine, one session, on `main` at `a0fa433` (these 18 commits touch
> neither `src/sim/plant.rs` nor `assets/species/`):
>
> | seed | root(-1) | root(+1) | ratio | clears the 10% bar |
> |---|---|---|---|---|
> | 1 | 354 | 378 | 1.07 | no |
> | 2 | 395 | 334 | 0.85 | no |
> | 3 | 308 | 346 | 1.12 | **yes** |
> | 4 | 285 | 300 | 1.05 | no |
> | 5 | 322 | 380 | 1.18 | **yes** |
> | 6 | 239 | 252 | 1.05 | no |
> | 7 | 254 | 239 | 0.94 | no |
> | 8 | 335 | 341 | 1.02 | no |
> | **mean** | **311.5** | **321.2** | **1.035** | **2 of 8** |
>
> Mean of the per-seed ratios **1.035, sd 0.102, SE 0.036**. That is **0.97 SE
> from 1.0** — still consistent with the lever being dead — 1.8 SE from the
> guard's 1.10, and **8.2 SE from the calibrated 1.33**, which it excludes as
> firmly as the 2026-08-22 sweep did. So the conclusion is unchanged and the
> quarantine stands.
>
> **What has changed is the sign, and the sentence below saying it never does
> is now wrong.** The 2026-08-22 sweep read a mean ratio of **0.90** —
> inverted, root(-1) beating root(+1) — and this one reads **1.035**, weakly
> the right way round. Seed 1 alone went 371/315 (0.85) then and 354/378
> (1.07) now. The direction is not a stable property of the bug; it wanders
> with the same ground-cover changes the margin does. **Neither a red nor a
> green nor a *sign* here means anything from one seed.** The guard itself is
> red at seed 1 as recorded (354 vs 378 = 6.8% against a 10% bar), which
> reproduces `5a9e594`'s figures exactly.
>
> Also down, and not obviously part of this bug: **absolute root cell counts
> fell about a fifth** across the sweep, mean 431.0 → 311.5 at draw -1
> (−27.7%) and 388.9 → 321.2 at draw +1 (−17.4%), same probe and same frame
> budget. Recorded here because it is the sort of thing that later reads as
> having always been true.
>
> Time-boxed per the implementation handoff's WP-3 and stopping here: the
> remaining fix is the plant genome's primed-site repair, which is model work
> over procedural content and belongs to whoever owns the plant line.

**Settled by seed sweep, 2026-08-22 — it is NOT a flaky guard, so do not
move the bar.** My third explanation was that the test is single-seed
(`root_slot_run(1, 1, ±1.0, 12_000)` — seed 1 both arms) over a system whose
spread is famously enormous, and so could not tell "the lever broke" from
"this seed reshuffled". House convention wants an order statistic over N
seeds. That reasoning was sound and the answer came back the other way.
`print_root_branch_slot_seed_sweep` (ignored probe, `plant.rs`), 8 seeds,
both draws, 12,000 frames each:

| seed | root(-1) | root(+1) | ratio |
|---|---|---|---|
| 1 | 371 | 315 | 0.85 |
| 2 | 444 | 421 | 0.95 |
| 3 | 395 | 362 | 0.92 |
| 4 | 397 | 457 | **1.15** |
| 5 | 628 | 390 | 0.62 |
| 6 | 491 | 422 | 0.86 |
| 7 | 469 | 508 | 1.08 |
| 8 | 253 | 236 | 0.93 |
| **mean** | **431.0** | **388.9** | **0.90** |

**1 of 8 seeds** clears the guard's 10% ordering. Mean of the per-seed
ratios is **0.92, SE 0.056** — that is 1.4 SE from 1.0, so the data are
**consistent with the lever being dead**, and 7 SE from the calibrated
**1.33**, which they firmly exclude. Whether it is exactly dead or slightly
inverted cannot be resolved at n=8; either way it does not do what it is
asserted to do.

**Do not read the draws' non-identical output as "the lever is connected".**
Before the primed-site repair both draws were *bit-identical* at 352. They
now differ per seed — but changing a genotype draw also perturbs the RNG
stream, so scatter alone is not evidence the mechanism responds. The
calibrated 33% ordering is the evidence, and it is gone.

**What this costs, stated plainly:** re-deriving `tree.ron` against the
current quantity is the shape of fix `CLAUDE.md` describes ("fixing a bug
often exposes a constant that was compensating for it"), but that is model
work over procedural content, and per house rule it needs the seed sweep
built *before* the change — which now exists. Original diagnosis follows.

**(was) The slot-1 root spread has collapsed, and the first two explanations
were both wrong — OPEN, 2026-08-22**

**Found by the merge that brought the plant lines onto `main`, not by a
playtest.** Two of the plant line's own tests fail after the merge and pass
on `plant-substrate-v2` alone. Both were controlled, so this is measured
rather than suspected.

| test | `plant-substrate-v2` alone | + main (step 2) | + ecology (step 3) |
|---|---|---|---|
| `a_tree_eventually_stops_growing` | plateaus at 565 cells (~frame 50,000) | **1,929 and still climbing at 120,000** | **passes again** |
| `root_and_shoot_branching_read_different_slots` | 336 vs 448 root cells, a 33% slot-1 spread | 411 vs 437, a 6% spread | **440 vs 448, a 1.8% spread** (bar is 10%) |

Controls: both pass on `plant-substrate-v2` alone in 35 s; every figure
above reproduced bit-identically across runs, so this is deterministic and
not load or seed noise.

**The termination failure fixed itself when the ecology line landed on
top**, which is worth more than the fix: it says the missing quantity was a
*sink*. `plant-ecology-design` sends abscised foliage to `litter` instead
of deleting it, and with that the tree plateaus again. So growth was not
running away because income rose without bound — it was running away
because nothing was taking mass back out. Whatever is done about the row
below should be judged against that, not against a carbon number in
isolation.

**What is left is the slot-1 spread, and it is getting narrower, not
wider** — 33% → 6% → 1.8%. The trait still orders root mass in the right
direction at every step; what has collapsed is the *size* of the effect,
which is what the bar was set to detect. That is the shape of a signal
being swamped rather than a mechanism being broken.

**Two explanations have now been offered and BOTH are falsified. Read this
before proposing a third.**

*First explanation — "main rewrote `field.rs` by +553/-44".* True, but it
was a diff statistic dressed up as a mechanism. It named no code path.

*Second explanation — "main added weather, so it rains into scenes
calibrated dry".* This one had a real mechanism behind it and survived a
code review: `weather::step` is the first call in both drivers
(`update.rs:76`, `parallel.rs:104`), the plant harness `run_with_fields`
drives `update::step`, and `root_slot_run`'s bed is open sky. Every step of
that is true.

**It is still wrong, because it never rains during this test.**
`weather::step` reads `at(world.seed, world.frame)`; `root_slot_run` pins
`w.seed = 1` and starts at frame 0. `weather::at` is a pure function of
those two, so the question is answerable without stepping a world at all —
which is what `print_dry_window_for_the_slot_seed` (this file's own control,
in `plant.rs`'s test module) now does:

```
seed 1: frames 0..12000 — 0 of them precipitating (0%)
  epoch 0 (frame 0):     None  intensity 0.00
  epoch 1 (frame 7200):  None  intensity 0.00
  epoch 2 (frame 14400): Rain  intensity 0.83   <- the first rain, after the test ends
```

**The first precipitation at seed 1 arrives at frame 14,400, and the test
stops at 12,000.** Rain cannot be the cause. Neither can evaporation (the
bed holds no `Liquid` at all, and `Material::evaporates` is Liquid-only), nor
the soil-moisture ratchet, which needs rain to ratchet.

**Third explanation, and this one is measured rather than argued.** A
paired `plant_probe species=tree trees=8 frames=12000` on
`plant-substrate-v2` against the merged tree — same scene, same harness,
one run each:

| | substrate-v2 | merged | |
|---|---|---|---|
| plant cells, mean | 3440.9 | 3435.0 | **unchanged — it is not income** |
| **root** cells, mean | 288.2 | 219.8 | −24% |
| **root** cells, max | **745** | **287** | −61% |
| root cells, range | 114–745 (6.5x) | 129–287 (**2.2x**) | **the spread collapses** |
| uptake / tick, mean | 16.46 | **27.43** | +67% |
| water stock, mean | 657.9 | 440.4 | −33% |
| **stomatal term, mean** | **0.90** | **0.96** | **crosses a threshold** |

Read the last row against `ROOT_REINITIATION_STATUS = 0.95` (`plant.rs`),
which `break_root_tips` tests as `if status >= 0.95 { return }`.

**`break_root_tips` is the amplifier, and main's world switches it off.** It
re-initiates a `RootTip` from mature root tissue, once per organism per
upkeep tick, and it is **genotype-blind** — slot 1 does not reach it. On
`plant-substrate-v2` the mean stomatal term is 0.90, under the gate, so it
fires routinely and multiplies the stepping lineages that *consume* primed
sites; that is what turned a difference in priming density into a 33%
difference in root mass, and what produced the 745-root outlier. In the
merged world plants take up 67% more water per tick and meet demand at 0.96
— over the gate — so the amplifier stays shut, root systems shrink and
converge, and slot 1 is left moving only the supply of sites in a plant that
no longer converts many of them.

Note what this is *not*: not more carbon (plant size is identical to within
0.2%), not rain, not evaporation. It is water reaching roots **more
efficiently**, which is a change in main's field/soil path — the same
`field.rs` rewrite the first explanation gestured at, but now with the
specific quantity (uptake per tick) and the specific consequence (a gate
crossing) attached.

**Judged by eye, 2026-08-22, and the alarming reading was wrong.** A
before/after of the stand was rendered at a dry noon frame and put to the
owner. The session's own reading was that the merged canopy had closed into
a continuous slab and the roots into a surface mat -- i.e. the "canopies
merge into a slab" failure `Reports/tree-architecture-research.md` exists
for. **The owner's verdict, given directly:** the new trees look *"a little
different, fatter merging a bit more, not wildly different"*, and the roots
*"a little different but not obvious given the plant to plant variability
that already exists."*

That is a much smaller claim than the one it replaced, and it changes what
this entry is. The −24% root mean and the collapsed max are real numbers,
but they do **not** cash out into a player-visible regression: they sit
inside the spread twelve identical genomes already produce (31 to 153 cells
in the recorded census). So §A is a **test-calibration problem**, not a
symptom of the world looking wrong -- which is the right prior for anyone
deciding how much to spend on it.

Recorded here because the session that rendered it argued itself into the
larger claim from two real measurements plus one picture, and only the
owner's eye cut it back. The card is
`20260822T081525474Z-8c4bc2` on the `plants` board and is still open in the
queue; this verdict arrived in conversation rather than through the tool.

**Still not confirmed, and this is the measurement that would do it:** a
direct count of `break_root_tips` firings per run on each branch. The
mechanism above is inferred from a threshold crossing in an aggregate mean,
and a mean can cross while the distribution that matters does not. Counting
the firings is a `#[cfg(test)]` counter at `plant.rs:3017` and one paired
run — the `S8E` atomic-array pattern in the same file is the template.

**The lesson worth keeping, because it cost two wrong answers.** Both
explanations were reached by reading diffs and reasoning about mechanism,
and the second one passed an independent code review. The thing that
settled it was a **pure function evaluated over the exact seed and frame
range the test uses** — a control that took one probe and no world stepping.
Ask "does this mechanism fire in *this* run" before "could this mechanism
cause this".

**Deliberately not fixed by the merge session.** The two available fixes
are re-deriving `tree.ron`'s constants against main's field model — a
retune over procedural content, which `CLAUDE.md` says wants a seed sweep
first and is a design decision — or moving a bar that was set from
measurement. Both are the owner's call. Recorded here so the next session
does not re-derive the diagnosis.

**The cheap next step, and it is now a different one.** Since weather is
excluded, the question is which of main's *remaining* changes moves a
plant's carbon income in a rain-free world. `examples/plant_probe.rs` runs
on both branches unchanged, so a paired
`plant_probe species=tree trees=8 frames=12000` on `plant-substrate-v2`
against the merged tree measures exactly that, in two runs and no code
change. If the merged trees are simply bigger, the income hypothesis holds
and the field solver is the place to look; if they are the same size, the
cause is inside the root pass and the priming sweep becomes the next move.

**What is *not* wrong:** the merge resolutions themselves. The slot
allocator, the species registries and the scheduler dedup sets were each
audited against both parents; the only real defect found was a scene error,
below.

### B. ~~`anchor_support` runs over creature organisms, unguarded~~ — **FIXED 2026-08-22. Churn, not damage; the guard is in.**

**A collision only the merge could produce.** `plant::anchor_support`
arrived on `plant-substrate-v2`; ants, beetles and worms arrived on `main`;
neither line ever had both. `plant::step_organisms` iterates
`world.live_organism_ids()`, which is **every** organism in the shared
generational storage — creatures included — and `anchor_support` guards
only on `state.cells.is_empty()`.

Creature cells really are in that map: `World::reindex_organism_cell`
inserts into `OrganismState::cells` for any organism whose id a cell
carries, not only plants.

So for a creature: `is_structural_anchor` wants a `Solid` 4-neighbour (the
`root_tissue` arm cannot fire — creature materials do not
`reinforces_powder` and a creature cell is not a `RootTip`), an airborne or
soil-surrounded creature reaches none, every cell settles at `u16::MAX`,
and since `was` defaults to 0 the `dist[i] > was` arm fires
`schedule_structural_check` on **every creature cell, every organism
tick**.

**A correction, because the first version of this entry got the reason
wrong.** It said the sibling pass `accumulate_support` is safe because it
returns early on `state.collar_y == None`, "which a creature never has."
**That is false after one organism tick.** `organism_upkeep` also runs over
creatures unguarded, and its census walk sorts every cell that is not a
`RootTip` and does not `reinforces_powder` into the shoot branch — which is
every creature `Head`/`Segment` cell. It then writes `shoot_cells`,
`collar_y` and `shoot_top_y` onto the creature's own `OrganismState`
(`plant.rs`, the `state.collar_y = collar_y` write at the end of that walk).
So from a creature's **second** organism tick, `collar_y` is `Some(...)` and
`accumulate_support` runs its full BFS on it too.

The consequence is still churn rather than damage, and the reasons are
elsewhere: creature species declare no behaviours, so nothing dispatches;
`settle_water` at demand 0 returns status 1.0; `break_root_tips`,
`break_buds` and `allocate_to_frontier` all bail on the missing `Grow`
entries; and `transport` builds `Plant`-kind topology only, so a creature
has no faces. But **six** plant passes run over every creature, not one,
and each writes something.

**It is wasted work, not damage — and that correction matters.** The
first reading of this was that it risked the amputation `CLAUDE.md` warns
about, where a structural check fired mid-organism converts everything past
the span limit to deadwood. **It does not**, and the reason is one line:
`structural::tick` bails at `is_body_material`, which is
`Solid | Plant` only, and creature materials are `kind: Creature`. Every
site scheduled this way is discarded on arrival. Nothing is converted,
nothing is broken free, no creature is taken apart.

Two further limits, both checked: `nest.ron` is `kind: Solid`, so ants
standing on the nest patch *are* anchored and never schedule at all; and
`schedule_active_site` dedups against `pending_decay_sites`' sibling index
for structural checks, so a cell cannot stack duplicates. The cost is
bounded, not unbounded.

So what is left is **scheduler churn on exactly the colony scenes the
creature line added its cost instrumentation for** — a structural check
enqueued, popped and thrown away, per creature cell, per organism tick.
That is worth a guard, and it is worth *not* being alarmed about.

**Evidence level: the mechanism is read and verified** — the iteration is
unguarded, `reindex_organism_cell` really does put creature cells in
`OrganismState::cells`, `OrganismCell::default()` really does reset
`support` to 0 on every move, and the discard really is unconditional.
**Not measured:** how many sites this actually costs per frame in a live
colony. `World::live_organism_count` and the existing creature counters
would say in one run, and nothing has asked them.

**Fixed as described**, in `plant::step_organisms` after the cadence check:
one species lookup, keyed on the **`creature` field** — the declaration of
intent — skipping all seven plant passes for creature organisms.

Two things it deliberately does not do. It does **not** key on `collar_y`:
per the correction above the plant side sets that on creatures itself, so
such a guard would switch itself off on the second tick and still look like
it was working. And it does not guard `anchor_support` alone, which would
have fixed the one pass this entry was named for and left five others
running.

**The slot-reclaim arm must stay outside the guard.** It is the one part of
`step_organisms` that is genuinely for every organism, and a creature death
path that empties a cell list between ticks relies on it. A bare `continue`
at the top of the loop would leak the slot and resurrect
`pixel-physics-issues.md` #8, which is the bug this whole allocator exists
to close.

### C. ~~`grass` and `creeper` root branching is running a retired model~~ — **MEASURED AND CLOSED, 2026-08-22. Both knobs fire. Do not zero them.**

**A legitimate question with the opposite answer, kept in full because the
reasoning that produced the wrong prediction was good reasoning.**

The concern: `plant-substrate-v2` measured that a root tip's *in-tick*
`branch_chance` roll cannot be funded — the tip must hold two steps' carbon
at once, and over 351 tree root steps the gate opened **twice** and the roll
fired **zero** times. It replaced the mechanism with `branch_priming` and
set root `branch_chance: [0.0]` in tree, conifer and shrub.
`plant-ecology-design`, developing in parallel, authored `grass` (0.4) and
`creeper` (0.05) with no `branch_priming` at all. The two lines edited
different species files, so this auto-merged in silence.

The prediction, from the shared gate: grass *might* differ, creeper is
"near-certainly inert" since its `cost: 0.25` gives it the tree's exact
≥0.50 gate and its 0.05 sits beside the 0.04 that never fired.

**Measured instead of argued, by running each species paired against its own
`branch_chance: [0.0]` and comparing output — deterministic, so identical
output would have proved the knob dead:**

| | as-shipped | knob zeroed | verdict |
|---|---|---|---|
| grass (sod bank test) | **137** grassroot cells, crest +27% | **55** grassroot, crest +23% | **fires hard — zeroing costs 60% of the mat** |
| creeper (`plant_probe`, 8 plants, 12k frames) | mean 204.1 cells | mean 202.5 cells | **fires weakly — 3 of 8 individuals moved, ~0.8% mass** |

**So both knobs work, and both proposed fixes were regressions.** Zeroing
grass's roll would have cut the fibrous mat its `reinforces_powder` bank
depends on by more than half. Zeroing creeper's would have been a smaller
regression, but a regression.

**Why the prediction failed, which is the part worth keeping.** The
inference transferred the *gate* (identical `cost` ⇒ identical ≥2× bar) and
silently assumed the *economy* transfers with it. It does not. A tree's
0.053 mean-held carbon was a 2,400-cell canopy's income diluted across a
large, distant frontier; grass's whole shoot photosynthesises, its frontier
is ≤22 cells, and its cost is 0.15 — a ≥0.30 bar it clears routinely. Even
creeper, which really does carry the tree's 0.25 cost, clears it sometimes,
because a ground-hugging plant's source-to-root path is short.

**A measurement on one species does not transfer to another through a shared
constant.** The constant was shared; the economy that has to pay it was not.

**Left as it is.** Nothing to fix. What remains open is a *documentation*
gap rather than a defect: `grass.ron`'s comment justifies 0.4 by comparison
to "a tree root's 0.04", a value that no longer exists anywhere — worth
rewording to cite this measurement instead, next time that file is touched.

### D. ~~Two smaller things the merge exposed~~ — **BOTH RESOLVED, 2026-08-22**

**E1. The repaired creature bed is damp but still has no floor and no
walls.** `eating_one_leaf_does_not_kill_the_tree_that_grew_it` fills soil
into `y=150..159` of a `0..199` world and plants on top. Soil is a
`Powder`, nothing floors or walls the bed, so it avalanches ~40 rows to the
world floor and the seed rides down with it. The test passes — dampening
the bed was enough to make the tree leaf — but it passes *despite* the
scene, not because of it. `plant::tests::plant_tree_on_ground` walls **and**
floors its bed, with a comment saying this exact error has cost time twice.
**Fixed 2026-08-22**, once the review pointed out that a test passing
*despite* its scene hands the next change a bed that does not stay where it
is put. Floor **and** walls, matching `plant_tree_on_ground` — a floor alone
still lets an open-sided bed spill off its own edges, which that helper's
comment records as having cost time twice. Still passes, in 2.96 s.

**E2. A bar in the ecology line's sod test predates the substrate line's
root economy.** `sod_crest > bare_crest * 1.10` is justified in-file by a
paired same-session measurement (bare 185 → sod 235, +27%, 135 `grassroot`
cells in the bank). Those runs happened on `plant-ecology-design` before
the stomatal reserve, the primed-site conversion and the root
`branch_chance` supersession existed — all three of which move how much
`grassroot` the sod arm grows, which is the quantity the margin is made of.
**Re-measured 2026-08-22 on the merged tree, and the provenance is
restored**: shed bare 327 / sod 305 (-7%), crest bare 185 / sod 235 (+27%),
**137** grassroot cells against the recorded 135. The recorded pair
reproduces almost exactly, so the bar is still a measurement of the system
it guards. Worth knowing *why* it barely moved: the sod scene is short and
its outcome turned out to be insensitive to everything the merge changed —
which is itself the reason the §C probe below had to vary the knob directly
rather than trust this test to reveal it.

### E. A test scene can outlive the economy it was written for — **FIXED 2026-08-21, kept for the reasoning**

`creature::tests::eating_one_leaf_does_not_kill_the_tree_that_grew_it` built
its bed as `Cell::new(soil, 0)` — and `aux == 0` is *dry* on a `Powder`.
That was fine while a plant ran on one currency: `main` has no
`absorb_water` at all. The plant line makes water a real second currency
with a real source, so a root in dry soil has **no income**: the tree grew
wood, never a leaf, and the test failed on its scene rather than on the
organism-freeing behaviour it is named for. Dampened to
`SOIL_FIELD_CAPACITY`, matching `plant::tests::plant_tree_on_ground`, which
has always done this — passes in 3.26 s.

Same class as the moss scene `main` repaired when evaporation landed, and
the third time `CLAUDE.md`'s "a scene that contradicts the code will look
like a bug in the code" has been paid for. **When a merge brings a new
currency, every scene that grows something is a scene that may no longer
supply it.**

### F. Cross-line seams neither branch's tests exercise — **OPEN, 2026-08-22**

Two plant branches developed for 111 commits while main added creatures,
weather, evaporation and a rewritten field solver. **The merge conflicts
were the easy part** — an independent three-way review found no runtime
defect in any of them. The risk is where a plant-line mechanism meets a
main-line one, which no test on either side covers. What follows was read
off the merged source; where something is measured it says so, and where it
is inference it says that too.

**F1. A litter blanket blocks rain from reaching the soil — ~~LIVE, verified~~ FIXED 2026-08-23, see §P1 below.**
`weather::step`'s soak loop walks down from the surface and `break`s at the
first cell whose `water_capacity == 0` (`weather.rs:482`, whose own comment
explains it as "a puddle on bare rock does not wet the rock beneath it").
`litter.ron` and `grassblade.ron` declare **no** `water_capacity`, so it
defaults to 0 — and cell `aux` is the only channel roots drink from. A
column topped by shed litter therefore takes **zero** soak. Real mulch
conserves soil moisture; this does the opposite, and the blanket deepens
fastest exactly over rooted ground, where root `deplete_moisture` also holds
the field dry enough to slow litter's own decay. *Measure:* paired storm
over littered vs bare soil, summing soil `aux` after one epoch.

**F2. Snow defoliates canopies through the shade-death rule — LIVE, inferred.**
Snow is placed one cell above the topmost non-empty cell, which for a treed
column is the crown; snow is non-empty, so it attenuates light; `tree.ron`
leaves carry `shade_death: 0.003` rolled as `0.003·darkness³` per organism
tick. A snow epoch is ~100 organism ticks. Nobody designed deciduous
winters; they may be lovely. *Not measured, and the field's 8x8 block
resolution may blunt a 1–2 cell snow cap.* *Measure:* paired leaf census
across a snow epoch vs a clear one, same seed.

**F3. Root drinking destroys water unconserved — ~~LIVE, verified by reading~~ FIXED 2026-08-23, see §P1 below.**
`absorb_water`'s Liquid arm sets the adjacent cell to `Cell::EMPTY` and
credits at most `rate` — the cell's remaining fill is destroyed, not
transferred. That was tuned on branches where ponds never evaporated; main
added evaporation drawing down the same ponds. Nothing tallies held water,
so the loss is silent. *Measure:* pond volume vs time, 2x2 over
tree/no-tree and weather/no-weather, plus a conservation tally on that arm.

**F4. Grass cannot die, and the guard that would have caught it was removed
— ~~LATENT~~ FIXED 2026-08-23, see §P3 below; read that before acting on
this entry, which also gets the grass economy a third wrong.** Both abscission
rules gate on `CellType::Leaf`; grass has `plastochron: 0` and never makes
one, so it has no shade death, no drought death, no age death. Separately,
the "do not germinate on another plant" guard was deleted on the explicit
argument that a mis-sited seedling "is shed leaf by leaf by
`drought_death`" — a cleanup that does not exist for grass. A grass seed
landing on a branch, a stone, a litter drift or a nest roof would germinate,
never root, and stand forever, holding an organism slot (reclamation
requires an *empty* cell list). At the 4,095 ceiling `push_organism`'s range
check is a `debug_assert` and `encode_organism_id` does not mask, so the
index would bleed into the generation bits — **silent organism identity
corruption in release**. Today worldgen plants only `tree` and moss and the
brush only plants trees, so nothing reaches it. *Measure:* count organisms
with `root_cells == 0 && shoot_cells > 0` on a grass stand under canopy at
30k frames, and the slot high-water mark.

**F5–F8, in brief.** (F8 is **FIXED 2026-08-23** — and its stated cause was
wrong; see §P1 below before acting on the sentence about it here.) Grass seeds are ant food and a nest-dropped seed loses
its organism id, so a colony beside a sward is an unbounded larder and a
sink on grass recruitment (LATENT with grass). Decay's settle-scan schedules
a whole chunk's cohort at the same `next_frame` where evaporation
deliberately staggers by a position-derived phase — a 200-frame comb, cost
not correctness, unmeasured at forest+pond+storm scale. Soil `aux` has three
sources and exactly one sink (root uptake): there is no soil-to-air drying,
so unplanted soil ratchets toward field capacity across rain epochs
(*measure:* sum soil `aux` over 10 epochs on a plantless world — monotone
non-decreasing confirms it in one number). And `reinforces_powder` does not
stop digging, only avalanching, so ants can hollow a sod bank into a lattice
that never collapses.

### P1. The water book, the root-tip counter, and what they said about §A and §U — **2026-08-23**

Package P1 of the plant implementation split (`Reports/plant-implementation-
split-2026-08-23.md`). Four of the entries above move; two of them move in a
direction nobody expected, and those two are the ones worth reading.

**§F3 is closed.** `absorb_water`'s `Liquid` arm wrote `Cell::EMPTY` and
credited at most `rate`, so a full 1,000-fill water cell was destroyed to pay
for 1.5 units of plant water. It now takes what it drinks and leaves the rest
as partial fill, at the exchange the `Powder` arm already uses
(`SOIL_UPTAKE_PER_TICK` of a cell's 0..1,000 store per `rate` of plant water —
and `LIQUID_FULL` and `SOIL_SATURATED` are the same 1,000, so the two arms are
now one currency). Measured on one drink from one full cell, same build:

| | fill taken | water credited | fill per unit of water |
|---|---|---|---|
| before | **1,000** | 1.50 | **667** |
| after | 60 | 1.50 | **40** |

40 is `SOIL_UPTAKE_PER_TICK / rate` exactly. **Income is unchanged** — the
plant still gains at most `rate` per tick per wet neighbour — so this is a
conservation fix and not an economy change. Guard:
`a_root_leaves_the_water_it_did_not_drink`.

*§F3's own 2x2 (tree/no-tree x weather/no-weather over pond volume) was built
first and does not work, which is worth recording.* Free water standing
against unsaturated soil **infiltrates**, so any pond within reach of a root
system drains into the bank far faster than anything drinks it, and the scene
measures infiltration wearing absorption's clothes. Three geometries were
tried and each measured zero: a tank under a stone shelf (the root stops a row
short of the water), a tank under a *punched* shelf (a seed is a `Powder` and
falls through the hole), and a sealed pocket inside the bed (infiltrated away
to nothing inside 1,500 frames). Driving the arm directly is the honest
measure. Related, and not touched because it is not this package's:
`roots_consume_adjacent_water` asserts that `w.get(50, 22)` is no longer
`WATER` in the first of those geometries, and the water there drains into the
tank on its own within a few frames — so it may be passing for that reason
rather than for its own.

**§F1 is closed.** `weather::step`'s soak loop stopped at the first cell whose
`water_capacity == 0`. That is right about rock and wrong about everything
that merely *lies on* soil, and litter declares no capacity. A drop now
crosses up to `SOAK_COVER_REACH` cells of loose cover — a `Powder` or a
`Plant` cell, i.e. litter, grass, sand, ash, lying snow — and starts its
`SOAK_DEPTH` profile at the first cell that can actually hold water. `Solid`
still stops it, and so does a gap, so **canopy interception is unchanged**: a
treed column's surface is its crown and the cell under a leaf is air, so a
drop still stops in the canopy. Changing that is a rain model, not a bug fix.

Paired storm, seed 4, 400 frames, same session and machine, soil `aux` gained:

| | before | after |
|---|---|---|
| bare bed | 4,295 | 4,295 |
| littered, the bed's own ten rows | **15 (0.3%)** | **1,073 (25%)** |
| littered, every soil cell in the world | 3,829 | 5,352 |

**Read the middle row, and note why the bottom one lies.** World-wide, the
littered arm was *already* taking 89% of the bare arm's water before the fix,
because litter rots into soil where it lies and a rotted cell has capacity —
so the column soaks into the blanket's own remains while the ground beneath
stays sealed. A world-wide metric reports this bug as nearly absent. The bed
is the thing §F1 says takes zero, and it took 0.3%. After the fix the littered
column holds *more* total water than the bare one, which is what mulch is for.
The after figure is a quarter rather than a whole because the soak profile now
starts at the rotted cell, one to three rows above the original bed — correct
behaviour, and the reason the guard's bar is a tenth of the bare arm rather
than most of it. Guard: `rain_soaks_through_a_litter_blanket`.

**§F8 is closed, and its stated cause was wrong.** §F8 says "there is no
soil-to-air drying". There is: `evaporation::tick_soil` dries a damp soil
surface and credits the atmosphere for exactly what it removes, and
`schedule_damp_soil` puts cells on that schedule from both places soil gets
wet. It also *ran* — 19,388 soil checks on seed 1 over ten epochs — and it was
**not** §1m's humidity shadow either: **3** of those 19,388 read becalmed.

The sink was busy and had nothing it was allowed to touch. It dried the
surface cell and only the surface cell, on the reasoning that soil under soil
"gives it up to the surface by capillary flow". Capillary flow does not do
that: `update.rs`'s exchange deliberately rests once the gradient falls under
`SOIL_CAPILLARY_REST` (`SOIL_SATURATED - SOIL_FIELD_CAPACITY` = 380), and that
band is **wider than the range the sink can pull** (`SOIL_FIELD_CAPACITY -
SOIL_WILTING_POINT` = 440). So the profile parked at "surface at the wilting
point, everything under it at up to 560", the surface cell then failed
`is_damp_soil_surface`, its site retired, and the bed held what it had for
ever. That is the shape `CLAUDE.md` records as *a constant compensating for a
bug* seen from the other side: two correct-looking rules whose rest states do
not overlap.

The fix is `SOIL_DRY_REACH`, set equal to `weather::SOAK_DEPTH`: a drying
*front* descends through the same few rows the rain reached, one cell per
check, at a rate falling as `1/(d+1)` — the soak's own profile, run backwards.
What the rain wets, the sun can take back; what drained deeper is the water
table and correctly does not evaporate.

Plantless 128-wide bed, ten epochs, three seeds, summed soil `aux`:

| seed | before | after |
|---|---|---|
| 1 | 230,400 -> 463,927, **never once falls** | 230,400 -> 308,067 -> 236,121, falls five times |
| 4 | 232,038 -> 233,802, then flat to the last frame | rises and returns to 230,521 |
| 7 | 240,000 -> 243,650, then flat to the last frame | rises and returns to 230,400, its own floor |

Guard: `unplanted_soil_gives_water_back_to_the_air`, which is §F8's own test
with its sign flipped — before, all three series were monotone non-decreasing;
after, none is. **There is no bar to set**, which is worth more than a
well-chosen one.

**§A: the amplifier is NOT shut, and the lever is dead centre.** §A's third
explanation — main's field model raises uptake 67%, the mean stomatal term
crosses `ROOT_REINITIATION_STATUS`, `break_root_tips` stops firing, the slot-1
spread collapses — is **falsified by the counter it asked for**. Exit histogram
over `root_slot_run(1, 1, +-1, 12_000)`, one run per arm:

| draw | root | shoot | calls | gated | at_cap | no_cand | poor | **FIRED** |
|---|---|---|---|---|---|---|---|---|
| -1 | 354 | 2246 | 313 | 214 | 2 | 1 | 0 | **96** |
| +1 | 378 | 2093 | 291 | 139 | 43 | 1 | 0 | **108** |

It fires around a hundred times a run in both arms. §A's own closing note
anticipated this — "a mean can cross while the distribution that matters does
not" — and that is exactly what happened: the mean sits at 0.96, over the gate,
while a third to a half of individual calls are under it. **Do not offer a
fourth explanation built on the amplifier being off.**

What the histogram does say is that the two draws differ at `at_cap` (2 against
43): the +1 arm spends far more of its calls already holding `max_active_tips`.
That is a *cap* difference, not an economy one, and it is where a fifth
explanation should start looking.


**§U does not reproduce, and its named mechanism is nevertheless real.** Two
separate findings, and conflating them is how this entry got written the first
time.

*The outcome is a single-seed artifact.* §U reports 982 cells and 428 wood on a
nearly dry bed against 734 and 299 at field capacity — drought growing a bigger
tree, backwards from dendrochronology. Swept over 8 seeds on
`plant_tree_on_ground`'s bed (the one §U's cell counts point at), 12,000 frames,
dry 310 against field capacity 620:

| | dry | wet |
|---|---|---|
| mean cells | **2,102** | 2,423 |
| mean wood | **1,146** | 1,362 |
| seeds where drought grew a bigger plant | **3/8** | |
| seeds where drought grew more wood | **2/8** | |

The means go the *right* way — drought costs 13% of mass and 16% of wood — and
a majority of seeds agree. §U as filed predicts 8/8 on both. `CLAUDE.md`:
compare two runs, not one run against a remembered number; a bed whose twelve
identical trees span 31 to 153 cells will hand you either sign if you take one
sample. Reproduction: `print_drought_size_seed_sweep`.

*The missing penalty is real and is now measured.* §U's unproven mechanism was
that water stress *triggers* root re-initiation while nothing throttles the
carbon that pays for it — "a compensation response with the penalty missing".
The counter has an exit for exactly that (`ROOT_TIP_POOR`: thirsty, under the
tip cap, sites available, and no cell holds `cost`). It reads **0 in every arm
measured** — both beds, both moistures, both slot draws. A thirsty plant is
never once short of the carbon for a new root tip. And the amplifier does track
stress: 209 firings dry against 90 wet on the deep bed, 214 against 174 on the
shallow one.

So the fix §U asks for — `water_status` scaling what a plant can *afford*, not
only what it decides to build — is still the right fix and now has a number
behind it. It belongs to P2, with the rest of the single economy pass. What
should **not** carry forward is the claim that drought currently grows a bigger
tree; on the evidence it grows a smaller one, most of the time.

**§A's guard: recalibrated, split, and un-quarantined — but the bug is NOT
closed.** Read this before assuming otherwise.

The 8-seed sweep, re-run after the P1 water fixes, on the same pairing §A
records:

| when | mean of per-seed root ratios | seeds clearing the 1.10 bar |
|---|---|---|
| at calibration, one seed (336 against 448) | **1.33** | — |
| 2026-08-22, 8 seeds | 0.92, SE 0.056 | 1/8 |
| **2026-08-23, 8 seeds, after the water fixes** | **0.994, SE 0.046** | 2/8 |

**0.1 SE from exactly no effect.** Note what the water fixes did: 0.92 → 0.994.
The small apparent *inversion* §A hedged about ("whether it is exactly dead or
slightly inverted cannot be resolved at n=8") was an artifact of the water
book, and it is gone. What is left is flat.

`CLAUDE.md` says to set a bar from measurement with headroom and, where a
report asks for a number the engine cannot yet hit, to *record both and leave
the gap visible rather than relabelling it away*. There is no bar with headroom
over data consistent with 1.0. So the guard was **split** rather than retuned:

- `slot_1_is_a_root_locus_and_not_a_shoot_one` — **live, in CI, seed-swept.**
  Asserts the half that is true: slot 1 must not move the *shoot* (mean
  per-seed spread measured **4.8%, SE 1.8%**, worst seed 13.0%; bar stays at
  the original 20%, now eight SE above the quantity instead of one seed's
  luck), and must not order root mass *backwards* (floor 0.85, three SE under
  the measurement — one-sided, because a forward bar is unreachable and a
  two-sided one would punish whoever revives the lever).
- `root_and_shoot_branching_read_different_slots` — **kept, `#[ignore]`d, and
  it still fails.** The forward claim, with all three measurements in its doc
  comment. This is bug §A, left visible and runnable by name.

The CI exclusions in `test`/`test-debug` and the `known-red-roots` job are
deleted, per that file's instruction. **That is not a claim that §A closed** —
a `continue-on-error` job pointed at an `#[ignore]`d test reports green, which
the CI file's own rule calls worse than a red one, so retargeting it would have
been the misleading option. The gap lives here and in the ignored test.

**What a fifth explanation should start from.** The amplifier is not off (see
the histogram above). The one place the two draws differ sharply is
`ROOT_TIP_AT_CAP` — 2 firings blocked by `max_active_tips` at draw −1 against
**43** at draw +1. Slot 1 raises root branching, which produces more tips,
which meet the species cap sooner; a cap is exactly the shape of thing that
converts a graded lever into a flat outcome. `tree.ron`'s `max_active_tips` is
the number to look at, and it is an economy constant, so it belongs to P2's
single re-derivation rather than to this package.

**§Z / C4: a metric that can fail, and it does.** §Z's two candidates — canopy
components at the field's resolution, and sky-gap width — are built in
`examples/plant_probe.rs` and calibrated against the answered cards. Swept over
founder spacing, default 512-wide stand, frame 28,800, current build:

| trees | spacing | **canopy fusion** | sky-gap widths | gaps >= 8 cells | `thickest contiguous run` |
|---|---|---|---|---|---|
| 8 | 56 | **99%** | [1] | 0 | 51 |
| 4 | 102 | **100%** | [4] | 0 | 43 |
| 3 | 128 | 38% | [1, 32] | 1 | 39 |
| 2 | 170 | 58% | [13] | 1 | 36 |

"Canopy fusion" is the largest connected component's share of the blocks that
hold any foliage, counted 8-connected at `field::FIELD_SCALE`.

**Calibrated against the absolute card, and it agrees.** On the 8-founder stand
— the one the owner judged "everything has merged together into a big mass, I
cannot identify individual trees" — it reads **99% fusion and no crown-scale
gap in seven**. §Z's requirement was a metric that can fail where the eye
fails; this one does. And it splits in exactly one place: **>= 99% on every
stand that reads as one mass, <= 58% on every stand that reads as separate
trees**, boundary between 102 and 128 cells of spacing.

**The last column is the point.** `thickest contiguous run` — the number §Z
records as having been believed once and overturned — reads **36 to 51 across
the entire range**, and is *highest* (51) on the stand that is most completely
fused. It cannot distinguish an eight-tree mass from two separate trees. That
is not a tuning problem; it measures whether crowns *touch*.

Three cautions, each measured rather than assumed, and two of them are mistakes
this session made and caught by looking at the render:

- **The gap census must count foliage, not any plant cell.** The first version
  reported *zero* gaps on the 4-founder stand whose render plainly shows sky
  between crowns, because the shed litter and root mound at the foot of a stand
  is continuous across every column — it was measuring the forest floor.
- **Its threshold must be absolute, not a fraction of founder spacing.** A gap
  counted as real only if it cleared a quarter of the spacing scored the
  2-founder stand — two obviously separate trees with a 13-cell strip of sky
  between them — at **zero**, because a quarter of 170 is 42. A 13-cell strip
  of sky is as visible at 170 spacing as at 60. It is one field block now.
- **Do not read the component count on its own.** It goes *above* the founder
  count on a widely spaced stand, because a sparse crown breaks into separate
  blocks. More components than founders means gappy foliage, not extra trees.

Even fixed, the gap census under-counts by construction: the 3-founder stand
shows two separations to the eye and scores one, because two of its crowns
touch at a single point. **Fusion is the headline; the gap census is supporting
evidence.** Fusion also needed no threshold, which is why it is the one to
trust.

**Not calibrated against the blind A/B card's "partial" verdict**, and this is
the honest limit: that card's other arm is `plant-substrate-v2`, a branch this
package cannot run. The sweep shows the metric is not stuck at "fused" — it
moves across the spacing range — so a partial stand is representable. Whether
it reads *partial* the way the owner reads partial is untested, and card
`20260823T092919055Z-ac816a` asks exactly that: three strips at 56, 102 and 128
spacing, with the question "how many separate trees can you count in each",
and the founder counts deliberately withheld.

**Lineage turnover (Phase 0d), printed for the first time.** Over 28,800 frames
on the 8-tree stand: **72 organisms born, 0 died**, and **0 of 8 established
plants carry an inherited genome** (deepest generation 0; 64 seeds set, all
still seeds). `plant-evolution-design.md` §5's own test — "if it reads ~0 at
30k frames, every evolution claim at that horizon is about founders" — reads
zero. Every plant result in this repo taken at 30k frames or less is a
statement about the eight trees somebody planted, not about selection. That is
A2/P3's brief, and it now has its number.

### P3. The generation loop — §F4 closed, seeds given a clock, the slot ceiling made real — **2026-08-23**

Package P3 of the plant implementation split (`Reports/plant-implementation-
split-2026-08-23.md`), following P1. Four things, and the one worth reading
first is the correction: **the review report's account of the grass economy
is a third wrong, and the wrong third is the part that would have sent the
fix at working code.**

**The grass economy, since it now exists in writing.**
`plant-project-review-2026-08-23.md` §3 files `grass.ron` as running "an
undocumented economy": `plastochron: [0,0]` means "no nodes, no leaves,
income permanently zero, `BudBreak` unreachable". Read against the code:

| claim | verdict |
|---|---|
| no leaves | **true** — no `Leaf` cell type exists in the file, by construction |
| income permanently zero | **FALSE** — `Photosynthesize` sits on `GrowingTip` *and* `MatureBody`, and both are dispatched (the tip from `organism_tick`, the retired blade from `organism_upkeep`'s cell-list walk). Grass earns from every blade it owns. |
| `BudBreak` unreachable | **true, twice over** — `break_buds` sums intercepted light over `Leaf` only, so `supportable` is 0; and nothing ever *creates* a `DormantBud` for grass, because buds come from `thicken` and grass declares no `SecondaryThicken` |

What is genuinely zero is every quantity **derived from `Leaf` cells**, which
is a different statement and has one consequence nobody had noticed:
`organism_upkeep` sums transpirational demand over `Leaf | GrowingTip`, so a
tussock that has retired every tip has **demand exactly zero**, and
`settle_water` returns status 1.0 and desiccation 0.0 for zero demand.
Measured: the 8-founder 45,000-frame ensemble ends with **252 `MatureBody`
cells and zero `GrowingTip`s**. So a mature grass plant is *drought-proof by
construction*, and `drought_death` cannot fire on one however dry the ground
is. The full path is now written into `grass.ron`'s own header and into
`wiki/plants.md`.

**§F4 is closed on the mortality half.** Both abscission rules gated on
`cell_type == Leaf`. `plant::is_foliage` now asks per species — a species
with a `Leaf` stage sheds leaves (bit-identical for `tree`, `conifer`,
`shrub`, `creeper`), one without sheds shoot tissue that photosynthesises —
and excludes root tissue by `reinforces_powder`. **That exclusion is
load-bearing and is the §F4-shaped trap inside the §F4 fix**: grass retires
its root tips into the same `MatureBody` that declares its
`Photosynthesize`, and underground `darkness` is 1, so a cell-type-only test
deletes every plant's root mat within a few ticks. Abscission also had to be
added to the *frontier* path, because `organism_upkeep` `continue`s on every
frontier cell and a grass `GrowingTip` was therefore unreachable by both
rules at once.

Given the zero-demand finding above, **the live arm for grass is shade, not
drought.** Paired, same plant, same frames, the only difference being whether
the light field was ever solved: **12 of 12 blades standing lit, 4 of 12
dark** (300 organism ticks). Guard:
`a_shaded_sward_thins_and_a_lit_one_does_not`.

**A plant that can no longer earn is dead, and its remains rot** — the case
`step_organisms` explicitly handed to somebody else ("left, deliberately and
visibly, to whoever decides what a dead tree's wood should *be*"). An
organism holding no cell that can photosynthesise, germinate or flush a bud
is marked `senescent`, one-way, in the upkeep walk that already visits every
cell; `rot_remains` then sheds its cells to litter at a species half-life and
the existing empty-cell-list rule returns the slot. Nothing becomes powder,
nothing falls, nothing schedules a structural check — those are the felling
work's decisions, and a *severed* piece should behave quite differently from
a *starved* one.

**Two guard tests found two real defects in the rule, in one run.** Worth
recording because both are the shapes `CLAUDE.md` names:

- **Moss read as a corpse.** `organism_tick` retires a stale `GrowingTip` to
  `MatureBody`, and `moss.ron` declares no `MatureBody`, so a retired moss
  cell has *no behaviours at all*. The senescence rule is starvation-shaped
  and moss has no economy to starve out of (`cost: 0.0`, no
  `Photosynthesize`, deliberately — the moss overhaul is call 4). Gated on
  `Species::has_economy`; without the guard test this would have shipped as
  "moss patches slowly disappear".
- **The paired shade test read 0 of 12 in *both* arms**, which looks exactly
  like "the new rule eats everything". It was the scene: the handmade sod's
  blades sat one row above its roots, and a shoot that does not connect to
  anchored root tissue is unreached by `anchor_support` and comes down as
  deadwood. "A scene that contradicts the code will look like a bug in the
  code", verbatim. The test now asserts its own setup before measuring.

**Seed decay (WP-D item 2).** A dormant seed was rescheduled for ever — the
not-ready branch of `Behavior::Germinate` sets `found_candidate`, so a
waiting seed never even reaches the staleness limit. Viability is now
`SpeciesDef::seed_half_life`, a **constant per-frame hazard** rather than a
lifespan, because `population-dynamics-research.md` §3 wants the bank to be
the ecology's reservoir: a fixed lifespan empties a cohort on a cliff, an
exponential tail settles at `input x 1.443 x half_life` and thins rather than
empties. It is also memoryless, so no per-seed age counter is needed. Tree
9,000 frames, grass 18,000 — the ruderal-versus-woody persistence axis as
data. A decayed seed becomes litter, not nothing. Guard:
`a_dormant_seed_bank_halves_over_a_half_life_and_does_not_empty` (**27 of 60
survive one half-life**, against 30 expected; sd is 3.9).

**The 4,095-slot ceiling is a release check.** `push_organism` returns
`Option<u16>` and refuses at the ceiling, counting refusals in
`World::organisms_refused`. The `Option` rather than a sentinel `0` is
deliberate: a sentinel would stamp an *ownerless* organism cell on the grid,
softer than corrupting an identity and still a leak. `organism_slot_usage`'s
first element already *was* the concurrent high-water mark for free — the
allocator pops the free list before it grows the vector — and
`organism_slot_high_water` says so rather than leaving it to be rediscovered.
Guard: `the_organism_slot_ceiling_refuses_a_birth_rather_than_aliasing_a_
live_one`, which fills the table, checks the refusal is counted, and checks
that a *differently-speciated* first organism still reads as itself.

**§F4's named case is superseded, not fixed, and the reproduction is what
found that.** §F4 says a grass seed landing on "a branch, a stone, a litter
drift or a nest roof would germinate, never root, and stand forever". The
obvious reading is that grass needs a *drought* death and that the way to get
one is to widen `organism_upkeep`'s transpirational-demand sum past
`Leaf | GrowingTip`. Reproduced first (`a_grass_seed_on_bare_rock_never_
germinates_and_does_not_stand_for_ever`): **it never germinates.**
`Behavior::Germinate` gates on the cell below declaring `water_capacity > 0`
before it reads `plant_available_fraction`, and stone, litter and wood all
declare none — so the seed-dormancy work already made §F4's *premise*
unreachable and the entry predates it. What actually leaked was the
**ungerminated seed**, rescheduled indefinitely on the rock it landed on, and
the seed clock closes that.

Recorded at this length because the fix the first reading pointed at would
have been a speculative economy change to one species with no live case
behind it — `CLAUDE.md`'s "reproduce before you fix", earning its keep in the
direction that saves work rather than the one that finds a bug.

**A green PR check board does not mean your head is green.** Worth its own
paragraph, because it cost a real exchange and it will recur.

On head `7b1becb`, CI produced two runs with **opposite conclusions**:

| run | event | started | conclusion |
|---|---|---|---|
| 32657999471 | `push` | 18:25:40 | **failure** — `generated_terrain_is_already_at_rest`, `a_forced_vault_world_is_sealed_and_arrives_at_rest` |
| 32659449058 | `pull_request` | 18:52:57 | **success**, all seven jobs |

Same `head_sha`. The workflow triggers on both `push` and `pull_request`, and
a **`pull_request` run checks out `refs/pull/N/merge`** — the head already
merged with the base — while a `push` run checks out the head alone. PR #24
merged at **18:51:21**, between the two, so the second run was testing a tree
that contained #24's `weather_override` fix and the first was not.

Two wrong readings were on the table and both are false: it was not this
package's diff moving the water, and it was not a determinism violation. The
tell that settles it is that **#24's own settle table records `terraced 3` at
57 cells**, which is exactly the count measured here on the unfixed tree — a
figure produced by `main`, reproduced here, and reported as this branch's.
(The `wetland seed 3 / 87` other runs saw is the same loop reaching a
different preset first: it panics on the first failing seed, and release and
debug builds do not order them the same way.)

The general form, for whoever meets it next: **when a PR board is green and a
local run on the same commit is red, compare the run's `event`, not the
`head_sha`.** A `pull_request` board is a statement about the merge result.
The `push` board is the one that describes your branch.

**What P3 does NOT fix, and who owns it.**

- **Adult tree mortality.** Nothing kills a healthy tree; a mature tree
  always holds dormant buds, so it is never senescent, which is correct. The
  cause arrives with P2's superlinear maintenance respiration, and this
  package is the plumbing that will carry it out.
- **Grass drought death**, per the zero-demand finding above — an economy
  change, P2's, and now known to have no live case pushing on it (see the
  bare-rock reproduction above). Grass's live mortality arm is shade, and
  **there is no canopy over grass in any current scene**, so the arm is
  proven by its paired guard rather than by an ensemble; putting a canopy
  over a sward is W3's sowing package, whose own acceptance it will be.
- **A dead *tree's* wood as an object.** Rot is what a starved plant does;
  what a *felled* one does is lane S's, and `BodyCell` carrying an organism
  id through promotion is S2's.

**Two things the next packages asked for, answered here.**

*Did the slot-ceiling work free a genome slot? No, and it could not have.*
The two budgets are unrelated: this package touched `Cell::organism_id`'s
12-bit **slot index**, which addresses *which organism a cell belongs to*,
while `GENOTYPE_TRAITS = 9` is the width of
`OrganismState::genotype_draws: [f32; 9]` and of each species'
`genotype_variance` tuple — sidecar state on the organism, not bits in the
8-byte `Cell`. Widening it is nevertheless **cheap**, precisely because it is
not a `Cell` budget: one added trait costs 4 bytes per live organism (~16 KB
at the full 4,095 ceiling) plus a field appended to `genotype_variance` in
all six species files, which is fixed-arity in RON so they must change
together. The binding constraint is `plant-genome-design.md`'s
positional-forever rule — a new slot must be **appended at index 9**, never
inserted, or every measured phenotype in the record silently re-keys. So a
heritable reaction norm can have its slot; it costs a coordinated `.ron`
edit, not a `Cell` redesign.

*Anchorage, for whoever picks up wind-throw.* The quantities a later
anchorage rule can read **for free** are already computed once per organism
tick and then discarded: `plant::anchor_support` runs a Dijkstra from the
anchor set over the organism's own cell list, and `is_structural_anchor` is
the predicate deciding membership (touching `Solid`, or root tissue embedded
in water-holding `Powder`). The anchor *set* itself is never materialised —
the walk seeds its heap from it and drops it — so an anchorage term wanting
"how many anchors, and how wide is their footprint" needs a tally added
inside that seeding loop, not a new traversal. `OrganismCell::support` is the
per-cell distance the same walk already writes.

### V3. Die-back's shed tissue feeds a pile that grows up through the canopy — **OPEN, isolated to P2's die-back, 2026-08-24**

The owner, on card `20260824T014630073Z-a10698`: *"the soil build-up in
between the branches is horrible."* Attached to the arm he was otherwise
praising, so it is this package's.

**Not a colour misreading — it is literal soil.** At contact-sheet zoom
soil, litter, deadwood and thickened wood are all mid-brown speckle and are
indistinguishable by eye, which is the case `CLAUDE.md` says wants a counter
rather than a picture. `examples/crown_census.rs` (new, and how this was
established) censuses every material above the ground line by height band.
One seed, 8 founders, 28,800 frames, ground line at y=200:

| band | rows | `main` `fcaa3d0` | P2, die-back **off** | P2, die-back **on** |
|---|---|---|---|---|
| 0 | y 160–200 | 4,503 | 4,122 | **6,285** |
| 1 | y 120–160 | 301 | **193** | **1,890** |
| 2 | y 80–120 | 13 | 4 | 72 |
| | **total above ground** | **4,817** | — | **8,247** |
| | **reaches up to** | y 98 | — | **y 89** |

**Isolated to the die-back, one variable, same seed.** With the die-back
block switched off, this branch sits *at or below* `main` (193 against 301
in mid-canopy). With it on, mid-canopy soil is **9.8x the die-back-off arm
and 6.3x `main`**. Nothing else in the package moves it.

**It is a pile, not lodged blobs**, and the row profile is what says so:
soil occupies **104 of the 111 rows** between y 89 and y 199, thinning from
thousands of cells per row at the bottom to one or two at the top. That is
material stacking upward from the forest floor, not material stranded in a
crown.

**Mechanism.** `shed_to_litter` probes downward for air and **stops at the
first non-air cell that is not organism-owned** — which includes litter
already lying there, and the soil that litter rots into. So every shed cell
stacks on the pile, and the pile grows without bound while shedding
continues. Die-back sheds 6,595–6,886 cells per stand at 28,800 frames on
top of what abscission already sheds, and it sheds from *high in the crown*
(most distal first), so its material has the furthest to fall and the most
chances to land on something.

**Pre-existing, and made substantially worse.** `main` already carries 4,817
cells of it reaching to y 98 — this is the same defect the owner named
during WP-11 (*"leaves are just falling too fast … creating a giant pile of
soil"*), which WP-11 addressed by cutting leaf fall to a quarter. P2 re-opens
it through a second source: **+71% total and +6.3x in mid-canopy.**

**Not fixed here, and the reason is in this package's own measurements.**
Every candidate touches `shed_to_litter`, which abscission, `rot_remains`
and the decay/ant/fire consumers all share, on top of an unmerged 2,800-line
diff — the half-calibrated-model failure `plant-economy-rederivation-2026-08-23.md`
opens by warning against. Ranked candidates, none started:

1. **Let shed tissue fall through what is already lying there**, to the
   first *solid* rest rather than the first non-organism cell. It is the
   narrowest description of the bug and it fixes abscission's contribution
   too. Wants care: the existing early-return exists so a leaf low over the
   ground is not deleted, and `CLAUDE.md` records that as a bound that must
   never gate whether the thing happens at all.
2. **Cap the standing litter/soil column above the original ground line**,
   so the floor rises and then stops. Cruder, but it is a bound on an
   accumulation that currently has none, and `filmstrip`'s `floor:` counter
   already measures the quantity.
3. **Give die-back its own disposal** that is not `shed_to_litter`. Least
   attractive — it splits one mechanism into two and the litter layer is
   exactly what the decay, ant and fire layers want fed.

`examples/crown_census.rs` is the instrument for any of them, and the
die-back-off arm is the control.

### V2. A tree cannot die of drought — shedding a leaf reduces the signal that shed it — **FIXED 2026-08-24 by `STARVATION_DEATH_TICKS`; two corrections to the entry below**

Raised by the owner against `plant-economy-rederivation-2026-08-23.md` §7,
which had folded the water case in with the light case: *"but economics
should be able to cause tree death right. if a tree doesn't get watered, it
will eventually die."* He is right. A shaded tree holding as a stump is a
suppressed tree waiting for a gap; **a potted tree is still watered**, and
the two cases have different mechanisms.

**The loop, three sites, all pre-existing and none of them P2's:**

1. `plant.rs`, the upkeep walk — transpirational demand is summed **over
   foliage only**: `if matches!(ty, Some(CellType::Leaf) |
   Some(CellType::GrowingTip))`. Wood and root declare no
   `Photosynthesize`, so they ask for no water at all.
2. `plant.rs::settle_water` — `let desiccation = if demand > 0.0 { 1.0 -
   open_drawn / demand } else { 0.0 };`. An explicit zero at zero demand.
3. `organism.rs`, `Behavior::Photosynthesize::drought_death` — drought only
   ever sheds **foliage**. There is no drought path to wood or root.

So drought sheds leaves, fewer leaves means less demand, less demand means
less desiccation. **Drought is a negative feedback on itself and a plant
escapes it by starving.**

**FIXED, and by the owner's own ruling rather than by tuning.** *"but
economics should be able to cause tree death right. if a tree doesn't get
watered, it will eventually die."* `STARVATION_DEATH_TICKS`: a plant that
cannot pay even the **mass term** of its own maintenance —
`income >= MAINTENANCE_PER_CELL x cells` — for 200 consecutive organism
ticks is marked `senescent`, which is the seam P3 shaped for exactly this
(*"gated by a cause other than starvation"*), and `rot_remains` carries it
out at the species half-life so the death is graded.

**The mass term and not the whole bill, and that is the load-bearing
choice.** A mature plant is in deficit on the *full* bill essentially
always — the girth term is superlinear and a grove's median tree runs
bill-to-income at 1.27–1.45 — so a rule keyed on "deficit" empties a healthy
stand. What separates a tree at its ceiling from a tree that is dying is
whether it can pay to keep the tissue it already has alive; a healthy tree
clears that line by about 2.6x.

Guard: `a_tree_denied_water_dies_and_a_watered_one_does_not`, paired, in a
bed large enough that "watered" means supplied. Ensemble effect, eight
seeds: **organisms senescent 0 → 4 at 45,000 frames** and 0 → 2 at 28,800,
with slots reclaimed for the first time (seed 4: 19 live in 25 slots against
27-in-27 without the rule) and the survivors *larger* — competitive release.

**Two corrections to what this entry originally claimed.**

1. **The recovery in the table below is partly rain, not purely the
   self-extinguishing loop.** The reproduction re-pinned the bed to the
   wilting point every *thousand* frames, and `run_with_fields` steps the
   weather — so rain fell between pins and the plant drank it. At a
   hundred-frame pin the drought is airtight and the plant does not recover.
   The same defect made the guard's first deep-bed run read as the rule
   failing (`starving_ticks` back to 0 at 3,871 cells) when it was the
   scene. **The mechanism below is unaffected** — it is read from source,
   not from that table — but the *"grows back at the wilting point"* reading
   was overstated.
2. **The general form is only half closed.** `STARVATION_DEATH_TICKS` gives
   an unpayable deficit a consequence, which is what §7.4 of the economy
   report said it lacked. It does **not** make wood and root *ask* for
   water, so the zero-demand immunity is still there in the water book:
   desiccation still collapses with foliage and `drought_death` still
   reaches leaves only. Fix 2 below is therefore still wanted — it is what
   makes the desiccation number mean something on a bare stump — and fix 3
   with it.

**Reproduction (records the pre-fix behaviour): `plant::tests::print_a_tree_with_the_water_withheld`**
(`#[ignore]`d; `cargo test --release print_a_tree_with_the_water --
--ignored --nocapture --test-threads=1`, `DROUGHT_EPOCHS=90` for the full
table). A grown tree, every soil cell in its bed pinned to
`SOIL_WILTING_POINT` — where `plant_available_fraction` is exactly zero —
and re-pinned every thousand frames so it stays there:

| frame | cells | leaves | water | demand | desiccation | status | senescent |
|---|---|---|---|---|---|---|---|
| 9,000 | 2,844 | 1,748 | 0.00 | 64.3 | 0.94 | 0.06 | false |
| 18,000 | 2,545 | 1,353 | 0.00 | 55.7 | **1.00** | 0.00 | false |
| 38,000 | 2,103 | 833 | 0.00 | 39.7 | **1.00** | 0.00 | false |
| 58,000 | 1,765 | 511 | 0.00 | 27.1 | **1.00** | 0.00 | false |
| 78,000 | 1,555 | 323 | 0.00 | 18.2 | **1.00** | 0.00 | false |
| 88,000 | 1,503 | 260 | 4.63 | 15.3 | 0.15 | 0.55 | false |
| 98,000 | **1,570** | 226 | 0.04 | 13.1 | 0.99 | 0.00 | false |

**Ninety thousand frames at maximum desiccation, and the tree is larger at
the end of that table than twenty thousand frames earlier.** It does not
need to reach the degenerate zero-foliage case: demand falls in lock-step
with foliage until what is left fits inside the trickle a wilting-point bed
still yields, at which point the stock comes off the floor, the stomatal
term lifts and the plant **resumes growing in ground that by definition
cannot supply a plant**.

**The general form, which is the one to carry forward.** Everything a
deficit does in this model is *shed* — the growth pool goes to zero,
`supportable` goes to zero, and die-back trims what it is allowed to trim
(and is nearly inert on a compact stump, because it is a topology-preserving
erosion). **There is starvation shedding and no starvation death.** A plant
whose bill has permanently exceeded its income is frozen, not dying, and
this entry is that hole seen through the water channel rather than the
carbon one.

**Candidate fixes, ranked, none started** — P2 deliberately did not stack a
second economy change on an unmerged one, which is the half-calibrated-model
failure its own report opens by warning against:

1. ~~**A sustained unpayable deficit kills the plant outright.**~~ **DONE** —
   `STARVATION_DEATH_TICKS`, above. The design question it posed answered
   itself in the measurement: "sustained" had to be measured against the
   *mass term* with a reset on any solvent tick, because the full bill is
   unpayable for healthy trees too.
2. **Make `allocate_to_frontier`'s `intercepted` ask the species, not the
   cell type.** It sums over `CellType::Leaf` only, so **`OrganismState::income`
   is exactly zero for any species with no `Leaf` stage however much it is
   earning** — grass photosynthesises from `MatureBody` and `GrowingTip`.
   Found by `STARVATION_DEATH_TICKS` reading that zero as starvation and
   killing **12 of 12 blades in the fully lit arm** of
   `a_shaded_sward_thins_and_a_lit_one_does_not`; the rule now exempts
   leafless species, which is a workaround and not the fix. `is_foliage`
   already asks the species and is what die-back and P3's abscission both
   use — this is the third place in one package to need it. Fixing it would
   also close `break_buds`' recorded grass defect, where `supportable` is 0
   for exactly this reason, so it is one change closing three things.
3. **Living non-foliage tissue carries a small water demand.** Breaks the
   self-extinguishing loop at its root, because demand would floor at the
   plant's own mass rather than at zero. It is also the biology: trees lose
   water through bark and respire in wood and root, and do not stop needing
   water when the leaves drop. Cheapest of the three, and it re-derives the
   water constants, so it wants the `plantsweep.sh` ensemble before and
   after.
4. **A drought consequence for wood and root.** The real mechanism is
   cavitation, which kills conducting tissue rather than leaves. The most
   faithful and the most work; it needs a per-cell channel that does not
   exist.

**Do not read this as P2 having caused it.** All three sites predate the
package, and P2's contact-only water capacity makes the *stock* smaller,
which makes drought bite marginally harder rather than softer. What P2 did
was give the model a carbon deficit that had nowhere to go, which is what
made the shape of the hole visible.

### P2. The economy re-derivation — what moved, what did not, and the six things that were built and withdrawn — **2026-08-24**

Package P2 of the plant implementation split, following P1 and P3. Full
account with every table in
`Reports/plant-economy-rederivation-2026-08-23.md`; this section is what the
*other lanes* need from it.

**What landed.** Superlinear maintenance respiration on `q_peak` (Takenaka's
1.5) plus a flat mass term on every living cell; the growth pool and
`break_buds`' `supportable` both net the bill, so growth is NPP rather than
GPP; night income at `0.25 + 0.75 x daylight_fraction`; water capacity read
off root cells that **touch soil** rather than off root mass; a whole-plant
die-back that sheds the most distal abandoned tissue when the bill exceeds
income; and anchorage as five quantities on `OrganismState`
(`anchor_cells`, `anchor_moment`, `crown_moment`, `anchor_status`,
`slenderness`), feeding the root-allocation weight and nothing else.

Eight world seeds, paired against `main`, ensemble medians: a plant is 27%
smaller at 28,800 frames and 28% smaller at 45,000, foliage share is up 2
and 4 points, the stem above the base is 13 against 15 and 17, and **8 of 8
founders establish on every seed at both horizons**, unchanged. Frame cost
on `ascii`'s tree scene is inside the run-to-run spread of a single binary.

**Three things every other lane should know.**

1. **`anchor_status`, `anchor_moment`, `crown_moment` and `slenderness` are
   live and free**, computed in walks that already ran. Lane S's wind-throw
   wants `crown_moment` (unclamped overturning demand) against
   `anchor_moment`. **Nothing in the plant lane schedules a structural check
   off any of them**, and §11.7's trap says nothing should.
2. **§U is not closed and the exit is structurally unreachable.**
   `ROOT_TIP_POOR` still reads 0 on all four arms with the whole economy in.
   The fix §U names — gating the amplifier on solvency — was built and
   withdrawn: under a maintenance economy a plant at its ceiling is
   insolvent by construction, so the gate shut root re-initiation on every
   mature plant and produced the death spiral `allocate_to_frontier`
   documents (6-8 founders of 8 against 8 of 8; median root cells 156
   against 305). A replacement needs a gate that can tell "at its ceiling"
   from "in trouble".
3. **Bug §A is now exactly, rather than approximately, dead — and
   `max_active_tips` is not the cause.** P1's lead was `ROOT_TIP_AT_CAP` at
   2 firings against 43 between the two slot-1 draws. With the economy
   re-derived that exit reads **0 in both arms**, and the two arms produce
   **byte-identical** root and shoot counts (135 / 2,859). The cap is no
   longer binding; root extension is carbon-bound. Raising
   `tree.ron`'s root `max_active_tips` would therefore be a change that
   moves nothing, and it was not made. A fifth explanation for §A should
   start from the carbon bound, not from the cap.

**Two negative results, both of which gate downstream work.**

- **Adult mortality has a cause, it fires, and nothing dies.** 5,300-9,950
  cells shed to starvation per stand over 45,000 frames, and `senescent`
  reads **0 on every seed at both horizons**, exactly as before. Four
  blockers now, and **the first version of this list got one of them
  wrong**: a *light*-limited plant holding as a stump is a suppressed tree
  waiting for a gap (correct), but a *water*-limited one is §V2 above and is
  a real bug — a potted tree is still watered. Dormant buds keeping a plant
  `is_vital` is correct and is P3's own observation. And a compact stump has
  almost no cells whose removal would not disconnect a neighbour, so the
  exclusions that make die-back safe on a crown make it nearly inert on a
  stump — whose general form is that **an unpayable deficit has no
  consequence but shedding.** **A2's turnover still has no woody arm.**
- **Selection throughput moved the wrong way.** Inherited-genome
  establishments: 1 -> 0 at 28,800 and 2 -> 0 at 45,000 over eight seeds;
  organisms born 1,067 -> 696. `Reproduce` fires per mature cell, so
  fecundity is canopy size and every plant is now smaller. **No selection or
  adaptation claim can be made for trees on this branch.** What moves this
  number is founder mortality and disturbance, not economy tuning.

**One guard changed, and it is lane S's file.** `scripts/acceptance.sh`'s
`wood` case went red here and green on `main`, so it was attributed first.
Swept over six growth windows, `main` at `cfee870` against this branch:
360/360/247/**59**/**50**/**57** (total 1,133) against
**175**/214/358/358/**54**/359 (total 1,518) — **`main` fails a 200 bar on
three of six windows and this branch on two.** The case was gated on one
arrangement with a bar set from one measurement, over a stand that is grown
procedurally; what stops the gnome at the worst window is §C1, already open.
It now runs four windows and gates the **total** at 400 against a measured
714 and 946, verified green on both binaries. Flagged rather than assumed:
the alternative was leaving a gate red or reverting an economy over a guard
the base fails half the time.

**Anchorage, in the shape lane S inherits it.** Five quantities on
`OrganismState`, all recovered free from walks that already ran:
`anchor_cells` and `anchor_moment` (`Sum |x - mean_x|` over the anchor set,
tallied in `anchor_support`'s seeding loop, which enumerated it and dropped
it), `crown_moment` (`Sum (collar - y)` over shoot tissue — mass times lever
arm, from `organism_upkeep`'s existing walk), `anchor_status` (the clamped
ratio, `ANCHOR_DEMAND` derived from the measured reach distribution) and
`slenderness` (shoot height over stem width at the base — read, never
assigned, per §11.2).

On a typical plant at 28,800 frames, eight seeds: **`anchor_status` median
0.34-0.55 by seed, min 0.02, max 1.00**; **`slenderness` median 25-52, range
4.4-127**. Both are live across the population rather than saturated, which
is the property `ANCHOR_DEMAND` was set from a measured distribution to get
— a term reading 1.00 on every plant is one nothing can select on.

**What T6 needs beyond this.** `crown_moment` is stored *unclamped* on
purpose, because a gust delivers a moment and what it has to beat is that
number against `anchor_moment`; `anchor_status` is the clamped economic
form and is the wrong input for a physical test. Three things are not here
and are lane S's to decide: the **conversion** from a gust's impulse to a
moment about the root plate (nothing in the plant lane has an opinion on
it); the **rung** — §11.3's uproot / snap / limb-off / shed ladder needs
`slenderness` read against a threshold that has never been measured; and a
**per-plant exposure** term, which W4 has now made possible and §11.5's
"cannot emerge today" no longer applies to. Nothing in the plant lane
schedules a structural check off any of these, and §11.7's trap says
nothing should.

**What blocks recruitment, since §9 measured it going backwards.** Asked
directly, and this is the one place I am reasoning past the measurement, so
it is flagged as such. Inherited-genome establishment needs three things and
this package moved two of them the wrong way: seeds set (down, because
`Reproduce` fires per mature cell and every plant is a quarter smaller),
somewhere to germinate (unchanged), and **a founder dying to make room**
(still zero). The first is a *consequence* of the economy and would reverse
if trees were larger; the third is the one that matters, and it is not an
economy problem at all — it is §V2 plus the stump case. My reading is that
recruitment is gated on mortality rather than on fecundity, and that no
amount of economy tuning reaches it: a stand of eight immortal founders at
57-cell spacing has no gap for a seedling whatever its seed rain. That
bears directly on the owner's question about plasticity deriving itself
from selection — selection cannot act until something dies.

**Both review cards came back, and the bole card needs its blinding applied
before it is read.** Root card `20260824T014648426Z-e32fca`: both arms
better, this branch chosen. Bole card `20260824T014630073Z-a10698` reads as
a contradiction and is not one — it was posted `--blind`, and
`review_page.html` labels a blind pane by *display slot*
(`"Option " + String.fromCharCode(65 + slot)`), not by the poster's item
label, with `blind_was` recording the permutation. This card's is `[1, 0]`,
so the owner's "A" is item 1 (`stand-p2`, this branch) and his "B" is item 0
(`main`). **"A looks better in that regard" is this branch reading as
separate trees; the stored choice is the same arm; click and comment agree.**
Anyone acting on that card must re-derive the mapping from `blind_was`
rather than from the letters — the trap applies to every blind card in this
repo. The rest of that sentence is §V3 above, and it is this branch's.

**If there were one more session, I would take §V2's fix 1 and 2 together,
in that order.** A sustained unpayable deficit killing the plant closes the
stump case and the drought case with one rule and uses P3's existing hook;
non-foliage water demand is the cheap complement that makes the drought
signal stop lying. I would *not* start with the free-thickening charge
(§8) — it is measured, it is real, and it is third, because it changes what
tissue costs and would want the whole ensemble re-derived again on top of a
mortality change that already does.

**One thing left undone deliberately.** `SecondaryThicken` lays wood for
free, and that is why upkeep bounds a plant's size without bounding its
tissue: a starving plant re-lays almost exactly what die-back removes (5,914
cells shed over 60,000 frames for a net loss of ten). Charging it was built
and withdrawn — it cost establishment at full pressure and bought nothing
measurable at the pressure that restored it. In `dead-ends.md` with the
re-test condition: the charge belongs in the allocation pool, not in the
thickening cell's own carbon, which transport refills within a tick.

**ADDENDUM, 2026-08-24 — §V2's fix 1 was taken, and this is P2's final
state.** Head `bd16112fb093df7d36e06650cd04b00a0e5d1b5f` on
`claude/p2-economy`. Everything above this addendum still stands except the
mortality paragraphs, which it supersedes.

`STARVATION_DEATH_TICKS = 200`: a plant that cannot pay the **mass term** of
its maintenance — `income >= MAINTENANCE_PER_CELL x cells` — for 200
consecutive organism ticks is marked `senescent`, and `rot_remains` carries
it out. The mass term and not the whole bill is the load-bearing choice; a
mature plant is in deficit on the full bill essentially always. Eight seeds:
organisms senescent 0 -> 2 at 28,800 and 0 -> 4 at 45,000, **organism slots
reclaimed for the first time** (seed 4: 27 live in 27 -> 19 live in 25) and
the survivors larger, median 4,034 -> 5,324 cells. Guard
`a_tree_denied_water_dies_and_a_watered_one_does_not`, paired. Full account
in §V2 and in the economy report §7.

**The recruitment paragraph above predicted this and was half right.** It
said establishment is gated on a founder dying rather than on fecundity.
Founders now die and **inherited-genome establishment is still 0 at both
horizons.** Mortality was necessary and is not sufficient. Whatever the
remaining gate is, it is not "nothing ever dies", and the next session
should stop re-deriving that hypothesis. Turnover also fell 25-30%.

**Three things the next session must not re-derive, because they are
measured and written down.**

1. **`CellType::Leaf` is not foliage, and `income` is zero for any species
   without a `Leaf` stage.** `allocate_to_frontier` sums intercepted light
   over `CellType::Leaf` only. This package hit it three times. The death
   rule exempts leafless species as a *workaround*; the repair is to make
   `intercepted` ask `is_foliage`, which also closes `break_buds`' grass
   defect. §V2 fix 2.
2. **A grow-out probe that re-pins a bed every thousand frames is measuring
   rain, not drought.** `run_with_fields` steps the weather. This produced a
   published table showing a droughted tree *recovering*, and read as a rule
   failing when it was the scene. Pin at a hundred frames. §V2 correction 1.
3. **The 17x8 `plant_tree_on_ground` bed is water-limited for a grown
   tree.** Income floors near 0.04 there whatever the soil says, so a
   "watered" arm in it starves. Use `root_slot_run`'s 61x30 bed for anything
   that compares watered against droughted.

**Gates, on `bd16112` after merging `origin/main` (bfcb879), all four run in
one session on that exact SHA and confirmed identical to the pushed head:**
tests 999 passed / 0 failed / 71 ignored; clippy clean; `docscheck` clean;
`acceptance.sh` all cases OK, including `wood` at 1091 cells over 4 windows
against a bar of 400. `ascii` on the same SHA: M16 tree-from-seed worst frame
growing 0.9352 ms, settled 0.4944 ms; moss 0.2827 / 0.0683; the organism
scene's mean 3.808 ms over 12,000 frames.

`lavadrop` failed three earlier acceptance runs on this box (85.12, 63.21,
71.65 ms against a 60 ms bar) and passed on the gating run. That is §T1d's
recorded flake and not this branch's: `lavadrop` builds no plant, this
change adds one float compare per organism per tick, and §T1d's uncontended
measurement has unmodified `main` itself at 74.96 ms — over the bar.
Recorded rather than waved away.

### G. Grassfire arrives with a standing negative verdict — **SPREAD AND MOISTURE FIXED 2026-08-23 (W2); the *colour* is open and is render's**

**Resolution of the two mechanical claims**, with the full account and every
number in `Reports/grassfire-and-the-desert-2026-08-23.md`:

- ***"It doesn't spread at all"* was not slow spread, it was a fire going
  out.** `try_ignite` scans four neighbours, so a front reaches one
  4-connected component of fuel and no more. A 160-founder sward looks
  continuous by a column census — one empty column in a 484-column span —
  and is **71 separate 4-connected islands**, largest 16% of the sward.
  Measured before the fix on the 64-founder sward: **14 grass cells
  consumed**, `alight 0` by frame 300 — one island's worth.
  Fixed by giving burning fuel a **flame body** (`assets/materials/
  flame.ron`, a `Gas` created already alight; `MaterialDef::flame_into` /
  `flame_chance`, unset by default so nothing else changes). Being *burning*
  means `try_ignite`'s existing scan ignites what a lick touches at no added
  cost to that scan. The load-bearing part is that the direction is
  **rolled**: a fixed search order sent every lick straight up (the cell
  above a blade is nearly always empty) and gained no lateral reach at all.
- ***"`MOISTURE_IGNITION_RESISTANCE` changes nothing"* was true, and neither
  standing suspect was the cause.** Not the 0.9 constant, and not the
  `include_str!` rebuild trap. The term's input reads **exactly 0.000 at
  96.8% of fuel cells, at every ground wetness from the wilting point to
  saturated** — `field::step_diffusion` skips a blocked block, and
  `rebuild_blocked` marks a block blocked if any `Solid` *or `Plant`* cell
  falls in it, so a block with fuel in it never diffuses. **The presence of
  fuel is what makes a block read bone dry.** For **96.8% of fuel cells the term
  reduced ignition by exactly zero** at every wetness; averaged over all
  fuel cells it reduced it by **2.9%** at saturation, all of that coming
  from the 3.2% of blades sitting in the soil's own block. (A band mean over
  the sward's *rows* reads 0.000/0.041/0.142/0.230 — monotone, plausible,
  and describing blocks the fuel is not in. That is what hid it.) Replaced by
  `CellSurface::ground_wetness_at` (the moisture *source*, at the cell's
  block and the one below) and a cutoff rather than a scale, because spread
  here is a percolation. Paired guard: `fire::tests::a_fire_crosses_a_dry_
  sward_and_stops_on_a_wet_one`, **171 cells consumed dry against 4 wet**.
  Swept over 12 procedurally different swards: at field capacity no sward
  loses more than **7.9%** of itself; dry, **5 of 12 burn out entirely**.

- ***"Just looks like you are cycling colors"* — closed by an owner verdict
  on a blind A/B, not by a judgement made here.** The fire now has a body,
  a plume and a char scar, and it still drew *pale*, because every burning
  thing saturates the heat ramp (400C above ambient; grass burns at 520C, a
  flame at 780C) and the top of that ramp was a yellow-white — so a burning
  meadow came out as **straw**. `FIRE_TINT_LOW`/`HIGH` are now
  (150,30,12)/(255,138,36). It went to the owner rather than being changed
  in passing because the same two constants colour **lava, fresh quench
  crust and warm water**, three looks already judged; the collateral is on
  its own card. Lava and the quench crust read *better* for it — a falling
  blob goes from sandy cream to molten orange. **The warm-water arm is
  unverified**, and is recorded that way rather than as checked: the pan
  has cooled by the time it is worth photographing, and where it is hot the
  tint barely registers against the blue. Two attempts that made it worse
  (flame `glow`, a widened `HEAT_GLOW_RANGE`) are in
  `Reports/dead-ends.md` under *rendering*.

**What is left of §G**: nothing in fire. The one loose thread is the
warm-water collateral above, which wants an eye on a scene where a pan is
actually hot in frame.

<details>
<summary>The original entry, kept because the verdict is the bar</summary>

### G (original). Grassfire arrives with a standing negative verdict — OPEN, inherited, 2026-08-22

Not a merge regression: it was built and judged on `plant-ecology-design`
before the merge, and the merge carries it forward unchanged. Recorded here
because a rejected mechanic that nobody tracks gets rediscovered.

The owner's verdict, in full, on the review card *"Grassfire: does a fire
front across a meadow read as its own regime?"*:

> **"The fire looks bad. Just looks like you are cycling colors. It also
> doesn't spread at all (if we are going to do this, moisture vs dryness
> should play a role."**

Three separate claims in that, and they want separating before anyone
works on it: the *look* is wrong (colour cycling rather than a fire front),
the *behaviour* is wrong (it does not spread), and there is a design steer
(**moisture vs dryness should gate spread**) which is a mechanic that does
not exist yet. The last one is the interesting one, and F1/F8 above are
about exactly the moisture channel it would have to read.

</details>

### 0f. ~~A melting `Powder` manufactures water~~ — **FIXED**

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

### 0g. ~~`scene=lavapour`'s pond simmers forever~~ — **CLOSED: the "eternal loop" had a literal heater in it**

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

### 0h. Lens-stress at 2048x640 puts gravel and water in motion, with no cave anywhere (worldgen)

Surfaced by Phase 2 moving the cave tests from 512x320 to 2048x640
(`Reports/world-scale-phase-2.md` §3), because a 4x cave will not fit in a
512-row world at all.

**Reproduction, kept and runnable:**
`cargo test --release --test worldgen probe_p2_does_the_lens_stress_move_
cells_without_a_cave -- --ignored --nocapture`. It builds the lens-stress
world -- `rolling`, `pocket_density: 20.0` against a shipped 0.6, trees and
moss off -- at both sizes, with vaults on and off, and counts what leaves its
position over 120 frames.

**Measured**, seeds 1..5:

| | 512x320 | 2048x640 |
|---|---|---|
| with vaults | 0 | 0, 0, 0, 0, **25** (seed 5) |
| **no vaults** | 0 | 0, 0, 0, 0, **25** (seed 5) |

**The cave is not the cause.** The same 25 cells move with
`vault_density: 0.0` and no chamber carved anywhere in the world. They are
gravel and water in a compact blob near the *surface* -- first three
`(332,141) water, (341,132) gravel, (337,132) gravel` -- hundreds of columns
and a hundred rows from where any system sits.

**What has been ruled out:** the cave pass (paired control above); tree and
moss growth (`vault_test_params` zeroes both); the size alone (0 at 512x320,
same params). What is left is `pockets` at 33x the shipped density on a world
eight times the area, interacting with standing water. Four of five seeds are
clean, so it is a seed-specific placement, not a systematic one.

**Not a live defect at shipped densities.** `pocket_density` ships at 0.6 and
`generated_terrain_is_already_at_rest` asserts **zero** cells move across
every preset and seed at that density. This is a stress reproduction escaping
its own subject.

`a_cave_system_survives_a_pocket_lens_inside_its_envelope` now asserts at-rest
within 16 cells of the carved system rather than over the whole world, so it
tests the thing it is named for; the probe above is what keeps the finding.
**Do not "fix" it by lowering `pocket_density` in that test** -- 20 is what
guarantees a lens lands inside a cave envelope, which is the entire
reproduction.

### 0i. Terrace risers are inert: erosion deletes them at any nonzero `world_age` (worldgen)

Found while attributing the Phase 2 review's "sharp vertical faces"
(`Reports/world-scale-phase-2.md` §7a).

`column.rs`'s `riser_roughness` term adds a second, much larger detail term
near a terrace riser, and carries a long justification for why a riser needs
breaking up: *"a riser is a single-column jump of `terrace_step * mask` rows
-- up to 34 on `canyon` -- and `detail_amplitude` is 2.5 to 3.0, which is
nowhere near enough to break a face that tall."* It reads as current.

**Measured, it never reaches the screen.** Pre-erosion, the largest adjacent
`|d elev|` is 19.93 rows in a single column (canyon seed 3, x 2937), and
those columns are entirely the riser term. `erosion.rs` caps every adjacent
pair at `THERMAL_STABLE_SOFT + hard * THERMAL_STABLE_HARD_BONUS` = 0.55 +
4.5 = 5.05 rows/column, and canyon ships `world_age: 1.0` (600 iterations).
Post-erosion, `probe_p2_how_sheer_is_the_ground` over all 8192 columns:

  canyon: med 0, p90 1, p99 3, max 5; columns >=6: 0, >=10: 0, >=20: 0

Every shipped preset except `flat` carries a nonzero `world_age`, and `flat`
zeroes `terrace_strength` anyway. So the roughening term fires only in a
configuration nothing ships.

**Not necessarily a defect** -- round-4 task 4 turned age on deliberately
after the riser work landed, and a subdued world may be what is wanted. What
is wrong is that the source says otherwise at length, so the next session to
read it will believe a mechanism is live that cannot be. Either the comment
gets the measurement, or the term gets removed, or `world_age` stops eating
it -- but the three cannot all stay as they are.

Related, same investigation, also unasked: **the palette-family thresholds
are gated on x only** (`passes.rs`'s `palette_family_for` takes them from
`character(x)`; only the comparison value and bias are 2-D). Measured on
canyon seed 3, the steepest ramp sweeps 0 to 1 in **11 columns** and shows in
the shipped render as warmth going -1.3 to +27.2 across world x 1358-1390,
coherent from the skyline to the bottom of the frame. That is a genuine
near-vertical colour seam and it may be a second referent for the owner's
*"the patterns don't flow"*. Untested by eye; nobody has been asked.

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

### 1m. Damp-soil evaporation barely runs, and the humidity shadow that would switch it off is already here

**Raised by the plant merge agent, verified, measured, and deliberately not
fixed** — the fix is a design call and the branch was in wrap-up.

Their claim, all three parts confirmed against source:

- `field::rebuild_blocked` grades a soil cell as `soil_moisture /
  water_capacity` and takes the **max over the whole 8x8 block**
  (`field.rs`, `moisture_level.max(held)`).
- `field::apply_moisture_sources` then forces the block to `MAX_MOISTURE *
  level`, and `MAX_MOISTURE` is 4.0.
- `evaporation::dryness` samples the block **one above** the surface
  (`y - FIELD_SCALE`, and `field_moisture_at` reads the containing block
  with no interpolation) and returns zero at or above `HUMID_STOP` = 2.0.

So any evaporating surface whose block-above contains soil at more than
half saturation evaporates **nothing at all**, rather than slowly.

**The correction to their handoff: this is not a plant-branch
consequence.** `soil.ron` already carries `water_capacity: 1000`, and
worldgen's existing `soil_moisture` pass already seeds soil *saturated*
where it touches liquid or sits at or below the water table. Saturated is
1000, so those blocks are pinned at 4.0 — double the stop, not the 2.28
their flat baseline would give. Their change widens the affected area from
the wetted perimeter of a pond to everywhere there is soil; it does not
create the effect.

**Measured** with the new `evaporation::DrynessCounts`, `scene=worldgen`,
3,600 frames, becalmed checks over total checks:

| preset | seed | soil | water |
|---|---|---|---|
| rolling | 1 | 0/0 | 1701/11738 (14%) |
| rolling | 7 | **31/53 (58%)** | 2024/16073 (13%) |
| rolling | 2900 | 0/0 | 19977/20242 (**99%**) |
| wetland | 1 | 0/0 | 4276/12981 (33%) |
| wetland | 7 | 0/0 | 7176/22792 (31%) |
| wetland | 2900 | 0/0 | 17351/17813 (**97%**) |

Three readings, in order of how much they matter:

1. **The soil path is essentially unexercised: zero checks in five of six
   runs.** `is_damp_soil_surface` needs damp soil *with air above it*, and
   worldgen wets soil near water and below the table — both below the
   surface. So the shadow cannot bite yet. It is the plant branch's flat
   baseline, which damps surface soil everywhere, that will make this path
   run at all — and the one run that did exercise it was becalmed **58% of
   the time**.
2. **Seed 2900 is a 99% outlier on both presets** against 13-33%
   elsewhere. Outcomes here are chaotic in the seed, so any guard over this
   has to gate an order statistic over a sweep, never one seed.
3. **The counter cannot yet attribute the cause**, and 2900 is why: air
   over a world in a long wet or cold spell is *legitimately* humid, and
   seed 2900 is the coldsnap seed. Do not read that 99% as the soil
   shadow. Splitting "saturated because of the soil below" from "saturated
   because it is raining" needs the source recorded in
   `apply_moisture_sources`, which is the next step if this is pursued.

Also worth noting: `the_worlds_water_is_flat_over_soil_too` passes today
and would pass just as well if soil evaporation never ran, because a
conservation test is satisfied by nothing moving — the shape `CLAUDE.md`
records for infiltration's dead gate. Given reading 1, it may already be
passing vacuously. Break soil evaporation deliberately and see.

Two candidate fixes, which trade differently and neither of which is
tuning: raising `HUMID_STOP` lifts a calm lake off exactly zero, which
that constant's doc says is the one reading that must stay zero; sampling
the surface cell's own block instead of the one above changes what the
number means everywhere, water included.

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

**Partly stale as written — re-read against the source 2026-08-23 before
acting on it.** Two of the three walks named below are charged now:
`subtree_sum` (`load.rs:1090`) and `supported_subtree` (`load.rs:1149`)
both take `budget: &mut u32` and decrement per cell, as does
`detached_piece`, and `failing_along_support_chain` itself checks
`*budget == 0` at two points. What is **still** uncharged is
`chain_reaches_anchor` (`load.rs:643`), whose signature takes no budget at
all. So the defect is narrower than the paragraph below claims, and the
118 ms measurement predates the change — it wants re-taking before anyone
concludes anything from it.

The original text follows.

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

### 1i. ~~The rigid-body rotation probe is vacuous, and a body can turn through a wall~~ — **DUPLICATE of §K, and FIXED there 2026-08-23**

Kept as a pointer rather than deleted, because the duplication is the
finding. This entry (written while fixing §1h, naming the function
`blocked_axis`) and **§K** below (written during the water-merge review,
naming it `try_step` after the rename) are the *same defect*, logged twice
in two sessions, neither cross-referencing the other — so the handoff
carried it as two open bugs and any count of what was outstanding was one
too high.

The fix, the measurement and the standing guard are all recorded at §K.

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

### H. `ascii`'s ants moisture-gradient scene asserts a gradient the scene no longer has — **CLOSED 2026-08-23. The well evaporated; the scene now maintains a spring, and the guard is a continuous margin.**

> **Read §L first (2026-08-23).** As of `main` at `a0fa433`, `ascii` panics
> at `:1678` on a *foraging* assert and **never reaches the `:1850`
> assertion below**. The reproduction in this section is real and was real
> when it was written, but you cannot currently observe it by running
> `ascii`: §L has to be got past first. The two are unrelated failures
> sharing one quarantined gate.

`examples/ascii.rs:1850` fails its own setup assertion:

```
=== ants: deposition follows the moisture gradient, with no build rule anywhere ===
  pickups 1764 drops 1731 digs 0 deaths 0
  mean |grad moisture|: steep half 0.000, flat half 0.000
  material left standing: steep half 3, flat half 0
panicked: the scene must actually contain the gradient it is testing: 0.000 vs 0.000
```

**Inherited, and measured rather than assumed.** Built `origin/main` at
`da1faf0` in a clean worktree and ran `ascii` there: it fails with
**byte-identical numbers** — same 1764/1731, same 3-versus-0, same panic
line. So this is `main`'s, not the explosion merge's, and the merge
reproduces it exactly.

**Why no one had seen it — the CI history, kept because the lesson outlived
the bug.** `.github/workflows/ci.yml` once ran all five gates as *sequential
steps of one job*. `cargo test` was step 4 and was red on `main` (bug A, the
slot-1 lever); when it failed, steps 5-9 — including `cargo run --example
ascii` and `scripts/acceptance.sh` — were marked **`skipped`**, not run.
Verified on run `32604849243`: one job, one failure, five skipped steps. So
"main is green" could not be concluded from CI for any gate after `cargo
test`, and had not been true for some time.

That topology is gone: the workflow is a parallel job matrix, so one red
gate no longer hides the rest. **Both quarantines this entry used to
describe are also gone**, in opposite directions — bug A's `--skip` was
retired when its test was `#[ignore]`d behind a seed-swept replacement
guard, and `ascii`'s blanket `continue-on-error` came off once `ascii`
learned scene selection, leaving `skip=foraging` and the one named scene in
`known-red-ascii` (§H2). The general lesson is the part worth keeping: **a
quarantine wide enough to hide the bug it was opened for is wide enough to
hide the next one**, and while the blanket was on, `forage_loop_scene` went
red and nobody saw it for two commits.

The assertion is a *setup* check — `wet_grad > dry_grad`, i.e. "the scene
contains the gradient this test is about" — so it is `CLAUDE.md`'s "a scene
that contradicts the code will look like a bug in the code", and the thing
to check first is whether the scene still builds the moisture gradient it
was written around, not whether the ants' deposition rule is wrong. Both
halves reading 0.000 to three decimals is the tell: it is not that
deposition stopped following the gradient, it is that there is no gradient
to follow. Note the printed 0.000 is rounded — the pre-merge explosion
branch printed the same 0.000/0.000 and *passed*, so the true values are
small and non-zero and the ordering flipped somewhere below the third
decimal.

**Closed 2026-08-23, and the record's own steer was right: the scene, not
the deposition rule.** The well is filled once at spawn and then left to the
world. Instrumented per 1,000 frames it goes

    34 -> 30 -> 39 -> 52 -> 66 -> 76 -> 98 -> 47 -> 1 -> 0

so it does not simply evaporate — it *rises* first, because `weather::step`
runs inside both CA drivers and rains into it, and then a dry spell takes the
lot. **By frame 10,000 there is no standing water anywhere in the scene**, and
the field it feeds reads `steep mean 0.000 peak 0.000 | flat mean 0.000 peak
0.000`. So `wet_grad > dry_grad` was deciding between two numerical residues,
which is why it flipped between CI runs 137 and 139 while printing identical
numbers.

That is `CLAUDE.md`'s "a channel that oscillates by design must be divided out
of decisions", in weather's costume rather than light's. There is no
`noon_equivalent` for weather, so the scene holds the *source* constant
instead:

1. **The well is topped up every frame** (`run_colony_with`'s new per-frame
   hook), making the left half wet at every phase while rain can still wet
   the right half without ever making it a spring.
2. **The gradient is averaged over 40 samples through the run**, not read at
   one instant — two instants fitted to one trajectory is the failure this
   file's own §V records by name.
3. **The spring is asserted to still be standing** (`water_after >= 20`)
   before anything is concluded from the field it feeds.

Measured after the fix: **steep half 1.9206, flat half 0.1061, margin 1.8146**
on `MAX_MOISTURE` = 4.0, against a residue below the sixth decimal before. The
bar is 0.5 — a little over a quarter of the measurement, and comfortably above
the flat half's own 0.1061.

**Both guards were broken deliberately to prove they bite**, and the result
changed the fix. Removing the spring alone leaves the *averaged* margin at
1.4033 — because the average still sees the rainy phases — so the time-average
by itself would have "closed" bug H while the scene was still empty at the
end. It is the standing-water assertion that catches it. Guard 3 exists
because of that break test, not in spite of it.

**One half of this scene is still not tested, and that is recorded rather than
tuned away.** The headline assertion `wet_drops > dry_drops` is **vacuous**:
deleting `moisture_gradient` from the drop probability in `creature.rs:1254`
entirely — the whole mechanism the scene is named for — left it passing
*harder*, steep 18 / flat 0 against steep 6 / flat 0. Removing a multiplier
below 1.0 raises the drop rate everywhere, and the flat half reads zero in
both arms because the ants never travel that far. It has been demoted to a
printed measurement.

Its successor is a **ratio**: mean `|grad moisture|` at the cells ants actually
dropped on, over the mean across the whole band they could have dropped on.
That does separate the arms — **4.97x with the bias against 2.84x without** —
but both stand on 6 and 18 standing drops, and a bar from a ratio of six cells
is the same knife-edge this scene has already been bitten by. It is printed,
not asserted. What it needs is more drops to average over, which is blocked on
the same thing everything else here is: **ants that leave home at all** (see
the foraging entry below).

`ascii` is gating in CI again on this basis, with `skip=foraging` naming the
one scene still red instead of the whole example being non-blocking.

### H2. The `ascii` colony has gone sessile — **CLOSED 2026-08-23 via §L: same bug, filed twice, one root cause**

> **Merged at landing (2026-08-23): this is §L, independently found by P1's
> gate run, and §L carries the close** — the rock-country fallback admitted
> only the argmax region and deleted the residual towers from the colony's
> home range; widening the fallback to the country field's own scale
> restores the scene. The entry below is P1's independent measurement,
> kept because it agrees with §L's to the digit and adds one datum §L did
> not have: the water-book fixes alone moved the scene 2 → 7 trips, which
> says the food's *water supply* participates in the collapse's magnitude
> but is not its cause. The `known-red-ascii` quarantine this heading
> referenced is deleted with the close.

> **Superseded in framing, 2026-08-23.** This entry was opened as "`ascii`
> never reaches bug H any more" — true at the time, and no longer the point:
> bug H is **closed** (above), `examples/ascii` has scene selection, and this
> scene is quarantined by name as `known-red-ascii` (`skip=foraging` in the
> gating job). What survives is the measurement and the attribution, kept
> because P1 made them independently and they agree with `main`'s to the
> digit.

**Found by running the gate rather than by reading it.** The foraging-loop
scene panics at `ascii.rs:1678`, the colony sessility guard, 172 lines
*before* bug H's assertion at 1850 — so for a while `ascii` never reached bug
H at all, and anyone re-checking §H saw this instead.

**Measured, paired, same machine and session** — `origin/main`'s `src/`
swapped into a clean tree against P1's, both `cargo run --release --example
ascii`:

| | forage trips (bar 14) | deepest | reach profile | live organisms | deliveries |
|---|---|---|---|---|---|
| `origin/main` | **2** | 15 | [689, 22, 8, 2, 0, 0, 0, 0] | 76 | 143 |
| P1 (the water book) | **7** | 15 | [998, 59, 19, 7, 0, 0, 0, 0] | 71 | — |

**Inherited, and the water fixes move the failing number the right way** — 2
to 7 against a bar of 14, with every reach bucket higher. Not fixed here: the
water fixes were not aimed at ant foraging, 7 is still under the bar, and a
guard over ant behaviour on generated terrain belongs to the creature line.
`main`'s own `known-red-ascii` comment reaches the identical numbers
independently, including the reach profile digit for digit, which is worth
more than either measurement alone.

**The bar's own doc says how it was set, and that is the first thing to
check.** It reads "measured 98 trips, deepest 18, mean depth 10.3 over 12,000
frames after the litter merge", with the bar at 14 — a seventh, chosen because
"outcome spread here is large and a bar near the measurement flakes". `main`
now measures 2. That is not a bar flaking; something took the colony from 98
to 2.

The scene's food is a **stand of trees** whose leaves the ants forage
(`ascii.rs:1429`'s own note: a corpse pile gave 2.5 pickups and zero
deliveries, trees gave 44.8 and 28.8), and both runs above deliver food
(main: 1,340 pickups, 143 deliveries). The colony is not starving; it is
finding food without going 8 cells to get it. So the quantity to census first
is how much foliage is standing and *where*, not the ant brain —
`CLAUDE.md`'s "when a mechanism appears inert, check the scene still contains
the situation you think it does". `main` has since carried this further: 88%
of the colony's food is leaf on standing trees and the stock triples over the
run while the ants eat none, which is on the owner's queue as
`20260823T091259637Z-9a41e4`.

### H3. ~~Both worldgen at-rest tests are red on `main`, and both are water~~ — **CLOSED 2026-08-24. Superseded by §M: it was the sky, not the water.**

> **Read §M instead.** Both tests pass on `main`. `45ba304` scoped them to the
> claim they are named for by holding the sky still
> (`World::weather_override = Some(Weather::CLEAR)`, at both at-rest tests in
> `tests/worldgen.rs`) — they had been asserting that a generated world holds
> still *while snow falls on it*. Seed 3 precipitates from frame 0 and is the
> seed both failed on; seeds 1, 2 and 5 never precipitate and always passed.
>
> **Verified on `main` at `fcaa3d0`** by the plant integrator, by re-running
> rather than taking a report's word for it:
> `cargo test --release --locked --test worldgen` → **44 passed, 0 failed**.
>
> **This entry is left standing rather than deleted, because the way its
> diagnosis was wrong is the useful part.** It identified the moving cells
> correctly — material 6, water — and then concluded "a liquid-at-rest failure
> wearing a worldgen test's name". The identification was right and the
> conclusion did not follow: the water was moving because something was landing
> on it, and no census of *which* cells moved can separate "this liquid will not
> settle" from "this liquid is being rained on". Only §M's control — the same
> binary with the sky held still — could, which is the shape of control this
> file keeps asking for and the reason a correct measurement can still support a
> wrong story.
>
> Everything below is the record as it stood while the entry was open. Its
> measurements remain true of the trees they were taken on; its framing does
> not. **Do not start a liquid-at-rest investigation from it.**

Not a new bug — `plant-implementation-split-2026-08-23.md` already warns that
main is red here. Recorded because the *content* of the failure was not, and
because it is the reason a plant branch's CI looks broken when it is not.

`tests/worldgen.rs:1794` snapshots every `(x, y, material)` in a forced-vault
world, steps 120 frames, and requires nothing to have left its position. The
cells that move are **material 6 — water** (`stone` is 2, `sand` 3, `gravel` 4,
`ash` 5, `water` 6, in `MATERIAL_FILES` order), so this is a liquid-at-rest
failure wearing a worldgen test's name. The assertion is inside the
preset/seed loop, so the run stops at the **first** failing case and says
nothing about the ones after it.

Measured, paired, same machine and session — `origin/main`'s `src/` swapped
into a clean tree against P1's:

| | first failing case | cells that moved |
|---|---|---|
| `origin/main` `a0fa433` | **rolling seed 3** | **47** |
| P1 (this branch) | wetland seed 3 | **8** |

**P1 gets past `rolling seed 3` entirely**, which main does not, and then stops
on a later case with a sixth as many cells. So the water fixes move this the
right way rather than causing it. What P1 does *not* establish is whether main
also fails `wetland seed 3` — main never reaches it, and finding out means
making the loop collect failures instead of panicking on the first, which is a
`tests/worldgen.rs` change and belongs to whoever owns worldgen.

**Why a plant package looked responsible for it, which is worth knowing before
the next one wastes the time.** §F3's fix leaves a partly-drunk water cell as
*partial fill* where the old code deleted the cell outright, and partial fill
is mobile — it seeks its level. That is a real, plausible route from a plant
change to "water did not hold still", and it is why this was measured against
main rather than waved through on the split document's say-so. The measurement
says the opposite of the suspicion.

**And the reason it reached CI at all: `cargo test --lib` does not run
`tests/`.** P1's local gate was `cargo test --release --lib` — 851 passed, 0
failed — which never compiled the integration tests. CI runs `cargo test
--release`, which does. Run the bare form locally before believing a green.

**The failing seed RELOCATES under unrelated work, which is the strongest
argument yet for making the loop collect. — 2026-08-23, later the same day**

Measured on a head that differs from `main` `86e73d5` in exactly two files, a
report and an example's comment block, with `src/worldgen`, `src/sim` and
`tests/worldgen.rs` byte-identical to it — so this is `main`'s behaviour by
construction, not an interaction:

| `generated_terrain_is_already_at_rest` | first failing case | cells that moved | suite |
|---|---|---|---|
| main `eda560d` | `terraced seed 3` | 57, first `(82,147)` water | 37 passed, 2 failed |
| main `86e73d5` | **`wetland seed 3`** | **87**, first `(114,133)` water | 40 passed, 2 failed |

The world-scale lane's worldgen work landed in between. `terraced seed 3` now
**passes**; a different preset fails instead, with more cells. The test name,
the failure count and the red/green of the job are all unchanged — only the
fingerprint moved, and nothing but reading the panic message would show it.

**So the headline number is not comparable across commits, and nobody can say
whether that change was an improvement.** Two presets swapped places at the
front of a loop that stops at the first failure; whether the total number of
failing seeds went from 4 to 2 or from 2 to 6 is not observable from anything
CI prints. This is the same blind spot the entry above describes, now caught
actively rather than argued: it is not a hypothetical cost of panicking on the
first seed, it is a measurement that was already lost once. Collecting the
failures — the `tests/worldgen.rs` change this entry says belongs to whoever
owns worldgen — is what turns this pair of tests back into a signal.

**A second at-rest test joins it, and it is water too —
`generated_terrain_is_already_at_rest`, `tests/worldgen.rs:182`.** It arrived
on `main` at `9b54be3` and P1 inherited it by merging main in to clear a
conflict. Measured the same way, `origin/main`'s `src/` against the merged
branch, same machine and session:

```
terraced seed 3: 57 cells left their position;
first: (82,147) water, (83,147) water, (84,147) water, (84,148) water, ...
```

**Byte-identical on both sides** — same seed, same count, same leading
coordinates. Unlike the forced-vault case above, where the two branches differ,
this one the water fixes do not touch at all. Purely main's.

**Two at-rest tests, both failing on water, is the shape worth acting on.**
They are not two bugs about worldgen; they are one question — *does generated
terrain's standing water actually settle?* — asked by two tests that stop at
the first seed that says no. Neither says how many seeds fail, because both
panic rather than collect. Whoever picks this up should make the loops gather
failures first; the count per preset is the measurement, and right now nobody
has it.

*This is §M, filed twice — §M carries the moved counts after §L's
rock-country fix (the worst natural mover shifts to wetland seed 3, and the
forced-vault stress case gains a collapsing spire that is not the water
bug). Read §M's dated note before attributing any count change here.*
### I. ~~The disturbance-extent guard inverts once rubble stops anchoring~~ — **FIXED 2026-08-23. The measure was wrong, not the mechanism.**

`sim::structural::tests::a_disturbance_extent_licenses_the_wound_but_not_the_chain`
was green on the explosion branch at `5f72fe2` and fails on the merge:

```
the extent bought nothing: 1022 cells failed with the wound licensed
against 1586 with a point licence -- TIGHT is leashing the blast's own seams
```

**Cause isolated by ablation, not inferred.** Reverting *only*
`load::rests_on_ground` to its pre-merge one-liner (`the cell below is a
Powder`) and changing nothing else makes the test pass. `origin/main`'s
`grain_is_footing` predicate — the §17e fix that stops a slab being anchored
by two grains of its own debris, and which `explosion-stone-review.md` §17h
directs this merge to take wholesale — is the sole trigger. **It is not a
defect in that fix.**

**The mechanism still works; the *measure* is what broke.** Sweeping the
test's own frame budget, everything else held:

| frames | verdict |
|---|---|
| 100 | passes |
| 200 | passes |
| 400 | fails, 1022 vs 1586 |
| 600 | fails, 1022 vs 1586 (identical — it has settled by ~400) |

So licensing the wound *does* buy more failure early, which is the claim the
test is named for. What happens later is that the point-licence arm, being
throttled, keeps failing cells for hundreds of frames while the licensed arm
has already collapsed and settled — and a **cumulative** cell count then
reads the throttled arm as the more damaged one. Once rubble stopped
anchoring, there is simply much more to cascade through.

**Fourth time a count has caught a mode shift rather than a behaviour
change** — see §17g's `roomcut` and case 6's `strike`.

**Fixed by owner decision: the guard now compares `promoted_cells`** — rock
lifted out of the grid as a moving body — instead of summing the failure
counters. Three candidates were measured before choosing, which is the only
reason the obvious one was not taken:

| quantity | wound | point | ordering |
|---|---|---|---|
| region sum (was) | 1022 | 1586 | inverted |
| **promoted cells (now)** | **840** | **649** | wound +29% |
| stone destroyed | 657 | 648 | wound +1.4% |

Stone destroyed is the intuitive census and orders *correctly* — and is
rejected anyway, on headroom: a bar is set from measurement with room, not
sitting on it. Shortening the run was rejected on principle rather than on
numbers: it passes at 100 and 200 frames, but `CHAIN_WINDOW_FRAMES` is 600,
so the licence is live for the entire run and stopping early would be tuning
to green rather than measuring anything.

**Red-checked**, because a guard that cannot fail for the replacement is
worth nothing (`CLAUDE.md`: a superseded mechanism's tests keep passing
while testing nothing). Flattening *both* arms to a point licence makes the
extent buy nothing by construction, and the guard fails there as it must —
649 against 649. Restored, it passes.

`grain_is_footing` itself is untouched and was never at fault; the ablation
that named it only established which landed change exposed the bad measure.

### J. A blocked substep still vents the smoke it was only *probing* — **OPEN, pre-existing, 2026-08-23**

Found by review of the water merge, in `rigid::clear_or_displaceable`.

`try_step` scans every cell of a body to find out whether the substep is
blocked, and the scan is meant to be side-effect free until the verdict is
in. Its own comment says so: *"A body that turns out to be blocked now
leaves the water where it was, instead of having shoved half of it on the
way to finding out."* That is what `Step::swaps` is for — the `Liquid` arm
records the exchange and defers it.

The `Gas` arm does not. It calls `world.set(x, y, Cell::EMPTY)` inline, so a
body that is then judged blocked has already erased the smoke it was only
asking about. On a fresh crater, which is 18% `SMOKE` by
`Tuning::smoke_fraction`, that is the one place it happens most.

**Pre-existing, not the merge's.** Byte-identical at `5f72fe2`, before
`origin/main` was merged. What the merge changed is that the deferral
discipline now sits three lines away, which is how the review saw it. The
`Powder` path via `displace` mutates in the same speculative way, so the
honest framing is that `swaps` fixed liquids and left the other two kinds
alone, not that gas is uniquely wrong.

**Not fixed here, deliberately.** Routing the vent through `swaps` changes
what a blast leaves behind, and the Gas arm's justification is a *measured*
paired result (`blast=300,45,20,180,60`, rolling seed 1, against the
`smoke=0` control: 1 body / 10 cells in flight at frame 80 against the
control's 6 / 100). Any change here has to re-run that pairing and be judged
by eye, which is a piece of work rather than a merge repair.

### Q. Settled debris stands in one-cell vertical needles that never topple — **OPEN, owner-reported 2026-08-23**

The owner's verdict on review card `20260823T155727949Z-b17b87`, which asked
which of two settled rubble piles read as real broken rock:

> **"Neither. and it is because the long skiny vertical pieces should fall
> over. instead of all standing upright"**

**Worth recording how it was found, because the card asked the wrong
question.** The card was a blind A/B on §K's rotation fix, and it explicitly
told the owner the thin spires were *"in both sides and not what I am asking
about ... a separate defect"*. They were the only thing he answered about.
Both arms were rejected on a feature the poster had bracketed off — which is
the `CLAUDE.md` "resolve an ambiguous complaint before building anything"
lesson arriving from the other direction: the thing you set aside as
background can be the whole of what a player sees.

**Reproduction.** `filmstrip scene=worked start=1500 every=1 count=1
crop=0,225,230,95 zoom=4 daylight=1.0` — any settled frame past ~800 shows
them. Present *before and after* §K's fix, so it is neither caused nor cured
by it, and visible in both panes of the card above.

**Two candidate owners, and the first step is to tell them apart** — this is
not yet measured and must not be treated as if it were:

1. **Settled rigid bodies.** `rigid::settle` writes a landed body back as
   `Cell::new(cell.material, cell.shade)`, i.e. **unattached** stone, and
   `structural.rs` then asks only whether it reaches an anchor. A 1-wide,
   20-tall column standing on the ground does. There is no slenderness
   ratio, no tipping moment and no bearing width anywhere in the load model,
   so a knife-edge column is indistinguishable from a wall. `worked`'s own
   census agrees the model is content: **1** unattached cell reaches no
   anchor in the whole scene.
2. **Rubble that will not avalanche.** `rubble` is a `Powder`
   (`rubble.ron`), stone `breaks_into` it, and powder takes no part in
   `structural.rs` at all — it falls via `update::update_powder`. That
   function tries straight down, then both diagonals, so a 1-wide column in
   open air *should* topple on the next frame, every frame. If these are
   rubble, something is refusing that diagonal and the bug is in the powder
   rule or its `flowing`/repose hysteresis, not in the load model.

**So the decisive first measurement is simply: what material is a standing
needle made of?** Nothing in the harness reports it today. Until that is
answered, do not tune either system — the two explanations want opposite
fixes, and `CLAUDE.md`'s "a scene that contradicts the code will look like a
bug in the code" applies to both readings.

If it turns out to be (1), the shape of the fix is the question `CLAUDE.md`
already names: **which object does this rule evaluate — a cell, a section,
or a whole piece?** A bearing rule needs a contact width and a tipping
moment, and neither is defined for a single cell. That is the same defect
recorded there as "a slab lying on its own rubble was judged as many
separate knife-edge footings", pointing the other way: here the knife edge
is what stands.

### P. `scene=worldcrack` is not deterministic, so `seedsweep.sh` cannot compare two models on a chaotic seed

*(Re-lettered from L at the 2026-08-23 lane landing: three unrelated bugs
had been filed as §L by three lines. The colony-sessile entry keeps the
letter — it is the one the lane PRs and CI job names cite.)* — **OPEN, pre-existing on `main`, 2026-08-23**

`CLAUDE.md` lists same-build determinism as **required**. It does not hold on
the scene the seed sweep is built out of.

**Reproduction.** One release binary, five identical invocations:

```
./target/release/examples/filmstrip.exe scene=worldcrack preset=canyon seed=3 \
    strike=12 start=2 every=900 count=5 zoom=1 out=target/filmstrips/d.png
```

rock destroyed: **837, 1077, 1083, 1083, 1283** — a 53% spread. An independent
audit got 993–1336 over nine runs, and `RAYON_NUM_THREADS=1` does **not**
remove it, so it is not rayon work-stealing.

**Pre-existing, ruled out by measurement.** It is not the load-sharing change:
a clean `origin/main` binary containing none of that code gives rock destroyed
**37, 37, 81** on three identical runs of the same scene. Sharing amplifies the
absolute spread (it fails more material, so the chaos has more to work with)
but does not cause it.

**Not universal.** `terraced 1` returned −1042 on six independent runs. The
signature is stable on most seeds and unstable on a few — which is the worst
possible shape, because the unstable ones are the ones carrying the signal, and
a single-sample grid cannot tell an unstable seed from a real regression.

**It is not confined to `worldcrack`, measured 2026-08-23.** `scene=ligament`
at `start=2 every=900 count=5` (frame 3,602), one release binary, three
identical invocations:

| run | bodies promoted | cells promoted | quarter turns asked |
|---|---|---|---|
| 1 | 166 | 5,939 | 48 |
| 2 | 398 | 10,327 | 166 |
| 3 | 407 | 10,689 | 166 |

A **1.8x spread in promoted mass** between runs of the same binary, and run 1
diverges so early that it asks a third as many rotations as the other two. The
paired control that makes this attributable rather than suggestive:
`scene=worked`, same treatment, came back **bit-identical three times over**
(40 bodies / 1,701 cells / 48 turns asked / 5 refused), so the harness, the
timing and the machine are not the variable — the scene is.

**What this costs:** `ligament` is one of `acceptance.sh`'s eight structural
cases, and it is the scene §1c's withdrawn fix was measured on
(18.1 ms -> 86.6 ms). Its acceptance bar is `min_overloaded=1` over a
~350-frame window, which is loose enough that the spread above cannot flip it
— so the gate is not currently flaky. But **no before/after comparison taken
on `ligament` at a long budget means anything**, including the ones already in
this file, and anything measured there in future needs a repeat count and an
order statistic rather than a single run. Prefer `worked` as the deterministic
control when a rigid-body change needs a paired reading.

**Leads, not verified.** Two candidates for a per-process perturbation that
chaos then amplifies: `structural.rs`'s single per-frame `world.load_budget` is
drained across all sites, so any ordering change moves which checks come back
`Deferred`; and `world.rs` builds `body_index` by iterating a
`std::collections::HashSet<ChunkCoord>`, whose iteration order `RandomState`
re-seeds **per process**, with `find_body_at` returning the first match in that
list. Either would give exactly this stable-on-most-seeds picture. Neither has
been confirmed.

**The `body_index` lead is now eliminated by reading, 2026-08-24** (found
during the frame-cost audit, `Reports/frame-cost-audit-2026-08.md`). It fails
on two independent grounds, either of which is sufficient:

- **The `HashSet` order cannot reach the list.** `promote_body` pushes the new
  body's id **once per touched coord** — `self.body_index.entry(coord).
  or_default().push(id)` — so every coord's `Vec<BodyId>` receives exactly one
  push per body, whatever order the set is walked in. A given list's contents
  and its order are therefore set by the order bodies are *promoted*, not by
  the hasher's seed, and `find_body_at`'s "first match" is the
  earliest-promoted owner either way. `demote_body` removes with `retain`,
  which is order-preserving.
- **The code does not run in production at all.** `promote_body` is the liquid
  heightfield path, and `CLAUDE.md` records that subsystem as test-only —
  "nothing in production promotes a body, because automatic promotion was
  implemented and reverted over a real architectural gap". `scene=worldcrack`
  never reaches it.

So **§P's cause is still open**, and the remaining lead is the `load_budget`
one. A related note for whoever picks this up: swapping the engine to a
fixed-seed hasher was considered as a fix for this and should **not** be sold
that way. It is a reasonable change for *speed*, but an audit of every
hash-container iteration in `src/` found them all already order-safe —
membership-only (`pending_*`), sorted immediately after collection (organism
`cells`, `rigid.rs`'s `remaining`, the field's `awake`/`read`), or
order-independent by construction (counts, dirty-rect unions, writes to
disjoint flat indices). That is why `tests/determinism.rs` passes despite std
seeding per *instance*, and it means a hasher swap has no known route to this
bug.

**Four candidate sources checked and eliminated, 2026-08-23** — recorded so
the next session does not re-walk them:

| candidate | verdict |
|---|---|
| `scheduler::step`'s `HashMap` drain (`PLAN.md` issue #7 / §8b) | **fixed, not the cause.** Now a `BinaryHeap<Reverse<ActiveSite>>` with `Ord` on `next_frame` then `(x, y, kind)` — a total order, stable across runs. §8b can no longer be quoted as the explanation for this bug. |
| `field::step`'s tile solve | **sorted.** `solve.sort_unstable_by_key(\|c\| (c.y, c.x, c.slice))`, with a comment naming this exact requirement. |
| `rigid.rs`'s fracture seeding | **sorted.** `remaining.sort_unstable()` before the seed loop, and the `left` set is only ever `contains`/`remove`d, never iterated; `take_fragment` is a `VecDeque` BFS over `NEIGHBOURS_4` in fixed order. |
| the `body_index` lead above | **weak.** `body_index` is a `HashMap<_, Vec<BodyId>>` whose per-chunk `Vec` is in *insertion* order, not hash order, so the `HashSet` iteration it is built from does not reorder it — and liquid-body promotion is test-only today, so `find_body_at` is not on this scene's path at all. |

**What the search should look at instead: `World::rng` is one shared
mutable stream** (`world.rs:286`), drawn at event time — `world.rng.below(
rungs)` is called *per fragment seed* inside `fracture_failing_region`, and
many other systems draw from the same sequence. So any upstream perturbation
that changes **how many draws happen before a given fracture** reshuffles
every random outcome after it. That is the amplifier, whatever the source
turns out to be, and it is what makes the first lead above (the
`load_budget` drain moving which checks come back `Deferred`) sufficient on
its own: it does not need to change *what* fails, only *when*, for every
fragment size downstream to change with it.

That also says what the remedy looks like, and the project has already
applied it once: per-organism RNG (`f9ab577`). A per-site stream derived
from `(x, y, frame, seed)` rather than drawn from a shared sequence makes
fracture immune to upstream draw-count drift, which is a narrower change
than finding the perturbation.

**Why it matters beyond one change.** `seedsweep.sh` is the instrument
`CLAUDE.md` prescribes for every change to a model over procedural content, and
the file's own guidance sends cascade comparisons to it. On the seeds that
actually move, it is currently reading noise at the same magnitude as signal. A
repeat count per cell (median of N) would make it usable again without fixing
the root cause, and is much cheaper than finding it.

---

### K. ~~`try_step`'s rotation-fit probe compares every cell against itself~~ — **FIXED 2026-08-23.** (Was also filed separately as §1i; same defect, two write-ups.)

Also from the merge review. In `rigid.rs` around the rotation fit, the probe
called `try_step(world, &probe, probe.x, probe.y, …)`, so each cell's target
position was its own current position. The `if (tx, ty) == (cx, cy) { continue }`
guard at the top of the scan then skipped **every** cell, `horizontal` and
`vertical` were never set, and `axis` was always `None` — the probe reported
"nothing blocks this rotation" unconditionally, so a wedged body rotated
through a wall. Live for the entire life of the mechanism, in both parents of
the water merge.

**The fix is `rigid::rotation_fits`, a read-only predicate**, and it is not
the obvious one. Correcting the offset and calling back into `try_step` was
rejected: `clear_or_displaceable` **mutates** as it answers — `displace`
shoves powder and the `Gas` arm calls `world.set(…, Cell::EMPTY)` inline — so
a probe built on it would rearrange the world to decide a turn it may then
refuse, which is §J's speculative-write defect on a path that discards the
answer. `rotation_fits` asks the same *classification* question and does
nothing but return it. `BodyCell::rotated` is now the single definition of
the quarter turn, so the predicted turn and the performed one cannot drift
apart.

**Powder is deliberately treated as yielding** without asking whether
`displace` could actually find it somewhere to go, which is more permissive
than the real move. A read-only ring search per cell per turn is the exact
cost, and refusing on a failed search would stop a piece tumbling the moment
it touched its own debris — the medium a collapse happens *in*. The cheat
this guard exists to stop is turning through a **wall**.

**Measured, `scene=worked`** (`start=2 every=900 count=5`), which returns
**bit-identical numbers across three runs of the same binary**, so this is
the change and not run-to-run spread. Both arms were taken at the same base
(`d5e7af8`, before this branch merged the lane landing in), which is what
makes the pair comparable — the absolute numbers will have moved since, the
delta between the arms is the result:

| | before | after |
|---|---|---|
| quarter turns asked / refused | 48 / **0** (probe vacuous) | 48 / **5** (10%) |
| bodies promoted | 76 | 40 |
| cells promoted | 2,847 | 1,701 |
| cells to dust | 683 | 577 |
| chunk by mass | 80% | 74% |
| all at rest by frame | 741 | 388 |

**The control that isolates it:** `scene=strike` asks **zero** quarter turns
(its pieces never reach `spin >= 1.0` at 1.05 cells/frame) and its output is
byte-identical across the change — same 20 bodies, 670 cells, 270 shattered.
A scene that never consults the probe is unmoved by repairing it, which is
what says the delta above is the probe and not collateral.

Two things that are **not** evidence, recorded so they are not read as such.
`scene=ligament` moved too, and its numbers are void — it is nondeterministic
(see §P). And every scene's worst-frame timing improved, including
`strike`'s, which moved 196 ms -> 60 ms *while producing byte-identical
output* — so the timings in this environment are noise-dominated and **no
performance claim is made here** in either direction.

**Judged by eye — and the verdict came back about something else.** The
baseline's debris pile is full of mushroom-capped one-cell stems (bodies
that turned into places they could not have reached) and those are gone; the
cost is 40% less rock coming away, because a piece that cannot turn jams and
re-embeds instead of cascading. Posted blind as review card
`20260823T155727949Z-b17b87`. The owner rejected **both** arms — *"Neither.
and it is because the long skiny vertical pieces should fall over. instead of
all standing upright"* — on the artifact the card had explicitly bracketed
off as not-the-question. That is filed as **§Q** and is present either side
of this change, so it neither vindicates nor condemns the fix: **this
repair is kept on correctness grounds** (bodies were passing through rock)
and the thing the owner is actually looking at is a different bug. Re-ask
the chunk-size question only once §Q is fixed, since until then the pile is
dominated by an artifact neither arm controls.

**Guarded by `a_wedged_body_will_not_rotate_through_the_wall`**, which was
`#[ignore]`d against this bug and is live again, now asserting **both**
directions — the wedged bar is refused, and the same bar fits once the slot
above and below it is opened. A probe that always refuses is exactly as
useless as one that always allows, and the one-sided version could not tell
them apart. `FailureCounts::rotations_asked` / `rotations_refused` are the
running readout (`filmstrip` prints `quarter turns:`); **refused == 0 on a
scene with walls in it is the tell that it has gone vacuous again.**

### N. Decayed litter makes soil that does not match the soil around it, and roots will not enter it — **OPEN, owner-reported 2026-08-23, both causes found**

From the owner's verdict on card `20260823T091259637Z-9a41e4`: *"why does the
soil from decayed leaf litter look different than the regular soil. and the
plant roots are not growing into it."* Both are real, both are in
`decay.rs`'s single `world.set`, and **both are already described by that
function's own comments** — as accepted costs whose visible price had not
been looked at.

**1. The colour. `decay.rs:142-143`:**

```rust
let shades = world.materials.get(into).base_shades.max(1) as u32;
let shade = world.rng.below(shades) as u8;
```

Two mismatches against how worldgen paints soil, not one:

- **Wrong family.** `base_shades` is "how many *leading* palette entries a
  random shade may pick from", i.e. family 0 only, and the comment says why
  outright: *"Decay has no region to consult, so it stays in the first
  family."* But `passes::palette_family` assigns families from regional
  aridity whenever `region_variation > 0`, and every populated preset has it
  — `wetland` (the colony scene) is **0.45**. So in any region that is not
  family 0, decayed litter lands as a different hue family from the ground it
  lands on.
- **Wrong tone within the family.** Worldgen does not pick soil shades at
  random at all: `passes::soil_shade` walks them **2 → 0 → 1 → 3**, "dark
  organic topsoil down to paler mineral subsoil", so tone carries depth.
  Decay draws uniformly, so a fresh patch at the surface is a speckle of all
  four tones where the surrounding topsoil is one.

The first is a known limitation the comment states; the second appears to be
unnoticed, and is the one that makes a *patch* rather than a *shift*.

**2. The roots.** `decay.rs` leaves the new soil **dry**, deliberately, and
its comment defends the choice at length — the two richer versions both
manufactured water and one took `a_tree_eventually_stops_growing` from 1,718
cells to 2,652. That reasoning is sound and should not be reverted. What the
comment anticipated was narrower than what happens: it names the cost as *"a
seed reseeded onto brand-new soil may wait a little before germinating"*.
But roots steer by `organism::moisture_pull` (`plant.rs:1592`), so
established roots avoid the new layer too, for as long as capillary flow
takes to wet it. The owner is watching that at play scale and reading it as
roots refusing the soil, which is exactly what it looks like.

**Not a licence to wet it on creation.** The fix shape is either to give
decay the region and depth it needs to pick a shade the way worldgen does, or
to let the new cell inherit them from the soil it is replacing — and, for the
roots, to establish how long capillary wetting actually takes before deciding
there is anything to fix.

**Merged at landing (2026-08-23) — Lane B filed this bug independently, and
its unique finding is a coupling.** The capillary remedy above is defeated by
the very material producing the dry layer: §F1 (LIVE, verified) has
`weather::step`'s soak loop `break` at the first cell with `water_capacity ==
0`, and `litter.ron` declares none — so a column under a litter blanket takes
**zero** rain. Fast rot then deepens a dry layer under a blanket that blocks
the rain, with only sideways capillary flow to wet it, and it predicts this
bug gets worse exactly as the litter economy gets better — the enrichment
shape `PLAN.md`'s standing note warns about. Measures specced before acting,
neither built yet, both paired: soil `aux` summed by depth under a littered
column against a bare one across rain epochs; root cells entering
newly-decayed soil against established soil.

Reported, not fixed: `decay.rs`, `plant.rs` and the palette passes are not
this lane's files.

### O. Litter rots into soil that never leaves, so the floor rises all run — **OPEN, owner-reported 2026-08-23, quantified**

Same verdict: *"the soil is piling up way too fast … I think leaves are just
falling too fast which creates too much food and is creating a giant pile of
soil."*

Measured, `filmstrip scene=colony`, wetland, seed 0, same run at two horizons:

| | frame 1,200 | frame 12,000 |
|---|---|---|
| decay events (damp + dry) | 179 | **6,331** |
| standing decayable cells | 194 | 1,081 |
| living plant tissue | 11,407 | 24,033 |

Every decay event is one `world.set` writing a soil cell, and **soil has no
`decays_into`** — nothing on this channel removes it. So the count is a
monotone floor level: **~6,331 soil cells manufactured in one 12,000-frame
colony run**, and it scales with leaf fall, which itself scales with a canopy
that doubled over the same run.

**The owner's causal reading is supported by the arms already measured.**
Litter is the only decay input in this scene, and rotting it *faster* makes
the pile worse, not better — the paired card arms read 6,331 events at
`decay_chance` 0.5/0.1 against **7,287** at 0.9/0.4, while standing litter
fell 1,081 → 260. So the two halves of his verdict agree: he picked arm A on
looks, and arm A is also the arm that buries the world more slowly. **The
faster-rot direction the implementation handoff proposed for this card would
have made his actual complaint worse**, which is the argument for having
measured both arms rather than shipping the proposal.

The lever he names is upstream of the one the card asked about: not how fast
litter rots, but how fast leaves fall. That is abscission
(`plant.rs`), not `litter.ron`.

Related and not the same: §L is the *foraging* consequence of the same
over-production (88% of the colony's food is standing leaf, the stock triples,
the colony has stopped ranging). One economy, three symptoms — sessile ants,
a rising floor, and soil that does not match.

**Update 2026-08-23 (WP-11): the named lever is pulled; the structural gap
stands.** New shed-cause counters (`World::shed_shade/shed_drought/
shed_stranded`, printed by `filmstrip` beside the decay line) attribute
~89% of leaf fall to `shade_death`, so that is what moved: `tree.ron`'s
leaf `shade_death`/`drought_death` went 0.003 → 0.00075, swept at
0.003/0.0015/0.00075 (soil-writing events 6,331/3,443/1,800 on the arms'
own tree) and chosen by the owner on card `20260823T161006584Z-6ecbab`
("C is best"). Verified on the tree it landed on (post-water-book `main`):
this same scene now reads **1,862 decay events at frame 12,000 against
6,653 paired baseline (−72%)**, standing litter 1,024 → 344, living tissue
+10%. What this does **not** fix, kept open under this heading: soil still
has no exit channel, so the count is still a monotone floor level — it
rises at a quarter the rate, it does not stop; and colony-band food
*energy* does not fall with the rate (census medians 70k/82k/91k across
the arms — retained foliage becomes low standing leaf), so §L's abundance
reading is not expected to move much. Both were on the card the owner
chose from.

### M. ~~Two gating worldgen tests are red, and both are the same thing: generated water never comes to rest~~ — **FIXED 2026-08-23. It was the sky, and the generator was innocent.**

> **The cause is weather, and both "where to start" leads below are wrong.**
> Recorded prominently because this entry sends the next session at the
> generator, and the generator turns out to have nothing to do with it.
>
> `weather::step` runs inside `parallel::step`, so the at-rest tests were
> asserting that a generated world holds still **while snow falls on it**.
> `weather::at` is a pure function of `(seed, frame)`, so this is checkable
> without simulating anything:
>
> | seed | sky |
> |---|---|
> | 1, 2, 5 | never precipitates in 12,000 frames — **passed** |
> | **3** | precipitates from **frame 0** (Snow, intensity 0.36; 1,786 wet frames in 12,000) — **the seed both tests failed on** |
> | 4 | first precipitation at frame 5,981 — outside the 120-frame window |
>
> **The settle curve killed the "unsettled placement" reading first.**
> `probe_m_does_generated_water_ever_settle` samples displacement-from-origin
> at a ladder of frame counts. Water that was merely slow to settle would
> *decay* toward zero. It climbs:
>
> | world | 120 | 240 | 600 | 1200 | 2400 | 4800 | 9600 |
> |---|---|---|---|---|---|---|---|
> | `terraced 3` | 57 | 85 | 287 | 271 | 309 | 324 | 362 |
> | `wetland 3` | 37 | 36 | 35 | 599 | 615 | 611 | 641 |
> | `terraced 2` | 0 | 0 | 0 | 0 | 0 | 18 | 41 |
>
> `terraced 2` is the tell: **perfectly at rest for 2,400 frames, then it
> starts moving.** Nothing that settles does that. (Seed 2 never
> precipitates, so its late drift is the other weather path —
> `DRY_FROST_CHILL`, "a clear freezing night still freezes", which changes
> standing water to ice *in place* and so changes the `(x, y, material)`
> triple the snapshot compares.)
>
> **The control, and what actually exonerates the generator.** The same
> worlds with `world.weather_override = Some(Weather::CLEAR)`:
> `terraced seed 3` reports **0 at every sample**, and the whole sweep
> reports **0 at 120 frames** — the gate's own budget. What is left is 2–3
> cells on `rolling 3` and `wetland 2`, appearing only after frame 1,200.
>
> **The fix is the test's scope, not the generator.** Both tests now hold
> the sky still. This is the treatment the terrain test *already* applies to
> plants, moss and `spring_flow`, each with a comment saying a growing thing
> is "a live process, not a placement defect"; weather arrived later and
> never got it. It is **not** a seed dodge — the seed list is untouched, and
> picking quiet seeds would have been tuning the sweep to the answer.
> `World::weather_override` is the hook, resolved once in `World::weather()`
> so the simulation and the renderer cannot disagree about the sky.
>
> **Left open, deliberately:** the 2–3 cells at 1,200+ frames under a clear
> sky are a real if tiny at-rest defect that the 120-frame gate does not
> reach. Not chased here — they are three orders of magnitude below what
> this entry was about, and the probe that finds them is kept
> (`probe_m_does_generated_water_ever_settle`, `#[ignore]`d). They are also
> the evidence the repaired gate is **not vacuous**: raise its budget to
> 1,200 and it goes red again.


> **Still red five merges later, and it is now blocking every pull request
> (confirmed 2026-08-23 from CI, not from this file).** `main`'s own CI has
> gone red on **every** run today — `d5e7af8`, `95f0a0d`, `7409d88`,
> `135c9a9`, `9f165ec` and `eda560d3` — and at `eda560d3` the failed jobs are
> exactly `cargo test (release)`, `cargo test (debug)` and the
> `continue-on-error` H2 quarantine. PR #24 reproduces it identically after
> merging `eda560d3` in, on a diff that touches only `rigid.rs`, two `u32`
> counters and a `println!`. Locally, `cargo test --release --locked --test
> worldgen` gives **37 passed / 2 failed**, the same two names.
>
> **A methodological note that cost this session a wrong claim.** `cargo test
> --lib` reports **879 passed / 0 failed** on the same tree, because it does
> not build or run the integration binaries *at all* — `tests/worldgen.rs`
> never appears in its output, not even as skipped. `CLAUDE.md`'s red-suite
> gotcha covers the case where a red lib test *hides* the integration
> binaries; this is the quieter sibling, where a **green** `--lib` is read as
> a green gate and is not evidence of one. Run it the way CI does
> (`cargo test --release --locked`) before claiming the test gate is green.


**Two** tests, not one, and neither is in any handoff's list — which records
`main` as one red (bug A). Neither is quarantined, so this is a **gating**
job failing. `cargo test --release --no-fail-fast` on `main` at `9b54be3`:
lib **855 passed / 0 failed**, worldgen **37 passed / 2 failed**.

| test | fails at |
|---|---|
| `generated_terrain_is_already_at_rest` (`:182`) | `terraced seed 3: 57 cells left their position` |
| `a_forced_vault_world_is_sealed_and_arrives_at_rest` (`:1794`) | `rolling seed 3: 47 cells left their position` |

**They are one bug wearing two names.** Both assert that a freshly generated
world holds still, and in both the cells that move are **water** — the
terrain test names them (`(82,147) water, (83,147) water, …`) and the vault
test prints material id `6`, which is `material::WATER`. Both fail on
**seed 3**. So the claim that is actually broken is "generation leaves
standing water at rest", and a fix for either should be checked against both
rather than treated as two jobs.

```
rolling seed 3: 47 cells left their position in a forced-vault world;
first [(1263, 138, 6), (1270, 138, 6), (1258, 138, 6), ...]
```

The claim in each case is the same: snapshot, step, assert nothing moved.

**Deterministic, and it got worse across the load port.** Three consecutive
runs on `main` at `9b54be3` give `rolling seed 3: 47 cells` every time. On
`a0fa433` — the same test, before main's load-concentration port (`5e6e79b`,
`b934041`) — `rolling` *passed* and the failure fell through to `wetland
seed 3: 8 cells`. The preset list is a fixed array `["rolling", "canyon",
"wetland"]` and the assertion aborts on the first failure, so reaching
`wetland` at all means `rolling` was green then. Two trees, two results,
both red: 8 cells on one preset before, 47 on an earlier preset after. Not
attributed further — the load model is what decides whether a cell holds
still, and it is what changed.

**The message is the trap.** The count is stable but the sample is not: the
cells are drawn from a `HashSet` difference, so the "first 6" printed
reshuffle on every run — `(1263, 138, 6)`, then `(1267, 138, 6)`, then
`(1255, 138, 6)` — while the count stays at exactly 47. A reader comparing
two failure messages sees different cells and concludes "flaky", which is the
one thing it is not. Sorting the sample before printing would cost nothing
and is the fix `CLAUDE.md`'s "a debug readout must not be a function of the
thing it debugs" implies here.

The moving cells are a wide band rather than one collapsed spot — y 137-139
across x 1250-1550 in the vault case — which is what a sheet of water finding
its level looks like, not a structure failing.

Reported, not fixed: `tests/worldgen.rs`, the load model and the liquid rules
are not this lane's files, and the point of finding it is that nothing said
it was red.

**Merged at landing (2026-08-23) — Lane B's independent filing adds the run
history, the local blind spot, and a starting commit:**

- Red on every `main` CI run since `a0fa433`; last green was **#146 on
  `c6ffba2`**, the creature-line parent of that merge. Consistent with the
  load-port worsening above: at `a0fa433` the failure was `wetland seed 3: 8
  cells` and `rolling` still passed.
- **A plain local `cargo test` cannot see this.** `cargo test` stops after
  the first failing test *binary*; bug A fails in the lib target, so a local
  run never executes `tests/worldgen.rs` or `tests/determinism.rs` at all —
  no error, no "skipped", just absence. Run the gate the way CI runs it:

  ```
  cargo test --release --locked -- --skip root_and_shoot_branching_read_different_slots
  ```

  The tell that you have the short version is the *absence* of
  `Running tests/worldgen.rs` from the output. The quarantine that made CI
  honest about bug A made local gate-running dishonest, in the direction
  that hides failures.
- **Where to start:** the world-scale line landed its springs/river pass
  immediately before the merge (`4b044b2`, `7120741`, `f5f3b19`); water
  placed by a new pass that has not settled by the time the at-rest
  assertion samples is the shape of the original 8-cell failure. Run both
  tests at seed 3 against `4b044b2^` first, then walk forward. Do not close
  this by widening the settle budget until that has been checked — 0 cells
  to 57 is a behaviour change, not a drift past a threshold.

**Counts moved with the §L fix (2026-08-23), bug unchanged — and the vault
red changed shape, which needs saying precisely.** The rock-country
fallback widening (§L's close) changes terrain on fallback worlds. Both
tests are still red, differently:

- `generated_terrain_is_already_at_rest` now reports worst `wetland seed
  3: 87 cells` (was `terraced seed 3: 57`), **still all water** — the same
  claim broken, a different pond under it. Across all presets x 5 seeds,
  worlds now carrying far more spires, **zero mineral cells move**: the
  widened band generates at rest.
- `a_forced_vault_world_is_sealed_and_arrives_at_rest` now fails at
  `rolling seed 3: 705 cells` of **stone** (was 47 of water). That is not
  the water bug: the test forces chambers at `vault_min_depth: 40` — five
  times shallower than the natural 200 — into a 2048-wide world the band
  now mostly covers, and a spire over a 40-row-deep forced chamber
  collapses when stepped. Natural worlds show no such motion (the bullet
  above), so this is the stress configuration meeting the band, not
  generation shedding stone in play. Whoever picks §M up should attribute
  the water half first and treat the stone count as this interaction.

Recorded so the next reader does not bisect the count change to the wrong
cause. Not the same root as §L: springs place zero in the foraging scene's
world (0 cliff candidates, measured under `SPRING_DEBUG=1`), so the
springs-pass lead above is untouched by §L's fix.

**Correction to the bullet above, measured 2026-08-23: the 705 stone cells
are the sky too, not "the stress configuration meeting the band".** The
note is right that the count changed shape and right that it needed saying
precisely; the attribution is the part to drop, because it sends the next
reader at `vault_min_depth` and the widened band, neither of which is
load-bearing. Paired run, one binary, the only difference being whether the
sky is held still:

| `a_forced_vault_world_is_sealed_and_arrives_at_rest` | result |
|---|---|
| weather running | `rolling seed 3: 705 cells` of stone — reproduces the count above exactly |
| `weather_override = Weather::CLEAR` | **passes** |

So the mechanism is snow and frost *loading* the spires the widened band
now carries, over chambers forced five times shallower than natural — the
spire is real and the forced chamber is real, but neither moves until
something lands on it. Which is why the water half and the stone half have
one fix between them: both tests were reading a live sky as a placement
defect.

Worth noting for **§Q**, which is about exactly these spires: a one-cell
stone needle that stands indefinitely in still air but comes down under a
snowfall is evidence that what holds it up is a *bearing* rule with no
slenderness term, rather than anything about the terrain it grew from.

### R. `filmstrip scene=colony` panics at its own default seed, and degrades badly at others — **OPEN, found 2026-08-23 (WP-9 arm 1)**

The scene every colony review card is rendered from cannot be run without a
`seed=` argument, and where it does run it mostly does not place a colony.

**Reproduction.** One release binary built from this branch, `scene=colony`
with nothing else varied:

```
./target/release/examples/filmstrip scene=colony genome=authored seed=N \
    start=1200 every=3600 count=1 cols=1 zoom=2 out=/tmp/p.png
```

| `seed=` | result |
|---|---|
| **1 — the default (`filmstrip.rs:2580`)** | **panics**, `filmstrip.rs:1461`, `.expect("some dry ground")` |
| 0 | 13 of 52 ants placed |
| 2 | 35 of 52 |
| 3 | **2 of 52** |
| 7 | 22 of 52 |

So `filmstrip scene=colony` with no `seed=` is a panic, which is the form
anyone reaches for first and the form the scene's own doc comment above it
suggests.

**Two distinct faults, and the second is the dangerous one.**

1. The `.expect("some dry ground")` at `:1461` is the outer search failing:
   no column in `102..WIDTH-102` has a dry surface. The scene was *already*
   softened once for this — its own comment records "on a wetland seed there
   may be no unbroken 200-cell beach, and demanding one made the scene panic
   rather than degrade" — and it is panicking again from the same place, so
   the previous softening addressed the inner window and not this.
2. **`assert!(placed > 0)` is not the guard it reads as.** At seed 3 it
   passes with **2 ants**, and a two-ant "colony scene" is exactly the
   *scene-lost-the-situation* failure `CLAUDE.md` warns about: the render
   looks plausible, the assertion is green, and the picture is not of a
   colony. A guard on a colony scene should bar on a fraction of
   `COLONY_ANTS` (52), not on zero.

**Why it matters beyond the scene.** This is the harness a judge-by-eye
verdict on colony behaviour is produced from. WP-9's founded-colony A/B card
(`20260823T180052569Z-c73d21`) had to be shot at `seed=2` for exactly this
reason, and 35 of 52 ants is the *best* of the seeds tried — so the card the
owner judged shows a colony a third short of a founded one. That is stated on
the card, but the honest fix is in the scene.

**Not caused by the creature work, and this is measured rather than argued.**
Three arms, and every number is identical across all three:

| | seed 1 (default) | seed 0 | seed 2 | seed 3 | seed 7 |
|---|---|---|---|---|---|
| `climbs_over_kin` **on** (WP-9 branch) | panic | 13/52 | 35/52 | 2/52 | 22/52 |
| `climbs_over_kin` **off**, same binary tree | panic | — | — | 2/52 | — |
| **clean `origin/main` `f245ebc`**, flag off, none of the WP-9 code | panic | 13/52 | 35/52 | 2/52 | 22/52 |

The third row is the one that settles it: a worktree at `origin/main`, its own
`target/`, `climbs_over_kin: false` as `main` ships it, and it panics at the
same `.expect("some dry ground")` and places the same counts at every other
seed. **§R is pre-existing on `main` and has nothing to do with the flag.**
(The line number differs between the rows — `:1461` on the WP-9 branch,
`:1485` on `main` — because `main` has since grown lines above the scene, not
because the assertion moved.)

Mechanically that was already the expectation, since the dry-ground search
runs *before* `found_colony` places anything, so no ant exists when it fails.
Terrain, not animals. It remains plausibly downstream of §L's rock-country
fallback widening, which changed fallback terrain — that half is still a lead
and nobody has bisected it.

**Blast radius: none of the gates.** `scene=colony` appears nowhere in the
repo outside `filmstrip.rs` itself. `scripts/acceptance.sh` renders twelve
scenes and this is not one of them (`capped`, `coldsnap`, `lavadrop`,
`ligament`, `rockdrop`, `room`, `strike`, `terrain`, `undercut`, `wood`,
`worked`, `worldcrack`); `ascii` does not use `filmstrip`; no test invokes it.
So this cannot redden CI. It bites exactly one activity — a human or agent
rendering a colony card for the review queue — which is how it was found.


### L. The colony has gone sessile: 98 round trips became 2 — **CLOSED 2026-08-23: the rock-country fallback gated on an argmax, and the colony's home terrain vanished with it**

**Root cause, found by looking at the scene, exactly as the bisect predicted.**
`region.rs`'s rock-country guarantee (`gate = FORMATION_BARREN.min(best)`)
admits, when it fires, only the single region that drew the field maximum.
The foraging scene's 512x120 world has **two** regions; at rolling seed 1
they read country 0.4141 (cx=47 — the colony's home range) against 0.4691
(cx=459), both far under `FORMATION_BARREN` (0.70) — "essentially a single
value", as the guarantee's own comment says a sub-period world samples — and
the knife-edge kept only cx=459. The **two residual stone towers standing
inside the nest patch (x≈42–68)** on the creature parent, the terrain every
foraging bar was measured on, vanished; the freed soil columns then grew
worldgen trees, so the canopy edge moved from x≈88 to x≈64, *inside* the
nest patch.

**Both halves matter, and they interact — measured by ablation on the merge's
world** (temporary scene switches, one build, same seed):

| arm | trips | deliveries | falls | nest-visits |
|---|---|---|---|---|
| parent `c6ffba2` (towers, canopy from x≈88) | 92 | 192 | 901 | 3,598 |
| merge world (no towers, canopy in nest patch) | 2 | 143 | 64 | 684 |
| merge + hand towers | 35 | 277 | 413 | 1,234 |
| merge + worldgen trees cleared x<210 | 30 | 9 | 709 | 18,426 |
| both | 245 | **0** | 2,423 | 11,052 |

No single lever restores the parent's shape: towers alone leave food at the
doorstep, clearing food alone leaves the loop unable to close (0–9
deliveries over the scene's food distance). The parent's balance — vertical
home terrain plus food starting at the nest patch's edge — is what the
92–98 bar measured.

**The fix is in worldgen, not the scene.** The fallback now reads the best
draw as *defining* the country and gives it the field's own extent: regions
within `ROCK_COUNTRY_SCALE / 2` of the best centre belong to it
(`region.rs`, beside `FORMATION_BARREN`). A 512 world becomes rock country
whole; a shipped-size fallback world (1 in 16 seeds) gets one country-sized
band instead of one region-sized cluster — the cluster shape is the exact
failure `FORMATION_BARREN`'s own comment records the owner rejecting, so the
knife-edge was wrong at both scales. On the gated path (best ≥ 0.70) nothing
changes.

**Restored, measured on the same scene:** forage trips **100** (bar 14, set
from 98; the parent read 92 on the same code path), nest-visits 3,792
(parent 3,598), falls 960 (901), mean depth 10.3 (10.3), deliveries 230
(192), profile `[3798, 452, 171, 100, 0, 0, 0, 0]`. The 2,000-frame counters
are **identical** to the parent's run — the towers regenerate at the same
sites. The bar stays at 14, unmoved, as this entry demanded. Re-measured
after merging the water book (PR #19) into the fix: **112 trips**, mean
depth 10.6, deepest 16, nest-visits 3,773 — the water fixes move the scene
the same direction they moved it alone (2 → 7 in §H2's paired datum), on
top of the restored terrain.

**Not §M's springs water.** The springs pass places nothing in this scene's
world; the collapse is the residuals/region gate, a different pass on the
same branch. §M stands untouched.

The original filing follows, kept for the record.

`examples/ascii.rs`'s `forage_loop_scene` fails its own sessility guard on
`main`:

```
the colony has gone sessile: 2 round trips of 8+ cells (measured 98 here),
deepest excursion 15 cells, reach profile [689, 22, 8, 2, 0, 0, 0, 0]
```

The bar (`forage_trips >= 14`) was set in `da252dc` from **98** measured on
this same scene at 12,000 frames, with the profile
`[3858, 475, 185, 98, 1, 0, 0, 0]` that README's M18 status still quotes.
Every bucket is down about 5x and the long tail is gone. **The bar has not
been moved**, and it should not be until the cause is known.

**Deterministic, not noise.** Identical counters on a contended run and a solo
one — `moves 5040 blocked 156 pickups 1340 drops 1310 deliveries 143` both
times — with only the timings moving (worst 66.3 vs 89.8 ms, mean 3.928 vs
3.957). One scene reproduces it in 50s (`ascii scene=foraging`).

**Why nobody saw it.** Neither `da252dc` nor `5a9e594` lists `ascii` among its
gates — both list tests, clippy, docscheck and acceptance — and the CI job had
been `continue-on-error` over bug H since `0a345c4`. A blanket quarantine taken
out for one known red absorbed a second, larger, unknown one, for two commits.
That is the same defect as a skipped step, and it is why `ascii` now
quarantines by scene name instead.

**Not attributed.** 25 commits sit between `5a9e594` and `main`, including the
world-scale branch's `worldgen`, `evaporation`, `field` and `weather` work, and
this scene builds its world from `worldgen::generate` — so a terrain, rain or
moisture change is as plausible as a creature one. A bisect over that range is
the obvious next step and is cheap now that one scene runs in 50s.

**What is ruled out, by measurement.** Not starvation and not a missing food
supply — the opposite. The scene's food census, attributed by material for the
first time, reads at 12,000 frames:

```
food stock 1459080 energy, of which corpse 0 | leaf 1279920 (88%),
litter 164520 (11%), ant 7200 (0%), moss 4680 (0%), seed 2760 (0%)
```

The stock **triples** over the run (441,360 -> 1,459,080) while the colony
eats **0** and delivers 143. So the world grows food faster than 55 ants can
consume it, and it grows it *overhead* — 88% is leaf on standing trees, within
a body length of wherever an ant is. This is README limitation #1 ("the floor
feeds the colony and the colony stops ranging") arriving far more extreme than
the numbers recorded there, and with the **canopy**, not the floor, as the
term that dominates.

Not the litter, also by measurement. Paired, same seed, rebuilt between arms:
`litter.ron`'s `decay_chance_damp/dry` 0.5/0.1 -> 0.9/0.4 cuts standing litter
**4.7x** (164,520 -> 34,800 energy, 11% -> 3%) and moves the colony from 2
round trips to **3**, deepest 15 -> 15, moves 5,040 -> 4,863, deliveries 143 ->
123. The knob is connected; the ants do not notice, because 96% of their food
is still hanging above them.

**Whether the colony *should* range more is a design call, not a bug fix**, and
it was on the owner's queue as card `20260823T091259637Z-9a41e4` ("How scarce
should the forest floor be?"). **Answered 2026-08-23: the abundance is not
intended, and the lever he names is upstream of the one the card asked
about** — *"I think leaves are just falling too fast which creates too much
food"*. So the target is abscission rate, not litter decay rate; he also
picked the *slower*-rotting arm on looks, and rotting faster measurably makes
the floor worse (§O). That does not change this entry: the guard was set from
a measurement and now misses it by 7x, whatever the intended abundance turns
out to be. The bug here is narrower and stands whatever he
answers: a guard set from a measurement now misses it by 7x, and nothing in CI
said so.

Blocks the deposition half of §H, which needs ants that travel to have
anything to measure.


### R2. An ant put down on open water stands on the surface for ever, and `found_colony` puts them there — **OPEN, found 2026-08-23 while rendering a WP-9 review card**

Found by looking, not by a metric: every attempt to render `scene=colony` for
a review card came back as a line of ants strung out across a lake.

    cargo run --release --example filmstrip -- scene=colony seed=2 \
        channel=gutbias start=0 every=4 count=1 cols=1 zoom=8 \
        crop=230,112,150,30 out=/tmp/water.png

**Frame 0 — before one creature tick has run — already shows it**, as an
evenly spaced dashed line sitting on the water surface at exactly
`COLONY_ANT_SPACING`. So the first half of this is placement, not
locomotion. Re-render the same crop at `start=600` and the same ants are
still up there, now *irregularly* spaced: they have been walking about on it
for six hundred frames. Reproduced with `climbs_over_kin: false`, so it is
not WP-9's climb-over-kin — that change grants footing on a *nestmate*, and
these ants are alone on open water.

Two independent causes, both in `creature.rs`, and either alone is enough:

1. **`World::found_colony` will plant on water.** Its `surface` closure takes
   the first cell that is not `Empty` or `Gas` — `Liquid` included — and
   plants an ant one row above it. The nest-painting loop six lines above it
   *does* exclude water, and says why in a comment ("painting over water or a
   creature would be a surprise"); the ant loop never got the same guard.

2. **Nothing makes it fall afterwards.** `step_chain`'s whole-piece support
   test wants `Solid | Powder | Plant` in the 8-neighbourhood, so an ant over
   water is correctly judged *unsupported* — but the fall it then attempts
   requires every cell of the fallen chain to land somewhere `World::is_empty`
   says is free, and water is not free. The fall is refused and the ant stays
   exactly where it was. `head_has_foothold` also refuses `Liquid`, which only
   means water is never *preferred*: footing is a score among three candidates,
   not a veto, so an ant with water on all sides still steps onto it.

**Why it matters past looking wrong.** `scene=colony` runs the `wetland`
preset, which has lakes by definition, and on the seeds where the colony
lands near a shore a large share of the population spends the whole run
standing on one. Foraging and survival numbers taken from that scene are
then partly numbers about ants on water. Seeds measured while looking for a
usable review scene: `seed=2` (35 placed) and `seed=8` (34 placed) are both
mostly-on-the-lake; `seed=12` (31 placed) and `seed=9` (34) are mostly on
land; the default seed and `seed=1` panic in the scene's own
`expect("some dry ground")`.

**Not established, and stated as a guess:** the scene's `would_place` scorer
uses a *different* surface predicate from the one `found_colony` uses —
`dry_surface` searches from `y = 0` and demands `Solid | Powder`, while
`found_colony` searches downward from the cursor row and accepts anything
solid-or-liquid — so the scene can believe it chose dry ground while
placement lands on the lake. That the two disagree is a fact about the
source; that it is the whole explanation for a given seed's count is not
measured.

**Not fixed here.** The placement half is a one-line guard (match the
nest-painting loop's `Solid | Powder`). The locomotion half is a design
question — does an ant drown, float, or swim? — and both are outside
WP-8/WP-9's scope, so this is filed rather than patched. Whoever takes it
should do the two halves together: fixing only placement leaves an ant that
wanders onto a pond still walking on it.


### S. Every destructive verb but the brush leaves the structural scheduler pinned at its cap for ever — **OPEN, found 2026-08-25 by measurement; rescoped the same day from "one explosion" to the pick and the hammer too**

Found by the frame-cost audit (`Reports/frame-cost-audit-2026-08.md`) while
answering the owner's question — *"saving a few ms in static play but then
everything freezes when actually playing is wasted effort"*. It is the one
finding of that audit that is a **bug** rather than a cost; everything else it
turned up was work the frame legitimately has to do.

**The clearest single case: one blast, and the world never recovers.** (The
pick and the hammer do the same thing over a couple of hundred uses — see the
verb table below, which is what this entry is really about.) 8192x2560,
`preset rolling seed 1`, a single radius-20 charge at frame 1,700 and nothing
else — no ants, no player, no second charge — measured to frame 10,500:

| | idle, before the charge | 9,000 frames after it |
|---|---|---|
| pending active sites | ~5,400 | **117,166**, still climbing linearly |
| sites served per frame | 9 | **2,000 — the cap, every frame** |
| scheduler phase | 0.01 ms | **8-13 ms** |
| whole frame, amortised | 13.33 ms | **31.21 ms** |
| frames over the 16.6 ms budget | 29.6% | **97.1%** |

The charge is visually over in a second or two. Two and a half minutes of play
later the frame is still more than twice its idle cost and getting worse, and
`blasts` itself is 0.002 ms — the explosion is not what is expensive, the
queue it left behind is.

**Reproduce** (about five minutes):

```
SCHED_PASS=600 cargo run --release --example scale_probe -- \
  size=8192x2560 phases=1 warm=1500 frames=9000 load=blast:200:1
```

`load=blast:EVERY:COUNT`'s `COUNT` exists for this measurement and is the
whole reason it is readable: with charges still arriving, *a queue that never
drains* and *a queue that drains slower than it fills* look identical. Fire
one and the two separate. This entry was first written from an
eleven-charge run and could not have distinguished them.

#### It is not the explosion. It is every destructive verb but the brush.

**§S was first written as an explosion bug and that was wrong.** Enumerating
the production callers of `World::record_disturbance` — the engine's "I
damaged something" signal — there are five, and exactly one pays for a
converged pass:

| verb | `structural::relax_region`? |
|---|---|
| `World::paint_capsule` (the brush) | **yes** |
| `explosion::trigger` (the charge) | no |
| `rigid::strike` (the hammer) | no |
| `rigid::mine_swept` (the pick) | no |
| `fire.rs` burnout, relayed through `parallel.rs` | no |

The other four stop at `record_disturbance` plus
`schedule_structural_check_around` and hand the whole correction to the
reactive wavefront.

**Measured on the pick and the hammer** — the two verbs a player spends most
of their time in. 200 uses each at the app's own `brush_radius` of 6, one
every 20 frames, no explosion anywhere in the world, all three arms over the
same 9,000 measured frames:

| | idle | 200 pick cuts | 200 hammer swings | 1 radius-20 charge |
|---|---|---|---|---|
| cells actually removed | — | 10,893 | 7,863 | — |
| whole frame, amortised | 13.33 ms | **30.44 ms** | **31.68 ms** | 31.21 ms |
| frames over the 16.6 ms budget | 29.6% | **86.2%** | **97.2%** | 97.1% |
| scheduler phase | 0.32 ms | 9.81 ms | 12.72 ms | 11.62 ms |
| pending sites at frame 10,200 | ~5,400 | 88,160 | 110,810 | 117,166 |
| awake chunks at end | — | 49 of 5,120 | 57 of 5,120 | 63 of 5,120 |

**Ordinary digging costs what a blast costs.** The world is materially still
at the end of all three — under 60 awake chunks of 5,120 — and every one of
them is still servicing 2,000 structural checks a frame.

**And the two hand verbs are a control on each other.** The hammer removes
**fewer** cells than the pick (7,863 against 10,893) and costs **more** (31.68
against 30.44 ms; queue 110,810 against 88,160). So the driver is not how much
material came out. The candidate it does fit is **crack reach**, which is the
one thing that orders the three verbs the same way the cost does:

| verb | cracks out to | leak arrives at |
|---|---|---|
| `mine_swept` | `radius + MINE_CRACK_REACH` = 8 | ~105 cuts |
| `strike` | `radius * CRACK_REACH` = 18 | ~15 swings |
| `explosion` | the joint halo, far wider | 1 charge |

That is a hypothesis with three points on it and an obvious mechanism —
`edge_is_cracked` removes edges from the relaxation graph, so a crack halo
invalidates shortest paths over its whole area rather than over the hole.
It is **not established**: the three verbs differ in force and staging as
well as in crack reach, and nothing here varied crack reach alone. The
measurement that would settle it is `MINE_CRACK_REACH` and `CRACK_REACH`
swept against the queue, one verb at a time.

**It arrives as a knee, not a ramp**, which is the part worth reading twice:

| frame | pick: cuts / pending | hammer: swings / pending |
|---|---|---|
| 1,800 | ~15 / 5,379 | ~15 / 10,512 |
| 2,400 | ~45 / 5,159 | ~45 / **29,473** |
| 3,000 | ~75 / 5,588 | ~75 / 43,982 |
| 3,600 | ~105 / **18,653** | ~105 / 53,322 |
| 4,200 | ~135 / 30,277 | ~135 / 67,188 |
| 10,200 | 200 / 88,160 | 200 / 110,810 |

The pick sits at the idle heap through seventy-five cuts and then goes to the
`MAX_SITES_PER_FRAME` cap and never recovers. So this is not "each cut costs a
little": it is a **state change**, presumably the point at which the
excavation becomes large enough to sever a load path rather than nibble a
face. The hammer reaches the same state within about fifteen swings.

**Anyone reproducing this must dig past the knee.** A twenty-cut probe with
the pick measures nothing and reads as a clean bill of health.

**The counter is what makes the null defensible, and the first version had
none.** `rigid::is_tool_target` takes `Solid | Plant` and refuses bedrock, so
a probe aimed at the topmost `Solid | Powder` cell — soil, on a rolling world
— swings into dirt every time. The first run of this measurement reported
**200 cuts, 0 cells removed** and a queue flat at 5,400 for 105 cuts, which
reads exactly like "the pick is fine". `scale_probe`'s `load:` line now prints
cells *actually removed* beside swings taken: the runs above removed **10,893**
and **7,863**.

**Fire is unmeasured and is the same shape.** A burning front reports itself
disturbed along its whole length, continuously, for as long as it burns —
a blast's geometry sustained rather than instantaneous — and it takes no
converged pass either. Nothing here has measured it and nothing should assume
it either way; there is no `load=fire` component yet.

#### It is not the ants — and that is worth saying, because it was the standing assumption

Ablated at the same size, 3,600 frames:

| load | sites served | heap after the batch | scheduler phase |
|---|---|---|---|
| `ants:64,gnome` @2,400 | 38 | 5,252 | 0.19 ms |
| `ants:64,gnome` @4,800 | 67 | 6,216 | 0.36 ms |
| `blast:300` @2,400 | **2,000 (the cap)** | 23,744 | 7.27 ms |
| `blast:300` @4,800 | **2,000** | 77,100 | 8.99 ms |

A 64-ant colony is nearly free and its queue is **stable**. `creature::tick`
was the expected answer and it is not the answer. ~99% of the loaded
scheduler time is `ActiveKind::StructuralCheck` (7.17 of 7.27 ms; 8.90 of
8.99 ms).

Read `deferred` for what it is: `world.active_site_count()` sampled *after*
the frame's batch has been popped, so it is the whole heap, not the capped
remainder. Its idle value at this size is ~5,400 — ordinary future-dated
growth and evaporation sites — and that is the number a drained queue comes
back to.

#### The shape: an 8x oversubscription, not a spike

`schedule_active_site` dedups `StructuralCheck` **by position**, so the heap
is a *set of cells with a check pending*. At 117,000 pending, each
rescheduling itself every `STRUCTURAL_TICK_INTERVAL` (5) frames, the queue
demands ~23,000 services a frame against a `MAX_SITES_PER_FRAME` of 2,000.
It cannot clear, and every cell it does serve schedules more:

```
[sched] frame 2400 sites 2000 produced 7651 deferred  25876  10.08ms
[sched] frame 4200 sites 2000 produced 7631 deferred  53077   9.32ms
[sched] frame 6000 sites 2000 produced 7014 deferred  73233  10.32ms
[sched] frame 7800 sites 2000 produced 5743 deferred  91409  10.17ms
[sched] frame 9600 sites 2000 produced 5854 deferred 112400  11.39ms
```

`produced` ~6,000-7,600 against 2,000 served. Most of that is absorbed by the
position dedup — the heap grows only ~+12 a frame — which is why this reads
as a *treadmill* rather than an explosion in memory, and why nothing has ever
noticed it: nothing runs out, nothing asserts, no frame spikes. It just costs
8-13 ms a frame for the rest of the session.

#### Chain mode is not the lever, and the source already says why

The obvious next thought is that `chain_reach` bounds it. It does not, **and
the argument for that is in the source rather than in the timings below** —
see the caveat after them.

The reason is in `structural.rs`'s own `MAX_DISTURBANCES` doc: the disturbance
ring is scanned *"once per record and once per cell that has **already reached
a failing verdict** (`within_disturbance`) — never per cell per frame."*
`chain_reach` is a leash on **consequences**, applied after the load walk has
already been paid for. It decides whether a failure is *licensed*, not whether
a check *runs*. A queue of checks is upstream of every point at which the
leash is consulted, so no setting of it can shrink one, and it must not be
offered as a fix for this.

The four-way run agrees, for what that is worth — same scene, 3,600 measured
frames each:

| `chain=` | reach | scheduler, amortised | whole frame | over budget |
|---|---|---|---|---|
| SPREAD (the shipped default) | `i32::MAX` | 9.722 ms | 27.589 ms | 91.0% |
| LOCAL | 48 | 9.717 ms | 27.312 ms | 90.9% |
| TIGHT | 16 | 10.030 ms | 27.890 ms | 90.9% |
| NONE | 0 | 9.747 ms | 27.250 ms | 91.3% |

**Read that table as corroboration, not evidence, and do not re-derive
anything from it.** It is wall clock and nothing else: no counter was taken
per mode, and the whole spread is 3% — inside what a contended box produces
on its own. The docs-audit lane measured two runs of a *byte-identical*
`ascii` binary disagreeing by 2.42x and reversing the serial/parallel
ordering (`Reports/measurement-under-contention.md` — **in flight on that
lane's branch as of 2026-08-25**, not in this directory yet), which is enough
to
manufacture a 3% null or to hide a 3% effect either way. The one thing the
table does establish is that the knob was **connected**: the worst frames
spread 72.9 / 96.7 / 99.4 / 74.7 ms, so these are four different worlds
rather than one stale binary run four times, which is `CLAUDE.md`'s
stale-binary tell.

If someone needs this claim to carry weight on its own rather than as a
footnote to the source argument, the measurement to take is `deferred` and
`produced` per mode — counters, which reproduce exactly where the clock does
not (see the census note in the mechanism section below).

#### Mechanism — attributed, 2026-08-25

`SCHED_PASS` now prints a second `[struct]` line splitting the structural
share by which branch of `structural::tick` produced it: `worsened` /
`improved` / `unmoved`, the two defer reasons, and the largest distance
written. Two candidates were visible in `tick` before the census, and they
wanted opposite fixes, so guessing between them was not on:

1. **The distance wavefront** (`moved`) fans out to five sites — itself plus
   four neighbours — and is load-bearing. Dropping it froze `scene=capped`
   completely: at frame 3,000 all 15,840 cells were still at `aux 0` and not
   one had ever been load-evaluated. Nothing here proposes removing it. But
   within it, `worsened` is the **count-to-infinity** climb an unanchored
   region performs — this module's own doc names the dynamic — and `aux` is a
   full `u16`, so such a region's cells climb toward 65,535 one step per tick,
   producing five sites each time, for ever.
2. **The out-of-budget defer** (`if world.load_budget == 0 { defer!(); }`) is
   fan-out 1 and cannot make progress on a frame whose budget was spent before
   it was reached. Dispatch is ordered by `(next_frame, x, y)`, so low-`x`
   work wins the budget every frame and starves everything behind it — a
   hazard the same file already records having been bitten by.

**It is the wavefront, and it is not count-to-infinity.** Same scene, same
charge, `SCHED_PASS=600`:

```
[struct] frame 2400 worsened 1400 improved  9 unmoved 267 | budget0  723 | max aux 438
[struct] frame 3600 worsened 1276 improved 29 unmoved 601 | budget0 1863 | max aux 639
[struct] frame 4800 worsened 1278 improved  9 unmoved 596 | budget0 1715 | max aux 816
[struct] frame 6000 worsened 1292 improved 10 unmoved 670 | budget0 1472 | max aux 917
[struct] frame 6600 worsened 1007 improved 18 unmoved 932 | budget0  773 | max aux 1018
```

About **1,300 of every 1,900 structural ticks are cells whose distance rose**,
against ~10-50 that fell, and `max aux` climbs almost perfectly linearly at
~85 per 600 frames — one step per service round. Candidate 2 is real but
secondary: `budget0` runs 700-1,900 of the batch and is a rider on the
wavefront, not its source.

(The `[sched]` line is byte-identical with the census compiled in — `produced
7042 deferred 61488` at 2,400, `7014 / 73233` at 6,000, `5513 / 79842` at
6,600, matching the run without it. The instrument does not perturb what it
measures.)

**And the third explanation is the right one, which the source states
outright.** `structural::relax_region`'s doc:

> `tick` relaxes one cell per scheduled check, and reschedules its
> neighbours `STRUCTURAL_TICK_INTERVAL` frames later — so a wavefront
> advances roughly one cell per 5 frames. That is the right shape for a
> *disturbance*, which is local and whose consequences should arrive
> progressively. **It is the wrong shape for material appearing from
> nothing** ... A 192-cell column needs ~192 rounds — over fifteen seconds at
> 60 Hz.

Removing a crater is the same event as adding one: the shortest path to
bedrock changes for every cell that routed through it, so the true distances
rise across a region far larger than the hole. `World::paint_capsule` already
knows this and pays for **one converged pass** over what the stroke touched
(`world.rs`, `if touched_structure { ... relax_region(self, region) }`).
Worldgen does the same, world-wide, with `compute_world_distances`.

**Nothing on the explosion path does either.** A radius-20 charge hands its
whole correction to the reactive wavefront, which then advances one cell per
five frames through a region the measurement above puts at >100,000 cells
while serving 2,000 a frame. That is the bug, and the fix already exists in
this file — it is simply not wired to this verb.

**What is still open**: whether the wavefront ever terminates. It had not
turned over 8,800 frames after the charge, and at ~85 distance-units per 600
frames it is nowhere near `u16::MAX`; a genuine convergence to true distances
would stop and this measurement cannot say when. For the player the
distinction is academic — either way the frame is over budget for minutes —
but it decides whether a converged pass is a *fix* or merely a large
speed-up, so do not assert one.


#### What this is *not*, and its neighbours

- **Not §1j.** §1j is that `MAX_LOAD_CELLS_PER_FRAME` bounds only one of the
  load model's walks, so a *single* frame can cost 118 ms. This is that the
  *number of expensive frames* is unbounded. They compound — the budget is
  what makes each capped frame cost 8-13 ms rather than 80 — but fixing either
  leaves the other standing. §1j's 118 ms figure is still owed a re-take with
  `PROBE_NO_LOAD=1`; that is not done here either.
- **Not the reverted parent-scheduling dead end**, though it has the same
  signature. `structural.rs` records "every settling cell raised a fresh check
  on its parent ... faster than the queue drained": `scene=capped` pending 26
  → 4,064, frame cost 2.5 ms → 3,160 ms. That mechanism is gone; this
  reproduces without it, at 20x the world size and through a different verb.
  Read that comment before proposing anything — it is the record of what the
  obvious fix costs.
- **Not visible at 512x320**, which is why nine sessions of explosion work did
  not find it. `scripts/blastsweep.sh` is the standing explosion artifact and
  it is a generated *rolling 512x320* world; it watches
  `FailureCounts::confined` for exactly this class of treadmill and has never
  watched the scheduler heap. And every earlier frame-cost number in the repo
  measured part of a frame, at idle.
- **Not confirmed in the real app.** Everything above is headless. Confirming
  it wants `xvfb-run` plus the capture hook and a stopwatch, not another
  probe, and it has not been done.

#### Do not "fix" this by

- **Raising `MAX_SITES_PER_FRAME`.** The queue's demand is ~23,000 services a
  frame; raising the cap raises the per-frame cost and clears nothing. §1j
  records the same non-result for `MAX_LOAD_CELLS_PER_FRAME`: 12,000 → 20,000
  → 40,000 gave 118 ms and 118.7 ms, identical.
- **Lowering `chain_reach`.** Measured above. It is the wrong quantity.
- **Dropping the wavefront.** Measured, in place, in `structural::tick`'s own
  comment. It does not degrade; it fails totally and silently.
- **Retiring a site after N unproductive checks**, in the shape
  `organism::STALE_LIMIT` uses, without first establishing what wakes it
  again. `GROUNDED_RECHECK_INTERVAL`'s doc is the record of what happens when
  a structural cell stops asking and nothing re-asks it: a lone stone hanging
  in open water for the rest of a run, and `load::evaluate` calling it
  UNSUPPORTED the whole time.

#### The fix: prototyped, measured, and the box size is the whole of it

Give the explosion the same converged pass the brush already takes —
`relax_region` once, when the blast's last stage has run. Prototyped on
`claude/perf-blast-relax` (`explosion.rs::settle_structure`), and the first
sizing was **wrong in an instructive way**.

**Sized to the charge (`damage_extent + 4`): nothing.** Pending 45,134
against the baseline's 43,789 at frame 3,600. It works perfectly for exactly
one frame — 100 frames after the bang the baseline reads `worsened 199,
produced 1393` and the pass reads `worsened 0, unmoved 391, produced 10`, so
the blast's own correction is genuinely finished — and by frame 2,400 the
wavefront is back at `worsened 1637`. `relax_region` seeds its boundary from
the values just *outside* the box and treats them as correct, so where a
charge severs a load path the outside is stale-low, the inside converges to a
value that has to rise again, and the correction simply restarts from the box
edge instead of the crater.

**At eight times that box the backlog appeared to disappear — and it was an
artifact.** The numbers first looked like this:

| frame | baseline | 8x box |
|---|---|---|
| 2,400 | 25,876 pending / 10.08 ms | 5,134 / 0.03 ms |
| 3,600 | 43,789 / 13.02 ms | 5,812 / 0.10 ms |
| 6,000 | 73,233 / 10.32 ms | 6,586 / 0.16 ms |

5,134 is *below* the ~5,400 idle heap, whole frame 31.21 → 18.98 ms, 97.1% →
49.2% over budget, and `scripts/acceptance.sh` green on every case with the
big box in. It reads as a complete fix.

**It is not one. `relax_region` anchors differently from `tick`, and that is
the whole of the result.** `relax_region` seeds any cell with
`is_resting_on_ground` at distance 0 outright; `tick` takes that root only
when relaxation leaves no path at all. So a big box over a blast's rubble
field roots a large region at zero and the structural system stops having
anything to say about it. The tell was `max aux` — 142 after the pass against
2,482 for the small box, an order of magnitude of support field simply gone.

**The control settles it.** `SETTLE_GROUND=0` on the prototype branch makes
`relax_region` use `compute_world_distances`' bedrock-only rule and changes
nothing else. Same scene, same 8x box:

| frame | baseline | 8x box, ground anchors on | 8x box, **bedrock only** |
|---|---|---|---|
| 2,400 | 25,876 / 10.08 ms | 5,134 / 0.03 ms | **19,674 / 10.00 ms** |
| 3,000 | 36,818 / 10.43 ms | 5,346 / 0.14 ms | **32,877 / 8.85 ms** |

With the anchor rule held fixed the converged pass buys **nothing**. The queue
tracks the baseline, and `max aux` is back to 2,482.

So: **the converged pass is a dead end for §S as prototyped**, at every box
size tried, and the encouraging arm was measuring an immunity rather than a
convergence. Filed separately as **§S2**, because the anchor-rule
disagreement is a defect in its own right and affects the brush today,
independently of anything here.

`CLAUDE.md`'s *"look again after the fix, for what you did not measure"*, and
its warning that a green suite is evidence about the tests: acceptance was
green on all cases *while* the blast neighbourhood was being rooted flat, and
the 8x arm would have shipped on it.

#### Why the pass does not work, as far as this went

`relax_region` computes exact distances inside its box **given correct
boundary values**, and after a charge severs a load path the values just
outside are stale-low for every cell that routed through the crater. The
interior then inherits the error. Growing the box moves the boundary without
removing it — which is consistent with both arms above, and is the reason a
multiplier was never going to be the shipping form.

What that leaves, untried: a region derived from *what actually changed*
rather than from the charge — invalidate the subtree of every cell whose
support parent was destroyed, then Dijkstra from the boundary of **that** set,
which has correct values by construction. That is the textbook
increase-aware dynamic shortest-path shape and it has no box at all. It is
also real work, and nothing here has measured it.

#### And a 440 ms frame, whatever the region turns out to be

The `blasts` row's worst goes to 440.75 ms with the pass in — the pass itself,
in one frame. A quarter-second freeze at the bang is not a trade this repo
makes, so any converged pass has to be amortised across frames regardless of
how its region is chosen.

#### And it is still a behaviour change, for the owner to judge

The owner's stated requirement is *"collapse must be obvious and delayed, so
the player can get supports in first"*, and `CHAIN_WINDOW_FRAMES` is 600
frames of deliberate generosity in service of it. **Some part of the delay a
player currently sees after a blast is this wavefront crawling** — so a
converged pass could make collapse arrive nearly instantly and read as worse,
even while the frame cost falls off a cliff. That is a judge-by-eye question
and belongs in front of the owner as a blind A/B (`filmstrip`, frames rather
than a GIF, with the failure counts in the card's `meta`), not in a commit
message. See also §D2, the same quantity from the other side: *"a room's
collapse arrives at frame ~350 where it used to arrive at ~150"*, filed as a
regression.


#### What the cost actually is — the census, 2026-08-26

**It is not the load model, and §S was filed believing it was.** The entry
above says the correction "advances one cell per five frames"; the `[struct]`
census says the correction never arrives at all, because the cells are not
advancing, they are climbing.

`scale_probe size=8192x2560 phases=1 warm=1500 frames=12000 load=blast:200:1`
with `SCHED_PASS`, one radius-20 charge at frame 200, read at **frame 13,200 —
eleven thousand frames after the only event**, with the world materially still
(59 awake chunks of 5,120, 0 particles in flight):

| | per frame |
|---|---|
| `worsened` | **1,464** |
| `improved` | 7–48 |
| reached the chain walk (`chain_deferred`) | **1** |
| `uninteresting` | 941–1,056 |
| sites drained / produced | 2,000 / **7,631** |
| `max aux` | 438–725, oscillating, never settling |

**One in two thousand sites reaches `failing_along_support_chain` at all.**
The 12–15 ms the scheduler spends is the distance relaxation itself: ~1,400
cells per frame each raising their distance by one step and fanning out to
five sites through `schedule_solid_neighbours`, which is the whole of the
~7,600 produced against the 2,000 drained. `improved` sitting near zero is the
tell — a converging field improves, and this one only ever worsens. It is
Bellman-Ford's count-to-infinity, which `compute_world_distances`' own doc
already names as the reason worldgen must not route through the reactive path.

So the load model is not the target, and neither is `MAX_SITES_PER_FRAME`.

#### The oracle: a converged field *is* a fixpoint — measured 2026-08-26

The question no amount of tuning the reactive path can answer, and the one
that decides whether `Reports/structural-reconvergence-design.md` is aimed at
the right quantity at all: **if the field were converged, would it stay
converged?** `RECONVERGE_AT=<frame>` in `scale_probe` runs
`compute_world_distances` once, mid-run, and prints the queue either side.

Same scene, charge at frame 200, converged pass at frame 3,000:

| frame | scheduler | pending | `worsened` | structural sites drained | `max aux` |
|---|---|---|---|---|---|
| 2,400 | 13.72 ms | 25,876 | 1,400 | 1,678 | 438 |
| 3,000 | 14.36 ms | 36,818 | 1,401 | 1,971 | 550 |
| 4,200 | 12.49 ms | 53,077 | 1,464 | 1,924 | 725 |
| — | *converged pass, 2,016 ms, one-off* | | | | |
| 4,800 | **0.25 ms** | **6,094** | — | **14** | — |
| 6,000 | **0.45 ms** | **6,932** | **2** | **9** | 203 |
| 7,200 | **1.04 ms** | **7,832** | **10** | **58** | 683 |

The queue collapses to its honest idle value and stays there; the scheduler
goes to ~1% of its loaded cost. **§S is a convergence bug, confirmed**, and
the target state is stable rather than something the world walks back out of.

*And it is not the immunity artifact this bug has already produced once*
(`CLAUDE.md`, *a cost that vanishes may be work that vanished*): `max aux`
after the pass reads **203, 443, 683** — live, honest, non-degenerate values,
not the 142 the rooted-flat prototype gave. The anchor rule is untouched here;
only the field's disagreement with itself is removed.

#### How big the real fix has to be — 67,100 cells, not 250,000

The same oracle censuses every body cell's `aux` either side of the pass, so
the affected set is now measured rather than estimated:

| | body cells | changed | of body | of which rose | largest rise |
|---|---|---|---|---|---|
| **idle, no blast** (the control) | 19,386,874 | **45** | 0.00% | 45 | 65,535 |
| **one radius-20 charge** | 19,386,483 | **67,100** | **0.35%** | 67,100 | 65,535 |

The idle arm is the sanity check the number needs: with nothing wrong it reads
45 cells out of 19.4 million, so 67,100 is the charge and not the instrument.
Every changed cell **rose**, which is what an increase-aware update is for, and
the largest rise is `u16::MAX` — cells that genuinely lost every path.

**Three consequences for the fix.**

- The scope report's estimate of *"~63 chunks, about 250,000 cells"* is **3.7x
  too pessimistic**. The true set is 67,100.
- At the whole-world pass's own rate (1,918 ms / 19.4 M ≈ **99 ns/cell**), a
  scoped pass over 67,100 cells is **~7 ms, once** — against the ~14 ms *every
  frame, for ever* that it removes. One frame's spike buys the rest of the
  session.
- **That is why the box prototype cost 440 ms**: an 8x box is a ~30x overshoot
  of the set that actually changed. The 440 ms is not the price of converging,
  it is the price of converging the wrong 97% of the region — which makes
  amortisation (scope report §3) genuinely optional rather than merely
  deferrable.

#### The queue goes quiet because it converged, not because the world fell

The control this finding needed and did not have when it was first written.
`CLAUDE.md`'s *a cost that vanishes may be work that vanished* has already
caught one §S prototype whose queue went silent because the whole blast
neighbourhood had been rooted flat — and **the same reading fits this
oracle**: a pass that raises 67,100 distances pushes some of them past their
span, so they fail, fall, and leave a quiet world with less in it. `max aux`
is the wrong instrument for that; it says the field is live, not that the
rock is.

The instrument that answers it is a body-cell census at the end of the run.
Same scene, same charge, 6,000 frames, the only difference being whether the
converged pass ran at frame 3,000:

| | body cells standing at end | whole frame | over budget |
|---|---|---|---|
| no pass | 19,471,238 | 42.27 ms | 96.9% |
| **converged pass at 3,000** | **19,496,708** | **33.29 ms** | **85.3%** |

**25,470 *more* cells survive with the pass.** So the direction is the
opposite of the demolition reading, and it should have been predictable:
`compute_world_distances`' own doc says the count-to-infinity dynamic makes
*"a cell whose true distance is small climb past its own span before the real
anchor value reaches it, break, and take its neighbours with it"*. Converging
the field does not merely stop the scheduler thrashing — it stops the engine
destroying rock that was never unsupported. §S is a correctness bug wearing a
performance bug's clothes.

(The mean-frame figures span the whole 6,000 frames, half of them before the
pass, so 33.29 ms understates the settled improvement; the scheduler table
above is the one to read for that.)

#### The error is *manufactured*, not delivered — and this redirects the fix

The single most important number here, and it overturns this entry's own
scope. The oracle run at increasing distances from the charge:

| oracle at | cells wrong | of body |
|---|---|---|
| **5 frames after the charge** | **369** | 0.00% |
| 50 frames after | 42,825 | 0.22% |
| 1,300 frames after | 67,100 | 0.35% |

**A radius-20 charge invalidates about three hundred and seventy cells.** The
other sixty-seven thousand are produced afterwards, by the reactive
correction itself — the climb spreading a small, real error across a huge
region while the collapse cascade keeps feeding it.

`Reports/structural-reconvergence-design.md` §1 is scoped as *"converge the
field over what actually changed"*, sized at the affected set. That set is
369 cells and converging it is nearly free — **but converging it once does
not fix §S**, because the manufacturing continues for as long as the cascade
does. Whatever ships has to keep the field converged *while a collapse is
running*, not repair it once at the bang.

#### Ruled out: damage-seeded reconvergence alone — 2026-08-26

Built, measured, landed **off by default** as `STRUCT_RECONVERGE=1`
(`structural::reconverge_from_damage`). It is §1 of the scope report as
written: seeds from every hole a verb opens, closes the invalidated set in
increasing-stored-distance order, Dijkstras inward from the boundary that is
still correct by construction, and takes `relax_region`'s ground fixpoint.

**It fires and it is not enough.** Firing evidence, from `SCHED_PASS`: the
charge frame invalidates and repaths in the hundreds to low thousands, at
0.02–5 ms a pass, and the queue is genuinely quiet immediately afterwards —
frame 1,800 reads `deferred 8,216 / worsened 71` where the control reads
`8,723 / 199`. Then the cascade starts and it climbs back to 106,444 by frame
8,100, which is the control's curve.

Against the oracle it recovers 369 → **292** wrong cells five frames after
the charge, and 67,100 → **70,683** at frame 3,000. So the immediate repair
is real and small, and it buys nothing downstream **because the error was
never downstream of the damage — it is downstream of the correction.**

One implementation trap found on the way and fixed, worth keeping because it
made the pass look inert for a whole measurement: `structural::break_free`
converts rock to debris — a structural hole — and **never calls
`schedule_structural_check`**. The failing region reaches the heap through
`schedule_solid_neighbours`, which builds `ActiveSite`s with `reschedule()`
and hands them to `scheduler::step` to push directly, bypassing the seed
funnel entirely. So every hole a *cascade* opened went unseeded while every
hole the *player* opened did not. `World::record_structural_hole` is the fix.
It changed the aggregate by 4 cells, which is its own lesson: the seeding gap
was real and was not what was wrong.

#### ~~The framework is the bug, not the tactics~~ — **WRONG, superseded 2026-08-26**

**The cause is a one-line aux carry in `particle.rs`, not the support model.**
Found by the support-model session (`Reports/structural-support-model.md`,
in flight at the time of writing); the two facts below are re-verified here
against the source rather than taken on report.

**And this entry had the sign backwards, which is what sent it at the
framework.** The oracle's `rose` counter increments when `now > *old`, where
`now` is the value *after* `compute_world_distances` — the truth — and `*old`
is what was stored. So `rose` means **the truth is higher than the stored
value**: the stored distance is too **low**, the cell reads as *closer* to an
anchor than it is, and therefore as **better** supported. Every sentence in
the original that read this as cells "looking further from support than they
are" was inverted. The `|delta|` histogram cannot catch it — it is an
absolute value, and `rose == changed` on every reading was the number that
said which way it pointed.

**The mechanism.** `particle::landed_cell` (`src/sim/particle.rs:409`) builds
a landed particle's cell as `Cell::new(material, shade)` — which starts at
`aux == 0` — and only carries the particle's own `aux` across when the
material is flagged `worth_in_aux`. That flag is the **food-value** gate
(`world.rs:335`, `creature::food_value`'s condition), so it is false for
stone. Every thrown rock therefore lands storing **aux 0**, and `aux == 0` on
a body cell is *bedrock-adjacent*: `load::support_parent` returns `None` for
it outright, "an anchor is held from outside the model". A landed chip is a
**fake anchor**, and everything that relaxes off it inherits a distance that
is too low — a load sink, exactly the failure `tick`'s own `grounded_root`
comment warns about, arriving by a route nobody was watching.

It fits every measurement in this entry, which is why it is worth stating
what the wrong story got right by accident:

- **only verbs that throw material leak.** The blast, the hammer and the pick
  all throw debris; the brush erases in place. §S was filed on exactly that
  split and attributed it to `paint_capsule`'s converged pass.
- **it tracks crack reach**, because crack reach is what decides how far
  debris is thrown.
- **the stored field is too low**, which is the sign above.
- **the climb.** Recovering a distance *upward* costs one relaxation round
  per unit, so a sink planted 2,000 below the truth takes thousands of ticks
  to undo — and that is the count-to-infinity this entry measured, with a
  cause rather than a framework behind it.

**Landing at `u16::MAX` instead takes the oracle from 37,629 wrong cells to
186, and the scheduler from 14.68 ms to 0.03 ms, with no converged pass at
all** (measured by the support-model session, not re-run here).

##### What survives

- **No local rule can see it.** Three fixes failed on that and the reason
  stands: a region relaxed off a sink is internally consistent, so no local
  test and no box that trusts its boundary can distinguish it.
- **Every consumer of `aux` in `load.rs` is a comparison, not a magnitude** —
  `support_parent`'s `neighbour.aux() >= own` and its `saturating_add`
  tie-break, `:856`'s `n.aux() < own`, `:897`'s `cell.aux() > own`, and
  `tick`'s `relaxed == u16::MAX`. Nothing multiplies by it or compares it to
  a span; `load::capacity` is built from section depth and span.
- **The walk is bounded at 48 hops** (`ROOTWARD_CHECK_STEPS`).
- **The coarse layer is small** — measured at **5,169** nodes against the
  5,120 estimated here, and a `(level, portal-distance)` potential packs into
  the existing `u16`.

##### What is falsified

- **The saturating short-horizon gradient.** Proposed here as the local half
  of the split; measured, a horizon of 64 strands **95.38%** of cells at the
  cap, because real distances run to ~2,400. In `dead-ends.md`.
- **"The framework is the bug."** The hierarchy survives only as an M10 item,
  and on a better argument than the one made here: recovering a distance
  upward costs one round per unit, so **the field's depth is the price of any
  accident in it**. Bounding the depth bounds the blast radius of the next
  bug of this shape — which is a reason to bound it, not a reason the model
  is wrong.

#### Ruled out: doing it reactively, inside `tick` — 2026-08-26

The cheap version anyone would try first, and it does not work. Full record in
`Reports/dead-ends.md`; the short form is that `tick` was made increase-aware
in place (a risen distance jumps straight to `u16::MAX` instead of writing the
one-step-worse value, and the invalidation step propagates without judging).
**It fires** — `improved` goes from ~30/frame to ~900/frame, so the field
starts doing convergence-shaped work — **and it does not converge**: pending
still climbs linearly 23,227 → 130,403 between frames 2,400 and 13,200, and
the whole frame is 41.19 ms against the control's 40.14 ms. A monotone climb
is traded for a stable oscillation at the same throughput, because a cell
cannot tell locally whether a neighbour's `u16::MAX` is an invalidation that
will be undone or a genuine dead end. **The closed affected set and the
ordered pass are necessary, not merely tidier.**


### S2. The brush's anchor rule destroys structures the other two rules leave standing — **OPEN, found 2026-08-25 by reading, MEASURED the same day, and the direction is the opposite of the prediction**

Found while prototyping §S's fix, which is the only reason it is written down:
the fix calls `structural::relax_region`, and checking whether that call was
buying convergence or buying *immunity* meant reading all three anchor rules
side by side. They do not agree.

| | anchors a cell when | |
|---|---|---|
| `compute_world_distances` | a `NEIGHBOURS_4` neighbour is `BEDROCK`, or the cell touches the world edge | worldgen, whole world |
| `relax_region` | ...that, **or** `is_resting_on_ground`, unconditionally | a brush stroke (`World::paint_capsule`) |
| `tick` | ...that, but the ground root **only when relaxation leaves no path at all** | every scheduled check |

`tick`'s last-resort rule is not an implementation detail; its own comment
calls it *"the whole of the dig cascade"*:

> Rooting a cell at 0 the moment powder touches its underside makes it a
> *load sink*: every neighbour with a longer path re-routes its load into
> it, which is exactly "a sprinkle of sand under a beam holds the beam up".
> The divisor in `capacity` existed to cancel that — two modelling errors
> roughly annulling each other, which is why tuning the divisor never worked.

`relax_region` does the thing that paragraph is a record of removing. And it
does it as a **Dijkstra seed**, which is the strongest possible version: an
eagerly-rooted rubble-backed cell does not merely read as supported itself,
it becomes a zero-distance source that every cell around it relaxes from.

**Measured, on §S's prototype.** Same scene, same charge, the only difference
being whether `relax_region` seeds `is_resting_on_ground` cells at 0:

| | largest distance written, frame 1,800 |
|---|---|
| ground anchors on (shipped) | **142** |
| ground anchors off (`compute_world_distances`' rule) | **2,482** |

An order of magnitude. That is not a rounding difference in a debug channel;
it is the support field of the whole blast neighbourhood reading as anchored
where it is not.

#### Measured: `examples/anchor_probe.rs`, 2026-08-25

**The prediction in the first draft of this entry was that a painted
structure over loose ground would be *harder* to bring down. It is the
opposite, and by a wide margin.**

One geometry, built once, with its distance field then written three ways —
so the arms differ in the rule and in nothing else. A stone deck three cells
thick runs from a pier that reaches bedrock; a sand pile sits under the
span's marginal region. Default `chain_reach` (SPREAD).

| span | worldgen | tick | **brush** (stroke box) | brush, world-wide |
|---|---|---|---|---|
| 40 | **intact** | **intact** | **59 of 63 deck cells gone** | 63 deck + **87 pier** |
| 45 | **intact** | **intact** | **74 of 78** | 78 + 87 |
| 50 | **intact** | **intact** | **89 of 93** | 93 + 87 |
| 55 | 84 of 108 | 84 | 104 of 108 | 108 + 87 |
| 100 | 219 of 243 | 219 | 239 of 243 | 243 + 87 |

**A deck that stands perfectly under the other two rules is destroyed under
the brush's.** At every span the brush arm also destroys *more* than the
others once failure begins (104 against 84; 239 against 219).

**The control says it is the ground rule and nothing else.** Remove the sand
pile and all four arms are identical at every span — 0 cells at zero, same
damage, same margin. Every difference above is `is_resting_on_ground` being
seeded at 0.

**And the debug field points the wrong way**, which is why this could never
have been found by looking at a support overlay. Under the brush's rule the
deck's largest distance to an anchor is **9** against worldgen's **82**: it
reads as *far better supported*, and it is the arm that collapses.

**The mechanism is the one `tick`'s comment already names.** The rooted
sand-backed cells are distance 0, so every cell in the deck re-routes its
support chain into them — *"every neighbour with a longer path re-routes its
load into it"*. The load model then judges that narrow footing as carrying
the whole deck, fails it, and takes the subtree it was holding, which is the
deck. Eager rooting does not confer immunity; it **concentrates load into a
footing the width of a sand contact.**

The world-wide arm additionally destroys **87 cells of the bedrock-founded
pier** at every span. That is not what a stroke does — `paint_capsule`
relaxes the stroke's box plus 4, which is the `brush` column — but it is the
same rule taken to its limit, and it shows the failure is not confined to
what was painted.

#### What is and is not established

- **Established**: the three rules differ; the difference is large in the
  field they all write; and on this scene it decides whether a structure
  stands.
- **Established**: the direction. Eager rooting makes structures **weaker**.
- **Established**: a freshly generated world has *no* ground roots at all —
  `compute_world_distances` never makes one — and acquires them lazily as
  `tick` visits cells. Painted rock gets them immediately and in bulk.
- **Measured, 2026-08-25**: how often a player meets it — see the frequency
  section below. Short version: the trigger is everywhere and the *outcome*
  is not distinguishable from seed noise.
- **Not established**: which rule is right. `compute_world_distances`'
  bedrock-only rule is the most conservative and is what the world is built
  with; `tick`'s is the considered one and has the reasoning behind it;
  `relax_region`'s appears to be neither, and no comment anywhere argues for
  it.

#### Frequency in generated terrain: the trigger is everywhere, the damage is noise

`anchor_probe worldgen=1` — 18 seeds x 12 build sites at 2048x640, `rolling`.
At each site a 40-cell platform is drawn level from the local surface (so the
terrain decides how much cantilevers and what its underside meets), painted
**identically in both arms**, with only the field rule differing afterwards.

| | |
|---|---|
| sites whose platform sits over loose material | **73–78%** |
| sites where the two rules disagree about what is anchored | **78%** |
| platform cells destroyed, brush rule | 5,026 |
| platform cells destroyed, bedrock-only rule | 4,062 |
| ratio | 1.24x |
| **per-seed extra damage, median** | **0** |
| per-seed extra damage, mean / p90 / range | +54 / +196 / −126 to +426 |
| seeds where the brush arm did worse / **better** / identical | 8 / **6** / 4 |

**So the mechanism's trigger condition is near-universal and its consequence
in ordinary terrain is not.** The geometry that actually destroys a structure
— a long span from a narrow support with loose material touching partway — is
one a player builds deliberately; terrain hands it to you rarely enough that
over 216 sites the median seed shows no difference at all, and a third of
seeds come out *better* under the brush's rule.

**This entry's own first answer was a small-sample artifact, and that is worth
recording.** The first six seeds gave **1.64x** and read as a clean effect.
The next twelve gave **1.08x**. Pooled, the median is zero. `CLAUDE.md`
already says outcomes here are chaotic in the seed and a guard must read an
order statistic over N seeds — the rule was written down, quoted in this very
session, and still under-applied at n=6. **Six seeds is not a sweep on this
engine.**

#### What that means for priority

**§S2 does not go ahead of §S.** §S doubles the frame cost on every
destructive verb, reproduces every time, and needs no special geometry. §S2 is
a genuine defect with a proven mechanism whose effect on a generated world is
inside seed noise. Fix the anchor rule because it is *wrong* and cheap to make
consistent, not because players are losing structures to it — nothing here
shows that they are.

#### For the wiki, when this is fixed

In play this reads as: **building over sand, gravel or rubble with the brush
makes the thing you built collapse** — entirely, not at its outer end — where
the identical structure dug out of the hillside or generated would stand.
That is a trap rather than a mechanic, and `wiki/structural-collapse.md` says
nothing about it.

#### Why this is not simply "fix `relax_region`"

Changing it changes what the brush does, which is player-visible and is a
`wiki/structural-collapse.md` behaviour. Do not change it as a side effect of
§S's performance work — §S can and should be measured with the rule held
fixed (`SETTLE_GROUND=0` on the prototype branch does exactly that). Whoever
takes this should decide the rule on its own merits and against the brush,
with the `paint_capsule` case rendered, not deduce it from a frame timing.


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

### ~~The plant model bounds height and does not bound width~~ **FIXED**

**Resolved by path-length turgor** (`OrganismCell::path_len`): the gate now
reads hydraulic distance from the collar, stamped at creation, instead of
`collar - y`. `a_tree_eventually_stops_growing` passes in 61s where it
previously ran its whole 120,000-frame budget and failed. `plant-branch-angle`
is merged. Kept below because the measurements are the reproduction, and
because the *reason* it went unnoticed for so long is reusable.

---


Found by measurement while building branch angle and the internode
straightness budget, which sit **unmerged** on branch `plant-branch-angle`
with `Reports/branch-angle-and-the-width-bound.md` beside them.

`plant.rs`'s turgor gate is `let height = (collar - y).max(0)`. That is
purely vertical, so a cell two hundred columns sideways at collar height has
`height = 0` and full margin. **Nothing in the model bounds lateral
extent** — width is limited only by self-shading and crowding, which is
enough in a tall scene and nothing in a shallow one:

| single tree | outcome |
|---|---|
| planted with 20 rows of sky (what `a_tree_eventually_stops_growing` uses) | **never plateaus** — +180–400 wood per window at frame 295,000, 24,946 cells |
| planted with 190 rows | plateaus at frame 180,000, flat for six windows |
| `PlantScene`, 200 rows | `MatureBody` identical at 120k / 200k / 300k |

Wide branch angles did not create this; they made lateral spread efficient
enough to reach it. It matters more once M10 streaming makes worlds wide.

**The fix it argues for** is bounding turgor by *path length from the
collar* rather than by height: water potential falls with the hydraulic path,
not with altitude, so a 200-cell horizontal limb is under the same
constraint as a 200-cell trunk, and one quantity change bounds both axes
with the mechanism already in place. The cost is that path length is not
tracked per cell today, and the property that made height attractive — it
never equalises when growth stops — has to be shown to hold for path length
too (it plausibly does; that is an argument, not a measurement).

Blocks: merging `plant-branch-angle`, which otherwise measures well and
appears to fix the conifer lean (handoff §4).

---

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

---

## Landing notes — lane W, package W1 (flora sowing + species identity), 2026-08-23

Appended by the W1 session; the full account is
`Reports/world-flora-sowing-2026-08-23.md`. Nothing here is a new bug — these
are the two things a later session will otherwise re-derive or trip over.

### W1a. `creeper.ron`'s root tips still run the superseded in-tick branch path — deliberately

`creeper.ron`'s `RootTip` `Grow` carries `branch_chance: [0.05]` and **no**
`branch_priming`, which is the path `tree`/`conifer`/`shrub` all abandoned
with the comment "it cleared that twice in twelve thousand frames and fired
zero times". Creeper's roots are therefore a single unbranched strand per
tip.

**Measured, not assumed, before deciding to ship it:** creeper establishes 45
of 46 sown across an eight-world sweep and 28 of 28 in the shipped
8,192-column world — the *highest* establishment rate of the four species. A
plant eight rows tall is not root-limited, so the dead knob is not blocking
the sowing work.

Left alone because `branch_priming` sits in the root block, which the lane
split assigns to lane P, and P4 is the package that rewrites root allocation.

**For whoever lands P4:** set `branch_chance: [0.0]` and `branch_priming: [3]`
in `creeper.ron` in the same change, and measure creeper's root cell count
paired against this branch rather than against a remembered number.

### W1b. A material-counting guard cannot see a species

`the_world_arrives_with_both_moss_and_trees_in_it` counts `wood`/`seed`
cells, and every woody plant in this engine is made of `wood` — so it passed
unchanged through the entire period in which the world contained exactly one
woody species. It is not a bad test; it is a test of a different claim.

Anything asking "which species are in this world" has to resolve
`Cell::organism_id()` through `World::organism` to a `SpeciesId` and count
*organisms*. `flora_census` in `tests/worldgen.rs` and
`examples/flora_census` both do it that way, and the same trap applies to any
future guard over creature species.

### W1c. ~~`generated_terrain_is_already_at_rest` went red on `main`~~ — **SUPERSEDED by §H3, which is the better record of the same failure**

§H3 (lane P) covers both at-rest tests together, identifies the moving cells
as **material 6, water**, and carries the paired before/after numbers across
the P1 merge. That is the entry to read. What follows is this lane's
independent attribution of the same thing, kept only because it rules out a
flora cause that §H3 does not address — the two lanes found it from opposite
ends within an hour of each other.

Not this lane's, and recorded here only so the next session does not spend
its afternoon attributing it. Measured, not assumed:

- At base commit `a0fa433`, `cargo test --release` on this branch failed
  **only** `a_forced_vault_world_is_sealed_and_arrives_at_rest` (main's
  known world-scale failure) and the quarantined bug-A test.
- After merging `origin/main` at `9b54be3`, `generated_terrain_is_already_at_rest`
  also fails: *"terraced seed 3: 57 cells left their position; first:
  (82,147) water, (83,147) water, (84,147) water, …"*.
- Built `9b54be3` alone in a clean worktree and ran the same test: it fails
  with **byte-identical numbers** — same preset, same seed, same 57 cells,
  same coordinates.

So it arrived with `main`'s own commits between `a0fa433` and `9b54be3`, and
any branch that merges `main` after that inherits it.

It cannot be a flora regression, and that is worth stating because the test
sits next to the flora work: the test sets `tree_density = 0.0` and
`moss_density = 0.0` before building, and `life_scatter` returns
immediately when both are zero, so the sowing rule does not execute in any
world this test looks at. `spring_flow = 0.0` is set too, so the moving
cells are not a spring either — they are standing water in a `terraced`
world, which points at the placement or settling of pooled water rather than
at any live process.

---

## ~~Fragility~~ **FIXED** — the genome stand fingerprint went red on *anyone's* plant change (lane P, genome slot 9, 2026-08-24)

**Not a bug in the engine. A bug in a test's shape**, filed because it has
already cost one wrong diagnosis within hours of being written and will keep
costing them until the test is restructured.

`plant::tests::appending_a_genome_slot_leaves_every_existing_genome_untouched`
asserts a hardcoded FNV-1a fingerprint over a stand grown 8,000 frames. It was
built to catch a genome slot being renumbered or re-purposed, and it does. But
it hashes **the whole grown stand**, so it also fails for every legitimate
change to plant behaviour landing from any lane.

**The worked example.** The slot-9 widening branch was green on its own commit
`89e50c7`. It then merged 32 commits of `main` — WP-11's leaf-fall rate
reaching all four woody species, P3's generation loop (abscission at the
frontier, senescence, `rot_remains`, seed viability as a per-frame hazard),
W2's grassfire and its new `flame` material. The guard went red at
`0x74d3fef3d454dd11` against a constant of `0x1a52804a2df78ebc`.

The integrator read that as a bug in the widening — specifically an RNG leak in
`set_seed` — and sent a fix for code that was, on that point, already correct.
An hour went into it before the check that settles it was run.

**The check that settles it, and it is one step.** Graft the test onto the
`main` being merged, with the genome change absent, and run it there. Same
value => the constant is stale, re-take it. Different => a real perturbation.
On this incident plain `main` at `GENOTYPE_TRAITS = 9` produced
`0x74d3fef3d454dd11` — byte-identical to the widened branch — so the constant
was stale and the widening was exonerated. The assertion message now carries
this procedure inline.

**Why it is still standing.** The fingerprint is the only guard that covers
*"some consumer now reads a different slot than it used to"*, which needs a
grown phenotype to express. The three cheaper guards beside it are all
drift-immune and cover the rest: `a_genome_slots_draw_is_a_pure_function_of_its
_own_index` (the mechanical contract, no frames stepped),
`widening_the_genome_does_not_move_the_breeding_draw_sequence` (200 direct
`set_seed` calls), and `set_seed_leaves_the_callers_rng_position_alone` (the
caller's stream position). None of those can be moved by another lane.

**Re-baselining did not work, and that is the part worth keeping.** The
constant was re-taken once against `main` at `cfee870`, correctly and with the
reasoning recorded — and went stale again within the hour, when W3's grass
sowing and W4's wind geography landed. Five lanes were touching plant behaviour
in one evening. A stored whole-stand number cannot survive that, and each
re-baseline just moves the failure to whoever merges next.

**Fixed by restructuring the test, not by another re-take.**
`expressing_the_appended_genome_slot_changes_no_plant` grows the same stand
twice **inside one process** — once with slot 9's width as shipped, once at
`0.0` — and asserts the two are identical. Both arms move together under any
upstream plant change, so nothing another lane lands can reach it, and there is
no magic number for anyone to decide whether to update. `SpeciesRegistry::
set_genotype_variance` is the switch, added on the `set_genome` /
`set_creature_params` harness-setter precedent and per-`World` rather than
global, because the test binary runs tests on many threads at once.

Confirmed not vacuous — two arms equal *by construction* would pass for ever
while testing nothing — by pointing the turgor read at slot 9 and watching it
go red.

The three guards beside it were already immune and stay:
`a_genome_slots_draw_is_a_pure_function_of_its_own_index` (the mechanical
contract, no frames stepped), `widening_the_genome_does_not_move_the_breeding_
draw_sequence` (200 direct `set_seed` calls), and
`set_seed_leaves_the_callers_rng_position_alone` (the caller's stream
position). **No genome guard now asserts a stored fingerprint.**

**One more trap this incident exposed, and it is the reason a careful check
looked conclusive and was not.** The workflow fires on `push:
['claude/**']` *and* `pull_request: [main]`. A `pull_request` job checks out
the **merge commit of head into base**, not the head SHA — so reading the
constant in the file at the head, confirming it matches CI's `right`, and
concluding "CI measured a different stand on my tree" is a sound-looking
inference from a tree CI never built. The head was fine; the merge tree had
W3 and W4 in it.

Reproduced exactly rather than argued: checking out the head, merging
`origin/main` into it and running the test locally returned
`14407512503826467350` — the identical value CI reported. There is no
machine-dependence and no determinism problem here.

**So when a simulation-output golden goes red in CI, the tree to reproduce
on is `merge(head, base)`, not head.** A local run on head that passes
proves nothing about a `pull_request` job.

**The general lesson, which is why this entry stays rather than being
deleted:** a guard that hashes whole-simulation output is a guard on every
lane's work, not on yours. When the property is "X changes nothing", the
checkable form is two arms in one build — not one arm against a number from
last week.

## Landing notes — lane S, package T1 (fell as pieces), 2026-08-23

Appended by the T1 session; the full account, with every figure, is
`Reports/physical-trees-t1-implementation.md`. One new open bug, one bug
closed, and one number now printed rather than remembered.

### T1a. `load::grain_is_footing` reads *attachment* where it means *supported* — **OPEN**

A settled piece resting on loose grit is not recognised as standing on
anything, once the grit is itself resting on another settled piece.

`grain_is_footing` probes down a column of powder and, on reaching body
material, returns `cell.attached()`. Attachment means *terrain* — a landed
`ChunkBody` is deliberately unattached (`rigid::settle`'s own note: "landing
must not silently re-attach it"). So the probe reads "there is an unattached
solid under this grain, therefore the grain is on its way down", which is
right for a piece in flight and wrong for one that stopped moving. In a deep
pile of alternating piece and grit — which is exactly what a felled tree
makes — every piece above the first fails as *unsupported*.

**Measured**, `scene=fell fell=7150` at frame 8,750, after the bearing-clamp
fix in T1: **318 unsupported failures / 1,307 cells**, and `log` standing at
624 of the **1,191** cells `settle` actually delivered. Before that fix the
same run read 431 standing, so this is the residue of a bigger problem, not
the whole of it.

**Downgraded in severity by T1e below, and the correction matters.** The gap
between "delivered" and "standing" is *not* a decay curve — `log` rises to a
plateau as bodies land and then holds flat to within three cells over two
hundred frames. Most of that gap is bodies landing on top of one another and
overwriting, plus `settle_lost_cells`, not pieces being crushed after they
arrive. This entry stands as a real modelling defect in the support
predicate; it is not the reason a felled tree did not look like fallen logs.

**Ruled out by measurement**, so do not re-derive them:

- It is not the overload path. `log` leaves `max_unsupported_span` unset, so
  `capacity_within` returns `i64::MAX`; after the T1b fix below, `overloaded`
  on that scene is **0 (0 cells)**, from 275 (711 cells).
- It is not decay. `log` runs `decay_chance_damp: 0.02`; the settled count
  drifts 445 → 431 over 1,200 frames while the litter beside it goes 1,232 →
  510.
- It is not `insubstantial`. That flag reaches `player.rs` only.
- Making `log` unbreakable is not a workaround, it is a treadmill: with
  `breaks_into` removed the same run reported **42,267 unsupported failures
  / 158,230 cells**, because a failure that cannot convert is rescheduled
  forever.

**This is not an oversight — it is a named, accepted trade-off whose
"nobody has reported it" clause has just expired.** `grain_is_footing`'s own
doc says so in as many words, under *What this gives up, named*:

> a slab resting on rubble resting on a *player-built* (and therefore
> unattached) platform now reads as unsupported, because the chain out of
> the powder lands on unattached rock. Solid-on-solid is untouched — only a
> granular layer sandwiched inside player-built structure loses, **which no
> scene here builds and nobody has reported.**

A felled tree builds exactly that, every time, at scale: log on grit on log
is a granular layer sandwiched inside unattached structure. So the clause
that licensed the trade-off no longer holds, and this entry exists to say so
rather than to propose a fourth support model.

**The shape of the fix, and why T1 did not attempt it.** The obvious answer
— ask whether the body-material cell under the grain *reaches an anchor*
rather than whether it is attached — is what `chain_reaches_anchor` already
computes, with a memo that `grain_is_footing` does not have, and it closes a
loop: `rests_on_ground` → `grain_is_footing` → `chain_reaches_anchor` →
`support_parent` → `is_anchor` → `rests_on_ground`. The other obvious
answer, reading the cell's stored `aux`, is **already rejected in that
function's doc** and for a reason that still stands ("here it would be
circular, since the distance under a swallowed grain is 0 *because of this
rule*").

What the doc's own framing suggests instead: the case it is protecting
against is a grain *swallowed inside the asking piece*, and the case that is
now failing is a grain resting on a **different** piece. That is a
distinction `evaluate_within` can already make — it holds `section_cells` —
and it is data rather than shape, which is the axis `CLAUDE.md` says these
predicates keep getting wrong. Three predicates have failed here by reading
shape; this needs to be someone's deliberate fourth, with the raft cases
(`scene=lavadrop`, `scene=rockdrop`) re-run, not a fix bolted on to a
felling package.

### T1b. The structural opt-out did not hold against bearing — **CLOSED**

`load::capacity_within` returns `i64::MAX` for `max_unsupported_span ==
u16::MAX` and documents it as "this material does not participate in the
structural system at all". `evaluate_within` then clamped that to
`bearing_moment`, and `i64::MAX.min(x)` is `x` — so the opt-out held against
bending and silently did not hold against the one mode left. Latent until
`log` arrived, because `log` is the first material to want the opt-out *and*
to spend its life lying on loose rubble. Fixed by guarding the clamp on the
opt-out itself; it reaches only `log` and `nest` in the shipped set.

### T1d. `acceptance.sh`'s `lavadrop` sits close enough to its frame budget to flake, and is over it on `main` — **OPEN, not this branch's**

Noticed while gating T1 and measured rather than assumed, because a frame
budget is exactly the assertion `CLAUDE.md` says never to read against a
remembered number.

Same command (`scene=lavadrop start=2 every=300 count=4 ... repeat=2
max_frame_ms=60`), one arm at a time on an otherwise idle machine:

| | worst frame (best of 2) | spread |
|---|---|---|
| `main` at `00d1551` | **74.96 ms — over the 60 ms budget** | 74.96-77.74 |
| `claude/t1-fell-as-pieces` at `8d89b93` | 56.33 ms — under it | **56.33-66.67** |

Two separate things, and they want different remedies:

- **The base branch is over the bar.** Whatever moved it is in `main`'s own
  recent history, not in a felling package; `lavadrop` builds no plant and
  the T1 changes it can reach are a comparison that short-circuits *earlier*
  than the code it guards.
- **The case is flaky either way.** The branch's own two runs span
  56.33-66.67, straddling 60, so a green here is a coin toss on this
  machine. `repeat=2` reports the *best* of two, which hides that. CI's
  acceptance job has passed on both, so the CI runner is faster than this
  one — which is the reason a bar this close to the measurement gets
  rubber-stamped rather than caught.

**First observed contended and nearly misattributed.** The failure surfaced
on a run sharing the machine with the full test suite and `ascii`, at 66.07
ms, and it read as a regression from this branch. It is not; the same window
also reported `ascii` worst at 118.6 ms against 72.6 uncontended. Recorded
because the misreading is the point: a timing gate measured beside other
work is measuring the other work.

### T1e. "The pieces hit the ground and turn to dust" was **not** `settle`, and the measurement says so — **CLOSED**

Worth recording because the hypothesis was reasonable, was held by two
sessions, and was wrong; the cost of not checking it would have been a
rewrite of `rigid::settle`.

Watching the fall frame by frame the owner reported: *"The branches fall off
as whole pieces (good), but then hit the ground and turn to dust."* The
natural suspect is `settle` — `Reports/physical-trees-design-2026-08-23.md`
§5.5 predicts a landed piece re-rasterizing as inert material and then
running whatever powder path applies to it.

**It is not, and `log` is stable on landing.** Tracing the settled census
frame by frame after the fall (`scene=fell fell=7150`, one run, same seed):

| frame | `log` standing | pieces >= 8 cells |
|---|---|---|
| 7,175 | 459 | 9 |
| 7,190 | 644 | 15 |
| 7,210 | **713** | 18 |
| 7,250 | 712 | 18 |
| 7,300 | 711 | 18 |
| 7,400 | 710 | 18 |

`log` *rises* as bodies land and then holds flat to within three cells over
two hundred frames. Nothing is converting it. The piece count is likewise
stable at 18 holding 609 cells.

**What actually turned to dust was the foliage.** At the same frames:
`litter` **1,652** cells against `log` 710 and `deadwood` 363 — the leaf
tier outnumbered the piece tier **2.3 to 1**, it is the brightest thing on
screen, and roughly **1,570 cells of it were created in a single frame** at
the instant of severance, because `fell_severed_tissue` converted every
non-woody cell of the region before the ladder ran. The pieces fell through
that cloud and were buried in it. Rendered at 4x it is unmistakable and it
is exactly what "turn to dust" describes.

**Fixed by letting foliage ride the piece it hangs on** rather than leaving
the branch at the moment the branch does: the whole severed region now goes
to the ladder and only *woody* cells may seed a fragment, so a leafy limb
comes off with its leaves on and lets go when it lands
(`leaf.ron`'s `severs_into`). §5.3 is intact where it argues foliage must
not be *on* the ladder — a leaf still never seeds a fragment and never sizes
one. Guarded both ways by
`a_severed_limb_carries_its_own_foliage_down` and
`foliage_no_piece_reaches_still_scatters_and_never_seeds_one`, the second of
which fails without the wood-only seed rule.

**The lesson, which is `CLAUDE.md`'s own:** an image says *what* and
*where*. Two sessions read "turns to dust" as a claim about the material
that was turning to dust, and the material that was turning to dust was the
one nobody was looking at. The frame-by-frame census is what separated them,
and it took one run.

### T1f. **The felled pile is 74% powder because the tree is 56% leaves.** The piece ladder cannot fix this — **OPEN, and it is the acceptance blocker**

The number that should have been taken before any of the earlier hypotheses,
and the one that decides whether T1's bar is reachable at all.

**Method note first, because it nearly produced a fourth wrong answer.** A
census of "unattached cells in the fall box" reads **8,308 cells, 78% of them
a `Powder` kind**, which looks damning and means nothing: run the identical
census *before the cut* and it reads 8,608 cells, 51% powder, with **soil
4,384**. The box is mostly the soil bank the tree is standing on. Only the
delta is the tree. `CLAUDE.md`: sanity-check a new metric against a case you
know is fine, before trusting it about a case you don't.

**The tree, and only the tree**, `scene=fell fell=7150`:

| | before the cut | 100 frames after landing (7,300) | settled (8,750) |
|---|---|---|---|
| `wood` / `log` | 1,280 | **724** | 631 |
| `leaf` / `litter` | 1,660 | **1,634** | 466 |
| `deadwood` | — | 382 | 384 |
| tree debris total | 2,940 | **2,740** | 1,481 |
| of the log, in coherent pieces >= 8 cells | — | **640 (12 pieces)** | 522 (11 pieces) |

So at the moment a player is looking at it:

- **coherent pieces: 640 of 2,740 = 23%**
- **loose grain (`litter` + `deadwood`): 2,016 of 2,740 = 74%**
- scattered single `log` cells: 84 = 3%

**The pile is three-quarters powder, and the dominant powder is the
foliage.** `leaf` is **1,660 of the tree's 2,940 cells — 56%** — and every
one of them becomes `litter`, which is a `Powder` with a friction angle.
Even if every single wood cell came down as a coherent log, over half the
pile would still be grain.

**This is why the piece ladder cannot reach the bar.** The ladder is working:
91%+ of *woody* mass promotes, the size distribution is real, and the pieces
survive landing (T1e). The bar — "logs lying on the ground, visible at
readable zoom" — is about what dominates the picture, and what dominates the
picture is leaves.

**One thing that does change it, and it is time.** By frame 8,750 the litter
has rotted and `log` 631 finally exceeds `litter` 466. The pile becomes
log-dominant *eventually*. Nobody watches a fall for 1,500 frames.

**What would actually move it**, none of which T1 owns:

- foliage that lands as something which reads as *foliage lying there*
  rather than as grain — a dead-leaf tier that is not a `Powder`, which is a
  new material and a new settling question;
- a less leaf-heavy species, which is worldgen/genome, not felling;
- rotation, so the wood that *is* there reads as logs lying rather than
  standing (6 of 11 pieces land upright — §6.1, T2's).

### T1g. A "refixed" claim went out over a settled state that had barely moved

Recorded as a method failure, because the correction is cheap and the cost
was a wasted review round and the owner's trust.

Foliage riding its branch (T1e) transformed the **fall**: promoted share
44% -> 99%, peak plant cells in flight 1,211 -> 2,878, pieces over 256 cells
2 -> 7. It moved the **settled** state by almost nothing:

| at rest | before the ride change | after |
|---|---|---|
| `log` | 711 | 724 |
| `litter` | 1,652 | 1,634 |
| `deadwood` | ~380 | 382 |

The leaves now arrive attached to the branch and convert to `litter` on
landing instead of in mid-air. The end state is the same pile. The owner's
verdict was *"It is still very clearly dust. Did you review the images
yourself?"* and both halves are fair: the images were reviewed and read as
"still a mound" — that reading was even written on the *previous* card — but
the new card was titled as a refix anyway, on the strength of the flight
numbers, **without putting the two settled tables side by side**.

The rule this breaks is already in `CLAUDE.md` and is one line long: *look
again after the fix, for what you did not measure*. A fix measured on the
quantity it obviously improves, and shipped without re-measuring the
quantity the acceptance bar is written in, is not verified. Print the
before/after of **the bar's own quantity** or do not claim the fix.

### T1c. §1c's settle loss is now a counter

`FailureCounts::settle_lost_cells` counts every cell `rigid::settle` could
not place anywhere. It matters more than it did: before T1 a body landed on
terrain, and now a felled crown's pieces land in a large pile of the same
crown's own grit, which is where `nearest_free`'s rings come back empty.
**188 of 1,160** promoted cells on the measured cut. Printed by `filmstrip`
beside the felling census, per the T1 brief; deliberately not fixed there,
because fixing it means deciding where a cell with nowhere to go *should*
end up, which is a settling question rather than a fragmentation one.

## Landing notes — lane W, package W3 (grass sowing + the A4 divergence instrument), 2026-08-24

Successor to the W1 notes above. Full derivation in
`Reports/grass-sowing-and-divergence-2026-08-23.md`; this is what survives the
session.

### What landed

**Grass into worldgen (A1's grass half), PR #38.** `life_scatter` sows grass as
its **own layer** off `grass_density`, between the woody loop and moss — not as
a fifth row of `WOODY`, because those four weights split one budget and a fifth
entry would have taken columns from the species W1 had just landed. Measured
paired against main over sixteen seeds, all four woody species come out
**bit-identical**; grass competes with *moss*. The weight is
`1 - ramp(woody_budget, 1.0, 2.0)` on the **unclamped** woody sum — the clamped
one saturates (p10 is already 1.00), which is the measurement that chose the
rule. Guarded by `grass_is_sown_across_a_seed_sweep` and
`sown_grass_also_comes_up`, both checked against the artifact they exist to
catch by breaking them deliberately.

**The A4 two-patch divergence instrument, `examples/divergence.rs`, PR #38.**
Same founders, two patches differing in one environmental axis, scored on
root:shoot and slenderness. Two *separate worlds* at the same seed and
coordinates, so "same founders" is literal rather than approximate. Its
identical-patch control returns **exactly zero** on both metrics while the
sample it draws from spans slenderness 1.26–57.00. On moisture it found
root:shoot diverging **8 of 8 seeds** and correctly **refused** slenderness
(5 of 8, swinging −5.15 to +3.94).

### Do not re-derive these

- **The instrument exists and is axis-agnostic.** Everything downstream of the
  axis — the two-world founder construction, the exact-zero control, both
  metrics, the seed sweep, the establishment-imbalance warning, the
  axis-survival check — does not know what is being varied. **Adding an axis is
  one arm on `Axis` and nothing else.**
- **It can already answer questions nobody has asked**, and this is the bullet
  that stops it being rebuilt:
  - **Any** single-axis morphology comparison — `soil=`, `founders=`,
    `width=`, `species=`, `frames=` are all parameters already, so "does soil
    depth change root:shoot", "does crowding change slenderness", "do two
    species differ in shape at the same size" are each a run, not a build.
  - **Does a new genome locus move morphology at all?** Point it at two
    patches differing only in the locus and read the sign agreement. This is
    the measurement `plant-species-authoring.md` §1 wanted when it found
    `light_weight` and `upward_weight` inert.
  - **A determinism check, for free.** The control asserts two identically
    built worlds diverge by *exactly* zero. If it ever returns non-zero on
    `control=1`, determinism has broken — which `PLAN.md` requires and nothing
    else routinely exercises at whole-organism scale.
- **`flora_census -- where=1 focus=NAME at=X`** answers "where in the world is
  this species, and what shares the frame". Built after a review card came back
  *"I don't see a difference"* and counting showed the rendered window held 125
  grass cells against 7,853 woody. **Audit a window with `at=` before believing
  a card**; a whole-world total in `meta` cannot say whether the thing is in
  frame.

### Measurements that contradict something written

- **`assets/species/grass.ron`'s header is now wrong in one clause.** It says
  *"There is no maintenance cost anywhere in the engine yet, so a retired mat
  cannot starve; superlinear maintenance respiration is package P2's"*. **P2
  landed** (PR #40). Re-measured across it: grass plant *counts* are unchanged
  within ±1, but **standing cells fall ~20% on every seed at both ends of a
  45,000-frame run** — superlinear maintenance makes each grass plant a fifth
  smaller without changing how many stand. Whoever next edits that file should
  correct the clause; the number is a datum for lane P on what the
  re-derivation bought on a species with no leaf stage.
- **`plant_tree_species`'s doc understates what it plants** — it says the seed
  germinates into a `wood`-material `GrowingTip`, and grass germinates into
  `grassblade`. Left alone deliberately: `plant.rs` had two packages live in it.
- **The owner's stated model of grass is not what the engine does.** He expects
  patches to *"spread over time and completely fill up an area without trees"*.
  Measured on the treeless control, grass reaches its sown footprint inside
  5,000 frames and holds it — over the next 40,000 it gains 13 plants on one
  seed and 2 on another. See the open question below.

### Open — does grass fail to spread because dispersal is one cell?

**Not a bug report; a question with a reproduction.** Measured (post-P2 `main`,
treeless control, 2,048-column worlds with ~500 plantable columns):

```
cargo build --release --examples
./target/release/examples/flora_census seeds=2 w=2047 h=639 \
    treedensity=0 mossdensity=0 frames=45000
```

Seed 1: 63 plants at 5,000 frames → 76 at 45,000. Seed 2: 61 → 63. Standing
cells move under 10%. Full cover of ~500 columns needs ~250 plants, so at the
observed rate that is on the order of **700,000 frames**.

**The leading explanation** is that `plant::set_seed` places a seed into an
empty **8-neighbour of the parent cell**, so offspring land inside or against
the clump that made them and grass cannot cross a gap — the sown positions are
very nearly the final ones. Two independent supports: the code says one cell,
and the measurement says the footprint does not grow.

**It is NOT isolated, which is why this is a question.** Crowding
(`crowding_weight: 30.0`), the seed bank's 18,000-frame half-life, and soil
moisture on marginal ground could each also cap the stand. **The run that would
settle it** — and which this package did not build — is *one founder on
uniformly ideal ground, scored on how far its descendants get by 45,000
frames*. That is a scene, not a knob: `PlantScene` already takes `soil=` and
`soil_moisture`, so it is a small addition to `examples/plant_probe.rs` or a
sibling, not new machinery.

**Why it matters beyond grass:** review item **A5 (dispersal)** — per-species
seed mass, float and carry — is the mechanism that would change this, and it
now has a named consumer and a measured motivation rather than being a
speculative nicety.

### Unmerged at close, and one of it is a fix `main` needs anyway

`w3-grass-density` is **not in `main`** (`main` still carries
`grass_density: 0.35`). It holds the owner-directed density bump with both
guard bars re-derived, the fill-curve answer above — **and a latent-bug fix
that is independent of the density change**:

`examples/ascii`'s foraging scene (and the same helper in
`examples/ant_ablation.rs` ×1 and `examples/creature_space.rs` ×5) computed
"the surface" as the topmost `Solid` **or** `Powder` cell. That is the ground
right up until something stands on it — a `seed` is a `Powder`, a grown blade
is a `Solid` — so a sown ground layer makes it return the top of a *plant*. At
`grass_density` 0.50 it stamped the ant nest a row above the soil and planted
55 ants into the vegetation: **1,901 pickups and zero deliveries**, a green
suite to a panic. `main` does not trigger it at 0.35 today, but the bug is
still there and the next thing that puts vegetation on those columns hits it.
Fixed by asking for ground — skip cells carrying an `organism_id`.
