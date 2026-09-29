# Lane note — the ant's foraging loop

*Kept current, edited in place. History and measurements live in
[`../ant-scenes-2026-09-23.md`](../ant-scenes-2026-09-23.md) (§1–§22); how the
ant works now lives in [`../how-the-ant-works.md`](../how-the-ant-works.md).
This note keeps the owner's rulings, the live question, the baseline, the
commands and the traps.*

- **Previous session:** `session_01Pt5N39pfcix13hMycPN9Xs`, branch
  `claude/ant-foraging-loop-handoff-986v7n` (2026-09-28/29): §22o-§22u,
  packed lunch through the `returns` drive and the trip reach.
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
  under a thoroughness mode). A 39-agent run cost 9.4M where its 3 tracers
  cost 1.35M (`agent-strategy.md` s4).
- **Sub-agents allowed** (09-27): in-process agents for reading and surveys
  (not runs: they share this box's 4 cores); a cloud session for a run that
  would otherwise queue, via the `lab-coordinator` skill with `model:` set.
  Either way the return path is files.
- **A nest step is not blocked on colony numbers** (09-28): the nest lane
  ships a nest change that costs the loop, and this lane answers the cost.
- An ant should breed only at the nest, and in the end *where* it breeds
  should be something a lineage evolves (09-23). `PIXEL_PHYSICS_BUD_SITE=nest`,
  off by default; the bed runs with it on.

## Live question

**Shipped on** (`=off`, or `face`, `always`, is the ant before it):
`FORAGE_DRIVE=returns`, `TRIP_REACH=on` (16), `CARRY_PATIENCE=pickup` (Z35), `PACKED_LUNCH=on`,
`BIRTH_PRICE=guaranteed` (Z36) -- all `PIXEL_PHYSICS_`; and the nest lane's
granary (#513: `NEST_DOOR`, `STOREROOM`). **`STORE_LUNCH` is off** (§22t).

**Which way to go is now the loop's blocker (§22t).** Store lunch was the
lane's largest gain before the granary (taken 4,566 -> 19,117) and on it is
all of a loss: starved 75 -> 215 (20/4), because a forager that takes food
at the door leaves with it at full drive, no bearing, and digs west of the
door until it starves, while the storeroom goes empty (6.1 -> 0.4 cells).
The granary alone keeps that ant home. So a lunch needs a bearing before it
can ship.

**Next:** give a driven forager a bearing out of the nest (open problem 1);
then store lunch again. The trip reach (§22u) made the `returns` drive fade
when food stops; what is left is its clock (open problem 2).

- **Read food taken, and food standing at the nest** (`FOOD STORE`'s
  `nest food`, mean from 6,000); "net food into home" overcounts (§22j).

## Baseline (`main` with the granary, `returns` and the trip reach, 2026-09-29)

Colony bed, no trail, 24 seeds (e9d6562f, the shipped build), unlimited pile:

| | food taken from the pile | food at the nest | starved | born |
|---|---:|---:|---:|---:|
| 90 cells | 5,806 | 11,971 J | 82 | 179 |
| 140 cells | 4,602 | 9,294 J | 110 | 87 |
| 80 founders at 135 | 8,106 | 8,726 J | 1,102 | 58 |
| 90, `TRIP_REACH=off` (`main` at ac8644fe) | 5,992 | 12,147 J | 73 | 223 |

Unlimited 90 is noisy: 48 more seeds read starved 206 -> 206.
Pulsed pile at 90 (`food=30 refill=6000`): 2,602 / 275 / 35 (off 2,680 /
273 / 46). Lab box (`played_bed`, 120,000 frames, 24
seeds, rain), medians off -> on: births 428 -> 522.5, food eaten 1,122k ->
1,283k J, ant-frames 9.5M -> 10.6M, starved per million ant-frames 9.2 ->
10.4 (none at p < 0.05). No earlier lab number is comparable.

## Ranked open problems

1. **Which way to go.** A driven forager has no bearing. Before the
   granary the strip of nest paint covered the ground west of the nest;
   the door leaves it open, and a forager sent out with a lunch digs there
   until it starves (§22t; 32% of ant-time west against 14%). A memory of
   where its last load came from would aim it, as desert ants aim by the
   vector that paid. Store lunch waits on it.
2. **The drive's clock hears of food only at the door** (§22u). The trip
   reach (on) stopped food beside the door booking returns; what is left is
   a false stand-down at 20 founders on a paying pile (seed 7: 83% of its
   low-drive time had a load on the road), and every refill of the pulsed
   pile starts with the drive down (0.613 in the first 1,000 frames, against
   0.936 off). A longer window fixed the bed and leaned worse on every lab
   gate, none significant. Births lean lower at 90 cells (72 seeds 562 ->
   484), food standing at the nest with them (27/37).
3. **Early deaths.** Founders that never reach the food die early (35% of
   the starved on the granary at 90 cells). Only the road and the nest
   (§17b, §19), or a founding store, reach them.
4. **Each ant fetches less in a bigger colony** (nest lane,
   `nest-colony-size-2026-09-28.md`): 20/40/80/200 founders starve 13% /
   28% / 42% / 52%; on the granary 80 founders starve 55% of the ants that
   lived. Trace it with the funnel: where do the extra ants stop?
5. **Food at home to the hungry.** The tether pays at 90 and kills at 140:
   the leash is the problem, not the store.
6. **Lab deliveries are 86% churn**: read net food into home.
7. **Latent:** a scout that has given up is released only by a nest contact.

## Tools and skills (use these; the names do not say what they answer)

- **`funnel` skill** before investigating anything; **`review` skill** when a
  change is visible.
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
- **`labforage`**: its `SUMMARY seed=` line, and `FORAGE seed=` for the drive.
- **`scripts/deadendindex.py --touching`** before a PR (needs an unshallowed
  clone to regenerate); **`scripts/branchcheck.sh --who-touched <file>`**.

## Commands

The colony bed. Run from anywhere; keep the env exactly this. Three batches of
8 seeds in parallel take about 5 minutes on this box.

```
export RAYON_NUM_THREADS=1 PIXEL_PHYSICS_COLONY_SPACING=2 PIXEL_PHYSICS_STACK_DEPTH=4 PIXEL_PHYSICS_BUD_SITE=nest
B="mode=gap gate=shipped frames=24000 ants=20 near=10 food=400 refill=400 arms=self gaps=90"
./trailfollow $B decisioncsv dtag=mine seeds=8 seed0=1 > mine-1.log   # and seed0=9, seed0=17
# decision CSVs land in /tmp as trailfollow-decisions-seed<S>-gap<G>-self-<dtag>.csv
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
filtered out (note the leading spaces), and compare the decision CSVs too.

## Traps that cost time here

- **Stale binaries.** Build only the two harnesses (`cargo build --release
  --example trailfollow --example labforage`, ~2 minutes incremental), copy
  them into the run directory, and `grep -c` the binary for the switch name.
- **Don't edit source while a build runs**, and never `pgrep -f`/`pkill -f`.
- **Never export the bed's env (`COLONY_SPACING`, `STACK_DEPTH`, `BUD_SITE`)
  in a script that also runs the lab.** A lab run inherited them on
  2026-09-28, founded 52 where the lab places 41, and every box died by frame
  35,000 -- read as a harm of the change under test. `labforage` now refuses
  to start with any of them set unless given `bedenv`.
- **Key every parse by seed.** `carry->nest` is cell-steps, not food.
- **The `energy` trace column is clamped at 200 J.** "At full energy" in it
  means *at or above the grant*; read `energy_j`. A forager "resting fed" was
  mostly one that came home hungry and ate back up off the nest (§22a).
- **A rule scaled by `drive - hunger` also reaches the hungry.** `,keep`'s
  first form starved foragers on the nest (§22e); state which ants a rule is
  for, then check the trace shows only those.
- **Watch every guard go red**, and write predictions before each run. About
  half of this lane's predictions have been wrong; the scored record is in
  the report (§22i).

## Predictions (written before each run)

Rows 1-3 are scored in `ant-scenes-2026-09-23.md` §22o, 4-9 in §22r, 10-12
in §22s.

| # | run | prediction | right? |
|---|---|---|---|
