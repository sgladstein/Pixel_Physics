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

## 10. Fissures that grow — the stamp complaint

Owner, off §9's sheets: the star is "a little better but it looks the same
everytime, it looks like a graphic stamped on the stone and not a realistic
fissure. It would be cool if you could see it grow." Diagnosis: the whole
star was written in one call on the bang frame and never changed — a thing
that appears whole and never moves is a decal whatever its outline.

Fix: the walker became resumable (`structural::FissureWalks`, two drivers
over one step core — the sequential one keeps the crush path bit-identical,
PNG-hash verified again; the round-robin one grows a little each frame).
The blast builds its rays at trigger with the identical jitter keys (the
pattern for a site is still a property of the rock — a repeat charge still
retraces and deepens) and walks them `crack_growth` (2) steps per frame,
each ray starting after a position-keyed delay up to `crack_stagger` (8)
frames. The blast stays alive until the star finishes (~25-35 frames —
half a second of tips visibly racing outward after the flash). Cavity work
is guarded on `stage < stages`; without that guard the growth frames kept
expanding the annulus and ate the world one ring per frame — caught in
implementation, pinned by the sync-driver tests.

Measured: per-tile cracked census over the growth sheet went from
`353 ×8` (the stamp) to **0 → 39 → 148 → 249 → 309 → 339 → 345 → 345**;
final pattern same family (345 vs 353 — the shared fork pool's licensed
delta); every §9 outcome bar holds (cave wall 5,366, roof +144, sandbed
identical, strike control hash-identical, `ascii` settled 0.000 ms,
615 tests, clippy clean). The sub-cell-resume trap (re-deriving fx/fy
straightens every crack) and the growth contract each have a
mutation-verified test.

Recorded, pre-existing (verified against the unmodified tree same
session): `scripts/acceptance.sh`'s `ligament` (72.6 ms vs 60 ms budget)
and `roomcut` (1 overload vs bar 5) fail before and after this change; and
a repeat charge at the *exact* same coordinates fissures nothing because
its own open crater vents the confinement probe (`struck_solid` false) —
the accumulation story holds for charges near, not in, the old crater.
Both belong to whoever picks up R3a/R4.

## 11. Round three — brittle cracks, calving pieces, an ember that cools

Owner playtest verdicts on §10: cracks "a little too organic" (what does
real black rock look like?); "I don't see the pieces moving at all after
the crack"; "the orange glow around also doesn't look great." Exploration
turned the third into mechanics: **scorched stone never cooled** (stone's
`heat_conductivity` 0.0 hits `fire::update`'s thermally-inert fast path, so
a 900° ring was permanent), the render ramp saturates at 420° with
non-burning blend capped at 0.5 (900° drew as flat bone-tan, not ember),
and cracks drew as whole-cell 0.43× smears with no direction. Also found
and parked: **blast smoke never dissipates anywhere in the sim** (no
removal rule; pools under ceilings forever) — a standing issue for a later
pass.

What landed (K1-K4):
- **Brittle crack style** (blast path only): straight runs of 3-8 cells,
  sharp position-keyed kinks, rare large deflections, acute-angle forks —
  jagged lightning instead of meanders. Crush path bit-identical again
  (PNG hash). An A/B sheet of `strike` with the brittle style exists for
  the owner to choose later (`round3_ab_strike_brittle.png`).
- **Calving**: when the star finishes growing (and the cavity is done),
  the rim fractures along the cracks — open sectors 8 deep, contained
  pockets 2 — and `take_fragment`'s crack-seam rule means the pieces are
  bounded by the fissures the player watched grow. Buried-stone peak
  bodies: **0 → 4**; `calved` added to the blast report.
- **The ember**: `flash_temperature` 900 → 260, `fireball_fraction` 0.5 →
  0.3 (both also corrected in `assets/explosion.ron`, which the app loads
  and which still pinned the old values — the one place the change would
  have been invisible in play). Crack tips write ~300° as they race
  (`crack_glow_temperature`). The blast then owns its afterglow: each
  frame it cools everything it heated toward ambient
  (`afterglow_retention` 0.94/frame; never raises, never touches burning
  cells, hard-capped at 180 frames — no tuning value can make a blast
  immortal, mutation-tested). Buried-stone hottest/lit cells: **891°/2,665
  frozen forever → 20°/0 by frame 181.**
- **Hairline cracks**: at zoom > 1, only a strip along the actually-severed
  edge darkens (harder, 0.35× vs 0.43×), so a crack is a directional line
  threading the rock; zoom 1 draws exactly the old way (pinned by test).

All §10 bars held: cave wall 5,367 (≥ 5,269), roof +144 retained, sandbed
unchanged, strike hash identical, `ascii` settled 0.000 ms, 625 tests,
clippy clean. Acceptance 14/16 with the same two pre-existing failures.
Net cells lost in the buried case unchanged at 112 — calving relocates
rim rock into the pocket, it does not eat more world.

## 12. R3a — big collapses arrive in stages

Built per §7e, with one deliberate design change forced by measurement:
the specced "fracture a slice, reschedule the remainder and let it
re-fail" was built first and failed two ways — it *stalls* (the load
budget starves the re-judgment: a miniature ligament was still 1,132 of
4,420 cells short after 1,200 frames) and it can *lose* (the remainder
escapes via the disturbance window at any setting but SPREAD, or via the
12× attached-bulk bonus it still carries) — each of which is rock hanging
in air. Recorded as a dead end in the source. What shipped instead is a
**work queue, not a fresh question**: `World::staged_fractures` holds the
remainder and `structural::advance_staged_fractures` (called from
`scheduler::step`, outside the site/load budgets) takes a jittered
BFS slice of `FRACTURE_CELLS_PER_TICK = 1,000` cells every 5 frames,
re-filtering each slice against current world contents so a later blast
or landing body cannot be double-fractured. The cut frontier is ordered
on `distance + jitter` because a plain BFS ring is an L1 diamond — the
§9 complaint re-manufactured by a new mechanism.

Measured: `ligament`'s 4,420-cell overhang now comes down in 5 bites over
~25 frames (bodies per tile 34→67→90→106→90→72→70→68→65 against a single
142-body burst), worst frame **72.3 → 30.5 ms**, and the pre-existing
`ligament` acceptance failure recorded in §10 is now a PASS (suite
15/16 — only `roomcut` remains, pre-existing). `worked`/`capped`/`snap`/
both cavern scenes byte-identical (the cavern scenes never reach the cap —
containment already shrank their failures; stated, not hidden). New
counters `staged_slices`/`staged_cells` print in filmstrip. 629 tests,
clippy clean. Known residuals, stated: confined crushes are not paced (a
20,000-cell confined crush is still one tick); the transient slice
boundary reads as a wobbly diagonal for the 5 frames each stage lives.

## 13. R4 — powder weighs on the stone beneath it

Built per §4's R4 and §7f trap 12. `load::powder_surcharge` walks the
contiguous `Powder` column above each cell the load walk enters, capped at
`POWDER_SURCHARGE_CAP = 12`, and adds `depth x POWDER_SURCHARGE_WEIGHT`
(1.0) to that cell's own mass and moment — the same accumulator the cell's
own `LOAD_SCALE` enters, before the `support_count` share division, so it
stays conserved across parallel routes. `Liquid` is deliberately excluded.

### The specced roof blast does not contain the situation any more

`scene=cavern explode=256,200,20,180,60` came back **byte-identical**
before and after: overloaded 3 (49 cells), cave 5,269 -> 5,413 (+144),
lost 214, in both. A `dump=` of the crater at frame 300 says why — the
crater is *empty*, one loose cell in seventy-two columns. R1+R2 already fixed
that case: the breach opens downward and the muck falls into the cave
instead of plugging, exactly as §5 predicted ("R4 makes standing plugs
matter afterwards"). There is no plug left for the surcharge to charge.
This is `CLAUDE.md`'s "when a mechanism appears inert, check the scene
still contains the situation you think it does", and it cost one dump
rather than an afternoon.

Raising the charge until the crater floor is a shell rather than a hole
puts the plug back. At **`explode=256,184,...`** (28 cells above the cave
roof) the crater fills and stands on ~8 cells of cracked shell:

| `explode=256,184,20,180,60` | weight 0.0 | weight 1.0 |
|---|---|---|
| overloaded failures | 7 (478 cells) | **95 (5,693 cells)** |
| cave volume | 5,400 (+131) | **5,682 (+413)** |
| cells lost | 174 | 467 |
| largest failing region | 219 | 1,258 |
| peak bodies in flight | 3 | **36** |

The sheets (`scratchpad/r4_roof184_wide_*.png`) show it as the two-beat:
blast, pause, and then the roof left of the crater comes down into the cave
with rubble landing on the cave floor. At weight 0.0 nothing moves after
tile 3.

### The paired control, as a 2x2 rather than a single arm

`filmstrip depowder=<frame>` keeps the world clear of loose material from
that frame on. Continuous, not one-shot, and that was measured: a single
sweep at the blast frame removes **zero** cells (the muck is still in the
particle system), one at frame 75 removes 74 and one at frame 100 removes
123 — the plug is still arriving, so any single instant is an arbitrary
fraction of it.

| overloaded / cave volume | plug standing | plug vacuumed |
|---|---|---|
| weight 0.0 | 7 (478 cells) / 5,400 | **0 (0 cells)** / 5,528 |
| weight 1.0 | 95 (5,693 cells) / 5,682 | **0 (0 cells)** / 5,528 |

The two vacuumed arms are identical to the digit. So the bare cracked shell
holds at either weight, and the surcharge — not a capacity regression
wearing its clothes — is the whole of what moved. (Cave volume is not
comparable *across* the vacuum flag: vacuuming empties cells the census
would otherwise still count. Compare vacuumed only to vacuumed.)

### The sweeps

Re-baselined in the same session on the same machine, per `CLAUDE.md`,
because the recorded `scratchpad/sweep_base_strike.log` does **not**
reproduce against current `HEAD`: it has `rolling/1` at 147 lost where a
clean `HEAD` build measures 351, and `flat/3` at 557 where `HEAD` measures
442. Every other row matches. That log predates something on this branch;
it is quoted below but the gate is the same-session weight-0.0 arm, which
was verified byte-identical to a clean-`HEAD` binary on three seeds.

| `strike=12`, 24 runs | recorded log | weight 0.0 (same session) | weight 1.0 | vs 0.0 |
|---|---|---|---|---|
| cells lost, max | 557 | 442 | 583 | 1.32x (bar 2x) |
| cells lost, p90 | 168 | 254 | 120 | 0.47x (bar 1.5x) |
| rock destroyed, max | 1,649 | 1,548 | 1,349 | 0.87x |
| rock destroyed, p90 | 726 | 1,111 | 948 | 0.85x |

`dig=6` is **bit-identical** across all three — every one of its 24 runs
produces zero overload failures at every seed, so there is nothing for a
surcharge to change. (Identical output is normally the tell that a knob was
never connected; here `strike` moves hard on the same binaries, and the
`overload` column is 0 in the baseline log too.)

So the term *cost* nothing at 1.0 and mostly bought margin: three of the
four order statistics improved. The escape hatch (halve the weight) was not
needed and was left in place. What did grow is the largest single failing
region on the worst seed — `flat/3` 1,598 -> 2,509 — which R3a stages at
1,000 cells a tick, so it arrives in three bites rather than one frame.

Frame cost, paired, `repeat=3` minimum: `cavern` 28.39 -> 30.95 ms on the
scene that now promotes 36 bodies instead of 3 (w00's own spread on that
scene is 28-85 ms, so this is inside noise and under the 60 ms acceptance
budget); `capped` 26.17 -> 17.83 ms; `terrain` unchanged. `ascii` still
reports every scene settling to 0/N chunks awake with the field's settled
pass at 0.0018 ms — a settled world pays nothing, because the scan runs
only inside a load walk and load walks run only on scheduled checks.

Acceptance 15/16, the same as before: only `roomcut`, which fails for its
pre-existing reason (0 overload failures against a bar of 5) and is
recorded in §10. 632 lib tests, clippy clean.

### The re-check hole, and what is left open

Powder movement schedules no structural check on the stone beneath it, and
that is unchanged. The bounded case was already covered rather than needing
building: `rigid::settle` schedules a check around **every** cell of a body
it writes back, not just the footprint row, so a slab arriving on a shelf
is re-judged. `settle`'s doc now says the surcharge depends on that, so
nobody narrows it later as an optimisation.

Deliberately not closed: scheduling from ordinary per-cell powder movement
would flood the scheduler from every avalanche in the world. The residual,
stated: **slow powder creep onto a marginal shelf may not re-trigger
judgment until something else disturbs it.** Closing it properly wants a
coalesced "this pile changed" signal, not a per-cell one.

Also stated rather than left to be rediscovered: the surcharge reaches
`bearing_moment` through `mass`, and the kern criterion is mass-independent
by construction, so loading a slab that stands on rubble raises both sides
equally and does not move that verdict. That is the physically right answer
and it is the quiet one — if the surcharge ever looks like it is tipping
pieces that used to sit still, that is the line to read first.

### What is still open, in order

1. ~~R3a~~ — done, §12.
2. ~~R4~~ — done, this section. Not yet judged **in play**: every number
   above is headless, and the ethos says the verdict is the hand, not the
   diff. The specific thing to look for is whether a *deep* pile now reads
   as too heavy — the cap is 12 and nobody has felt it.
3. **R3b** (converged relax after mass failure) — behind the
   ground-rooting port (§5), and **downgraded from emergency to
   improvement** by a HEAD re-measurement of §1d's livelock: pending
   sites now peak (~13.8k at f800) and *fall* (6.7k at f1200) with awake
   chunks declining 7 → 6 → 5, where the §1 baseline climbed forever
   (17.9k and rising, 10-11 awake). Containment shrank the wound and R3a
   paced what remains; the queue drains. Still worth doing — f1200
   should be quiet, not merely quieting — but it is the touchiest
   remaining change and deliberately not attempted unsupervised.
4. ~~Live-panel wiring~~ — done: all 22 `Tuning` fields are in the panel,
   and the round-trip test now destructures `Tuning` exhaustively so an
   unwired field fails to compile.
5. Crack-tint contrast (R5) — the halo is the mining loop's progress bar
   and still draws faint at play zoom. (§8b's ray-shape polish is done —
   §9/§11.)
6. The bedrock-edge flag above, if blasting near the world floor ever
   shows it.
7. ~~The stale sweep baseline~~ — resolved: `sweep_base_*.log` was frozen
   against the round-3 binary (`990758b`), and the only behavioural commit
   between it and `HEAD` is **R3a** (`ed58fc9`; the tunables commit is
   panel-only). Staged fracture changes cascade timing, and outcomes are
   chaotic in the seed, so `rolling/1` and `flat/3` reshuffling while the
   envelope holds (max fell 557 → 442) is R3a behaving as measured, not a
   mystery. Standing lesson kept: a sweep baseline is only valid against
   the exact commit it was taken on — re-baseline in-session, which is
   what §13's gate did.
8. ~~Smoke never dissipates~~ — done. `MaterialDef::dissipation` (a
   per-tick chance, `0.0` = never, so every other gas keeps today's
   behaviour *and* today's random stream); `smoke.ron` sets `0.004`, a
   ~173-frame half-life. Not as contained as it looked, and the reason is
   worth keeping: **rolling it on the CA sweep alone does not reach the
   case the bug is about.** A stone box packed with 336 smoke cells lost
   25 and kept the other 311 for 2,500 frames, because its chunks settle
   about nineteen frames after the smoke does; the buried crater kept
   three of its five. That is `evaporation.rs`'s lesson arriving one
   material-kind over, so it took the same answer — a gas cell that could
   not move schedules an `ActiveKind::Dissipate` site (deduped by
   position, load-bearing for the *rate*, not just for cost) and the
   sweep forgets about it. Cleared: pocket 1,687 frames serial / 1,565
   parallel, crater 430. A *second*, higher rate for trapped gas was
   considered and not taken — the scheduler already gives trapped smoke
   its own path, and one number is the whole knob.


---

## 14. The "pale light" report — it was not the explosion

Filed under §14 rather than in the open list because the answer turned out
to have nothing to do with blasting, and the next session to read a report
like it should not spend the day here.

**The report.** *"A pale light effect spreads through rock, lightening it
over time"* — slowly, from the core of whatever was hit, every time, deep as
well as shallow — plus *"random flashes"* during cascades.

**What it actually was.** `render.rs`'s damp-ground darkening read
`Cell::aux` with no material-kind check, and `aux` is a tagged union: on a
`Solid` it is the **distance to the nearest structural anchor**. Every stone
cell in the world was therefore drawn darker in proportion to how far it
stood from an anchor, and `structural::tick`'s relaxation wavefront re-lit
it over the following frames wherever anything was disturbed. The owner's
own observation settled it — *it also happens on a strike, with no explosion
anywhere*. Measured on `scene=worldcrack preset=rolling seed=1 strike=12`
against the same world unstruck, diffed at frame 400: **2,683 grey-stone
pixels brightening, in a halo x 200-339 around a 12-cell strike at x=256**,
998 of them more than 40 cells from anything the blow touched, mean delta
dead achromatic. After the one-line kind gate: **94, all inside x 238-284,
none beyond 40 cells** — and those are crater cells whose material really
changed. The flashes were a second defect in the same file: bodies and free
particles were painted raw palette colour with no `sky::apply_light`, so
every promotion flashed its cells up to 0.42x brighter and every landing
flashed them back.

Three explosion-side candidates were ruled out by measurement and code read
before landing on that, and they are recorded so nobody re-derives them:
the skyline is **frozen at worldgen** (`World::sky_surface`, never revised),
so removing material cannot re-lighten a column; rock brightness comes from
a **global** scalar (`sky::apply_light`) and never from the light field,
which `render.rs` documents as a deliberate correction; and nothing copies
field temperature into cell temperature.

**One measurement trap worth keeping.** Above the soil line a paired
blast/no-blast comparison is **useless**: the blast draws from `world.rng`
and shifts every later roll, so plants and weather diverge and the diff is
mostly foliage. The strike control and a deep-rock restriction are what made
the pair readable.

### 14a. Two real leaks found on the way, left for a later pass

Neither fits the owner's report — the first decays rather than lightens —
and both want a counter next to the image before anyone touches them.

1. **A glowing cell that becomes a falling grain escapes the fade
   permanently.** `cool_toward_ambient` cools **by position** (the shell box
   plus `walks.scored()`), while `rigid::shatter_to_rubble` carries the
   source cell's temperature into the new grain — and that grain then
   *moves*. It is never at a cooled position again, and rubble is thermally
   inert (no conductivity, so `fire.rs`'s fast path returns before any
   decay), so it stays hot for the rest of the run. The afterglow's extent
   is real too: the crack star reaches ~85 cells against the fireball box's
   ~29, and those cells are cooled only via `walks.scored()`.
2. **Weather lightning is not a blast artifact.** `weather::strike` is a
   whole-frame white lift with **no world damage** at all, so it explains
   "random flashes" seen over several day/night cycles and has nothing to do
   with the charge. Nothing to fix; recorded so it is not investigated a
   second time.

## 15. The joint fabric — the pattern stops being drawn and starts being read

Three sheets in a row came back rejected on the same axis, in escalating
terms: *"thin criss-cross wiggly crack patterns — looks like a graphic, not
physics"*, then *"it shouldn't look like a scribble"*, then — after a blind
A/B of two tunings of the walker, **both** of which he declined — *"I thought
we were going to match the Voronoi type pattern from my worldgen example
image."* On that same card: *"The voronoi pattern is too much. I like a
little of it, but there is too much."* So: right *kind*, wrong *density*.

### 15a. Why it could not be a tuning

Two properties of the reference are out of `structural::FissureWalks`' reach
by construction, and no setting of `CRACK_WANDER` gets to either.

- **Straight edges.** The wander is 0.9 rad *per cell*. A heading re-rolled
  every cell cannot draw a straight segment; the scribble is the walker's
  statistic, not its tuning.
- **Closed cells.** A walker encloses a piece only by luck, and four rounds
  of work went into improving that luck (decomposed diagonals, both
  perpendicular edges, the mirror write). The measured table in
  `FissureWalks` says the best of those still escaped in three of four
  builds before the last two landed together.

`sim::fracture_field` replaces both with one rule: **an edge is a joint iff
its two cells lie in different Worley domains.** That set is *exactly* the
boundary of each domain on the 4-connected grid, and support is 4-connected
everywhere (`NEIGHBOURS_4` in `structural.rs`, `rigid::take_fragment`), so a
domain whose boundary is severed is enclosed — watertight, not lucky. Worley
boundaries are straight by construction. It is an **identity comparison, not
a threshold on `f2 - f1`**: a threshold has width, and width is what leaked
and needed lateral patching in the cave work.

### 15b. What a blast does with it

Three zones off one distance (`explosion::JointSeams`): the inner one
**opens** a joint into a one-cell seam of void and grit, the middle one
**scores** it, the outer does nothing. Activation is `joint_draw(pair) <
ramp(distance)` with the ramp flat to the crater wall and linear to zero at
`joint_reach * radius` — no hard cut anywhere, so the damaged region's edge
is ragged and some joints reach much further out than their neighbours.

**The draw is keyed on the pair of domains, not on the edge**, and that is
the difference between a craquelure and a dotted line: one number per
boundary means a joint is either a full straight segment or absent, and
comparing it against a falling ramp draws each boundary *from the blast
outward until the ramp drops under its own draw*. That is where "some cracks
short and near, some reaching further" comes from — geometry, not a length
distribution.

Opening happens **at trigger**, which also answers the other complaint on
that card (breakage arriving 7–15 seconds late, because it waited on a
relaxation wavefront moving one cell per five frames). The scored halo keeps
the growth beat, on the same `crack_growth` / `crack_stagger` knobs.

### 15c. The sweep, and which knob is the cost lever

Nine charges into a generated rolling world, seeds 1/3/7/24301, max over the
four (`scripts/blastsweep.sh`). Baseline is `d9eec7f`, re-measured in the
same session on the same machine and matching its recorded figures exactly.

| setting | cells lost | rock destroyed | promoted max / **min** | sites, final tile |
|---|---|---|---|---|
| baseline (walker star) | 2,801 | 4,136 | 4,862 / **654** | 8,643 |
| spacing 13, open 0.45 | 5,245 | 10,539 | 16,266 / 11,641 | 21,577 |
| spacing 16, open 0.45 | 5,454 | 9,978 | 17,467 / 10,953 | 22,448 |
| **spacing 13, open 0.30** | **4,695** | **8,472** | **11,021 / 6,043** | **13,056** |
| spacing 16, open 0.30 | 4,860 | 7,941 | 14,437 / 10,181 | 23,576 |

**`joint_spacing` is not the cost lever, and that was not obvious.**
Coarsening 13 → 16 cuts total boundary length by a fifth and the material
bill by a twentieth, because most of what a blast removes is not the seam
cells themselves but the blocks the seams cut free — bigger polygons, same
mass. `joint_open_fraction` is the lever: 0.45 → 0.30 takes rock destroyed
down 20% and the still-busy site count down 40%.

Read the **minimum** of the promoted column, not the max. 654 cells over
nine charges is *"no pieces move, ever"* expressed as a number; every fabric
setting clears it by an order of magnitude, and the bodies count went 77 →
559 on seed 1.

Cracked cells in the world went **down**, 6,185 → 3,368 on seed 1: the
fabric leaves less ink on the rock than the walker did, which is the
"sparser" half of the brief as a number rather than an opinion.

### 15d. Open, and deliberately not fixed here

**Four of the 36 sweep charges wake no joints at all**, all of them with
16/16 open sectors. The gate is `probe_confinement`'s `struck_solid`, shared
with the walker — a shallow charge on a slope whose every probe ray vents to
air before crossing a `Solid` cell reads as "no rock to crack". It is
pre-existing (those same charges scored zero fissures at `d9eec7f`), and the
obvious fix — let `JointSeams::wake` decide, since it already returns `None`
when nothing in reach is jointed — makes an *airburst* dice the ground under
it at full ramp, which is worse. It wants a distance term on the charge's own
standoff, not a gate flip.

**Closed in §15f below**, with that distance term.

## 16. The crush stops drawing a scribble over the fabric

The owner reviewed ten animated blasts, liked the pattern for the first time,
and gave four objections. Two of them turned out to be one mechanism:

> **"they all have these thick lines that appear later in the blast that I
> don't like"** — three pin-drop annotations, on all three of his favourites.
>
> **"there are other random movements and small random collapses that keep
> happening that are not as good"** — explicitly distinguished from the crack
> spread, which he likes.

### 16a. It was `crush_in_place`, and only that

Three things write cracks. `explosion::sever` and `rigid::score_cracks` each
mark **one** edge. `FissureWalks::step_walker` marks the edge **and its
mirror** (added by `1c0bcf7` so a line drawn *through* cells seals against a
4-connected flood), so it darkens **two** adjacent cells — and at zoom 1,
which is what the app and every GIF use, a crack darkens the whole cell.
With `crack_rays` defaulting to `0` the blast builds no walkers at all, so
**the only walker still running by default was the crush**: `CrackStyle::
Wander` at 0.9 rad per cell, fired off the relaxation wavefront hundreds of
frames after the flash, and `run_to_completion`, which draws the whole
3-ray, 4-fork, 10-to-55-cell star in a single frame, fully formed. Thick,
scribbly, late and arriving whole — the complaint exactly.

Measured, `blast=300,45,20,180,60` on rolling seed 1, read at frame 1,200,
against the shipped `confine=0` control:

| | crush on | crush off |
|---|---|---|
| cracked cells in the world | 610 | **145** |
| overloaded failures | 679 | **87** |
| unsupported judgements | 2,365 | **29** |
| cells promoted | 2,226 | **2,814** |

The crush wrote **76% of every crack cell in the world**, caused **80x the
unsupported judgements**, and moved *less* material than not running at all.

### 16b. What replaced it

`crush_in_place` now reveals `fracture_field`'s joints instead of walking a
star, so **every crack in the game is one cell wide, straight and closed**.
Two objects, two rules, keeping the `CrushedObject` split that `d9eec7f`
paid for:

- an over-capacity **section** is attached and cracking into the massif
  around it, so it reveals the joints in a disc about the cell that gave
  way, sized off the region's extent between `CRACK_MIN_LENGTH` and
  `CRACK_MAX_LENGTH` — the bounds the star's length used;
- a **severed piece** has no claim on the rock outside it and reveals only
  joints with *both* cells inside itself. A piece smaller than the 13-cell
  grain then contains no joint and writes nothing, without needing a floor
  to say so.

Nothing here calls `detach_around_crack` or `schedule_structural_check_
around`, and nothing opens a seam. That split is what keeps a confined crush
from being a treadmill and it is the single easiest thing to get wrong;
`a_crush_neither_unbraces_the_rock_nor_reschedules_it` is now written against
the crush rather than against the walker, because the old version would have
gone on passing whatever the crush did.

The same charge, at frame 1,200: cracked cells **610 -> 379**, unsupported
judgements **2,365 -> 277**, overloaded 679 -> 787, worst frame 58.7 -> 39.7
ms.

### 16c. The promotion figure falls, and it is lateness rather than breakage

Cells promoted at frame 1,200 go 2,226 -> 1,213, which read alone looks like
lost breakage. Sampled as a curve instead (bang at frame 60):

```text
  frame       100    300    500    700    900   1100   1300
  walker      503    528    528  1,745  2,184  2,226  2,710   ...still climbing
  fabric      503    693    830  1,155  1,213  1,213  1,213   settled
```

**Identical at the bang, and the walker is still climbing at 1,300.** What
went is a collapse trickle that never terminates — the owner's *"small
random collapses that keep happening"* as a number. Cracked cells tell the
same story on the same run: 233 -> 693 for the walker, 197 -> 379 settling by
frame 700 for the fabric.

The nine-charge sweep agrees, and it is the statistic that matters because
`promoted min` is the *"no pieces move, ever"* guard. Four seeds, baseline
re-measured this session and matching `d5cb19a`'s recorded figures exactly:

| | cells lost | rock destroyed | promoted max / **min** | sites, final tile | worst frame |
|---|---|---|---|---|---|
| `d5cb19a` | 4,695 | 8,472 | 11,021 / **6,043** | 13,056 | 104 ms |
| this work | 5,248 | 9,526 | 13,313 / **9,042** | 20,188 | 73 ms |

### 16d. The cost, and why no setting of it is cheaper

`scripts/seedsweep.sh strike=12` fires no blast, so it isolates the crush.
24 runs, order statistics, same session:

```text
                                 cells lost        rock destroyed
                                 max     p90       max     p90
  d5cb19a, the walked star       192     118     1,106     565
  disc, density 0.9 (shipped)    521     174     1,345     852
  disc capped at 20, not 55      562       -     2,034       -
  density 0.5 rather than 0.9  1,229     580     2,308   1,815
  section bounded by its region  430     250     1,774   1,086
```

**Every attempt to damp it made it worse**, and not because the tuning was
missed. A region diced *completely* has no free face anywhere — every block
is wedged — so it is judged confined and stays where it is; a region diced
*partially* leaves pieces with an open side, and those fall. Thinning the
reveal moves material off the hillside rather than saving it. The shipped
setting is the most complete of the four rather than the gentlest, and
`CLAUDE.md`'s "when a term resists tuning in both directions, ask what it is
compensating for" applies: the trade is structural. `dig=6` is bit-identical
before and after.

### 16e. `CrackStyle::Wander` is retired, not deleted

`crack_rays > 0` is still the owner's hybrid A/B knob and the blast's walker
star is byte-identical, mirror write included. What became `#[cfg(test)]` is
the *one-shot* entry: `walk_fissures`, `FissureWalks::new` (the constructor
that picks `Wander`) and `run_to_completion`. The tests that pin the walker's
archived shape all still run through them.

### 16f. §15d closed: a standoff distance instead of a gate

The fabric no longer consults `struck_solid` at all. Confinement reaches it
as **two continuous scales on one radius**, never as a yes/no:

- **vent compensation.** A fixed halo over a half-sky disc wakes half the
  joints for reasons of geometry alone, which is why the surface burst read
  as *"not much happening"*. The reach is stretched by `1/sqrt(contained)`,
  capped at 2x. A fully buried charge has `contained == 1` and is
  bit-identical.
- **standoff coupling.** Air is a poor coupler, so the distance from the
  epicentre to the nearest ground scales the reach *and* the activation
  density to zero over half a crater radius.

On the owner's own ten configurations, seed 1:

| | before | after |
|---|---|---|
| 1 deep, standard | 606 | 606 |
| 2 shallow crater | 144 | **461** |
| 3 surface burst | 105 | **335** |
| 4 seabed, water overhead | 376 | **507** |
| 5 bigger charge | 1,318 | **1,539** |
| 6 small charge | 222 | 222 |
| 7 finer grain | 950 | 950 |
| 8 coarser grain | 341 | 341 |
| 9 bolder seams | 606 | 606 |
| 10 wider halo | 1,154 | 1,154 |

Buried:surface narrows from 5.8:1 to 1.8:1 with the buried case untouched.
Every fully contained charge is bit-identical, which is the check that the
scale is a compensation and not a general loosening.

**And the airburst does not dice the ground.** `blast=300,-8,20,180` woke 0
joints before (gated out); it now wakes **12, all scored, none opened** — it
marks the rock and removes nothing from it. The harness's own airburst
(`blast=470,-8,...`) still wakes 0, because at that site the stone is under
ten rows of unjointed soil and sits right at the edge of the shortened
reach. So §15d's four zero-joint charges are partly fixed: the shallow ones
on slopes now crack (charge 7 of the sweep, 16/16 open, goes from 0 to 825
joints), and a true airburst stays nearly silent, which is what it should
be.
