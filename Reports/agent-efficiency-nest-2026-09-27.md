# Agent efficiency, nest lane: what 19 sub-agents cost, and why

*Measurement, 2026-09-27. `engine` (process, every game). An input to the
combined report the foraging lane is writing,
`agent-efficiency-2026-09-27.md`. Source: the nest session's own
transcripts. That is Claude Code session
`02cfa3f7-cc46-597e-8f90-5488d21d3601` (cloud session
`session_01WF4wABj2ewSmzWVJTk6DsC`), files
`~/.claude/projects/-home-user-Pixel-Physics/<session>/subagents/**/agent-*.jsonl`
and the main thread's `<session>.jsonl`, on the container that ran them.*

## 0. The answer

**The agents got the right information and were run expensively.**

- **Accuracy was good.** The workflow's skeptics confirmed 58 of the readers'
  102 checked claims and qualified 40; only 3 were wrong.
- **Cost came from the setup, not the questions.** Four choices set it:
  - the heaviest agent type;
  - the session's top model at its maximum effort;
  - one tool call per turn, against a context that grew to 230,000–375,000
    tokens;
  - raw hand-offs between workflow phases.
- **The same work could likely be done for about a third of the tokens the
  agents re-read, with the same checking.** That is an estimate: §6 builds
  it from the parts measured here.
- **The main thread cost twice what the agents did** (§5).

| | re-read from cache | written to cache | share of the session |
|---|---:|---:|---:|
| 19 sub-agents | 219.8 M | 4.41 M | about a third |
| main thread | 477.1 M | 4.59 M | about two thirds |

Only the input side is compared, because output and thinking are not
recorded reliably in these transcripts:
- Output is recorded as its value at the start of each streamed message.
  One skeptic logs 706 output tokens across 64 thinking blocks.
- Thinking text is not kept at all, only its signatures (183,364 characters
  of them for that agent, and zero characters of text).

Cache re-reads dominate the volume in any case. The foraging lane's
`agentmeter.py` estimates output from content (§8).

## 1. Method

**One API message can span several transcript lines** (one per content
block), and every line repeats the same usage. So everything is
de-duplicated by message id.

What is read from each agent:

- **Starting context** is the first message's input plus cache write plus
  cache read. It is what the agent pays before it has done anything, and it
  is paid again on every turn.
- **Peak context** is the largest per-turn total.
- **Re-read** and **written** are the summed cache reads and cache writes.
- **Tool results** are sized in characters and joined back to the call that
  produced them, so the largest ones can be named.

The workflow's own result JSON (`workflows/wf_61ef24dd-7e1.json`) gives what
its verify phase found. Its journal gives each agent's label.

## 2. Per agent

Five were spawned directly with the Agent tool. Fourteen ran in one
workflow, `nest-digging-understanding`: six readers, six skeptics and two
designers.

| agent | turns | tool calls | starting context | peak context | re-read (M) | written (k) |
|---|---:|---:|---:|---:|---:|---:|
| Explore: dig and spoil code path survey | 68 | 128 | 27,514 | 315,257 | 14.4 | 315 |
| Explore: nest/dig harness examples survey | 78 | 126 | 27,466 | 374,743 | 20.0 | 375 |
| general-purpose: independent review of PR #493 | 48 | 52 | 74,753 | 239,457 | 8.2 | 239 |
| Explore: dead-ends grep by mechanism | 42 | 44 | 27,365 | 213,687 | 5.3 | 214 |
| Explore: prior art for a home that follows the ground | 28 | 40 | 27,708 | 105,665 | 1.9 | 106 |
| read:dig-chain | 66 | 68 | 78,455 | 250,637 | 12.2 | 251 |
| read:spoil | 79 | 84 | 78,371 | 265,581 | 14.8 | 231 |
| read:goals | 81 | 85 | 78,427 | 264,622 | 15.3 | 230 |
| read:instruments | 63 | 68 | 78,441 | 232,588 | 10.6 | 198 |
| read:biology | 50 | 54 | 78,478 | 252,945 | 9.1 | 219 |
| read:per-ant-data | 64 | 68 | 78,506 | 248,273 | 11.0 | 214 |
| verify:dig-chain | 94 | 97 | 85,802 | 243,239 | 16.2 | 209 |
| verify:spoil | 94 | 97 | 85,976 | 257,392 | 16.1 | 257 |
| verify:goals | 90 | 93 | 86,282 | 269,852 | 15.9 | 236 |
| verify:instruments | 59 | 61 | 85,617 | 245,413 | 10.6 | 211 |
| verify:biology | 80 | 83 | 88,622 | 253,155 | 13.8 | 219 |
| verify:per-ant-data | 81 | 83 | 86,494 | 236,713 | 13.3 | 203 |
| design:nest-funnel | 37 | 38 | 160,210 | 283,559 | 8.2 | 284 |
| design:nest-vs-random | 14 | 16 | 160,332 | 228,918 | 2.6 | 195 |

All 19 ran on the session's model (`claude-opus-5-5`), read from each
message's `model` field. None set `model` or `effort`, so they inherited
the session's.

## 3. What drove the cost, largest first

1. **Agent type sets the starting context.**
   - **Explore agents start at ~27,500 tokens.** Explore and Plan agents
     leave out CLAUDE.md, whose always-loaded part is ~25,300 tokens by
     `scripts/contextbudget.py`, and most of the tool set.
   - **The default workflow agent starts at ~78,400 tokens.** About 34,000 of
     that is a cached prefix shared with its siblings. The general-purpose
     agent is the same shape, at 74,800.
   - **Every turn re-reads that start.** For the 12 workflow readers and
     skeptics, ~50,000 extra tokens over ~900 turns is about **45 M of the
     workflow's 170 M re-read tokens**.
   - **Measured as a share of everything each agent read,** the starting
     context was:
     - 41–46% for the readers (median 42%);
     - 47–52% for the skeptics (median 49%);
     - 70–81% for the designers;
     - 43% for the general-purpose reviewer;
     - 11–38% for the Explore agents, whose cost was their reading.

     That matches the foraging lane's 41–71% for default agents, and
     `agent-strategy.md`'s earlier split of 26% prefix to 74% reading.
2. **Turns times context.**
   - Workflow agents took 50–94 turns, and almost every turn was a single
     Bash command.
   - **The skeptics took the most turns (59–94).** They opened each claim's
     citation one at a time.
   - The two broad Explore surveys were the costliest single agents: 68 and
     78 turns, peaking at 315,000 and 375,000 tokens.
   - Grouping independent reads into one call, with a budget of ~30 calls,
     would roughly halve what remains.
3. **Reads larger than the question.**
   - The largest single results were 400–470-line reads of `creature.rs`
     (25,000–33,000 characters each) and whole reports (35,000–46,000
     characters). Each stays in the context for every later turn.
   - `creature.rs` was opened 67 times across the agents.
   - In the region read most, `act`'s spoil and dig branches (lines
     10,900–11,330), **68% of the lines are comments**, and the code alone is
     6,711 of 26,196 characters. Over the whole file, comments are 61% of
     the characters.
   - Those comments are load-bearing here (CLAUDE.md), so the lever is not
     stripping them. It is **reading the code first to locate, then the
     comments the question needs**.
4. **Hand-offs between phases.**
   - The two designers were handed the whole map as raw JSON: 221,968
     characters of reader and skeptic output. They started at 160,000
     tokens.
   - A digest of the corrected claims and the missed facts would have cut
     that by more than half. This is the same finding as the foraging
     lane's verified shared brief.
5. **Context the agents needed and did not get.**
   - The two agents on the "goals" question dug the owner's rulings out of
     this session's transcript and the review queue, over about 20 calls:
     the session brief, "too many ants", and card verdicts.
   - The rulings were not in their prompt, although the session that spawned
     them held every one of them.
   - Put in the brief, they would have cost nothing to find. This is a
     missing-context finding, not drift.
   - A third agent read a data file the session had produced, which a path
     in its prompt would have covered.
6. **Model and effort.**
   - Every agent ran on the top tier at maximum effort, inherited from the
     session. Under ultracode that is by design ("token cost is not a
     constraint").
   - A reader whose every claim is re-checked by a skeptic is **cheap and
     loud** to get wrong. That is CLAUDE.md's own test for stepping down a
     tier.

## 4. What the checking bought

This is the part to keep. From the workflow's result JSON, across six
readers:

- 102 checks: 58 confirmed, 40 partly right, 3 refuted, 1 unverifiable;
- 60 facts the readers missed;
- the four stale statements that PR #497 corrected.

The refutations were real. One was a number attributed to the wrong regime:
1,073 hanging cells came from a 3,182-ant soil study, not the 300-ant
`digbox` arch. Another was a claim that a null model was blocked by a
private function, when a public route already existed.

**So the verify phase is worth its price.** The waste was in how it read,
not in whether it ran.

## 5. The main thread

The main thread is the larger bill:

- 1,153 turns;
- context per turn: median ~400,000 tokens, 90th percentile ~696,000,
  maximum 783,500;
- 477 M tokens re-read.

**The agents did not cause it.** Their results came back as ~1,100-character
notices, and the reports were read in slices.

**What filled it was the thread's own work:** 888 Bash calls returning 1.32 M
characters, 84 file reads and long answers. Many of the largest items were
images, which are billed by pixel size, not by their base64 length, so they
are not the problem they look like in a character count.

Levers:

- hand read-heavy work to agents, which absorb a big read at a small
  context and return a summary;
- keep large command output out of context (`head`, `grep`, write to a file
  and read slices);
- compact at the end of a phase, not only when forced.

## 6. Recommendations, for the combined report

1. **Read-only stages use `agentType: 'Explore'`** (workflow `agent()`), or
   `subagent_type: Explore` (the Agent tool). That is a starting context of
   27,500 against 78,000. Name any CLAUDE.md rule the stage needs in its
   prompt, since Explore does not load it.
2. **Every prompt carries five things:**
   - line anchors for the code the session has already located;
   - the owner's rulings and the data paths the question depends on, which
     the spawning session already holds;
   - a call budget (~30), with independent reads grouped into one call;
   - "read the code first (`grep -v '^\s*//'` on a range), then only the
     comments the question needs";
   - a list of the paths it may read.
3. **Skeptics get the cited line ranges** and open them in one batch, rather
   than re-finding each claim.
4. **Phases hand each other digests, not raw results.** That is the verified
   shared brief.
5. **Set model and effort per stage.**
   - Readers whose claims are all re-checked: a cheaper tier at `effort:
     'medium'`.
   - Skeptics and designers: the session's model.
6. **In the main thread:** delegate bulk reading, keep big outputs out of
   context, and compact between phases.

**Estimated effect on this workflow.** The agent type alone removes ~27% of
the re-read tokens (§3.1). Grouped calls and smaller reads plausibly halve
what remains. Together that is about a third of the tokens re-read, with
the same verify step. The step-down in model tier lowers the price per
token on top of that; this report does not measure it.

## 7. What this measurement has that `agentmeter` (as proposed) does not

- **Peak context**, and the largest single tool results joined to the calls
  that produced them. That is what grew each context.
- **The main thread's share** and its per-turn context distribution. Here it
  was the larger bill.
- **The hand-off size between workflow phases,** read as the next phase's
  starting context.
- **What the verify phase found** (confirmed, partly, refuted, missed), read
  from the workflow's result JSON, so verification can be priced against
  what it caught.
- **The comment share of the code being read,** a property of this repo
  that decides how much a code read costs.

## 8. For the combined report: `agentmeter` on these transcripts, and the foraging lane's four questions

**`agentmeter.py` at `ac0c885a`, run on this container** (`--all`, from a
checkout):

| run | agents | cost, input-token equivalents | per agent | fixed start, share of cost | tool calls |
|---|---:|---:|---:|---:|---|
| the workflow | 14 | 21.6 M | 1.54 M | 36–69% | read the repo 860, data 120, write 0 |
| the five direct spawns | 5 | 6.75 M | 1.35 M | 10–39% | read the repo 385, data 0 |

- The two instruments agree on the total: 28.4 M against §0's ~28–31 M.
- **In the workflow, all 14 agents read `creature.rs` (1.13 M characters
  between them) and `digbox.rs` (0.45 M).** 11–13 read `how-the-ant-works.md`,
  `dead-ends.md`, `ant.ron`, `world.rs` and the nest-mouth report. That list
  is the shared brief the run should have been handed.
- **Its selftest fails when the script runs from outside a checkout.** It
  takes both the repo root and the transcript directory from its own path.
  From `scripts/` it passes.

**Other agent types.**
- The built-in Explore type is the measured form of the foraging lane's
  proposed tool-limited agent type: 27,500 tokens to start, against 78,400,
  because it drops CLAUDE.md and most tool schemas.
- It cannot write files, so it does not fit a data agent that has to
  checkpoint. That case still needs the proposed `.claude/agents/` type,
  which is the owner's to approve.

**Cost per useful finding.** In input-token equivalents:

| phase | cost | what it produced |
|---|---:|---|
| six readers | 9.31 M | 87 claims |
| six skeptics | 10.54 M | 103 corrections: 3 claims refuted, 40 qualified, 60 missed facts added |
| two designers | 1.76 M | the two designs that became PR #498's instruments |

So a correction cost about 102,000 tokens, and a refutation about 3.5 M.
Checking cost 13% more than reading. It still paid: without it, three wrong
claims would have reached the designs.

**Prompt caching across agents.** Sibling agents shared a cached prefix
(system prompt plus tool schemas) only within a phase. The most likely
reason is that each phase's output schema sits in its tool list: the three
shared prefixes differ by a few hundred tokens, as the schemas do. That is
inferred, not observed:

| phase | shared prefix (tokens) | first agent's write | each later sibling's write |
|---|---:|---:|---:|
| readers | 34,285 | 78,453 (read 0) | 44,000–44,200 |
| skeptics | 34,059 | 85,974 (read 0) | 51,500–54,600 |
| designers | 34,161 | 160,208 (read 0) | 126,169 |

- **CLAUDE.md, the skill listing and the task prompt come after that shared
  block**, so every agent writes them fresh, at the cache-write price.
- **The five direct spawns shared nothing** (all read 0 at start).
- **Within one agent, caching works:** fresh input is about zero from the
  second turn on.
- `agent-strategy.md` adds that sub-agents get a 5-minute cache against the
  main session's hour, so an agent idle for more than five minutes between
  turns pays the full write again.

**Cloud lanes.** This session used none, so there is no data here.

**Checkpointing.** The workflow runtime journals each agent's return value
(`journal.jsonl`), which makes a run resumable one agent at a time. Nothing
inside a running agent is saved. That agrees with the foraging lane's
finding that a rate-limit kill (HTTP 429) left only what agents had already
written to files: a long agent should write its findings as it goes.
