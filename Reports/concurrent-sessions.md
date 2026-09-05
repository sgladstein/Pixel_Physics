# Concurrent sessions — the failure modes, in full

**Status: living record.** The operative rules live in `CLAUDE.md`
("Working alongside another session"); this holds the *narratives* behind
them — the incidents, their measurements, and the forensics needed to
recognise each one again. Split out 2026-08-25 per
[`claude-md-recommendations.md`](claude-md-recommendations.md) rec 5: a rare
failure mode of one specific manoeuvre does not need to sit in every
session's context in full narrative detail, but it does need to exist
somewhere findable when the manoeuvre goes wrong.

Current as of: 2026-09-05.

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

---

*Sections below moved out of `CLAUDE.md` on 2026-09-05, when its
"Working alongside another session" section was cut from 291 lines to 164.
Every **rule** and every **headline number** stayed there; what is here is the
evidence each was derived from — narrative that every session in the repo was
paying for at launch and that only a session actually hitting the manoeuvre
needs. Nothing below is new.*

## Who can open a pull request, measured rather than assumed

**Who opens it is decided by capability, not by role, and this is measured
rather than assumed.** The rule was nearly written as "a sub-agent opens the
PR", which would have encoded a step that silently does not happen:

| how the session was started | GitHub tools |
|---|---|
| in-process subagent (the `Agent` tool) | **yes** — verified 2026-08-25, a probe called `mcp__github__get_me` and authenticated |
| trigger-fired session (`create_trigger` + `fire_trigger`) | **no** — the trigger stamps its own `allowed_tools`, carrying no `mcp__*` |
| cloud child (`create_session`) | **yes** — verified 2026-08-30: a creature-program lane opened its own PR (#146) unaided, and a second reported holding the tools |

So an in-process subagent normally *can* open its own PR, and a woken lane
cannot — which is the case the "PR list is not the work list" section below
is about. Any session settles it in seconds: `ToolSearch` for
`mcp__github__get_me`. A session without the tools pushes its branch, writes
the PR body to a file on it, and reports the head SHA; whoever coordinates
opens the PR. **Either way the coordinating agent owns the merge.**

`create_trigger`'s `connectors:` parameter is *not* the fix for the woken
lane — it resolves against claude.ai connectors, and the GitHub server comes
from the Claude Code Remote environment instead (checked 2026-08-25: the only
connector installed is Google Drive). Connecting the GitHub App at org level,
as the section below says, is still the real one.

## What no pull requests cost, measured 2026-08-23

What it cost to leave unsaid, measured 2026-08-23: **133 CI runs, every one on
`main` or `master`. Zero on any feature branch, zero from a `pull_request`
event.** No PR ever existed, so the workflow's `pull_request` trigger never
fired and pushes to `claude/**` matched nothing — the first time CI saw a
branch's code was *after* it landed, when a red suite can no longer tell you
whether the branch broke it or the merge resolution did. And a branch nobody
can see is a branch nobody merges: 27 accumulated, ten of them cut in one
fan-out and never once pulled forward.

## Where the 300 landing threshold comes from, and what it cannot see

Every painful merge in that history (3+ conflicts) scored **above 340**; no
clean merge exceeded 1440 and the clean ones reach p90 at **280**. So 300 sits
in the gap rather than on a measured value. `bash scripts/branchcheck.sh`
prints your two numbers -- **`FILES` and `BxF`, which it did not until
2026-08-25** while this paragraph claimed it did. The consequence was not
cosmetic: with no printed operand each reader invented one, and two readings
of the *same* merge scored it **132** and **198**, one counting the branch's
changed files and the other main's. The script now settles it — `files` is
branch-side, `git diff --name-only origin/main...<ref>`, which is the operand
this rule's own reasoning implies (a large `files` means *this branch* has
become more than one feature).

**Read the screen for what it is.** 100% sensitivity is what the numbers
support — every painful merge was above 340, so **at or under 300 you are
safe** — and about 90% specificity, since clean merges reach p90 at 280. It
will therefore fire on roughly one clean merge in ten, and that is fine: the
action it prescribes (merge `main` in) is near-free and worth doing anyway.
Do not "improve" it into a lower bound; that throws away the only half that
prompts action. Two caveats on the provenance, both real: the 49 merges are
reported as "3+ conflicts" and "clean", so **merges with 1-2 conflicts are
unreported** and the classes do not partition; and the threshold was placed
in a gap by eye, not fitted.

**And it answers only one of the two questions.** It predicts *"will this
merge be laborious?"* — it is built from conflict counts. It cannot see
*"will this merge be wrong?"*, and that is not a tuning problem: measured
2026-08-25, two merges scoring 132 and 96 — comfortably "safe" — were
**zero-conflict by `git merge-tree`** and still broke the tree, because
`main` had added generated-file gates while the branch edited their sources.
The second question has its own instrument, below.

**The two terms want different remedies, and this is the part that gets
confused.** If `behind` is driving the product, merge `main` in — that fixes
it in place, costs nothing you were not going to pay at landing time, and
landing does *not* reduce drift: a 337-behind branch that opens a PR still
owes the same 337-commit reconciliation. If `files` is driving it, the branch
has quietly become more than one feature; land it and start another.

## The 188-landing collision census

**Know which files are yours.** Collisions are almost never random — they
land in the same few files every time. Counted over **188 branch landings**
(2026-08-25, `git log --merges` with each merge diffed against its first
parent), here is how many landings touched each file:

| Area | Files, with landings that touched them |
|---|---|
| **Contested — anyone may be in these** | `Reports/open-bugs-handoff.md` **118**, `README.md` **103**, `Reports/README.md` **103**, `src/sim/world.rs` **103**, `examples/filmstrip.rs` **99**, `Reports/dead-ends.md` **79**, `src/render.rs` **70**, this file **66**, `src/app.rs` 51, `PLAN-log.md` 50, `PLAN.md` 41 |
| Plants | `src/sim/plant.rs` 60, `organism.rs` 44, `examples/plant_probe.rs` 44, `wiki/plants.md` 67, `assets/species/*.ron` |
| Structural / destruction | `src/sim/structural.rs` 57, `rigid.rs` 32, `load.rs` 22, `scripts/acceptance.sh` 41, `wiki/structural-collapse.md` 43 |
| Creatures | `src/sim/creature.rs` 39, `brain.rs` 19, `assets/species/ant.ron` |
| Worldgen | `tests/worldgen.rs` 36, `src/worldgen/passes.rs` 27, `params.rs` 18, `assets/worldgen.ron` 15 |
| Fields, fire, weather | `src/sim/field.rs` 34, `fire.rs` 38, `weather.rs` 43, `decay.rs` 19, `src/sky.rs` 11 |
| The sweep itself | `src/sim/parallel.rs` 38, `update.rs` 24, `material.rs` 33, `scheduler.rs` 16 |
| The gnome | `src/sim/player.rs` 45 |

**Two things in that table correct what this file used to say**, and both
were measured rather than assumed:

- It claimed *"everything that has actually collided here collided in
  `src/app.rs`."* `app.rs` is **sixth**, at 51. `world.rs` (103),
  `filmstrip.rs` (99) and `open-bugs-handoff.md` (118) are all far more
  exposed, and none of the three was listed at all.
- The old table named only structural and worldgen, so **every other line —
  plants, creatures, fire, weather, the sweep, the gnome — had no row**, and
  an agent on one of them could not tell whether its files were shared.

`src/sim/liquid.rs` and `chunk.rs` show **0** landings in that window: real
files, currently dormant. A zero here means nobody is in your way, not that
the file does not matter.

`src/sim/liquid.rs` and `chunk.rs` show **0** landings in that window: real
files, currently dormant. A zero here means nobody is in your way, not that
the file does not matter. Recompute the table rather than trusting it — it is
a snapshot, and the command that produced it is `git log --merges` with each
merge diffed against its first parent.

## The roster that was stale within the hour

**A file-ownership split is only as current as your last look at the branch
list.** Read once at session start it is stale within the hour, and nothing
prompts a re-read — the drift check has `branchcheck.sh` nagging for it, this
has nothing, which is exactly why it goes unasked. Measured 2026-08-23 on the
creature line's three-lane split: Lane A fetched the remotes before Lanes B
and C had branches at all, never looked again, and spent the whole session
believing it was the only lane. On that belief it filed four bug entries into
`Reports/open-bugs-handoff.md`, a file the split assigns to **Lane B** — which
had already filed all four, and better, its version of the foraging regression
bisected where Lane A's said "unattributed". Lane B then spent a merge
unifying the duplicates (`e3c5e76`), and Lane A had meanwhile told the owner
those lanes did not exist. **Before writing into a file another lane owns,
re-list the branches.** It costs one command, and the roster you were handed
is a claim about the past, not evidence about who is running now.

**The roster is the narrow case; the general one is that a shared append-only
file must be *read* before it is appended to.** Re-listing branches fixes
staleness of the roster, and two collisions the same day had no roster
confusion in them at all — both were single-owner filings by sessions that
knew exactly who else was running. `Reports/open-bugs-handoff.md` is where
they land, because it is append-only, lettered, and written into by every
line at once:

- Two different bugs were filed as **§Q** — one branch's colony-scene panic
  against `main`'s owner-reported debris needles, which already carried three
  inbound references. Landing them naively would have left two §Q headings in
  one document with those references silently resolving to whichever sorted
  first. The newcomer was renamed §R, its self-references repointed.
- A branch carried a stale copy of **§M still headed OPEN** which `main` had
  since closed. Resolved keep-both, that merge would have re-opened a fixed
  bug and sent the next reader at a generator with nothing to do with it —
  which is the failure §M's own entry opens by warning about.

So before adding a section: **grep the file for the thing you are about to
file** — and for the letter, run `python3 scripts/bugindex.py --check`, which
names both lines when one is used twice (`identifier 'D3' is used by 2
entries`) and is already gated by `docscheck`. Do not check the letter by
eye; that is what the tool is for. And when a merge conflicts there,
ask which side is *newer* rather than which is yours — a stale copy of an
entry the other side has since closed looks exactly like your own work.

## The two collisions in `open-bugs-handoff.md` that had no roster confusion in them

Both were single-owner filings by sessions that knew exactly who else was
running, which is why the general rule is *read a shared append-only file
before appending to it* rather than *re-list the branches*.

- Two different bugs were filed as **§Q** — one branch's colony-scene panic
  against `main`'s owner-reported debris needles, which already carried three
  inbound references. Landing them naively would have left two §Q headings in
  one document with those references silently resolving to whichever sorted
  first. The newcomer was renamed §R, its self-references repointed.
- A branch carried a stale copy of **§M still headed OPEN** which `main` had
  since closed. Resolved keep-both, that merge would have re-opened a fixed
  bug and sent the next reader at a generator with nothing to do with it —
  which is the failure §M's own entry opens by warning about.
