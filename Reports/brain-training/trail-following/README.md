# Trail following: files (2026-10-22)

The maze thread's record of why trail following hurt the colony, the teacher session that followed, and the
rock-blocks-trail switch that was dropped. Read
[trail-following-2026-10-22.md](trail-following-2026-10-22.md) for the findings.

- `genomes/`: genomes as `.raw.txt` (`train::save_genome`'s sparse format, `genome 2797`).
  - Colony arms on c1: `c1-noB`, `c1-ladenB` (B laid only while carrying, emit `crop*12 - 2`), `c1g-noB` and
    `c1g-ladenB` (with the trail graft), `c1ge-ladenB` (the graft gated to empty ants).
  - On hA1: `g-hA1-noB` (crowd trail silenced, **the one to judge**), `g-hA1-ladenB`, `g-hA1-sup` (ladenB plus
    laying less where B is high).
  - Copies that failed: `tA1` (rule 1), `oA1` and `oA2` (rule 2).
  - The maze lessons: `c1-trail-graft-u15` (the graft delivered to the Teacher lane) and
    `c1-trail-graft-laden-gated`.
  - The maze training runs: `maze-only-g34`, `suite-maze-dig-fetch-g45`, and `h1-stand` (the one-link mutation the
    maze needs).
- `patches/trail-teacher-vs-ee38ae5b.patch`: the scratch code. It applies to the compute branch
  `claude/project-thread-8y5ak0` at `ee38ae5b` and is never for `main`. Every switch is an environment variable
  and is off when unset.
  - `PP_DLOG`: the per-decision log, with `PP_DLOG_MOD` sampling and the trail columns. Builds on the starve-trace
    lane's patch.
  - `PP_PHERMAP`: A and B field dumps per window.
  - `PP_TRAIL_LABEL=hp[:spread]`: the trail teacher's label in copy-log field 16. With `PP_TRAIL_OUT`, it is rule 2.
  - `PP_TRAIL_ACT`: the teacher acts.
  - `PP_COPY_SAMPLE`: the hungry lane's sampler, ported.
- `scripts/`: saved as they ran. Paths point at the session's scratch directories (`/tmp/claude-0/...`), so edit
  them before reuse.
  - Runners: `run.sh` (a traced colony), `judge.sh` (a lean colony for 32-seed arms), `dlogT.sh` and `dlogO.sh`
    (DAgger logs for rules 1 and 2).
  - Copier: `copier_t.py`, the foraging lane's `copier_c.py` plus `target_only`.
  - Analysis: `summ.py` (32-seed arm table, paired counts), `ana1-4.py` (trace census), `deathtrace.py`,
    `emptytrace.py`, `pher.py`, `pair.py`, `fig.py`.
  - Graft builders: `graft.py`, `gate.py`, `gate3.py`.
- `graft-ants-stay-home-seed6.png`: every ant's position over frames 0-3,000 on seed 6, with c1 above and the
  graft below.
- `trail-fields.png`: the maze arena's trail as laid, then after 144 and 1,800 frames of spreading, through rock and with rock blocking it.
- `trail-walls-labbench-2026-10-22.txt`: the rock-blocks-trail switch in the played lab box.
