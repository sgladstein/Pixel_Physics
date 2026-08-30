# Lane I — pricing the three routes past the body stamp

*Brief: price economics §§3.1 (born small), 3.2 (fission) and 3.5 (a
specialised gut) against this engine's numbers, so the owner chooses between
measured options. Cost fork taken in the first turn: **price all three**,
route 3 as itself and routes 1 and 2 by in-process stamp proxy, with what
each proxy cannot see stated rather than assumed.*

## Precondition: PR #142 had not landed

The brief opens by asserting PR #142 landed and telling this lane to confirm
`birth_grant` on `origin/main`. **It is not there.** Measured at session
start and re-fetched: `origin/main` is `6d5cbcf`, PR #142 is **open**, not
merged, and GitHub reports it `mergeable_state: "dirty"` — it has a conflict
against main. The brief says to re-fetch until the grep is non-zero; that
would never have terminated.

**#142 landed later the same session**, while this lane's report was being
written; `main` has since been merged in and the workaround is now redundant
rather than load-bearing. It is recorded because the sweep ran before that,
and because "re-fetch until the grep is non-zero" would not have terminated.

Resolved by standing on Lane A's branch rather than on main: this lane's
branch is cut from `origin/main` with
`origin/claude/creature-lane-a-birth-grant` merged in (one conflict, two
independent appends to `Reports/dead-ends.md`, both kept). **Every number in
this lane is therefore measured against Lane A's tree, which is what the
brief intends.** When #142 lands, this lane's diff reduces to its own files.

## What was found

See `Reports/creature-stamp-routes-2026-08-30.md` for the full account. The
three things a reader should carry away:

1. **The routes were priced against a budget that no longer ships.**
   Economics §3 does its arithmetic against a satiety line of 450
   (`0.5 x 900`). E14 has since cut `start_energy` to 200, putting that line
   at **100**. Every route's headroom fell by 350; the 960-point stamp did
   not move.
2. **Neither stamp route removes the stamp — both defer it**, and the
   deferred instalment is 480 against a bank that caps at 220. At the shipped
   diet the second payment is unaffordable for ever, so route 1 yields a
   species of permanent juveniles and route 2 a colony that stops at twice
   its founder count. **Route 3 is the precondition for both, not an
   alternative to them.**
3. **Creatures cannot grow.** Nothing in `src/sim/creature.rs` ever appends a
   body cell to a live organism. Both stamp routes need a growth verb the
   engine does not have, which neither §3.1 nor §3.2 costs.

## Files this lane owns

- `examples/stamp_probe.rs` — new; the harness. Adds `gut=`, a ceiling priced
  on food **standing in the world** rather than on the material table, and
  `placed=`.
- `Reports/creature-stamp-routes-2026-08-30.md` — new; the report.
- `Reports/instruments.md`, `Reports/README.md` — one line each.
- This note.

Nothing in `src/`, nothing in `assets/`. The package was scoped to need no
source change and it did not need one.

## Gates, all on the merged tree

`cargo test --lib` **1092 passed / 0 failed / 54 ignored** ·
`cargo +1.98.0 clippy --all-targets -- -D warnings` **clean** (CI pins 1.98
and the container ships 1.94, so the default toolchain proves nothing) ·
`cargo build --release --examples` **clean** under `set -o pipefail` ·
`ascii` **31 scenes, 0 skipped** · `acceptance.sh` **all cases** ·
`worldgencheck.sh` **clean** · `docscheck.sh` **clean**.

All re-run *after* merging `origin/main` (which by then carried #142), not
only before.

## Review queue

`20260830T062042271Z-8bcaa0`, board `creatures`, blind A/B: a colony that
bred 79 times and grew 38 -> 106 ants, against the shipped ant that never
bred, same seed and frame. **Neither picture shows an ant**, and the in-frame
counts (186 against 66) are what say they are there to be seen. The question
put to the owner is whether the difference reads at all — because if it does
not, the reproduction economy is invisible whichever route is built.

## Head

**`147af0a3feb7fb8ae125552f3481e51565231c20`** — the last commit carrying work, and the one to hand on.
A note cannot name its own commit, so this deliberately names its parent
rather than a SHA that goes stale the moment the note is amended (which is
how it went wrong the first time here).
