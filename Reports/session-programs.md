# Running a program of sessions (coordinator ↔ lane)

**Status: living. Written 2026-08-24; moved out of `CLAUDE.md` 2026-08-25.**

**Read this if you are coordinating other sessions, or were spawned by one.**
If you are a single session doing ordinary work, nothing here applies and
`CLAUDE.md`'s *Working alongside another session* already covers what you
need — that is why this lives in a report rather than in the always-loaded
file. It cost every session ~2,200 tokens of protocol that most of them
never used.

Everything below is a fact about *this* harness, measured, not a guess about
how agents ought to work. It came out of one evening in which a coordinating
session ran seven lanes, merged nine pull requests and lost several hours to
four failures that had nothing to do with the engine.

Written 2026-08-24, from one evening in which a coordinating session ran seven
lanes, merged nine pull requests and lost several hours to four failures that
had nothing to do with the engine. Everything here is a fact about *this*
harness, measured, not a guess about how agents ought to work.

### The sessions can talk to each other — and by default they cannot

**This is the one that cost the most, and it looked like everyone forgetting to
report in.** It was not. The channel was one-directional by construction:
every lane's reply reached the coordinator only because the *owner* copied it
in by hand.

Measured:

| | |
|---|---|
| `SendMessage` to a session id | **fails** — "No agent named … is reachable" |
| `ListAgents` | shows only in-process subagents, never sibling cloud sessions |
| `create_trigger(persistent_session_id=…)` then `fire_trigger` | **works** — delivered immediately, the session wakes and acts |

So the coordinator *can* reach any lane, with the trigger pair above. Use a
poke-only trigger — omit both `cron_expression` and `run_once_at`, then
`fire_trigger` — rather than a timed one; `run_once_at` rejects a timestamp in
the past and a long turn drifts you into that error.

**The lane cannot reach back, and here is why.** A trigger stamps its own
`allowed_tools` onto the session it fires, and that list contains no `mcp__*`
entries — the API says so out loud when you create one: *"this trigger stores
no MCP connectors, so the sessions it fires will run without connector
tools."* A lane woken this way therefore has no `create_trigger`, no
`fire_trigger`, and no `SendMessage` that resolves. It can push commits and it
can write files, and that is the whole of its outbound vocabulary.

Two consequences, both of which should shape how a program is set up:

- **Tell every lane, in its dispatch brief, that its coordinator is session
  `<id>` and that the human is not the postbox.** Even without a live channel,
  a lane that knows it has a coordinator writes *to* that coordinator.
- **Because the return path is files, insist the return path is files.**
  "Reply to me" is not a channel a lane can honour; "commit it and push, then
  tell me the head SHA" is. This is the same rule as *handoffs are committed,
  not replied* below, and it is not a stylistic preference — it is the only
  outbound bandwidth a woken lane actually has.

If a future harness gives lanes the MCP tools, check it rather than assume it:
have one lane try `create_trigger` against the coordinator early, while a
failure is still cheap to route around.

### Coordinators can talk to each other, and that channel is two-way

The asymmetry above is only between a coordinator and the lanes it wakes. Two
*coordinators* — the plant program and the creature line, say — are both
top-level sessions that kept their own MCP tools, so the same
`create_trigger(persistent_session_id=…)` + `fire_trigger` pair works **in both
directions** between them. Neither needs the owner to carry a message.

It went through the owner anyway, all evening, and the traffic was worth
having: the creature-line coordinator sent four items, two of which were
defects in the *plant* program's own record — a bug-register section that was
stale and belonged to the plant integrator, and a review card whose verdict the
plant side was about to misread. A neighbouring program is the cheapest
available reviewer of your blind spots, precisely because it is not inside
them.

**Find your peers with `list_sessions(mine: true)` and read the `tags`.** This
is why the tag convention earns its keep: tag every session with its program
(`plant-program`, `creature-line`) and its role (`integrator`, `lane-W`), and
a coordinator can enumerate its own lanes, and the other programs' coordinators,
without asking anyone. A session with no tags is a session nobody can route to.

Worth sending across programs, and cheap: anything you notice in *their* files,
a shared file you are about to touch that they own, a verdict you think they
will read the wrong way, and the fact that you are winding down. Not worth
sending: status for its own sake.

### The PR list is not the work list

A pull request is what makes work *visible*, and a session that cannot reach
the GitHub API cannot open one — which was every spawned session here. They
push a branch, write the PR body into a file on it, and wait for someone who
never looks.

Package W3 sat at **13 commits ahead, 37 behind**, holding 660 insertions and
a latent bug `main` needed, for hours. It was not stale — 37 is under the
40-commit bar — so `branchcheck.sh` printed it as `ok`, and no PR existed, so
the coordinator's board did not show it either. It was found by enumerating
branches by hand.

`scripts/branchcheck.sh` now reports **UNLANDED** — every branch with `ahead >
0`, whatever its staleness, with a count. Read it every cycle. `ahead > 0` is
the only statement that survives a session dying, a PR never being opened, and
a branch looking healthy.

The structural fix is to connect the GitHub App for the org so lanes can open
their own pull requests; until then the coordinator is the only route to a PR
and must behave like it.

### Doc files are the conflict surface, not source

Five lanes touched plant code in one evening and produced **zero source
conflicts**. All three conflicts were two sessions appending an entry at the
same tail of an append-only record. `.gitattributes` now gives `PLAN-log.md`
a `union` merge; read that file's header before adding to the list, because
`union` is actively harmful on any document whose existing text gets
corrected.

### Give every package a cost fork at dispatch, not when you notice the number

Lanes ran to $52–$170 unprompted. The single time a brief carried an explicit
fork — *build the fix, or write the finding up and stop, but not a half-built
fix with no writeup* — the lane chose in one turn and acted. Put the fork in
the brief.

### Reaching another lane: write a file, do not ask the human to carry it

**Measured 2026-08-25, and it is the same lesson as the section above with the
cost made visible.** The docs-audit and perf lanes exchanged two substantive
corrections in one evening — a rule that was too strong, and a counter that was
measuring nothing, both of which changed `CLAUDE.md`. **Every message was
copied by hand by the owner.** Both lanes could read each other's branches the
whole time; neither had a place to write. Direct session-to-session messaging
is not available across accounts, so the repo is the channel.

`Reports/lanes/<lane>.md` — **one file per lane, and you write only your own.**
Single-writer files cannot collide, which is why it is not one shared document:
the three conflicts recorded above were all two sessions appending at the same
tail. `scripts/branchcheck.sh --brief` names any lane note touched in the last
seven days at session start, so nobody has to remember to look, and
`git show origin/<branch>:Reports/lanes/<lane>.md` reads one **without merging
anything** — which is how both of those corrections were verified before being
acted on. Protocol in `Reports/lanes/README.md`.

A lane note is a *finding*, not a status update, and it does not replace the
real record: what belongs in `CLAUDE.md`, a report or `dead-ends.md` goes
there. The note is how the other lane learns it happened.

### Handoffs are committed, not replied

A reply dies with the container. Ask for the handoff **as a commit**: what
landed, what the next session must not re-derive, and anything measured that
contradicts something already written. Two lanes did this and both documents
are now merged; one of them is what answered a question the owner had asked
twice.

### Re-cut a bar the package cannot reach, explicitly and in writing

T1 could not meet its acceptance bar because `leaf` is 56% of a tree and every
leaf cell is a `Powder` — nothing a fragment ladder can touch. Holding the
mechanism hostage to a bar it structurally could not reach would have been the
default. Naming a narrower bar (*a landed piece survives as a piece, and the
owner has seen it*), saying why, and landing the work was right. State the
re-cut; do not silently accept a miss, and do not stall.

### Verify what a lane relays, including a lane correcting you

Two relayed claims were stale and checking took seconds — a lane reported
blocked on something already done, and a suite reported at 42 tests that was
really 44. Relayed status is a hypothesis.

**And the inverse, which is the expensive one.** The coordinator withdrew a
*correct* finding — a genome widening leaking a draw from a shared `Rng` —
because CI was green, and was corrected by the lane holding the measurement.
A coordinator's prior should be that **the session with the measurement is
right**, and overruling it needs a measurement, not an argument. See the
green-suite gotcha this produced.