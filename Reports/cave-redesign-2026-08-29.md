# Caves: what is wrong with them, and what replaces the generator

2026-08-29. Lane E of the worldgen revamp program. The brief, from the owner,
verbatim: **"Caves also need to be fully redone as they are crap."** A
redesign, not a retune — so this document is a diagnosis and a proposed
generator, not a parameter table.

No `src/` behaviour is changed here. Two read-only instruments were repaired
(`examples/viewshot.rs`, `examples/cave_probe.rs`); both repairs are described
in §3 because in each case the broken ruler is itself part of the finding.

---

## The one-paragraph answer

**A cave here is a noise texture thresholded inside a box, so its shape
vocabulary is the texture's shape vocabulary and nothing else.** Every
corridor is a straight Voronoi boundary segment, every junction is a
three-way Voronoi vertex, the lattice is the *same* 8.2 x 3.9 cells at every
size, and the one thing in the picture that is a room was stamped on top as
an ellipse because the texture has no rooms in it. Six of the owner's eight
recorded cave verdicts are that one fact seen from six angles.

**The replacement has no field in it.** A cave becomes **rooms joined by
passages**: rooms grown by letting a roof collapse until the rock above it is
strong enough to hold, so their shape is a consequence rather than a formula;
passages found by shortest path through a cost field the codebase already has
(the strata), so they follow the bedding and the joints and they arrive
somewhere. The Worley web is deleted — the owner withdrew his earlier liking
for it by name (§2.2).

---

## 1. Look first

One full-size world (`terraced`, seed 1, 8192x2560), photographed from inside
the cave with the gnome standing in it for scale, and the same frame again
with every open cell painted magenta by the `F11` reveal so the cave's own
shape is visible through the rock.

Three review cards on board `caves` carry these frames —
`20260829T170504038Z-cd73e9`, `…170543996Z-3d6418`, `…170546697Z-715c0f` —
and the reading below is what they show:

* **The cave is a drawn Voronoi diagram.** Straight-line corridors meeting
  three at a time at ~120°, uniform width, over the whole envelope. Not
  "noisy" — *straight*. This is what *"too much voroni patterns"* is
  describing, literally rather than figuratively.
* **The one room is a circle** with a hard rim that cuts across the strata,
  the joint web and everything else in the frame without acknowledging any of
  it. *"It looks like a perfect oval, not natural."*
* **The room's interior is uniform black.** No wall shading, no depth, no
  falloff. *"the shape and the flatness."*
* **Two formations cross the entire room at one cell wide.** Not tapered,
  not attached to anything the eye can see: hairlines. *"they are all 1 pixle
  thick."*
* **The floor is a dead-straight ruled line** of gravel, and the crystal is a
  white smear with a soft glow round it rather than an object with facets.
* **The web ends abruptly** at the top and right of frame — that is the
  envelope's edge fade, i.e. the box.

---

## 2. The diagnosis: each verdict, and the mechanism under it

| Owner's words | Mechanism |
|---|---|
| *"too much voroni patterns"*; *"The honey comb… shouldnt be everywhere"* | The cave **is** `noise::worley_f2_f1` under one constant `CAVE_THRESHOLD = 0.09` (`carve_cave_void`, `passes.rs:1775`). A Voronoi boundary web is straight segments and 3-valent vertices; that is the entire shape vocabulary available |
| *"It is also looks like a single room instead of a cave system"* | A stationary threshold has no hierarchy — every part of the field has the same passage-width distribution. The only room is `grow_monumental_chamber`, which grows **exactly one** ellipse per system |
| *"That overall cave shape here is bad… It looks like a perfect oval"* | …and that room is an ellipse test, `(dx/rh)² + (dy/rv)² > 1.0` (`passes.rs:2010`). A geode vug (25% of placements) is a bare ellipse with no web at all |
| *"you could go bigger or more even better longer or have chains of caves"* | Each system carves inside its own `CaveEnv` box and `keep_seed_component` deletes everything not attached to one seed component. **Two systems cannot join, by construction** |
| *"there should be variability between caves"*; *"Again heterogenity is best"* | `CaveEnv::cell() = CAVE_CELL * half_w / ROUND_3_HALF_W`, so `half_w` cancels and the lattice is **8.2 x 3.9 cells for every cave in every world**. The only free parameter is scale |
| *"I see very little practical differences between any of the images"* (a cave retune A/B) | The same fact arriving as a playtest report. Retuning the lattice can only zoom it |
| *"the stalagmite and stalactites… are all 1 pixle thick"* | §3.2 — a formation is a column scan, not an object, and its taper is anchored to a row that is often in a different cavity |
| *"Nothing here look like a crystal"* | Crystal's four palette tones are all in the top fifth of luma and are assigned **per cell at random**, so there is no dark inside the object and no contiguous facet (`cave-beauty-review-2026-08.md`). A silhouette needs coherent shading; this codebase shades per cell nearly everywhere |
| *"It doesn't look like I could even enter it"* | Two separate things, and both are true. §3.3 (the passages are the gnome's own height) and §3.4 (**there is no cave entrance in this game at all**) |

### 2.1 Which of these are one cause?

**Six are one cause.** Rows 1–6 above are all *"the cave is a stationary
thresholded field inside a box"*, seen from different sides — the texture, the
absence of rooms, the stamped room that compensates for it, the box that
forbids chains, the constant lattice that forbids variety, and the retune that
therefore did nothing. Fixing the field fixes all six or none of them; that is
why five rounds of tuning have moved nothing the owner could see.

**Two are separate and must be fixed separately.** The formations (row 7) are
their own mechanism and would still be hairlines under a perfect network. The
crystal (row 8) is not a worldgen problem at all — it is per-cell random
shading applied to an object with a silhouette, and it has three symptoms
already recorded under different names.

**And one is prior to all of them** (row 9's second half): a cave with no
mouth is not a place, it is a buried cavity. §3.4.

### 2.2 The verdict that arrived while this was being written

The three cards in §1 were posted mid-session and two were answered inside ten
minutes. **They settle the design, so they are quoted in full and everything
after this section is written to them.**

On the room (`20260829T170543996Z-3d6418`), verbatim:

> *"1) This is way too small. There should be a system of caves that you can
> walk through and explore. Not just one small room like this. **This should
> be considered a small room and there should be rooms 3-7x bigger and
> multiple of them chained together so you can walk directly from one to the
> other.** 2) **The whole shape and generation of the cave shold be rebuilt
> from the ground up.** The caves are still all slightly modified circles or
> ovals, not realistic at all. **The voroni worly patter around the cave
> should be removed. I know I said that I liked it before but no.**"*

On the formations (`20260829T170546697Z-715c0f`), verbatim:

> *"Yes they should be wider and they usually are. not sure why they are one
> pixel here. **This problem has been solved. That said this is not at all the
> main issue.**"*

Four things change because of this, and three of them reverse what an earlier
draft of this document proposed:

1. **The Worley web is deleted, not demoted.** The earlier draft kept the
   honeycomb as background scenery on the strength of *"The honey comb is
   interesting background sometimes"*. That verdict is now withdrawn by name.
   There is no cave texture; there are rooms and there are passages.
2. **Rooms are the primary object, not a swelling of a passage.** *"rooms 3-7x
   bigger and multiple of them chained together"* makes the room the thing the
   generator places and the passage the thing that joins two of them.
3. **The size bar moves by most of an order of magnitude.** §8.
4. **Speleothems are a residual bug, not a pillar of the redesign.** *"not at
   all the main issue."* §7 stays because the hairline is a live defect with a
   named cause, but it moves to the back of the staging.

The third card (`20260829T170504038Z-cd73e9`, whether to keep the honeycomb as
background) is still open and is **superseded** by the answer above; it needs
no reply.

---

## 3. What I measured today

`cave_probe`, 16 seeds x 5 presets, 8192x2560, at `9265d0a`. The full logs are
reproducible with `cargo run --release --example cave_probe`.

### 3.1 The envelope is the silhouette, in every preset

| preset | systems/world | worlds with none | max span across | max span down | largest **connected** walkable | walkable regions (med / p90 / max) |
|---|---|---|---|---|---|---|
| arid | 1.5 | 9 of 16 | 1542 | 620 | 43% | 1 / 38 / 95 |
| canyon | 1.4 | 8 of 16 | 1543 | 619 | 36% | 3 / 35 / 94 |
| rolling | 1.6 | 8 of 16 | 1528 | 619 | 38% | 3 / 32 / 88 |
| terraced | 1.3 | 9 of 16 | 1544 | 619 | 38% | 3 / 44 / 86 |
| wetland | 1.4 | 9 of 16 | 1544 | 619 | 37% | 2 / 35 / 93 |

Five presets, five maxima agreeing to within 16 cells in 1,544 across and to
**one cell** in 620 down. Nothing in a preset or a region touches caves.

Two numbers there are worth stating in the owner's terms rather than the
instrument's:

* **Half of all worlds have no cave in them.** 8–9 of 16 seeds per preset.
* **A cave is typically one walkable place plus a lot of unreachable
  decoration.** The median system's largest connected walkable region is
  **36–43%** of its own void, and at p90 a system shatters into **32–44
  separate walkable pockets** with no way between them. The web is not a
  network the player can use; it is a texture that happens to be air.

The void is **0.37–0.41% of the deep massif**. Cross that with Lane A's
finding that `vaults` writes 43,208 cells and moves **0.000%** of the
player's surface view, and the standing picture is: this pass writes about
110,000 cells per world, none of which the player can see and half of whose
volume they cannot walk to.

### 3.2 The formations, and a ruler that was measuring the intention

The pass prints `vaults detail: … base-width med 19 range 12-31`. That number
is drawn from `VaultReport::formation_widths`, and `widths.push(half_l +
half_r + 1)` executes **before** the cone is rasterised (`passes.rs`, the A3
block). Everything after it can decline to draw:

```rust
let anchor = |from: i32, step: i32| -> Option<i32> {
    (0..=CONE_ANCHOR_SEARCH).map(|d| from + d * step).find(|&y| !solid(dx + o, y))
};
if lt > 0 { if let Some(y0) = anchor(t, 1) { … } }   // None => no cone at all
```

`CONE_ANCHOR_SEARCH` is **3**. A trunk is drawn from `t`, the top of the
column's whole maximal void run — and in this generator a run chains a narrow
honeycomb crack straight into a room, so `t` is frequently tens of rows above
the room's ceiling, in a crack one cell wide. Every offset column of the cone
then looks for its own ceiling within 3 rows of `t`, finds solid rock, and
draws nothing. The trunk survives at one cell wide for its entire drawn
length.

**That last paragraph is a reading of the code and a render that is consistent
with it, not an experiment.** The render shows two full-height hairlines
hanging into a room whose ceiling is far below the top of their columns, which
is what the mechanism predicts, and the drawn-versus-intended gap is
arithmetic. It has not been confirmed by widening `CONE_ANCHOR_SEARCH` and
watching the hairline count fall, and it should be before the fix is
scoped — that is the one-command positive control.

That is the *exact* failure the block's own comment says it fixed: *"`widths`
recorded the drawn base width, so `vaults detail` reported a median of 5 for
formations that were one pixel wide on screen — a ruler measuring the
intention instead of the artifact."* The `is_bottom` and 22%-flare gates were
removed; the census was not moved.

`cave_probe` reads formations back off the world, so it is honest — but its
`base width` is the **widest single row**, and that cannot tell a cone from a
wire with a lump on the end. Both score 12. So this session added **mean
width** (body cells / height) to the probe, which is the quantity the eye
actually integrates. Measured, 12 seeds each:

| preset | base width (widest row) | **mean width over the whole length** | hairlines (mean under 2 cells) | **hairlines over 3 gnome-heights tall** |
|---|---|---|---|---|
| terraced | med 12, p90 20, max 29 | med **6.5**, p90 10.5, max 15.4 | **25%** (53 of 214) | 4 of the 46 that tall (**9%**) |
| canyon | med 12, p90 19, max 31 | med **6.7**, p90 10.4, max 14.4 | **20%** (40 of 197) | 3 of 39 (**8%**) |
| rolling | med 11, p90 19, max 29 | med **6.6**, p90 10.4, max 16.0 | **25%** (55 of 218) | 4 of 48 (**8%**) |

**One in four or five formations in every world is a hairline**, and the pass
reports all of them at 11–31.

**A hypothesis I had and the measurement killed.** From the render I read the
hairlines as *the tall ones* — the trunks that cross a whole room — and put
that on a review card before checking. Split by height, the taller half of all
formations has a **larger** mean width than the shorter half in every preset
(7.7–7.8 against 3.9–5.6, in hundredths of a cell). The hairlines are not "the
tall ones". They are a **tail**: 8–9% of formations over three gnome-heights,
three or four per twelve-seed sweep — rare, and each one crosses an entire
frame, which is why the render reads the way it does and the median does not.
That count is the gate §7 proposes, and the card was corrected.

**One control and one caution on these numbers.** Run twice from the same
binary, the whole sweep is byte-identical, so same-build determinism holds as
`PLAN.md` requires. Run either side of a **rebuild that changed only an
example's `println!`s**, the same 12 seeds gave 20 systems against 18 and 224
formations against 214 — the release profile is `codegen-units = 1` with LTO,
so adding code anywhere re-inlines the lib and moves float rounding in
worldgen. Legitimate (the requirement is same-build) but worth knowing before
anyone A/Bs a cave change across a recompile. And **four seeds is not a
sweep**: at `seeds=4` the same instrument reads 44% hairlines and 60% of the
tall ones, against 25% and 9% at twelve.

### 3.3 The passages are exactly the gnome's height

`PLAYER_WIDTH x PLAYER_HEIGHT` is **7 x 14**. The median open column across
all five presets is **14–16 cells**. So the typical passage is the player's
own height with nothing to spare, and the walkable-region numbers in §3.1 are
what that produces: a network whose corridors are, statistically, exactly at
the threshold of admitting the character.

This is not a tuning miss. A stationary threshold produces one width
distribution everywhere; there is no setting of it at which the passages are
generous and the rock is not swiss cheese. It is C3 with a number on it.

### 3.4 There is no cave entrance in this game

`vaults` places a system between `plan.surface_y + vault_min_depth` (200) and
`bedrock_top_y - 16`, and `cave_system` **asserts** that every cell of the
system and its 2-cell rind is solid stone. A cave is therefore sealed by
definition — the seal is not incidental, it is the pass's postcondition.

`docgrep "cave mouth"` returns exactly one hit in the whole documentation
corpus, in `render.rs`'s dark-ramp comment. Nothing generates an entrance.
`examples/viewshot.rs` has to **mine a three-wide shaft from the surface**
before it can photograph a cave, and that shaft is in the harness, not the
game.

So: *"it looks like it comes from nowhere and goes nowhere"* is not a figure
of speech about the shape. Nothing about a cave is reachable, findable or
visible without the player first digging 200+ rows of rock on a guess. That is
the single largest reason six rounds of cave work have produced no playtest
reaction: **the owner's playtests cannot contain a cave.**

### 3.5 Two instruments repaired, because each was part of the finding

* **`viewshot vault=1` could not find a cave at the shipped world size.** It
  searched for deep air below `world_h / 2`, which was right when the world
  was 2048x640 and is 1,280 rows down in an 8192x2560 world — below every
  cave. It printed `NO VAULT in this world -- try another seed` on a seed
  whose own pass counter printed `systems 1` in the same run. It now searches
  from each column's own `surface + vault_min_depth`, which is the band the
  pass places into and which excludes brow and overhang air by construction.
  **Every "photograph a cave" instruction in this repo has been running
  against this since the world grew.**
* **`viewshot gnome=1` stood the gnome on the skyline**, hundreds of rows
  above a vault shot. It now stands him on the nearest cave floor that can
  actually hold him — the standing position nearest the view centre where a
  whole 7x14 box is open with footing under it — and prints `NO STANDING
  ROOM` when no such position exists in frame. That search is also a stated
  answer to *"it doesn't look like I could even enter it"*.
* **`cave_probe` gained mean width**, the tall-and-thin count, and the
  height split (§3.2).

---

## 4. The replacement: rooms made by collapse, joined by conduits

The lead is `worldgen-prior-art-and-dead-ends-2026-08-29.md` §3.4 — Paris et
al., *Synthesizing Geologically Coherent Cave Networks* (CGF 2021), with
released source; the same algorithm exists independently in the karst
hydrology literature (KarstNSim, pyKasso, anisotropic fast marching).
`karst` returns **zero hits across all 595 dead-end entries**: nothing here
has been tried and rejected.

**Two sentences.** *A passage is the path water took from where it went in to
where it came out, and the rock decides how expensive each step is.* And, the
half the literature does not hand you and the owner's verdict demands: *a room
is not drawn at all — it is a roof that fell in, and it stops growing when it
reaches rock strong enough to hold its own span.*

### 4.1 What a cave is made of, after the verdict

**Two objects, and no field.**

| | what it is | how it is made | rule |
|---|---|---|---|
| **Room** | a space you stand in and look around | a collapse dome, grown until the rock above it holds | ≥ 6 gnomes tall; the big ones 3–7x the one photographed in §1 |
| **Conduit** | the passage between two rooms | a skeleton with a radius profile | must admit the gnome **along its whole length**, so you can *"walk directly from one to the other"* |

Nothing else is written. `carve_cave_void`, `CaveEnv`, `CAVE_CELL`,
`CAVE_SQUASH`, `CAVE_THRESHOLD`, the edge fade, `keep_seed_component` and
`grow_monumental_chamber` all go. That is most of `passes.rs:1746–2100`.

**The generator's shape is: place rooms, connect rooms.** That is the sentence
the current one cannot say, because it has no rooms — it has a texture, and
one ellipse apologising for it.

### 4.2 What the generator does

**Stage A — the karst cell and its rooms.** A coarse lattice (proposal: 1,024
columns; §6 says why that number) decides whether a cell holds a cave system
and, if so, seeds **3–6 room sites** in it from its own hash, spread by a
Poisson-disk-style rejection so they are neither gridded nor clumped, at
depths drawn per site. Placement is a hash, so it is locally determinable and
streams (§6).

Room *sites*, not rooms: a site is a seed point and a target volume. The shape
comes from Stage C.

**Stage B — the cost field, which already exists.** This is what makes the
cave geology rather than noise, and the inputs are already in the codebase:

* **Inception horizons.** `TerrainColumns::hardness_field()` gives per-band
  hardness in the same banded coordinate `strata_offset` uses — the coordinate
  the shade pass bands the rock with, the benches snap to and the lenses lie
  in. **A soft band is cheap to travel along and a hard band is dear to
  cross.** That one term gives long, near-horizontal, bedding-parallel
  galleries: the anatomy round 3 tried to get by shearing the Worley frame,
  arriving as a consequence instead of as a warp.
* **Joints.** A conjugate fracture set — two directions — drawn **per band of
  a coarse quantising lattice**, never varying smoothly. Dead-end #18 is
  explicit that a smoothly varying Worley pitch is structurally broken where
  the consumer is an identity test between neighbours, *and it names this
  fix.* Cheap travel along a joint gives the vertical shafts and the angular
  direction changes.
* **Vertical bias.** Cheap downward above the palaeo-water-table (vadose,
  gravity-driven), cheap horizontal below it (phreatic, pressure-driven). One
  term, and it is what makes a cave read as having had a history.

The result is an anisotropic cost `C(cell, direction)`, evaluated lazily.

**Stage C — the room, which is a collapse and not a shape.** This is the part
that answers *"still all slightly modified circles or ovals"*, and the answer
is that **the room must not be drawn by a formula at all.**

1. Open a modest initial void at the site — a phreatic bubble, a few gnomes
   across. It may be an ellipse; nobody will ever see it.
2. **Let the roof fail, and let the rock decide when it stops.** Repeatedly:
   find the ceiling cells whose unsupported span exceeds what the *local band*
   can carry (soft band → short span, hard band → long span, straight off
   `hardness_field`), drop that slab, and put the rubble on the floor. Stop
   when every remaining ceiling cell is in rock competent enough to hold its
   span.
3. What that produces is a **breakdown dome**: a room whose ceiling is a
   bedding plane (it stopped when it reached a strong bed), whose walls step
   along joints, whose floor is a pile of exactly the rock that came out of
   the roof, and whose **size is a consequence of how weak the rock there
   was**. Two rooms differ because their rock differs — which is *"there
   should be variability between caves"* delivered structurally rather than by
   widening a draw.

   It is also the ethos's first law: the outcome is a distribution, not a
   binary. A room in strong rock stays small; in weak rock it runs away
   upward until it finds a bed that holds. Nothing in it is parametric, so
   nothing in it can read as a primitive.

4. **Volume balance.** Rock removed from the roof equals rubble on the floor.
   That is what makes a room read as *explained* rather than as excavated, and
   it is free: the breakdown loop already knows both numbers.
5. **Pillars are a by-product and are wanted.** Where a column of rock happens
   to survive the collapse — because it sits under a strong band, or between
   two failures — leave it standing. §6 notes that a room of the requested
   size *needs* pillars anyway; this is where they come from, and they are
   also the eye's scale ruler and beauty criterion 3's "necks".

**Stage D — the conduits that chain the rooms.** Dijkstra (or A\*) between
room pairs on a coarse lattice graph — every 4th cell is ample — under Stage
B's cost. Ties broken **explicitly by index**, or two equal-cost paths are a
build-to-build coin flip and same-build determinism is gone.

* Connect the rooms as a **spanning tree plus one or two extra edges**, so the
  system is connected (you can reach every room) and has a loop or two (real
  karst has both branchwork and maze).
* The conduit's radius is at least the walkable bar **along its whole
  length** — this is the literal content of *"chained together so you can walk
  directly from one to the other"* — and grows where tributaries join, so the
  cave gets bigger as you go and progress is legible.
* Cross-section is **not a circle**. The signature cave section is a
  **keyhole**: a wide phreatic tube with a narrow vadose canyon incised down
  from its floor, cut when the water table fell. Two numbers, and the shape is
  compression-and-release in a single section — beauty criterion 3 from the
  geometry rather than from a stamped room.

**Stage E — the mouth.** The system's shallowest room, or the conduit leaving
it, **daylights**: it reaches a valley wall, a cliff face or a brow and opens.
A doline at the far end marks the inlet on the skyline. Both are surface
features, which is the only thing on this list the owner sees without a debug
harness — see §3.4 for why that matters more than anything else here.

**What is deleted.** No Worley field, no threshold, no envelope box, no edge
fade, no seed-component keep, no `grow_monumental_chamber`. If any part of the
new generator finds itself thresholding a noise field to decide where rock is
absent, it has reproduced the thing being removed.

### 4.3 What it needs from the rest of worldgen

* `TerrainColumns::hardness_field()` and `strata_offset` — **already exist**,
  already shared by four consumers, and are the whole cost field. Nothing new.
* A **coarse karst lattice** (§6) deciding which cells hold a system, where
  its rooms sit and where a chain crosses into the next cell. New, small, one
  hash per cell.
* A **palaeo-water-table** per karst cell — one number, and today's
  `water_line` draw is already most of it.
* **A per-band roof-span capacity**, which is `hardness_field` read with a
  different question: *how far can this bed span unsupported?* It is the only
  input the collapse loop needs beyond geometry, and it is what makes two
  rooms differ.
* **`Ctx` must carry the cave as an object** — rooms as cell sets or
  bounding shapes, conduits as polylines with radii — so `pockets`,
  `springs`, `soil_moisture` and `life_scatter` can see it. This is Lane B's
  C1 (*no feature is an object*), scoped to one feature rather than solved in
  general, and it is what lets `pockets` stop eating caves (§5).

### 4.4 What it writes

Void, **breakdown blocks on the floor in the volume the roof lost**, gravel,
water below the line, formations, and — new — **a record of itself**:
`Vec<Room>` and `Vec<Conduit>` in `Ctx`, a room as its cell set or hull and a
conduit as a polyline of `(x, y, radius, section)`. That record is what
dead-end #14 demands ("anything a later system needs to know about a generated
void must be recorded at generation, never re-derived") and it costs a few
kilobytes per world.

---

## 5. The seal, and the rule any new pass must obey

Dead-end #28 is the trap most likely to recur here, and it has already shipped
twice in this generator: *"one grain of sand deleted a whole cave"*, and
`pockets` removes 100% of caves in `arid`. **`pockets` still suppresses
`vaults` by +8% to +59% on every preset that has caves** at the shipped world
size — reported to this lane rather than measured by it; Lane B's C6 table
reads +8%..+14% from its own 6-seed `pass_ablation` run. The two disagree on
range and agree on sign and mechanism. The defect is from the round-5 review
and is live.

The current code is on the right side of the rule — `erode_breaches` retracts
the void from a breach rather than rejecting the system — and **any new pass
must be written the same way**: reject a *breach*, never a system. Under the
new design the retraction is better still, because both objects know their own
centre: a breach shrinks a conduit's local **radius**, or pulls a room's wall
in a few cells, rather than deleting anything. The passage narrows past a sand
lens instead of being holed by it, which is also what a real cave does.

**And a room the size §8 asks for makes the wholesale form of this rule
actively dangerous.** A 1,000-cell-wide room's rind is tens of thousands of
cells; the probability that *none* of them is a stray grain is near zero. A
generator that rejects on one breach would place no large room in any world,
and would do it silently.

But the *right* fix is upstream and it is C6's cheap one. `pockets` is pass 6
and `vaults` is pass 14, so lenses are written first and the cave has to
retreat from them. With the cave recorded as an object (§4.3), `pockets` can
simply decline to write into a claimed room or conduit, and the interference
goes to zero rather than being eroded around. **Pass order plus a shared
representation, not a bigger rind.**

---

## 6. What it costs

**Reach.** `VAULTS_MARGIN = MAX_CAVE_HALF_W + VAULT_RIND = 802` today, so
generating one 64-column chunk requires **1,668 planned columns — 26x
amplification**, the worst declared margin in the pipeline. The new design
asks for much bigger caves, and the naive version of it would be far worse.
It does not have to be:

> A system belongs to a **karst cell** on a coarse lattice (proposal: 1,024
> columns). All of its rooms sit inside its own cell. **Chains cross cells by
> agreeing on the boundary**: a conduit leaving the east face does so at a
> point derived from the shared edge's hash, and the neighbouring cell's
> conduit enters at the same point from the same hash. Neither side ever
> looks at the other.

That bounds the margin at **one lattice cell plus the largest room's
half-width** — ~1,530 at the §8 bar, against today's 802. It is roughly double
and it is the price of the size the owner asked for; what it buys is that the
margin **stops growing with the cave**. Length is free after that: a chain of
eight karst cells is an 8,000-column cave still costing 1,530 columns of
reach, where today an 8,000-column cave would need a 4,000-column margin.
Making the lattice cell smaller trades reach against how many rooms a single
cell can hold, and that trade should be measured before the number is fixed.

**Structural support — and this is the constraint most likely to bite.** Stone
carries `max_unsupported_span: 16`, and `vaults` today drops a stone tooth
into any ceiling run over `MAX_CEILING_SPAN = 36` for exactly this reason. A
room 900–2,000 cells wide has a roof span two orders of magnitude past that.
**It will not stand unless the generator gives it support**, and `structural`
runs on generated terrain — this is not a rendering question, the roof will
actually come down.

Read the right way round, that is a gift rather than an obstacle: the support
a big room needs is **pillars**, and pillars are what a real cathedral room
has. §4.2 Stage C already produces them as collapse survivors; this makes them
mandatory rather than incidental. The instrument is `examples/support_census`
and `scripts/acceptance.sh`, and the check is CLAUDE.md's: put the fault back
— build the room without pillars and watch the roof fail — before believing a
green run.

**Frame cost.** Build-time only; nothing runs per frame. Today `vaults` is
**38–116 ms** of a ~5,700 ms world build. The conduit search is Dijkstra over
a coarse graph (every 4th cell), and the collapse loop is a bounded iteration
over one room's boundary. Against that, the design writes **more** void than
today (bigger rooms) and **less** than today per unit area (no web). It should
land in the same tens of milliseconds; that is a claim, not a measurement, and
`PASS_TIMING=1` is what settles it. The number to watch is the *world build*,
not the pass: an isolated harness overstates.

**Determinism.** Same-build determinism is required. Two hazards, both known:
the shortest-path tie order must be **broken explicitly by node index**, or
two equal-cost paths are a build-to-build coin flip; and no unstable sort may
decide anything in the frontier (CLAUDE.md's `sort_unstable` gotcha — tie
order is not a function of the comparator alone). The collapse loop must also
process failures in a fixed order, for the same reason.

**The render skip.** Nothing here keeps chunks awake; caves are static
geometry written at genesis. The one place to watch is daylight reaching an
entrance passage, which is the existing `sky_depth` ramp and already costs
what it costs.

---

## 7. Speleothems: an object with a profile, not a column scan

**Demoted, on the owner's instruction.** *"This problem has been solved. That
said this is not at all the main issue."* He is right on both halves: the
formation width work landed and three quarters of formations are now properly
tapered (§3.2), and the residual — one in four still coming out as a hairline
— is a bug with a named cause rather than a design gap. This section stays
because the cause **is** named and the fix is cheap; it goes last in §9.

The failure is Lane B's C2 in its cave costume, and it is structural: a
formation is not a thing, it is what a **1-D scan over the void array** wrote.
`for i in 0..floor.len()` walks columns; the trunk is a run down one column;
the "cone" is the same run repeated at offsets, each of which independently
looks for a ceiling within 3 rows and gives up if it does not find one. There
is no object, so there is nothing to give a profile to.

**The replacement.**

1. **A formation is an object**: an anchor cell, an axis, a length, a base
   radius, and a profile function `r(t)`, `t` from root to tip. Rasterise the
   object, clipped to void. No per-column anchor exists, so the
   `CONE_ANCHOR_SEARCH` failure cannot occur; a formation hanging into a room
   from a crack simply starts where the room's ceiling is, because that is
   where its anchor was placed.
2. **Three profiles, one parameter each.** A stalactite is **concave** — a
   carrot, `r(t) = R(1-t)^p` with `p ≈ 0.6`, plus a flared collar where it
   meets the rock. A stalagmite is **convex** — a candle, `r(t) = R√(1-t)`,
   blunt-tipped. A column has a **waist**. Linear interpolation between two
   radii is what gives a triangle, and a triangle is the "rectangle with one
   step" complaint one iteration later.
3. **Fewer and much thicker**, as asked twice. The base radius budget goes up
   and the count comes down; the *length* keeps its heavy tail so the
   soda-straw fringe survives (beauty criterion 1 is explicitly a heavy-tailed
   distribution, not a uniform size).
4. **A minimum aspect.** No formation may have a mean width below 2 cells
   over more than N rows — the tail measured in §3.2. Enforce it by
   **shortening**, never by declining to draw: a size cap must bound work,
   never gate whether something happens.
5. **Coherent shading.** The general rule from `cave-beauty-review-2026-08.md`
   §"Nothing here looks like a crystal": *a shape needs contiguous shading,
   and this codebase assigns per-cell random tone almost everywhere.* Correct
   for bulk rock, fatal for an object with a silhouette. A formation's tone
   must be a function of position **within the object** — a lit side, a shaded
   side, and horizontal growth banding for flowstone. Crystal additionally
   needs its darkest facet **below** the surrounding rock's luma, which no
   current crystal tone is.
6. **Drapery, not only sticks.** A curtain hanging along a sloping ceiling,
   and flowstone sheeting down a wall, are what read as *cave*; a third
   vertical stick does not.

**The gate.** `cave_probe`'s new `mean width` and `hairlines over 3
gnome-heights` are the numbers to hold, not `base width`. Widest-single-row
cannot distinguish a cone from a wire with a lump on the end, which is exactly
the pair the complaint separates.

**Not filed as a bug.** The hairline is a live defect with a reproduction and
a named cause, and `Reports/open-bugs-handoff.md` is where that belongs — but
it is the most contested file in the repo (118 landings) and three lanes of
this program are running concurrently, so this section is the record and
whoever coordinates should decide whether it also earns a letter. Grep the
register for `CONE_ANCHOR_SEARCH` before filing; nothing there today.

---

## 8. What makes a cave worth being in

### 8.1 The size bar, in gnomes

The owner is 7 cells wide and 14 tall, and *"3-7x bigger"* was said of the
room in card `…3d6418`, which is **145 cells across and 92 rows tall** — the
largest room a full-size world currently produces at that seed.

Read as linear extent, which is how a space you walk through is normally
compared:

| | today | proposed bar |
|---|---|---|
| a **small** room | 145 x 92 — *"this should be considered a small room"* | the same. It stays; it is the floor, not the target |
| a **big** room | does not exist | **435–1015 wide, 276–644 tall** — 3–7x the above. **20–46 gnomes tall** |
| rooms per system | 1 | **several, chained**, with walkable passage between |
| walkable passage | median open column 14–16 cells = **1.0 gnome** | ≥ 1.4 gnomes tall (20 rows), ≥ 1.3 wide, **along the whole conduit** |
| connectivity | largest walkable region **36–43%** of the void | ≥ 85% of the system in one walkable region — *"walk directly from one to the other"* |
| caves per world | 1.3–1.6, and **half of all worlds have none** | at least one reachable system per world |
| entrance | **none exists** | a mouth on the skyline, findable without digging |

**Two consequences worth stating before anyone builds this**, because they are
where a number this size stops being a number:

* **At 3x a room no longer fits on one screen.** The viewport is 512x320
  cells. A 435x276 room fills it; a 1015x644 room is **two screens wide and
  two deep**. That is the scale the owner is asking for, and it means a big
  room is something you traverse rather than something you see.
* **A roof that wide does not stand** without pillars (§6). The size bar and
  the pillar requirement are the same requirement.

The reading is linear rather than by area, and that is an interpretation of
*"3-7x bigger"* rather than a quotation. By area the bar would be 250–380
wide, which is still four to six times more room than exists today and still
past one screen in width — so the design does not fork on the ambiguity, only
the top of the range does.

### 8.2 The four things that are not shape

* **Light: one source, darkness preserved.** The frames in §1 are flat because
  they are uniformly black — full darkness kills depth exactly as full
  illumination does, and *"the light and the block, the shape and the
  flatness"* is one sentence for a reason. Daylight falling into the entrance
  passage, fading over `render.rs`'s existing 24-row ramp, gives the mouth its
  falloff for nothing. Deep in, the crystal accent is the only light, and it
  must be **rare** to read as treasure.
* **Water doubles everything.** The still pool already works and is the best
  thing in the current render. Put it at the bottom of the system where the
  passage continues under it, so the water is on the route rather than beside
  it.
* **It goes somewhere.** Chained rooms with a passage between them *are* the
  answer to *"comes from nowhere and goes nowhere"*; the terminal room is the
  payoff. **The vug already exists** — make it the reward at the end of a
  system rather than an independent sealed lottery ticket that 25% of
  placements are spent on.
* **Scale cues.** A room reads as huge only against something known. Pillars,
  breakdown blocks, formations at a range of sizes and a daylight shaft are
  all rulers the eye uses. A uniform black ellipse offers none, which is why
  the owner said twice that he could not judge cave scale.

---

## 9. Staging

Ordered by **what the owner would see**, not by the dependency graph — and
re-ordered after §2.2's verdict, which moved the formations to the back and
made the room the front.

| | ships | what the owner sees | why here |
|---|---|---|---|
| **S1** | **The mouth.** Keep today's cave; drive one passage from its highest room to the surface, daylighting on a slope, with a doline at the far end. | A cave you can **walk into**, with daylight falling down the entrance. The first cave that has ever appeared in a playtest. | Smallest change that moves caves from 0.000% of the player's surface view to a visible feature — and it builds and proves the skeleton-plus-radius primitive S2 needs |
| **S2** | **Rooms, made by collapse** (§4.2 Stage C), several per system, chained by walkable passages (Stage D), at §8's size. The Worley field, the envelope and `grow_monumental_chamber` are deleted in the same change. | *"a system of caves that you can walk through and explore"*. Rooms two screens across, with pillars, rubble on the floor that came out of the roof, and passages between them he can actually walk | The main event. Everything in §2.1's "six are one cause" resolves here or nowhere |
| **S3** | **Geology in the cost field** (Stage B): bedding-parallel galleries, joint-controlled shafts, per-band roof capacity so two rooms differ because their rock differs. | Caves that differ from each other, and passages that lie along the banding the rock is already drawn with | Separable from S2 — S2 can ship with a plain isotropic cost and still be a system. This is where *"there should be variability between caves"* is answered |
| **S4** | **The causal hookups.** `pockets`/`springs` reading the cave record; chains across karst cells; the vug as the reward at the end. | Springs that come out of a cave mouth; caves longer than a screen; something to find at the end | Needs the object record from S2 |
| **S5** | **Formations as objects with profiles** (§7), plus coherent shading. | Thick, tapered, fewer; no hairlines; a crystal that reads as a crystal | Explicitly *"not at all the main issue"*. It is a live defect with a known cause and a one-line gate, and it goes last |

**S1 first, even though S2 is the redesign.** Every cave verdict in the record
has been given on a debug render of a sealed cavity that the player cannot
reach, because §3.4 says no cave in this game has a way in. Until one is
enterable the owner is judging photographs of a place he cannot visit — which
is the deepest reason six rounds of cave work have produced no playtest
reaction, and it is one pass to fix.

---

## 10. What I could not establish

* **Whether the search and the collapse loop land inside the current build
  budget.** §6's cost is derived from node counts, not measured. It is S2's
  first check, and `PASS_TIMING=1` already prints the number.
* **Whether a room at §8's size can be made to stand**, and how many pillars
  it takes. This is the largest open risk in the design, it is measurable
  today with `support_census` against a hand-built room, and it should be
  measured **before** S2 is scoped rather than discovered inside it.
* **Whether "3-7x bigger" means linear or by area.** §8.1 takes it as linear
  and shows the design does not fork on the difference — only the top of the
  range does. Not worth a round trip to confirm; worth confirming with the
  first render that shows a big room.
* **Whether `pockets` suppression is +8%..+59% or +8%..+14%.** Two
  measurements at the shipped size disagree in range; both agree on the sign
  and on the mechanism. Not re-measured here because it does not change the
  design — the fix is pass order and a shared record either way.
* **Whether the crystal fix belongs to caves at all.** *"Nothing here look
  like a crystal"* is a per-cell-random-shading problem with three recorded
  symptoms elsewhere (`palette_family`'s dithering, flowstone blocks, this).
  §7 states it because a cave is where it is seen, but a shading lane may own
  it better.
* **Seed count.** §3.1 is 16 seeds x 5 presets; §3.2 is 12 seeds x 3 presets.
  Both are order statistics, per CLAUDE.md, but the formation figures should
  be re-run at 16+ if anything is gated on them.

---

*Freshness: written 2026-08-29 against `claude/worldgen-revamp-plan-dot67g` at
`9265d0a`, and revised the same day against the owner's verdicts on review
cards `20260829T170543996Z-3d6418` and `…170546697Z-715c0f` (§2.2). Every
figure is reproducible from `examples/cave_probe.rs` and
`examples/viewshot.rs` at this commit; the invocations are in each file's doc
comment.*
