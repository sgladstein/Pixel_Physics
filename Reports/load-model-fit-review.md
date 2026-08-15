# Does the load model carry the game? — a fit review

**Status:** review, not a plan. Written immediately after
`7e13e42 Fail rock on load, not reach: torque > capacity` landed, against
the owner's stated target: *"We want this engine to be able to mine caves,
explode walls, build houses/castles, while being physics/simulation based
with eventual procedural generation."*

**Scope.** Forward-looking architecture and game-feel only. Line-by-line
correctness is somebody else's pass and is deliberately not duplicated
here. Where this document names a defect it is because the defect changes
what the *game* can be, not because the code is untidy.

**What is measured and what is not.** Everything in §1's arithmetic and
every number in a fenced block below was read out of the running engine
through `filmstrip`'s `load=` / `loadmap=` probes, on the commit as
shipped. Everything about M10 streaming is reasoning from the code, since
there is no streaming world to measure. Both are labelled at the point of
use. The four probe runs are reproducible as:

```
cargo run --release --example filmstrip -- scene=undercut start=1 every=1 count=1 load=95,150 loadmap=1
cargo run --release --example filmstrip -- scene=capped   start=1 every=120 count=6 load=256,200 load=200,100 loadmap=1
cargo run --release --example filmstrip -- scene=terrain  start=2 every=1 count=1 loadmap=1
cargo run --release --example filmstrip -- scene=ligament start=1 every=1 count=1 load=105,152 loadmap=1
```

---

## 0. The verdict in one paragraph

The criterion is right and the arithmetic is right. `torque > capacity` is
the correct shape for this game and the measured numbers confirm the
formula does what its doc says. **But the model is currently switched off
for most of what the owner wants to build with it**, by three separate
"cannot fail" escape hatches that are not part of the design: a cell whose
`aux` is 0 is exempt, a cell with powder beneath it is exempt, and a
structure large enough to exhaust the per-frame budget is exempt. Every
one of them is *binary immunity* — which is exactly the failure mode the
four earlier support models were rejected for. The tension between "hold a
mountain" and "let a tower break" has not been re-opened; it has been
moved out of the failure criterion, where it was visible and argued about,
and into the guards around it, where it is invisible. That is the single
most important thing in this document.

---

## 1. Building: what a player can actually make

### 1a. The arithmetic, confirmed against the engine

`capacity = (span²/2) × section² × attachment × uncracked/4`, and the
support forest turns out to have a property the handoff did not anticipate
and which changes every number.

**Measured** (`scene=undercut`, an attached 12-deep shelf running from a
cliff at x=90 out to x=209, probed at its root):

```
load (95,150): mass 115 torque 6555 capacity 13824 stress 0.47
```

`mass 115` is exactly the count of cells in **row 150 alone** from x=95 to
x=209 — not the 690 cells of the six-deep slab above and below it. And
`torque 6555` is exactly `114·115/2`, the moment of that one row about
itself. `capacity 13824` is `128 × 3² × 12`.

The reason is the tie-break. In slab geometry a cell can reach an anchor
equally cheaply through its left neighbour or the one below it, and
`NEIGHBOURS_4` puts `(-1, 0)` first, so **horizontal wins every tie and a
slab decomposes into independent single-row chains**. Each row carries
only itself. Confirmed again on plain terrain, where the shipped right-hand
ledge (110 cells long, 6 deep) reads `mass 109 torque 5886 capacity 55296`
— one row, and `128 × 6² × 12` — and on `scene=capped`, whose 36-deep cap
reads `capacity 165888` = `128 × 36² × 1`.

That is not what `Reports/load-model-handoff.md` §2b calibrated against.
It says "root torque of a beam length `L`, depth `D` … is about `D·L²/2`",
i.e. the whole section carries the whole beam. What the engine computes is
`L²/2` per row against `128·D²` per row. **Depth therefore helps twice** —
once by dividing the load `D` ways and once by squaring the capacity — so
effective strength goes as `D³`, not `D²`, and every beam is `√D` longer
than the calibration intended.

### 1b. The table

A beam of depth `D` cantilevered from a wall fails at the first length `L`
(in cells, measured from the wall face) where `L(L−1)/2 > 128·D²·A`, with
`A = 12` if the beam cell is still attached and `1` if it is foreground.
Stone only; wood's base is 32 instead of 128, so every number below halves.

| Depth `D` | Foreground `L` fails at | Attached `L` fails at |
|---|---|---|
| 1  | **17**  | 56  |
| 2  | **33**  | 112 |
| 3  | **49**  | 167 |
| 4  | **65**  | 223 |
| 6  | **97**  | 334 |
| 8  | **129** | 444 |
| 12 | **193** | 666 |
| 16 | **257** | 888 |
| 40 (`MAX_SECTION` ceiling) | 641 | 2218 |

Foreground is exactly `16D + 1`; attached is `16√12·D ≈ 55.4D`. Two
consequences worth reading off directly:

- **A roof between two supports is two cantilevers**, so a foreground slab
  of depth `D` clears a room `32D` wide — 32 cells at one thick, 64 at two,
  96 at three. That is a generous house.
- **A column's capacity goes as its width squared, measured horizontally**
  (`section` reads perpendicular to the parent direction, and a column's
  parent is below it). So a one-wide column topples under an eccentric cap
  at a moment of 128, a three-wide one at 1152. Wide foundations holding
  eccentric loads is exactly the right affordance for castles, and it falls
  out of the model rather than being added.

**So the nominal build envelope is not the problem.** It is roomier than
the owner's complaint suggests, and if anything the `D³` scaling makes
thick construction *too* forgiving. There is a real argument for taking the
`√D` back by making a section's rows share their load — see §6.

### 1c. What actually happens today, which is not the table

Three exemptions currently keep large player-built structures out of the
model entirely.

**(i) `aux == 0` means "anchor", and everything a player paints starts at
`aux == 0`.** `Cell::new` zeroes `aux`; `load::evaluate_within` returns
`None` on `aux() == 0` and `support_parent` returns `None` on `own == 0`.
So a freshly painted cell is not "unjudged", it is *an anchor*, and so is
everything that can reach one.

Measured on `scene=capped` — a 60×192 foreground column with a 120×36 cap,
15,840 cells, the scene whose own comment says it is built "exactly as the
stone brush lays it down" — across 600 frames:

```
tile 0: frame   1, sites 10 ... worst stress: 0.00 at (200,107) -- mass 1 torque 0
tile 5: frame 601, sites 10 ... worst stress: 0.00 at (199,108) -- mass 2 torque 1
load (256,200): not evaluated -- stone, aux 0, attached false
load (200,100): not evaluated -- stone, aux 0, attached false
```

Two cells of 15,840 are ever evaluated, and the world is in that state at
frame 601 exactly as it was at frame 1. **Acceptance case 2 in the commit
message — "scene=capped thick column still stands, worst stress 0.00" — is
vacuous.** The column stands because nothing looked at it. This is
`CLAUDE.md`'s own "did it fire at all needs a counter" in a new place: the
counter was printed (`worst stress 0.00`) and read as a pass.

Part of this is the scene: `capped` is the only structural scene that does
not call `compute_world_distances`, and the brush would at least schedule a
check per painted cell. But the underlying convention is real, and it also
covers **landed debris** — `rigid::settle` writes `Cell::new(...)` with
`aux` at 0 *deliberately*, reasoning that a body must not re-break on
landing. Under the current convention that reasoning produces an anchor: a
chunk that falls and settles becomes a permanent anchor wherever it lands,
and anything built on it inherits that.

**(ii) A single grain of powder beneath a cell makes it an anchor.**
`is_anchor` / `is_resting_on_ground` accept any `Powder` below, and the
anchor sets `aux` to 0, which routes straight back into (i) — the cell is
not merely *supported*, it is *exempt from the overload test as well*. This
is one mechanism behind three separately-reported symptoms:

- the freed slab in `scene=ligament` "propped by rubble wedged in the
  notch" (recorded in the commit message);
- the 1-cell skin surviving on a collapsed shelf — the collapse's own
  rubble lands underneath what is left of it;
- a player being able to cantilever indefinitely by sprinkling sand under
  the far end.

The predicate's own doc argues carefully that powder is the one case the
relaxation cannot see, and that is right. What does not follow is that a
granular pile should confer *immunity to bending*. A pile carries
compression, not moment.

**(iii) The per-frame budget is exhausted proving a foreground structure is
standing up, so nothing else gets checked.** `is_supported` first tries the
cheap chain walk; for any cell in a `aux == 0` sea the chain dies
immediately (its parent is an "anchor" with no parent), so it falls through
to the bounded flood — which must walk the entire connected component
before it finds the floor. On `capped` that is ~15,000 cells against a
`MAX_LOAD_CELLS_PER_FRAME` of 12,000, so **one check consumes the whole
frame's budget**, every subsequent site that frame returns
`ChainVerdict::Deferred`, and `Deferred` reschedules only itself and never
its neighbours. That is why the site count sits at exactly 10 for 600
frames: the relaxation wavefront cannot advance, because the budget is gone
before anything moves.

It also means a *settled, standing, motionless* foreground structure costs
a 12,000-cell walk on a repeating schedule forever. `awake 0/40` — the CA
sweep is asleep, the dirty-rect skip is doing its job — and the structural
phase is still burning a frame's worth of work. `CLAUDE.md` is explicit
that a cost must be measured against the state the optimisation exists for;
this one is worst exactly there.

### 1d. What should change (§1)

Ordered by ratio of effect to cost.

1. **Stop treating `aux == 0` as proof of anchorage.** `is_anchor` already
   reads the world rather than the cache, and its doc says why. Apply the
   same discipline at the two early-outs: `evaluate_within` and
   `support_parent` should ask `is_anchor(world, x, y)`, not `aux() == 0`.
   Cost: two extra neighbour reads on a path that already does four. This
   is the fix that turns the model back on for built structures, and it
   will surface real failures that are currently hidden — expect fallout,
   and expect it to be the point.
2. **Give the brush the converged pass generation gets.** At the end of a
   stroke, run `compute_world_distances`' relaxation scoped to the stroke's
   bounding box plus a margin, seeded from the surrounding cells' existing
   values. Cost: one Dijkstra over the stroke area, once. It replaces a
   reactive count-up whose length is the structure's own height in 5-frame
   rounds — inferred, not measured: a 192-tall painted column needs ~192
   rounds at 5 frames each while saturating `MAX_SITES_PER_FRAME`, which is
   the shape of "I built a thing and it collapsed ten seconds later".
3. **Split "supported" from "exempt" for powder.** Resting on powder should
   terminate the support chain (so landed debris does not shatter — the bug
   the predicate was written for) *without* zeroing `aux` and without
   skipping the torque test. Concretely: powder support satisfies
   `is_supported`, and contributes a capacity no larger than a granular
   pile's, which for a bending moment is approximately none. Cost: one
   predicate split. Pays for the propped slab, the surviving skin, and the
   sand-cantilever exploit at once.
4. **Make the anchor answer survive across frames.** `AnchorMemo` is
   cleared every frame, so a static structure re-proves itself from scratch
   forever. Keying it to chunk-dirty generations instead would make a
   settled world free and stop the budget starvation in (iii). Cost: a
   generation counter per chunk, which `touched_chunks` is already most of.

---

## 2. Foreground ↔ background attachment

### 2a. What the owner is actually asking for, decomposed

"We should be able to attach foreground objects to background objects" is
three separable asks, and only one of them is missing.

1. **A load path across the interface.** *Already works.* `support_parent`
   does not read `attached` at all; a foreground beam butted against a
   cliff relaxes off the cliff's real distance and its chain reaches
   bedrock. `scene=snap` is exactly this and behaves.
2. **Capacity at the joint.** *Missing.* The joint cell is foreground, so
   it gets `A = 1` and fails at `16D` — identical to a beam keyed into a
   pile of loose bricks. Keying into the massif currently buys nothing.
3. **A verb, and legibility.** *Half-present and wrong-shaped.* The `B`
   background brush already grants full `attached` to anything painted,
   which is a creative-mode authoring tool ("paint terrain") sitting where a
   game mechanic ("anchor this beam") should be. It is unbounded: a player
   can paint an entire castle as background and it is 12× stronger
   everywhere, forever.

The naive chain rule fails for the reason stated in the brief, and worse:
attachment is not just a capacity multiplier, it is also what
`is_structurally_interesting` uses to skip evaluation. An attachment that
spreads does not merely make things strong, it makes them **invisible to
the model**, which is failure mode (i) again by another route.

### 2b. Proposal — it is a *capacity* question, answered from the parent

**Do not spread attachment. Read the parent's.**

```
attachment(c) = if c.attached()            { attached_span_bonus }   // 12
                else if parent(c).attached() { keyed_bonus }         // new, ~4
                else                       { 1 }
```

`support_parent(world, x, y)` is already called inside `capacity`, so this
is one extra read of a cell that has just been fetched. Its properties are
the ones the naive rule lacks:

- **It cannot chain.** The bonus is a function of the *edge* between a cell
  and its parent. A cell two steps out from the cliff has a foreground
  parent and gets `1`. Painting cell after cell out from a wall does not
  propagate anything, because nothing is propagated.
- **The "the floor is attached, so everything is" objection dissolves in
  the right direction.** A structure standing on attached ground gets the
  bonus on its bottom row only — which is its *foundation*, the cell that
  carries the overturning moment (§1b). A wide base keyed into bedrock
  being harder to topple than one standing on a wooden floor is the
  behaviour a castle game wants, and it is bounded to one cell deep.
- **It is stateless.** Nothing is stored, nothing goes stale, nothing has
  to be invalidated, and it inherits `support_parent`'s determinism.
- **It degrades correctly under damage.** `detach_around_crack` already
  strips attachment near a fissure, so working a crack at the joint removes
  the joint's bonus — the graded outcome, for free.

`keyed_bonus` belongs in `.ron` per `design-philosophy.md` §2a, and should
be visibly smaller than `attached_span_bonus`: being *bolted to* the massif
is not the same as *being* the massif. Starting at 4 (i.e. 2× the reach)
is a guess and should be tuned by eye, not by test.

### 2c. The verb, and where any new state lives

For the gameplay affordance — the explicit "key this into the rock" action
— **make the joint a material, not a flag.** A `mortar.ron`:

- costs no new bits anywhere (`Cell::flags` is 8/8, and `aux` is a `u16`
  distance whose `u16::MAX` is a live sentinel and whose whole meaning
  changes when `organism_id != 0` — bit-stealing from either is a trap this
  file's own comments warn about twice);
- is hot-reloadable data, so its strength is tunable without a rebuild;
- renders distinctly, so the player can *see* where their structure is
  keyed, which is half of what "satisfying" means here;
- is destructible on the same terms as everything else — it cracks, it
  loses attachment, it has a `breaks_into`. A keystone you can knock out is
  a mechanic; a flag you can't see isn't.

Combine the two: mortar carries a large `keyed_bonus`, applied by §2b's
parent rule, so a mortared joint is strong only for the cell that actually
touches the massif and only while the mortar is intact.

If a stated joint really cannot be a material, the fallback is a sparse
side table on `World` — the thing `load-model-handoff.md` §3 asked for and
this commit correctly refused. The refusal's reasoning was that a *derived*
quantity should not be cached, and that is right; an *authored* one cannot
go stale by definition, so the objection does not transfer. It would have
to be a `BTreeMap` or a sorted `Vec` (issue #7), and it would have to be
cleared on `World::set`. That is real bookkeeping for something a material
gives away free, hence the ordering.

**What must not happen:** granting attachment on placement (the `B` brush
generalised), because attachment is an evaluation skip as well as a
strength bonus, so an attached castle is an unjudged castle.

---

## 3. Mining and castles: has the tension moved or resolved?

**The part that was genuinely resolved stays resolved.** Geometry cannot
tell a mountain from a stacked wall; a stated bit can; and load answers the
separate question of *where* a structure gives way. Both halves are sound
and neither should be re-litigated.

**What the tension became.** It is now a single scalar,
`attached_span_bonus`, and the two regimes it has to straddle are measured:

| | measured stress |
|---|---|
| Shipped terrain, worst cell anywhere (`scene=terrain`) | **0.11** |
| Undercut shelf mid-collapse (`scene=undercut`) | 0.47–0.50 |
| Ligament neck (`scene=ligament`) | 9.94 |

Nine times headroom on terrain at today's scale, which is comfortable. The
number that eats that headroom is *span*, quadratically: at 6 cells deep an
attached feature is free to 334 cells and the shipped ledge is 110. A
streamed world with 500-cell overhangs and 3-cell cave roofs will sit on
the wrong side of that line, and the `D³` scaling of §1a means the answer
will keep being "make it thicker" rather than "tune the constant". That is
tolerable — it is a knob with a physical meaning, and PLAN.md already
flags a span-aware worldgen post-pass as unsolved and still needed.

**Where it will actually break next is not the constant.** It is the three
exemptions in §1c, because each of them is the *old* failure — binary
immunity — reintroduced as bookkeeping rather than as geometry:

| Exemption | Mining consequence | Building consequence |
|---|---|---|
| `aux == 0` ⇒ anchor | landed debris and settled bodies anchor whatever is above them | a painted castle is unjudged until relaxation reaches it, then judged all at once |
| powder ⇒ anchor | your own rubble stabilises the ceiling you were mining | sprinkle sand, cantilever forever |
| budget/cap exhaustion ⇒ "supported" | the biggest collapses are the least likely to resolve | the biggest builds are the least likely to be evaluated |

Note the direction of the third row: it is `CLAUDE.md`'s "a size cap must
bound work, never gate whether something happens", written for
`MAX_BODY_CELLS` and fixed there, recurring in `is_supported` and
`failing_along_support_chain`. The caps are individually well-argued —
"never conclude falling from an unfinished search" is the right paranoia —
but their *sum* is that scale buys immunity. The fix is not to make the
caps braver; it is to make the question cheap enough not to need them
(§1d.4).

---

## 4. Streaming (M10)

All of this section is read off the code; there is no streaming world to
measure against.

### 4a. What happens today if the world goes unbounded

Three things, and they compound:

1. **`compute_world_distances` returns immediately.** `world.bounds()` is
   `None`, and the function no-ops by design ("It no-ops on an unbounded
   world rather than pretending to handle that case here"). So no streamed
   cell ever gets a distance from the converged pass.
2. **Every streamed cell therefore reads `aux == 0`, which means anchor.**
   §1c(i) applies to the entire world at once. The failure mode is not
   `worldgen-design.md` §6b's predicted *global collapse* — it is global
   *inertia*. Safer, and much harder to notice.
3. **A non-resident chunk reads as `Cell::EMPTY`, not as bedrock.**
   `World::get` returns `OUT_OF_BOUNDS` (bedrock) only when `in_bounds`
   fails, and on an unbounded world `in_bounds` is always true, so a
   missing chunk is empty space. Two consequences: every cell at the
   loading frontier acquires an "empty" neighbour and so becomes
   *structurally interesting*, defeating the attached-bulk early-out along
   the whole frontier surface; and a support chain that would have run into
   unloaded terrain instead finds nothing there, so `is_supported` falls
   through to the flood, which terminates at the frontier without finding
   an anchor. Whether that reads as "falling" then depends entirely on
   whether the piece happens to be under `MAX_REGION_CELLS` and under
   budget — i.e. small pieces at the frontier fall, large ones do not.

### 4b. The costs that do not survive streaming

- **A support chain is `O(depth to bedrock)`.** On a world whose surface is
  a thousand cells above bedrock, one cold chain walk is a thousand reads,
  and the 12,000-cell frame budget buys about twelve of them. The cost of a
  single structural check scaling with the world's *depth* is precisely the
  thing that must not happen when the world stops having a fixed size.
- **`MAX_SUPPORT_WALK = 512` starts firing routinely** and resolves to
  "supported", so above 512 cells of depth the *unsupported* failure mode
  quietly stops working for anything with a long chain. (Genuinely detached
  pieces still work — their chains are short — so this is a slow leak, not
  a cliff.)
- **`MAX_SUBTREE_CELLS` did not bound what it says it bounds.** Its doc
  says "cells one subtree walk may visit"; the guard as committed was
  `stack.len() > MAX_SUBTREE_CELLS`, the DFS stack depth. Measured on
  `scene=ligament` at HEAD: `mass 2315` out of a walk capped at 1024. *Being
  fixed concurrently in the working tree* (counting walked cells, cap raised
  to 8192) — noted here only because it changes the streaming arithmetic:
  once the cap really bounds a walk, the per-walk and per-frame bounds
  finally mean different things, and it is the *per-frame* one that has to
  grow with the number of resident chunks rather than staying a single
  world-wide 12,000.
- **`Sx` is accumulated in absolute world `x`.** `i64` was chosen for
  exactly this reason and that was right, but it means a subtree summary
  cannot be stored per-chunk and reused, because the value is expressed in
  a frame that moves.

### 4c. What has to change

1. **Anchor distance becomes chunk-local and boundary-seeded**, which is
   what `worldgen-design.md` §6b already proposes ("a cheap BFS from
   bedrock, once per chunk, with anchor distance living on the coarse
   layer"). Each chunk stores a boundary vector of distances (2·(W+H)
   `u16`s — 512 bytes for a 64×64 chunk) that survives the chunk being
   unloaded; a chunk relaxes from its own boundary values and re-relaxes
   whenever a neighbour's boundary value *decreases*. That is standard
   blocked Dijkstra and it converges to the same field as the global pass,
   provided the tie-break stays `NEIGHBOURS_4` order.
2. **A non-resident chunk must read as "unknown", not as empty.** Empty is
   an active claim that there is no support there, and it is the wrong
   claim. The cheapest honest encoding is to treat a non-resident
   neighbour the way the caps already treat exhaustion: supported, and
   *not* structurally interesting. That keeps the frontier from lighting
   up and keeps frontier terrain from deciding it is falling. It also means
   a player mining at a seam gets no collapse until the next chunk resides
   — an acceptable trade, and one to state out loud rather than discover.
3. **Store the moment about a chunk-local origin.** `Sx_parent =
   Sx_local + x_offset · M` is the translation; without it no cross-chunk
   summary is possible at all.
4. **Both floods (`is_supported`, `detached_piece`) need a residency
   stop.** A 20,000-cell flood that pages chunks in violates M10's own
   verify criterion ("no hitching at chunk boundaries") directly.

**Is the model viable under streaming?** Yes, but only with (1) and (3).
The support *forest* is a shortest-path structure and those decompose
cleanly across chunks. The *load* is a subtree sum, which decomposes only
in the rootward direction — a chunk can summarise, for each boundary cell
that roots a subtree inside it, the `(mass, moment)` of everything beyond
it. That is 12 bytes per boundary cell, ~3 KB per chunk, and it is the
piece of design work M10 actually needs from this subsystem.

---

## 5. Procedural generation and the per-chunk pass

**Does the support-forest model admit a per-chunk formulation?** For the
distance half, yes, and cleanly — see §4c(1). Seams are correct exactly
when a chunk re-relaxes on any boundary decrease, which is the same
label-correcting rule `structural::tick` already implements, applied one
level up. Determinism holds because the field is a shortest path with a
stated tie-break, so generation order cannot change the answer.

**What breaks:**

1. **The pass does not run at all on an unbounded world.** Today that is an
   honest no-op; the moment M10 lands it is silent global inertia (§4a).
   This is the single thing to fix first, before any streaming work, and
   it is testable now: a bounded world whose chunks are generated
   incrementally reproduces the whole problem without any streaming code.
2. **Cost is `O(chunk volume)` through hashed `World::get`.** 7 ms for
   6,808 solid cells over a 512×320 world is ~1 µs per lookup, dominated
   by the seeding scan (issue #5's pattern, already named in the function's
   own doc). Per 64×64 chunk that is ~4 ms *per chunk generated*. At the
   rate a walking player crosses chunks that is not affordable, so
   "iterating chunks directly rather than a cleverer search" stops being
   the optional fix the doc calls it and becomes required work.
3. **"Generated terrain must be at rest" now includes "no cell over
   capacity".** The good news is that §1b's numbers make this far less
   dangerous than PLAN.md feared when the free span was 3 cells: attached
   stone is free to ~55·D, so a 6-deep cave roof spans 334 cells before it
   is in trouble. The bad news is that it is still a real constraint and
   `worldgen-design.md`'s own "genuinely unsolved" flag stands — the
   threshold moved, the need for a span-aware post-pass did not. Cheapest
   version: after generating a chunk, run `load::evaluate` over its surface
   cells only (the early-out already makes that surface-proportional) and
   thicken anything over ~0.6 stress. Cost: one surface pass per chunk,
   which is the same order as the seeding scan it rides along with.
4. **Worldgen must set `attached` on everything it places, without
   exception.** Any generator that forgets produces a chunk of foreground
   rock with a 12× capacity deficit, and it will come down. This is
   currently guaranteed by `build_terrain` doing it by hand at three call
   sites — an invariant with no enforcement. Worth making structural: place
   generated solids through one function that sets it.

---

## 6. Fun and feel, ranked

Ordered by how much they change what the hand feels, with cost stated.
Items 1–3 are the ones I would do before anything else in this document.

**1. Fail the *section*, not the cell — one fix for three symptoms.**
Because a slab decomposes into single-row chains (§1a), the failing region
handed to `rigid::fracture` is a **one-cell-thick strip**. Everything
downstream follows from that: `take_fragment`'s BFS was changed to BFS
specifically so fragments would be blocky rather than "thin individual
pixel lines", and it cannot help when the input is already one cell wide —
so a collapse delivers sticks and grit instead of blocks. The surviving
1-cell skin is the same fact from the other side: the top row is a separate
chain that has to fail on its own account. And `DETACH_DEPTH: 3` /
`CRACK_DETACH_DEPTH: 2` were both sized so that "pieces can only be as
thick as the loosened rock they came from" — a rationale the load model has
silently invalidated, since the piece's thickness is now set by the subtree
and is 1 regardless.

The fix is physical as well as cosmetic: rock does not part one lamina at a
time, it breaks across its section. When a cell fails, union its subtree
with the subtrees of the cells `section()` already walked across. Cost: one
extra flood bounded by `MAX_SECTION` (40), reusing the memo, on a path that
only runs when something has already failed. This is the highest-value
change available and it is not large.

**2. Split "resting on powder" into supported-but-not-exempt** (§1d.3).
Fixes the propped slab, the surviving skin's second cause, and the sand
cantilever. One predicate.

**3. Turn the model on for what the player builds** — §1d.1 and §1d.2
together. Until this lands, playtest feedback about building is feedback
about a system that is not running, and every conclusion drawn from it is
unsafe. Expect this to *create* complaints; that is the model finally
having an opinion.

**4. Give the player strain feedback before failure.** `Load::stress()`
exists, is exactly the right quantity, and is visible nowhere in the
running game — only in `filmstrip`. Two versions:
 - *Cheap and immediately useful to the owner:* wire it into the hover
   inspector (`I`), which is where `load-model-handoff.md` §9 step 3 asked
   for it and it did not land. Cost: nothing structural.
 - *The real answer:* **crack propagation along the stress ratio**, which
   is step 3 of `fracture-mechanics-design.md`'s own build order and is
   the principled version of "the beam visibly strains before it goes".
   Cracks are already rendered, already cut capacity, and already
   accumulate — a beam that cracks at its root as you overload it is the
   graded outcome §0a asks for, delivered by a mechanism rather than a
   tint. Cost: a bounded walk from the highest-stress cell per structural
   tick; note it will make failures *more* frequent, since cracks cut
   capacity.

A live per-cell strain tint is the tempting third option and it is the
expensive one: there is no per-cell storage for it (`flags` full, `aux`
taken), and re-deriving it at render time would defeat the dirty-rect skip
on exactly the settled worlds where that skip pays — the animated-grain
lesson, again.

**5. Landing owes feedback, and currently pays none.** `break_free` writes
a pressure impulse per broken cell and `fracture_with_impulse` writes one
per collapse, but `rigid::settle` writes nothing at all — a slab falls
forty cells, lands, and shoves no air, throws no dust, and marks nothing.
Cost: one `add_pressure_impulse` scaled by impact speed and cell count,
plus a few debris particles, at an existing call site. Cheapest item on
this list by a distance.

**6. There is no audio in this engine.** `design-philosophy.md` §0a names
"no sound of consequence" as one of the three things that makes a
destructive event unfinished, and the crate has no audio dependency and no
sound module. Every collapse in the game is silent. This is not a load-model
problem and it is not small, but it is worth stating plainly that a third
of the stated bar for "satisfying destruction" is not merely unmet, it is
unimplemented.

**7. Reconsider the `D³` strength scaling.** §1a's finding means thick
construction is `√D` stronger than the calibration intended, which reads as
"anything chunky is indestructible". If failures feel too rare once the
model is actually running on player structures, this is the knob — and it
is a *model* change (share load across a section's rows), not a constant,
so it belongs with item 1, which walks the same section.

**8. Bound the background brush, or replace it with the mortar verb**
(§2c). "Paint indestructible terrain anywhere, unlimited" is the right tool
for authoring test scenes and the wrong one for a game about building
things that can fall down.

---

## 7. What this review did not check

Stated so the next session does not assume coverage that is not here.

- **The parallel driver.** Every measurement above is `filmstrip`'s
  default, which is `parallel::step` — but nothing here compared it against
  `update::step` or `step_monolithic`, so none of the findings are
  attributed to the sweep decomposition versus the rules.
- **Wood, and `Plant`.** All arithmetic is stone. Wood's base is 32 and it
  has no `attached_span_bonus` or support costs of its own in `wood.ron`, so
  it is running on registry defaults that nobody has looked at under the new
  criterion. Trees route through `organism_structural_tick`, which still
  uses `max_unsupported_span` as a *reach* limit — the one place the
  superseded criterion is still live, and §5.7's exact trap.
- **The owner's "crumbles after a few seconds" report** was not reproduced.
  §1c(i) and §1d.2 together are a *hypothesis* about it with the right
  shape — it explains both the delay and the "except a perfectly vertical
  column" exception, since a column's torque is zero under any forest — but
  `CLAUDE.md` is emphatic that an ambiguous complaint gets resolved by
  measurement before anything is built on it. Reproduce it by painting a
  structure through the real brush and printing the standing count of cells
  with `aux == 0` per frame; that number going to zero over ~10 seconds
  confirms the mechanism, and its shape says which fix to build.
