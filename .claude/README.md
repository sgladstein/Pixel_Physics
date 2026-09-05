# `.claude/` — the Claude Code configuration, and why it is tracked

Until 2026-08-25 `.gitignore` ignored this whole directory, on the reasoning
that none of it is project content. Five files were force-added anyway — the
review skill, its CATCHUP note, three workflow harnesses — because they are, so
the exception got re-argued every time somebody added one, and anything nobody
thought to force-add simply vanished.

**Most development now happens in cloud containers that are destroyed after the
session.** Configuration that is not committed does not merely live on one
machine: for those sessions it does not exist. That, plus Claude Code's own
split (`settings.json` shared, `settings.local.json` per-machine), is why the
blanket ignore was reversed. `worktrees/` and `*.local.json` are still ignored
by name.

## What is here

| Path | What it is |
|---|---|
| `settings.json` | the shared project baseline: the SessionStart hook, and the permission allow/deny/ask lists |
| `rules/` | gotchas scoped to one part of the tree, loaded **only** when a matching file is read — see below |
| `skills/review/` | the owner's visual review queue — post a rendered artifact, collect a verdict later |
| `workflows/` | multi-agent harnesses, including the ten-agent census that produced `Reports/dead-ends.md` |

`settings.local.json` is yours, per-machine, and gitignored. Put anything
personal there; it overrides this file.

## The SessionStart hook

Runs `bash scripts/branchcheck.sh --brief` and puts the result in context
before the session acts. **This exists because the convention did not work.**
`CLAUDE.md` has long asked every session to run `branchcheck.sh` when picking up
a branch, and the drift it exists to prevent happened anyway, at scale — a
2026-08-22 census found ten branches sitting at *exactly* 160 commits behind,
cut at the same moment and never once updated. This repo's own stated lesson,
from `scripts/docscheck.sh`'s header, is that a check that runs catches what a
convention does not.

It costs about **430 tokens** of context per session and prints: which branch
you are on with its ahead/behind and changed-file count, the merged/stale/
current counts, and the deepest few branches carrying commits `main` does not
have. `--brief` exists because the full report is a 49-row table — right for a
human auditing the repo, far too much for every session start.

If it is ever noise, delete the `hooks` block; nothing else depends on it.

## The permission lists, and one thing deliberately *not* denied

`allow` is narrow and mostly read-only: the repo's own gates (`cargo test`,
`clippy`, `check`, the `scripts/*.sh` checks, the two index generators) and
read-only `git`. Nothing that writes to a remote is on it.

`deny` holds exactly one rule, and it is the one `CLAUDE.md` states without
qualification: **`git add -A` (and `--all`) is banned here**, after it once
swept ~1,200 lines of another session's in-progress work into an unrelated
commit. A written rule became an unexecutable one.

**Force-push is on `ask`, not `deny`, and that is a correction to the first
proposal.** The actual rule is conditional — `CLAUDE.md` forbids rewriting
history *on someone else's branch* and defers to repo convention on a branch
you created. A blanket deny would have blocked `--force-with-lease` on your own
branch, which is routine and legitimate. `ask` matches the rule: it stops the
dangerous case and lets the ordinary one through with a confirmation. Same
reasoning for `rebase`, `commit --amend`, `reset --hard` and `git add .`.

Add to these lists rather than widening a pattern: the whole value is that the
list is short enough to read.

## `rules/` — the part of the guidance that is *not* always-loaded

A `.claude/rules/*.md` file with `paths:` frontmatter loads only when Claude
reads a file matching one of its globs. That is how eleven gotchas about
`src/sim/`, `assets/` and `Cargo.toml` came out of `CLAUDE.md` without being
lost: they arrive when the code they are about is opened, and cost nothing in
every other session.

**The criterion for putting a rule here, rather than in `CLAUDE.md`:** does
reading the file precede the mistake the rule prevents? For a gotcha about
`Cell::aux` it does — and an edit is always preceded by a read, so a rule that
only matters when code is *changed* is always in time. For "the app locks its
own exe", which bites at `cargo build` before anything is read, it does not;
that one stays in `CLAUDE.md`.

**A rule here with no `paths:` is loaded at launch exactly like `CLAUDE.md`**
and saves nothing — `Reports/two-games-one-repo-2026-08-30.md` records a whole
proposal that missed this. `scripts/contextbudget.py` counts unconditional
rules inside the gated figure for that reason, and reports scoped ones
separately.

**Do not trust either claim without the instrument.** `bash
scripts/contextprobe.sh <path>` reports what the runtime actually loads and
why, over the `InstructionsLoaded` hook, and `--selftest` is the positive
control: it plants a scoped rule and a nested `CLAUDE.md` and fails unless the
probe reports both. Two upstream bugs (#16299, #23569) once made this
conditionality unreliable and are measured not to reproduce on CLI 2.1.261 —
if a future CLI regresses them, the selftest is what will say so.

## The always-loaded context budget

<!-- BEGIN GENERATED CONTEXT BUDGET -- regenerate with scripts/contextbudget.py --write -->

**Always-loaded floor: ~24,773 tokens** — `CLAUDE.md` at 99,092 B / 1,522 lines, bytes/4.0. Ceiling 28,000 (3,227 under). Plus ~430 for the hook, and the harness system prompt and tool schemas on top; this is a floor.

Paid by **every session, agent and subagent** — ten heads is ~247,730 tokens before any of them reads source.

Consulted by lookup, paid unconditionally: 60% (~14,848 tokens) across Method, Gotchas, Conventions. On demand instead, the floor would be ~8,800. That gap is the work; the ceiling only holds the line.

Cache-prefix churn, distinct versions per day (newest first): 2026-09-05 x1, 2026-09-02 x3, 2026-08-30 x1. Each one is a prefix no later session can share. A running session keeps the version it started with, so the remedy is batching edits into one commit near session end, not editing less.

<!-- END GENERATED CONTEXT BUDGET -->
