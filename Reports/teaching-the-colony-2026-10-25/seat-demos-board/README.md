# Seat demos: snapshot of the shared board (2026-10-27)

The six seat-demo threads (queen, keeper, guest, colony mind, enemy, scout;
started 2026-10-25) coordinated through a shared folder outside the repo. This
is a snapshot of its coordination files, taken 2026-10-27 before the project
moved accounts, so the record survives:

- `BOARD.md`: the steps every seat thread followed and the collaboration rules.
- `shared/`: one file per engine piece built once and shared between seats
  (who owned it, its interface, its branch).
- `status/`: each seat thread's last status line, verdict and links.

Each seat's own reviews, plans and results were landed by its thread; see
`seats-overview.md` beside this folder and the seat demo reports in `Reports/`.
Paths inside these files that start `seat-demos/` or `/mnt/project-files/`
refer to that shared folder, which no longer exists.
