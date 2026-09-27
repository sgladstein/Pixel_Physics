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
- An ant should breed only at the nest, and in the end *where* it breeds
  should be something a lineage evolves (09-23). `PIXEL_PHYSICS_BUD_SITE=nest`,
  off by default; the bed runs with it on.

## Live question

**Food at home does not reach the hungry.** The whole nest holds a median of
4 food cells. A hungry ant at home stands 11–17 cells from the nearest, and on
about 1 census in 5 there is none at all. Hungry ants at home were beside food
42 times in 285, and ate it 71–84% of the time when they were. Sharing between
ants (`share` / trophallaxis) is about 4% of what a colony eats (§13).

**The candidate:** the social stomach. A returning forager feeds hungry
nestmates directly from its crop before, or instead of, dropping food on the
ground. This is **not** §17f's `hungergate`, a fed carrier that does not
digest its own cargo. That one was inert: it held 1–3% of digestion
(dead-ends, `digest_hunger_weight`). Grep `dead-ends.md` for `share` and
`trophallaxis` before building. Measure it by `antloop`'s HUNGRY AT HOME section and by net food into
home, then re-test `HUNGRY_HOME=tether` on top of it, on both beds and at
140 cells.

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

1. **Food at home to the hungry** (above). The colony is short of transport,
   not of food at the source: the pile refills.
2. **Scouts that pick the dead-end side** waste their reserve there: 108 died
   having only ever gone the empty way (§20). Real desert ants remember the
   direction that paid.
3. **Where food lands at home**: at the end of the strip facing the food, a
   median 15 cells east of centre. `NestSite::larder` tracks it; only
   `HUNGRY_HOME` reads it.
4. **Lab deliveries are 86% churn** (the nest lane): read net food into home,
   never `deliveries`.

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
