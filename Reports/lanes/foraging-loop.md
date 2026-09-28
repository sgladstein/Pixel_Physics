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

**Four forage switches ship on** (§22m, §22o, §22p); `=off` (`face` for the
birth price) on each is the ant before it, bit for bit:
`PIXEL_PHYSICS_FORAGE_DRIVE=always` (a forager goes back out fed or hungry),
`PIXEL_PHYSICS_CARRY_PATIENCE=pickup` (Z35), `PIXEL_PHYSICS_PACKED_LUNCH=on`
(a fed forager takes store food out and eats it on the road; food taken
3,744 -> 5,377 at 90) and `PIXEL_PHYSICS_BIRTH_PRICE=guaranteed` (a birth no
longer overdraws its parent, Z36; lab starved 257 -> 137, generations
28 -> 18; bed byte-identical).

**Why so little food builds up at the nest (§22q, owner's question):** the
colony keeps its food in its bodies. Fed ants at home take the store, eat
from it and put it back (a quarter of all decisions), and that is how banks
fill for births and lunches fuel trips. Stopping it doubles the floor store
and drops births 151 -> 7-16. The limit is trips: loopers make about two in
24,000 frames. Levers, not built: a load topped up at home becomes a lunch;
a digger holding a pellet does not eat; a lunch carrier is not held by food
beside it.

**Then:** the lab's remaining starvation is ordinary. Most starved ants die
out in the box (55% more than 128 cells from the nest) after it is grazed
down, and a third die within 3 cells of food that is mostly corpse, litter
or buried crumbs. Traced with `labforage lifetrace=`. Behind it:
an off-switch that reads what the lab is short of, food out there (Gordon
2002, *Am Nat* 159:509: harvesters stop when returning foragers stop
bringing food).

- **"Net food into home" overcounts 3.4-4.6x** (§22j); read food taken, and
  food standing at the nest (`FOOD STORE`'s `nest food`, mean from frame
  6,000).

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

1. **Lab starvation out in a grazed box** (above). `labforage lifetrace=`
   traces it per ant. The birth-overdraw quarter of it is fixed (Z36). Then
   the drive's off-switch, lab first.
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

Rows 1-3 (packed lunch, scored) are in `ant-scenes-2026-09-23.md` §22o.

| # | run | prediction | right? |
|---|---|---|---|
| 4 | `BIRTH_PRICE=guaranteed` vs `face`, lab 24 seeds (2026-09-28, this branch after #508) | parents killed by their own birth ~0 (from 11-15% of births); starved deaths down ~20-25% over the run; births up a little; alive at the end up; died out no worse | overdrawn 88.5 -> 0 (right); starved -47% (right way, twice the size); births -33% (wrong); alive at the end flat (wrong); died out 2 -> 1 (right) |
| 5 | same, bed 90 and 140, 24 seeds | neutral: bed births are paid at the nest in crumbs, not seeds; starved and born within the spread | right: byte-identical at both distances (no seeds in the bed's diet) |
| 6 | `wire=AtNest:Feed:-0.7,Energy:Feed:-0.7` on `main` after #510, bed 90, 24 seeds (2026-09-28): fed ants at home stop re-taking the store | food standing on the nest at least 2x (1,400-2,800 J to an ant today); food taken off the pile up 5-15%; starved within the spread; ants' bodies hold less | food on the nest 7,743 -> 13,832 J (right); taken 4,574 -> 2,523, lower on 24 (wrong); starved 93 -> 180 (wrong); bodies 8,038 -> 2,124 J (right); born 151 -> 7. s22q |
