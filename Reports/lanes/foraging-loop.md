# Lane note — the ant's foraging loop

*Kept current, edited in place. History and measurements live in
[`../ant-scenes-2026-09-23.md`](../ant-scenes-2026-09-23.md) (§1–§22); how the
ant works now lives in [`../how-the-ant-works.md`](../how-the-ant-works.md).
This note keeps the owner's rulings, the live question, the baseline, the
commands and the traps.*

- **Previous session:** `session_01Pt5N39pfcix13hMycPN9Xs`, branch
  `claude/ant-foraging-loop-handoff-986v7n` (2026-09-28): packed lunch, the
  birth price, why food does not build up (§22q), foragers retire (§22r).
- **Peer lanes:** the nest-mouth lane ([`nest-mouth.md`](nest-mouth.md))
  shipped dig down for an enclosed digger (#508, 2026-09-28). With packed
  lunch it costs the loop (below); the owner ruled a nest step is not blocked
  on colony numbers, and that lane is tracing which ants dig when food is
  wanted.

## Standing owner rulings

- **The goal:** foragers earn enough to feed themselves *and* extra for the
  colony (2026-09-25). "All we care about is the loop is improving."
- **The colony bed decides; the lab box is the pre-ship check** (09-28,
  replacing "test in both games before any ruling"). Develop and rule on
  loop changes on the bed (`trailfollow`) at 90 *and* 140 cells, traced.
  Before a change ships, run the lab (`labforage`, 24 seeds) once as a
  regression check: died out, starved, births, alive at the end. It is not
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
- An ant should breed only at the nest, and in the end *where* it breeds
  should be something a lineage evolves (09-23). `PIXEL_PHYSICS_BUD_SITE=nest`,
  off by default; the bed runs with it on.

## Live question

**Shipped on** (`=off`, or `face`, is the ant before it, bit for bit):
`FORAGE_DRIVE=always`, `CARRY_PATIENCE=pickup` (Z35), `PACKED_LUNCH=on`,
`BIRTH_PRICE=guaranteed` (Z36) -- all `PIXEL_PHYSICS_`.

**Foragers retire (§22r, the owner's "is the loop broken or the economy too
hard?").** Neither: the trip works and pays ~11x its cost, and foragers stop
making it. 41% of a forager's life comes after its last delivery, 70% of that
fed at home, and the drive reaches 19% of it: the retiree holds store food
that is not a lunch (taken just off nest material, or a lunch turned load by
a later cell) or a pellet. Halving the cost of living (`burn=0.5`) moves the
loop rate 0.93 -> 0.91 per 10,000 ant-frames: price is not the limit.

**`STORE_LUNCH=on` is built and held off.** Food taken before an ant has been
8 cells out since its last nest contact is a lunch. Bed, door off: food taken
4,574 -> 19,732 (24/0), loops per 10,000 ant-frames 0.78 -> 1.46, starved flat.
With the door it sends 41% of ant-time west (open problem 3). **The lab flags
it**: starved 137 -> 263 (17/7), died out 1 -> 5 -- boom and bust in a finite
box. **Next: the off-switch** (open problem 1), then the lab with both on.

- **Read food taken, and food standing at the nest** (`FOOD STORE`'s
  `nest food`, mean from 6,000); "net food into home" overcounts (§22j).

## Baseline (`main` after #511: drive, carry patience, packed lunch, birth price, dig down)

Colony bed, no trail, 24 seeds (seeds 1-24), measured 2026-09-28:

| food distance | food taken from the pile | food at the nest | starved of 480 | born |
|---|---:|---:|---:|---:|
| 90 | 4,574 | 7,743 J | 93 | 151 |
| 140 | 4,174 | 7,554 J | 89 | 91 |
| 90, `NEST_DOOR=2` | 5,611 | 6,279 J | 15 | 150 |
| 140, `NEST_DOOR=2` | 5,046 | 4,773 J | 38 | 82 |

The door rows are the nest lane's switch (§22r). Lab box, 24 seeds, median,
same tree: food eaten 1,149k J, births 408, starved 137, alive at the end 78,
ant-frames lived 10.5M; died out 1, under 10 at the end 6. With the door:
1,053k J, 406, 156, 113, 9.4M; 2 and 2 (no sign test below p 0.15).

Before #508 (dig down) and #510 the 90-cell bed read 5,377 / 8,591 J / 64 /
185 and 140 read 4,859 / 7,827 J / 79 / 115; packed lunch off on that tree
3,744 / 7,160 J / 83 / 59. #507 (founding shaft, heap cue) moved the bed by
itself, so numbers from before it are a different tree.

## Ranked open problems

1. **The drive has no off-switch.** In the lab, ants starve out in a grazed
   box (55% more than 128 cells from the nest, a third beside corpse, litter
   or buried crumbs), and store lunch doubles it (§22r). Harvesters stop when
   returning foragers stop bringing food (Gordon 2002, *Am Nat* 159:509):
   read that, not a clock. Blocks `STORE_LUNCH`.
2. **Early deaths.** Before #507, at 140 cells, 171–177 of ~207 starved never
   reached the food and died around frame 3,800, before any forager existed;
   #507's founding shaft cut starvation to 79 of 480 there, 58 by frame 6,000. Only the road
   and the nest (§17b, §19), or a colony founded with a store, can reach them.
3. **Which way to go.** A driven forager leaving the nest's west end walks
   the dead end (§20); with the door and store lunch, 41% of ant-time is west
   of the nest (§22r). A memory of where its last load came from would aim
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

Rows 1-3 are scored in `ant-scenes-2026-09-23.md` §22o, rows 4-9 in §22r.

| # | run | prediction | right? |
|---|---|---|---|
