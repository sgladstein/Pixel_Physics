# The old shared project folder: what was kept

The old project had a shared folder (`/mnt/project-files/`, about 7,000 files) that every thread wrote into. It stays
behind on the old account. On 2026-10-27, every file in it was compared by content (git blob hash) against every
file on `main` and on the open hand-off PRs (#944-#952). The 5,728 files whose exact content was not already in the
repo were sorted, and the ones below were copied here, **at their original paths under the old folder**.

**Read `Reports/` first.** Many reports here (for example `teacher-lane/dig-on-copy-v2-2026-10-21.md` and
`starve-trace/why-ants-starve-2026-10-22.md`) were landed in `Reports/` by the 2026-10-24 sweep (PR #864) or by
their own thread, after edits. The copy here is the thread's working version, kept because its bytes differ. The
landed version is the one of record. A file here with no counterpart in `Reports/` is a working file: a patch,
script, jobs line, genome, small CSV or picture that the report was built from.

**Kept:** every text file (md, py, sh, patch, rs, ron, jobs, csv, txt, log, json, html) under 300 KB, and every
picture under 1 MB, except in the four bulk folders below.

**Left out** (about 260 MB, still on the old account until it is closed):
- `selection-review/elite1/` per-ant round-95 files (291 CSVs, 113 MB). The summary tables are in
  `Reports/brain-training/teacher-training/` (PR #948 `data-1027/`).
- `plant-food/plantwean3/`, `plantwean4/` per-world files (4,083 files, 120 MB). Summary in
  `Reports/` plant-food report (#863).
- `teacher-lane/nestmaps-1022/` per-world nest maps (414 files, 24 MB). Its README and notes are kept.
- 12 pictures over 1 MB (46 MB): seven nest raider GIFs (30 MB; stills are in #951), plant-community GIFs and some
  large plant PNGs.
- 5 archives and videos (12 MB): `brain-lane/cvs/*.tar.gz`, `training-efficiency/*.tgz`, a Teacher lane mp4.
- 2 text files over 300 KB and 3 `.pyc` files.

`uploads/hearth/` holds the owner's own uploads to the project (a lab chronicle, two screenshots, two CSV
logs) that were not already in the repo. The originals had no extension; one was added from the content type.

## By folder

| folder | files | size | kinds |
|---|---|---|---|
| `ant-cost-review` | 1 | 0.01 MB | .md 1 |
| `body-genes` | 1 | 0.01 MB | .md 1 |
| `brain-lane` | 3 | 0.43 MB | .png 2, .patch 1 |
| `brain-research/pictures` | 6 | 1.10 MB | .png 6 |
| `brood` | 2 | 0.01 MB | .md 1, .sh 1 |
| `colony-war` | 3 | 0.36 MB | .png 3 |
| `colony-war/batch1` | 146 | 0.56 MB | .log 72, .csv 72, .sh 1, (none) 1 |
| `colony-war/batch2` | 242 | 0.99 MB | .log 120, .csv 120, .sh 1, (none) 1 |
| `colony-war/tournament1` | 10 | 0.07 MB | .txt 4, .csv 3, .log 3 |
| `colony-war/tournament2` | 4 | 0.05 MB | .csv 2, .sh 2 |
| `crater-refusal` | 1 | 0.01 MB | .md 1 |
| `lab-perf` | 1 | 0.00 MB | .md 1 |
| `lab-playtest` | 4 | 0.39 MB | .png 4 |
| `lab-ui-redesign` | 3 | 0.64 MB | .png 2, .md 1 |
| `lab-ui-redesign/crisp` | 6 | 1.65 MB | .png 6 |
| `lab-ui-redesign/mockups` | 9 | 1.45 MB | .png 9 |
| `lab-ui-redesign/today` | 10 | 0.79 MB | .png 10 |
| `maze` | 1 | 0.74 MB | .gif 1 |
| `maze/colony` | 1 | 0.00 MB | .md 1 |
| `nest-score/dig-lesson` | 1 | 0.02 MB | .md 1 |
| `nest-score/digloss` | 3 | 1.71 MB | .gif 2, .md 1 |
| `parallel-merge` | 3 | 0.02 MB | .jobs 2, .md 1 |
| `parallel-merge/genomes` | 4 | 0.14 MB | .txt 4 |
| `parallel-merge/scripts` | 1 | 0.00 MB | .sh 1 |
| `perf/genomes` | 1 | 0.02 MB | .txt 1 |
| `plant-community` | 9 | 3.33 MB | .png 9 |
| `plant-community/bedsel` | 3 | 0.62 MB | .png 3 |
| `plant-community/lab-pool` | 1 | 0.72 MB | .png 1 |
| `plant-community/leaf-light` | 3 | 1.50 MB | .png 3 |
| `plant-community/runners` | 169 | 1.62 MB | .txt 69, .log 57, .csv 32, .sh 7 |
| `plant-community/tree-gaps` | 3 | 1.03 MB | .png 3 |
| `plant-food` | 2 | 0.00 MB | .sh 2 |
| `plant-food/plantwean1` | 10 | 0.14 MB | .csv 7, .txt 2, .log 1 |
| `plant-food/plantwean5` | 3 | 0.20 MB | .csv 2, .txt 1 |
| `plant-food/pw3mini` | 3 | 0.00 MB | .py 2, .log 1 |
| `plant-food/trees` | 7 | 1.10 MB | .png 5, .txt 2 |
| `plant-perf` | 1 | 0.03 MB | .md 1 |
| `playtest` | 1 | 0.39 MB | .gif 1 |
| `predator` | 1 | 0.00 MB | .md 1 |
| `predator/nest-raider` | 1 | 0.86 MB | .gif 1 |
| `predator/pools` | 32 | 0.01 MB | .txt 32 |
| `scoring-review` | 4 | 0.01 MB | .csv 3, .md 1 |
| `seat-demos/keeper` | 19 | 0.30 MB | .txt 14, .md 4, .png 1 |
| `seat-demos/scout` | 1 | 0.00 MB | .rs 1 |
| `selection-review` | 2 | 0.01 MB | .patch 2 |
| `selection-review/drift1` | 3 | 0.36 MB | .csv 3 |
| `selection-review/exam` | 6 | 0.09 MB | .txt 6 |
| `selection-review/self1` | 3 | 0.82 MB | .csv 3 |
| `sense-caps` | 9 | 0.03 MB | .py 4, .hist 4, .ron 1 |
| `shade-tolerance/pictures` | 2 | 0.16 MB | .png 2 |
| `shade-tolerance/runners` | 19 | 0.18 MB | .log 6, .txt 6, .csv 4, .ron 2 |
| `soil-physics` | 4 | 0.15 MB | .png 4 |
| `soil-water-perf` | 1 | 0.01 MB | .md 1 |
| `soil-water-perf/pictures` | 3 | 0.13 MB | .png 3 |
| `starve-trace` | 1 | 0.01 MB | .md 1 |
| `starve-trace/come-home` | 1 | 0.01 MB | .md 1 |
| `teacher-lane` | 20 | 0.68 MB | .png 8, .py 5, .md 4, .patch 2 |
| `teacher-lane/cloud` | 13 | 0.11 MB | .txt 7, .csv 6 |
| `teacher-lane/genomes` | 3 | 0.03 MB | .txt 2, .json 1 |
| `teacher-lane/hungry-lesson` | 3 | 0.06 MB | .md 1, .py 1, .patch 1 |
| `teacher-lane/inrun` | 42 | 1.31 MB | .py 19, .sh 9, .gif 6, .patch 5 |
| `teacher-lane/lessons` | 1 | 0.01 MB | .patch 1 |
| `teacher-lane/nestmaps-1022` | 4 | 0.01 MB | .md 3, .py 1 |
| `teacher-lane/tools` | 2 | 0.00 MB | .sh 1, .py 1 |
| `teaching-the-colony` | 1 | 0.04 MB | .md 1 |
| `training-efficiency` | 18 | 0.12 MB | .py 16, .html 2 |
| `training-efficiency/results` | 12 | 0.13 MB | .txt 8, .json 4 |
| `uploads/hearth` | 5 | 0.44 MB | .png 2, .csv 2, .txt 1 |
