# Population dynamics: why the ecology will go extinct, and what actually prevents it

**Report D of four.** Independent of A, B and C. Serves M18 Phase 2 and anything
multi-species that follows it, including the plant evolution work.

Extends `research/m18-creature-biology.md` §4, which is one paragraph. That
paragraph identifies the right prior art (Wa-Tor) and then makes one claim worth
correcting.

Out of scope: creature *behaviour* (covered by the existing M18 research —
burrowing mechanics, thermotaxis, foraging), Reynolds steering (well documented,
needs no research), and evolution of traits (covered by the plant-simulation
report's §7).

---

## 0. Summary, stated first

1. **The default outcome of a two-species predator–prey system is extinction, and
   this is an experimental result, not a modelling artifact.** Gause could not
   keep predator and prey coexisting in a homogeneous microcosm at any starting
   population. Huffaker only achieved it by deliberately engineering spatial
   structure *and* handicapping the predator's dispersal. If M18 Phase 2 ships a
   worm and a thing that eats worms and expects an ecology, it will get an empty
   world, and the bug report will read "predators too strong."

2. **Space is the stabilizing mechanism, and this engine gets more of it for free
   than any purpose-built ecology simulator.** Huffaker's apparatus was oranges,
   rubber balls and petroleum-jelly barriers — a hand-built approximation of
   caves, water, impassable stone and material-gated movement, all of which this
   world already has. This is the report's most encouraging finding.

3. **Mobility has a critical threshold above which coexistence collapses**, and
   it is sharp rather than gradual. This makes creature speed a *stability*
   parameter, not just a feel parameter, and it collides directly with M18's
   stated design note that entity perception is "cheap and unconstrained."

4. **Enrichment destabilizes.** Increasing the prey's food supply increases the
   amplitude of population cycles until one crosses zero. Every improvement to
   the plant, water and worldgen systems is an enrichment event. The ecology will
   get *less* stable as the rest of the engine gets better.

5. **A three-species cycle coexists where a two-species chain does not.** If the
   goal is a persistent ecosystem rather than a demonstration, cyclic dominance
   is the structure to build.

6. **One concrete engine bug this will expose immediately:**
   `World::push_creature` allocates `u16` slots from a `Vec` that never shrinks,
   guarded only by a `debug_assert` — and CI runs release only. A breeding
   population reaches 65,536 cumulative births quickly. §7c.

---

## 1. The claim in the existing research worth correcting

`m18-creature-biology.md` §4 says:

> Later CA variants (Cattaneo et al.) show these grid-local rules reproduce real
> predator-prey population oscillations more robustly than the continuous ODE
> version.

Directionally right, misleading as stated. The robustness does not come from
being a cellular automaton; it comes from being *spatially extended*. A CA with
well-mixed dynamics is no more stable than the ODE, and a CA with high mobility
is *less* stable (§4). The mechanism is spatial decorrelation — local patches
going through their own cycles out of phase with each other, so a crash in one
patch is repopulated from another.

This matters because it changes what to build. "Use a CA" is not a design
decision that buys stability. "Preserve spatial structure, and keep mobility
below the threshold that destroys it" is.

---

## 2. The default outcome is extinction

This is the finding to internalise before writing any code.

**Gause (1934).** In homogeneous microcosms, <cite index="36-1">Gause's
experiments had found that predator and prey populations would become extinct
regardless of initial population size.</cite> Not sometimes. Regardless.

**Huffaker (1958)** is the canonical demonstration of what fixes it, and the
degree of intervention required is the instructive part. He built
<cite index="36-1">model universes with arrays of rubber balls and oranges (food
for the herbivorous mites) on trays, investigating how spatial heterogeneity and
the varying dispersal ability of each species affected long-term population
dynamics.</cite> The result: <cite index="37-1">in a structurally simple
environment the predators always drove the prey extinct; in a more complex
environment persistence was greatly extended — with barriers to dispersal, the
prey could move to vacant, predator-free patches, staying one jump ahead of the
predator that decimated the patches where it found them.</cite>

Note what it took. Heterogeneity **alone was not enough**:
<cite index="40-1">with predatory mite added both prey and predator go extinct,
despite heterogeneous habitat; coexistence [was only] possible when, in addition
to heterogeneity, temporary barriers to predator migration are
established.</cite> Huffaker <cite index="39-1">reduced the dispersal of
predators by slowing them with petroleum jelly and encouraged dispersal in prey
by providing wooden dowels for long-distance migration.</cite>

And even then it was fragile — three oscillation cycles, in an apparatus designed
specifically to produce them. <cite index="41-1">It seemed important to make the
environment patchy, but not too patchy.</cite>

**Design consequences, stated as rules:**

- **Prey must disperse better than predators.** Not equally. Better. This is a
  designed asymmetry, and it is the single most load-bearing parameter in the
  system.
- **Barriers matter as much as patches.** A heterogeneous but freely traversable
  world is not enough.
- **"Patchy but not too patchy"** — there is an interior optimum, which means
  this needs tuning against a measured persistence metric (§9), not tuning by
  eye.

---

## 3. Discreteness cuts both ways: the atto-fox problem

Continuous models cheat. <cite index="62-1">The density of infected at the place
of origin of the epidemic never becomes zero — it only declines to a minimum of
around one atto-fox (10⁻¹⁸ of a fox) per square kilometre. The model then allows
this atto-fox to start the second wave as soon as the susceptible population has
regrown sufficiently.</cite> Mollison's criticism named this, and it is now the
standard caution: <cite index="55-1">for small populations the model begins to
report the existence of fractional individuals, a biologically meaningless
concept — the "atto-fox problem" — which makes purely deterministic, continuous
models inappropriate for small populations.</cite>

**This engine is individual-based, so it does not have this bug — which means it
does not have this safety net either.** In a cell grid, a species at one
individual is at one individual, and at zero it is gone permanently. Extinction
is an absorbing state. Every Lotka–Volterra intuition about "the population dips
low and recovers" is an artifact of continuous mathematics that will not happen
here.

The published resolution is directly usable. Fowler's treatment suggests
<cite index="63-1">that the modelling resolution is to allow for the existence of
a reservoir</cite> — a refuge population that persists through the trough.

**Recommendation.** Build an explicit reservoir mechanism rather than hoping the
dynamics avoid zero. Options that fit this engine, in order of preference:

- **A physical refuge the predator cannot enter.** Prey that burrow into
  substrate too resistant for the predator to follow, per the existing M18
  burrowing-cost research. This is Huffaker's petroleum jelly, expressed through
  a mechanic the engine already has, and it is the most in-idiom answer by a wide
  margin.
- **A dormant life stage** — eggs, spores, cysts — that survives when the mobile
  form does not. Biologically real and cheap: a `Cell` kind with a timer.
- **Off-screen immigration.** Effective, and the least satisfying. It also
  interacts badly with M10's determinism requirement, since immigration during
  off-camera catch-up must be a pure function of elapsed time.

---

## 4. Mobility has a critical threshold

This is the finding most likely to be surprising, and it is a design parameter
rather than a caveat.

Reichenbach, Mobilia & Frey (*Nature* 448:1046, 2007) studied cyclic
three-species competition on a lattice and found:
<cite index="47-1">when mobility exceeds a certain value, biodiversity is
jeopardized and lost; in contrast, below this critical threshold all
subpopulations coexist and an entanglement of travelling spiral waves forms in
the course of time. We establish that this phenomenon is robust — it does not
depend on the details of cyclic competition or spatial environment.</cite>

The transition is sharp, not gradual: <cite index="44-1">in static populations
the status quo holds, but when mobility exceeds a critical value, biodiversity is
lost. Below the threshold, subpopulations can coexist and biodiversity is
maintained.</cite> Later work in the same family confirms the mechanism:
<cite index="51-1">under high mobility conditions the spatial patterns are
destabilized, leading to a significant decline in biodiversity and the emergence
of single-species dominance.</cite>

The mechanism is the one from §1 — mobility mixes the population, mixing destroys
the spatial decorrelation, and without decorrelation the system reverts to
well-mixed behaviour, which is Gause's result.

### 4a. This collides with an existing M18 design note

`PLAN.md`'s M18 Phase 2 says, correctly for its own purposes:

> **Perception is cheap and unconstrained.** `MAX_REACH` binds CA rules because
> of how waking works; entities are outside the sweep and may read anywhere.

True as an engineering statement, and dangerous as an ecological one. An entity
that perceives arbitrarily far and moves toward what it perceives has *effective*
mobility far above its step size — a predator that can see the whole map has
infinite mobility for stability purposes, regardless of how fast it walks. The
sharp threshold applies to effective mixing, not to velocity.

**Recommendation.** Constrain perception range for ecological reasons even though
nothing technical requires it, and treat perception radius and movement speed as
one combined stability parameter with a measured threshold (§9b). Note that
adaptive movement partially rescues this — later work incorporating
<cite index="46-1">local habitat suitability into the RMF model, where an
individual is more likely to move when the local habitat becomes hostile and is
no longer favourable</cite> changes the picture — which is an argument for the
existing energy-budget/Marginal-Value-Theorem foraging design over naive seek
behaviour.

---

## 5. Enrichment destabilizes, and the rest of the roadmap is enrichment

Rosenzweig's result, from six independent models:
<cite index="70-1">in each case, increasing the supply of limiting nutrients or
energy tends to destroy the steady state — there is a real chance that such
activity may result in decimation of the food species that are wanted in greater
abundance.</cite> Mechanically, <cite index="64-1">enrichment was taken to be
increasing the prey carrying capacity, showing that the prey population
destabilized, usually into a limit cycle</cite>, and
<cite index="67-1">destabilization might lead to extinction, when the limit cycle
is sufficiently large that it approaches zero for one or all species.</cite>

**The direct implication for this project:** every planned improvement is an
enrichment event. Working plants mean more prey food. Fixed water leveling means
more habitable area. Worldgen with a water table and aquifers means a richer
world everywhere. Each of these will increase cycle amplitude, and amplitude
crossing zero is extinction (§3 — with no atto-fox to rescue it).

So the ecology will get *less* stable as the rest of the engine gets *better*,
and the failure will be attributed to whatever shipped most recently. This is
worth writing into `PLAN.md` as a standing note, because it is exactly the kind
of cross-system interaction that produces a week of misdirected debugging.

Fairness note: the paradox is contested empirically —
<cite index="67-1">several experimental studies rejected the hypothesis that the
enrichment phenomenon would destabilize community dynamics</cite> — and the
proposed resolutions include <cite index="71-1">inedible, invulnerable,
unpalatable and toxic prey, ratio-dependent functional forms, inducible defence,
and density-dependent predator mortality</cite>. Several of those are cheap
mechanics worth having anyway. **Density-dependent predator mortality is the
cheapest**: predators that starve faster when crowded, which the existing energy
budget already almost expresses.

---

## 6. Build a cycle, not a chain

The Reichenbach result above concerns *cyclic* three-species competition, and
that is not incidental. <cite index="47-1">Species diversity in ecosystems is
promoted by cyclic, non-hierarchical interactions among competing populations.
In combination with spatial dispersal of static populations, this type of
competition results in the stable coexistence of all species and the long-term
maintenance of biodiversity.</cite>

A linear food chain has a top predator with no check on it except starvation,
which is the Gause configuration. A cycle — A beats B beats C beats A — has every
species checked by another, and coexists on a lattice under conditions where a
two-species chain does not.

**Recommendation for M18 Phase 2's species set:** three creatures in a
non-transitive relationship, not two in a food chain. The relationship does not
have to be predation — competition for substrate, one species' waste being
another's food, or a material-mediated interaction (a creature that hardens sand,
another that only burrows loose sand, a third that loosens it) all work, and the
last is more interesting than "eats" and uses systems that already exist.

This is a bigger design lever than any tuning parameter in this report.

---

## 7. Engine-specific hazards

### 7a. Chunk sleeping is an accidental ecological mechanism

A sleeping chunk's creatures don't tick. That silently creates either a perfect
refuge (a prey population in a quiet chunk is immortal and doesn't starve) or a
silent extinction (a chunk sleeps with a starving population that should have
migrated), depending on how the wake rules interact with creature scheduling.

Neither is intended, both are load-bearing for population dynamics, and both will
be very hard to diagnose because the symptom appears far from the cause.
**Creatures must either keep their chunk awake or have their timers advanced on
wake.** Decide explicitly.

### 7b. Off-camera catch-up under M10

`PLAN.md` requires that off-camera catch-up be a pure function of (state at
unload, elapsed time, seed). Population dynamics is the hardest possible case:
the correct answer after 10,000 elapsed frames requires either simulating them or
having a closed-form model, and the closed-form model is exactly the ODE whose
atto-fox behaviour §3 warns against. A chunk that unloads with two prey and one
predator and reloads 10,000 frames later has no cheap correct answer.

Flagged rather than answered. The plausible resolution is that populations are
*frozen* off-camera rather than advanced, which is defensible if refuges are
physical (§3) but should be a stated decision.

**Answered in `Reports/ecological-lod-design.md`** (recommendation, awaiting
owner sign-off), which takes up the freeze and narrows it: **freeze individuals,
advance the fields and a coarse patch tier, and quantize every population-like
quantity to integers.** Freezing individuals is what makes the atto-fox
impossible rather than merely unlikely; advancing the aggregate tier is what
stops the off-camera world from being paused, which §5e of
`emergent-world-architecture.md` ("nothing reseeds") and its "the world currently
runs down" finding both need.

The reframing that made it tractable: the off-camera world does not owe the
player a *correct* population, only a conserved and plausible one plus a supply
of colonists — and §9f, the criterion this most directly serves, is a statement
about a reservoir rather than about accuracy. That document also argues §7a's
accidental refuge should be made deliberate rather than removed, since a rate
difference across the tier boundary is the dispersal handicap Huffaker needed
and §2 says this engine otherwise lacks.

### 7c. `push_creature` will overflow, and CI will not catch it

```rust
pub(crate) fn push_creature(&mut self, creature: CreatureState) -> u16 {
    debug_assert!(self.creatures.len() < u16::MAX as usize, ...);
    self.creatures.push(creature);
    (self.creatures.len() - 1) as u16
}
```

`creatures` never shrinks — there is no reclamation path, the same gap as
`World::trees` in issue #8 and the unimplemented `free_organism`. With a
*breeding* population this stops being a slow leak and becomes a hard limit on
**cumulative births**, not live population. A modest ecology producing 100 births
a second exhausts 65,535 slots in about eleven minutes.

Three problems compound:

- The guard is a `debug_assert`, and CI runs `--release` exclusively, so it is
  never checked anywhere.
- In release the assert vanishes and `(len - 1) as u16` silently wraps, so
  creature 65,536 *becomes* creature 0. That is not a crash; it is two creatures
  sharing a state slot, which will present as a creature behaving erratically.
- Population dynamics is precisely the feature that makes cumulative births
  unbounded.

**This must be fixed before M18 Phase 2 ships breeding**, and the fix is the same
free-list that `free_organism` needs. Do them together.

### 7d. Per-chunk RNG is a confound here too

~~`Chunk::rng` is seeded from chunk coordinates, so identical creatures in
different places draw different sequences.~~ For population statistics that is
noise correlated with position, which is the kind of thing that manufactures a
spurious spatial pattern. Same recommendation as the plant work: a per-creature
RNG stream seeded from the creature id.

> **CORRECTED 2026-08-28**, on the same finding as
> `plant-simulation-research.md` §7d, and `src/sim/rng.rs:105-117` names this
> file explicitly. **Creatures never touch `Chunk::rng`** — it is reached only
> by the CA sweep via `CellSurface::rng()`. The mechanism was order coupling,
> not position, and the per-organism stream recommended here has shipped as
> `rng::stream`.

---

## 8. Testing an ecology is not like testing a rule

`PLAN.md` already contains the right observation, made about a different system:
"a 3-agent test scene will look broken when the code is correct." Population
dynamics is that problem in its strongest form.

Three consequences for the test strategy:

- **Single runs prove nothing.** Extinction is stochastic. A parameter set with a
  30% extinction rate will look fine three times and then fail in front of a
  player. Acceptance must be over an **ensemble** — N seeds, reporting the
  fraction that persist.
- **Runs must be long.** Huffaker's apparatus sustained three cycles. A test that
  runs for one cycle cannot distinguish "stable" from "about to crash."
- **The metric is persistence, not population.** "Both species alive at frame N"
  across an ensemble is the measurement. Mean population is nearly useless,
  because the mean over runs that went extinct is meaningless.

The ascii harness is the right home for all of this, and it already runs headless
in CI.

---

## 9. Acceptance criteria

- **9a. Persistence.** Both (or all three) species alive at **100,000 frames** in
  **≥ 80%** of 20 seeds. This is the headline number and everything else is
  diagnostic.
- **9b. The mobility threshold is located, not guessed.** Sweep combined
  mobility (movement rate × perception radius) and report the value at which 9a's
  persistence rate falls below 50%. Ship at **no more than half** that value.
  Per §4 the transition is sharp, so a comfortable margin is cheap insurance.
- **9c. Dispersal asymmetry holds.** Prey effective dispersal must exceed
  predator effective dispersal. Assert it as a property of the `.ron` data, so
  a well-meaning tuning change can't silently invert it.
- **9d. Spatial decorrelation exists.** Local population density measured per
  chunk must vary substantially across chunks at any given frame. If every chunk
  has the same density, the system is well-mixed and is running on borrowed time
  regardless of what 9a currently says. This is the leading indicator.
- **9e. Enrichment doesn't kill it.** Re-run 9a with prey food supply doubled.
  Persistence may drop but must stay **above 50%**. This is the regression test
  for §5, and it is the one that will fire when the plant work lands.
- **9f. Refuge works.** With predators removed, a prey population reduced to
  **five individuals** must recover to carrying capacity. Tests §3's reservoir
  directly.
- **9g. No slot exhaustion.** A 100,000-frame breeding run must not exceed the
  live-creature slot budget, and the free-list must return slots. Add a release-
  mode check, not a `debug_assert`.

---

## 10. Deletion tests

| Mechanism | Test that must fail without it |
|---|---|
| Physical refuge / dormant stage | 9f — prey at five individuals goes extinct |
| Dispersal asymmetry | 9a — persistence collapses when predator dispersal is raised to match prey |
| Mobility cap | 9a and 9d — raising mobility past the measured threshold loses a species and flattens spatial variance |
| Cyclic (3-species) structure | 9a — the same parameters as a 2-species chain persist measurably worse |
| Density-dependent predator mortality | 9e — enrichment run drops below the 50% bar |
| Creature slot free-list | 9g — slot count grows without bound over a breeding run |

---

## 11. What was not directly accessible

- **Huffaker (1958),** *Hilgardia* 27:343–383 — read through five secondary
  sources (two encyclopaedia entries, two university lecture notes, and a 2018
  *Ecosphere* paper revisiting the experiment) which agree on the apparatus, the
  petroleum-jelly barriers, the dowels, and the three sustained cycles. The
  original was not read. **The "patchy but not too patchy" interior optimum in §2
  comes from a graduate seminar blog post paraphrasing the paper**, and is the
  weakest-sourced claim in this report; treat it as a hypothesis to test via 9b
  rather than an established result.
- **Gause (1934),** *The Struggle for Existence* — the "extinct regardless of
  initial population size" claim is quoted from encyclopaedia summaries. Widely
  and consistently repeated, but not verified against the original.
- **Reichenbach, Mobilia & Frey (2007),** *Nature* 448:1046 — the arXiv preprint
  and the supplementary information were both read at abstract and figure-caption
  level, including the statement that the phenomenon is robust to the details of
  competition and environment. **The actual functional form of the critical
  mobility M_c and its dependence on reproduction rate μ were not extracted.**
  That is fine for this report, which recommends measuring the threshold
  empirically (9b) rather than predicting it — but if a predicted value is ever
  wanted, the supplementary information is openly hosted and contains it.
- **Rosenzweig (1971),** *Science* 171:385 — abstract read directly; the six
  models were not. The mechanism (enrichment raises carrying capacity, which
  raises limit-cycle amplitude, which can reach zero) is consistent across four
  secondary sources including a full review of proposed resolutions.
- **Cattaneo et al.**, cited in the existing `m18-creature-biology.md` §4, was
  **not re-examined** for this report. §1's correction is directed at the summary
  claim as written in that file, not at the paper, which may well be more careful.
  Worth a check before the correction is treated as settled.

---

## 12. Handoff

**To M18 Phase 2's design:** §6 is the recommendation that should change the
milestone's shape — three species in a cycle, not two in a chain — and it should
be decided before any creature `.ron` files are written, because it determines
what they contain.

**To the plant evolution work:** §5 and §6 are the same findings the plant report
reached from the other direction. Its conclusion that multi-task fitness
landscapes produce diversity while single-task ones collapse to one optimum is
the evolutionary statement of §6's ecological one — non-transitive structure
maintains diversity, transitive structure destroys it. If both a plant ecology
and a creature ecology are built, they should share one persistence-testing
harness (§8) rather than growing two.

**To `PLAN.md`:** §5 deserves a standing note. The ecology becomes less stable
as everything else improves, and that will not be obvious when it happens.

**To M10 and the worldgen work:** §7b is answered in
`Reports/ecological-lod-design.md`, which also takes up §7a's "decide
explicitly" and decides it toward the refuge. Two of this report's findings turn
out to be load-bearing for the streaming architecture rather than for creature
design: §2 (space is the stabilizing mechanism) makes the off-camera world an
ecological component rather than a performance concern, and §7c's slot overflow
becomes a prerequisite rather than a follow-up, because a refuge tier keeps
populations alive that would otherwise have died and so raises cumulative
births.
