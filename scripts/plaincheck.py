#!/usr/bin/env python3
"""Score a draft message to the owner, and name the lines that fail.

Why this exists. `CLAUDE.md` §"Writing to the owner" asks every message to
open with what changed in the world, why it matters, where it sits, and its
status -- then to put the mechanism below a fold. That rule was written
because 56% of PR subjects and an unmeasurable share of chat replies open
from inside the implementation (`Reports/agent-communication.md`).

A rule of that shape does not survive a real session as prose. `Reports/
lanes/README.md` records its own convention being followed 9% of the time
until a number and a checker were attached to it, and `docscheck.sh` exists
for the same reason. So: a command.

**What it CANNOT do, stated first so nobody cites its green as evidence.**
Chat is not an artifact the repo can gate -- there is no file to check at the
moment the message is sent. And the rule's most important clause, "where it
sits", is a semantic claim no regex reaches: a draft can name an arc, name
the wrong one, and pass. This tool measures *register* -- jargon in the part
the owner reads first -- and prompts for the rest. It is a mirror, not a
fence. `--selftest` proves each check can go red; nothing proves a green
message is a good one.
"""

import argparse
import json
import re
import sys
from pathlib import Path

# --- detectors --------------------------------------------------------------
# Deliberately narrow. A false positive costs an agent a re-read; a detector
# broad enough to catch every implementation-voiced sentence would also catch
# ordinary English, and the census that motivated this file already records
# that register is not reachable by pattern (see that report's caveats -- four
# subjects score clean and are still written from inside the code).

SYMBOL = [
    (re.compile(r"\b\w+::\w+"),                       "a type or module path"),
    (re.compile(r"\b\w+\(\)"),                        "a function call"),
    (re.compile(r"\b[\w/.-]+\.(?:rs|ron|py|sh|toml|json)\b"), "a file name"),
    (re.compile(r"\b[a-z][a-z0-9]*_[a-z0-9_]+\b"),    "a snake_case identifier"),
    (re.compile(r"\b[A-Z][A-Z0-9]*_[A-Z0-9_]+\b"),    "a SCREAMING_CASE constant"),
]

# A register code is fine in parentheses after a plain clause -- "(§S)" -- and
# is a finding when it carries the sentence. So the bare form is matched only
# outside brackets, which is why the text is blanked inside them first.
REGISTER = re.compile(
    r"§[A-Z0-9]+"
    r"|\bWP-\d+\b"
    r"|\b(?:[PWTSDA]\d+(?:\.\d+)?|M\d{1,2})\b"
    r"|\b[Bb]ug [A-Z]\b"
)
BRACKETED = re.compile(r"\([^)]*\)")

# Abbreviations a card label may use without expanding. Everything else in
# caps is a finding: `luma MAD` was printed under 262 images before anyone
# noticed nothing on the card said what MAD meant.
KNOWN_CAPS = {"MS", "FPS", "CPU", "GPU", "RAM", "CI", "PR", "UI", "ID", "OK",
              "PNG", "GIF", "HD", "AI", "API", "RGB", "2D", "3D"}
CAPS = re.compile(r"\b[A-Z]{2,5}\b")

FOLD = re.compile(r"^\s*(?:-{3,}|<details|#{1,4}\s*(?:mechanism|technical|detail))",
                  re.IGNORECASE)

OPENER_MAX_WORDS = 45
PLAIN_MAX_WORDS = 220


def find_symbols(text):
    hits = []
    for rx, what in SYMBOL:
        for m in rx.finditer(text):
            hits.append((m.group(0), what))
    return hits


def find_register(text):
    return [m.group(0) for m in REGISTER.finditer(BRACKETED.sub(lambda m: " " * len(m.group(0)), text))]


def split_fold(lines):
    """Everything before the first fold marker is what the owner reads first."""
    for i, line in enumerate(lines):
        if FOLD.match(line):
            return lines[:i], lines[i:]
    return lines, []


def check_draft(text):
    """Return (findings, notes). A finding is (check_id, line_no, message)."""
    findings = []
    lines = text.split("\n")
    plain, folded = split_fold(lines)
    plain_text = "\n".join(plain)

    for n, line in enumerate(plain, 1):
        for tok, what in find_symbols(line):
            findings.append(("C1", n, "%s -- %s, in the part read first" % (tok, what)))
        for tok in find_register(line):
            findings.append(("C2", n, "%s -- a register code carrying the sentence; "
                                      "put it in parentheses after a plain clause" % tok))

    opener = next((l for l in plain if l.strip()), "")
    if len(opener.split()) > OPENER_MAX_WORDS:
        findings.append(("C3", 1, "the opening sentence runs %d words; it has to be "
                                  "readable on its own" % len(opener.split())))

    words = len(plain_text.split())
    if not folded and words > PLAIN_MAX_WORDS:
        findings.append(("C4", len(plain), "%d words with no fold -- the mechanism is not "
                                           "separated from the brief" % words))
    return findings


def check_card(spec):
    findings = []
    for i, item in enumerate(spec.get("items") or [], 1):
        for label in (item.get("meta") or {}):
            for tok, what in find_symbols(label):
                findings.append(("C5", i, "meta label %r contains %s" % (label, tok)))
            for tok in CAPS.findall(label):
                if tok not in KNOWN_CAPS:
                    findings.append(("C5", i, "meta label %r uses %r without expanding it"
                                     % (label, tok)))
    for key in ("title", "question"):
        for tok, what in find_symbols(spec.get(key) or ""):
            findings.append(("C1", 0, "%s in the card %s -- %s" % (tok, key, what)))
    return findings


UNCHECKABLE = """
  Not checked, and not checkable here -- confirm by reading:
    * does the opening say what the change DOES, in the world's words or the
      work's, rather than the code's? (not every change shows on screen --
      when it doesn't, say what it is for)
    * does it say where this sits in an arc, not where it sits in your task
      list? ("a position in a queue is not a direction")
    * is it scaled to the message -- a one-line update kept to one line,
      not padded out to four headings?
    * could the owner stop reading at any line and still be oriented?
""".rstrip()


def report(findings, label):
    if findings:
        print("plaincheck: %d finding(s) in %s" % (len(findings), label))
        for cid, n, msg in findings:
            where = "line %d" % n if n else "header"
            print("  %s  %-8s %s" % (cid, where, msg))
    else:
        print("plaincheck: no register findings in %s" % label)
    print(UNCHECKABLE)
    return 1 if findings else 0


# --- selftest ---------------------------------------------------------------
# CLAUDE.md: "before you cite a guard's green as evidence, put the fault it is
# named for back and watch it go red." Built as a command for the reason that
# file gives -- as a discipline it is skipped, and a blind check reads exactly
# like a passing one. Add a row whenever you add a check.

GOOD = """Rock sitting on a pile of rubble no longer pretends it's sitting on
bedrock, so hitting the ground near a pile stops caving in ground that
should have held.

Why it matters: the last of three places the world mis-read what was
holding something up. Frames near a strike got about 20% faster too.

Where it sits: end of the arc on making broken rock carry weight.
Nothing queued behind it.

Landed on main, tests green.

--- mechanism, if you want it ---
load::ground_footing_distance walks the grain column; wrong cells
35,102 -> 1,337 on strike:20:200.
"""

FAULTS = [
    ("C1", "a code symbol in the brief",
     GOOD.replace("Rock sitting on a pile", "load::ground_footing_distance on a pile")),
    ("C2", "a register code carrying the sentence",
     GOOD.replace("Where it sits: end of the arc", "Where it sits: §S's last verb, end of the arc")),
    ("C3", "an opening sentence too long to read",
     GOOD.replace("Rock sitting on a pile of rubble no longer pretends it's sitting on\nbedrock, so hitting",
                  "Rock sitting on a pile of rubble no longer pretends that it is sitting upon "
                  "bedrock in the manner it previously did before this change was made and "
                  "landed, which is to say that when it comes to hitting " * 1)),
    ("C4", "the mechanism never separated from the brief",
     GOOD.replace("--- mechanism, if you want it ---", "and more:") + ("filler word " * 230)),
]

CARD_FAULTS = [
    ("C5", "a meta label naming the harness",
     {"title": "t", "question": "q", "items": [{"meta": {"luma MAD (deep rock crop)": 3.0}}]}),
    ("C5", "a meta label carrying a code symbol",
     {"title": "t", "question": "q", "items": [{"meta": {"crystal Material::glow": 1.8}}]}),
]


def selftest():
    ok = True
    base = check_draft(GOOD)
    print("plaincheck selftest")
    if base:
        ok = False
        print("  POSITIVE CONTROL FAILED -- the known-good draft has findings:")
        for f in base:
            print("     ", f)
    else:
        print("  positive control: the known-good draft is clean")

    for cid, what, text in FAULTS:
        hit = [f for f in check_draft(text) if f[0] == cid]
        print("  %s  %-45s %s" % (cid, what, "red" if hit else "STAYED GREEN -- blind"))
        ok &= bool(hit)
    for cid, what, spec in CARD_FAULTS:
        hit = [f for f in check_card(spec) if f[0] == cid]
        print("  %s  %-45s %s" % (cid, what, "red" if hit else "STAYED GREEN -- blind"))
        ok &= bool(hit)

    print("plaincheck selftest: %s" % ("every check can fire" if ok
                                       else "A CHECK IS BLIND -- replace it, do not widen it"))
    return 0 if ok else 1


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("path", nargs="?", help="draft file, or '-' for stdin")
    ap.add_argument("--card", metavar="JSON", help="check a review card spec instead")
    ap.add_argument("--selftest", action="store_true",
                    help="put each fault back and watch its check go red")
    args = ap.parse_args()

    if args.selftest:
        return selftest()
    if args.card:
        spec = json.loads(Path(args.card).read_text(encoding="utf-8"))
        return report(check_card(spec), args.card)
    if not args.path:
        ap.error("give a draft file, '-' for stdin, --card, or --selftest")
    text = sys.stdin.read() if args.path == "-" else Path(args.path).read_text(encoding="utf-8")
    return report(check_draft(text), args.path if args.path != "-" else "the draft")


if __name__ == "__main__":
    sys.exit(main())
