# Open bugs handoff

Rewritten at the end of the session that landed `15b2e51` … `ad1e227`.
Everything here was measured, not reasoned — where something is a guess it
says so, and where a plausible idea was measured and found wrong it is
recorded with its numbers so it is not tried twice.

Read `CLAUDE.md` first; it holds the method these bugs keep re-teaching.

---

## Open

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

### Z. A free particle drops `Cell::aux`, so a blast under-prices a corpse — **REPRODUCED AND FIXED, 2026-08-23**

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
  the two presets that put it past the world floor. So the decision inside
  this lever is a *worldgen* one first — does the desert get a table at all
  — and only then a root-reach one. Give it one and the existing terms land
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

### X. A desert with no desert plants — **DESIGN DIRECTION, 2026-08-22. Do not "fix" this by watering deserts.**

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
— LATENT, and it goes LIVE the day grass is plantable.** Both abscission
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

**Still open, and it is `render.rs`'s, not fire's.** The fire now has a body,
a plume and a char scar, and it still draws *pale*. Every burning thing
saturates the heat ramp (it tops out 400C above ambient; grass burns at
520C, a flame at 780C) and the top of that ramp, `FIRE_TINT_HIGH`, is
(255, 210, 110) — a yellow-white. A burning meadow therefore draws as
**straw**. A two-constant prototype (LOW (150,30,12) / HIGH (255,138,36))
reads as fire at a glance and is **not shipped**, because those constants
also colour lava, quench crust and warm water — three looks the owner has
already judged. The A/B is on the owner's review queue; whoever takes it
owns re-checking those three. Two attempts that made it worse are in
`Reports/dead-ends.md` under *rendering*.

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

### H3. Both worldgen at-rest tests are red on `main`, and both are water — **OPEN, inherited, 2026-08-23**

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
