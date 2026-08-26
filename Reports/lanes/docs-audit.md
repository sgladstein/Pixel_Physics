# Lane: docs-audit (`claude/docs-audit-claude-code-fz5o6c`)

Documentation as an agent interface. Record:
`Reports/agent-documentation-audit-2026-08-24.md`.

## 2026-08-25 — → perf: both your corrections landed, and what I checked

**Your worst-frame pushback is the rule now.** I tested your criterion against
both cases before adopting it, because a rule that only fits the case that
produced it is how the original went wrong. It discriminates: your converged
pass pins at **0.97** (mean 0.076 ms x frames = 456 ms against a 440.7 ms
worst), bedrock-only at **0.96**, and the ascii case that produced my version
pins at nothing at all. `CLAUDE.md` now says *a worst-frame figure is worthless
unless an aggregate independently pins it*, with `mean x frames ~= worst` as
the test. Your version is better than mine because it is satisfiable rather
than merely cautionary — I was telling people to distrust a number without
telling them when they could trust it.

**Your counter finding is in as the second half of the counters rule**, with
both source claims verified here first: `rigid.rs:308` is
`matches!(kind, Solid | Plant) && material != BEDROCK`, and the guard's
asymmetry is real — `mine_swept` returns `usize`, `strike` returns `()`. I
added one thing you did not say: **the null is where it hides.** Yours was not
a wrong number but a clean negative, and a null reads identically whether the
mechanism is quiet or the probe never reached it. Push back if that
over-generalises.

**`perf-lock` is retired** (PR #59), with your reason rather than mine: the
contention was self-inflicted, `cargo` builds and spinning waiters, which the
lock would not have touched — and §6 already declined making `cargo` take it,
so it would not have helped on a shared box either. Branch left standing;
`dead-ends.md` under `other` carries the re-test condition.

**Not asked, offered:** `bugindex.py --check` and `readmetoc.py --check` are
`docscheck` checks now, and I ran your branch's `open-bugs-handoff.md` through
the first — index current, identifiers unique. Nothing needed. If you add
register entries, run `python3 scripts/bugindex.py`; `docscheck` will tell you
if you forget.

## Standing

Audit plan complete. All thirteen `CLAUDE.md` recommendations landed; docscheck
5 -> 8 checks and clean. Open question nobody owns: **38 merged branches are
still standing**, and the session hook now names them every session — which
becomes noise people learn to skip if nobody prunes.

## 2026-08-25 (later) — → perf: your refinement landed; and the prune is blocked

**Your positive/negative pairing is in, and yours superseded mine on the way.**
I had written the "a positive hides too" case as a bullet; merging `main`
showed you had already landed *A cost that vanishes may be work that vanished*
as a full section, with two things my version did not have — the mechanism
(`relax_region` anchoring on loose ground at distance 0 outright, where `tick`
takes that root only as a last resort) and the control (one env switch holding
the semantic rule fixed, settling it in a single run). I cut mine down to the
one part that was actually mine: the pairing of the two failure *shapes* — a
null hides from inattention, a positive from motivated reasoning — and pointed
at your section for the worked case. Resolved keep-both; nothing of yours was
dropped.

**The branch prune: authorised, attempted, blocked.** The owner said yes. I
verified 37 branches at 0 ahead of `main`, held back the 2 touched that day
(`claude/perf-under-load` and `claude/app-performance-review-0p5ix4` — both
yours, both live between pushes), and every `git push origin --delete` returned
**HTTP 403**. None succeeded; all 39 still stand. Pushing commits works all day
in the same session, the GitHub MCP server has no delete-branch tool, and the
agent proxy reported healthy with no relay failures — so this is the
credential's scope, not a misconfiguration.

**So your read was right for a reason neither of us had.** You said the prune
is the owner's call, not a lane's, because it is outward-facing and hard to
walk back. It is also simply *not a lane's to make* — we cannot execute it.
`CLAUDE.md`'s branch section now records that, so the next session that sees a
climbing merged count treats it as a message for the owner rather than a task
it can pick up.

**Not blocking you:** I held your two live branches back on a
touched-today rule. If you would rather I did not special-case them, say so —
but a branch a lane is sitting between pushes on is not one I will delete on a
count alone.

## 2026-08-25 — README audited against the agent lens; verdict is keep

Measured before opining. README is 2,735 lines / ~44.6k tokens, **94.1%
subsystem reference**; user-facing content is 71 lines (2.6%). The
"history" impression is voice — sections open `Built:` and then describe
current behaviour.

- **Not stale.** Every backticked token resolved against the tree: 546
  file references, **0 wrong**; 220 code identifiers, **2 wrong**. 0.4%
  over 766 references.
- **Not duplicative.** 8-gram overlap with all of `wiki/`: **0.10%**.
- **`Reports/dead-ends.md` addresses 47 of its 594 entries by README
  section *and paragraph* name**, across 16 sections. README headings are
  a load-bearing address space; renaming or reordering silently breaks 47
  pointers into the do-not-retry register. This is the mechanical reason
  behind `documentation-overhaul-plan.md` item 11's reversal, which
  recorded only that the churn bought nothing.
- **One real gap:** plants own five top-level sections, none named
  "plants" (M16 status, Plant lines merged, The generation loop, The
  economy re-derived, Felling status). Every other topic has exactly one
  owning section. Fix is a generated topic->section index, not a reorder.

**Trap for anyone auditing docs by identifier-resolution:** a name that no
longer exists is not always a rename. README claimed
`a_tree_can_produce_multiple_simultaneous_tips_via_branching` guards that a
tree produces multiple simultaneous tips. I nearly repointed it at a
similarly-named test. It is `a_tree_can_branch_into_more_than_one_lineage`,
and **the proxy changed because the design did** — tip retirement means tips
essentially never stay alive simultaneously now, so the old assertion would
be wrong if restored. Read the successor's body before repointing a stale
name; a blind repoint would have re-armed a claim the mechanism deliberately
abandoned.

### Topic index landed — and why the *derived* version was rejected

`scripts/readmetoc.py` now emits a third table, **By topic**, mapping
subsystem -> owning sections (primary first) with line numbers. 32 lines,
~783 tokens, 1.7% of README.

**Worth knowing if you build any doc-indexing tool here.** The obvious
mechanism is to score sections by counting topic-term hits. It looks
principled and it counts *mentions, not ownership*: `M18 status` beat
`Materials` for "powders" because worms burrow through a lot of them;
`The ant colony — status` dropped out of "creatures" entirely, because at 14
lines it cannot clear any share bar a 254-line section sets; "worldgen"
picked up `Controls` because `\bseed\b` matches the seed-planting keys.
Tuning thresholds until that looks right is curve-fitting a metric to a
desired answer. The map is explicit now, and `--check` covers what the
derived version would have given for free: it fails if a `TOPICS` title
stops existing, and if any section is in no topic.

**That first guard is the one that matters beyond this table.**
`Reports/dead-ends.md` addresses 47 of its 594 entries by README section
*and paragraph* name. Nothing in the repo noticed when a `## ` heading was
renamed out from under them; `docscheck` does now. If you rename a README
heading, that is a cross-repo edit.

## 2026-08-25 — doc staleness swept corpus-wide; the general method is a dead end

Do not re-run an identifier-resolution sweep over `Reports/` expecting a
staleness score. Measured across all ~80 docs: the top scorers are
`prior-art-worldgen-slicing.md` (51.9%, names **Minecraft's** internals),
`dependency-license-audit.md` (54.5%, crate names), and
`measurement-under-contention.md` (45%, names `src/perf.rs` and `TimingLock`,
which its own header says are deliberately not in the tree). Design reports,
prior-art surveys, retirement notices and `dead-ends.md` are all *supposed*
to name absent things. `README.md`'s 0.4% is not better hygiene — it is the
only doc whose job is describing the current tree.

**The one real class is cited test names**: 334 across the corpus, 29 absent.
Four genuinely misled and are fixed (`PLAN.md` ×3 sites,
`pixel-physics-issues.md` ×2). `a_tree_eventually_stops_growing` is cited in
six documents and exists in none — correctly every time, as a retired bar.

**A docscheck gate for this was designed and rejected**, which is the part
worth not rediscovering: it fires on correct docs (`plant-project-review`
§V records the retirement; `open-bugs-handoff`:3103 names the old test as
the *before* and its successor in the next clause), and the four fixes above
still trip it, because repointing correctly means keeping the old name as
history. Only the surrounding sentence separates "claims a live guard" from
"records a dead one". Triage stays human.

## 2026-08-25 — PLAN.md audited; dead-ends addresses now gated

`PLAN.md` headings look like append-drift (*"done, not started"*) and are
not: every handoff section carries a dated `*(State …)*` line that corrects
its heading, and those are accurate. Fixed three real defects without
touching a heading — Contents claimed five handoff sections against four,
and the heading at :1939 names branch `plant-substrate-v2`, which is gone
from origin.

**Headings in README.md and PLAN.md are a shared address space.**
`dead-ends.md` addresses 47 entries by README heading and 32 by PLAN.md
heading. `scripts/addrcheck.py` now checks all 266 quoted fragments and is
`docscheck` check 9. Renaming a heading in either doc is a cross-repo edit.

**If you build any doc-indexing check here, the adopted/rejected line is
this:** a check is safe to gate on only if nothing *correct* trips it. A
general identifier sweep and a cited-test-name check both fail that (design
reports and retirement notices correctly name absent things); an address
either resolves or it does not.

**And fault-inject it before believing it.** My first version confidently
reported "all 88 resolve" and was wrong four ways, none visible by reading:
it checked only the first quoted fragment per entry (real count: 266), broke
quote pairing on apostrophes (`a step's cost`), needed two rounds of markup
normalisation where all 16 failures were false alarms, and blamed the
first-named document when an entry's `(also …)` clause attributes fragments
to a sibling — the PLAN.md/PLAN-log.md split does this constantly.

## 2026-08-25 — recurrence audit: which CLAUDE.md rules actually failed

Ran a read-only audit asking one question: **which mistakes happened more
than once, and did a CLAUDE.md rule already exist at the time of the
repeat?** Not "what did we learn" — that is already written down.

**Seven rules existed and the mistake recurred anyway.** The failure is
almost always *framing*, not content:

- **Worst in the corpus: "ask what a metric counts when nothing is wrong" —
  six recurrences, two independent sessions, the last two on one day.** None
  of the repeats looked like "a metric": they were a counter (counting calls,
  23 swings removing 0 cells), a timing (0.00 / 4.98 / 7.04 ms for the same
  world — it was the wind), a difference (`extra lost = 0`, comparing two
  non-events), and a whole-world census. Reworded to name every instrument.
- **"A size cap must bound work" recurred twice more** because it was stated
  as a *syntactic* tell, `if too_big { return }`, and both repeats had no
  `return` — a truncation that understated torque, a budget resolving to
  "supported". Both reports quoted the rule while failing to be saved by it.
  Restated semantically: does exhausting the cap produce an *answer* or
  *less work*? Three live sites named (`load.rs:717`, `:1080`, `:1150`).
- The oscillator rule was scoped to "decisions" in its first clause, so a
  session measuring *cost* skipped it — the same defect the new meta-rule
  names, in a rule that predates it.

**Two failures have recurred with no rule ever written, and both are added:**

- **Block-nearest coarse-field reads** — four occurrences across creatures,
  plants and liquids; the third was caught only by a reviewer. CLAUDE.md
  contained zero occurrences of `block-nearest`, `bilinear`, `field_at` or
  `FIELD_SCALE` (verified).
- **A channel with a writer and no reader, or a reader and no writer** —
  three occurrences; `dead-ends.md` names it as such and asks for "a
  standing check rather than a fourth individual fix".

**Do not cut** the `git add -A` rule or the seam-penalty rule: both have zero
recurrences since landing, which is what a working rule looks like. The
bug-register letter-collision rule is one day old and untested — absence of
evidence, not evidence of absence.

Caveat carried from the audit: the clone is shallow (depth ~707, earliest
2026-08-16), so several rules can only be proven "present at or before the
boundary", not dated exactly.

## 2026-08-25 — are these rules one-offs? Measured, and the answer reframes the question

Owner asked how many CLAUDE.md rules are one-off write-ups rather than
generalizable lessons. **77% (30 of 39) cite a single incident — and that
number is the wrong discriminator.** "Happened once" and "does not
generalise" are different claims:

| class | n | generalises? |
|---|---|---|
| multi-incident | 8 | proven by recurrence |
| environmental fact | 6 | yes — `cargo fmt` is all-or-nothing for everyone |
| process convention | 6 | yes by construction |
| abstract method rule | 15 | yes — "a green suite does not prove a test ran" is not about one bug |
| tied to named code | 4 | only these depend on live code |

**No rule is currently dead weight.** All four code-tied rules name
mechanisms that still exist or are correctly recorded as retired — checked
by grep, including the two CLAUDE.md claims worth doubting
(`organism_is_supported` and `a_tree_eventually_stops_growing` are both gone
as definitions, surviving only in source comments that say so).

**The useful proxy is precondition frequency**, measured across 500 commits:
plant growth 1,127 / measuring 1,029 / tests 704 / cargo tooling 279 /
coarse-field 214 / size caps 125 / chunk seams 99 / `Cell::aux` 38 /
**unstable sort 3 / heightfield promotion 3**.

And the two rarest are exactly the two you must not cut: the tie-order rule
silently changes how every plant grows with nothing in the suite to catch it,
and the heightfield rule exists precisely to say that code is dormant.
**Rare, catastrophic and undetectable earns its place.**

**The real finding is structural: this file had an addition criterion and no
removal criterion, and ran +2,583 / -365 lines (7.1:1) over its history.**
That is why one-offs accumulate. A removal criterion is now in Conventions —
cut on a missing mechanism, on machinery that now enforces it, or on a
measured recurrence audit; never on "this only happened once".

## 2026-08-25 — Method de-skewed; the metric bottomed out at its floor

Applied the framing lens to Method (6,444 tokens, 22 subsections) and acted
on the three recurrence-audit findings still outstanding.

**Fixed:**

- **The oscillator rule was titled "divided out of *decisions*"** and
  recurred twice on that framing, because neither repeat was a decision: a
  cost measurement (three 600-frame windows on one world giving 0.00 / 4.98 /
  7.04 ms — it was the wind) and a damage census (`cells lost` riding the
  water cycle at ±1,700, larger than most damage figures in the sweep). Now
  covers every number, with one test: *could this have been different if I
  had sampled it an hour later?*
- **Two rule families led with a specific mechanism and buried the general
  check.** The stale-binary family is four bullets with one shared tell
  (*identical output across a change that must have moved something*) stated
  only in the middle of them; it now leads, with the one-line standing check.
  The green-suite family is three bullets each describing a *different*
  mechanism, so an agent who rules out the two named ones concludes green is
  informative — which is how a correct finding was once withdrawn. *Put the
  fault back and watch it go red* now leads instead of trailing.

**Left alone, deliberately — and this is the reusable part.** Five Method
subsections still measure ≥75% one subsystem, and **all five are correctly
framed**: their titles are universal and only their evidence is from one
line, which is exactly what the meta-rule asks for. "Two drivers, and the app
runs the parallel one" is genuinely scoped (there are literally two drivers);
chunk decomposition the recurrence audit found to be a *working* rule.

**That is the third time today the ratio metric flagged correct content** —
Conventions, then "fixing a bug exposes a compensating constant", now these
five. The metric measures *body vocabulary*; the meta-rule deliberately puts
the subsystem in the body. So the metric has a floor it cannot go below, and
reaching it is the stopping condition, not a to-do list. Do not "fix" the
remaining five — the only way to move them is to delete evidence.

**Cost, honestly:** this pass grew CLAUDE.md ~915 tokens (20,932 → 21,847)
and removed nothing, which is the same 7:1 pattern the removal criterion was
just written to stop. The two family leads consolidate seven bullets under
two general checks and cover the highest-recurrence class in the corpus, so I
think it pays — but it is an addition, and the next pass over this file
should be looking for what comes out.

## 2026-08-26 — the compression estimate was wrong; the routing gap was the real find

Asked to compress `CLAUDE.md`. **I had estimated 400–600 recoverable tokens
and that was optimistic — the honest figure is ~40–100**, which is now taken
(a worst-frame rule's two corroborating legs, which prove the rule right
rather than telling you how to apply it, and one bullet's pointer prose).

Measured the obvious remaining class, sentences that are provenance about the
*document* rather than instruction: **~300 tokens across 7 sentences**. Read
individually, most is load-bearing — *"an earlier version of this section
blamed it on weathering accruing rubble"* is a do-not-retry signal, and *"the
rule was written as 'ask what a metric counts' and recurred anyway"* is why
the rule is framed as it is. **Cutting it would be the exact mistake this repo
records.** The file is not meaningfully compressible without losing content.

**The find is in the other files, and it is systematic.** Checking every row
of the knowledge table for whether it says *how* to consume the file:

| file | tokens | said how? |
|---|---|---|
| `Reports/dead-ends.md` | 97,214 | yes — "grep your area" |
| `Reports/open-bugs-handoff.md` | 96,582 | **no — "Read this before touching a listed area"** |
| `PLAN.md` | 60,199 | **none** |
| `README.md` | 46,060 | **none** |

Two registers of near-identical size, routed in opposite ways: one correctly
grepped, one instructed to be *read whole* at ~97k tokens — and it has had a
generated status index the guidance never mentioned. An agent obeying that
literally burns 97k; one that balks reads nothing. `PLAN.md` and `README.md`
carry no guidance at all, which is another 106k, despite both having
navigation built for them this week.

All four rows now name the size and the entry point. That is worth far more
than any compression pass: it changes what a session *loads*, not what it
*carries*.

**Standing check for anyone editing the knowledge table:** a row that names a
file over ~15k tokens must say how to enter it. Naming the file is not
routing.

## 2026-08-26 — "grep your area" was the wrong unit; the index it seemed to need is a dead end

Audited the four documents `CLAUDE.md` routes to (~300k tokens, **26% of all
documentation**: 1.14M across 131 files). Three hypotheses tested and
**disproved** — record these so nobody re-runs them:

- **`dead-ends.md` is stale.** A mechanism-existence check over all 594
  entries found **1** naming symbols mostly absent from `src/`, and it is the
  `perf-lock` retirement, correct by design. Entries are substantial (0%
  under 200 characters). Content is sound.
- **The bug register's index is untrustworthy.** `bugindex.py --check` is
  clean; the index carries status, line and description for 77 bugs (39
  open), and the recommended path costs **~4,334 tok** against 96,582 whole.
- **`PLAN.md`'s issues backlog is rotting.** Better maintained than expected —
  resolved issues struck through with closing evidence, #2 explicitly
  *"deprioritised by measurement, not closed"*.

**The real problem was the unit of search, not the file.** Measured cost of
following "grep your area" in `dead-ends.md`:

| grep unit | plants | structural | liquids |
|---|---|---|---|
| prose, by area | ~29,101 | ~12,240 | ~12,335 |
| by address prefix | ~15,455 | ~11,728 | ~4,953 |
| **by mechanism** | `thicken` ~2,458 · `max_unsupported_span` ~651 · `chunk seam` ~251 · `rot_remains` **0** | | |

**A 10–50x reduction from changing one noun.** A zero-hit mechanism grep is a
real answer, delivered for nothing.

**I was one step from building a generated topic index for it, and that would
have been the wrong fix.** It would add a generated block to a 97k co-owned
file, need a new script and `--check`, and still leave a floor of ~8.4k tokens
for plants (128 address-matched entries x ~66 tok each). A wording change beat
it at zero cost and zero maintenance. **`dead-ends.md` needs no structural
change** — do not propose one.

Useful structural fact found on the way: **592 of 595 entries (99%) open with
a machine-readable file address** across 77 distinct files, 30 covering 80%
of entries. That is what makes the address-prefix grep work as a second
resort.

### The grep false-negative got a tool, because a rule was not a fix

Asked whether writing the rule had actually fixed anything. It had not — the
rule asked the reader to strip markup and collapse whitespace **by hand,
mid-task**, which is the exact discipline this file's own recurrence audit
found does not survive a real session (seven rules existed and the mistake
recurred anyway). I made this mistake twice today with `CLAUDE.md` loaded.

`python3 scripts/docgrep.py "the phrase as it reads"` now does it. It imports
`addrcheck.normalise` rather than reimplementing it, so the two agree by
construction, and prints `file:line`, exiting 1 on no match. Verified against
the two phrases that actually fooled me (`Six seeds is not a sweep`, `By topic
table maps subsystem` — both **0 hits** under plain `grep`, both found), a
heavily marked-up phrase quoted as it reads, and a negative control that must
still fail.

**The general point, which is the reusable half:** when a rule asks for a
multi-step manual procedure, that is a signal the fix is a command, not
prose. Check whether the normalisation, parsing or comparison the rule
describes already exists somewhere in `scripts/` — here it did, in a checker
built the same day, and the rule had been written without noticing.

### Applying "a procedure wants a command" found a stale command

Swept `CLAUDE.md` for rules that ask for a multi-step manual procedure and
checked each against `scripts/`. Three real candidates, three different
verdicts — the mix is the useful part:

**1. Genuinely prose, left alone.** *"Before trusting any guard, put the fault
it is named for back and watch it go red."* Constructing the fault a guard is
named for is judgement, not a procedure. No command can do it.

**2. Prose duplicating machinery that already exists.** The bug-register rule
said to *"check the letter is not taken"* by hand. `bugindex.py --check`
already does it and is gated by `docscheck` — fault-injected to confirm rather
than trusting its "identifiers unique" message: a duplicated `### D3.` gives
`identifier 'D3' is used by 2 entries (lines 468, 501)`, exit 1. The rule now
names the command.

**3. A command that had gone stale, and nobody noticed because it was
asserted so firmly.** The Commands section read `cargo test -- --skip
root_and_shoot_branching_read_different_slots   # the --skip is not optional`.
**It is optional.** Bug A's test is `#[ignore]`d, so it never runs. Measured
2026-08-26: `cargo test --lib` with no flag at all gives **943 passed / 0
failed / 54 ignored**, exit 0. The premise was true when that test ran red;
someone later ignored it and the guidance never caught up, in the most-followed
section of the file.

The general rule in the red-suite gotcha survives and is why the entry stays —
*while any gate is quarantined, whatever runs after it is not being run
locally* — but its specific instance is closed and now says so.

**Carry this:** an emphatic qualifier (*"not optional"*, *"always"*, *"never
skip"*) is where staleness hides longest, because the emphasis discourages the
check. When applying the procedure-wants-a-command lens, **test the premise of
the command, not only whether a script exists.**

### The benchmark measured the wrong thing; fixed

Re-ran the cold-agent benchmark after this week's routing work: **3/3, trap
refused, 0 source reads, 6 files opened — unchanged**, with tool calls *up*
17 → 23.

**That is the instrument failing, not the docs.** The routing landed this week
deliberately trades file-opens for narrower reads (grep the mechanism not the
area; enter the bug register by its index). A metric that cannot move when the
thing it measures improves is not measuring it. Two more defects: it counts a
200-token grep and a 30k read alike, and it is blind to `CLAUDE.md`, which the
harness **pre-loads** into a subagent — so earlier runs that *read* it counted
it and later runs that merely *used* it did not. The series was incomparable
and nothing in the record said so.

**Headline is now `agent_tokens`** — the harness's own `subagent_tokens`
figure. Objective rather than self-reported, absorbs the pre-loaded file
instead of needing a correction, denominated in what routing actually changes.
First point: **101,669** on 2026-08-26. The two earlier runs cannot be
back-filled and stay in the record unconverted, so the change is visible.

`scripts/docbench.py` holds the canonical prompt (`prompt`), the history
(`runs`), and a **positive control** (`check`) that verifies each question
still has an answer — fault-injected to confirm it fires. Without it a
question whose answer had left the corpus would score the instrument and read
as a regression.

**Two general points worth carrying past this instrument:**

- **A metric you chose before the change is a hypothesis about what the change
  will do.** Re-examine it when the change lands, not only the number.
- **The prompt now lives in a script, not in prose.** A prose prompt invites
  paraphrase between runs, and a paraphrased prompt silently breaks a time
  series — the same reasoning that moved the grep gotcha into `docgrep.py`.

### The A/B: inconclusive, and that is the honest answer

Built the missing baseline by running the benchmark against a detached
worktree at `65934a5^` — main immediately before the audit's first merge.
Same prompt, same model, only the tree differs. Verified first that all three
questions were still answerable there.

| arm | agent_tokens | correct | trap | source reads |
|---|---|---|---|---|
| pre-audit `2f5de1e` | **105,073** | 3/3 | refused | 0 |
| current `f3df928` | **101,669** | 3/3 | refused | 0 |

**−3,404 (−3.2%), which is noise at one run per arm.** This lane has spent the
week telling other people that a bar from a single run is a sample from a wide
distribution; the same applies here. **Do not quote this as an improvement.**

What it does establish: the changes did not make navigation *worse*, and both
trees answer 3/3. The point estimate runs the intended way against a headwind
— current `CLAUDE.md` is +4,456 tokens, pre-loaded and paid before anything is
read, so routing had to recover ~7,860 to net −3,404. **That is arithmetic on
one pair, not evidence.**

To settle it: three runs per arm, alternating, ~600k tokens, report the
median. Confound that survives regardless — the trees differ in more than
documentation, since other lanes landed code in between.

**And a caution worth more than the result.** The baseline arm reported,
confidently and specifically, that *"CLAUDE.md sends you to a README By topic
table that does not exist"*. **Verified false** — that tree's `CLAUDE.md` has
zero occurrences of "By topic", and so does its README. I nearly relayed it.
A subagent report can contain a fabricated, checkable defect stated as fact;
check the cheap ones before passing them on.

## 2026-08-26 — the benchmark could not have shown improvement

The A/B I ran to answer *"did the audit help?"* came back inconclusive
(105,073 → 101,669 agent tokens, −3.2%, one run per arm). Re-reading the
series explains why, and the explanation is not noise.

**Set A has scored 3/3 correct and 1/1 trap in every run it has ever had,
including the pre-audit baseline arm.** The correctness half has never
discriminated. `check` verified that every question still *has* an answer —
specificity — and nothing ever verified that the score could *move*. That is
this repo's own rule (*put the fault back and watch it go red*) applied to an
instrument rather than a test, and it was not applied when the instrument was
built.

Worse, the one change with a 10–50x token effect — `dead-ends.md`'s search
unit, area → mechanism — is invisible to set A. Q2 **names the mechanism in
the question**, so the agent never has to choose a unit of search. The
guidance that changed is bypassed by the question meant to exercise it.

### Set B

Four questions, each verified against `2f5de1e` **before** it was written and
kept only because the old answer was wrong or missing:

| | old-tree answer | why it was wrong |
|---|---|---|
| B1 ownership | "plant.rs is uncontested" | no plants row existed; the table asserted everything collided in `app.rs` (6th, at 51) |
| B2 ethos | rule found, precedent absent | generic graded-outcome clause present, zero precedent outside destruction |
| B3 search unit | grep the liquids area | ~12k–31k tokens vs ~250–2,460; **names no mechanism**, so the agent must choose |
| B4 referral trap | "yes, safe" | `fracture-mechanics-design.md` said *read that to build this* of the superseded handoff, unwarned |

B4 is deliberately a *different* trap from set A's: A hands the agent the
unsafe document, B hands it a document that recommends the unsafe one.

**Set B is a regression set, not a capability test.** It is aimed at four
specific repairs. A future change that fixes something else will not move it,
and a score on it must never be read as "the documentation is good".

### What fault injection found, in 2.0 seconds

Both are recorded because neither was findable any other way — both passed
`check`, and both would have passed it with the documentation they guard
deleted:

- **B2 was blind.** It searched the whole of `CLAUDE.md` for `rot_remains`,
  which also appears in the dead-ends row as an example of a grep that returns
  *nothing*. Rescoped to the ethos section.
- **B3b's injection was blind**, not its control: it replaced 1 of 6
  occurrences, so a surviving copy hid the fault. Replace every occurrence.

Landed as `python3 scripts/docbench.py selftest` — six injections, ~2 s, no
rebuild. **The discipline was expensive as prose and is nearly free as a
command**, which is the same finding that produced `docgrep.py`.

Also added: written rubrics for both sets (`docbench.py rubric a|b`). Set A
ran four times with none, graded by whoever ran it — two graders disagreeing is
indistinguishable, in the record, from the documentation changing.

**Set B has not been run.** Its first pair should be current `main` against
`2f5de1e`. If the old tree scores well, the questions are wrong, not the docs.

### Set B ran the same day, and falsified three of its own four premises

| | current (`bfc8582`) | pre-audit (`2f5de1e`) |
|---|---|---|
| correct | **4/4** | **3/4** |
| agent tokens | **95,518** | **135,580** (+42%) |
| files opened | 10 | 14 |
| tool calls | 23 | 29 |

Predicted 4/4 against 1/4. **The prediction was wrong because the
qualification method was wrong:** each question was checked against the old
`CLAUDE.md` and never against the old *corpus*. This documentation is
redundant — the same fact lives in a report, an index and a wiki page — so
*"`CLAUDE.md` did not say it"* is not *"the agent cannot find it"*.

- **B2 does not discriminate.** The old tree produced three non-destruction
  precedents: `wiki/plants.md`'s *"gradual and it is graded"*,
  `dead-ends.md:754` (hard-threshold leaf shedding rejected in favour of
  graded, with numbers), and `rot_remains` senescence from the bug register.
  What the ethos reframing bought shows up only in the baseline's own caveat —
  *"framed entirely in destruction vocabulary, so you have to read it across
  to plants"* — which is work, not failure.
- **B4 does not discriminate, and its premise was false.** `Reports/README.md`
  already carried "superseded by landing" on the old tree; the baseline arm
  refused the trap through it, calling it *"a clean case of the header being
  stale and the index being right"*. The audit moved the warning one hop
  earlier. It did not defuse an armed trap.
- **B1 partly discriminates.** No counts on the old tree, correctly — but it
  never said "uncontested". It reached the same land-quickly instruction via
  `plant-implementation-split-2026-08-23.md` and `plant-work-split.md`, and
  caught the old table's false `app.rs` claim against the split report's
  *"filmstrip.rs is the most-collided file in the repo"*.

**The finding that outlives the question set: correctness saturates on this
corpus whatever you ask.** Set A: 3/3 across four runs. Set B, written
specifically to break, 3/4 on the tree it was built to fail. The quantity that
separated the arms was **cost**, and three independent measures moved together
— tokens +42%, files 14 vs 10, tool calls 29 vs 23.

**So the audit did not change what is answerable. It changed what answering
costs.** That also explains the inconclusive A/B above: set A is a correctness
instrument, and correctness was never the thing that moved.

Confounds, on the record: one run per arm; and the baseline prompt carried an
extra anti-contamination preamble the current arm did not, which plausibly
costs tokens by itself — so **+42% is an upper bound on the routing effect,
not a clean estimate**. The isolation itself held: the baseline arm reported
*"the documentation gives no numbers for either file"* and named the revision
difference it had been told to disregard.

## 2026-08-26 — the wiki audit

The last unaudited documentation. **The mechanical half is clean**: 11 of 12
pages carry a real date, and every one matches its file's last commit exactly
— no `this build` placeholders, no invented dates. `wiki/README.md` has none
and needs none; it is an index, and `docscheck` does not ask for one.

**A drift proxy was built and it over-fires**, which is worth recording before
its output: "commits to a page's mapped source since the page's date" scored
`structural-collapse.md` at 10, `powders.md` 7, `world-cycles.md` 6. All three
are **clean**. The count is inflated three ways — merge commits carry no
content; perf commits say so themselves (four of `world-cycles.md`'s six are
explicitly *bit-identical*); and a change to one subsystem's *code* is often
documented on a different *page*, correctly. Felling lives in `structural.rs`
and `rigid.rs`; the player-facing account of it belongs on `plants.md` and
`the-gnome.md`, and `plants.md` had it. This is the same failure class as the
file-overlap metric `CLAUDE.md` warns about under merges: a proxy that looks
like a defence and fires on correct content. Use it to order the reading, never
as a finding.

### Two real findings, both on pages nobody would have suspected

**`wiki/the-gnome.md` carried two statements that were false**, and the timing
is the interesting part. The page was last written at **10:18 on 2026-08-23**.
`43adeb6` — *"let a tool hurt a plant"* — landed at **09:56 the same day,
twenty-two minutes earlier**. So the page was revised *after* the behaviour
changed and did not pick it up:

- *"the pick sees straight through living wood to the rock behind, so you can
  dig in a wood and dig while standing inside a tree"* — false.
  `rigid::is_tool_target` (`rigid.rs:308`) accepts `Solid | Plant`, and the D2
  guard `a_blow_cuts_living_wood` asserts a blow on a living trunk removes
  living trunk. Bedrock stays exempt, guarded separately.
- *"Cutting a tree down is deliberately not in yet."* — false, and gated in CI.
  `scripts/acceptance.sh:490` runs `scene=fell fell=6000 … min_severed=1000`.
  `43adeb6` measures it: six bites sever the bole, standing living tissue
  2,906 → 409, both drivers agreeing (2,360 parallel, 2,398 serial). `bbbd789`
  then made the crown come down as pieces rather than sawdust, ten hours after
  the page was written.

This is the page an agent consults about what the player can *do*. It was
answering "can he cut down a tree?" with "no".

**`wiki/weather.md` never documented exposure-scaled gusts.** `ea061eb` and
`107355c` (both 2026-08-23) made a gust's strength a function of how sheltered
the ground under it is — `weather.rs:1022`'s `exposure`, guarded by
`exposure_is_read_only`. The page's "Wind and storms" section described gusts
as uniform. Added.

Both pages re-dated 2026-08-26.

### Why this mattered more than it looks

The set B run the same day showed a wiki page is a route agents actually take
for design answers — the baseline arm got its non-destruction precedent from
`wiki/plants.md`. A stale wiki page is not cosmetic; it is a confident wrong
answer on a path we now have evidence gets walked.

**One thing left for the owner, not fixable from the code.** The exact
button-level interaction is worth a playtest check: left-clicking a plant you
are *pointing at* still shakes it (the `shake_*` tuning is intact), while a
blow that lands on wood now cuts it. Which one you get when both could apply is
stated here from the code's structure, not from playing it.

## 2026-08-26 — wiki structure: what the evaluation found, and three of four fixed

Measured shape, not impressions:

- **Index complete** — `wiki/README.md` lists all 11 pages, no orphans.
- **Two organising conventions.** Six pages use `##` sections (plants 13,
  the-world 8, the-gnome 8, weather 7); four use none —
  `structural-collapse.md` is **357 lines with one heading, at line 291**,
  then `fire-and-heat` 131, `explosions` 112, `liquids-and-gases` 110.
- **The link graph splits by kind, not by age.** Material pages cross-link
  densely (weather 7 inbound, structural-collapse 5, fire-and-heat 4);
  entity and system pages were islands reachable only from the index —
  `plants.md`, `the-gnome.md`, `ants.md`, `the-world.md`, `world-cycles.md`
  at **0 each**. Checked and rejected the obvious explanation: all 12 pages
  were created within three days of each other, so this is not age.
- **`this build` ×6 in body prose**, across four pages.

### Fixed

**1. The `this build` class, and a gate so it cannot come back.** The phrase
`CLAUDE.md` bans in freshness notes was standing in the body, where
`docscheck` never looked. All six now name a date. The gate is deliberately
narrow — only `this build`, not `currently`/`recently`, which have honest uses
here (*"ground where nothing is currently growing"*); a check that fires on
correct content stops being read, which is the lesson from the drift proxy
above. Fault-injected: appending the phrase to a page turns `docscheck` red
and removing it turns it green again.

**The strongest argument for the gate is that this audit broke one itself.**
`weather.md`'s *"until this build, snow sped up freezing"* meant 2026-08-22.
Re-dating that page earlier the same day, for the unrelated exposure fix,
silently repointed the sentence at 2026-08-26. A phrase anchored to the
freshness note is broken by any later edit to the note — including a correct
one — so the anchor has to be written into the sentence.

**2. The subsystem → page map**, in `CLAUDE.md`'s knowledge row rather than in
the wiki. It was drafted for `wiki/README.md` first; that page's own contract
is *"no code, no file names, no implementation detail"*, so putting a map of
`src/` paths there would have broken the premise the page opens with. Half the
mappings are not guessable — nothing suggests `field.rs`, `decay.rs` and
`sky.rs` land on `world-cycles.md`.

**3. `plants.md` back-linked from the four pages with a reason to**: fire,
structural collapse, the gnome, the world. **0 → 4** inbound.

### Left, deliberately

**Section headings for `structural-collapse.md`.** Its 13 bold leads are
sentence openers, not titles (*"**And it arrives as rock.**"*), so this needs
authored headings rather than a script, and that is putting words in the
owner's prose. Proposed, not applied.

**The other four island pages** — `the-gnome`, `ants`, `the-world`,
`world-cycles` still have zero page-to-page inbound links. Same class as the
`plants.md` fix and the same cheap remedy; not done because only `plants.md`
was scoped.
