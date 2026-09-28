# Lane note — the ant's foraging loop

*Kept current, edited in place. History and measurements live in
[`../ant-scenes-2026-09-23.md`](../ant-scenes-2026-09-23.md) (§1–§22); how the
ant works now lives in [`../how-the-ant-works.md`](../how-the-ant-works.md).
This note keeps the owner's rulings, the live question, the baseline, the
commands and the traps.*

- **Previous session:** `session_01Pt5N39pfcix13hMycPN9Xs`, branch
  `claude/ant-foraging-loop-handoff-986v7n` (2026-09-27): the forage drive and
  carry patience, both shipped on (§22, §22m).
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
- **Features default on unless there is a good reason not to** (09-27,
  replacing "never flip a default without the owner's ruling"). A good reason
  is a measured harm; neutral ships on. Defaults on the owner's word before
  that: the `trailaway` walk (09-24), crop 5,760 J with food at half weight
  (09-25), scouting at gain 2 (09-26).
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

**Both switches shipped on, 2026-09-27** (§22m), under the owner's ruling
that a feature is on unless it measures as a harm:
`PIXEL_PHYSICS_FORAGE_DRIVE` is `always` (a forager that has picked food up
away from home goes back out fed or hungry) and
`PIXEL_PHYSICS_CARRY_PATIENCE` is `pickup` (bug Z35 fixed: every pickup
restarts a carry's home patience, so loaded foragers no longer walk off the
pile's far side). `=off` on either is the ant before it, bit for bit.

- **What carry patience moved:** laden time past the pile per cell taken
  184 → 65 at 90 cells, 107 → 42 at 140 (medians, 24 seeds). The time saved
  goes to home, not to more trips: a forager comes home sooner and less
  hungry, so it waits longer. Loads leave the pile the same size.
- **Its cost, measured and accepted:** with the drive on, 48 seeds, food
  taken off the pile 5,717 → 5,402 at 140 cells (lower on 31 of 48, p 0.06);
  food at the nest, starvation and births did not move. At 90 it is a gain:
  food at the nest 6,537 → 6,958 J, starved 346 → 313.
- **Lab** (`main` after #503, both on against both off, 12 seeds): food
  eaten 0.94M → 1.61M J (8/4), born 424 → 746, alive at the end 50 → 106,
  died out 3 → 1. Before #503 it read 0 → 3. The grazing crashes are crash
  timing; seed 2 dies with food standing (198 edible cells, both on; to 6
  ants with 532, both off before #503), untraced. The lab's limit is its
  regrowing pasture.
- **"Net food into home" overcounts 3.4-4.6×** (§22j); read food taken, and
  food standing at the nest (`FOOD STORE`'s `nest food`, mean from frame
  6,000).

**Answered 2026-09-28 (§22n): the third of ant time spent at home holding
food is the colony eating the store**, a cell at a time: pick up, eat while
holding, put the rest back, again. 94% of it is store food, the whole rise
under the switches is fed ants doing it, nothing is lost, and 56 of 58 births
come from a parent at home. **Open: does that eating delay a fed forager's
next trip?** If yes, "packed lunch" (the drive sees through a crop filled at
home) could turn up to ~22% of ant time into trips; if it is filler, leave
it. A visit-level trace is deciding it. Behind it, still open:
an off-switch that reads what the lab is short of, food out there (Gordon
2002, *Am Nat* 159:509: harvesters stop when returning foragers stop
bringing food).

## Baseline (the shipped default: drive and carry patience on, `main` after #503)

Colony bed, no trail, 24 seeds (seeds 1-24):

| food distance | food taken from the pile | food at the nest | starved of 480 | by frame 6,000 | born |
|---|---:|---:|---:|---:|---:|
| 90 | 2,921 | 6,166 J | 174 | 136 | 58 |
| 140 | 2,479 | 5,975 J | 223 | 181 | 26 |

Both `=off` on the same tree: 90 cells 2,209, 4,539 J, 230, 159, 15; 140
cells 1,940, 3,956 J, 242, 182, 9. #503 (spoil stays spoil) moved the bed by
itself, so numbers from before it (§22m's component table) are a different
tree. Lab box, 12 seeds, median, both on (`main` after #503): food eaten 1,610k J,
births 746, alive at the end 106, ant-frames lived 14.4M; died out 1, under
10 at the end 3. Both off on the same tree: 940k J, 424, 50, 7.7M; 3 and 5.

## Ranked open problems

1. **Loaded foragers holding food at home** (above), a third of ant time.
   Then the drive's off-switch, lab first.
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
