# Which agent strategy to use, and what enforces it

**Status: measured and partly executed, 2026-08-27.** The decision guide is
below; the machinery that enforces it is landed (`scripts/readguard.py`,
`scripts/contextbudget.py`, the `PreToolUse` hook and `subagentPromptCacheTtl`
in `.claude/settings.json`). The one large item it recommends and does **not**
do is the `open-bugs-handoff.md` split, §7.

Written because the question "should this be one session, several, or a manager
with sub-agents?" had been answered by instinct, and the instinct was aimed at
the wrong term. Every number here was measured in this repo; none is a guess
about how agents ought to work.

---

## 1. The number that decides everything

`CLAUDE.md` is auto-loaded into **every session, every agent and every
subagent** (`agent-documentation-audit-2026-08-24.md` §3). At ~24,295 tokens it
is the per-head entry fee, paid before a single line of source is read.

But it is the **minority term**. Decomposing a real measured agent run —
`docbench`'s 95,518 `agent_tokens`, 2026-08-26:

| | tokens | share |
|---|---|---|
| always-loaded prefix | ~24,295 | 26% |
| everything else — **reading** | ~71,223 | **74%** |

Both numbers matter, and they matter in different places. The prefix is
multiplied by **head count**, so it prices the *topology* decision. The reading
is multiplied by **what each head opens**, so it prices the *brief*. A strategy
choice that gets the topology right and the brief wrong has optimised the
smaller term.

**Three corrections to that table, none of which reverses it** (`pr89-review.md`
§6):

* **The 74% is not all reading.** `agent_tokens` is everything the subagent
  consumed — tool results *plus* the harness system prompt, the tool schemas, the
  task prompt and the agent's own output. Those are multiplied by head count
  exactly as `CLAUDE.md` is, so on a topology decision they sit on the *prefix*
  side; counted there the split is nearer **41/59**. The original framing —
  "the prefix is the minority term", "optimised a quarter of the bill" — was
  overstated. The ordering was not.
* **The numerator is estimated, the denominator measured.** ~24,295 is bytes/4.0
  (see §8 on why that divisor is unvalidated); 95,518 is the harness's own
  figure. The ratio moves across 23–29% on the divisor alone.
* **There are two recorded arms, not one.** `docbench` also records 135,580
  agent_tokens, giving 18% prefix / 82%. Both arms point the same way; the split
  would have to fall past ~50/50 to change any conclusion here, and nothing
  plausible gets it there.

**And both arms are `docbench` agents** — documentation-question agents, the most
read-heavy task class in the repo. A write-heavy lane making code changes with
few large reads has a different split, and this number should not be quoted at
one. The decision table below rests on the disjoint-partition argument, not on
74%.

`python3 scripts/contextbudget.py` prints the first; `--corpus` prints the
second.

## 2. The three strategies, and the axis that actually separates them

The useful question is not "manager or no manager". It is **whether the agents'
reading overlaps**.

- If they read **disjoint** material, fan-out is nearly free in the term that
  dominates: each head pays its own prefix and reads its own slice, and only a
  conclusion comes back.
- If they read the **same** material, fan-out multiplies the 74%. N heads pay
  for the same pages N times, and a coordinator that read it once would have
  paid once.

Over the repo's own harnesses:

| harness | agents | reading |
|---|---|---|
| `doc-audit-agent-framing.js` | 10 census (+2 review) | **disjoint by construction** — an explicit file list per agent, no file in two lists |
| `world-review.js` | 7 lenses | overlapping by construction — one shared image set, overlapping source |

`doc-audit` is the design to copy, and the reason it works is the explicit
disjoint file list per agent, not the topology.

**This table said "1.00x duplication, measured" and that was the wrong word**
(`pr89-review.md` §8). The file lists are disjoint *because they were written as
a partition*; reading them back and finding no overlap is a tautology, not a
result. It is also not the whole run: the `parallel()` call launches **twelve**,
and the two review agents are told to grep across `README.md`, `PLAN.md`,
`Reports/*.md`, `wiki/*.md` and all of `src/` — files assigned to census groups.
The quantity worth measuring is *files read*, which nothing measured. The design
conclusion is unaffected; the evidence class was misstated.

### The decision table

| Use | When | Why |
|---|---|---|
| **A single session** | The work is one landable change on one contested-file set. | A second head buys nothing and costs a full prefix. Most work is this. |
| **Parallel lanes** (`create_session`, separate branches) | Several independent features, each write-heavy, each ending in its own PR. | The product is a diff, and **diffs merge in git, not in a coordinator's context**. A manager here is pure overhead. Measured: five lanes touched plant code in one evening with **zero source conflicts**; all three conflicts were doc appends. |
| **Manager + sub-agents** (`.claude/workflows/*.js`) | Read-heavy, divergent work — review, census, survey — that can be **partitioned into disjoint reading**. | The expensive thing is reading, and each sub-agent's reading is discarded; only its conclusion returns. This is how `dead-ends.md` was produced. |

**The disqualifier for the third row is the partition, not the size.** If you
cannot write down a disjoint file list per agent before you start, you do not
have a fan-out — you have N agents reading the same corpus, and a single session
is cheaper.

## 3. How long a worker should last

**Not a token ceiling.** A 40,000-token reset against a ~24,295-token cold start
spends 60-70% of the budget re-reading `CLAUDE.md`. Amortisation: a worker doing
100k of work carries 19% overhead; one doing 30k carries 44%. Give a worker
enough to justify **>=100k tokens**.

**Reset on a task boundary, and the instrument already exists.**
`scripts/branchcheck.sh` prints `files`, and `CLAUDE.md` already rules that
`files > 300` means the branch has become more than one feature. That is the
session-lifetime signal. Land at feature-complete or at `files > 300`,
whichever comes first.

For fan-out sub-agents the rule inverts — short and many, because discarding
their reading is the point. But each still pays the prefix, so **head count is
a real cost**: ten heads is ~243,000 tokens before any of them starts.

## 4. Prompts

### The shape, which is not negotiable

A prompt cache is a prefix match: shared text is cacheable only while nothing
per-agent sits in front of it. So every brief is

```
[ shared BRIEF, byte-identical across all agents ]
=== YOUR ASSIGNMENT: <name> ===
[ per-agent text, including the file list ]
```

`world-review.js` and `creature-evolution-review.js` already do this.
`doc-audit-agent-framing.js` did not — its file list sat at byte 469 with 1,177 B
of identical method text behind it — and was fixed 2026-08-27.

**Do not vary `model` across agents in one fan-out.** Caches are model-scoped, so
one agent on a different model shares no prefix with the others even when its
brief is byte-identical, and rewrites the whole ~27k. Measured: the odd model out
cost roughly 8x on input against a warm read, so the cheaper model was the more
expensive one. Varying `effort` is fine when it is the default (`high`); setting
a default explicitly is equivalent to omitting it.

### The manager brief

The parts that are load-bearing are the ones that stop a lane doing something
expensive and irreversible. Everything else is task text.

```
You are coordinating <N> lanes on <program>. You own the merge; they own the work.

BEFORE DISPATCH
- Re-list the branches. A file-ownership split is only as current as your last
  look: a lane once spent a whole session believing it was alone, and filed four
  bug entries into a file another lane owned -- which had already filed all four.
- Check the split against itself. One plan gave Lane A everything under
  examples/* and told Lane C to add a mode to a file under examples/*.
- Give every lane a DISJOINT file list. If you cannot, you do not have a
  fan-out; do it in one session.

IN EVERY LANE BRIEF, VERBATIM
- "Your coordinator is session <id>. The human is not the postbox."
- The cost fork: "build the fix, OR write the finding up and stop -- not a
  half-built fix with no writeup." Lanes ran to $52-$170 unprompted; the one
  time a brief carried this fork, the lane chose in one turn.
- "Return by committing and pushing, then report the head SHA." A woken lane
  has no mcp__* tools -- files are its only outbound bandwidth.
- model: "claude-opus-5", never inherited. Three workers once inherited a
  premium tier and ran $25-71 each inside ninety minutes.

WHILE THEY RUN
- Read Reports/lanes/<lane>.md. Do NOT invent a shared status file: a shared
  append-only file is this repo's most reliable source of merge conflicts.
- The PR list is not the work list. `ahead > 0` is the only statement that
  survives a session dying.
- Verify what a lane relays -- and when a lane corrects YOU, its prior is
  better than yours. Overruling the session holding the measurement needs a
  measurement, not an argument. A correct finding was once withdrawn this way.

BEFORE YOU MERGE
- CI green on the head being merged. That is the only gate left.
- bash scripts/docscheck.sh after every merge, unconditionally.
```

### The worker brief

```
=== YOUR ASSIGNMENT: <name> ===
Files (and no others):
  <explicit disjoint list>

Never read whole: open-bugs-handoff.md, dead-ends.md, PLAN.md, PLAN-log.md,
README.md. Grep the MECHANISM, not your subsystem. A PreToolUse hook enforces
this and will deny the read with a pointer to the right index.

Fork: build it, OR write the finding up and stop. Not a half-built fix.
Return: commit, push, report the head SHA.
```

## 5. What is enforced mechanically, and why prose was not enough

`.claude/README.md` records the founding argument: `CLAUDE.md` asked every
session to run `branchcheck.sh`, and ten branches still sat at *exactly* 160
commits behind. **A check catches what a convention does not.**

| Rule | Enforced by | Strength |
|---|---|---|
| No whole-file read of an indexed document | `scripts/readguard.py`, `PreToolUse` on `Read` | **a nudge on one tool** — it cannot see `cat`, and in Bash-first working styles that is the common path. Its value is the pointer in the denial, not the denial |
| The always-loaded budget stays under its ceiling | `contextbudget.py --gate` via `docscheck` **check 9b** | real, and red when breached — but `docscheck` is informational in CI, so it stops a merge only if someone runs it |
| The recorded budget matches the file | `contextbudget.py --check` via `docscheck` check 9 | real |
| A lane note stays a channel | `scripts/lanecheck.py` via `docscheck` check 10 | **the hard exit gates only that two numbers agree.** The size rule itself warns and does not fail, deliberately — a lane writes only its own note |
| `git add -A` | `permissions.deny` | absolute — the only one here that cannot be walked past |
| Branch drift is known before acting | `SessionStart` hook | real |

**That column is new, and it is the honest version** (`pr89-review.md` §2, §3,
§10). The table originally read as six equivalent gates. They are not
equivalent, and three of them had to be corrected rather than reworded: the read
guard denied every `README.md` in the repo including the four index documents
`CLAUDE.md` routes agents to; the ceiling was wired to nothing, so `docscheck`
returned clean against a `CLAUDE.md` 58% over it; and `lanecheck`'s own selftest
re-implemented its predicate inline and passed with the check gutted. Each is
fixed and each now carries a control that has been watched going red. **A row in
this table is worth exactly what its selftest proves.**

The read guard denies only the **whole-file** read; a sliced read passes, and
the denial names the index to use instead. It **fails open** on any error: a
guard over the *cost* of a correct action must never be able to block the action.

## 6. Sub-agents and workflows default to a five-minute cache — and setting an hour was the wrong move

**Corrected 2026-08-28** (`pr89-review.md` §1). The finding below is real; the
arithmetic built on it priced the two arms under opposite premises, and the
setting it justified has been **reverted**.

`promptCacheTtl` (the main conversation) is automatic — **1 hour** on a
subscription. `subagentPromptCacheTtl` covers *"subagents, workflows, background
and helper requests"* and its automatic value is **5 minutes**. That much stands,
and it is worth knowing: the fan-out harnesses are literally workflows.

**What does not stand is the conclusion.** The original arithmetic, on a
12-agent census at ~24,295 tokens of shared prefix:

| | token-equivalents | premise |
|---|---|---|
| 5m, every agent misses | ~364,400 | agents **serialised**, each starting >5m after the last write |
| 1h, one write + eleven reads | ~75,300 | agents **serialised**, each hitting the previous write |

Those cannot both describe one run. The 5m arm was priced under all-miss and the
1h arm under all-hit, and the ~289,100 "saving" is the gap between two different
worlds. `CLAUDE.md`'s own rule, from the noise-bar section: *whichever bar you
pick applies to both signs.* This report contained the contradicting premise
itself, in `world-review.js`'s own comment — *"`parallel([...])` launches all
seven at once, and if they start together they may all miss and all write"* —
and did not notice.

**The documented behaviour settles it.** Multipliers confirmed against
Anthropic's prompt-caching documentation: reads ~0.1x, writes **1.25x at 5m and
2x at 1h**. And:

> A cache read **refreshes the entry's timer at no additional cost**, on either
> TTL. […] Requests that share a prefix and start less than 5 minutes apart keep
> the 5-minute cache warm **indefinitely** — the 1-hour TTL buys nothing there
> except the doubled write price.

A fan-out is continuous traffic by construction: ten census agents looping tool
calls each issue requests sharing that prefix, and every read pushes the 5m timer
forward. For the "later phases start cold" premise to hold, the run would need a
>5-minute window in which *no* request sharing the prefix started. During an
active census that window essentially cannot open.

So the honest table, for the three premises that might actually describe a run:

| what the run does | 5m | 1h | 1h is |
|---|---|---|---|
| continuous traffic, prefix stays warm (the documented default) | 2.35P | 3.1P | **~18,200 worse** |
| concurrent launch, cache shared within a phase (10 census + 2 later) | 85,032 | 75,314 | ~9,700 better |
| concurrent launch races, all in a phase write | 364,425 | 490,759 | **~126,300 worse** |

The first row is the PR's own "penalty if all twelve *did* fit inside five
minutes: ~18,200" — computed correctly, and filed as the unlikely branch when the
documentation says it is the ordinary one. Break-even confirms the direction: 5m
pays off at two requests, **1h needs three**.

**`subagentPromptCacheTtl` is therefore not set.** Reverting costs nothing and
the default is the cheaper arm under the premise most likely to hold.

**What is still unmeasured, and what would settle it.** This is a Claude Code
harness setting, not an API feature; whether Claude Code's subagents share a
cache namespace with their parent, and whether the harness's "automatic" TTL
behaves as the API's explicit `ttl` does, is not established by anything read
here. One instrumented `parallel()` run reading `cache_creation_input_tokens`
and `cache_read_input_tokens` off the agents answers it. Worth the ~100k that
costs *only* if someone wants the lever back — the revert is free and the
default is defensible without it.

**The transferable lesson is not about caches.** Three of this report's numbers
were arithmetically correct and answered a different question — this one, the
`bytes/4.0` calibration, and the 1.00x duplication factor. None was catchable by
re-checking the sum. All three fall to the control this repo already demands of
instruments and had not been applying to its own figures: **ask what the number
would read if nothing were wrong.**

## 7. The item this report recommends and does not do

`Reports/open-bugs-handoff.md` is ~107,743 tokens, and most of it is not open
bugs:

- **38 entries whose own headings say FIXED / CLOSED / RETIRED — 55%, ~59,126 tokens**
- `## Landing notes`, which are not bugs at all — 7%, ~7,900 tokens
- genuinely open: 58 entries, ~46,328 tokens

A register carrying only open bugs is **~40,716 tokens instead of 107,743** — a
**~67,000-token** cut, the same surgery `PLAN-log.md` already got when it was
split out of `PLAN.md`. `scripts/bugindex.py` already derives status from the
headings, so the classification exists.

**Why it was not done here.** The file is append-only, co-owned by every lane,
and has no `union` merge in `.gitattributes`; `agent-documentation-audit-2026-08-24.md`
§4 records the same reasoning for declining to reorder it. It wants one commit
landed fast, at a moment when no other lane is mid-edit — not a change made in
passing.

## 8. The materiality floor, and why it is written down

`MATERIALITY_FLOOR = 10,000` tokens per affected run, in
`scripts/contextbudget.py` — roughly 10% of one agent run and 1% of a fan-out.
Below it: fix it if it is free, do not report it as a finding, do not spend a
session on it.

It exists because its absence was expensive in the session that produced this
report: a prompt-prefix repair worth ~2,940 tokens per run was surfaced as an
actionable finding while a ~67,000-token repair sat unmeasured in the same tree,
and the ~289,000-token one in §6 was actively argued against on a wrong premise.
Ranking by size is not a refinement here; it is the whole method.

**One interaction to know about** (`pr89-review.md` §11). The lane-note item in
§5 is worth ~10,717 tokens — the only item within 8% of the floor — and the 91%
figure behind it keys on whether a heading contains `→`, so it is an upper
bound. If half those sections were addressed-without-marker the saving falls
below the floor. The floor is defensible and predates the item; the two were
simply never cross-checked. The cap still justifies itself on absolute size
(47,168 B in a file read at every session start), which is the argument to lean
on.
