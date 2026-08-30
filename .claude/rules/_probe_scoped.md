---
paths:
  - "src/sim/structural.rs"
---

# Temporary probe — delete me

**This file is an experiment, not guidance, and it should not survive the
session that added it (2026-08-30).** It exists to settle one question that
`Reports/two-games-one-repo-2026-08-30.md` step 7 depends on and that cannot
be settled by reading documentation:

> Does a `paths:`-scoped rule actually stay out of context when nothing
> matches the glob — **inside a git worktree**, which `CLAUDE.md` mandates
> working in?

Two live Claude Code bugs say it may not: **#16299** (open, with a repro —
scoped rules loading regardless of the glob) and **#23569** (closed as not
planned — under a git worktree the filter is ignored). If either still holds
here, moving `CLAUDE.md`'s evidence narrative behind a `paths:` glob saves
nothing in exactly the sessions this repo prescribes, and step 7 has to use a
routed report instead.

`src/sim/structural.rs` is the glob deliberately: it is a real file, it is
**outdoor-only**, and no lab session has any reason to open it. So a lab
session that finds this text in its context has observed the bug.

**The marker to grep for is `PROBE_MARKER_7F3A9C`.** If you are reading this
sentence and you have not opened `src/sim/structural.rs`, the filter did not
apply — say so, and to whoever is running the evolution-lab program.
