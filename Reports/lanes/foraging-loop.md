# Lane note — the ant's foraging loop

*Kept current, edited in place. History and measurements live in
[`../ant-scenes-2026-09-23.md`](../ant-scenes-2026-09-23.md) (§1–§22); how the
ant works now lives in [`../how-the-ant-works.md`](../how-the-ant-works.md).
This note keeps the owner's rulings, the live question, the baseline, the
commands and the traps.*

- **Previous session:** `session_01Pt5N39pfcix13hMycPN9Xs`, branch
  `claude/ant-foraging-loop-handoff-986v7n` (09-28/30): §22o-§23c. **It ended
  on a handoff (owner's usage, 09-30):** everything it needed is committed;
  `runs/` in a container is not, so rebuild from `Reports/data/`.
- **Peer lanes:** the nest-mouth lane ([`nest-mouth.md`](nest-mouth.md))
  shipped the granary (#513, 2026-09-29: a door, a storeroom, nest workers,
  `keep`). Every baseline before it is a different ant; its `keep` and
  nest-bound drive sit in this lane's region, reviewed and agreed.

## Standing owner rulings

- **The goal:** foragers earn enough to feed themselves *and* extra for the
  colony (2026-09-25). "All we care about is the loop is improving."
- **The colony bed decides; the lab box is the pre-ship check** (09-28,
  replacing "test in both games before any ruling"). Develop and rule on
  loop changes on the bed (`trailfollow`) at 90 *and* 140 cells, traced.
  Before a change ships, run the lab (`labforage`, 24 seeds) once as a
  regression check, read with `scripts/labpair.py`: births, food eaten,
  ant-frames, **starved per million ant-frames** (died out and alive at the
  end time the grazing crash; they are not the gate). A change that acts at
  home also gets one bed pair at `STACK_DEPTH=1`, the game's cap (the bed
  runs 4); only an opposite sign at p<0.05 counts against it. It is not
  evidence the loop improved: it came back neutral on the drive, carry
  patience, packed lunch and the door, and costs ~25x the bed per arm. What
  it caught was harm the bed cannot contain: the birth overdraw (Z36, seeds),
  the forage drive's early crashes (§22h), the painted door's lab loss (§19,
  gone on the 09-28 ant).
- **Lead with the specific quantity a change targets**; colony totals
  (starved, net food into home) are the check, not the headline (09-26).
- **Post images and questions in the chat, not the review queue** (09-29:
  the owner cannot reach it). Label each image; ask in words.
- **Features default on unless there is a good reason not to** (09-27,
  replacing "never flip a default without the owner's ruling"). A good reason
  is a measured harm; neutral ships on. Defaults on the owner's word before
  that: the `trailaway` walk (09-24), crop 5,760 J with food at half weight
  (09-25), scouting at gain 2 (09-26).
- **"Scout or any ant should come home when they get so hungry before they
  are going to starve to death"** (09-26). `PIXEL_PHYSICS_HUNGRY_HOME`, off:
  it helps at 90 cells and kills at 140 whatever home holds (§21, §22g).
- **Agents: judge each situation, use them where they help, never
  wastefully** (09-28; no default count, and not scale for its own sake
  under a thoroughness mode; `agent-strategy.md` s4).
- **Sub-agents allowed** (09-27): in-process for reading and surveys (not
  runs: 4 cores); a cloud session for a long run, via `lab-coordinator`
  with `model:` set. The return path is files.
- **A nest step is not blocked on colony numbers** (09-28): the nest lane
  ships a nest change that costs the loop, and this lane answers the cost.
- An ant should breed only at the nest, and in the end *where* it breeds
  should be something a lineage evolves (09-23). `PIXEL_PHYSICS_BUD_SITE=nest`,
  off by default; the bed runs with it on.

## Live question

**Shipped on** (`=off`, or `face`, `always`, is the ant before it):
`FORAGE_DRIVE=returns`, `TRIP_REACH=on` (16), `CARRY_PATIENCE=pickup` (Z35), `PACKED_LUNCH=on`,
`BIRTH_PRICE=guaranteed` (Z36), `FOOD_TRAIL=lay` (09-30) -- all
`PIXEL_PHYSICS_`; and the nest lane's granary (#513). **`STORE_LUNCH` is
off** (§22t).

**Which way to go is the loop's blocker (§22t).** The food-trail plan
(`food-trail-plan-2026-09-29.md`, owner-agreed 09-29), bar = `mute`.
**Stages 0-1 done (§23a, §23c):** only a trip load lays B, shipped on (#523);
it beats `off` and `mute` on every bed. **Its cost:** hungry ants at home
off any trail sit longer (terrain 8/12, lab starved rate 16/24, neither
significant): the reader must give them a reason to leave. `mute` is
heritable since 09-30 (`World::mute_emit_b`), so the control no longer leaks.

**Next, in order (start here):**
1. **B5 is built** (#526, 2026-09-30, `pile2=west`; numbers in its commit).
   Oracle beats mute 21/3, so the bed can reward a trail; oracle beats self
   18/6 on the take in the first 1,500 frames after a swap, which is the
   reader's headroom. **P4.3 failed: east phases take ~2x west in every
   arm**, not the storeroom and not the founding. At 80 founders the
   ants that never reach the food walk west and die there (135 cells, seeds
   1-8). Trace the east/west lean ant by ant before building the reader.
2. **Stages 2-3 from `food-trail-reader-design-2026-09-30.md`**: fix its
   eight must-fix defects first, port `giveup.py`/`departures.py` (in
   `data/food-trail-lay-2026-09-29.tar.gz`) into `trailclimb.py` §6, register
   §23d, then build `read`/`giveup` behind `FOOD_TRAIL`.
3. Then the plan's Stage 5-6 and the genome follow-up. Store lunch waits.

**To the nest lane (09-30):** `FOOD_TRAIL=lay` is on (#523). Keep
`trip_load` and the door's geometry as they are, and poke first: the
reader will read B across the door.

- **Read food taken and food at the nest** (`FOOD STORE`'s `nest food`,
  mean from 6,000); "net food into home" overcounts (§22j).

## Baseline (`FOOD_TRAIL=lay` on, 09-30; `off` in brackets, #518)

Colony bed, 24 seeds, shipped defaults, unlimited pile:

| | food taken from the pile | food at the nest | starved | born |
|---|---:|---:|---:|---:|
| 90 cells | 9,217 (6,062) | 15,463 (12,414) J | 14 (57) | 416 (188) |
| 140 cells | 6,374 (4,287) | 11,048 (8,112) J | 50 (127) | 157 (63) |
| 80 founders at 135 | 12,350 (8,298) | 10,549 (9,159) J | 645 (1,109) | 48 (74) |

Pulsed pile at 90 (`food=30 refill=6000`): 2,760 / 250 / 54 (taken,
starved, born). Runs: `food-trail-lay-2026-09-29.tar.gz`. Lab box
(`played_bed`, 120,000 frames, 24 seeds, rain), medians: births 448.5,
food eaten 1,160k J, ant-frames 9.8M, starved per million ant-frames 13.1.

## Ranked open problems

1. **Which way to go.** A driven forager has little bearing: west
   departures fell 23% -> 18% with `lay` (§23c) and none reach food. The
   anchor re-anchors on every nest contact, so `HomeAligned` reads 0 at the
   door. Stage 2, the reader, is the answer being built.
2. **The drive's clock hears of food only at the door** (§22u): a false
   stand-down on a paying pile, and most refills start with the drive down
   (0.613 against 0.936; lower on 57 of 71). A longer window fixed the bed
   and leaned worse on every lab gate.
3. **Early deaths.** Founders that never reach the food die early (35% of
   the starved on the granary at 90 cells). Only the road and the nest
   (§17b, §19), or a founding store, reach them.
4. **Each ant fetches less in a bigger colony** (nest lane,
   `nest-colony-size-2026-09-28.md`): 20/40/80/200 founders starve 13% /
   28% / 42% / 52%; on the granary 80 founders starve 55% of the ants that
   lived. Trace it with the funnel: where do the extra ants stop?
5. **Food at home to the hungry.** The tether pays at 90 and kills at 140.
6. **Latent:** a scout that gave up is released only at the nest.

## Tools and skills (use these; the names do not say what they answer)

- **`funnel` skill** before investigating anything; when a change is
  visible, show it in the chat (ruling above).
- **`scripts/antloop.py`**: the loop ant by ant (funnel, who starved and
  where, time budget, economy). `--vs base.log` pairs the checks by seed.
- **`scripts/antidle.py '<glob>' <gap> [--vs '<base glob>']`**: do foragers
  keep foraging -- loops per forager, waits, a forager's life after its first
  loop (fed/hungry x home/out, food held off the nest). `--vs` pairs these,
  the target quantities, by seed. **Give it the gap**; 90 is the default.
- **`scripts/labpair.py dir base new`**: the lab box's **pre-ship regression
  check** (`played_bed`, 120,000 frames, 24 seeds), paired by seed. Gate:
  births, food eaten, ant-frames, old age, **starved per million
  ant-frames** (raw starved beside it). Died out / alive at the end are
  **crash timing, not harm** (the box peaks ~90,000 and grazes out), in their
  own block; net food into home is last, an overcount (§22j).
- **`trailfollow decisioncsv dtag=`**: the per-decision trace. Since 09-27 it
  carries `energy_j` and `drive`,
  `scout_w`, `scout_patience`, `scout_home`; since 09-29 `trip_load` and
  `forage_max` (what the `returns` drive books: `scripts/tripsrc.py`), and
  `bite_x,bite_y,bite_tissue,bite_door,trip_src` (the trip reach's food cell;
  `scripts/drivefade.py` reads the drive's fade). `gifants framesdir=` for cards.
  Trail (09-29): `dwide`, `shadow`, `cf=gate`, `btrail`, `gifoverlay=b`;
  `scripts/btrailchart.py --stats --door`, `trailclimb.py`, `trailpace.py`.
- **`labforage`**: its `SUMMARY seed=` line, and `FORAGE seed=` for the drive.
- **`scripts/deadendindex.py --touching`** before a PR (needs an unshallowed
  clone to regenerate); **`scripts/branchcheck.sh --who-touched <file>`**.

## Commands

The colony bed. Run from anywhere; keep the env exactly this. **Measured
2026-09-30 in a 4-core cloud container, `xargs -P 4`:** four 4-seed jobs of
the 24,000-frame bed at 20 or 80 founders took **20 s**; 72 two-pile runs of
36,000 frames took **97 s**. Runs are cheap: no agent or cloud lane is needed
for a bed round.

```
export RAYON_NUM_THREADS=1 PIXEL_PHYSICS_COLONY_SPACING=2 PIXEL_PHYSICS_STACK_DEPTH=4 PIXEL_PHYSICS_BUD_SITE=nest
B="mode=gap gate=shipped frames=24000 ants=20 near=10 food=400 refill=400 arms=self gaps=90"
./trailfollow $B decisioncsv dtag=mine seeds=8 seed0=1 > mine-1.log   # and seed0=9, seed0=17
# decision CSVs land in /tmp as trailfollow-decisions-seed<S>-gap<G>-self-<dtag>.csv
```

**Every bed, exactly** (all with the env above, `seeds=`/`seed0=` to taste;
`arms=self,mute` unless the row says otherwise):

| Bed | Arguments |
|---|---|
| B1, 90 cells | `mode=gap gate=shipped frames=24000 ants=20 near=10 food=400 refill=400 gaps=90` |
| B2, 140 cells | the same with `gaps=140` |
| B3, 80 founders | `mode=gap gate=shipped frames=24000 ants=80 near=10 food=400 refill=400 gaps=135` (80 founders span 158 cells, so a third start 27-78 cells west of the nest: split any readout by start side) |
| B4, pulsed pile | B1 or B2 with `food=30 refill=6000` |
| B5, two piles | `mode=gap gate=shipped frames=36000 ants=20 near=10 food=400 refill=400 gaps=90 pile2=west alt=6000 arms=self,mute,oracle`, read with `python3 scripts/twopile.py '<logs>'` |
| B6, lab box | `labforage scenario=played_bed frames=120000 seed=N`, **from the repo root, with none of the bed env set** |

```
# worked example, B5 over 24 seeds:
for s in 1 7 13 19; do for a in self mute oracle; do echo "$a $s"; done; done | \
  xargs -P 4 -L 1 bash -c './trailfollow <B5 args, arms=$0> seeds=6 seed0=$1 > b5-$0-$1.log'
python3 scripts/antloop.py <csv dir> --log mine.log --vs base.log
python3 scripts/antidle.py '<csv dir>/*.csv' 90 --vs '<base csv dir>/*.csv'
```

The lab box **must run from the repo root** (it reads `assets/`). A round of
4 seeds takes about 7 minutes here, so 12 seeds about 21:

```
RAYON_NUM_THREADS=1 labforage scenario=played_bed frames=120000 seed=N
```

Since 2026-09-29 it rains at the scenario's own rate (`rain=` defaults to
the spec's Light; `rain=off` reproduces the dry runs before that, line for
line), and it refuses to start with the bed's env set unless given `bedenv`.

**Identity first:** every new switch, unset, must reproduce the default line
for line. Diff the logs with lines matching `^\s*(trailfollow:|breadoff=|ant.ron:|DECISIONS:)`
filtered out, and compare the decision CSVs too.

## Traps that cost time here

- **Stale binaries.** Build only the two harnesses (`cargo build --release
  --example trailfollow --example labforage`, ~2 minutes incremental), copy
  them into the run directory, and `grep -c` the binary for the switch name.
- **Don't edit source while a build runs.**
- **Never export the bed's env (`COLONY_SPACING`, `STACK_DEPTH`, `BUD_SITE`)
  in a script that also runs the lab.** A lab run inherited them on
  2026-09-28, founded 52 where the lab places 41, and every box died by frame
  35,000 -- read as a harm of the change under test. `labforage` now refuses
  to start with any of them set unless given `bedenv`.
- **Key every parse by seed and gap.**
- **The `energy` trace column is clamped at 200 J.** "At full energy" in it
  means *at or above the grant*; read `energy_j`. A forager "resting fed" was
  mostly one that came home hungry and ate back up off the nest (§22a).
- **A rule scaled by `drive - hunger` also reaches the hungry.** `,keep`'s
  first form starved foragers on the nest (§22e); state which ants a rule is
  for, then check the trace shows only those.
- **Watch every guard go red**, and write predictions before each run. About
  half of this lane's predictions have been wrong; the scored record is in
  the report (§22i).

## Predictions

Write them before each run; they are registered and scored in the report
(§22o-§23c), not here.
