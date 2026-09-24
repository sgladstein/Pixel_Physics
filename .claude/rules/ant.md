---
paths:
  - "src/sim/creature.rs"
  - "src/sim/brain.rs"
  - "src/sim/pheromone.rs"
  - "assets/species/ant*.ron"
  - "assets/species/ancestor.ron"
  - "examples/trailfollow.rs"
  - "examples/onetrail.rs"
  - "Reports/ant-*.md"
  - "Reports/pheromone-*.md"
  - "Reports/what-controls-creature-movement-*.md"
  - "Reports/lanes/homing-return-arm.md"
  - "wiki/ants.md"
---

# The ant: reason from the living reference

**`Reports/how-the-ant-works.md` is how the shipped ant works now**: the tick
order, every wired sense, the brain's wiring, the step and the tumble, the
trail planes, the crop, and every laden-versus-empty difference. It is
written from the source and kept current.

- **Reason from it, not from a dated report.** Each dated ant or pheromone
  report describes the ant as it was when that report was written, and the
  line has lost days to decisions built on an ant that no longer existed.
  If a report and the reference disagree, read the code, then fix whichever
  one is wrong.
- **Change anything it describes? Update it in the same commit.** Edit in
  place, with no history, measurements, or questions and answers; those
  belong in dated reports. Found it wrong? Fix it, and say so in the commit
  message.
- **Re-checked a section against the code? Bump its "Verified against"
  line.**
