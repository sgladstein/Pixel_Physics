---
paths:
  - "Reports/open-bugs-handoff.md"
---

# Filing in the bug register

Moved out of `CLAUDE.md` on 2026-09-30; the incidents are in
`Reports/claude-md-evidence-2026-09-30.md`, *Working alongside another session*.

- **Read before you append.** Grep the register for the thing you are about to
  file. Two bugs were once both filed as §Q, and a branch carried a stale §M
  still headed OPEN after `main` had closed it.
- **Pick the letter with `python3 scripts/bugindex.py --branches`**, which
  sweeps every fetched branch. **Not `--check`**: it reads one working tree
  and passes on a letter already taken on an unlanded branch (§Z16, measured
  2026-09-12). `--check` keeps its own job, the one `docscheck` gates.
- **On a merge conflict here, ask which side is *newer***, not which is yours.
- After editing, run `bash scripts/docscheck.sh`: the index at the top of the
  file is generated and goes stale silently.
