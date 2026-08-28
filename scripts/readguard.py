#!/usr/bin/env python3
"""PreToolUse guard: refuse a whole-file read of a document that has an index.

Why this exists
---------------
Measured 2026-08-27, decomposing a real agent run (docbench's 95,518
`agent_tokens`): **74% of what an agent spends is reading**, against 26% for the
auto-loaded prefix. The five documents `CLAUDE.md` routes sessions into cost
this much to read whole:

    Reports/open-bugs-handoff.md   ~107,743 tokens
    Reports/dead-ends.md           ~102,378
    PLAN.md                         ~60,200
    PLAN-log.md                     ~48,282
    README.md                       ~46,833

Every one of them already carries a "do not read it whole" warning, and every
one of those warnings is a **convention**. This repo's own record on conventions
is `.claude/README.md`'s account of the SessionStart hook: `CLAUDE.md` asked
every session to run `branchcheck.sh` and ten branches still sat at exactly 160
commits behind. A check catches what a convention does not.

One unguided read of `open-bugs-handoff.md` costs more than relocating the whole
of `CLAUDE.md` across a seven-agent fan-out. This is the largest single lever in
the repo and it is one `if` statement.

What it does and does not block
-------------------------------
It denies a `Read` with **no `offset`/`limit`** on a listed file. A sliced read
passes untouched, because slicing is exactly the behaviour the warnings ask for
and the indexes exist to aim. The denial names the index to use instead, so the
agent is redirected rather than merely stopped -- a guard that blocks without
saying where to go next gets worked around, and a worked-around guard is worse
than none because it also costs a turn.

**It is deliberately not a `deny` permission rule.** A permission rule cannot
read `offset`, so it could only ban the file outright, and these documents must
stay readable -- they are where the answers are.

Fail-open, on purpose
---------------------
Any error -- unparseable stdin, a path shape not seen before -- exits 0 and
allows the read. A guard over the *cost* of a correct action must never be able
to block the action itself: the failure mode of a false positive here is an
agent that cannot reach the bug register, which is far worse than a large read.
"""

import json
import os
import sys

# The repo this guard belongs to. `CLAUDE_PROJECT_DIR` is what the hook runs
# under; the script's own location is the fallback for a direct invocation.
ROOT = os.environ.get("CLAUDE_PROJECT_DIR") or os.path.dirname(
    os.path.dirname(os.path.abspath(__file__)))

# A slice larger than this is a whole-file read wearing a parameter. Set above
# any real navigation slice (the largest index block in the corpus is
# open-bugs-handoff.md's status table, ~120 lines) and far below every guarded
# file's line count.
MAX_SLICE_LINES = 600

# rel path -> how to read it instead. The pointer is the whole value of the
# denial; keep each one specific enough to act on without another lookup.
GUARDED = {
    "Reports/open-bugs-handoff.md": (
        "~107,743 tokens. Read the generated status index at the TOP of the file "
        "first (`sed -n '1,120p'`), then only the sections it lists for your area."
    ),
    "Reports/dead-ends.md": (
        "~102,378 tokens. Never read it whole -- grep the MECHANISM you are about "
        "to touch or propose, not your subsystem. `thicken` returns ~2,460 tokens; "
        "grepping an area costs ~12k-31k, more than this file. For a survey, grep "
        "the address prefix: `^- \\*\\*.\\?src/sim/plant`."
    ),
    "PLAN.md": (
        "~60,200 tokens. Start from its Contents, and in a session-handoff section "
        "read the dated *(State ...)* line rather than the heading."
    ),
    "PLAN-log.md": "~48,282 tokens. Append-only progress log -- grep it, do not read it.",
    "README.md": (
        "~46,833 tokens. Its **By topic** table maps subsystem to owning sections "
        "with line numbers; milestone sections are named for the BUILD, not the "
        "subsystem (`M17 status` is the structural-collapse write-up)."
    ),
}


def match(path, root=None):
    """The guarded doc this path names, or None.

    Anchored to the repo root, and it has to be. Two weaker rules both ship-
    broke here:

    * `path.endswith(rel)` -- what shipped. `"README.md"` matched every README
      in the repo: `wiki/README.md` (2,362 B), `.claude/README.md`,
      `Reports/lanes/README.md` and `Reports/README.md` were all denied with the
      root README's 46,833-token message and its **By topic** pointer, which none
      of them has. Those four are the index documents CLAUDE.md routes agents TO,
      so the guard was densest exactly where it was most wrong.
    * `path.endswith("/" + rel)` -- the obvious repair, and still wrong for the
      identical reason: `.../wiki/README.md` ends with `/README.md`. Caught by
      the selftest below on its first run, which is the argument for having one.

    Only a root-relative comparison separates `<root>/README.md` from
    `<root>/wiki/README.md`. Outside the repo, nothing is guarded.

    Fail-open does not cover any of this: a confidently-wrong DENY is not an
    error, so it never reaches the exception handler.
    """
    root = (root or ROOT).replace("\\", "/").rstrip("/")
    if not path.startswith(root + "/"):
        return None
    rel = path[len(root) + 1:]
    return rel if rel in GUARDED else None


def selftest():
    """Positive and negative controls over `match`, in milliseconds.

    Exists because the endswith bug was invisible to every other check: the hook
    fires only inside a live session, `docscheck` never runs it, and a false
    DENY looks like the guard working. Drives the real predicate -- a selftest
    that re-implements the rule inline proves only that the rule is expressible
    (see scripts/lanecheck.py, which shipped exactly that mistake).
    """
    R = "/home/u/repo"
    cases = [
        # (path, expected rel or None)
        (f"{R}/README.md", "README.md"),
        (f"{R}/PLAN.md", "PLAN.md"),
        (f"{R}/PLAN-log.md", "PLAN-log.md"),
        (f"{R}/Reports/open-bugs-handoff.md", "Reports/open-bugs-handoff.md"),
        (f"{R}/Reports/dead-ends.md", "Reports/dead-ends.md"),
        # the regression: index documents that merely END in a guarded name.
        # Every one of these is a file CLAUDE.md routes agents to.
        (f"{R}/wiki/README.md", None),
        (f"{R}/.claude/README.md", None),
        (f"{R}/Reports/README.md", None),
        (f"{R}/Reports/lanes/README.md", None),
        (f"{R}/docs/screenshots/plant-v2-baseline/README.md", None),
        # not a guarded doc at all
        (f"{R}/Reports/agent-strategy.md", None),
        (f"{R}/src/sim/world.rs", None),
        # outside the repo entirely
        ("/somewhere/else/PLAN.md", None),
    ]
    bad = 0
    for path, want in cases:
        got = match(path, root=R)
        if got != want:
            print(f"readguard: FAILED {path}: match -> {got!r}, want {want!r}")
            bad = 1
    if not bad:
        print(f"readguard: {len(cases)} path controls pass "
              "-- guarded docs match, every other README does not")

    # The size gate: a sliced read must pass and a whole read must deny.
    whole = decide({"tool_name": "Read", "tool_input": {"file_path": f"{R}/PLAN.md"}}, root=R)
    sliced = decide({"tool_name": "Read",
                     "tool_input": {"file_path": f"{R}/PLAN.md", "offset": 1, "limit": 50}}, root=R)
    huge = decide({"tool_name": "Read",
                   "tool_input": {"file_path": f"{R}/PLAN.md", "limit": 10 ** 9}}, root=R)
    if whole is None:
        print("readguard: POSITIVE CONTROL FAILED -- a whole-file read was allowed"); bad = 1
    if sliced is not None:
        print("readguard: NEGATIVE CONTROL FAILED -- a sliced read was denied"); bad = 1
    if huge is None:
        print("readguard: FAILED -- limit=1e9 reads the whole file and must deny"); bad = 1
    if not bad:
        print("readguard: whole denied, sliced allowed, oversized slice denied")
    return bad


def decide(payload, root=None):
    """The denial reason for this tool call, or None to allow. Pure; no I/O."""
    if payload.get("tool_name") != "Read":
        return None
    ti = payload.get("tool_input") or {}

    path = (ti.get("file_path") or "").replace("\\", "/")
    hit = match(path, root=root)
    if hit is None:
        return None

    # A sliced read is the behaviour the warnings ask for. Let it through --
    # but a slice big enough to swallow the file is a whole-file read wearing a
    # parameter, and the denial text below names offset/limit as the way out,
    # so the bypass was the guard's own suggestion. Cap it at the largest slice
    # that is still navigation rather than ingestion.
    offset, limit = ti.get("offset"), ti.get("limit")
    if offset is not None or limit is not None:
        if limit is None or limit <= MAX_SLICE_LINES:
            return None

    return (
        f"readguard: {hit} is {GUARDED[hit]}\n"
        "Re-issue the Read with offset/limit for the part you need, or "
        "grep it. This guard only blocks the WHOLE-file read; a sliced "
        f"read of up to {MAX_SLICE_LINES:,} lines passes. Reading is ~74% of an "
        "agent's token spend here (measured), which is why this is a check "
        "and not a convention."
    )


def main():
    if "--selftest" in sys.argv:
        return selftest()

    try:
        payload = json.load(sys.stdin)
    except Exception:
        return 0  # fail open -- see the module docstring

    try:
        reason = decide(payload)
        if reason is None:
            return 0
        print(json.dumps({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": reason,
            }
        }))
    except Exception:
        return 0  # fail open
    return 0


if __name__ == "__main__":
    sys.exit(main())
