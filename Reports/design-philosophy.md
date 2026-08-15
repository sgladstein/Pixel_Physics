# Design philosophy

**Audience:** the coding agent working on Pixel Physics.
**Status:** settled with the owner. Synthesizes `emergent-world-architecture.md`
§0/§1 (already most of a philosophy, stated inside an architecture document)
plus five new decisions from a dedicated design discussion, prompted by the
owner reviewing all four other `Reports/` documents and asking to make the
philosophy underneath them explicit.

**Read this one first.** The other four `Reports/` documents each solve a
specific problem — world architecture, worldgen, stigmergy, a bug backlog.
This is the short, opinionated statement of the philosophy underneath all of
them, in one place. Read the relevant longer report for mechanism and
citations; read this one when a new situation isn't covered by any of them
yet, and you need to know which way to lean.

**Settled with the owner — do not re-litigate:**

- **Everything should feel satisfying. This is a core ethos, not a polish
  pass.** A mechanic that is defensible in principle and dull in the hand has
  failed. See §0a — it is listed first because it has already overturned
  three separate models that were correct by their own tests.

- A tuning constant becomes `.ron` data **immediately**, if a non-programmer
  might plausibly want to tweak it. Not everything needs to be data-driven —
  purely internal constants stay in Rust. See §2.
- "No hardcoding" forbids **outcomes**, not simple rules. A weighted local
  rule built from tuned constants is fine; an authored curve shaped to
  produce a specific silhouette is not. See §2.
- The simulation itself never gets a designed win-condition. Authored
  objectives, if they ever exist, are a layer bolted on top of the emergent
  substrate for a specific game mode — never a change to how the world
  itself behaves. See §2.
- Private per-organism state is minimized aggressively, including state that
  already exists (`TreeState`'s attractor list), not just new code. See §2.
- **The organism model is being redesigned toward cell-typed, CA-native
  organisms — generalized to any species, not tree-specific.** This is a
  committed direction, not yet a finished design. See §3, which is the
  reason this document exists.

---

## 0a. The ethos: satisfying is a requirement, not a finish

Stated by the owner directly, and placed ahead of everything else here
because it outranks the rest when they conflict: **making everything feel
satisfying should be a core ethos.**

It is distinct from §0's emergence thesis and from `CLAUDE.md`'s "looks good
in motion." Emergence is about where behaviour *comes from*; looks-good is
about *appearance*; this is about **response** — what it feels like to act on
the world and have the world answer. A simulation can be emergent, look
correct, pass its tests, and still be inert to play.

The destruction work is the worked example, and it failed this bar three
times in a row while passing everything else:

- **Support inferred from geometry** (confinement, then thickness-scaling)
  made bulk terrain stable, which is what it was asked to do — and silently
  made everything the player built indestructible. Reported from play as "it
  only really takes effect for pretty narrow stone lines."
- **All-or-nothing breakage.** A failure produced one large coherent body or
  a uniform dissolve to powder. Real fracture is a size *distribution*, and
  the absence of the middle read as fake on sight.
- **No verb.** Destruction could only be provoked by erasing support, which
  applies no load and no impulse. There was no way to *hit* anything, so
  nothing ever broke from being struck, and the falling pieces dropped
  perfectly flat because nothing had imparted any rotation.

The general lessons, stated so they transfer past this one subsystem:

1. **Graded beats binary.** Where an outcome could be a spectrum, make it
   one. Binary outcomes are the single most reliable source of "this feels
   fake" in the feedback so far.
2. **Every destructive event owes feedback** — debris, an impulse, a mark
   left behind. `emergent-world-architecture.md` §5c already made this
   argument for the *field*; this generalizes it to the player's senses.
3. **A mechanic needs a verb.** If the only way to trigger something is an
   editor operation, it is not yet a game mechanic, however correct the
   simulation underneath is.
4. **Only play settles it.** None of the three failures above was visible in
   test output, and one of them was invisible in a rendered contact sheet
   too — it took printing a counter to discover the feature had never once
   fired. When judging feel, the artifact is the running game.

This does not license authored outcomes: §2b's line still holds, and the fix
for "unsatisfying" is a better *mechanism*, never a curve fitted to produce a
specific result.

## 0. Where this comes from

`emergent-world-architecture.md` §0–§1 already contains the core thesis; this
section is a two-paragraph pointer, not a restatement.

The owner's goal, in their own words: *a world that feels alive on its own
with unplanned emergent behavior that I can explore and interact with...
complex behavior from simple rules.* The mechanism for that, argued at length
in the architecture report, is one primitive repeated everywhere:
**deposit → diffuse → decay → follow.** Something writes a scalar into
shared world state, the scalar spreads and fades, something reads the
gradient and acts — which writes more. Fire → heat → ignition → more fire is
the one closed loop the engine had before this round of work; the priority
list that report drove (light, moisture, day/night, ash → soil, wind on
plants) exists entirely to close more loops, because **behavior count scales
with loops, not with systems written.**

The operational consequence, stated as a rule in §1 of that report and
reaffirmed here without exception: **if two systems need to interact, they do
it by reading and writing a shared piece of world state, not by calling each
other.** Agents — plants, creatures, and whatever comes after them — are thin
readers/writers of a rich, shared world. They do not hold private simulations
of their environment, and they never talk to each other directly.

---

## 1. Why this document exists

The architecture report's thesis is right but abstract enough to leave real
gray areas open in practice. Two rounds of discussion closed five of them —
four small, one large enough to be its own section (§3). The trigger was
concrete: the owner playtested the actual running game, found explosions and
fire lacking (fixed, see this session's `PLAN.md` entry) and — separately —
found tree growth unconvincing (mid-air start, uniform trunk thickness, roots
that don't fail sensibly on bare stone). That last one wasn't treated as a
bug to patch. It was treated as a question about how far "simple rules,
complex behavior" should actually be pushed, and that question turned into
this document.

---

## 2. Four boundaries, settled

### 2a. When a constant becomes data

`worldgen-design.md` §10 already named the gap: 56 hardcoded Rust `const`s
across `plant.rs`, `field.rs`, `creature.rs`, `structural.rs`, and only
materials are data-driven, despite `PLAN.md`'s own stated reasoning for
materials-as-data ("hot reload sidesteps slow compiles exactly where
iteration speed matters most") applying to all of them equally.

**Rule:** a constant graduates to a hot-reloadable `.ron` value immediately
if a non-programmer might plausibly want to tune it — growth rates, chances,
thresholds, anything gameplay-facing. It does not need to wait until someone
is actively mid-tuning-session on that system. Purely internal or structural
constants (array capacities, epsilon values, anything a design pass would
never touch) stay as Rust `const`s.

**Named honestly, not swept under:** this session's own explosion and
fire-animation work added `VAPORIZE_FRACTION`, `SHOCKWAVE_RADIUS_MULTIPLIER`,
`FIRE_TINT_LOW`/`FIRE_TINT_HIGH`, and `FLAME_FLICKER_STRENGTH` as Rust
constants under time pressure to ship a playtest fix — every one of them is
exactly the kind of value this rule says should be data. Recorded here as a
concrete instance of the gap, not fixed retroactively by this document; worth
migrating when that code is next touched.

### 2b. What "no hardcoding" actually forbids

The owner's own framing, verbatim: *"I don't want you to hardcode most of
these habits, we want to create realistic complex behavior from simple
rules"* — and later, confirmed directly: **the outcome is forbidden, not
simple rules.**

Concretely: you may not hardcode "a trunk widens at height 10" or any other
authored shape. You may absolutely use a simple, tuned, weighted local rule —
"branch with probability `p`, where `p` is shaped by a resource channel" is
exactly the pattern `plant.rs`'s existing auxin `channel` already uses
(reinforced on successful growth, decayed at dead ends), cited in
`emergent-world-architecture.md` §0's table as proof the primitive already
works inside this codebase. The test for any new rule: is the resulting
shape a side effect of a mechanism, or is it curve-fit to look a particular
way? The first is always fine. The second never is, no matter how the number
that produces it is stored.

### 2c. Scope: emergent substrate, optional authored layer

The simulation itself never gets a designed win-condition — this reaffirms
`PLAN.md`'s already-dropped puzzle-game target (*"emergence needs no
solvability guarantee"*), generalized past that one case. Future game modes
(platformer, roguelike — `PLAN.md`'s original target stack) may add an
authored objective layer on top later, but that layer sits **above** the
emergent world and never changes how the underlying simulation behaves.
Sandbox-with-optional-authored-layer, not sandbox-forever and not
game-first-emergence-second.

### 2d. Private state: minimize aggressively, including what already exists

`emergent-world-architecture.md` §1 already requires new code to prefer a
shared channel over a private scan — `is_damp`, `strongest_water_pull`, and
the worm's neighbour temperature scan are named there as violations that
arrived one reasonable local decision at a time, not by intent. **Extend
that standard backward, not just forward:** existing bookkeeping such as
`TreeState`'s attractor list is not grandfathered in just because it
predates the rule. Where it can be pushed toward a world-native
representation over time, it should be — see §3, which is the largest single
instance of this.

---

## 3. Organisms as cells, not objects

This is the section that took the discussion, and the direction that's
actually new relative to the other four reports.

### The current shape, and what's wrong with it

`plant.rs`'s `TreeState` is a private struct sitting beside the world: an
`attractors` point cloud, a `Vec<Tip>`, each `Tip` carrying a `channel`
scalar standing in for real transport capacity. It works, and space
colonization is a legitimately good algorithm for breaking L-system
symmetry — but architecturally it is exactly what
`emergent-world-architecture.md` §1 calls "the deep-sim pattern arriving by
default rather than by argument." A tree is not made of world state; it is
an object that writes into the world from outside it. That mismatch is very
plausibly *why* it reads as scripted to a player, independent of how good
the underlying algorithm is: the plant is a simulation running next to the
grid, not a thing living in it.

### The committed direction

**Organisms become cell-typed and CA-native — real `Cell`s carrying a small
type tag in `aux` (growing-tip, mature-wood, leaf, dormant-seed, root-tip,
and so on), with behavior driven by local rules reading state that already
exists** — the light field for phototropism, the wind/velocity field
(already wired for canopy lean), an upward bias for gravitropism, the
moisture field for hydrotropism. Resource transport (photosynthate down from
leaves, water up from roots) becomes a genuine diffusion-like process along
connected plant cells, not an abstracted `energy` pool. Only actively
growing or leaf cells stay on the M16 active-site schedule; mature cells go
fully inert — the "plants only change at their tips" principle M16 already
established for moss, applied to trees for real instead of approximated by a
`Tip` proxy.

No private per-organism struct survives this long-term. An organism *is* its
cells. Once built, this would close issue #8 (`TreeState` never shrinks) for
real, rather than the interim mitigation currently in place — there would be
no side arena left to leak. **Not done yet:** `TreeState` and its interim
mitigation are unchanged by this document; this is the direction the eventual
rewrite follows, not a description of the current code.

**Chosen over full biophysical simulation, explicitly, so it doesn't get
reinvented later.** A literal model — real auxin transport PDEs, distinct
xylem/phloem systems, hormone signaling — was considered and rejected. Real
hormone transport runs on timescales a game will always compress past
relevance, so building the actual biochemistry buys fidelity you immediately
have to fight against for playability, and it is not visually
distinguishable from the cell-typed tier at pixel-art resolution anyway. The
line is: **model the signals that drive the behavior (light, gravity, wind,
water), not the biochemistry that carries those signals in real plants.**

### Generalized on purpose, not tree-specific

The owner's own framing: *"build a flexible architecture that can be
modified to create new unique organisms and it isn't specifically designed
for a tree or a tree+moss+worm... think future proof."* This is a
commitment to **one substrate, many species defined as data on top of it** —
moss, worms, and whatever comes after them should eventually be expressible
in the same cell-typed shape, not each getting bespoke Rust systems the way
`plant.rs` and `creature.rs` currently diverge (`worm.ron`'s thermal numbers
are already data; its behavioral numbers are still `creature.rs` constants —
the exact split `worldgen-design.md` §10 flags as historical, not
principled).

### What this document does not resolve

This is direction, not a finished design. Explicitly out of scope here,
and left for a dedicated report written just-in-time when the tree rewrite
is actually scoped — the same way each of the other four reports was written
right before its subject was tackled, not speculatively far ahead of it:

- The data schema for authoring a "species" — what a `tree.ron`-equivalent
  looks like, and what exactly a cell type needs to encode within one `aux`
  byte.
- How the transport/diffusion pass actually works mechanically — whether it
  reuses `field.rs`'s existing diffusion machinery scoped to `Plant`-kind
  cells, or is a separate pass.
- Whether moss and the worm get retrofitted onto the new substrate
  immediately once it exists for trees, or are left alone until they're
  touched anyway for other reasons.
- How this interacts with M17's existing anchor-distance BFS — a cell-typed
  tree growing through real `Solid`/`Plant`-kind wood cells may not need any
  *additional* connectivity tracking of its own, since structural integrity
  already computes exactly that for the material grid. Worth checking
  before building a second mechanism that duplicates it.

**Deliberately generalizing from one case is a known trap** — the
architecture report's own standing review question (§12: "would this channel
have more than one consumer?") argues against building the general substrate
before a second real case exists to generalize from. The plan is: build it
for trees (the case actually motivating this), let moss be the second case
once the pattern has proven out, not before.

---

## 4. Open questions, deliberately not resolved here

- The full technical schema listed in §3 — data format, `aux` encoding,
  transport mechanics, migration order.
- Whether "realism as a calibration, not a constraint" (`worldgen-design.md`
  §9's framing — parameters start realistic, then get tuned for what's fun)
  needs its own explicit statement here, or whether it's adequately covered
  by §2b's outcome-vs-rule boundary. Current answer, per the owner: *"we
  will have to figure it out as we test"* — deliberately left to play-testing
  rather than decided in the abstract.
- Whether `CreatureState` should be folded into the same generalization
  effort as `TreeState`, or treated as a separate, later migration once the
  organism substrate exists and has a second real user.
