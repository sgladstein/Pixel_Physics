# Nest raider (centipede predator) — lane hand-off, 2026-10-27

Thread: cmsg_01Mei3dwkcdzPxa5Jmuqi2bBXGhNsrMxHxZy8iwx34N6pS (child of the lone-predator / spider lane).
The write-up of record is `Reports/nest-raider-2026-10-25.md` on main (rounds one and two).

## Landed
- PR #887 (f3b504d): the centipede, the crumb shove (`digs_inedible_food`), the provision gate for predators with handling time, the lab pool `assets/minds/centipede_pool.txt`, and the report.
- PR #894 (f0a9798): report correction.
- PR #927 (825022e): predtrain `nestpay=K`, the nest smell (`PIXEL_PHYSICS_NEST_SMELL=on`, off by default because of a measured lab harm), and spiderlab `nest_kills` plus `PP_KILLLOG`.

## Open PR
- **#951**, branch `claude/project-thread-2c5af2`, head 2448e015: data only, no code. It saves the raw data behind the report into `Reports/nest-raider-2026-10-25/` (pools, lab grids 1/2/4, training and judge logs, driver scripts, stills). It has been handed to the Merge desk. No other unmerged branches.

## Open to-dos
None are required. Ideas the owner was offered but has not taken up:
- Test the raider against the c13 trained ant's open-room nest in a lab box. Every lab number so far is against the hand-built ant (lab_mid_fruit).
- Select raider pools in the lab rather than on training beds: `spiderlab`, long runs, scored on deep kills from `PP_KILLLOG`. Bed and lab rank predators the opposite way round.
- Training the ants to defend the nest belongs to the Teacher lane.

## Pending owner decisions
None. Both decision cards were answered or withdrawn.

## Files outside the repo
- `/mnt/project-files/predator/nest-raider/*.gif`: seven films, about 30 MB in total (lab-boom-seed3, lab-boom-seed3-wholebox, lab-coexist-seed2-100k, lab-deep-raid-seed3, lab-smell-door-seed2, raider-at-lab-door, raider-in-larder). Not committed because of their size; the stills are in the repo. Everything else in that folder is copied into #951.
- Pictures page: https://claude.ai/artifact/3mqNqp9SDTs5aSSDBwn653 (artifact on the old account).
- Session scratchpad (np/, grid2-4/, tr/): the per-round pool directories of intermediate training rounds. Anything worth keeping is in #951. The scratchpad will be lost with the session.

## Jobs running
None.
