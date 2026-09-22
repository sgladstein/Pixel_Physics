# What controls creature movement, in detail

*2026-09-22. Written from the source on `claude/upbeat-shannon-cez0w4`, every
link read rather than recalled. Owner's ask: a report on exactly what controls
movement.*

**Scope**: the walking path — how a creature decides *whether* to step, *which
way*, and what can refuse it. Flight (`Impulse`/`launch`), swimming and the
`Crossing` state are named where they branch off and not developed.

**Why this exists**: the ant line spent 2026-09-21/22 measuring a foraging loop
that fails at movement, and three separate wrong diagnoses were reached because
the chain below was assembled from memory. It has six stages and a decision can
die at any of them.

---

## 0. Summary — the six stages

| # | stage | decided by | can refuse? |
|---|---|---|---|
| 1 | **Is it this creature's turn?** | `tick_interval`, scaled by `TRAIT_PACE` | — |
| 2 | **Step at all?** | `p_move`, one RNG draw | yes, silently |
| 3 | **Fly instead?** | `Impulse` | branches away |
| 4 | **Which of three cells?** | `Turn`, `Persist`, `Caution` + footing, via `choose_weighted` | — |
| 5 | **Is the landing legal?** | passability, foothold, stack cap | yes |
| 6 | **Nothing ahead works** | `tumble` / crossing / reversal | the step is lost |

**The pheromone reaches stage 2 and nothing else.** Stage 4 — the only stage
that chooses a *direction* — has no pheromone term anywhere in it.

---

## 1. Whose turn it is

A creature is scheduled by `organism_tick_interval`, which is
`CreatureDef::tick_interval` scaled by the animal's expressed `TRAIT_PACE`
(`tick_interval_of`, `.max(1)`). `assets/species/ant.ron` authors
`tick_interval: 6`.

**This is the denominator of every movement rate**, and getting it wrong cost a
wrong headline in `ant-forage-bed-and-gates-2026-09-21.md` §11b: a per-frame
movement figure was quoted against an implied ceiling of 100% when the real
ceiling is **1 in 6 ≈ 16.7%**. Any "moves per frame" number must be divided by
this to become "moves per opportunity".

## 2. Whether to step at all

```rust
let p_move = outputs[BrainOutput::Move as usize].clamp(0.0, 1.0);
world.creature_stats.p_move_hist[...] += 1;
if draw.unit_f32() < p_move {          // ... walk or launch
} else if draw.unit_f32() < unit_scale(outputs[BrainOutput::Tumble], 1.0) {
    tumble(world, organism, def, &mut draw);
}
```

Three things follow, and all three were misread at some point this week:

- **The move and the tumble are exclusive.** The tumble is the `else` branch, so
  a creature that steps does not re-roll its heading on that tick, and one whose
  `p_move` roll fails gets a chance to turn instead. An animal with `p_move == 0`
  *always* falls through to the tumble roll.
- **`p_move` is a probability, not a decision.** A high `p_move` still fails
  sometimes; `p_move == 0.0` can never move, and the histogram's bucket 0 is
  that exact zero deliberately (§Z13).
- **The clamp manufactures exact zeros.** `Move` is a `squash` output in
  `(-1, 1)`; everything at or below 0 clamps to exactly 0.0. So a sufficiently
  negative weighted sum does not merely discourage stepping, it forbids it.

### What is wired into `Move` on the shipped ant

| wire | weight |
|---|---|
| `(Bias, Move, 2.0)` | +2.0 constant |
| `(HomeAligned, Move, 3.0)` | the homing cosine — **the largest single term** |
| `(Stillness, Move, 1.5)` | |
| `(KinNeed, Move, 1.25)` | |
| `(Energy, Move, -1.75)` | |
| `(FoodAdjacent, Move, -1.16)` | |
| `(Alarm, Move, -1.0)` | |
| `(Crowding, Move, -0.3)` | |
| `(0, Move, 2.5)` `(1, Move, -2.5)` | hidden pair fed by **`PheroAAlong`** (±6.0) |
| `(2, Move, 2.5)` `(3, Move, -2.5)` | hidden pair fed by **`PheroBAlong`** (±6.0) |

**This is the whole of the pheromone's influence on movement.** Both planes
enter as `Along` — the forward gradient — through hidden units 0–3, and those
units output to `Move`. The pheromone is a **throttle**.

## 3. Fly instead (branch, not developed here)

Inside the move branch, `Impulse` is read raw and gated strictly positive, so a
species that never authored it takes **no RNG draw** and stays bit-identical.
A successful `launch` sets `left_the_spot` but deliberately not `moved` — which
matters downstream, because `moved` is what gates the pheromone deposit.

## 4. Which way — and there is no pheromone here

`step_chain` offers exactly **three** candidates, a forward cone:

```rust
let dirs = [(heading + AHEAD_LEFT) % 8, heading, (heading + AHEAD_RIGHT) % 8];
let turn    = outputs[BrainOutput::Turn as usize];
let persist = unit_scale(outputs[BrainOutput::Persist], PERSIST_MAX);   // 2.0
let footing = unit_scale(outputs[BrainOutput::Caution], FOOTING_MAX);   // 1.2
let base    = [turn.max(0.0), persist, (-turn).max(0.0)];
```

then per candidate:

```rust
scores[i] = if passable[i] {
    base[i] + if body_has_foothold(...) { footing } else { 0.0 }
} else { 0.0 };
```

and the pick:

```rust
let pick = choose_weighted(&scores, CHOICE_EXPLORATION_K, draw.unit_f32());
```

`choose_weighted` is `(k + max(s, 0))²` normalised, with `CHOICE_EXPLORATION_K =
0.1` — Deneubourg's choice function, and its noise is load-bearing (P-10: never
replace it with an argmax).

**Four consequences.**

- **A creature can only turn 45° per step.** The cone is `heading ± 1`. Reaching
  the opposite heading takes four consecutive steps, each of which must win its
  own `p_move` roll.
- **`Turn` is the only steering input, and on the shipped ant it carries one
  wire**: `(TempAboveAmb, Turn, -0.8)`. Temperature.
- **`PheroALateral` / `PheroBLateral` — the left/right difference, the only part
  of a pheromone reading that carries *which way* — are wired to nothing.** Zero
  non-comment occurrences as a source in `ant.ron`. Two comment lines above the
  `Turn` wire claim they are "now via hidden units 0–3"; those units read
  `Along`, and output to `Move`. **The comment asserts a rerouting that did not
  happen.**
- **Footing outweighs persistence in practice.** `footing` (≤1.2) is added to a
  `base` whose middle term is `persist` (≤2.0), and the comment records why it
  is *added* rather than multiplied: multiplying left a step into thin air at
  16% probability and the colony spent 59% of its moves falling.

## 5. What can refuse a landing

`passable[i] = landing_is_placeable_through_tissue(world, &chain, &landing, push, stacker)`,
computed over **every cell of the body after the step** (`body_after_step`), not
just the head. Refusals come from:

- **Occupancy** — with `Stacker` deciding whether a nestmate counts, which is
  `World::stack_cap` / `PIXEL_PHYSICS_STACK_DEPTH`, default 1.
- **Body geometry** — a rigid 2-wide creature cannot enter a 1-wide gap. This
  is, with no other code, why a wide predator cannot follow a narrow ant into
  its tunnel.
- **Foothold** — `body_has_foothold`, which is a *discount*, not a veto: an
  unsupported candidate scores `base[i]` with no footing bonus, so walking off a
  ledge stays possible.

## 6. Nothing ahead works

```rust
let footing_ahead = passable.iter().zip(&scores).any(|(&p, &s)| p && s > footing * 0.5);
if !passable.iter().any(|&p| p) || !footing_ahead { ... }
```

**A creature with all three candidates in open air counts as blocked**, which is
what stops ants marching into the sky. On that path, in order: a `trunk_crossing`
if the next cell is woody; the blocked census (off unless asked); the `is_boxed`
reversal (`ReverseRule`, and only for a laden animal boxed by terrain); and
otherwise `tumble`. **The step is lost and nothing is deposited** (P-11).

## 7. The tumble

`tumble` re-rolls `state.heading` **uniformly among the viable headings** — the
subset of all eight that pass the same passability and foothold predicates the
walk uses. The comment records why not uniformly over all eight: on flat ground
three of eight point into open air, so a uniform re-roll lands back in the
blocked state a third of the time (29,344 blocked ticks against 41,843 moves).

### The one steering mechanism that exists, and its gates

`home_weighted_pick` can override the uniform re-roll with the viable heading
whose cosine to a target is greatest. For a food-carrying animal:

```rust
(def.home_bias * fill, state.forage_anchor)
```

and it returns `None` — falling back to uniform — on **any** of:

| gate | meaning |
|---|---|
| `viable.is_empty()` | nowhere to turn |
| `def.home_bias <= 0.0` | species opts out (`ant.ron` authors **1.0**) |
| `state.crop?` | **not carrying anything** |
| `fill <= 0.0` | crop empty by worth |
| `len < 1.0` | **standing on the anchor** — a point is not a direction |
| `draw.unit_f32() >= home_bias * fill` | probabilistic, scaled by how full |

So the only thing in this engine that can aim a walking creature is available
**only to a laden animal, only when it is off its anchor, and only with
probability `home_bias × fill`** — and it steers toward `forage_anchor`, which
re-anchors to whichever nest cell was last touched (`ant-forage-bed-and-gates-2026-09-21.md` §8c).

### Measured: it does not visibly bias the re-roll

`home_bias: 1.0` ships on, so the question is not whether the lever exists but
whether it acts. Isolating tumbles directly — a heading change with **no**
displacement, which the exclusive `if`/`else if` above makes a clean signature —
and keeping only laden animals **off** their anchor, the population the gates
admit:

| | measured | chance (uniform over 8) |
|---|---|---|
| new heading has a homeward x-component | **31.2%** | 37.5% |
| new heading is vertical (`dx == 0`) | 29.6% | 25.0% |

**n = 372**, 4 seeds. The re-roll is at or slightly *below* chance homeward for
exactly the animals `home_weighted_pick` is written for. That is consistent with
the whole-population heading distribution measured independently on the foraging
bed — homeward 32.5% against a uniform 37.5%.

**Treat this as a flag, not a verdict**: n is small, and the sample comes from
one bed. What it rules out is the comfortable reading that the lever is working
and merely under-reported. It is authored on, it is gated to the right animals,
and on the evidence available it does not move their headings. **The honest next
step is a counter inside `home_weighted_pick` recording calls, gate rejections
by reason, and firings** — none of which exists, which is why this had to be
reconstructed from position traces.

**And its readout has a denominator problem worth naming.** `trailfollow` prints
`tumbles N (homeward M, x%)` with *all* tumbles as the denominator — but empty
animals are structurally incapable of a homeward tumble (`state.crop?`), and
they are most of the population. Measured rates of 0.51% and 1.25% are therefore
**not** evidence the lever is inert; they are mostly a count of ants that could
never have qualified. The honest denominator is *tumbles by laden animals off
their anchor*, which nothing currently reports.

## 8. What this means for the trail

- **The deposit is gated on `moved`**, so a creature that does not step lays
  nothing. A trail is therefore a record of *movement*, not of intent — and
  `emit_b` can read 0.71 on every tick of an animal that lays nothing at all.
- **The pheromone gradient can only speed a creature up or stop it.** It cannot
  turn it. A reading that says "worse that way" does not redirect the animal; it
  subtracts from `Move` and the animal stands still.
- **So the loop's homeward leg is a duty cycle**: the animal steps when the
  forward cone happens to contain home, which the tumble reaches by chance.
  Measured on the foraging bed: facing homeward, 72.1% of decision ticks move;
  facing any other way, 5.7%.

## 9. Open, and named elsewhere

- **`open-bugs-handoff.md` §R4** — `Turn` is structurally inert on flat ground:
  both outer candidates fail their own checks, so *"a walking creature on level
  ground cannot be steered; it can only be scattered."* Any plan to wire a
  gradient into `Turn` must fix this first or it will act through nothing.
- **`ant-navigation-plan-2026-09-20.md`** — *"we ship Deneubourg's function with
  the pheromone removed from its arguments"*: `choose_weighted` is the right
  function and `s` is `persist + footing`.
- **The lateral wires** (§4) are a documentation defect as well as a design one.
