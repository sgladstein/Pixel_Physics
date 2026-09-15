---
name: lab-coordinator
description: Run a round of parallel cloud sessions as coordinator, for ANY program in this repo — the evolution lab, the held world (druid), a plant or perf round, anything. Covers spawning lanes so they actually have a repository, choosing each lane's model, reaching one once it is running, and closing the round out. Use BEFORE you spawn or brief another session, whatever the subject: the spawn mechanics are the same for every program and getting them wrong costs the whole round. Also read it if you were spawned BY a coordinator and need to know how to answer one. (The name says lab for historical reasons; the contents are not lab-specific.)
---

# Coordinating a round

A round is: brief several lanes, spawn each as its own cloud session, let them
build, keep them honest, land their work, and write the next round's brief.

**This file is about *running* sessions and nothing else, so it applies to
every program in this repo — not only the lab.** That is worth saying at the
top because the name did not say it and a round was lost to exactly that:
2026-09-14, a coordinator running a **held-world** round did not invoke this
skill, because it is called `lab-coordinator` and the round was not about the
lab. It then made the precise mistake the next section exists to prevent, on
four lanes at once. `CLAUDE.md`'s own rule for rules — *state the rule
universally, put the subsystem in the evidence clause; would an agent working
on weather recognise this as theirs?* — applies to skills, and this one failed
it.

**What the round is *about* lives elsewhere, per program**: the lab in
`Reports/lanes/evolution-lab-coordinator.md`, the held world in README's
`Held world status` and `Reports/held-world-game-concept-2026-09-13.md`. Read
your program's note for the subject; read this for the machinery.

Everything here was paid for. Where a rule cites a measurement, that
measurement is the reason the rule exists, and re-deriving it costs what it
cost the first time.

## Spawn a lane with its repository, or it can do nothing at all

`mcp__Claude_Code_Remote__create_session` **inherits `environment_id` and does
not inherit sources.** That asymmetry is the whole trap: the default that
works is the one you were not worried about.

Measured 2026-09-12 — two lanes spawned in one minute, both with an empty
`/home/user`. One reported `need_input: "no repository found"` and stopped.
The other cloned the public tree read-only through the git proxy, worked seven
minutes, then found it could never push:

```
access denied by the git proxy: sgladstein/Pixel_Physics is not in this
session's authorized repository set                              (HTTP 403)
```

**Pass `source_url` and `source_revision` on every spawn, then check the
returned record has a non-empty `sources`.** The call hands it back, so the
check is free, and it is the only one that runs before the lane burns anything:

```
working:  "sources":[{"git_repository":{"url":".../Pixel_Physics","revision":"main"}}]
dead:     (no `sources` key at all)
```

**A sourceless lane cannot dig itself out.** `add_repo` with `access: "push"`
is the documented fix and a spawned lane cannot use it — the permission
classifier needs a human — and such a lane also has no `mcp__github__*` tools,
so it could not open a pull request even if it could push. Spawn it again
correctly and archive the original, so two lanes are not building one thing.
**Carry the dead lane's findings into the relaunch** and verify them yourself;
the seven-minute lane had already found three stale line numbers in its own
brief.

**Tag every session** with its program and role (`lab-round-31`, `lane-C`).
`list_sessions(mine: true)` plus tags is how you enumerate your own lanes and
find peer coordinators. An untagged session is one nobody can route to.

## Which model a lane gets

**Always pass `model:` explicitly. Never let a lane inherit yours.** Three
workers once silently inherited a premium tier and ran $25–71 each inside
ninety minutes. With Fable at twice Opus, a Fable coordinator spawning
inheriting lanes is that accident with a bigger multiplier.

| | input $/MTok | output $/MTok |
|---|---|---|
| Fable 5.1 `claude-fable-5-1` | $10.00 | $50.00 |
| Opus 5 `claude-opus-5` | $5.00 | $25.00 |
| Sonnet 5 `claude-sonnet-5` | $2.00 | $10.00 |
| Haiku 4.5 `claude-haiku-4-5` | $1.00 | $5.00 |

**Choose on stakes, not on rate.** The premium is proportional to run length,
and run length is *inversely* related to how much of the lane's value is in its
thinking — so the expensive tier is nearly free exactly where it might matter:

One tier step, priced on two real round-30 lanes — the premium to run a lane
on **Fable rather than Opus**:

| lane shape | round-30 lane | Opus | Fable | premium |
|---|---|---|---|---|
| a design report | two soils | ~$11 | $22 | **~$11** |
| a 12-seed sweep | anthill | $105 | ~$210 | **~$105** |

Eleven dollars against a report that sets a round's direction is not a cost
decision. A hundred and five against a sweep Opus demonstrably got right is a
different question. (These assume equal token counts at a different rate,
which is not measured — a more capable model may think longer or arrive
sooner.)

- **Opus 5 is the default.** Anything not obviously covered below.
- **Down to Sonnet 5 only when being wrong is cheap *and loud*** — the lane has
  an acceptance test it runs itself and a bad result shows in minutes. UI work,
  a bounded instrument, a mechanical pass. Its one recorded refusal: **Sonnet
  refuses a brief dense in genetics vocabulary** on a `[bio]` classifier.
  Write such a brief in the world's words, or run the lane on Opus.
- **Never down where the output becomes another lane's premise** — sweeps,
  constants, anything under `src/sim/`, any number that goes into a report. A
  wrong measurement here does not stay in its lane; it is the premise of the
  next three.
- **Up to Fable 5.1 where being wrong is silent and compounds** — design
  direction, a decision with interacting constraints, anything whose failure
  mode is *it looks correct and gets believed*. This repo's worst failures are
  that shape: `phototropism_dir` reshaped a codomain, reallocated a weighted
  sum nobody had priced, sent reproduction to zero, and **every gate stayed
  green but one**. Three architectural levers ranked "very high" on silhouette
  all fired and none moved a pixel. Rough bound: take it when the lane is short
  enough that doubling it costs under ~$25.
- **Haiku 4.5 for in-process lookups only, never a lane.** Its 200K context
  against this repo's ~21.8k-token startup tax leaves little room, and zero
  Haiku lanes have run — a limit, not a finding.

**The vendor's rule is "start on Opus, escalate to Fable when your evals fall
short", and it assumes a cheap retry we do not have.** Their retry is one eval
run; ours is a whole lane and a round slot. Where the retry is expensive and
the failure is silent, start high.

**Budget the round by run length.** Round 30 was ~$294 across seven lanes. The
lane that wrecks a budget is the long one that produces nothing usable, never
the one on an expensive tier.

## Reaching a lane once it is running

**This is the single most-forgotten mechanism in the repo. It is not the
obvious one.**

| | |
|---|---|
| `SendMessage` to a session id | **fails** — "No agent named … is reachable" |
| `ListAgents` | shows in-process subagents only, never sibling cloud sessions |
| `create_trigger(persistent_session_id=…)` then `fire_trigger` | **works** |

The recipe, and every part of it has drawn blood:

1. `create_trigger` with `persistent_session_id: <the lane's session id>`,
   **the whole message in the trigger's `prompt`**, and `initiation` set.
2. **Omit both `cron_expression` and `run_once_at`** — a poke-only trigger.
   `run_once_at` rejects a past timestamp and a long turn drifts you into it.
3. `fire_trigger` **bare** — trigger id only, nothing in `text`.
4. **Check the response reads `session_id: cse_<the lane's id>`.** A new id
   means you spawned a fresh session instead of poking the lane.
5. **One fresh trigger per message.**

A coordinator that put a preamble in `prompt` and the message in
`fire_trigger`'s `text` got, twice, a **fresh sourceless session** (~$0.50,
invisible to `list_sessions`, not archivable) that ran the message as its own
prompt and went idle — while the lane sat waiting. Environments are not the
condition; a recorded poke crossed one and worked.

**And re-read the session id before every poke.** Two messages in round 30
went to the wrong lane because the id was copied from notes rather than
re-checked.

**That guards the address. The claim needs its own re-read.** A poke that
reassigns a file, hands over a task, or says nobody has done X yet is a
statement about what the receiving lane has *not* done — and it is drafted
minutes before it is sent, against a trunk moving ~40 commits a night. Round
36: a reassignment of `src/sim/creature.rs` went out at 19:58 to a lane that
had committed that exact fix at 19:47 and opened its PR at 19:51. Eleven
minutes, and the message read as correct the whole way, because nothing in
the loop read a branch head. So before any poke that moves work between
lanes, run

```
bash scripts/branchcheck.sh --who-touched src/sim/creature.rs
```

and **quote a head SHA rather than your notes**. It fetches first, names
every branch holding an unlanded commit in that path, and prints what landed
on `main` since — which is the half a branch scan cannot see, a lane that
merged an hour ago being 0 ahead. Cheap enough to be unconditional: 2.5 s.

With six lanes running this is the expected case, not bad luck. The window
scales with how fast `main` moves, which scales with the number of lanes — so
a bigger round buys a higher coordinator error rate unless the check is
mechanical.

**The lane cannot reply.** A trigger stamps its own `allowed_tools` onto the
session it fires and that list carries no `mcp__*` entries — so a woken lane
has no `create_trigger`, no `fire_trigger`, and no `SendMessage` that resolves.
It can push commits and write files. That is its entire outbound vocabulary.

Two things follow, and both belong in every dispatch brief:

- **Tell the lane who its coordinator is** (`session_…`) and that the human is
  not the postbox. A lane that knows it has a coordinator writes *to* it.
- **Insist the return path is files.** "Reply to me" is not a channel a lane
  can honour. "Commit it, push, and tell me the head SHA in the note" is.

**Coordinator to coordinator is two-way.** Two top-level sessions both kept
their MCP tools, so the same trigger pair works in both directions with no
human in between.

## When to send a message, and when not to

**A poke is not a ping. It costs the lane a full turn** — it wakes, re-reads
its context, reasons about your message, and acts. So the discipline is not
"message less"; it is **batch, and make each message worth a turn**.

Send, without hesitating:

- **A finding that invalidates their premise.** A brief resting on a number you
  have since found wrong means they are burning hours on a void baseline. This
  is the one where silence is most expensive.
- **A correction to something you told them.** Round 30: a coordinator told a
  lane not to flip a default; the lane flipped it on the owner's verdict and
  was right. One message stopped it second-guessing correct work.
- **A file you are about to write that they own.** Re-list the branches first —
  a file-ownership split is only as current as your last look at it.
- **Anything you notice in *their* files.** A neighbouring lane is the cheapest
  reviewer of your blind spots, precisely because it is not inside them.
- **That you are winding down**, so nothing waits on a session that has stopped.

Do not send:

- **Status for its own sake.** "Still working", "received", "sounds good".
- **Anything they will see on the branch anyway.**
- **A question**, unless you also say to answer by committing a file — you will
  otherwise get no answer and not know why.

**Verify what a lane relays, including a lane correcting you.** Round 30: a
lane reported a review-queue collision that `git rev-list --count` disproved in
one command. Lanes are right often enough to be trusted and wrong often enough
that trusting them unchecked has cost a round.

**A lane's status field describes the turn that ENDED, not what it is doing.**
The only current fact about another session is what is on its branch, and **a
healthy lane is not evidence of durable work**: one lane three hours and $7.85
into a working build had never pushed — `git ls-remote origin 'refs/heads/…'`
returned nothing.

## Cloud sessions, not subagents on this machine

**Spawn lanes as full cloud sessions.** Three reasons, in order of how much
they have cost:

1. **A shared checkout breaks both sessions.** Two sessions in one clone share
   `target/`, so one's half-finished edit fails the *other's* `cargo test` on
   code it did not write, and a running sandbox locks the exe the other needs
   to link. Both happened in one afternoon. A cloud session gets a fresh
   container.
2. **A subagent's transcript lands in your context; a lane's does not.** A
   cloud lane returns a branch and a PR body, and you pay only for what you
   choose to read. A multi-hour subagent would exhaust the coordinator outright.
3. **The token cost is the same.** Measured: `scripts/contextbudget.py` reports
   **~21,860 tokens always-loaded**, and a cloud lane and an in-process
   subagent each pay it once. Container compute is not token-billed.

**So the standing answer is cloud, and here is what would change it** — re-run
`python3 scripts/contextbudget.py` if you suspect it has:

- **~21.8k tokens is the floor price of *any* spawn.** If a job is smaller than
  its own startup tax — grep one file, read one number, check one branch — **do
  it yourself.** You have already paid the tax; a lane pays it again to do 500
  tokens of work. This is the most common waste in a round.
- If that always-loaded figure ever grows enough that a normal lane's startup
  dominates its work, the calculus moves and this section needs re-measuring,
  not re-arguing.

## Closing the round

- **Land every lane's work.** The PR list is not the work list — a finished
  branch with no PR is invisible. `bash scripts/branchcheck.sh --prs` says
  which unlanded branches have none.
- **Run `bash scripts/docscheck.sh` after every merge, unconditionally.** It is
  sub-second and it is the only thing that catches a generated file going stale
  against its source.
- **Check each design report's factual claims, and record the result with the
  model that wrote it.** Round 30 produced one data point for free: a Fable
  lane claimed a review-queue collision that one command disproved. Put the
  checks in the round report and carry a one-line tally in the coordinator
  note, which is capped at 12,000 B. Over several rounds this accumulates into
  real evidence about model choice, with ground truth, at no extra spend —
  which a two-lane A/B cannot give, because outcomes here have enormous spread
  and a design report has no scalar to compare.
- **Write the next round's brief before you finish.** The closing coordinator
  writes it; nobody else has the context. Check every path and constant in it
  against the current trunk rather than from memory — doing that caught one
  handed-over task that was already done and one claim that was wrong.
- **Archive lanes that are finished**, so a later `list_sessions` is a list of
  live work.
