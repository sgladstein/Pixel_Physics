# Recovery of three artifacts that existed on no remote branch

**All three were found on this machine, and everything recoverable is now
pushed.** Run 2026-08-24 against the local clone at
`C:\Users\Scott\Code\Pixel Physics`, in response to a cloud-session audit
that found `Reports/README.md` pointing at work absent from all 49 remote
branches. The cause is the one the brief guessed: untracked git worktrees
under `.claude/worktrees/`, which are pushed nowhere by definition. Nothing
had been cleaned yet.

## What is now on the remote

| # | Artifact | State when found | Now at |
|---|---|---|---|
| 1 | `measurement-under-contention.md`, `scripts/perf.sh`, CLAUDE.md +91 | **committed**, 6 commits ahead of `main`, clean worktree | `origin/perf-lock` (`bdda4a9`) |
| 2 | `performance-audit.md` + 5 harnesses | **never committed anywhere** — untracked files | `origin/claude/perf-audit-recovery` (`f7bebae`) |
| 3 | `plant-branch-angle` | **merged**, ref simply never pushed | already on `main`; see below |
| — | `plant-appearance-design.md` §6a | **uncommitted**, found while searching | `origin/claude/plant-appearance-6a-recovery` (`8c35cff`) |

Rec 12's sole blocker is cleared: `perf-lock` is on the remote.

## 1. perf-lock — FOUND, pushed unmodified

Local branch `perf-lock` at `bdda4a9`, worktree
`.claude/worktrees/perf-lock`, **working tree clean** — nothing uncommitted,
so nothing needed rescuing out of it. Six commits ahead of `main`
(`5cf0470` "Stamp a timing run TRUSTED, or admit it is not" through
`bdda4a9`), forked directly off `0efeb24`, the documentation overhaul's own
last commit. `Reports/measurement-under-contention.md`, `scripts/perf.sh`
and the CLAUDE.md edit are all *in the tree of that branch*, not untracked —
the "untracked" note in the index is out of date by several commits.

`git push -u origin perf-lock`. Pushed as found: no rebase, no squash, no
merge into `main`.

**The CLAUDE.md divergence is real and is now visible to both sides.** That
branch adds 91 lines `main` does not have — `scripts/perf.sh`, the `TRUSTED`
gate, the `sccache` note. `main`'s CLAUDE.md and the working checkout's
CLAUDE.md are different files, and until `perf-lock` merges, any statement
about "what CLAUDE.md says" needs to name which one.

## 2. perf-audit — FOUND, but not where the index says

`Reports/README.md` lists `performance-audit.md` as in flight on branch
`perf-audit`. **It is not on that branch.** `perf-audit` (`bb20167`) is
**zero commits ahead of `main`**; the report was never committed to any
branch, local or remote. It was an untracked file in
`.claude/worktrees/perf-audit`, one `git clean` from gone.

Recovered by copying out — **the originating worktree is untouched** and
still holds its own copies, exactly as found:

- `Reports/performance-audit.md` (17 KB, written 2026-08-19 against
  `bb20167`) — rain forcing whole-screen repaints (~12 ms), the field
  solving most of the world every frame, a still-open gnome stall. It opens
  by naming two measurement traps it fell into first: a loaded machine
  reading 45.6 ms where an idle one read 35.5, and a binary in that worktree
  built from another worktree's source. Both are traps `CLAUDE.md`
  documents.
- `examples/frame_profile.rs`, `perf_counters.rs`, `render_cost.rs`,
  `weather_duty.rs` — the four harnesses the report names as its own.
- `examples/camera_snap.rs` — untracked in the same worktree, same session
  window, *not* named in the report. Recovered on proximity; treat its
  provenance as weaker than the other four.

**Dirty tracked paths, listed rather than committed**, per the brief:
`src/render.rs` (+54/-12) and `src/sim/field.rs` (+22). They are the audit's
own instrumentation — `Renderer::last_full_reason`, and `SOLVED_TILES` /
`READ_TILES` / `TOTAL_TILES` counters in `field.rs` — not feature work.
Captured as `Reports/perf-audit-worktree-src.patch` on the recovery branch
so they survive a machine clean without being committed as though finished.
The patch applies against `bb20167`, not against the branch's own base.

Not built, not `cargo check`ed, not tidied. Those examples were written
against `bb20167` plus that patch and `main` has moved ~300 commits since;
some will not compile as they stand. That branch is a rescue, not a merge
candidate.

## 3. plant-branch-angle — FOUND locally, and it is genuinely merged

The brief was right to withhold judgement, and the answer is the benign one.
Observed, not repeated from `e20e338`:

```
git merge-base --is-ancestor plant-branch-angle origin/main   # exit 0
plant-branch-angle head = 9b0cccc "Seeds stop germinating in the canopy"
```

Its head **is an ancestor of `origin/main`**, so every commit on it is on
`main`. It has no remote ref because the ref itself was never pushed — the
work travelled into `main` some other way. "Merged and never pushed", not
"unpushed work at risk". Nothing to recover from the branch.

Its worktree is `.claude/worktrees/plant-crown` — the directory name and the
branch name differ, which is part of why a name-based search misses it.

**But that worktree was not empty.** `Reports/plant-appearance-design.md`
carried **50 uncommitted lines** — a §6a on no branch anywhere. Nothing
asked for it; the same clean that would have taken the frame-cost audit
would have taken it. Recovered verbatim onto `main`'s copy of the file,
which is byte-identical to the worktree's HEAD version, so the branch is
exactly those 50 lines and nothing else. In summary: `leaf_cluster` 5→10
moved foliage share only 7%→11%, and neither real cause was cluster size —
`thicken` places along `cross_section_axis` while `stem_run` walked the row
with `y` fixed, so leaning trunks mismeasured at 30–70 cells and never
widened (third time that denominator has been wrong, and the first two fixes
changed its scope rather than its axis); and `shade_death` was still charged
for a job four other mechanisms had taken over (0.03 → 9% foliage, 0.003 →
30%, 0.0 → 54% but crowns fuse, run 51→107). It closes with two ways the
*viewing* was wrong — a night render hid the girth, and 4× work was compared
against 1× sheets.

## What else is on this machine

`~/.claude/` still holds session state and is **not** cleaned: 20 plan files
(2026-08-03 to 2026-08-22) and 56 session transcripts for this project, the
oldest dated 2026-08-13. The default 30-day cleanup therefore starts biting
the oldest of them around **2026-09-12**, earlier than the 2026-09-18 figure
`e20e338` quoted for its own session. Anything still wanted from there
should be pulled out before then.

A sweep of every worktree for uncommitted `Reports/` or `wiki/` work found
nothing further at risk. What it did turn up, all benign and all left alone:
`wiki/the-gnome.md` dirty in the shared checkout (another session's live
work), two CRLF-only "modifications" in `.claude/worktrees/docs-overhaul`
(`git diff HEAD` is empty for both), and a stale scratchpad worktree under
`%TEMP%` showing mass deletions against an old base.

## Searches run

`git worktree list` · `git branch -a --list '*perf*' '*branch-angle*'` ·
`git ls-remote --heads origin` · `git stash list` (empty) ·
`git reflog --all` (49 entries naming the three) ·
`git log origin/main..<branch>` for each · `git cat-file -e <branch>:<path>`
for each named artifact · `git merge-base --is-ancestor` for the ancestry
question · `git status --porcelain -uall` in every worktree ·
`find /c/Users/Scott -name ...` for the three filenames (5 hits, all inside
this clone) · `ls` and `grep` over `~/.claude/plans/` and
`~/.claude/projects/`.

## One discrepancy with the brief

The brief said `scripts/docscheck.sh` currently reports three known
collisions in `open-bugs-handoff.md` (§R, §X, §Z). **It does not here.** On
`origin/main` at `e20e338`, and on this branch, docscheck reports `clean`
with no findings at all. Either the cloud session is working from a
different base or those collisions have since been fixed — worth resolving
before anyone treats those three as expected noise, because right now any
docscheck finding at all is new.

*Written 2026-08-24. Nothing in this report was merged into `main`; every
recovery is on its own branch.*
