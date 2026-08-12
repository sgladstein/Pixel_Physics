# Plant substrate v2: growth mode, storage, soil moisture, leaves, and environment

**Audience:** the coding agent implementing this.
**Status:** design only, written just-in-time before implementation, per
`design-philosophy.md` §3's standing instruction and the precedent
`organism-substrate-design.md` / `tree-rewrite-design.md` both set. No code
in this pass.
**Companion to:** `Reports/organism-substrate-design.md` (the shipped
substrate), `Reports/tree-rewrite-design.md` (the shipped tree),
`research/m16-plant-biology.md` (the biology this whole system is grounded
in), `Reports/design-philosophy.md` (§2b's outcome-vs-rule boundary, which
every decision below is tested against), and `PLAN.md`'s
"Live playtest feedback: tree growth is real but tiny" section — the
owner's own stated vision, which is the actual brief this document answers.

---

## 0. A sourcing correction, stated before anything that depends on it

**`Reports/plant-simulation-research.md` does not exist on disk.** Commit
`08d33fe` ("Record plant-simulation-research.md findings alongside the
soil/leaf vision") touched `PLAN.md` and nothing else — `git show --stat`
confirms a one-file diff, and `git log --diff-filter=A` finds no commit
that ever added the file. The owner-supplied document was read in-session
and summarized into `PLAN.md`, but never written into the repository.

**Consequence for this document, stated so a future reader doesn't assume
otherwise:** every reference below to "the research document" is a
reference to `PLAN.md`'s own four-bullet summary of it, which is the only
surviving record. That summary is detailed and internally consistent, and
two of the source document's findings were independently reproduced by the
session that read it (the crowding-reads-an-always-empty-cell bug and
"finish or delete `TreeState`"), which is real evidence its other claims
are trustworthy. But this document could not verify any claim against the
original text, and does not pretend to have. **Recommended:** recover the
original from wherever the owner supplied it and commit it, before the
implementation pass, so the next document in this chain has a real primary
source rather than a summary of one.

Everything else below is verified directly against the code as it exists at
`08d33fe`, not against what the docs claim exists. Where the two diverge,
the code wins and the divergence is named.

---

## 1. The five decisions, stated first

| # | Question | Decision |
|---|---|---|
| 1 | Growth mode | Accretion for canopy (**and it was never the binding constraint**); one-cell displacement for roots into soil; **reject** a separate sub-cell turgor scalar; **add** bud break, which `PLAN.md`'s lean omits |
| 2 | `Cell::aux` ceiling | `aux` for an organism cell becomes `4 bits CellType \| 12 bits cell-slot index`; all scalars move to a per-organism `Vec<OrganismCell>` on `OrganismState`; the diffusion pass leaves the CA sweep |
| 3 | Soil moisture | Per-cell fill in a `Powder`'s currently-unused `aux`, on a four-threshold curve (saturation / field capacity / wilting point / air-filled-porosity limit); too-wet costs a `RootTip` **necrosis**, duration-gated; `mud` is a real new material at the Atterberg plastic limit |
| 4 | Leaves | A **plastochron counter** turns the *retiring* `GrowingTip` into a `Leaf`; seed reserve `2.0`, derived from the shipped economy; starvation before first leaf frees the organism id — real mortality, and it forces `free_organism` to finally exist |
| 5 | Materials & environment | `leaf` and `rootwood` become real materials (their *physics* differ); tip-vs-mature is shading only. **Debris already catches on branches** — verified in code, no new mechanic needed. Root soil-stabilization is one new check in `update_powder` |

Decision 1 is first because everything inherits from it, exactly as
`PLAN.md` frames it. Decision 2 is second because it is the only one that
is a *prerequisite* for the others rather than a peer of them.

---

## 2. Decision 1 — the growth-mode question

### 2a. `PLAN.md`'s own preliminary lean, and where it holds

`PLAN.md` records the lean as: **(i)** accept accretion for canopy,
**(ii)** small-scope displacement for roots into soil, **(iii)** a
continuous sub-cell turgor/extension scalar for growth *rate*, reusing the
liquid rewrite's fill-amount trick.

**This document accepts (i) and (ii), rejects (iii), and adds a fourth
element the lean omits.** Each in turn, with the argument the
one-paragraph lean did not have room for.

### 2b. (i) Accretion for canopy — accepted, but the diagnosis needs sharpening

The research document's load-bearing claim, per `PLAN.md`: *"even if
[`SecondaryThicken`'s pipe-ratio trigger] fired constantly, 'grow sideways
into an empty cell' is still accretion, not real thickening — a trunk
already surrounded by wood has no empty neighbour to accrete into at
all."*

**That claim is true and also does not bite, and the reason it doesn't is
worth stating precisely rather than hand-waving past.** A real tree does
not add wood *inside* the trunk. Secondary thickening happens at the
**vascular cambium** — a single cylindrical layer of dividing cells at the
*outside* of the woody core, laying xylem inward and phloem outward. The
interior of a trunk is dead heartwood; nothing divides there. So the case
the research document identifies as accretion's failure mode — an interior
cell with no empty neighbour — is a case where *real biology also adds no
cells at that location*. `plant.rs`'s `thicken()` already only writes into
`world.is_empty(nx, ny)`, i.e. it only ever succeeds on a cell at the
trunk's own surface, which is exactly where a cambium is.

Accretion is therefore a **faithful** model of cambial growth, not a
compromise version of it, for as long as the trunk's lateral faces are
open air. It stops being faithful in exactly one situation: when the
trunk's lateral face is against something solid — buried, or grown against
a rock. That is the same case as (ii), which is why (ii) is correctly
scoped to it.

**The honest cost of accepting (i), named rather than buried:** a tree
that grows flush against a stone wall cannot thicken on that side, and
will stay one cell thick there forever. Real trees do deform and split
rock. This engine will not. Accepted.

**And the actual reason the trunk is one cell thick has nothing to do with
growth mode.** `PLAN.md`'s own diagnosis is the correct one and it is a
*counting* problem: `thicken()` (plant.rs:633) counts downstream cells
whose current `CellType` is `Leaf` or `GrowingTip`, and this session's own
tip-retirement fix (plant.rs:419) converts a `GrowingTip` to `MatureBody`
the instant it grows. So the count is almost always 0–2 and never clears
`pipe_ratio: 2.5`. **Decision 4 fixes this directly** — persistent `Leaf`
cells that accumulate along the shoot give `thicken()` a real, monotonically
growing downstream load to count, which is what Shinozaki's pipe model
(`organism-substrate-design.md` §4) actually asks for. No growth-mode
change is involved at any point.

That is the rigor `PLAN.md`'s one-paragraph lean was missing: **growth mode
was never the binding constraint on trunk width, and changing it would not
have fixed the observed symptom.**

### 2c. (ii) One-cell displacement for roots into soil — accepted, with a penetration gate

Scope, exactly as `PLAN.md` frames it and no wider: `Grow`'s candidate
loop, **for a `RootTip` cell only**, gains a second growable case alongside
`world.is_empty(nx, ny)` — a `Powder`-kind neighbour whose material is
penetrable is converted to root tissue in place. **Not** a pushed column,
**not** a general piston primitive.

**Why one-cell conversion is a better model than a piston, not merely a
cheaper one.** A real root tip does not push a column of soil ahead of
itself. It sheds lubricating border cells from its cap, exerts axial
growth pressure on the order of 0.2–1.5 MPa, and the soil deforms
*plastically and locally* around the advancing tip; roots also
preferentially follow existing pores, cracks and old root channels rather
than displacing bulk soil at all. Converting one cell is closer to that
than a piston would be.

**The penetration gate, and where the number comes from.** A root cannot
grow through arbitrarily strong material, and the literature gives a
specific bound: the dry-end limit of the **least limiting water range** is
set at a penetrometer resistance of 2–3 MPa, above which root elongation
effectively stops (Silva, Kay & Perfect 1994 — see §4 for the full
citation). Engine translation, data-driven per this project's own §2a rule:
gate the displacement on the target material's existing `density`, or add a
`root_penetrable: bool` / `penetration_resistance: f32` to `Material`.
Either way, `soil` (density 1.3) yields, `gravel` (1.9) does not, and
anything `Solid`-kind never does — which preserves the already-shipped,
already-playtested behaviour that a tree planted on bare stone fails to
root, rather than silently letting roots eat through the floor.

**The simplification, named:** the displaced soil cell is *deleted*, not
relocated. Mass is not conserved. The real-world analogue is genuine —
roots occupy pore space and measurably compact the rhizosphere around
themselves — but at one cell per pixel there is nowhere to put the
compaction, so the cell is consumed. An alternative (relocate the soil to
the nearest empty cell within a short reach, delete only if there is none)
is available and is strictly more faithful; it is **not** recommended for
this pass, because it makes root growth a *two-cell* write with its own
failure mode, and roots are the part of the system with the least live
verification behind it. Revisit if soil visibly disappearing under a mature
tree reads badly.

### 2d. (iii) A sub-cell turgor/extension scalar — rejected

**This is where this document disagrees with `PLAN.md`'s lean.**

The proposal is to give a growing cell a continuous extension accumulator
that fills over several ticks and promotes to a whole cell on saturation,
reusing the compressible-liquid fill trick (`update.rs`'s module doc;
`material::LIQUID_FULL`) — sub-pixel growth *rate* without displacement.

**The engine already has that accumulator, and it is the resource scalar.**
`Grow`'s gate is `if resource < cost { continue; }` (plant.rs:310). A cell
that cannot afford a step does not die; it accumulates resource from
`Photosynthesize`/`Absorb`/diffusion across successive ticks and grows when
it can. That *is* a continuous sub-cell growth rate, integrated over time,
denominated in the one currency the entire economy already uses. A turgor
scalar would be a second accumulator integrating the same thing under a
different name, and `Grow` would then have two independent gates that can
disagree — a cell rich in resource but low in turgor, or the reverse, with
no mechanism that makes the distinction mean anything.

**Why the liquid-fill analogy does not transfer.** A liquid's fill amount
earns its place because it is a *conserved, transferable* quantity: the
whole point is that neighbouring cells equalize against it, and volume in
equals volume out. That is what makes `transfer_liquid_horizontal`'s
half-the-difference rule a physical process rather than bookkeeping.
Growth extension is neither conserved nor transferred — it is a private
counter that only ever goes up and then resets. The trick's actual load-
bearing property is absent.

**Where the liquid-fill trick genuinely does belong, and it is not
here:** **soil moisture** (Decision 3) is conserved, is transferable
between neighbouring cells, drains under gravity, and needs a per-cell
continuous quantity in a slot that `Cell::aux` currently leaves empty for
`Powder` — every property that makes the fill trick right for liquids. The
lean identified the right tool and pointed it at the wrong problem.
Decision 3 points it at the right one.

### 2e. The fourth element `PLAN.md`'s lean omits: bud break

`PLAN.md`'s own diagnosis, in its own words: *"nothing ever creates a new
independent frontier once every existing lineage has either dead-ended...
or run its course... growth is over for that organism, forever, regardless
of remaining light or space — there is no mechanism (epicormic budding, or
anything like it) for a mature tree to issue a new shoot later. **This is
the actual ceiling on total size**, not the resource economy just
retuned."*

That diagnosis is correct, and **no combination of (i), (ii) and (iii)
addresses it.** A growth-mode change alters how a frontier advances; it
cannot create a frontier that no longer exists. The lean, as recorded, would
have been implemented in full and left the headline symptom — "a tiny tree,
~18 cells, growth stops for good" — exactly where it is. This document
therefore treats bud break as **part of Decision 1**, not a separate later
item.

**Mechanism, grounded not invented.** Latent and epicormic buds are real,
standard tree biology, and this project's own research file already cites
them: `research/m16-plant-biology.md` §5 records that *"fire-adapted trees
resprout epicormically from protected buds using stored reserves."* The
same file's §3–4 gives the controlling mechanism in detail — apical auxin
flowing basipetally suppresses axillary buds; remove or weaken the apical
source and buds nearest the disturbance activate first (Prusinkiewicz et
al. 2009, PNAS 106:17431-17436, already cited in `plant.rs`'s own module
doc).

**Engine translation, deliberately the weak version.** Full canalization
is out of scope for the same reason `tree-rewrite-design.md` §3 already
gives, and this document does not reopen it. What is in scope is the
observable consequence: a `MatureBody` cell that has *surplus* resource and
*low* local canopy density has, by construction, nothing downstream
consuming what it is being fed and open space beside it. Give `MatureBody`
a `BudBreak { resource_threshold, crowding_threshold, chance }` behavior
that converts the cell back to `GrowingTip` under exactly those two local
conditions.

This is a simple local rule producing an emergent outcome, which is what
`design-philosophy.md` §2b explicitly permits. It is also **self-limiting
without a cap**: a new tip immediately starts spending resource and
depositing canopy density, which raises local crowding and drops local
surplus, closing the condition behind it. And it reproduces the real
observable — a tree that loses a limb re-sprouts near the wound, because
that is precisely where downstream demand vanished and resource backs up.
Nothing about that outcome is authored; it falls out of the two thresholds.

**Honest limitation:** without canalization this will not produce a single
clearly dominant leader, and this document claims no such thing — the same
walk-back `tree-rewrite-design.md` §3 already made for apical dominance
applies here unchanged.

---

## 3. Decision 2 — `Cell::aux` → sidecar storage

### 3a. The current layout, verified against code

`organism.rs`'s actual functions, not the docs' description of them:

- `pack_aux` (organism.rs:402) — `CellType` in bits 0–3, resource as a
  `u8` fixed-point on `RESOURCE_SCALE = 4.0` in bits 4–11.
- `with_canopy_density` (organism.rs:455) — bits 12–15, 4 bits, 16 levels
  on `CANOPY_DENSITY_SCALE = 4.0`.

**16 of 16 bits used. Confirmed full.** `tree-rewrite-design.md` §6 left
the placement of canopy density as its one open question and resolved it
into the last spare 4 bits; there is no spare left.

What this phase still needs and cannot fit: a second resource currency
(carbon vs. water/nitrogen — collapsing them removes the allocation
trade-off entirely), organ age (Decision 4's leaf lifespan), an anoxia
duration counter (Decision 3), a plastochron counter (Decision 4). Four new
scalars into zero bits.

Canopy density's own 4 bits are already a live problem, not a hypothetical:
`plant.rs`'s `CANOPY_DENSITY_DECAY_PER_TICK` doc records that a previous
decay rate had to be raised to `0.5` specifically to clear
`with_canopy_density`'s quantization half-step of `≈0.133` on every
application, because a smaller rate got quantization-locked. That is a
constant tuned around a storage limitation rather than around the
behaviour it controls — exactly the tail wagging the dog.

### 3b. Is `CreatureState` the precedent? Partly — and it is not the one to copy

`creature.rs`'s `CreatureState { energy: f32 }` lives in `World::creatures:
Vec<CreatureState>`, indexed by a `u16` stored directly in `Cell::aux`
(creature.rs:138). It is a genuine working precedent for *side storage
keyed off a cell field*, and it proves the pattern runs in this engine.

**But it is per-entity, not per-cell, and the worm is one cell.** It says
nothing about how a multi-cell organism stores per-cell data. It also has
no generational check, so a stale index silently reads another creature's
state — a bug shape `world.rs` already fixed on the organism side.

**The precedent to copy is `World::organisms`, which is better and is
already built.** `OrganismSlot { generation, state: Option<OrganismState> }`
in a stable-index `Vec`, addressed by `Cell::organism_id`'s 12-bit index +
4-bit generation (`encode_organism_id`/`decode_organism_id`, world.rs:44).
Generational, tested (`organism_ids_round_trip_and_encode_a_nonzero_
generation`, `organism_id_zero_is_always_none`), and already the addressing
scheme every organism-owned cell uses. This design adds one layer *under*
it and invents nothing new.

### 3c. The decision

**For an organism-owned cell (`organism_id != 0`), `Cell::aux` becomes:**

```
bits 0-3    CellType          (unchanged, same encoding, same helpers)
bits 4-15   cell slot index   (12 bits -> 4095 cells per organism)
```

**And `OrganismState` gains a per-cell table:**

```
pub struct OrganismState {
    pub species: SpeciesId,
    cells: Vec<Option<OrganismCell>>,   // indexed by the cell slot above
    free_cell_slots: Vec<u16>,          // same free-list shape World::organisms already uses
}

pub struct OrganismCell {
    pos: (i32, i32),
    carbon: f32,            // was the packed resource scalar
    water: f32,             // new -- the second currency
    canopy_density: f32,    // was the packed 4 bits
    age: u16,               // new -- leaf lifespan (Decision 4)
    anoxia_ticks: u8,       // new -- waterlogging duration (Decision 3)
    plastochron: u8,        // new -- leaf placement (Decision 4)
}
```

Plain `f32`s. No packing, no scale constants, no quantization, and no
ceiling ever again. `RESOURCE_SCALE` and `CANOPY_DENSITY_SCALE` survive
only as *clamps* if a clamp is still wanted behaviourally, not as encoding
parameters.

**What stays on `Cell`, and why — this project's own convention applied
consistently.** `Cell::organism_id`'s doc states the rule: the meaning
lives with the caller, and the cell carries only what a caller holding a
bare `Cell` must be able to answer. Three call sites need to answer "what
kind of cell is this" from a `Cell` alone, sometimes with no `&World` in
hand:

- `diffuse`'s wall test (`n.organism_id() != organism_id || kind != Plant`),
- `structural::tick`'s branch (structural.rs:82) and `organism_is_
  supported`'s traversal filter (structural.rs:225),
- `World::organism_active_tip_count`'s filter (world.rs:225).

So **`CellType` stays inline** and every one of those keeps its current
shape. Everything that is a *scalar* moves out. `organism_id` stays
untouched.

**For an inert cell (`organism_id == 0`), `aux` is completely
unchanged** — anchor distance for `Solid`/`Plant`, liquid fill for
`Liquid`, and (new, Decision 3) soil moisture for `Powder`.
`structural.rs`'s incremental relaxation and `update.rs`'s `liquid_fill`
are not touched by any of this. That is the property that keeps the
migration's blast radius small.

### 3d. The hard part, named rather than discovered later: `diffuse_resource` cannot follow

`organism::diffuse_resource` is generic over `CellSurface` (organism.rs:547)
because it runs from `update.rs`'s CA sweep (update.rs:130), which the M5
parallel driver executes through `ChunkView`. `organism.rs`'s own module
doc already explains why `TransportChannel` was cut from the last pass:
*"making its decay rate genuinely per-species would need `CellSurface`...
to expose species lookups, which today it deliberately doesn't."*

**A per-organism `Vec` living on `World` hits that exact wall.** Moving the
resource scalar off `Cell::aux` means `diffuse_resource` can no longer read
it through `CellSurface`, and the parallel sweep stops diffusing entirely.
This is the single largest cost of the migration and it must be resolved
before step one, not during it.

Three options considered:

1. **Extend `CellSurface` with organism-data access.** Rejected, for the
   reason `organism.rs` already gives for `TransportChannel` — `ChunkView`
   is deliberately shaped around what `update.rs`'s CA rules need, and
   organism state is a new, nontrivial surface neither implementer carries.
2. **Split-brain: keep resource on `aux`, put only the new scalars in the
   sidecar.** Rejected. Every write site would have to know which half of
   the layout each field lives in, and the bit budget stays full, so the
   *next* scalar reopens the whole question. This is precisely the
   "building on a foundation about to need restructuring anyway" the
   research document warned against.
3. **Move diffusion off the CA sweep entirely, into a per-organism pass
   over `OrganismState::cells`.** **Chosen.**

**Why (3) is an improvement and not merely the least-bad option.**
`OrganismCell` already carries `pos`, so the pass iterates a contiguous
`Vec` of an organism's own cells, reads each cell's four `Plant`
neighbours from the grid, and writes back — no scheduler involvement, no
active site required, no `CellSurface` genericity, and cache-friendly in a
way the current scattered per-cell CA visit is not. It also *removes* work:
today `diffuse_resource` runs on every organism cell of every awake chunk
**every single frame**, which is far more often than any consumer reads
it (`ORGANISM_TICK_INTERVAL` is 45). The new pass runs at whatever cadence
the behaviour actually needs.

It also preserves the property that motivated the current placement
verbatim: `update.rs`'s comment says the CA sweep was chosen because *"a
`MatureBody` trunk cell needs to keep relaying resource even though it is
deliberately off the active-site schedule."* Iterating the organism's own
cell list keeps mature cells relaying resource **while still being off the
schedule** — which is a strictly better answer to that requirement than the
workaround it replaces.

### 3e. Four existing mechanisms this migration fixes for free

Worth stating explicitly, because they change the cost/benefit of doing
this now rather than later:

- **`World::organism_active_tip_count`** (world.rs:219) is currently a
  linear scan of the *entire* active-site heap, with its own doc
  apologizing for it. It becomes a count over one organism's own cell list.
- **`organism_is_supported`** (structural.rs:203) currently BFSes outward
  from the cell under test because, in its own words, *"`OrganismState`
  still doesn't track [an anchor list] — real future work, not faked here."*
  It now can: `RootTip` positions are directly enumerable from `cells`.
- **`free_organism` / issue #8.** `world.rs`'s own comment says the missing
  half of issue #8's fix was deferred because *"detecting 'this organism
  has no cells left' cheaply needs a real anchor/tip list to search from or
  a live cell count — both real, deliberately deferred work for the tree
  retrofit."* A live cell count is now `cells.iter().flatten().count()`.
  Decision 4 needs `free_organism` anyway (seedling mortality), so this is
  the pass where it lands.
- **`MAX_THICKEN_SCAN_CELLS = 2000`** (plant.rs:609) can be bounded by the
  organism's own cell count instead of a magic number. *Caveat, honestly:*
  `thicken()`'s flood fill must **stay** a flood fill — "downstream" is
  load-bearing for the pipe model and a whole-organism leaf count is not
  the same quantity. Only the cap changes.

### 3f. Migration plan that does not break what is already tested

Four steps, each independently committable and each leaving the test suite
green. The named tests are every current reader of the organism `aux`
encoding; `grep` confirms the surface is small (`organism.rs`,
`plant.rs`, `world.rs:225`, `structural.rs:647` — nothing else).

**Step 2a — additive only.** Add `OrganismState::cells` / `free_cell_slots`
/ `OrganismCell` with `pos` only. Register a slot on every organism-cell
creation (`germinate`, `Grow`'s two child writes, `Divide`'s child write,
`thicken`'s write, `plant_moss_seed`, `plant_tree_species`) and release it
on every removal (`break_free`, `fire`'s burnout, brush erase). **Read
nothing from it. Change no existing behaviour.** Add one test asserting the
list agrees with a full grid scan of that organism's cells. Every existing
test passes unchanged, because nothing existing consults the new structure.

*This step is where the real bugs are.* Cell removal happens in several
places that currently have no idea organisms exist (`structural::break_
free`, `fire::transform`, `World::paint_*`). A leaked slot here is
harmless (a stale `Option<OrganismCell>` with a position nothing points
at); a *reused* slot pointing at a live cell is not. Prefer leaking to
double-freeing, and make the grid-scan agreement test the gate.

**Step 2b — move canopy density.** It is the newest, coarsest and least
depended-upon channel, and it has the known quantization problem, so it is
the cheapest thing to move first and the one that most obviously improves.
`canopy_density`/`with_canopy_density` become accessors on the sidecar;
`pack_aux_preserving_density` (plant.rs:116) — which exists *only* to work
around bits 12–15 being clobbered by `pack_aux` — **is deleted outright**,
along with the whole class of bug its doc describes. Bits 12–15 free up.
`canopy_density_round_trips_and_leaves_cell_type_and_resource_untouched`
and `a_freshly_packed_aux_has_zero_canopy_density` become sidecar
round-trip tests with the same assertions.
`diffuse_resource_no_longer_decays_density_itself` moves to the new pass.

**Step 2c — move resource, and move the diffusion pass.** The one big step.
`pack_aux`/`unpack_aux` shrink to `CellType` + slot index and remain the
*only* aux accessors, so the layout change is confined to those two
functions plus every `plant.rs` write site that currently threads
`resource` through them. `RESOURCE_SCALE`'s round-trip tests
(`pack_and_unpack_aux_round_trip`, `resource_is_clamped_into_range_rather_
than_wrapping`) become sidecar tests — keep the clamp assertion if the
clamp is kept behaviourally, delete it honestly if it is not.
`resource_diffuses_from_a_full_cell_toward_an_empty_same_organism_
neighbour` and `resource_does_not_cross_an_organism_boundary` move to the
new per-organism pass and keep their exact assertions; the organism-
boundary test is the important one and must not be weakened, since the new
pass iterates one organism at a time and could pass it vacuously — rewrite
it so a second organism's cell is a *4-neighbour* of the first and assert
the wall still holds.

`an_unrecognized_type_bit_pattern_is_none` survives unchanged (bit pattern
5 is still not a valid `CellType`). `structural.rs:647`'s test helper needs
one added line to register its hand-built cell in the organism's cell list.

**Step 2d — add the new scalars.** `water`, `age`, `anoxia_ticks`,
`plastochron`. Free. No layout change now or ever again. Decisions 3 and 4
unblock here and only here.

**Explicitly rejected: a global `HashMap<(i32,i32), CellData>` sidecar.**
It puts a hash lookup on the diffusion path; positions are not stable under
a world that streams chunks (`World::chunks: HashMap<ChunkCoord, Chunk>`);
and an overwritten cell leaves an entry nobody owns or reclaims — issue
#8's leak shape, reintroduced at cell granularity instead of organism
granularity. The per-organism `Vec` has an obvious owner for every entry,
which is the whole reason it is the right structure.

---

## 4. Decision 3 — soil moisture, grounded in real soil physics

This is the one area with no prior coverage in any existing project
document, so it is researched from primary sources here.

### 4a. Where the value lives

**In the soil cell's own `Cell::aux`.** `cell.rs`'s field doc states the
current allocation: *"`Powder` / `Gas` → unused, always 0."* That slot is
documented-free, and this is exactly the compressible-fill idiom the water
rewrite already proved in this engine (`update.rs`'s module doc;
`material::LIQUID_FULL = 1000`).

**One convention inversion that must be written down, because getting it
backwards is precisely the bug `LIQUID_FULL`'s own doc exists to prevent:**
for a `Liquid`, `aux == 0` means *full*. For soil, **`aux == 0` means
dry.** Worldgen and brush-painted soil should start dry; the ash→soil decay
path (`decay.rs`) sets a fresh soil cell's moisture from the local field
reading at the moment it decays, which is a one-line addition at
decay.rs's `world.set(x, y, Cell::new(soil_id, shade))`.

This lives on the *inert* side of Decision 2's split (`organism_id == 0`),
so it composes with the sidecar migration without interacting with it at
all.

### 4b. The real soil-water curve, and the four thresholds that come off it

Soil water is not a single "wetness" number in the science; it is a
position on a retention curve with named, standard breakpoints:

- **Saturation** — every pore filled with water, zero air.
- **Field capacity (FC)** — the water held against gravity after free
  drainage, conventionally the water content at a matric potential of
  **−33 kPa** (−1/3 bar).
- **Permanent wilting point (PWP)** — **−1500 kPa**, the potential at
  which most plants can no longer extract water at all.
- **Plant available water (PAW)** = FC − PWP. This is the band, and the
  only band, from which a plant actually drinks.

([METER Group, "Plant available water: how do I determine field capacity
and permanent wilting point?"](https://metergroup.com/measurement-insights/plant-available-water-how-do-i-determine-field-capacity-and-permanent-wilting-point/);
[SDSU Extension, "How Soil Holds Water"](https://extension.sdstate.edu/how-soil-holds-water);
[ScienceDirect topic: Permanent Wilting Point](https://www.sciencedirect.com/topics/agricultural-and-biological-sciences/permanent-wilting-point).)

The wet end has a *fourth* threshold that is not on the retention curve at
all, and it is the one that answers "why is too much water bad":

- **Minimum air-filled porosity**, conventionally **10%** by volume, below
  which soil oxygen diffusion effectively stops. Originating from **Grable
  & Siemer (1968), "Effects of Bulk Density, Aggregate Size, and Soil Water
  Suction on Oxygen Diffusion, Redox Potentials, and Elongation of Corn
  Roots," Soil Science Society of America Proceedings 32:180-186**
  ([Semantic Scholar record](https://www.semanticscholar.org/paper/Effects-of-Bulk-Density,-Aggregate-Size,-and-Soil-1-Grable-Siemer/80220346787e4719bc55d9fd2f36ec2bc0b9a93a)).
  *Their own caveat, worth carrying:* Grable & Siemer concluded 12–15%
  would be a safer limit and stated that no single value is optimal for all
  situations — species optima range roughly 6–10% (sorghum) to 15–20%
  (barley, beet). **Implementation consequence, exactly parallel to
  `organism-substrate-design.md` §4's treatment of `pipe_ratio`:** the
  aeration threshold is a **per-species parameter**, not a universal
  constant, and no code should assume one number spans species.

**All four thresholds are unified by one existing framework, which is the
right thing to implement against rather than four ad-hoc rules: the least
limiting water range (LLWR)** — Da Silva, A.P., Kay, B.D. & Perfect, E.
(1994), *"Characterization of the Least Limiting Water Range of Soils,"*
Soil Science Society of America Journal 58:1775-1781
([SSSAJ record](https://acsess.onlinelibrary.wiley.com/doi/abs/10.2136/sssaj1994.03615995005800060028x)),
refining Letey, J. (1985), *"Relationship between soil physical properties
and crop production,"* Advances in Soil Science 1:277-294. LLWR defines the
water-content band in which limitations from matric potential, aeration and
mechanical resistance are all minimal:

```
upper (wet) bound = min{ water content at 10% air-filled porosity, field capacity }
lower (dry) bound = max{ water content at 2-3 MPa penetration resistance, PWP }
```

([Wikipedia: Nonlimiting water range](https://en.wikipedia.org/wiki/Nonlimiting_water_range), which carries both primary citations.)

**This is the model.** Root growth rate is unimpeded inside the LLWR and
falls to zero at both bounds — a two-sided band with a real name, four real
breakpoints and a 30-year literature, instead of an invented "too wet is
bad" penalty. It also hands Decision 1(ii)'s penetration gate (§2c) the
same numbers, from the same framework, for free.

### 4c. What "too much moisture" actually costs a root: necrosis, not a soft penalty

The task asks for one of *reduced absorb efficiency*, *slowed growth*, or
*literal necrosis*, grounded in the real mechanism. **The mechanism is
oxygen starvation, and the literature is unambiguous that its signature is
threshold-then-death, not graded inefficiency.**

**The physical cause.** Waterlogging fills the pore space, and *"the oxygen
diffusion rate in water is only 1/10,000 of that in air"* — Pan, J.,
Sharif, R., Xu, X. & Chen, X. (2021), *"Mechanisms of Waterlogging
Tolerance in Plants: Research Progress and Prospects,"* Frontiers in Plant
Science 11:627331, [doi:10.3389/fpls.2020.627331](https://doi.org/10.3389/fpls.2020.627331)
([PMC7902513](https://pmc.ncbi.nlm.nih.gov/articles/PMC7902513/)). The same
review: gas exchange between soil and atmosphere is blocked, *"resulting in
suppressed root respiration, decreased root activity, and energy
shortage"* — oxygen deficit rapidly halts ATP production by interrupting
the mitochondrial electron transport chain.

**Why this is fatal to the tip specifically, which is the load-bearing
detail.** Root *tip* cells placed into anoxia without acclimation die
within a few hours; hypoxia-induced (ferroptosis-like) cell death in barley
root tips has been measured triggering within **1–2 hours** at moderately
elevated temperature ([ScienceDirect S0098847225002230](https://www.sciencedirect.com/science/article/pii/S0098847225002230)).
Mature root cortex behaves completely differently: under *hypoxia* it forms
**aerenchyma** — gas-conducting lacunae built by programmed cell death,
which is an adaptive response, not damage — whereas under true *anoxia*
aerenchyma formation is arrested and the tissue dies by necrosis
([Evans 2004, "Aerenchyma formation," New Phytologist 161:35-49](https://nph.onlinelibrary.wiley.com/doi/10.1046/j.1469-8137.2003.00907.x);
[Drew et al., programmed cell death and aerenchyma formation in roots, PubMed 10707078](https://pubmed.ncbi.nlm.nih.gov/10707078/)).

**Decision: the `RootTip` cell necroses. The `MatureBody` root behind it
does not.**

Concretely: a `RootTip` sitting in soil above the aeration threshold
increments `anoxia_ticks` (Decision 2's sidecar); below the threshold the
counter decays to zero. Crossing `ANOXIA_LIMIT`, the cell loses its
`organism_id` and becomes inert `deadwood` — dropping off the schedule via
`organism_tick`'s existing `cell.organism_id() != organism_id` guard, which
needs no new code path at all. Everything upstream survives untouched.

**Why duration-gated rather than instantaneous:** every measurement above
is a time-to-death, and the literature explicitly distinguishes survivable
*transient* waterlogging from lethal *sustained* waterlogging. A counter
that accumulates and decays is the faithful translation, and it is the same
idiom `ORGANISM_STALE_LIMIT` already uses in this file, so it introduces no
new pattern.

**Why the two softer options are rejected, explicitly.** *Reduced absorb
efficiency* has the mechanism backwards — water is not the scarce resource
in waterlogged soil, oxygen is, and a root surrounded by water that drinks
*less* is a rule with no physical story behind it. *Slowed growth* is
closer (energy shortage does slow growth) but it makes waterlogging
indistinguishable from mild drought at the observable level, which throws
away the one asymmetry that makes the mechanic interesting.

**And necrosis produces a genuinely emergent outcome the other two do
not.** Roots grow down the moisture gradient toward water; the ones that
reach saturated ground die back; the survivors are the ones that stopped
just short. The root system stabilizes at the capillary fringe — **which is
where real root systems actually stop.** That shape is a side effect of two
local rules (grow toward moisture; die in anoxia), not a curve fitted to
produce it, which is exactly the test `design-philosophy.md` §2b sets.

### 4d. `Absorb` and `Grow`, reading and depleting the per-cell value

`Absorb` (plant.rs:495) currently has one path: drain adjacent `Liquid`
cells. It gains a second, and `Grow` supplies a third:

1. **Drink-in-place from adjacent liquid** — unchanged from today.
2. **Drink from adjacent soil** *(new)* — credit `rate × paw_fraction`,
   decrement that soil cell's own `aux`, where
   `paw_fraction = clamp((moisture − PWP) / (FC − PWP), 0, 1)`. **Below
   PWP the fraction is exactly zero**, which is the whole point of the
   wilting-point threshold and what makes drought a real, terminal failure
   rather than a slow one.
3. **Absorb-on-displacement** *(new, from Decision 1(ii))* — a `RootTip`
   converting a soil cell to root tissue credits that cell's remaining
   water on the way through. Growing through soil *is* drinking.

**Path 2 is the direct fix for a gap `PLAN.md` already recorded and
could not close.** From the step-7 entry: *"`RootTip` has no income source
of its own besides `Absorb` (which only pays off once already touching
water) — a root with no adjacent water lives entirely off resource slowly
diffusing over from the trunk, and can permanently go dormant... well
before ever reaching a water pocket even a few cells away."* Confirmed
there at both 1,500 and 6,000 ticks: a permanent stall, not a timing
issue. A root embedded in ordinary damp soil now has continuous income
proportional to how damp that soil is. **That is the actual fix for the
stall, and it is a mechanism rather than a re-tune** — worth stating,
because `PLAN.md` proposed a `RootTip` cost/rate tuning pass as the
candidate remedy, and tuning cannot fix an income source that does not
exist.

**Gravity drainage, one extra rule, nearly free.** A soil cell above FC
transfers its excess downward into the soil cell below, capped at that
cell's remaining room to saturation, on the soil cell's own check cadence.
Excess at the bottom of a soil column with nowhere to go, or above
saturation with no room below, sheds a real `water` cell into an adjacent
empty cell. This is `transfer_liquid_vertical`'s exact shape
(update.rs:230) applied to a different scale constant, and it produces a
**wetting front** — rain or a burst pipe soaking downward through soil over
time — for essentially no new machinery.

**And it closes a real loop** (`design-philosophy.md` §0: behaviour count
scales with loops, not systems). Today the moisture channel is a coarse
field forced to `MAX_MOISTURE` wherever a `Liquid` cell sits
(`field.rs`'s `apply_moisture_sources`) and read by moss, roots and ash
decay. It has exactly one source and it is "standing water is here." With
per-cell soil moisture: liquid infiltrates into soil → soil holds and
drains it → roots drink it and deplete it → depleted soil re-reads as dry
→ moss and ash decay both notice. Deposit → diffuse → decay → follow, on a
channel that currently only does the first step.

### 4e. Mud: a real material, at a real threshold

**Decision: a new material, `mud`, produced by a moisture-triggered
transition from `soil`, following `decay.rs`'s ash→soil template exactly.
Not a `Cell` flag, not a render tint.**

The test is this engine's own, from `material.rs`: behaviour comes from
`kind` plus numeric parameters. Mud's *behaviour* genuinely differs from
soil's, so it is a material:

- **`friction_angle` much lower** than `soil.ron`'s `33.0`. Wet granular
  material slumps at a shallower angle of repose, and `roll_along_slope`
  (update.rs:161) already turns that number into visible behaviour with no
  new code.
- **`density` slightly higher** — water-filled pores.
- **Darker palette** — which is what wet soil looks like, for free.
- **Lower `root_penetrable` resistance** — a root advances more easily
  through mud than through dry compacted soil, consistent with §2c's LLWR
  penetration bound, which is itself water-content dependent.

**The threshold, grounded: the Atterberg limits.** These are *the* standard
framework for "at what water content does soil stop behaving like a solid,"
originating with Albert Atterberg (1911) and standardized as ASTM D4318.
Soil passes through four consistency states as water content rises —
solid → semi-solid → **plastic** (above the *plastic limit*) → **liquid**
(above the *liquid limit*, defined as the minimum water content at which
soil flows under a very small shear force)
([Wikipedia: Atterberg limits](https://en.wikipedia.org/wiki/Atterberg_limits);
[ASTM D4318](https://store.astm.org/d4318-17e01.html);
[Geoengineer.org, Atterberg Limits](https://www.geoengineer.org/education/laboratory-testing/atterberg-limits)).

Three states map onto three engine states with no invention:

| Water content | State | Engine |
|---|---|---|
| below plastic limit | semi-solid, crumbles | `soil` — `friction_angle: 33.0` |
| plastic limit → liquid limit | plastic, moulds without cracking | `mud` — low `friction_angle`, `Powder` |
| above liquid limit | flows under small shear | sheds free `water` (§4d's drainage) |

**Simplification, named honestly:** Atterberg limits are gravimetric water
contents of *fine-grained* soil measured by a standardized test; the
engine's `aux` value is a volumetric pore-filling fraction. They are not
the same measurement and the engine collapses them onto one 0..1 wetness
scale. The *ordering* and the *three-state structure* are faithful; the
numeric limits are calibration targets, not conversions. Also: real soil
above the liquid limit flows as a mudslide; this engine gives you `mud`
plus free water instead. A `Liquid`-kind `mud` variant is a legitimate
future extension and is deliberately not attempted here.

---

## 5. Decision 4 — real `Leaf` cells, seed reserve, leaf-gated photosynthesis

### 5a. What triggers `Grow` to produce a `Leaf`

**Decision: a plastochron counter on the growing tip. Every `N`-th
successful growth step, the *retiring parent* becomes a `Leaf` instead of a
`MatureBody`.**

**The mechanism is real and named.** The **plastochron** is the time
interval between the initiation of successive leaf primordia at the shoot
apical meristem — the standard botanical term for the periodicity that
places leaves along a shoot
([ScienceDirect topic: Plastochron](https://www.sciencedirect.com/topics/agricultural-and-biological-sciences/plastochron);
[Meicenheimer 2014, "The plastochron index: still useful after nearly six
decades," Am. J. Bot. 101:1821-1835](https://bsapubs.onlinelibrary.wiley.com/doi/10.3732/ajb.1400305)).

**It is also a mechanism this project already committed to and never
built.** `research/m16-plant-biology.md` §2 recommends exactly this shape
for lateral root priming, over a flat probability: *"Instead of a flat
per-tick branch probability, run a simple oscillator counter on each
growing root tip: every N growth-ticks, mark the current node as a 'primed'
site... This gives naturally regular spacing 'for free'"* — grounded in
Moreno-Risueno et al. (2010)'s oscillating auxin-response priming, cited
there. Adopting the same counter for leaf placement makes it **one
mechanism with two users**, both already researched, neither invented. That
is the strongest available answer and it is strictly better than a chance
roll.

**Why the retiring parent and not the new child.** The child carries the
frontier forward — that is this session's own tip-retirement fix
(plant.rs:399-419), and making the child a `Leaf` would terminate the
lineage every plastochron. Making the *parent* a `Leaf` places foliage
along the shoot *behind* the advancing tip, which is where leaves are on a
real shoot, and it requires no new cell creation whatsoever: it is a
one-line change to `self_type_after_grow` (plant.rs:419).

**`Photosynthesize` moves to `Leaf` only.** `tree.ron`'s `GrowingTip` loses
it. `tree.ron`'s existing `(Leaf, [Photosynthesize(rate: 0.35)])` entry —
defined but currently unreachable, since nothing produces a `Leaf` — starts
being real for the first time.

**And this is what fixes the one-cell-thick trunk** (§2b). `thicken()`
(plant.rs:633) counts downstream `Leaf | GrowingTip` cells; today that
count is 0–2 because tips retire instantly and no `Leaf` is ever produced.
With persistent `Leaf` cells accumulating along every shoot, the count
grows monotonically with canopy size and `pipe_ratio: 2.5` becomes
reachable. `SecondaryThicken` starts firing for the first time, on the
signal Shinozaki's pipe model actually specifies.

### 5b. Leaves die: `age`, and the trade-off the evolution milestone will need

A `Leaf` accumulates `age` (Decision 2's sidecar) and abscises past a
per-species `lifespan` — the cell becomes inert detritus, falls, and feeds
the existing ash→soil cycle and §6b's debris-catching.

**Why bother now rather than later.** `PLAN.md`'s summary of the evolution
research is explicit that the **leaf economics spectrum** — fast
photosynthesis inversely coupled to leaf lifespan and durability — is one
of the two real trade-offs that must *already exist* before any selection
runs, or selection collapses the whole population onto one morphology.
(Wright, I.J. et al. (2004), *"The worldwide leaf economics spectrum,"*
Nature 428:821-827 — the standard reference for that inverse coupling.)
`PLAN.md`'s standing constraint is to *"prefer adding new per-organism
state over hardcoding more assumptions."* A `Photosynthesize.rate` /
`lifespan` pair per species is that trade-off, built as a seam rather than
retrofitted onto a system that already assumed leaves are permanent.

### 5c. The starting reserve, derived rather than guessed

**Recommendation: `2.0`** — half of `RESOURCE_SCALE = 4.0`.

Derivation against the *shipped* economy (`tree.ron`, retuned this session
via the 6-way parallel comparison recorded in that file's own header):

- `GrowingTip.cost = 0.2` per growth step.
- Once `Photosynthesize` is leaf-only, a seedling funds `plastochron`
  growth steps entirely out of reserve before any income exists.
- Minimum viable reserve is therefore `plastochron × cost`. At
  `plastochron = 4`, that is `0.8` — with **zero** margin for an unlucky
  low-light tick, a blocked candidate, or a `Grow` miss.
- Real seed reserves exist precisely to provide that margin.
- `2.0` buys 10 growth steps at `cost 0.2` — roughly 2.5 plastochrons of
  slack. It is also exactly half the scale, so it stays inside
  `RESOURCE_SCALE` whether or not the clamp survives Decision 2.

Split at germination: `germinate()` (plant.rs:590) currently gives both the
shoot and the companion root `0.0`. Give the shoot ~70% and the root ~30% —
a seedling's first priority is reaching light, and the shoot is the one
that must reach a leaf before the reserve runs out. **The 70/30 split is
untuned and flagged as such.**

**A tuning consequence that must not be discovered by surprise.**
`tree.ron`'s header records that the 6-way comparison found `cost` to be
the dominant lever specifically *because* "a fresh cell's very first `Grow`
check always reads resource=0 and `Grow` runs before `Photosynthesize` each
tick, so a lower cost mainly buys margin against an unlucky low-light
tick." **Both halves of that reasoning are removed by this decision** — a
seed now starts with resource, and tips no longer photosynthesize at all.
`cost` and `rate` must be re-tuned after this lands, and the existing
values carry no authority over the new economy.
`examples/debug_tree_variants.rs` already exists to do exactly this, and
running it is a required implementation step, not an optional follow-up.

### 5d. Seedling mortality, emergent rather than special-cased

**What happens today if reserve runs out before a leaf exists:** `Grow`
hits `resource < cost` and `continue`s; `found_candidate` stays false;
`stale_ticks` increments; at `ORGANISM_STALE_LIMIT` (4) a `GrowingTip`
converts to `MatureBody` (plant.rs:558-573) and stops. The result is a
permanent inert 1–2 cell wood stub that never dies and never grows. That is
*nearly* seedling mortality, but the artifact is wrong and the organism id
leaks forever.

**Decision: a `GrowingTip` reaching the staleness limit while its organism
has zero `Leaf` cells dies rather than matures.** Its cells lose their
`organism_id` (becoming ordinary inert wood, which the existing fire and
decay paths already handle correctly — `organism-substrate-design.md` §2's
"a fully-reclaimed dead tree's former trunk" case, which the code already
anticipates), and the organism id is returned to `free_organism_slots`.

Two things worth noting about that:

- **It requires `free_organism` to finally exist**, the missing half of
  issue #8 that `world.rs`'s own comment defers *"for the tree retrofit
  (which already needs exactly this)."* This decision is the caller that
  makes it non-dead code. Decision 2's cell list makes the liveness check
  cheap.
- **It is one condition on an existing branch, not a new failure path.**
  `design-philosophy.md` §2b forbids hardcoded *outcomes*; "a seedling that
  never reaches a leaf before exhausting its reserve dies" is a consequence
  of the resource economy, not an authored rule about seedlings.

**Grounded.** The heterotrophic→autotrophic transition — the point at which
seed reserves are exhausted and the seedling must be self-supporting on
photosynthesis — is a documented developmental checkpoint and a real cause
of seedling mortality; light regime at that transition is a critical
survival factor, and low soil moisture shortly after germination is the
major cause of seedling mortality in natural habitats
([Arabidopsis *katamari2* seedling arrest at the heterotrophic-to-
autotrophic phase transition, Plant Cell Physiol. 65:350](https://academic.oup.com/pcp/article/65/3/350/7510911);
[PRC2 facilitates the transition from heterotrophy to photoautotrophy
during seedling emergence, PMC12236341](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC12236341/)).

Combined with Decision 3, this makes both of the real-world killers of
seedlings real in the engine: **too dark to reach a leaf in time**
(`ambient_light_above` too low → `Photosynthesize` never repays the
reserve) and **too dry to root** (soil below PWP → `paw_fraction` is
exactly zero). Neither is special-cased anywhere.

---

## 6. Decision 5 — differentiated materials, and environmental interaction

### 6a. Which cell types get their own material, and which get shading

**Test applied:** this engine's own, from `material.rs` — a material exists
when *physics* differ; behaviour that differs only in *which rules run* is
already `CellType`'s job. Duplicating `wood.ron` for cell types with
identical physics is precisely what "behaviour from data, not a branch per
material" exists to prevent.

**`leaf` — a real new material.** Its physics genuinely differ, on four
numbers that already exist in `Material`:
- much lower `density` (foliage is not wood),
- much higher `flammability` and much shorter `burn_duration` — a real
  canopy fire runs through foliage, not heartwood, and this feeds the
  Drossel-Schwabl forest-fire loop the engine already cares about,
- `max_unsupported_span` small, or `leaf` is deliberately *not*
  `is_body_material` at all — a leaf is not structural, and treating it as
  a load path would let a canopy hold up a trunk,
- `breaks_into` a light detritus that falls (and is Decision 5b's debris).

**`rootwood` — a real new material.** Also genuinely different: it must be
distinguishable *by material* for §6c's soil stabilization (which runs in
`update_powder`, where no organism sidecar lookup is available); it should
be far less flammable than trunk wood, because it is wet and buried; and
darker.

**`GrowingTip` vs `MatureBody` — shading only, no new material.** Identical
physics (`wood`), differing only in which behaviors run. Modulate colour at
render time from `CellType` — a young shoot lighter and greener than
lignified wood. This is the same CPU-side, no-shader work M19's own tier 1
already scopes (`research/m19-visual-polish.md`: per-cell brightness
modulation from `Cell::shade`), and it should be built there rather than
duplicated here.

### 6b. Does a `Powder` rest on a `Plant` cell? — answered from the code

`PLAN.md` records this as unchecked: *"does a powder cell resting against a
`Plant` cell already count as supported by the existing CA fall rules, or
does it currently fall through/around? Not yet checked."*

**Checked. It rests. No new mechanic is needed.** The trace:

- `update_powder` (update.rs:142) first calls `try_move(x, y, x, y+1)`.
- `try_move` (update.rs:546) moves into an empty destination; otherwise it
  requires `dst_kind.is_displaceable()`.
- `MaterialKind::is_displaceable` (material.rs:104) is
  `matches!(self, Liquid | Gas)` — **`Plant` is not displaceable.**
- So the straight-down move fails and the grain is supported by the plant
  cell.
- `roll_along_slope` → `downhill_distance` (update.rs:503) only walks over
  `surface.is_empty` cells, so a plant surface also gives a pile a real
  angle of repose rather than letting it slide off.

**The real blocker is geometry, not physics.** `update_powder` next tries
the two diagonals `(x±1, y+1)`. For a **one-cell-thick** branch both are
empty, so the grain falls past after one frame of contact. Debris catches
on branches *that are more than one cell wide, or that run horizontally* —
and the tree currently has neither.

So the correct design answer is: **build nothing, and note that
`SecondaryThicken` (unblocked by Decision 4's persistent `Leaf` cells) is
what actually delivers this feature.** Saying that plainly is more useful
than inventing a support mechanic for a case the CA already handles.

### 6c. What *is* genuinely missing: load

Nothing weighs on a branch. `structural.rs` measures **distance from an
anchor**, not **load**: `organism_is_supported` (structural.rs:203) BFSes
outward up to `max_unsupported_span` (wood: 8) and returns a boolean. A
branch with a metre of sand piled on it breaks at exactly the same span as
a bare one. `PLAN.md` treats "too much weight breaks a branch" as already
in scope; it is only half-built.

**Minimal, honest extension:** reduce the *effective* `max_span` by the
count of non-organism, non-`Empty` cells resting directly on that
organism's cells within the search. One extra term in an existing function,
no new storage, no new schedule.

```
effective_span = max_span.saturating_sub(supported_load / LOAD_PER_SPAN_UNIT)
```

**Named as an analogue, not an equation.** Real allowable cantilever span
falls with load, and that is the only property being borrowed. This is not
a beam-deflection calculation and must not be described as one. It is a
weighted local rule — permitted by `design-philosophy.md` §2b — whose
*outcome* (which branch breaks, when) is emergent from what the player
actually piled on it.

### 6d. Roots stabilize soil — the mirror, correctly identified

`PLAN.md` frames this as "extending anchor-distance credit outward from a
root into adjacent soil." **That framing does not work, and it is worth
being precise about why:** `Powder` cells do not participate in
`structural.rs` at all. They have no anchor distance, they never "break
free," they simply fall via `update_powder` every frame. There is no
distance to extend credit into.

**The correct mirror is in `update_powder`, not `structural.rs`:** a
`Powder` cell with a root-material cell among its 4-neighbours **does not
move**. One check, at the top of `update_powder`, before the first
`try_move`.

**Grounded in real, measured geotechnics, not analogy.** Root reinforcement
of soil is a standard and quantified effect: roots crossing a shear plane
act as laterally loaded fibres in tension, resolving into a tangential
component that adds **apparent cohesion** to the soil. The two founding
references are Waldron, L.J. (1977), *"The shear resistance of
root-permeated homogeneous and stratified soil,"* Soil Science Society of
America Journal 41:843-849, and Wu, T.H., McKinnell, W.P. & Swanston, D.N.
(1979), *"Strength of tree roots and landslides on Prince of Wales Island,
Alaska,"* Canadian Geotechnical Journal 16:19-33 — together the
**Wu–Waldron model**, still the baseline in slope-stability practice
([review context: Assessing the influence of root reinforcement on slope
stability by finite elements, Int. J. Geo-Eng. 6:12](https://link.springer.com/article/10.1186/s40703-015-0012-5)).

**Simplification, named:** apparent cohesion is a continuous strength
increment; this engine gives a binary "does not fall." A graded version
(root-adjacent soil gets a reduced `roll_reach_at`, so it holds a steeper
slope without being fully immobile) is strictly more faithful and is the
obvious upgrade if binary reads as too absolute. Start binary — it is one
line and immediately verifiable by screenshot.

**Why this is worth building:** it makes "plant trees to stop the hillside
collapsing" a real, discoverable, entirely emergent mechanic, and it closes
a loop in the direction that currently does not exist — today the world
acts on plants (light, moisture, wind, fire) and plants act back on almost
nothing.

---

## 7. Deliberately out of scope

### 7a. Polarity and directional diffusion — deferred, by direct owner instruction

`PLAN.md`'s summary of the research document's §5 identifies this as *"the
highest emergent-behavior-per-effort item in the whole document"*:
`organism::diffuse_resource` is isotropic neighbour-averaging, every real
shape-generating process in plant development is polar, and symmetric
diffusion can blur a gradient but never canalize it into a channel no
matter how long it runs or how the weights are tuned.

**It is nonetheless deliberately not designed here**, on the owner's
explicit mid-session instruction: *"Let's plan all of this before we start
implementing any of it. I don't want to optimize if we are going to make
large changes to our diffusion mechanism."*

**Why that instruction is correct, briefly.** Polarity changes the *core
transport mechanism* that Decisions 1, 3 and 4 would each otherwise be
tuned against. `Grow`'s `cost`, `Photosynthesize`'s `rate`, the seed
reserve of §5c, and `RootTip`'s soil-water income of §4d are all
calibrated against how fast resource actually arrives where it is spent —
which is exactly what a polar, flux-following transport rule would change,
globally, for every one of them at once. Tuning that economy against
isotropic diffusion and then replacing the diffusion is doing the tuning
pass twice.

**One thing this document does do for it, which costs nothing:** Decision
2's `OrganismCell` is a plain struct with room for any number of fields. A
future polarity vector is two more `f32`s in it — no layout question, no
bit budget, no migration. That is the seam, and it is deliberate.

### 7b. Evolution and genetics — future milestone, acknowledged only

Out of scope, per `PLAN.md`'s own framing (*"a real future milestone, not
this phase"*). Two things above were nonetheless shaped so it is not
foreclosed, and both were cheap: §5b's leaf `rate`/`lifespan` pairing gives
the leaf-economics trade-off a place to exist before selection needs it,
and Decision 2's per-organism `OrganismCell` table is the natural home for
per-organism trait variation, per `PLAN.md`'s standing constraint to prefer
per-organism state over new assumptions that every individual is identical.

---

## 8. Simplifications, stated honestly

Collected in one place, in the spirit of `tree-rewrite-design.md` §3's
walk-back of its own revision-1 overclaim.

1. **Soil mass is not conserved** when a root displaces a cell (§2c). Real
   root growth compacts the rhizosphere; at one cell per pixel there is
   nowhere to put the compaction.
2. **Bud break is not canalization** (§2e). It produces *a* new frontier
   under local surplus. It does not produce apical dominance, and this
   document claims no single-leader outcome — same walk-back
   `tree-rewrite-design.md` §3 already made.
3. **Atterberg limits are gravimetric; the engine's soil `aux` is
   volumetric** (§4e). The three-state ordering is faithful; the numeric
   limits are calibration targets, not unit conversions.
4. **Aeration threshold is one number per species** where Grable & Siemer
   explicitly say no single value is optimal (§4b). Made a per-species
   parameter for exactly the reason `organism-substrate-design.md` §4 makes
   `pipe_ratio` one.
5. **Load reduces span; it is not a beam equation** (§6c). Qualitatively
   right direction, quantitatively an analogue.
6. **Root reinforcement is binary, not graded** (§6d). Apparent cohesion is
   a continuous strength increment in the Wu–Waldron model.
7. **`thicken()`'s "downstream" is still a flood fill** and must stay one
   (§3e) — the organism cell list gives a *whole-organism* leaf count,
   which is a different quantity and would silently break the pipe model.
8. **No claim is made that any of this reads well.** Every number above is
   a starting point for `examples/debug_tree_variants.rs`, and §9's
   verification gates are the actual authority — the same standard
   `tree-rewrite-design.md` §11 step 6 set and the tree rewrite honoured.

---

## 9. Retrofit order

Shaped like `tree-rewrite-design.md` §11: what unlocks what, what is safe
in parallel, what is strictly sequential, and where the real gates are.

**Sequential, and genuinely blocking:**

1. **Decision 2, steps 2a–2d** (§3f). Everything else needs sidecar
   storage; there are no free bits. **2a is the risky step** (cell-slot
   registration across every creation and removal site, including
   `structural::break_free` and `fire::transform`, which currently have no
   organism awareness) and it is deliberately behaviour-free, so a bug
   there shows up as the grid-scan agreement test failing rather than as a
   corrupted tree.
   **Gate:** full test suite green after each of 2a/2b/2c/2d
   independently. The organism-boundary diffusion test must be rewritten
   before 2c, not after — see §3f for why it could otherwise pass
   vacuously.

2. **Decision 4** (§5), immediately after 2d. Chosen second, ahead of the
   soil work, for three reasons: it is the smallest change with the largest
   visible effect (visible leaves, and `SecondaryThicken` firing for the
   first time); it is what actually fixes the one-cell trunk (§2b); and it
   forces `free_organism` to exist, which Decision 3's necrosis path then
   reuses rather than reinvents.
   **Gate:** re-run `examples/debug_tree_variants.rs`. `cost`/`rate` **must**
   be re-tuned here — §5c explains why both halves of the existing tuning
   rationale are invalidated. Live screenshots under
   `docs/screenshots/`, per standing practice: visible leaves, a trunk more
   than one cell thick, and a seedling planted in shade that dies rather
   than becoming an immortal stub.

**Safe in parallel with each other, once 1 and 2 land:**

3. **Decision 3, part A — soil moisture storage and drainage** (§4a, §4b,
   §4d's drainage rule, §4e's `mud`). Touches only inert-cell `aux`,
   `decay.rs` and `material.rs`. No organism code at all. Independently
   testable: soak a soil column, watch a wetting front descend, watch mud
   appear at the plastic limit.

4. **Decision 5, parts A and C — `leaf`/`rootwood` materials, and root soil
   stabilization** (§6a, §6d). `.ron` data plus one check in
   `update_powder`. Independent of everything above except that `leaf` as a
   material wants Decision 4's `Leaf` cells to exist to be visible.

5. **Decision 5, part B — load reduces span** (§6c). One term in
   `organism_is_supported`. Fully independent.

**Sequential again, and last:**

6. **Decision 1(ii) — root displacement into soil** (§2c). Needs Decision
   3A's per-cell moisture (to credit water on the way through) and
   Decision 5A's `rootwood`. **This is where roots become real for the
   first time** — `germinate()` (plant.rs:594) should also stop gating the
   companion `RootTip` on `world.is_empty(x, y + 1)`, which is why the test
   scene's stone floor produced no roots at all.

7. **Decision 3, part B — `Absorb` from soil, and anoxia necrosis** (§4c,
   §4d paths 2 and 3). Needs 6. This is the step that closes `PLAN.md`'s
   recorded `RootTip` income gap.
   **Gate, and it is the interesting one:** plant a tree over a water
   table and confirm the root system stabilizes *above* the saturated zone
   rather than growing into it and dying wholesale, or stalling short of
   it. That emergent equilibrium (§4c) is the single best evidence the
   soil model is doing real work, and if it does not appear, the aeration
   threshold and `ANOXIA_LIMIT` are the two knobs.

8. **Decision 1(iv) — bud break** (§2e). Deliberately **last**, and
   deliberately after everything else has been screenshot-verified. It is
   the one mechanism that removes the ceiling on total size, which means it
   is also the one that will expose every scaling problem in every decision
   above — a tree that can grow indefinitely will find whatever breaks at
   50 cells that never showed at 18. Adding it earlier would confound
   "does this mechanism work" with "does it still work at ten times the
   size."
   **Gate:** run to 50,000+ ticks and confirm the tree keeps growing,
   `active_site_count` stays bounded, and the shape does not degenerate
   into a blob — the canopy-density self-avoidance term
   (`tree-rewrite-design.md` §2b) is what should prevent that, and this is
   the first workload that genuinely tests it.

9. **Independent design review before commit**, per standing practice for a
   change this size, specifically re-checking: that 2a's cell-slot
   registration has no double-free path; that the organism-boundary
   diffusion test is not vacuous after 2c; and that §5d's seedling death
   actually frees the organism id rather than merely stopping the schedule.

**Explicitly not in this pass:** polarity/directional diffusion (§7a),
evolution (§7b), a `Liquid`-kind flowing mud (§4e), a resistance-network
transport solve, and Palubicki-style shadow-voxel light competition — the
last two carried over unchanged from `organism-substrate-design.md` §7's
own out-of-scope list.
