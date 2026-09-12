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
import subprocess
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


def _series(ident):
    """`Z17` -> `("Z", 17)`; anything not a lettered-numbered pair -> None.

    Bare letters (`### G.`) and the historic oddities (`1h`) have no successor
    to compute, so they are excluded rather than guessed at.
    """
    m = re.fullmatch(r"([A-Za-z]+)([0-9]+)", ident)
    return (m.group(1).upper(), int(m.group(2))) if m else None


def branch_registers():
    """Every fetched ref's copy of the register, as `{ref: [(ident, title)]}`.

    Reads `git show <ref>:Reports/open-bugs-handoff.md` over `refs/remotes/`
    and `refs/heads/`. Refs with no copy of the file are skipped silently and
    counted -- the `review-queue` orphan carries data rather than source, and
    branches cut before the register existed have none either.
    """
    root = DOC.resolve().parent.parent
    rel = DOC.resolve().relative_to(root).as_posix()
    refs = subprocess.run(
        ["git", "for-each-ref", "--format=%(refname:short)",
         "refs/remotes/", "refs/heads/"],
        capture_output=True, text=True, cwd=root, check=False,
    ).stdout.split()
    out, missing = {}, 0
    for ref in refs:
        blob = subprocess.run(
            ["git", "show", f"{ref}:{rel}"],
            capture_output=True, text=True, cwd=root, check=False,
        )
        if blob.returncode != 0 or not blob.stdout:
            missing += 1
            continue
        out[ref] = [
            (ident, title)
            for ident, _s, title, _ln, is_bug in entries(blob.stdout.split("\n"))
            if is_bug and ident != "--"
        ]
    return out, missing


def landed_refs():
    """Refs wholly contained in `origin/main`, or `None` if unanswerable.

    **`None` in a shallow clone, and that is the whole reason this returns an
    option rather than a set.** `--merged` needs the common ancestor, and a
    clone cut at depth 413 does not have it for a branch older than the
    boundary: measured 2026-09-12, 7 refs of 75 came back merged and the other
    68 read as unlanded whether they were or not. A set with 68 wrong members
    is not a degraded answer, it is a confident wrong one -- the collision
    report built on it named 8 collisions of which 7 were superseded letters
    on branches that landed weeks ago.
    """
    root = DOC.resolve().parent.parent
    shallow = subprocess.run(
        ["git", "rev-parse", "--is-shallow-repository"],
        capture_output=True, text=True, cwd=root, check=False,
    )
    if shallow.stdout.strip() == "true":
        return None
    for trunk in ("origin/main", "main"):
        r = subprocess.run(
            ["git", "for-each-ref", "--merged", trunk,
             "--format=%(refname:short)", "refs/remotes/", "refs/heads/"],
            capture_output=True, text=True, cwd=root, check=False,
        )
        if r.returncode == 0 and r.stdout.strip():
            return set(r.stdout.split()) | {trunk}
    return None


def branch_collisions(per_branch, landed=frozenset(), trunk="origin/main"):
    """`{ident: {title: [refs]}}` for identifiers that will be ambiguous.

    Same identifier plus same title is the *same entry* seen on two branches,
    which is what a merged or shared branch looks like and is not a problem.
    Two different titles under one letter is the collision -- an inbound
    "see §Z16" then resolves to whichever branch the reader is standing on.

    **A title carried only by branches already contained in the trunk is
    history, not a collision, and reporting it buries the one that matters.**
    Measured 2026-09-12 on 75 refs: the unfiltered sweep found 8 collisions of
    which 7 were superseded letters on long-merged branches -- a live §Z16
    against a wall of noise. A merged branch cannot introduce anything the
    trunk does not already say, so only titles with an unlanded ref can still
    make an identifier ambiguous. The trunk's own title always counts, since
    that is the one an inbound reference resolves to today.
    """
    claims = {}
    for ref, rows in per_branch.items():
        for ident, title in rows:
            claims.setdefault(ident, {}).setdefault(title, []).append(ref)
    out = {}
    for ident, titles in claims.items():
        live = {
            t: refs for t, refs in titles.items()
            if trunk in refs or any(r not in landed for r in refs)
        }
        if len(live) > 1:
            out[ident] = live
    return out


def next_free(per_branch):
    """`{prefix: n}` -- the lowest number no branch has claimed in each series.

    This is the number the tool exists to print. `--check` cannot supply it:
    it reads one working tree, so it says a letter is free when the only
    thing holding it is a branch that has not landed yet. Measured 2026-09-12:
    a lane filed §Z16 against a register whose highest was §Z15, `--check`
    passed, and §Z16 was already taken on an unmerged branch.
    """
    high = {}
    for rows in per_branch.values():
        for ident, _title in rows:
            s = _series(ident)
            if s:
                high[s[0]] = max(high.get(s[0], 0), s[1])
    return {k: v + 1 for k, v in high.items()}


def report_branches(per_branch, missing):
    """Print the sweep. Returns an exit code: 1 on a collision, 2 if blind.

    **A sweep that read nothing reports no collisions, which looks exactly
    like a clean result** -- the null this repo keeps paying for. So the
    branch count is printed unconditionally and an empty sweep is an error,
    never a pass.
    """
    if not per_branch:
        print("bugindex: --branches read ZERO copies of the register. "
              "Not a clean result -- the sweep is blind. Is this a git tree, "
              "and have you fetched?")
        return 2
    print(f"bugindex: swept {len(per_branch)} ref(s) carrying the register "
          f"({missing} ref(s) have no copy). Refs you have NOT fetched are "
          f"invisible -- `git fetch --all` first if this matters.")
    free = next_free(per_branch)
    if free:
        cols = ", ".join(f"§{k}{v}" for k, v in sorted(free.items()))
        print(f"bugindex: next free identifier in each series -- {cols}")
    landed = landed_refs()
    if landed is None:
        print("bugindex: collision report SUPPRESSED -- this clone cannot tell "
              "a landed branch from an unlanded one (shallow), so every "
              "superseded letter on an old branch would read as live. "
              "`git fetch --unshallow` to enable it. "
              "The next-free line above does not depend on it.")
        return 0
    dups = branch_collisions(per_branch, landed)
    for ident, titles in sorted(dups.items()):
        print(f"bugindex: identifier '{ident}' is titled "
              f"{len(titles)} different ways and both can still land:")
        for title, refs in sorted(titles.items()):
            short = title if len(title) <= 62 else title[:59] + "..."
            # Name the *unlanded* refs. A merged ref in the list is the same
            # entry riding along and says nothing about who has to renumber.
            unlanded = sorted(r for r in refs if r not in landed)
            where = ", ".join(unlanded[:3]) or "the trunk"
            if len(unlanded) > 3:
                where += f" (+{len(unlanded) - 3} more)"
            print(f"    {short!r}")
            print(f"        on {where}")
    if dups:
        print("bugindex: renumber the entry that has NOT landed on main -- "
              "`--branches` again after pushing to confirm.")
        return 1
    print("bugindex: no identifier is titled two ways on anything unlanded")
    return 0


def selftest(per_branch):
    """Put the fault back and watch it go red -- over the comparison, not the
    plumbing. The plumbing's control is the branch count `report_branches`
    prints, which is why that is unconditional and an empty sweep is fatal.
    """
    if not per_branch:
        print("bugindex: --selftest needs a non-empty sweep")
        return 2
    ident = sorted(next_free(per_branch))[0] if next_free(per_branch) else None
    if ident is None:
        print("bugindex: --selftest found no numbered series to fake")
        return 2
    real = dict(per_branch)
    taken = sorted(
        i for rows in per_branch.values() for i, _t in rows if _series(i)
    )[0]
    real["refs/synthetic/selftest"] = [(taken, "a title no real entry carries")]
    landed = landed_refs() or frozenset()
    if taken not in branch_collisions(real, landed):
        print(f"bugindex: SELFTEST FAILED -- an injected second title for "
              f"'{taken}' was not reported. The sweep is blind.")
        return 1
    clean = branch_collisions(dict(per_branch), landed)
    print(f"bugindex: selftest OK -- an injected duplicate of '{taken}' is "
          f"caught; the unmodified sweep reports {len(clean)} collision(s)")
    if landed_refs() is None:
        print("bugindex: note -- this clone is shallow, so `--branches` will "
              "suppress its collision report. The selftest bypasses that "
              "filter deliberately: it is a control over the comparison.")
    return 0


def main():
    # `"--check" in sys.argv` was the first version, and it fails the way this
    # repo has already paid for: "an unknown argument is silently ignored"
    # (CLAUDE.md). `--chekc` or `--check=1` missed the membership test and fell
    # through to the *write* path, rewriting a 343 KB co-owned file for someone
    # who meant to verify it. Unrecognised argv is now an error.
    args = set(sys.argv[1:])
    check = "--check" in args
    unknown = args - {"--check", "--branches", "--selftest"}
    if unknown:
        print(f"bugindex: unrecognised argument(s): {' '.join(sorted(unknown))}")
        print("usage: bugindex.py [--check] [--branches] [--selftest]")
        return 2

    # `--branches` is deliberately NOT run by `docscheck.sh`. Its answer
    # depends on which refs happen to be fetched, so as a gate it would pass
    # or fail on the state of somebody's clone rather than on the content --
    # and a gate whose verdict moves with your fetch state teaches people to
    # ignore it. It is the command you run *before filing a bug*, which is the
    # one moment its answer is actionable.
    if "--branches" in args or "--selftest" in args:
        per_branch, missing = branch_registers()
        if "--selftest" in args:
            return selftest(per_branch)
        return report_branches(per_branch, missing)

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
