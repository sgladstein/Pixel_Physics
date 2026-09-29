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

**Shipped on** (`=off`, or `face`, `always`, is the ant before it):
`FORAGE_DRIVE=returns`, `CARRY_PATIENCE=pickup` (Z35), `PACKED_LUNCH=on`,
`STORE_LUNCH=on`, `BIRTH_PRICE=guaranteed` (Z36) -- all `PIXEL_PHYSICS_`.

**Foragers retire, and now go back out (§22r, §22s).** The trip pays ~11x
its cost; foragers stopped after 1-3 loops because store food they held was
not a lunch. Store lunch makes it one; the `returns` drive stands fed
foragers down when no food has come home for a round trip, which is what
the lab needed. Bed, today's nest, 90 cells: food taken 4,566 -> 19,117,
born 144 -> 1,391. Lab: births 373 -> 749, ant-time +54%, died out 4 -> 3.
Price is not the limit (`burn=0.5`: loop rate 0.93 -> 0.91).

**Next:** why each ant fetches less in a bigger colony (open problem 5),
and which way to go once the door ships (open problem 3).

- **Read food taken, and food standing at the nest** (`FOOD STORE`'s
  `nest food`, mean from 6,000); "net food into home" overcounts (§22j).

## Baseline (`main` with store lunch and the `returns` drive, 2026-09-29)

Colony bed, no trail, 24 seeds, tree after #512:

| food distance | food taken from the pile | food at the nest | starved | born |
|---|---:|---:|---:|---:|
| 90 | 19,117 | 8,530 J | 74 | 1,391 |
| 140 | 13,233 | 8,586 J | 73 | 712 |
| 90, both off (the ant of #512) | 4,566 | 8,003 J | 83 | 144 |
| 140, both off | 4,264 | 7,903 J | 87 | 100 |

Lab box, 24 seeds, medians: births 749, starved 282, alive at the end 124,
food eaten 1,745k J, ant-time 15.1M; died out 3, under 10 at the end 6.
Both off: 373, 195, 112, 1,051k J, 9.8M; 4 and 5. The nest lane's door
(`NEST_DOOR=2`, off) cuts starvation on the bed and sends store-lunch
foragers west (§22r).

## Ranked open problems

1. **Lab starvation out in a grazed box** is now damped by the `returns`
   drive (§22s), not gone: starved per ant-time 19.8 -> 18.7 per million.
   The ants that die out there are hungry scouts, which no drive reaches.
2. **Early deaths.** Founders that never reach the food die around frame
   3,800; #507's founding shaft cut them to 58 of 480 by frame 6,000 at 140.
   Only the road and the nest (§17b, §19), or a founding store, reach them.
3. **Which way to go.** A driven forager leaving the nest's west end walks
   the dead end (§20); with the door and store lunch, 41% of ant-time is west
   of the nest (§22r). A memory of where its last load came from would aim
   it, as desert ants aim by the vector that paid.
4. **Food at home to the hungry.** The tether pays at 90 on top of `always`
   (201 → 123) and kills at 140 (429): the leash is the problem, not the store.
5. **Each ant fetches less in a bigger colony** (nest lane, 2026-09-28,
   `nest-colony-size-2026-09-28.md`): on the bed with the food kept ~75
   cells past the colony's near edge, 20/40/80/200 founders starve 13% /
   28% / 42% / 52% of all ants and take 9.2 / 6.1 / 4.7 / 4.2 food cells
   per founder; not the supply (the 200-colony took ~850 of ~24,000 cells
   offered) and not founding energy. Trace it with the funnel: where do the
   extra ants stop?
6. **Lab deliveries are 86% churn**: read net food into home.
7. **Latent:** a scout that has given up is released only by a nest contact.
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

## Predictions (written before each run)

Rows 1-3 are scored in `ant-scenes-2026-09-23.md` §22o, 4-9 in §22r, 10-12
in §22s.

| # | run | prediction | right? |
|---|---|---|---|
