# Explosions in stone — three-lens review

**Status:** review with measurements; recommendations at the end. Written to
be picked up cold by an implementation session (the owner has asked for the
prototype and follow-on work to be done by a cheaper model — §7 is written
for that model specifically, and assumes less context than the rest).

**The assignment, in the owner's framing:** what really happens when you
detonate a charge in solid rock, in caves; what of that is realistic inside
this engine (engine changes allowed); what is actually fun — for mining and
for blowing things up for its own sake; and the best intersection of the
three. Everything below is bound to one standing constraint, stated by the
owner at plan review: **it must work in a 2D side-scrolling cellular world.**

**Method:** per `CLAUDE.md` — look first. §1 is rendered scenes read against
printed counters, at shipped defaults, before any analysis. A new
`filmstrip` scene (`cavern`) was added for it, because nothing could
previously stage the one situation every mining blast is actually in: a
charge next to a void inside attached rock.

---

## 1. What a blast in stone actually does today, measured

All at shipped defaults (`radius 22` keyed, harness runs `r=20,
strength=180`), `cargo run --release --example filmstrip -- scene=...
explode=x,y,20,180,60`. Same-session `ascii` baseline: settled scenes
0.000 ms, blast scenes worst ~14-16 ms on this (contended) machine.

### 1a. Buried in solid stone: a bruise, not a cave

`scene=boom_stone explode=256,220` (40 cells of cover):

| quantity | value |
|---|---|
| cells dug at peak (frame 69) | ~1,350 |
| chunk bodies in flight, peak | 20 (1,192 cells) |
| net cells lost by frame 211 | 286 (rock −357, rubble +71) |
| surviving void | **57 cells** |
| cells fissured by the aftermath | 47 |
| debris particles at any sampled frame | **0** |

The staged cavity opens, `fracture_shell` cracks the wall into real tumbling
pieces — and every piece lands back in the hole it came from, because inside
a solid mass there is nowhere else. The end state, by eye: a perfectly
circular, orange-rimmed disc of packed rubble with a ~57-cell irregular void
at its centre. A bruise. The 47 fissured cells (confined crush failures) are
nearly invisible at play zoom. Nothing about the outcome depends on
direction: up, down and sideways are identical.

### 1b. Ten cells under the face: the one case that already reads well

`explode=256,190`: a real plume of dark ejecta into the sky, a crater that
keeps a 141-cell void, scorched rim. Still refills most of itself (net loss
213 of ~1,300 dug), and the rim is still a compass-perfect circle — but this
is what the mechanic was tuned on, and it shows.

### 1c. The same charge in sand, same 40-cell depth

`scene=sandbed explode=256,160`: **991 cells evacuated against stone's
286.** The blast itself has no material term at all — `clear_annulus` clears
stone and sand at the same radius, and the entire measured difference is
what happens to the material *afterwards* (sand avalanches and stays out;
rock pieces re-land inside). Stone is only "harder" today by accident of
settling dynamics.

### 1d. A cave wall: the overburden chimneys, and the world never settles

`scene=cavern explode=186,240` — charge sitting in the wall beside a 120×56
cave under an 82-cell roof:

| quantity | value |
|---|---|
| chunk bodies one frame after detonation | **189 (5,699 cells)** |
| cave volume | 5,269 → **3,598** (−32%) |
| net cells lost | 3,238 (rock −3,948, rubble +710) |
| worst frame | **264 ms at frame 201** — deterministic, reproduced exactly |
| pending structural sites | 6.3k (f61) → 14.9k (f400) → **17.9k (f1200), still climbing** |
| chunks awake at f1200 | 10-11 of 40, indefinitely; new bodies still spawning |

The blast undercuts the rock above it; the load model finds the whole
~40-wide, ~90-tall column to the surface unsupported; it detaches in one
frame, shatters into 189 bodies, and pours down — into the cave. **Blasting
a cave wall makes the cave smaller.** The aftermath then never goes quiet:
twenty seconds later the scheduler still holds three times the pending sites
it had one frame after the blast, and the 264 ms spike lands well after
everything is visually still.

### 1e. A cave roof: immortal

`explode=256,200` — charge 12 cells above a 120-cell-wide ceiling: cave
volume 5,269 → 5,305. **Nothing falls.** The crater's own rubble hangs as a
plug above the intact shell; no spall, no rockfall, no delayed collapse.

### 1f. The inversion, stated once

1d and 1e together are real blasting **mirrored**. In rock mechanics the
roof over a void is the classic failure — the reflected wave returns as
tension, rock is ~10× weaker in tension, gravity points into the void — while
laterally-confined overburden holds by shear. Here the roof cannot be
brought down by a blast that all but touches it, and the side-supported
column above a wall shot collapses to the surface wholesale. A player who
places a charge *well* (under a roof, to drop it) is punished with nothing;
a player who places it badly (into a wall) levels the neighbourhood and
fills their own excavation.

---

## 2. Lens one: what really happens when a charge fires inside rock

Stated in this engine's own 2D cell vocabulary, because that is the form the
mechanisms have to take. Confined blasting has a zonal structure:

1. **Crush pocket** (~1-3 charge radii): shock exceeds compressive strength;
   rock is pulverized in place. Small.
2. **Radial fracture halo** (~3-15 radii): the diverging wave opens radial
   cracks in hoop tension, and then the gas *wedges* them wider and longer.
   **The halo is the main product of a confined shot** — removed material is
   a small fraction of cracked material, which is why real mining is a
   *sequence* of shots, each exploiting the last one's damage.
3. **Spall at free faces**: the compressive wave reflects off a rock/air
   boundary as tension, and rock is ~10x weaker in tension, so slabs peel
   off *into* the void.
4. **Confinement dominates yield.** No nearby free face: pocket + halo,
   nothing thrown. Face nearby: a graded muck pile thrown toward and along
   it. This is why miners drill toward a free face, always.
5. **Over a cave, the roof is the classic failure** — tension, gravity into
   the void, reflected-wave spall all conspire — while laterally-confined
   overburden holds by shear on its flanks.

Set §1 against this and the engine's blast is not "approximately right,
needs tuning" — it is the *inverse* on every point: no halo (fissures never
written by the blast at all), yield independent of confinement and of
material, nothing directional, the roof immortal and the overburden
chimneying.

## 3. Lens three: what makes blasting fun, and where it agrees with lens one

(Lens two — the engine — is woven through §4 and §5, where each mechanism is
named against the code that already exists.)

From the prior art that works (Noita's material contrast and never-smooth
craters; Minecraft TNT's place-retreat-repeat loop; Terraria's bombs as
mining accelerant; Deep Rock's craters as *usable space*), and this
project's own ethos (`design-philosophy.md` §0a):

- **Anticipation → impact → aftershock.** Delayed rockfall and settling
  multiply perceived power at zero extra yield. A collapse that arrives one
  frame after the blast reads as a glitch; the same collapse over a second
  reads as consequence.
- **Placement skill.** A charge placed well (at a face, under a roof) must
  visibly outperform a lazy one. Today placement changes nothing except by
  accident of depth (§1a vs §1b), and the two cave placements *punish* good
  play (§1d/1e).
- **Progress must bank between blasts.** A mining loop needs the state left
  by blast N to make blast N+1 measurably cheaper, and the player needs to
  *see* that state. The crack halo is exactly this — which is the striking
  thing about this review: **the main product of real confined blasting and
  the missing feedback mechanic of the mining loop are the same object.**
  The three lenses do not trade against each other here; they converge.
- **Craters are spaces.** In a side-view mining game the hole is somewhere
  the player will stand. §1d's outcome — the blast fills the room you were
  enlarging — is the exact inverse of the Deep Rock lesson.
- **Graded beats binary, and every event owes feedback** (§0a, verbatim).
  A buried charge that produces a visible crack star has answered the
  player; one that produces an invisible 47-cell fissure count has not.

## 4. The intersection, and what to build

Three designer passes were run over §1-§3 with deliberately different
priors (physics-first, feel-first, engine-conservative), then their
code-level claims adversarially verified against source. They converged —
independently — on the same core set. Ranked:

### R1 — The blast scores cracks: the radial halo (consensus #1 of all three passes)

One measured absence explains most of §1a: **blasts never call
`rigid::score_cracks` — strikes and cuts do.** The engine already owns a
site-keyed, accumulation-aware fissure scorer whose every downstream
consequence is already wired: cracks cut capacity (`load::uncracked_faces`),
sever support edges, strip the attachment bonus (`detach_around_crack`),
schedule the checks that produce delayed failures, and `CRACK_TIP_BONUS`
makes a repeat shot at the same spot drive the *same* fissures deeper.

Mechanism: on the blast's final stage — after `clear_annulus`, before its
debris re-lands (rays die on non-body material, so a rubble-filled crater
would eat them) — call `score_cracks` with `from = radius`, so rays start at
the crater wall and run `radius × blast_crack_reach` into the rock.
Tunables `blast_crack_rays`, `blast_crack_reach` on `explosion::Tuning`
(→ `assets/explosion.ron`, live panel, `#[serde(default)]` keeps old files
loading; defaults of 0 reproduce today exactly).

Cost: ~16 rays × ~40 cells of writes, once, on the trigger path. Zero
settled-world cost. Buys: the halo (lens 1's main product, lens 3's
progress feedback), a raggedly-broken rim instead of a compass circle, and
the aftershock seeding.

### R2 — Confinement decides yield: buried charges crush, face charges bite

At trigger time, probe ~16 rays from the epicentre for distance-to-air
(the same bounded march `burial_depth` already runs), stored on `Blast`:

- **Contained sectors** (face beyond `containment_floor × radius`): don't
  clear outside the small crush core — leave the rock standing for R1's
  cracks to ruin. A fully buried charge becomes: muffled flash, small
  pocket, big crack star. This is the real physics of zone 4, it deletes
  §1a's self-refilling bruise honestly, and it is *cheaper than today* —
  no 1,350-cell dig, no 20 bodies tumbling inside a sealed cavity.
- **Open sectors**: clearing radius biased modestly toward the face
  (`face_bias`), and `fracture_shell`'s chunks keyed to those sectors — the
  muck leaves through the mouth into air the player can see, instead of
  re-landing in the hole (fixes §1a/§1b refill and §1's standing
  zero-debris-in-flight).
- **Material term**: each ray's advance divided by a new
  `blast_resistance: f32` on `MaterialDef` (data, per
  `design-philosophy.md` §2a; sand well under stone; default preserves
  today). Fixes §1c: stone finally clears smaller than sand *by mechanism*.

The per-cell test in `clear_annulus` is one sector-bucket lookup computed
from (dx, dy) — no trig needed at the loop.

### R3 — Pace the fracture, then converge the field (the §1d killers)

Two small diffs in the failure path, both fixing measured pathologies whose
causes were located at the code level (§6):

- **(a)** When an `Overloaded`/`Unsupported` region exceeds
  `FRACTURE_CELLS_PER_TICK` (~1,000), fracture only the BFS-nearest slice
  from `failure.at` and reschedule the remainder — it re-fails on later
  ticks, so the column still comes down, in visible stages. This is
  explicitly *not* the forbidden size-cap shape: it bounds work per tick,
  never whether breakage happens, and it is the "per-frame cap on
  fractures" `fracture-mechanics-design.md` §3.4 required and never got.
  Kills the deterministic 264 ms frame (189 bodies in one call).
- **(b)** After any mass failure, run the existing
  `structural::relax_region` over the region's bounding box — precisely
  what the paint path already does, for the stated reason ("one converged
  pass … rather than letting a reactive wavefront climb through it a cell
  per five frames"). This is the never-settling aftermath's main fix:
  anchor-less pockets resolve in one pass instead of counting to infinity
  at a cell per five frames while re-scheduling their neighbourhoods
  forever.

### R4 — Powder weighs something: the roof-killer

§1e's roof is immortal for stacked reasons (§6), but the decisive one is
that **a ~1,300-cell rubble plug contributes zero load to the stone shell
under it** — the load walk accumulates mass only over body-material cells.
The engine already charges *tree branches* for powder piled on them
(`structural::supported_load`); stone never got the same term. Mechanism: in
the load walk, a cell with powder directly above adds the contiguous powder
column (capped, ~12) to its mass at its own x. With R1's cracks having cut
the shell's capacity, the plug then overloads it and pours through — the
roof-drop verb works, as the classic two-beat: blast … pause … rockfall.

**This is a load-model change over procedural content, and it carries the
full seed-sweep obligation** (`CLAUDE.md`: two prior load-model changes
shipped green on every acceptance scene and were badly wrong; build the
sweep first, gate an order statistic). It is therefore *not* in the first
prototype — it is the first follow-up, with the sweep as its opening move.

### R5 — Legibility riders

- A per-blast report line — `excavated / crushed / fissured / thrown-bodies
  / thrown-particles` — printed by `filmstrip` and the app's debug HUD.
  Lands *with* R1, not after: every acceptance case below quotes it, and
  "did it fire at all" needs a counter, not a picture.
- Crack-tint contrast pass in `render.rs`, judged on a contact sheet: §1a's
  47 fissures were nearly invisible, and the halo is the mining loop's
  progress bar — it has to read at play zoom.
- Cracked rock clears cheaper on the *next* blast (admit an outer annulus
  band only where `cell.cracked()`): closes the sequencing loop
  R1 opens. Small; second session.

### Deferred, deliberately, and said out loud

- **Explicit spall** (reflected-tension slab peeling as its own mechanism):
  R1+R4 produce the same outcome — roof over void fails easily — through
  machinery that already exists. If playtest still wants the sharp
  peel-toward-the-void look, the physics-first pass left a worked design
  (face-region flood + `fracture_with_impulse` toward the charge).
- **The chimney's total size** (§1d takes ~5,700 cells because the parent
  forest routes every cell above the void through the roof section — no
  shear on the flanks, and `arch_span`'s 8-cell cover probe cannot reach an
  82-cell roof). R3 makes the collapse staged and survivable; whether that
  much *should* fall is a load-model calibration with the same seed-sweep
  obligation as R4. Real block-caving does chimney — over hours, not
  frames — so pacing may simply be enough. Decide after playing R1-R3.
- **A fuse / thrown-charge verb** (anticipation beat): pure app-verb work,
  owner's call on feel; nothing below depends on it.
- **Debris punching through solid rock**: correctly impossible today
  (`pierce` passes loose material only) and correctly left alone — R2 gives
  debris real air to fly through instead.

---

## 5. What adversarial verification changed

Every code-level claim the recommendations rest on was checked against
source by a reviewer instructed to refute. The architecture survived; three
load-bearing details did not, and they reshape the spec in §7:

- **Crack rays must start beyond the fractured shell band.**
  `score_cracks` rays `break` on the first non-Solid cell
  (`rigid::is_body_material`, rigid.rs:771), and by the blast's last stage
  the crater is empty *and* `fracture_shell` has removed the annulus out to
  `radius + BLAST_SHELL_REACH`. `from = radius` dies on its first cell.
  Correct: call at **trigger time** (rock still intact everywhere, no
  re-landed debris to eat rays) with `from = radius + BLAST_SHELL_REACH + 1`
  — rays then start in what will remain standing rock and never interact
  with the crater at all.
- **Sector gating must gate `fracture_shell` too, not just `clear_annulus`.**
  Otherwise a "contained" sector still gets its rim unattached and fractured
  — rock that never lost a cell loses its 12× attachment bonus anyway, which
  quietly re-manufactures §1d.
- **Cracks alone cannot drop the roof — verified arithmetically.** With the
  real constants (`base = span²/2 = 128`, `attached_span_bonus = 12`,
  `uncracked_faces` floor ¼), a breach-spanning shell decomposes into
  independent single-cell-deep row chains (load.rs:1296's own comment), so
  torque does not grow with shell thickness while capacity grows with
  thickness². A 40-cell breach's abutment row carries torque ≈ 210 (≈ 820
  fully cantilevered) against capacity 32·t² even fully cracked and
  detached: t=5 is marginal, t≥6 holds by 2-10×. **Only the powder
  surcharge (R4) closes that gap** — a 600-cell plug adds ≈ 3,000 torque.
  R4 stays the roof-killer; R1+R2 make the roof *verb* work at blast time
  (the breach opens downward and the muck falls into the cave instead of
  plugging), R4 makes standing plugs matter afterwards.
- **`relax_region` is not safe to bolt onto the failure path as-is.** It
  seeds `is_resting_on_ground` at distance 0 — the eager powder-rooting
  `structural::tick` was deliberately demoted away from (structural.rs:189)
  — so running it over a fresh rubble field re-manufactures the
  load-sink/counterweight dynamic that took two annulled bugs to untangle.
  It is also unbudgeted. R3b therefore needs the tick's last-resort ground
  semantics ported into (or parameterized on) `relax_region` first; R3a
  (fracture pacing) is independent and carries no such caveat.
- **The chimney's mechanism, confirmed:** one `Overloaded` check's region is
  the union of supported subtrees over the failing section — including
  attached bulk (harvested even though it would never be judged itself) —
  and for `Unsupported`, `detached_piece` runs to `MAX_REGION_CELLS =
  20,000` without consuming the frame budget in its own loop. Nothing caps
  what one tick hands to `fracture_failing_region`; `MAX_SUBTREE_CELLS`'s
  own doc already *promises* staging ("a piece bigger than this simply
  comes down in stages") that the handoff does not deliver.
- **The livelock's three engines, confirmed:** a massif-wide reactive
  re-relaxation at one cell per 5 frames; count-to-infinity pockets that
  climb forever after a collapse (nothing on the failure path runs the
  converged pass the paint path runs for exactly this reason,
  world.rs:2098); and everything that fell being permanently
  structurally-interesting (unattached ⇒ `is_structurally_interesting`
  unconditionally true) under `MAX_LOAD_CELLS_PER_FRAME` exhaustion →
  `Deferred` → requeue every 5 frames. Dispatch order `(next_frame, x, y)`
  means a frozen structure at low x can starve every check behind it —
  the code says so itself (structural.rs:295).

## 6. The three pathologies — disposition

| Pathology | Root cause (verified) | Disposition |
|---|---|---|
| §1d chimney: 5,700 cells, 189 bodies, one frame, 264 ms | Region = whole-column subtree union through attached bulk; no per-tick fracture cap; no shear/arch relief reaches an 82-cell roof (`MAX_COVER_PROBE = 8`, and `arch_span` clamps the arm, not the mass) | **R3a fixes the spike and stages the fall.** R2 shrinks the wound that provokes it. Whether ~5,700 cells *should* eventually fall is a load-model calibration (flank shear / Terzaghi relief) — **deferred**, behind the seed-sweep obligation, with real block-caving cited as the reason pacing alone may be enough. |
| §1d livelock: sites 6.3k → 17.9k at f1200, 10-11 chunks awake forever | The three engines in §5 | **R3b fixes engines 1-2** (converged relax after mass failure), once the eager-ground caveat is addressed; R3a shrinks engine 3's population. Gate: pending-sites at f1200 < 500 and falling, awake chunks back to ~1-2. |
| §1e immortal roof | Blasts score no cracks; powder mass invisible to the load walk (a tree branch is charged for powder piled on it — `structural::supported_load` — stone never got the term); `DETACH_DEPTH = 3` leaves the shell attached and uninteresting; shell rows hold on own mass at t≥6 (§5 arithmetic) | **R1+R2 make the roof-blast verb work at detonation** (breach opens toward the cave, muck falls in, cracks seed delayed failures). **R4 is the standing fix** — powder surcharge mass, behind the seed sweep, with the §7 traps (re-check hole, kern-test interaction, unit conservation) recorded. |

## 7. Implementation handoff — the prototype, written for a cold start

The owner has directed that implementation be done by a cheaper model.
This section is the complete spec for the first prototype (**R1 + R2 + the
report line**), plus R3a as a separate follow-on commit. Read
`CLAUDE.md` in full first; the traps at the end of this section are not
optional reading — every one is a measured, in-repo failure mode.

### 7a. Scope of the prototype

One behavioural change, stated as the player will see it: **a blast now
reads its surroundings.** Fully buried in stone it crushes a small pocket
and drives a visible star of fissures into the rock (cheaper than today —
nothing thrown inside a sealed cavity); with a free face nearby it bites
toward the face and throws its muck out through the mouth; sand still
behaves as today; stone near a face yields less than sand for the same
charge. Every blast prints what it did.

### 7b. Changes, file by file

**`src/sim/material.rs` + `assets/materials/*.ron`** — add
`blast_resistance: f32` to `MaterialDef`, `#[serde(default =` 1.0`)]`.
Set sand 0.35, soil 0.5, gravel 0.4, rubble 0.4, snow 0.2, stone 1.0.
(Data per `design-philosophy.md` §2a. Remember: `.ron` is
`include_str!`-compiled — a sweep that edits it must rebuild between
points.)

**`src/sim/explosion.rs`** —
1. `Tuning` gains: `crack_rays: u32` (12), `crack_reach: f32` (1.5),
   `containment_floor: f32` (1.4), `confined_cavity_fraction: f32` (0.35).
   `#[serde(default)]` already covers old files.
2. New `probe_confinement(world, cx, cy, radius, tuning) -> [SectorReach; 16]`
   run **once at trigger** (both constructors — `Blasts::trigger_with` AND
   the synchronous `trigger_tuned`; `Blast` is built in two places). Sixteen
   fixed directions, 22.5° apart. March each ray from the epicentre,
   accumulating `blast_resistance` per cell (EMPTY and gases cost 0 and mark
   the ray **vented**; stop there). A sector whose resistance-weighted
   cost-to-air ≤ `containment_floor × radius` is **open**: effective clear
   radius = `radius`. Otherwise **contained**: effective clear radius =
   `confined_cavity_fraction × radius`. Store on `Blast` (it is `Copy`; a
   `[u8; 16]` of effective radii is enough). Directions are fixed constants
   — no `world.rng` draws (determinism: an extra draw shifts every later
   roll in the frame), no per-stage recomputation (sector membership must be
   stable across stages).
3. `clear_annulus`: per cell, sector index from `(dx, dy)` (integer octant
   test, no atan2 in the loop — a 16-way branch on `dy/dx` signs and
   `|dx|` vs `|dy|` vs `2|dx|` comparisons, or a small precomputed method);
   admit the cell only if `dist2 ≤ sector_reach²`. The vaporize core is
   inside every sector's reach and is unchanged.
4. The `fracture_shell` call: skip contained sectors — pass the sector
   reaches (or a `&Blast`) into `fracture_shell` and apply the same test in
   its annulus loop. **This is required, not optional** (§5): without it,
   contained rock is unbraced and fractured without having lost a cell.
5. `scorch` stays ungated (a buried flash glowing through rock reads
   correctly and ignites nothing new), as do the one-shot pressure/heat
   writes.
6. **The crack halo**: at trigger, immediately after the probe, if any
   probe ray crossed Solid rock: call
   `rigid::score_cracks(world, cx, cy, from, length, rays)` with
   `from = radius + BLAST_SHELL_REACH + 1` (import or re-derive the
   constant; rays must start beyond the band `fracture_shell` will remove),
   `length = (radius as f32 * tuning.crack_reach) as i32 + from`,
   `rays = tuning.crack_rays`. Make `score_cracks` `pub(crate)` — one-word
   change in `rigid.rs`. Do NOT call it on the last stage: by then the
   crater is empty and re-landed debris eats rays (§5).
7. **The report line**: accumulate per-blast counters (cells cleared, cells
   left standing by containment, open/contained sector counts; have
   `score_cracks` return the number of cells it scored — trivial signature
   change, update `strike`/`mine_swept` call sites to ignore it) and print
   one line when the blast finishes, gated so tests stay quiet (a
   `pub fn last_blast_report()` the harnesses read, or print from
   `filmstrip`'s boom hook — filmstrip already prints `boom:` at
   examples/filmstrip.rs:1388; extend that).

**`examples/filmstrip.rs`** — add a cracked-cell census print alongside the
existing per-tile counters: count cells with either crack bit set within a
`3×radius` box of the last `explode=` site. Baseline to beat: **47** on
`boom_stone explode=256,220` (all from confined crushes).

### 7c. Tests: which guards legitimately change meaning

Run `cargo test --release --lib` and expect these to need attention — fix
the *contract*, never weaken the guard (`CLAUDE.md`: a guard must be able
to fail for the replacement artifact):

- `most_of_the_blast_radius_becomes_debris_not_vaporized` and
  `the_shipped_defaults_still_throw_plenty_of_debris`: both build worlds
  where the charge is enclosed on all sides by 20+ cells of material. Under
  the new contract a fully-contained stone blast deliberately clears only
  the crush core. The sand test keeps passing on resistance alone
  (40 cells × 0.35 = 14 ≤ 28 = floor × radius ⇒ open). The stone test's
  *intent* ("the vaporize curve must not creep back") should be preserved
  by pinning its Tuning: `containment_floor: f32::INFINITY` (every sector
  open) reproduces the old geometry exactly — same trick the test already
  uses for `debris_fraction`.
- `an_explosion_clears_material_within_its_radius` (epicentre) survives:
  the crush core always clears.
- Add two new guards: (i) a fully-buried stone blast leaves ≥ N cracked
  cells and < M cleared cells (the new contract, stated positively);
  (ii) sand at the same geometry still clears the full disc (the
  resistance term's other direction).

### 7d. Acceptance — images AND counters, then play

```
cargo run --release --example filmstrip -- scene=boom_stone explode=256,220,20,180,60 start=61 every=30 count=6 cols=3 crop=176,160,160,120 zoom=3
cargo run --release --example filmstrip -- scene=boom_stone explode=256,190,20,180,60 start=61 every=30 count=6 cols=3 crop=176,130,160,120 zoom=3
cargo run --release --example filmstrip -- scene=sandbed   explode=256,160,20,180,60 start=61 every=30 count=6 cols=3 crop=176,100,160,120 zoom=3
cargo run --release --example filmstrip -- scene=cavern    explode=186,240,20,180,60 start=61 every=30 count=6 cols=3 crop=126,150,260,150 zoom=2
cargo run --release --example filmstrip -- scene=cavern    explode=256,200,20,180,60 start=61 every=30 count=6 cols=3 crop=126,120,260,180 zoom=2
cargo run --release --example ascii
```

Bars (baselines from §1, measured this session on this machine):

| case | counter | baseline | bar |
|---|---|---|---|
| buried stone | cracked census | 47 | ≥ 300, and *visible* fissure lines on the sheet |
| buried stone | net cells lost | 286 | **down** (pocket only) |
| buried stone | peak bodies | 20 | ~0 (nothing thrown in a sealed cavity) |
| near-face stone | final void | 141 | up, and muck visibly outside the mouth |
| sand 40 deep | cells evacuated | 991 | within ~20% of baseline (sand unchanged) |
| cave roof | cave-volume census | 5,269 → 5,305 | **grows** by ≥ 300 (breach + rockfall into the cave) |
| cave wall | cave-volume census | 5,269 → 3,598 | ≥ pre-blast volume ("a blast must at least pay for itself") |
| any | `ascii` settled scenes | 0.000 ms | unchanged; re-baseline same-session first |

Then look again for what was not measured (`CLAUDE.md` method step 4): the
sheet that matters most is the one nobody wrote a bar for.

### 7e. R3a — the follow-on commit (fracture pacing)

In `structural.rs` (~line 467), before handing `failure.region` to
`rigid::fracture_failing_region`: if `region.len() > FRACTURE_CELLS_PER_TICK`
(~1,000, const for now), BFS from `failure.at` within the region for the
nearest slice of that size, fracture only the slice, and
`schedule_structural_check_around` the remainder's boundary so it re-fails
on later ticks. This bounds work per tick, never whether the rest comes
down — the remainder must be guaranteed a re-visit (the reschedule is that
guarantee; do not rely on the disturbance window alone, and note
`within_disturbance` gating at structural.rs:379 can expire before late
slices fail — test the whole column still falls at `chain_reach` defaults).
Acceptance: cave-wall scene worst frame < 50 ms (baseline 264, headroom per
convention); the per-tile `bodies` line showing a *series* of bursts;
total cells fractured comparable to baseline.

### 7f. Traps (each has already cost this repo real time — do not skip)

1. Two predicates named `is_body_material` exist: `rigid::` (Solid only)
   and `structural::` (Solid|Plant). Crack rays use rigid's — they stop at
   Plant, Powder, EMPTY. Do not "fix" either.
2. No `world.rng` draws in the probe or anywhere position-stable — use
   fixed directions / `rng::jitter`. An extra draw reorders every later
   roll in the frame and breaks replay determinism.
3. `Cell::is_empty()` is managed-aware; the probe's "is this air" test is
   `cell.material == material::EMPTY` (explosion.rs already has the
   precedent comment at clear_annulus).
4. Liquid `aux == 0` means FULL; never write `with_aux(0)` to mean empty.
5. Gate hot-path material reads through the `MaterialDef` field at the call
   site that already holds the cell — never `id_of("name")` in a sweep.
6. Test both drivers (`update::step` and `parallel::step`) — the app runs
   the parallel one. Do not touch `MAX_REACH`.
7. `cargo fmt` is all-or-nothing; do not let it ride along. CI gates
   `cargo clippy --all-targets -- -D warnings`.
8. Stage explicit paths (`git add src/... examples/...`); never `git add -A`.
9. Commit messages carry the measurement: number before, number after,
   what was tried and rejected.
10. A green suite is not a screen change. The acceptance table in §7d is
    the definition of done, and the last step is *looking* at the sheets.
11. Source comments are load-bearing; do not strip them, and record any
    new dead end in the same voice where you hit it.
12. R4 (powder surcharge), if attempted later: it changes what `torque`
    means — every capacity constant and acceptance bar was set against
    body-only torque, so re-deriving them is part of the fix, and the
    N-seed sweep gating an order statistic must exist *before* the change
    lands. Also: powder movement schedules no structural checks on the
    stone beneath it (the re-check hole), and `bearing_moment`'s kern test
    is deliberately mass-independent — feeding surcharge into it changes
    that model's meaning. See §5.

### 7g. What was deliberately not built, so nobody re-derives it

- Explicit spall (worked design exists in the fan-out record if wanted).
- Field-coupled shockwaves moving CA material (do-not-retry: field forcing
  broke field sleep, measured 0.0002 → 3.55 ms permanent).
- Chimney-size calibration (flank shear / Terzaghi mass relief) — behind
  the seed sweep, after R1-R3 are judged in play.
- A fuse/thrown-charge verb — owner's call, app-side, independent.
- `relax_region` on the failure path (R3b) — blocked on porting tick's
  last-resort ground-rooting semantics into it (§5); the diagnosis and the
  gate counters are in §6.

---

## 8. The prototype, built and measured

R1 + R2 + the report line were implemented from §7 (by the cheaper model,
per the owner's direction; adversarially reviewed and measured before
landing). Results against §7d's bars, same session, same machine:

| case | counter | baseline | after |
|---|---|---|---|
| buried stone | cracked census (3r box) | 47 | **340**, a legible fissure star |
| buried stone | net cells lost | 286 | **108** (crush pocket only) |
| buried stone | peak bodies | 20 | **0** |
| near-face stone | crater | mostly refills, circle rim | open bowl, ejecta plume, muck on the surface |
| sand 40 deep | behaviour | — | visually unchanged (resistance 0.35 keeps it open) |
| water 40 deep | behaviour | transient cavity + spray | unchanged after the §8a fix below |
| cave wall | cave volume | 5,269 → 3,598 | 5,269 → **5,399** — the cave *grows* |
| cave wall | worst frame / peak bodies | 264 ms / 189 | **19.1 ms / 2** |
| cave roof | cave volume | +36, nothing falls | +26 with visible breach + small rockfall — the verb works; the full drop correctly awaits R4 (§5 arithmetic) |
| settled world | `ascii` | 0.000 ms | 0.000 ms |

Two §7d bars were missed for reasons worth keeping: the near-face
`roofed_void` census *fell* (93 vs 141) while the crater visibly improved,
because a crater now open to the sky stops counting as "roofed" — a
metric-definition artifact, the fourth time this investigation has had a
metric quietly change meaning under a mechanism change. And the 40-deep
`sandbed` evacuation dropped ~30% because that scene's cover is 40 cells
*upward only* — sideways and down it is hundreds of cells, which genuinely
reads contained. The shallow-cover sand cases the mechanic was built for
are unchanged.

Notable: §1d's chimney collapse and its 264 ms frame disappeared **without
R3a having been built** — containment means a wall shot no longer clears a
disc through to the void and never undercuts the overburden. R3a is still
worth building (any future large failure region still lands in one tick),
but it is no longer the blast's own emergency.

### 8a. One regression caught by looking, not by the bars

The first build made every underwater charge deeper than ~28 cells read as
contained (water had no `blast_resistance`, so it defaulted to
stone-equivalent 1.0), quietly regressing the round-3 water win. No §7d
row covered water — the reviewer's "look again for what you did not
measure" pass caught it. Fixed semantically: the confinement ray treats
`Liquid` as a free face (a liquid displaces rather than confines on a
detonation's timescale), so `blast_resistance` is never read for liquids
and an entry in `water.ron`/`oil.ron` would be silently inert — the field's
doc now says so.

### 8b. Polish noted for a later pass, judged from the sheets

The 12 crack rays at blast scale read slightly *geometric* — straight,
evenly-spaced spokes, an asterisk more than a fracture star. `score_cracks`
was tuned for 5 short rays. More wander/fork and per-ray length variance at
blast scale is a looks-only follow-up; per the runtime-selector convention,
a keybound A/B against the current look would settle it in minutes.

## 9. The believability pass — §8b's polish, done properly

The owner's verdict on §8's sheets, verbatim intent: the radial blast lines
were perfectly uniform and mirrored, and collapses fell as perfect columns
or sharp triangles. A geometry audit found the four mechanical causes, and
this pass (implemented by the cheaper model from a full spec, adversarially
reviewed) replaced the shapes without changing what fails:

- **The blast's fissures now use the crush walker, not straight rays.**
  `structural::walk_fissures` is `crush_in_place`'s wandering, forking,
  position-keyed walker extracted with a `detach` flag: `false` keeps the
  crush path *bit-identical* (verified by PNG hash on `scene=strike`), and
  `true` gives the blast what `score_cracks` gave it — detach + scheduling
  per cell, and the crack-tip bonus so a repeat charge deepens its own
  fissures (gated on `detach` deliberately: a tip bonus on the crush path
  would re-fund the re-crush treadmill `tick`'s wrote-nothing guard exists
  to stop). Per-ray heading jitter inside the fan slot and squared-jitter
  heavy-tailed lengths kill the uniformity and the mirroring.
- **The crater edge is ragged and the sector cliff is gone.**
  `CRATER_RAGGEDNESS` (0.35) is the crater's own `SCORCH_RAGGEDNESS`, and
  the sector-reach array is smoothed — with the smoothed value clamped to
  `max(smoothed, probed)`, because the plain kernel shaved a 3-sector open
  run to 19-of-20 reach and silently flipped the wall shot to 16/16
  contained (kept as a reproduction test). `fracture_shell` asks the same
  ragged limit, so the thrown shell and the crater agree about the edge.
- **Fissures are now fragmentation seams.** `take_fragment` refuses to
  flood across a cracked edge (tested before the claim, so region material
  is conserved), so pieces separate along the cracks instead of along BFS
  ring boundaries.
- **Failing regions get a torn edge.** `erode_failing_boundary` drops
  rock-facing boundary cells at 45% (position-keyed, erosion-only, floored
  at 12 cells); dropped cells stay standing and re-fail on later ticks —
  the staged crumble `MAX_SUBTREE_CELLS`'s doc always promised.

Measured (same session, baselines re-run on the spot): buried-stone
fissures 340 → 353 and the star reads as fracture — unequal, wandering,
forked, unmirrored; cave-wall keeps its outcome (cave grows, 5,358 vs bar
≥ 5,269) with a ragged breach; cave-roof breach *improved* (+153 cave cells
vs +26) with rockfall visibly entering the cave; `capped`, `snap`, and the
`strike` crush control are bit-identical to baseline; `ligament` still
snaps at the neck, now in two staged bites with a ragged stub;
`ascii` settled 0.000 ms; 609 tests green; clippy clean.

**Deltas to watch in play** (stated, not hidden): `worked`'s shelf now
comes down completely as a graded shower where it used to leave a razor-cut
remnant (judged better under §0a, but it is the pass's largest behavioural
change); the cave-wall overburden failure is somewhat smaller (493 vs 751
cells — erosion working as intended). One flagged pre-existing edge the
blast path now shares with the crush path: the walker can score crack bits
against bedrock edges (`structural::is_body_material` does not exclude
bedrock), which near a bedrock floor could sever an anchor edge — none of
the captures reaches bedrock; left unfixed rather than silently narrowed
(§7f trap 1), recorded here for whoever first blasts the world floor.

### What is still open, in order

1. **R3a** (fracture pacing) — small, specced in §7e, still wanted.
2. **R4** (powder surcharge) — the roof-drop completion, behind the seed
   sweep (§7f trap 12).
3. **R3b** (converged relax after mass failure) — behind the
   ground-rooting port (§5).
4. Live-panel wiring for the four new `Tuning` fields (`tunables.rs` —
   deliberately left out of the prototype's scope).
5. Crack-tint contrast (R5) — the halo is the mining loop's progress bar
   and still draws faint at play zoom. (§8b's ray-shape polish is done —
   this section.)
6. The bedrock-edge flag above, if blasting near the world floor ever
   shows it.

