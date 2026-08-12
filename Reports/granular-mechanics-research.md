# Granular mechanics research: what a pile of sand actually does, and which of it this engine can express

**Report A of four.** Scope: the physics of dry granular material at rest and in
motion, and how much of it a fixed-lattice CA can carry. Serves M3's unfinished
half (BTW toppling, hole-propagation flow — both named in the README's "not yet
built" list), M9's burial/digging, and supplies the yield-criterion vocabulary
Report B (liquid, revision 2) and Report C (solid–granular–fluid coupling) both
need.

Out of scope, deliberately: liquid behaviour (Report B), rigid-body interaction
(Report C), and anything requiring per-cell velocity (see §7).

---

## 0. Summary, stated first

Four findings, in descending order of how much they change the code:

1. **Repose is two angles, not one.** Every real granular pile has a *maximum
   angle of stability* θ_ms at which it starts to avalanche, and a lower *angle
   of repose* θ_r at which it stops. The gap is real, measurable, and around 8°
   for glass beads. This engine models one angle. Modelling two is a **single
   spare flag bit** in `Cell`, costs one extra comparison in a branch already
   being taken, and is the source of essentially every granular behaviour the
   engine currently can't produce: avalanches as discrete events, piles that
   hold a steeper slope than they settle at, and — as a free side effect — a
   fix for the systematic 5–6° under-angle in the README's known limitations.

2. **BTW toppling should not be built.** The plan lists it as pending work. It
   is a model of self-organized criticality, not of sand, and the experimental
   record is clear that real 2D sandpiles do not behave the way it predicts.
   Building it would produce *worse* granular behaviour than what is already
   there, at real cost. This report recommends deleting it from the plan.

3. **Hole/void propagation is the right idea with a known failure mode.** The
   void model is the natural CA formulation of granular drainage and is the
   correct dual of the current particle-moves rule. But it is experimentally
   known to over-predict mixing by a wide margin. The fix — correlated
   multi-cell motion rather than single-cell void random walks — is cheap and
   is the difference between a drained layered pile keeping its layers and
   turning to mush.

4. **Dilatancy is the missing mechanism behind M9.** Dense granular material
   must expand to flow. Nothing in the engine expresses that, which is why
   there is no principled model for burial, digging resistance, or why a packed
   pile resists a footstep differently from a freshly poured one. `Cell::aux`
   is documented as unused for `Powder`; it is the natural home, and this is
   the same move the liquid rewrite already made for fill fraction.

Plus one thing that should be measured before anything else: the engine has no
quantitative granular acceptance test at all. §8 supplies one with published
numbers.

---

## 1. The current model, in the engine's own terms

`update_powder` tries, in order: straight down, then diagonally down (random
left/right first), then `roll_along_slope`. `roll_along_slope` looks up to
`roll_reach_at(x, y)` cells each way for a downhill opportunity and creeps one
cell toward the nearer one.

`Material::roll_reach_at` derives that reach from `friction_angle` as
`reach = 1 / tan(angle)`, floored to an integer, with the fractional part spent
by giving *some positions* the longer reach via `rng::jitter(x, y)`. The
position-keying is correct and load-bearing — the plan's standing invariant 2
exists because of it, and nothing below proposes changing it.

What this model gets right, and should be preserved:

- A pile comes to rest at a slope set by material data, not by the lattice's own
  45° diagonal. That is the whole reason `roll_along_slope` exists and it works.
- Reach is bounded and declared through `Material::sweep_reach`, so the dirty
  rectangle widening and `parallel.rs`'s write-disjointness proof both stay
  valid.
- Irregular surfaces fall out of the fractional-reach jitter rather than being
  authored.

What it structurally cannot express, all traceable to one property — **a grain's
behaviour depends only on its position and its material, never on its own
state**:

- **No hysteresis.** A grain's decision is identical whether the pile was just
  disturbed or has sat still for a thousand frames. Real piles carry memory of
  how they were built.
- **No avalanches as events.** Material creeps continuously at one rate. There
  is no mechanism by which a slope holds, holds, holds, and then releases.
- **No packing state.** A freshly poured pile and a settled, compacted one are
  the same material in the same configuration.
- **No stress.** Nothing in the engine knows that the bottom of a deep pile is
  loaded differently from the top, or that a pile against a wall sheds load into
  it.

The README's own known limitation — "repose angles come out a few degrees
shallower than requested, roughly 39/30/18 against 45/34/22" — is attributed
there to integer reach quantization. That is a real contributor, but §2 shows it
is not the whole story: a single-angle model measured against a two-angle
reality will read low regardless of quantization, because the angle it produces
is the *stopping* angle and the number authors think they are setting is usually
the *starting* one.

---

## 2. Finding 1: repose is two angles

This is the central result of the report.

A granular slope has two distinct critical angles. The **maximum angle of
stability** (θ_ms, also called the maximum angle of stability or marginal
stability) is the steepest slope a pile can hold before it fails. The **angle of
repose** (θ_r) is the shallower angle it comes to rest at after the resulting
avalanche. Lee & Herrmann's molecular-dynamics study established the distinction
computationally: <cite index="102-1">piles generated by avalanches have a finite
angle of repose θ_R, and are stable under tilting by a non-zero angle, showing
that θ_R is different from the angle of marginal stability θ_MS, the maximum
angle of stable piles</cite>. The review literature is explicit that these are
routinely conflated: <cite index="104-1">the angle of marginal stability is the
maximum stable angle of a heap but is not necessarily the angle of repose</cite>.

The gap is not marginal. In rotating-drum experiments the difference δ between
the two angles is <cite index="106-1">approximately 8° in the experiment of
Metcalfe et al.</cite>, and the avalanche cycle is exactly this gap: <cite index="106-1">grains fall when the free surface reaches the angle of marginal
stability, the avalanche stops when the slope approaches the repose angle, and
the drum must turn by δ again to launch the next avalanche</cite>. Under reduced
gravity the two angles even move in *opposite* directions — <cite index="105-1">the static angle of repose increases about 5° with reduced
gravity, whereas the dynamic angle decreases by about 10°, so avalanche size
increases</cite> — which is as clean a demonstration as one could want that they
are independent physical quantities and not two measurements of one.

The consequence, in granular-physics terms: <cite index="103-1">this results in
hysteresis, since the sandpile carries forward a memory of its initial
conditions; bistability at the angle of repose is another consequence, since the
manner in which the sandpile was formed determines whether avalanche motion
will or will not occur at a given angle</cite>.

### What this costs to implement here

Almost nothing, which is the surprising part.

`Cell::flags` is a `u8` with exactly two bits used (`FLAG_MOVED`, `FLAG_BURNING`).
Six are free. Add `FLAG_FLOWING`.

- A grain **at rest** (`FLAG_FLOWING` clear) evaluates `roll_along_slope` against
  the reach derived from **θ_ms** — the steeper, harder-to-trigger threshold. It
  mostly doesn't move.
- A grain **already flowing** (`FLAG_FLOWING` set) evaluates against the reach
  derived from **θ_r** — the shallower, easier one. It keeps going further than
  it could have started.
- A grain that successfully moves sets the flag. A grain that finds nowhere to
  go clears it.

That is velocity-weakening friction expressed as one bit, and it produces
hysteresis, bistability, and discrete avalanches directly, because a slope
between the two angles is now genuinely bistable: static if nothing is moving
through it, mobile if something is.

Two hazards, both already solved elsewhere in the codebase:

- **Sleeping.** A flowing grain writes itself every frame it flows, so its chunk
  stays awake — the same argument `fire.rs` already makes for why
  neighbour-driven ignition can be re-rolled per frame while `roll_reach_at`
  cannot. A grain that stops clears the flag, which is also a write. No new
  wake-up machinery.
- **Reach budget.** θ_r < θ_ms means `1/tan(θ_r) > 1/tan(θ_ms)`, so the
  *flowing* reach is the larger one and is what `Material::sweep_reach` must
  report. Strictly a widening relative to today for the same authored angle,
  which is safe (issue #3's own note that `sweep_region` only ever shrinks
  relative to `MAX_REACH` still holds), but it must be the number `sweep_reach`
  returns, not the resting one.

### Content model

`friction_angle` becomes two fields. Suggested: keep `friction_angle` meaning
θ_r (so existing `.ron` files keep working and keep meaning what they meant),
and add an optional `max_stability_angle` defaulting to `friction_angle + 8.0`
— the measured drum figure, used as a default rather than an authored one, in
the same spirit as `default_friction_angle`'s 45.

This also directly addresses the README's under-angle limitation: piles will now
*hold* at θ_ms and *settle* at θ_r, so the visible resting slope of an undisturbed
pile moves up toward the authored number instead of sitting several degrees
under it.

---

## 3. Finding 2: do not build BTW toppling

`PLAN.md` and the README both list Bak–Tang–Wiesenfeld toppling as pending work
for avalanches. It should be removed from the plan, and this is a substantive
recommendation rather than a scoping preference.

The BTW sandpile was constructed to demonstrate self-organized criticality — the
claim that a driven dissipative system evolves to a critical state with
scale-free avalanches. Its defining prediction is a **power-law avalanche size
distribution**. Real granular piles do not produce one.

The experimental record:

- Jaeger, Liu & Nagel's rotating-drum work found a sharply peaked avalanche size
  distribution, not a power law, and <cite index="112-1">argued that these
  experiments indicate avalanches in a sandpile do not behave in a
  self-organized critical manner</cite>.
- The Oslo rice-pile experiment is the closest thing to a confirmation, and it
  confirms the opposite of universality: <cite index="118-1">the dynamics exhibit
  self-organized critical behaviour in one case (grains with a large aspect
  ratio) but not in another (less elongated grains) — showing SOC is not as
  'universal' and insensitive to system details as initially supposed, but that
  its occurrence depends on the detailed mechanism of energy dissipation</cite>.
  In the follow-up analysis, <cite index="119-1">only with sufficiently
  elongated grains did power-law avalanches occur; for more symmetric grains a
  stretched exponential distribution was seen</cite>.
- The mechanisms responsible for the failure are named directly, and both are
  physical things this engine would want anyway: <cite index="115-1">experiments
  have not in general shown evidence of criticality in sandpiles due to the
  effects of inertia and dilatation (moving grains require more space), except
  for small avalanches or with elongated rice grains where these effects are
  minimized</cite>.
- More recent drum work across five bead types found <cite index="109-1">two
  clearly definable angles θ_r and θ_m, with Gaussian-like avalanche size
  distributions — contrasting with the power law predicted by self-organized
  criticality, and reminiscent instead of a first-order phase transition, with
  inertia and velocity-weakening enhancing the first-order features</cite>.

Read together: the thing BTW exists to produce is the thing real sand doesn't do,
and the reason it doesn't is dilatancy and velocity-weakening — which are
precisely §2's and §5's recommendations. **The two-angle model in §2 is not a
cheaper substitute for BTW toppling; it is a more accurate one.** Velocity
weakening is explicitly named above as a source of the first-order (sharp,
characteristic-size) avalanche behaviour that experiments actually see, and §2
implements velocity weakening.

Worth keeping from the SOC literature: the *stability condition* formulation is
clean and lattice-native — <cite index="107-1">on a discrete square lattice the
stability condition with respect to the angle of repose reads
|h_x − h_{x+1}| < tan(θ_r), and an avalanche occurs when it is violated</cite>.
That is a useful way to state an invariant and to write a test. It is not a
reason to adopt the dynamics that usually come attached to it.

---

## 4. Finding 3: force chains and the Janssen effect

Granular material at rest does not transmit stress like a fluid. Load travels
through a sparse network of **force chains**, and where those chains meet a wall,
the wall carries part of the weight.

The measurable consequence is the **Janssen effect**: vertical stress in a
confined column saturates with depth rather than growing linearly. Janssen's
own derivation gives σ_zz(z) = ρgl(1 − e^(−z/l)), where <cite index="128-1">a
fraction κ of the vertical stress converts to horizontal stress and all wall
friction is at the Coulomb failure criterion</cite>. Physically, <cite index="126-1">frictional interactions between grains and the vessel walls
partially support the weight of the column, decreasing its apparent mass</cite>.
It is also why an hourglass keeps time: <cite index="123-1">this law guarantees
the flow rate in an hourglass to be constant</cite>, since discharge depends on
local stress at the orifice, which has saturated.

**Recommendation: know about this, don't build it yet.** It requires a vertical
stress field with lateral shedding, which is a whole new pass. Three notes for
when it comes up:

- It is the correct explanation for arching and for why a hopper doesn't drain at
  a rate proportional to how full it is. If the engine ever wants a working
  hourglass or silo, this is the mechanism, not a tuned constant.
- `structural.rs` already runs a label-correcting relaxation over connected
  `Solid`/`Plant` cells (`d = 1 + min(neighbours' d)`). A vertical-stress
  relaxation with a lateral-shedding term has the identical computational shape.
  If this is ever built, it should reuse that machinery and the active-site
  scheduler rather than adding a third relaxation system.
- The engine's coarse `pressure` channel is an *air* pressure field. Granular
  stress is a different quantity with different boundary conditions and must not
  be folded into it. (Report B separately recommends that liquids should be
  reading and writing that channel, which they currently do not — but that is
  hydrostatic pressure, not granular stress, and conflating the three would be a
  mistake.)

---

## 5. Finding 4: dilatancy, and why M9 has no model without it

**Dense granular matter must expand in order to flow.** This is Reynolds
dilatancy, and it is the single most under-appreciated fact about sand for
someone building a game with it. <cite index="147-1">When slowly sheared,
granulates flow, but to do so they must overcome geometrical (steric) hindrance;
the resulting expansion of the material is referred to as Reynolds
dilatancy</cite>, and MRI measurement of the local packing density in slow shear
found <cite index="147-1">the dilatancy to be surprisingly strong, with the
dilated zone following the region of large strain rate and slowly spreading over
time — suggesting the local packing density is governed by the total amount of
local strain experienced since the start</cite>.

That last clause is the useful one: **packing state is a memory of accumulated
strain.** It is per-cell state that persists and decays, which is exactly the
kind of thing this engine is good at.

Everything M9 needs falls out of it:

- **Burial.** A body sinking into loose sand compacts it locally; compacted sand
  resists further sinking. Currently there is no variable to express the
  difference.
- **Digging.** Removing material from a dense pack requires dilating it first,
  which is why digging into packed sand is harder than scooping loose sand, and
  why the effort is front-loaded.
- **Footprints and tracks.** Compaction under load, persisting after the load
  leaves.
- **Why a poured pile differs from a settled one** — which is the same
  bistability §2 produces, arriving from the other direction.

### Where it lives

`Cell::aux`'s own doc says: `Powder`/`Gas` → unused, always 0. That is the slot.
Powder `aux` becomes a packing-fraction scalar on a fixed-point scale, exactly
as `Liquid` `aux` became a fill fraction on the `LIQUID_FULL` = 1000 scale.

The rule shape, kept deliberately minimal:

- A grain that moves **dilates**: its own packing and its neighbours' drop by a
  small increment.
- A grain at rest **compacts** slowly toward a material-specific maximum.
- Packing modulates the reach thresholds from §2: densely packed material holds a
  steeper slope. This is well supported — <cite index="111-1">for a given grain
  shape, denser packings are generally more stable</cite>.

Two cautions:

- **Cost.** `update_powder` runs for every visited powder cell. The engine's own
  standing lesson from M14 — a check that is individually cheap is not free at
  CA-sweep scale, and defaulting `heat_conductivity` to 0.0 was the fix — applies
  directly. Dilatancy should have the same shape: an early exit when packing is
  at its resting default, so a settled pile pays nothing.
- **Don't over-model.** The full constitutive treatment is the μ(I) rheology
  (§7), which needs shear rate and pressure per cell. A single packing scalar
  with dilate-on-move and compact-at-rest captures the *qualitative* behaviour —
  hysteresis, burial, digging resistance — at a small fraction of the cost. That
  is the right first cut.

---

## 6. Finding 5: hole propagation — the void model, and its known failure

The README lists "hole-propagation granular flow" as unbuilt. The literature has
a specific name and a specific warning for it.

Two dual formulations of granular drainage exist. Litwiniszyn's has particles
random-walking through a fixed array of cages; <cite index="145-1">Mullins
independently derived the same kinematic model from the hypothesis that
particles move passively downward in response to the upward diffusion of "voids"
emanating from the silo orifice — formally equivalent, but analogous to vacancy
diffusion in crystalline solids</cite>. Caram & Hong later implemented the void
version directly as a lattice simulation, which is essentially what a
falling-sand engine would build.

**The warning:** the void model over-predicts particle diffusion, badly. Direct
measurement in a hopper found <cite index="141-1">the diffusion of the particles
to be significantly less than predicted by the void model</cite>. The mechanism
of the failure is stated plainly: <cite index="144-1">if a tracer particle is
placed in a uniform flow driven by voids, the particle makes a directed random
walk downward with precisely the same diffusion length as the voids moving up —
so particles are easily mixed before they drop by a few particle diameters,
which goes against everyday experience and experiments</cite>.

In engine terms: a layered pile of coloured sand drained through a hole would
turn to uniform mush within a few cells of falling. That is a visible, obviously
wrong outcome, and it would be blamed on the renderer or on shade selection
rather than on the movement rule.

The fix is Bazant's **spot model**: <cite index="140-1">rather than voids, there
exist spatially distributed spots responsible for transporting the interstitial
space generated during discharge</cite>, based on <cite index="146-1">a simple
mechanism for cooperative diffusion — and the experimental data are consistent
with it</cite>.

**Practical translation for a CA:** if hole propagation is built, a hole must
displace a small *correlated group* of cells rather than swapping with one
random neighbour. Correlated motion is the entire difference between right and
wrong here, and it is cheap — a spot of a few cells wide, moving as a unit, with
the randomness applied to the spot's path rather than to each cell
independently.

Also worth having as a target: the discharge scaling. The same hopper study found
<cite index="141-1">the flow rate scales with the orifice size to the power of
1.5, consistent with dimensional analysis</cite> in their quasi-2D geometry, and
notes the counterintuitive result that flow rate *increases* with funnel angle.
Both are testable in this engine directly.

---

## 7. What was considered and rejected for now: μ(I) rheology

The modern unifying framework for dense granular flow is the μ(I) rheology,
which makes both the effective friction and the packing fraction functions of a
single dimensionless **inertial number** I = γ̇d/√(P/ρ), combining shear rate,
grain size, density and pressure. The empirical laws are compact:
<cite index="153-1">μ(I) = μ_s + Δμ/(I_0/I + 1) and φ(I) = φ_max − Δ_φI, with
Δμ ≈ 0.3, I_0 ≈ 0.3 and Δ_φ ≈ 0.1</cite>. It has been validated across
<cite index="148-1">chute flows, plane shear, annular shear cells, granular
collapse, planar silos, heap flows and rotating cylinders</cite>.

Note that φ(I) *is* dilatancy — §5's packing scalar is the crude version of this
same relation.

**Rejected for this pass, for a structural reason rather than a performance one:**
μ(I) requires a shear rate and a local pressure per cell. This engine's powder
cells have no velocity at all — movement is a discrete cell swap, not an
integrated trajectory — and there is no granular stress field (§4). Adopting
μ(I) means adding both, which is a larger change than everything else in this
report combined, and it would sit awkwardly next to a movement rule that can only
move one cell per frame.

It is also known to have limits: <cite index="149-1">the μ(I) approach is
considered to break down above I ≈ 0.5 for glass beads, when collisional
mechanisms come into play</cite>, and <cite index="150-1">nonlocal/cooperative
effects are necessary to properly describe dense granular flows</cite>.

**Flagged for Report C.** Rigid-body–granular coupling is where a real
constitutive model stops being optional, because a rigid body needs a *force*
back from the sand, and "which cell swapped with which" doesn't supply one. If C
concludes a stress field is needed, μ(I) is the framework to evaluate at that
point, with §5's packing scalar already in place as its φ term.

---

## 8. Acceptance criteria

The engine currently has no quantitative granular test. Every number below is
published, measured in a real experiment, and checkable in `examples/ascii.rs`
without a GPU.

### 8a. The primary test: 2D granular column collapse

This is the standard granular benchmark and it happens to be *natively 2D*,
which almost nothing else in the granular literature is. A column of height H₀
and width L₀ is released onto a flat floor; the runout is measured. Lube et al.
released columns confined between two vertical walls and obtained, for aspect
ratio a = H₀/L₀:

<cite index="138-1">(r∞ − r₀)/r₀ ≈ 1.2a for a < 2.3, and 1.9a^(2/3) for
a ≥ 2.3.</cite>

Two things make this the right acceptance test:

- **The transition matters more than the constants.** The change in exponent
  around a ≈ 2.3 is a genuine regime change — <cite index="135-1">at low a the
  flow is friction-dominated, with the column's edges falling while its inner
  part stays relatively undisturbed, and a simple linear relation between runout
  and aspect ratio is commonly reported</cite>. A model that produces a single
  power law across all aspect ratios is missing something real. The current
  engine, with no hysteresis and no packing state, will almost certainly produce
  one exponent.
- **It is cheap.** A column, a floor, and a measurement of the final deposit
  extent. Ten aspect ratios from 0.5 to 6, worst-frame timing recorded alongside.

Suggested pass bar, deliberately loose given lattice resolution: the fitted
low-a slope within ±30% of 1.2, a detectable exponent break in 1.5 < a < 3.5,
and the high-a exponent below 0.85 (i.e. genuinely sublinear). Tighten later.

### 8b. Two angles, and the gap between them

Build a pile by pouring, measure the resting slope: that is θ_r. Then tilt (or
add grains at the apex) until it fails, and measure the slope at the instant of
failure: that is θ_ms. **The test is that these are different numbers**, with
θ_ms > θ_r by roughly the authored gap. Target δ ≈ 8°, per the drum
measurements in §2.

Deletion test: with `FLAG_FLOWING` removed, δ must collapse to ≈ 0 and the test
must fail. If it still passes, the mechanism isn't doing anything.

### 8c. Avalanche size distribution is peaked, not power-law

Drive a pile grain-by-grain, record avalanche sizes, histogram them. The
distribution must be <cite index="109-1">Gaussian-like</cite> — a characteristic
size with a spread — not scale-free. This is simultaneously the acceptance test
for §2 and the regression test against anyone reintroducing BTW dynamics later.

### 8d. Repose accuracy

The existing README limitation: 39/30/18 measured against 45/34/22 authored.
After §2, the *resting* slope of an undisturbed pile should read within ~2° of
the authored θ_ms rather than 5–6° under. Keep the existing angle-of-repose
ascii scene as the harness; just start recording the number instead of eyeballing
the shape.

### 8e. Drainage does not over-mix (only if §6 is built)

Pour two visually distinct powders in horizontal layers, drain through a
centre hole, and check that the layers remain distinguishable in the discharged
material for at least ~10 cells of fall. A single-void random-walk implementation
will fail this; a spot-based one should pass. This is the test that makes §6's
warning actionable rather than advisory.

### 8f. Cost ceiling

Every mechanism here runs inside `update_powder`, which is the hottest loop in
the engine. The existing full-screen stress scene is the guard: **the CA-only
worst frame must not regress by more than 15% (serial) against the ~23 ms on
record.** If dilatancy alone costs more than that, its early exit is wrong.

---

## 9. Deletion tests

Per the standing rule for this research plan — every recommended mechanism ships
with a test that fails when the mechanism is *removed*, not merely one that
exercises it.

| Mechanism | Test that must fail without it |
|---|---|
| `FLAG_FLOWING` / two-angle model | 8b — the two measured angles collapse to one |
| Velocity weakening | 8c — avalanche sizes lose their characteristic scale |
| Packing scalar (dilatancy) | A body dropped into freshly poured sand must sink measurably deeper than into settled sand of the same depth |
| Packing → reach coupling | A compacted pile must hold a measurably steeper slope than a loose one of the same material |
| Spot correlation in hole propagation | 8e — layers must survive drainage; single-void diffusion destroys them |

Note that the current codebase would fail every row of this table, because none
of these mechanisms exist yet. That is the point of writing the table before the
code.

---

## 10. Recommended build order

1. **§2, the two-angle model.** One flag bit, one extra threshold, one new
   optional `.ron` field. Highest ratio of behaviour gained to lines changed of
   anything in this report, and it retires a README limitation.
2. **§8a and §8b as tests, immediately after.** Before any further mechanism.
   The column-collapse harness is what makes every subsequent change measurable
   instead of judged by eye, and it is the thing the engine most conspicuously
   lacks.
3. **Delete BTW toppling from PLAN.md**, with §3 as the recorded reason. A
   one-line change that prevents a wasted milestone.
4. **§5, the packing scalar**, with a hard early exit. Gate on 8f.
5. **§6, hole propagation, only if a hopper/drain use case actually appears.**
   It is not needed for piles, pours or avalanches — §2 covers those — and it
   carries the mixing hazard. Build it when something needs a silo.
6. **§4 (Janssen) and §7 (μ(I)): not now.** Revisit §4 if silos or hourglasses
   become content; revisit §7 when Report C reports back on whether rigid-body
   coupling needs a real stress field.

---

## 11. What was not directly accessible

Named explicitly, matching this repo's existing convention rather than quietly
glossed:

- **Lee & Herrmann (1993), J. Phys. A 26(2)** — the θ_ms/θ_r distinction's
  primary source. Read via the IOP abstract and a full-text mirror of the figures
  and discussion; the specific claims attributed to it here (finite θ_R from
  avalanche-generated piles, non-zero tilting angle, θ_MS > θ_R) appear in both
  and are corroborated by the 2018 *Powder Technology* review, but the paper was
  not read in full from the publisher.
- **Lube et al. (2005), the 2D confined-column collapse scaling** — the numbers
  in §8a are quoted from a secondary source (an SPH validation paper reproducing
  the scaling laws) rather than from the original *J. Fluid Mech.* paper, which
  is paywalled. Three independent papers quote the same 2D form, and the 3D and
  axisymmetric variants are quoted consistently alongside, so the numbers are
  well corroborated — but **if these are going to be used as a hard pass bar,
  the primary should be checked**, since the three geometries have genuinely
  different constants and picking the wrong one would silently mis-calibrate the
  test.
- **Jaeger, Liu & Nagel (1989), PRL 62:40** — the original "real sandpiles aren't
  SOC" measurement. Cited here through multiple later papers that quote its
  result identically; the original was not read.
- **GDR MiDi (2004) and Jop, Forterre & Pouliquen (2006)** — the μ(I) primaries.
  The functional forms and parameter values in §7 come from two later papers
  quoting them; adequate for a "not now" recommendation, insufficient if §7 is
  ever promoted to a build item.
- **Metcalfe et al.'s δ ≈ 8°** — quoted via a rotating-drum mixing paper, not
  from the original. Treat 8° as an order-of-magnitude default for
  `max_stability_angle`, not a calibrated constant.

Everything else — the rice-pile Nature abstract and its follow-up analyses, the
Janssen derivations, the Reynolds-dilatancy MRI paper, the void/spot model
comparison and its hopper measurements, the reduced-gravity repose experiment,
the packing-fraction stability study, and the lattice stability-condition
formulation — was read directly in the relevant sections.

---

## 12. Handoff

**To Report B (liquid, revision 2):** §2's two-threshold structure is the
mechanism Zhu & Bridson's Mohr–Coulomb yield check was reaching for in the
existing liquid report's §5 recommendation. A liquid cell choosing per step
between "settled mass" and "flowing surface" is the same shape as a grain
choosing between θ_ms and θ_r, and both should be expressed the same way — one
state bit, two thresholds — rather than as two unrelated mechanisms that happen
to rhyme. Report B should adopt this rather than re-deriving it.

**To Report C (coupling):** §4 and §7 are C's inheritance. The open question C
must answer is whether granular stress can be approximated well enough by an
impulse exchange through the existing field infrastructure, or whether a real
stress field is required. §5's packing scalar should be in place first either
way, since it is φ in any constitutive model C ends up recommending.
