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

*Sections 5-7 — verification results against source, pathology diagnoses in
full, and the implementation handoff — are being finalized.*
