# What a sub-agent spends, and on what

*2026-09-27. DRAFT, in flight. Measured by the foraging-loop lane over its own
sub-agents; the nest lane's findings are being combined into §6 when they land,
then the recommendations go into `agent-strategy.md` §4, the `lab-coordinator`
skill and `instruments.md`. Instrument: `scripts/agentmeter.py` (new), which
reads the transcripts Claude Code already writes per sub-agent.*

**What this changes.** `agent-strategy.md` §1 priced an agent run as 26%
auto-loaded prefix and 74% reading, from `contextbudget.py`'s estimate of the
always-loaded files. Read off the transcripts, a workflow sub-agent's fixed
prefix is **~78k tokens, not ~22k**, because it also carries every tool's
schema, and that prefix is **35-54% of what the agent costs**. A brief is therefore competing with
three things, not one: the prefix, the reading each agent repeats, and the
agent's own reasoning, which stays in its context for the whole run.

## 1. The sample

13 agents in 4 runs of one session (`1289a7cd…`), 2026-09-27, all on the ant
foraging work: a 5-agent survey (`wf_213828e7`), a 3-agent verification
(`wf_445acba6`), 2 tracers of a 10-agent workflow (`wf_10e1b181`, still
running), and 1 background agent. Costs are **input-token equivalents** at
list-price ratios (uncached input 1×, cache write 1.25× at the 5-minute TTL,
cache read 0.1×, output 5×), not dollars.

| run | agents | per agent | fixed prefix share | tool calls reading the repo / on data |
|---|---:|---:|---:|---:|
| survey (reading is the job) | 5 | 1.21M | 35-54% | 238 / 19 |
| verification, given exact data paths and formats | 3 | **0.53M** | 51-54% | 23 / 59 |
| tracers, given the question only | 2 | 1.75M (running) | 35-37% | 40 / 103 |
| background tracer | 1 | 1.54M (running) | 42% | 45 / 29 |

## 2. The fixed prefix: ~78k tokens before the task is read

From the first call's `usage` and the attachments recorded before it:

| part | size | who needs it |
|---|---:|---|
| tool schemas | ~37k tokens (148k chars) | a data agent uses ~4k of it |
| — `Artifact` | ~13.5k | nobody in a workflow |
| — Claude Code Remote MCP, 24 tools | ~14k | nobody in a workflow |
| — Bash, Read, Write, Edit, Grep, Glob | ~4k | everyone |
| `CLAUDE.md` | ~26k (104k chars) | the method rules; most of it is other subsystems' |
| skill listing | ~4k | rarely |
| deferred-tool list | ~1.5k | rarely |
| the task prompt | 0.8-1k | the only part written for this job |

The prefix is cached and re-read at 0.1× on every call, so it is cheap per call
and large in total: an agent of 50 calls re-reads it 50 times.

## 3. Context grows, and what grows it is mostly not visible

Agents ended at 140-290k tokens of context. Three sources, of which the
transcript shows only two:

- **Tool results**: 15-200k characters per agent.
- **The agent's own tool inputs** (scripts written inline in `Bash`), a few
  thousand tokens.
- **Its thinking**, which is kept in context for the whole tool loop and is
  **not recorded**: the transcript holds signatures only (one tracer: 58
  thinking blocks, 0 characters of text, 237k characters of signature). This
  is the likely bulk of the ~45-150k tokens of growth the other two do not
  explain. It is set by the reasoning effort the agent inherits, which a
  workflow can lower per stage (`effort:`); untested here.

A late call re-reads the whole accumulated context, so the last third of a
long agent costs far more per call than the first.

## 4. The same reading, done again by every agent

`agentmeter.py` lists the repo files read by two or more agents of one run:

- **Survey (5 agents):** `creature.rs` by all 5 (319k chars), `trailfollow.rs`
  by 3, `dead-ends.md` by 4, `world.rs` by 4, the lane note by 5.
- **Tracers (2 agents):** each read `creature.rs` (73k), `antloop.py` (55k),
  the report's §22 (38k), `how-the-ant-works.md` (38k) and `antidle.py` (22k)
  before its first data command. All of it was already known to the session
  that spawned them.

The cheapest agents were the three whose prompt gave exact data paths, formats
and the parser to use: 2-15 reading calls each, 0.53M per agent.

## 5. Recommendations, each marked measured or not

1. **Give every agent a verified brief, not just a question** (measured in the
   comparison of §1; the controlled test is running). It holds what is already
   parsed and where, the exact functions to import, line pointers to the code
   in question, the definitions and traps already found, and what earlier work
   established. ~1.7k tokens replaced ~30k of repeated reading per agent. The
   running workflow gives it to 8 agents and not to 2; §7 will carry the result.
2. **Hand over parsed data, not raw data** (measured: the 0.53M run).
3. **A data-analysis agent type with only the tools it uses** (untested). A
   `.claude/agents/*.md` definition limited to Bash, Read, Write, Edit, Grep and
   Glob would drop ~30k tokens from every call. It needs the owner's sign-off,
   since `.claude/` is shared configuration.
4. **Lower the reasoning effort on mechanical stages** (untested): thinking is
   kept in context for the whole run.
5. **Keep tool output small**: print summaries and write tables to files. A
   20k-character table printed once is re-read on every later call.
6. **Expect two agents at a time**: workflows run CPUs − 2 in parallel, which
   is 2 on these 4-CPU containers, so a 10-agent pipeline takes five rounds.
7. **Measure after every fan-out**: `python3 scripts/agentmeter.py`. A run
   whose agents spend more calls reading the repo than analysing data, or that
   shows the same file read by several agents, had a brief that was too thin.

## 6. The nest lane's findings

*Pending: being combined when the nest lane's file lands.*

## 7. The brief test

*Pending: `wf_10e1b181`, 2 tracers without the brief and 8 agents with it.*
