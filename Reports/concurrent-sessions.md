# Concurrent sessions — the failure modes, in full

**Status: living record.** The operative rules live in `CLAUDE.md`
("Working alongside another session"); this holds the *narratives* behind
them — the incidents, their measurements, and the forensics needed to
recognise each one again. Split out 2026-08-25 per
[`claude-md-recommendations.md`](claude-md-recommendations.md) rec 5: a rare
failure mode of one specific manoeuvre does not need to sit in every
session's context in full narrative detail, but it does need to exist
somewhere findable when the manoeuvre goes wrong.

Current as of: 2026-08-25.

## The `git reset --mixed` that strands stale files

**The manoeuvre.** You need to commit while a contested file holds somebody
else's unfinished work. The procedure in `CLAUDE.md` is: add a worktree at
`origin/main`, re-apply your own change there, verify, commit and push from
it, then bring the main tree's branch pointer forward with `git reset --mixed
origin/main`. That moves the branch and leaves their working tree untouched,
which is the whole point.

**What goes wrong, and it is not obvious.** The reset strands stale files
whenever the main tree was *behind*. It moves the pointer and deliberately
does not touch the working tree, so every file the main tree had not yet
updated now differs from `HEAD` and appears in `git status` as a
modification — one that is really a **revert of the upstream commit it
missed**.

Nobody will recognise it as theirs, because it is not anyone's edit. The next
session to commit that file silently undoes the change.

**This is not specific to `CLAUDE.md`-style contested files.** It hits any
file the branch skipped over. Seen for real on `src/sim/structural.rs`, which
came back as the exact inverse of the commit that had just landed it.

**The forensics, which is why this entry exists.** Afterwards a stale file
and an edited one look identical in `git status`, so the recognition has to
be done by diff rather than by eye:

1. **Before the reset**, note which files are genuinely dirty. This is the
   step that makes the rest cheap, and it is the one that gets skipped.
2. **After it**, diff anything newly modified against the commits you were
   behind by.
3. If a change is the **exact inverse** of one of those commits *and* the
   file was clean beforehand, it is stranded, not yours: `git checkout --` it.

The operative two-sentence version is in `CLAUDE.md`. If you are reading this
because a diff made no sense, step 3 is the test you want.
