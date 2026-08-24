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

**Audited against:** `1882dc9` (merge of #47). Working tree clean apart from
this line of work; no other branch existed on the remote at audit time, which
is worth stating because it is the reason the register edits below were safe
to make at all.

---

## 1. The earlier audit is done, not pending — check this before re-running it

Every headline finding of `documentation-audit.md` was verified against the
current tree and **has been fixed**:

| Its finding | State now |
|---|---|
| R1 — architecture map covered ~half of `src/` | Fixed, and *enforced*: `docscheck.sh` check 2 fails if any module is unmentioned. Clean. |
| R2 — Controls table wrong in 3 places, 6 keys missing | Fixed. `F3`/`F4`/`F2`, `S`, `Enter`, `Y`, `F6`–`F9`, `L` all correct and present. |
| W1/W2 — wiki not as fresh as it claims | Fixed. All 11 pages' claimed dates match their last commit date **exactly**. |
| Link integrity | Still clean; `docscheck.sh` check 1 gates it. |

**`Reports/README.md:26` still describes it as "findings, being executed."**
That line is now itself the stale artifact. Proposed correction: mark it
*executed*, and note that its recurring defect classes are carried by
`docscheck.sh` rather than by the report.

This matters beyond tidiness: an agent that reads that line spends a session
re-executing work that landed.

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
and most were expensive. The finding is about *container*, not *value*.

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
and reordering 5,000 lines turns every concurrent edit into a conflict. The
index is additive and costs no merge surface.

Two traps the generator had to survive, both recorded in its docstrings:

1. **The line numbers cite a document the table is inside**, so a single pass
   always ships numbers wrong by the height of its own block. It iterates to a
   fixpoint, guarded rather than assumed.
2. **Narrative sections enumerate their findings `### 1.`, `### 2.`, `### 3.`** —
   the same shape a bug identifier takes in a file where real bugs are *also*
   numbered. A naive pass counted them as bugs and reported them as duplicates
   of the genuine §1/§2/§3. The section test is an **allowlist** of the three
   register sections, so a newly appended narrative section is inert by default.

### 4b. Three identifier collisions remain — OPEN, editorial

`§R`, `§X` and `§Z` each name two entries. `docscheck.sh` now reports them.

**§Z is already doing measurable harm across reports:**

- `Reports/creature-review-2026-08.md:166-168` — "§Z" means the free-particle
  `Cell::aux` bug (register line 1241).
- `Reports/plant-project-review-2026-08-23.md:36,61` — "§Z" means *"the stand
  still reads as one mass"* (register line 1107).

Two reports, one reference, different bugs. `src/sim/world.rs:4129` says
"bug Z" and means the `Cell::aux` one.

| § | Entries | Nature |
|---|---|---|
| Z | 1107 (stand reads as one mass, JUDGED) / 1241 (particle drops `Cell::aux`, FIXED) | **Two different bugs.** Line 1241 owns the source reference. |
| X | 1502 (DECISION CARD, 2026-08-23) / 1551 (DESIGN DIRECTION, 2026-08-22) | **Same bug filed twice** — "A desert with no desert plants". The newer supersedes. |
| R | 5162 (`filmstrip scene=colony` panics) / 5373 (ant stands on open water) | **Two different bugs**, both OPEN, both found 2026-08-23. |

Only **S** and **T** remain free as single letters. The file already has two
conventions for this: suffixes (`V2`, `P1`/`P2`/`P3`, `H2`/`H3`, `C1`, `D1`)
and a parenthetical marker (`G (original)`, `(was) 1h.`), the latter rendering
an entry deliberately unreferenceable.

**Proposed, needs an owner call because it is editorial:**

- **§X** — the two are the same bug, so retitle the *older* (1551) to
  `X (original).`, matching the `G (original)` precedent already in the file.
  No new letter consumed, collision gone.
- **§Z** and **§R** — genuinely distinct bugs. `CLAUDE.md`'s own precedent
  ("the newcomer was renamed §R, its self-references repointed") says rename by
  recency; but for §Z the newcomer holds the only source-code reference, so
  renaming it also edits `src/sim/world.rs:4129`. Either is one line. **Which
  entry keeps the letter is the owner's call.**

Deliberately not done unilaterally: renaming a bug changes its address, and
three reports plus one source file cite these.

## 5. Not yet acted on — proposals

### 5a. The Claude Code configuration surface is empty *(highest leverage)*

This repo is built end-to-end by Claude Code agents, and **none of its Claude
Code configuration is version-controlled.** `.claude/` holds one skill
(`review`) and two workflow scripts. There is:

- no `.claude/settings.json` — no shared permission allowlist, no env, no hooks
- no `.claude/hooks/` — nothing runs at session start
- no `.claude/agents/` — the lane/coordinator program described across 145
  lines of `CLAUDE.md` is assembled by hand each time
- no `.claude/commands/` — no slash commands

**Measured: zero mentions of `settings.json`, `SessionStart`, `.claude/agents`
or "slash command" across all 122 markdown files.**

The sharpest instance: `CLAUDE.md` says *"run `bash scripts/branchcheck.sh`
when you pick up a branch"* — and the drift it exists to prevent happened
anyway, at scale (measured 2026-08-22: ten branches at exactly 160 behind).
That is a convention with no enforcement, in a repo whose own stated lesson is
*"a convention alone did not catch it; a check that runs catches it."*
A `SessionStart` hook running `branchcheck.sh` is that check.

Candidates, in leverage order:

1. **`SessionStart` hook** — `branchcheck.sh`, and `review.py inbox` (which
   `CLAUDE.md` asks every session to run when picking a thread back up).
2. **`.claude/settings.json`** — permission allowlist for the read-only and
   routine commands (`cargo test`, `cargo clippy`, `bash scripts/*.sh`,
   `git status/log/diff`), so agents stop burning turns on prompts.
3. **`.claude/agents/lane.md`** — encode the dispatch brief that `CLAUDE.md`
   §"Running a program of sessions" describes in prose, including the two rules
   that cost real money: **workers run `claude-opus-5`, never inherited** (three
   workers silently inherited a premium tier and ran $25–71 each inside ninety
   minutes), and **every package gets a cost fork at dispatch.**
4. **`.claude/commands/`** — `/gates` (run the suite exactly as CI does,
   `--skip` included), `/land`, `/docs-check`.

### 5b. Move the situational half of CLAUDE.md into skills

Method, Conventions and Gotchas — 50.6% of the file — are consulted by
situation, and `CLAUDE.md`'s own routing table already names the situations.
Skills load on description match, which is exactly that routing, done by the
harness instead of by prose.

**The tradeoff, stated honestly:** an always-loaded rule is read every time; a
skill is read when its description matches. For lessons this expensive, a miss
is costly, and several of these gotchas are the kind an agent does not know it
needs until after it has paid. **Recommend splitting conservatively** — move
`Running a program of sessions` (13.4%, coordinator-only, and a lane genuinely
never needs it) and the longest Method case studies, keep every "never" and
every one-line trap in `CLAUDE.md`. That is roughly a third of the budget back
at low risk. **Do not do this without an owner call**; the content is the
project's memory.

### 5c. README's status sections

2,626 lines, ~42,800 tokens. It carries **25 milestone status sections in the
order they were written, not numeric order**, plus a final `## Status` that
partially restates them. The file's own `## Finding things` section admits it:
*"find them by search."* An index with line numbers — the `bugindex.py`
treatment — would cost one commit and no restructuring.

### 5d. The report index's status vocabulary is uncontrolled

`Reports/README.md` carries a status per report, which is genuinely valuable.
But there are **40+ distinct labels** across 97 reports — `research.`,
`handoff.`, `shipped.`, `plan.`, `proposal, not built.`, `measured study.`,
`direction agreed.`, `mostly closed.`, `historical, written at M3.` — so an
agent cannot filter for "still true." A small controlled vocabulary
(`settled` / `shipped` / `superseded` / `research` / `proposal` / `live`) as a
*prefix*, with the existing prose kept after it, would make the index
greppable without losing anything.

### 5e. Two in-flight reports may be lost work

`Reports/README.md`'s in-flight section names `performance-audit.md`
(worktree `perf-audit`, *untracked*) and `measurement-under-contention.md`
(worktree `perf-lock`, *untracked, with a CLAUDE.md edit adding
`scripts/perf.sh`*). Neither file exists in this clone, and the remote holds
only `main` and this branch.

An untracked worktree is pushed nowhere by definition. **If those containers
are gone, so is that work.** Worth confirming with the owner before the
entries are removed — and worth noting as the general risk: `CLAUDE.md`'s
"handoffs are committed, not replied" applies to reports in progress too.

## 6. What this pass changed

`fbc10e6` — additive only, 156 insertions, 0 deletions:

- `scripts/bugindex.py` (new) — the register's status index, with `--check`.
- `scripts/docscheck.sh` — check 6 (index current, identifiers unique) and
  check 8 (every `KeyCode::` binding has a README Controls row).
- `README.md` — the four bindings check 8 found undocumented: `F10`/`;`
  (terrain light), `F11`/`0` (void reveal), `F12` (sky light).
- `Reports/open-bugs-handoff.md` — the generated index.

`docscheck.sh` now exits 1, reporting the three identifier collisions in §4b.
That is the intended behaviour — the script is deliberately not a CI gate, and
its findings are a work order.

## 7. Verification

Every number above is reproducible:

```
find . -name '*.md' -not -path './target/*' -not -path './.git/*' -printf '%s\n' | awk '{s+=$1} END {print s, NR}'
git log --numstat --format='' --follow -- CLAUDE.md | awk 'NF==3 {a+=$1; d+=$2} END {print a, d}'
awk '/^## /{if(n)printf "%-52s %5d\n", n, NR-s; n=$0; s=NR} END{printf "%-52s %5d\n", n, NR-s}' CLAUDE.md
python3 scripts/bugindex.py --check
bash scripts/docscheck.sh
```
