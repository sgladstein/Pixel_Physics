#!/usr/bin/env python3
"""Regenerate the contents table at the top of the long `Reports/*.md`.

Why this exists
---------------
Measured 2026-08-27: plants are **42 of this directory's 110 reports and about
269,000 tokens**, and **seven of those reports carry 115,000 of it -- 43% in
seven files**. `plant-substrate-v2-design.md` alone is ~29,500 tokens and is
cited from 28 source comments, so it is opened often and read whole never.

**None of the 42 had a table of contents.** An agent that wants one fact from a
29,000-token report has three options: read it whole (a fifth of a working
context for one fact), grep it (which false-negatives on this prose -- 23% of
bolded phrases straddle a hard wrap, see `scripts/docgrep.py`), or skip it and
re-derive. All three are worse than a table saying which section to `sed`.

This is the same trade `Reports/documentation-overhaul-plan.md` item 11 made
for `README.md` -- a wholesale reorder was refused and "a TOC buys the same
navigation for 3% of the churn". `scripts/readmetoc.py` is that TOC and this
is its sibling for the reports; the two share their mechanics deliberately.

What is in the table, and why the line numbers
----------------------------------------------
Three columns: **section**, **line**, **~tokens**. The anchor is for a human
reading on GitHub; the line number is for an agent about to run
`sed -n '212,309p'`; the token column is the one that changes behaviour,
because it turns "read the report" into a priced decision. `~tokens` is
`len(section) // 4` -- an estimate, labelled as one, and never used for
anything but ranking.

Membership is EXPLICIT, not by size
-----------------------------------
`MANAGED` below is a written-down list, which is the same reversal
`readmetoc.py` records for its own topic index: membership is editorial, so it
is data rather than a threshold. Two concrete reasons here:

* `dead-ends.md` and `open-bugs-handoff.md` are the two largest documents in
  the directory and must NOT get one -- they already carry generated indexes
  of their own (`scripts/bugindex.py`), and a second index would be a second
  thing to keep current.
* This tree is worked concurrently. A size threshold would splice a table into
  another lane's report the moment it grew past the bar, mid-session, in a
  file that lane is holding uncommitted. `--candidates` reports what is over
  the bar and leaves the decision to a person.

`--check` verifies every managed report's table is current, and that every
managed path still exists -- so deleting or renaming a report fails
`scripts/docscheck.sh` rather than leaving a dead entry here.

Shares two hard-won mechanics with `readmetoc.py` and `bugindex.py`: the line
numbers cite a document the table is *inside*, so generation iterates to a
fixpoint; and the block is **derived**, so a merge conflict inside it is never
hand-merged -- take either side whole and re-run.

Regenerate with `python3 scripts/reporttoc.py`; `--check` is what
`scripts/docscheck.sh` runs, `--candidates` lists unmanaged reports over the
size bar.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REPORTS = ROOT / "Reports"

BEGIN = "<!-- BEGIN GENERATED TOC -- regenerate with scripts/reporttoc.py -->"
END = "<!-- END GENERATED TOC -->"

# Roughly 10,000 tokens. Only used to *suggest* candidates; nothing is added
# to MANAGED automatically.
BAR_CHARS = 40_000

# The reports that carry a generated contents table. Seeded 2026-08-27 with
# the seven plant reports over the bar -- 43% of the plant corpus by tokens.
# Add a path here after deciding it is worth the churn in a file others may be
# holding; `--candidates` tells you what is eligible.
MANAGED = [
    # Plants -- the seven over the bar, 43% of the plant corpus by tokens.
    "plant-substrate-v2-design.md",
    "plant-genome-design.md",
    "physical-trees-design-2026-08-23.md",
    "plant-evolution-design.md",
    "tree-architecture-variety-review.md",
    "grass-sowing-and-divergence-2026-08-23.md",
    "plant-economy-rederivation-2026-08-23.md",
    # Everything else that was over the bar on 2026-08-27, added on the owner's
    # call after the plant seven proved out. Every one carries 6-17 `##`
    # sections, so a table is a real read unit rather than a formality -- the
    # thinnest is prior-art-destruction at 6 sections over ~10,800 tokens.
    # Checked before adding: neither open PR (#80, #12) touches any of them.
    "worldgen-implementation-tasks-2026-08.md",
    "creature-evolution-plan.md",
    "creature-direction.md",
    "explosion-stone-review.md",
    "liquid-heightfield-design.md",
    "prior-art-worldgen-slicing.md",
    "next-session-handoff.md",
    "agent-documentation-audit-2026-08-24.md",
    "worldgen-design.md",
    "emergent-world-architecture.md",
    "structural-support-model.md",
    "world-review-2026-08.md",
    "liquid-simulation-research.md",
    "prior-art-destruction.md",
]

# Never eligible: they carry their own generated index already.
EXCLUDED = {"dead-ends.md", "open-bugs-handoff.md", "README.md"}


def anchor(title):
    """GitHub's heading-anchor rule, character-identical to `readmetoc.py`'s.

    Lowercase, drop everything that is not `[a-z0-9 -]`, spaces to hyphens.
    **Each space becomes its own hyphen** -- an em dash surrounded by spaces
    therefore yields a DOUBLE hyphen, which is correct and looks like a typo.
    The first version here collapsed whitespace runs with `\\s+`, which is the
    intuitive reading and produces a link that 404s on every one of these
    headings: they are full of ` — `, ` → ` and backticks. `readmetoc.py` had
    already found and documented this; copied rather than re-derived."""
    a = title.lower()
    a = re.sub(r"[^a-z0-9 \-]", "", a)
    return a.replace(" ", "-")


def sections(lines):
    """Every `## ` heading: (title, 1-based line, token estimate of its body).

    `##` only. These reports run 5-16 top-level sections, which averages ~2,700
    tokens a section on the largest -- a sensible read unit. Going to `###`
    would produce a table long enough to need its own table."""
    heads = [(i, ln[3:].strip()) for i, ln in enumerate(lines) if ln.startswith("## ")]
    out = []
    for k, (i, title) in enumerate(heads):
        end = heads[k + 1][0] if k + 1 < len(heads) else len(lines)
        body = "\n".join(lines[i:end])
        out.append((title, i + 1, len(body) // 4))
    return out


def render(secs, total_tokens):
    rows = [
        BEGIN,
        "",
        f"**Contents** — {len(secs)} sections, ~{total_tokens:,} tokens. Read a "
        "section, not the file: the line number is there so you can "
        "`sed -n 'START,ENDp'` it, and the token column is there so that is a "
        "priced decision rather than a guess. Regenerated by "
        "`scripts/reporttoc.py`; a merge conflict inside this block is never "
        "hand-merged — take either side whole and re-run.",
        "",
        "| Section | Line | ~tokens |",
        "|---|---|---|",
    ]
    for title, line, tok in secs:
        # Pipes inside a heading would break the table; none exist today, and
        # escaping keeps that true if one appears.
        safe = title.replace("|", "\\|")
        rows.append(f"| [{safe}](#{anchor(title)}) | {line} | {tok:,} |")
    rows += ["", END]
    return "\n".join(rows)


def splice(text, block):
    """Replace an existing block, or insert one just before the first `## `.

    Before the first section, not at the very top: the `**Status:**` header is
    the thing a reader must see first, and burying it under a table is how a
    stale-standing header goes unread -- the exact defect `docscheck` 3b/3c
    exist for."""
    if BEGIN in text and END in text:
        head, rest = text.split(BEGIN, 1)
        _, tail = rest.split(END, 1)
        return head + block + tail
    lines = text.split("\n")
    for i, ln in enumerate(lines):
        if ln.startswith("## "):
            return "\n".join(lines[:i] + [block, ""] + lines[i:])
    raise SystemExit(f"reporttoc: no `## ` heading to anchor against")


def build(text):
    """Iterate to a fixpoint: the table cites line numbers in the document it
    is inside, so splicing it shifts every number after it."""
    updated = text
    for _ in range(6):
        lines = updated.split("\n")
        secs = sections(lines)
        # Total excludes the generated block itself, so it does not inflate as
        # the table grows.
        gross = len(updated) // 4
        candidate = splice(updated, render(secs, gross))
        if candidate == updated:
            return candidate
        updated = candidate
    raise SystemExit("reporttoc: did not converge -- line numbering is unstable")


def candidates():
    over = []
    for f in sorted(REPORTS.glob("*.md")):
        if f.name in EXCLUDED or f.name in MANAGED:
            continue
        n = f.stat().st_size
        if n >= BAR_CHARS:
            over.append((n // 4, f.name))
    return sorted(over, reverse=True)


def main():
    args = set(sys.argv[1:])
    unknown = args - {"--check", "--candidates"}
    if unknown:
        print(f"reporttoc: unrecognised argument(s): {' '.join(sorted(unknown))}")
        print("usage: reporttoc.py [--check | --candidates]")
        return 2

    if "--candidates" in args:
        over = candidates()
        if not over:
            print("reporttoc: no unmanaged report is over the bar")
            return 0
        print(f"reporttoc: {len(over)} unmanaged report(s) over ~{BAR_CHARS // 4:,} "
              f"tokens -- add to MANAGED only if the churn is worth it:")
        for tok, name in over:
            print(f"  ~{tok:>7,} tok  {name}")
        return 0

    missing = [m for m in MANAGED if not (REPORTS / m).exists()]
    if missing:
        for m in missing:
            print(f"reporttoc: MANAGED lists {m}, which does not exist -- "
                  f"renamed or deleted; fix the list in scripts/reporttoc.py")
        return 1

    stale, wrote = [], 0
    for name in MANAGED:
        path = REPORTS / name
        with open(path, encoding="utf-8", newline="") as fh:
            text = fh.read()
        updated = build(text)
        if "--check" in args:
            if updated != text:
                stale.append(name)
            continue
        if updated != text:
            with open(path, "w", encoding="utf-8", newline="\n") as fh:
                fh.write(updated)
            wrote += 1

    if "--check" in args:
        for name in stale:
            print(f"reporttoc: {name} contents table is stale -- "
                  f"run `python3 scripts/reporttoc.py`")
        if stale:
            return 1
        print(f"reporttoc: all {len(MANAGED)} report contents tables current")
        return 0

    print(f"reporttoc: wrote {wrote} of {len(MANAGED)} report contents tables")
    return 0


if __name__ == "__main__":
    sys.exit(main())
