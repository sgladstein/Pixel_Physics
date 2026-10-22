# Foraging loop lane: files (2026-10-22)

The record of the "Foraging food loop" lane, which compared the new ant with the old ant in the in-run bed and found
the hungry-at-home lesson that became founder **hA1**. Read `foraging-loop-2026-10-22.md` for the findings.

- `foraging-loop-2026-10-22.md`: the report, with every table (8-seed tables first, then the 32-seed correction).
- `founders/`: genomes as `.raw.txt`. `hA1` is the Teacher lane's founder since 2026-10-22 ~11Z; `c1` is the
  founder before it. `s4`, `c6` and `d1-not-recommended` are copies that failed. `h1-noeat` is h1 with Eat removed.
- `patches/`: the scratch code. Every switch is an environment variable and is off when unset, so unset means
  unchanged behaviour. Both patches apply to the compute branch `claude/project-thread-8y5ak0` at `9d19121c`.
  - `old-ant-in-bed-vs-9d19121c.patch`: the old ant in the bed (`PP_NOMIND=1`).
  - `shadow-rim-vs-9d19121c.patch`: the full set. It holds the carry teacher (`PP_CARRY*`, including
    `PP_CARRY_HUNGRYONLY`, which taught hA1) and the shadow teacher (`PP_SHADOW`, the old ant acting while the mind
    is labelled). It also holds the rim refusal and the `PP_DROPWHY` counters, and the body levers `PP_PACK`
    (shipped as the default since PR #794), `PP_NEEDDROP` (no effect on hA1) and `PP_NOADULTFEED`.
- `scripts/`: the lane's runners and analysis, saved as they ran. The paths inside point at the session's scratch
  directory and the `/home/user/cloud` worktree, so edit them before reuse.
  - Runners: `runl.sh ARM SEED FRAMES` is the arm table. `dlogA.sh` writes the DAgger logs for hA1.
  - Copier: `copier_c.py` with the `*.json` configs. `hA1.json` is the one that worked.
  - Scoring: `hs32.py` scores 32 seeds paired by seed, which is the comparison to use.
- `jam-c1-vs-old-seed3.png`, `arms-200k-8seeds.txt` and `old-vs-new-200k-8seeds.txt`: the picture and raw tables
  the report cites.
