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
