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
import sys

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


def main():
    try:
        payload = json.load(sys.stdin)
    except Exception:
        return 0  # fail open -- see the module docstring

    try:
        if payload.get("tool_name") != "Read":
            return 0
        ti = payload.get("tool_input") or {}

        # A sliced read is the behaviour the warnings ask for. Let it through.
        if ti.get("offset") is not None or ti.get("limit") is not None:
            return 0

        path = (ti.get("file_path") or "").replace("\\", "/")
        hit = next((rel for rel in GUARDED if path.endswith(rel)), None)
        if hit is None:
            return 0

        print(json.dumps({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": (
                    f"readguard: {hit} is {GUARDED[hit]}\n"
                    "Re-issue the Read with offset/limit for the part you need, or "
                    "grep it. This guard only blocks the WHOLE-file read; a sliced "
                    "read passes. Reading is ~74% of an agent's token spend here "
                    "(measured), which is why this is a check and not a convention."
                ),
            }
        }))
    except Exception:
        return 0  # fail open
    return 0


if __name__ == "__main__":
    sys.exit(main())
