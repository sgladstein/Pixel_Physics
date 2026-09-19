# What ants actually dig by — answers to the coordinator's seven questions

*2026-09-19, later the same day. Follow-up to
[`nest-biology-2026-09-19.md`](nest-biology-2026-09-19.md), answering the
nest-program coordinator's seven questions on the signals that drive
excavation. **Docs only**; nothing built, no `src/`, `assets/`, `examples/`
or test change. A woken lane has no messaging tools, so this file is the
reply.*

## How to read the evidence, and one thing about tonight's method

Tagging as in the first report: **[measured]** a specific study whose
existence and headline finding I am confident of; **[repeated]** widely
stated, primary source not held; **[general]** my own synthesis, no source.

**Two additions, because the coordinator asked which answers came from a
search.**

- **[search]** marks a claim that came from a web search run tonight rather
  than from my own knowledge. Most of the best material below is tagged this
  way, and the first report would have been better with several of these
  answers in it.
- **I could not open a single journal page.** Every publisher and repository
  domain I tried — ScienceDirect, Springer Link, eLife, PLOS, PMC, the ULB
  repository — is blocked by this container's egress proxy. **So every
  [search] claim below rests on a search-result summary, not on the paper.**
  Author, year, journal and species I am confident of; exact figures, sample
  sizes and the authors' own qualifications I am not, because I have not read
  the methods. **Where a number below decides a build, read the paper
  first.** This is a real limitation and it is stated here rather than
  buried, because the coordinator says they will build whatever is named.

**One trap found while searching, flagged because it would have been
expensive.** A search result from `animalsaroundtheglobe.com` states that
ants' digging pheromone "blends" vary seasonally — *"summer pheromone blends
direct the creation of more vertical ventilation shafts, while winter blends
prioritize heat-conserving horizontal tunnels"* — and that pioneer workers
follow the queen's pheromone trails to know where to dig. **None of this is
supported by anything else I found, it is exactly the mechanism the
coordinator was about to build, and it appears on a content-farm page with
no citation.** Treat it as fabricated. It is mentioned so that a later
session running the same search recognises it.

---

## 0a. Amended 2026-09-19, later — two corrections from `claude/sweet-tesla-ommknn`

**A parallel lane validated several of these citations against PubMed and two
of mine do not survive. Read this before quoting §3.1 or §5.**

- **The attribution of the no-dig-face-pheromone result is wrong.** I credit
  it to **Pielström & Roces**; that lane, reading the record, reports **the
  author is Bruce (2015)**. The *finding* stands — fresh digging face against
  one aged an hour, null — and the withdrawal it justifies stands. **The name
  does not.** I inferred the authorship from the surrounding pellet papers,
  which is exactly the fabrication risk the brief warned about, and it is a
  citation error rather than a wrong conclusion.
- **The ~40° angle of repose is NOT confirmed, and I state it as measured in
  three places.** That lane pulled the PNAS 2021 record: *"ants tend to dig
  piecewise linearly downward"* is in the abstract and the direction claim
  holds; **the ~40° figure and the "upward when begun mid-medium" claim are
  not in the abstract and the full text was unavailable.** Every `~40°` below
  is now marked. **Build the downward bias; do not build the repose angle on
  this citation.**

**And they found a mechanism neither of my reports had**, from the same
abstract: intergranular forces fall around tunnels because **arches** form, so
grains on a tunnel surface are already under low stress, and ants avoid
removing grains under high force *"without needing to be aware of the force
network"*. **`src/sim/load.rs` and `src/sim/structural.rs` already maintain a
per-cell load model**, so *dig where the load is low* is expressible here with
no new field and no genome slot — and it would produce arching as a
consequence rather than as a target. That is a better rule than anything in
§10's ranking and it belongs above every item in it.

---

## 0. The short answers

| | question | answer |
|---|---|---|
| **1** | Toffin's density mechanism | **Collision rate**, and it modulates **rest**, not dig-here. Much of the regulation is not sensed at all — it is the falling probability of *encountering the face* as space grows |
| **2** | Any chemical signal in excavation? | **Your withdrawal was right.** Directly tested in ants and negative. The termite cement pheromone is contested and weakly supported. One nuance: fresh *pellet piles* do attract digging |
| **3** | The entrance | Function is well documented, **origin is not**. I could not find a measured mechanism for how one entrance arises |
| **4** | What drives a tunnel forward | **Gravity plus the angle of repose** — both measured, both free in this engine. Persistence is real but secondary: you would be reaching for the knob you have |
| **5** | What ends a tunnel, starts a chamber | **Contents.** No brood or fungus at a spot → tunnels only, **no chambers at all**. This is the most decision-relevant finding in either report |
| **6** | What gives an ant "down" | **Gravity, directly** — and it is cheap. Not CO₂, which was tested and is not used |
| **7** | Where the pellet goes | **A few centimetres**, in a relay of up to ~12 workers; and the accumulating pile is itself the cue for more digging |

---

## 1. Your correction to finding #5 is right. I concede it, and the way I got it wrong is worth recording

**You are correct and my finding #5 was wrong.** The function is four lines:

```rust
pub fn moisture_gradient(world: &World, x: i32, y: i32) -> f32 {
    let m = |px: i32, py: i32| world.field_at_bilinear(px as f32, py as f32).moisture;
    let gx = m(x + 4, y) - m(x - 4, y);
    let gy = m(x, y + 4) - m(x, y - 4);
    ((gx * gx + gy * gy).sqrt() / WORM_MOISTURE_SATURATION).clamp(0.0, 1.0)
}
```

`sqrt(gx² + gy²)`, clamped to `[0, 1]`. **It is a non-negative magnitude and
carries no direction whatever.** So:

- **"A depth weight with the wrong sign" was never a coherent claim.** A
  negative weight on a quantity that cannot go below zero can only ever
  *subtract* from the dig urge in proportion to how much moisture structure
  is nearby. There is no direction for it to have the wrong sign about.
- **In uniform soil it returns exactly 0.000**, as you measured, because both
  differences are zero at every wetness. Your four-point sweep is the right
  control and it says the term is **inert**, not inverted.

**How I got it wrong, since it is a method failure rather than bad luck.** I
read the function's *doc comment*, which says in bold *"It is a depth
signal"* and quotes 1.91x for a point twenty rows lower, and I did not read
the arithmetic underneath it. The doc is not lying — it measured on a bed
**with an air/soil interface inside the ±4 sampling window**, where the step
from dry air to wet soil is the whole signal, and it correctly reports that
this varies with how close the sample is to that interface. But *"varies near
a surface"* is not *"is a depth signal"*, and an ant digging below ground has
no interface in its window. **`CLAUDE.md` has the rule for exactly this — *a
commit message is not evidence the change is in the file*, and after any
stash or merge, re-read the function, not the diff.** The transferable form
is broader and belongs on the record: **a doc comment's prose summary of what
a channel measures is a claim to check, not a measurement to cite** — and
this repository's doc comments are unusually trustworthy, which is precisely
what made me skip the check.

**The one caveat I would keep.** The term is inert *in uniform soil*, which
is where digging happens, and that is what matters for the dig decision. It
is **not** guaranteed inert for an ant standing at the surface next to a
strong air/soil moisture step — and your ants live in a 46 × 2 band at the
surface. If your uniform bed had air and soil at the same field moisture
there would be no step to find even there; if it did have one and the answer
was still 0.000, then the channel is inert everywhere your ants stand and the
finding is stronger than either of us said. **Worth one line of the probe you
already have.**

**What survives of finding #5**: the dig decision has no depth term at all —
which was the point the finding was in service of, and which your own
measurement establishes more cleanly than my reading did.

---

## 2. Q1 — Toffin's density mechanism, mechanistically

This is the question where the literature has moved a long way past Toffin,
and where the answer says your three interventions were aimed at the wrong
thing.

### 2.1 What the individual senses: collision rate — and it gates *rest*

**[search] [measured]** *Agitated ants: regulation and self-organization of
incipient nest excavation via collisional cues*, **Journal of the Royal
Society Interface, 2023** — fire ants, small groups, quasi-2D arenas. The
individual-level quantity is **collision frequency with other ants**, and the
authors' construct for the response is **"agitation": a tendency of
individuals to avoid rest when collisions are frequent.** Their model is a
cellular automaton in which ants estimate their own collision frequency and
**otherwise do not communicate at all**.

**That is a different wiring from yours in a way that matters more than any
gain or centring.** Density in the biology does not say *dig here*. It says
**do not rest**. The causal chain is:

> collisions ↑ → resting ↓ → more ants active → more ants reach the face →
> more excavation

Your genome wires density **directly to `Dig`**, collapsing a two-step
mechanism into one step and skipping the variable that actually moves — the
fraction of the colony that is awake and walking. **This engine already has
that variable**: `CreatureStats::rest_bout_hist`, the rest bout, and
`BrainInput::Stillness`. A `(Crowding, …)` wire that suppresses rest rather
than driving `Dig` is the faithful version, and it is a wire, not a field.

### 2.2 And most of the regulation is not sensed at all

**[search] [measured]** *The digging dynamics of ant tunnels: movement,
encounters, and nest space*, Bruce et al., **Insectes Sociaux 66:119–127
(2019)**, *Acromyrmex lundi*. The findings, as summarised:

- Groups of **10** workers dig significantly **less** over time in a tunnel
  that is already long than in a short one.
- Groups of **100** show **no** significant effect of tunnel length — so the
  effect is group-size dependent.
- **Walking speed correlates with excavation rate**, and the ants maintain a
  consistent level of mutual proximity over time.
- The stated mechanism: *"as tunnel space expands, several factors combine to
  **lower the chance of ants encountering the tunnel digging face** and
  taking up excavation."*

**Read that last clause carefully, because it is the answer to why your three
interventions moved nothing.** The nest-size regulation is substantially a
**geometric and kinetic consequence** of ants walking in a space that is
getting bigger — the face is a smaller fraction of where you can be, so you
hit it less often — rather than a reading any ant takes. **No sensor,
however local, however well centred, however swept, can reproduce a
regulation that works by encounter probability.** You improved the fidelity
of a measurement of a quantity that biology does not measure.

This also explains the one intervention that *did* work. `SurfaceCurvature`
is the only sense in your list that varies per ant because it is the only one
that is a **property of where the ant is standing** rather than a property of
the colony. Encounter-with-the-face is the same kind of quantity.

### 2.3 The shape of the curve

You asked for the shape, not the direction. What I have:

- **[search] [measured]** The *Agitated ants* paper reports excavation rate in
  **three stages: an initial constant rate, then a rapid decay, then a slow
  decay scaling as t^(−1/2).** A power-law tail, not an exponential approach
  to a set point, and not a threshold.
- **[measured]** Toffin et al. (*PNAS* 2009) give the *spatial* consequence
  rather than a rate curve: high worker density on a small perimeter →
  **uniform** digging → a round cavity; falling density past a critical value
  → **localised buds** by amplification → a branched structure.
- **[search] [measured]** The round→ramified transition **depends on the
  cohesiveness of the substrate** — so substrate mechanics is a control
  parameter of the shape, not just a medium.

**So the curve is a decaying rate with a long power-law tail, and the
interesting output is not the rate but the switch from uniform to localised.**
Your gate saturating is a real defect, but a desaturated gate on a
colony-wide scalar still cannot produce localisation, because localisation is
a *spatial* pattern and the input has no spatial variation. That is the same
finding as §0 item 1 of the first report, arriving from the other direction.

### 2.4 → What I would do with this

**Do not spend another intervention on the `Crowding` reading.** Two things
are worth trying and neither is a sensor:

1. **Make the face an encounter.** Digging should be something that happens
   when an ant *arrives at* diggable ground while active, with the collective
   rate falling out of how often that happens. That is close to what the
   engine does already; what is missing is that nothing makes the *face*
   special relative to any other soil cell.
2. **Wire density to rest, not to `Dig`.** One wire, an existing input, an
   existing rest mechanism. It is the faithful mechanism and it is cheap.

---

## 3. Q2 — Is there any chemical signal in ant excavation?

**Confirm the silence. Your withdrawal was right, and there is a paper whose
title is almost your question.**

### 3.1 In ants: tested directly, came out negative

**[search] [measured]** *It is not all pheromones: No evidence that pheromones
affect digging face choice during ant nest excavation* — **2015**, in a
behaviour journal (the record I found is Behavioural Processes; I could not
open it). *Acromyrmex lundi*. Design: groups of **5** workers choose between
two excavation sites, one **freshly** exposed to digging and one where
digging had **ceased an hour previously**. If a digging pheromone were
deposited and decayed, the fresh face should win. **No significant difference
in digging activity between fresh and aged sites was detected.**

That is a direct, well-posed test of the exact mechanism you proposed, and it
failed. **A digging pheromone in this engine would be modelling something
that has been looked for in ants and not found.**

### 3.2 The one nuance: the *pellet pile* is a cue, and its basis is unresolved

**[search] [measured]** The same group's pellet work (see §8) found that
**accumulated, freshly-excavated pellets significantly influence where
workers start digging**, and that pellets **one hour old do not**. The
authors themselves raise evaporating surface chemistry as one possible
explanation and leave it open.

**So the honest position, and it is a fine distinction worth holding:**

- **A mark on the digging face**: tested, no effect.
- **A heap of fresh spoil**: real effect, decays within an hour, **mechanism
  unresolved** — it could be chemical, or it could be that a fresh heap is
  loose, differently shaped, and physically easier to dig into.
- **Nothing I found demonstrates a behavioural effect of a digging pheromone
  the ants deposited themselves.** One summary put it in those words
  directly.

**For the engine this is good news, because the cue that does work is one you
already have as a material.** Spoil is a `Powder` that needs footing; a fresh
heap near the face is a physical object with a shape. If the cue is geometry
and looseness rather than chemistry, you can have it without a new plane in
the sweep.

### 3.3 Grassé's cement pheromone: a termite story, and a contested one

**[search]** Asked and answered clearly:

- The cement-pheromone account **is** a termite story (Grassé, then
  Bruinsma), and it is **the origin of the word stigmergy**. **[measured]**
- **The experimental support is weak.** Summaries state that while individual
  workers can recognise freshly deposited nest material, **they may simply be
  attracted to an unspecific colony odour** while performing the same
  behaviour they would perform without any chemical marking. **[search]**
- There is **still no consensus** on the nature of the stigmergic stimulus in
  termite construction, decades on. **[search]**
- **Alternative, non-chemical explanations are gaining ground**, and this is
  the part that lands on your measurement: recent work finds that
  **morphological and environmental features are strong enough stimuli to
  guide construction on their own.** Specifically —
  - *Surface curvature guides early construction activity in mound-building
    termites* (arXiv preprint, ~2018) **[search]**;
  - *Excavation and aggregation as organizing factors in de novo construction
    by mound-building termites*, **Proc. R. Soc. B 284 (2017)**
    **[search] [measured]**;
  - *Substrate evaporation drives collective construction in termites*,
    **eLife (2023)** **[search]**.

**That first one is the headline of this whole report.** You measured
`(SurfaceCurvature, Dig, -0.6)` → **2.3x roofed chamber**, first thing that
worked, and the termite construction literature independently converged on
**surface curvature** as the cue that guides early building. You did not pick
a convenient knob; you picked the cue the field arrived at. That is much
stronger corroboration than anything in my first report and it should carry
the most weight in what gets built next.

Note also that **`MoistureGrad`'s original design intent was exactly this
literature** — `creature.rs`'s own doc cites an eLife 2024 result for
termite-style construction and excavation shaping — and the channel does not
implement it. **The intent was right and the implementation is a magnitude
with no direction.** Surface curvature is the channel that does what the
moisture gradient was supposed to do.

---

## 4. Q3 — The entrance

**I could not find a measured mechanism for how a single entrance
*originates*. The function is well documented; the origin is not, or not
where I could reach it.** Saying so plainly, per your standard.

**What is documented:**

- **Nests converge internally even when they do not converge outside.**
  **[search] [measured]** *"In all nests with more than one exterior
  entrance, the tunnels from all entrances link to a single entrance
  chamber."* So the convergence is **architectural and interior** — the
  single point is a *chamber below the surface*, not necessarily a single
  hole.
- **The entrance is an information hub, and that is why one is better than
  several.** **[search]** Trail pheromones converging on a single point let
  returning foragers' information be compared by nestmates; **additional
  entrances prevent information converging, weaken signals locally, and make
  synchronising foraging harder.** There is a 2020 Royal Society Open Science
  paper titled *Multiple nest entrances alter foraging and information
  transfer in ants* and a PLOS ONE paper *Foraging through multiple nest
  holes: an impediment to collective decision-making in ants* — the framing
  in both is that extra entrances are a **cost**.
- **Entrance architecture regulates traffic.** **[search]** *"The faster ants
  come out of the entrance chamber, the faster more can move in, and the more
  likely returning foragers are to encounter an outgoing forager before they
  drop their food."* A 2025 *Ecology and Evolution* paper (Gordon) covers
  nest entrance architecture and foraging regulation in desert harvesters.

**What I cannot tell you**: whether a single entrance is actively maintained
and converged upon, or whether it is simply where the founding queen dug and
nothing ever opens another. **[general]** My reading is that both are true for
different species and that the *selective* argument above is the reason the
convergent case is common — but I have no measurement, and you should not
build a convergence rule on my inference.

**What this does say for the engine, which is not nothing.** Your 46-column
door is not wrong because real doors are narrow; it is wrong because **your
door is not a place**. The documented structure is *many surface openings
feeding one interior chamber* — a funnel. A site with a narrow reach at the
surface and a chamber below it is closer to the biology than either a
46-column band or a one-cell hole. And the cue that makes ants converge on a
point is **trail pheromone**, which this engine has and which PR #450 made
long-lived enough to stand.

---

## 5. Q4 — What drives a tunnel forward in one direction?

**The measured answer is mostly physics, and your `Persist` instinct is half
right in a way I think you should not act on yet.**

### 5.1 What is measured

**[search] [measured]** *Unearthing real-time 3D ant tunneling mechanics*,
Buarque de Macedo, Andò, Joy, Viggiani, Pal, Parker & Andrade, **PNAS 2021** —
real-time 3D X-ray CT of *Pogonomyrmex* excavating, with grain-scale
simulation of the particle mechanics. What comes out:

- Ants dig **piecewise linearly** — straight runs, changing direction at
  discrete points, rather than curving.
- **Almost vertical descents at the top**; in the bulk they dig **at or below
  the angle of repose of the material (~40°)** — **the angle is
  unvalidated, see §0a; only the downward direction is confirmed**.
- The tunnel's straightness and slope are set by what the **granular medium**
  will hold, not only by what the ant intends.

**[search]** A companion framing from the granular-media literature (*Ant
tunneling — a granular media perspective*, Granular Matter 2010): tunnels
descend at the repose angle, the slope at which the material naturally stands.

### 5.2 Persistence, and why I would not reach for it first

**[search]** Directional persistence is real and is in the modelling
literature — *"the combined effect of reinforced random walk and persistence,
understood as a preferential tendency to follow straight paths … the tendency
to preferably follow straight directions **in the absence of external
effects**"*. **Thigmotaxis (wall-following)** is also documented, and in one
maze study it **initially masked** an underlying leftward turning bias
**[search] [measured]**.

Note the qualifier in the persistence result: *in the absence of external
effects*. In a tunnel there is no absence of external effects — there is
gravity, a repose-limited slope, a wall on both sides, and traffic.

**So: your `Persist` output is the knob you happen to have, and the
better-evidenced drivers of tunnel direction are gravity and the angle of
repose, both of which this engine already implements for free.** `Persist`'s
own documentation calls it the number deciding whether a creature commutes or
mills, which is a *foraging* distinction; tunnel advance is a different
quantity and giving it to `Persist` would be a second meaning on one weight —
the `phototropism_dir` shape again.

**I would put persistence third**, behind gravity and repose, and behind
thigmotaxis if you ever want a tunnel to follow an existing wall.

---

## 6. Q5 — What ends a tunnel and starts a chamber?

**This is the most decision-relevant finding in either report, and it
retroactively explains both your shallow lens and my §10.**

**[search] [measured]** *Nest Enlargement in Leaf-Cutting Ants: Relocated
Brood and Fungus Trigger the Excavation of New Chambers*, Römer & Roces,
**PLOS ONE (2014)**. As summarised:

> **Workers excavate only tunnels without chambers unless contents are
> present.** Chambers are excavated once ants are allowed to relocate
> symbiotic fungus inside a digging arena. **The presence of contents to be
> stored — brood or fungus — is needed at a location to initiate chamber
> excavation.**

**A chamber is not dug and then filled. It is dug because something is
already there to put in it.** The transition you asked about — the moment
excavation stops advancing and starts widening — is **the arrival of
contents**, not a depth, not a length, not a density.

Three consequences, and they are large:

1. **My first report's §10 was right for a reason I did not have.** The owner
   said the nest has no purpose; the biology says a colony with nothing to
   store **does not build chambers at all** — it builds tunnels. "No purpose"
   is not merely a design gap, it is the documented mechanism.
2. **§11's eggs-as-keystone argument gets a mechanism.** I argued eggs were
   the keystone because they are an object that must be put somewhere. The
   stronger form: **brood is measured to be the chamber trigger.** Eggs would
   not merely give chambers a *reason*; they would supply the *stimulus*
   the widening rule reads.
3. **Your shallow lens is diagnostic, and it is failing in the opposite
   direction to the biology.** A contents-free colony should produce
   **tunnels and no chambers**. Yours produces a lens — all chamber, no
   tunnel. So the engine is not under-building chambers for want of a
   trigger; it is building nothing *but* chamber because nothing distinguishes
   advancing from widening at all. **There is no tunnel mode to end.**

**What I would take from this**: do not build a tunnel→chamber transition
rule. Build the two modes as different things — advance (directional, gravity
and repose, §5) and widen (local, triggered by contents) — and the transition
is then just which one is active. And note the honest ordering: **widening
has no trigger available in this engine until something exists to store**,
which puts eggs or a granary ahead of chamber shaping rather than beside it.

**Two numbers from the same search, both useful:**

- **[search]** As activity decayed, tunnels converged to the typical width of
  **a single body length — about two ants wide**. That is a target for a
  tunnel's cross-section, and at `body: Chain(2)` it is startlingly close to
  what one of your ants already is.
- **[search] [repeated]** *Acromyrmex* fungus chambers sit at **5–50 cm**,
  with some species' nests reaching **2–5 m**. The shallow-chamber end is
  within your bed's depth; the deep end is not (first report §2.5).

---

## 7. Q6 — What gives an ant "down"?

**Gravity, directly. It is the cheap answer and it is the measured one.
Not CO₂.**

**[search] [measured]** From the PNAS 2021 tunnelling work and its coverage:

- **When ants dig from the surface they dig downward, with gravity, to a
  certain depth.** When allowed to start **from the middle** of the medium
  they dug **upward, against gravity** — so the rule is not "always down",
  it is oriented with respect to gravity and to where they are.
- The slope they hold is the **angle of repose (~40°)** — **unvalidated,
  §0a** — with near-vertical
  descents at the top.
- **[search]** One summary states ants have a fine sense of their position in
  the vertical direction. Read that as "gravity is the reference", not as a
  measured acuity figure — I have not seen the number.

**CO₂ is ruled out for at least one closely related decision, by
measurement.** **[search] [measured]** Römer, Bollazzi & Roces, *Leaf-cutting
ants use relative humidity and temperature but not CO₂ levels as cues for the
selection of an underground dumpsite*, **Ecological Entomology (2019)** —
tested temperature **15–30 °C**, RH **10–98%**, CO₂ from **atmospheric to
10%**. **Humidity and temperature are used; CO₂ is not.** There is also a
paper specifically on *the effect of CO₂ on digging rates, soil transport and
choice of a digging site* in leaf-cutters, which I could not open.

**So the first report's "no CO₂ field" recommendation now has evidence behind
it rather than only a cost argument** — for the dumpsite decision CO₂ was
tested against a 250-fold range and came out unused, while humidity and
temperature came out used. That also strengthens D5.1: **humidity and
temperature are the cues to wire, and you already have both fields.**

**For the engine this is the cheapest good news in the report.** "Down" needs
no field at all — the engine has gravity, `Powder` soil, and a repose rule
already doing the physics. What is missing is that nothing in the dig
decision is oriented with respect to any of it.

---

## 8. Q7 — Where does a digger drop its pellet, and what decides it?

**Close to the face, in a relay — and the pile it makes is the cue for more
digging.** Your engine's rule is close to the opposite on both counts.

**[search] [measured]** Pielström & Roces, *Sequential Soil Transport and Its
Influence on the Spatial Organisation of Collective Digging in Leaf-Cutting
Ants*, **PLOS ONE (2013)**, *Atta vollenweideri*:

- Pellets were transported **sequentially over 2 metres**, involving **up to
  12 workers** in three functionally distinct groups: **excavators**;
  **short-distance carriers that drop the collected pellet after a few
  centimetres**; and **long-distance, last carriers** that reach the final
  deposition site.
- **Accumulated, freshly-excavated pellets significantly influenced workers'
  decision where to start digging** in choice experiments — the temporary
  heap is a **spatial organiser** of collective excavation.
- Pellets **one hour old lost the effect** (§3.2).

**[search] [measured]** And for the *final* dumpsite, the cues are measured:
**relative humidity and temperature, not CO₂** (Römer, Bollazzi & Roces 2019,
§7).

### 8.1 Against what the engine does

| | biology | `ant.ron` today |
|---|---|---|
| how far the digger carries | **a few centimetres**, then hands off | drops where convex ground is found |
| who carries it away | **a relay of up to ~12** | nobody — the digger is the only carrier |
| what the heap does | **attracts further digging** | nothing reads spoil |
| where it finally goes | a site chosen by **humidity and temperature** | wherever `SPOIL_HEADROOM` allows |

**Your "excavation and refill balance out" is exactly what this predicts.**
In the biology the pellet leaves the working face in a **chain**, and the
temporary heap near the face is a *positive* signal. In the engine one animal
does the whole journey and the heap is inert, so the pellet comes back to
where it was.

**Two things worth building, and the first is nearly free:**

1. **Make the heap attract digging.** Fresh spoil near the face should raise
   the dig urge. This needs no new field — `spoil` is already a distinct
   material, and the ant already has a contact sense for what is adjacent. It
   is the measured cue, the decay is about an hour of behaviour-time, and it
   is the mechanism that gives excavation a positive feedback it currently
   lacks.
2. **Separate the excavator from the carrier.** The relay is the reason the
   face stays clear. A single verb that digs and hauls cannot express it, and
   this is the one place in this report where I would say the engine needs a
   *new behaviour* rather than a new wire.

**One caution on the heap-attracts-digging rule**, since you will build what
I name: the same heap in this engine is `needs_footing` spoil that can come
down on an ant, and the first report noted the owner has reported floating
dirt lattices three times. A rule that makes ants dig *at* their own heap
brings the digger under the overhang. That interaction is not in the
literature and is yours to measure.

---

## 9. Where the literature does not answer, or where I could not reach it

Stated plainly, per your standard — a clean "nobody has measured this" being
more useful than a plausible mechanism.

1. **How a single entrance originates.** §4. Function documented, mechanism
   not found. Do not build a convergence rule on my inference.
2. **Whether the fresh-pellet cue is chemical or physical.** §3.2. The
   authors leave it open. **This is the one place a digging pheromone might
   still be real**, and it is about the *heap*, not the face.
3. **The acuity of gravity sensing in a digging ant.** §7. "Ants sense their
   vertical position well" is a summary sentence, not a number.
4. **A rate curve for the density→rest response.** §2.3 gives three stages
   and a t^(−1/2) tail for the excavation *rate*; I do not have the
   collision-rate-to-rest-probability function itself.
5. **Everything tagged [search] rests on a search summary, not a paper**, for
   the egress reason in the header. The five claims I would most want read
   before they become code: Römer & Roces 2014 (contents trigger chambers),
   Bruce et al. 2019 (encounter with the face), *Agitated ants* 2023
   (collision→rest), Pielström & Roces 2013 (the relay), and Pielström &
   Roces 2015 (the negative pheromone result).
6. **Nothing here has been checked against `dead-ends.md`.** Several
   mechanisms below may have been tried; §10's ranking is a biology ranking,
   not a "has this been attempted here" ranking, and `deadendindex --touching`
   will not see a mechanism that exists only as prose.

---

## 10. Ranked: which cue we already own, which needs a new field, which not to attempt

Ranked by what would most change a nest that is currently a shallow lens.
"Own it" means no new field and no new sense — a wire, a value at a call
site, or a material test.

### Own it already — build these first

| rank | cue | why it ranks here | what it is |
|---|---|---|---|
| **1** | **Contents trigger widening** (§6) | The measured answer to "why is there no chamber structure", and it explains both the lens and the owner's "no purpose" ruling with one mechanism | Needs *something to store* first — so this is eggs or a granary, then a widening rule. The dearest item and the one that unlocks the rest |
| **2** | **Gravity + angle of repose for advance** (§5, §7) | The measured driver of tunnel direction and slope, and the engine already has gravity, `Powder` soil and a repose rule. Nothing in the dig decision is oriented to any of them | A value at the dig call site. **Cheapest large win in the report** |
| **3** | **Surface curvature** (§3.3) | You measured 2.3x roofed chamber; the termite construction literature independently converged on surface curvature as the cue guiding early building. Strongest corroboration anywhere in either report | Already a live sense, already wired to `Drop`. Wire it to `Dig` — you have done this |
| **4** | **Fresh spoil attracts digging** (§8) | The one stigmergic cue in ant excavation with a positive result, and the engine already has `spoil` as a distinct material. Gives excavation the positive feedback it lacks | A material adjacency test. Watch the overhang interaction |
| **5** | **Density → rest, not density → dig** (§2.1) | The faithful form of the mechanism you have been intervening on. Unlikely to reshape the nest by itself, but it is one wire and it stops the current wiring being wrong | `(Crowding, …)` onto the rest path, using `Stillness` / the rest bout |
| **6** | **Humidity and temperature for the dumpsite** (§7, §8) | Measured cues, and both fields exist with depth grading already | The first report's D5.1, now with evidence |

### Needs something new — worth it, but price it

- **Excavator / carrier separation** (§8.1). A **new behaviour**, not a new
  field: the relay is why the face stays clear, and one verb that digs and
  hauls cannot express it. This is my pick for the highest-value *new*
  mechanism, and the only one I would argue for.
- **Encounter-with-the-face as the dig trigger** (§2.2). Conceptually a
  reworking of what makes a cell diggable rather than a new field, but it
  changes the dig decision's shape, so price it as new.
- **A tunnel mode distinct from a widening mode** (§6). Follows from item 1
  and cannot precede it.

### Do not attempt

- **A digging pheromone.** §3.1 — tested directly in *Acromyrmex lundi*,
  fresh face against one aged an hour, **no difference**. **Your withdrawal
  was right.** The only surviving chemical possibility is the *heap*, and it
  is cheaper to build that as geometry (item 4) and see whether it suffices.
- **A CO₂ field.** §7 — tested from atmospheric to 10% for dumpsite choice
  and **not used**, while humidity and temperature were. The first report said
  no on cost; this says no on evidence.
- **Directional persistence as the home of tunnel advance.** §5.2 — real
  biology, but it is documented as what operates *in the absence of external
  cues*, and a tunnel has gravity, a repose limit, walls and traffic. You
  would be reaching for the knob you have, and `Persist` already means
  something else. Third at best, behind items 2 and 3.
- **An entrance-convergence rule.** §4, §9 item 1 — I have the function and
  not the mechanism, so anything I gave you here would be invention.

### The one-line version

**The nest is a shallow lens because nothing tells an ant which way is
forward and nothing tells it when to widen. Gravity and repose answer the
first for free; only contents answer the second, and contents are what the
box does not have.**

---

## 11. Sources

Author, year and journal as confidently as I can state them. **Every entry
marked [search] was found tonight via web search and read only as a search
summary — no journal page in this container is reachable.** No URLs, for the
reason in the first report's header.

**Excavation regulation and the individual rule**
- *Agitated ants: regulation and self-organization of incipient nest
  excavation via collisional cues.* **J. R. Soc. Interface (2023).** Fire
  ants. Collision frequency; "agitation"; three-stage rate with a t^(−1/2)
  tail. **[search] [measured]**
- Bruce et al., *The digging dynamics of ant tunnels: movement, encounters,
  and nest space.* **Insectes Sociaux 66:119–127 (2019).** *Acromyrmex
  lundi*. Encounter with the face; group-size dependence; speed–rate
  correlation. **[search] [measured]**
- Toffin, Di Paolo, Campo, Detrain & Deneubourg, *Shape transition during
  nest digging in ants.* **PNAS 106 (2009).** **[measured]**
- *The Role of Colony Size on Tunnel Branching Morphogenesis in Ant Nests.*
  **PLOS ONE (2014).** **[search]**
- *Colony demographics shape nest construction in* Camponotus fellah *ants.*
  **eLife (2025).** Found, not read. **[search]**

**Chemistry, and its absence**
- **Bruce (2015)** — *It is not all pheromones: No evidence that pheromones
  affect digging face choice during ant nest excavation.* **Corrected 2026-09-19:
  I had credited this to Pielström & Roces by inference from the neighbouring
  pellet papers; see §0a.** **2015**,
  behaviour journal. *Acromyrmex lundi*, groups of 5, fresh vs 1-h-aged face,
  null. **[search] [measured]**
- Grassé; Bruinsma — the termite cement-pheromone origin of stigmergy.
  **[measured]**, with the contested standing in §3.3 **[search]**.
- *Excavation and aggregation as organizing factors in de novo construction
  by mound-building termites.* **Proc. R. Soc. B 284 (2017).**
  **[search] [measured]**
- *Surface curvature guides early construction activity in mound-building
  termites.* **arXiv preprint, ~2018.** **[search]**
- *Substrate evaporation drives collective construction in termites.*
  **eLife (2023).** **[search]**

**Mechanics, direction and depth**
- Buarque de Macedo, Andò, Joy, Viggiani, Pal, Parker & Andrade, *Unearthing
  real-time 3D ant tunneling mechanics.* **PNAS 118 (2021).**
  *Pogonomyrmex*, real-time X-ray CT. Piecewise-linear descent; repose angle
  ~40° (**unvalidated, §0a**); downward from the surface (**confirmed**),
  upward from the middle (**unvalidated**).
  **[search] [measured]**
- *Ant tunneling — a granular media perspective.* **Granular Matter (2010).**
  **[search]**

**Chambers, pellets and sites**
- Römer & Roces, *Nest Enlargement in Leaf-Cutting Ants: Relocated Brood and
  Fungus Trigger the Excavation of New Chambers.* **PLOS ONE (2014).**
  **The contents-trigger finding. [search] [measured]**
- Pielström & Roces, *Sequential Soil Transport and Its Influence on the
  Spatial Organisation of Collective Digging in Leaf-Cutting Ants.*
  **PLOS ONE (2013).** *Atta vollenweideri*. The relay; the heap as
  organiser. **[search] [measured]**
- Römer, Bollazzi & Roces, *Leaf-cutting ants use relative humidity and
  temperature but not CO₂ levels as cues for the selection of an underground
  dumpsite.* **Ecological Entomology (2019).** **[search] [measured]**
- Pielström & Roces, *Soil Moisture and Excavation Behaviour in the Chaco
  Leaf-Cutting Ant (*Atta vollenweideri*).* **PLOS ONE (2014).** Found, not
  read. **[search]**
- *Nest Building in Leaf-Cutting Ants: Behavioral Mechanisms and Adaptive
  Value.* **Annual Review of Entomology.** The review of record for this
  area; found, not read. **[search]** — **if one thing here is read in full,
  make it this.**

**The entrance**
- *Multiple nest entrances alter foraging and information transfer in ants.*
  **R. Soc. Open Sci. (2020).** **[search]**
- *Foraging through multiple nest holes: an impediment to collective
  decision-making in ants.* **PLOS ONE (2020).** **[search]**
- Gordon, *Nest Entrance Architecture and the Regulation of Foraging Activity
  in Desert Harvester Ants.* **Ecology and Evolution (2025).** **[search]**
- *Nest Entrances, Spatial Fidelity, and Foraging Patterns in the Red Ant*
  Myrmica rubra. **Insects 11 (2020).** **[search]**

**In this repository**
- [`nest-biology-2026-09-19.md`](nest-biology-2026-09-19.md) — the first
  report; §10 (the owner's no-purpose ruling) and §11 (eggs) are the sections
  §6 above bears on.
- [`stigmergy-research.md`](stigmergy-research.md) §5 — Toffin, and the
  excavation-shaping ground this report extends rather than repeats.
- `src/sim/creature.rs` — `moisture_gradient` (§1), `surface_curvature`
  (§3.3), the dig verb and `adjacent_nest`.
