# Working in this repo

This file is for *how to work here*, not what the code does. The codebase is
already heavily documented and the architecture is written up at length
elsewhere — see below. What is not written down anywhere else, and what this
project keeps re-learning the expensive way, is the method.

## The ethos: it has to feel satisfying

**Stated by the owner as a core value, above correctness of any individual
mechanic: everything should feel satisfying.** A mechanic that is right on
paper and dull in the hand has failed, and "the test passes" is not a
defence.

This is not a restatement of "looks good in motion" below — that is about
*appearance*, and this is about *response*. **It applies to every line in
the engine, not to destruction**, even though destruction is where it was
learned and most of the evidence below comes from. If you are working on
plants, water, weather or creatures, the two laws are yours.

**1. An outcome is a distribution, not a binary.** Structural failure once
produced either a single coherent body or a uniform dissolve into powder,
with nothing between; real breakage is a few blocks, more cobbles, a lot of
grit, and its absence read as fake immediately. The same law, arrived at
separately on the plant line: a tree that cannot pay its maintenance is
marked senescent and carried out by `rot_remains` at the species half-life,
**so the death is graded rather than a disappearance** — the owner's own
ruling. Ask of any change: does this have a middle? A plant that is either
thriving or gone, a pool that is either full or empty, a fire that is either
out or total, has the same defect the rubble did.

**2. There must be a verb, and it must deliver something.** Destruction
could once only be triggered by *erasing* support, which carries no load and
no impulse, so nothing ever failed from being *hit* — the mechanic worked
and still felt inert, because the player had no way to strike anything. The
plant line hit this too, and closed it the same way: `Felling status` is
titled *the verb works, and what it produces is pieces*. If a system can
only be changed by the world changing around it, the player is a spectator
of it.

Practical consequences when weighing a change, in any subsystem: prefer the
version with more legible feedback even when it is less exact; a graded
outcome beats a binary one; and if an event produces no visible consequence
— no debris, no impulse, no sound, no mark left behind — it is not finished
regardless of what the simulation believes. Judge this by playing it, not by
reading the diff — the owner's playtest reports have overturned three
separate models that all looked correct in tests.

## What this project is optimising for

**Features default on unless there is a good reason not to** (owner,
2026-09-27). A switch that measures as a gain, or as neutral, ships on; a
good reason is a *measured* harm, stated in the switch's doc and the report.

**Looks good and realistic, in motion, at play scale — without ruining
performance.** Stated by the owner directly. Three consequences that have
already changed decisions:

- **Exactness is not a goal.** A mechanism whose measured advantage is
  numerical precision — an exactly flat surface rather than a nearly flat
  one — is not buying anything here, however well argued. Judge liquid work
  by how it looks while it is moving, not by its final residual.
- **The current 512x320 world is a test environment, not the target.** It
  will grow (M10 streaming). So a cost that is invisible today because the
  world is small is still worth taking seriously, and a mechanism whose
  advantage only appears at large width is not automatically useless — but
  it does have to actually *have* that advantage when measured, which is
  not something to take from a report on faith. See
  `Reports/open-bugs-handoff.md` §6 for a case where it did not.
- **Frame cost is a hard constraint, not a tiebreaker.** A visual
  improvement that costs the dirty-rect render skip, keeps chunks awake, or
  slows the sweep is not automatically worth it — say what it costs when
  proposing it. `examples/ascii.rs` reports worst-frame timings and CI runs
  it; that is the number to quote. The corollary cuts the other way too:
  because exactness is not wanted, *stopping work early* is a legitimate
  optimisation. A pool that is visually flat but still shuffling fill for
  another quarter of an hour is a real cost buying nothing.

## Where knowledge already lives — read it, don't re-derive it

| File | Holds |
|---|---|
| `README.md` | Architecture, and per-milestone status. **~72k tokens, the largest document here — do not read it whole**: its **By topic** table maps subsystem to owning sections with line numbers **and says which game each topic belongs to** (`engine` is shared and is most of them; `outdoor`, `lab` and `held` mark what the other two games never reach). Milestone sections are named for the *build*, not the subsystem — `M17 status` is the structural-collapse write-up |
| `wiki/*.md` | What a material or mechanic *does*, in plain language — no code, no file names. `Reports/*.md` is *why it's built that way*; this is *what it looks like when it's right*, which makes it **the written form of the bar your change is judged against**. ~34k tokens over 11 pages, so read the one page, not the directory. **Which page owns your file is not guessable for half of them**, and the map lives here because the wiki refuses file names by design: `field.rs`/`decay.rs`/`sky.rs` → `world-cycles.md`; `structural.rs`/`load.rs`/`rigid.rs` → `structural-collapse.md`; `explosion.rs`/`fracture_field.rs` → `explosions.md`; `plant.rs`/`organism.rs`/`assets/species` → `plants.md`; `creature.rs`/`brain.rs` → `ants.md`; `player.rs` → `the-gnome.md`; `worldgen/` → `the-world.md`; `lab/`, `bin/lab.rs` → no page yet, read `Reports/lanes/evolution-lab-coordinator.md`; `druid/`, `bin/druid.rs` → no wiki page yet, read `Reports/lanes/druid-program-coordinator.md` first (the standing note: owner rulings, live round, the environment facts that cost time), then `Reports/held-world-game-concept-2026-09-13.md` and README's `Held world status`; `update.rs`/`material.rs` → `powders.md` and `liquids-and-gases.md`; `liquid.rs` → `liquids-and-gases.md`; `fire.rs` → `fire-and-heat.md`; `weather.rs` → `weather.md` |
| `PLAN.md` | Roadmap, settled decisions, the issues backlog; the append-only progress log lives beside it in `PLAN-log.md`. **~60k tokens — do not read it whole**: start from its Contents, and in any session-handoff section read the dated *(State …)* line rather than the heading, which records only what was true when written |
| `Reports/README.md` | **The index of every design report**, with per-report status and an in-flight section for documents still on unmerged branches — check a report's standing there before trusting it or writing a new one. **Its sections are tagged by game**, so a lab session can skip `outdoor` and vice versa; `engine` is shared and is most of the index |
| `Reports/dead-ends.md` | **Tried-and-reverted approaches** (726 at last census, 2026-09-02), each with the condition its rejection depended on and where the full record lives. **~97k tokens — grep the *mechanism* you are about to touch or propose, never your subsystem.** Measured 2026-08-26: `thicken` returns ~2,460 tokens, `max_unsupported_span` ~650, `chunk seam` ~250, and `rot_remains` **zero** — a real answer, cheaply. Grepping an area instead costs ~12k–31k, more than this file. For a genuine survey, grep the address prefix (`^- \*\*.\?src/sim/plant`) rather than the prose: 99% of entries open with the file they apply to, which halves it |
| `Reports/open-bugs-handoff.md` | **Open bugs.** Working reproductions and what has been ruled out *by measurement*. **~97k tokens — do not read it whole**: its generated status index is the first table in the file, so read that, then only the sections it lists for your area. (`dead-ends.md` owns "was this tried?"; this owns "is this broken?") |
| `Reports/design-philosophy.md` | Settles arguments about constants, hardcoding, and scope boundaries |
| **`Reports/how-the-ant-works.md`** | **Working on ants, trails, or anything that walks? Read this first.** The living reference for how the shipped ant actually works: tick order, senses, brain wiring, the step and the tumble, trails, the crop, laden versus empty. Written from the source and **edited in place, never appended**, so unlike the dated ant reports it describes the ant as it is now. **Change a mechanism it describes → update it in the same commit** (owner, 2026-09-22) |
| **`Reports/lanes/evolution-lab-coordinator.md`** | **Working on the lab? Start here and nowhere else.** `cargo run --release --bin lab` is **the main game** (owner, 2026-10-02) — a sealed box of soil under grow lights where the shipped plants and ants live. **Kept small on purpose** (~10 KB, against a 93 KB high-water mark that every lab session paid before doing any work): it carries the standing owner rulings, the live round, and the environment notes that cost time here. The nineteen finished rounds — **what each *overturned*, which is the part a later session cannot reconstruct** — are in [`Reports/evolution-lab-rounds-archive.md`](Reports/evolution-lab-rounds-archive.md), priced per round and mapped to the three concurrent lines they braid; read the one round, not the file. **File ownership is re-derived from the open PR list, never from a table in a note.** The design of record is [`Reports/evolution-lab-design-guide-2026-08-30.md`](Reports/evolution-lab-design-guide-2026-08-30.md), with [`evolution-lab-feasibility-2026-08-30.md`](Reports/evolution-lab-feasibility-2026-08-30.md) under it — both on `main` since PR #158 |
| `Reports/session-programs.md` | **Coordinator ↔ lane protocol** — only if you are coordinating sessions or were spawned by one |
| `Reports/two-games-one-repo-2026-08-30.md` | **Why one repository holds two games, and what is scoped rather than shared.** There are **three** since 2026-09-13 — the held world (`--bin druid`) added a fourth tag, `held`, and the report's reasoning does not depend on the number two. Read it before proposing that anything be split, forked, or moved — it carries what the routing tables' `engine`/`outdoor`/`lab`/`held` tags mean, which rules can be `paths:`-scoped and which must stay always-loaded (§3), and which of five `docscheck` globs a `Reports/` reorganisation would silently break |
| `Reports/instruments.md` | **What every `examples/` binary can already answer** — grep it before building a measurement harness. Several generalise well past the question they were built for, which is not guessable from their names |
| `.claude/README.md` | **The Claude Code configuration** — the `SessionStart` check, the permission allow/deny/ask lists, and why `.claude/` is tracked rather than ignored |
| `.claude/skills/review/SKILL.md` | How to put an artifact in front of the owner and get a verdict back — the primary feedback channel, used constantly |

**Which rules apply to what you are doing right now.** The rule statements
below are always loaded. Rules that only bite in one part of the tree live in
`.claude/rules/*.md` with `paths:` frontmatter and **arrive on their own when
you read a matching file with the Read tool** (measured: `bash
scripts/contextprobe.sh --selftest`, which probes Read only — `cat`, `grep`
and a `cargo run` are not shown to trigger it, so when the bullet below names
a rules file and you have not Read a matching file, read the rules file). **The worked case and the numbers behind every rule** are in
[`Reports/claude-md-evidence-2026-09-30.md`](Reports/claude-md-evidence-2026-09-30.md),
under the same heading — read it when a rule seems not to apply, before
arguing with it.

- Measuring anything in the world (liquids, powders, destruction, plants,
  ants, frame cost) → Method below, then `.claude/rules/measuring-the-world.md`,
  which loads when you open `src/sim/**`, `src/worldgen/**` or `examples/**`.
  **If your measurement never opens one of those, read it by hand.**
- Running a parameter sweep → *When every setting of a sweep fails the same
  way* (Method), *A change that moves nothing* (Conventions), and the
  `include_str!` gotcha (`.claude/rules/assets.md`, loads with `assets/**`). If it censuses a collapse, the cascade rules in
  `measuring-the-world.md` — `seedsweep.sh`'s default frame budget stops
  mid-cascade.
- Writing or trusting a guard test → the guard bullets in Conventions and
  *A green suite does not prove a test ran* (Gotchas).
- Touching `src/sim/` → `.claude/rules/src-sim-cells.md` (the sweep and the
  cell) arrives with the first read; organism code also gets
  `plants-and-organisms.md`, ant code `ant.md`.
- Running the real app or taking a screenshot → `.claude/rules/running-the-app.md`
  (loads with `src/main.rs`, `src/app.rs`, `src/render.rs`, `src/bin/**`).
- Filing a bug → read `.claude/rules/bug-register.md` first: it loads only on
  a Read of `Reports/open-bugs-handoff.md`, and that file is grepped, not Read.
- Proposing, building or retrying any mechanism → `Reports/dead-ends.md`
  first.
- Needing a number nobody has measured → `Reports/instruments.md` before
  writing a harness; 26 already exist and the names do not say what they
  answer.
- Coordinating other sessions, or spawned by one → `Reports/session-programs.md`.
  The lanes and the coordinator **can** message each other, and the mechanism
  is not the obvious one.

**Source comments are load-bearing.** They record *why*, including approaches
that were tried and reverted and must not be retried. Do not strip them when
editing nearby code, and add to them in the same voice when you learn
something that cost effort to find.

## Commands

```
cargo test                                       # unit + integration -- the ONLY one that reaches tests/*.rs, where the preset and worldgen guards live; `--lib` cannot see them
cargo clippy --all-targets --release --locked -- -D warnings   # exactly what CI runs. `rust-toolchain.toml` pins 1.98 so this needs no `+1.98.0`
cargo run --release --example ascii              # headless behaviour + worst-frame timing; CI runs it
cargo run --release --example filmstrip -- scene=fall zoom=2 crop=0,140,256,110
python3 scripts/review.py serve --open      # the owner's review queue; see below
python3 scripts/review.py serve --lan       # ...also reachable from a phone on the same Wi-Fi
bash scripts/acceptance.sh                  # the structural acceptance cases; CI gates this
bash scripts/worldgencheck.sh               # is a generation pass eating another's output, or has one stopped firing; CI gates this. --selftest puts the defect back
bash scripts/seedsweep.sh                   # the order-statistic seed sweep; run BEFORE changing any model over procedural content
bash scripts/docscheck.sh                   # documentation checks: links, map-vs-tree, freshness notes, report index
python3 scripts/bugindex.py --branches      # WHICH BUG LETTER IS FREE -- swept over every fetched branch, not just this tree. Run it BEFORE filing in Reports/open-bugs-handoff.md; --check cannot see a letter claimed on an unlanded branch. --selftest is the positive control
python3 scripts/agentmeter.py               # what each sub-agent SPENT, and on what (reading vs data, files several agents re-read); run after any fan-out. Brief agents from Reports/agent-strategy.md s4
python3 scripts/contextbudget.py            # what every session, agent and subagent pays before it starts; --gate is the ceiling, --check is gated by docscheck
bash scripts/contextprobe.sh                 # ...and what the runtime ACTUALLY loads, over the InstructionsLoaded hook; contextbudget infers, this measures. --selftest is the positive control
bash scripts/branchcheck.sh                 # how far behind main this branch is, and which branches are merged-and-deletable; --gate is the CI trunk check
bash scripts/branchcheck.sh --brief         # ...summary only; this is what the SessionStart hook runs (`.claude/README.md`)
bash scripts/branchcheck.sh --prs           # ...and say which unlanded branches have NO OPEN PR -- i.e. which finished work is invisible
bash scripts/branchcheck.sh --prs-from F    # ...reading the PR listing from F, because the in-session credential gets 403 (use the MCP GitHub tools to write F)
bash scripts/branchcheck.sh --who-touched src/sim/foo.rs   # WHICH LIVE BRANCH IS IN THIS FILE, and what landed in it while you were not looking. Run it before handing a file to anyone
bash scripts/branchcheck.sh --selftest      # the ten sensitivity rows over the PR annotation and --who-touched; rows F and J mutate the file to prove rows C and I are not blind
```

**The real app can be screenshotted headlessly** (xvfb + lavapipe), and
`filmstrip` renders contact sheets or, with `gif=1 out=x.gif`, animations with
no window and no GPU — reach for the GIF when the question is whether
something *moves* right. The recipe, and the two traps that each cost twenty
minutes, are in `.claude/rules/running-the-app.md`.

**Never wait on a process with `pgrep -f`/`pkill -f`.** Its pattern matches the
waiting shell's own command line, so `while pgrep -f 'cargo test'; do sleep
20; done` never exits and `pkill -f` kills your own script. `pgrep -x <exe>`
matches the process name and cannot match the shell.

**Having rendered something, show it — don't describe it.** See *Getting the
owner's judgement* below; it is not an occasional tool.

## Getting the owner's judgement

**`scripts/review.py` is the primary way to get feedback from the owner, and it
is meant to be used constantly — not saved for big moments.** Everything this
project optimises for is judged by eye: whether a collapse *feels* like
destruction, whether a fall reads as sand, whether a pool looks flat while it is
still moving. None of that is a test result. Describing it in chat has
repeatedly failed — three separate models were overturned only by the owner's
playtest reports, and nearly every fix judged by test output alone left the
screen unchanged.

So when a change is visible, **post it rather than describe it**. "This looks
better" is precisely the claim the owner has to check, and a sentence is not
checkable.

Post when:

- a change alters anything on screen — including one you are confident about;
- you are about to claim something looks, moves or feels better;
- a complaint could mean two things — render both readings and ask which one it
  is, rather than spending the whole detour on the wrong one;
- a step is "judge by eye" — post *before* declaring it done, not after;
- you are choosing between approaches and the difference is visual: post a blind
  A/B (`review.py ab --blind`) instead of arguing it out. Blinding costs you
  nothing, because the stored verdict names the real option.

Posting is **fire-and-forget**: post, carry on, and collect the verdict with
`review.py inbox` later or in a later session — run it when you pick a thread
back up, including at the start of a new one. Do not stall a session waiting.
`--wait` exists for one case only: a wrong guess would waste the work you are
about to do. A `--wait` that times out changes nothing — the card stays queued
and answerable — so it is only ever your own time at risk.

Two house rules, both from failures already paid for here:

- **Put the discrete event count in the card's `meta`.** The page renders it
  directly under the image. A collapse once read as "chunks are working" from a
  picture whose body count was zero for the whole run; an image says *what* and
  *where*, and only the number says *whether it fired*.
- **Prefer a paired comparison** over one run against a remembered impression.
  Outcomes here have enormous spread, so a single run is a sample from a wide
  distribution.

The owner may be reading the queue on a phone (`serve --lan`), where a card is
one column and an image is judged in the full-screen viewer. Nothing about
posting changes; it is one more reason the card must stand on its own — a title,
a question, and the count in `meta`.

The queue is shared by every worktree of the clone, so a card posted from
`.claude/worktrees/foo` and one from the main checkout land in the same place,
and an answer outlives the session — and the worktree — that asked for it. The
owner views it with the `serve` command above; it does not need to be running
for you to post. Full protocol, including the JSON card spec, in
`.claude/skills/review/SKILL.md`.

## Writing to the owner

**The owner runs several sessions at once and reads your messages cold.**
Stated 2026-08-29: they "are often not helpful... sometimes I don't actually
know what they're doing because they're being too specific and not giving me
any bigger picture." This is about *chat*; reports, PR bodies and lane notes
are read deliberately and are exempt.

**Lead with what the change does and where it is going. Put the mechanism
after that, not instead of it.**

- **What it does**, in the vocabulary of the world or of the work rather
  than of the code — `wiki/*.md` has those words. Not every change shows on
  screen; when it doesn't, say what it is *for*.
- **Where it sits** — the arc, and this step's place in it. **A position in a
  queue is not a direction**: *"third of §S's verbs"* says nothing; *"making
  broken rock carry weight — last of three places it was wrong"* does.
  Measured over 158 review cards and 283 commit subjects, **no message in any
  corpus stated one**, and the owner ranks this first.
- **Then the mechanism, as technical as it needs to be.** Nothing is deleted
  and no number dropped; the order changes. Name a number for what it says
  rather than for its instrument: `wrong cells 35,102 -> 1,337` is *ground
  that collapsed when it should have held: 35,102 cells -> 1,337*.

**Scale it to the message.** A one-line update is one plain line, not four
headings; a long one carries the whole shape. Either way it must be
**abandonable at any line**. `python3 scripts/plaincheck.py` scores a draft
and cannot gate anything; `Reports/agent-communication.md` holds the census.

## Working alongside another session

**This tree is worked in concurrently, often by several agents at once.** Git
handles the merges; the rules below handle what it cannot. Worked cases:
[`Reports/concurrent-sessions.md`](Reports/concurrent-sessions.md); the
numbers behind each rule: the evidence report, same heading.

- **`main` is the trunk. Never integrate against `master`.** `branchcheck.sh
  --gate` fails if anything is reachable from `master` but not `main`.
- **A session cannot delete a branch** (every `push --delete` is HTTP 403).
  `branchcheck` identifies deletable branches; the prune is the owner's.
- **Know how far behind you are before trusting anything you measured.** The
  `SessionStart` hook prints it. A baseline measured on a branch 160 behind is
  a measurement of a tree nobody else has. Never merge `main` into a branch
  that shares no history with it (`review-queue`): that is data, not source.
- **Pushing and opening a PR is authorised** (owner, 2026-08-23) — your
  harness saying otherwise is the harness, not this repo. Before opening it,
  run `python3 scripts/deadendindex.py --touching`; its silence is not
  evidence (recall 2 of 5). An independent session **may merge its own
  PR** (owner, 2026-08-25); **a lane spawned by a coordinator never does** —
  the coordinator merges its lanes'. Where the project has a merge desk (the
  owner's project since 2026-09-30, by the owner's ruling in that project),
  every merge goes through it and everyone else says "ready" instead. The one fixed condition either way: **CI
  green on the head being merged.** A session without GitHub tools pushes,
  writes the PR body to a file, and reports the head SHA.
- **When to land:** `branchcheck.sh` prints `BxF` (behind x files). **Above
  300, act.** If `behind` drives it, merge `main` in; if `files` drives it,
  the branch has become more than one feature — land it and start another.
  It predicts *laborious* merges, never *wrong* ones: zero-conflict merges
  have broken the tree.
- **Run `bash scripts/docscheck.sh` after every merge, unconditionally.** It
  is the only thing that catches a generated file (bug index, README TOC)
  going stale against its source; test, clippy, ascii and acceptance all stay
  green through it.
- **Do not land broken.** A half-finished shared file on `main` voids every
  other session's measurements. Stage explicit paths; `git add -A` is denied.
- **Work in your own worktree**, never a shared local checkout: a shared
  `target/` makes another session's half-edit fail your build.
- **The contested files** — `Reports/open-bugs-handoff.md`, `README.md`,
  `Reports/README.md`, `src/sim/world.rs`, `examples/filmstrip.rs`,
  `Reports/dead-ends.md`, `src/render.rs`, this file. If you touch one, land
  quickly.
- **Before writing into a file another lane owns, or handing one off, run
  `bash scripts/branchcheck.sh --who-touched <path>`.** A roster you were
  handed is a claim about the past.
- **A shared append-only file must be read before it is appended to** — grep
  for what you are about to file. On a conflict there, ask which side is
  *newer*, not which is yours. The bug-letter procedure loads with the
  register (`.claude/rules/bug-register.md`).
- **To commit past somebody else's unfinished work in a contested file**, do
  not stage around it: re-apply your change in a worktree at `origin/main`,
  commit and push from there. Note which files are genuinely dirty *before*
  any `reset --mixed`; afterwards a stale file and an edited one look
  identical.

## Running a program of sessions — moved out

**Coordinating other sessions, or spawned by one?** The whole protocol is
`Reports/session-programs.md`: how a coordinator reaches a lane (the
mechanism is not the obvious one — `SendMessage` fails, a poke-only trigger
works), why a woken lane cannot reply, why the return path must be files,
and the four failures that cost an evening. **Actually spawning and running
lanes — for *any* program, not only the lab — is the `lab-coordinator`
skill**, which carries the same mechanics as runnable recipes. Its name says
lab for historical reasons and its contents are not lab-specific: **invoke it
before you spawn a session, whatever the round is about.** A held-world
coordinator skipped it on 2026-09-14 precisely because the name said "lab",
and made the one mistake its opening section exists to prevent — on four
lanes at once.

It is a report rather than a section here because it applies to a minority
of sessions and cost every one of them ~2,200 always-loaded tokens.
`CLAUDE.md` is read before any work begins, so anything most sessions do not
need belongs behind a pointer.

**One line of it is load-bearing enough to keep here.** Every
`create_session` call passes `model:` explicitly — **never inherited from the
coordinator** — owner cost policy, 2026-08-23. The default inherits the
caller's model, and three workers once silently inherited a premium tier and
ran $25–71 each inside ninety minutes. **Opus (`claude-opus-5`) is the
default**; step down to Sonnet only where being wrong is cheap and loud, and
up to Fable — which is *twice* Opus per token — only where being wrong is
silent and compounds. Which is which: the `lab-coordinator` skill — which,
despite the name, is the skill for running lanes in **every** program here.

## Method

Nearly every fix here that was judged by test output alone failed to change
what the owner saw on screen. The ones that worked all had the same shape:

1. **Look before you measure.** Render the scene and look at it first. Every
   metric written before anyone had looked has measured the wrong thing.
2. **Reproduce before you fix**, from the owner's description of the *initial
   state*, and confirm the reproduction shows the complained-about quantity.
3. **Verify live before declaring done** — `filmstrip` or the app's capture
   hook. Tests passing is not evidence that the screen changed.
4. **Look again after the fix, for what you did not measure.** A fix that
   cleared one artifact while introducing a worse one has shipped here once.

An image tells you *what* and *where*; a metric tells you *how much* and
*whether it came back*. Use each for its own question, and pair every debug
overlay with a probe that prints the values.

Each heading below is one rule. Its worked case and numbers are in the
evidence report under the same heading. Rules about *measuring the world* —
metric traps, cascades, the two drivers, chunk seams, frame cost — load with
`.claude/rules/measuring-the-world.md`.

### "Why did it do that" is answered by tracing individuals, never by a population statistic

Owner's rule (2026-09-20), and it holds for anything that decides per
individual. An aggregate cannot carry *because the gate it needed was shut*.
Trace every individual that reached the state in question (not one focal
animal), and put the **inputs and the chosen output** in each row, not just
the position. The `funnel` skill is the procedure.

### "Did it fire at all" needs a counter, not a picture

When a change adds a discrete event, print its count beside the image and read
both — a collapse read as "chunks working" had a body count of zero.

### Check that a planned step can demonstrate itself, before promising it will

Ask **which object this rule evaluates — a cell, a section, or a whole
piece?** — and check the quantities it needs (centroid, contact width,
moment) are even defined for that object.

### Resolve an ambiguous complaint before building anything

If a report could mean two things, measure both or ask. Much cheaper than the
fix for the wrong reading.

### Ask what your number counts when nothing is wrong — metric, counter, timing, difference or census alike

**The worst-recurring failure in this repo.** Check any new number against a
case you know is fine (specificity) **and** against a case you know is broken
(sensitivity — the positive control: construct the case whose answer is known
non-zero and see the instrument report it). **The tell with no control to
hand is tidiness**: outcomes here are chaotic, so a clean first result (a flat
queue, 1712/1712, a round 2.7x) usually means something collapsed the
complexity.

### A parse is a measurement, and it inherits every dimension the run swept

Before believing a parsed table, **print the key's cardinality and check it
against what the run swept** (24 seeds × 3 gaps, not 24 rows), and **wait for
the writer to exit** (`pgrep -x`). Two of your own instruments disagreeing on
one file is the tell.

### When the complaint is visible and persistent, measure the standing state, not the event rate

Count what is on screen now, not how often it was made: an artifact made once
and persisting outweighs one made constantly and gone next frame.

### Ask which *pixels* a lever moves, before ranking it by silhouette

The sibling of the rule above, and not only for plants: a lever that changes
which cell gets a *label* cannot move a picture set by material mix and palette.
Check the lever reaches the pixels before ranking it by how the result looks.

### A debug readout must not be a function of the thing it debugs

Build the overlay before the mechanism, as a **full replace on a fixed
dark→bright ramp**, never a blend into the cell's own colour.

### Fixing a bug often exposes a constant that was compensating for it

When a fix changes what a number *means*, re-deriving the constants that read
it is part of the fix. And a term in a weighted sum is not an independent
knob: **before a change that reallocates a shared budget, name the constants
calibrated against current behaviour and budget re-deriving them** — a correct
mechanism at inherited constants is a regression. A gate can also hide a bug
by making it unreachable: a test can pass because the code under it is dead.

### When every setting of a sweep fails the same way, suspect the sweep

Something that rode along with the mechanism is in every data point. Run the
control: the mechanism at its gentlest with every rider stripped. Identical
outputs across settings mean the knob was never connected; and prove a
pattern edit touched only its target.

### A designed oscillator must be divided out of every number it reaches — measurements as much as decisions

Day/night, the water cycle, weather and wind alias into thresholds, timings
and censuses alike. **Test: could this number have been different if I had
sampled it an hour later?** For light, `field::noon_equivalent_light` divides
the day out.

### A pass/fail read of a graded quantity hides the gradient

Bisecting against a guard, score arms on the **quantity**, never the exit
code — 15 → 2 wrong cells was once scored "still fails".

### A mean over *events* is not a mean over the thing you care about

**Before quoting any per-run aggregate across arms, read its `n` across arms
first.** When arms change how much run there is, a pooled aggregate is
weighted by the thing under test. Compare paired within seed.

### A cost that vanishes may be work that vanished

A 300x improvement in a subsystem nobody optimised is a claim that it was
doing nothing useful. Find the quantity that says whether it still is, and
control by **holding the semantic rule fixed**, not by adding a metric.

### A noise bar belongs to the job it was measured on

It does not transfer between jobs, and whatever bar you pick applies to
**both signs** — if it kills your bad news it must kill your good news. An
A/B's two commits must differ only by the change.

### Compare two runs, not one run against a remembered number

Prefer paired comparisons; always **re-measure the baseline in the same
session, on the same machine**, before reporting a regression.

### A scene that contradicts the code will look like a bug in the code

When a mechanism appears inert, check the scene still contains the situation
you think it does before touching the mechanism.

## Conventions

**Tests and guards**

- **A guard must be able to fail for the *replacement* artifact**, not only
  the original — if a fix trades one artifact for another, the test should
  catch the trade.
- **Check a guard's inputs actually vary what it guards.** A guard over a
  procedural system has to sweep the procedure and gate an **order
  statistic** (p90 or max over N seeds), never one seed. **Six seeds is not a
  sweep.** Build the sweep *before* changing a model over procedural content.
- **A superseded mechanism's tests keep passing while testing nothing.** Break
  the replacement and confirm the old tests fail; if not, delete them.
- **Determinism is required** (same build).

**Tuning and sweeps**

- **Set bars from measurement with headroom**, never from aspiration and never
  on the measured value. Leave an unreachable target's gap visible.
- **Two fixes failing the same way means the approach is wrong, not the
  tuning.**
- **A change that moves *nothing* is different evidence from one that moves a
  little**: an exactly-zero delta means suspect the condition you keyed on is
  degenerate before calling the lever dead.
- **A constant nobody can tune in either direction may be a counterweight** —
  ask what it is compensating for.
- **When several knobs move the same number, check what each one trades.**
- **When a rule must tell apart two things that can look identical, state the
  difference as data** (a bit on the cell), not as a shape heuristic.
- **For "does this look right", ship a runtime selector rather than
  choosing** — default to current behaviour and name the active one on screen.

**Performance**

- **Measure a cost against the state the optimisation exists for** — a
  settled world is where the dirty-rect skip works.
- **A size cap must bound work, never gate whether something happens.** Test:
  does exhausting the cap produce an *answer*, or merely *less work*? An
  answer is the bug. Live sites: `src/sim/load.rs:717`, `:1080`, `:1150`.

**Process and records**

- **A revert keeps the knowledge — and gets an address**: keep the
  reproduction (`#[ignore]` it), and add a `Reports/dead-ends.md` entry with
  the condition its rejection depends on, in the same change.
- **A new report gets its line in `Reports/README.md` in the same commit**;
  a superseding report updates the superseded line.
- **A shipped feature gets its README status section before it is called
  done.**
- **A significant change affecting a `wiki/` page updates that page and its
  freshness note (a real date) in the same change.**
- **Commit messages carry the measurement**: before, after, and what was tried
  and rejected.
- **Adding a rule here: state it universally, put the subsystem in the
  evidence** — would an agent working on weather recognise it as theirs? A
  rule that only bites in one part of the tree goes in a `paths:`-scoped
  `.claude/rules/` file instead. **Its worked case goes in the evidence
  report, not here** — this file is paid for by every session.
- **Removing a rule: judge frequency × cost × whether anything else would
  catch it**, never frequency alone. Cut when the mechanism no longer exists,
  machinery now enforces it, or a recurrence audit shows it is not what
  prevents the failure. Never cut on "this only happened once".
- Prefer an independent review before significant commits; batch small ones.

## Gotchas that have each caused a real bug

Gotchas tied to one part of the tree live in `.claude/rules/` and arrive when
you read a matching file. What stays here fires before any file is read: the
build, the suite, the measurement and the record.

- **You are probably measuring a binary that is not the code you wrote.**
  `cargo build --release` does **not** rebuild examples, and a stale
  `target/release/examples/foo` has a newer mtime than your edit. Build with
  `set -o pipefail; cargo build --release --examples` and check the output
  actually moved. **The tell: identical output across a change that must have
  moved something.** Rebuild before launching anything long, and make a
  harness echo its own parameters — an unknown argument is silently ignored.
- **Piping `cargo` into `tail`/`grep` throws away its exit code.** Use
  `set -o pipefail` and `${PIPESTATUS[0]}`; a failed `--examples` build leaves
  most binaries missing, not stale.
- **A commit message is not evidence the change is in the file.** After any
  stash, rebase or merge, re-read the function, not the diff.
- **Adding a member to a set something sweeps enrols it in every rule over
  that set.** Before adding to a registry (`presets()`, `paintable()`,
  `pass_names()`, species, materials), grep what iterates it and run those
  tests — they live in `tests/*.rs`, which `cargo test --lib` never reaches.
- **Before you cite a guard's green as evidence, put the fault back and watch
  it go red.** If it stays green it is blind: replace it. Exempt: a guard
  written before its fix, and tight asserts on deterministic functions.
- **A green suite does not prove a test ran** (a deleted `#[ignore]` took its
  `#[test]` with it; clippy's dead-code warning caught it) — **nor that it
  could fail** (a guard that hashes too little, or builds a fresh `Rng` the
  caller never would).
- **A red suite proves even less:** `cargo test` stops at the first failing
  binary, so the integration tests after it silently do not run. The tell is
  `Running tests/worldgen.rs` missing from the output.
- **Assert the property, not two instants fitted to one trajectory.**
- **`cargo fmt` is all-or-nothing** — `cargo fmt -- file.rs` formats the whole
  project. Do not let it ride along with an unrelated change.
- **Local and CI tool versions can drift.** `rust-toolchain.toml` pins 1.98 so
  plain `cargo clippy` matches CI; `rustup check` if you suspect otherwise.
- **The app locks its own exe** while running: `cargo build` and plain `cargo
  test` fail; `cargo test --lib` works. Bogus `LNK2019 anon.…` errors are stale
  incrementals: `rm -rf target/debug/incremental`.
- **Never `git add -A`** (denied in `settings.json`); stage explicit paths.
- **Grepping a prose phrase gives false negatives** — 23% of bolded phrases
  straddle a line wrap. Use `python3 scripts/docgrep.py "the phrase as it
  reads"`; short unique tokens grep fine.
