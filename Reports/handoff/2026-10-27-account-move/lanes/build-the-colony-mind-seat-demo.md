# Lane: Build the colony mind seat demo (2026-10-25)

**State: finished; no work open.** Everything is on main: PR #905, merged as 8329b37 on 2026-10-25.

- **Result:** there is no standalone colony mind demo on today's trained ants. The record is
  `Reports/teaching-the-colony-2026-10-25/colony-mind-verdict.md`, plus the notes in its `colony-mind/` folder:
  the feasibility review, the step-0 measurements and their correction, plan v1, the plan review, and the
  `demo_mind.ron` stake bed. There is also an entry in `Reports/dead-ends.md`.
- **Tools on main:** `examples/mindprobe.rs`, a paired verb probe that compares a played run against an unplayed
  twin; `scripts/mindprobe_cmp.py`; `scripts/mindstake.py`. Two hooks are on main and off by default:
  `Minds::closed_stomach` (closes the shared stomach) and `World::scale_pheromone` (lowers one scent cell).
- **Owner decision still pending:** a card asking "Stop the standalone colony mind demo and fold its parts into
  other seats?" got no answer. My recommendation was to fold it in. The close-the-stomach verb would go to the
  keeper seat, where it gates "a place spreads by meals". Favour would become a use of the queen seat's who-lays
  store. The other two options on the card were "build a small labelled toy anyway" and "wait until the Teacher
  lane trains an ant that reads a returning-forager sense".
- **Lessons:** these are already in the repo, in `how-we-test.md` §4 and `how-the-ant-works.md` §13.
  - Check any player-verb claim against the trained ant. A mind ant skips `act()` and the hand-built walk.
  - Score food brought home with `trip_deliveries`. Never use `MindCounts::delivered`.
- **To-dos:** none.
- **Files outside the repo worth keeping:** none. The `/mnt/project-files/seat-demos/colony-mind/*.md` files are
  identical to the copies in the repo. The raw probe outputs were scratch, and every number re-runs from the
  commands in `step0-measurements.md`.
- **Jobs running:** none, on the VM or anywhere else.
- **Branches with unmerged work:** none. `claude/project-thread-6xokl9` was fully merged by #905.
