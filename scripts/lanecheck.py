#!/usr/bin/env python3
"""Keep a lane note a message channel, not a work journal.

Why this exists
---------------
`Reports/lanes/<lane>.md` is how two concurrent sessions correct each other
without a human carrying the message, and it is a good design: single-writer, so
it costs **zero merge surface**, and `git show origin/<branch>:Reports/lanes/
<lane>.md` reads another lane's note without merging anything.

`Reports/lanes/README.md` already rules what belongs in one:

    A lane note is a *finding*, not a status update, and it does not replace
    the real record: what belongs in CLAUDE.md, a report or dead-ends.md goes
    there.

Measured 2026-08-27, that rule was being followed **9% of the time**.
`docs-audit.md` had gone 18,011 -> 47,168 B in a single day over 21 commits;
split by the README's own `-> lane` addressing convention, **2 sections were
addressed to another lane (~1,035 tokens) and 14 were unaddressed work journal
(~10,717)**. The content was good -- every entry a finding with numbers -- but
it had become a report living in the channel's directory.

The cost lands on the next reader: a session that follows the SessionStart
hook's pointer pays ~11,792 tokens to reach ~1,035 tokens of message. The
channel did not get expensive; a journal moved in.

Same failure class as everything else here -- a convention with nothing checking
it. `.claude/README.md` records the founding argument: `CLAUDE.md` asked every
session to run `branchcheck.sh` and ten branches still sat at exactly 160
commits behind. A check catches what a convention does not.

Why the size finding WARNS and does not fail
--------------------------------------------
**A lane may only write its own note.** That is the single-writer property the
whole channel depends on, so no other session may trim an oversized note -- the
remedy belongs to its owner and to nobody else. A gate that fails on a condition
the runner is forbidden to fix is a gate that gets disabled within a week, so
the size finding prints and returns 0.

What DOES fail is the cap in `lanes/README.md` drifting from the cap here:
anyone can fix that, it is a generated-consistency defect of exactly the kind
`docscheck` exists for, and it is what keeps the documented rule and the
enforced rule the same number.

The cap
-------
12,000 B (~3,000 tokens), set from measurement with headroom per `CLAUDE.md`'s
bar rule rather than from an aspiration: `perf.md`, a healthy note doing the job
as designed, is 7,661 B -- so the cap sits ~57% above a known-good example and
far below the 47,168 B case that prompted it.
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
LANES = ROOT / "Reports" / "lanes"
PROTOCOL = LANES / "README.md"

CAP_BYTES = 12_000
BYTES_PER_TOKEN = 4.0

# The sentence in lanes/README.md that must carry the same number. Kept as a
# regex over the prose rather than a generated block: the protocol doc is short,
# read by humans, and a generated block in it would be more machinery than the
# one number justifies.
CAP_IN_PROTOCOL = re.compile(r"soft cap of \*\*([\d,]+) B\*\*")


def notes():
    if not LANES.is_dir():
        return []
    return sorted(p for p in LANES.glob("*.md") if p.name != "README.md")


def oversized():
    return [(p, p.stat().st_size) for p in notes() if p.stat().st_size > CAP_BYTES]


def report():
    ns = notes()
    if not ns:
        print("lanecheck: no lane notes")
        return 0
    print(f"lanecheck: cap {CAP_BYTES:,} B (~{int(CAP_BYTES / BYTES_PER_TOKEN):,} tokens)")
    for p in ns:
        b = p.stat().st_size
        flag = "  OVER" if b > CAP_BYTES else ""
        print(f"  {p.name:<24} {b:>7,} B  ~{int(b / BYTES_PER_TOKEN):>6,} tok{flag}")
    return 0


def check():
    rc = 0
    # (1) hard: the documented cap must match the enforced one. Anyone can fix.
    # Reported under `lanecheck-cap:`, deliberately NOT sharing a token with the
    # warning below: the warning fires whenever any note is over cap, so a
    # selftest grepping for `lanecheck` would match no matter what it injected
    # and report a blind row as green.
    if PROTOCOL.exists():
        m = CAP_IN_PROTOCOL.search(PROTOCOL.read_text(encoding="utf-8"))
        if not m:
            print("lanecheck-cap: Reports/lanes/README.md does not state the size cap"
                  f" -- it must say: soft cap of **{CAP_BYTES:,} B**")
            rc = 1
        elif int(m.group(1).replace(",", "")) != CAP_BYTES:
            print(f"lanecheck-cap: Reports/lanes/README.md says the cap is {m.group(1)} B,"
                  f" scripts/lanecheck.py enforces {CAP_BYTES:,} B -- they must agree")
            rc = 1

    # (2) soft: an oversized note. Only its owner may fix it, so this warns.
    for p, b in oversized():
        print(f"lanecheck: {p.relative_to(ROOT)} is {b:,} B "
              f"(~{int(b / BYTES_PER_TOKEN):,} tokens), over the {CAP_BYTES:,} B cap.")
        print("  -> it has outgrown a note. Promote the findings to a report or to")
        print("     dead-ends.md and leave a pointer; keep here only what is addressed")
        print("     to another lane. WARNING ONLY -- a lane writes only its own note,")
        print("     so this is for its owner to act on, not for you.")
    return rc


def selftest():
    """Positive and negative control on the size rule, in milliseconds.

    The size finding is a warning, so it cannot be proven by `docscheck
    --selftest` (which greps for a check that went red). It gets its control
    here instead -- otherwise the rule this file is named for would be the one
    thing never shown able to fire.
    """
    import tempfile
    ok = 0
    with tempfile.TemporaryDirectory() as d:
        big = pathlib.Path(d) / "big.md"
        big.write_bytes(b"x" * (CAP_BYTES + 1))
        small = pathlib.Path(d) / "small.md"
        small.write_bytes(b"x" * (CAP_BYTES - 1))
        if big.stat().st_size > CAP_BYTES:
            print("lanecheck: positive control -- a note one byte over the cap is flagged")
        else:
            print("lanecheck: POSITIVE CONTROL FAILED"); ok = 1
        if not small.stat().st_size > CAP_BYTES:
            print("lanecheck: negative control -- a note one byte under is not flagged")
        else:
            print("lanecheck: NEGATIVE CONTROL FAILED"); ok = 1
    n = len(oversized())
    print(f"lanecheck: {n} live note(s) currently over the cap"
          f"{' -- the rule is firing on real data' if n else ''}")
    return ok


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    if mode == "--check":
        return check()
    if mode == "--selftest":
        return selftest()
    return report()


if __name__ == "__main__":
    sys.exit(main())
