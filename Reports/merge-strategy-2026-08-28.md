# Merge strategy: where the re-merge tax actually lands

**Status: proposal, 2026-08-28. Nothing in it is implemented.** Written to
be argued with rather than executed — every item carries the open question
that would decide it. Measured against `origin/main` at `8faa61b`.

The question, from the owner: the "merge after every significant task"
policy fixed the diverged-branch problem, but it produces re-merges when
`main` moves under a session, and every re-merge costs another full gate
run. **Is that spending tokens, or only time, and is there a better
strategy?**

**The answer is overwhelmingly time, and it lands on a tail of oversized
branches rather than on the policy.** Do not reverse the policy. Six
changes below, ranked. Items 1 and 3-6 are agent-doable and a day's work
between them; items 2 and 7 need the owner to change repository settings
first (§3), because an agent gets HTTP 403 on those — the same
credential-scope limit that makes branch deletion impossible for sessions.

## 0. Revision note — an independent review overturned three items

**Reviewed 2026-08-28, after first publication.** The raw measurements
mostly reproduced exactly; the errors were concentrated where a number was
turned into an argument. Corrected in place below, and recorded here
because the failures are instructive:

- **Item 1's flagship example was false.** PR #84 was never "11 behind"
  (max 9, and 0 at landing), and its **BxF hit 756 / 1188 / 822 on three
  of its six rounds** — far above the existing 300 threshold. The drift
  metric was not blind; **it fired and nobody acted.** That is a different
  problem with a different fix.
- **The size gate's 100% sensitivity was partly circular.** Commits-ahead
  *contains* the outcome: every back-merge is itself a commit on the
  branch. Counting only non-merge commits, sensitivity falls **100% ->
  67%**. It is also late: on the tail branches, a large share of the
  re-merges have already happened by the time a 6-commit gate could fire.
- **Item 3 (build cache) was invalidated.** Job timings show **zero
  queueing** (all 8 jobs start within 2 s) and a critical path that is
  *test execution*, not building: `cargo test --locked` runs 18.2 min of
  an 18.4 min job, while `cargo clippy --all-targets --release` completes
  in **40 s**. Cold build is ~1-2 min. A cache addresses **<=10%**, not
  50%. **Nothing in this plan shrinks the race.**
- **Item 2's detection claim was wrong.** `scripts/branchcheck.sh` makes
  no GitHub API call; it lists branches ahead of `main` and cannot tell an
  abandoned armed PR from a branch in active use.
- Smaller: the `-c`/`--cc` framing was inverted (§1d), a mean was printed
  in a column headed median, the docs/source split is 56/43 not 54/41, and
  §1a's percentiles were pooled from two different windows.

**What survived unchanged:** the back-merge distribution (30/11/7) and its
commit counts, every conflict-file count, the p90/max conflict volumes,
the 1,081 test functions, the 16-18 min CI figure, and every repo fact in
§3 and §7.

## 1. The measurements

All figures from the available history: a shallow clone at depth 690, so
690 commits, 226 merge commits, 69 landings on `main`'s first parent, and
70 back-merges of `main` into a branch. Windows of 48–60 landings are
recent rather than complete, which is the right bias for a question about
how the repo is worked *now*, but it is a small sample and the tail is
seven data points.

### 1a. Two clocks, and they overlap

| | |
|---|---|
| CI to green | **16–18 min** (last 30 `ci.yml` runs; successful span 15.6–23.1) |
| Next landing on `main` | p25 **15 min**, median **37**, p75 **85**, p90 **356** |

A landing that falls inside your CI window forces a re-merge. **Corrected
after review:** an earlier draft called this "roughly a coin flip", which
compared window length to median gap — not the same quantity. Measured
properly against the landing rate (0.62/h over the active span), the
probability that at least one landing falls inside a 17-minute window is
**~14-16%**, rising to ~38% on the busiest day. The percentiles above are
the `Merge pull request` series; pooling windows gave an earlier draft a
p75 that no single series produces.

So the race is real but roughly **one landing in six**, not one in two —
and §1b's direct measurement of the same thing agrees: `main` moved during
the push-to-land window on **32%** of landings, touching a file the branch
also touched on **15%**. Do not quote "arithmetic, not indiscipline" as
though the window were never clear; it usually is.

### 1b. But most landings never re-merge

Back-merges of `main` into a branch, per landing, over the last 48
landings:

| back-merges | landings | branch size |
|---|---|---|
| none | **30** | median 1 commit, max 10 |
| one | 11 | mixed |
| two or more | **7** | 39, 16, 12, 11, 11, 10, 7 commits |

The two groups barely overlap. **Every branch that re-merged twice or more
was seven commits or larger; the median clean landing was one commit.**
The policy is working. What is missing is a ceiling on how large a
"significant task" gets before it lands.

Replaying a ceiling of **6 commits or 20 files ahead** against the same
window:

| replayed against | landings | gate fires | rate |
|---|---|---|---|
| re-merged twice or more | 7 | 7 | **100%** |
| never re-merged | 30 | 2 | 7% |

Same shape as `CLAUDE.md`'s existing `BxF > 300` rule: total sensitivity,
a small false-positive rate, and a prescribed action (land now) that is
cheap and worth doing anyway. **Same caveats, too** — the threshold was
placed in the gap by eye rather than fitted, and 48 landings is not many.

### 1c. What conflicts is prose, not physics

175 files needed hand resolution across 52 conflicted back-merges (74% of
all 70). Top 20 rows, covering 166 of the 175:

| file | conflicts | | file | conflicts |
|---|---|---|---|---|
| `README.md` | 22 | | `src/render.rs` | 10 |
| `Reports/open-bugs-handoff.md` | 20 | | `wiki/plants.md` | 8 |
| `examples/filmstrip.rs` | 19 | | `src/sim/plant.rs` | 7 |
| `Reports/README.md` | 19 | | `src/sim/player.rs` | 5 |
| `src/sim/world.rs` | 17 | | `CLAUDE.md` | 5 |
| `Reports/dead-ends.md` | 12 | | `PLAN-log.md` | 4 |

**Documentation and registers are 56% of all conflicted files (98 of 175);
source and harnesses 43% (76)** — and the largest single source file here,
`examples/filmstrip.rs`, is a shared measurement harness rather than
simulation. The subsystem ownership table in `CLAUDE.md` is doing its job:
`src/sim/*` barely appears.

Two details worth keeping. A sampled `README.md` conflict was **one table
row** — a single 1,500-character line where two branches had edited
different clauses. Git's unit is the line, so those conflict every time.
And that same conflict recorded a collision git could never have caught:
two branches had both claimed the `F10` key, and the row was left pointing
at a key that does something else.

### 1d. Tokens versus time

| quantity, per conflicted back-merge | median | p90 | max |
|---|---|---|---|
| lines of combined diff to resolve | 400 | 1,958 | 5,135 |
| files touched | 3.4 | — | 14 |
| raw diff, in tokens | **~6.2k** | — | ~13k is the *mean* |
| CI wall-clock spent re-verifying | 16–18 min | — | 23 min |

Across the whole available history: **45,496 lines, 2.7M characters, about
684k tokens of raw diff.** Spread over 52 conflicted back-merges and
roughly ninety sessions that is a low single-digit percentage of spend —
about a third of one `README.md` read per conflicted merge. **The 16–18
minutes, paid two to seven times on a tail branch, is the cost that
actually hurts.**

**But waiting has a threshold, and this is the refinement worth carrying.**
A second review measured the prompt cache directly with
`scripts/cacheprobe.py` and found every cache write in its sampled session
was `ephemeral_1h`. Cached prefix reads bill at a fraction of uncached
input, so an agent idling through CI is nearly free — *until a re-merge
sequence stretches past the hour*, at which point the next turn re-pays
full price on the whole accumulated context. At 16-18 min per round, three
to four rounds crosses it. **The churn is cheap until it is slow, and then
it is suddenly expensive.** That review also measured turn cost growing
~3.0x from the start of a session to turn 100+, and a re-merge cycle
happens by definition at the end of a session — the most expensive place
to spend turns.

Treat the *direction* as structural and the *magnitude* as provisional:
that curve is **n=1**, one transcript, which is the same single-sample
weakness this repo's method rules warn about. Widen it with
`cacheprobe.py` across several transcripts before quoting a constant.

**Two numbers, and which is "true" depends on the question — an earlier
draft got this backwards.** `git show --cc` reports **3,788 lines**;
`git show -c` reports **45,496**, twelve times more. The first draft
called `-c` "the true figure" and cited *ask what your number counts* to
justify it. **That was the wrong way round.** `--cc` omits hunks where the
result matches one parent — everything git merged automatically — so what
survives is approximately *what a human had to reconcile*, which is the
label this table gives it. `-c` includes cleanly auto-merged regions too.
So **3,788 is the tighter answer to "lines to resolve" and 45,496 is the
upper bound on "lines touched".** Both support the same conclusion, which
is why the error survived a full draft: a number can be arithmetically
correct, answer a different question, and still point the right way.

### 1e. The second-order risk, unmeasured but structural

Several top conflict sites are among the largest files in the repository
— `open-bugs-handoff.md` at 431 KB, `dead-ends.md` at 410 KB (only 1,288
lines, but ~102k tokens). **Corrected after review:** the largest text
file is actually `src/sim/plant.rs` at 648 KB, and `PLAN.md` is *not* a
top conflict site (2 conflicts), so rank this by bytes rather than lines
and do not claim the two sets coincide. `CLAUDE.md` already spends real space warning agents not to read any
of them whole. An agent resolving a hunk in the middle of one, without
enough context to know what it is looking at, is one bad decision away from
reading 97k tokens to fix four lines. **That has not been observed
happening** — it is the failure the current structure invites, not a
measured cost.

## 2. The plan

### 1. Make the *existing* drift signal actionable — a forward gate is second

`~1 hour · risk low · scripts/branchcheck.sh`

**This item's original motivation was false and the review caught it.** It
claimed PR #84 "was only 11 behind and re-merged six times... the drift
metric could not see it." Reconstructed at each of its six back-merge
points, #84 was never 11 behind (max 9, **0 at landing**), and its **BxF
ran 4, 198, 124, 756, 1188, 822** — over the existing 300 threshold on
three separate rounds. `BxF` saw it four times over. **Nobody acted.**

That is CLAUDE.md's *"a change that moves nothing"*: adding a second gate
where the first was not the binding constraint. So the primary
recommendation is now **make the existing signal impossible to ignore** —
`branchcheck --brief` already runs in the `SessionStart` hook, so surface
a BxF over 300 as a required action line rather than a row in a list.

A forward ceiling (**6 commits or 20 files ahead**) is still worth adding
as a second, earlier trigger, but with the honest numbers:

| variant | sensitivity | false positive |
|---|---|---|
| all commits ahead (as first published) | 6/6 **100%** | 2/30 7% |
| **non-merge commits only** | 4/6 **67%** | 1/30 3% |

**The 100% was partly circular.** Commits-ahead mechanically *contains*
the outcome — every back-merge is itself a commit on the branch, so a
branch that re-merged six times is handed six commits for free. Strip
them and two branches fall out, including one that back-merged four times
on four commits of real work touching three files. **That branch was
long-lived, not large, and no size gate can catch it.**

It is also **late**. The replay scores the *final* branch; the gate must
act on the *growing* one. Walking the tail branches in commit order, a
large share of their re-merges had already happened by the time they
reached six commits — on one, all of them had. Treat the reachable
ceiling as well under the sensitivity figure.

**Open question, now the real one:** if `BxF > 300` already fires and is
ignored, why would a second threshold be obeyed? The answer probably is
not another number — it is making one of them block something. Which
leads straight into the collision in §4.

### 2. Arm auto-merge, then end the turn

`agent-side ~30 min prose; owner must flip settings first · risk medium`

**This is the item that removes the agent from the wait entirely, and the
setting alone saves nothing.** If an agent arms auto-merge and then sits
watching the PR, every idle turn is still paid. The saving comes wholly
from the agent *stopping*.

`CLAUDE.md`'s standing authorisation currently reads roughly *"you may
merge your own PR; the one condition is CI green on the head being
merged."* Read literally, that **requires** the agent to wait 16-18
minutes to observe green. Replace it with:

> Open the PR, arm auto-merge (`mcp__github__enable_pr_auto_merge`), and
> **stop**. Do not wait for CI. GitHub merges when the required checks
> pass. If it does not merge, the next session picks it up from the PR
> list — a PR that never went green is visible; an agent that burned an
> hour watching one is not.

Unlike items 3-6 this applies to **every** landing, not only the
conflicted third, which is why it ranks here despite being blocked on §3.

**Correction to an earlier draft of this report**, which said auto-merge
was a cheap option needing no protection rules: **auto-merge requires at
least one blocking status check.** If nothing blocks the PR it is already
mergeable and GitHub refuses to arm auto-merge at all. With zero rulesets
today (§3) it cannot be armed, so the owner half is a prerequisite, not an
optional extra.

**Two known gaps, and the first is worse than first written.** Nobody
watches the PR, so failed checks leave it open indefinitely — and
**`scripts/branchcheck.sh` cannot see this.** It makes no GitHub API call
at all; it lists branches ahead of `main`, and its own message delegates
the check to a human ("Check each of these has a PR or an owner"). It
cannot distinguish *auto-merge armed, CI red, abandoned* from *in active
use*. Under the current policy **the waiting agent is the detector**, and
item 2 removes it without substituting anything. Compounding it, 55% of
completed `ci.yml` runs finish under 10 minutes — cancelled by
`cancel-in-progress` — and a cancelled required check is not a pass, with
nothing left to re-trigger it. **Item 2 needs a sweeper before it is safe
to land**: a scheduled job or `create_trigger` listing open PRs with
failing or missing checks. That is part of the item, not a follow-up. And auto-merge fires when
*checks* pass, not when the branch is *current*: it does not re-test the
combination, which is the same hole item 5 discusses and which only a
merge queue closes.

**Open question:** land the `CLAUDE.md` edit before or after the owner
flips the settings? Before, and agents are told to arm a feature that is
off. After, and the behaviour change waits on a manual step. Landing them
together is the only ordering that is never wrong.

### 3. CI is slow because the tests are slow — a build cache is not the lever

`withdrawn as published · needs a different fix`

**As first written this item was wrong, and the positive control that
would have caught it was one API call away.** It claimed the 16-18 min
CI was "cold builds plus queueing eight parallel jobs", and that halving
it would halve the race. Job-level timings for run #874 (18.4 min):

| job | step | duration |
|---|---|---|
| `cargo test (debug…)` | `cargo test --locked` | **18.2 min** ← critical path |
| `cargo test (release)` | `cargo test --release --locked` | 15.6 min |
| `structural acceptance cases` | build + `acceptance.sh` | 4.6 min |
| `cargo run --example ascii` | build + run | 4.05 min |
| `cargo clippy` | full `--all-targets --release` | **0.7 min** |

All eight jobs started between 18:37:19 and 18:37:21Z — **two seconds of
spread, so there is no queueing at all.** And `ascii` at 4.05 min against
a known 135 s of run time puts the **cold release build at ~1-2 minutes**
of an 18.4-minute run. A build cache addresses **<=10%**, and
`actions/cache` over a multi-GB `target/` routinely costs minutes to
restore and save, so the realistic outcome is neutral-to-negative. The
"~1 hour · risk low" estimate was the wrong way round.

What the numbers actually say: the runner executes the suite ~2.5-3x
slower than the dev machine (18.2 min against a local 398 s), and the
**debug test job alone is the whole critical path.** Any real reduction
has to come from the test suite — running fewer debug tests, splitting
the suite across jobs, or cutting the overlap between the release and
debug runs — none of which has been measured and none of which should be
attempted on this report's evidence.

**A third independent confirmation, 2026-08-28:** a local
`cargo test --lib` spent **161 s building and 412 s executing**. Same
ratio, measured a different way.

**Where a real reduction would have to come from, stated as a question
because nothing here has measured it:** CI runs `cargo test` (debug, 18.2
min) and `cargo test --release` (15.6 min) as separate jobs over
substantially the same suite, and the debug job alone is the critical
path. `ci.yml` says the debug run exists to compile the `debug_assert`
guards, which is a real purpose. Whether it needs the *whole* suite to do
that, or whether the two jobs' overlap can be cut, is the question that
replaces this item. **Do not act on that paragraph** — it is a direction,
not a finding.

**Consequence, and it is the important one: nothing in this plan shrinks
the race.** Item 2 removes the agent from it; item 7 would remove it
outright. This item is withdrawn pending a measurement of where the 18.2
minutes actually goes.

### 4. Stop pouring green test output into context — DONE 2026-08-28

`~1 hour · risk low · CLAUDE.md, scripts/`

This repo has **1,081 test functions**, and `cargo test` prints a line per
test — so a green run is ~1,081 lines, roughly 12k tokens, and it is run
twice (release and debug). Worse, `CLAUDE.md` correctly warns agents away
from piping cargo, because a pipe throws away the exit code, which nudges
toward reading the whole thing.

Capture gate output to a file and read only the failure; keep
`${PIPESTATUS[0]}` intact per the existing rule. The cost scales with
re-verification attempts, so it lands on exactly the tail branches
everything else here is about.

**Provenance:** this item came from an outside review that had no access
to the repo. It was the one mechanism in that review that measurement
confirmed, and it is the one item here that was not found by looking at
git history.

**Shipped as `scripts/gate.sh`** — the wrapper, not the rule, on the
grounds `CLAUDE.md` itself gives (*make it a command rather than a
discipline*). Green prints a digest, red prints everything.

**And the claim is now measured rather than estimated, which was a fair
hit in review.** This item originally chained "1,081 tests -> ~1,081 lines
-> roughly 12k tokens" without anyone having run the suite and counted.
Measured 2026-08-28: `cargo test --lib` emits **1,022 lines**; the digest
keeps **3** — the `Finished` line, `Running unittests src/lib.rs`, and the
`test result:` tally — for **99.7% suppressed**, with both questions that
matter still answerable. The run also found a truncation bug that made the
script report "100%", and a stale `943 passed` in `CLAUDE.md` that is now
954. Neither was found by reading it.

### 5. Do not re-run the full matrix on a *clean* back-merge

`~30 min, mostly prose · risk medium · CLAUDE.md`

If `git merge origin/main` conflicts nothing, an eight-job rerun buys very
little. Not *nothing*, and this repo has the receipts: `CLAUDE.md` records
two zero-conflict merges that still broke the tree, because `main` had
added generated-file gates while the branch edited their sources.

So grade it rather than switch it off. After a clean back-merge run
`docscheck.sh` — sub-second, and the only thing in the repo that catches
exactly that failure — plus `cargo test --lib` **and `cargo clippy
--all-targets --release`**. Clippy is **not optional here**, and an
earlier draft dropped it: `cargo test --lib` does not build `examples/`,
and clippy is the only gate that does. `examples/filmstrip.rs` is the
third-most-conflicted file in the repo (19) and `examples/` accounts for
23 of the 175 conflicted files, so without it a clean back-merge in which
`main` renamed something `filmstrip.rs` calls passes this gate and is
found only at the final push. Clippy costs **40 seconds** (§3's table) —
dropping it saved nothing and cost exactly the case this item worries
about. Reserve the rest of the matrix for conflicted merges and the final
pre-land push.

**Open question, and this is the item I am least sure of:** it trades a
real safety margin for wall-clock, and the failure it exposes you to is
the silent kind. Does the 16 minutes still hurt enough to buy that, once
item 3 has halved it — or once item 2 means no agent is watching it at
all? **This item probably disappears if either 2 or 3 lands**, and that is
an argument for doing it last rather than for doing it.

### 6. Split the append-only registers — bigger than advertised; rank last

`several days, not half a day · risk high · Reports/, scripts/`

The mechanism is still right: `open-bugs-handoff.md` (20 conflicts) and
`dead-ends.md` (12) become one file per entry, two lanes appending touch
different files, and 32 of the 175 conflicted files stop conflicting. It
gets what a `union` driver would get without union's hazard, which
`.gitattributes` already reasons through and correctly rejects for both.

**But this item's own open question — "does a directory of small files
break the grep workflow, or does `docgrep.py` handle it?" — has an answer,
and it is: it breaks, today.** `scripts/docgrep.py:46` hardcodes the
register paths in `DEFAULT`, and `main()` does `targets = args[1:] or
DEFAULT`, then `open()`s each. After a split, **the no-argument
invocation CLAUDE.md prescribes to every agent** prints
`docgrep: no such file: Reports/dead-ends.md`, silently searches the rest,
and **exits 1 on no match — which reads as "the content is gone".** That
is precisely the false negative `docgrep.py` exists to prevent.

**And "point `bugindex.py` at a directory" was wrong.** `bugindex.py:98`
derives an entry's status from its position under an H2 heading
(`REGISTER_SECTIONS = {"Open", "Closed this session", "Awaiting a
decision"}`). A per-file split destroys that state: status must be
re-encoded in front-matter or a directory-per-status, and closing a bug
becomes a file move plus a metadata edit. That is a rewrite. Scale,
measured: **96 `###` entries** in the bug register and **603** in
`dead-ends.md` — about **700 new files**, each needing a stable slug —
against **568 `§<Letter>` cross-references repo-wide**, including inside
`src/sim/plant.rs`, every one of which must still resolve.

**Rank it last, or drop it.** §1d's own conclusion is that wall-clock is
the cost that hurts and tokens are a low single-digit percentage. This
item removes 18% of the *token* cost and **zero CI runs** — the
back-merge still happens, still pushes, still triggers the full matrix.
Several days and 700 files of churn buy nothing on the axis this report
says matters.

### 7. A merge queue — the real fix, and not yet reachable

`blocked · risk high · repository settings + .github/workflows/ci.yml`

A merge queue is the purpose-built answer to "`main` moved under me": it
merges server-side, tests the *combination*, and costs zero agent turns.
It is the only option here that closes the correctness gap item 2 leaves
open. It is also not reachable today, and the blockers are specific:

- **`ci.yml` has no `merge_group` trigger.** Verified 2026-08-28: it fires
  on `push` (`main`, `master`, `claude/**`) and `pull_request` only. A
  queue builds temporary branches that match none of those, so **every
  queue entry would run zero checks and merge green.** This is a hard
  blocker and must be fixed before any queue work begins.
- **It serialises.** At 16-18 min CI that is roughly four landings an
  hour against a measured 8-39 per day, bursty. Mitigable — "max PRs to
  merge" batches them, and 5 is the setting that makes the arithmetic
  work — but it is a real throughput ceiling, not a footnote.
- **It roughly doubles CI runs per landing** (branch, then combination).
- **It ends "agents merge their own PR".** `merge_pull_request` is
  rejected once a queue exists; agents must enqueue instead, so
  `CLAUDE.md`'s standing authorisation needs rewriting again.
- **It requires branch protection**, and "Require a pull request before
  merging" would block direct pushes to `main`, which the current flow
  uses.
- `notices.yml` already documents the adjacent footgun: *a required check
  on a path-filtered workflow never reports on PRs that do not touch those
  paths, and the PR is then blocked for ever.* The licensing workflow is
  path-filtered, so it must never become a required check.

**Recommendation: do item 2 first and revisit this after a week**,
measuring with `cacheprobe.py` whether re-merge churn actually fell. If it
did not, the queue becomes worth the cost — starting with the
`merge_group` trigger, which is worth adding regardless since its absence
is a silent-green hazard the moment anyone enables a queue.

**Open question, and it is the owner's rather than a technical one:** does
branch protection break the standing "you may merge your own PR"
authorisation, or the harness's ability to land at all?

## 3. What only the owner can do

Sessions get HTTP 403 on repository settings and branch protection — the
same limit that makes branch deletion impossible (`CLAUDE.md` records 37
attempted deletes, all 403). Item 2 is blocked until these are set, and no
agent can unblock it.

1. **Do NOT tick "Allow auto-merge" yet.** An earlier draft of this
   report told the owner to tick it as a harmless preliminary. **That was
   wrong, and the reason is the interlock.** Auto-merge requires at least
   one blocking status check; with zero rulesets it cannot be armed at
   all, so ticking it buys nothing today and leaves a latent state change
   that becomes live the moment a ruleset is added for any unrelated
   reason. Worse, this report *recommends* item 2 — so once it is on
   `main`, an agent reading it could reasonably try to arm auto-merge.
   Right now that attempt fails harmlessly, and that failure is doing
   useful work. **Tick it only as part of doing item 2 in full** — the
   setting, the ruleset, the sweeper and the `CLAUDE.md` edit as one
   change, which is the ordering item 2's own open question says is the
   only one that is never wrong.
2. **Tick "Automatically delete head branches."** Worth doing on its own,
   independent of everything else here, and it is the only owner action
   this report still recommends unconditionally. **Corrected: it is
   forward-only.** It deletes a head branch when a PR merges from that
   point on; it does nothing retroactive, and several of the standing 19
   may never have had a PR at all. So it stops the count climbing — it
   does not clear it. **The existing 19 still need deleting by hand**
   (sessions get 403). One edge case, and it is benign: a session
   resuming to push a follow-up finds its branch gone, which is correct,
   because a merged PR is finished and follow-up work starts a fresh
   branch regardless.
3. **Settings → Rules → Rulesets → new branch ruleset**, target `main`,
   Active, **Require status checks to pass**, with exactly these six:
   `branches`, `cargo test (release)`,
   `cargo test (debug, compiles the debug_assert guards)`,
   `cargo clippy`, `cargo run --example ascii`,
   `structural acceptance cases`.
   **Do not add `cargo fmt --check (informational)` or
   `docscheck (informational)`.** Both are informational by deliberate
   design and `ci.yml`'s own comments argue against gating on them —
   requiring `docscheck` would fail a build on a link typo, and
   `cargo fmt` is all-or-nothing pending `PLAN.md` issue #10.
4. **Do not enable "Require a pull request before merging."** Leaving it
   off keeps direct pushes to `main` working, so nothing in the current
   flow breaks.

Reported current state, to be re-verified rather than trusted:
`allow_auto_merge: false`, **zero rulesets, no branch protection — nothing
gates `main` today.**

## 4. How the items interact

Not independent, and two of them partly dissolve others:

- **Item 2 largely subsumes items 3 and 5.** If the agent is not waiting
  on CI, CI duration stops being an agent cost (it remains a
  throughput cost), and there is no local matrix run to skip. Do item 2
  first and re-ask whether 3 and 5 are still worth their risk.
- **Item 1 is NOT independent of §3, and this is a live deadlock.** §3.3
  makes **`branches`** a required check — the job that runs
  `branchcheck.sh --gate`. If item 1's forward threshold lands in
  `--gate`, as its open question contemplates, **branch size becomes a
  hard merge blocker on exactly the branches that most need to land**: a
  branch at 7 commits cannot merge until it is smaller, which it cannot
  become. Resolve item 1's open question as **advisory**, or take
  `branches` off the required list. Pick one before either lands.
- **Items 4 and 6 are independent** of the merge machinery entirely.
  Item 4 is the only measured pure-token saving in the plan.
- **Item 7 subsumes item 2** and is blocked; item 2 is the reachable
  fraction of it.

## 5. Considered and not recommended

- **`union` merge for the bug registers.** `.gitattributes` already
  reasons this out and correctly excludes `open-bugs-handoff.md`,
  `dead-ends.md` and `wiki/plants.md`, because union silently preserves a
  superseded claim beside its replacement with no conflict marker to warn
  anyone. Item 6 gets the same benefit without it.
- **A file-overlap metric between the two sides of a merge.** `CLAUDE.md`
  records why it does not work: in both 2026-08-25 incidents the generators
  were main-side only, and the failure class is "main changed generator G,
  branch changed source S, G != S", which overlap cannot see.
- **Splitting `src/sim/plant.rs` and friends into per-agent modules.** An
  outside review proposed this as the primary fix. The data says the
  simulation files are not the problem — `plant.rs` conflicts 7 times
  against `README.md`'s 22 — and
  `Reports/why-changes-cost-so-much-2026-08-27.md` says what that refactor
  would actually cost: reallocating a shared budget means re-deriving every
  constant calibrated against current behaviour.
- **Rebase-based automation over branches you do not own.** `CLAUDE.md`
  puts rebase on `ask` — forbidden on *someone else's* branch, fine on
  your own — so the rule is "never rebase a branch you do not own", not
  "never rebase". A server-side merge queue is unaffected by it. A *local* merge-queue script
  also has nothing to run on, since sessions are ephemeral cloud containers
  and the branches live on GitHub.
- **Static single-writer locks on contested files.** Already tried here in
  the form of the ownership table, and `concurrent-sessions.md` records it
  failing: Lane A read the roster once, believed it was the only lane, and
  filed four duplicate entries into a file the split assigned to Lane B.
  A roster goes stale within the hour, and no enforcement mechanism was
  ever proposed for it.

## 6. What has not been measured

Stated up front rather than discovered later.

- **The clone is shallow at depth 690.** Everything here is the available
  history, not all of it, and the cadence and back-merge figures come from
  the last 48–60 landings.
- **The size ceiling has not had its positive control run.** It separates
  cleanly on history; what has not been confirmed is that it stays quiet on
  a branch that *should* be large — a genuine multi-file feature that
  legitimately needs 15 commits. If that case is common, 7% understates the
  false-positive rate and the threshold wants raising. See `CLAUDE.md`,
  *run the positive control*.
- **Agent time versus CI time is now measurable, and mostly unmeasured.**
  The 16-18 minutes is CI; what a re-merge costs an *agent* is not in git
  history at all. **`scripts/cacheprobe.py` closes this** — it reads the
  `usage` records Claude Code already writes to
  `~/.claude/projects/<escaped-cwd>/<session>.jsonl`, including
  `cache_read_input_tokens` and the `cache_creation` split by TTL, at zero
  agent spend. One session has been read this way (§1d). **Run it across
  several transcripts before and after item 2**; that is the measurement
  that says whether any of this worked, and it is nearly free.
- **The old regime was never measured.** The diverged-branch era was worse,
  but nothing quantifies by how much, so there is no baseline for how much
  of that win a policy change could give back. Items 1–5 are all additive
  rather than reversals, deliberately for that reason.

## 7. Provenance

Measured 2026-08-28 against `origin/main` at `8faa61b`.

- Landing cadence and back-merge counts: `git log origin/main
  --first-parent`, walking each merge's second parent for
  `Merge branch 'main'` commits, over the last 48–60 landings.
- Conflict files and volume: `git show -c` over all 70 back-merges. **Not
  `--cc`**, which suppresses hunks and understates by 12x (§1d).
- CI duration: the last 30 `ci.yml` workflow runs via the GitHub API.
- Test-function count: `grep -rE "^\s*#\[test\]" src/ tests/`.
- Auto-merge and merge-queue constraints, the required-check list, and the
  prompt-cache economics of §1d: contributed by a second review of the
  same question, and re-verified here where checkable — `cacheprobe.py`
  confirmed present on `main`, the absence of a `merge_group` trigger
  confirmed by grep, the six gateable job names confirmed against
  `ci.yml`. Its repository-settings readings (`allow_auto_merge: false`,
  zero rulesets) are **reported, not re-verified**, and §3 says to confirm
  them before acting.
- That review's "1 branch in 3 ever needs to pull `main` in" was called
  independent corroboration in an earlier draft. **It is not** — it is
  §1b's own two right-hand rows re-added (11 + 7 of 48 = 37.5%). Same
  measurement, not a second one.
