# Prior art: how other games do structural failure and destruction

**Status:** research, no code changed. Written after `src/sim/load.rs` shipped
the net-section criterion (`torque > capacity` over the support forest), to
ask the question that was never asked before building it: *what have the
games that already solved this actually done, and where does our approach sit
among them?*

**Read `Reports/fracture-mechanics-design.md` and
`Reports/load-model-handoff.md` first** — this document assumes the model and
argues about it, it does not restate it.

**Evidence discipline.** This engine has been burned by reports taken on
faith (`CLAUDE.md`: "a mechanism whose advantage only appears at large width
… does have to actually *have* that advantage when measured, which is not
something to take from a report on faith"). So every claim below is tagged:

- **[V]** verified against a primary or near-primary source — a developer
  statement, an official wiki, or a talk.
- **[S]** secondary — a write-up of a developer talk, a well-maintained
  community wiki documenting shipped behaviour.
- **[U]** uncertain — community reconstruction, forum consensus, or a claim I
  could find asserted but not sourced. Treat as a hypothesis, not a fact.

The single most consequential thing in this document is §1's finding, and it
is uncomfortable, so it is stated first.

---

## 0. The headline

**The destruction games that are considered the best-feeling ship no
structural model at all.** Noita, Teardown and Deep Rock Galactic — three
different genres, three different engines, all praised specifically for how
destruction feels — each decline to compute support, stress or load. What
they compute is *connectivity*, and what makes the destruction feel good is
delivered by three other mechanisms entirely: material conversion on impact,
rigid-body promotion of disconnected pieces, and an authored debris size
ladder.

Meanwhile, **every game that does ship a real support/stress model is a
building game, and in all of them the system is a construction constraint
rather than a destruction mechanic** — 7 Days to Die, Rust, Medieval
Engineers, Vintage Story. Two of those four are widely reported as
frustrating, one ships the feature *disabled by default*, and one survives by
being a simple multiplicative decay the player can read as a percentage.

That is not an argument that our load model is wrong. It is an argument that
**the load model is not the thing that will make destruction satisfying**,
and that its main risk is the one the building games actually hit: a
structure that fails for a reason the player cannot see. §3 is about that.

---

## 1. The survey

| System | What propagates | Cost bound | What breaks off | How it reads |
|---|---|---|---|---|
| **Noita** | *Nothing.* No support model. | n/a | Marching-squares contour of a disconnected pixel region → Box2D body | Impact **converts** static stone into a "collapsing sand" powder; rigid bodies re-contour when cut |
| **Teardown** | *Nothing.* Connectivity only. | Flood fill claimed O(1) while nothing breaks | Disconnected voxel component → dynamic body | Small debris is culled by size for frame cost |
| **Deep Rock Galactic** | *Nothing,* by explicit design | n/a | Fully disconnected terrain → crumbles to dust and despawns | Pillar stands on one voxel of contact, forever |
| **Red Faction: Guerrilla** | "Stress" over a graph of authored building parts | Authored per-building, not per-voxel | Pre-fractured shards, sized per material | Buildings visibly sag and topple; shards hidden until activated |
| **UE5 Chaos** | *Strain* over a connection graph, plus leftover damage after a break | Hierarchical clusters; only the active level simulates | Cluster at the level whose damage threshold was exceeded | Descending thresholds per level ⇒ big pieces first, then smaller |
| **7 Days to Die** | Mass, against a per-material horizontal-support budget | Recomputed on place/remove | "All now-unstable blocks" collapse at once | Binary, immediate, and famously unpredictable |
| **Rust** | Stability as a multiplicative **percentage** decaying along the support path | Local recompute per placement | Piece fails to place, or falls | Player sees a literal `%` on the hammer |
| **Medieval Engineers** | Weight and **moment (torque)** | Unknown | Blocks over threshold | Overlay view (`N`); players report it as unpredictable |
| **Minecraft (scaffolding only)** | Horizontal distance to a grounded column, cap 6 | Trivially local | The whole column above a broken base | Deliberately toy-like, and used for exactly one block type |
| **Vintage Story** | Ceiling "instability", reduced by support beams | Unknown | Cave-in | **Off by default** |
| **Poly Bridge / Besiege** | Real forces — mass-spring (Hooke) / rigid bodies + breakable joints | Tiny graphs (tens to hundreds of nodes) | The joint that exceeded its break force | Per-member stress colouring is the whole UI |
| **Dwarf Fortress** | Connectivity, via maintained connected components | Incremental component index; flood fill only on change | Anything detached from all support | Cave-in as a discrete, announced event |
| **The Powder Toy** | Air pressure field | CA-local | Per-material pressure threshold (e.g. BRCK at ~8.8) | Threshold on a *field*, not on a support path |

### 1.1 Noita — the closest architectural relative, and it has no support model [S]

Noita's architecture is ours: a 64×64 chunk grid, a per-chunk dirty rect, and
a four-pass checkerboard update so threads touch every other chunk plus a
32-pixel cardinal margin without locks or atomics
([80.lv](https://80.lv/articles/noita-a-game-based-on-falling-sand-simulation),
[GDC 2019 talk](https://www.youtube.com/watch?v=prXuyMCgbTc)). That is
`parallel.rs` and `MAX_REACH == CHUNK_SIZE / 2`, arrived at independently.

What it does *not* have is any notion of support:

> "Static materials like stone don't naturally collapse. Instead,
> sufficiently large explosions trigger conversion of contacted pixels into
> 'collapsing sand materials'."

So Noita's destruction is a **material substitution**: a blast rewrites stone
into a powder that then falls under the ordinary CA rules. There is no
question of "is this held up" — the answer is always yes until something
converts it. Rigid bodies exist separately: each body's pixels know their
position within it, and "if a pixel that belongs to a rigid body gets
destroyed, the simulation recalculates the shape for the rigid body (or rigid
bodies if the shape is now cut in two or more)". Contours come from marching
squares [S].

**What this means for us.** Noita gets the "cracks, throws debris, comes
apart" feel from *conversion plus rigid-body re-contouring*, and pays nothing
for support. We already have the equivalent of both halves (`break_free`'s
`breaks_into`, and `rigid::fracture`/`ChunkBody`). The load model is a
capability Noita does not have and did not need for feel — it is what buys
*mining and building*, which Noita does not do.

### 1.2 Teardown — connectivity, and a deliberate refusal [V/U]

Dennis Gustafsson, on the cost of the connectivity check:

> "All physics and destruction in Teardown is computed on the CPU. The flood
> fill of large objects was definitely one of the major bottlenecks, but this
> new algorithm takes it to **O(1) as long as something doesn't break**,
> which is the most common case."
> — [@voxagonlabs, 2021-02-27](https://twitter.com/voxagonlabs/status/1365546378374561792)

Quoted from a search result title; a direct fetch of the tweet returned HTTP
402, so treat the wording as near-verbatim rather than certain. The
*structure* of the claim is the important part and it is unambiguous: the
cost is paid **on change**, not per frame, and the steady state is free.

On structural integrity, Gustafsson has described it as a wanted-but-hard
feature rather than a shipped one — the difficulty being that Teardown scenes
hold hundreds of millions of voxels, and the analysis would have to work out
what each is carrying, along different material lines
([80.lv interview](https://80.lv/articles/teardown-developer-breaks-down-multiplayer-and-voxel-destruction-tech))
[V]. Community consensus is that the omission is also a *design* choice —
that unpredictable collapses would work against the player's ability to plan
a heist — but I could not find that from the developer directly, only
second-hand in a [Steam
thread](https://steamcommunity.com/app/1167630/discussions/0/3075377162316153906/)
[U]. Players in that thread complain about exactly the resulting artifact: "a
2-wide voxel pole holding up an entire house."

**Teardown also culls debris by size** — small voxel pieces despawn when out
of view, and the community's performance mods are all size-threshold knobs on
that behaviour ([Dynamic Debris
Removal](https://steamcommunity.com/sharedfiles/filedetails/?id=2677340659))
[S]. The grit tier is the expensive one everywhere, not just here.

### 1.3 Deep Rock Galactic — the rule stated as a design decision [V]

From the official wiki:

> "There is no calculation of structural integrity or tensile strength for
> terrain in the caves of Hoxxes… a pillar of rock will stand as long as
> there is even a tiny bit connecting it to a floor, a wall or a ceiling, no
> matter the size. However, if a piece of terrain becomes completely
> disconnected from the rest of the cave, it immediately crumbles to dust and
> disappears."
> — [DRG wiki, Terrain](https://deeprockgalactic.wiki.gg/wiki/Terrain)

A mining game — our exact genre — that shipped connectivity-only and is
widely liked for how digging feels. Note the second half: disconnected
terrain **crumbles to dust and despawns** rather than falling. They did not
even pay for the falling body.

### 1.4 Red Faction: Guerrilla — the one that really did propagate stress [U]

GeoMod 2.0 is described everywhere as a "stress-based collapse model" over
buildings built from separate parts (girders, panels, pipes), with
**pre-broken meshes** that activate on destruction
([Red Faction wiki](https://www.redfactionwiki.com/wiki/Geo-Mod_2.0)). I could
not find a primary technical description of the propagation — the GDC 2009
talks that exist in the Vault are level-design talks, not the tech talk —
so the algorithm itself is **[U]**.

The *authoring* side is documented and is the more useful lesson. From the
Armageddon post-mortem interview
([Game Developer](https://www.gamedeveloper.com/design/the-destructible-world-building-i-red-faction-armageddon-i-))
[V]:

- Buildings carry **invisible pre-authored shards** that stay dormant until
  destruction fires, so the intact building keeps a high-res mesh and pays no
  shard cost.
- **Shard size is set per material.** Concrete and fibreglass fracture at
  different scales. The team had previously over-fractured and produced
  hundreds of thousands of polygons.
- Design lesson stated outright: making *some things indestructible* improved
  the feel — "when you suddenly encounter something you can't destroy, it
  suddenly brings it back to being a game again."

That third point is the opposite of the instinct this project has been
following, and worth sitting with.

### 1.5 UE5 Chaos — the best answer to "how do you get a size distribution" [V/S]

Chaos is the most directly transferable system in this list, and none of its
ideas are in our engine yet.

- A **Geometry Collection** is a *hierarchy* of pre-fractured pieces: large
  pieces that break into smaller pieces that break into smaller pieces again.
- Each level has its own **damage threshold**, set descending across levels,
  so higher (larger) levels fracture first as damage rises and the smaller
  levels only come apart under more.
- Pieces are joined by a **connection graph** of rigid constraints carrying
  **strain** values; the simulation evaluates strains in that graph.
- **Break damage propagation:** when a piece breaks, the *leftover* damage
  carries into the connected pieces. If a wall of strength 100 is hit for
  200, 100 remains and is pushed into the neighbours. "Shock damage
  propagation" is the separate knob controlling how much a large piece
  absorbs before a small piece calves off it.

Sources: [Destruction
Overview](https://dev.epicgames.com/documentation/unreal-engine/destruction-overview),
[Chaos Destruction
Overview](https://dev.epicgames.com/documentation/en-us/unreal-engine/chaos-destruction-overview?application_version=4.27),
[Chaos Destruction
Optimization](https://dev.epicgames.com/documentation/en-us/unreal-engine/chaos-destruction-optimization).

Two things here matter to us. First, **the size distribution is authored as
levels, not derived from a physical model** — and Chaos is a AAA destruction
system that could afford to derive it. Second, **the propagation is
event-driven from the break outward, carrying leftover damage**, rather than
a standing field that must be kept correct. That is a fundamentally cheaper
shape than "keep every cell's load current", and it is the shape §3.3
recommends.

### 1.6 The building games — support as a construction constraint

**7 Days to Die** [V, from the [official
wiki](https://7daystodie.wiki.gg/wiki/Structural_Integrity)]. Three per-block
numbers: `mass`, a binary `vertical support`, and `horizontal support` — the
total mass a side face can carry. Vertical support is *unconditional*: "blocks
that are both vertically supported and capable of providing vertical support
can hold an unlimited number and mass of blocks in a column above them", and
vertically supported blocks never collapse. Horizontal chains spend a budget:
a wood cube has mass 5 and horizontal support 40, hence roughly 8 blocks of
cantilever. On violation, "all now-unstable blocks will immediately collapse
and fall to the ground."

Note the shape: *vertical is free and unlimited; horizontal spends a budget
along the chain.* That is `support_cost_below` vs `support_cost_beside` with
`support_cost_below: 0` — i.e. **exactly the model this project deleted** in
the load change. 7D2D shipped it and it is the most complained-about system in
the game (§2.3).

**Rust** [S, from [wiki](https://rust.fandom.com/wiki/Stability) and a
community [algorithm
guide](https://steamcommunity.com/sharedfiles/filedetails/?id=720344213)].
Stability is a percentage: ground = 100%; a solid wall multiplies by ~0.86, a
support wall by ~0.70; a floor's stability is the mean of its four supporting
sides times the floor below's. Below 5% it cannot be placed. Cap around 19
floors. The exact constants are community-derived **[U]**, the *shape* is
documented **[V]**: a monotone multiplicative decay along the support path.

The important design detail is that **the number is on the HUD.** Mouse over
any piece with a hammer and read the percentage. A player learns the rule by
playing, not by reading a wiki, and the rule is simple enough that the number
is predictive. Rust's system is the least physical on this list and by a wide
margin the least complained about.

**Medieval Engineers** [U, from community feedback threads]. Community
reconstruction says it used block weight, per-material multipliers, and
**moment/torque** — "a horizontal beam causing fractures at pivot points"
([support thread](https://support.keenswh.com/medievalengineers/topic/structural-integrity)).
There is a structural-integrity overlay bound to `N`. Player reports:
buttresses "don't seem to take weight from the legs; instead they seem to be
pushing into the legs"; wooden supports "don't seem to take away structural
load"; walls get "heavy" for no visible reason
([Steam](https://steamcommunity.com/app/333950/discussions/0/618453594765397168/)).

**This is the closest published analogue to our model, and it is the one
whose players could not predict it.** That is worth more weight than any
other single data point in this document.

**Vintage Story** ships cave-ins **deactivated by default**
([wiki](https://wiki.vintagestory.at/Support_beam)) [V]. Support beams reduce
ceiling instability, one beam per block of span. The numbers are not
published.

**Minecraft** uses a support model for exactly one block: scaffolding, which
can be placed at most **6 blocks horizontally** from a column that reaches
solid ground, and the whole column collapses when its base breaks
([wiki](https://minecraft.wiki/w/Scaffolding)) [V]. That is
distance-to-anchor with a hard cap — precisely the criterion this project
just replaced — and Minecraft applies it to a deliberately toy-like temporary
block and to nothing else. Sand and gravel fall on "nothing beneath", full
stop.

### 1.7 Real solvers — and why they only appear in tiny graphs [S]

Poly Bridge is a mass-spring system: nodes have unit mass, beams are
weightless springs obeying Hooke's law, and a member breaks past a stress
limit; ropes carry tension only, not compression
([Steam
discussion](https://steamcommunity.com/app/367450/discussions/0/483366528923135119/)).
Besiege is Unity rigid bodies joined by breakable joints. Bridge Constructor
is the same family.

Every one of them runs on **tens to hundreds of members**, authored by the
player, with a build/simulate split that lets the solver run only on demand.
None of them is a continuum, none is 160,000 cells, and none of them runs a
solver every frame over an open world. The genre's actual contribution to us
is the **UI**: per-member stress colouring is not a debug view in these
games, it is the primary interface. Poly Bridge even has a mod purely to
separate tensile from compressive stress in the display
([StressTypeVisualization](https://github.com/56kyle/StressTypeVisualization)).

### 1.8 The one useful independent write-up [V]

Zarkonnen's ["Structural Integrity in Block
Games"](https://zarkonnen.com/structural_integrity_in_block_games) is a
worked toy of the design space and lands where the shipped games do: a
**strength scalar that degrades as it flows** from block to block, with
different loss for vertical, horizontal and diagonal transmission per
material, plus a bonus for double-thickness walls. The author is explicit
that the implementation is naive and that a real one would "only recalculate
local effects when a block is added/removed."

Same substrate again: a monotone scalar on a path, recomputed on edit.

---

## 2. The four questions

### 2.1 Is a support-tree / shortest-path forest the right substrate?

**Yes, and it is the mainstream choice — but the mainstream stops one step
short of where we went.**

Every shipped block-game system in §1.6 propagates a **monotone scalar along
a support path**: Rust's multiplicative percentage, 7D2D's spent horizontal
budget, Minecraft's hop count, Zarkonnen's degrading strength, Vintage
Story's instability. Our `structural.rs` distance relaxation is that same
object, and the choice of a *shortest-path forest with per-direction costs*
is if anything a cleaner formulation than any of them, because it makes
"which way is downhill" an explicit, deterministic, tie-broken function
rather than an implicit search order.

The alternatives, scored at our scale:

- **Flood-fill connectivity only** (Noita, Teardown, DRG, Dwarf Fortress).
  Cheapest, most predictable, and the choice of every game praised for
  destruction *feel*. It cannot express "this is overloaded", so it cannot
  make a worked root give way — which is the specific complaint that
  motivated `load.rs`. It is not a fit for a game about mining and building,
  but it is the right *fallback* whenever the load answer is unavailable, and
  `load.rs` already treats it that way (`is_supported`'s bounded flood).
- **Per-cell stress field** (The Powder Toy's pressure). Works when the
  quantity genuinely diffuses locally and has no direction. Bending moment is
  neither — it is defined relative to a support point, so a scalar field
  cannot represent it without a companion "which way is support" field, which
  is the forest again.
- **Real solver** (Poly Bridge, Besiege). Dead end at our scale. 512×320
  today, streaming later; even at 1% structurally interesting cells that is
  thousands of unknowns per frame with no author-imposed structure. Nobody
  ships this over an open world.

**Where we differ from all prior art:** we accumulate a *second-order*
quantity (moment, `|Sx − x·M|`) up the forest, not a first-order one. Only
Medieval Engineers is reported to have done anything like it in a block game
[U], and it is the one that players found unreadable. See §2.3 — the risk
here is legibility, not correctness.

### 2.2 Load is non-local. How does prior art avoid paying O(N) per edit?

Four distinct answers, all of them worth stealing, and one of which we
already have.

**(a) Pay only on change; make the steady state free.** Teardown's O(1)
claim, Zarkonnen's "only recalculate local effects when a block is
added/removed", Dwarf Fortress's maintained connected-component index that
"is pretty easy to update even when the map changes quickly"
([Stack Overflow
interview](https://stackoverflow.blog/2021/12/31/700000-lines-of-code-20-years-and-one-developer-how-dwarf-fortress-is-built/))
[V]. **We already do this** — `scheduler`'s active sites, the label-correcting
relaxation that dies out when nothing changes, and `Cache` cleared per frame.
This is our strongest alignment with prior art.

**(b) Propagate the *event*, not the state.** Chaos's break-damage
propagation is the sharpest version: nobody recomputes global strain after a
break. The break emits its leftover damage into the connection graph and that
is the entire cascade mechanism. The cost is proportional to *what broke*,
not to what exists.

Our current answer to the same problem is `failing_along_support_chain`,
which walks up to `ROOTWARD_CHECK_STEPS = 128` ancestors from a settling
cell, re-evaluating each. That is the *reverse* direction — pull rather than
push — and it is the thing that makes a failure non-local in the direction
the player cannot see (§2.3). Push-from-the-break is both cheaper and more
legible.

**(c) Coarsen the graph.** Red Faction runs stress over authored building
*parts*, not voxels. Chaos runs over cluster levels, and only the currently
active level simulates. Neither propagates through the finest representation
they own. Our closest analogue is `is_structurally_interesting`'s early-out,
which reduces cost from volume to surface — a good reduction, but it is a
*filter*, not a coarsening: the surface of a big structure still costs
proportional to its size.

**(d) Bound it and accept the error, in the direction that fails safe.**
`MAX_SUBTREE_CELLS`, `MAX_SUPPORT_WALK` and `MAX_LOAD_CELLS_PER_FRAME` are
this, and the choice to truncate *downward* (understate torque, never
overstate) is the same conservatism DRG chose by never failing anything at
all. Prior art supports the direction. It does not support the side effect —
see §4.2.

### 2.3 7 Days to Die and Medieval Engineers both shipped stability systems players disliked. What went wrong?

Two different failures, and **we are currently set up for both.**

**7D2D: the invisible non-local dependency.** The A17 complaint pattern is
consistent and specific — bases collapsing days after being built, with no
change the player made ([Steam
1](https://steamcommunity.com/app/251570/discussions/1/3106892784350386156/),
[Steam
2](https://steamcommunity.com/app/251570/discussions/0/1694969274386684542/))
[S]. One diagnosis that recurs: *"sand or ore deposits anywhere between
bedrock and support structures would throw off the structural integrity
algorithm, especially noticeable on tall bases"* [U]. That is a failure whose
cause is tens of blocks away from its effect, through terrain the player did
not build and cannot see. The 7D2D wiki also documents a variant where the
game permits an over-supported placement and the collapse fires later when a
player *walks* on it [V] — the trigger and the cause separated in time as
well as space.

**Direct prediction for us.** `failing_along_support_chain` walks 128
ancestors and returns the first failing one, and `is_structurally_interesting`
means a load reading can change because a cell 128 steps away gained a free
face. When a player strikes at position A and rock falls at position B a
hundred cells away, that is the 7D2D bug, arrived at from a more defensible
model. Our per-frame budget makes it worse, not better: `ChainVerdict::
Deferred` means the collapse can also fire *frames later than the blow*, so
cause and effect are separated in time too.

**Medieval Engineers: torque without legibility.** Players' complaints are
not "it collapses too easily" — they are *"buttresses seem to push into the
legs"* and *"wooden supports don't seem to take away structural load"* [U].
Those are complaints about a **mental model that will not form**. Torque is
not intuitable from a screenshot: it depends on lever arms, on which way
support is coming from, and on section depth squared, none of which is
visible in the rendered rock. The game had an overlay (`N`) and it did not
save it, which suggests an overlay alone is necessary and not sufficient —
Rust's number is on the *hammer*, in the ordinary flow of play, not behind a
toggle.

**What it predicts for us.** Our capacity formula is
`base × section² × attachment × uncracked/4`, with `section` measured
*perpendicular to the parent direction*. Two visually identical cells can
differ in capacity by a factor of 1,600 (the `MAX_SECTION` ceiling) purely
because their support arrives from a different side. That is precisely the
Medieval Engineers failure shape. It is defensible physics and it is
unguessable by eye.

The counter-example is instructive: **Rust survives with a much worse
physical model because the player can read the number and predict the
outcome.** Given the ethos in `Reports/design-philosophy.md` §0a, legibility
is not a nice-to-have here, it *is* the requirement.

### 2.4 How do games get a satisfying size distribution of debris?

**They stack two or three mechanisms, each producing one size class. Nobody
derives the distribution from a single physical model.**

- **Red Faction:** shard scale is authored **per material**. Concrete
  fractures at one size, fibreglass at another. Over-fracturing was an
  explicit, costly mistake they had to walk back [V].
- **Chaos:** a fracture *hierarchy* with a descending damage threshold per
  level. Light damage calves a few big pieces; heavier damage carries into
  the next level down and produces many smaller ones. The distribution is a
  direct function of how much damage arrived [V].
- **Noita:** two mechanisms with no shared parent. Rigid-body promotion via
  marching squares gives the *chunk* tier; explosion-driven conversion of
  stone into "collapsing sand" gives the *grit* tier [S]. There is no middle
  tier and nobody minds.
- **Teardown:** small debris is culled by voxel count, and the community's
  performance mods are entirely thresholds on that [S]. The grit tier is the
  one that costs, everywhere.

**Where we already are, and it is good.** `rigid::fracture_with_impulse`
draws a fragment target from `1 << (1 + rng.below(5) + size_bias)` — 2 to 32
cells, log-uniform in the exponent, so each doubling is half as likely per
cell consumed. Anything under `MIN_BODY_CELLS = 8` becomes rubble rather than
a body, so the grit tier falls out of the same draw. `size_bias(radius)`
shifts the whole ladder for a heavier blow.

That is a heavy-tailed draw and it is *already* the mainstream shape,
arrived at independently. Two upgrades prior art suggests:

1. **Make the ladder material-dependent** (Red Faction's per-material shard
   size). Stone and wood should not fracture at the same scale, and it is a
   `.ron` field, which is exactly what
   `Reports/design-philosophy.md` §2 says such a constant should be.
2. **Drive the bias from delivered energy, not just brush radius** (Chaos's
   descending thresholds). Today `size_bias` reads the brush radius only, so
   a collapse under its own weight and a hard blow produce the same ladder.
   The excess `torque − capacity` at the failing cell is a delivered-energy
   proxy we already compute and currently discard.

---

## 3. Recommendations

Ordered by expected effect on *satisfying*, with effort and frame cost. Frame
cost is stated against the `examples/ascii.rs` worst-frame number, which is
the figure `CLAUDE.md` says to quote — none of these have been measured, and
saying so is the point.

### 3.1 Make failure legible before making it more correct — **S**

Cost: near zero in the common case; a debug overlay costs the dirty-rect skip
*only while enabled*, which `CLAUDE.md` already warns about for the animated
grain.

Three parts, in order:

1. **A stress-ratio overlay behind a key**, colouring every structurally
   interesting cell by `Load::stress()`. `load.rs` already computes it and
   `Load::stress` already exists with no consumer beyond the inspector. This
   is Poly Bridge's primary UI and Medieval Engineers' `N` view, and per
   `CLAUDE.md` ("for 'does this look right', ship a runtime selector") it is
   the cheapest way to settle arguments this project keeps having in prose.
2. **The break must originate visibly at the neck.** When
   `failing_along_support_chain` returns a failing ancestor, the fracture,
   the impulse and the debris should read as starting *there*. Right now
   `fracture_with_impulse` centres its pressure impulse on the region's
   bounding box, which for a subtree failure is the middle of the falling
   piece, not the joint that gave way. Moving the impulse origin to the
   failing cell is a small change and it is the difference between "the neck
   snapped" and "some rock fell".
3. **Report the count next to the image.** `CLAUDE.md`'s own rule — a
   contact sheet cannot distinguish a subtree failure from an unsupported-
   region failure, and those are two different mechanisms that will need
   telling apart constantly. `filmstrip` should print how many of each fired.

### 3.2 Bound failure by *locality to the disturbance*, not by step count — **S**

Cost: strictly negative (less work).

`ROOTWARD_CHECK_STEPS = 128` was set from the geometry of `scene=ligament`,
which is right for that scene and is also the 7D2D bug's shape: a blow can
bring down rock a hundred cells away. Prior art's answer is that a collapse
the player did not cause is worse than a collapse that does not happen —
DRG makes that trade absolutely, Teardown makes it deliberately [U], Vintage
Story makes it by defaulting the feature off.

Concretely: the chain walk should stop when it leaves the neighbourhood of
what actually changed. Either carry the disturbance origin into the walk and
bound by distance from it, or bound by "steps since the last cell whose
distance changed this tick". The far end of a genuinely overloaded ligament
still fails — it just fails when the wavefront reaches it, one structural
tick later, which is `Reports/fracture-mechanics-design.md` §3.4's
progressive collapse anyway.

**This directly conflicts with the 128 constant's own doc comment**, which
records that 16 was too small and let `scene=ligament`'s neck stand at a
stress ratio of 1.87. So this must be verified against that scene by eye
before it is believed, and if it re-breaks it, the right answer is a
disturbance-anchored bound rather than a smaller number.

### 3.3 Push damage from the break instead of pulling loads to the root — **M**

Cost: expected to be a large *reduction*. The rootward walk currently runs on
every settling cell in a disturbed structure; a push runs once per break.

This is Chaos's break-damage propagation, and it is the single most
transferable idea in this document. When a cell fails, it carries surplus
`torque − capacity` outward into its neighbours in the support graph, where
it adds to their effective load and can carry them too. The cascade becomes a
consequence of the break rather than of re-evaluating everyone, and it gives
three things at once:

- **Bounded cost** proportional to the failure, not to the structure.
- **A graded size distribution for free** — a small surplus takes a few
  cells, a large one carries several levels out. That is §2.4's
  "descending thresholds" without an authored hierarchy.
- **Locality by construction** — the cascade spreads outward from a visible
  event, which is §3.2's property arriving as a property of the model rather
  than as a cap.

Keep the load model as the *criterion* for the first break. Replace the
rootward re-evaluation with the push. Expect this to be the change that makes
a collapse read as one event rather than as a stutter of independent
failures.

### 3.4 Material-scaled fragment ladder — **S**

Cost: nil.

Move the `1 + rng.below(5)` exponent range into `.ron` per material (Red
Faction's per-material shard size). Stone calving at the same scale as wood
is a visible tell, and this is `design-philosophy.md` §2's "a tuning constant
becomes `.ron` data immediately" case exactly.

### 3.5 Drive `size_bias` from delivered surplus, not brush radius — **S/M**

Cost: nil to compute (the surplus is already in hand at the failure site);
possibly *more* bodies at high energy, which costs render and `step_chunk_
bodies` time.

Today a gentle collapse and a heavy blow produce the same ladder unless the
blow came from a wide brush. Chaos's whole visual grammar is "more damage ⇒
finer break". Feed `(torque − capacity) / capacity` into `size_bias` and a
barely-failing shelf calves slabs while a hammered one shatters.

### 3.6 Cull grit, don't compute it forever — **S**

Cost: a reduction in the settled-world case, which per `CLAUDE.md` is exactly
where the dirty-rect skip earns its keep.

Teardown despawns small debris by size threshold and its performance mods are
all knobs on that. `Reports/load-model-handoff.md` §7 already flags that
`render.rs` forces a full-screen redraw while any chunk body exists, which
gets worse as fracture produces more fragments. The two fixes belong together:
dirty only each body's bounding box, and let sub-`MIN_BODY_CELLS` rubble be
cheap and, if necessary, transient.

### 3.7 A player-facing stability readout — **M**, and only if building becomes a real pillar of the game

Cost: an overlay's cost only while shown.

Rust's percentage on the hammer is the only stability system in this survey
that players actively engage with rather than fight. If "build houses and
castles" is to be a first-class verb, the player needs a predictive number
in the ordinary flow of play, not behind a debug key — and it should be
something a person can reason about, which `stress()` as a 0–1 ratio already
is. This is listed last because it is a game-design commitment, not a
simulation change.

---

## 4. What prior art says is a dead end, or a trap we are standing in

### 4.1 A real solver, at any point on our roadmap — **dead end**

Nothing in the survey runs a stiffness solve over an open world. The games
that solve properly (Poly Bridge, Besiege) run on player-authored graphs of
tens to hundreds of members with an explicit build/simulate split. At 512×320
growing to streamed chunks, this is not a tuning problem, it is a category
error. Do not revisit.

### 4.2 Truncation that makes *bigger* structures *safer* — **a trap, live now**

`MAX_SUBTREE_CELLS` truncates the accumulation and records the truncated node
as carrying only itself, deliberately understating torque so the failure mode
is "terrain holds" rather than "mountain falls". The direction is right and
matches DRG's conservatism.

The side effect is not. A structure whose subtree exceeds the cap has its
neck's torque understated, so **the larger the overhang, the less likely its
neck is judged overloaded.** `CLAUDE.md` records this exact shape as having
been written twice already: "a size cap must bound work, never gate whether
something happens… any `if too_big { return }` is a claim that the largest
cases deserve the least behaviour." This is the third instance, wearing a
different hat — it does not gate the *decision*, it degrades the *evidence*,
which produces the same size-inverted behaviour.

**Incidental finding while reading, flagged and not acted on:** the two uses
of `MAX_SUBTREE_CELLS` are not measuring the same thing. `supported_subtree`
tests `out.len() >= MAX_SUBTREE_CELLS`, a count of cells. `subtree_sum` tests
`stack.len() > MAX_SUBTREE_CELLS`, the depth of the pending work stack — a
frontier width, not a cell count, and its doc comment describes it as "cells
one subtree walk may visit". Worth a look; it changes what the truncation
above actually bounds.

The prior-art-informed fix is §3.3: if the cascade is a push from the break,
nobody needs a whole subtree's total, and the cap stops being load-bearing.

### 4.3 `support_cost_below: 0` — **correctly deleted, and 7D2D is the evidence**

7 Days to Die ships free unlimited vertical support and budgeted horizontal
support, and it is the most-complained-about part of that game. The load
model's deletion of the zero cost was argued from internal reasoning
(`Reports/load-model-handoff.md` §4); prior art independently agrees. Do not
reintroduce it.

### 4.4 Capacity terms invisible in the render — **a trap, live now**

`section²` measured perpendicular to the parent direction, and
`attached_span_bonus`, can differ by three orders of magnitude between two
cells that draw identically. Medieval Engineers is the recorded case of what
happens next. This does not mean the terms are wrong — it means **any term in
`capacity` that the player cannot see must be visible in §3.1's overlay**, or
it will be experienced as randomness. `attached` in particular is a *bit on
the cell*, invisible in the render, and it multiplies capacity by a
material-defined factor. If a player cannot tell background rock from
foreground rock at a glance, they cannot predict anything.

### 4.5 Indestructibility as a feature — **worth reconsidering, and it cuts against our instinct**

Red Faction's own post-mortem says limits on destruction improved the game.
DRG's pillar stands on one voxel forever, on purpose. Vintage Story ships
cave-ins off. Teardown declines structural integrity partly to keep the
player's plan valid [U].

This project's whole direction is the opposite — everything simulated,
nothing exempt. That is a legitimate design position and it is the owner's.
But the reason `attached` exists is already a partial concession to it, and
the survey suggests the concession should be *stated as a design tool* rather
than treated as a modelling compromise: bedrock, deep background mass and
possibly specific structures being unbreakable is what makes the breakable
things read as breakable.

---

## 5. Sources

Grouped by system. Tags as in the header.

**Noita** — [GDC 2019, "Exploring the Tech and Design of
Noita"](https://www.youtube.com/watch?v=prXuyMCgbTc) [V, not watched in full
for this report]; [80.lv
write-up](https://80.lv/articles/noita-a-game-based-on-falling-sand-simulation)
[S]; [talk notes](https://braindump.jethro.dev/posts/gdc_vault_exploring_the_tech_and_design_of_noita/) [S].

**Teardown** — [@voxagonlabs on flood fill
cost](https://twitter.com/voxagonlabs/status/1365546378374561792) [V, quoted
from search snippet; direct fetch returned 402]; [80.lv
interview](https://80.lv/articles/teardown-developer-breaks-down-multiplayer-and-voxel-destruction-tech)
[V]; [Steam thread on structural
integrity](https://steamcommunity.com/app/1167630/discussions/0/3075377162316153906/)
[U]; [Dynamic Debris Removal
mod](https://steamcommunity.com/sharedfiles/filedetails/?id=2677340659) [S].

**Deep Rock Galactic** — [official wiki,
Terrain](https://deeprockgalactic.wiki.gg/wiki/Terrain) [V].

**Red Faction** — [Geo-Mod 2.0
wiki](https://www.redfactionwiki.com/wiki/Geo-Mod_2.0) [U]; ["The Destructible
World: Building Red Faction:
Armageddon"](https://www.gamedeveloper.com/design/the-destructible-world-building-i-red-faction-armageddon-i-)
[V].

**UE5 Chaos** — [Destruction
Overview](https://dev.epicgames.com/documentation/unreal-engine/destruction-overview)
[V]; [Chaos Destruction
Overview](https://dev.epicgames.com/documentation/en-us/unreal-engine/chaos-destruction-overview?application_version=4.27)
[V]; [Optimization](https://dev.epicgames.com/documentation/en-us/unreal-engine/chaos-destruction-optimization) [V].

**7 Days to Die** — [official wiki, Structural
Integrity](https://7daystodie.wiki.gg/wiki/Structural_Integrity) [V]; A17
complaint threads
[1](https://steamcommunity.com/app/251570/discussions/1/3106892784350386156/),
[2](https://steamcommunity.com/app/251570/discussions/0/1694969274386684542/),
[3](https://steamcommunity.com/app/251570/discussions/0/1742229167192111950/)
[S/U].

**Rust** — [wiki, Stability](https://rust.fandom.com/wiki/Stability) [V];
[community algorithm
guide](https://steamcommunity.com/sharedfiles/filedetails/?id=720344213) [U].

**Medieval Engineers** — [structural integrity feedback
thread](https://support.keenswh.com/medievalengineers/topic/structural-integrity)
[U]; [Steam, structural integrity & weight
distribution](https://steamcommunity.com/app/333950/discussions/0/618453594765397168/)
[U]; [wiki](https://medievalengineerswiki.com/w/Structural_Integrity) [S].

**Minecraft** — [wiki, Scaffolding](https://minecraft.wiki/w/Scaffolding)
[V].

**Vintage Story** — [wiki, Support
beam](https://wiki.vintagestory.at/Support_beam) [V].

**Dwarf Fortress** — [wiki,
Cave-in](https://dwarffortresswiki.org/index.php/DF2014:Cave-in) [S]; [Tarn
Adams on connected components, Stack Overflow
blog](https://stackoverflow.blog/2021/12/31/700000-lines-of-code-20-years-and-one-developer-how-dwarf-fortress-is-built/)
[V].

**Poly Bridge / Besiege** — [Poly Bridge physics
discussion](https://steamcommunity.com/app/367450/discussions/0/483366528923135119/)
[U]; [StressTypeVisualization
mod](https://github.com/56kyle/StressTypeVisualization) [V].

**The Powder Toy** — [element
notes](https://steamcommunity.com/sharedfiles/filedetails/?id=3280367565)
[U]; [solids and maximum pressure
issue](https://github.com/The-Powder-Toy/The-Powder-Toy/issues/868) [S].

**Independent analysis** — [Zarkonnen, "Structural Integrity in Block
Games"](https://zarkonnen.com/structural_integrity_in_block_games) [V];
[fragment size distributions in brittle
fragmentation](https://arxiv.org/pdf/1106.1506) [V — physics, power-law
fragment sizes, which is the shape `rigid::fracture`'s ladder already
approximates].

**Not what it looks like, recorded so it is not mis-cited later:** Voxagon's
["Cracking destruction"](https://blog.voxagon.se/2014/05/13/cracking-destruction.html)
is about **Smash Hit**, not Teardown. It is still relevant — "objects always
break where they get hit" as an explicit simplification, and a
connectivity check by shape graph after carving — but it predates Teardown
and says nothing about voxels.
