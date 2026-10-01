---
paths:
  - "src/sim/plant.rs"
  - "src/sim/organism.rs"
  - "assets/species/**"
  - "examples/plant_probe.rs"
  - "examples/thicket_probe.rs"
  - "Reports/plant-*.md"
---

# Plants and organisms: which pixels a lever moves

Moved out of `CLAUDE.md` on 2026-09-30. Worked case:
`Reports/claude-md-evidence-2026-09-30.md`, *Ask which pixels a lever moves*.

**Ask which *pixels* a lever moves, before ranking it by silhouette.**
Sympody, tropism and acrotony all fired (46–186 forks, 1,797–2,750
plagiotropic steps) and the owner saw no change, because all three only change
**which cell gets a label**, while the silhouette was set by material mix and
palette: every species was ~90% wood and ~5% leaf, from one shared palette. A
lever that relabels a cell cannot move a silhouette that texture and colour
set. See `Reports/plant-appearance-design.md`.

**A threshold on light must divide the day out**: use
`field::noon_equivalent_light`, or a night-time sample makes a nightly
extinction event (live tips 71 at noon against 28 at night).
