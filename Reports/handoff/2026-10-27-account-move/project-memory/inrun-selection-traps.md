---
name: inrun-selection-traps
description: Mistakes already paid for in the teacher lane's in-run selection harness (2026-10-21); read before running or changing it
metadata:
  type: project
---

**Traps already paid for:**
- `slay(x,y)` kills whoever owns the cell; ants overlap. Cull by id (`cull_organism`). Runs before 18:45Z void.
- train::mutate prune 0 grow 1 bloats links to ~2,200 and ants starve. Use `mutation=neutral`.
- Score farming (found 21:00Z, p11): ants cut surface soil 20-40 cols out and dropped it beside the hole. Fix `nest_r=20`: only cells cut within 20 cols of nest pay dig/clear. p5-p11 spoil numbers are inflated by this.
- Pellets dropped were handled in HashMap order (`held.keys()`): 2 of 64 seeds diverged between runs. Fixed by sorting (ea0f40e6, 2026-10-22). Any new loop over a HashMap that touches credit or counters must sort first.
- Never `pkill -f` or grep-kill bash PIDs (kills own shell). index.csv is HashMap-ordered: sort before carrying forward.

- Adversarial review of the design doc (2026-10-22, /mnt/project-files/teacher-lane/design-doc-adversarial-review-2026-10-22.md): the food log books EVERY trip-cell put-down as a delivery (6% lay 13+ cols out); home-meal and hungry-departure pay need no field bite; clear pay peaks at 25 cols = the food scatter; ~220 free-tank newborns per colony-round prop up "survival" (judge births off: h1 loses 11/24 a round, old ant 0); many arm calls were inside seed noise (use paired McNemar, review/tools/pair.py); armsum pools food over unequal round counts. `strict=1` in inrun.rs closes the first three holes plus per-cell progress take-back (off by default, strict=0 byte-identical).
- The mind DOES read trail direction (options.rs f::SCENT..+8: there, there-minus-here, two-cells-minus-here), per the maze thread.

- The game's "home" (creature.rs adjacent_nest, NestHome::Dug default since 2026-10-02) is the open space joined to the door below the old ground line (world.rs step_nest_dug, reach 60 cols, every 256 frames). The in-run score instead uses column bands (dig <=20, home <=12). Seed-3 side pit at 138-145 is NOT home (4-cell soil wall). Owner proposed 2026-10-22 a never-shrinking dug nest as the score's only dig/food zone; debated on the replay page; owner handed it to the dig lane (05:03Z), teacher lane does not build it (dig lane: `nest=zone nest_pad=2 nest_top=3`). Owner 05:10Z: buried food and the nest creeping toward the food are NOT problems; soil in the top 3 rows never counts as nest except the stamped mouth; extra starvation is acceptable if it buys a clear fix that foraging can be trained on top of (feeding gate is not an absolute veto).

**Why:** each cost a batch of runs or voided results.
**How to apply:** check these before trusting any in-run number; see [[inrun-selection]].
