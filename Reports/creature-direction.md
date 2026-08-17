# Creature direction: cell-chain ants, a caged brain, and a heritable genome

**Status:** direction agreed with the owner (2026-08-17). This document is the
decision record plus a full implementation plan, written to be executed cold by
a session that has read nothing else. Where it says "decided," do not
re-litigate; where it says "open," ask or measure.

**Required reading before implementing anything here, in this order:**

1. `CLAUDE.md` — the method. Every rule in it was paid for.
2. `Reports/stigmergy-research.md` — the algorithm this milestone is built on,
   with tuned parameters. Read the whole thing, especially §2 (choice
   function), §3 (evaporation + crowding), §6 (Physarum loop + the resolution
   constraint), and the side-view staging caveat at the end.
3. `Reports/emergent-world-architecture.md` §0, §1, §6, §7, §12 — the
   architecture the creatures live inside.
4. `Reports/population-dynamics-research.md` §0 — why naive ecologies go
   extinct. Not load-bearing for the ant milestone (one species, no
   predator), but it sets standing constraints on perception and mobility.
5. `research/m18-creature-biology.md` — burrowing, thermotaxis, foraging.
6. `Reports/organism-substrate-design.md` — the substrate creatures join.

Line references below were checked at commit `c70363b` and will drift; treat
them as "where to start looking," and re-read the function, not the diff.

---

## 0. Decision log

All decided with the owner in the 2026-08-17 design conversation:

| # | Decision | Rejected alternatives |
|---|---|---|
| D1 | **Body plan: cell chains.** A creature is 1..N connected cells on the organism substrate; the head picks a move, the body follows snake-fashion. | Life-Engine-style rigid multi-cell bodies (translating/rotating a shape through a falling-sand world is an unsolved hard problem); Noita-style sprite entities atop the grid (creature stops being made of the world; no genome-as-anatomy; no partial damage as cell loss). |
| D2 | **First shipping fantasy: ant/termite colony.** Trails, digging, construction. | Evolving grazers first; predator–prey cycle first (PLAN.md's worm/binder/borer cycle stays queued behind this, still awaiting separate sign-off). |
| D3 | **Heritable genome: one shared mechanism, implemented creature-first**, designed so plants adopt it later without rework. | Plants first; both at once. |
| D4 | **Behavior: a fixed-scaffold evolvable brain, initialized from authored instincts.** A small fixed-topology network whose weights live in a fixed-length positional genome; species `.ron` authors the initial weights as plain taxis gains; hidden units start silent. | Pure taxis with no upgrade path (ceiling too low for later solitary creatures); NEAT-style topology evolution (variable-length graph genome, speciation machinery, hours-of-noise bootstrap, illegible — every downside traced to topology mutation, so topology is what got caged, not the brain). |
| D5 | **"Never creature-to-creature queries" is softened.** Fields are the *default* for at-a-distance sensing because they leave visible, exploitable traces; contact-range cell-grid reads are always fine; bounded local neighbor queries are legitimate when a field buys nothing visible. The hard line holds at colony scale, where the field *is* the mechanism. | The absolute rule as written in `emergent-world-architecture.md` §7/§12-Q5. Do not "fix" that document as part of this work; note the softening if you touch it for other reasons. |

Nothing in this plan needs a neighbor query anyway — ants are the case where
the field genuinely is the mechanism.

---

## 1. What is being built, in one page

Five stages. Each is independently shippable, judged by eye per the
verification protocol (`emergent-world-architecture.md` §10), and each has a
"definition of done" in §9. Do them in order; later stages assume earlier ones.

- **Stage 0 — prerequisites.** The pheromone-resolution experiment; organism
  free list; per-creature RNG stream. Small, unblocking, measurable.
- **Stage 1 — creature-on-organism unification.** Retire
  `CreatureState`/the `aux`-index scheme; the worm becomes a one-cell chain
  species on the organism substrate. No new behavior; everything the worm does
  today still passes.
- **Stage 2 — pheromone substrate + overlays.** Two meaning-free channels,
  deposit/diffuse/decay, rendered before anything reads them. A synthetic
  trail scene proves a follower can track it.
- **Stage 3 — ants.** Chain creatures with the caged brain running authored
  instinct weights (hidden layer silent). Foraging loop: leave nest, find
  food, carry home, trails form. The double-bridge scene converges on the
  short route. Digging and deposition give nest excavation and termite-style
  construction against the moisture field.
- **Stage 4 — colony lifecycle + heredity.** Queen, eggs, worker hatching,
  death → corpse cells, organism-slot reclamation, genome inheritance with
  mutation queen→queen. Evolution is now *on*, on the slow colony clock —
  which is accepted; fast-generation creatures (grazers, later) are what will
  make brain evolution visibly move.

Population target for anything colony-visible: **50+ ants** (stigmergy has a
minimum viable population — Grassé's threshold, `stigmergy-research.md` §1). A
three-ant test scene will look broken when the code is correct. Budget check:
100 ants ticking every 6 frames is ~17 site-ticks/frame against the
scheduler's `MAX_SITES_PER_FRAME = 2000` (`scheduler.rs`) — noise.

---

## 2. Stage 0 — prerequisites

### 2a. The pheromone-resolution experiment (do this before designing anything)

`stigmergy-research.md` §6 sets a hard constraint: gradient-following needs a
sensor offset of ≥3 cells that actually resolves differences. At
`FIELD_SCALE = 8` (`field.rs:47`), a 3-cell offset lands inside one field
cell. Bilinear sampling (`World::field_at_bilinear`, `world.rs:922`) recovers
a gradient but not trail *width* — a one-cell trail smeared across an 8-cell
block stays smeared, and two trails 4 cells apart are indistinguishable.

The report's own instruction: **test cheaply before redesigning.** Write a
throwaway harness (an `ascii.rs` scene or a `#[test]`):

1. Seed a synthetic one-cell-wide trail into a field-scale channel (moisture
   is fine as a stand-in — don't add a channel for the test).
2. Run a single Jones-style follower (three sensors at offset 8, turn toward
   strongest, per §6's loop) for a few hundred steps.
3. Measure: fraction of steps the follower stays within 2 cells of the trail.

**Expected outcome, and the decision it feeds:** the literature says offset-8
tracking of a smeared trail *may* work but trail *separation* will not. The
default plan below therefore assumes the answer is "not good enough" and
builds pheromone as its own CA-resolution plane (§5). If the test surprises
you and tracking is robust, say so in the commit and reconsider — a sixth
`FieldCell` channel is less new code. Either way the experiment costs an hour
and the alternative is discovering the resolution problem after the ants are
built, with the colony milling in circles and nothing telling you whether the
bug is in the brain, the deposit, or the grid.

### 2b. Organism free list (`free_organism` does not exist)

`World::organisms` grows forever; the free list at `world.rs:178` is **never
populated** — there is no `free_organism`. Plants get away with it because
trees are planted by hand. A colony laying eggs will exhaust the 4095-slot
ceiling (12-bit index + 4-bit generation, `world.rs:28-52`) in one long
session.

Implement: `World::free_organism(handle)` — verify the slot's generation
matches, clear the state, bump the generation, push the index on the free
list. `push_organism` pops the free list before growing. A stale
`ActiveSite` whose handle's generation no longer matches must resolve to
`None` and drop itself silently — this is the whole point of the generational
scheme (`Reports/organism-substrate-design.md` §6). Add a test that kills an
organism, reuses its slot, and confirms a stale site does not resurrect or
panic. Note the 4-bit generation wraps at 16 — after 16 reuses a stale handle
reads as live again; acceptable at current scales, but assert-count it in
debug builds so it is a known quantity, not a surprise.

### 2c. Per-creature RNG stream

`creature.rs` still draws from the shared `World::rng`; plants already moved
to `rng::stream` (`rng.rs:109`). `PLAN.md` flags this as required before any
breeding. Key every creature draw as
`rng::stream(world_seed, organism_handle, tick_or_purpose, slot)` — **keyed
on the organism handle, never on position.** Position-keyed draws make
location a hidden inherited variable, which
`Reports/plant-simulation-research.md` §7d identifies as exactly the confound
that manufactures spurious "evolution" results. (The plant genotype is
currently keyed on germination coordinate — `plant.rs:324-342`. Do not copy
that; it is on the list of things the shared-genome work will fix for plants
later.)

---

## 3. Stage 1 — creatures join the organism substrate

### 3a. What exists and what it becomes

Today's worm (`src/sim/creature.rs`, M18 Phase 1) is a *parallel* system:
`CreatureState { energy }` in `World::creatures: Vec<CreatureState>`, indexed
by a raw `u16` in `Cell::aux` (`creature.rs:118-123`), no generations, a
`u16` overflow guarded only by a `debug_assert` (`world.rs:579-580`), and no
reclamation. Every one of those defects is already solved by the organism
substrate. **Retire the parallel system; do not extend it** — extending it
would be the third private solution to per-organism state, the exact failure
`organism-substrate-design.md` opens with.

Concretely:

- A creature is an organism: `Cell::organism_id` carries the generational
  handle; `OrganismState` carries its state; the species comes from
  `SpeciesRegistry` (`assets/species/*.ron`, and **must also be added to
  `SpeciesRegistry::builtin()`** (`organism.rs:745`) or it won't exist when
  the assets directory isn't present).
- `Cell::aux` for a `Creature`-kind cell is thereby freed: low 4 bits become
  `CellType` like every organism cell (`pack_cell_type`, `organism.rs:932`).
  This is the *clean* case — unlike wood there is no "hand-painted unowned
  worm" to preserve (`organism-substrate-design.md` §2).
- `ActiveKind::Creature`'s payload becomes the organism handle. Scheduling is
  otherwise unchanged: creatures tick in the serial active-site phase where
  `MAX_REACH` does not apply and ordinary `World::set` is safe
  (`scheduler.rs:12-25`).
- `CreatureState`, `World::creatures`, `push_creature`, `creature_mut` are
  deleted once the worm's nine existing tests pass on the new substrate.
  Per CLAUDE.md: deliberately break the replacement and confirm the old tests
  fail — a superseded mechanism's tests can keep passing while testing
  nothing. Any that cannot fail for the replacement get deleted, not ported.

### 3b. New cell types and organism state

`CellType` has 4 bits, 6 values used (`organism.rs:66-119`). Add:

```rust
Head = 6,      // the deciding cell; carries heading; the scheduled site
Segment = 7,   // follows the chain; no behavior of its own
```

Eggs reuse `Seed` (= 0): an egg is a `Powder`-kind material cell (it falls
and rolls, exactly like a plant seed) with a `Germinate`-shaped behavior that
hatches it. Extend `Germinate` with an optional `hatch_into: CellType`
defaulting to the current plant behavior, rather than writing a parallel
`Hatch` behavior — the relocated-seed machinery (`plant.rs:605`) that
re-finds a seed after it rolls is needed verbatim for eggs and comes free.

`OrganismState` gains (all cheap, empty/zero for plants):

```rust
/// Chain order, head first. Plants leave it empty. The cells HashMap is
/// membership; this is *sequence*, which movement needs and plants don't.
pub chain: Vec<(i32, i32)>,
/// 0..8 compass heading index for the head. See §4a — heading is discrete.
pub heading: u8,
/// What the creature is carrying (None = nothing). One item, not a stack.
pub carrying: Option<MaterialId>,
/// Ticks since last touching the nest; drives nest-scent deposit falloff.
pub since_nest: u16,
/// Hidden-unit activations, persisted so recurrence works. [f32; BRAIN_HIDDEN]
pub brain_state: [f32; 4],
/// The heritable genome. Empty vec for plants until the plant migration.
pub genome: Vec<f32>,
/// Energy. The worm's CreatureState scalar, relocated.
pub energy: f32,
```

Do **not** try to pack any of this into `Cell::aux` bits 4–15. The project
rule, learned twice at measured cost, is "the next per-cell scalar goes in
the sidecar, not into repacked bits" (`organism.rs:993-1004`,
`design-philosophy.md`). Per-*organism* state is even less appropriate in a
cell: it goes in `OrganismState`, full stop. `Cell` stays 12 bytes
(`cell_is_twelve_bytes`-style assertion at `cell.rs:495`); all 8 flag bits
are taken; touch neither.

### 3c. Chain movement

The head decides (§4); the body follows. One move, executed in the serial
active-site phase:

```text
1. target = chosen neighbour of head (may be a dig — see §6)
2. remember old positions: chain = [p0 (head), p1, ..., pk (tail)]
3. write head's cell into target (carrying its temperature, flags — see
   pitfall P-1), each segment cell moves into its predecessor's old
   position, tail's old cell becomes what the creature leaves behind
   (air for walking; air for burrowing = the tunnel)
4. update state.chain, reindex organism cell membership
```

Rules that are easy to get wrong:

- **Move the `Cell` values, not just the material ids.** Temperature,
  `FLAG_BURNING`, and `burn_timer` must travel with each cell. A burning worm
  losing `FLAG_BURNING` on move was a real shipped bug (`README.md` M18
  notes); it will come back N-fold with chains if movement rebuilds cells
  from scratch.
- **Membership maintenance has one seam.** `World::set` →
  `reindex_organism_cell` (`world.rs:1202`). Creature moves happen in the
  serial phase, so plain `World::set` per cell is correct and the parallel
  driver's queued-replay path (`world.rs:1207-1223`) is not involved — but
  only as long as nothing creature-related ever writes cells from inside the
  CA sweep. Nothing should.
- **Passability** (walking, not digging): target must satisfy
  `cell.material == material::EMPTY` — the **raw** check.
  `Cell::is_empty()` is managed-aware (`cell.rs:259`) and correct here for
  once (a promoted liquid body's air is *not* available to walk into), but be
  deliberate: use `is_empty()` and say so in a comment, because the two
  checks differ exactly when liquid bodies go live.
- **Support and falling.** After a move, if no cell of the chain has any
  `Solid | Powder | Plant` in its 8-neighbourhood, the whole chain falls one
  cell (repeat next tick). This is a **per-piece rule, evaluated on the whole
  chain** — CLAUDE.md's "which object does this rule evaluate?" question,
  answered in advance. Evaluating support per cell would take a bridge-walking
  ant apart the way the per-cell bearing rule took slabs apart. 8-neighbour
  support means ants climb walls and ceilings; that is correct and good (real
  ants do), and it is what makes the side-view world traversable at all.
- Chains start at length 1 (the worm) and 2–3 (ants). Do not build for long
  chains yet; `Vec` shuffling at these lengths is nothing.

---

## 4. The brain

### 4a. Heading, sensors, and the motor loop (Jones, discretized)

Heading is a **discrete 0..8 compass index**, not a float vector. Turning is
±1 on the index. This kills three problems at once: no `sin_cos` (a
cross-platform determinism trap named in `emergent-world-architecture.md`
§8d), rotation angle RA is exactly 45° which is the Physarum-literature
default, and all sensor offsets come from one const table:

```rust
/// Index 0 = east, counterclockwise. dx, dy per compass direction.
pub const DIRS: [(i32, i32); 8] =
    [(1,0),(1,-1),(0,-1),(-1,-1),(-1,0),(-1,1),(0,1),(1,1)];
```

Sensory stage (per tick, before deciding): sample each sensed channel at
three points — ahead, ahead-left, ahead-right:

```rust
let f  = DIRS[heading as usize];
let l  = DIRS[((heading + 1) % 8) as usize];   // SA = 45°, the default
let r  = DIRS[((heading + 7) % 8) as usize];
// sample at head + dir * SO for each of f, l, r
```

`SO` (sensor offset) starts at **6 cells** — inside the literature's 5–9 band
and comfortably above the ≥3 hard floor. Per channel the brain receives two
inputs: `front` (the f sample) and `lateral = r_sample - l_sample` (positive
= stronger to the right). This "concentration + direction hint" pairing is
the single best interface idea in the surveyed sims (Bibites' `PheroSense` +
`PheroAngle`): it makes trail-following reachable by *one* connection from
`lateral` to `turn`.

Motor stage: the brain outputs a turn bias and a move urge; movement is a
**probabilistic choice among the three forward candidates** (ahead,
ahead-left, ahead-right), never an argmax. `stigmergy-research.md` §2:
deterministic selection kills the exploration the whole mechanism depends on,
and the noise is load-bearing, not a nuisance term. Use Deneubourg's
nonlinear choice without any libm call — squaring is the nonlinearity:

```rust
/// Deneubourg choice: p_i ∝ (k + s_i)^2. k > 0 keeps exploration alive
/// when all signals are ~0; the exponent 2 is the literature's default
/// nonlinearity and needs no libm. Never replace this with min_by/max_by —
/// see stigmergy-research.md §2, which names that exact regression.
fn choose_candidate(scores: [f32; 3], k: f32, rng_draw: f32) -> usize {
    let w = scores.map(|s| { let b = k + s.max(0.0); b * b });
    let total = w[0] + w[1] + w[2];
    if total <= 0.0 { return 1; } // all blocked-ish: keep heading
    let mut t = rng_draw * total;
    for (i, wi) in w.iter().enumerate() {
        if t < *wi { return i; }
        t -= *wi;
    }
    2
}
```

If the chosen candidate cell is impassable, the move **fails**: pick a new
random heading and — critically — **deposit nothing this tick.** Blocked
agents must not reinforce, or congested dead ends accumulate trail
(`stigmergy-research.md` §6, "deposit only on successful movement," flagged
there as the omission a naive implementation makes).

### 4b. The scaffold

Fixed topology, evolvable weights. Sizes are consts, deliberately small:

```rust
pub const BRAIN_INPUTS: usize = 14;
pub const BRAIN_HIDDEN: usize = 4;
pub const BRAIN_OUTPUTS: usize = 6;
```

**Inputs** (positional; slot indices are a permanent public contract):

| # | Input | Source |
|---|---|---|
| 0 | bias (always 1.0) | — |
| 1 | pheromone A, front | §5 plane, sampled at SO |
| 2 | pheromone A, lateral (R−L) | §5 plane |
| 3 | pheromone B, front | §5 plane |
| 4 | pheromone B, lateral | §5 plane |
| 5 | moisture, front | `field_at_bilinear` |
| 6 | moisture, lateral | `field_at_bilinear` |
| 7 | light, here | `field_at_bilinear` |
| 8 | temperature above ambient, here | `field_at_bilinear`, scaled |
| 9 | food adjacent (0/1) | contact scan of head's 8-neighbours |
| 10 | at nest (0/1) | contact scan for nest material |
| 11 | energy, normalized 0..1 | `state.energy / start_energy` |
| 12 | carrying (0/1) | `state.carrying.is_some()` |
| 13 | crowding: creature cells within r=2 of head, /8 | bounded local grid read (D5 makes this legitimate; it is also the negative-feedback term §7 of the stigmergy report says a naive build omits and then ossifies without) |

**Outputs:**

| # | Output | Effect |
|---|---|---|
| 0 | turn | added to the ahead-left/-right candidate scores with opposite signs |
| 1 | move urge | P(move this tick) = clamp(0..1); failing it costs idle energy only |
| 2 | emit A | deposit rate on channel A this tick (clamped ≥ 0) |
| 3 | emit B | deposit rate on channel B |
| 4 | dig urge | gate on the dig/pickup action (§6) |
| 5 | drop urge | gate on deposit/drop action (§6) |

**Genome layout** — one flat `Vec<f32>`, four contiguous positional blocks:

```rust
// [0..84)    input→output direct   (14 × 6)   — the "taxis gains"
// [84..140)  input→hidden          (14 × 4)
// [140..144) hidden self-recurrence (4)
// [144..168) hidden→output          (4 × 6)
pub const GENOME_LEN: usize = 168;
```

**Slots are positional and must never be renumbered or reordered** — the
same law the plant genome already lives under (`organism.rs:384-394`), for
the same reason: the slot index is the meaning. Growing the scaffold later
means *appending* blocks, never inserting. Write this in a comment at the
const, in exactly these words.

**Evaluation**, once per creature tick (~200 multiply-adds worst case,
usually far fewer — see the sparsity note):

```rust
/// |w| below this is "no connection": skipped in eval AND exempt from the
/// synapse energy tax. Gives evolution a real way to delete a connection.
pub const W_EPS: f32 = 0.01;

/// Fast sigmoid: x / (1 + |x|). No libm, deterministic everywhere,
/// output in (-1, 1). Do NOT use tanh/exp here — transcendentals are the
/// named cross-platform determinism trap (emergent-world-architecture §8d).
#[inline]
fn squash(x: f32) -> f32 { x / (1.0 + x.abs()) }

pub fn eval_brain(
    g: &[f32],                       // GENOME_LEN
    inputs: &[f32; BRAIN_INPUTS],
    state: &mut [f32; BRAIN_HIDDEN], // persisted hidden activations
) -> ([f32; BRAIN_OUTPUTS], u32 /* active synapse count, for the tax */) {
    let mut active = 0u32;
    let (io, ih, hh, ho) = (&g[0..84], &g[84..140], &g[140..144], &g[144..168]);

    let mut hidden = [0.0f32; BRAIN_HIDDEN];
    for h in 0..BRAIN_HIDDEN {
        let mut sum = hh[h] * state[h]; // recurrence reads LAST tick's value
        if hh[h].abs() >= W_EPS { active += 1; }
        for i in 0..BRAIN_INPUTS {
            let w = ih[i * BRAIN_HIDDEN + h];
            if w.abs() >= W_EPS { sum += w * inputs[i]; active += 1; }
        }
        hidden[h] = squash(sum);
    }
    let mut out = [0.0f32; BRAIN_OUTPUTS];
    for o in 0..BRAIN_OUTPUTS {
        let mut sum = 0.0;
        for i in 0..BRAIN_INPUTS {
            let w = io[i * BRAIN_OUTPUTS + o];
            if w.abs() >= W_EPS { sum += w * inputs[i]; active += 1; }
        }
        for h in 0..BRAIN_HIDDEN {
            let w = ho[h * BRAIN_OUTPUTS + o];
            if w.abs() >= W_EPS { sum += w * hidden[h]; active += 1; }
        }
        out[o] = squash(sum);
    }
    *state = hidden;
    (out, active)
}
```

Update `state` *after* computing hidden from the previous state (as above) —
recurrence must read last tick's activations, or it is just a slower
feed-forward pass and the memory it exists to provide never happens.

### 4c. Authored instincts

The species `.ron` carries the full initial genome — but authored as a
sparse, human-readable wiring list, not 168 raw floats:

```ron
// ant.ron (sketch — final field names to match SpeciesDef conventions)
instincts: [
    // (input, output, weight) into the input→output block. Everything
    // not listed is 0.0 (= no connection). Hidden blocks start all-zero.
    (PheroBLateral, Turn,  0.9),   // not carrying: steer along food trail
    (Carrying,      Turn,  0.0),   // placeholder; carrying flips channels via A
    (PheroALateral, Turn,  0.6),   // carrying: steer along nest scent — see note
    (Bias,          Move,  0.7),   // baseline restlessness
    (FoodAdjacent,  Move, -0.8),   // stop on food
    (TempAboveAmb,  Turn, -0.8),   // flee heat (the worm's thermotaxis, kept)
    (Crowding,      Move, -0.3),   // negative feedback; anti-ossification
    (Carrying,      EmitB, 0.9),   // returning with food: lay food trail
    (Bias,          EmitA, 0.3),   // everyone leaks nest-scent; scaled by
                                   // since_nest falloff in code, §5c
    (FoodAdjacent,  Dig,   0.8),   // pick food up
    (AtNest,        Drop,  0.9),   // drop it at home
],
```

Two things to hold onto:

- **Generation zero must already behave.** The whole point of authored
  instincts is that the colony works on day one, exactly as the brainless
  taxis design would have, and evolution refines rather than bootstraps.
  If the authored ant does not forage convincingly, fix the instincts, not
  the mutation rate.
- A pure single-layer network cannot express "follow B when *not* carrying,
  A when carrying" as a product — that conditionality is exactly what the
  hidden layer is *for*, and it is fine for gen-zero to approximate it
  additively (both gains active; the carried state changes which channel has
  signal nearby in practice, since food trails exist where food is and nest
  scent where the nest is). If playtesting shows the approximation mills,
  the honest fallback is one **authored** hidden unit implementing the gate
  — author it in the `.ron` like any other weights, don't special-case code.

### 4d. The synapse tax

Per creature metabolic tick: `energy -= SYNAPSE_COST * active as f32`, with
`SYNAPSE_COST` starting around `0.002` against the worm's
`WORM_IDLE_COST = 0.3` scale (`creature.rs:71`) — i.e. a fully-dense brain
(~144 connections) costs about as much as standing still, a sparse authored
brain costs a tenth of that. Tune freely; the *sign* of the mechanism is the
point: connections must pay for themselves or evolution prunes them, which
is simultaneously the sparsity pressure that keeps evolved brains legible
and a real energetic trade-off (brains are metabolically expensive). The
`active` count comes back from `eval_brain` for free.

---

## 5. The pheromone substrate

### 5a. Two channels, meaning-free, CA resolution

**Two channels, named `A` and `B` in the engine, with no semantics attached.**
One channel gets milling; two get commuting (nest-scent + food-trail is the
minimal published configuration that produces actual out-and-back foraging).
Meaning lives in species instinct weights, not in the channel — that is what
lets a future second species reuse, or parasitize, the same fields.
Resist adding a third channel until a concrete consumer exists
(`stigmergy-research.md` §8's standing rule).

Default plan (pending the Stage-0 experiment): a **dedicated CA-resolution
plane per channel**, not new `FieldCell` members. `FIELD_SCALE = 8` smears a
one-cell trail into meaninglessness; trails are the one signal in the engine
that genuinely needs fine resolution (`emergent-world-architecture.md` §6b
explicitly says decide this before ants, and says don't assume 8 will do).

```rust
/// One world-sized u8 plane per channel, double-buffered for the diffuse
/// pass. 512×320 × 1 byte × 2 channels × 2 buffers ≈ 640 KB. Per-chunk
/// activity flags let settled regions skip the pass entirely.
pub struct PheromonePlane {
    w: usize, h: usize,
    front: Vec<u8>, back: Vec<u8>,
    /// max value per chunk, updated during the pass; a chunk with max 0
    /// and no deposits since the last pass is skipped ("pheromone sleep").
    chunk_max: Vec<u8>,
}
```

Known limitation, stated now: world-sized planes are the wrong shape for M10
streaming, where they will need to become per-chunk like `FieldTile`. That is
a mechanical migration; do not build it speculatively.

### 5b. The pass

Every `PHEROMONE_INTERVAL = 4` frames, over awake chunks only: 3×3 mean
filter (the standard cheap diffusion kernel) at `DIFFUSE = 0.1` blend, then
decay. Interior-of-chunk cells never read across a seam mid-pass because the
whole plane double-buffers — read `front`, write `back`, swap. This is Jacobi
shaped, like `field.rs`, so it is order-independent and trivially
deterministic; do not "optimize" it into an in-place pass.

**Decay on u8 must provably reach zero.** Multiplying a small u8 by 0.95 and
rounding can fix-point above zero and the world slowly fills with permanent
ghost trails — the quantization failure mode the canopy-density 4-bit
packing already demonstrated once in this codebase (`organism.rs:961-989`).
Use a startup-built LUT with a forced floor step:

```rust
/// decay_lut[v] < v for all v > 0 — asserted at startup, so evaporation
/// provably drains to zero. ρ ≈ 0.1 per pass to start (literature band
/// 0.1–0.5); expect to spend real tuning time here, it is the parameter
/// the whole mechanism balances on.
fn build_decay_lut(rho: f32) -> [u8; 256] {
    let mut lut = [0u8; 256];
    for v in 1..256usize {
        let d = ((v as f32) * (1.0 - rho)) as u8;
        lut[v] = d.min(v as u8 - 1); // force strict decrease
    }
    lut
}
```

Deposit: `plane[head] = plane[head].saturating_add(amount)` where `amount`
scales the brain's emit output to roughly `DEPOSIT = 40` (of 255) per
successful move — a trail a dozen ants share should sit well below
saturation, or differential reinforcement (the entire path-selection
algorithm) clips flat. If trails pin at 255, halve the deposit before
touching anything else.

Sampling for sensors: read `front` at the three sensor points with plain
nearest-cell indexing — the plane is already CA-resolution, so no
interpolation is needed. Out-of-world samples read 0.

### 5c. Nest scent without a nest query

Ants deposit channel A scaled by recency-of-nest-contact:
`emitA_effective = emitA * (1.0 - (since_nest as f32 / NEST_MEMORY) ).max(0.0)`
with `since_nest` reset to 0 on nest contact and `NEST_MEMORY ≈ 600` ticks.
Outbound ants therefore paint a nest-scent gradient that literally points
home (fresher = nearer the nest), and carrying ants follow it up-gradient.
No ant ever queries where the nest is; the field knows. This is the
two-channel commuting model from the literature, verbatim.

### 5d. Overlays and the probe, before the ants

House law (`CLAUDE.md`, "A debug readout must not be a function of the thing
it debugs"): the channels get rendered **before** anything reads them.

- Add `PheromoneA` / `PheromoneB` to `FieldOverlay` (`render.rs:170`) — a
  **full-replace fixed dark→bright ramp, never a blend into the cell's own
  colour** (the magnitude-scaled blend was tried once and read as blank).
- Add `channel=pheromone_a|pheromone_b` to `filmstrip`.
- Extend or sibling `examples/plant_probe.rs` with a creature probe that
  prints, per ant: position, heading, energy, carrying, the 14 inputs, the 6
  outputs, and active-synapse count. The moment a question about behavior
  turns quantitative, this is the tool — an overlay cannot show "input 2 was
  0.03" and one-cell-wide signals are unjudgeable by eye (both lessons
  already paid for, `CLAUDE.md`).

---

## 6. Actions: eat, dig, carry, drop

All gated on the corresponding brain output crossing 0.5, then on world
state. Every action increments a counter (§9d).

- **Eat** (food adjacent, not carrying, energy below full): convert the food
  cell to air, credit `EAT_ENERGY`. "Food" is a species-data material list
  (`food: ["corpse", "leaf", "seed"]` — herbivory against live plants works
  because a `Leaf` cell is just a cell; the plant will find out via its own
  connectivity check, which is emergent interaction nobody has to write).
- **Pick up** (dig urge, food adjacent, not carrying): remove the food cell,
  set `carrying = Some(material)`. Carrying is state, not a cell — do not
  try to make the carried grain a chain cell; it doubles every movement edge
  case for zero visible payoff at this zoom.
- **Drop** (drop urge, carrying, adjacent air cell): write the carried
  material back into the world. At the nest this is food storage; mid-route
  it is termite construction — same verb, and §7's deposition bias is what
  makes the difference emerge.
- **Dig** (dig urge, no food adjacent, blocked ahead by a diggable
  material): remove the blocking cell — gate on the material's
  `penetration_resistance` versus a species `dig_force`, exactly the pattern
  roots already use (`penetration_force`, `organism.rs:412-428`), **not** a
  material-name whitelist. Digging with nothing carried destroys the spoil
  for v1 (worms already eat tunnels); carrying spoil out is a Stage-4+
  refinement — note it, don't build it.

Termite-style construction and excavation shaping then come from **biasing,
not scripting** (`stigmergy-research.md` §4, the eLife 2024 result):
multiply drop probability by local moisture-gradient magnitude (deposit at
convex, drying sites) and dig probability by its inverse (excavate toward
concave, wetter ones). Both read `field_at_bilinear` moisture, already built.
Pillars, walls, and chambers are consequences, not features — do not write a
"build wall" behavior, and treat any urge to do so as the signal to re-read
that section.

---

## 7. Heredity: the shared genome mechanism (creature-first, plant-shaped)

### 7a. Representation and the contract with plants

One mechanism, three rules, designed so `genotype_draws: [f32; 6]` becomes a
trivial adopter later:

1. A genome is a **fixed-length `Vec<f32>` with positional slots**. For
   creatures, `GENOME_LEN = 168` brain weights (plus, later, body-plan
   scalars appended — chain length, size; append-only). For plants it will
   be their 6 draws.
2. **Per-slot mutation width** lives in species data, exactly like
   `genotype_variance` (`organism.rs:411`) — because "one width for all
   slots" was already measured wrong on plants (a third of the genome was
   dead on arrival at a flat ±15%; `organism.rs:396-409`). Brain weights
   want a default width around `0.05` absolute (not proportional — a 0.0
   weight must be able to *become* a connection, and proportional mutation
   of zero is zero forever; this is the single most important line in this
   section).
3. Mutation draws come from `rng::stream(world_seed, child_handle, GENOME_STREAM, slot)`
   — keyed by the child's identity, never by position (§2c).

```rust
/// Child = parent with per-slot additive perturbation. No crossover in v1:
/// reproduction is asexual (queens). Crossover is compatible later because
/// every genome shares one scaffold — that compatibility is the entire
/// reason topology mutation was rejected (Decision D4).
pub fn mutate_genome(
    parent: &[f32], widths: &[f32], world_seed: u64, child: OrganismHandle,
) -> Vec<f32> {
    parent.iter().enumerate().map(|(slot, &w)| {
        let u = rng::stream_unit(world_seed, child.raw(), GENOME_STREAM, slot as u64);
        (w + (u * 2.0 - 1.0) * widths[slot]).clamp(-4.0, 4.0)
    }).collect()
}
```

Clamp bounds keep `squash` inputs sane; `±4.0` is deep in its saturated
region already.

### 7b. Colony lifecycle (Stage 4)

- **Founding:** a queen is a creature species (a chain of 2–3 cells, slow,
  large) placed by brush/scene, later spawned by a mature colony. Her
  organism state holds the colony's genome.
- **Workers:** the queen lays egg cells (a `Powder` material; they fall —
  the relocated-seed machinery finds them, §3b) on an interval, paying
  `EGG_COST` energy. Workers hatch carrying the queen's genome **verbatim**
  (clonal). Mutation does not apply queen→worker.
- **Selection unit = the colony.** Mutation applies **queen→daughter-queen**:
  when a colony's stored food crosses a threshold, it produces a new queen
  egg with `mutate_genome`. This is biologically right for ants and honest
  about the clock: colony generations are slow, so brain evolution under
  D2's fantasy is *on* but *gentle*. The fast-generation creatures that will
  visibly drive evolution (grazers) inherit all of this machinery unchanged
  with a per-individual selection unit — that is the next milestone after
  this document, not this one.
- **Death:** energy ≤ 0, or killed by fire/damage → every chain cell becomes
  `corpse` (the material already exists and already burns), the organism
  slot is freed (§2b). A dead ant is *matter* — food for something, fuel
  for fire. This closes the loop that makes death legible and satisfying,
  and it costs nothing because the material system does all of it.

Worker population per colony: cap via species data (`max_workers ≈ 80`) —
enforced by the queen not laying, never by culling. Two colonies at 80 plus
strays is comfortably inside both the 4095 organism ceiling and the
scheduler budget.

---

## 8. Energy conservation census

**Evolution is a fuzzer for your conservation laws.** Every surveyed sim
that evolved anything eventually evolved exploitation of an energy-accounting
bug (Karl Sims' creatures harvesting integration error is the canonical
case). Before Stage 4 turns mutation on, build the census:

- A per-world `EnergyLedger { eaten, metabolized, moved, synapse_tax, egg_cost, died_holding }`
  — plain counters incremented at the same call sites that move energy, in
  the style of `FailureCounts`.
- Invariant test: over any run,
  `sum(current creature energy) == eaten − metabolized − moved − synapse_tax − egg_cost − died_holding`
  to f32 tolerance. Run it in the ascii scenes.
- Any imbalance is a bug **now** and an evolutionary attractor **later**.
  This test is cheap and it is the one that matters most for everything
  after it.

---

## 9. Verification: scenes, assertions, counters, sweeps

Protocol per `emergent-world-architecture.md` §10: **scene first, then the
weakest assertion that would fail if the mechanism were deleted.** Look at
every scene before trusting any number about it. Test **both drivers** —
behavior only the player sees is behavior only `parallel::step` produces —
and note creatures tick in the serial phase either way, so the driver risk
concentrates on cell writes racing the sweep, not on creature logic.

### 9a. Scenes (ascii + filmstrip)

| Scene | Stage | Watch for | Weakest assertion |
|---|---|---|---|
| `pheromone_decay` | 2 | painted blob spreads, fades, disappears | plane max is 0 after N frames; before that it decreased monotonically |
| `trail_follow` | 2 | synthetic trail; one follower tracks it | ≥70% of steps within 2 cells of the trail (this number is the Stage-0 experiment's, promoted to a guard) |
| `double_bridge` | 3 | 50+ ants, nest on a ledge, food below, two routes over/around an obstacle — **vertical geometry**, per the side-view caveat: two competing routes must come from terrain, two distances on flat ground is not a bridge experiment here | after N frames, mean channel-B on the short route > long route (this single comparison fails if deposit, decay, or following is broken) |
| `forage_loop` | 3 | ants leave nest, return carrying; trail visibly forms and tightens | `trips_completed > 0` **and** food stored at nest > 0 |
| `nest_dig` | 3 | 50 ants + diggable soil block: round chamber, then tunnels budding as it outgrows the colony (density transition, free — needs no new channel) | excavated-cell count > 0; chamber perimeter/area ratio increases over the run |
| `construction` | 3 | carriers over a moisture gradient: deposition clusters at gradient maxima | drops on the high-`|∇moisture|` half > low half |
| `colony_cycle` | 4 | queen lays, workers hatch, forage, die, slots recycle | hatch count > 0, organism slot high-water mark stable over a long run |

Expect trail formation to look **less impressive than published figures** —
those are top-down arenas; a side-view strip has fewer competing paths. That
is the geometry, not a bug (`stigmergy-research.md`, orientation caveat).
Expect a **sub-50-ant scene to look broken** with correct code.

### 9b. "Did it fire" counters

An image cannot show whether the mechanism you built produced it — a
collapse once rendered plausibly for a whole run while the body count said
the feature never executed (`CLAUDE.md`). Counters, printed next to every
scene's output: `moves, moves_blocked, deposits_a, deposits_b, eats, digs,
drops, trips_completed, hatches, deaths, falls`. `trips_completed` (nest →
food → nest with cargo) is the one that proves the *loop* rather than its
parts.

### 9c. Paired comparisons and sweeps, not single runs

- Outcomes here have enormous spread; never set a bar from one run. Judge
  changes by **paired comparison** (same seed, mechanism on vs off) and gate
  guards on an **order statistic over a seed sweep** (p90 over ≥10 seeds),
  because which seed is worst reshuffles on any legitimate change. Build the
  sweep before tuning starts, not after — the load-model work shipped two
  green-on-all-cases regressions in one session that a sweep caught in one
  command each (`CLAUDE.md`).
- Re-measure timing baselines in the same session on the same machine.
  Quote `examples/ascii.rs` worst-frame numbers in commits, with the
  before/after and what was rejected on the way (house convention).

### 9d. Frame-budget checkpoints

Measure at Stage 2 (pheromone pass on a full-world painted plane — this is
the new unconditional cost; it must sleep to ~0 on a settled world or it
violates the same law field sleeping exists for) and Stage 3 (100-ant scene,
worst frame, both drivers). The budget line: worst frame with a busy colony
stays under the existing headroom (`README.md` timing tables; ~6–9 ms
parallel worst on saturated scenes against 16.6 ms). If the pheromone pass
costs >0.5 ms on a *settled* world, stop and fix sleeping before adding
anything.

---

## 10. Pitfalls

The ones already named inline, plus the rest, grouped. Each of these is
either already-paid-for project experience or a documented failure in the
surveyed sims.

**Engine gotchas**

- P-1: moving a creature must move the whole `Cell` (temperature,
  `FLAG_BURNING`, `burn_timer`) — the burning-worm bug, chain-multiplied.
- P-2: `aux` conventions bite in both directions: `Liquid` aux 0 = full,
  `Powder` aux = soil water (despite `cell.rs` doc saying unused). Never
  write literal auxes; use the accessors.
- P-3: `Cell::is_empty()` is managed-aware. Decide raw-vs-managed per call
  site, in a comment.
- P-4: all 8 `Cell::flags` bits are taken; `Cell` is asserted 12 bytes. New
  state goes in the sidecar/`OrganismState`, never in repacked bits.
- P-5: never write cells from inside the CA sweep; creatures act only in the
  serial active-site phase. `MAX_REACH` does not bind there, but the
  population-dynamics report's warning stands: perception × mobility is a
  stability parameter, so keep SO and per-tick movement modest anyway.
- P-6: eggs are `Powder` — they *fall*. Schedule by organism and re-find via
  the cell list (relocated-seed pattern), or hatching sites go stale.
- P-7: `SpeciesRegistry::builtin()` must embed `ant.ron` or tests and
  asset-less runs get a species that silently doesn't exist.
- P-8: 4-bit generation wrap at 16 reuses; count it in debug builds.
- P-9: `cargo test` fails while the app is running (exe lock) — use
  `cargo test --lib`. Never `cargo fmt` a file (it formats the world), never
  `git add -A`, work in a worktree (`CLAUDE.md`, all three).

**Algorithm gotchas (each one is a specific published failure)**

- P-10: deterministic selection (`min_by`/argmax) anywhere in movement kills
  exploration and the mechanism collapses. The noise is load-bearing.
- P-11: deposit only on successful movement, or dead ends self-reinforce.
- P-12: no crowding/negative-feedback term → the colony ossifies on the
  first path found, and **more evaporation will not fix it** (input 13
  exists for this; check its gain is nonzero in the authored genome).
- P-13: u8 decay must strictly decrease to zero (LUT + assert) or ghost
  trails accumulate forever.
- P-14: deposit saturation at 255 flattens differential reinforcement —
  if trails pin, lower deposit first.
- P-15: sub-50 populations look broken with correct code; do not debug the
  mechanism from a 3-ant scene.
- P-16: the double-bridge scene needs *terrain-made* alternative routes in
  side view; flat ground with two distances tests nothing.
- P-17: recurrence must read last-tick activations (§4b) or memory silently
  doesn't exist while everything still "works."
- P-18: mutation width for brain weights is **absolute**, not proportional —
  proportional mutation can never grow a connection from zero, which
  quietly freezes the entire hidden layer at its authored (zero) state and
  makes D4's whole ladder vacuous. An exactly-zero delta after generations
  of evolution means suspect this first (the "zero delta is evidence about
  the condition" rule).
- P-19: no libm in the hot path or anywhere determinism-relevant: `squash`
  not `tanh`, squared-sum choice not `exp`, table headings not `sin_cos`.

**Evolution gotchas**

- P-20: degenerate attractors are the *default outcome*, not an edge case —
  spinning in place, sessile freeloading, exploit-the-energy-bug. The
  census (§8) plus per-verb counters are the instrumentation; build them
  before mutation is switched on, because afterward every anomaly is
  ambiguous between "bug" and "adaptation."
- P-21: a fitness-relevant RNG keyed on position makes location heritable
  (§2c) and manufactures fake selection results.
- P-22: judge "did evolution do anything" by paired comparison across seeds,
  never one lineage against a remembered number — outcome spread here is
  five-fold on identical genomes.

**Method gotchas**

- P-23: overlays are full-replace fixed ramps; pair every overlay with a
  probe that prints numbers. Both lessons individually cost a misdiagnosis
  already.
- P-24: when replacing the worm's mechanism (Stage 1), break the replacement
  and confirm the old tests fail; delete any that can't.
- P-25: any rule with a footing/support/failure flavor: state **which object
  it evaluates** (cell, chain, colony) before writing it. The per-cell
  bearing rule dismantling slabs is the twice-paid version of skipping this.
- P-26: if a constant resists tuning in both directions, it is probably
  counterweighting a modeling error — stop tuning and find what it hides.
- P-27: a session that changes player-visible creature behavior updates the
  relevant `wiki/` page (create `wiki/ants.md` when Stage 3 ships) in the
  same change.

---

## 11. Explicitly out of scope (named so nobody wonders)

- Topology-growing brains (NEAT proper). Re-open only when a specific
  creature's story demonstrably needs structure the scaffold can't hold;
  the reopening question is *"which on-screen behavior can this brain
  produce that the current one cannot?"*
- Crossover / sexual reproduction. The shared scaffold keeps it cheap to add.
- Predator–prey ecology (PLAN.md's cycle proposal — separate sign-off).
- Carrying spoil / true pellet logistics; multi-item inventories.
- Liquid interaction (drowning, swimming) beyond "water is impassable."
- Per-chunk pheromone planes for M10 streaming (§5a notes the migration).
- Plant adoption of the shared genome (the design contract in §7a is the
  deliverable; the migration is its own session).
- Reynolds steering / flocking — belongs to a future free-moving species,
  and per D5 may lawfully use local neighbor queries when it comes.

## 12. Open questions for the owner (fine to ship Stage 3 without answers)

1. Should ants be visible at 1 cell, or is a 2–3-cell chain the minimum
   read at play zoom? (Cheap to decide from the first filmstrip; body
   length is species data either way.)
2. Herbivory on live plants from day one, or detritus/corpse-only until the
   plant-damage interaction is watched once? (Species `food:` list either
   way; the difference is one string.)
3. Colony failure: when a queen dies, do workers persist until starvation
   (ghost colony) or wind down faster? Starvation-only is the no-code
   default and probably fine.

---

## 13. Findings from building stages 0–3 (2026-08-17)

Written from measurement, not from re-reading the plan. Everything in §§1–12
above is the design as agreed; this section is what happened when it was
built, and it contradicts the design in three places.

### 13a. What works, and is measured

| Mechanism | Evidence |
|---|---|
| Creature-on-organism substrate | worm ported, `CreatureState` deleted, slot released on death and on fire; 5 break-checks recorded |
| Pheromone planes at CA resolution | drains to exactly 0; settled pass **0.0014 ms** against the 0.5 ms gate, **0 tiles** processed |
| Overlays + probe | both channels render as legible distinct trails; `creature_probe` prints all 14 inputs / 6 outputs / synapse count |
| Free-space trail following | 0.817 of steps within 2 cells, 0.961 of the trail traversed, against a 0.050 no-trail control |
| The caged brain | authored instincts expand to a genome; recurrence verified to read last tick; sub-`W_EPS` weights neither evaluated nor taxed |
| Eat / pick up / dig / drop | all four fire; digging gated by `penetration_resistance` vs `dig_force` — 1,551 soil cells excavated, **stone floor 200/200 intact** |
| Deposition bias (§6) | drops cluster where `\|∇moisture\|` is steep: **10 vs 1** across halves measured at 2.279 vs 0.787 |
| Energy census (§8) | balances to f32 rounding over 12,000-frame runs (delta ~1.6 on ~20,000) |

### 13b. The homing half does not close, and here is why

**`forage_loop` reaches 33 pickups and 0 deliveries.** Carriers pick food
up, lay channel B, and then drift *away* from the nest — measured with the
probe: 33 ants carrying, mean position x=201, nest at x<120.

The cause is geometric, not a tuning failure. §4a specifies three sensors —
ahead, ahead-left, ahead-right — at offset `SO`, and the brain receives
`lateral = right − left`. **On a horizontal surface both lateral sensors sit
in open air.** For an ant on a floor heading east at SO=6 they sample
`(x+6, y−6)` and `(x+6, y+6)`; the trail is in the row the ant is standing
in. Measured directly: an ant standing on a cell holding `A = 27` reads
`pheroA_lr = 0.000`.

So `lateral` is **identically zero on flat ground**, for both channels, and
no gain on it can steer anything. Two consequences:

1. The Jones/Physarum sensor triad assumes agents in open 2D. This is a
   side-view world where creatures walk on surfaces, and §9a's own
   orientation caveat turns out to understate the problem: it is not that
   side view gives *fewer competing paths*, it is that the sensor geometry
   does not sample the surface the trail is on.
2. Even with a working lateral signal, the three-forward-candidate rule
   cannot express **reversal**. A carrier that needs to go back the way it
   came must accumulate four ±1 turns, and the intermediate headings point
   into the floor or into open air, where the footing rule blocks them. A
   surface creature needs a front-versus-behind comparison, which no input
   in §4b's table provides.

**Do not fix this by re-tuning.** The gated hidden units §4c prescribes were
built and *are* in `ant.ron` (the additive approximation failed exactly as
§4c predicted) — they change nothing here, because the signal they gate is
zero. Candidate next steps, unbuilt:

* Add a `PheroAlongHeading` input: pheromone ahead minus pheromone at the
  head, per channel. That is a scalar with directional meaning on a
  surface, and it makes run-and-tumble chemotaxis expressible — which is
  how bacteria solve exactly this problem without being able to steer.
* Give a failed move roll a chance to re-roll the heading (the "tumble").
  The mechanism is already half-present in the blocked path's
  viable-heading re-roll.

### 13c. `double_bridge` measured the terrain, not the colony

An intermediate build reported 21.00 mean channel-B on the short route
against 6.22 on the long one, and passed §9a's assertion. It was not
measuring path selection: with lateral steering dead, what produced it was
ants using the only route a ground-dweller can use and depositing on it.
The assertion is now a printed note. **A guard that passes for the wrong
reason is worse than no guard** — this one would have certified stigmergy
that does not exist.

### 13d. Corrections to the design's own numbers

* `DIFFUSE = 0.1` (§5b) does essentially nothing on a u8 plane — the
  blended value at distance 2 rounds to zero before it can propagate. 0.25,
  set from a sweep of tracking performance.
* `DECAY_RHO` wants to be **below** the literature band, not above it, and
  `PHEROMONE_INTERVAL` is the load-bearing knob: `build_decay_lut`'s forced
  strict decrease costs at least 1 per pass, so the longest an unreinforced
  trail can live is 255 passes. At one pass per 4 frames that is ~1,000
  frames against a ~2,200-frame round trip. Interval 12, rho 0.03.
* "Gate an action on the output crossing 0.5" (§6) cannot work as written:
  `squash(0.9) = 0.474`, so a plainly-authored instinct sits just under the
  gate and the verb never fires. Actions are probabilities, which §6's own
  "multiply drop probability by the moisture gradient" requires anyway.
* §4c's instinct list has no standing drive to dig, so an ant facing a bank
  of soil computed a dig urge of exactly zero and excavation never
  happened. `(Bias, Dig, 0.4)` added.
* "Eat if energy below full" makes every creature permanently hungry and
  deletes carrying entirely. A `hunger_fraction` (0.5) is the real
  parameter.

### 13e. Homing, built (2026-08-17, after §13b)

§13b's two candidate fixes were both built, and the mechanism works. What
is left is a different problem from the one that was diagnosed.

**The scaffold grew by two inputs**, `PheroAAlong` / `PheroBAlong` (slots
14, 15): the pheromone at the forward sensor minus the pheromone underfoot,
normalized by their sum so it is scale-free. Appended, never inserted —
every existing slot kept its index and meaning. `GENOME_LEN` 168 → 188.
Lawful only because nothing persists a genome yet; after stage 4 this
becomes a migration.

**Movement gained a tumble.** A failed move roll re-orients the creature
with probability `TUMBLE_ON_FAILED_MOVE = 0.35`, choosing uniformly among
headings that actually have footing. With `Move` driven by the
along-gradient, that is run-and-tumble chemotaxis: run while it improves,
re-orient at random when it does not. No steering is involved, which is the
point — there is nothing on a surface to steer on.

**The ant's four hidden units were repurposed** from gating *steering* (a
signal that is identically zero, §13b) to gating the *run*. Verified with
the probe, which is the only instrument that could have shown it:

```
carrier:   pheroA_along -0.750  carrying +1.000  ->  move -0.688 (clamps to 0)
empty ant: pheroA_along +0.270  carrying +0.000  ->  move +0.663
```

A laden ant pointed away from home refuses to step and tumbles until it is
pointed somewhere better; an empty one ignores the nest scent entirely and
keeps its baseline run. That is the gate working and the leak cancelled.

**Two things this cost, both worth recording.** Tumbling on *every* failed
move destroys the persistent run that finds food (33 pickups → 1); "how
often do I step" and "how often do I change my mind" are different
questions. And a symmetric gate pair only cancels exactly at equal
activation, so an ungated pair leaks: at offset 5 the leak was 0.27 and
emptied the foraging range immediately, at 12 it was 0.09 and the colony
still drifted home over 12,000 frames. 30 puts both units at -0.967 vs
-0.971 and costs the gate nothing.

**What is still not demonstrated: food discovery, not homing.** The colony
reaches ~1 pickup per run because its eastern front stalls around 200 cells
out and the food sits beyond it. A separate finding explains part of it:
**a dense line of ants gridlocks**, because a creature is neither a
foothold nor passable, so ants founded shoulder to shoulder simply stop
(27,386 blocked ticks, and a picture showing an unbroken wall of them).
Colonies need more corridor than they occupy — `found_colony` now spaces
them four apart.

Next, in order: raise the exploration range (a dispersal drive, or letting
ants pass over one another), then re-run `forage_loop` and `double_bridge`,
whose assertions are still printed notes rather than guards.

### 13f. The landscape was flat because the economy was broken (2026-08-17)

An ablation harness (`examples/ant_ablation.rs`) was built to answer one
question: **is the authored brain doing anything, or is the substrate?** It
varies only the genome, holds everything else fixed, and reports *behaviour*
rather than event totals -- because the scene counters (moves, pickups)
cannot tell a colony vibrating on the spot from one commuting, cannot tell
five busy ants from fifty, and say nothing about spatial range, which was
the thing actually failing. That had been diagnosed by reading an ascii
picture, because no number described it.

**Two of the first five metrics were measuring the initial condition.** A
colony with a genome of literally zero connections, which provably never
moves a cell, scored "range 118" and "left the nest 0.63" -- both reporting
where ants had been *placed*. Only the zero-genome control exposed it.

**With a finite corpse pile, eight of ten authored instincts produced
bit-identical behaviour.** So three constants moved into the genome as
outputs -- `Persist` (an anonymous 0.15 that decided milling versus
commuting), `Tumble`, `Caution` -- and even then only `Tumble` had leverage.

**Terrain matters, but less than it first appeared.** On a hand-built ridge
profile `Persist` swung travel 2.2x; on *generated* terrain, almost
nothing. The hand-built profile was a special case, caught only because the
generated arm was run as a check. Baseline behaviour does improve hugely on
real terrain (coverage 49 -> 1670 over control).

**The finding that reframes the milestone: the loop was never broken, the
food distribution was.** Trees regrow leaves; a corpse pile does not.
Adding "leaf" to the food list -- open question #2, answered herbivory, and
no new code at all:

| food source | foraged | pickups | deliveries |
|---|---|---|---|
| finite corpse pile | 0.05 | 2.5 | **0** |
| regrowing leaves | **0.39** | **44.8** | **28.8** |

13b's diagnosis was correct and 13e's fix necessary. The reason the loop
still would not close is that almost no ant ever found food, and a colony
cannot demonstrate a foraging loop it never enters.

**The fitness landscape now has slope.** Single weight changes move
deliveries 28-99%, and four *beat* the authored genome: `Caution=lo` 55.0,
`Tumble=lo` 41.2, `Persist=hi` 39.0, `-Bias->EmitA` 37.0, against authored
28.8. The hand-tuned ant is measurably suboptimal and simple mutations find
better -- exactly the gradient selection needs, and it did not exist a day
ago.

Two further results: `Bias->EmitA` is **net-negative in both economies** --
the nest-scent homing mechanism is paid for and not returning.
`Crowding->Move` looked inert in the flat corpse-pile world and is
load-bearing with a real economy (removing it costs 69% of deliveries).

The lesson to keep: **an ablation in a broken economy measures nothing.**
Every conclusion drawn from the corpse-pile world was wrong or
unfalsifiable.

### 13g. Scenes must run in the environment the mechanism was verified in

`forage_loop`'s `deliveries > 0` is an assertion again (measured 414 per
run). Getting there needed the scene moved onto **generated terrain**, and
two cheaper stand-ins failed first -- both of which looked obviously fine:

* **A flat floor is degenerate.** An ant on level ground has its
  up-diagonals in open air and its down-diagonal inside the floor, so it
  usually has exactly one legal step and is not deciding anything. 248
  distinct cells visited against 1,670 on generated terrain; 0.54 of ants
  ever leaving their start against 0.97.
* **A hand-built ridge profile** produced one-column cliffs; ants walked
  off them constantly (6,985 falls, 1 pickup). Smoothing every slope to
  under a cell per column still left 6,269, because ants also climb trees
  and drop out of a sparse canopy.

This is the third time a hand-built stand-in has produced a wrong answer
that a generated-terrain arm then corrected (the others: `Persist`
appearing to have leverage, and the whole flat-world ablation). **Prefer
the real generator over a controlled approximation of it**, even when the
approximation is easier to reason about -- the approximations here have not
been representative once.

Also: the scene's print window was cropping the canopy, i.e. cropping the
food out of a foraging scene. A scene that cannot be judged by eye is not
doing its job.

### 13h. Width, and the refuge that falls out of it

`BodyPlan` splits movement into two rules on the same substrate: `Chain(n)`
*follows* (the body steps into the head's old cells, which is why it flows
over any terrain and why it is exactly one cell wide -- a path has no
width), and `Rigid(offsets)` *translates* (every cell shifts by the same
offset, so it can be any shape).

**D1 rejected rotation, not width.** Translating one cell is a passability
check; rotating is the hard half -- a rotated shape does not land on the
grid cleanly. And gravity spares us it: a walking creature has a canonical
up, so it needs only facing-left and facing-right, which is a **mirror of
the authored template**.

`beetle.ron` is the first non-chain body: a 2x2 rigid block that eats ants.
Three properties, and none of them is code that knows what a beetle is:

* **It cannot enter a one-cell tunnel an ant walks through.** Purely
  because a rigid body's passability check covers every cell of it. This is
  the refuge, with no hiding logic anywhere -- and it is the property D1's
  rejection of rigid bodies was assumed to have cost us.
* **It eats ants with no predation code.** `food:` is a list of material
  names and "ant" is a material, so the existing eat verb does it. Found by
  accident: an isolation test put both in one world and the ant vanished.
* **`dig_force: 0.3`** is below soil's 0.8, so it cannot dig where an ant
  (1.0) can. A second, independent asymmetry.

Two things the tests had to be rescued from, both of which were the scene
rather than the mechanism:

* A beetle on a short floor **run-and-tumbled off the end and fell out of
  the world**. It has no pheromone instincts, so it cannot sense food at
  range; whether a predator can *find* prey is a different question from
  whether eating one works, and conflating them made the test measure the
  first while claiming the second.
* Predator and prey in one world **confounded the tunnel-geometry test** --
  the beetle ate the ant. One creature per world when the claim is
  geometric.

And a real behaviour note: a beetle near full energy **carries** its prey
rather than eating it, because carry-versus-eat branches on
`hunger_fraction`. Asserting `eats` alone made the test depend on the
predator's energy budget rather than on predation.

### 13i. Readiness check: the metrics are ready, the ecology is not

Before spending an hour sampling random genomes, three questions were asked
of the setup: is now the time, is the environment right, are the metrics
right? Two of the three were no, and finding that out cost minutes instead
of an hour.

**The metrics work.** `examples/creature_space.rs` treats *survival* as the
only outcome -- deliberately, because it is strategy-agnostic: a sessile
leaf-camper and a wide-ranging forager are judged on the same number, and
it encodes no opinion about how an ant ought to live. Everything else
(`travelled`, `commute`, `feeding`, `depth`) is a **descriptor**: what a
genome did, never how well. Diversity is behaviour-space coverage over
those four axes, which is MAP-Elites' measure and answers "how many
distinguishable ways" without ranking them against a target.

**The environment is not ready, and the reason is structural.** Four
attempts, each caught by the smoke test rather than by reasoning:

1. Abundant food, no predator -> every genome scored survival 0.91,
   including the zero genome *which cannot move*. An outcome identical for
   every behaviour is not an outcome.
2. Start energy 260 -> an idle ant outlived the run, so doing nothing was
   cheaper than living and the zero genome won outright.
3. Start energy 150 -> zero still won (0.923); the budget has to fall
   *below* the run length, not near it.
4. Start energy 90, danger moved in among the colony, food brought into
   reach -> **bit-identical output**. Neither the beetles nor the trees
   were ever part of the loop.

That last one exposed the real dynamic: **"corpse" is in the ant food list,
so a starved ant feeds the ants around it and a colony sustains itself on
its own dead without foraging at all.** Removing corpses from the menu
dropped the forager from 0.361 to 0.237 while the zero genome stayed at
exactly 0.554, which confirms it.

**And even without cannibalism, immobility wins: 0.554 against 0.237.**
Movement is a pure cost with an unreliable payoff -- at this food density
the expected return on a foraging trip is negative, so the best strategy
available is to do nothing. Sampling random genomes in this environment
would faithfully measure which genome moved least, and would have looked
like a result.

**The next experiment is therefore not the genome sweep**, it is a sweep of
the energy economy itself: `eat_energy`, `move_cost` and food density,
looking for the band where foraging pays *and* food is still scarce. That
band may be narrow and it may not exist at the current numbers; either
answer is worth having, and it is cheap. The genome sweep goes after it,
in an environment that can tell strategies apart.
