"""Every rule that was in `CLAUDE.md` is still *somewhere*.

**The one edit to `CLAUDE.md` that cannot be allowed to lose content is the
one that makes it smaller.** `Reports/two-games-one-repo-2026-08-30.md` step 7
moves ~13,700 tokens of evidence narrative out to a routed report and keeps
the rule statements inlined. That is a judgement call per rule, made across
roughly a thousand lines, and the failure it invites is silent: a rule that
simply stops existing reads, in the diff, exactly like a rule that was
correctly condensed.

So this is the mechanical half. It takes the **bolded statements** -- which is
how this file writes a rule, by house style -- from a baseline revision of
`CLAUDE.md`, and requires each one to still appear either in the current
`CLAUDE.md` or in the report the narrative moved to. It does not judge whether
the split was done *well*; it judges that nothing vanished.

    python3 scripts/rulecheck.py --baseline <git-rev>
    python3 scripts/rulecheck.py --baseline <git-rev> --selftest

**Why bolded statements and not headings or sentences.** `CLAUDE.md`'s own
addition rule is *"state the rule universally, put the subsystem in the
evidence clause"*, and in practice every rule in the file is written as a
bolded imperative followed by its evidence. Measured 2026-08-30: the lookup
sections are 14% bold and 86% prose, and the bold is the part that has to
survive. Headings are too coarse (a section holds many rules) and sentences
too fine (the prose is what is meant to go).

**Matching is normalised, deliberately.** The house style puts `**bold**` and
`` `code` `` inside hard-wrapped sentences, so a phrase that reads as one line
is stored across two -- measured at **23% of all bolded phrases** in these
documents, which is why `scripts/docgrep.py` exists at all. Comparing raw
strings would report a quarter of the file as lost. Whitespace is collapsed
and markup stripped before comparison, the same normalisation `docgrep` does.
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CLAUDE_MD = ROOT / "CLAUDE.md"
# Where the narrative is allowed to have gone. A rule found in any of these
# counts as surviving; the point is that it exists, not where.
DESTINATIONS = [
    ROOT / "CLAUDE.md",
    ROOT / "Reports" / "method-evidence.md",
    ROOT / "Reports" / "session-programs.md",
]
# Below this many characters a "rule" is a formatting flourish -- a bolded
# word inside a sentence -- not a statement. Set from looking at what falls
# either side rather than from a round number: at 24 the shortest kept item is
# a real rule ("Determinism is required") and the longest dropped one is a
# two-word emphasis.
MIN_RULE_CHARS = 24


def strip_code(text):
    """Blank inline code spans, keeping length so offsets survive.

    **This file is the reason, and it is its own illustration.** One of
    `CLAUDE.md`'s rules reads *"the house style puts `**bold**` and `` `code`
    `` inside sentences"* -- so the literal characters `**bold**` sit inside a
    backtick span, and a bold-finder run before this reads them as real
    delimiters and emits a fragment. The rule about markup defeating a naive
    grep, defeating a naive grep.

    **Applied to both sides or to neither.** Blanking the baseline while the
    haystack merely strips backticks makes every rule containing code fail to
    match -- measured at 70 of 223 when they disagreed, against 0 when they
    agree. A normalisation is only ever a property of a *comparison*.
    """
    return re.sub(r"``.+?``|`[^`\n]*`", lambda m: " " * len(m.group(0)), text, flags=re.S)


def normalise(text):
    """Collapse the differences that hard wrapping and inline markup create."""
    text = strip_code(text)
    text = re.sub(r"\*+", "", text)
    text = re.sub(r"\s+", " ", text)
    return text.strip().lower()


def bolded(text):
    """Bolded statements, without the fragments a naive pattern invents.

    `\*\*(.+?)\*\*` under `re.S` looks right and is not: where a bolded phrase
    contains a backtick span, the lazy match can close on a `**` belonging to
    a *different* rule further down, yielding a fragment that starts or ends
    mid-markup. Measured on this file: three such fragments, each of which
    then reports as a missing rule when the document is compared with itself.
    A check whose baseline does not compare clean to itself cannot be trusted
    about anything else.

    Excluding `*` from the body confines a match to one bolded span, and the
    length ceiling drops the runaway case where an unpaired marker would
    otherwise swallow a page.
    """
    text = strip_code(text)
    return [
        m.group(1)
        for m in re.finditer(r"\*\*([^*]{%d,700}?)\*\*" % MIN_RULE_CHARS, text, re.S)
        if len(normalise(m.group(1))) >= MIN_RULE_CHARS
    ]


def read_rev(rev, path):
    rel = path.relative_to(ROOT)
    r = subprocess.run(
        ["git", "show", f"{rev}:{rel}"],
        cwd=ROOT, capture_output=True, text=True,
    )
    if r.returncode != 0:
        sys.exit(f"rulecheck: cannot read {rel} at {rev}: {r.stderr.strip()}")
    return r.stdout


def haystack():
    parts = []
    for p in DESTINATIONS:
        if p.is_file():
            parts.append(p.read_text(encoding="utf-8"))
    return normalise("\n".join(parts))


def missing(baseline_text, hay):
    return [r for r in bolded(baseline_text) if normalise(r) not in hay]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--baseline", required=True,
                    help="git revision of CLAUDE.md to compare against")
    ap.add_argument("--selftest", action="store_true",
                    help="prove the check can fail: drop a rule and expect a report")
    a = ap.parse_args()

    base = read_rev(a.baseline, CLAUDE_MD)
    rules = bolded(base)

    if a.selftest:
        # **The positive control.** A check that only ever passes is worth
        # nothing, and this repo has the rule about citing a green written
        # down because it has been burned by exactly that. Delete one rule
        # from the haystack and require the check to notice.
        hay = haystack()
        victim = next((r for r in rules if normalise(r) in hay), None)
        if victim is None:
            sys.exit("rulecheck: selftest cannot run -- no rule is currently present")
        wounded = hay.replace(normalise(victim), "", 1)
        if not missing(base, wounded):
            sys.exit("rulecheck: SELFTEST FAILED -- removing a rule was not detected")
        print(f"rulecheck: selftest ok -- removing {normalise(victim)[:60]!r} is detected")
        return 0

    gone = missing(base, haystack())
    print(f"rulecheck: {len(rules)} rule statements in CLAUDE.md at {a.baseline}")
    if not gone:
        print("rulecheck: all of them still present")
        return 0
    print(f"rulecheck: {len(gone)} NOT FOUND in {', '.join(p.name for p in DESTINATIONS if p.is_file())}:")
    for g in gone:
        print(f"  - {normalise(g)[:110]}")
    print("\n  -> each of these was a rule and is now in no destination. Either")
    print("     restore it, or -- if it was cut deliberately -- say which of the")
    print("     three removal criteria applies, in the commit message.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
