# Nest race handoff (2026-10-07)

What the Nest race lane learned getting the lab colony to live *in* its dug
nest, copied here from the project's shared folder because the next session
reads only the repository. Paths in these files that start
`/mnt/project-files/` point at the old shared folder and are gone; the
write-ups named below are the copies that matter, and every raw run was local.

## Where it stands

- **The goal** (owner, 2026-10-03): a lab colony with a stable population,
  separate chambers for brood and workers, and ants that spend their time in
  the nest. **Still one room**, and most ants still live in the spoil mound
  over the door.
- **The stack is on `main` with every switch off** (this PR). With no
  `PIXEL_PHYSICS_*` set it reproduces `main` exactly: seed 1, 60k frames,
  `colony.csv`, `ledger.csv`, `cells.csv.gz`, `events.txt`, `brood.csv`,
  `feeds.csv`, `cuts.csv` byte-identical, and every `stats.csv` column `main`
  has identical.
- **The stack tested** (heap 90, `foodgap=90`, a harness-only argument):
  ```
  PIXEL_PHYSICS_NEEDS_FIRST=on,backfill PIXEL_PHYSICS_CARRY_HOME=on PIXEL_PHYSICS_DOOR_COLUMN=on
  PIXEL_PHYSICS_LAY_BAR=body PIXEL_PHYSICS_NEST_STORE=on,pick=20,jaws,sky,meal,smell=10,edible
  PIXEL_PHYSICS_WAY_FOOT=on RAYON_NUM_THREADS=1
  cargo run --release --example deeptrace -- scenario=nest_goal founder=evolved ants=0 hungry=1 dig=1 foodgap=90 frames=300000 seed=N out=DIR
  ```
  Which of these go on by default is a second PR, after 12 seeds at heap 30
  and 90 paired against `main` by seed.

## What was found, newest first

| Finding | Measured how | File |
|---|---|---|
| **Births are capped by egg space, not food**: an ant that can afford to lay is refused for want of a free cell beside it (`brood::pile_site`) over 99.8% of the time. The brood column *is* the laying cap; carrying brood away frees cells and eggs rise 1.5-2x, then larvae starve | counter, heap 90 s1-4 | `brood-carry-egg-space-2026-10-07.md` |
| **The store counted crumbs it could not feed** (`NEST_STORE`'s `edible`): crumbs worth 0-15 J kept the store reading "can feed" and held hungry ants in. Deep starvers s1-4: 10/3/77/179 -> 0/2/1/1 | traced, then a paired run | `store-edible-proposal-2026-10-07.md`, its review, `store-edible-results-2026-10-07.md`, `residual-nest-deaths-deep-trace-2026-10-07.md` |
| **The way out ran over the backs of the crowd** (`WAY_FOOT`). Heap 90, 12 seeds: starved lower 12/12 (8,273 -> 1,750), deep starvers 5,165 -> 957, no die-offs, no second entrance at any 5k mark, colony bigger 9/12 | 12 seeds, paired | `way-foot-twelve-seeds-2026-10-07.md`, `way-foot-results-2026-10-07.md`, `way-over-ground-*` |
| Ant-time in the dug nest: 11-13% on the stack against 4-6% on `main`; about a third in mound tunnels and a third on the mound top; about a quarter of ants never enter | ant-time census, heap 90 s1-4 | `findings.md` |
| Retests of older room and brood rules (two crowds, dig modes below ground, brood carry) on the working base: brood floods the floor (the egg-space cap again); the dig-mode starvers were 99% the crumb trap above, so that one is **inconclusive** until rerun with `edible` | paired runs | `brood-carry-egg-space-2026-10-07.md`, `findings.md` |

`scoreboard.md` is the lane's running table of every arm; `findings.md` the
running notes, including the owner's own observations of the pictures (one
giant room under tunnelled mound; the brood column used as a ladder over a
drop; side rooms appearing near 300k).

## Next steps, in the owner's order (2026-10-07)

1. Default-flip PR for the stack (12 seeds each heap, paired against `main`).
2. Let food, not egg space, cap births (fix the laying cap before any brood
   carrying).
3. Why most ants live in the mound: no rule pulls an idle ant inward. Try
   local cues (brood scent, depth, crowding) rather than more pulls to
   places; most recent bugs here were a pull misfiring.
4. Retry the room rules (brood carry, dig modes) on top.

## Finishing the default flip (the next session's first job)

The 48 runs below were left going in the old container, at
`/home/claude/runs/ss/{m90,ed90ed,m30,ed30ed}-s{1..12}`. **That container is
not reachable from a new account**, so plan on re-running them. Each is about
20 minutes of one core and costs no tokens while it runs.

```
cargo build --release --example deeptrace
for heap in 90 30; do for s in $(seq 1 12); do
  for arm in main stack; do bash Reports/handoff/nest-race/tools/pair12.sh $arm $heap $s runs & done
done; wait; done          # 24 at a time; throttle to your core count
for d in runs/*-s*/; do python3 Reports/handoff/nest-race/tools/one12.py ${d%/}; done
python3 Reports/handoff/nest-race/tools/second_way.py runs/stack90-s1 50000 100000 150000 200000 250000 300000
python3 Reports/handoff/nest-race/tools/nestpic.py out.png "main=runs/main90-s1@300000,stack=runs/stack90-s1@300000"
```

- `main` is the stack with every switch off, which is `main`'s game
  (identity checked). Use it rather than a `main` build, because only this
  branch's `deeptrace` knows `foodgap=`. Older builds silently ignore it and
  put the heap where they always did.
- **Pair by seed.** Read heap 90 first.
- **The bar to turn a switch on by default** (owner, 2026-10-07):
  - clearly better, or a traced fix of a specific problem;
  - 12 seeds at each heap;
  - no untraced die-offs, and a colony-killing change stays off until its deaths are traced;
  - a smaller colony alone is not a veto;
  - a second reviewer's written yes;
  - CI green.
- **Report deep time as N of M ants**, with the baseline beside it, the door column split out, and a fed/staying check. Say plainly when a number is near zero.
- **Deep trace's written yes** (`second-key-verdict-deep-trace-2026-10-07.md`):
  - `WAY_FOOT` is cleared to go on by default.
  - `edible` goes on once its 12-seed set is in. In that set, count the
    starved deaths with ledger `zone_end == mound_in` per seed, on both arms.
  - The seed-2 mound deaths are traced and are not the store. They were 40
    fed foragers walled into soil pockets in the mound, with no food in
    reach. Tools: `zonedeaths.py`, `moundfood.py`.
  - The rest of the stack has no yes yet: there is no 12-seed table of it
    against `main`.

## Tools (`tools/`)

Python 3, no libraries. Read `deeptrace` output directories.

- `nestpic.py OUT.png "label=RUNDIR@FRAME,..." ...` -- nest pictures over time from `nest_fNNNNNN.txt`.
- `second_way.py RUNDIR F1 F2 ...` -- is the nest open to the sky anywhere but its door (positive control: a run with a known second entrance).
- `pair12.sh ARM HEAP SEED OUTROOT` -- one run of the default-flip comparison (`main` or `stack`).
- `one12.py` -- the per-seed table (ants, starved, deep falls, deep starvers, store bites, digs); reads `hungry.csv` reduced to `hred.csv.gz` (frame,id,hy,zone,outcome).
- `rebuild.py`, `spellfunnel.py`, `storecensus2.py`, `nestdeaths.py`, `hspells.awk` -- Deep trace's census tools behind the edible finding.

Pictures of the 12-seed run and the edible and brood-carry arms are in `pictures/`.
