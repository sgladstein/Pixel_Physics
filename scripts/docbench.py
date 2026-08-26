#!/usr/bin/env python3
"""The cold-agent documentation benchmark: canonical prompts, and the run record.

Two question sets, and why there are two
----------------------------------------
**Set A** asks a fresh agent three questions answerable only from the markdown,
one of which is a **trap**: a report that reads as a live work order and must be
refused. Correctness and the trap are pass/fail; the interesting number is what
it *cost* to get there.

**Set A has scored 3/3 correct and 1/1 trap in every run it has ever had** --
including the 2026-08-26 baseline arm run against pre-audit `main`. A guard that
has never gone red cannot show that anything improved. That is this repo's own
rule (*"before trusting any guard, put the fault it is named for back and watch
it go red"*) applied to an instrument rather than to a test, and it was not
applied when set A was written: `check` below verifies **specificity** (every
question still has an answer) and nothing verified **sensitivity** (the score
can move at all).

**Set B is the sensitivity half.** Every question in it was verified, before it
was written, to have a *wrong or missing* answer on pre-audit `main`
(`2f5de1e`) and a right one on the tree that followed. It is a **regression**
set, not a general-capability set: it is aimed at specific documentation
defects that specific changes fixed, and a future change that fixes something
else will not move it. Read a set B score as "are these particular repairs
still holding", never as "is the documentation good".

**Set B was run 2026-08-26 and three of its four qualifications were wrong.**
Current tree 4/4, pre-audit tree **3/4** -- not the 4/4 against 1/4 that was
predicted. The error was in the method, and it is worth more than the result:
each question was qualified by checking the old **`CLAUDE.md`**, never the old
**corpus**. This documentation is redundant -- the same fact lives in a report,
an index and a wiki page -- so "`CLAUDE.md` did not say it" is not "the agent
cannot find it". What actually happened, per question, is in the table below.

The consequence generalises past set B: **correctness saturates on this corpus
whatever you ask.** Set A has been at 3/3 for four runs; set B, written
specifically to break, came back 3/4 on the tree it was built to fail. The
quantity that separated the arms was **cost -- 95,518 tokens against 135,580,
+42%, with 10 files opened against 14 and 23 tool calls against 29.** Read that
as the shape of the whole benchmark: the audit did not change what is
answerable, it changed what answering costs. Which is also why set A, a
correctness instrument, could never have shown it.

The old-tree answers as claimed when the questions were written, against what
the run actually produced:

===  ==================================================================
B1   CLAIMED: the ownership table had no plants row and asserted
     *"Everything that has actually collided here collided in
     `src/app.rs`"* -- measurably false (`app.rs` is 6th at 51 landings) --
     so an agent was told its files were uncontested and safe to hold.
     RAN: **partly discriminating.** No counts, correctly, since they do
     not exist there. But it never said "uncontested": it reached the same
     instruction through `plant-implementation-split-2026-08-23.md`
     (*"one of the two shared substrates ... strictly one session at a
     time"*) and `plant-work-split.md` (*"`plant.rs` is where everything
     collides"*), and caught the table's false claim against the split
     report's *"filmstrip.rs is the most-collided file in the repo"*.
     KEEP -- it separates "has numbers" from "has prose", which is the
     real difference, but it is not the pass/fail it was written as.
B2   CLAIMED: the ethos was framed *"Destroying something should feel like
     destroying it"*, so the rule was findable but there was **no
     precedent outside destruction to point at**.
     RAN: **does not discriminate.** The old tree produced three --
     `wiki/plants.md`'s *"gradual and it is graded"*, `dead-ends.md:754`
     (hard-threshold leaf shedding rejected in favour of graded, with
     numbers), and `rot_remains` senescence from `open-bugs-handoff.md`.
     The precedent was in the corpus; only `CLAUDE.md` lacked it, and only
     `CLAUDE.md` was checked. What the reframing bought is visible in the
     answer's own caveat -- *"framed entirely in destruction vocabulary, so
     you have to read it across to plants"* -- which is work, not failure.
B3   CLAIMED: the `dead-ends.md` row said *"grep your area"*; the question
     names no mechanism, so the agent must choose its own unit of search.
     RAN: **both arms answered well**, the baseline in more detail (line
     numbers). This is the cost question and it stays one; it is not a
     correctness question and should not be scored as one.
B4   CLAIMED: `fracture-mechanics-design.md` said *"Read that to build
     this"* of the superseded handoff, unwarned, and the index said
     nothing either -- a referral trap.
     RAN: **does not discriminate, and the claim about the index was
     false.** `Reports/README.md` already carried "superseded by landing"
     on the old tree, and the baseline arm refused the trap through it,
     calling it *"a clean case of the header being stale and the index
     being right"*. The audit moved the warning one hop earlier; it did
     not defuse an armed trap.
===  ==================================================================

**Do not read B2 or B4 as correctness gates.** They are retained as cost
probes and as the record of a wrong prediction. The lesson for anyone adding
a question here: qualify it against the **whole old corpus**, not against the
one file you changed -- and expect that a well-posed question is usually
answerable on any revision of this repo, so aim at cost.

What it measures, and why that changed
--------------------------------------

**Until 2026-08-26 the headline was `files_opened`, and that was the wrong
metric.** Measured that day: routing work landed across the week that
deliberately trades file-opens for narrower reads -- grep the mechanism rather
than the area, enter the bug register by its generated index rather than
reading it -- and `files_opened` came back **6, unchanged**, with tool calls
*up* 17 -> 23. A metric that cannot move when the thing it measures improves
is not measuring it. Two further defects in the old scheme:

* `files_opened` counts a 200-token grep and a 30,000-token read the same.
* `CLAUDE.md` is pre-loaded into a subagent's context by the harness, so it
  is invisible to a file count -- earlier runs that *read* it counted it and
  later runs that merely *used* it did not, which makes the series
  incomparable in a way nothing in the record admitted.

**The headline is now `agent_tokens`: the total the harness reports the
subagent consumed.** It is objective rather than self-reported, it absorbs the
pre-loaded `CLAUDE.md` instead of needing a correction, and it is denominated
in the thing routing actually changes. `files_opened` is kept as secondary
colour and is **not** the optimisation target -- do not tune for it.

**Runs before 2026-08-26 have no `agent_tokens` and cannot be back-filled**;
their transcripts are gone. The series restarts on the new metric, and the old
numbers stay in the record so the change is visible rather than quiet.

Usage
-----
    python3 scripts/docbench.py prompt [a|b]   # a canonical prompt, verbatim
    python3 scripts/docbench.py rubric [a|b]   # how to grade it
    python3 scripts/docbench.py runs           # the run history
    python3 scripts/docbench.py check          # verify every question still has an answer
    python3 scripts/docbench.py selftest       # ...and that the checks can go red

`check` is the positive control, and it is not optional: a question whose
answer has since left the corpus would score the *instrument*, not the
documentation, and a low run would be misread as a regression. It covers both
sets.

`rubric` exists because set A ran four times with **no written rubric** and was
graded by whoever ran it. Two people scoring the same transcript differently is
indistinguishable, in the record, from the documentation changing.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
KEY = ROOT / ".claude" / "workflows" / "doc-audit-benchmark-key.json"

PROMPT_A = """You are a fresh agent in the repo at `/home/user/Pixel_Physics`, a Rust falling-sand physics engine. You know NOTHING about it beyond what its documentation tells you. Answer three questions using ONLY the markdown documentation (`README.md`, `CLAUDE.md`, `PLAN.md`, `PLAN-log.md`, `wiki/`, `Reports/`) — you may open source files ONLY to confirm a file exists, not to derive answers. For each answer, state which document(s) led you there and how many files you had to open to find it.

1. Which source file owns the load/torque structural-failure criterion, and what should you read before touching that area?
2. Has "horizontal-before-vertical liquid fill transfer" (moving water sideways before letting it fall/compress) been tried in this engine? If so, what happened, and under what condition would it be worth re-testing?
3. Where would you find the current status of the tree-architecture work, and is the report `Reports/load-model-handoff.md` safe to execute as written?

Answer concisely. Search breadth: medium. Your final message is a report to another agent, not the user.

At the very end of your report, add a short METRICS block:

    METRICS
    files_opened: <count of distinct markdown files you READ>
    files_in_order: <those files, in the order you first read them>
    source_files_read: <count of source files read for content, not to confirm existence>
    tool_calls: <total>, reads: <n>, greps: <n>, listings: <n>

Count honestly, including any file you opened and found unhelpful."""

PROMPT_B = """You are a fresh agent in the repo at `/home/user/Pixel_Physics`, a Rust falling-sand physics engine. You know NOTHING about it beyond what its documentation tells you. Answer four questions using ONLY the markdown documentation (`README.md`, `CLAUDE.md`, `PLAN.md`, `PLAN-log.md`, `wiki/`, `Reports/`) — you may open source files ONLY to confirm a file exists, not to derive answers. For each answer, state which document(s) led you there.

1. You are starting a change to `src/sim/plant.rs` and `wiki/plants.md` that will take several sessions, and you expect to hold the diff until it is finished. How exposed are those two files to another session working at the same time? Give whatever numbers the documentation gives you, and say what it tells you to do about it.
2. You are working on plants. A change you are considering would make an unhealthy tree vanish in a single frame rather than decline. Does this project have a stated rule bearing on that? Quote it — and point to a precedent for it from a subsystem OTHER than destruction.
3. You are about to change how water levels out between adjacent columns, so that pools settle sooner. Before you write any code, do whatever this project requires you to do first, and report what you found.
4. `Reports/fracture-mechanics-design.md` names an implementation handoff document. Is that handoff safe to execute as written today, and what should you read instead, if anything?

Answer concisely. Search breadth: medium. Your final message is a report to another agent, not the user.

At the very end of your report, add a short METRICS block:

    METRICS
    files_opened: <count of distinct markdown files you READ>
    files_in_order: <those files, in the order you first read them>
    source_files_read: <count of source files read for content, not to confirm existence>
    tool_calls: <total>, reads: <n>, greps: <n>, listings: <n>

Count honestly, including any file you opened and found unhelpful."""

RUBRIC_A = """Set A -- 3 correctness points, 1 trap point.

Q1  PASS if it names `src/sim/load.rs` AND routes to at least one of the
    load/fracture reports or the dead-end register before touching it.
Q2  PASS if it reports the reversal was tried and reverted, and gives the
    condition under which re-testing is warranted. Naming the mechanism
    without the condition is a FAIL -- the condition is the whole value of
    a dead-end entry.
Q3  Two parts, and only the second scores the trap:
      correctness -- names where tree-architecture status lives;
      TRAP -- must answer NO, `load-model-handoff.md` is superseded by
      landing. Any answer that treats it as executable is a trap failure,
      including a hedged 'mostly safe'."""

RUBRIC_B = """Set B -- 4 points, one per question. Each was verified to fail on
pre-audit `main` (2f5de1e) before the question was written.

B1  PASS requires BOTH files identified as contested WITH the landing
    counts the table gives (plant.rs 60, wiki/plants.md 67), AND the
    instruction that follows from it -- land quickly, do not hold the diff.
    FAIL if it reports either file as uncontested, or gives the advice with
    no numbers. (Old tree: no plants row existed; answer was 'you're fine'.)
B2  COST PROBE, not a gate -- measured 2026-08-26 as non-discriminating
    (the old tree produced three non-destruction precedents from the wider
    corpus). Score it for (a) the graded-outcome law and (b) a precedent
    outside destruction, but record HOW FAR it had to go for (b): from
    `CLAUDE.md` directly, or reconstructed from `wiki/`, `dead-ends.md` and
    the bug register. The distance is the measurement.
B3  PASS if it consults the dead-end register BEFORE proposing anything and
    reports real liquid-levelling entries (MIN_LIQUID_TRANSFER's dead band,
    flow_rate not fixing wide-body levelling, the dispersion-search model).
    This is also the COST question -- record agent_tokens against it
    specifically if the harness reports per-question cost. It names no
    mechanism on purpose: the agent must choose its own unit of search.
B4  COST PROBE, not a gate -- measured 2026-08-26 as non-discriminating.
    Both trees refuse it; the old one via `Reports/README.md`, which
    already said "superseded by landing". PASS is still NO plus a redirect
    to `load-model-fit-review.md`; what to record is whether the refusal
    came from the report header (one hop) or from the index (two).

A run scores 4/4 on both trees more often than not. The separating number is
`agent_tokens`; see the module docstring."""

# Each question's answer must still exist. (question, what to look for, where)
CONTROLS_A = [
    ("Q1", "src/sim/load.rs must exist", lambda: (ROOT / "src/sim/load.rs").exists()),
    ("Q2", "the horizontal-before-vertical reversal must still be recorded",
     lambda: _docgrep("horizontal-before-vertical liquid ordering", "Reports/dead-ends.md")),
    ("Q3a", "the trap report must still exist",
     lambda: (ROOT / "Reports/load-model-handoff.md").exists()),
    ("Q3b", "the index must still mark it superseded",
     lambda: _docgrep("superseded by landing", "Reports/README.md")),
    ("Q3c", "its section 3 must still be a recorded dead end",
     lambda: _docgrep("Do not add the table back", "Reports/dead-ends.md")),
]

CONTROLS_B = [
    ("B1", "the ownership table must still give plant.rs and wiki/plants.md landing counts",
     lambda: _table_has_counts("src/sim/plant.rs", "wiki/plants.md")),
    ("B2", "the ethos section must still carry a non-destruction precedent",
     lambda: _ethos_cites("rot_remains")),
    ("B3a", "the dead-ends row must still say to grep the mechanism, not the subsystem",
     lambda: _docgrep("grep the mechanism you are about to touch or propose", "CLAUDE.md")),
    ("B3b", "liquid-levelling dead ends must still be on the register",
     lambda: _docgrep("MIN_LIQUID_TRANSFER", "Reports/dead-ends.md")),
    ("B4a", "the referring report must still carry the do-not-execute warning",
     lambda: _docgrep("SUPERSEDED BY LANDING, do not execute it",
                      "Reports/fracture-mechanics-design.md")),
    ("B4b", "it must still name the replacement to read instead",
     lambda: _docgrep("Reports/load-model-fit-review.md",
                      "Reports/fracture-mechanics-design.md")),
]


# `check` proves SPECIFICITY: every question still has an answer. It cannot
# prove SENSITIVITY: that a control would notice if the answer went away. These
# are the faults each control is named for, put back one at a time by
# `selftest`. Six injections, ~2 seconds -- which is the whole argument for
# doing this by command rather than by discipline.
#
# (control id, file, exact text to break, what to break it to)
FAULTS = [
    ("B1", "CLAUDE.md", "`src/sim/plant.rs` 60", "`src/sim/plant.rs`"),
    ("B2", "CLAUDE.md", "rot_remains", "rot_XXXXXXX"),
    ("B3a", "CLAUDE.md", "grep the *mechanism* you are about to touch or propose",
     "grep your area"),
    ("B3b", "Reports/dead-ends.md", "MIN_LIQUID_TRANSFER", "MIN_LIQUID_XXXXXXX"),
    ("B4a", "Reports/fracture-mechanics-design.md",
     "SUPERSEDED BY\nLANDING, do not execute it", "the implementation handoff"),
    ("B4b", "Reports/fracture-mechanics-design.md",
     "`Reports/load-model-fit-review.md`", "`Reports/somewhere-else.md`"),
]


def _selftest():
    """Put each fault back and confirm its control goes red.

    Found two blind controls on its first run, 2026-08-26, and neither was
    findable any other way -- both passed `check`, and both would have passed
    it with the documentation they guard deleted:

      * B2 searched the whole of CLAUDE.md for `rot_remains`, which also
        appears in the dead-ends row as an example of a grep returning
        nothing. Rescoped to the ethos section.
      * B3b's first injection replaced one of six occurrences, so the
        *injection* was blind rather than the control. Replace every
        occurrence, or a surviving copy hides the fault.

    Restores every file it touches, including on failure.
    """
    ok = True
    for cid, fname, needle, repl in FAULTS:
        f = ROOT / fname
        orig = f.read_text(encoding="utf-8")
        if needle not in orig:
            print(f"docbench: {cid} INJECTION FAILED -- '{needle[:40]}' not in {fname}")
            print("  -> the control may be fine; the fault this test injects has moved.")
            ok = False
            continue
        try:
            f.write_text(orig.replace(needle, repl), encoding="utf-8")
            r = subprocess.run([sys.executable, str(Path(__file__)), "check"],
                               capture_output=True, text=True, cwd=ROOT)
        finally:
            f.write_text(orig, encoding="utf-8")
        fired = [ln for ln in r.stdout.splitlines() if "CONTROL FAILED" in ln]
        if r.returncode == 1 and any(f"docbench: {cid} CONTROL" in ln for ln in fired):
            extra = f"  ({len(fired)} controls fired)" if len(fired) > 1 else ""
            print(f"docbench: {cid} went red{extra}")
        else:
            print(f"docbench: {cid} STAYED GREEN -- the control is blind, not weak.")
            print("  -> widening its assertion will not help. Replace it.")
            ok = False
    if ok:
        print(f"docbench: all {len(FAULTS)} faults detected -- the set B controls "
              f"can go red")
    return 0 if ok else 1


def _ethos_cites(token):
    """Scoped to the ethos section on purpose. `rot_remains` also appears in the
    dead-ends row as an example of a grep that returns *nothing*, so an
    unscoped search over CLAUDE.md passes with the precedent deleted -- caught
    by fault injection 2026-08-26, which is the only thing that would have."""
    text = (ROOT / "CLAUDE.md").read_text(encoding="utf-8")
    start = text.find("## The ethos")
    end = text.find("\n## ", start + 1) if start >= 0 else -1
    return start >= 0 and end > start and token in text[start:end]


def _table_has_counts(*files):
    """B1 needs numbers, not just a mention: a row naming the file with no
    landing count beside it is the old table's failure wearing a plants row."""
    text = (ROOT / "CLAUDE.md").read_text(encoding="utf-8")
    start = text.find("Know which files are yours")
    if start < 0:
        return False
    table = text[start:start + 4000]
    return all(re.search(re.escape(f) + r"`?\*{0,2}\s*\*{0,2}\d+", table) for f in files)


def _docgrep(phrase, *files):
    """Normalised search -- a raw grep gives false negatives on this prose."""
    cmd = [sys.executable, str(ROOT / "scripts" / "docgrep.py"), phrase, *files]
    return subprocess.run(cmd, capture_output=True, cwd=ROOT).returncode == 0


def main():
    what = sys.argv[1] if len(sys.argv) > 1 else ""

    which = (sys.argv[2] if len(sys.argv) > 2 else "a").lower()

    if what == "prompt":
        if which not in ("a", "b"):
            print("docbench: prompt takes 'a' or 'b'")
            return 2
        print(PROMPT_A if which == "a" else PROMPT_B)
        return 0

    if what == "rubric":
        if which not in ("a", "b"):
            print("docbench: rubric takes 'a' or 'b'")
            return 2
        print(RUBRIC_A if which == "a" else RUBRIC_B)
        return 0

    if what == "runs":
        d = json.loads(KEY.read_text())
        print(f"{'date':12s} {'set':4s} {'correct':8s} {'trap':6s} {'agent_tokens':>13s} "
              f"{'files':>6s}  tree")
        for r in d["runs"]:
            tok = r.get("agent_tokens")
            print(f"{r['date']:12s} {r.get('set', 'A'):4s} {r['correct']:8s} "
                  f"{r['traps_refused']:6s} "
                  f"{(str(tok) if tok else '-- (pre-metric)'):>13s} "
                  f"{r['files_opened']:>6d}  {r['tree'][:44]}")
        print("\nagent_tokens is the headline; files_opened is colour, not a target.")
        print("Correctness saturates on this corpus: set A has scored 3/3 in every run")
        print("ever taken, pre-audit included, and set B -- written specifically to")
        print("break on the old tree -- came back 4/4 against 3/4. Read both as floor")
        print("checks. The quantity that separates the arms is agent_tokens: 95,518")
        print("against 135,580, +42%, one run each. See the module docstring.")
        return 0

    if what == "selftest":
        return _selftest()

    if what == "check":
        controls = CONTROLS_A + CONTROLS_B
        bad = [(q, why) for q, why, test in controls if not test()]
        for q, why in bad:
            print(f"docbench: {q} CONTROL FAILED -- {why}")
        if bad:
            print("  -> the benchmark cannot be trusted until these are restored or the")
            print("     question is rewritten. A run now would score the instrument.")
            return 1
        print(f"docbench: all {len(controls)} controls pass "
              f"({len(CONTROLS_A)} set A, {len(CONTROLS_B)} set B) -- "
              f"every question still has an answer")
        return 0

    print(__doc__.split("Usage\n-----\n")[1].strip())
    return 2


if __name__ == "__main__":
    sys.exit(main())
