# Nest raider: the raw data

The runs behind [`../nest-raider-2026-10-25.md`](../nest-raider-2026-10-25.md),
saved so a later session can re-read them without re-running anything. Paths
inside the logs point at a scratchpad that no longer exists; the file names
here are what they became.

| Folder | What it is |
|---|---|
| `pools/` | Three raider pools as `predtrain` writes them (8 minds each, `NN.raw.txt`). `pool-hunter2-r7` is the one that ships as `assets/minds/centipede_pool.txt`. The two `pool-smell-snp10-*` pools were trained with the nest smell on and `nestpay=10`; they are the lab-grid-4 arms `n1` and `n2` |
| `labgrid1/` | First lab test: blank and untrained-hunter raiders vs the old ant, `spiderlab scenario=lab_mid_fruit`, 2 placements x 3 seeds each |
| `labgrid2/` | Second lab test, 6 seeds, four minds: `blank`, `hunter` (untrained hunter), `tb1`/`th2` (trained lines) |
| `labgrid4/` | Round two, 6 seeds x 300k frames, with `PP_KILLLOG` kill lines: `off` = shipped pool, smell off; `on` = shipped pool, smell on; `n1`/`n2` = the smell-trained pools with the smell on. Deep kills (more than 2 cells below the column's surface) 538 / 78 / 64 / 75 |
| `training/v1`, `v2` | `predtrain` batches for `nestpay` alone (v1 paid on bites, which turned out to be farmable; v2 pays kills) |
| `training/v3` | Smell + `nestpay` batches (`s*` logs) |
| `training/judge/` | Holdout judging of the trained pools against the founder on fresh training beds (the 11/12 and 12/12 wins) |
| `scripts/` | The shell drivers used for the batches, the judging and the lab grid — templates, with scratchpad paths |
| `img/` | Stills; the two 4-panel PNGs are frames 0, 1/3, 2/3 and last of the round-two films. The full GIFs (~30 MB) were not committed |

RESULT and row formats are documented in `examples/spiderlab.rs` and
`examples/predtrain.rs`.
