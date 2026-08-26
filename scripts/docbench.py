#!/usr/bin/env python3
"""The cold-agent documentation benchmark: canonical prompt, and the run record.

What it measures, and why that changed
--------------------------------------
The benchmark asks a fresh agent three questions answerable only from the
markdown, one of which is a **trap**: a report that reads as a live work order
and must be refused. Correctness and the trap are pass/fail. The interesting
number is what it *cost* to get there.

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
    python3 scripts/docbench.py prompt     # the canonical prompt, verbatim
    python3 scripts/docbench.py runs       # the run history
    python3 scripts/docbench.py check      # verify each question still has an answer

`check` is the positive control, and it is not optional: a question whose
answer has since left the corpus would score the *instrument*, not the
documentation, and a low run would be misread as a regression.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
KEY = ROOT / ".claude" / "workflows" / "doc-audit-benchmark-key.json"

PROMPT = """You are a fresh agent in the repo at `/home/user/Pixel_Physics`, a Rust falling-sand physics engine. You know NOTHING about it beyond what its documentation tells you. Answer three questions using ONLY the markdown documentation (`README.md`, `CLAUDE.md`, `PLAN.md`, `PLAN-log.md`, `wiki/`, `Reports/`) — you may open source files ONLY to confirm a file exists, not to derive answers. For each answer, state which document(s) led you there and how many files you had to open to find it.

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

# Each question's answer must still exist. (question, what to look for, where)
CONTROLS = [
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


def _docgrep(phrase, *files):
    """Normalised search -- a raw grep gives false negatives on this prose."""
    cmd = [sys.executable, str(ROOT / "scripts" / "docgrep.py"), phrase, *files]
    return subprocess.run(cmd, capture_output=True, cwd=ROOT).returncode == 0


def main():
    what = sys.argv[1] if len(sys.argv) > 1 else ""

    if what == "prompt":
        print(PROMPT)
        return 0

    if what == "runs":
        d = json.loads(KEY.read_text())
        print(f"{'date':12s} {'correct':8s} {'trap':6s} {'agent_tokens':>13s} {'files':>6s}  tree")
        for r in d["runs"]:
            tok = r.get("agent_tokens")
            print(f"{r['date']:12s} {r['correct']:8s} {r['traps_refused']:6s} "
                  f"{(str(tok) if tok else '-- (pre-metric)'):>13s} {r['files_opened']:>6d}  {r['tree'][:44]}")
        print("\nagent_tokens is the headline; files_opened is colour, not a target.")
        return 0

    if what == "check":
        bad = [(q, why) for q, why, test in CONTROLS if not test()]
        for q, why in bad:
            print(f"docbench: {q} CONTROL FAILED -- {why}")
        if bad:
            print("  -> the benchmark cannot be trusted until these are restored or the")
            print("     question is rewritten. A run now would score the instrument.")
            return 1
        print(f"docbench: all {len(CONTROLS)} controls pass -- every question still has an answer")
        return 0

    print(__doc__.split("Usage\n-----\n")[1].strip())
    return 2


if __name__ == "__main__":
    sys.exit(main())
