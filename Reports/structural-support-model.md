# Replacing the support field — what it is for, and what each replacement costs

**Status: design and measurement. Nothing here is proposed for merge.** One
new read-only instrument (`examples/support_census.rs`) and four env-gated
probes: `STRUCT_NO_CLIMB` and `STRUCT_NO_GROUND_ROOT` in `structural.rs`,
`SETTLE_AUX_MAX` in `rigid.rs`, and `ORACLE_COLUMN` on `scale_probe`. None
changes a default, and one `[struct]` census field (`grounded`) was added
because an ablation without it is unreadable. Arm A in §5 reproduces
`open-bugs-handoff.md` §S's recorded line byte-for-byte (`worsened 1400
improved 9 unmoved 267 | budget0 723 | max aux 438` at frame 2,400), which is
what says the default path is untouched.

Written 2026-08-26, after §S's *"the framework is the bug, not the tactics"*
noted that `Cell::aux` carries two jobs and sketched a two-part replacement —
a short-horizon saturating gradient plus connectivity over the coarse chunk
layer — with the note that **nothing there was sized**. This sizes it.

Five things came out, and two of them contradict what this report set out to
show:

1. **The coarse-layer half of the sketch is right, within 1%.** 5,169 nodes
   against the estimated 5,120, and the pair `(level, offset)` fits the `u16`
   that is already there.
2. **The short-horizon half, taken literally, is falsified.** Clamping the
   existing field at the sketch's own suggested horizon of 64 strands
   **95.38%** of body cells.
3. **But the sketch's *diagnosis* is better founded than §S argues, and this
   report's own preferred alternative died on it.** Recovering a wrong
   distance *upward* costs one relaxation round per unit, so the field's range
   is the price of any accident in it: a region knocked to zero inside a
   massif owes ~2,400 rounds, which at `STRUCTURAL_TICK_INTERVAL` is the
   eleven thousand frames §S measures. **The range is the cost**, and
   suppressing the recovery (§5) makes the field worse, not better.
4. **§S's "the error is manufactured, not delivered" needs one correction of
   sign**, and it redirects the fix: the affected cells read *too low*, not
   too high, which makes the damaged region a **load sink**. See §1.2.
5. **And the trigger is one line.** A write-seam trap named it: every thrown
   rock lands at `aux 0` — *bedrock-adjacent* — because
   `particle.rs::landed_cell` carries `aux` only for materials flagged
   `worth_in_aux`, which is the **food-value** flag. Landing at `u16::MAX`
   instead takes the oracle's wrong-cell count from **37,629 to 186** and the
   scheduler from 14.68 ms to **0.03 ms**, with the queue back at its idle
   value and *more* rock standing. **§S is not a framework bug** (§6).

Read `open-bugs-handoff.md` §S first for the bug, and `dead-ends.md`'s
structural section for the three attempts already withdrawn.

---

## 1. What a replacement has to reproduce, stated exactly

§S establishes that every consumer of `aux` in `load.rs` is a comparison
rather than a magnitude. Reading all of them again, the statement can be made
sharper, and the sharper form is what makes the question measurable:

**Every consumer reads the field as a strict order over a cell's four
neighbours, and nothing else.**

| site | what it computes | what it needs from the field |
|---|---|---|
| `load::support_count` | `n.aux() < own` | which neighbours are below me |
| `load::dependants` | `n.aux() > own` | which neighbours are above me |
| `load::support_parent` | argmin of `n.aux() + step` over the strictly-lower ones | a choice *among* those, on local differences |
| `load::chain_reaches_anchor` | follows `support_parent` | nothing beyond the above |
| `structural::tick` | `relaxed == u16::MAX` | reachability, once — see §1.1 |

So define, for each body cell, the four bits *"which of my four neighbours
does the field say are holding me up"*. **Two fields that produce the same
four bits at every cell are indistinguishable to `load.rs`, whatever their
magnitudes.** That is the equivalence class a replacement has to land in, and
it is exactly what `examples/support_census.rs` measures. Everything below is
a statement about how far outside that class a candidate falls.

### 1.1 The reachability job is smaller than §S says, because `load.rs` already answers it twice

§S proposes building connectivity over the coarse layer. Before building a
third answer to that question it is worth writing down that there are already
two, and that neither reads a stored distance for its verdict:

- **`load::is_anchor` is a physical test**, not a field read:
  `touches_bedrock(world, x, y) || rests_on_ground(world, x, y)`. Its own doc
  says why in as many words — *"Read from the world directly rather than from
  `aux == 0`, deliberately. A stored distance is a cache that lags a
  disturbance by up to a tick; this is the thing the cache is of."*
- **`load::is_supported` falls back to a bounded flood** of the connected
  region when the chain fails, and that flood *"does not consult a stored
  distance for anything except as a shortcut"*. It terminates on the first
  cell with an intact chain, so the supported case stays cheap.

The stored `u16::MAX` is consumed as reachability in exactly **one** place:
`structural::tick`'s `grounded_root` last resort (`relaxed == u16::MAX &&
is_resting_on_ground`).

That does not make a coarse layer worthless — §4.6 has the one case the flood
gets wrong — but it does change the pitch. The coarse layer is not the missing
answer to reachability. It is a *third* answer, exact where the existing flood
is bounded and conservative.

### 1.2 The sign of the error, which §S has the wrong way round

§S reports the affected set as *"cells wrong"* with a `|delta|` histogram —
an absolute value — and the reading that has circulated from it is that the
cells read *too high*, hence "weaker than they are". **The oracle prints the
sign and it is the other way.** Every arm in §5, including the untouched
baseline:

```
changed 37629 (0.19% of body), of which rose 37629, largest rise 65329
sample: (3846,148) 247->2453   (3879,184) 198->2384   (3888,201) 218->2360
```

`old -> now`, where `now` is what the converged pass computes. **Every one of
the 37,629 cells stores a value below the truth**, by ~2,200. The damaged
region reads as *better* supported than it is, not worse.

That matters because it names the harm channel. `structural::tick`'s own
comment already describes this failure, for the ground root:

> Rooting a cell at 0 the moment powder touches its underside makes it a
> **load sink**: every neighbour with a longer path re-routes its load into
> it, which is exactly "a sprinkle of sand under a beam holds the beam up".

A stale-low region is that sink at scale. Cells outside it hold correct values
near 2,450; cells inside store ~250. Through `load::dependants` (`n.aux() >
own`) the stale-low cells inherit the subtree of the whole correctly-valued
region above them, blow past `capacity`, and fail as **Overloaded**. That is
consistent with §S's control — **25,470 more body cells standing** with a
converged pass — without needing the span comparison that
`compute_world_distances`' doc still claims and that the load model no longer
contains (`max_unsupported_span` is a per-material constant fed to
`capacity_within`; it is never compared against `aux`).

This is the mechanism the code supports, not one this report measured. §8
names the measurement that would settle it.

---

## 2. The instrument

`examples/support_census.rs`, new, read-only. It generates a world, runs
`compute_world_distances`, then builds candidate fields beside the real one
and compares the four bits above, cell by cell. No simulation runs and no
default changes.

**Its controls are the reason its nulls mean anything** (`CLAUDE.md`: a null
looks the same whether the mechanism is quiet or the probe never reached it).
`control=1` runs two:

| control | reads | says |
|---|---|---|
| flat-zero field vs exact | **97.20% differ**, 178,822 held→unheld | the comparison can see a difference |
| exact field vs itself | **100.00% same**, 0 differ | and does not manufacture one |

---

## 3. Candidate A — the short-horizon saturating gradient: **falsified**

§S's sketch: *"a short-horizon field, saturating at something like 64. It is
local by construction, so damage propagates 64 cells and stops."*

Its premise is measured and true, and §5 turns out to support it more strongly
than §S itself does. The mechanism as literally specified destroys the load
DAG for nearly the whole world.

### 3.1 The premise holds: the magnitude really is unread

Body-cell distance after `compute_world_distances`, `preset rolling`:

| | 1024x320 s1 | 2048x640 s1 | 2048x640 s7 | 2048x640 s24301 | **8192x2560 s1** |
|---|---|---|---|---|---|
| body cells | 183,967 | 1,007,468 | 980,040 | 870,906 | **19,386,828** |
| max | 267 | 933 | 595 | 1,353 | **2,577** |
| mean | 86.3 | 223.7 | 211.7 | 355.9 | **974.4** |
| at or under 48 | 33.58% | 14.88% | 14.90% | 16.32% | **3.40%** |
| over 256 | 0.04% | 40.47% | 36.73% | 48.82% | **83.43%** |

At the shipped size **96.6% of the field is a number no consumer walks far
enough to read** — `ROOTWARD_CHECK_STEPS` is 48. §S's claim is confirmed, and
it gets *worse* with world size, which is the direction that matters given
M10.

### 3.2 And the mechanism fails on the same fact

Clamping is not free, because the consumers read an **order**. Clamp every
value at a horizon `H` and any cell whose four neighbours all sit at or above
`H` loses its last strictly-lower neighbour: `support_count` returns 0,
`dependants` returns nothing, `support_parent` returns `None`. That is not a
weaker answer, it is a different question.

**Cells that have supports under the exact field and none under the clamped
one**, as a share of body cells:

| horizon | 1024x320 | 2048x640 s1 | s7 | s24301 | **8192x2560** |
|---|---|---|---|---|---|
| 32 | 76.19% | 88.55% | 89.54% | 86.60% | **97.46%** |
| 48 | 66.42% | 84.06% | 85.11% | 82.14% | **96.42%** |
| **64** | 56.96% | 79.74% | 80.87% | 78.14% | **95.38%** |
| 128 | 26.18% | 64.95% | 64.58% | 65.89% | **91.32%** |
| 256 | 0.04% | 40.46% | 36.73% | 48.80% | **83.43%** |
| 1024 | 0.00% | 0.00% | 0.00% | 3.76% | **43.29%** |

**At the sketch's own suggested horizon of 64, 95.38% of the world's body
cells are stranded**, and the share rises with world size at every horizon. It
is not something a bigger constant fixes, because the two requirements pull
opposite ways: the horizon has to be **small** to bound the correction (§5.1),
and small is exactly what strands.

### 3.3 What survives of it

The failure is in *what the field measures*, not in the idea of bounding it. A
clamp of "distance to bedrock" is a ceiling on a number whose whole content is
how far bedrock is. A short-horizon field has to measure distance to something
**local that is itself known-good** — which is candidate B, and which keeps a
strict order all the way down because nothing is ever clamped.

---

## 4. Candidate B — a hierarchical potential over the coarse layer

The shape §S points at, made concrete, and the shape `worldgen-design.md` §6b
already plans for M10 (*"a cheap BFS from bedrock, once per chunk, with anchor
distance living on the coarse layer"*).

**Node** = a connected component of body material *within one chunk*, not the
chunk. **Edge** = an uncracked 4-adjacency between two components across a
chunk boundary. **Level** = BFS hops over that graph from the components
touching bedrock or the world edge. **Offset** = a weighted multi-source
search *inside* a component, from the cells that lead out of it into a
strictly-lower level. **Potential** = the pair, compared lexicographically.

That pair is a valid potential by construction, and this is the part that
matters: descending offset reaches a portal, crossing the portal drops the
level, the level cannot rise, so every descent path terminates at an anchor.
It is not the shortest path and does not try to be. **Neither level is
maintained by iterative relaxation** — both are recomputed exactly over a
bounded domain — so the count-to-infinity climb is not merely bounded here, it
has nowhere to live.

### 4.1 The coarse layer really is chunk-sized — §S's estimate confirmed

The claim needing a check was *"5,120 chunks at the shipped size"*, because a
chunk holding a cliff and a detached boulder is two nodes, and rubble could in
principle shatter the graph.

| | chunks | nodes | edges | anchored | component cells (med / p90 / max) | nodes ≤4 cells |
|---|---|---|---|---|---|---|
| 1024x320 s1 | 80 | 62 | 94 | 20 | 3,986 / 4,096 / 4,096 | 2 (3.2%) |
| 2048x640 s1 | 320 | 326 | 542 | 46 | 4,016 / 4,096 / 4,096 | 6 (1.8%) |
| 2048x640 s7 | 320 | 294 | 520 | 45 | 4,049 / 4,096 / 4,096 | 8 (2.7%) |
| 2048x640 s24301 | 320 | 468 | 558 | 46 | 1,209 / 4,096 / 4,096 | 111 (23.7%) |
| **8192x2560 s1** | **5,120** | **5,169** | **9,933** | **202** | **4,096 / 4,096 / 4,096** | **73 (1.4%)** |

**5,169 nodes against 5,120 chunks — the estimate is right within 1%**, and
the median component is a whole chunk. A BFS over that graph is genuinely
microseconds, and the "against 19.4 M cells" framing is fair.

Seed 24301 is the caveat and it is worth keeping: a world with more detached
material runs 468 nodes for 320 chunks, 23.7% of them four cells or fewer. The
node count is a property of the world's fragmentation, not of the chunk grid,
so a heavily-worked world sits above the estimate — still three orders of
magnitude under the cell count.

### 4.2 It packs into the `u16` that is already there

Measured, not assumed: **max level 39, max offset 239** at the shipped size —
6 bits and 8 bits, 14 of the 16 available. The largest offset over every world
measured is 263 (9 bits), on `2048x640 s24301`. The pair fits `Cell::aux` with
two bits spare and no cell-layout change.

### 4.3 Reachability agrees exactly, and nothing loses its support

Over four world/seed combinations, every one of these read **zero**:

- cells the exact field calls reachable and the hierarchy calls unreachable: **0**
- cells the hierarchy calls reachable and the exact field calls unreachable: **0**
- cells held under the exact field and held by *nothing* under the hierarchy
  (`held->unheld`): **0**
- and the converse (`unheld->held`): **0**

So the dangerous direction — a cell reading as detached because the model
changed under it — never fires on any world measured. That is the strongest
single result in this report, and it is the one the flat-zero control in §2
proves the instrument could have seen.

### 4.4 But it re-orients, and here is by how much

| | body cells differing | exposed cells differing | exposed cells |
|---|---|---|---|
| 1024x320 s1 | 17.69% | — | — |
| 2048x640 s1 | 25.00% | **20.67%** | 4,750 (0.47%) |
| 2048x640 s7 | 30.24% | **37.28%** | 1,537 (0.16%) |
| 2048x640 s24301 | 38.44% | **31.35%** | 8,820 (1.01%) |
| **8192x2560 s1** | **50.49%** | **21.82%** | 10,344 (0.05%) |

**Read the second column, not the first.** `load::is_structurally_interesting`
skips attached bulk with no crack and no empty neighbour, which at the shipped
size is 99.95% of the world — only **10,344 of 19.4 M cells** are ever
evaluated. So the headline 50.49% is mostly a re-orientation of rock nothing
asks about; among the cells the load model actually looks at it is **21.82%**,
and 21–37% across the four combinations.

**Four world/seed pairs is not a sweep** (`CLAUDE.md`: six seeds is not one
either), and the exposed sets are small — 1,537 cells on seed 7. Treat 21–37%
as an order of magnitude, not a value.

### 4.5 It is *not* a chunk-grid artifact, which was the thing to check

A potential that resets its offset at every chunk boundary is exactly the
shape `CLAUDE.md` warns about — *"artifacts that line up with the F1 chunk grid
are usually this, not the physics"*. Disagreement rate by distance to the
nearest chunk edge, as a rate within each band rather than a count (a count
would be won by the interior band whatever the truth is):

| | 0–1 cells from an edge | 2–7 | 8+ |
|---|---|---|---|
| 2048x640 s1 | 24.85% | 24.93% | 25.08% |
| 2048x640 s7 | 32.28% | 31.52% | 29.09% |
| 2048x640 s24301 | 42.25% | 39.09% | 37.24% |
| 8192x2560 s1 | 51.61% | 50.97% | 49.97% |

Flat, with a mild tilt toward the boundary on two of four. The disagreement is
distributed through the volume, not concentrated on the seam. The worry was
legitimate and the measurement does not support it.

### 4.6 What it buys that the existing flood does not

`load::is_supported`'s flood is capped at `MAX_REGION_CELLS` (20,000) and
**resolves to "supported" over the cap** — deliberately, because a mountain
deciding it is falling is the outcome worth being paranoid about. So a
genuinely detached piece larger than 20,000 cells reads as held, for ever. A
coarse connectivity structure answers that case exactly, in microseconds. That
is a correctness argument rather than a performance one, and it is the coarse
layer's real value-add.

### 4.7 The cost that is **not** measured here, and it is the one that decides it

A blast dirties some chunks; each dirty chunk needs its components and offsets
rebuilt (4,096 cells, one BFS), and if chunk-level connectivity changed, the
coarse BFS re-runs and every chunk whose *level* moved rebuilds too. All
bounded, none iterative — but **nobody has counted how many chunks change
body-material topology per frame during a cascade.** The arms in §5 end with
43–70 awake chunks of 5,120; if a comparable number needed rebuilding every
frame that is ~70 × 4,096 × 99 ns ≈ **28 ms**, which would be worse than the
bug. "Awake" is a much weaker condition than "topology changed" and the true
figure is presumably far smaller, but presumably is not a number. §6 names the
counter.

---

## 5. Candidate C — take away the reactive path's authority to climb: **tried, and it is a trade, not a fix**

Not in §S's sketch. It was this report's preferred answer on entry, and the
measurement went against it, which is why it is written up in full.

### 5.1 The argument, and the arithmetic that kills it

The reasoning was: `reconverge_from_damage` (the closed-set increase-aware
pass, `STRUCT_RECONVERGE=1`) is built and does not hold, and reading Phase A
says why. `distance_is_achievable` asks *"does a neighbour offer exactly the
value I store?"* — the right question **against a field that was correct
before the damage**. A region that has drifted together answers *yes*: its
cells all hold exactly a neighbour's value plus a step. So the pass is blind to
an already-wrong field by construction, and once the first cascade corrupts
the field every later pass repairs against a corrupt baseline. Forbid `tick`
to raise a distance, the reasoning went, and the pass's precondition holds by
induction.

**The blindness is real and now measured.** Arm B's `[reconv]` lines through
the whole cascade read

```
[reconv] frame 1502 seeds 1 invalidated 0 repathed 0 | 0.00ms
[reconv] frame 1505 seeds 1 invalidated 0 repathed 0 | 0.00ms
```

— seeds arriving, **nothing invalidated, nothing repathed**, for hundreds of
frames. The pass is not being under-triggered; it is looking at a consistent
field and correctly finding nothing to fix.

**But the prediction that followed from it was wrong, and the arithmetic says
why.** The prediction was that the 37,629 wrong cells are manufactured error,
so freezing the field would leave it near the 369-cell figure §S measures five
frames after the charge. Freezing it left **80,441**.

The climb is not a pathology sitting on top of the correction — **it is the
correction.** The region's true distance is ~2,400 both before the charge and
after it (§6 measures both), and its *stored* value is what collapsed, to
near zero, from a false anchor at the crater. Getting back is an **increase**,
and Bellman-Ford raises a distance by one unit per relaxation round: ~2,400
rounds at `STRUCTURAL_TICK_INTERVAL` = 5 frames is **twelve thousand frames**,
which is precisely §S's *"eleven thousand frames after the only event and
still climbing"* and precisely its `max aux` rising ~85 per 600 frames.
Suppressing the rise does not spare the field that work; it abandons it
part-way.

**That is the strongest argument in this report for bounding the range**, and
it is one §S does not make: the cost of recovering from a false zero is one
round per unit of the field's range, whatever caused the zero. A field whose
range is 2,577 owes up to 2,577 rounds; one whose range is 239 — the
hierarchical potential's measured maximum — cannot owe more than 239, and the
same accident costs ten times less.

### 5.2 The measurement

`STRUCT_NO_CLIMB=1` — documented as a probe at its gate — suppresses the write
and the fan-out when a relaxation comes back worse, and changes nothing else.
Four arms, `scale_probe size=8192x2560 phases=1 warm=1500 frames=1600
load=blast:200:1`, `preset rolling seed 1`, one radius-20 charge at frame 200,
oracle at frame 1,500 (+1,300 frames after the only event):

| arm | | wrong vs oracle | `produced` @3,000 | `pending` @3,000 | sites drained |
|---|---|---|---|---|---|
| **A** | baseline | **37,629** | 7,491 | 36,818 | 2,000 (cap) |
| **B** | `STRUCT_RECONVERGE=1` | **33,469** | 4,281 | 31,645 | 2,000 (cap) |
| **C** | `STRUCT_NO_CLIMB=1` | **80,441** | 2,568 | 18,217 | 2,000 (cap) |
| **D** | both | **51,372** | 2,128 | 16,815 | 2,000 (cap) |

Wall clock, as corroboration only (`CLAUDE.md`: gate on counters, and this box
is shared): frames over the 16.6 ms budget were 86.9 / 87.6 / 86.1 / **74.2%**
for A / B / C / D.

**In every arm, every changed cell rose** — the stored field is stale-low in
all four, never stale-high.

### 5.3 Reading it

- **C and D do what they were built to do.** The fan-out is the cost, and
  `schedule_solid_neighbours` fires on `moved`, so a suppressed rise produces
  zero sites where it produced five. `produced` falls from 7,491 to 2,128, the
  heap from 36,818 to 16,815, and D is the only arm that takes a fifth of the
  frames back under budget.
- **And they make the field worse, which is the point.** C alone doubles the
  wrong-cell count (80,441 against 37,629) because it freezes a region
  part-way through a recovery it genuinely owed. Stale-low is the **load-sink** direction
  (§1.2), so this is not a harmless conservatism — it is more of exactly the
  error that §S's converged-pass control shows destroys rock.
- **B on its own is nearly inert**, as its own counters say.

So candidate C buys queue with field accuracy, at a rate this report cannot
recommend. It is worth recording rather than discarding, because *the queue
half works*: whatever ships, **a rise should not fan out to five sites**, and
that is separable from whether the rise is written.

### 5.4 Filed

`dead-ends.md`, structural: **suppressing `tick`'s upward writes** — fires
(`worsened` 1,400/frame → 0–72), cuts `produced` 7,491 → 2,568 and `pending`
36,818 → 18,217, and **doubles the field's disagreement with a converged
oracle, 37,629 → 80,441**, all in the stale-low direction. Re-test only if the
field's range is bounded first, which removes the thing it was suppressing.

---

## 6. Found: a thrown rock lands claiming to be bedrock

The hunt this report did not set out to run, and it ends §S rather than
informing it.

### 6.1 The symptom

§5.1 says the stored region is ~2,200 *below* the truth and climbing at one
unit per round. That is how the error is *corrected*; this is where it comes
from. Something writes a **zero** at the crater, the neighbourhood relaxes
from it — downhill, the cheap direction, so one improvement wave drags tens of
thousands of cells down inside a few hundred frames — and every one of them
then owes the full climb back.

`ORACLE_COLUMN=1` on `scale_probe`, ten frames after the charge
(`RECONVERGE_AT=210`, charge at world frame 1,700, blast at x=3904):

```
[oracle] wrong-cell bbox x 3873..3936 (64 wide) y 163..203 (41 tall)  -- 595 cells
(3898,173) stone fg stored 0 -> true 2398 | nbrs rubble soil rubble stone
(3906,168) stone fg stored 1 -> true 65535
```

At +50 frames the same column shows the consequence — a smooth ramp rising
away from that point and a one-cell cliff where it meets untouched rock:

```
y=  209 stone bg    133 ->   2374
y=  210 stone bg   2373 ->   2373
```

By +1,300 frames it reads 219–252 against a truth of 2,368–2,428: **the same
shape, offset by a near-constant ~2,160**, which is the region climbing back
in lockstep and is why no local test can see it.

### 6.2 The trap, and what it caught

Two ablations had already ruled out the obvious writers by name (§6.4), which
is exactly the situation a **write-seam trap** is for — `World::set`'s own doc
records the principle: *"an enumeration that has to stay complete is the
failure mode this project keeps rediscovering."*

`AUX_TRAP=<frame>` reports any write that makes a cell body material reading
`aux <= 2` where nothing adjacent is bedrock and the cell it replaced was
nowhere near an anchor — a **false anchor** — with a backtrace, capped at 12.

**Twelve reports, on the two frames after the charge, one caller:**

```
[auxtrap] frame 1701 (3901,168) empty aux 0 -> stone aux 0 | nbrs stone:2405 empty:0 empty:0 stone:2405
   1: pixel_physics::sim::world::World::set
   2: pixel_physics::sim::particle::ParticleSystem::step
[auxtrap] frame 1702 (3908,168) empty aux 0 -> stone aux 0 | nbrs stone:0 empty:0 gravel:0 stone:0
```

All twelve are `ParticleSystem::step`, and the spread is visible in the
neighbour column: the first reports sit beside `stone:2405`, the last beside
`stone:0`.

### 6.3 The line

`particle.rs::landed_cell`:

```rust
let cell = Cell::new(particle.material, particle.shade);   // aux = 0
if world.materials.get(particle.material).worth_in_aux {
    cell.with_aux(particle.aux)
} else {
    cell                                                    // stone lands at 0
}
```

`worth_in_aux` is the **food-value** flag. Stone does not carry it, so every
thrown rock lands claiming `aux == 0` — *adjacent to bedrock*.

The function's doc is careful and is right about what it enumerates: a free
particle must not carry a `Liquid`'s fill or a `Powder`'s moisture, and *"on
`Solid`/`Plant` it is the organism packing"*. On an **inert** `Solid` there is
a third convention — the anchor distance — and there `Cell::new`'s 0 does not
decline to make a claim, it makes the strongest one available. This is
`CLAUDE.md`'s *two conventions for `Cell::aux` point opposite ways* with a
third, and §Z2's corpse fix is what put the `worth_in_aux` gate in front of it.

**It explains §S completely**, including the parts §S could not attribute:

- **Only verbs that throw material leak.** The brush throws no particles,
  which is exactly §S's verb table — §S credited that to `relax_region` and
  this is a simpler second reason.
- **It scales with crack reach**, because more cracks make more fragments make
  more particles. That is §S's unexplained hammer-vs-pick ordering (the hammer
  removes *fewer* cells than the pick and costs *more*).
- **It is at the surface**, where debris lands, and an idle world has none.
- No organism, no ground root and no landed rigid body is required.

### 6.4 The fix, measured

`PARTICLE_AUX_MAX=1` lands a body-material particle at `u16::MAX` — "no known
path, earn one" — instead of 0. That is both the honest value and the cheap
one: an improvement is a **single** relaxation round whatever its size, while
climbing out of a false 0 costs one round per unit of a 2,577-deep field.

Same scene as §5.2, oracle at +1,300 frames:

| | baseline | `PARTICLE_AUX_MAX=1` |
|---|---|---|
| wrong cells vs the converged oracle | 37,629 | **186** |
| of those, the climb (`\|delta\|` 1k–60k) | 37,593 | **0** |
| `produced` per frame @3,000 | 7,491 | **13** |
| `pending` @3,000 | 36,818 | **5,957** (idle ~5,400) |
| structural sites drained @3,000 | 2,000 (the cap) | **10** |
| scheduler phase @3,000 | 14.68 ms | **0.03 ms** |
| frames over the 16.6 ms budget | 86.9% | **72.8%** |
| body cells standing at end | 19,409,002 | **19,410,719** |

That is essentially the state §S's whole-world `compute_world_distances`
oracle reached (12.49 → 0.25 ms, pending 53,077 → 6,094), reached with **no
converged pass at all**.

**Four controls, because this bug has faked a quiet queue before**
(`CLAUDE.md`: *a cost that vanishes may be work that vanished*):

- **The trap goes to 0 reports** with the fix in, so the ablation hit its
  target rather than missing it.
- **`max aux` reads 2,422** — a live, honest field. The rooted-flat prototype
  §S withdrew read 142.
- **More rock survives** (+1,717), the same direction as §S's converged-pass
  control (+25,470). The queue is not quiet because the world emptied.
- **The residual 186 are all `>60k (detached)`** — cells the reactive path
  calls reachable and the converged pass calls `u16::MAX`. The climb bucket is
  **empty**. That residual is a different and much smaller question.

### 6.5 What was ruled out on the way

| candidate | how | result |
|---|---|---|
| `rigid::settle` — landed body cells go in through `Cell::new`, `aux` **0** | `SETTLE_AUX_MAX=1` writes `u16::MAX` | **fires** (arms diverge from +100 frames) and the oracle reads **37,036 against 37,629** — 1.6% |
| `structural::tick`'s `grounded_root` | `STRUCT_NO_GROUND_ROOT=1`, with a new `grounded` counter as the control | **byte-identical** to baseline, and the counter reads **0 every frame**. A vacuous null, and the counter is the only reason that is knowable |
| the powder-`aux` collision | reading | `is_body_material` is `Solid \| Plant`; `rubble`, `gravel` and `soil` are all `Powder`, so the relaxation never reads their moisture |
| organism-owned neighbours | reading | real gap — `tick`'s neighbour loop filters on `is_body_material` and `is_burning` but **not** `organism_id != 0`, unlike `compute_world_distances`, `support_parent`, `support_count` and `dependants`, so an inert cell beside live tissue relaxes off a cell-type tag. Not what fired here (the trap named `landed_cell` twelve times out of twelve), and worth closing anyway |

### 6.6 Two adjacent gaps this turned up

- **`land()` schedules nothing.** It writes through `World::set` and raises no
  `StructuralCheck`, so a landed body cell is invisible to the structural
  scheduler until something else wakes it. Under `aux 0` that meant it read as
  anchored indefinitely; under `u16::MAX` it reads as pathless indefinitely.
  Either way it should be scheduled.
- **`tick`'s neighbour loop does not exclude organism-owned cells**, as above.

## 7. The cheapest experiment that would falsify each

One per candidate, smallest first. None needs a new harness.

| what | the experiment | what falsifies it |
|---|---|---|
| **A — saturating clamp** | none needed; §3.2 *is* the falsification | — |
| **C — no climb** | §5.2, already run | already falsified as a fix |
| **the false anchor (§6)** | done — `AUX_TRAP` at the `World::set` seam, and `PARTICLE_AUX_MAX=1` as the ablation | already answered: 12/12 backtraces name `ParticleSystem::step`, and the fix takes 37,629 wrong cells to 186 |
| **B — hierarchy, cost** | a per-frame counter of **distinct chunks whose body-material topology changed**, printed beside `SCHED_PASS`'s `[struct]` line, on the same `load=blast:200:1` scene | a cascade dirtying tens of chunks a frame: at ~0.4 ms per chunk rebuild that is worse than the bug it replaces (§4.7) |
| **B — hierarchy, behaviour** | `examples/anchor_probe` with a third arm whose field is the hierarchical potential | the **margin** moving |

**The false anchor was the cheapest and it was the whole thing** — one probe,
no model built, and it turned §S from an architecture question into a
one-line write. What is left for candidate B is B's own cost counter: one
line, and the only thing that can kill it outright. §4.4's orientation figures
cannot, because they say a fifth of evaluated cells change support, not that
any *verdict* changes, and a verdict is what a player feels.

**Then `anchor_probe` for the behaviour**, and `anchor_probe` specifically
rather than a whole-world census, because the instruments index says to reach
for its shape for exactly this — *"two writers of the same cached field, a
fast path against its slow reference"* — and because it sweeps for a **margin**
rather than an outcome. Past its margin every rule agrees a structure falls;
short of it every rule agrees it stands; a rule can only show itself in where
the margin is, which is also the quantity a player feels: how far you can build
before it comes down. Its own first run put the scene where the margin could
not reach it and produced a null that said nothing about anything.

**Two checks before believing any result here**, both from `CLAUDE.md` and both
already paid for once by this bug:

- **Pair the queue with `max aux`.** A queue that goes quiet because the system
  stopped asking is indistinguishable in every timing from one that converged;
  §S's 8x-box prototype read as a complete fix and had rooted the blast
  neighbourhood flat. An honest `max aux` is the tell.
- **Census the body cells at the end of the run.** §S's converged pass leaves
  25,470 *more* cells standing, so any change here is a destruction-quantity
  change as well as a cost one, and `scripts/blastsweep.sh` at the order
  statistic is what sizes it.

---

## 8. Still unmeasured

- **The harm channel.** §1.2 argues the stale-low region acts as a load sink
  and that the manufactured failures are therefore **Overloaded** rather than
  **Unsupported**. The measurement is `FailureCounts` split by `FailureMode`,
  with and without a converged field. If converging removes overload failures
  specifically, the reading is right and a replacement's job is to get the four
  bits right rather than the distances. Nothing in `scale_probe` prints that
  split today.
- **The residual 186.** With the false anchor fixed, what is left is cells the
  reactive path calls reachable and the converged pass calls `u16::MAX` —
  genuinely detached fragments reading as attached. Two orders of magnitude
  smaller than the climb, and a different bug.
- **Whether `PARTICLE_AUX_MAX` is safe to make the default.** It is a
  behaviour change: a landed grain reads pathless until `tick` relaxes it, and
  `land()` schedules nothing (§6.6). `acceptance.sh`, `cargo test --lib` and
  `blastsweep.sh` at the order statistic are the gates.
- **A real seed sweep** of §4.4's exposed-cell disagreement.
- **Fire**, which §S notes is unmeasured and the same shape, and for which
  there is still no `load=fire` component.

## 9. Recommendation

**Do not replace the structural model for §S.** §S is a one-line bug in
`particle.rs::landed_cell` (§6), and the measured fix reaches the same state
as §S's own whole-world converged-pass oracle with no converged pass at all:
wrong cells 37,629 → **186**, scheduler 14.68 → **0.03 ms**, queue back at its
idle value, and *more* rock standing. Land that, with its gates, before
anything else here is considered.

**What survives of the architecture argument, and it is real but no longer
urgent.** Recovering a wrong distance *upward* costs one relaxation round per
unit, so the field's depth is the price of any accident in it: 2,577 deep
means an accident can cost 2,577 rounds. This one cost eleven thousand frames
for that reason. Candidate B bounds it to 239 by construction and is what
`worldgen-design.md` §6b already plans for M10 — so it is a roadmap item that
also happens to cap the blast radius of the *next* mistake of this shape, not
a rescue.

**Candidate A is closed**; §3.2's table is in `dead-ends.md`. **Candidate C is
closed**; §5 is in `dead-ends.md`, and it is worth reading for what it got
wrong — it suppressed a *correction* on the belief that it was manufactured
error, which the false anchor explains.

**Two adjacent gaps worth closing whatever happens** (§6.6): `land()` raises
no structural check, and `tick`'s neighbour loop is the one reader of `aux`
that does not exclude organism-owned cells.

**And one that is separable from all of it**: a rise still fans out to five
sites, because `schedule_solid_neighbours` fires on `moved` rather than on
direction. §5.2 shows that alone is `produced` 7,491 → 2,568.

**Finally, the behaviour question does not go away.** §S flags that part of
the delay a player sees after a blast *is* this crawl, and
`CHAIN_WINDOW_FRAMES` is 600 frames of deliberate generosity built around it.
Removing the crawl could make collapse arrive nearly instantly and read worse
even as the frame cost falls off a cliff. That is a blind A/B with the failure
counts in the card's `meta`, not a commit message.
