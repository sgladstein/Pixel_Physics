#!/usr/bin/env python3
"""Search documentation for a phrase the way it *reads*, not the way it is stored.

Why this exists
---------------
`grep` is line-based and this repo's prose is hard-wrapped at a median 72-73
characters, so **a phrase that straddles a wrap can never match**. Measured
2026-08-26: **750 of 3,233 bolded phrases** across `CLAUDE.md`, `README.md`,
`PLAN.md` and the two registers span a line break -- 23%, nearly one in four.
The house style also puts `**bold**` and `` `code` `` *inside* sentences, so a
phrase quoted the way it reads does not match the way it is stored.

**The failure direction is what makes this worth a tool rather than a rule.** A
false negative reads as *"the content is gone"*. Hit twice in one session: a
post-merge check reported two lanes' work missing when it was present and
intact, and that nearly became a report that the merge had dropped it -- which
would have prompted re-adding content already there.

`CLAUDE.md` carried a rule about this first. The rule asks the reader to strip
the markup and collapse the whitespace by hand, mid-task, which is precisely
the kind of discipline this repo's own recurrence audit found does not survive
contact with a real session: seven rules existed and the mistake recurred
anyway. A command is cheaper than the discipline it replaces.

Normalisation is `addrcheck.normalise` -- deliberately shared, not
reimplemented, so the two agree by construction. Every rule in it was added
because a *correct* match failed without it, never speculatively.

Usage
-----
    python3 scripts/docgrep.py "a phrase the way it reads"
    python3 scripts/docgrep.py "phrase" CLAUDE.md README.md

With no files, searches the documents agents are routed to. Prints
`file:line` for each hit, with the original (un-normalised) text, and exits 1
when there are none -- so it can be used in a conditional.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from addrcheck import normalise  # noqa: E402  -- shared on purpose

ROOT = Path(__file__).resolve().parent.parent
DEFAULT = [
    "CLAUDE.md",
    "README.md",
    "PLAN.md",
    "PLAN-log.md",
    "Reports/dead-ends.md",
    "Reports/open-bugs-handoff.md",
    "Reports/README.md",
]


def hits(path, needle):
    """Yield (line_number, original_line) for each normalised match.

    The whole file is normalised as one string so a match may span any number
    of source lines; a parallel index maps each normalised character back to
    the line it came from, which is what lets the result cite a real line.
    """
    raw = path.read_text(encoding="utf-8", errors="ignore")
    lines = raw.split("\n")
    flat, origin = [], []
    for n, line in enumerate(lines, 1):
        piece = normalise(line)
        if not piece:
            continue
        if flat:
            flat.append(" ")
            origin.append(n)
        flat.append(piece)
        origin.extend([n] * len(piece))
    hay = "".join(flat)

    at = 0
    while True:
        at = hay.find(needle, at)
        if at < 0:
            return
        yield origin[at], lines[origin[at] - 1]
        at += 1


def main():
    args = [a for a in sys.argv[1:]]
    if not args or args[0] in ("-h", "--help"):
        print(__doc__.split("Usage\n-----\n")[1].strip())
        return 2
    needle = normalise(args[0])
    if not needle:
        print("docgrep: the phrase is empty once normalised")
        return 2

    targets = args[1:] or DEFAULT
    total = 0
    for t in targets:
        p = ROOT / t
        if not p.exists():
            p = Path(t)
        if not p.exists():
            print(f"docgrep: no such file: {t}")
            continue
        for n, line in hits(p, needle):
            total += 1
            print(f"{p.relative_to(ROOT) if ROOT in p.parents or p.parent == ROOT else p}:{n}: {line.strip()}")

    if not total:
        print(f"docgrep: no match for {args[0]!r} in {len(targets)} file(s)")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
