# Documentation audit for agent consumption — 2026-08-24

**Status:** findings and measurements, plus one executed change (the
mechanical half, `fbc10e6`). The recommendations in §5 are **proposals not
yet acted on** and need an owner call.

**The question this asks**, and how it differs from
[`documentation-audit.md`](documentation-audit.md): that report asked *is the
documentation true?* This one asks *can an agent find what it needs, and what
does looking cost?* The owner's framing, 2026-08-24: "we have a lot of
documentation so agents may be missing important information or required to
use a lot of tokens to find the relevant documentation."

**Audited against:** `e20e338` (`main`), after merging it in mid-pass. The
first half of this audit was taken against `1882dc9` — one commit behind — and
that one commit is `e20e338`, *the recovery of the previous documentation
review's method*. The irony is the finding: `CLAUDE.md` warns "know how far
behind you are, before you trust anything you measured on it", and a one-commit
drift was enough to make this report's first draft wrong about its own subject.

**Which `CLAUDE.md` this report measured.** `main`'s. Every figure in §3 is
against `e20e338`. **`origin/perf-lock` carries a different `CLAUDE.md`** — a
91-line section on timing under contention (`scripts/perf.sh`, the TRUSTED
gate, sccache) that exists on no other branch. Until it lands, any claim about
"what `CLAUDE.md` says" has to name which one, and the always-loaded budget in
§3 is the `main` figure, not the merged one.

**Running `docscheck` on `main` will not reproduce this report's findings, and
that is not a disagreement.** `main`'s `docscheck.sh` has **five** checks and
no `scripts/bugindex.py`; this branch adds checks 6 and 7. So the three §4b
identifier collisions are invisible on `main` — they report clean there
because nothing looks for them, not because they are fixed. Confirmed against
`e20e338` directly. On `main`, any docscheck finding at all is genuinely new.

**A correction to this report's own first draft.** It stated that no other
branch existed on the remote. That was false, and instructively so: the
container's clone had fetched only `main`, so `git branch -r` showed two refs.
A full `git fetch --prune` shows **49**. Nothing else in the report depended on
it, but the claim was made from a tool's default rather than from a
measurement, which is the error this repo's method section is largely about.

---

## 0. Read the previous review's method before forming findings

The 2026-08-19/22 overhaul shipped, and its **method** was recovered onto
`main` only on 2026-08-24 (`e20e338`) out of one machine's session state. Four
artifacts, none of which existed in the tree when this audit began:

| File | What it is for |
|---|---|
| [`documentation-audit.md`](documentation-audit.md) | the 21 findings, the agent-consumer re-ranking, **and the cold-agent benchmark at its end** |
| [`documentation-overhaul-plan.md`](documentation-overhaul-plan.md) | what was *done*, in what order, and what was **refused** |
| [`claude-md-recommendations.md`](claude-md-recommendations.md) | 13 recommendations on `CLAUDE.md` as always-loaded infrastructure; twelve landed, only 12 open |
| `.claude/workflows/doc-audit-agent-framing.js` | the ten-agent census harness that produced `dead-ends.md`; three of its inputs are stale 2026-08-19 snapshots |

**Two proposals were considered, approved, and then reversed. Do not
re-propose either without new argument** (`documentation-overhaul-plan.md`
items 11 and 17):

- **A wholesale README milestone reorder** — agents navigate by grep, the
  reorder is a huge diff on a contested file, and *a table of contents buys
  the same navigation for 3% of the churn*.
- **`Reports/archive/`** — with per-report status in the index an agent routes
  by index, not by directory browsing, and the `git mv` breaks `Reports/`
  paths held in other sessions' uncommitted worktrees.

This report honours both. §5c proposes the **TOC**, which is the approved
substitute and was never executed — not the reorder.

## 1. The earlier audit is done, not pending — check this before re-running it

Every headline finding of `documentation-audit.md` was verified against the
current tree and **has been fixed**:

| Its finding | State now |
|---|---|
| R1 — architecture map covered ~half of `src/` | Fixed, and *enforced*: `docscheck.sh` check 2 fails if any module is unmentioned. Clean. |
| R2 — Controls table wrong in 3 places, 6 keys missing | Fixed. `F3`/`F4`/`F2`, `S`, `Enter`, `Y`, `F6`–`F9`, `L` all correct and present. |
| W1/W2 — wiki not as fresh as it claims | Fixed. All 11 pages' claimed dates match their last commit date **exactly**. |
| Link integrity | Still clean; `docscheck.sh` check 1 gates it. |
| Its *method* was unrecorded | Fixed by `e20e338` — see §0. This was the real gap, and it was outside the audit's own scope. |

**`Reports/README.md:26` still describes it as "findings, being executed."**
That line is now itself the stale artifact. Proposed correction: mark it
*executed*, and note that its recurring defect classes are carried by
`docscheck.sh` rather than by the report.

This matters beyond tidiness: an agent that reads that line spends a session
re-executing work that landed.

## 1b. The cold-agent benchmark, re-run — 6 file-opens against a baseline of 8

`documentation-audit.md`'s benchmark was re-run **verbatim** against this
tree (`e20e338` plus this branch's additive changes), fresh `Explore` agent,
search breadth medium, as its own instructions specify.

| | 2026-08-21 baseline | 2026-08-24 re-run |
|---|---|---|
| Questions correct | 3/3 | **3/3** |
| Distinct files opened | 8 | **6** |
| Source files read | 0 | **0** |
| Traps refused | 1/1 | **1/1** |

Routing has **not** regressed across the 258 commits and ~14 new reports since
the overhaul; on this instrument it improved. The trap was refused, and for one
more reason than the baseline run found.

**Which files were opened, and which reasons the trap was refused for, are
deliberately not written here** — see §1c. They are in
`.claude/workflows/doc-audit-benchmark-key.json`, outside the corpus the
benchmark allows itself to read.

**This is the number that answers "are agents missing important information".**
On this instrument, they are not. The token cost of the corpus (§2) and the
always-loaded budget (§3) are real problems, but they are *cost* problems, not
*findability* problems, and the two should not be argued as one.

### 1c. The benchmark was contaminated by its own recording — FIXED 2026-08-25

`e20e338` recorded the benchmark into `documentation-audit.md`, which is
**inside the corpus the benchmark measures**. The questions were there
verbatim, and so was the graded per-question analysis — including the sentence
naming the trap and saying why it was refused.

The re-run agent hit it and said so unprompted: *"flagging it because it makes
this benchmark non-blind on repeat runs."* It reported deriving nothing from
it, and its answers carried detail the recorded analysis did not, so that run
stands. The next one would not have.

This is the repo's own *"a debug readout must not be a function of the thing it
debugs"*, one level up, with the instrument as the readout.

This is the repo's own rule — *a debug readout must not be a function of the
thing it debugs* (`CLAUDE.md`, Method) — with the instrument as the readout.

**The fix, and why it needed no new rule.** The *questions* stay in the report:
the runner pastes them in anyway, so reading them is no advantage, and keeping
them is what makes the numbers comparable across runs. The **result and the
graded analysis** moved to `.claude/workflows/doc-audit-benchmark-key.json` —
which the benchmark prompt already places out of bounds, because that prompt
restricts the agent to `README.md`, `CLAUDE.md`, `PLAN.md`, `PLAN-log.md`,
`wiki/` and `Reports/`, and `.claude/` is none of them. The instrument's own
constraint does the excluding; nothing was added to it.

**Rotating the questions was the alternative and is worse.** It would cost
comparability against the 8 / 0 / 1-of-1 baseline, which is the only thing that
makes the instrument worth running. Same questions, hidden answers, keeps the
number meaningful.

**A leak this report had to fix in itself.** §1b above originally listed the
six files the re-run opened and spelled out both reasons the trap was refused —
in `Reports/`, inside the corpus. Moving the key out of `documentation-audit.md`
while leaving that in place would have been a cosmetic fix that felt complete.
Verified after the change by grepping the allowed corpus for the graded
verdict's phrasing: gone, while the evidence a run is *supposed* to discover
(the index's "superseded by landing", `dead-ends.md`'s "Do not add the table
back") is untouched — those are the trail, not the answer.

## 2. The corpus, measured

```
122 markdown files, 4,309,821 bytes, ~1,077,000 tokens
  Reports    97 files  3,406,094 B  (~851k tok)
  (root)      4 files    654,866 B  (~164k tok)   CLAUDE.md PLAN.md PLAN-log.md README.md
  wiki       12 files    138,307 B  (~35k tok)
  research    3 files     49,854 B
  docs        3 files      6,168 B
```

The six documents `CLAUDE.md` routes an agent to, read whole, cost
**~306,000 tokens**:

| File | Bytes | ~Tokens |
|---|---|---|
| `Reports/dead-ends.md` | 381,944 | 95,486 |
| `Reports/open-bugs-handoff.md` | 343,196 | 85,799 |
| `PLAN.md` | 235,872 | 58,968 |
| `README.md` | 171,194 | 42,798 |
| `CLAUDE.md` | 65,182 | 16,295 |
| `Reports/README.md` | 26,571 | 6,642 |

No agent reads that. So the real question is never "is the document good" but
**"does the document answer a question without being read whole"** — which is
the axis everything below is graded on.

**What already passes that test, and should not be changed:**

- **`dead-ends.md`** — 14 `##` sections, and every entry is a *single line*
  carrying its own `*Re-test when:*` clause. `grep` returns a complete,
  self-contained answer. 382 KB and genuinely cheap to query. This is the
  model the other large files should copy.
- **`instruments.md`** — 25 rows for 25 `examples/` binaries, both directions
  gated by `docscheck.sh` check 5.
- **`wiki/`** — small, coarse-grained, player-visible, dated, all accurate.
  The healthiest surface in the repo.
- **`.github/workflows/ci.yml`** — the reasoning density here is exemplary;
  every gate says what it cost to learn.

## 3. CLAUDE.md: the always-loaded budget

`CLAUDE.md` is auto-loaded into **every session, every agent, every subagent**.
It is 65,182 B / 1,081 lines ≈ **16,300 tokens of every context window.**

**It has only ever grown.** Across its whole history:

```
lines added: 5,475     lines removed: 56     net: +5,419      (98 : 1)
```

| Date | Size |
|---|---|
| 2026-08-16 | 19,706 B / 332 L |
| 2026-08-21 | 37,627 B / 621 L |
| 2026-08-23 | 52,978 B / 869 L |
| 2026-08-24 | **65,182 B / 1,081 L** |

**3.3x in eight days, +9,375 B in the single most recent commit.** Nothing has
ever been consolidated out. This is the owner's "things have continued to get
appended," and it is not an impression — it is 98:1.

Where the budget goes:

| Section | Lines | % |
|---|---|---|
| Method | 305 | 28.2 |
| Working alongside another session | 176 | 16.3 |
| Running a program of sessions (coordinator ↔ lane) | 145 | 13.4 |
| Gotchas that have each caused a real bug | 132 | 12.2 |
| Conventions | 110 | 10.2 |
| Getting the owner's judgement | 55 | 5.1 |
| Where knowledge already lives | 47 | 4.3 |
| Commands | 47 | 4.3 |
| The ethos | 30 | 2.8 |
| What this project is optimising for | 26 | 2.4 |

**Method + Conventions + Gotchas = 50.6%** — an encyclopedia of hard-won
lessons, consulted *situationally*, paid for *unconditionally*. **Running a
program of sessions = 13.4%** that only a coordinator ever needs; a lane pays
it every time and never uses it.

The file already knows this about itself: it opens with a *"Which rules apply
to what you are doing right now"* table that maps situations to its own
sections. That table is a routing layer written because the content is too big
to read — which is the argument for the content being loaded on demand.

**Nothing here says the content is wrong.** Every measurement in it is real
and most were expensive. The finding is about *container*, not *value* — and
the remedy is not this report's to design: `claude-md-recommendations.md`
already specifies it passage by passage and the owner already approved it. See
**§5b**, which supersedes what this report first proposed here.

## 4. The bug register — fixed this pass

`open-bugs-handoff.md`, the file `CLAUDE.md` tells every session to read
"before touching a listed area": 343,196 B ≈ 85,800 tokens, 93 `###` headings.

It is append-only, and **a bug's verdict is written into its own heading
rather than by moving the entry**:

- **74 entries sit under `## Open`. 32 of them — 43% — are headed
  FIXED / CLOSED / RESOLVED / RETIRED / DUPLICATE or struck through.**
- Register-wide (the three register sections): 75 entries → **38 open, 33
  closed, 4 awaiting a decision.** The other 18 `###` headings are enumerated
  sub-items of narrative sections, not bugs.
- 31,603 B (9%) of the file is `## Landing notes` — lane write-ups, not bugs.

**What was done:** `scripts/bugindex.py` generates a status table at the top —
every entry, its status, its line number — so the question costs a few hundred
tokens instead of eighty thousand. Status is *derived from the headings*, never
stored, because the heading is the one place the verdict is already written.

**Why not simply move the closed entries** (recorded so it is not re-proposed):
the file is co-owned by every lane, has no `union` merge in `.gitattributes`,
and reordering 5,000 lines turns every concurrent edit into a conflict.

**The index is not free of merge surface either, and this report first claimed
it was.** The line numbers are the useful part and the expensive one: inserting
an entry shifts every row below it, so two lanes that both file and regenerate
conflict inside the block. What makes it cheap is that the block is *derived* —
a conflict there is never hand-merged. Take either side whole and re-run the
generator; the output is a pure function of the headings. That instruction is
now printed into the block itself.

Two traps the generator had to survive, both recorded in its docstrings:

1. **The line numbers cite a document the table is inside**, so a single pass
   always ships numbers wrong by the height of its own block. It iterates to a
   fixpoint, guarded rather than assumed.
2. **Narrative sections enumerate their findings `### 1.`, `### 2.`, `### 3.`** —
   the same shape a bug identifier takes in a file where real bugs are *also*
   numbered. A naive pass counted them as bugs and reported them as duplicates
   of the genuine §1/§2/§3. The section test is an **allowlist** of the three
   register sections, so a newly appended narrative section is inert by default.

### 4b. The three identifier collisions — RESOLVED 2026-08-24

`docscheck` reports **clean**. All three are fixed, and the three turned out to
need three different remedies, which is why treating them as one job would have
been wrong.

| § | Two entries | Remedy | Cost |
|---|---|---|---|
| X | the *same* bug filed twice — "A desert with no desert plants", 2026-08-22 and 2026-08-23 | older retitled **`X (original).`**, matching the `G (original).` shape already in this file | **zero repointing** — every inbound §X means the desert thing, and the surviving entry is it |
| R | two genuinely different bugs (`filmstrip scene=colony` panic; an ant standing on open water) | newcomer renamed **`R2`** | **zero** — neither had a single inbound reference |
| Z | two genuinely different bugs (the stand reading as one mass; a free particle dropping `Cell::aux`) | newcomer renamed **`Z2`** | **13 references repointed** across 5 files |

**A correction to this report's own estimate.** §4b previously called the §Z
rename "one line in a comment". It was **13** — `src/sim/world.rs`,
`explosion.rs`, `particle.rs` (×4), `creature-review-2026-08.md` (×3) and
`creature-implementation-handoff-2026-08.md` (×4). The earlier figure came from
grepping only `bug Z` in one file and not the `§Z` form anywhere else.

**The danger a blind fix would have hit.** The *other* §Z — the stand bug — has
**17** references of its own, ten of them in `examples/plant_probe.rs`. A
repo-wide `sed` on `§Z` would have silently repointed those at the corpse bug,
turning an ambiguity into a wrong answer. The rename was scoped to the five
files that reference the corpse bug and nothing else, verified both directions
afterwards: `plant_probe.rs` still holds 10 `§Z` and zero `§Z2`.

Renamed by recency, per `CLAUDE.md`'s own §Q→§R precedent — the first claimant
keeps the letter. The §Z2 entry carries a note recording the rename and what was
repointed, so the next reader of an old `§Z` reference elsewhere can resolve it.

`cargo check` passes; the edits are comments and prose only.

## 5. Not yet acted on — proposals

### 5a. The Claude Code configuration surface — LANDED 2026-08-25

**The finding, and a correction to how this report first stated it.** The repo
version-controlled none of its Claude Code configuration: no settings, hooks,
agent definitions or commands, and **zero mentions of any of them across all
122 markdown files.** That much was true. Calling it an oversight was not —
`.gitignore` ignored `/.claude/` deliberately, and its comment named
"per-machine permission settings" explicitly.

But five files under it had been force-added anyway (the review skill, its
CATCHUP, three workflow harnesses) *because they are project content*, so the
exception was re-argued every time and anything nobody thought to force-add
vanished. The owner's question settled it: **most development now happens in
cloud containers that are destroyed after the session, so configuration that is
not committed does not merely live on one machine — for those sessions it does
not exist.** The ignore was inverted to name the machine-local things
(`worktrees/`, `*.local.json`) instead of the whole directory.

**What landed:**

- **`scripts/branchcheck.sh --brief`** — a summary mode. The full report is a
  49-row table, right for auditing the repo and far too much for every session
  start; `--brief` is 15 lines / ~430 tokens.
- **A `SessionStart` hook** running it, in `.claude/settings.json`. This is
  `CLAUDE.md`'s "run `branchcheck.sh` when you pick up a branch" turned from a
  convention into a check that runs — the convention demonstrably failed (ten
  branches at *exactly* 160 behind, 2026-08-22), and this repo's own stated
  lesson is that a check catches what a convention does not.
- **Permission lists** — a narrow, mostly read-only `allow`; a `deny` holding
  the one rule `CLAUDE.md` states unconditionally (`git add -A`, after it once
  swept ~1,200 lines of another session's work into an unrelated commit); and
  an `ask` list for the conditional ones.
- **`.claude/README.md`** explaining all of it, now that the directory is
  tracked and a session will find it.

**One design correction worth recording.** This report first proposed denying
force-push outright. That would have been wrong: the actual rule is
*conditional* — `CLAUDE.md` forbids rewriting history **on someone else's
branch** and defers to convention on a branch you created. A blanket deny would
have blocked `--force-with-lease` on your own branch, which is routine, and
which this very session used repeatedly. It is on `ask` instead, with `rebase`,
`commit --amend`, `reset --hard` and `git add .`. A rule stated unconditionally
can be denied; a conditional one can only be asked.

**And a finding the hook produced on its first run.** Against the real remote:
**37 branches are merged and still standing**, and **14 carry 135 commits
`main` does not have** — `claude/worldgen-caves-r6` at 87 ahead / 546 behind,
`claude/app-performance-review-0p5ix4` at 8 ahead and only 2 behind (current
work, no PR). That is §5f's branch hygiene and the "PR list is not the work
list" problem, measured rather than asserted, by the thing that now runs
automatically.

**A caveat the hook had to be honest about.** `branchcheck` refused to run at
all on a shallow clone — and cloud containers clone shallow, so the check could
not execute in the environment where most sessions now start, which is how a
convention nobody can run stays unrun. The **gate** still refuses (it asks an
ancestry question shallow history cannot answer); the **report** now runs and
labels what it cannot vouch for. Measured at depth 645 here, every ahead/behind
count matched the same figures taken with full history; the unreliable part is
`DATA` classification, where a common ancestor beyond the boundary looks
identical to a genuine orphan.

### 5b. All thirteen CLAUDE.md recommendations have landed — 2026-08-25

**This supersedes what this report's first draft proposed here.** It argued for
moving Method/Conventions/Gotchas into skills, having not seen
`claude-md-recommendations.md`. That document already specifies the same idea
more precisely, at passage granularity, and **the owner already approved it**.
Re-deriving it would have been exactly the waste §1 warns about.

Nine of its thirteen landed. Four are open, and their blocking branches were
re-checked against the remote:

| Rec | What it moves | Deferred behind | State now |
|---|---|---|---|
| 5 | git-reset forensics narrative → a Reports note, keep the recipe | `plant-branch-angle` | **LANDED** — narrative now in `concurrent-sessions.md` |
| 6 | the day/night oscillator rationale → a design report, keep the rule and `field::noon_equivalent_light` | `load-share` | **LANDED** — needed no new home; `plant-economy-rederivation-2026-08-23.md` already held it |
| 7 | the amputation gotcha + liquid-heightfield latency note → `open-bugs-handoff.md` | with 5 and 6 | **LANDED — and the gotcha had gone stale**, see below |
| 12 | cluster Conventions' 93 flat bullets under four sub-leads, no rewording | `perf-lock` | **LANDED — the blocker was never real**, see below |

**Recs 5, 6 and 7 landed 2026-08-25**, and two of the three turned out
differently from what the recommendation assumed — which is the argument for
executing an approved plan by *reading the tree*, not by applying its diff.

- **Rec 6 needed no new home.** It asked for the rationale to move to "the
  relevant design report".
  `Reports/plant-economy-rederivation-2026-08-23.md` already carried it,
  including the exact 71-at-noon-against-28-at-night measurement. The move was
  a pointer.
- **Rec 7's gotcha had gone stale, which is the failure rec 7 exists to
  prevent.** It asked for the entry to be *moved* to the register. But
  `open-bugs-handoff.md` §0d already covered it *and* already recorded the
  supersession, so filing a second entry would have duplicated it — and the
  gotcha's stated reason was no longer true. The hop-bounded
  `organism_is_supported` it named **does not exist anywhere in the tree**;
  `plant::anchor_support` replaced it, a Dijkstra from the anchors outward
  with no span budget, which schedules checks for newly-unreached cells
  itself. So the inline rule was rewritten around the reason that *is* still
  live — growth adds material, so it is not a disturbance, and a `GrowingTip`
  is expected to be transiently unsupported — with §0d carrying the history.
- **The saving was 118 tokens, not the ~350–400 estimated.** Reported rather
  than padded: rec 7a needed *more* inline text, not less, because a stale
  reason had to be replaced with a live one rather than deleted. The token
  figure was never the main value there — an always-loaded gotcha naming a
  function that no longer exists is worse than a long one that is true.

**Rec 12 is the one this report got wrong three times, and the third is the
instructive one.** The first draft said `perf-lock` was unreachable; the second
implied pushing it would settle the matter; both were corrected. The third
error was inherited rather than invented: **`perf-lock` never touched
Conventions at all.**

Checked 2026-08-25 — its four `CLAUDE.md` hunks land at fork-point lines 56,
90, 105 and 224, and `## Conventions` begins at **436**. Rec 12 had been
deferred since 2026-08-19 on a conflict that did not exist, and this report
repeated the claim into two documents before anyone opened the diff. That is a
miniature of the failure the whole audit is about: **a blocker recorded once
and thereafter inherited, never re-tested.** The lesson generalises past this
instance — a deferral is a claim about the tree at a moment, and it decays
exactly like a measurement does.

So rec 12 landed with no dependency: the nineteen bullets clustered under
**Tests and guards / Tuning and sweeps / Performance / Process and records**,
reordered only. "No rewording" was verified mechanically rather than by
reading — the multiset of bullet texts was hashed before and after and is
unchanged (`0ef24257d3da56d0`). Cost: **+10 lines**.

**`perf-lock` remains unlanded, and it is not a documentation task.** This
report previously implied it was a 91-line CLAUDE.md re-apply. It is **1,546
insertions across 11 files**: `src/perf.rs` alone is 733 new lines, plus
`examples/quiet_probe.rs`, `scripts/perf.sh`, a 317-line rework of the
CI-gated `examples/ascii.rs`, a new CI job, and an edit to
`examples/filmstrip.rs` — the most-collided file in the repo. Its `CLAUDE.md`
section *documents that feature*: land the prose without the code and the file
tells every agent to run `scripts/perf.sh` and quote `TRUSTED`, neither of
which exists. It is a feature-landing project, and it should be scoped as one.

Two things worth weighing before it is: the branch is **518 behind**, and a
good deal of its content is specific to a **Windows/Git Bash four-core box**
(`strings` missing from that shell, `sccache` as a user environment variable,
contention between local sessions). With development now mostly in cloud
containers, some of it may need re-deriving rather than re-applying.

### 5c. README's table of contents — approved in 2026-08, never executed

README is 2,626 lines / ~42,800 tokens with **25 milestone status sections in
the order they were written**, plus a final `## Status` that partly restates
them.

**The wholesale reorder was proposed, approved, then reversed** — churn on a
contested file, and agents navigate by grep
(`documentation-overhaul-plan.md` item 11). **This report does not re-propose
it.**

The same item names the substitute — *"a TOC buys the same navigation for 3% of
the churn"* — and ships it as part of R7. **The status sections landed; the TOC
did not.** `## Finding things` still resolves to prose ending *"find them by
search"* and an inline list of milestone numbers.

So this is not a new proposal but an **unexecuted half of an approved one**.
The `bugindex.py` treatment applies directly: generated, line-numbered,
additive, no reordering. Cheap, and it discharges the item.

### 5d. The report index's status vocabulary is uncontrolled

`Reports/README.md` carries a status per report, which is genuinely valuable.
But there are **40+ distinct labels** across 97 reports — `research.`,
`handoff.`, `shipped.`, `plan.`, `proposal, not built.`, `measured study.`,
`direction agreed.`, `mostly closed.`, `historical, written at M3.` — so an
agent cannot filter for "still true." A small controlled vocabulary
(`settled` / `shipped` / `superseded` / `research` / `proposal` / `live`) as a
*prefix*, with the existing prose kept after it, would make the index
greppable without losing anything.

### 5e. All three artifacts were on the owner's machine — RECOVERED AND PUSHED

**Resolved 2026-08-24 by a local agent, and the answer was the good one:
nothing had been cleaned.** All three were in untracked worktrees under
`.claude/worktrees/`. Full findings on `claude/perf-lock-recovery` (`369ec1d`),
`Reports/perf-lock-recovery-2026-08-24.md`.

| Artifact | Now at | State |
|---|---|---|
| `perf-lock` | `origin/perf-lock` `bdda4a9` | pushed unmodified; 6 ahead of `main`, **518 behind**, forked at `0efeb24` |
| `performance-audit.md` + 4 harnesses | `origin/claude/perf-audit-recovery` `f7bebae` | **was never committed to any branch** — an untracked file, one `git clean` from gone |
| `plant-branch-angle` | merged; head `9b0cccc` | ref never pushed, but the work is in `main` |

**Three ways the in-flight entries were wrong**, all now correctable:

- `perf-lock`'s files were *not* untracked — they were already committed on the
  branch. The index's "(untracked)" note was several commits stale.
- The branch `perf-audit` (`bb20167`) is **zero commits ahead of `main`**. The
  report was never on it. Pointing anyone at that branch would have found
  nothing and concluded the work never existed.
- The `plant-branch-angle` *worktree* is named `plant-crown`, which is part of
  why a name-based search misses it. Worktree name ≠ branch name.

**Why this report could not settle `plant-branch-angle` itself, and the local
agent could.** This session's clone is **shallow** — 633 commits — so
`9b0cccc` is not present in it and `git merge-base --is-ancestor` cannot run;
`branchcheck.sh` refuses on the same grounds. The local agent ran the real test
on a full clone and it exits 0. That is the measurement, and it beats this
report's inference. It is also the second time in this audit that a claim about
branches came from a clone's fetch depth rather than from the repository — see
the header correction and §5f.

**The bonus find, which nobody asked for and which the same `git clean` would
have taken:** 50 uncommitted lines of `Reports/plant-appearance-design.md` §6a,
on no branch anywhere — the `thicken`/`stem_run` axis bug (leaning trunks
mismeasured at 30–70 cells, so they never widened) and a re-swept `shade_death`
(0.03 → 9% foliage, 0.003 → 30%, 0.0 → 54% but crowns fuse). Now on
`origin/claude/plant-appearance-6a-recovery` (`8c35cff`).

**A tighter deadline than this report quoted.** `e20e338` cited ~2026-09-18 for
its own session directory. The transcripts on that machine start **2026-08-13**,
so the default 30-day cleanup begins biting around **2026-09-12**. Anything
still living only in session state has less time than the earlier figure
implied.

### 5e-bis. What the recovered branches owe before they can land

Recorded so nobody discovers it at merge time:

- `claude/perf-audit-recovery` adds **five `examples/` binaries**
  (`frame_profile`, `perf_counters`, `render_cost`, `weather_duty`,
  `camera_snap`) and **none has a row in `Reports/instruments.md`**, so
  `docscheck` check 5 fires on merge. That check exists because harnesses were
  being rebuilt; five new ones landing unindexed is exactly what it guards.
  `camera_snap` has weaker provenance than the other four — untracked in the
  same worktree and same session window, but not named in the report — and the
  recovery flags it as such.
- The branch also carries `Reports/perf-audit-worktree-src.patch` rather than
  committing that worktree's `src/render.rs` (+54/−12) and `src/sim/field.rs`
  (+22). That is the right call for a rescue — it is the audit's own
  instrumentation, not feature work — but it means the branch is **not a merge
  candidate as it stands**: not built, not checked, not tidied.
- `perf-lock` adds `scripts/perf.sh`, which `main`'s `CLAUDE.md` Commands
  block does not mention at all. Landing the script without its Commands row
  repeats the defect `Reports/instruments.md` was created to fix.

### 5f. Branch hygiene: 49 remote branches, and `branchcheck.sh` cannot see most of them

Incidental but cheap to state. `load-share` is fully merged into `main` and
still standing, and it is not alone — `CLAUDE.md` records twelve such at the
2026-08-22 census. Worth a prune pass. Noted also because this report's first
draft asserted the remote held two branches, on a container clone that had
fetched only `main`: **`branchcheck.sh`'s UNLANDED report is only as complete
as the refs the local clone has fetched**, which is a real blind spot for a
fresh remote session and is not mentioned where the script is documented.

## 6. This pass was itself reviewed, and the review found real defects

Recorded because the findings are the argument for doing it. An independent
review of this branch's own diff returned 15 findings; every one checked here
held up. The ones that mattered:

**In the tooling this pass added** — the thing that now gates other sessions'
work:

- `bugindex.py` derived status by matching the **whole heading**, so
  `### P3. The generation loop — §F4 closed, …` filed a live bug as *closed*
  on the words "§F4 closed" in its own title. Any future entry titled "the
  frame budget is not fixed" would have done the same. Status now reads only
  the bolded verdict clause, and a verdict naming a live half (`§G`) wins over
  a closed word in the same clause — false-open is the safe direction.
- Three deliberately-superseded headings (`(was) 1h.`, `G (original).`)
  carried no verdict, so they rendered **OPEN** beside their FIXED successors
  — telling readers that fixed bugs were live and inflating the open count.
- `"--check" in sys.argv` meant `--chekc` fell through to the **write** path
  and rewrote a 343 KB co-owned file for someone who meant to verify it. This
  is `CLAUDE.md`'s own "an unknown argument is silently ignored", paid for
  again; unrecognised argv is now an error.
- `Path.write_text` would have rewritten all ~5,700 lines as CRLF on the
  Windows box this repo's gotchas are written for.

**In `docscheck` check 8** — a guard that could not fail:

- It bounded the Controls table with a hardcoded `## Materials` end-anchor, so
  renaming that heading made `sed` print to EOF and any key mentioned anywhere
  in README's 2,600 lines counted as documented. It now stops at the next
  `## ` heading, whatever it is.
- It scanned `KeyCode::` arms through a hand-written variant list that omitted
  `Backquote`, `Backslash` and `Quote` — **three keys bound today, on the app's
  own help page, and absent from the README**. It now drives off
  `App::help_columns`, the curated user-facing list, so what the player is
  shown is exactly what the README owes a row.

**In the README rows this pass added** — two rationales asserted without
reading the source comment that gives the real one:

- The `;` alias was explained as "60% keyboards have no function row". The
  source says the real reason is that **macOS owns F9–F12** (F11 is Show
  Desktop, F10 often Exposé) and ends "keep any future binding off F9–F12 for
  the same reason". The invented rationale would have licensed exactly the
  binding mistake the source forbids — and `CLAUDE.md` says source comments
  are load-bearing precisely for this.
- `F12` was documented as a two-way toggle. It is a **four-way cycle** whose
  last mode costs ~4.4 ms and is, by its own doc, "far too slow for a frame —
  kept as the bar, not as a shipping option".
- `README.md:158` named `,` and `;` for the boiling and gas selectors; both
  are wrong (they are `` ` `` and `\`), and this pass added a `;` row directly
  contradicting it without noticing.

**And one methodological error in this report**, §5b: it read
`plant-branch-angle`'s absence from the remote as *discharged* and `perf-lock`'s
identical absence as *possibly lost*. Same evidence, opposite readings, each
chosen to suit its conclusion. Corrected in §5b.

**What this says about the pass generally.** The tooling was written by the
same session that wrote the audit criticising unverified claims, and it
shipped four classes of defect the audit's own source documents name by name.
Two of them (`bugindex`'s section allowlist, check 8's key list) had *already*
been caught and fixed once during the session, which was evidence the code was
trap-prone and was read as evidence it was now clean. **`docscheck` also caught
a regression this repair introduced** — restoring the `F12` row deleted the `K`
row, and check 8 named it on the next run.

## 7. What this pass changed

Additive to the documentation — no prose was rewritten or deleted, and
`CLAUDE.md`, `PLAN.md` and `wiki/` are untouched:

- `scripts/bugindex.py` (new) — the register's status index, with `--check`.
  Status reads only the bolded verdict clause; historical `(was)` / `(original)`
  headings are marked `historic` rather than open; unrecognised argv is an
  error rather than a silent write; newlines are pinned to LF; collisions are
  warned about on write as well as under `--check`.
- `scripts/docscheck.sh` — check 6 (index current, identifiers unique, with an
  else-note if the generator goes missing) and check 7 (every key on
  `App::help_columns` has a README Controls row, table bounded by the next
  `## ` heading).
- `README.md` — seven corrections: rows added for `F10`/`;`, `F11`/`0`, `F12`,
  `` ` ``, `\` and `'`; the `;` rationale corrected to the macOS one the source
  actually gives; `F12` documented as the four-way cycle it is, with its costs;
  and line 158's boiling/gas key names fixed.
- `Reports/open-bugs-handoff.md` — the generated index, carrying its own
  conflict-resolution instruction.
- `Reports/README.md` — this report indexed; `documentation-audit.md` corrected
  from "being executed" to executed; the `claude-md-recommendations.md` line
  corrected to say which blocker is *provably* discharged and which two cannot
  be tested.

`docscheck.sh` exits 1, reporting only the three identifier collisions in §4b.
Every other check is green. That residue is deliberate — the script is not a CI
gate and its findings are a work order — but it is also a **cost**: while it
stands, `docscheck: clean` is unreachable, an acceptance line some PR bodies
use, and a genuinely new finding arrives alongside three standing ones. That
raises the priority of the §4b call rather than lowering it.

## 8. Verification

Every number above is reproducible:

```
# corpus size, and CLAUDE.md's add/remove ratio and section budget
find . -name '*.md' -not -path './target/*' -not -path './.git/*' -printf '%s\n' | awk '{s+=$1} END {print s, NR}'
git log --numstat --format='' --follow -- CLAUDE.md | awk 'NF==3 {a+=$1; d+=$2} END {print a, d}'
awk '/^## /{if(n)printf "%-52s %5d\n", n, NR-s; n=$0; s=NR} END{printf "%-52s %5d\n", n, NR-s}' CLAUDE.md

# the register's split, the index, and the identifier collisions
python3 scripts/bugindex.py --check
bash scripts/docscheck.sh

# §5e -- the three unreachable artifacts, across every remote branch
git fetch origin --prune
for b in $(git branch -r --format='%(refname:short)'); do
  git cat-file -e "$b:scripts/perf.sh" 2>/dev/null && echo "perf.sh on $b"
done
```

The cold-agent benchmark (§1b) re-runs by pasting the prompt in
`documentation-audit.md`'s final section into a fresh `Explore` agent with no
project context, search breadth medium. Compare **files opened**, **source
files read**, and **traps refused** — 8 / 0 / 1-of-1 in 2026-08-21, 6 / 0 /
1-of-1 here. Read §1c first: the instrument now lives inside the corpus it
measures.

## 9. README.md audited against the agent lens — verdict: keep it

Asked 2026-08-25: *is the primary README useful for agents developing the
project?* The owner's suspicion was that it mixes four audiences — a message
to the user, an explanation of the code, a history of development, and notes
to other agents — and that the mix costs agents.

**Measured, the mix is not there.** README is 2,735 lines / ~44.6k tokens.
By audience:

| | lines | share |
|---|---|---|
| user-facing (`## Running`, `## Controls`) | 71 | 2.6% |
| navigation (`## Contents`, `## Finding things`) | 88 | 3.2% |
| `## License` | 2 | 0.1% |
| **subsystem reference** | **2,574** | **94.1%** |

The history impression is **voice, not content**. Every status section opens
`Built:` / `Landed`, then describes what the code does *today*, with live
file paths and identifiers. It reads as a changelog and functions as a
reference manual.

### It is not stale, and this was the concern with the strongest prior

The audit's founding worry was that documentation predates implementation.
For README that is measurably false. Every backticked token was extracted and
resolved against the tree:

| | checked | wrong |
|---|---|---|
| file/path references | 546 | **0** |
| code identifiers (consts, fns, types) | 220 | **2** |

The two are renamed tests — `same_group_chunks_are_never_within_reach_of_
each_other` (README:994) is now `concurrent_chunks_are_never_within_reach_of_
each_other`, and `a_tree_can_produce_multiple_simultaneous_tips_via_
branching` (README:1417) is now `root_and_shoot_branching_read_different_
slots`. `assets/player.ron` looked missing and is not: `player.rs`'s
`ASSET_PATH` writes it at runtime. `min_neighbour` (README:1621) is a
*deliberately* dead identifier — the section is describing a fixed bug.

**0.4% error rate over 766 references.** Do not "refresh" this file.

### It duplicates nothing

8-gram shingle overlap, README against all of `wiki/`: **0.10%** (28 of
28,620). The trailing `## Status` against the rest of README: **2.5%** — it
reads like a summary of the milestone sections and is not one; it is 20
cross-references plus a **Known limitations** list that exists nowhere else
in the repo. That list is the single highest-value paragraph in the file for
an arriving agent, and it is at line 2,632 of 2,735.

### The one real cost: subsystem knowledge split by build date

Per-topic section spread (sections with >=5 substantive line hits):

| topic | dominant section | other sections |
|---|---|---|
| fire/heat | M14 status (46) | incidental |
| structural | M17 status (42) | Felling (12), M8 (9) |
| creatures | M18 status (58) | incidental |
| worldgen | M10 status (25) | incidental |
| **plants** | **M16 status (65)** | **The economy re-derived (25), Plant lines merged (19), The generation loop (15), Felling status (14)** |

Five of six topics have exactly one owning section, so the milestone framing
costs nothing — `M17 status` is `structural collapse` wearing a number. Plants
are the exception: five top-level sections, **none of them named "plants"**.
An agent that greps `plant` stops at M16 and never learns the standing-tissue
economy exists.

### Why a reorder is the wrong fix — and would break 47 addresses

`Reports/dead-ends.md` **addresses its entries by README section and
paragraph name**: `- **README.md 'M17 status' — 'A step's cost now depends on
which direction the support comes from' paragraph**`. 47 of its 594 entries
are addressed this way, across 16 distinct sections (7 in `The coarse field
grid`, 6 each in `M8 status` and `M16 status`, 5 in `M17 status`).

**README's section structure is a load-bearing address space for the
do-not-retry register.** Renaming or reordering sections silently invalidates
47 pointers into the one document whose whole job is stopping an agent
re-walking a dead end. This is the mechanical reason behind
`Reports/documentation-overhaul-plan.md` item 11's reversal, which recorded
only that the churn "bought nothing a table does not" — the cost is larger
than that entry states.

### Recommendation — both items landed 2026-08-25

**Keep the README as it is.** Not stale, not duplicative, 94% reference, and
its headings are referenced infrastructure. Two narrow, non-structural items:

1. **Fix the two renamed test names** (README:994, README:1417). Mechanical.
2. **Add a generated topic -> section index** to the existing TOC block —
   `plants -> M16 status, Plant lines merged, The generation loop, The
   economy re-derived, Felling status`. It solves the only measured
   navigation failure without renaming a heading, so no `dead-ends.md`
   address moves; and because `scripts/readmetoc.py` generates it under
   `scripts/docscheck.sh`, it cannot drift. Cost: ~40 lines (2.6% -> 4.0% of
   the file spent on navigation).

Explicitly **not** recommended: reordering sections, renaming milestone
headings to subsystem names, splitting the file, or moving status sections
into `Reports/`.

### 9a. What landed, and the mechanism that was rejected on the way

Both items are in. The two stale identifiers are fixed — and the second was
not the rename it looked like, which is the transferable part: README claimed
`a_tree_can_produce_multiple_simultaneous_tips_via_branching` guards that a
tree produces multiple simultaneous tips. The successor is
`a_tree_can_branch_into_more_than_one_lineage`, and **the proxy changed
because the design did** — tip retirement means tips essentially never stay
alive simultaneously now, so a blind repoint would have re-armed a claim the
mechanism deliberately abandoned. Read a successor's body before repointing a
stale name.

**The topic index is an explicit map, and the first cut was not.** The
obvious mechanism — score each section by counting topic-term hits, keep
those above a share of the top — produces a table that looks principled and
is wrong, because it counts *mentions* rather than *ownership*:

- `M18 status` outranked `Materials` for **powders**, purely because a worm
  burrows through a great many of them;
- `The ant colony — status` fell out of **creatures** altogether — at 14
  lines it cannot clear any share bar set by a 254-line section;
- **worldgen** picked up `Controls`, because `\bseed\b` matches the keys that
  plant a seed.

Tuning the thresholds until that output looked right is what
`Reports/design-philosophy.md` 2b calls curve-fitting and what `CLAUDE.md`
means by *ask what a metric counts when nothing is wrong*. Membership is an
editorial judgement, so it is written down as data instead.

The scored version had exactly one virtue — a new section could not be
silently missing — and `--check` buys it back: it fails if a title in
`TOPICS` stops existing, **and** if any `## ` section belongs to no topic and
is not on an explicit `UNINDEXED` list. The first guard is worth more than
its cost on its own, because nothing else in the repo notices when a README
heading is renamed out from under those 47 `dead-ends.md` addresses.

Every guard was verified by putting its fault back: a renamed heading, an
unplaced new section, a hand-edited line number in each of the two tables,
and `docscheck` exiting non-zero for all of them (checked without a pipe —
`docscheck | tail` reports `tail`'s status, which is the gotcha `CLAUDE.md`
records).

Cost: **32 lines, ~783 tokens, 1.7% of README**. Navigation is now 3.7% of
the file.

## 10. Doc staleness, measured across the whole corpus — mostly a negative result

Concern (b) of the founding brief, verbatim: *"Many of the reports were
generated prior to development and may be out of date from how we implemented
the features."* Measured 2026-08-25 across all ~80 documents.

**Do not re-run the general sweep. It does not measure staleness.**

The method that worked on `README.md` in §9 — extract every backticked token,
resolve it against the tree — was run over `Reports/`, `wiki/`, `PLAN.md` and
`CLAUDE.md`. The top scorers by miss rate:

| document | rate | what the misses actually are |
|---|---|---|
| `dependency-license-audit.md` | 54.5% | crate names (`ab_glyph`, `khronos_api`) |
| `prior-art-worldgen-slicing.md` | 51.9% | **Minecraft's** internals (`BIOME_USE_BIG_WANG`, `stbhw_generate_image`) |
| `measurement-under-contention.md` | 45.0% | `src/perf.rs`, `TimingLock`, `TRUSTED` — which its own header says are deliberately **not** in the tree |
| `agent-documentation-audit-2026-08-24.md` | 42.4% | this document, quoting names from other branches |
| `CLAUDE.md` | 26.5% | harness names (`SessionStart`, `ToolSearch`, `create_trigger`) |
| `dead-ends.md` | 7.8% | rejected species files (`prostrate.ron`, `weeping.ron`) — **absent because they were rejected** |

A design report, a prior-art survey, a retirement notice and a do-not-retry
register are all *supposed* to name things that are not in the tree. The
metric counts "identifier not found" and calls it staleness; what it is
measuring is mostly a document doing its job. This is `CLAUDE.md`'s *ask what
a metric counts when nothing is wrong* — and the answer was: a great deal.

`README.md` scored 0.4% not because it is better maintained than the reports,
but because it is the one document in the corpus whose whole purpose is to
describe the tree as it stands. **The sweep is valid only for documents that
claim to describe the current tree**, which is `README.md` and parts of
`PLAN.md`, and nothing else.

### The one class that is real: cited test names

A doc saying *"test X guards this"* is a checkable claim, and when it is wrong
an agent trusts a guard that does not exist — the failure `CLAUDE.md`'s
green-suite gotchas are entirely about. Restricting to backticked
`snake_case` names of four-plus words:

**334 cited test-shaped names across the corpus, 29 absent from the tree.**

Four of those were genuinely misleading and are fixed here:

| site | was | now |
|---|---|---|
| `PLAN.md`:1274 | `same_group_chunks_are_never_within_reach_of_each_other` cited as having "held up" | repointed to `concurrent_chunks_...`, old name kept as history |
| `PLAN.md`:1276 | `a_settled_world_with_a_growing_tree_still_sleeps_between_growth_ticks` | the guard survives keyed on a worm; repointed to `a_settled_world_with_a_worm_still_sleeps_between_movement_ticks` |
| `PLAN.md`:1660 | "`pack_aux_preserving_density` **now wraps** every self-update" | it was **deleted as unnecessary** by the substrate migration — the fields are no longer co-located, so the bug class is unrepresentable. `plant.rs`:1017 holds the removal note. Marked do-not-restore |
| `pixel-physics-issues.md`:132, 550 | same rename, two sites | repointed the same way |

### …and a gate for it was designed and rejected

A `docscheck` rule over cited test names looks like the obvious next step. It
would fire on **correct** documentation:

- `plant-project-review-2026-08-23.md` §V reads *"`a_tree_eventually_stops_growing`
  retired by owner decision"* — correctly recording a retirement;
- `open-bugs-handoff.md`:3103 names `a_snowstorm_leaves_no_snow_floating_on_open_water`
  as the *before* and names its successor in the next clause;
- `liquid-heightfield-design.md` proposes restoring a test it calls deleted;
- `plant-substrate-v2-design.md` proposes six tests that were never built.

And the four fixes above **still trip it**, deliberately: repointing a stale
name correctly means keeping the old one as history. A name-only rule cannot
separate "claims a live guard" from "records a dead one"; only the surrounding
sentence can. Triage stays human, and the sweep is a one-off instrument, not a
gate.

`a_tree_eventually_stops_growing` is cited in **six** documents and exists in
none — every one of them correctly, as a retired bar. That is the shape of
this whole finding.
