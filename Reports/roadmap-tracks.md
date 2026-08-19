# Six tracks — the organised backlog

**Status:** organising document, written at the owner's request after a
review of `PLAN.md`, the four handoffs, and the code itself. It does not
decide anything; it says what is built, what is open, and which questions
are the owner's rather than an implementer's.

Scope was set by the owner: **destruction & structural, world building,
visual polish, water→steam/ice, player mechanics, explosions.** Trees,
plants and creatures are deliberately out of scope here and keep their own
documents. Water *flow* is out of scope — the owner reports those issues
solved on `master`; see §0b.

Every "built" claim below was verified against the code in this tree, not
taken from a report. Where a claim comes from a report and was not
re-verified, it says so.

---

## 0. Before any track starts

### 0a. `load-share` is unmerged work on the largest open defect

`origin/load-share` is **5 commits ahead of `master`, 35 behind**. It
contains `43a57b7` "A wall carries its roof through all of itself, not down
one line" plus a review write-up and a follow-up fix.

That is a direct attack on §2d load concentration, which
`Reports/next-session-handoff.md:856` calls "the largest open defect in
`load.rs`" and the defect the owner has reported most often. **Track 1
cannot be planned honestly until this branch is merged or abandoned** — any
load work started on `master` risks colliding with it, and it drifts further
behind every day.

**Decision needed:** merge it, or close it and keep the review.

**Its own §7 named three reasons it was not merged. One is already gone:**

| blocker | status |
|---|---|
| §2 overturns the reviewer's finding, and the author declined to self-certify | **checked, and §2 does not hold — see §0a-i** |
| timing uncertified | open — needs a quiet machine or CI |
| the main checkout's `master` was 27 commits ahead and **unpushed** | **resolved.** Every piece it named — creatures, genome, evaporation, lightning, skyline — is on `origin/master` now, which has advanced 35 commits past the point that reply was written against |

The change is **entirely absent from master** — `section_share`,
`MAX_MEMBER_WIDTH`, `ShareCounts` and `capacity_within` all return zero hits
there. Nothing was partially ported.

Two `CLAUDE.md` method lessons are also stranded on the branch, and they are
general rather than structural: *a cascade censused before it settles reads a
delay as damage* (the same two binaries measured 251 against 1,501 cells at
frame 202, and 235 against 273 at 1,500 — opposite conclusions from the same
seeds), and *a mean over events is not the size of the pieces*. Both would
help every track above.

### 0a-i. §2 was checked independently, and its numbers do not survive the fix

The reply's §2 asked for exactly this ("one command, and I have every reason
to want a particular answer"). Run on Linux at branch tip `222de5c`, both
arms, `scene=worldcrack preset=flat seed=7 dig=4 tunnel=35 depth=6`,
`start=2 every=1200 count=5`:

| `caveshallow` at frame 4802 | §2 claims | measured at tip |
|---|---|---|
| `share=0` crumbled to grit | 10 regions, 30 cells | **10 regions, 30 cells** ✓ |
| `share=0` rock destroyed | −64 | **−64** ✓ |
| `share=0` confined | 488 (4,697) | **488 (4,697)** ✓ |
| `share=0` mean region | 10.0 | **10.0** ✓ |
| `share=1` crumbled to grit | 6 regions, **16 cells** | **11 regions, 38 cells** ✗ |
| `share=1` rock destroyed | −64, "identical outcome" | **−100** ✗ |
| `share=1` confined | 5,496 (22,039) | **2,988 (13,313)** ✗ |
| `share=1` mean region | 4.1 | **4.6** ✗ |

Settling was checked first, per this file's own new lesson: `crumbled` and
`cells lost` are stable across tiles 2–4 in both arms (`awake 2/40` in both),
so those are settled numbers.

**The control reproduces on four independent numbers and the treatment on
none** — which localises the cause, because `share=0` never enters the member
logic that `f224f10` changed. Re-running `share=1` at `64ed288` (pre-fix)
returns **rock −64, confined 5,496 (22,039), mean 4.1** — §2's figures,
exactly.

**But `crumbled` cannot come from that build: `git log -S` shows the counter
was introduced *by* `f224f10`, the fix commit.** So §2's table is a mixture
that no single commit on the branch produces — confined/mean/rock from the
pre-fix code, `crumbled` from an uncommitted intermediate that had the
counter and not the fix. The branch's own review records this exact trap
biting `flows_down`: *"taken before `MAX_MEMBER_WIDTH` existed, so over a
different population of members."* It caught §2 as well, in a section written
25 minutes after the fix landed.

**What this does and does not mean.** It does not show the reviewer was right
and the change wrong; it shows §2's evidence is invalid, so the question is
reopened rather than settled either way. On the tip's own numbers the
reviewer's concern reads as *more* live, not less: the dust path fires more
(30 → 38 cells), and the "identical material outcome" the argument rests on
is −64 against −100.

**Re-running §2's table at the tip is now a ten-minute job** — the harness is
built and the commands are above.

### 0a-ii. The confined-failure counter never converges

Found while checking the above, and it is a defect in its own right. On a
world that is materially settled — `cells lost` pinned at 15, `cells
fissured` pinned at 284 — confined failures climbed **1,548 → 2,268 → 2,988
events** across tiles 2, 3 and 4 and were still rising. The structural system
re-evaluates the same buried rock indefinitely, changing nothing.

Any figure derived from that counter is therefore **a function of how long
you ran, not of the outcome** — including §2's 5,496 and the mean, which fell
5.4 → 4.9 → 4.6 purely as its denominator grew. That strengthens the reply's
argument about the mean while undermining its own numbers by the same
mechanism.

It is also the likeliest frame cost: worst frame 48.58 ms at `share=1`
against 28.42 ms at `share=0`, same machine, same run. The reply flagged the
waste ("~11x the work for an identical outcome... belongs in
`crush_in_place`") and left it. On this evidence it is the top item on the
branch, ahead of the merge.

**Caveats on these numbers.** Linux, where that session ran Windows, and
determinism here is same-build only — though the exact four-number match on
the control makes platform an unlikely explanation for the treatment
diverging. Timings are a valid *paired* comparison on one machine and are not
comparable to any absolute figure in the reports.

### 0b. Report freshness is uneven, and one file is actively misleading

`Reports/open-bugs-handoff.md` still lists whiskers, sand-into-water
displacement and the heightfield items under **`## Open`**. The owner
reports the water work is done and on `master`. The file was last touched
`2026-08-18` (`bb20167`), so this is not simple staleness — the water
section was not revised when the work landed.

Nothing in this document depends on that file. It is flagged because the
next person to read it cold will plan against bugs that no longer exist,
which is the exact failure `CLAUDE.md` wrote the handoff convention to
prevent. **Fixing it is a five-minute job and should ride along with
whatever lands first.**

Same shape, smaller: `README.md`'s Status section says character physics and
the streaming world are "not yet built" while `src/sim/player.rs` is 1,881
lines and the gnome digs tunnels; it does not mention weather, pheromones or
the brain at all.

`Reports/load-model-handoff.md` opens with **"Status: not started"** and is
superseded — `next-session-handoff.md` describes the load model as built and
working. It should say so at the top rather than in another file.

---

## 1. Destruction & structural

The most-worked area in the repo and the one with the most measured
knowledge behind it. Read `Reports/next-session-handoff.md` §2–§3 before
touching any of it; §3 is the do-not-retry list.

**Built and working:** torque vs capacity, section failure, load flow over
parallel supports, crack-driven detachment, a stress view (`N`), a
rectangle/room/line build tool (`Z`), a precise dig, one spoil model for
every digger, confined rock cracking in place. 19 acceptance cases gate in
CI via `scripts/acceptance.sh`, plus `scripts/seedsweep.sh`.

### Open, in the handoff's own order

| # | Item | Kind | Notes |
|---|---|---|---|
| 1.1 | §2a `wall=3 span=200` collapses untouched while 2 and 5 stand | bug, **shrunk** | `next-session-handoff.md` records 1,064 cells against 0 either side. **`load-share` retires that number**: 48 cells, not 1,064, and identical with sharing on and off at wall 2, 3, 5 and 8 — the worst-stressed cell in every run is a *roof* cell, taking its support horizontally. So the handoff's suspicion that load concentration caused §2a is wrong. Still non-monotonic (48 against 0), but small, and it wants **re-baselining after the merge** rather than work now. |
| 1.2 | §2d load concentration | bug | Inner face carries 1707, outer 307. See §0a — work exists on `load-share`. |
| 1.3 | Tumbling | feel | Owner wants "things tilted and fell over more as large pieces". 45–62 bodies form per collapse now, so there is finally something to tumble. **Check whether it already reads right before touching `SPIN_PER_SPEED`.** |
| 1.4 | §2b the build envelope | **decision** | Rooms past ~200 wide fail at every thickness. May be correct (15:1 span-to-depth; real masonry does not do it either). Nobody has decided if it is the envelope we want. |
| 1.5 | §2c the dig always severs a room wall | **decision** | `Tool::Room` and `App::mine` share `brush_radius`, so a cut is always exactly as thick as the wall — no ligament can ever remain. The handoff's own suggestion is the satisfying one: a doorway dug from the ground up over several clicks, making it a verb rather than a click. |
| 1.6 | C2 — mortar as a material; doorway/window cuts on the room tool | feature | Downstream of 1.5's answer. |

### Not confirmed, and worth confirming early

- **`GRANULAR_CAPACITY_DIVISOR` may be dead code** — `evaluate_within`
  early-returns on `is_anchor`, which includes `rests_on_ground`. Flagged by
  a concurrent review, never verified. `CLAUDE.md` has a standing entry
  about this exact constant being a counterweight rather than a model.

---

## 2. World building

**Much further along than `PLAN.md`'s M10 section reads.** `src/worldgen/`
is 2,432 lines across seven files with the decide/realise split the design
doc asked for: `column` decides (pure functions of `(seed, params, x)`),
`passes` realises (named passes declaring their own column margins).

**Built passes:** `stone_massif`, `bedrock_floor`, `soil_blanket`, `brows`,
`talus`, `ponds`, `soil_moisture`, `moisture_init`, `life_scatter`,
`pockets`. The water table exists and is removable by data rather than code
(`table_offset` past world height → no pools, no moisture floor; the `arid`
and `flat` presets ship that way).

**The module doc names its own gaps: caves, erosion, world age, streaming.**

### Open

| # | Item | Kind | Notes |
|---|---|---|---|
| 2.1 | **Caves** | feature | Additive density-function generation, not carving — reversed deliberately after checking Minecraft 1.18. **Coupled to Track 1**, see below. |
| 2.2 | Reserve a slice-identifier field on `ChunkCoord` | chore, **urgent** | Issue #11. Constructed in 42 places and will hit the save format. A bare `u32`, always zero, added now is mechanical; added later is a 42-site migration *plus* a save-format break. Cheap now, expensive later, and nothing blocks it. |
| 2.3 | Erosion | feature | Also the source of loose material, given "generate only `Solid`" below. |
| 2.4 | The closed water cycle | feature | Rain exists (weather). Evaporation exists. Rivers are meant to be a *consequence* of drainage plus real water, not a generated feature — which needs a real source and a real sink or a river is a puddle that formed once. |
| 2.5 | `worldgen(seed, coord, world_age)` | architecture | A chunk generated on day 400 must generate 400-day-old ecology directly, or walking into fresh territory shows a seam. Makes worldgen and succession the same function at different times. |
| 2.6 | Two `GLOBAL` passes | debt | Named in the module doc as "the honest debt" — they need the whole world's shape. Paying them off is what the coarse `(x, z)` map is for, and they block per-chunk generation. |
| 2.7 | Streaming (M10 proper) | large | Last. Needs everything above plus the persistence taxonomy. |

### The cave/structural coupling is the interesting problem here

A noise-defined cave ceiling has **no bounded thickness** against
`stone.ron`'s `max_unsupported_span: 3`. The design doc calls this
"genuinely unsolved" and the strongest surviving case for keeping a
controllable-radius worm-carve or a span-aware post-pass somewhere in the
pipeline.

Meanwhile Track 1 already knows what governs a dug cave: **roof cover**
(`next-session-handoff.md` §1a-ii, gated with guards seen to fail). So the
two tracks are solving the same problem from opposite ends — one for caves
that are generated, one for caves that are dug. **Whoever does 2.1 should
read §1a-ii first**, and the honest possibility is that generated caves
should be produced by the same span-aware mechanism that already decides
whether a dug one holds.

Also standing: **generated terrain must be at rest.** Unique to a
falling-sand engine — a 50° sand slope against a 34° repose angle slumps the
instant the chunk wakes, and the slump can propagate into chunks that do not
exist yet. Recommendation on record: generate only `Solid`.

---

## 3. Visual polish

`research/m19-visual-polish.md` did the research and the execution plan is
tiered by how much a human has to be watching. Tiers 1–2 are partly landed.

**Built:** per-cell brightness jitter driven from `Cell::shade`,
temperature-driven heat glow, five `GrainMode` variants behind `G`, the
scalar ramp used by the field overlays, a cave-depth fade ramp.

**Cut, with a measurement:** fake ambient occlusion — recorded in
`render.rs` as cut for measured cost, on top of jitter and heat glow that
"stayed only after being budgeted". Do not re-propose it without a cheaper
formulation.

### Open

| # | Item | Kind | Notes |
|---|---|---|---|
| 3.1 | **Choose a `GrainMode`** | **decision** | Five variants prototyped, GIFs generated, default still `Position` (today's behaviour). This is finished work blocked entirely on someone looking at it. Worth knowing: a settled pool changes 431 cells per step with **zero occupancy changes** — its interior genuinely does not move, so nothing can animate it except `Animated`, which is the only variant that costs the dirty-rect render skip. |
| 3.2 | Palette overhaul — ramps in HSL, one per material family | feature | The rule that unifies a palette: shift hue *while* shifting value (darks toward blue/purple, lights toward yellow), not brightness scaling. Distinguish adjacent materials by **hue**, not value — value-only differences vanish at small pixel sizes. Reserve peak saturation *and* peak lightness together for emissive materials only; that combination is what reads as "glowing" with no lighting engine at all. Resurrect 64 / Endesga 32 are directly adoptable. |
| 3.3 | Ordered (Bayer) dithering | feature | Not present in `render.rs`. Kills flat-colour banding. |
| 3.4 | Wire the light channel | feature | `add_light` has one caller, a test. Lighting it resurrects two already-implemented, currently inert mechanisms and retires a documented M16 simplification. Flood-fill/BFS at the field grid's existing 1/8 resolution, upsampled and multiplied over the frame the way Noita's fog layer works. |
| 3.5 | GPU bloom via `Pixels::render_with` | large | Tier 4, stays deferred for the reason M6 always was — needs a human watching it render. Confirmed a first-class extension point, not a hack; `pixels` ships a working `custom-shader` example. |

**Standing constraint for this whole track:** frame cost is a hard
constraint, not a tiebreaker, and the thing most easily destroyed here is
the dirty-rect render skip — which does its work precisely on the settled
worlds where an animated effect looks most tempting. State what an effect
costs when proposing it.

---

## 4. Water → steam and ice

**These are two very different jobs and should not be planned as one.**

The phase-change machinery is **built and proven**: `fire::try_phase_change`
reads `melting_point`/`melts_into` and `boiling_point`/`boils_into` off the
material, runs before movement dispatch, and returns whether identity
changed. `snow.ron` uses it today — snow's melting point sits *below*
ambient deliberately, so a drift thaws when the cold front that brought it
moves on.

But the mechanism is **one-directional**. `snow.ron`'s own comment states
it: *"That gets thaw for free from the existing upward phase change and
needs no freezing code at all — water does not become snow, weather simply
makes more of it while it is cold."*

### 4a. Steam — mostly data, small

`water.ron` has **no `boiling_point` and no `boils_into`**, and there is no
`steam` material (`smoke.ron` exists; steam does not). So:

1. Author `assets/materials/steam.ron` as a `Gas` — it can borrow smoke's
   rules, and gases already drift on `PREVAILING_DRIFT`.
2. Add `boiling_point: 100.0, boils_into: "steam"` to `water.ron`.
3. Decide what steam does at the top of its life — condense, or simply
   disperse like smoke.

Steps 1–2 are data and should produce visible behaviour immediately. **Note
the `include_str!` gotcha**: materials are compiled into the binary, so a
headless harness will produce bit-identical runs until it is rebuilt.

**This closes a known explosion defect for free.** `explosion-mechanics-
diagnosis.md`'s caveat list: *"Heated water draws with the fire tint, since
`render.rs` keys the glow off cell temperature regardless of material. It
reads as a bright rim around an underwater cavity, which is not wrong
exactly, but **steam would be the honest answer**."* An underwater blast
that flashes water to steam is the same feature as Track 6's missing
feedback.

### 4b. Ice — needs a mechanism that does not exist, larger

Freezing is a **downward** transition and the engine has none. Condensation
(steam → water) is the same missing mechanism, which means one piece of work
serves both and the design should cover both at once.

Open questions before any code:

- **Where does the rule live?** `try_phase_change` is the obvious home, but
  it currently only compares upward against thresholds. A symmetric
  `freezing_point`/`freezes_into` is the small version.
- **What does ice do to the liquid model?** Water carries continuous fill in
  `aux`; a `Solid` ice cell does not. Freezing a partially-filled cell has
  to decide what happens to the remainder, and `CLAUDE.md` is explicit that
  a partly-drained liquid must be written `with_aux(remaining)` and a fully
  drained one as `Cell::EMPTY`, never `with_aux(0)`. **Getting this
  backwards manufactures water out of nothing.**
- **Does ice bear load?** If it is `Solid` it enters the structural model,
  gets a `max_unsupported_span`, and an ice sheet over a pond becomes a
  roof. That is a feature, but it is Track 1's model taking on a new
  material.
- **Does it float?** Real ice does; density-driven displacement would sink
  it unless its density is set below water's.

**Decision needed:** is ice a weather/seasonal feature (ponds freeze when a
cold front sits over them, thaw after) or a player material? The first
reuses weather, which is already a pure function of `(seed, frame)`; the
second needs none of it.

---

## 5. Player mechanics

`src/sim/player.rs` is 1,881 lines and well past the plan. Its module doc
says the gnome "(from phase 2 onward) digs, plants and throws", so all five
planned phases exist in some form. `Tuning`, `SpoilMode`, `MovementFeel`,
`WaterFeel` and `PlayerInput` are all real, with live tuning under `O`.

`next-session-handoff.md` §1d records an ownership change worth carrying:
**the gnome and creatures are now structural work**, because the tunnel
envelope, the spoil model and whether a dug cave holds are all one problem.

### Open

| # | Item | Kind | Notes |
|---|---|---|---|
| 5.1 | Doorways as a multi-click verb | **decision**, feel | Same item as 1.5, from the player's side. The handoff calls the multi-click version "the *satisfying* answer". |
| 5.2 | Judge the feel knobs live | feel | `MovementFeel` and `WaterFeel` exist as tunables; nothing records that anyone has swept them on real terrain. This is exactly the category playtest reports have overturned repeatedly. |
| 5.3 | Burial and swimming | verify | `WaterFeel` exists; M9's own verify criterion is "can be buried by a sand dump and dig out; swims in water; stands on a tumbling rigid body". The third depends on Track 1's tumbling (1.3). |
| 5.4 | Explosives in hand | feature | Phase 5. Depends on Track 6 being worth throwing. |

**No audio.** `grep` for `rodio`/`cpal`/any audio system returns nothing.
This is not on any track and belongs to all of them — see §7.

---

## 6. Explosions

**Four rounds of work already exist** in
`Reports/explosion-mechanics-diagnosis.md`, and the remaining work is
specific rather than broad. Read that document before proposing anything;
its round-1 findings list is a good map of what "feels wrong" decomposes
into.

**Built:** `Tuning` + `Blast` (a cavity front expanding one stage per frame)
+ `Blasts`, with the same two-drivers-one-rule shape as `update`/`parallel`.
Staged duration, `scorch` writing cell temperature and rolling flammability,
`debris_fraction`, `BURN_DURATION_JITTER`, `smoke_fraction`, blast radius
decoupled from brush, and `particle::Particle::pierce` so debris can move
*through* material. Measured: evacuation up across every cover depth (170 →
712 at cover 60), peak in-flight particles 810 against 7,213 for the naive
prototype, worst frame 3.4–4.5 ms.

### Open

| # | Item | Kind | Notes |
|---|---|---|---|
| 6.1 | **Every explosion is identical** | feel | Round-1 finding #7, and the one item from that list with no entry in the round-3 "built as" table. Variation is what stops a repeated verb going dead in the hand. |
| 6.2 | Steam instead of a fire-tinted rim underwater | feel | Free once 4a lands. See §4a. |
| 6.3 | Field pressure still moves no solids or powders | architecture | Round 4 gave the channel *some* consumers — gases drift, particles feel `WIND_DRAG` — but the shockwave still propagates and reflects across the world without moving terrain. |
| 6.4 | Real field-level wind | blocked | **The prerequisite is making the field solver settle with a steady forcing term present** — not re-attempting the nudge. Measured and reverted: a small `vx` on every unblocked field cell took settled-field cost from **0.0002 ms to 3.55 ms permanently, on every scene**, and failed six field tests, because a uniform velocity in a bounded world hits walls → divergence → pressure → more velocity, so `is_converged` never returns true. `PREVAILING_DRIFT` shipped instead and is honestly an approximation. |
| 6.5 | The crater-retention metric understates itself | measurement | It counts materially-empty cells and `smoke_fraction` backfills the crater, so unrisen smoke reads as "collapsed". **Trust the evacuation column.** Third time in this investigation a metric changed meaning under a mechanism change. |

---

## 7. Cross-cutting, and not on any track

### 7a. Audio — the largest single gap against the project's own core value

The engine has **no audio at all**. `CLAUDE.md`'s ethos section states:
*"if a destructive event produces no debris, no impulse and **no sound of
consequence**, it is not finished regardless of what the simulation
believes."* By that criterion every destructive event in the engine is
unfinished, and destruction, digging, explosions and weather are four of the
six tracks above.

`Reports/weather-handoff.md` already hit this wall and deferred: *"Lightning
flashes and forks; the world is silent. The engine has no audio at all, so
this is a larger question than it sounds and should probably wait for a
decision about sound generally."*

**Decision needed:** is sound in scope at all? It is a new dependency, a new
subsystem, and a determinism question (audio must not feed back into the
sim). Nothing above blocks on it, and it improves five of the six tracks.

### 7b. `filmstrip` never renders inside its timed loop

Recorded as a known defect not yet confirmed. If true, **every worst-frame
number in this repo's history excludes drawing** — and the owner found a
render regression the harness structurally could not see. That matters most
to Track 3, which is entirely about adding render cost.

**Confirm or kill this before Track 3 starts**, because every "what does
this effect cost" answer that track produces depends on the harness being
able to see rendering at all.

### 7c. F3 — replay a playtest report from a world dump

Called "still the biggest gap in the loop": every report has had to be
reconstructed into a scene by hand, and **at least two reconstructions have
been wrong**. Given that playtest reports have overturned three separate
models here, this pays for itself faster than most features — and it shares
its machinery with the save format that Track 2's streaming needs anyway.

---

## 8. Suggested order — and what it is based on

Not a commitment, and each item should be re-judged rather than inheriting
its justification here.

1. **Resolve `load-share`** (§0a). Nothing in Track 1 is safe to plan
   around it, and it addresses the defect the owner reports most.
2. **`ChunkCoord` slice field** (2.2). The only item on this list that gets
   *more expensive* by waiting, and it is mechanical.
3. **Steam** (4a). Cheapest satisfying win on the board: two data edits and
   a material file, closes an explosion defect for free, and needs no new
   mechanism.
4. **Re-baseline §2a** (1.1) — *after* the merge, not before. The 1,064-cell
   figure it was ranked on does not survive `load-share`; what is left is 48
   cells and worth re-measuring before anyone works it.
5. **The three decisions** — grain mode (3.1), build envelope (1.4),
   doorways (1.5/5.1). All three are finished or near-finished work blocked
   on a judgement, and all three are the owner's to make.
6. **Confirm 7b**, then start Track 3 tier 1 (palette, dithering).
7. Everything else by appetite.

**Two things this order deliberately front-loads:** work that gets more
expensive if deferred, and work that is already done but blocked on someone
looking at it. Both are cheaper than anything genuinely new.
