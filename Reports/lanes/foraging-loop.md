# Lane note — the ant's foraging loop

*Kept current, edited in place. History and measurements live in
[`../ant-scenes-2026-09-23.md`](../ant-scenes-2026-09-23.md) (§1–§22); how the
ant works now lives in [`../how-the-ant-works.md`](../how-the-ant-works.md).
This note keeps the owner's rulings, the live question, the baseline, the
commands and the traps.*

- **Previous session:** `session_01Pt5N39pfcix13hMycPN9Xs`, branch
  `claude/ant-foraging-loop-handoff-986v7n` (2026-09-27): the forage drive (§22).
- **Peer lanes:** the nest-mouth lane ([`nest-mouth.md`](nest-mouth.md)) has
  concluded: no mouth beats today's nest on both beds.

## Standing owner rulings

- **The goal:** foragers earn enough to feed themselves *and* extra for the
  colony (2026-09-25). "All we care about is the loop is improving."
- **Test in both games**, the colony bed (`trailfollow`) and the lab box
  (`labforage`), and at 140 cells, before asking for any ruling. The lab
  caught the painted door (§19) and the forage drive's early crashes (§22h).
- **Lead with the specific quantity a change targets**; colony totals
  (starved, net food into home) are the check, not the headline (09-26).
- **Never flip a default without the owner's explicit ruling.** Defaults on
  the owner's word so far: the `trailaway` walk (09-24), crop 5,760 J with
  food at half weight (09-25), scouting at gain 2 (09-26).
- **"Scout or any ant should come home when they get so hungry before they
  are going to starve to death"** (09-26). `PIXEL_PHYSICS_HUNGRY_HOME`, off:
  it helps at 90 cells and kills at 140 whatever home holds (§21, §22g).
- **Sub-agents allowed** (09-27): in-process agents for reading and surveys
  (not runs: they share this box's 4 cores); a cloud session for a run that
  would otherwise queue, via the `lab-coordinator` skill with `model:` set.
  Either way the return path is files.
- An ant should breed only at the nest, and in the end *where* it breeds
  should be something a lineage evolves (09-23). `PIXEL_PHYSICS_BUD_SITE=nest`,
  off by default; the bed runs with it on.

## Live question

**The forage drive moves the loop on the bed and not in the lab; the owner
has not ruled.** `PIXEL_PHYSICS_FORAGE_DRIVE` (§22, off). Under `always` a
forager that has been to the food goes back out while fed, scouting and pacing
like a hungry ant:
- **bed 90** (rerun on `main`; the first run was an intermediate build, §22):
  loops per forager 1.46 → 2.04 (22/0); food taken from the pile
  2,241 → 3,327 (24/0); starved 201 → 172 (more by frame 6,000, 125 → 133,
  far fewer after, 76 → 39); born 10 → 67. With the come-home tether on top:
  starved 123 and food taken 3,674, the best arm at 90;
- **bed 140:** loops per forager 1.57 → 1.97 (21/2); food taken
  2,005 → 2,895 (24/0); born 7 → 48; starvation unchanged (the early cull,
  problem 2);
- **lab:** 52% more carried home, but food eaten, births and survival flat.
  "2 of 12 die out, 0 today" is crash timing: 4 of 12 end under 10 ants in
  both arms. The lab's limit is its regrowing pasture.
- **"Net food into home" overcounts 3.4-4.6×** (§22j); read food taken.

The forms that read the colony's need did worse. `hunger` is inert on the bed
(the colony is fed once the early deaths are over) and ended 4 of 12 lab
colonies (one a founding that never grew, three boom and bust). `larder` is a weaker `always` on the bed and ends lab
colonies smaller. `,keep` (fed foragers leave the store) made them stand in it
digging, and births fell 67 → 9. A fed forager eating the store is how the
surplus becomes new ants: budding reads body energy.

**Shown to the owner:** card `20260927T172924982Z-46b30b` (seed 22, frames
6,000–18,000, both arms). **Next, if the owner wants it:** an off-switch
that reads what the lab is short of, food out there, not need at home.
Harvester ants stop going out when returning foragers stop bringing food
(Gordon 2002, *Am Nat* 159:509). So the drive would fall with how recently
this ant, or laden nestmates it met, found food. Test the lab first.

## Baseline (the shipped default, 2026-09-27)

Colony bed, no trail, 24 seeds:

| food distance | loops per forager | starved of 480 | net food into home | born |
|---|---:|---:|---:|---:|
| 90 | 1.46 | 201 | 7,506 | 10 |
| 140 | 1.57 | 209 | 5,999 | 7 |
| 200 | – | 322 | 2,913 | – |

At 90 the colony takes in 64% of what it burns. Lab box, 12 seeds, median:
net food into home 856, food eaten 1,150k J, births 530, alive 80, extinct 0.

## Ranked open problems

1. **The drive's off-switch** (above). Behind the same switch, lab first.
2. **Early deaths.** At 140 cells, 171–177 of ~207 starved never reach the
   food and die around frame 3,800, before any forager exists. Only the road
   and the nest (§17b, §19), or a colony founded with a store, can reach them.
3. **Which way to go.** A driven forager leaving the nest's west end walks
   the dead end (§20). A memory of where its last load came from would aim
   it, as desert ants aim by the vector that paid.
4. **Food at home to the hungry.** The tether pays at 90 on top of `always`
   (201 → 123) and kills at 140 (429): the leash is the problem, not the store.
5. **Lab deliveries are 86% churn**: read net food into home.
6. **Latent:** a scout that has given up is released only by a nest contact.
   Harmless on both beds; `HUNGRY_HOME=tether` already lets go on arrival.

## Tools and skills (use these; the names do not say what they answer)

- **`funnel` skill** before investigating anything; **`review` skill** when a
  change is visible.
- **`scripts/antloop.py`**: the loop ant by ant (funnel, who starved and
  where, time budget, economy). `--vs base.log` pairs the checks by seed.
- **`scripts/antidle.py '<glob>' <gap> [--vs '<base glob>']`**: do foragers
  keep foraging -- loops per forager, waits, a forager's life after its first
  loop (fed/hungry x home/out, food held off the nest). `--vs` pairs these,
  the target quantities, by seed. **Give it the gap**; 90 is the default.
- **`scripts/labpair.py`**: lab arms paired by seed on net food into home.
  The lab's end-of-run `alive` is one frame of a boom-and-bust: read the whole
  curve (`labforage`'s 900-frame table; peak, ant-frames, when it grazed out).
- **`trailfollow decisioncsv dtag=`**: the per-decision trace. Since 09-27 it
  carries `energy_j` (`energy` is clamped at the 200 J grant) and `drive`,
  `scout_w`, `scout_patience`, `scout_home`. `gifants framesdir=` for cards.
- **`labforage`**: its `SUMMARY seed=` line, and `FORAGE seed=` for the drive.
- **`scripts/deadendindex.py --touching`** before a PR (needs an unshallowed
  clone to regenerate); **`scripts/branchcheck.sh --who-touched <file>`**.

## Commands

The colony bed. Run from anywhere; keep the env exactly this. Three batches of
8 seeds in parallel take about 5 minutes on this box.

```
export RAYON_NUM_THREADS=1 PIXEL_PHYSICS_COLONY_SPACING=2 PIXEL_PHYSICS_STACK_DEPTH=4 PIXEL_PHYSICS_BUD_SITE=nest
B="mode=gap gate=shipped frames=24000 ants=20 relay=60 near=10 food=400 refill=400 stop=6000 layfrom=founders arms=self gaps=90"
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

**Identity first:** every new switch, unset, must reproduce the default line
for line. Diff the logs with lines matching `^\s*(trailfollow:|breadoff=|ant.ron:|DECISIONS:)`
filtered out (note the leading spaces), and compare the decision CSVs too.

## Traps that cost time here

- **Stale binaries.** Build only the two harnesses (`cargo build --release
  --example trailfollow --example labforage`, ~2 minutes incremental), copy
  them into the run directory, and `grep -c` the binary for the switch name.
- **Don't edit source while a build runs**, and never `pgrep -f`/`pkill -f`.
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
- **Messages from other sessions may not reach you**; files are the channel.

## Predictions (written before each run)

| # | run | prediction | right? |
|---|---|---|---|
| | | | |
