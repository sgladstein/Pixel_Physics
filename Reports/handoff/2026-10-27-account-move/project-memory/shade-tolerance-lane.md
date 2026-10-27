---
name: shade-tolerance-lane
description: A5 shade-tolerance price lane (2026-10-22): sun scald behind World::shade_scald, Station::SHADE, training runs, branch claude/project-thread-fprel1
metadata:
  type: project
  modified: 2026-10-22T14:07:59.438Z
---

"Plant shade tolerance" thread (Opus lane split from [[plant-community-lane]], 2026-10-22). Branch `claude/project-thread-fprel1`.

- Why an income price failed: a lone plant in bright light is sized by development, not carbon, so `SHADE_LEAF_PRICE` (0.9) never bites; trained genomes picked up tolerance for free (seed-trained herb 0.53, shrub 0).
- Built: **sun scald** in `organism_upkeep` (plant.rs): tolerant leaves shed at `SCALD_RATE(16) * shade_death * tolerate * glare^3`, glare vs `World::open_light`. Switch `World::shade_scald` (off by default; coordinator asked off until owner sees it), env `PIXEL_PHYSICS_SHADE_SCALD=1` for trainer beds, `PIXEL_PHYSICS_SCALD_RATE` sweep, `DeathCause::Scorched`, `World::shed_scald`. `Station::SHADE` (lamp 0.2) in train_plant/arena.rs (agreed with plant lane).
- Numbers (lone shrub tol 0.7, 16 seeds, 60k): bright smaller 13/16 at rate 16 (0.74x); canopy shade (lamp 0.2) 5/16 alive vs 0/16, scald never fires. Creeper bright 16/16 smaller. Dim 0.4: tolerance neutral.
- Trap: trainer `rung=life` IGNORES `stations=` (grows in bright `bed`). Use `rung=hold neighbours=tree stations=bright,shade` or `rung=grow`. bed_score can't see size in bright anyway.
- Canopy judge: plant lane's stand probe `plant_m0 mode=stand` (patch /mnt/project-files/plant-community/runners/stand.patch, worktree /tmp/claude-0/wt-stand).
- Runner script: /mnt/project-files/shade-tolerance/runners/run.sh NAME SPECIES STATIONS SCALD SEED.

- 2026-10-22 ~15:40Z: PR #818 with merge desk (all switches off, no result change). Added `World::seedling_waits` (off). Stand judge 12 seeds: shrub/creeper 0/12 alive in every arm (tolerance 1.0, waits on/off); floor 2-3% open light. Seedling walls: germination light_threshold 0.1 absolute -> tip gives up (waits fixes part) -> first leaf unaffordable + root unpaid. Trainer picks free photosynthesis rate over tolerance. Lane CLOSED; A5 handed back to Plant community plan. Owner chose "Fix the shade seedling" (15:31Z); forwarded to Plant community plan, which owns it now.

**Why:** later sessions need the price design and traps without re-deriving.
**How to apply:** check the thread/PR state before citing; nothing lands that changes results without owner pictures+numbers.
