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
lines turns any concurrent edit into a conflict. An index answers the question
the reader actually has -- *which of these is still live, and what line is it
on* -- for a few hundred tokens instead of eighty thousand.

**It does not cost zero merge surface, and an earlier version of this docstring
claimed it did.** The line numbers are the useful part and they are also the
expensive part: inserting an entry shifts every row below it, so two lanes that
both file a bug and regenerate produce conflicting hunks in this one block, in
a file `.gitattributes` deliberately denies a `union` merge. What makes that
cheap is that the block is *derived*: a conflict here is never hand-merged.
Take either side whole and re-run the generator -- the output is a pure
function of the headings, so the regenerated block is correct by construction
whichever side you started from. That instruction is printed into the block
itself, where whoever hits the conflict will see it.

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

# Status is read from the heading's **bold verdict clause** and its
# strikethrough -- never from the whole heading. Searching the whole heading
# was the first implementation and it was wrong in both directions: `### P3.
# The generation loop -- §F4 closed, ...` matched CLOSED on the words "§F4
# closed" in its *title* and filed a live bug as closed, and any future entry
# titled "the frame budget is not fixed" would do the same.
CLOSED = re.compile(r"FIXED|CLOSED|RESOLVED|RETIRED|DUPLICATE|DOES NOT REPRODUCE", re.I)
DECIDED = re.compile(
    r"DECISION CARD|SEQUENCING DECIDED|DESIGN DIRECTION|JUDGED", re.I
)
# A verdict that says any part of the entry is still live wins over a closed
# word in the same clause. `### G. Grassfire ... **SPREAD AND MOISTURE FIXED
# ...; the *colour* is open and is render's**` is half-done, and filing it as
# closed hides the live half. False-open is the safe direction here: it costs a
# reader one heading, where false-closed costs them the bug.
STILL_OPEN = re.compile(r"\bopen\b", re.I)
# Deliberately superseded headings, kept beside their live successors as
# history. They carry no verdict, so without this they render as **OPEN** and
# tell a reader that a fixed bug is live -- measured: three of them did.
HISTORIC = re.compile(r"^\(was\)|\(original\)")

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
    fenced = False
    for i, line in enumerate(lines, 1):
        # Fenced code can contain heading-shaped lines. None in this file does
        # today; a pasted shell transcript or markdown example would, and the
        # failure is a confident register entry that is not a bug.
        if line.startswith("```"):
            fenced = not fenced
            continue
        if fenced:
            continue
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
        verdict = " ".join(re.findall(r"\*\*(.+?)\*\*", head))
        if not is_bug:
            status = "note"
        elif HISTORIC.search(head):
            status = "historic"
        elif STILL_OPEN.search(verdict):
            status = "**OPEN**"
        elif "~~" in head or CLOSED.search(verdict):
            status = "closed"
        elif DECIDED.search(verdict):
            status = "decided"
        else:
            # No verdict clause at all -- an entry nobody has ruled on.
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
        "**Merge conflict in this block?** Do not hand-merge it. Take either side",
        "whole, then run `python3 scripts/bugindex.py` -- the table is derived from",
        "the headings, so the regenerated block is correct from either starting",
        "point.",
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
    # `"--check" in sys.argv` was the first version, and it fails the way this
    # repo has already paid for: "an unknown argument is silently ignored"
    # (CLAUDE.md). `--chekc` or `--check=1` missed the membership test and fell
    # through to the *write* path, rewriting a 343 KB co-owned file for someone
    # who meant to verify it. Unrecognised argv is now an error.
    args = set(sys.argv[1:])
    check = "--check" in args
    unknown = args - {"--check"}
    if unknown:
        print(f"bugindex: unrecognised argument(s): {' '.join(sorted(unknown))}")
        print("usage: bugindex.py [--check]")
        return 2

    # Explicit newline="" on read and "\n" on write. `Path.read_text` grew a
    # `newline` argument only in 3.13, so both go through `open`. Without this,
    # `write_text` translates to CRLF on Windows -- and this repo's gotchas are
    # written for a Windows dev box -- so one run there would rewrite all
    # ~5,700 lines of the register and conflict with every open branch.
    with open(DOC, encoding="utf-8", newline="") as fh:
        text = fh.read()
    updated = build(text)
    rows = list(entries(updated.split("\n")))

    dups = duplicates(rows)

    if check:
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

    with open(DOC, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(updated)
    live = sum(1 for r in rows if r[1] == "**OPEN**")
    print(f"bugindex: wrote {len(rows)} entries ({live} open)")
    # The person regenerating is the one most likely to have just introduced a
    # collision, and discarding `dups` here meant they were the last to hear
    # about it -- via a docscheck run that is already red for other reasons and
    # so gets skimmed. Close the loop where the collision is created.
    for ident, lines_at in sorted(dups.items()):
        at = ", ".join(str(n) for n in lines_at)
        print(
            f"bugindex: WARNING identifier '{ident}' is used by "
            f"{len(lines_at)} entries (lines {at})"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
