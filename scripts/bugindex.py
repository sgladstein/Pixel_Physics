#!/usr/bin/env python3
"""Regenerate the status index at the top of `Reports/open-bugs-handoff.md`.

Why this exists
---------------
`CLAUDE.md` tells every session to read the bug register "before touching a
listed area". The register is ~86k tokens across 93 entries, and 40% of the
entries under its `## Open` heading are headed FIXED/CLOSED/RESOLVED -- the
file is append-only and a bug's verdict is written into its own heading rather
than by moving it. So a reader who obeys the instruction cannot tell the live
half from the archive without reading all of it.

Moving the closed entries was the obvious fix and is the wrong one: the file is
co-owned by every lane, `union` merges do not apply to it, and reordering 5,000
lines turns any concurrent edit into a conflict. An index is additive, costs no
merge surface, and answers the question the reader actually has -- *which of
these is still live, and what line is it on* -- for a few hundred tokens
instead of eighty thousand.

Status is derived from the heading, never stored separately: a heading is the
one place the verdict is already written, so an index built from anything else
would be a second thing to keep true. Regenerate with `python3
scripts/bugindex.py`; `--check` verifies it is current and is what
`scripts/docscheck.sh` runs.
"""

import re
import sys
from pathlib import Path

DOC = Path(__file__).resolve().parent.parent / "Reports" / "open-bugs-handoff.md"
BEGIN = "<!-- BEGIN GENERATED INDEX -- regenerate with scripts/bugindex.py -->"
END = "<!-- END GENERATED INDEX -->"

# A verdict written into the heading. `~~strikethrough~~` is the file's other
# convention for the same thing and is treated identically.
CLOSED = re.compile(
    r"FIXED|CLOSED|RESOLVED|RETIRED|DUPLICATE|DOES NOT REPRODUCE|~~", re.I
)
# Not closed and not actionable either: parked on someone's judgement.
DECIDED = re.compile(
    r"DECISION CARD|SEQUENCING DECIDED|DESIGN DIRECTION|JUDGED", re.I
)

# The sections that hold the register proper. Everything else in the file is
# narrative -- landing notes, closed-out review batches -- whose `###` headings
# are enumerated sub-items, not bugs anyone references by identifier.
REGISTER_SECTIONS = {"Open", "Closed this session", "Awaiting a decision"}


def entries(lines):
    """Yield (ident, status, title, line, is_bug) for every `###` heading.

    `is_bug` is the section test, and it is load-bearing rather than cosmetic.
    The narrative sections -- `## Landing notes ...`, `## ~~Open~~ **CLOSED** --
    the three the polarity review raised` -- enumerate their findings `### 1.`,
    `### 2.`, `### 3.`, the same shape a bug identifier takes in a file where
    real bugs are also numbered. Counting those as bugs inflates the open count
    and reports them as duplicates of genuine entries. Only the three register
    sections carry referenceable identifiers, so the test is an allowlist: a new
    narrative section is then inert by default, where a denylist would need
    editing every time someone appends one.
    """
    section = ""
    for i, line in enumerate(lines, 1):
        if line.startswith("## "):
            section = line[3:].strip()
            continue
        if not line.startswith("### "):
            continue
        is_bug = section in REGISTER_SECTIONS
        head = line[4:].strip()
        # A few headings carry a historical marker instead of an identifier
        # (`### (was) 1h. ...`, `### G (original). ...`). They are deliberately
        # unreferenceable, so they get no identifier rather than a shared `?`
        # that would then collide with each other.
        m = re.match(r"([0-9A-Za-z-]+)\.\s*(.*)", head)
        ident, title = (m.group(1), m.group(2)) if m else ("--", head)
        # The heading carries emphasis and an em-dash verdict clause; the index
        # wants the claim alone, so both come off.
        title = re.sub(r"\*\*|~~|`", "", title)
        title = re.split(r"\s+—\s+|\s+--\s+", title)[0].strip()
        if len(title) > 92:
            title = title[:89] + "..."
        if not is_bug:
            status = "note"
        elif CLOSED.search(head):
            status = "closed"
        elif DECIDED.search(head):
            status = "decided"
        else:
            status = "**OPEN**"
        yield ident, status, title, i, is_bug


def render(rows):
    live = sum(1 for r in rows if r[1] == "**OPEN**")
    notes = sum(1 for r in rows if r[1] == "note")
    out = [
        BEGIN,
        "",
        f"**{live} open, {len(rows) - notes} bugs** (plus {notes} landing-note items,",
        "marked `note`). Generated from the headings by",
        "`scripts/bugindex.py` -- a bug's verdict is written into its own heading, so",
        "this is derived, never maintained by hand. Entries are never moved when they",
        "close (the file is co-owned and reordering it conflicts with every open",
        "branch), so **this table, not the `## Open` heading, is what says whether a",
        "bug is live.** Jump by line number.",
        "",
        "| § | Status | Line | What it is |",
        "|---|---|---|---|",
    ]
    for ident, status, title, line, _ in rows:
        out.append(f"| {ident} | {status} | {line} | {title} |")
    out += ["", END]
    return "\n".join(out)


def splice(text, block):
    """Put `block` in the document, replacing any block already there."""
    if BEGIN in text:
        start = text.index(BEGIN)
        stop = text.index(END) + len(END)
        return text[:start] + block + text[stop:]
    # First run: place it under the intro, above the first `## ` section.
    anchor = text.index("\n## ")
    return text[:anchor] + "\n" + block + "\n" + text[anchor:]


def build(text):
    """Render the index against the document that will *contain* it.

    The line numbers are the point of the table, and inserting the table moves
    every line it cites -- so a single pass always ships numbers that are wrong
    by the height of its own block. Iterating to a fixpoint is the whole fix:
    the block's height depends only on the entry count, so the second pass
    lands and the third confirms it. Guarded rather than assumed, because a
    silent non-convergence would ship plausible wrong numbers.
    """
    updated = text
    for _ in range(5):
        candidate = splice(updated, render(list(entries(updated.split("\n")))))
        if candidate == updated:
            return candidate
        updated = candidate
    raise SystemExit("bugindex: index did not converge -- line numbering is unstable")


def duplicates(rows):
    """Identifiers used by more than one bug entry.

    A duplicate is not cosmetic: the register's inbound references are textual
    ("see §Z"), so a repeated letter resolves to whichever heading the reader
    reaches first. `CLAUDE.md` already tells this story -- two bugs filed as §Q,
    one renamed §R with its self-references repointed -- and it recurred anyway,
    which is the argument for a check over a convention.

    Landing-note items are excluded: they are enumerated within their own
    section and nothing references them by bare number.
    """
    seen = {}
    for ident, _status, _title, line, is_bug in rows:
        if is_bug and ident != "--":
            seen.setdefault(ident, []).append(line)
    return {k: v for k, v in seen.items() if len(v) > 1}


def main():
    text = DOC.read_text(encoding="utf-8")
    updated = build(text)
    rows = list(entries(updated.split("\n")))

    dups = duplicates(rows)

    if "--check" in sys.argv:
        bad = 0
        if updated != text:
            print(
                "bugindex: Reports/open-bugs-handoff.md index is stale -- "
                "run `python3 scripts/bugindex.py`"
            )
            bad = 1
        for ident, lines_at in sorted(dups.items()):
            at = ", ".join(str(n) for n in lines_at)
            print(
                f"bugindex: identifier '{ident}' is used by {len(lines_at)} entries "
                f"(lines {at}) -- an inbound '§{ident}' is ambiguous"
            )
            bad = 1
        if not bad:
            print("bugindex: index current, identifiers unique")
        return bad

    DOC.write_text(updated, encoding="utf-8")
    live = sum(1 for r in rows if r[1] == "**OPEN**")
    print(f"bugindex: wrote {len(rows)} entries ({live} open)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
