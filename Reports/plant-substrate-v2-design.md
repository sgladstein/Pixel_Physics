# Plant substrate v2: growth mode, storage, soil moisture, leaves, environment, and polarity

**Audience:** the coding agent implementing this.
**Status: implemented and merged.** *Design only, written just-in-time
before implementation, per `design-philosophy.md` §3's standing instruction
and the precedent `organism-substrate-design.md` / `tree-rewrite-design.md`
both set. No code in this pass* — that was true when this was written and
has not been true for some time; it is quoted rather than deleted because
the just-in-time precedent is the part worth keeping. **Corrected
2026-08-27.** This is the most-cited plant report in the tree (28 source
citations), so an agent opening it directly and reading "no code in this
pass" is the most expensive instance of that mistake available.
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

**Update, at the Decision 6 pass:** the recommendation above was taken.
`Reports/plant-simulation-research.md` was recovered from the owner's
original upload and committed as `5a3c9b9`, before any further work — see
`PLAN.md`'s own handoff entry. Every citation of it in §7 below is a
citation of the real primary source, read directly, not of `PLAN.md`'s
summary. §1–§6 were written before the recovery and still stand on the
summary; nothing in the recovered text contradicts them, and §7 below
quotes that document's own §5 verbatim where it matters.

Everything else below is verified directly against the code as it exists at
`08d33fe`, not against what the docs claim exists. Where the two diverge,
the code wins and the divergence is named.

---

## 1. The six decisions, stated first

| # | Question | Decision |
|---|---|---|
| 1 | Growth mode | Accretion for canopy (**and it was never the binding constraint**); one-cell displacement for roots into soil; **reject** a separate sub-cell turgor scalar; **add** bud break, which `PLAN.md`'s lean omits |
| 2 | `Cell::aux` ceiling | `aux` for an organism cell becomes `4 bits CellType \| 12 bits cell-slot index`; all scalars move to a per-organism `Vec<OrganismCell>` on `OrganismState`; the diffusion pass leaves the CA sweep |
| 3 | Soil moisture | Per-cell fill in a `Powder`'s currently-unused `aux`, on a four-threshold curve (saturation / field capacity / wilting point / air-filled-porosity limit); too-wet costs a `RootTip` **necrosis**, duration-gated; `mud` is a real new material at the Atterberg plastic limit |
| 4 | Leaves | A **plastochron counter** turns the *retiring* `GrowingTip` into a `Leaf`; seed reserve `2.0`, derived from the shipped economy; starvation before first leaf frees the organism id — real mortality, and it forces `free_organism` to finally exist |
| 5 | Materials & environment | `leaf` and `rootwood` become real materials (their *physics* differ); tip-vs-mature is shading only. **Debris already catches on branches** — verified in code, no new mechanic needed. Root soil-stabilization is one new check in `update_powder` |
| 6 | Polarity / directional flux | **Reject** the research sketch's packed 8-direction enum; store a **per-face conductance `[f32; 4]`** on `OrganismCell`, one entry per 4-neighbour face, and make transport carrier-shaped (`RATE · c_ij · R_i`) instead of symmetric averaging. Conductance ratchets on measured flux through a **Hill-n=2** response — Sachs's canalization made mechanical. Canopy density **stays isotropic**. `Grow` gains **no new weight**: `away_from_growth` is replaced by a flux-derived `away_from_supply` |

Decision 1 is first because everything inherits from it, exactly as
`PLAN.md` frames it. Decision 2 is second because it is the only one that
is a *prerequisite* for the others rather than a peer of them.

**Decision 6, in one paragraph, because it arrived last and reverses an
earlier call.** `organism::diffuse_resource` is symmetric neighbour-
averaging, and a symmetric operator has no flux term, therefore no
feedback, therefore no channels — it blurs a gradient and can never
canalize one, no matter how the weights are tuned. Decision 6 gives every
organism cell four *directional conductances*, one per shared face, and
replaces the average with a pairwise carrier-mediated exchange whose
conductance is itself updated from the flux it just carried: a path that
carries flux gets better at carrying flux, which is Sachs's canalization
hypothesis stated as an update rule rather than as an aspiration. The
storage question that made this expensive is already gone — Decision 2's
`OrganismCell` is a plain struct, so four `f32`s cost nothing and need no
quantization, which is exactly why the research sketch's "three or four
bits, eight directions plus none" is the wrong layout now and why a set of
four continuous conductances is strictly richer: it can represent a real
branch point (two faces both conducting), which a single stored direction
structurally cannot. The rule reduces *exactly* to today's isotropic
diffusion when all conductances are equal, so it is a strict
generalization, not a replacement, and Decision 2's diffusion tests keep
their meaning. What it buys, honestly scoped in §7i: a real vein/
parenchyma conductance hierarchy, source-to-sink transport from leaves to
whichever sink is actually drawing, and a *persistent* leader bias at
branch points where today's mechanism produces an alternating one — not
real auxin-based apical dominance, which needs a second channel flowing
the other way and is not built here.

**Why it is sequenced between Decisions 2 and 4 rather than after
everything**, per `PLAN.md`'s own post-landing revision (see §8a): it is
not independent of either. Decision 2 already restructures
`diffuse_resource`'s execution shape and is what gives a polarity field
room to exist at all; Decision 4 re-tunes `tree.ron`'s entire resource
economy, and tuning that economy against isotropic transport and again
after polarity lands is the exact double-tuning the owner's original
"don't optimize if the diffusion mechanism is going to change" instruction
was meant to prevent.

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

## 7. Decision 6 — polarity and directional resource flux

Written after §1–§6 landed, as a deliberate reversal of what was then §7a's
"deliberately out of scope" call. §8a records the reversal and its reason;
this section is the design that reversal requires.

### 7a. The mechanism being modelled, from the primary source

**`Reports/plant-simulation-research.md` §5 states the problem exactly, and
it is worth quoting rather than paraphrasing** (the file exists now — see
§0's update):

> Sachs's canalization hypothesis — the basis of the auxin mechanism
> already cited in `m16-plant-biology.md` — is explicitly a **positive
> feedback between flux and conductivity**: a path that carries more flux
> becomes better at carrying flux, which is what turns a diffuse field into
> a discrete channel. That is how veins form, how vascular strands form,
> how one leader dominates its siblings.
>
> Symmetric averaging has no flux, therefore no feedback, therefore no
> channels.

That is the whole argument and this document accepts it without
qualification. The formalization to implement against is **Mitchison, G.J.
(1980), "A model for vein formation in higher plants," Proc. R. Soc. Lond.
B 207:79–109**, which turned Sachs's verbal hypothesis into the first
explicit flux/conductivity positive-feedback model, and its 1981 successor
which moves the feedback onto membrane carrier permeability rather than a
bulk diffusion coefficient — the form that survived, because it is the one
that matches PIN biology.

**The modern statement of that model, and the source of every functional
form used below, is Feller, C., Farcot, E. & Mazza, C. (2015),
*"Self-Organization of Plant Vascular Systems: Claims and Counter-Claims
about the Flux-Based Auxin Transport Model,"* PLOS ONE 10(3):e0118238**
([full text](https://journals.plos.org/plosone/article?id=10.1371%2Fjournal.pone.0118238)).
Its carrier-update equation is

```
d p_ij / dt  =  ρ₀ · Φ( J_i→j )  −  μ · p_ij
```

— carrier density on cell *i*'s membrane facing cell *j* is inserted in
proportion to a non-negative increasing response `Φ` of the auxin flux
across that face, and removed at a constant turnover rate. **Three of that
paper's findings are load-bearing for the decisions below**, and all three
are used rather than cited decoratively:

1. `Φ(x) = x²` (Mitchison's own quadratic) yields **steady-state patterns
   that are loopless directed trees** — which is precisely the topology a
   tree's vasculature has, and precisely the topology this engine wants.
2. Unbounded `Φ` **diverges**, and the paper is explicit that this is an
   intrinsic property of the model, not a numerical artefact. A real-time
   simulation that runs indefinitely and never solves for a steady state
   cannot ship a divergent update rule.
3. A **bounded** `Φ` (their example: a Hill form `κx/(J_ref + x)`) is
   stable but permits **loops** at steady state.

§7e resolves (1) against (2)/(3) explicitly rather than picking one and
hoping.

**And the other half of the biology, which the auxin literature does not
cover, is that the thing actually being transported here is
photosynthate, not auxin.** Phloem transport is the **Münch (1930)
pressure-flow** mechanism: sugar loaded at a source raises turgor,
unloading at a sink lowers it, and bulk flow follows the resulting
pressure gradient from source to sink. The direction in a given strand is
therefore **set by which end is currently the sink**, not fixed by anatomy
— which is why §7j can claim, as a mechanism rather than a hope, that a
leaf's carbon goes to whichever sink is actually drawing. The hypothesis
is well over ninety years old and was only recently tested end to end:
Knoblauch et al. (2016), *"Testing the Münch hypothesis of long distance
phloem transport in plants,"* eLife 5:e15341
([eLife](https://elifesciences.org/articles/15341);
[PMC4946904](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4946904/)), which
found that **sieve-tube conductivity and turgor both rise sharply as
source-to-sink distance increases** — i.e. the conductance of a real
transport path is itself tuned to the transport demand placed on it. That
is the same flux→conductance feedback Sachs described, measured in phloem
rather than inferred from vein patterns, and it is direct empirical
support for applying a canalization rule to the *resource* channel and not
only to a hypothetical auxin one. See also
[Phloem transport: a review of mechanisms and controls (J. Exp. Bot.
64:4839)](https://academic.oup.com/jxb/article/64/16/4839/593231) for the
source–sink control framing.

### 7b. Data layout — four per-face conductances, not a packed direction

**Decision: `OrganismCell` (Decision 2, §3c) gains one field.**

```rust
pub struct OrganismCell {
    // ... every field Decision 2 §3c already defines, unchanged ...

    /// Per-face carbon efflux conductance, indexed in `NEIGHBOURS_4`
    /// order. `carbon_conductance[k]` governs export *out of this cell*
    /// across face `k`; the neighbour on the other side of that face
    /// stores its own, independent, opposing value.
    carbon_conductance: [f32; 4],
}
```

Sixteen bytes. Plain `f32`s, per Decision 2's whole point.

**This deliberately rejects `plant-simulation-research.md` §5's own
proposed layout** — *"three or four bits of per-cell polarity (8
directions, plus 'none')"* — and the rejection is not a detail. Four
reasons, in increasing order of importance:

1. **The bit budget that motivated packing no longer exists.** The sketch
   was written against `Cell::aux` at 16/16 bits full, where three or four
   bits was the most anyone could hope for. Decision 2 removed that
   constraint entirely. Quantizing a signal into 3 bits when the storage
   is a plain struct field would repeat exactly the mistake §3a documents
   canopy density already made — a constant (`CANOPY_DENSITY_DECAY_PER_
   TICK`) tuned around a quantization half-step rather than around the
   behaviour it controls.
2. **Transport happens across shared faces, and the engine's diffusion
   neighbourhood is already 4.** `diffuse_resource` iterates
   `NEIGHBOURS_4`; `Grow` scores `NEIGHBOURS_8`. Those are different
   things and both are correct as they stand: growth is a *placement*
   decision, which has eight options in a square lattice, while transport
   is an *exchange across a shared boundary*, and diagonal cells share
   only a corner. There is no membrane there to put a carrier on. Adding
   diagonal transport would also break the explicit-diffusion Fourier
   bound (≤ 0.25) that `organism.rs`'s own `DIFFUSION_RATE` doc, and
   `fire.rs`'s and `field.rs`'s, all derive and respect.
3. **The biology is per-face.** PIN efflux carriers sit on a specific
   membrane face; the model in §7a is indexed `p_ij`, per ordered cell
   pair, for that reason. A single per-cell direction is a *summary* of
   the per-face state, not the state itself.
4. **A single stored direction cannot represent a branch point — which is
   the one case the entire mechanism exists to resolve.** A cell feeding
   two children has two faces genuinely carrying flux. An 8-direction
   enum must pick one of them, which means the data structure decides
   apical dominance before the update rule ever runs. That is precisely
   the "authored outcome" `design-philosophy.md` §2b forbids, and it
   would make the worked example in §7h vacuous. Four independent
   conductances represent "one strong channel," "two co-equal channels,"
   and "nothing established yet" natively, and let the *rule* decide which
   one obtains.

**How it interacts with the resource scalar it biases.** `carbon` (the
scalar) is the amount present; `carbon_conductance` is the per-face
capacity to move it. They are separate quantities with separate update
rules on separate timescales — the scalar changes every transport substep,
the conductance once per organism tick (§7e). A cell's *polarity*, where
some caller wants a single direction (only `Grow` does, §7g), is a derived
read, never a stored field.

**Not stored, deliberately:** measured flux. Flux is produced and consumed
inside one pass (§7d measures it, §7e folds it into conductance at the end
of the same pass), so it lives in a scratch `Vec<[f32; 4]>` sized to the
organism's own cell list, not on `OrganismCell`. Storing it would be a
second copy of derivable state, and Decision 2's whole thesis is that
storage is cheap but *duplicated* storage is where bugs live.

### 7c. What replaces `diffuse_resource`'s symmetric average

The current rule (organism.rs:588) is, for a cell with *n* same-organism
`Plant` 4-neighbours:

```
new = here + (mean(neighbours) − here) · DIFFUSION_RATE
```

**Decision: replace it with a pairwise, carrier-shaped exchange, evaluated
once per shared face.** For the face between cell *i* and cell *j*:

```
efflux(i→j) = RATE · c_ij · R_i          // carrier-mediated, source-concentration proportional
efflux(j→i) = RATE · c_ji · R_j
net J_ij    = efflux(i→j) − efflux(j→i)
             = RATE · ( c_ij·R_i − c_ji·R_j )
```

and `J_ij` is moved from *i* to *j* (negative moves the other way). Four
properties, each of which is a reason this is the right form and not
merely a workable one:

- **It reduces exactly to Fickian diffusion when polarity is absent.** Set
  `c_ij = c_ji = c` and the net becomes `RATE · c · (R_i − R_j)` — the
  symmetric rule, with `RATE · c` as the diffusion coefficient. **So
  polarity is a strict generalization of what ships today**, and setting
  `VEIN_GAIN = 0` (§7e) recovers current behaviour exactly. That is the
  implementation's own A/B switch and it costs nothing.
- **It is exactly conserving.** Every unit leaving *i* arrives at *j* in
  the same statement. The current form is not: each cell independently
  moves itself toward its neighbours' mean, in place, in sweep order, so
  the total is only approximately preserved and depends on visit order.
  Making transport pairwise is a free correctness win that falls out of
  moving to Decision 2's per-organism pass.
- **It matches the model in §7a.** `RATE · c_ij · R_i` is exactly the
  carrier-mediated efflux term (`p_ij` times source concentration) that
  `Φ` is a function of the *net* of.
- **Walls are unchanged.** `is_wall` (a different `organism_id`, or a
  non-`Plant` kind) is untouched, so `resource_does_not_cross_an_organism_
  boundary` keeps its exact assertion. Decision 2 §3f's warning about
  rewriting that test so it is not vacuous applies verbatim and is not
  weakened here.

**Implementation shape, concretely.** Iterate the organism's `cells` list;
for each cell process only its `+x` and `+y` faces, applying both halves of
the exchange. Every face is then visited exactly once, with no
double-counting and no ordering dependence.

**Substeps, and the stability bound they exist to respect.** The
explicit 4-neighbour scheme requires `RATE · c ≤ 0.25` for the *largest*
conductance in play, not the typical one. With the contrast ratio §7e
lands on, that caps a single step's coefficient well below what today's
flat `DIFFUSION_RATE = 0.2` delivers, so the pass runs
`TRANSPORT_SUBSTEPS` iterations per organism tick instead of one.

This is not a workaround — it is `plant-simulation-research.md` §6's own
recommendation, adopted verbatim: *"Run diffusion to convergence, not one
step per frame. Growth ticks are every 20–45 frames; the resource field
could relax many times between them."* And it replaces something that is
currently an accident rather than a decision: today `diffuse_resource`
runs from the CA sweep on **every frame of every awake chunk**, so it
happens to execute ~45 times per `ORGANISM_TICK_INTERVAL` because that is
how often the sweep runs, not because 45 is right. `TRANSPORT_SUBSTEPS`
makes that number an explicit, tunable parameter for the first time.
**Starting value 16, and it is a first-class tuning target for Decision
4's re-tune, not a constant to leave alone** — it sets, directly, how far
resource travels between two growth decisions.

### 7d. Measuring flux, and what "flux" means per tick

Each substep accumulates `|J_ij|` (signed, per ordered face) into the
scratch buffer. At the end of the organism tick, face *k* of cell *i* has
`F_ik` = total net resource exported across that face this tick. **That
total, not a per-substep value, is what feeds the conductance update** —
which gives the two-timescale structure the biology actually has: bulk
flow is fast, carrier turnover is slow. Only the *positive* part
reinforces:

```
J_ik = max( F_ik , 0 )
```

**Why the clamp matters and is not a fudge:** conductance on cell *i*'s
face toward *j* is an *efflux* capacity. Net import across that face is
evidence that *j*'s opposing face is conducting, and *j* is the cell that
should be credited for it — it will be, by its own entry. Crediting both
sides of a reversing face would make a face that oscillates in direction
read as a strong channel, which is the opposite of what canalization
means.

### 7e. The conductance update rule, and the response function

**Decision:**

```
c ← c + VEIN_BASAL + VEIN_GAIN · Φ(J) − VEIN_DECAY · c

           J²
Φ(J) = ───────────
        J_REF² + J²
```

**`Φ` is a Hill function with exponent 2, and that is a deliberate
resolution of §7a's findings (1) versus (2)/(3), not a third option picked
at random.** It is *convex* below `J_REF` and *concave* above it:

- Below `J_REF` it behaves like Mitchison's quadratic — superlinear, so a
  face with a flux advantage compounds that advantage faster than
  linearly. This is the regime that produces §7a(1)'s loopless directed
  trees, and it is the regime that makes canalization canalize.
- Above `J_REF` it saturates at 1. This is §7a(3)'s bounded form, and it
  is what makes the rule **provably non-divergent** where the pure
  quadratic is provably divergent: `c` has a hard fixed-point ceiling at
  `(VEIN_BASAL + VEIN_GAIN)/VEIN_DECAY`, reached only as `Φ → 1`.

So the engine gets the quadratic's topology in the regime where topology
is being decided, and the bounded form's stability in the regime where a
long-running simulation would otherwise blow up. `J_REF` is the knob that
places the operating point between the two, and it has a natural default:
**`J_REF` = the species' `Grow.cost`** — the flux one growing tip's demand
represents. Below one tip's worth of demand, the response is
competition-amplifying; above it, it saturates. `tree.ron`'s `cost: 0.2`
makes `J_REF = 0.2`.

**The three constants, and the single ratio that actually matters.** The
flux-free fixed point is `c_min = VEIN_BASAL / VEIN_DECAY`; the saturated
fixed point is `c_max = (VEIN_BASAL + VEIN_GAIN) / VEIN_DECAY`. Their
ratio is the only thing the behaviour depends on, and it deserves a name:

```
canalization contrast  =  c_max / c_min  =  1 + VEIN_GAIN / VEIN_BASAL
```

**Starting values: `VEIN_BASAL = 0.1`, `VEIN_DECAY = 0.1`, `VEIN_GAIN =
2.9`** — so `c_min = 1.0`, `c_max = 30.0`, contrast 30:1. And with the
stability bound `RATE · c_max ≤ 0.25`, `TRANSPORT_RATE = 0.008`.

**The consequence of that contrast, stated before someone discovers it as
a bug: unpolarized tissue transports resource much more slowly than it
does today.** `TRANSPORT_RATE · c_min = 0.008` against today's flat `0.2`,
recovered only over 16 substeps and only partially. **This is correct and
is the point.** Undifferentiated parenchyma *is* a poor conductor; a
vascular strand *is* a good one; the entire biological function of vascular
tissue is that the contrast exists. It also makes a real, falsifiable
prediction: a fresh seedling with no established vasculature will be
transport-limited, and its first act will be to canalize a strand from its
first `Leaf` to its tip. If instead seedlings simply never get going, the
contrast ratio and `TRANSPORT_SUBSTEPS` are the two knobs, in that order.

**A fresh cell with no established polarity.** All four faces initialize to
`c_min`, not zero. **Zero would be a bootstrap deadlock** — zero
conductance gives zero flux gives zero reinforcement, forever — and the
literature has the same term for the same reason: `ρ₀` in §7a's equation
is a *basal* insertion rate, constitutive and flux-independent, because
carriers are inserted before they are polarized. `VEIN_BASAL` is that term
and its presence is required, not decorative. Every cell therefore starts
perfectly isotropic and identical to today's behaviour, and differentiates
only from flux it actually carried.

**A branch point — the case that matters.** Nothing special-cases it,
which is the design's main claim. Two faces both carrying real flux both
get reinforced, both rise, and the *ratio* between them is set by the
ratio of their fluxes through `Φ`. Co-dominance is representable and is a
genuine possible outcome; so is one channel pulling away. §7h works the
arithmetic rather than asserting which.

### 7f. Canopy density stays isotropic — and why that is principled

`organism::diffuse_resource` currently carries two channels through one
loop: resource and `tree-rewrite-design.md` §2b's canopy density.
**Decision: only the resource channel becomes polar. Canopy density keeps
the symmetric rule exactly as it is today.**

Not a scope cut — the wrong thing would be for it to follow:

- **Canopy density is not a transported substance.** It is a stigmergic
  proxy for *how much of my own tissue is near this location*, deposited
  at cell creation and read by `Grow` as a crowding penalty. There is no
  vessel carrying it, no source, no sink, and no conserved quantity. The
  deposit→diffuse→decay→follow shape is a spatial smoothing kernel, not a
  flow.
- **Following veins would make it blind exactly where it needs to see.**
  A tip must avoid growing into dense canopy *in any direction*. If
  density propagated preferentially along established conductance, a dense
  clump sitting off-vein — which is the crowded direction a tip most needs
  to detect — would be invisible to it. The signal would be strongest along
  the path already taken, which is the one direction crowding does not
  need to warn about.
- **It would resurrect a known bug class.** `plant-simulation-research.md`
  §2b's own framing is that the crowding term and isotropic diffusion are
  *the same failure mode* — a mechanism named after a directional process
  implemented as a symmetric one. The symmetric half of that pairing is
  wrong for resource and right for crowding; treating them as one problem
  is what produced the "always reads 0.0" bug in the first place.

**Implementation consequence, and it keeps `tree-rewrite-design.md` §2b's
"not a second diffusion implementation" property intact.** The pass stays
one function over both channels, parameterized by where conductance comes
from: the carbon channel reads `carbon_conductance`; the density channel
passes a constant `1.0`. With a constant conductance the pairwise rule is
identically Fickian (§7c), so density's behaviour is bit-for-bit the
symmetric average it is today, expressed through the general form. One
rule, two channels, one of which happens to have flat conductance.

**Water, when it becomes real, gets its own array — and this is a
prediction of the design, not an afterthought.**
`plant-simulation-research.md` §5 notes that *"xylem moves water up and
phloem moves photosynthate down, in separate directional tissues."* Two
substances with genuinely opposite polarity cannot share one conductance
field. `OrganismCell` therefore gets a second `[f32; 4]` when Decision 3B
makes soil water a real currency with a real source — **at retrofit step 8,
not here** (§10). Building it speculatively now would mean tuning a channel
with no source against a sink that does not draw yet.

### 7g. `Grow`'s candidate scoring — a replacement, not a new term

**Decision: polarity adds no new scoring term and no new species
parameter. It replaces the computation behind the existing
`continuation_weight`.**

Today (plant.rs:329–345), `away_from_growth` is the negated, normalized
vector average of every same-organism 8-neighbour's offset —
`tree-rewrite-design.md` §2a's fix for "grow away from the parent" being
undefined at a branch point. It is a purely *geometric* proxy for "which
way did I come from."

Polarity supplies the real thing. For each of the four faces `d`, the
neighbour `n = (x,y) + d` stores its own conductance on the face pointing
back at this cell; that value is exactly "how strongly does `n` export
into me":

```
supply_weight(d) = c_n[ face of n pointing toward this cell ]
away_from_supply = −normalize( Σ_d supply_weight(d) · d )
```

and the scoring line becomes `dot(dir, away_from_supply) *
continuation_weight`, with everything else in the formula untouched.

**Why this is strictly better than the geometric version, on §2a's own
terms.** §2a's problem was that a tip with several same-organism
neighbours has no well-defined parent. The geometric average solves it by
treating every neighbour as equally "behind you" — including a *sibling*
tip created by the same branch event, which is beside you and feeds you
nothing. That sibling's offset drags `away_from_growth` sideways and makes
two fresh branches actively repel each other for purely positional
reasons. Under `away_from_supply`, a sibling that exports nothing into
this cell sits at `c_min` and contributes almost nothing to the sum, while
the stem cell that actually feeds this tip has a strongly ratcheted face
and dominates it. **The mechanism now distinguishes "adjacent to" from
"supplied by," which is what §2a was approximating and could not express.**

**Fallback, so §2a's proof survives the degenerate case.** When every
supply weight is still at the basal floor — a seed's very first `Grow`,
before any flux has ever been carried — the sum carries no information.
Detect it (`max(supply_weight) < c_min · (1 + ε)`) and fall back to the
existing geometric `away_from_growth`, whose `(0.0, −1.0)` zero-neighbour
case is then reached exactly as it is today. So the first growth step of
every organism is unchanged, and `tree.ron`'s `continuation_weight: 0.7`
keeps its meaning and its tuning.

**And nothing else in the scoring formula moves.** `photo`, `wind`,
`gravity_or_water` and `crowding_weight` are untouched, including the
`RootTip`/`GrowingTip` split and the MIZ1 branch. That matters for the
re-tune: Decision 4 has to re-tune `cost`/`rate` regardless, and adding a
seventh weight to a six-weight blend at the same time would make the
comparison in `examples/debug_tree_variants.rs` unreadable.

### 7h. Worked example — why one channel wins, and why it does not today

`tree-rewrite-design.md` §2a set the standard here: a worked proof, not an
assertion. And `tree-rewrite-design.md` §3 set the honesty standard by
explicitly *withdrawing* revision 1's claim that plain diffusion produces
apical dominance. This section has to clear both bars, so it works the
same scenario twice — once with the shipped isotropic rule, once with
Decision 6's — and compares.

**Setup.** One stem cell `S` immediately below two `GrowingTip`s `A` and
`B`, created by the same `branch_chance` roll, symmetric in every respect.
`S` is fed from below by leaves and is held near `R_S = 1.0`. Total supply
delivered from `S` is `Q = 0.3` per organism tick — the scarce regime, and
deliberately so: two tips at `cost = 0.2` demand `0.4`, so supply limits
growth, which is exactly the regime `tree.ron`'s own tuning header
describes finding. Constants from §7e: `c_min = 1`, `Φ(J) = J²/(0.04 +
J²)` at `J_REF = 0.2`, `c ← 0.9c + 0.1 + 2.9·Φ(J)`.

The split between the two faces follows §7c directly: with a common
upstream `R_S`, each face's flux is proportional to `c · h` where `h = R_S
− R` is that tip's *hunger*. So

```
share_A = c_A·h_A / ( c_A·h_A + c_B·h_B )
```

**What this abstracts, said plainly so the example is not read as more
precise than it is.** The tables below work at the granularity of one
organism tick and treat `Q` as delivered in one lump, rather than
simulating `TRANSPORT_SUBSTEPS` iterations. That is legitimate here, and
only here, because **every claim in this section is about the *ratio*
`share_A : share_B`, and that ratio is set by `c·h` at every substep
alike** — the substep count changes how much total resource moves per
tick (which is why it is a tuning target, §7c) but not how it is divided
between two faces of the same cell. The absolute value `Q = 0.3` is
illustrative; the split is not.

**The one asymmetry, and it is not invented.** At `t = 1` both tips reach
`0.30` and both try to grow; `A` grows, `B` does not, because `B`'s
candidate set came back empty (`plant.rs`: `if candidates.is_empty() {
continue; }` — every open neighbour scored ≤ 0, a routine outcome in
crowded canopy). `A` spends `0.2` and drops to `0.10`; `B` keeps `0.30`.
Nothing else ever differs. **The whole question is what the system does
with one lost growth step.**

#### Run 1 — today's isotropic rule (`c_A = c_B` always)

| tick | h_A | h_B | share_A | R_A after | R_B after | grew |
|---|---|---|---|---|---|---|
| 2 | 0.90 | 0.70 | 0.563 | 0.069 | 0.231 | A, B |
| 3 | 0.931 | 0.769 | 0.548 | 0.033 | 0.167 | A, B |
| 4 | 0.967 | 0.833 | 0.537 | 0.195 | 0.106 | **B only** |
| 5 | 0.806 | 0.894 | **0.474** | 0.137 | 0.063 | A, B |

**By tick 5 the lead has flipped.** The reason is structural: hunger is a
*negative* feedback. The tip that grows spends, becomes less hungry
relative to the one that stalled, and hands the advantage straight back.
The system alternates. Nothing accumulates, because nothing in the
substrate remembers which face carried the resource.

**This is not a hypothetical — it is the honest description
`tree-rewrite-design.md` §3 already published**, that whether growth
produces "one clearly dominant leader, several co-dominant branches, or a
fairly even canopy is left genuinely emergent and variable." The
arithmetic above is *why* it came out that way, and it is the same reason
`plant-simulation-research.md` §5 gives: no flux term, no feedback, no
channel.

#### Run 2 — Decision 6's rule

Ticks 0 and 1 are symmetric, so both faces ratchet identically: `c = 1 →
2.044 → 2.984`. Then `A` grows and `B` does not, and the two diverge:

| tick | c_A | c_B | h_A | h_B | share_A | J_A | J_B | grew |
|---|---|---|---|---|---|---|---|---|
| 2 | 2.984 | 2.984 | 0.900 | 0.700 | 0.563 | 0.169 | 0.131 | A, B |
| 3 | 3.991 | 3.658 | 0.931 | 0.769 | 0.569 | 0.171 | 0.129 | A, B |
| 4 | 4.915 | 4.246 | 0.960 | 0.840 | 0.570 | 0.171 | 0.129 | A, B |
| 5 | 5.748 | 4.775 | 0.990 | 0.910 | 0.567 | 0.170 | 0.130 | **B only** |
| 6 | 6.490 | 5.258 | 0.820 | 0.980 | **0.508** | 0.152 | 0.148 | **A only** |

**Tick 6 is the whole argument.** `A` has just stalled and is markedly
*less* hungry than `B` — `h_A = 0.820` against `h_B = 0.980`. Under Run
1's rule that hunger gap alone determines the split and `A` would receive
`0.820/1.800 = 45.6%`, losing the lead exactly as it did at tick 5 there.
Under Decision 6, `A`'s face has ratcheted to `6.490` against `B`'s
`5.258`, and `6.490 × 0.820 = 5.32` still beats `5.258 × 0.980 = 5.15`.
**`A` takes 50.8% while being the less hungry tip, and grows again while
`B` stalls.** The stored conductance has converted a transient, self-
cancelling hunger lead into a persistent supply lead.

**Where this converges, stated honestly rather than extrapolated
optimistically.** The conductance ratio climbs 1.000 → 1.091 → 1.158 →
1.204 → 1.234 and is decelerating. Its fixed point is computable directly:
`c* = 1 + 29·Φ`, with `Φ_A ≈ 0.42` and `Φ_B ≈ 0.294`, giving `c_A* ≈ 13.2`,
`c_B* ≈ 9.5`, **ratio ≈ 1.38**. So the flux/conductance feedback on its
own does *not* run away to a single channel. It converges to a stable,
permanent ~57/43 supply bias — roughly a 1.3:1 growth-rate ratio between
the two branches, sustained indefinitely instead of oscillating.

**That is loop one. The winner-take-all is loop two, and it needs loop one
to exist.** A branch that grows 1.3× faster accumulates tips 1.3× faster,
and every additional downstream tip is another `cost` per tick of demand
drawing on the same face. Sink count enters the split the same way hunger
does: with `n_A` and `n_B` tips downstream, the standing resource on each
channel is depressed roughly in proportion to its own sink count, so

```
share_A  ≈  (c_A·n_A) / (c_A·n_A + c_B·n_B)
```

and growth-rate ratio ≈ share ratio ≈ `x = c_A n_A / (c_B n_B)`. So `n_A/
n_B` grows at rate `x`, which raises `x`, which raises the flux ratio,
which raises `c_A/c_B` through `Φ`, which raises `x` again. **Unlike loop
one, that map has no stable fixed point above `x = 1`: any `x > 1`
diverges.** Concretely, at the conductance ratio 1.38 and a subtree-size
ratio of only 1.3, `share_A` is already `1.79/2.79 = 64%`, well past the
57% loop one alone sustains.

**And loop two cannot start without loop one.** Run 1's table is the
proof: without stored conductance the lead flips every few ticks, so the
subtree-size ratio random-walks around 1.0 and never establishes a
direction for loop two to amplify. Conductance memory is what converts a
sequence of coin flips into a committed choice. That is Sachs's
hysteresis, and it is the same property Prusinkiewicz et al. (2009, PNAS
106:17431–17436, already cited in `plant.rs`'s module doc) identify as the
defining feature of the canalization switch: *hard to reverse once
established.*

**This is also exactly the Borchert–Honda allocation scheme that
`plant-simulation-research.md` §5 says polarity unlocks** — basipetal flux
accumulation followed by acropetal allocation, extended by Palubicki et
al. (2009, ACM TOG 28:58) — arrived at from a local per-face rule rather
than implemented as a two-pass whole-tree traversal. No cell ever computes
its subtree size; the conductance field is what carries that information,
because it is what the flux from that subtree wrote into it.

### 7i. What this produces, and what it does not — the walk-back, up front

Following `tree-rewrite-design.md` §3's precedent of stating the limit
*before* someone tests for the thing that was never claimed:

**Claimed, and supported by §7h:**
- A real conductance hierarchy between established transport paths and
  undifferentiated tissue (up to 30:1), i.e. vein-like structure.
- Source-to-sink transport: a leaf's carbon preferentially reaching
  whichever sink is actually drawing on it (§7j).
- A *persistent* growth-rate bias between sibling branches where today's
  mechanism produces an alternating one, and — via loop two — genuine
  divergence of subtree sizes over long runs.
- A trunk emerging as the highest-conductance path in the plant, because
  it is the only path between the largest source aggregate (canopy) and
  the largest sink aggregate (roots plus tips). This is Shinozaki's pipe
  model arrived at from transport rather than asserted, and it is
  *consistent with* `SecondaryThicken`'s independent leaf-count rule
  rather than a competing account of the same thing.

**Not claimed:**
- **This is not real apical dominance.** Real apical dominance is an
  *inhibitory* signal: auxin made at the apex flows basipetally and
  prevents lateral buds from canalizing into the main stream at all
  (`research/m16-plant-biology.md` §3–4). It flows in the **opposite**
  direction to photosynthate, and this design carries exactly one polar
  channel, in the source→sink direction. What §7h produces is competitive
  *allocation*, not suppression. A lateral here is under-supplied; it is
  not switched off.
- The seam for the real version is drawn and costs nothing to leave open:
  a second `[f32; 4]` and a second scalar on `OrganismCell`, sourced at
  `GrowingTip` and sunk at the base, running the *identical* update rule
  in the opposite polarity, feeding §2e's `BudBreak` threshold. That is a
  future decision, deliberately not made here — §8's out-of-scope list.
- No claim that any of the numbers above are tuned. They are internally
  consistent and dimensionally sane; `examples/debug_tree_variants.rs` is
  the authority, per §9's item 14.

### 7j. Interaction with Decision 4's leaves and Decision 1's bud break

**This is the main reason to build polarity at all, so it is stated
directly rather than left implicit.**

**Decision 4 makes a leaf a real source, and that changes what transport
has to do.** Today `GrowingTip` carries `Photosynthesize` itself
(`tree.ron`), so every sink is also its own source and transport barely
matters — a tip largely funds itself. After Decision 4, `Photosynthesize`
moves to `Leaf` only, and leaves sit *behind* the advancing tip along the
shoot. **The source and the sink are now different cells, and every unit
of carbon a tip spends has to be transported to it.** Transport stops
being a background smoothing pass and becomes the plant's circulatory
system.

Under isotropic diffusion that goes badly, and the failure is documented
in advance by `plant-simulation-research.md` §6: allocation becomes
*"distance-dependent for numerical rather than biological reasons"* — a
leaf's output spreads equally in all directions including backwards into
mature wood that is not drawing, and the fraction reaching the tip falls
off geometrically with path length. Tall plants would starve their
extremities *because the solver converges slowly*, which, in that
document's own words, *"will look like a biological result and won't be
one."*

**Under Decision 6 the leaf→tip face carries real net flux** — the tip
drains itself to near zero every time it grows, so the gradient is
persistently in that direction — **and ratchets, while the leaf→mature-wood
face carries little and decays back toward `c_min`.** A strand
differentiates between each leaf and the sink it is actually feeding. That
is Münch source-to-sink flow (§7a), and it is why the direction is not
hardcoded anywhere: a face's polarity is a consequence of which end is
currently the sink, so **when a tip retires to `MatureBody` and stops
spending, the flux across that face falls, its conductance decays, and the
same leaf's output redirects to whatever is still drawing — the root
system, or `SecondaryThicken`, or a newer tip.** Phloem in real plants
reverses direction for exactly this reason when a sink becomes a source or
stops drawing; nothing in the rule needs to know that it happened.

**This is precisely why Decision 6 must land before Decision 4's re-tune,
and the mechanism gives the reason `PLAN.md` states in the abstract.**
`tree.ron`'s `Photosynthesize.rate` and `Grow.cost` are calibrated against
*how much of a source's output actually reaches a sink per tick*. Under
isotropic diffusion that fraction is one number; under a 30:1 vein/
parenchyma contrast with 16 substeps it is a completely different number,
and it is different *by different amounts* for a seedling (no vasculature,
transport-limited) than for a mature tree (established strands,
transport-saturated). Tuning `rate`/`cost` against the isotropic economy
would produce values that are wrong in a size-dependent way the moment
polarity lands. §10 sequences accordingly.

**Decision 1's bud break gets a mechanism it did not have.** §2e defines
`BudBreak` on a `MatureBody` cell with *surplus* resource and *low* local
canopy density, and honestly notes that without canalization it will not
produce a single dominant leader. That limitation stands (§7i). But
polarity makes the *surplus* condition mean something specific: a cell
whose downstream faces have decayed to `c_min` — because whatever they
fed died, burned, was cut, or retired — **physically cannot export what it
receives, so resource accumulates there and nowhere else.** Bud break then
fires preferentially at the cell immediately upstream of a lost limb.

That is the real epicormic-resprouting observable §2e cites, and under
Decision 6 it is a consequence of the transport rule rather than a
coincidence of two thresholds: resource backs up at a wound because the
channel past the wound stopped carrying flux and decayed. §2e's own
sentence — *"a tree that loses a limb re-sprouts near the wound, because
that is precisely where downstream demand vanished and resource backs
up"* — describes a mechanism that only exists once conductance can decay.
Before Decision 6 it was an aspiration. **No change to Decision 2e is
required or made**; it simply becomes true.

### 7k. Migration, and what it does to the existing tests

Polarity lands as one step (§10, step 2), after Decision 2's four
sub-steps and before Decision 4. It is small because Decision 2 did the
hard part: the per-organism pass over `OrganismState::cells` already
exists, is already off the `CellSurface` trait, and already runs at its own
cadence.

- **`carbon_conductance: [f32; 4]`** added to `OrganismCell`, initialized
  to `c_min` at every cell-creation site Decision 2 §3f step 2a already
  enumerates. No new registration sites.
- **The transport pass** gains the pairwise form (§7c), the substep loop,
  the scratch flux buffer, and the end-of-tick conductance update (§7e).
- **`Grow`** swaps `away_from_growth` for `away_from_supply` with the
  documented fallback (§7g). One function, no new parameter, no `.ron`
  change.
- **`tree.ron` is not edited in this step.** That is the whole point of the
  sequencing: the re-tune happens once, in step 3, with polarity already
  running.

Tests, named individually because §3f set that precedent:

- `resource_diffuses_from_a_full_cell_toward_an_empty_same_organism_
  neighbour` **passes unchanged.** Both cells start at `c_min`, the rule
  reduces exactly to Fick (§7c), and resource still flows down the
  gradient. If it fails, the reduction is wrong and that is the bug.
- `resource_does_not_cross_an_organism_boundary` **passes unchanged** and
  keeps §3f's warning about not letting it become vacuous.
- `diffuse_resource_no_longer_decays_density_itself` **passes unchanged** —
  the density channel is bit-for-bit today's behaviour (§7f).
- **New, and the actual gate on this step:** a straight chain of cells with
  a source at one end and a drained sink at the other develops a
  conductance ratio well above 1 along the chain's axis versus across it,
  within a bounded number of organism ticks. This is the minimal
  observable proof that canalization is occurring at all.
- **New:** the §7h Y-junction, as a regression test with the tick-6
  assertion made explicit — the less-hungry-but-better-connected tip
  receives the larger share. **This is the test that fails if `Φ`, `J_REF`
  or the contrast ratio are mis-set**, and it fails in the specific,
  diagnosable way of the lead flipping, which is Run 1's signature.
- **New:** conductance is bounded — drive one face at saturating flux for
  many ticks and assert `c ≤ c_max`. §7a(2) is explicit that the unbounded
  form diverges; this asserts the bounded form was actually used.
- **New:** setting `VEIN_GAIN = 0` reproduces the isotropic results
  exactly. The A/B switch of §7c, made a test rather than a claim.

---

## 8. Deliberately out of scope

### 8a. Polarity — no longer out of scope; this section records the reversal

**Earlier revisions of this document deferred polarity/directional
diffusion here, on the owner's explicit mid-session instruction:** *"Let's
plan all of this before we start implementing any of it. I don't want to
optimize if we are going to make large changes to our diffusion
mechanism."* The reasoning given was that polarity changes the core
transport mechanism Decisions 1, 3 and 4 would each be tuned against, so
tuning against isotropic diffusion and then replacing it does the tuning
pass twice.

**That reasoning was right and the conclusion drawn from it was wrong, and
`PLAN.md`'s post-landing revision ("Revised after landing", at the end of
the `plant-substrate-v2-design.md` handoff entry) is where the correction
is recorded.** Deferring polarity to *after* this whole phase does not
avoid the double tuning — it guarantees it, because retrofit step 3 (real
leaves) re-tunes `tree.ron`'s entire resource economy, and that re-tune is
worth nothing if the transport mechanism underneath it changes afterwards.
Two further points from that entry, both correct: Decision 2 already has
to restructure `diffuse_resource`'s execution shape (off the generic
`CellSurface` trait, onto a per-organism pass), which is the same code
polarity has to change; and Decision 2 is also what gives a polarity field
room to exist at all, since the old packed `aux` had no spare bits.

**Polarity is therefore now Decision 6 (§7), sequenced between retrofit
steps 1 and 3** — after sidecar storage, before the leaf/reserve re-tune.
The seam this document originally left for it (*"a future polarity vector
is two more `f32`s in `OrganismCell` — no layout question, no bit budget,
no migration"*) turned out to be exactly right about the cost and slightly
wrong about the shape: §7b lands on four `f32`s per face, not a vector.

**Still out of scope after Decision 6**, and named so the list stays
honest: a second, oppositely-polarized auxin-like channel (§7i) — the
piece that would produce real inhibitory apical dominance rather than
competitive allocation; and directional transport of *water*, which needs
its own conductance array and is deliberately deferred to retrofit step 8,
where Decision 3B first gives water a real source (§7f).

### 8b. Evolution and genetics — future milestone, acknowledged only

Out of scope, per `PLAN.md`'s own framing (*"a real future milestone, not
this phase"*). Two things above were nonetheless shaped so it is not
foreclosed, and both were cheap: §5b's leaf `rate`/`lifespan` pairing gives
the leaf-economics trade-off a place to exist before selection needs it,
and Decision 2's per-organism `OrganismCell` table is the natural home for
per-organism trait variation, per `PLAN.md`'s standing constraint to prefer
per-organism state over new assumptions that every individual is identical.

---

## 9. Simplifications, stated honestly

*(This was §8 before Decision 6 was added; §7 is new and everything after
it shifted by one.)*

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
8. **Polarity produces competitive allocation, not apical dominance**
   (§7i). Real apical dominance is an inhibitory auxin signal flowing
   apex→base, the *opposite* direction to photosynthate; this design
   carries one polar channel, source→sink. A lateral branch here is
   under-supplied, not switched off. Simplification 2 above (bud break is
   not canalization) is narrowed but **not** retired by Decision 6: bud
   break now has a real mechanism for *why* resource backs up at a wound
   (§7j), and still no mechanism for suppressing a bud that is not at one.
9. **The flux→conductance loop alone converges; it does not run away**
   (§7h). Its fixed point is a ~1.38 conductance ratio and a ~57/43
   supply split between siblings. The divergent, winner-take-all part of
   the argument is the *second*, structural loop (subtree size → demand →
   supply share), and it is the one that has not been demonstrated by
   arithmetic here — only argued to have no stable fixed point above
   parity. **A long run is the only real evidence**, and §10's step 10
   gate is where it gets looked for.
10. **`Φ` is a Hill function with exponent 2, which is neither of the two
    forms the literature tests** (§7e). It is a deliberate hybrid: the
    quadratic's convex, competition-amplifying behaviour below `J_REF`
    (Mitchison's form, which Feller et al. 2015 show yields loopless
    directed trees) with a bounded form's ceiling above it (which the same
    paper shows is required, since unbounded `Φ` provably diverges). No
    source tests exactly this function, and no claim is made that it
    reproduces either paper's published patterns.
11. **The two-timescale split is asserted, not measured** (§7d). Bulk flow
    is updated `TRANSPORT_SUBSTEPS` times per organism tick and
    conductance once. The direction is right — carrier turnover is far
    slower than transport — but the ratio 16:1 is an engine number, not a
    biological one.
12. **Conductance is per-face and per-channel, and only carbon gets one**
    (§7f). Real tissue runs xylem and phloem as separate strands with
    opposite polarity; water keeps symmetric transport until retrofit step
    8 gives it a real source.
13. **Unpolarized tissue transports far more slowly than today's flat
    `DIFFUSION_RATE = 0.2`** (§7e) — `c_min · TRANSPORT_RATE = 0.008` per
    substep. This is intended (parenchyma is a poor conductor; that is
    what vascular tissue is *for*) but it is a real behavioural change to
    every seedling before it establishes a strand, and it is the most
    likely thing to read badly first.
14. **No claim is made that any of this reads well.** Every number above is
    a starting point for `examples/debug_tree_variants.rs`, and §10's
    verification gates are the actual authority — the same standard
    `tree-rewrite-design.md` §11 step 6 set and the tree rewrite honoured.

---

## 10. Retrofit order

*(This was §9 before Decision 6 was added — `PLAN.md`'s handoff entry cites
it under the old number. Polarity is now **step 2**, and every step after
it shifted by one.)*

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

2. **Decision 6 — polarity** (§7), immediately after 2d and **before
   anything re-tunes `tree.ron`.** This is the step whose placement the
   whole of §8a exists to justify: it needs Decision 2's sidecar to have
   somewhere to live and needs Decision 2's per-organism transport pass to
   have something to modify, and step 3 must not tune a resource economy
   against a transport mechanism that is about to be replaced. Scope is
   §7k: one field on `OrganismCell`, the pairwise transport rule with its
   substep loop and conductance update, and `Grow`'s `away_from_growth` →
   `away_from_supply` swap. **No `.ron` edits in this step at all.**
   **Gate:** the four new tests in §7k, and the Y-junction one is the real
   gate — the less-hungry-but-better-connected tip must take the larger
   share at §7h's tick 6. Also assert `VEIN_GAIN = 0` reproduces the
   isotropic results exactly, since that is what makes a regression here
   bisectable. **Do not screenshot-verify tree shape at this step**: with
   `Photosynthesize` still on `GrowingTip` (Decision 4 has not landed),
   every tip largely funds itself and transport barely matters, so the
   visible shape is expected to be nearly unchanged and proves nothing
   either way. The unit gates are the authority here; the visible payoff
   arrives in step 3.

3. **Decision 4** (§5), immediately after polarity. Chosen ahead of the
   soil work, for three reasons: it is the smallest change with the largest
   visible effect (visible leaves, and `SecondaryThicken` firing for the
   first time); it is what actually fixes the one-cell trunk (§2b); and it
   forces `free_organism` to exist, which Decision 3's necrosis path then
   reuses rather than reinvents.
   **Gate:** re-run `examples/debug_tree_variants.rs`. `cost`/`rate` **must**
   be re-tuned here — §5c explains why both halves of the existing tuning
   rationale are invalidated. **And this is the single tuning pass for the
   whole phase's resource economy, which is why polarity had to land
   first:** the variant sweep must now cover `TRANSPORT_SUBSTEPS` and the
   canalization contrast (§7e) alongside `cost`/`rate`/`reserve`, because
   after Decision 4 the source and the sink are different cells (§7j) and
   how much of a leaf's output reaches a tip is set by transport, not by
   `rate` alone. Tuning `cost`/`rate` here and revisiting transport later
   would be the exact double pass §8a's reversal exists to avoid.
   Live screenshots under `docs/screenshots/`, per standing practice:
   visible leaves, a trunk more than one cell thick, and a seedling planted
   in shade that dies rather than becoming an immortal stub. **One new
   thing to look for**, per §7e's own honest warning: a seedling that never
   establishes a strand and stalls before its first leaf means the
   contrast ratio or `TRANSPORT_SUBSTEPS` is wrong, not that the seed
   reserve is too small — check transport before re-deriving §5c.

**Safe in parallel with each other, once 1–3 land:**

4. **Decision 3, part A — soil moisture storage and drainage** (§4a, §4b,
   §4d's drainage rule, §4e's `mud`). Touches only inert-cell `aux`,
   `decay.rs` and `material.rs`. No organism code at all. Independently
   testable: soak a soil column, watch a wetting front descend, watch mud
   appear at the plastic limit.

5. **Decision 5, parts A and C — `leaf`/`rootwood` materials, and root soil
   stabilization** (§6a, §6d). `.ron` data plus one check in
   `update_powder`. Independent of everything above except that `leaf` as a
   material wants Decision 4's `Leaf` cells to exist to be visible.

6. **Decision 5, part B — load reduces span** (§6c). One term in
   `organism_is_supported`. Fully independent.

**Sequential again, and last:**

7. **Decision 1(ii) — root displacement into soil** (§2c). Needs Decision
   3A's per-cell moisture (to credit water on the way through) and
   Decision 5A's `rootwood`. **This is where roots become real for the
   first time** — `germinate()` (plant.rs:594) should also stop gating the
   companion `RootTip` on `world.is_empty(x, y + 1)`, which is why the test
   scene's stone floor produced no roots at all.

8. **Decision 3, part B — `Absorb` from soil, and anoxia necrosis** (§4c,
   §4d paths 2 and 3). Needs 7. This is the step that closes `PLAN.md`'s
   recorded `RootTip` income gap. **This is also where water becomes a
   real second currency with a real source, and therefore where §7f's
   second conductance array lands if it is wanted** — a `water_conductance:
   [f32; 4]` running the identical rule at the opposite polarity (root
   source → canopy sink). Optional at this step and explicitly not
   required by anything above it; symmetric water transport is a
   defensible stopping point.
   **Gate, and it is the interesting one:** plant a tree over a water
   table and confirm the root system stabilizes *above* the saturated zone
   rather than growing into it and dying wholesale, or stalling short of
   it. That emergent equilibrium (§4c) is the single best evidence the
   soil model is doing real work, and if it does not appear, the aeration
   threshold and `ANOXIA_LIMIT` are the two knobs.

9. **Decision 1(iv) — bud break** (§2e). Deliberately **last**, and
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
   **Second gate, added by Decision 6, and it is the one that tests §7h's
   central claim:** this is the first workload long enough for the
   structural loop to diverge. Over 50,000 ticks, sibling subtrees at an
   early branch point should show a *growing* size disparity, not a random
   walk around parity, and the conductance along the trunk should be
   visibly higher than along a minor branch. §9's simplification 9 is
   explicit that this loop is argued rather than demonstrated; **this is
   where it gets demonstrated or withdrawn.** Also cut a limb here and
   confirm the resprout appears near the wound (§7j) rather than uniformly
   over the canopy.

10. **Independent design review before commit**, per standing practice for
    a change this size, specifically re-checking: that 2a's cell-slot
    registration has no double-free path; that the organism-boundary
    diffusion test is not vacuous after 2c; that §5d's seedling death
    actually frees the organism id rather than merely stopping the
    schedule; and — added by Decision 6 — that the transport pass visits
    each shared face exactly once (§7c's `+x`/`+y`-only iteration is what
    makes it conserving, and double-visiting it would silently double the
    effective rate), and that `Grow`'s `away_from_supply` fallback (§7g)
    actually fires for a fresh organism rather than reading four floor
    values as a meaningful direction.

**Explicitly not in this pass:** a second, oppositely-polarized auxin-like
channel and the real inhibitory apical dominance it would give (§7i),
evolution (§8b), a `Liquid`-kind flowing mud (§4e), a resistance-network
transport solve, and Palubicki-style shadow-voxel light competition — the
last two carried over unchanged from `organism-substrate-design.md` §7's
own out-of-scope list. **Polarity itself is no longer on this list** — it
is Decision 6 (§7) and retrofit step 2; §8a records why it moved.
