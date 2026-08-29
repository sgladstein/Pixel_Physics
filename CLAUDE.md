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
| `README.md` | Architecture, and per-milestone status. **~46k tokens — do not read it whole**: its **By topic** table maps subsystem to owning sections with line numbers, and milestone sections are named for the *build*, not the subsystem (`M17 status` is the structural-collapse write-up) |
| `wiki/*.md` | What a material or mechanic *does*, in plain language — no code, no file names. `Reports/*.md` is *why it's built that way*; this is *what it looks like when it's right*, which makes it **the written form of the bar your change is judged against**. ~34k tokens over 11 pages, so read the one page, not the directory. **Which page owns your file is not guessable for half of them**, and the map lives here because the wiki refuses file names by design: `field.rs`/`decay.rs`/`sky.rs` → `world-cycles.md`; `structural.rs`/`load.rs`/`rigid.rs` → `structural-collapse.md`; `explosion.rs`/`fracture_field.rs` → `explosions.md`; `plant.rs`/`organism.rs`/`assets/species` → `plants.md`; `creature.rs`/`brain.rs` → `ants.md`; `player.rs` → `the-gnome.md`; `worldgen/` → `the-world.md`; `update.rs`/`material.rs` → `powders.md` and `liquids-and-gases.md`; `liquid.rs` → `liquids-and-gases.md`; `fire.rs` → `fire-and-heat.md`; `weather.rs` → `weather.md` |
| `PLAN.md` | Roadmap, settled decisions, the issues backlog; the append-only progress log lives beside it in `PLAN-log.md`. **~60k tokens — do not read it whole**: start from its Contents, and in any session-handoff section read the dated *(State …)* line rather than the heading, which records only what was true when written |
| `Reports/README.md` | **The index of every design report**, with per-report status and an in-flight section for documents still on unmerged branches — check a report's standing there before trusting it or writing a new one |
| `Reports/dead-ends.md` | **Tried-and-reverted approaches** (595 at last census, 2026-08-26), each with the condition its rejection depended on and where the full record lives. **~97k tokens — grep the *mechanism* you are about to touch or propose, never your subsystem.** Measured 2026-08-26: `thicken` returns ~2,460 tokens, `max_unsupported_span` ~650, `chunk seam` ~250, and `rot_remains` **zero** — a real answer, cheaply. Grepping an area instead costs ~12k–31k, more than this file. For a genuine survey, grep the address prefix (`^- \*\*.\?src/sim/plant`) rather than the prose: 99% of entries open with the file they apply to, which halves it |
| `Reports/open-bugs-handoff.md` | **Open bugs.** Working reproductions and what has been ruled out *by measurement*. **~97k tokens — do not read it whole**: its generated status index is the first table in the file, so read that, then only the sections it lists for your area. (`dead-ends.md` owns "was this tried?"; this owns "is this broken?") |
| `Reports/design-philosophy.md` | Settles arguments about constants, hardcoding, and scope boundaries |
| `Reports/session-programs.md` | **Coordinator ↔ lane protocol** — only if you are coordinating sessions or were spawned by one |
| `Reports/instruments.md` | **What every `examples/` binary can already answer** — grep it before building a measurement harness. Several generalise well past the question they were built for, which is not guessable from their names |
| `.claude/README.md` | **The Claude Code configuration** — the `SessionStart` check, the permission allow/deny/ask lists, and why `.claude/` is tracked rather than ignored |
| `.claude/skills/review/SKILL.md` | How to put an artifact in front of the owner and get a verdict back — the primary feedback channel, used constantly |

**Which rules apply to what you are doing right now** (all in this file
unless named otherwise):

- Running a parameter sweep → *When every setting of a sweep fails the same
  way* (Method), *A change that moves nothing* (Conventions), and the
  `include_str!` gotcha. If it censuses a collapse, also *A cascade
  censused before it settles* — `seedsweep.sh`'s default frame budget
  stops mid-cascade.
- Writing or trusting a guard test → the guard bullets in Conventions
  (*fail for the replacement*, *inputs actually vary*, *superseded tests
  keep passing*) and *A green suite does not prove a test ran* (Gotchas).
- Measuring liquids, powders or destruction → *Metric traps*, *A mean over
  events is not the size of the pieces* and *Chunk decomposition is a
  recurring root cause* (Method).
- Adding per-cell work to the sweep → *Guard hot-path work at the call
  site* (Method), and README's Performance section on sweep-scale costs.
- Touching organism code → the structural-check amputation gotcha, *A
  traversal must use the same neighbourhood the writer used* (Gotchas),
  and *A channel that oscillates by design* (Method).
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
cargo test                                       # unit + integration. The --skip this line carried until 2026-08-26 is vestigial: bug A's test is #[ignore]d, so it does not run. Measured: `cargo test --lib` with no flag gives 943 passed / 0 failed / 54 ignored
cargo clippy --all-targets -- -D warnings        # CI gates this
cargo run --release --example ascii              # headless behaviour + worst-frame timing; CI runs it
cargo run --release --example filmstrip -- scene=fall zoom=2 crop=0,140,256,110
python3 scripts/review.py serve --open      # the owner's review queue; see below
python3 scripts/review.py serve --lan       # ...also reachable from a phone on the same Wi-Fi
bash scripts/acceptance.sh                  # the structural acceptance cases; CI gates this
bash scripts/seedsweep.sh                   # the order-statistic seed sweep; run BEFORE changing any model over procedural content
bash scripts/docscheck.sh                   # documentation checks: links, map-vs-tree, freshness notes, report index
python3 scripts/contextbudget.py            # what every session, agent and subagent pays before it starts; --gate is the ceiling, --check is gated by docscheck
bash scripts/branchcheck.sh                 # how far behind main this branch is, and which branches are merged-and-deletable; --gate is the CI trunk check
bash scripts/branchcheck.sh --brief         # ...summary only; this is what the SessionStart hook runs (`.claude/README.md`)
```

**The real app can be screenshotted headlessly**, which this file previously
assumed impossible — every "verify live" instruction routed through
`filmstrip` because the sandbox has no display. On a headless Linux box:

```
apt-get install -y libxkbcommon-x11-0 mesa-vulkan-drivers   # once per container
xvfb-run -a -s "-screen 0 1280x800x24" \
  env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
      PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=3 \
  ./target/release/pixel-physics                            # writes %TEMP%/pixel_physics_screenshot.png
```

`lvp_icd.json` is lavapipe, Mesa's software rasteriser; without it `Pixels::
new` fails with "Unable to create a surface" and the panic looks like a code
bug rather than a missing driver. It is slow — seconds per frame — so it is
for *looking at one frame of the real thing*, not for timing anything. Frame
timings still come from `ascii`, and `PIXEL_PHYSICS_CAPTURE_SEQUENCE` still
works for a strip.

`filmstrip` writes a contact-sheet PNG — several frames of one run in a grid —
so an artifact can be judged by eye without a window. Add `gif=1 out=x.gif` and
it encodes an animation instead, still with no window and no GPU: reach for that
when the question is whether something *moves* right, which a grid of stills
cannot answer. For the real app, press
`F7` to the `flat` preset — dead-level bare rock with 200 rows of sky, the
structural test bed — or set
`PIXEL_PHYSICS_CAPTURE_SEQUENCE=<start>,<interval>,<count>`; frames and a GIF
land under `%TEMP%`.

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

**This tree is worked in concurrently, and often by more than one agent at
once.** Git handles the merges; what it cannot handle is the three failures
below, each of which has cost real hours.

**`main` is the trunk. Never integrate against `master`.** `main` began as a
15-byte stub while the project lived on `master`, and the fix was to copy
`master` onto `main` — but the copy left `master` standing, so for a while
both names looked equally plausible and the docs named both. `3d53351`
records the result: a branch merged `master` while `main` was 10 commits
ahead, and silently missed the CLAUDE.md restructure, the map-scroll feature
and the play-button fix. Nothing failed; the session noticed because a diff
made no sense. `main` is the GitHub default, is the only branch CI gates, and
is what the reset procedure below names. `master` is a mirror with nothing of
its own and is on its way out. `scripts/branchcheck.sh --gate` fails if any
commit is ever reachable from `master` but not `main`, and CI runs it.

**A session cannot delete a branch, so the prune is the owner's to run.**
Measured 2026-08-25: 37 branches verified at 0 ahead of `main`, every
`git push origin --delete` returned **HTTP 403**, none succeeded. Pushing
commits works all day; deleting a ref does not, and the GitHub MCP server
offers no delete-branch tool either. The proxy was healthy with no relay
failures, so this is the credential's scope rather than a misconfiguration.
`branchcheck` can therefore *identify* deletable branches and never act on
them — when the merged count climbs, that is a message for the owner, not a
task any lane can pick up.

**Know how far behind you are, before you trust anything you measured on
it.** The worktree rule below keeps two sessions from breaking each other's
build; nothing was ever written down about staying current, so nothing pulled
a branch forward and the drift compounded silently. Measured 2026-08-22
across 27 remote branches: one was current, **ten sat at exactly 160 commits
behind `main`** — cut at the same moment and never once updated — and twelve
more were already fully merged and still standing as clutter. A branch does
not notice it is 160 behind. The merge does, and by then the conflict surface
is the whole session rather than a file or two.

So: pull `main` in *while* you work rather than saving it for the end. **You
no longer have to remember to check** — a `SessionStart` hook
(`.claude/settings.json`) runs `scripts/branchcheck.sh --brief` and puts your
ahead/behind, the merged/stale counts and the deepest unlanded branches in
context before you act. That hook exists because this paragraph asked for the
check by convention and the drift happened anyway; run the full
`bash scripts/branchcheck.sh` when the summary says something worth opening. This is not
tidiness — a baseline measured on a 160-behind branch is a measurement of a
tree nobody else has, and the numbers in a report written from it do not
transfer. The one exception the script prints for you: a branch sharing *no*
history with `main` is a deliberate orphan carrying data, not source
(`review-queue`, the review queue's transport — see `review_lib.py`'s
`SYNC_BRANCH`). Never merge `main` into one of those.

**Open a pull request for your work. This paragraph is the owner's standing
authorisation, given 2026-08-23 — you do not need to ask again.** The agent
harness declines to open a PR "unless the user explicitly asks"; this is that
ask, and it stands for every session in this repo. Nothing in the repo ever
said otherwise, which is why sessions kept reporting they had been told not
to: they were reading their own harness, not this file.

**You may merge your own pull request. Owner policy, 2026-08-25 — the
standing authorisation above now covers landing, not just opening.** An
independent session merges its own PR; a coordinator merges its lanes'. The
one condition is **CI green on the head being merged** — with no human
reviewer in the loop, CI is the only gate left, and "may merge my own PR"
must not become "may merge a red one". Your own harness may tell you never
to merge; as with opening a PR, that is the harness talking and this file is
the owner's instruction for this repo.

**Who opens it is decided by capability, not by role, and this is measured
rather than assumed.** The rule was nearly written as "a sub-agent opens the
PR", which would have encoded a step that silently does not happen:

| how the session was started | GitHub tools |
|---|---|
| in-process subagent (the `Agent` tool) | **yes** — verified 2026-08-25, a probe called `mcp__github__get_me` and authenticated |
| trigger-fired session (`create_trigger` + `fire_trigger`) | **no** — the trigger stamps its own `allowed_tools`, carrying no `mcp__*` |
| cloud child (`create_session`) | untested |

So an in-process subagent normally *can* open its own PR, and a woken lane
cannot — which is the case the "PR list is not the work list" section below
is about. Any session settles it in seconds: `ToolSearch` for
`mcp__github__get_me`. A session without the tools pushes its branch, writes
the PR body to a file on it, and reports the head SHA; whoever coordinates
opens the PR. **Either way the coordinating agent owns the merge.**

`create_trigger`'s `connectors:` parameter is *not* the fix for the woken
lane — it resolves against claude.ai connectors, and the GitHub server comes
from the Claude Code Remote environment instead (checked 2026-08-25: the only
connector installed is Google Drive). Connecting the GitHub App at org level,
as the section below says, is still the real one.

**Spawned worker sessions run on Opus (`model: "claude-opus-5"` on the
`create_session` call), never inherited from the coordinator. Owner cost
policy, 2026-08-23.** The default inherits the calling session's model, and
that default is the trap: a coordinator on a premium tier that omits the
parameter fans its own price out to every worker. It happened the day this
was written — three workers silently inherited the premium tier and ran
$25–71 each inside ninety minutes before anyone looked. A coordinating
session may itself be premium; the sessions it spawns may not.

What it cost to leave unsaid, measured 2026-08-23: **133 CI runs, every one on
`main` or `master`. Zero on any feature branch, zero from a `pull_request`
event.** No PR ever existed, so the workflow's `pull_request` trigger never
fired and pushes to `claude/**` matched nothing — the first time CI saw a
branch's code was *after* it landed, when a red suite can no longer tell you
whether the branch broke it or the merge resolution did. And a branch nobody
can see is a branch nobody merges: 27 accumulated, ten of them cut in one
fan-out and never once pulled forward.

**When to land**, from this repo's own 49 two-parent merges, each replayed
with `git merge-tree` to count the conflicts it actually produced:

| | |
|---|---|
| `behind x files > 300` | past the point where merges get expensive — act |
| feature complete | open the PR |

Every painful merge in that history (3+ conflicts) scored **above 340**; no
clean merge exceeded 1440 and the clean ones reach p90 at **280**. So 300 sits
in the gap rather than on a measured value. `bash scripts/branchcheck.sh`
prints your two numbers -- **`FILES` and `BxF`, which it did not until
2026-08-25** while this paragraph claimed it did. The consequence was not
cosmetic: with no printed operand each reader invented one, and two readings
of the *same* merge scored it **132** and **198**, one counting the branch's
changed files and the other main's. The script now settles it — `files` is
branch-side, `git diff --name-only origin/main...<ref>`, which is the operand
this rule's own reasoning implies (a large `files` means *this branch* has
become more than one feature).

**Read the screen for what it is.** 100% sensitivity is what the numbers
support — every painful merge was above 340, so **at or under 300 you are
safe** — and about 90% specificity, since clean merges reach p90 at 280. It
will therefore fire on roughly one clean merge in ten, and that is fine: the
action it prescribes (merge `main` in) is near-free and worth doing anyway.
Do not "improve" it into a lower bound; that throws away the only half that
prompts action. Two caveats on the provenance, both real: the 49 merges are
reported as "3+ conflicts" and "clean", so **merges with 1-2 conflicts are
unreported** and the classes do not partition; and the threshold was placed
in a gap by eye, not fitted.

**And it answers only one of the two questions.** It predicts *"will this
merge be laborious?"* — it is built from conflict counts. It cannot see
*"will this merge be wrong?"*, and that is not a tuning problem: measured
2026-08-25, two merges scoring 132 and 96 — comfortably "safe" — were
**zero-conflict by `git merge-tree`** and still broke the tree, because
`main` had added generated-file gates while the branch edited their sources.
The second question has its own instrument, below.

**The two terms want different remedies, and this is the part that gets
confused.** If `behind` is driving the product, merge `main` in — that fixes
it in place, costs nothing you were not going to pay at landing time, and
landing does *not* reduce drift: a 337-behind branch that opens a PR still
owes the same 337-commit reconciliation. If `files` is driving it, the branch
has quietly become more than one feature; land it and start another.

**Run `bash scripts/docscheck.sh` after every merge. Unconditionally.**
It is sub-second, and it is the *only* thing in the repo that catches a
generated file going stale against its source — `scripts/bugindex.py` over
the bug register's index, `scripts/readmetoc.py` over README's table of
contents. Both 2026-08-25 incidents were caught by it immediately and by
nothing else: `test`, `test-debug`, `clippy`, `ascii` and `acceptance` were
all green through a stale index, and CI carries `docscheck` only as an
informational job, so a stale index can reach `main` with every gate
passing.

**Do not reach for a file-overlap metric here — it was proposed and it does
not work.** Intersecting the two sides' changed files looks like a second
line of defence and is not: in both incidents the *generators* were
main-side only, and landed in the intersection purely because main happened
to regenerate the artifacts in the same commits. Had main added
`readmetoc.py` without touching `README.md`, overlap on it would have been
zero and the gate would have broken identically. It also over-fires — this
file, `README.md` and `Reports/README.md` are the contested row of the table
below and sit in nearly every intersection. The failure class is "main
changed generator G, branch changed source S, G != S", and only running the
checker sees it.

Do not read "land early" as "land broken". A half-finished `src/sim/load.rs`
on `main` costs every concurrent session, because they all build on it and
every measurement taken against it is void. And the fastest way to satisfy a
"commit and push now" impulse is `git add -A`, which is banned here for a
reason recorded below. Stage explicit paths, green the gates, then land.

**Work in your own worktree, not the shared checkout.** Two sessions in one
checkout share a `target/`, so one session's half-finished edit makes the
*other* session's `cargo test` and `cargo clippy` fail on code it did not
write and must not fix — and a running sandbox in one session locks the exe
the other needs to link. Both happened in a single afternoon. A worktree
gives each session its own `target/`, so a broken build stays local to
whoever broke it. (A cloud session gets a fresh container with its own
checkout, so this is about a shared *local* clone.)

**Know which files are yours.** Collisions are almost never random — they
land in the same few files every time. Counted over **188 branch landings**
(2026-08-25, `git log --merges` with each merge diffed against its first
parent), here is how many landings touched each file:

| Area | Files, with landings that touched them |
|---|---|
| **Contested — anyone may be in these** | `Reports/open-bugs-handoff.md` **118**, `README.md` **103**, `Reports/README.md` **103**, `src/sim/world.rs` **103**, `examples/filmstrip.rs` **99**, `Reports/dead-ends.md` **79**, `src/render.rs` **70**, this file **66**, `src/app.rs` 51, `PLAN-log.md` 50, `PLAN.md` 41 |
| Plants | `src/sim/plant.rs` 60, `organism.rs` 44, `examples/plant_probe.rs` 44, `wiki/plants.md` 67, `assets/species/*.ron` |
| Structural / destruction | `src/sim/structural.rs` 57, `rigid.rs` 32, `load.rs` 22, `scripts/acceptance.sh` 41, `wiki/structural-collapse.md` 43 |
| Creatures | `src/sim/creature.rs` 39, `brain.rs` 19, `assets/species/ant.ron` |
| Worldgen | `tests/worldgen.rs` 36, `src/worldgen/passes.rs` 27, `params.rs` 18, `assets/worldgen.ron` 15 |
| Fields, fire, weather | `src/sim/field.rs` 34, `fire.rs` 38, `weather.rs` 43, `decay.rs` 19, `src/sky.rs` 11 |
| The sweep itself | `src/sim/parallel.rs` 38, `update.rs` 24, `material.rs` 33, `scheduler.rs` 16 |
| The gnome | `src/sim/player.rs` 45 |

**Two things in that table correct what this file used to say**, and both
were measured rather than assumed:

- It claimed *"everything that has actually collided here collided in
  `src/app.rs`."* `app.rs` is **sixth**, at 51. `world.rs` (103),
  `filmstrip.rs` (99) and `open-bugs-handoff.md` (118) are all far more
  exposed, and none of the three was listed at all.
- The old table named only structural and worldgen, so **every other line —
  plants, creatures, fire, weather, the sweep, the gnome — had no row**, and
  an agent on one of them could not tell whether its files were shared.

`src/sim/liquid.rs` and `chunk.rs` show **0** landings in that window: real
files, currently dormant. A zero here means nobody is in your way, not that
the file does not matter.

So: **if you touch a contested file, land it quickly** rather than holding a
large diff across a session — the window in which someone else's work cannot
compile is the window you created. Recompute the table rather than trusting
it: it is a snapshot, and the command that produced it is named above.

**A file-ownership split is only as current as your last look at the branch
list.** Read once at session start it is stale within the hour, and nothing
prompts a re-read — the drift check has `branchcheck.sh` nagging for it, this
has nothing, which is exactly why it goes unasked. Measured 2026-08-23 on the
creature line's three-lane split: Lane A fetched the remotes before Lanes B
and C had branches at all, never looked again, and spent the whole session
believing it was the only lane. On that belief it filed four bug entries into
`Reports/open-bugs-handoff.md`, a file the split assigns to **Lane B** — which
had already filed all four, and better, its version of the foraging regression
bisected where Lane A's said "unattributed". Lane B then spent a merge
unifying the duplicates (`e3c5e76`), and Lane A had meanwhile told the owner
those lanes did not exist. **Before writing into a file another lane owns,
re-list the branches.** It costs one command, and the roster you were handed
is a claim about the past, not evidence about who is running now.

**The roster is the narrow case; the general one is that a shared append-only
file must be *read* before it is appended to.** Re-listing branches fixes
staleness of the roster, and two collisions the same day had no roster
confusion in them at all — both were single-owner filings by sessions that
knew exactly who else was running. `Reports/open-bugs-handoff.md` is where
they land, because it is append-only, lettered, and written into by every
line at once:

- Two different bugs were filed as **§Q** — one branch's colony-scene panic
  against `main`'s owner-reported debris needles, which already carried three
  inbound references. Landing them naively would have left two §Q headings in
  one document with those references silently resolving to whichever sorted
  first. The newcomer was renamed §R, its self-references repointed.
- A branch carried a stale copy of **§M still headed OPEN** which `main` had
  since closed. Resolved keep-both, that merge would have re-opened a fixed
  bug and sent the next reader at a generator with nothing to do with it —
  which is the failure §M's own entry opens by warning about.

So before adding a section: **grep the file for the thing you are about to
file** — and for the letter, run `python3 scripts/bugindex.py --check`, which
names both lines when one is used twice (`identifier 'D3' is used by 2
entries`) and is already gated by `docscheck`. Do not check the letter by
eye; that is what the tool is for. And when a merge conflicts there,
ask which side is *newer* rather than which is yours — a stale copy of an
entry the other side has since closed looks exactly like your own work.

**Check the split is self-consistent before trusting it, too.** The same
document gave Lane A everything under `examples/*` and told Lane C to add a
`creature_space` mode — a file under `examples/*`. Two lanes were directed
into one file by the plan itself, so the collision was authored in rather
than stumbled into.

If you find yourself needing to commit while a contested file holds
somebody else's unfinished work, do **not** try to stage around it. Add a
worktree at `origin/main`, re-apply your own change there, verify, commit
and push from it, then bring the main tree's branch pointer forward with
`git reset --mixed origin/main` — which moves the branch and leaves their
working tree untouched.

**That reset strands stale files whenever the main tree was *behind*** — they
show up as modifications that are really a *revert* of the upstream commit the
tree missed, and nobody recognises them as theirs. So: **note which files are
genuinely dirty *before* the reset**, because afterwards a stale file and an
edited one look identical. After it, diff anything newly modified against the
commits you were behind by; if it is their exact inverse and the file was clean
beforehand, `git checkout --` it. Full account, and the case it really happened
to, in `Reports/concurrent-sessions.md`.

## Running a program of sessions — moved out

**Coordinating other sessions, or spawned by one?** The whole protocol is
`Reports/session-programs.md`: how a coordinator reaches a lane (the
mechanism is not the obvious one — `SendMessage` fails, a poke-only trigger
works), why a woken lane cannot reply, why the return path must be files,
and the four failures that cost an evening.

It is a report rather than a section here because it applies to a minority
of sessions and cost every one of them ~2,200 always-loaded tokens.
`CLAUDE.md` is read before any work begins, so anything most sessions do not
need belongs behind a pointer.

**One line of it is load-bearing enough to keep here.** Spawned worker
sessions run on Opus (`model: "claude-opus-5"` on the `create_session`
call), **never inherited from the coordinator** — owner cost policy,
2026-08-23. The default inherits the caller's model, and three workers once
silently inherited a premium tier and ran $25–71 each inside ninety minutes.

## Method

Nearly every fix in this engine that was judged by test output alone failed to
change what the owner saw on screen. The ones that worked all followed the
same shape.

1. **Look before you measure.** Render the scene and look at it first. Every
   metric written before anyone had looked at the artifact has measured the
   wrong thing.
2. **Reproduce before you fix**, from the owner's description of the *initial
   state*, and confirm the reproduction actually shows the complained-about
   quantity before writing a line of fix.
3. **Verify live before declaring done** — `filmstrip`, or the app's capture
   hook. Tests passing is not evidence that the screen changed.
4. **Look again after the fix, for what you did not measure.** A metric only
   sees the quantity it was written to see. A fix that cleared one artifact
   while introducing a worse one has already shipped and been reverted here
   once, because its test only looked at the rows it expected to be wrong.

An image tells you *what* and *where*. A metric tells you *how much* and
*whether it came back*. Reaching for a metric to answer "what and where" is
the recurring mistake — and the inverse bites too: a corrected overlay was
still misread as "everything at the ramp floor" when the real value was 40%
of scale, genuinely hard to judge on a one-cell-wide twig. Pair every debug
channel with a probe that prints the values (`examples/plant_probe.rs`),
and reach for it the moment the question turns quantitative.

### "Did it fire at all" needs a counter, not a picture

An image shows
*what* and *where*; it cannot show whether the thing you built is what
produced it. A collapse rendered as coherent falling slabs, was read as
"chunks are working," and the harness's own body count said **zero for the
whole run** — the feature had never once executed, and what was on screen
was loose rubble that happened to hold its shape. Two very different
mechanisms look identical at the zoom a contact sheet is read at. When a
change adds a discrete "this happened" event, print the count next to the
image and read both.

### Check that a planned step can demonstrate itself, before promising it will

"Cracks weaken rock" was scheduled as an independently shippable,
judge-by-eye milestone. Built, it did almost nothing visible, because
failure was evaluated per cell against *its own* reach and a crack at a
beam's root weakens a cell the criterion never tests. One question asked
earlier — *which cell does this rule actually evaluate?* — would have caught
it before the work.

The same question was missed again later, in a different costume, **by a
session that had this paragraph in front of it**: a bearing rule that is
correct for a *piece* resting on loose ground was applied per cell, so a slab
lying on its own rubble was judged as many separate knife-edge footings and
taken apart one cell at a time. Ask it as **which object does this rule
evaluate — a cell, a section, or a whole piece?** — and check that the
quantities it needs (a centroid, a contact width, a tipping moment) are even
defined for that object.

### Ask which *pixels* a lever moves, before ranking it by silhouette

The sibling of the question above, and it has now cost a whole phase. Three
discrete architectural levers — sympody, tropism, acrotony — were built,
ranked "very high" and "high" on silhouette impact by a botany review, and
all three demonstrably *fired*: 46–186 sympodial forks per shrub, 1,797–2,750
plagiotropic steps per conifer, counters printed beside the sheets exactly as
this file demands. The owner's reading of those sheets was that nothing had
changed, and the composition numbers agreed — the three species differed in
height and mass and in nothing else. All three levers change **which cell
gets a label**: the order a child inherits, the reference vector it scores
against, which bud flushes. The silhouette was set by two things none of them
touch — every species was ~90% wood and ~5% leaf, and every plant in the
world drew from one four-brown palette and one four-green one. A lever that
relabels a cell cannot move a silhouette that texture and colour set. See
`Reports/plant-appearance-design.md`.

### Resolve an ambiguous complaint before building anything

"Flatness at rest" was read as the surface texture and turned out to mean a screen-wide
tilt — opposite directions, a whole detour spent on the wrong one. When a
report could mean two different things, the cheap move is to measure both
and see which one is actually there, or to ask. It is much cheaper than the
fix you build for the wrong reading.

### Ask what your number counts when nothing is wrong — metric, counter, timing, difference or census alike

**The single worst-recurring failure in this repo: six occurrences across two
independent sessions, the last two on one day.** Sanity-check any new number
against a case you know is fine, before trusting it about a case you don't.

The rule was written as *"ask what a **metric** counts"* and recurred
anyway, because none of the repeats looked like a metric in the moment. Name
the instrument and it stops hiding:

| instrument | how it lied |
|---|---|
| a **metric** | the whisker hunt defined a "film" as water with air above and below — *what falling water looks like* — so it counted every droplet in the world |
| a **counter** | 200 cuts reported against a flat queue; the counter counted **calls**, the harness aimed at soil, and 23 swings removed **0** cells |
| a **timing** | three 600-frame windows on the same world gave **0.00, 4.98 and 7.04 ms/frame**, each offered as "the settled field cost" — it was the wind |
| a **difference** | `extra lost = 0`, comparing two things that had both not happened |
| a **census** | counted every `Solid` in the world rather than the platform under test |

Its numbers are real every time. That is the point: a number that is
arithmetically correct and answers a different question than the one asked
looks exactly like a result.

**And against a case you know is broken, which is the half this rule was
missing.** The sentence above checks *specificity* — that the number stays
quiet when nothing is wrong. It does not check **sensitivity**: that the
number *moves* when something is. This file already has the sensitivity rule,
written for guards — *"before you cite a guard's green as evidence, put the
fault it is named for back and watch it go red"* — and it was never crossed to
measurements. Measured 2026-08-25, in one session: **six numbers that were
arithmetically correct, plausible, and about the wrong thing**, of which
five needed the guard rule applied to an instrument. Two of the six are the
counter and the census in the table above, seen from the other side — they
did not merely count the wrong thing, they *could not have moved*. The rest:

| what was measured | why it could not answer |
|---|---|
| a flat platform's damage | no span, so no load to concentrate, so no support rule could matter |
| a queue going quiet | means "converged" *or* "made immune", and queue depth cannot tell them apart |
| an A/B whose arms differed in two things | the paint path, not the rule under test, carried half the effect |
| six seeds | 1.64x; the next twelve gave 1.08x and the pooled median was **zero** |

**So run the positive control**: construct the case whose answer you *know* is
non-zero and check the instrument reports it. It is cheap, and it would have
caught three of those six outright and pointed straight at a fourth.

**The tell, when there is no control to hand: tidiness.** Outcomes in this
engine are chaotic, so a clean first result is evidence of an artifact rather
than of a strong effect. Every wrong number that day was tidy — a queue flat
at exactly its idle value, two arms agreeing at 1712/1712 and 1710/1710 and
1711/1711, a clean 2.7x, a clean 1.64x. The true answer was messy: 1.24x, a
per-seed median of zero, eight seeds worse and six *better*. When the first
number tells a clean story, something has usually collapsed the complexity —
often the very thing being measured.

### When the complaint is visible and persistent, measure the standing state, not the event rate

Attributing film *creation* blamed the
plain straight-down fall for 76% of them — true, and useless, because those
films existed for one frame each. The artifact that persists came from
somewhere else entirely, and only a standing count showed it.

### A debug readout must not be a function of the thing it debugs

Several channels that decide behaviour are invisible — per-cell scalars, not
occupancy. Build the overlay *before* the mechanism that uses them
(`render.rs`'s `FieldOverlay` / `OrganismOverlay`, `filmstrip`'s `channel=`),
and make it a **full replace on a fixed dark→bright ramp, not a blend into
the cell's own colour**. A magnitude-scaled blend was tried and produced a
canopy-density sheet that read as blank: the ramp was red, wood is brown,
and a mid-range value moved one colour byte from 139 to 155. The obvious
reading — "the mechanism is dead" — would have sent a fix at working code.

### Fixing a bug often exposes a constant that was compensating for it

`thicken()`'s flood fill traversed 4 neighbours while growth places cells at
8, so it counted a fragment of a tree rather than the tree. Fixing the
traversal made every cell see the true count and thicken uniformly — because
`pipe_ratio` had been calibrated against the broken quantity. **When a fix
changes what a number *means*, re-deriving the constants that read it is
part of the fix, not scope creep** — and sweeping is how you re-derive it.

Watch for the inverse too: a gate can hide a second bug by making it
unreachable. Infiltration's conservation test passed against a version whose
gate meant infiltration never ran at all. A test can pass because the code
under it is dead, which looks exactly like passing because it is correct.

**The same rule has a second shape, and it has to fire *before* you start
rather than after.** A term in a weighted sum is not an independent knob:
changing what one term can *express* reallocates the whole sum, even when no
number's meaning changed. Measured 2026-08-27 on the plant line, though the
shape is any scored choice or economy: `phototropism_dir`'s codomain was
`{(0,-1), (0,0)}`, so `light_weight` — authored up to 0.6 — could only ever
reinforce the up-vector. Reshaping it into a real 2D gradient, **the repair
`dead-ends.md` itself prescribes**, gave those weights a direction they had
never had; trees spread instead of climbing, never reached `seed_maturity`,
and reproduction went to **zero**. Every gate stayed green but one, and that
one fired for an unrelated reason. **So before starting a change that
reallocates a shared budget, name the constants calibrated against the
current behaviour and budget re-deriving them as part of the work — if that
is unaffordable, the change is not scoped, it is merely started.** A correct
mechanism at inherited constants is a regression. Full account, including why
a system of *unpriced* levers makes this the normal case rather than the
exception: `Reports/why-changes-cost-so-much-2026-08-27.md`.

### When every setting of a sweep fails the same way, suspect the sweep

The sibling of "two fixes failing the same way means the approach is
wrong", and it points the opposite direction. Eight settings across two
*forms* of leaf abscission all collapsed the stand identically — which read
as "the approach is wrong" and was not: a rider that had landed *with* the
mechanism was constant across every run and was alone the collapse (the
full account, with its numbers, is the structural-check amputation gotcha
below). A sweep only varies the knob; anything that rode along with the
mechanism is part of every data point. Before condemning an approach, run
the control that isolates it — the mechanism at its gentlest setting with
every rider stripped out.

And two more ways a sweep lies, both of which have produced whole invalid
sweeps: identical *outputs* across settings mean the knob was never
connected at all (see the `include_str!` gotcha below), and a pattern edit
can vary more than its knob — `tree.ron` holds two `crowding_weight`
lines, and a blind `sed` on the field name dragged the root's deliberate
`0.0` along with the shoot's through every data point. Prove the edit
touched only its target before trusting anything downstream of it.

### A designed oscillator must be divided out of every number it reaches — measurements as much as decisions

**Any** number sampled from a world that contains a designed cycle — day/night,
the water cycle, weather, the clock — carries that cycle's phase unless you
remove it. The cycle stays real on screen and in the field; it just must not
alias into anything you then compare.

This rule was written as *"divided out of decisions"* and recurred twice on
that framing, because neither repeat was a decision:

- a **cost measurement** — three 600-frame windows on one world reported
  **0.00, 4.98 and 7.04 ms/frame**, each offered as "the settled field cost".
  It was the wind;
- a **damage census** — `seedsweep`'s `cells lost` column rides the water
  cycle at about **±1,700 cells**, larger than most damage figures in the
  sweep, so any single-frame reading is that frame's phase plus the damage
  and the two are not separable.

The original case was a decision and is still the cleanest illustration: a
threshold on light sampled at an arbitrary phase is a different threshold
every hour, which produced a nightly extinction event — live tips 71 at noon
against 28 at night — until it divided the cycle out with
`field::noon_equivalent_light`. Full account in
`Reports/plant-economy-rederivation-2026-08-23.md`.

**The test is the same for a threshold and for a benchmark: could this number
have been different if I had sampled it an hour later?**

### Size a problem at the moment it starts, not after it has been running

The sibling of the cascade rule below, pointing the other way: that one says
a census taken **too early** reads a delay as damage, and this one says a
census taken **late** can be measuring the system's *response* to the event
rather than the event. Both are the same question — *what was the world doing
between the thing happening and me looking?* — and the second is the more
expensive mistake, because it sizes the fix.

Measured 2026-08-26 on `open-bugs-handoff.md` §S. One radius-20 charge, the
support field censused against a converged oracle at increasing distances
from the bang:

| censused at | cells wrong |
|---|---|
| **5 frames after** | **369** |
| 50 frames after | 42,825 |
| 1,300 frames after | 67,100 |

Every one of those is a real count of genuinely wrong cells. Read at 1,300
frames it says *"a charge invalidates 67,000 cells, so build a pass that
converges 67,000 cells"* — and a whole scope report was written on that
reading. Read at 5 frames it says the charge invalidates **370**, the other
sixty-seven thousand are manufactured by the engine's own slow correction,
and a pass that converges the damage once fixes almost nothing. The fix those
two readings call for is not the same fix, and the second one is the
expensive one to discover after building the first.

**So: before sizing a repair from a measurement, ask when it was taken
relative to the event, and take a second one close to the event.** If the two
disagree by two orders of magnitude, what you are looking at is a response,
and the thing to fix is whatever is producing it.

### A cascade censused before it settles reads a *delay* as damage

Both runs have to have **landed**, or the census is comparing mid-air with
on-the-floor. A change that made a room stand two hundred frames longer
measured as `roomcut` losing 251 cells against 1,501 at frame 202, and as
235 against 273 once both runs were given 1,500 frames — a disaster and a
rounding error, from the same two binaries.

**`seedsweep.sh`'s own default does this**, still, today. `FRAMES="start=2
every=400 count=4"` stops at frame 1,202, which is mid-collapse. Measured on
`scene=worldcrack strike=12`, one build, eight preset/seed pairs, frame
1,202 against frame 3,602:

| | rock destroyed @1,202 | @3,602 |
|---|---|---|
| `terraced 1` | 557 | **1,042** |
| `terraced 7` | none — rock *gained* 647 | **260** |
| `flat 1` | 20 | **199** |
| `rolling 7` | none — rock *gained* 223 | **88** |

Four of the eight destroy rock by 3,602. **The default misses two of them
outright** — it reads rock as net *gained* where the collapse has not yet
arrived — and understates the two it does see, by 1.9x on `terraced 1` and
**10x** on `flat 1`. `terraced 7` reverses outright: −634 cells lost at 1,202
becomes +326 at 3,602.

**Read `rock`, not `cells lost`, for the settling question.** `rock`
plateaus — `terraced 1` runs −952, −1,042, −1,042, −1,052, −1,052 across
frames 1,802 to 9,002 — while `cells lost` never settles at all: the same run
goes 849 → 1,109 → 745 → −126 → −1,322.

**That drift is an oscillation, not accumulation, and it is not the cascade.**
An earlier version of this section blamed it on weathering accruing rubble.
The control that settles it is the same scene with **no verb at all**: at
`strike=0`, `terraced 1` reports **zero failures and `rock +0` at every tile**
while `cells lost` swings 0 → 290 → 471 → 44 → −725 → −1,684. Nothing broke,
so no rock became rubble; the rubble census is simply riding something
periodic, and on `wetland` the `rock` column matches the frozen-water count
exactly — `rock +833` against `833 frozen` — which points at the water cycle.
Amplitude is about ±1,700 cells, **larger than most damage figures in the
sweep**, so a `cells lost` reading taken at any single frame is that frame's
phase plus the damage, and the two are not separable. This is the
*divide-the-oscillator-out* problem below, not a too-short budget — and until
it is divided out, `cells lost` cannot be used to compare two models on these
presets at all.

So `awake` and `sites` are a weaker tell than they look: on `rolling` and
`terraced` both sit near 5,000 sites indefinitely and never reach zero. The
tell that works is that **the quantity being censused has stopped moving**
across two consecutive tiles. `every=900 count=5` is enough for `rock`;
no budget is enough for `cells lost`, because the problem is phase, not length.

Worse, and the reason this needs its own heading rather than a footnote: two
runs that diverge on one frame are **different worlds** by the next, so a
single cascade scene cannot compare two models at all, settled or not. One
term measured *ten times worse* on `scene=worldcrack strike=12` and nearly
halved the worst case over 24 seeded runs. Comparisons of cascades belong in
`seedsweep.sh`, run to rest, read at the order statistic.

### A mean over *events* is not a mean over the thing you care about

The sibling of the metric traps below, one level up, and it nearly cost a
correct change. `failing region size: mean` divides cells by **failure
events**, and a change that makes marginal rock fail more often moves that
mean without moving what comes out: `caveshallow` went from mean 10.0 to
4.1 — below `rigid::MIN_FRACTURE_CELLS` (6), which reads as *"the typical
break is now too small to fracture, so it is dust"* — while losing the
identical 64 cells of rock and sending **fewer** cells down the powder path
(30 → 16). The extra events were confined failures cracking in place, which
never reach the fragment ladder at all.

A mean cannot separate "smaller pieces" from "more evaluations of the same
piece". If the question is whether something turned to dust, count the
regions that fell below the fracture threshold and read that against the
total that failed. `FailureCounts::crumbled` counts exactly that -- the
regions `rigid::fracture_failing_region` declined and the cells they took --
and `filmstrip` prints it as `crumbled to grit` beside the mean. Read that,
not the mean, whenever the question is whether something turned to dust.

### A timing number is only as trustworthy as the box was quiet

Two runs of a **byte-identical** `examples/ascii` on bit-identical
deterministic work disagreed **2.42x** on one scene, and reversed the
serial/parallel ordering on another — one run reported the *parallel* stress
scene slower than the serial one, backwards from M5's whole purpose, and the
other reversed it. Both orderings cannot be true. Nothing in the simulation
changed: the statistic was measuring the rest of the machine. Two rules come
out of that, and neither depends on the machine it was measured on:

- **Gate on counters, never on wall clock.** Everything `examples/ascii.rs`
  asserts is a deterministic count, identical under any load. A wall-clock
  assertion is a flake generator — and usually the counter above it is the
  stronger claim anyway, because "the pass did no work at all" cannot be
  explained away by a busy box. Measured again independently 2026-08-25 by the
  perf lane: a scheduler census recompiled between two runs of one scene came
  back **byte-identical** (`produced 7042 / deferred 61488` at frame 4,800,
  both times) while the wall-clock column on the same frame moved 9.54 → 8.16
  ms. The counters reproduced exactly where the clock moved 17%.
- **…and check what the counter counts.** A counter inherits the wall clock's
  failure mode by a different route: it is exactly as trustworthy as the claim
  that the thing it counts is the thing you care about. Measured 2026-08-25,
  two hours after the rule above landed, and it nearly published a null: a
  harness probing whether the pick leaks the way a blast does reported 200 cuts
  and a queue flat at its idle 5,400 — a clean, counter-based negative. **The
  counter was counting calls.** `rigid::is_tool_target` accepts
  `Solid | Plant` and refuses bedrock, and the harness aimed at the topmost
  `Solid | Powder` cell, which on a rolling world is soil — so every swing
  landed in dirt. 23 swings, **0 cells removed**. With the aim corrected the
  same 23 swings remove **1,157**, and the queue then goes to the scheduler's
  cap and stays there. The cheap guard is to **pair every "it fired" counter
  with an effect counter from the far side of the call**: `rigid::mine_swept`
  returns its own loosened count, `rigid::strike` returns `()`, so the second
  needs a census of the neighbourhood either side of the blow. This is *Ask
  what a metric counts when nothing is wrong* (below) applied to counters
  rather than to metrics — a null is where it hides, because a null looks the
  same whether the mechanism is quiet or the probe never reached it.
- **…and a *positive* hides from the opposite direction.** A null hides from
  **inattention**: nothing demands an explanation. A positive hides from
  **motivated reasoning**: it is the result you wanted, and every check you
  reach for is one it passes. Worked case and remedy: *A cost that vanishes
  may be work that vanished*, below.
- **Measure one scene, not the suite.** A short run can land inside a quiet
  window; a long one structurally cannot, so a full-suite timing figure is
  untrustworthy by construction rather than by luck. Run the whole suite for
  the counter gates, where load is irrelevant.

**A worst-frame figure is worthless unless an aggregate independently pins
it.** This is the corrected form of the rule, and the correction came from the
perf lane pushing back with a case the original got wrong. The original said
flatly that an untrusted worst is worth nothing — true for the case it was
drawn from, where the worst is one frame among thousands of *comparable* ones
and a single scheduler preemption can set it (measured across three attempts at
one scene: the worst moved **6x** with machine state while the median moved
~30%).

It is false when the expensive event is **rare**, because then the mean is not
independent of the worst — it contains it. The test is arithmetic: **mean ×
frames ≈ worst**. One blast per run puts essentially all time ever spent in the
blasts phase into a single frame, and the perf lane's converged-pass figure
pins at 0.97 (mean 0.076 ms × frames = 456 ms against a 440.7 ms worst), its
bedrock-only control at 0.96 — while the ascii case above pins at nothing at
all. (Two further independent legs held there; they are in the report.)

So: run the ratio before quoting a worst. If an aggregate pins it, quote it; if
it is an order statistic over many similar frames, it is noise wearing a
number. An untrusted *median* is worth something either way.

`Reports/measurement-under-contention.md` has the evidence, and records why the
machine-wide lock it designed was deliberately not landed.
### A cost that vanishes may be work that vanished

The sharpest version of *look again for what you did not measure*, and it
cost a night. §S's backlog — a blast leaving the structural scheduler pinned
at its cap for ever — was attacked with a converged relaxation pass over the
damaged region. At a large enough region the queue did not shrink, it
**disappeared**: 5,134 pending against 25,876, scheduler 0.03 ms against
10.08 ms, whole frame 31.21 → 18.98 ms, and `scripts/acceptance.sh` green on
every case. It reads as a complete fix and it was an artifact:
`relax_region` anchors any cell resting on loose ground at distance 0
outright, where `tick` takes that root only as a last resort, so the pass had
rooted the whole blast neighbourhood flat and the structural system simply
had nothing left to say about it. **A queue that goes quiet because the
system stopped asking is indistinguishable, in every timing, from one that
went quiet because it converged.**

Two things to carry:

- **When a cost disappears rather than shrinks, suspect the work
  disappeared.** A 300x improvement in a subsystem nobody optimised is a
  claim that the subsystem was doing nothing useful. Find the quantity that
  says whether it still is — here `max aux`, the largest support distance in
  the field, which read 142 with the "fix" and 2,482 without it.
- **The control is to hold the semantic rule fixed, not to add another
  metric.** One env switch putting `relax_region` back on
  `compute_world_distances`' bedrock-only rule, changing nothing else,
  settled it in one run: the queue came straight back to baseline. Measuring
  *around* the confound would have taken all night and convinced nobody.

And note what did **not** catch it: acceptance was green on all cases
throughout, damage counters still fired, pieces still came off. A guard over
"does destruction still happen" cannot see "destruction happens over a region
that has quietly been made immune".

### An isolated harness overstates what the app will see

The sibling of the paired-baseline rule below, and it cost a wrong headline
before it was caught. The same field change measured **−50%** in
`field_cost` — which runs the sweep and the field and nothing else — and
**−27%** in `scale_probe phases=1`, which runs the whole of `App::update`,
in the same session on the same machine. Neither is wrong; they answer
different questions. The app-level number is smaller because the other
phases keep chunks awake and enlarge the solve set the optimised pass has to
walk. **Quote the whole-frame figure**, and treat a subsystem harness as
aiming the work rather than sizing it.

**A sub-phase breakdown of the *same* harness overstates in the same way, and
the mechanism is different enough to be worth stating separately: removing
work is not the same as removing cost.** Measured 2026-08-26 on the field's
momentum passes. A gate that skipped them for tiles whose neighbourhood
provably could not give them momentum removed **91% of that work** — 1,497
solved tiles down to 147 — and the per-pass timings moved a long way with it:
pressure 0.92 → 0.39, velocity 2.87 → 1.11, advection 3.25 → 1.49, the field
step 14.50 → 9.87 ms. It was bit-identical, and it made the frame **slower**:
eight alternating paired runs of two fixed binaries put the difference at
**+0.59 ms, slower in 7 of 8**. The gate's own bookkeeping was timed and is
not the answer (0.15 ms amortised). What was left is that the skipped passes
had been *touching every solved tile*, and the full-set pass that runs after
them then paid the cold misses instead — the arithmetic went away and the
memory traffic only moved. On a `HashMap` of tiles walked by pointer-chasing,
the traffic is the cost.

So when a change makes one phase cheaper, **the phase it was made cheaper
against is the whole frame**, measured paired and alternating. A sub-phase
row that falls by a third while the frame does not move is not a partial win
being masked by noise; it is usually the cost relocating.

### A noise bar belongs to the job it was measured on

Two rules, both from one overturned claim (2026-08-25, `Reports/frame-cost-
audit-2026-08.md`), and both cheap to get wrong.

**A noise figure does not transfer between jobs.** A +56 s slowdown in
`cargo test (release)` was dismissed as noise using a ±60 s spread measured
on `cargo test (debug)` — a different job, on a different profile, that the
change provably could not affect. Measured on its own terms the release job
reproduces to **11 s**, so the slowdown was never inside noise. The debug job
was a valid *control* (it answers "is this job affected") and an invalid
*ruler*.

**And whichever bar you pick applies to both signs.** The same ±60 s was used
as noise where it was inconvenient (+56 s) and as signal where it was
convenient (−52 s, claimed as a speedup). That inconsistency is visible in
one's own table and is the thing to check before publishing a delta: if the
bar kills your bad news, it must also kill your good news.

The paired corollary: **an A/B needs the two commits to differ only by the
change.** The invalid version compared a one-run baseline against the median
of three later runs that also carried fifteen commits of another lane's work.
The valid one was already in the history — the change's own commit against
its parent.

### Compare two runs, not one run against a remembered number

Outcomes here have enormous spread — twelve identical trees from one genome
span 31 to 153 cells. A bar set from a single run is a sample from a wide
distribution and will flake in whichever direction that run landed. Prefer a
**paired comparison** (rooted bank vs bare bank, loaded branch vs bare
branch): it cancels everything the rule under test is not about.

This applies to frame timings too. A change once looked like a 25–50%
regression against a figure measured an hour earlier; re-measuring the
baseline on the spot showed the machine had slowed and the change cost
nothing. **Always re-measure the baseline in the same session, on the same
machine, before reporting a regression.**

### Guard hot-path work at the call site that already has the data

New per-cell behaviour usually applies to one material. Gating *inside* the
function still pays a `World::get` and a lookup per cell per frame, and
matching a material by `id_of("name")` is a string hash in the sweep. Put the
opt-in on `Material` as a field and test it at the dispatch site, which
already holds the `Cell` — a `Vec` index instead.

### A scene that contradicts the code will look like a bug in the code

Two Phase-2 "root bugs" were scene errors: free water buried in a soil
column sank (soil is a `Powder`, it sinks through `Liquid`) and surfaced, so
every seed germinated onto water; and a soil column with no floor or walls
fell out of the world and toppled, leaving the sampled cells empty. Both read
exactly like "the mechanism does nothing". **When a mechanism appears inert,
check the scene still contains the situation you think it does** before
touching the mechanism.

### Metric traps, each of which has already cost real time

- **Liquids: measure column *volume*, not the topmost cell.** A `Liquid` cell
  holds continuous fill, and near-empty cells fringe every artifact. Topmost-
  cell said chunk seams were 1.7x the interior roughness; volume said 9x.
- **Dark or torn rows: measure *fill*, not occupancy.** `render.rs` dims a
  liquid toward black by fill, so a row can draw as a black line while every
  cell in it is still occupied. An occupancy metric finds literally nothing.
- **Powder faces: measure the face, not the spreading front.** The front
  crosses seams smoothly while a vertical face persists behind it.
- **Destruction: a failure count is not a damage count.** `FailureCounts`
  counts cells that *failed*; a failed cell that became rubble is still
  standing there. Two digs whose event counts look comparable removed 894 and
  23,042 cells. If the question is "how much did this eat", census the
  materials before and after — nothing in the engine measured that until it
  was needed, which is why the regression below went unseen.
- Prefer a **continuous** quantity (a summed deficit) over a **count** of bad
  cells. Counts give knife-edge margins; sums separate cleanly.

### Two drivers, and the app runs the parallel one

`update::step` is serial; `parallel::step` is a four-pass checkerboard, and it
is what `App::update` calls. **Test both.** Behaviour that only the player
sees is behaviour only the parallel driver produces.

`update::step_monolithic` (test-only) sweeps the whole world as a single
region. It is the control for the question that took three wrong hypotheses to
reach the first time: *is this coming from the movement rules, or from how the
sweep is cut into chunks?*

### Chunk decomposition is a recurring root cause

Both drivers sweep chunk by chunk, so every cell in a chunk updates before any
cell in the chunk to its right, and half of all horizontal seams invert the
bottom-to-top row order. Artifacts that line up with the F1 chunk grid are
usually this, not the physics. Suspect it early; the reports on liquids do not
consider it at all.

## Conventions
**Tests and guards**

- **A guard test must be able to fail for the *replacement* artifact**, not
  only the original one. A fix that cleared torn seam rows and introduced
  much worse banding passed its own test, because that test only looked at
  rows lying on a seam. If a fix trades one artifact for another, its test
  should be the thing that catches the trade.
- **Check that a guard's inputs actually vary what it guards.** All eight
  acceptance scenes stayed green through a change that made one world seed
  lose 26x more material to a single dig — and a ninth hand-authored scene
  would have been just as blind, because `seed=` reaches only two scenes and
  every structural case builds hand-placed geometry at the default seed. The
  scenes were not too few; they were blind *by construction*. A guard over a
  procedural system has to sweep the procedure, and it should gate an **order
  statistic** (p90 or max over N seeds) rather than any single seed: outcomes
  here are chaotic in the seed, so which one is worst reshuffles on any
  legitimate change and a per-seed baseline gets rubber-stamped. **Six seeds
  is not a sweep**, measured 2026-08-25: §S2's anchor-rule census read
  **1.64x** over its first six seeds and **1.08x** over the next twelve,
  pooling to a per-seed *median of zero* with a third of seeds running the
  other way. The six-seed sample was not wrong, it was unrepresentative —
  and it looked exactly like a clean result. **This
  happened twice in one session** — two different changes to the load model,
  both green on all eight cases, the second eating fifty times more world than
  the bug it was fixing. A seed sweep caught each in one command. So build the
  sweep *before* changing a model that governs procedural content, not after:
  on green alone, both would have shipped.
- **A superseded mechanism's tests keep passing while testing nothing.**
  Distinct from the `#[ignore]` case below — these *run*, pass, and exercise
  nothing, because the scenario is trivially stable once the mechanism they
  were written for is gone. When replacing a mechanism, deliberately break
  the *replacement* and confirm the old tests fail. If they still pass,
  delete them rather than porting them.
- **Determinism is required** (same-build, per `PLAN.md`) — reversed from
  "not required"; the reasoning is `Reports/emergent-world-architecture.md`
  §8. (An earlier version of this bullet warned of stale comments saying
  otherwise; a sweep found none survive.)

**Tuning and sweeps**

- **Set bars from measurement with headroom**, never from an aspiration and
  never sitting on the measured value. Where a report asks for a number the
  engine cannot yet hit, record both and leave the gap visible rather than
  relabelling it away.
- **Two fixes failing the same way means the approach is wrong, not the
  tuning.** Two separate attempts to penalise a cell crossing a chunk seam
  both replaced the tear with a throttle at the same seam. That is a signal
  to change the approach — the third attempt fixed the sweep *order* instead
  and cost nothing.
- **A change that moves *nothing* is different evidence from one that moves a
  little.** Gating the granular capacity divisor on `parent.is_none()` was
  recorded as a dead end because not one cell moved, across six presets and
  three seeds. It was not wrong, it was **vacuous**: `structural::tick` rooted
  a cell's distance at 0 the moment powder touched its underside, so every
  powder-backed cell was parentless *by construction* and the gate had nothing
  to discriminate. An exactly-zero delta means suspect the condition you keyed
  on is degenerate, before concluding the lever is dead — and re-test any
  do-not-retry entry of that shape after something changes its condition.
- **A constant nobody can tune in either direction may be a counterweight, not
  a model.** That same divisor existed to cancel the eager rooting above: two
  modelling errors roughly annulling each other. Every attempt to tune it made
  some case worse because it was holding a different mistake in place. When a
  term resists tuning in both directions, ask what it is compensating for.
- **When several knobs move the same number, check what each one trades.**
  `min_transfer` and `HORIZONTAL_TRANSFER_REACH` both make water settle
  sooner; the first does it by giving up on the last of the levelling
  (residual tilt 1 → 5 cells), the second costs no accuracy at all. Knobs
  that look interchangeable on the headline metric usually are not.
- **When a rule must tell apart two things that can look identical, state
  the difference as data.** Four successive support models tried to infer
  "is this held up" from *shape*, and every one was either strong enough to
  hold a mountain or weak enough to let a player's tower break, never both —
  because geometry cannot distinguish a mountain from a wall someone
  stacked. The fix was a bit on the cell saying which it is. If tuning keeps
  trading one case for the other, the rule is reading the wrong quantity;
  more tuning will not find a setting that does not exist.
- **For "does this look right", ship a runtime selector rather than choosing.**
  Five grain modes behind one key settled in minutes a question that no
  amount of argument or still images had. Default to current behaviour, name
  the active one on screen, and state what each option *costs*.

**Performance**

- **Measure a cost against the state the optimisation exists for.** An
  animated grain looked free in every moving scene and cost ~10 ms/frame on a
  *settled* one, because what it defeats is the dirty-rect render skip — and
  a settled world is exactly where that skip does its work.
- **A size cap must bound work, never gate whether something happens.**
  **The test is semantic, not syntactic: does exhausting the cap produce an
  *answer*, or merely *less work*?** An answer is the bug. Stated as
  `if too_big { return }` this rule was findable and still missed twice more,
  because neither repeat had a `return` in it — a truncation that *understated*
  a subtree's torque, and a budget whose exhaustion resolved to
  **"supported"**. Both reports quoted this rule while failing to be saved by
  it. The original: fracture declined any region larger than its body-size cap
  and fell through to per-cell conversion, so the *bigger* the collapse the
  more certain it dissolved into dust — the cap belonged on a fragment, not on
  the decision to break at all. **Three live sites carry this shape today**
  (`src/sim/load.rs:717`, `:1080`, `:1150`, each `budget == 0 || >= MAX_*`);
  check what each returns when the budget runs out before trusting it.

**Process and records**

- **A revert keeps the knowledge — and gets an address.** Keep the
  reproduction (`#[ignore]` it if it now fails), record what the withdrawn
  fix was, what it improved, and why it went, and add the entry to
  `Reports/dead-ends.md` in the same change, with the condition the
  rejection depends on. A reverted fix's genuine improvements become the
  bar its replacement must meet — not the pre-fix baseline.
- **A new report gets its line in `Reports/README.md` in the same commit**,
  and a report that supersedes another updates the superseded line.
  `scripts/docscheck.sh` flags omissions, including a merged report still
  listed as in-flight.
- **A shipped milestone or feature gets its README status section before
  the work is called done.** Five features went undocumented for multiple
  milestones and had to be reconstructed after the fact; the Status
  section's "known limitations" also outlived two of the fixes that
  removed them.
- **A session that makes a significant change affecting a `wiki/` page must
  update that page (and its freshness note — a real date, never "this
  build", which can never go stale) in the same change.** This is a
  cheap backstop, not the real defence against `wiki/*.md` going stale the
  way early design Reports did — the real defence is that each page
  describes coarse, player-visible behavior, not implementation, which is
  inherently more stable. This rule just shortens the gap on whatever does
  drift.
- **Commit messages carry the measurement**, not just the intent: the number
  before, the number after, and what was tried and rejected on the way.
- **Adding a rule to this file: state the rule universally, put the subsystem
  in the evidence clause.** This file is loaded before every session begins,
  so a rule that *reads* as belonging to one line is silently lost on every
  agent working elsewhere. The test is one question — **would an agent
  working on weather recognise this as theirs?** If not, either scope it
  explicitly in the first clause, so the sessions it does not cover can skip
  it, or it is universal and mis-framed. Measured 2026-08-25: the ethos
  section — the owner's stated core value, *above correctness of any
  individual mechanic* — was **90.9% destruction vocabulary and mentioned
  plants, liquids and creatures zero times**, because its framing sentence
  was "destroying something should feel like destroying it". The two laws
  under it were always universal; only the framing was not, and the plant
  line had independently rediscovered both. **Gotchas are exempt** —
  they are concrete bugs and their specificity is the whole value.
- **Removing a rule from this file: a rule earns its place on *frequency x
  cost of the failure x whether anything else would catch it*, never on
  frequency alone.** Until 2026-08-25 this file had an addition criterion
  and **no removal criterion at all**, which is why it ran **+2,583 / -365
  lines, a 7.1:1 add-to-remove ratio**, over its whole history. A file that
  only grows dilutes every rule in it.
  **Low frequency is not grounds for cutting**, and the clearest case says
  so: measured across 500 commits, `sort_unstable`/tie-order arises **3
  times** — against 1,029 for "measuring anything" — yet it silently changes
  how every plant in the world grows and its own entry records that nothing
  in the suite would catch it. Rare, catastrophic and undetectable earns its
  place; common, cheap and caught-by-CI does not.
  **Cut on one of three findings**, all checkable: the mechanism it names no
  longer exists (grep it); machinery now enforces it, so the prose is a
  pointer at best (`git add -A` is in `settings.json`'s `deny` list); or a
  measured recurrence audit shows the situation arises and the rule is not
  what prevents the failure. **Never cut on "this only happened once"** —
  measured the same day, 30 of 39 rules cite a single incident, and the bulk
  of those are environmental facts or method rules that generalise regardless
  (`cargo fmt` is all-or-nothing for everyone, whoever hit it first).
- Prefer an independent review before significant commits; batch small ones.

## Gotchas that have each caused a real bug

- **Two conventions for `Cell::aux` point opposite ways.** On a `Liquid`,
  `aux == 0` means **full**. On a `Powder`, `aux == 0` means **dry**
  (`material::SOIL_SATURATED`). Both defaults are deliberate — liquids are
  created full, soil is created dry — and getting either backwards
  manufactures water out of nothing. A partly-drained liquid must be written
  as `with_aux(remaining)`, and a fully-drained one as `Cell::EMPTY`, never
  `with_aux(0)`.
- **A traversal must use the same neighbourhood the writer used.** `Grow`
  places organism cells at 8 neighbours; anything reading a grown organism
  back has to traverse 8 or it sees disconnected fragments. Transport
  (`diffuse_resource`) is the deliberate exception and stays at 4: an
  exchange crosses a shared face, and diagonal cells share only a corner.
- `Cell::is_empty()` is **managed-aware** — a promoted liquid body's container
  cells are materially empty but read as not-empty. Use the raw
  `cell.material == material::EMPTY` when the question is "is there material
  here", not "is this position available".
- `MAX_REACH == CHUNK_SIZE / 2` exactly, and that equality is load-bearing for
  `parallel.rs`'s cross-chunk write-safety proof *and* for its
  reinsert-then-replay loop. Changing it needs both re-derived.
- **A commit message is not evidence the change is in the file.** A `git
  stash` cycle restored an older blob over a source file, so a commit that
  claimed a behaviour change shipped only its *doc comment* — the code kept
  the old predicate, and nobody noticed for four commits, because the
  message read correctly and the tests still passed. After any stash, rebase
  or merge, re-read the function, not the diff.
- **The app locks its own exe.** While the sandbox is running, `cargo build`
  fails with "failed to remove `pixel-physics.exe`" — and so does plain
  `cargo test`, which builds the bin target to run `main.rs`'s eight tests.
  (This file said `cargo test` still works; it does not, and that cost a
  confusing ten minutes.) `cargo test --lib` works throughout, and is what
  to reach for with the app open. Separately, stale incremental artifacts
  produce bogus `LNK2019 unresolved external symbol anon.…` link errors —
  `rm -rf target/debug/incremental` clears it, and it is not a code error.
- **An unstable sort's tie order is not a function of the comparator alone —
  it depends on the element type.** `sort_unstable_by` (ipnsort) specialises
  its small-sort strategy on the type's size and properties, so two sorts
  that ask the comparator identical questions in identical order can still
  order **equal** elements differently. Measured 2026-08-24 in
  `plant.rs`'s `allocate_to_frontier`: caching the sort key to stop the
  comparator calling `world.carbon_at` twice per comparison changed the
  element from `(i32, i32)` to `(f32, (i32, i32))`, and the stand diverged —
  tree heights 101 → 103, stem thickness 9 → 6, root depth histogram
  [49, 43, 7] → [47, 38, 13]. Donor carbon is equal constantly (mature cells
  sit pinned at `RESOURCE_SCALE`), so the tie order decides which donor is
  drained. So: **any "cache the sort key" or "change the element type"
  optimisation over an unstable sort is a behaviour change until the
  comparator breaks ties explicitly**, and the free-looking half of that
  trade does not exist. The standing risk this leaves, recorded in
  `Reports/dead-ends.md`: a Rust upgrade that retunes the sort can silently
  change how every plant in the world grows, and nothing in the suite would
  catch it.

- **Never `git add -A` here** — and you now cannot: it is the one rule in
  `.claude/settings.json`'s `deny` list, because it is the one this file
  states unconditionally. Doing so once swept ~1,200 lines of someone else's
  in-progress work into an unrelated commit. Stage explicit paths, and see
  "Working alongside another session" above — `git add -A` is the symptom, a
  shared checkout is the cause. Force-push, rebase, amend and `reset --hard`
  are on `ask` rather than `deny`: those are forbidden *on someone else's
  branch* and fine on your own, and a conditional rule can only be asked.
- **`cargo fmt` is all-or-nothing.** `cargo fmt -- some/file.rs` formats the
  whole project, not that file — 28 files and ~3,000 lines in one go. The
  full-format pass is deliberately deferred work (`PLAN.md` issue #10) and
  CI keeps `cargo fmt --check` informational for exactly that reason, so do
  not let it ride along with an unrelated change.
- **Before you cite a guard's green as evidence, put the fault it is named
  for back and watch it go red.** This is the general remedy for the three
  bullets below, and it is stated first because each of them describes a
  *different* mechanism by which green means nothing — so an agent who has
  ruled out the two named mechanisms concludes green is informative, which is
  how a correct finding was once withdrawn. If the guard does not go red it is
  not weak, it is **blind**: replace it rather than widening its assertion.
  **The trigger is citing the green, not owning the guard** — a blind guard
  costs nothing sitting there and costs a day when someone argues from it.
  **Two exemptions keep this cheap.** A guard written *before* the fix has
  already been watched going red; you have it for free, and the rule only
  costs extra when the test came after the code. And a tight assertion on a
  deterministic function cannot be blind in an interesting way. What it is
  *for* is the case where green is the **default state** — loose assertions
  over emergent behaviour, hand-constructed inputs, order statistics — which
  is most of this engine. **Make it a command rather than a discipline.**
  Measured 2026-08-26: six controls over the documentation benchmark, written
  by an agent that had just finished writing this rule down, **two of them
  blind** — caught in 2.3 s by `scripts/docbench.py selftest`, and by nothing
  else, since both passed the positive control and would have passed it with
  the documentation they guard deleted. As prose the same check runs 1–3k
  tokens a time and its own injection can silently match nothing, which reads
  as a pass.
- **A green suite does not prove a test ran.** Deleting an `#[ignore]` took
  the `#[test]` above it with it; the test compiled, was never collected, and
  the suite stayed green. Clippy's dead-code warning caught it, not the tests.
- **A green suite does not prove a test *could* fail, and that is a different
  claim from the one above.** The sibling of the `#[ignore]` case: those tests
  never ran, these run and pass and could not have done otherwise. A genome
  widening shifted one draw out of a shared `Rng` that the *caller* went on
  using, and both guards over it stayed green through the regression —
  measured, by putting the fault back. One hashed a grown stand, which is
  insensitive to a single reordered draw; the other built a fresh `Rng` per
  call to model production, so it never observed a caller that continues.
  Green was evidence about the tests, not about the code, and it was used to
  withdraw a correct finding. The same session found a `let generation = state.generation;`
  shadowing the parent's `generation` — every bred child pinned at generation 1
  for ever, silently flattening lineage depth — and it was caught only because
  one guard hashed enough state to notice.
- **A *red* suite proves even less: `cargo test` stops at the first failing
  test binary, so a known-red lib test hides every integration test from a
  local run.** Bug A lives in the lib target, so plain `cargo test` fails
  there and never runs `tests/worldgen.rs` or `tests/determinism.rs` — they
  do not appear in the output at all, not even as skipped. That asymmetry hid
  **two gating failures on `main` for a whole day**
  (`Reports/open-bugs-handoff.md` §M). **The specific instance is closed and
  the general rule is not.** Bug A's test is now `#[ignore]`d, so it no longer
  runs and no longer blocks anything: measured 2026-08-26, `cargo test --lib`
  with no flag gives **943 passed / 0 failed / 54 ignored**. The `--skip
  root_and_shoot_branching_read_different_slots` this bullet and the Commands
  section both insisted on is therefore vestigial, and CI still passes it
  harmlessly. What survives, and is the reason to keep this entry: **while any
  gate is quarantined, whatever runs after it is not being run locally** — so
  treat the *absence* of `Running tests/worldgen.rs` from the output as the
  tell, since it reads as a pass rather than an error.
- **You are probably measuring a binary that is not the code you wrote.**
  Four separate bullets below are one failure — the artifact under test is
  stale, and it happens on *every* route into it. **The shared tell is
  identical output across a change that must have moved something**: three
  bit-identical sweep runs, a lens-shape before/after that came back
  byte-for-byte equal, a `scale_probe` count unchanged by a change that had
  to move it. **The standing check is one line** — `cargo build --release
  --examples` with `set -o pipefail`, then confirm the output actually moved
  before believing any of it. Four occurrences to date, one of which bit a
  single session three times in an afternoon, and each produced another
  bullet rather than being caught by the last.
- **Editing an asset `.ron` does nothing until the next build.** Materials
  and species are compiled into the binary via `include_str!`; only the
  app's F5 reload reads the directory, and headless harnesses do not. A
  sweep that edits `tree.ron` and re-runs a prebuilt example produces
  bit-identical "runs" — three of them, once, before anyone noticed the
  knob was not connected. Identical output across settings is the tell;
  rebuild between sweep points.
- **`cargo build --release` does not rebuild the examples**, and every
  measurement in this repo comes out of an example. It builds the lib and the
  bin; `--examples` builds them all, `--example NAME` builds one — and a
  stale `target/release/examples/foo` runs happily, prints plausible numbers,
  and has a *newer mtime than the source you just edited*, so the obvious
  sanity check says it is fresh. This bit one session three times in an
  afternoon: a `viewshot` render that showed round-7 formations after the
  4x-formation change had landed; a lens-shape before/after that came back
  **byte-identical** because only the "before" had been rebuilt; and a
  `scale_probe` cell count identical to the pre-change one for the same
  reason. The tell is the same one the `include_str!` gotcha above has:
  **identical output across a change that must have moved something.** When
  you see it, suspect the binary before the code — and prefer
  `cargo build --release --examples` over naming one, because the pass you
  are about to measure with is rarely the only example you will run.

- **Piping `cargo` into `tail`/`grep` throws away its exit code, and the
  build-all above is exactly where that bites.** `cargo build --release
  --examples 2>&1 | tail -25` reports `tail`'s status, which is 0 whatever
  cargo did. Measured 2026-08-24: a background build reported success while
  it had actually *failed* — and a failed `--examples` build aborts the
  remaining examples, so **4 of the 25 binaries existed** and the next
  measurement ran against a missing one. This is the previous bullet's
  failure with the tell removed: there is no stale output to notice, because
  there is no binary at all. Use `set -o pipefail` and read
  `${PIPESTATUS[0]}`, and never trust a bare `echo $?` after a pipe.

- **A `cargo` flag can be a performance change, and the obvious half may be
  the worthless half.** There was no `[profile.release]` in `Cargo.toml` at
  all until 2026-08-24 — every release build ran without LTO at
  `codegen-units = 16`. Adding it is worth ~4% of the frame, but the split
  is the lesson: `lto = "thin"` **alone measured no gain at all** (10.58 ms
  against a 9.84 ms baseline), and the entire win is `codegen-units = 1`,
  which is also the whole of the +50% build-time cost. Measure the settings
  separately before attributing a win to the one whose name sounds like it
  did the work.

- **The harness is as stale-able as the assets it reads, and an unknown
  argument is silently ignored.** A 3.5-hour detached megastudy (3 species x
  8 world seeds x 16 plants x 45,000 frames) produced eight *byte-identical*
  logs per species: the release binary was built fourteen minutes before
  `worldseed=` was added to `plant_probe.rs`, so every run took the default
  seed and the study was 3 populations wearing 24 logs. It looked exactly
  like a study. **Rebuild before launching anything long, and make the
  harness echo its own parameters** — `plant_probe`'s first line now names
  species/trees/frames/worldseed, so a log that does not name its seed was
  written by a binary that never had one. A knob nobody can see the value of
  is a knob nobody can tell is disconnected.
- **Do not add `schedule_structural_check_around` to an organism growth
  path.** Growth only ever *adds* material, so it is not a disturbance, and
  a `GrowingTip` is expected to be transiently unsupported until it
  reconnects — checking it there prunes ordinary growth as if it were
  damage (`plant.rs`'s `Grow` and germination both say so at the call
  site). The historical reason was different and is now **stale**: the
  hop-bounded `organism_is_supported` that amputated crowns no longer
  exists, replaced by `plant::anchor_support`, a Dijkstra from the anchors
  outward with no span budget. `open-bugs-handoff.md` §0d has that story
  and the 26x measurement; read it before trusting any Phase 3 damage
  result written while the old search was live.
- **Assert the property, not two instants fitted to one trajectory.**
  `a_tree_eventually_stops_growing` compared wood counts at two fixed
  frames and broke the moment genotypes were re-keyed — the tree at that
  spot became a different individual that was simply still growing at the
  first sample. A termination claim is "the count holds still across N
  consecutive windows inside a budget set from a measured curve"; it
  survives redraws and retunes because it asks the question the test is
  named for.
- The liquid heightfield bodies in `liquid.rs` are **test-only today** —
  nothing in production promotes a body, so bugs there are latent, not
  live, and go live the moment promotion lands. Why promotion was
  implemented and reverted is in `liquid.rs`'s own module doc and
  `Reports/liquid-heightfield-design.md`.
- **Grepping a prose phrase gives false negatives, and a false negative here
  reads as "the content is gone".** Two causes, both structural rather than
  careless. **The prose is hard-wrapped** at a median 72-73 characters, and
  `grep` is line-based, so a phrase that straddles a wrap can never match:
  measured 2026-08-26, **750 of 3,233 bolded phrases across `CLAUDE.md`,
  `README.md`, `PLAN.md` and the two registers span a line break — 23%, very
  nearly one in four.** And the house style puts `**bold**` and `` `code` ``
  *inside* sentences, so a phrase quoted the way it reads does not match the
  way it is stored. Both were hit in one session: a post-merge check reported
  two lanes' work missing when it was present and intact, which nearly became
  a report that the merge had dropped it. **Use
  `python3 scripts/docgrep.py "the phrase as it reads"`**, which normalises
  both sides and prints `file:line`; with no file arguments it searches the
  documents agents are routed to, and it exits 1 on no match so it can be used
  in a conditional. A short unique token (`rot_remains`,
  `max_unsupported_span`) greps fine; a sentence does not. This is a tool
  rather than a rule deliberately -- the rule that stood here first asked the
  reader to strip the markup and collapse the whitespace by hand, mid-task,
  which is exactly the discipline this file's own recurrence audit found does
  not survive a real session.
- **A coarse-field read is block-nearest, so neighbouring cells sample the
  same value — never build a per-cell decision on the difference between
  two of them.** At `FIELD_SCALE`, four sensors one cell apart land in the
  same field block roughly seven times in eight, so their differences are
  zero and whatever tie-break follows becomes a constant direction. **Hit
  four times, on three different lines, and never once caught by a test:**
  worm thermotaxis resolved to "always flee west"; tree phototropism
  reproduced the identical degeneracy; a third proposal for per-candidate
  self-avoidance was stopped only by a reviewer noticing the pattern; and
  it stands recorded as a live trap for the first liquid code to read
  pressure per cell. If a rule needs a *gradient*, interpolate or sample
  far enough apart to cross a block boundary — and prove the two reads can
  actually differ before trusting the sign.
- **A channel needs a writer and a reader, and the compiler checks neither.**
  A field that is written and never read is dead weight; one that is read
  and never written is worse, because **every consumer of it is dead code
  that looks alive** — the reads compile, the values are plausible, and the
  behaviour they drive silently does not exist. `Reports/dead-ends.md` calls
  this "the failure mode this project has hit three times": light with no
  writer, canopy density with an always-zero reader, pressure with no liquid
  consumer. It is a standing check, not three individual fixes — when you
  add or inherit a per-cell or per-tile channel, name its writer and its
  reader out loud before building on it, and if either is missing say so
  rather than assuming the other end is somewhere you have not looked.
