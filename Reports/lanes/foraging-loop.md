# Lane note — the ant's foraging loop

*Kept current, edited in place. History and measurements live in
[`../ant-scenes-2026-09-23.md`](../ant-scenes-2026-09-23.md) (§1–§22); how the
ant works now lives in [`../how-the-ant-works.md`](../how-the-ant-works.md).
This note keeps the owner's rulings, the live question, the baseline, the
commands and the traps.*

- **Previous session:** `session_01Pt5N39pfcix13hMycPN9Xs`, branch
  `claude/ant-foraging-loop-handoff-986v7n` (2026-09-27): the forage drive and
  carry patience, both shipped on (§22, §22m).
- **Peer lanes:** the nest-mouth lane ([`nest-mouth.md`](nest-mouth.md))
  shipped dig down for an enclosed digger (#508, 2026-09-28). With packed
  lunch it costs the loop (below); the owner ruled a nest step is not blocked
  on colony numbers, and that lane is tracing which ants dig when food is
  wanted.

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
- **Agents: judge each situation, use them where they help, never
  wastefully** (09-28; no default count, and not scale for its own sake
  under a thoroughness mode). A 39-agent run cost 9.4M where its 3 tracers
  cost 1.35M (`agent-strategy.md` s4).
- **Sub-agents allowed** (09-27): in-process agents for reading and surveys
  (not runs: they share this box's 4 cores); a cloud session for a run that
  would otherwise queue, via the `lab-coordinator` skill with `model:` set.
  Either way the return path is files.
- An ant should breed only at the nest, and in the end *where* it breeds
  should be something a lineage evolves (09-23). `PIXEL_PHYSICS_BUD_SITE=nest`,
  off by default; the bed runs with it on.

## Live question

**Three forage switches ship on** (§22m, §22o), under the owner's ruling that
a feature is on unless it measures as a harm; `=off` on each is the ant
before it, bit for bit:
- `PIXEL_PHYSICS_FORAGE_DRIVE=always`: a forager that has picked food up away
  from home goes back out fed or hungry;
- `PIXEL_PHYSICS_CARRY_PATIENCE=pickup`: every pickup restarts a carry's home
  patience (bug Z35 fixed);
- `PIXEL_PHYSICS_PACKED_LUNCH=on` (2026-09-28): food taken at home is not a
  load, so a fed forager nibbling the store is driven back out and eats it on
  the road, finishing it where it meets food its crop cannot swallow.

**Packed lunch answered §22n's question: the store time was a delay.** Given
the drive, a fed forager holding store food goes: trips to the pile
1,326 -> 1,851 (23/1), food taken 3,744 -> 5,377 (24/0), born 59 -> 185 at 90
cells; 140 cells and the lab as in §22o. Home-holding did not fall (35.7% ->
39.3%): a richer colony has more floor beside food to sip. Setting the lunch
down instead exported the store (a dead end).

**Next:** two lab seeds (7, 20) fall under 10 ants with food standing under
every form of packed lunch; untraced, and `labforage` has no per-ant trace,
so tracing them means adding one. Behind it: an off-switch that reads what
the lab is short of, food out there (Gordon 2002, *Am Nat* 159:509:
harvesters stop when returning foragers stop bringing food).

- **"Net food into home" overcounts 3.4-4.6x** (§22j); read food taken, and
  food standing at the nest (`FOOD STORE`'s `nest food`, mean from frame
  6,000).

## Baseline (the shipped default: drive, carry patience and packed lunch on)

Colony bed, no trail, 24 seeds (seeds 1-24):

| food distance | food taken from the pile | food at the nest | starved of 480 | born |
|---|---:|---:|---:|---:|
| 90 | 5,377 | 8,591 J | 64 | 185 |
| 140 | 4,859 | 7,827 J | 79 | 115 |
| 90, with #508's dig down | **4,574** | 7,743 J | 93 | 151 |

The first two rows are `main` after #507; the third is this branch merged
with #508's head (24 seeds, 90 cells only; 140 and the lab not yet re-run
with both). Against the first row, dig down takes less off the pile on 21 of
24 seeds and starves more on 12 (5 fewer). Neither switch on that tree:
3,744, 7,160 J, 83, 59; dig down alone 3,057, 6,670 J, 152, 30.

Packed lunch `=off` on the same tree: 90 cells 3,744, 7,160 J, 83, 59; 140
cells 3,797, 6,762 J, 79, 55. #507 (founding shaft, heap cue) moved the bed
by itself, so numbers from before it are a different tree. Lab box, 24 seeds,
median, shipped: food eaten 1,150k J, births 482, alive at the end 55,
ant-frames lived 10.9M; died out 3, under 10 at the end 6. Packed lunch off:
1,170k J, 515, 48, 9.8M; 1 and 3.

## Ranked open problems

1. **Lab boxes that die with food standing** (seeds 7, 20; above). Needs a
   per-ant trace in `labforage`. Then the drive's off-switch, lab first.
2. **Early deaths.** Before #507, at 140 cells, 171–177 of ~207 starved never
   reached the food and died around frame 3,800, before any forager existed;
   #507's founding shaft cut starvation to 79 of 480 there, 58 by frame 6,000. Only the road
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
- **Never export the bed's env (`COLONY_SPACING`, `STACK_DEPTH`, `BUD_SITE`)
  in a script that also runs the lab.** A lab run inherited them on
  2026-09-28, founded 52 where the lab places 41, and every box died by frame
  35,000 -- read as a harm of the change under test. `labforage` now echoes
  all three; check the header says `shipped`.
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
| 1 | `PACKED_LUNCH=on` vs off, bed 90, 24 seeds (2026-09-28, on `main` after #505) | pile trips up: food taken from the pile +10-30%; home-holding share of ant time 37% -> 25-30%; food standing at the nest flat or down (the lunch is eaten on the road, not put back); births flat or slightly down (budding needs the nest); starved flat | half. First form, before #507: food taken +12% (right), births 59 -> 93 (wrong, up), nest food flat (right). Shipped form after #507: food taken +44%, trips +40%, births x3.1, nest food +20%, home-holding UP 35.7% -> 39.3% (wrong) |
| 2 | same, bed 140 | same direction, smaller: food taken +5-20% | right in direction, wrong in size: +21% (first form), +28% (shipped); births x2.1 |
| 3 | same, lab 12 seeds | food eaten up; boom-and-bust sooner, so extinctions no better | first form before #507: every measure lower, died out 1 -> 4 (wrong); after #507, 24 seeds: food eaten up 17/7 (right); shipped form neutral, died out 1 -> 3 (right on extinctions) |
