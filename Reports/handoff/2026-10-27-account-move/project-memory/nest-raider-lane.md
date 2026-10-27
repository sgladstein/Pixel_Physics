---
name: nest-raider-lane
description: Nest raider (centipede predator) thread 2026-10-25 — what shipped in PR #887, what it found (merged)
metadata:
  type: project
  modified: 2026-10-25T04:59:13.642Z
---

Thread cmsg_01Mei3dwkcdzPxa5Jmuqi2bBXGhNsrMxHxZy8iwx34N6pS (child of [[predator-lane]]). PR #887 merged to main as f3b504d (2026-10-25 05:23Z). Correction PR #894 merged as f0a9798. Open offer to owner: test the raider vs the c13 trained ant's open-room nest in a lab box.
Write-up: Reports/nest-raider-2026-10-25.md; pictures /mnt/project-files/predator/nest-raider/.

- Centipede: Chain(5), 1 cell wide, spider's plate/gut/handling; jaw can't cut soil, enters by the door. On lab COLONY chip; pool assets/minds/centipede_pool.txt (trained line hunter-2 r7).
- Trained std_pool nests are packed solid with crumbs (no tunnels), so the raider got `digs_inedible_food` (shoves inedible food, keeps its worth). Owner chose YES (2026-10-25 05:09Z): the raider shoves through crumbs.
- Default-on rule change: a species with handling can't pay a birth with living prey (provision_at). Lowers spider breeding too.
- Trap: any mind with Put Down can farm food_log swallow credit (7,300 swallows vs 37 bites) — pay/judge on digest_log.
- Result: training didn't beat the untrained hunter; the lab pool makes populations (4 of 6 boxes, eats colony 2 of 6) from the surface at the far east wall (cols 428-507, 170-250 from the door), eating far foragers early; in long runs the crowd DOES raid deep (round 2). (First write-up wrongly said 'heaps by the door' -- those were fruit bushes; correction PR #894.) Pictures page: https://claude.ai/artifact/3mqNqp9SDTs5aSSDBwn653 Next: pay in-nest kills, keep elitism.

**Why:** later predator/defence work starts from here. **How to apply:** don't re-derive the larder-nest finding or the credit trap; the raid is a crowd's: judge predators in long lab runs, not on short beds.

**Round 2 (2026-10-26, PR #927 merged 825022e):** predtrain `nestpay=K` pays in-nest KILLS (bites are farmable: life.bites = every crop pickup). Nest smell `PIXEL_PHYSICS_NEST_SMELL=on` (OFF by default, measured harm): wins on training beds (23/24) but in the lab deep kills 538 (shipped, smell off) vs 78 on. The shipped raider DOES raid deep once its crowd builds (seed 3 film). Lesson: bed and lab rank predators oppositely -- judge in the lab, long runs, deep kills (spiderlab PP_KILLLOG). Data scratchpad np/, grid3/, grid4/; /mnt/project-files/predator/nest-raider/labgrid4.
