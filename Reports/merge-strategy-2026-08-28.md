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
| Next landing on `main` | p25 **13 min**, median **37**, p75 **126**, p90 **301** |

A landing that falls inside your CI window forces a re-merge. At median
cadence that is roughly a coin flip; at p25 it is certain. On 2026-08-23
there were **39 landings in one day** and the window was never clear.
**This is arithmetic, not indiscipline** — no amount of agent care closes
a 17-minute window against a 13-minute p25.

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

**Documentation and registers are 54% of all conflicted files; source and
harnesses 41%** — and the largest single source file here,
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
| raw diff, in tokens | ~13k | — | — |
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

**A correction, recorded because it is the kind that gets repeated.** The
first pass at this number used `git show --cc`, which suppresses hunks it
judges uninteresting, and reported **3,788 lines**. Measured with `-c`,
which shows all of them, the true figure is **45,496** — twelve times
larger. It did not change the recommendation, but a number that is
arithmetically correct and answers a narrower question than the one asked
looks exactly like a result. See `CLAUDE.md`, *ask what your number counts*.

### 1e. The second-order risk, unmeasured but structural

The top conflict sites are the largest files in the repository:
`open-bugs-handoff.md` at 7,563 lines, `PLAN.md` at 3,707, `README.md` at
2,856. `CLAUDE.md` already spends real space warning agents not to read any
of them whole. An agent resolving a hunk in the middle of one, without
enough context to know what it is looking at, is one bad decision away from
reading 97k tokens to fix four lines. **That has not been observed
happening** — it is the failure the current structure invites, not a
measured cost.

## 2. The plan

### 1. Gate branch size going forward, not just drift

`~1 hour · risk low · scripts/branchcheck.sh`

`branchcheck.sh` already computes `BxF` — *behind* x *files* — which
predicts whether a merge will be laborious. It computes nothing on the
*ahead* side. PR #84 was only 11 behind and re-merged six times, because
it was 39 commits and 137 files; the drift metric could not see it.

Add a forward threshold at roughly **6 commits or 20 files ahead**,
printed by the same `--brief` summary the `SessionStart` hook already
runs. Replayed against the window it fires on all seven re-merging
landings and two of thirty clean ones (§1b).

**Open question:** advisory text in the hook, or a hard `--gate` that CI
fails on? The advisory form is what the drift rule does today — and the
drift rule is the one `CLAUDE.md` records agents ignoring until the hook
started printing it unprompted.

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

**Two known gaps, both real.** Nobody watches the PR, so failed checks
leave it open indefinitely — `branchcheck.sh` already prints "THE PR LIST
IS NOT THE WORK LIST" for exactly this, so detection exists, but a failed
auto-merge is a new way to strand a branch. And auto-merge fires when
*checks* pass, not when the branch is *current*: it does not re-test the
combination, which is the same hole item 5 discusses and which only a
merge queue closes.

**Open question:** land the `CLAUDE.md` edit before or after the owner
flips the settings? Before, and agents are told to arm a feature that is
off. After, and the behaviour change waits on a manual step. Landing them
together is the only ordering that is never wrong.

### 3. Put a Rust build cache in CI

`~1 hour · risk low · .github/workflows/ci.yml`

There is none. Eight jobs, five of which build the project from scratch,
on every push. `ci.yml`'s own header predicts ~9 minutes of wall-clock
from local gate timings (292s release tests, 398s debug, 9s clippy, 135s
`ascii`, 239s acceptance) and the measured runs are 16–18. The gap is cold
builds plus queueing eight parallel jobs. That header already names the
fix: *"if the compute ever matters, a Rust build cache is the lever."*

**This is the only item that shrinks the race itself** rather than
managing its consequences — halving CI against a fixed 37-minute cadence
roughly halves the probability that `main` moves inside the window.

**Open question:** cache keyed per-job or shared? Debug and release
profiles do not share artifacts, so a shared cache may buy less than it
looks. Measure before committing to a shape.

### 4. Stop pouring green test output into context

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

**Open question:** is this worth a rule in `CLAUDE.md`, or a wrapper
script that every gate invocation goes through? A rule is cheaper; a
wrapper is the thing that actually happens.

### 5. Do not re-run the full matrix on a *clean* back-merge

`~30 min, mostly prose · risk medium · CLAUDE.md`

If `git merge origin/main` conflicts nothing, an eight-job rerun buys very
little. Not *nothing*, and this repo has the receipts: `CLAUDE.md` records
two zero-conflict merges that still broke the tree, because `main` had
added generated-file gates while the branch edited their sources.

So grade it rather than switch it off. After a clean back-merge run
`docscheck.sh` — sub-second, and the only thing in the repo that catches
exactly that failure — plus `cargo test --lib`. Reserve the full matrix
for conflicted merges and the final pre-land push.

**Open question, and this is the item I am least sure of:** it trades a
real safety margin for wall-clock, and the failure it exposes you to is
the silent kind. Does the 16 minutes still hurt enough to buy that, once
item 3 has halved it — or once item 2 means no agent is watching it at
all? **This item probably disappears if either 2 or 3 lands**, and that is
an argument for doing it last rather than for doing it.

### 6. Split the append-only registers into one file per entry

`half a day · risk medium · Reports/, scripts/bugindex.py`

`open-bugs-handoff.md` (20 conflicts, 7,563 lines) and `dead-ends.md` (12)
become `Reports/bugs/<letter>.md` and `Reports/dead-ends/<slug>.md`, with
a generated index. Two lanes appending then touch different files and
cannot conflict at all. That removes 32 of the 175 conflicted files.

This gets what a `union` merge driver would get **without union's hazard**
— and `.gitattributes` has already reasoned that through and correctly
excluded both files, because their entries get *revised* in place, not
only appended. Separate files are revisable and still collision-free.
`scripts/bugindex.py` already generates the index and would need pointing
at a directory.

The same move applies to the generated indices themselves: README's table
of contents and the bug index conflict **by construction** when two
branches both regenerate them. Regenerate on `main` after the merge rather
than committing them from a branch.

**Open question, and it is a real objection:** both registers are *read*
by grepping one large file, and `CLAUDE.md`'s routing advice is built on
that — "grep the mechanism, not your subsystem", with measured per-query
token costs. Does a directory of small files break that workflow, or does
`docgrep.py` handle it transparently? **Answer that before touching
anything.**

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

1. **Settings → General → Pull Requests → tick "Allow auto-merge."**
   Reported `false` today; confirm before relying on it.
2. **Tick "Automatically delete head branches"** while there. This quietly
   solves the standing 19 merged-and-deletable branches that no session
   can remove, and stops the count climbing.
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
- **Item 1 is independent of everything** and is the only one that
  attacks branch size rather than merge cost. It stays worth doing
  whatever else lands.
- **Items 4 and 6 are independent** of the merge machinery entirely.
  Item 4 is the only measured pure-token saving in the plan.
- **Item 7 subsumes item 2** and is blocked; item 2 is the reachable
  fraction of it.

## 5. Considered and not recommended

- **`union` merge for the bug registers.** `.gitattributes` already
  reasons this out and correctly excludes `open-bugs-handoff.md`,
  `dead-ends.md` and `wiki/plants.md`, because union silently preserves a
  superseded claim beside its replacement with no conflict marker to warn
  anyone. Item 5 gets the same benefit without it.
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
- **Rebase-based automation of any kind.** `CLAUDE.md` forbids rebasing
  someone else's branch; merge, never rebase. A *local* merge-queue script
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
- That review's independent cadence figure — "1 branch in 3 ever needs to
  pull `main` in" — corroborates §1b from a different direction: 18 of the
  last 48 landings carried at least one back-merge, **37.5%**.
