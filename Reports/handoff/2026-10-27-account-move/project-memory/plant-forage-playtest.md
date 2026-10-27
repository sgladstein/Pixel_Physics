---
name: plant-forage-playtest
description: Owner's 2026-10-25 plant-foraging lab playtest: trained ant's bite rate ignores hunger, founders starve by plants, fat ants never go home; replay tool
metadata:
  type: project
---

Thread "Analyze far64 lab playtest" (same thread as [[far64-lab-playtest]]), 2026-10-25. Owner build f367374f, hand-built box with planted tree/shrub/herbs/creepers, 5 colony drops, one food paint. Report + data: Reports/brain-training/playtest/plant-forage-lab-2026-10-25/ (PR #890, MERGED e88b2e4 2026-10-25).

- **Trained ant's grazing ignores hunger**: bite rate beside a plant ~5% of 25-frame windows at every energy (4.4% starving, 4.8% at 10+ tanks). Bite ~11.7 energy. Founders (1 tank) starve beside plants (median life ~1,100).
- Lucky ones gorge: 55,029 windows >2,000 energy vs old ant 3,078; 49 died old age at ~22 tanks; home 0.4-2% of time, ~never laden at home.
- Trail A strongest at plants (41.6k vs 35.5k home) because the colony lives there.
- Founders heaped at door (pile): more home time, same life. Spread is not the cause.
- Old ant same box: founders live 4-13x longer, 165 adults vs 11 at F480k. Box is survivable; brain fails.
- Suggested to Teacher lane: training case with plants, no pile, founders on 1 tank, scored on survival + food carried home.
- Tool: `examples/replay actions=CHRONICLE.actions.csv newant=1|0 trace=25` rebuilds a playtest; census matched owner exactly to F40k. Always run newant=0 beside it.
- Chronicle now logs ANT BRAIN: NEW/OLD after rebuild (PR #890, merged).

**Why:** next plant/forage session shouldn't re-derive this. **How to apply:** judge grazing changes by bites per hunger band, not bite totals; see [[teacher-lane]], [[plant-food-lane]].
