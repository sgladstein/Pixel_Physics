# Lane note — the ant's foraging loop

*Kept current, edited in place. History and measurements live in
[`../ant-scenes-2026-09-23.md`](../ant-scenes-2026-09-23.md) (§1–§21); how the
ant works now lives in [`../how-the-ant-works.md`](../how-the-ant-works.md).
This note keeps the owner's rulings, the live question, the baseline, the
commands and the traps.*

- **Previous session:** `session_01AFH5xR442VuoZsXm7VzJmx`, branch
  `claude/upbeat-gates-u90pys` (handed off 2026-09-27).
- **Peer lanes:** the nest-mouth lane ([`nest-mouth.md`](nest-mouth.md)) has
  concluded: no mouth tried beats today's nest on both beds. Its switches
  (`PIXEL_PHYSICS_NEST_SHAFT`, `_NEST_HOME`) and the counter
  `CreatureStats::pickups_at_nest` are on `main`.

## Standing owner rulings

- **The goal:** foragers earn enough to feed themselves *and* extra for the
  colony (2026-09-25). "All we care about is the loop is improving."
- **Test in both games**, the colony bed (`trailfollow`) and the lab box
  (`labforage`). The lab caught the painted door that the bed alone would
  have shipped (§19).
- **Lead with the specific quantity a change targets**; colony totals
  (starved of 480) are the check, not the headline (2026-09-26).
- **Never flip a default without the owner's explicit ruling.** Build behind
  a switch, measure, then ask. Defaults on the owner's word so far: the
  `trailaway` walk (09-24), crop 5,760 J with food at half weight (09-25),
  scouting at gain 2 (09-26).
- **"Scout or any ant should come home when they get so hungry before they
  are going to starve to death"** (09-26). Built as
  `PIXEL_PHYSICS_HUNGRY_HOME` and **off**, because it only pays when home can
  feed the hungry (§21). The owner has not yet seen that result; bring it
  with the fix below.
- **You may spin up sub-agents if that helps** (2026-09-27). Pick the kind by
  what the job is short of:
  - **In-process** (`Agent` tool): for reading and surveying, e.g. grepping
    `dead-ends.md`, tracing a mechanism through `creature.rs`, or checking
    a guard. **Not for runs**: they share this container's 4 cores, and a
    lab box already takes about 15 minutes over 12 seeds on all four.
  - **A cloud session** (`create_session`): gets its own machine. Use one
    for a run that would otherwise queue, e.g. a lab arm beside a bed arm.
    Invoke the `lab-coordinator` skill first, whatever the task (the name
    is historical), and pass `model:` explicitly, never inherited (Opus
    unless there is a reason).
  - **Either way the return path is files, not messages.** A sub-agent
    writes its findings where you will read them, and you own the merge.
- An ant should breed only at the nest, and in the end *where* it breeds
  should be something a lineage evolves (09-23). `PIXEL_PHYSICS_BUD_SITE=nest`,
  off by default; the bed runs with it on.

## Live question

**Fed foragers stop foraging.** The owner asked (2026-09-27): *food not
building up at the nest seems like the #1 limit -- are ants foraging too slowly,
or is the economy still too hard?* **Too slowly**, measured on the 90-cell bed
(scouting default, 24 seeds):

- **Supply never limits.** The pile refills. The colony takes 91 cells a run
  against the ~116 it burns, and absorbs 64% of what it burns.
- **Each trip pays.** A loop brings home 4.9 cells (~1,170 J to an ant). A
  forager burns 0.06 J a frame, so a loop pays about 3x its cost even counting
  the wait between loops, and far more on the walk alone.
- **The effort is missing.** 303 of 480 founders completed a loop, but they
  averaged **1.5 loops each**. After its last loop a forager lives a median
  **11,712 frames at full energy** and never goes out again. 200 of the 303
  are alive at the end, while 201 nestmates starve.
- **What sends an ant out is its own hunger.** Scouting scales with it, by
  design, and the brain's `Move` reads `Energy`. The colony's need barely
  reaches a fed forager: `KinNeed` (hungry kin beside it) reads 0 on 99% of
  its decisions at home.
- **So food cannot build up at home.** About 19 loops a run cannot cover the
  colony's burn, and whatever lands is eaten at once. Food at home does not
  reach the hungry either (a median 4 cells on the whole nest; hungry ants
  beside food 42 times in 285, eating it 71–84% of the time when they are),
  but that is downstream: there is too little to share.

**The candidate: a forager's drive to go out follows the colony's need, not
its own belly.** Real foragers keep foraging while nestmates take their loads
quickly, and slow down when unloading is slow (honeybees: unloading time).
Engine forms to weigh, each behind a switch:

- let `scout_w` read the colony's need, e.g. how little food the nest's larder
  point holds, or hungry kin met at home;
- give the forager a fidelity that persists after its first loop;
- the social stomach: unload into hungry nestmates, so a forager's crop, not
  its belly, is what it fills.

Target quantity: **loops per looper, and the frames a fed forager spends at
home** (`scripts/antidle.py` prints both). Check: net food into home and starved. Not §17f's `hungergate` (inert,
dead-ends `digest_hunger_weight`). Grep `dead-ends.md` for `share`,
`trophallaxis`, `KinNeed` and `forager` first. Then re-test
`HUNGRY_HOME=tether` on top, on both beds and at 140 cells.

## Baseline (the shipped default, 2026-09-27)

Colony bed, no trail, 24 seeds, paired against scouting off:

| food distance | starved of 480 | net food into home, cells | reached the food |
|---|---:|---:|---:|
| 90 | 201 (was 279) | 7,506 (was 4,642) | 322 |
| 140 | 209 (was 416) | 5,999 (was 882) | 294 |
| 200 | 322 (was 467) | 2,913 (was 183) | 255 |

At 90 cells the colony takes in 64% of what it burns. Lab box, 12 seeds,
median: net food into home 856 cells, food eaten 1,150k J, births 530,
extinct 0 of 12.

## Ranked open problems

1. **Fed foragers stop foraging** (above): 1.5 loops per looper, then a
   median 11,712 frames resting at full energy. Food at home reaching the
   hungry comes after; there is too little to share until this moves.
2. **Scouts that pick the dead-end side** waste their reserve there: 108 died
   having only ever gone the empty way (§20). Real desert ants remember the
   direction that paid.
3. **Where food lands at home**: at the end of the strip facing the food, a
   median 15 cells east of centre. `NestSite::larder` tracks it; only
   `HUNGRY_HOME` reads it.
4. **Lab deliveries are 86% churn** (the nest lane): read net food into home,
   never `deliveries`.
5. **Latent: a scout that has given up is released only by a nest contact**
   (`scout_for` resets when `forage_anchor` moves). Where home is not beside
   nest material -- an ant in a nestless box, a nest dug away -- it stays
   homebound for good. It is harmless on both beds, since arriving home means
   touching the nest. `HUNGRY_HOME=tether` already lets go on arrival
   (`HUNGRY_ARRIVED`); the same rule for the scout would change the default,
   so measure it. Found when `an_ant_eats_a_living_worm_...` failed under
   scouting (traced; it was the path, not this, and that test now pins
   scouting off).

## Tools and skills (use these; the names do not say what they answer)

- **`funnel` skill**: invoke it before investigating anything. The method
  behind every finding here.
- **`review` skill** (`scripts/review.py`): post a card whenever a change is
  visible. The owner judges by eye.
- **`scripts/antloop.py`**: the loop ant by ant over `trailfollow
  decisioncsv` traces. It prints the funnel, GOING OUT, who starved and
  where, HUNGRY AT HOME, the time budget and the economy. `--vs base.log`
  pairs starved and net food into home by seed; `--selftest` is the control.
- **`scripts/antidle.py`**: do foragers keep foraging? It prints the loop
  period, the wait at home, and life after the last loop.
- **`scripts/labpair.py`**: two arms of `labforage` logs paired by seed on
  net food into home. Never compare lab `deliveries`.
- **`trailfollow`**, the colony bed:
  - `decisioncsv dtag=` writes the per-decision trace that the scripts read;
  - `gifants framesdir= gifevery= gifstart= gifcount= gifw= gifh= gifat=`
    make card frames, ants magenta when empty and cyan when carrying;
  - the log's `FOOD BUDGET`, `food into home` and `HUNGRY AT HOME` lines
    carry the economy.
- **`labforage`**, the lab box: its `SUMMARY seed=` line.
- **`scripts/deadendindex.py --touching`**: before opening a PR.
- **`scripts/branchcheck.sh --who-touched src/sim/creature.rs`**: before
  editing the ant, since other lanes work in it too.
- **`Reports/instruments.md`**: grep it before building any new harness.

## Commands

The colony bed. Run from anywhere; keep the env exactly this. Run three
batches of 8 seeds in parallel; one arm takes about a minute.

```
export RAYON_NUM_THREADS=1 PIXEL_PHYSICS_COLONY_SPACING=2 PIXEL_PHYSICS_STACK_DEPTH=4 PIXEL_PHYSICS_BUD_SITE=nest
B="mode=gap gate=shipped frames=24000 ants=20 relay=60 near=10 food=400 refill=400 stop=6000 layfrom=founders arms=self gaps=90"
./trailfollow $B decisioncsv dtag=mine seeds=8 seed0=1 > mine-1.log   # and seed0=9, seed0=17
# decision CSVs land in /tmp as trailfollow-decisions-seed<S>-gap<G>-self-<dtag>.csv
python3 scripts/antloop.py <csv dir> --log mine.log --vs base.log   # --vs pairs by seed
```

The lab box. It **must run from the repo root** (it reads `assets/`
relatively). 12 seeds, 4 at a time, takes about 15 minutes:

```
RAYON_NUM_THREADS=1 labforage scenario=played_bed frames=120000 seed=N
```

Read the `SUMMARY seed=` line: net food into home is `deliveries - pickups_at_nest`.

**Identity first:** every new switch, unset, must reproduce the default line
for line. Diff the logs with the `trailfollow:`, `breadoff=`, `ant.ron:` and
`DECISIONS:` lines filtered out.

## Traps that cost time here

- **Stale binaries.** Rebuild before any measurement. Build only the two
  harnesses with `cargo build --release --example trailfollow --example
  labforage` (2–4 minutes; all examples take ~20). Copy the binary into your
  scratch run directory so a rebuild cannot change a run under you. `grep -c`
  the binary for your switch's env name to prove it is in.
- **Don't edit source while a build runs.** The build reads it part-way
  through and fails with a type error that isn't real.
- **Never use `pgrep -f` / `pkill -f`.** It matches your own shell and kills
  it. Use `pgrep -x <exe>` or kill by PID.
- **Key every parse by seed, not by position.** A negative `carry->nest`
  once dropped a line and shifted every pairing after it. Also,
  `carry->nest` is cell-steps homeward, not food.
- **The "west of the nest" death class includes the nest's own west end**
  (the band is ±26). Trace the individuals before believing where they died.
- **Watch every guard go red** by removing its mechanism, and write
  predictions into your notes *before* each run. About half of this lane's
  predictions were wrong, and the record is what kept the conclusions
  honest.
- **Messages from other sessions may not reach you**; files are the channel.

## Predictions (written before each run)

| # | run | prediction | right? |
|---|---|---|---|
| | | | |
