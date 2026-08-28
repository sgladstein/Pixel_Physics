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

**That rule has a number now, because as prose it was followed 9% of the time.**
Measured 2026-08-27: `docs-audit.md` had gone 18,011 → 47,168 B in one day, and
splitting it by the `→ lane` convention above gave **2 addressed sections
(~1,035 tokens) against 14 of unaddressed work journal (~10,717)**. The content
was good; it had just become a report living in the channel's directory, and the
cost lands on the next reader — ~11,792 tokens to reach ~1,035 tokens of message.

A note carries a **soft cap of **12,000 B** (~3,000 tokens)**, checked by
`python3 scripts/lanecheck.py --check` and reported by `docscheck`. Over it,
promote the findings to a report or to `dead-ends.md`, leave a pointer, and keep
here only what is addressed to another lane. The cap is set with headroom above
`perf.md` (7,661 B), a note doing the job exactly as designed.

**The size finding warns rather than fails, and that is deliberate**: a lane
writes only its own note, so nobody else may trim an oversized one. It is for
its owner. What *does* fail is this sentence's number drifting from the one
`lanecheck.py` enforces — anyone can fix that, and it is what keeps the
documented rule and the enforced rule the same rule.

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
