#!/usr/bin/env python3
"""The always-loaded context budget, and the cache-prefix churn beside it.

Why this exists
---------------
`CLAUDE.md` is auto-loaded into every session, every agent and every subagent
(`Reports/agent-documentation-audit-2026-08-24.md` §3). It is therefore the one
file whose size is multiplied by every head you run: a ten-agent census pays for
it ten times before any agent has read a line of source. Nothing else in the
repo has that property.

That audit measured it at 65,182 B / 16,300 tokens on 2026-08-24, found a
**98:1 add-to-remove ratio** across its whole history, and the thirteen repairs
in `claude-md-recommendations.md` all landed on 2026-08-25. The file was
97,019 B three days later -- **+49% after the cleanup**, and `CLAUDE.md`
acquired an explicit removal criterion inside that same window. The growth did
not stop.

This repo's own stated lesson, from `docscheck.sh`'s header and from
`.claude/README.md`'s account of the SessionStart hook, is that **a check that
runs catches what a convention does not**. The removal criterion is a
convention. This is the check.

The term this file measured first is the smaller one
--------------------------------------------------
Added 2026-08-27, after the always-loaded gate had already landed. Decomposing a
**real** measured agent run -- docbench's 95,518 `agent_tokens`, 2026-08-26 --
against the floor this file gates:

    always-loaded prefix   ~24,295   26%
    everything else        ~71,223   74%   <- reading

So three quarters of what an agent spends is **reading**, and the gate below
covers the other quarter. That is worth stating plainly because this file was
built first and could otherwise be mistaken for the whole picture: shrinking the
prefix is real and bounded, and it is not where the money is.

`--corpus` ranks the read side. It exists because a materiality floor was missing
and small savings kept looking actionable: a prompt-prefix repair worth ~2,940
tokens per run was surfaced as a finding in the same session that a **67,027**
token repair sat unmeasured. One unguided read of `Reports/open-bugs-handoff.md`
costs more than relocating CLAUDE.md across a seven-agent fan-out.

**The floor: 10,000 tokens per affected run.** Below it, fix it if it is free and
do not report it as a finding. Roughly 10% of one agent run and 1% of a fan-out
-- set so that the arithmetic, not enthusiasm, decides what gets a session.

The second number: cache-prefix churn
-------------------------------------
`CLAUDE.md` renders inside the cached prompt prefix (render order is
tools -> system -> messages, and a prefix cache is an exact byte match, so any
change invalidates everything after it). Two sessions that start either side of
a `CLAUDE.md` edit therefore cannot share a cached prefix: each distinct version
of the file is a distinct prefix.

It does **not** invalidate a session already running -- that session holds the
version it started with. This is why the remedy is *batching* the edits into one
commit near the end of a session, not avoiding them: the cost is the number of
distinct versions other sessions can start against, not the fact of editing.

Measured when this was written: 21 commits touched `CLAUDE.md` on 2026-08-25
alone, 60 across six days. That is cross-session cache reuse given up on the
single largest block of shared context, and it is invisible unless something
counts it.

What the numbers are, and what they are not
-------------------------------------------
Tokens are **bytes / 4.0**, this repo's own published calibration -- the audit's
65,182 B = 16,300 tokens is 3.999 B/token. It is an estimate and is labelled one
everywhere it is printed. `messages.count_tokens` is the real instrument and
needs an API key, which CI does not have; if one is ever available, re-derive
the divisor here rather than trusting this line.

The budget counts `CLAUDE.md` only. The SessionStart hook adds about 430 tokens
(`.claude/README.md`), and the harness system prompt and tool schemas add more
that this repo does not control. The figure is a **floor on the always-loaded
cost**, not the whole of it -- which is the safe direction for a gate to err in.

The ceiling, and why it is where it is
--------------------------------------
`CLAUDE.md` requires a bar set from measurement with headroom, never sitting on
the measured value. Today's figure is ~24,250. The gate is **28,000** -- about
15% of headroom, so an ordinary session that adds a rule does not trip it, and a
second week like the last one does.

The *reachable* number is recorded beside it rather than relabelled away, per
the same rule. Method, Gotchas and Conventions are ~64% of the file and are
consulted by lookup -- the audit's finding, and the file's own "which rules
apply to what you are doing right now" table is a routing layer that exists
because the content is already too big to read. Loaded on demand instead, the
always-loaded figure would be roughly 8,800 tokens. The gap between 24,250 and
8,800 is the work. The ceiling only stops it getting worse while nobody is
doing that work.
"""

import re
import subprocess
import sys
from collections import OrderedDict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALWAYS_LOADED = ROOT / "CLAUDE.md"
RECORD = ROOT / ".claude" / "README.md"

# The audit's own datum: 65,182 B measured as 16,300 tokens. Named, not guessed.
BYTES_PER_TOKEN = 4.0
CEILING_TOKENS = 28_000
# What dropping the three lookup-consulted sections would leave. Recorded so the
# gap stays visible; nothing enforces it.
TARGET_TOKENS = 8_800

BEGIN = "<!-- BEGIN GENERATED CONTEXT BUDGET -- regenerate with scripts/contextbudget.py --write -->"
END = "<!-- END GENERATED CONTEXT BUDGET -->"

# Sections whose content is consulted situationally rather than read: the audit's
# finding, restated as data so the report can price the on-demand option instead
# of asserting it. A section named here is NOT excluded from the budget -- it is
# loaded today and the budget says so.
LOOKUP_SECTIONS = ("Method", "Gotchas", "Conventions")


def tokens(nbytes):
    return int(round(nbytes / BYTES_PER_TOKEN))


def sections(text):
    """(name, lines, bytes) per `## ` heading, in file order."""
    out = []
    name, lines, nbytes = None, 0, 0
    for line in text.splitlines():
        if line.startswith("## "):
            if name is not None:
                out.append((name, lines, nbytes))
            name, lines, nbytes = line[3:].strip(), 0, 0
        elif name is not None:
            lines += 1
            nbytes += len(line.encode("utf-8")) + 1
    if name is not None:
        out.append((name, lines, nbytes))
    return out


def is_lookup(name):
    return any(name.startswith(p) for p in LOOKUP_SECTIONS)


def git(*args):
    try:
        return subprocess.run(
            ["git", *args], cwd=ROOT, capture_output=True, text=True, check=True
        ).stdout
    except (subprocess.CalledProcessError, FileNotFoundError):
        return ""


def churn(days=7):
    """[(date, distinct_versions)] for the always-loaded file, newest day first.

    Counts distinct *blobs*, not commits: two commits that land the same bytes
    (a revert, a merge taking one side whole) are one prefix, not two. Counting
    commits would overstate exactly the case that costs nothing.
    """
    log = git("log", f"-n{days * 40}", "--format=%H %ad", "--date=short", "--", "CLAUDE.md")
    rows = [ln.split() for ln in log.splitlines() if ln.strip()]
    if not rows:
        return []
    spec = "".join(f"{sha}:CLAUDE.md\n" for sha, _ in rows)
    try:
        out = subprocess.run(
            ["git", "cat-file", "--batch-check=%(objectname)"],
            cwd=ROOT, input=spec, capture_output=True, text=True, check=True,
        ).stdout.split()
    except (subprocess.CalledProcessError, FileNotFoundError):
        return []
    by_day = OrderedDict()
    for (_, date), blob in zip(rows, out):
        if blob and not blob.startswith("missing"):
            by_day.setdefault(date, set()).add(blob)
    return [(d, len(v)) for d, v in list(by_day.items())[:days]]


# The floor below which a saving is not worth a session. See the docstring.
MATERIALITY_FLOOR = 10_000

# The documents CLAUDE.md routes sessions into. Each carries an explicit
# "do not read it whole" warning, which is a convention -- so the number that
# matters is what one unguided read costs when the convention does not hold.
CORPUS = (
    "Reports/open-bugs-handoff.md",
    "Reports/dead-ends.md",
    "PLAN.md",
    "PLAN-log.md",
    "README.md",
    "Reports/README.md",
)


def corpus():
    """Rank the read side by what one unguided read costs."""
    rows = []
    for rel in CORPUS:
        f = ROOT / rel
        if f.exists():
            b = f.stat().st_size
            rows.append((rel, b, tokens(b)))
    rows.sort(key=lambda r: -r[1])
    print("contextbudget: the READ side -- ~74% of a measured agent run")
    print(f"  (floor for acting: {MATERIALITY_FLOOR:,} tokens per affected run)\n")
    print(f"  {'document':<34} {'~tokens':>9}  {'vs floor':>9}")
    for rel, _, tok in rows:
        print(f"  {rel:<34} {tok:>9,}  {tok / MATERIALITY_FLOOR:>8.1f}x")
    total = sum(r[2] for r in rows)
    print(f"\n  corpus total if read whole: ~{total:,} tokens")
    print(f"  always-loaded floor, for scale: ~{measure()['tokens']:,} "
          f"({100 * measure()['tokens'] / total:.0f}% of it)")
    print("\n  A single unguided read of the largest of these outweighs every")
    print("  prompt-prefix repair in the repo combined. Aim here first.")


def measure():
    text = ALWAYS_LOADED.read_text(encoding="utf-8")
    nbytes = len(text.encode("utf-8"))
    secs = sections(text)
    lookup = sum(b for n, _, b in secs if is_lookup(n))
    return {
        "bytes": nbytes,
        "tokens": tokens(nbytes),
        "lines": len(text.splitlines()),
        "sections": secs,
        "lookup_bytes": lookup,
        "lookup_tokens": tokens(lookup),
        "lookup_pct": 100.0 * lookup / nbytes if nbytes else 0.0,
    }


def block(m):
    """The generated record. Kept small on purpose -- `.claude/README.md` is read
    on demand, but a generated block that grows is the defect this file exists to
    catch, arriving by the back door."""
    ch = churn()
    churn_str = ", ".join(f"{d} x{n}" for d, n in ch[:5]) or "no history available"
    over = m["tokens"] - CEILING_TOKENS
    verdict = f"**{over:+,} over**" if over > 0 else f"{-over:,} under"
    return "\n".join([
        BEGIN,
        "",
        f"**Always-loaded floor: ~{m['tokens']:,} tokens** — `CLAUDE.md` at "
        f"{m['bytes']:,} B / {m['lines']:,} lines, bytes/4.0. Ceiling "
        f"{CEILING_TOKENS:,} ({verdict}). Plus ~430 for the hook, and the "
        "harness system prompt and tool schemas on top; this is a floor.",
        "",
        f"Paid by **every session, agent and subagent** — ten heads is "
        f"~{10 * m['tokens']:,} tokens before any of them reads source.",
        "",
        f"Consulted by lookup, paid unconditionally: {m['lookup_pct']:.0f}% "
        f"(~{m['lookup_tokens']:,} tokens) across "
        f"{', '.join(LOOKUP_SECTIONS)}. On demand instead, the floor would be "
        f"~{TARGET_TOKENS:,}. That gap is the work; the ceiling only holds the line.",
        "",
        f"Cache-prefix churn, distinct versions per day (newest first): {churn_str}. "
        "Each one is a prefix no later session can share. A running session keeps "
        "the version it started with, so the remedy is batching edits into one "
        "commit near session end, not editing less.",
        "",
        END,
    ])


def report(m):
    print(f"contextbudget: CLAUDE.md {m['bytes']:,} B / {m['lines']:,} lines")
    print(f"contextbudget: ~{m['tokens']:,} tokens always-loaded (bytes/4.0, estimate)")
    print(f"contextbudget: ceiling {CEILING_TOKENS:,}, reachable target ~{TARGET_TOKENS:,}")
    print()
    print(f"  {'section':<52} {'lines':>6} {'~tokens':>8}  {'%':>5}")
    for name, lines, nbytes in sorted(m["sections"], key=lambda s: -s[2]):
        mark = " *" if is_lookup(name) else "  "
        pct = 100.0 * nbytes / m["bytes"]
        print(f"{mark}{name[:52]:<52} {lines:>6} {tokens(nbytes):>8,}  {pct:>4.1f}%")
    print()
    print(f"  * consulted by lookup, paid unconditionally: {m['lookup_pct']:.0f}% "
          f"(~{m['lookup_tokens']:,} tokens)")
    print()
    ch = churn()
    if ch:
        print("  cache-prefix churn (distinct versions per day, newest first):")
        for d, n in ch:
            print(f"    {d}  {n:>3} version(s){'   <-- each is a prefix no later session can share' if n > 3 else ''}")
    print()
    if m["tokens"] > CEILING_TOKENS:
        print(f"contextbudget: OVER CEILING by {m['tokens'] - CEILING_TOKENS:,} tokens")
    else:
        print(f"contextbudget: {CEILING_TOKENS - m['tokens']:,} tokens of headroom")


def _comparable(b):
    """The part of the record that staleness is about: everything but churn."""
    return re.sub(r"Cache-prefix churn.*?(?=\n\n|$)", "", b, flags=re.S).strip()


def read_block():
    if not RECORD.exists():
        return None
    t = RECORD.read_text(encoding="utf-8")
    i, j = t.find(BEGIN), t.find(END)
    if i < 0 or j < 0:
        return None
    return t[i:j + len(END)]


def write_block(m):
    new = block(m)
    t = RECORD.read_text(encoding="utf-8")
    i, j = t.find(BEGIN), t.find(END)
    if i < 0 or j < 0:
        t = t.rstrip("\n") + "\n\n## The always-loaded context budget\n\n" + new + "\n"
    else:
        t = t[:i] + new + t[j + len(END):]
    RECORD.write_text(t, encoding="utf-8")
    print(f"contextbudget: wrote the record into {RECORD.relative_to(ROOT)}")


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    m = measure()

    if mode == "--corpus":
        corpus()
        return 0

    if mode == "--write":
        write_block(m)
        return 0

    if mode == "--check":
        # Staleness only. The ceiling is a separate question and a separate
        # exit: a repo can be honestly over budget and still have a current
        # record, and conflating the two makes the gate un-actionable.
        #
        # The churn line is excluded from the comparison, and that is not a
        # loosening -- it is the fix for a check that fired when nothing was
        # wrong. Churn is derived from git history, so it moves on every commit
        # touching CLAUDE.md, including the commit that regenerates this record.
        # Left in, the check demanded a second commit after every first one and
        # reported "stale" about a budget figure that had not changed. What is
        # gated is the budget; churn rides along as reporting and self-heals on
        # the next --write.
        cur = read_block()
        if cur is None:
            print(f"contextbudget: no generated block in {RECORD.relative_to(ROOT)}"
                  " -- run scripts/contextbudget.py --write")
            return 1
        if _comparable(cur) != _comparable(block(m)):
            print("contextbudget: the recorded budget is stale against CLAUDE.md"
                  " -- run scripts/contextbudget.py --write")
            return 1
        print("contextbudget: record current")
        return 0

    if mode == "--gate":
        if m["tokens"] > CEILING_TOKENS:
            print(f"contextbudget: {m['tokens']:,} tokens always-loaded, ceiling "
                  f"{CEILING_TOKENS:,} -- over by "
                  f"{m['tokens'] - CEILING_TOKENS:,}")
            print("  -> every session, agent and subagent pays this. Cut before adding.")
            return 1
        print(f"contextbudget: {m['tokens']:,} tokens, under the {CEILING_TOKENS:,} ceiling")
        return 0

    report(m)
    return 0


if __name__ == "__main__":
    sys.exit(main())
