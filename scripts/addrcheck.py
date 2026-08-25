#!/usr/bin/env python3
"""Verify that every cross-document address in `Reports/dead-ends.md` resolves.

Why this exists
---------------
`dead-ends.md` addresses its entries by *document and heading or paragraph
name*, not by line number:

    - **PLAN.md 'Step 4 (§11) — quiescence, the terminal snap, and body sleep'**
    - **README.md 'M17 status' — 'A step's cost now depends ...' paragraph**

Measured 2026-08-25: **133 such addresses** (a heading and, often, a named paragraph within it), of which 47 point into `README.md`
and 32 into `PLAN.md`. That makes the headings of those two documents a
load-bearing address space for the one register whose whole job is stopping an
agent re-walking a dead end — and **nothing noticed when a heading was renamed
out from under them**. A rename is silent: the entry still reads correctly, it
just no longer points anywhere.

This is the check that was worth building. Two others considered in the same
pass were not, and the difference is the useful part:

* A gate over *cited test names* was rejected — a name alone cannot separate
  "claims a live guard" from "correctly records a retired one", and correct
  documentation trips it. See `Reports/agent-documentation-audit-2026-08-24.md`
  §10.
* A general identifier-resolution sweep was rejected outright: design reports,
  prior-art surveys and this very register are all *supposed* to name things
  that are not in the tree.

An address is different. It is an unambiguous claim that some text exists in
some document, with no semantic content to misread — so it either resolves or
it does not.

Normalisation, all of it learned from real entries rather than guessed
---------------------------------------------------------------------
Authors quote a heading the way it reads, not the way it is marked up, so the
raw string almost never matches byte-for-byte. Without these, 10 of 88
addresses fail and **every one of those ten is a false alarm**:

* backticks -- `PLAN.md 'Reports/liquid-heightfield-design.md — landed'` against
  a heading that wraps the filename in backticks;
* `**bold**` and `~~strike~~` -- `open-bugs-handoff.md`'s headings carry
  `## ~~Open~~ **CLOSED** — ...`, and the address quotes only the words;
* em/en dashes, quoted as either;
* `...` elision -- authors abbreviate a long heading mid-quote
  (`'Plant substrate v2 — started...'`, `'Live playtest feedback (screenshots
  of cargo run...)'`), so it is treated as a wildcard and the fragments must
  appear in order.

What it does *not* catch, stated so nobody over-trusts it
--------------------------------------------------------
It asks whether the quoted text exists **anywhere in the document**, not
whether it is still a heading. Renaming `worldgen-design.md`'s
`### 6a. Generated terrain must be *at rest*` still passes, because the same
phrase appears in that file's prose twice more. Verified by fault injection:
of three realistic renames, two fire and that one does not.

That is the right way round. The failure mode is a false **pass**, never a
false alarm, so wiring it into `docscheck` cannot cost anyone a red gate over
correct documentation -- which is exactly what disqualified the two checks
rejected above.

Run `python3 scripts/addrcheck.py`; `--check` is the same thing with an exit
code, and is what `scripts/docscheck.sh` runs.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REGISTER = ROOT / "Reports" / "dead-ends.md"
# The whole bolded address prefix of an entry, then every quoted fragment
# inside it. Capturing only the *first* quote was a real defect, caught by
# putting the fault back: an entry reading
#   - **README.md 'M17 status' - 'A step\'s cost now depends ...' paragraph**
# names two things, and checking only `M17 status` let a reworded paragraph
# pass silently. Both are addresses and both are checked.
ADDRESS = re.compile(r"^- \*\*([A-Za-z0-9_./-]+\.md)\s(.*?)\*\*", re.M | re.S)
# A quote mark followed by a lowercase letter is an apostrophe, not a closing
# delimiter -- this repo's headings are full of "a step's cost", "main's
# rule", "doesn't". Pairing naively split those fragments in half and the
# check silently passed on a reworded paragraph; found by fault injection,
# not by reading.
OTHER_DOCS = re.compile(r"([A-Za-z0-9_./-]+\.md)")
QUOTED = re.compile(r"'((?:[^']|'(?=[a-z]))+)'")


def normalise(s):
    """Reduce both sides to the words, dropping every difference an author
    introduces by quoting a heading from memory rather than copying it.

    Each rule below was added because a *correct* address failed without it,
    never speculatively -- see the module docstring for why that distinction
    decided whether this check was worth having at all."""
    s = s.lower()
    for mark in ("`", "**", "~~", "*", "_"):
        s = s.replace(mark, "")
    for dash in ("—", "–"):
        s = s.replace(dash, "-")
    s = s.replace("→", "->").replace("⇒", "->")
    for q in ("\u201c", "\u201d", "\u2018", "\u2019"):
        s = s.replace(q, "'" if q in ("\u2018", "\u2019") else '"')
    s = s.replace('"', "").replace("'", "")
    s = re.sub(r"§\s*", "section ", s)
    s = re.sub(r"[ \t]+", " ", s)
    return re.sub(r"\s+", " ", s).strip()


def resolve(doc):
    """`dead-ends.md` names documents by basename as often as by path."""
    p = ROOT / doc
    if p.exists():
        return p
    for cand in ROOT.rglob(Path(doc).name):
        if "target" not in cand.parts and ".git" not in cand.parts:
            return cand
    return None


def contains(body, quoted):
    """`...` is an author elision: the fragments must appear in order."""
    parts = [p for p in normalise(quoted).split("...") if p]
    at = 0
    for part in parts:
        at = body.find(part, at)
        if at < 0:
            return False
        at += len(part)
    return True


def main():
    args = set(sys.argv[1:])
    unknown = args - {"--check"}
    if unknown:
        print(f"addrcheck: unrecognised argument(s): {' '.join(sorted(unknown))}")
        print("usage: addrcheck.py [--check]")
        return 2

    if not REGISTER.exists():
        print(f"addrcheck: {REGISTER} not found")
        return 1

    bodies = {}
    total = 0
    broken = []
    for doc, rest in ADDRESS.findall(REGISTER.read_text(encoding="utf-8")):
        fragments = QUOTED.findall(rest)
        if not fragments:
            continue

        # An entry addresses a *set* of documents, not one. Two real shapes
        # forced this and neither is an edge case:
        #   - "PLAN-log.md 'Overnight run, section 4'; 'Liquid (Report B)' item 2"
        #     names one document and then quotes a heading that lives in its
        #     sibling;
        #   - "PLAN.md Progress log, M6 entry" -- but the progress log was
        #     *split out* into PLAN-log.md, so the text moved and the address
        #     did not.
        # Checking each fragment against the first-named document alone
        # reported both as broken when the text was exactly where the author
        # meant. A fragment resolves if it is in any document the entry names,
        # plus PLAN.md/PLAN-log.md always standing in for each other.
        named = [doc] + OTHER_DOCS.findall(rest)
        if any(d in ("PLAN.md", "PLAN-log.md") for d in named):
            named += ["PLAN.md", "PLAN-log.md"]

        searched = []
        for d in dict.fromkeys(named):
            path = resolve(d)
            if path is None:
                continue
            searched.append(
                bodies.setdefault(
                    path, normalise(path.read_text(encoding="utf-8", errors="ignore"))
                )
            )
        if not searched:
            total += len(fragments)
            broken += [(doc, f, "document does not exist") for f in fragments]
            continue

        for quoted in fragments:
            total += 1
            if not any(contains(body, quoted) for body in searched):
                broken.append((doc, quoted, "text not found in that document"))

    if broken:
        print(f"addrcheck: {len(broken)} of {total} dead-ends.md addresses do not resolve:")
        for doc, quoted, why in broken:
            print(f"  {doc}: {why}")
            print(f"    '{quoted[:100]}'")
        print("  -> a heading or paragraph was renamed. Repoint the dead-ends.md entry,")
        print("     or restore the wording. Renaming a heading in README.md or PLAN.md")
        print("     is a cross-repo edit: 79 of these addresses point at those two.")
        return 1

    print(f"addrcheck: all {total} dead-ends.md addresses resolve")
    return 0


if __name__ == "__main__":
    sys.exit(main())
