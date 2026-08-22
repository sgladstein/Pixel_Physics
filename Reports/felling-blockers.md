# Felling a tree: what has to be true first

**Status:** scope only, nothing built. Written because cutting trees down
was designed, costed, and then cut from the tree-interaction pass by the
owner — and the costing turned up three things that would have been
rediscovered expensively. A rejected direction keeps its knowledge.

**Audience:** whoever picks felling up. Read `CLAUDE.md` first; this
assumes it.

The short version: **the blocker is not the chop verb, and it is not the
rigid-body code. It is the organism support model, which answers a
different question from the one felling needs to ask.** One change
unblocks most of it, and two of the remaining items are redesigns wearing
the costume of constants.

---

## 1. The support model is the blocker

`structural::organism_is_supported` (`structural.rs:620`) is what decides
whether a plant cell stays up. It takes the cell under test and walks
outward through same-organism `Plant` tissue looking for ground, bounded
by `max_unsupported_span` (`wood.ron`: 8).

Three properties matter, and they compound:

**It asks about a cell, not about a tree.** The search starts at the
checked cell and must reach ground within 8 hops *of that cell*. A crown
cell forty hops up a trunk is unsupported by construction, no matter how
sound the trunk is. This is `CLAUDE.md`'s "which object does this rule
evaluate?" — the rule reads a cell where felling needs it to read an
organism.

**The anchor is `MaterialKind::Solid` only** (`structural.rs:621-623`),
and soil is a `Powder` (`soil.ron:8`). So soil does not anchor anything. A
tree is anchored only where a root has physically reached the stone
layer — and every cell more than `max_span` hops from such a root reads as
unsupported regardless.

**The failure is per-cell and ungated.** `organism_structural_tick`
(`structural.rs:526`) calls `break_free` on one cell, then reschedules its
four same-organism neighbours, each of which fails the same test. Unlike
the rock path it has no `within_disturbance` gate, so nothing bounds how
far the cascade travels.

The measured consequence is on record at `plant.rs:2324`: one structural
check scheduled mid-crown took a stand from 20,213 living cells to 772 —
the check destroyed ~96% of the world's living tissue, and that is the
only reason growth, germination, abscission and the shake all deliberately
schedule none.

### The fix, which three docs already name

Anchor on the organism's **real root set** and ask the question once per
organism instead of once per cell:

- `OrganismState::cells` (`organism.rs:867`) is a complete, maintained
  cell list — the risk of keeping it honest was taken and verified.
- `CellType::RootTip` is enumerable from it via `organism::cell_type`.
- `organism::reachable_from_anchors` (`organism.rs:1440`) already takes an
  anchor set, is **8-connected**, and its own doc names this branch as its
  intended first caller. It is not wired up here.

Cost is better, not worse. Today's per-cell BFS is ~145 visited positions
per checked cell, and a cascade checks thousands. One whole-tree walk is
~2,000 map iterations and ~16,000 neighbour probes — the same magnitude
`player::shake` already sanctions for `SHAKE_CELLS = 3000`, against a CA
sweep that does 163,840 cells *per frame*.

Two things ride along and are part of the fix rather than scope creep.
"Touches `Solid`" has to become "reaches a root", or a tree in soil never
anchors. And per `CLAUDE.md`, changing what the number *means* means
re-deriving `wood.ron`'s span of 8 and `rootwood.ron`'s 12 — by a **seed
sweep gating an order statistic**, because every structural test in the
tree is hand-placed geometry at the default seed and is therefore blind by
construction.

### The 4-vs-8 bug is real but is not the one

`organism_is_supported` walks `NEIGHBOURS_4` while `Grow` places children
at eight. The identical bug in `reachable_from_anchors` was measured at **4
of 30 cells reachable from the base of an intact tree** before it was
fixed (`organism.rs:1460`). Fixing it here alone changes an already-`false`
answer into a differently-`false` one; anchoring on roots subsumes it,
since the primitive that does the anchoring is already 8-connected.

### Six schedulers are live amputation triggers until this lands

`world.rs:2072` (brush paint/erase), `explosion.rs:389`, `fire.rs:517`,
and `rigid.rs`'s `promote` / `shatter_to_rubble` / `settle` all fan out on
`Solid | Plant` with no `organism_id` exclusion. Note that
`structural.rs`'s burning-tree test uses hand-painted `organism_id == 0`
wood, so it has never exercised the organism branch at all — a superseded
test in the sense `CLAUDE.md` warns about.

---

## 2. Toppling: one good surprise, two redesigns

**The good surprise: there is no material barrier to flying wood.**
`structural::is_body_material` (`structural.rs:1527`) is `Solid | Plant`,
not `Solid` — so **hand-painted wood already promotes to `ChunkBody`s
today**, and `an_overloaded_wood_beam_comes_down_as_deadwood` accepts
either a body or debris precisely because the load model started lifting
the beam out of the grid. `promote`, `settle`, `advance` and the renderer
all handle wood already. The barrier is entirely `organism_id != 0`
routing to a branch that has no fracture path, plus `BodyCell` having
nowhere to carry an organism id.

**Redesign 1: `ChunkBody` cannot topple.** Rotation is quarter-turn snaps
about the body origin, and `spin` accrues *from speed*
(`SPIN_PER_SPEED = 0.012`). A just-cut trunk has no speed, so it
accumulates no spin and falls flat — the exact "they just fall directly
perfectly flat down" complaint the rotation was added to answer. It also
snaps 90° at once, gated on the rotated shape fitting, so a 30-cell trunk
needs 30 cells of side clearance before it may turn at all. Angular motion
about a *base hinge*, arriving over a second or two, is not expressible
here. This looks like a constant and is not.

**Redesign 2: `fracture`'s size ladder is wrong for a trunk.** `wood`
takes the default `fragment_rungs: 5`, so at `size_bias 0` targets are
uniform over {2,4,8,16,32}. A 2,000-cell tree would come apart into
roughly a hundred bodies of ≤32 cells — right for granite, wrong for a
tree, and `MAX_BODY_CELLS = 400` caps a single body below tree size
anyway. Felling wants the severed region promoted as *one* piece, subject
to a cap that bounds work rather than gating whether it happens. That is a
new decision inside `fracture_with_impulse`, not a `.ron` number.

**What a severed tree does today**, for the record: `break_free` converts
one cell to `deadwood`, which is a `Powder`. So it dissolves into a column
of falling sawdust, one cell per reactive tick — `design-philosophy.md`
§0a's "uniform dissolve into powder" failure mode, verbatim. Nothing
anywhere measures a single tree's severance; there is no felling scene.

---

## 3. Ordering

0. **Build the instrument first.** There is no felling scene and no
   severance measurement. A `filmstrip` scene that cuts a trunk and prints
   *both* a standing-biomass census and a `chunk_bodies` count — because a
   coherent-looking collapse with a body count of zero has fooled this
   project once already, and a failure count is not a damage count.
1. **Re-anchor the support search on the root set.** The single change
   that unblocks the most, and a fix rather than a redesign: the
   primitive, the data and the intent all already exist.
2. **Gate the six dangerous schedulers**, or step 1 is unverifiable.
3. **Give the organism branch a severed *region*** instead of throwing it
   away one cell at a time.
4. **Carry organism identity through promotion** — `BodyCell` needs it, or
   the fall re-triggers step 1's landmine from inside itself.
5. **The verb, last.** A chop slots into `paint_stroke`'s existing
   rock-vs-plant match as a third arm, discriminating on `CellType` (a
   trunk chops, foliage shakes) or accumulating damage across bites on one
   target. `cut_resistance` would go beside `Material::climbable` and
   `fall_drag`, which are player-read properties by convention.

Wired to steps 1–4 in their current state, a verb would be a button that
deletes trees. `design-philosophy.md` §0a is why it is last and why it
cannot be first.

---

## 4. One correction to make regardless

`organism.rs`'s `CellType::RootTip` doc claims `structural.rs`'s organism
branch "anchors its reachability search specifically on `RootTip` cells".
**It does not** — it anchors on any `Solid` neighbour, starting from the
cell under test. The doc describes the intended design; the code
implements something else, and this is the first thing anyone
investigating will read.
