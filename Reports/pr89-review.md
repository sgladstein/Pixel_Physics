# Adversarial review of PR #89 — the agent-strategy work

**Status: reviewed 2026-08-28, at PR head `b24522a`.** Findings 1, 2, 3, 5 and
the record corrections were fixed in the same branch as this report; findings
6-12 are recorded here and left as they stand. Written against
`Reports/agent-strategy.md`, `scripts/contextbudget.py`, `scripts/readguard.py`,
`scripts/lanecheck.py` and the two `.claude/workflows/` changes.

**Verdict: the central conclusion survives; the largest number does not.** The
direction — *reading dominates, so partition the reading and price the topology
by head count* — is right and holds under every perturbation applied. But the
biggest claimed lever (§6, ~289,100 tokens) is computed by comparing the worst
case of one arm against the best case of the other, and the PR contains both
premises, in two files, without noticing they contradict. Two of the four new
pieces of machinery could not go red for the condition they name.

Everything below was run, not read.

---

## 1. §6's cache saving mixed premises across the two arms — wrong sign

**Claim.** `subagentPromptCacheTtl: "1h"` saves ~289,100 tokens on a 12-agent
census against a ~18,200 downside, "about 16:1".

**The arithmetic is exactly right and it answers a different question.** Every
figure reproduces (364,425 / 75,314 / 289,110 / 18,221). The premise differs
between the arms:

| arm | premise used |
|---|---|
| 5m: 364,425 | all twelve **miss and write** — agents serialised, each starting >5m after the last write |
| 1h: 75,314 | one writes, **eleven read** — agents serialised, each hitting the previous write |

Those cannot both describe one run. The 5m arm is priced under all-miss, the 1h
arm under all-hit. *Whichever bar you pick applies to both signs* — `CLAUDE.md`'s
own rule, from the noise-bar section.

**The PR already recorded the premise that breaks it.** `world-review.js`, added
in the same diff: *"`parallel([...])` launches all seven at once, and if they
start together they may all miss and all write."*

| premise | 5m | 1h | saving |
|---|---|---|---|
| PR's (serial misses vs serial hits) | 364,425 | 75,314 | +289,110 |
| concurrent launch, cache shared within a phase (10 census + 2 later) | 85,032 | 75,314 | +9,718 |
| concurrent launch **races**, all in a phase write (the PR's own caveat) | 364,425 | 490,759 | **−126,334** |

**Checked against the Anthropic prompt-caching documentation**, which settles it
further. Multipliers confirmed: reads ~0.1x, writes **1.25x at 5m, 2x at 1h**.
And:

> A cache read **refreshes the entry's timer at no additional cost**, on either
> TTL. […] Requests that share a prefix and start less than 5 minutes apart keep
> the 5-minute cache warm **indefinitely** — the 1-hour TTL buys nothing there
> except the doubled write price.

A fan-out is continuous traffic by construction: ten census agents looping tool
calls each issue requests sharing the prefix, and every read pushes the 5m timer
forward. For §6's premise to hold, the run would need a >5-minute window in which
no request sharing the prefix started. During an active census that window
essentially cannot open.

**Which makes the PR's own downside figure the default case.** 5m = 1 write +
11 reads = 2.35P; 1h = 2.0 + 1.1 = 3.1P; 1h costs 0.75P more = **18,221 tokens**
— identical to the "penalty if all twelve *did* fit inside five minutes" the
report computes and files as the unlikely branch. The documentation says that
branch is the ordinary one. The report has the right number in the right place
with the wrong label on it. Break-even confirms: 5m pays off at two requests,
1h needs three.

**Fixed:** the setting is reverted and §6 rewritten with the branches relabelled.

**Partly measured since, and the rest is free** (`scripts/cacheprobe.py`, added
with this review). This review originally closed by calling the remaining
question un-answerable without an instrumented fan-out costing ~100k tokens.
That was wrong, and wrong in this repo's most characteristic way — *reaching for
a new measurement before checking what already exists*. Claude Code writes every
turn's `usage` to `~/.claude/projects/<cwd>/<session>.jsonl`, including
`cache_read_input_tokens` and a `cache_creation` split **by TTL**, and it tags
sub-agent turns `isSidechain: true`.

Measured on this review's own session, 112 turns: the instrument is not blind
(107 of 111 turns after the first show a cache read), and **every write in the
main conversation is `ephemeral_1h` — 983,153 tokens against 0 at 5m.** §6 read
the main session's 1-hour TTL off a schema; it is now measured, and it is right.

What remains needs no extra spend: point `cacheprobe.py` at the transcript of any
fan-out that was going to run anyway. It reports three questions apart, because
conflating them is what broke §6 — **namespace** (do sub-agents share a prefix at
all; needs a *sequential* launch), **race** (launched concurrently, how many
miss), and **TTL**. Namespace alone can void the lever: if sub-agents do not
share, `subagentPromptCacheTtl` cannot help a fan-out at any value.

`subagentPromptCacheTtl` is a harness setting rather than an API feature, so the
transcript is the only evidence available either way. The PR was honest that it
read the schema rather than measuring; that disclosure holds up.

## 2. `readguard.py` denied a whole-file read of **every** `README.md` in the repo

`path.endswith(rel)` with `rel = "README.md"` matched six files, not one:

    wiki/README.md            2,362 B  -> DENIED, message claimed "~46,833 tokens"
    .claude/README.md         4,605 B  -> DENIED
    Reports/lanes/README.md   3,401 B  -> DENIED  (added by this PR)
    Reports/README.md        40,493 B  -> DENIED  (with the wrong file's guidance)

Every one is an index document `CLAUDE.md` routes agents *to*. A 590-token read
of `wiki/README.md` was refused with a message claiming it cost 46,833 tokens and
pointing at a **By topic** table it does not contain.

This is the failure the module docstring forbids: *"A guard over the cost of a
correct action must never be able to block the action itself."* Fail-open
protects against errors, not against a guard that is confidently wrong.

**Fixed**, with a selftest so it cannot regress.

Two further holes, now closed:

- **`limit: 999999` passed.** The guard tested presence of `offset`/`limit`, not
  magnitude, and its own denial text said *"Re-issue the Read with offset/limit
  for the part you need."* The redirect named the bypass.
- **`cat` bypasses it entirely** — `tool_name: "Bash"` returns allow. Not fixable
  in this guard, and left as-is: in auto mode an agent is told to prefer Bash for
  file reads, so the repo's default working style routes around it. Still worth
  having — the denial's pointer is the real value — but §5's table said
  "Enforces", which overstated a nudge on one tool. Reworded.

## 3. The 28,000 ceiling was enforced by nothing, and the only check went green on a violation

§5's table listed *"The always-loaded budget cannot grow unwatched —
`contextbudget.py --gate`, `--check` via `docscheck` check 9."* `--gate` was
invoked by no script and no CI job. `docscheck` called `--check` only, which is
staleness.

Demonstrated by appending 40,000 bytes to `CLAUDE.md` (44,295 tokens, **58% over
ceiling**), then running `--write` and `--check`:

    contextbudget: 44,295 tokens ... over by 16,295     <- --gate, run by nobody
    contextbudget: record current                        <- --check, rc=0
    Ceiling 28,000 (**+16,295 over**)                    <- the recorded block
    docscheck: clean

The only automated check went red for a *stale record*, and the remedy was to
regenerate the record — which happily wrote down "+16,295 over" and turned it
green. `CLAUDE.md`'s rule: *"A size cap must bound work, never gate whether
something happens. Does exhausting the cap produce an answer, or merely less
work? An answer is the bug."* Here exhausting the cap produced a **record**.

**Fixed** by wiring `--gate` into `docscheck` as check 9b — a *separate* exit
from the staleness check, preserving the split the source correctly argues for
— with a selftest row. Blast radius is right: `docscheck` is informational in
CI, so this goes red locally and in the informational job without breaking a
build.

## 4. `bytes/4.0` was calibrated against itself

`contextbudget.py` said: *"Tokens are bytes / 4.0, this repo's own published
calibration — the audit's 65,182 B = 16,300 tokens is 3.999 B/token."*

Every row of the audit table it cites is exactly bytes/4:

    381,944/4 = 95,486    343,196/4 = 85,799    235,872/4 = 58,968
    171,194/4 = 42,798     65,182/4 = 16,295     26,571/4 =  6,642

The 3.999 is the assumption round-tripped, presented as its validation. No
tokenizer measurement exists anywhere in the chain. **This is the repo's own
most-recurring failure — arithmetically correct, answers a different question.**

**Sensitivity — one site changes a conclusion:**

| divisor | CLAUDE.md tokens | headroom vs 28,000 |
|---|---|---|
| 3.5 | 27,766 | **+234** |
| 4.0 | 24,295 | +3,705 (the claimed "~15%") |
| 4.5 | 21,596 | +6,404 |

At 3.5 B/token — plausible for prose this dense in tables, pipes and backticks —
headroom is 0.8%, not 15%. Everything else is divisor-invariant: the corpus
ranking is a sort; §6's ratio has the divisor in both terms; `MATERIALITY_FLOOR`
is compared against savings in the same estimated units.

A second flaw never stated: the 26/74 split has an **estimated numerator over a
measured denominator** (24,295 by bytes/4 over 95,518 reported by the harness).
That alone moves the split across 23%-29%.

**Fixed:** the provenance is restated as unvalidated, and the ceiling is now
carried in bytes as well as tokens so the gate's real threshold is legible.

## 5. `lanecheck.py --selftest` was blind

Its docstring said the size rule *"gets its control here — otherwise the rule
this file is named for would be the one thing never shown able to fire."* It
still was. The selftest wrote temp files then asserted `big.stat().st_size >
CAP_BYTES` inline; it never called `oversized()`, `check()`, or any code path the
check uses. It proved Python's `>` operator works.

Gutting `oversized()` to `return []`:

    lanecheck: positive control -- a note one byte over the cap is flagged
    lanecheck: negative control -- a note one byte under is not flagged
    lanecheck: 0 live note(s) currently over the cap
    RC=0

Both controls "passed" against a check that could not fire. The only tell was
`0 live note(s)`, unasserted, reading as good news.

`docscheck --selftest` could not cover it either — its `lanecheck-cap` row
injects a *cap-drift* fault, testing number-agreement, not the size rule.

**Fixed:** the selftest now drives `oversized()` over a real directory.

## 6. The 74/26 split — conclusion sound, framing overstated

**Better than n=1.** `docbench.py` records *two* arms, 95,518 and 135,580. The
second gives 18% prefix / 82% reading. Both straddle 18-25% prefix, both point
the same way. The report could have said "18-25% across two recorded arms" for
free. The ordering would need the split past ~50/50 to flip. **Sound.**

**Worse, and this is the real correction:** the 74% bucket is not reading.
`agent_tokens` is everything the subagent consumed — tool results *plus* the
harness system prompt, tool schemas, the task prompt, and the agent's own output.
`contextbudget.py`'s own docstring admits it: *"the harness system prompt and
tool schemas add more that this repo does not control."* Those tokens are counted
as **not-prefix** in §1's split and as **prefix-we-don't-control** in the script —
same tokens, two buckets, opposite sides. They are also multiplied by head count,
so they belong on the prefix side. With ~15k for Claude Code's system prompt and
tool schemas, the split is ~41/59, not 26/74.

Direction is worth noting: the error makes the cacheable shared prefix *larger*,
which makes §6's lever bigger — an error that flatters the biggest claim.

**External validity, which nobody raised.** Both arms are `docbench` agents —
documentation-question agents, the most read-heavy task class in the repo. §2's
decision table and §3's session-lifetime rule are about *work* sessions:
write-heavy lanes making code changes with few large reads. Generalising a
read-heavy agent's split to those is unsupported. The topology advice stands on
the disjoint-partition argument, not on 74%, but 74% should not be quoted at a
lane.

## 7. The Sonnet conclusion reverses under its own stated caveat

The caveat at `world-review.js` is **stated honestly** — it says plainly that
warmth "is an assumption, not a measurement", names `parallel()` as the reason to
doubt it, and names the right fix. Credit where due.

But the sentence it ends on — *"re-pinning this lens to a cheaper model would
still not be the answer"* — is asserted and false under the caveat's own premise.
Reconstructing the figures: `$0.013` is an **Opus warm read** (26,780 × 0.1 ×
$5/M) and `$0.107` is a **Sonnet cold write** (26,780 × 2 × $2/M). The "8x"
compares *Opus-warm* against *Sonnet-cold*. If the prefix is cold for everyone —
the case the caveat raises — Opus cold write is ~$0.268 and Sonnet is **2.5x
cheaper**, not 8x dearer. The sign flips.

The honest conclusion: *if the prefix is warm, remove the pin; if cold, the pin
was saving money.* Removing it may still be right on quality grounds — but the
argument made was purely cost, and the cost argument does not survive its own
caveat. Left as-is (removing the pin is defensible); the comment is corrected.

## 8. "1.00x duplication" is a property of the partition, not a measurement

§2's table reported `doc-audit-agent-framing.js` at **1.00x** over ten agents.
The file lists are disjoint *because they were written as a partition*; reading
them back and finding no overlap is a tautology.

It is also not 1.00x for the run as it exists. The `parallel()` call launches
**twelve**, and `planRefsThunk` is instructed to grep across `CLAUDE.md`,
`README.md`, `Reports/*.md`, `wiki/*.md` and all of `src/`, `examples/`,
`tests/`, `scripts/` — files assigned to `census:readme`, `census:plan-md` and
every source group. `claudeMdThunk` reads `CLAUDE.md` in full. The quantity that
should have been measured is *files read*, which nothing measured.

The report also says "10 census" in §2 and "12-agent census" in §6 for the same
workflow. The saving scales with head count, so this is not cosmetic.

**Conclusion unaffected** — disjoint partitions are the right design. Only the
evidence class was misstated. **Fixed** in the report's wording.

## 9. The new `Reports/README.md` bullet split an existing one

The `agent-strategy.md` entry was inserted between
`claude-md-recommendations.md`'s first paragraph and its continuation, so *"Only
**rec 6** is provably unblocked…"* came to hang under `agent-strategy.md`, which
has no numbered recommendations.

This is the exact failure `docbench.py`'s new §2 — added in the same diff — names
as *"an address that no longer resolves reads to the next agent as 'the content
is gone'."* `docscheck` was clean: it gates links and index presence, not bullet
continuity. **Fixed.**

## 10. The 91% figure — crude classifier, finding holds

Keying on `→` makes 91% an upper bound. Checked by hand rather than by marker:
2 of 16 sections carry `→ perf`; the other fourteen read as *"README audited
against the agent lens; verdict is keep"*, *"the benchmark measured the wrong
thing; fixed"* — genuine findings for the record, not messages to another lane.
The classifier is crude; the substance supports it.

**The cap does not need the figure.** `docs-audit.md` is 47,168 B / ~11,792
tokens, paid by every session following the `SessionStart` pointer. Even if all
of it were addressed messages, that is a channel that has become a document.

What the cap does **not** address is the cause: `lanes/README.md`'s own rule is
that a finding belongs in a report and the note says it *happened* — the note
should hold pointers. Most of `docs-audit.md` also landed in reports, so it is a
duplicate, not an overflow. A size cap treats the symptom. Left as-is.

**Enforcement is the weak part.** Check 10's hard exit gates only that two
numbers in two files agree — the *statement* of the cap, not the cap. The
reasoning for warn-not-fail is good and is kept, but this row should not be
listed as machinery on the same footing as `permissions.deny`. Reworded.

## 11. `MATERIALITY_FLOOR = 10,000` — defensible, with one uncomfortable interaction

Fine as an absolute: ~10% of one agent run, correctly admitting `readguard`
(46k-107k per averted read), the prefix budget, §6 and the bugs split while
excluding the 2,940-token workflow-prefix repair — which the PR made anyway and
labelled *"free to keep right and not worth contorting the prompt for"*. That is
the floor used honestly.

The problem is the **interaction with finding 10**. The lane-note item is worth
~10,717 tokens — the one item within 8% of the floor — and its 91% is an admitted
upper bound. If half those sections were addressed-without-marker, the saving is
~5,400 and falls below the floor. The single item nearest the line rests on the
softest number, and the two were never cross-checked. Not "picked to justify the
work" — the floor predates it and excludes an item the author had already done —
but not independent of it either. Recorded, not changed.

## 12. Deferring the `open-bugs-handoff.md` split — right call, weaker reason than available

The stated reason (append-only, co-owned, no `union` merge, wants one commit at a
quiet moment) is a scheduling argument, and sits oddly beside a PR that lands
changes to `CLAUDE.md`, `.claude/settings.json` and `Reports/README.md` — all
contested.

**The stronger reasons went unstated.** `scripts/addrcheck.py` exists because
`dead-ends.md` addresses entries by document-and-heading, and *"a rename is
silent."* Relocating 38 lettered entries is a mass rename of an address space
`addrcheck.py` currently covers only for `dead-ends.md` → `README.md`/`PLAN.md`.
And `CLAUDE.md` records what a botched merge of that file did (§M re-opened after
being closed). Those are technical blockers; "wants a quiet moment" is not.

**And `readguard.py` weakens the case for doing it at all** — an argument the
report had available and did not make. The 67,027-token figure is the cost of one
unguided *whole-file* read. With the guard in place that read is denied, so the
realistic saving drops to whatever a sliced entry into a 40k-token register saves
over a sliced entry into a 108k one — much less, since the generated index at the
top is what both readers use. Ranking the split at 6.7x the floor uses a
pre-guard number to justify a post-guard priority.

---

## What is sound, plainly

- **The topology axis.** "Does the agents' reading overlap" is the right
  question, better than "manager or not", and the decision table follows from it
  correctly. It does not depend on 74/26 being exact.
- **Session lifetime as a task boundary, not a token ceiling.** The amortisation
  arithmetic (100k work → 19% overhead; 30k → 44%) is correct, and reusing
  `branchcheck.sh`'s `files > 300` instead of inventing a new instrument is right.
- **`readguard`'s fail-open design and its redirect-don't-block posture.**
  Verified working. The reasoning for a hook rather than a `permissions.deny`
  (a permission rule cannot read `offset`) is correct.
- **`docscheck --selftest` itself.** All six original rows go red for their
  injected faults. The gap was that neither new row injected the condition its
  section is actually about.
- **`docbench.py`'s new "cheap half first" section.** The three-question
  decomposition — arithmetic, static, sampled — and "exhaust 1 and 2 before
  spending anything on 3" is the best thinking in the diff, and it is honest that
  a single confirmation run is defensible only because the floor drop is
  deterministic and correctness has saturated.
- **The prefix-shape rule** (shared brief first, per-agent text after a marker)
  and the `doc-audit-agent-framing.js` fix implementing it. Correct, cheap, and
  correctly kept in proportion at ~1% of the fan-out's prefix bill.
- **The cache-prefix churn measurement**, and its remedy being *batch the edits*
  rather than *edit less*.

## Method note for whoever reviews the next one of these

Three of the twelve findings are the same failure in different costumes, and it
is the one `CLAUDE.md` already names most often: **a number that is
arithmetically correct and answers a different question.** §6's saving (right
arithmetic, mixed premises), `bytes/4.0` (right division, circular calibration),
and 1.00x duplication (right count, tautological input). None was catchable by
re-checking the sum. All three were caught by asking what the number would read
if nothing were wrong — the positive control this repo already demands of
instruments, applied to a report's own figures.
