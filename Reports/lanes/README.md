# Lane notes — how two agents talk without a human carrying the message

**The problem this solves, measured 2026-08-25.** The docs-audit lane and the
perf lane exchanged two substantive corrections in one evening — a rule that
was too strong, and a counter that was measuring nothing — and **every message
was copied by hand by the owner.** Both lanes could read each other's branches
the whole time. Neither had a place to write.

**One file per lane. You write only your own.** That is the whole protocol, and
it is chosen for one reason: a shared append-only file is the repo's most
reliable source of merge conflicts. `CLAUDE.md` records three in a single
evening, all of them two sessions appending at the same tail. Single-writer
files cannot collide, so this channel costs no merge surface at all.

## Writing

Add to your own file, under today's date. Say what you found, with the numbers
— a lane note is a *finding*, not a status update. If it is for a particular
lane, name it in the heading (`→ perf`), and **put the other lane's name in the
commit subject** so `git log --all --grep=<lane>` finds it.

Nothing here is a substitute for the real record. A finding that belongs in
`CLAUDE.md`, a report, or `dead-ends.md` goes there; the lane note is how the
other lane learns it happened.

## Reading

`scripts/branchcheck.sh --brief` runs at session start (the `SessionStart`
hook) and names any lane note touched in the last 20 commits, so you do not
have to remember to look. To read one that is still on an unmerged branch:

```
git log --all --oneline -- Reports/lanes/          # who wrote what, where
git show origin/<branch>:Reports/lanes/<lane>.md   # read it without checking out
```

That second command is the one worth knowing: **you can read another lane's
note without merging anything**, which is how both corrections in the founding
case were verified before being acted on.

## Housekeeping

A lane that finishes deletes its file in its last commit. A file nobody has
written to in weeks is a lane that ended without saying so — treat it as
history, not as a live correspondent, and check `branchcheck` for whether the
branch still moves.
