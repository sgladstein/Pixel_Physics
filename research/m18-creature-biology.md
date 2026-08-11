# M18 research: animal behavior science, for scientifically-grounded creature mechanisms

Raw findings kept in full here because `PLAN.md`'s M18 section only carries
a condensed synthesis. This is the source material to build the actual M18
Phase 1 (cell-based creatures) implementation against.

Scope: Phase 1 only — a creature as one simulated cell with a behaviour,
scheduled periodically (not every frame), example creatures being a worm
that burrows through powder and slime/fungus-like spreaders. Phase 2 (full
Reynolds-steering entities, after M8) is out of scope for this research;
that citation is already settled in the plan.

---

## 1. Burrowing locomotion mechanics

Earthworms burrow via **peristalsis**: circular/longitudinal muscle waves
alternately elongate and radially expand body segments, wedging into pore
spaces and compacting/anchoring against surrounding grains via hydrostatic
(coelomic fluid) pressure. Critically, peristaltic burrowing **only works
within a narrow band of substrate mechanical resistance** — it requires a
granular medium that (a) is displaceable/compactable ahead of the expanding
segment and (b) flows back in behind the worm to fill the vacated space.
Direct force measurements show this fails outside that resistance range —
too loose and there's nothing to anchor against, too resistant
(compacted/cohesive/solid) and the worm cannot generate enough radial
pressure to deform it (Kurth et al., *J. R. Soc. Interface* 2018; *J. Exp.
Biol.* 2018).

This is a clean, simulatable basis for "can burrow through loose sand,
cannot burrow through solid stone": gate burrowing by a
substrate-resistance/compaction threshold, not material identity by name.
Moisture and grain size modulate that resistance (Britannica; Jayne lab work
on fossorial lizards).

Energetics scale sharply with substrate: the Namib golden mole spends **~26x
more energy per metre burrowing through loose sand (80 J/m) than running on
the surface (3 J/m)**. A real, citable number to drive a movement-cost
model, not an invented multiplier.

**Simulation translation:** tie burrow cost/speed to the target material's
own already-tracked physical properties (density, friction angle) rather
than a hardcoded material-kind whitelist, so the rule generalizes to future
granular materials automatically. Apply a real energy-cost multiplier
(order of magnitude ~10-30x ground movement cost) rather than a flat
"slower" tweak.

Sources:
- [Biomechanical limits to soil penetration by earthworms (J. R. Soc. Interface, 2018)](https://royalsocietypublishing.org/doi/10.1098/rsif.2018.0127)
- [Kinematics of burrowing by peristalsis in granular sands (J. Exp. Biol., 2018)](https://journals.biologists.com/jeb/article/221/10/jeb167759/19480/Kinematics-of-burrowing-by-peristalsis-in-granular)
- [Earthworm burrowing modes and soil mechanical resistance (ScienceDirect)](https://www.sciencedirect.com/science/article/pii/S0929139322001846)
- [How head shape and substrate particle size affect fossorial locomotion in lizards (J. Exp. Biol.)](https://dx.doi.org/10.1242/jeb.242244)
- [Fossorial locomotion — Britannica](https://www.britannica.com/topic/locomotion/Fossorial-locomotion)

## 2. Heat/fire sensing and avoidance

Real animals use tiered sensory cues at very different ranges: olfactory
(smoke) > auditory > visual/infrared, with the "smoke detector principle"
explaining why animals over-respond to weak cues rather than risk missing a
real fire. Fire beetles (*Melanophila*) detect fires via infrared-sensing
pit organs at ~5 km (extreme claims to 130 km via smoke). At close range,
reptiles reflexively flee radiant heat from flames at distances up to ~30 ft.

For a cell-based, local-temperature-field sense (much closer to this
engine's actual design than the beetle's long-range infrared), ***C.
elegans* thermotaxis** is the best-studied simple mechanism: a single
thermosensory neuron (AFD) compares current temperature to a remembered
set-point and drives movement down the gradient when above it (cryophilic).
Directly analogous to "read the local ambient-temperature field, flee
down-gradient once above a threshold" — and notably, this is *both* the
scientifically grounded version *and* the cheap one; no need to invent
something more complex than what a real 302-neuron animal actually uses for
this exact behaviour.

Sources:
- [Animal Functional Traits Associated with Fire Sensitivity (MDPI Encyclopedia)](https://encyclopedia.pub/entry/46082)
- [In Case of Fire, Escape or Die: trait-based approach (MDPI, 2023)](https://www.mdpi.com/2571-6255/6/6/242)
- [Integrating sensory ecology and predator-prey theory for fire response (Ecology Letters, 2023)](https://onlinelibrary.wiley.com/doi/10.1111/ele.14231)
- [Animal response to a bushfire (The Conversation)](https://theconversation.com/animal-response-to-a-bushfire-is-astounding-these-are-the-tricks-they-use-to-survive-129327)
- [Are Animals Afraid of Fire? (Berry Patch Farms)](https://www.berrypatchfarms.net/are-animals-afraid-of-fire/)
- [Bidirectional thermotaxis mediated by AFD neurons (PNAS)](https://www.pnas.org/doi/10.1073/pnas.1315205111)
- [Neural regulation of thermotaxis in C. elegans (Nature, 1995)](https://www.nature.com/articles/376344a0)

## 3. Foraging as a reason to move

Optimal Foraging Theory frames movement as a cost-benefit optimization:
animals should maximize net energy gain, weighing energy/time spent
searching against energy obtained. The **Marginal Value Theorem**
specifically predicts patch-leaving behaviour: leave a patch once its local
intake rate drops below the environment's average intake rate.

**Simulation translation:** give a creature an internal energy/hunger stat
that depletes over time, is satisfied by consuming material as it moves
(e.g. the worm burrowing through/consuming organic content in sand), and
triggers directed movement toward higher local resource density with a
leave-threshold. Replaces "wander randomly" with a real behavioural-ecology
model at negligible extra simulation cost — this maps cleanly onto a worm
needing an internal energy/hunger stat, exactly the same shape as the plant
root's water-deficit mechanic in the M16 research.

Sources:
- [Optimal foraging theory — Wikipedia](https://en.wikipedia.org/wiki/Optimal_foraging_theory)
- [Optimal Foraging Theory: An Introduction (CEC)](https://www.cec.org/files/sem/20231030/aaa003.pdf)
- [Foraging Ecology — Biology LibreTexts](https://bio.libretexts.org/Workbench/General_Ecology_Ecology/Chapter_11:_Behavioral_Ecology/11.2:_Foraging_Ecology)

## 4. Predator-prey / resource-competition dynamics

The **Wa-Tor model** (Dewdney, *Scientific American*, 1984) is the most
directly relevant prior art: a toroidal grid where prey move/breed on timers
and predators move toward prey, eat, gain energy, and starve without food —
essentially a discretized Lotka-Volterra system built exactly for grid cells
with periodic behaviour ticks, matching this engine's active-site-scheduler
architecture closely. Later CA variants (Cattaneo et al.) show these
grid-local rules reproduce real predator-prey population oscillations more
robustly than the continuous ODE version.

Sources:
- [Wa-Tor: A Predator-Prey simulation (beltoforion.de)](https://beltoforion.de/en/wator/)
- [A full cellular automaton to simulate predator-prey systems (Cattaneo et al.)](https://web2.qatar.cmu.edu/~gdicaro/15382-Spring18/additional/prey-predator-CA-2006.pdf)
- [Self-organized patterns of coexistence out of a predator-prey CA (arXiv)](https://arxiv.org/pdf/q-bio/0604030)

## 5. Adjacent prior art for simple grid creatures

**Braitenberg Vehicles** (1984) — minimal sensor-to-motor wiring producing
complex-looking behaviour from tiny rule-sets. A good design philosophy for
keeping each creature's rule-set small rather than over-engineering
individual behaviours.

**For the slime/fungus creature specifically:** *Physarum polycephalum*
foraging-algorithm models (Jeff Jones, 2010) and fungal-mycelium CA growth
models (nutrient uptake and translocation on a lattice) are direct
grounding — both are literally grid/network growth-and-pruning driven by
local nutrient gradients. This is the *same shape* as the plant root
mechanic from the M16 research (consume local resource, propagate deficit,
grow/prune toward gradient) — worth building as one shared primitive that
serves roots, fungus, and slime creatures, rather than three bespoke
mechanisms.

**Noted but explicitly not applicable:** Lenia/SmoothLife are continuous-field
artificial life, not discrete per-cell creatures — a different problem class
from Phase 1's cell-based creatures and not worth pulling from for this
milestone.

Sources:
- [Slime Mould Foraging: an inspiration for algorithmic design](https://ncra.ucd.ie/papers/IJICA_Slime_Mould.pdf)
- [physarum simulation — Sage Jenson](https://cargocollective.com/sagejenson/physarum)
- [Cellular automata simulations of fungal growth on solid substrates (PubMed)](https://pubmed.ncbi.nlm.nih.gov/14545682/)
- [Modelling mycelial networks in structured environments (Boswell)](https://www.davidmoore.org.uk/21st_Century_Guidebook_to_Fungi_PLATINUM/REPRINT_collection/Boswell_modelling_mycelial_networks2008.pdf)
- [Braitenberg Vehicles as Computational Tools for Neuroscience (Frontiers, 2020)](https://www.frontiersin.org/journals/bioengineering-and-biotechnology/articles/10.3389/fbioe.2020.565963/full)
- [Lenia and the Continuous Revolution in Artificial Life](https://life.angen.ai/blog/lenia-and-the-continuous-revolution-in-artificial-life)

---

## Summary: what this means for the M18 build

1. **Burrowing gated by substrate resistance** (density/friction-angle
   derived), not a material-kind whitelist — and a real ~10-30x energy-cost
   multiplier for moving through granular material vs. open space.
2. ***C. elegans*-style thermotaxis** for fire avoidance: compare local
   temperature-field reading against a remembered/threshold set-point, flee
   down-gradient once exceeded. This is simultaneously the accurate model
   and the cheap one.
3. **Energy/hunger stat driving movement** (Marginal Value Theorem-style
   patch-leaving), replacing random wandering with foraging — shares its
   basic shape with M16's root water-deficit mechanic.
4. **Wa-Tor-style rules** if/when multiple creature kinds interact
   (predator moves toward prey, eats, gains energy; prey moves/breeds on a
   timer; starvation without food) — close to exact prior art for this
   engine's scheduler shape.
5. **Shared "consume local resource, propagate deficit, grow/prune toward
   gradient" primitive** for slime/fungus creatures, roots, and potentially
   moss — one mechanism, three uses, rather than reinventing it per system.
