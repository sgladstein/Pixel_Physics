# Destruction eager, building forgiving — the plan

**Status:** plan, not built. Written after the load model landed
(`7e13e42`, `84541e3`) and after a multi-agent review produced
`Reports/prior-art-destruction.md` and `Reports/load-model-fit-review.md`.
This document is the synthesis: what to do, in what order, and why that
order.

**Read first:** `Reports/fracture-mechanics-design.md` for the model,
those two reviews for the evidence. This document does not repeat their
arguments, it decides between them.

---

## 0. The organising principle

The prior-art survey turned up an uncomfortable pattern. The games
*famous* for destruction — Noita, Teardown, Deep Rock Galactic — ship no
structural integrity model at all. Every shipped stress system is in a
**building** game, and two of the four (7 Days to Die, Medieval Engineers)
are remembered as frustrating.

The tempting conclusion is "structural models are a mistake". That is the
wrong reading. The right one is about **which direction the model points**,
because it is the same arithmetic either way:

| Direction | What the player experiences | Examples |
|---|---|---|
| **At destruction** | Consequence propagating out from something they did on purpose | Teardown, Red Faction |
| **At building** | The system saying *no* to a structure they are mid-way through authoring, for reasons invisible on screen | 7D2D, Medieval Engineers |

Both are "correct" physics. Only one is fun. And the failure mode of the
second is never "too fragile" — Medieval Engineers' complaints were
*"buttresses push into the legs"*, i.e. **unpredictable**, and Rust ships a
cruder model than ours that players tolerate because the number is written
on the hammer.

**We have already shipped the frustrating kind.** Reported from play
before any of this research existed: *"Most everything built in the
foreground (other than a perfectly vertical column) crumbles after a few
seconds."* That is 7D2D's failure, in our build, observed rather than
predicted.

So the principle, and the thing every item below is sorted against:

> **Destruction should be eager. Building should be forgiving.**

Eager: a blow produces consequence immediately, across a section, in
pieces, with feedback. Forgiving: what the player stacks does not fail
spontaneously, does not fail late, and does not fail for reasons that were
never on screen.

The model does not currently distinguish these. It fires the same way in
both directions, which is the worst of both.

---

## 1. Sequencing, and why legibility comes first

Every system in this class that failed, failed on legibility rather than
on physics. We are set up for exactly that failure: `attached` is an
invisible bit multiplying capacity twelvefold, and `section()` can differ
by 1,600x between two cells that draw identically.

`Load::stress()` is computed, is exactly the right quantity, and **nothing
in the running game reads it** — verified, its only consumer is
`examples/filmstrip.rs`. `load-model-handoff.md` §9 step 3 asked for the
inspector readout and it did not land.

Legibility also comes first for a selfish reason: it is how we will judge
everything else in this document. `CLAUDE.md` already says it — *"for
'does this look right', ship a runtime selector rather than choosing"* —
and this project keeps settling in prose what one overlay would settle in
minutes.

---

## Phase A — Make failure legible (before making it more correct)

Small, and it changes how every later phase gets judged.

**A1. Wire `Load::stress()` into the hover inspector (`I`).** The readout
the handoff asked for. Show mass, torque, capacity, stress ratio, and
which of the two failure modes would fire. Near zero cost — the inspector
already forces a full redraw while open.

**A2. A stress overlay behind a key.** Colour every structurally
interesting cell by stress ratio. This is Poly Bridge's primary UI and
Medieval Engineers' `N` view. Costs the dirty-rect skip *only while
enabled*, which is the animated-grain lesson and is acceptable on the same
terms.

**A3. Move the failure impulse to the cell that actually failed.**
`fracture_with_impulse` centres its pressure impulse on the region's
bounding box, which for a subtree failure is the middle of the falling
piece rather than the joint that gave way. Small change, and it is the
difference between *"the neck snapped"* and *"some rock fell"*.

**A4. `filmstrip` should report which failure mode fired**, and how many
of each. A contact sheet cannot distinguish an overload failure from an
unsupported-region failure, and those are two different mechanisms we will
need to tell apart constantly. This is `CLAUDE.md`'s own "did it fire at
all needs a counter, not a picture", applied before we need it rather than
after.

---

## Phase B — Turn the model on for what the player builds

**This phase must land before any further playtest feedback about
building is trusted.** The fit review's central finding is that the model
is currently *switched off* for player structures by three "cannot fail"
escape hatches, none of which is part of the design:

1. `aux == 0` is treated as proof of anchorage — and everything a player
   paints starts at `aux == 0`. `rigid::settle` also writes `aux = 0`, so
   **landed debris becomes a permanent anchor**.
2. A cell with powder beneath it is exempt via `is_resting_on_ground`.
3. A structure large enough to exhaust the per-frame budget is exempt.

Every one is *binary immunity* — precisely what the four earlier support
models were rejected for. The mountain-vs-tower tension has not re-opened;
it has moved out of the failure criterion, where it was visible and
argued about, into the guards, where it is not.

**B1. Stop treating `aux == 0` as proof of anchorage.** `is_anchor`
already reads the world rather than the cache and its doc says why; apply
the same discipline at `evaluate_within` and `support_parent`. Two extra
neighbour reads on a path that already does four.

**B2. Split "resting on powder" into supported-but-not-exempt.** Powder
should terminate the support chain — so landed debris does not shatter,
which is the bug the predicate was written for — *without* zeroing `aux`
and without skipping the torque test. A granular pile's bending capacity
is approximately none, and saying so is the whole fix. One predicate,
and it pays for the propped ligament slab, the surviving 1-cell skin, and
the sand-sprinkle infinite cantilever at once.

**B3. Give the brush the converged pass generation already gets.** At the
end of a stroke, run the `compute_world_distances` relaxation scoped to
the stroke's bounding box plus a margin. Today a painted structure relaxes
reactively at one cell per 5-frame round, so a 192-tall column needs ~192
rounds — which is the shape of *"I built a thing and it collapsed ten
seconds later"*. One Dijkstra over the stroke area, once.

**Expect B1–B3 to create complaints.** That is the model finally having
an opinion, and it is why Phase C exists.

---

## Phase C — Make building forgiving

Having turned the model on, make it generous in the direction that
matters.

**C1. Keying, answered from the support parent.** The owner's ask —
*"we should be able to attach foreground objects to background objects"*.
The load path already crosses the interface; only the *capacity* at the
joint does not. So read the attachment bonus from the **support parent's**
`attached` bit rather than the cell's own.

This cannot chain, which is the whole reason it works: the bonus is a
property of an *edge*, not a cell, so painting A against a cliff does not
make A a source of the bonus for B. The "but the floor is attached"
objection dissolves in the right direction — it lands on foundations, which
is where a keyed joint belongs. Stateless, no new bits.

**C2. Mortar as a *material*, not a flag.** For the deliberate,
player-facing version of keying. `Cell::flags` is 8/8 full and `aux` is
taken, so a flag has nowhere to live — but a material costs zero bits, is
hot-reloadable, renders visibly, and is itself destructible. A joint you
can see and a joint an enemy can break are both better than an invisible
bit.

**C3. Fire on events, not spontaneously.** Destruction eager, building
forgiving, stated mechanically: failures should overwhelmingly be
triggered by blows, blasts and undercutting rather than by a structure
re-evaluating itself while the player is still working. Related, and
flagged by the prior-art review as the 7D2D bug's exact shape:
`ROOTWARD_CHECK_STEPS = 128` lets a blow bring down rock a hundred cells
away, frames later. Bound the chain walk by **distance from what actually
changed** rather than by step count.

⚠️ This conflicts with that constant's own doc comment, which records that
16 was too small and left `scene=ligament`'s neck standing at stress 1.87.
So it must be verified against that scene by eye before it is believed;
if it re-breaks it, the answer is a disturbance-anchored bound, not a
smaller number.

**C4. Bound the background brush, or replace it with C2's verb.** "Paint
indestructible terrain anywhere, unlimited" is the right tool for
authoring test scenes and the wrong one for a game about building things
that can fall down.

---

## Phase D — Make destruction eager and chunky

**D1. Fail the *section*, not the cell.** The highest-value change
available, and one fix for three separate symptoms.

Because the `NEIGHBOURS_4` tie-break makes horizontal parents win, a slab
decomposes into **independent single-row chains** — measured, not
supposed: `mass 115 torque 6555` on a 12-deep shelf is one row, exactly
`114·115/2`. Three consequences follow, and all three are on the current
bug list:

- The region handed to `rigid::fracture` is a **one-cell-thick strip**, so
  a collapse delivers sticks and grit. `take_fragment` was changed to BFS
  specifically to stop fragments reading as "thin individual pixel lines"
  and it cannot help when its input is already one cell wide.
- The surviving 1-cell skin is the same fact from the other side: the top
  row is a separate chain that must fail on its own account.
- `DETACH_DEPTH: 3` and `CRACK_DETACH_DEPTH: 2` were both sized so "pieces
  can only be as thick as the loosened rock they came from" — a rationale
  the load model silently invalidated.

The fix is physical, not cosmetic: rock does not part one lamina at a
time, it breaks across its section. When a cell fails, union its subtree
with the subtrees of the cells `section()` already walked across. One
extra bounded flood, reusing the memo, on a path that only runs after
something has already failed.

**D2. Landing owes feedback.** `break_free` writes a pressure impulse per
broken cell and `fracture_with_impulse` writes one per collapse, but
`rigid::settle` writes **nothing**: a slab falls forty cells, lands, and
shoves no air, throws no dust, marks nothing. One `add_pressure_impulse`
scaled by impact speed and cell count, plus a few particles, at a call
site that already exists. Cheapest item in this document.

**D3. Material-scaled fragment ladder.** The power-of-two ladder in
`rigid::fracture` is already the right *shape* — prior art confirms nobody
derives debris size from physics; Red Faction authors shard scale per
material and UE5 Chaos uses a cluster hierarchy with per-level thresholds.
The upgrade is to move the ladder into `.ron` so slate and granite break
differently, and to drive `size_bias` from **delivered surplus** rather
than brush radius.

---

## Phase E — The model change that pays for itself

**E1. Push damage outward from the break, instead of pulling loads to the
root.** This is Unreal Chaos's break-damage propagation and it is the most
transferable idea the survey found. When a cell fails, carry the surplus
`torque − capacity` outward into its neighbours in the support graph,
where it adds to their effective load and can carry them too.

Three things at once:

- **Cost bounded by the failure**, not by the structure. It replaces the
  rootward walk that currently runs on every settling cell in a disturbed
  structure, which is where our frame time goes.
- **A graded size distribution for free** — a small surplus takes a few
  cells, a large one carries several levels out. That is a descending-
  threshold hierarchy without an authored one.
- **Locality by construction**, which is C3's requirement delivered by the
  model rather than by a cap.

Deliberately last: it is the largest change, it subsumes several earlier
items, and it should be attempted only once Phase A can show whether it
made things better.

---

## Phase F - Improve the testing environment

Requested by the owner, and by the time it was requested this document
had already accumulated the evidence for it three times over. Every item
below is a real cost already paid, not a hypothetical.

**F1. Timings on this machine are not trustworthy, and nothing says so at
the point of use.** Contention has produced 18.0 ms twice in a row on
`scene=terrain` - a scene with **zero** pending sites and zero awake
chunks, which cannot be doing structural work at all - and 40.5 ms and
55.6 ms on scenes that measured 14-19 ms moments later. Three separate
near-misses where a phantom regression was almost chased. `filmstrip`
should run each measurement a few times and report the **minimum** with
the spread beside it, so a noisy sample is visible as noise rather than
read as a result.

**F2. The acceptance cases are run by hand and can pass vacuously.** The
five cases in `load-model-handoff.md` section 7 are checked by rendering a
contact sheet and looking, which is right and must stay - but nothing
*runs* them. That is how `scene=capped` came to be reported as passing
while the entire 15,840-cell structure was frozen and had never been
evaluated: the assertion "it still stands" was true and meant nothing. An
acceptance harness should assert, per scene, both the outcome **and** that
the mechanism fired - `failures: overloaded N` is already printed, and a
scene that passes with `N == 0` when it should collapse is exactly the
vacuous case.

**F3. There is no way to replay what the owner saw.** Playtest reports
arrive as prose and screenshots, and every one so far has had to be
reconstructed into a `filmstrip` scene by hand - sometimes wrongly, and
"resolve an ambiguous complaint before building anything" is already a
`CLAUDE.md` rule paid for in a wasted detour. A record/replay of input
against a seed, or simply a "dump the current world to a scene file" key,
would turn a report into a reproduction.

**F4. `cargo test` takes ~90 seconds and the release filmstrip build
dominates the edit loop.** Not urgent, but it is the reason measurement
gets batched, and batching measurement is how a regression gets attributed
to the wrong change.

**F5. Nothing guards the frame budget.** `examples/ascii.rs` reports
worst-frame and CI runs it, but no test fails when a scene regresses from
4 ms to 40 ms. Given F1, that guard has to be built on the minimum of
several runs or it will be permanently flaky - which is probably why it
does not exist yet, and is not a reason to keep going without it.

---

## Pending owner verification

Things that are built, tested, and **not yet judged by the person whose
judgement decides them**. Recorded because `CLAUDE.md` is emphatic that
tests passing is not evidence the screen changed, and three models have
already shipped that passed everything and were rejected on sight.

- **Does the build envelope feel right?** C1 keys a structure into terrain
  at the joint. Verified: it applies at the joint and does not chain.
  *Unverified:* whether what you can now build is satisfying, which is a
  taste question and cannot be settled from here.
- **Does destruction feel eager enough now?** B1/B2 removed three immunity
  hatches. The counters say the criterion fires; nothing says it feels
  good.
- **Is the collapse *timing* right?** The per-frame load budget paces
  cascades deliberately, and `scene=ligament` resolves over ~350 frames.
  Progressive was the design intent; sluggish is the risk.
- **Does the landing impulse read?** D2 added one; it moves smoke and
  grit, and whether that registers has to be seen in motion.
- **A2's key binding** for the stress overlay, and whether it should be a
  `GrainMode`-style render selector or a separate toggle.

---

## What we are explicitly not doing

- **A real stiffness solver, at any point on the roadmap.** Category
  error: Poly Bridge and Besiege solve tens of *authored* members, not
  hundreds of thousands of cells.
- **A live per-cell strain tint.** No per-cell storage exists (`flags`
  full, `aux` taken), and re-deriving it at render time defeats the
  dirty-rect skip on exactly the settled worlds where that skip pays —
  the animated-grain lesson again. A2's on-demand overlay is the version
  that survives.
- **Truncation caps that gate *whether* something fails.** Already bitten
  us twice; `MAX_SUBTREE_CELLS` was the third instance and is fixed.

## Stated plainly, and not scheduled

**There is no audio in this engine.** `design-philosophy.md` §0a names "no
sound of consequence" as one of three things that make a destructive event
unfinished. The crate has no audio dependency and no sound module, so
every collapse in the game is silent. A third of the stated bar for
satisfying destruction is not merely unmet — it is unimplemented. That is
not a load-model problem and it is not small, but it should not stay
unsaid.

---

## Order of work

```
A1 A3 A4   legibility + the neck reading as the neck   DONE
B1 B2 B3   turn the model on for built structures      DONE
D2         landing feedback                            DONE
C1         keying from the support parent              DONE
F1 F2      trustworthy timings, acceptance harness     (small, next)
D1         fail the section, not the cell              (medium, highest value)
A2         stress overlay                              (small)
C3 C4      locality, and bound the background brush    (small)
D3 C2      fragment ladder, mortar                     (medium)
E1         push damage from the break                  (large, subsumes work)
```

`D2` is placed early only because it is nearly free and improves every
subsequent playtest.

`F1` and `F2` move ahead of `D1` deliberately. `D1` is the highest-value
*behaviour* change left, and it is precisely the kind whose effect has to
be judged by measurement and by eye - which is the thing currently least
trustworthy. Fixing the instrument before taking the reading is cheaper
than doubting the reading afterwards, and this document now has three
worked examples of the alternative.
