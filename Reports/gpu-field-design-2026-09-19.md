# The GPU field — what it would take, and the number that would make it worth taking

*Lane P of the ant-survey follow-up round, item 4 of its brief:
**a design note, nothing built.** It prices §8.5 step 4 of
`ant-sim-research-review-2026-09-19.md`,
which made the GPU field conditional on a share measured after step 2. Step 2 is
built and measured in
[`ant-field-wake-2026-09-19.md`](ant-field-wake-2026-09-19.md); this says what
the condition now reads, and what the work would actually be.*

## 0. The answer

**The condition is not met, by a factor of about 1.6, and step 2 did not move
it.** §8.5 step 4 asked for the field above roughly a third of the tick at play
population after step 2. On the shipped lab bed at 52 ants, with the per-phase
stopwatch that step 1 built, **the field is 20–21% of the tick** and the switch
step 2 produced moves it by under one percent. The phase above it is
`ca_sweep` at 56–60%.

**And the share is the wrong number to have been waiting for anyway.** It was
chosen as a proxy for "is this pass big enough to be worth a different
device", which is right, but it says nothing about whether the pass *can* move.
Two harder facts do, and neither depends on the share:

- **the readback is per tick and is not optional** — the ants, the plants, the
  fire and the renderer all read the field on the CPU, in the same tick it is
  solved, so a GPU solve buys a round trip every frame at a granularity that
  defeats the usual way of paying for one;
- **the sleeping-tile economy is the field's actual optimisation, and it is the
  one a per-pixel device cannot express.** The field's cost here is already
  proportional to *what is awake*, not to world area. A compute shader over the
  whole grid would replace a pass that solves 65 of 128 tiles with one that
  solves 128, and would have to beat the CPU by that ratio before it broke even
  on a lab bed — before the readback.

So the honest recommendation is **not "later"** but **"the share was never going
to be the deciding number"**: if the field is ever to move off the CPU, the case
has to be made on a world where the awake fraction is high and the readback can
be deferred, and M10 streaming is the only thing on the roadmap that changes
either.

## 1. What `PLAN.md`'s decision row currently rests on, and why it is not enough

> | Simulation device | **CPU** (GPU renders only) | Shaders have no `rand()`, so identical cells clump into symmetry artifacts; falling sand is order-dependent, which parallel-per-pixel can't express. Noita's author made the same call. |

Both clauses are about the **CA grid** and neither is about the field. The field
is not order-dependent — it is a double-buffered Jacobi relaxation, which is the
textbook shape for a per-pixel device (`dead-ends.md:1119` records why it is
*not* in place, and that decision is what makes it GPU-shaped). And it needs no
`rand()` at all. **So the standing decision does not actually cover this
proposal**, and a session reading that row as a refusal would be reading it for
a different question.

What does cover it is three other standing facts, and they are the ones a
proposal has to answer:

- **`PLAN.md` requires same-build determinism**, per
  `emergent-world-architecture.md` §8. That is a hard constraint on a GPU field
  and §3 is about it.
- **The renderer is `pixels` 0.17**, which is presentation-only here.
  `dead-ends.md` has two separate entries recording that its architecture
  re-uploads the whole texture and exposes no surface accessor — the *reason*
  four earlier GPU proposals in this tree died was never the arithmetic, it was
  that there is no compute path at all. A GPU field means adding `wgpu` as a
  compute dependency alongside `pixels`, or replacing `pixels`. That is the
  first line item and it is not small.
- **Frame cost is a hard constraint, not a tiebreaker** (`CLAUDE.md`), and
  `examples/ascii.rs`'s worst-frame figure is the number that gates it.

## 2. What it would need, item by item

### 2.1 The readback, which is the structural problem

The field is read on the CPU, by name, every tick:

| reader | what it reads |
|---|---|
| `creature.rs` | `LightHere`, the temperature sense, the worm's thermotaxis — brain inputs, so they gate a decision this tick |
| `plant.rs` | light and moisture for the growth economy, per active site |
| `fire.rs` | `ground_wetness_at` for contact ignition, `diffuse_heat` writes back |
| `weather.rs`, `explosion.rs` | `add_heat` / `add_pressure_impulse` write in |
| `render.rs` | `FieldOverlay`, and the light that dims every cell drawn |

That list is the problem, not the solve. A GPU pass that finishes and is read
next frame would be a one-frame lag on every one of those, and two of them
(`fire::try_ignite`, the brain) are decisions rather than displays. A GPU pass
that is read *this* frame is a pipeline barrier plus a buffer map in the middle
of the tick — the shape that costs more than the arithmetic it replaced,
routinely, and `CLAUDE.md` has the general form of that trap already
(*removing work is not the same as removing cost*: a gate that removed 91% of
the field's momentum arithmetic made the frame **slower**, because the memory
traffic only moved).

The only version that avoids the barrier is **the GPU owning the field
outright** — every reader moved to a shader, including the brain. That is not a
field optimisation, it is a different engine, and it would take the creature
decisions with it into a device where `PLAN.md`'s determinism requirement is
hardest.

**So price the readback first and the solve second.** A `wgpu` buffer map of six
`f32` per field cell at the shipped 8192x2560 world is 512x160 field cells x 24
bytes ≈ **2 MB a tick**, which is bandwidth-trivial and latency-hostile: it is
the synchronisation, not the volume. The measurement that would settle it needs
no field code at all — a `wgpu` compute dispatch of the right shape over a
buffer of the right size, mapped and read every frame, timed against nothing.
**That is the experiment to run before any of the rest, and it is a day's work
that can kill the whole idea.**

### 2.2 The sleeping-tile economy, which a per-pixel device cannot express

The field's cost is already proportional to activity:

- the whole pass early-outs when no chunk is awake and nothing has drifted;
- the solve set is the awake tiles plus one ring, not the world — measured, one
  radius-4 impulse held in a corner cost 2.5 ms at 512x320 and **53 ms at
  2048x1280** before that subset existed, identical to disturbing the whole
  world;
- `rebuild_blocked` rescans only the blocks written since the last solve
  (`Chunk::stale_blocks`);
- the three momentum passes skip entirely when every readable tile is at zero;
- `apply_sky` reads attenuation through the old map for tiles outside the
  subset.

Every one of those is a *scatter* — an irregular, data-dependent subset — and
every one of them is what makes the field cost 0.117 ms on a lab bed instead of
whole-world work. A compute shader's natural shape is the opposite: dispatch
over everything, uniformly, because the divergence and the indirection of a
compacted work list often cost more than the cells they skip.

**So the break-even is not "GPU vs CPU per cell", it is "GPU over 128 tiles vs
CPU over 65".** On the lab bed that is a 2x handicap before the readback. On the
shipped outdoor world at 8192x2560 the awake fraction is far lower still — ~1,500
tiles solved of a much larger grid — so the handicap is *worse* where the world
is bigger, which is the opposite of the way this usually goes. An indirect
dispatch over a compacted tile list would recover it and is the only design
worth writing down; it also reintroduces a CPU-side compaction pass and a
second readback (the list), and it is where a real proposal would have to start.

### 2.3 Determinism across drivers

`PLAN.md` requires same-build reproducibility, and `tests/determinism.rs` gates
it. A GPU field has to hold that across:

- **`fma` contraction.** A shader compiler may or may not fuse `a*b+c`, and the
  two differ in the last bit. The diffusion and advection stencils are full of
  that shape. Same build, different driver, different result — and "same build"
  is exactly the bar, so a driver update is a broken gate.
- **Reduction order.** `all_settled` is a reduction over tiles and the momentum
  skip is a reduction over `momentum_zero`. A tree reduction's order is
  implementation-defined; `field_hash` and `field_channels` would both move.
- **Denormals and `fast-math`.** The settle epsilons are 0.001–0.02 and the
  channels decay toward zero, so denormal handling is directly on the
  convergence path — a driver that flushes to zero settles a tile a frame
  earlier than one that does not.

None of that is fatal and all of it is work. The shape that survives is a
shader written against a fixed arithmetic contract (no `fma`, explicit ordering,
denormals handled) plus `field_hash` run in CI on the *reference CPU path* and
compared to the GPU path on the developer's machine — which means keeping the
CPU field forever as the oracle. **That is a reasonable design and it doubles
the surface**, and it should be said out loud rather than discovered: a GPU field
does not replace `field.rs`, it sits beside it.

### 2.4 What the survey actually recommended, and the narrower thing that survives

The survey's GPU recommendation is about **pheromone planes**, which is where
the compute-shader hobbyist sims put them, and the review's §2.13 answered it on
the planes' settled cost. §8.3 then found the planes at 0.144 ms with 129 ants —
**twice the field** — and that is the pass with the better shape for a device:
the planes are a plain decay-and-diffuse over a dense `u16` grid with no derived
arrays, no sleeping-tile economy worth defending, and — critically — **the ant
reads them through a small number of point samples**, not the whole plane. A
readback of the samples the ants actually take is a different order of problem
from a readback of the field.

That is not this note's subject and it is not built either. It is recorded
because if anything in this engine goes to a compute shader first, the evidence
says it is the planes and not the field, and the review's own §8.3 table is what
says so.

## 3. The number that would change the decision

Three, and they are in dependency order. None of them is the share.

1. **The readback's floor, measured with no field code** (§2.1): a `wgpu`
   compute dispatch over a 512x160x6 `f32` buffer, mapped and read every frame,
   on the owner's machine. If that alone costs more than the field's whole
   current budget on his bed, the idea is dead and the measurement is a day.
2. **The awake fraction at play population**, from the stopwatch step 1 built:
   `awake_chunks` and the `field` share in the same chronicle row, on his own
   colony of thousands. A GPU field needs that fraction *high*. If it is 65 of
   128 on a lab bed and lower outdoors, an indirect dispatch is mandatory rather
   than an optimisation.
3. **The field's share after `ca_sweep` is addressed.** The field is 20% and the
   sweep is 56–60%. Even a *free* field is a 20% win; the same effort spent on
   the pass that is three times larger is worth more, and the parallel-creature
   switch the owner is testing on his own machine is upstream of that number.
   **The field becomes the right target when it is the largest phase, and today
   it is the third.**

`PLAN.md`'s *Simulation device* row should stay as it is. What it needs is not a
change of verdict but a note that its two reasons are about the CA grid, so the
next session proposing a GPU field is arguing with §2 above rather than with a
row that does not cover it.
