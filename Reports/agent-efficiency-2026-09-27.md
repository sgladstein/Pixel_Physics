# What a sub-agent spends, and on what

*2026-09-27. The combined findings of the foraging-loop and nest lanes, each
measured over its own session's agents, 37 sub-agent transcripts in all. The nest lane's
full record is [`agent-efficiency-nest-2026-09-27.md`](agent-efficiency-nest-2026-09-27.md)
and is cited here as **[nest §n]**. The recommendations in §6 are built into
`agent-strategy.md` §4, the `lab-coordinator` skill, `instruments.md` and
`CLAUDE.md`'s command list. Instrument: `scripts/agentmeter.py`.*

## 0. The answer

**The agents were accurate and expensive, and most of the expense was set
before the first question was read.**

- **A default sub-agent starts at ~78k tokens; an Explore agent at ~28-30k.**
  The default carries every tool's schema (~37k, of which a data agent uses
  ~4k), `CLAUDE.md` (~26k), the skill listing (~4k) and the deferred-tool list
  (~1.5k). The task prompt is under 1k. Every call re-reads the start, so it is
  **35-54% of a default agent's cost** (foraging lane) and 41-52% for the nest
  lane's readers and skeptics [nest §3.1]. `agent-strategy.md` §1 priced the
  prefix at ~22k from `contextbudget.py`, which counts the always-loaded files
  and not the tool schemas.
- **Cost is calls × context, and both grow.** Agents took 20-94 calls, almost
  all single-command, while their context grew to 140-375k. Thinking stays in
  context for the whole run and is **not recorded** in the transcript
  (signatures only), so no meter can count it.
- **Agents paid again for what their spawner already knew.** They re-read the
  same files before any data work (all 5 of one survey read `creature.rs`), and
  dug up the owner's rulings and data paths their prompt had left out
  [nest §3.5].
- **The main thread is the biggest bill in both sessions.** The nest lane's
  re-read 477M tokens against 220M for its 19 agents [nest §5]; the foraging
  lane's ran at a median 402k tokens of context per turn.
- **Checking paid.** The nest lane's skeptics checked 102 claims: 3 refuted,
  40 qualified, 60 missed facts added [nest §4]. The foraging lane's checks
  overturned a harm finding in the lab box and found that its 90-cell forage
  results came from an intermediate build.
- **Two failures wasted whole agents.** A session limit (HTTP 429) killed 6
  running agents at once, one of them 1.8M tokens in; only what they had
  written to files survived. And 8 agents were set to trace data that turned
  out to come from a stale binary.

## 1. The instruments

- **`scripts/agentmeter.py`** (foraging lane) reads the transcripts Claude Code
  writes per sub-agent under `~/.claude/projects/<cwd>/<session>/subagents/`.
  Per agent it gives calls, starting and peak context, cost in input-token
  equivalents (uncached input 1×, cache write 1.25× at 5 minutes or 2× at an
  hour, cache read 0.1×, output 5×), the share of cost that is the fixed
  start, tool calls split into reading the repo and data work, and the largest
  single tool result. Per run it lists the files two or more agents read.
  `--main` meters the session's own conversation. Output tokens are estimated
  from content, because the transcript records only the value at the start of
  each stream. The nest lane found and fixed-by-report its one bug: a copy run
  from outside a checkout took `/` for the repo (now `--repo`, or the git top
  level of the working directory).
- **The nest lane's analysis** [nest §1, §7] adds what the meter leaves out:
  what the verify phase caught, priced against its cost, and the comment share
  of the code being read (61% of `creature.rs`'s characters). Both are read
  from a particular run's results or source, not from a transcript, so they
  stay analyses rather than meter columns.

## 2. Where the tokens went

| run | agents | per agent | fixed start as a share | reading the repo / data calls |
|---|---:|---:|---:|---:|
| foraging: survey (reading is the job) | 5 | 1.21M | 35-54% | 238 / 19 |
| foraging: checks, given exact data paths and parsers | 3 | **0.53M** | 51-54% | 23 / 59 |
| foraging: tracers, given the question only | 2 | 1.9M | 34-36% | 41 / 111 |
| nest: workflow readers, skeptics, designers | 14 | 1.5M | 41-81% | [nest §2] |
| nest: direct spawns (Explore, general-purpose) | 5 | 1.35M | 11-43% | [nest §2] |

The starting context, from the first call's `usage` and the attachments
recorded before it:

| part | default agent | Explore |
|---|---:|---:|
| tool schemas | ~37k (148k chars) | a subset |
| — `Artifact` | ~13.5k | no |
| — Claude Code Remote MCP, 24 tools | ~14k | yes |
| — Bash, Read, Write, Edit, Grep, Glob | ~4k | read-only subset |
| `CLAUDE.md` | ~26k | **no** |
| skill listing | ~4k | yes |
| deferred-tool list | ~1.5k | yes |
| **total at the first call** | **~78k** | **~28-30k** |

A third form, measured the same evening: an agent type defined in
`~/.claude/agents/data-analyst.md` with `tools: Bash, Read, Write, Edit,
Grep, Glob` starts at **~41k**. It keeps `CLAUDE.md` (26k) and the tools a
data agent writes files with, and drops ~34k of schemas it never uses; the
skill and deferred-tool listings go too. Defined per user it is only in the
container that made it; in the repo's `.claude/agents/` it would reach every
session. A new definition is picked up by a running session only after the
agent registry reloads, not at once.

Siblings in one workflow phase share a ~34k cached prefix; each phase's first
agent reads none of it, and nothing after that prefix is shared [nest §8].

## 3. Context grows, and the largest part is not visible

Agents ended at 140-375k tokens. What the transcript shows is the tool
results (15-200k characters per agent, the largest single ones being
400-line reads of `creature.rs` at 25-33k characters and whole reports at
35-46k) and the agent's own tool inputs. What it does not show is thinking:
one tracer had 58 thinking blocks, 0 characters of text and 237k characters
of signature. Thinking is set by the effort the agent inherits, which a
workflow can set per stage. A late call re-reads everything before it, so the
last third of a long agent is its most expensive.

Hand-offs are the same effect between phases: the nest workflow's designers
were given 222k characters of raw reader and skeptic output and started at
160k tokens [nest §3.4].

## 4. Paying twice for what the spawner knew

- **Shared reads.** `agentmeter.py` lists them per run. The foraging survey:
  `creature.rs` by all 5 agents (319k characters), `dead-ends.md` and
  `world.rs` by 4, the lane note by 5. Two tracers: `creature.rs`, `antloop.py`,
  the report's §22, `how-the-ant-works.md` and `antidle.py`, ~30k tokens each,
  before their first data command.
- **Missing context.** Two nest agents spent ~20 calls fetching the owner's
  rulings from the session transcript and the review queue [nest §3.5].
- **What worked.** The three foraging checks whose prompts named the data
  files, their formats and the parser to import cost 0.53M each, with 2-15
  reading calls.

## 5. The brief test: inconclusive, and what it showed

A verified ~2k-token brief (what is already parsed and where, the functions to
import, line anchors, the traps already found, what earlier work established)
was given to the restarted tracers and not to the first two. The test was cut
short twice, by the session limit and then by the stale-data finding, so the
brief arm has two partial agents (17 and 35 calls). They cost less per call
and read less in total (29k and 80k characters, against 101k and 145k), but
the late-survival tracer still spent **9 of its first 12 calls reading code**,
against 10 of 12 without the brief. **Line anchors invite reading**: an agent
told where the budding code is goes and reads it. The nest lane's additions
answer that: a list of the paths the agent may read, a call budget, and
reading the code before the comments.

## 6. Recommendations

Each is marked **measured**, **estimated** or **untested**, and names where it
now lives.

1. **Choose the agent type by the job** (measured). A stage that only reads:
   `agentType: 'Explore'` in a workflow, or `subagent_type: Explore`, at
   ~28-30k against ~78k. It does not load `CLAUDE.md`, so name any rule the
   stage needs in its prompt. It has no Write or Edit tool. A stage that must
   write files or checkpoint uses a tool-limited type (measured: ~41k to
   start, keeping `CLAUDE.md`), or the default type until one is in the repo's
   `.claude/agents/`, which is the owner's call.
2. **Brief every agent with what the session already holds** (measured
   against its absence; the controlled test is §5):
   - the verified facts, definitions and traps already found;
   - parsed data and the functions to import, with paths;
   - the owner's rulings the question depends on;
   - line anchors for the code already located, **and the list of paths it may
     read**;
   - a call budget (~30), with independent reads grouped into one call;
   - "read the code first (`grep -v '^\s*//'` on a range), then only the
     comments the question needs".
3. **Hand digests between phases, not raw results** (estimated: more than
   half of a designer's 160k start).
4. **Keep tool output small, and checkpoint** (measured, by loss): print
   summaries and write tables to files; append each established result to a
   `FINDINGS.md` in the agent's scratch directory. A killed agent leaves only
   its files.
5. **Prefer short agents** (measured): a late call re-reads everything, and
   resuming a long agent costs its whole context per call (a resumed tracer's
   last 6 calls cost 248k tokens at 236k context).
6. **Set model and effort per stage** (untested for quality): readers whose
   every claim is re-checked are cheap and loud to get wrong, which is
   `CLAUDE.md`'s test for stepping down a tier.
7. **In the main thread** (measured as the largest bill): delegate bulk
   reading, keep large output out of context, compact between phases.
8. **Verify inputs before fanning out** (measured, by loss): check that the
   data the agents will read came from the code under test. A harness log
   should name the build that wrote it; `trailfollow`'s does not yet
   (proposed: `git describe --dirty` at build time).
9. **Expect CPUs − 2 agents at a time**: 2 on these 4-CPU containers.
10. **Measure after every fan-out** with `python3 scripts/agentmeter.py`. More
    reading calls than data calls, or one file read by several agents, means
    the brief was too thin.

## 7. Where it is built in

- `Reports/agent-strategy.md` §4: the brief checklist, and a correction to §1's
  prefix figure.
- `.claude/skills/lab-coordinator/SKILL.md`: a "before you spawn" block that
  every spawner loads.
- `Reports/instruments.md`: `agentmeter.py`'s row.
- `CLAUDE.md`: one command line, inside the `contextbudget.py` gate.
