#!/usr/bin/env python3
"""A structured index over `Reports/dead-ends.md`, so its 773 entries can be
triaged without reading 167k tokens of prose.

Why this exists
---------------
`dead-ends.md` is the register of approaches this project tried and rejected.
708 of its entries end with a `*Re-test when:* <condition>` clause naming the
precondition for revival, and `CLAUDE.md` carries the standing convention that
*"a dead end whose condition has since changed is due for re-testing, not
permanently banned"*. Nobody has ever swept those conditions: exactly **three**
entries carry the file's own `CONDITION MET` marker.

The obstacle is size. At ~167k tokens the file cannot be held by one reader, and
its own header says so ("grep the *mechanism* you are about to touch, never your
subsystem"). But the *skeleton* -- each entry's address, its claim, and its
re-test clause -- is **~34k tokens for all 773 entries**, a fifth of the text.
That is holdable. This script extracts it, plus the metadata a triage needs to
stratify, and writes both a TSV and the skeleton.

**It makes no judgements, deliberately.** The obvious automation -- *if entry X
was rejected using instrument Y, and Y is recorded elsewhere in this file as
faulty, then X is suspect* -- was measured and does not work: only 46 entries
name a scene at all, and the instrument-fault entries barely overlap them. So
this script stratifies and routes; the judgement is made by reading. Recording
that here because the cross-reference looks like the whole answer until you
count it.

Provenance dates come from `git blame`, not from the prose: **594 of 773 entries
(77%) cite no date at all**, and 1,060 of the file's 1,678 entry lines were
written on 2026-08-21 by the original ten-agent census. An entry that names no
date is not undated -- it is dated by the commit that added it, which is the
only date that survives an author forgetting to write one.

Regeneration therefore needs full history and **refuses without it**: a shallow
clone silently dates every pre-boundary line to the boundary commit, which is
wrong and plausible at the same time. `--check` does not need full history,
because it compares on the columns blame does not touch -- that is what lets CI
run it at its default checkout depth.

    python3 scripts/deadendindex.py            # regenerate TSV + skeleton (needs full history)
    python3 scripts/deadendindex.py --check    # is the committed index stale? writes nothing
    python3 scripts/deadendindex.py --skeleton <section>[,<section>]
    python3 scripts/deadendindex.py --watch      # clauses waiting on an arrival

`--check` compares the committed TSV and skeleton against a regeneration and
also verifies the per-section counts written into the `##` headings. They were
stale in nine of fourteen sections when this was written -- `## other  (20
entries)` held **105** -- and a count nobody can check is worse than none,
which is the same reasoning the file's own header gives for recounting its
total.

**`--check` used to do neither of those things, and its green meant nothing.**
`main()` called `write_outputs()` before it looked at the flag, so the check
always rewrote both generated files and then compared only the headings against
a count it had just produced itself -- it could not see a stale committed index,
which is the one job CLAUDE.md gives a generated-file gate. In a depth-1 clone
it rewrote all 804 rows of both files with the boundary commit's dates and still
exited 0, and CI's `docscheck` job checks out at default depth, so CI had been
running that regeneration and passing. Three controls, measured 2026-09-12 and
re-runnable: delete one register entry and `--check` goes red on the row count
and the heading (it does, and writes nothing while doing it); run it in a
depth-1 clone and `git status` stays clean (it does -- the old script left 1,585
changed lines in two tracked files); run the *regeneration* there and it refuses
rather than writing wrong dates.
"""

import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOC = ROOT / "Reports" / "dead-ends.md"
TSV = ROOT / "Reports" / "data" / "dead-ends-index.tsv"
SKEL = ROOT / "Reports" / "data" / "dead-ends-skeleton.md"
SCREENED = ROOT / "Reports" / "data" / "dead-ends-triage" / "screened.tsv"
CANDIDATES = ROOT / "Reports" / "data" / "dead-ends-triage" / "candidates.tsv"

# The five verdicts that leave an entry open: the idea may not have had a fair
# test. Everything else (DEAD, META, UNBUILT, LANDED, RE-TESTED) is closed.
CANDIDATE_LABELS = ("CONFOUNDED", "COSTED", "SUSPECT-INSTRUMENT", "EXPIRED", "UNWIRED")
CANDIDATE_COLUMNS = ["label", "id", "line", "confidence", "section", "live_flags",
                     "evidence", "address", "reason"]

# The day six couplings landed at once -- soil nutrient as per-cell data, plants
# held by roots rather than walls, tissue parting, colony groups, kin-as-scent,
# and the plasticity dial. Any plant or creature measurement older than this was
# taken in a world where none of them existed, and every creature null older
# than it was measured at what is now the clonal control (plasticity 0). It is a
# date rather than a commit because the cluster spans 148 commits in one day.
COUPLING_CLUSTER = "2026-09-06"

# Instruments with a *measured* noise floor, and the floor as the file records
# it. An effect smaller than its instrument's floor is not a negative result.
NOISE_FLOOR = {
    "labbatch": "seed alone moves the lab census 2.42-3.12x with no true effect",
    "selection_arena": "resolution floor ~9.3 share-points of noise per seed",
    "labsoil": "2.32x plant cells / 3.12x plants over 12 seeds at fixed settings",
    "seedsweep": "default FRAMES stops at frame 1,202 -- mid-cascade",
    "megastudy": "produced 8 byte-identical logs per species off a stale binary",
    "ascii": "worst-frame moved 6x with machine state; median moved ~30%",
}

SUBSYSTEM = {
    "liquids": r"\b(water|liquid|fill|pool|rain)\b",
    "plants": r"\b(plant|tree|root|leaf|leaves|canopy|seed|species)\b",
    "creatures": r"\b(creature|ant|colony|brain|forag)\w*",
    "structural": r"\b(support|load|structural|torque|footing|beam)\b",
    "destruction": r"\b(fracture|rubble|explosion|collaps|debris)\w*",
    "field": r"\b(field|diffus|nutrient|aux)\w*",
    "weather": r"\b(weather|wind|storm|cloud)\b",
    "rendering": r"\b(render|colour|color|palette|overlay|dirty-rect)\b",
    "worldgen": r"\b(worldgen|cave|terrain|generation pass)\b",
    "powders": r"\b(powder|sand|grain|scree)\b",
}

# Each family is how the *author* qualified the rejection. The vocabulary is
# theirs and it is consistent enough to key on: "holds while X" is 120 entries,
# "never as" 30, "unconditional" 19. Order matters -- PERMANENT is tested first
# because "never, unless" is a permanent claim wearing a conditional's clothes.
QUALIFIER = [
    ("PERMANENT", r"^\**\s*(never|permanent|unconditional|do not retry|research-grounded|grounded in)"),
    ("HOLDS-WHILE", r"^\**\s*(holds?|held)\b"),
    ("ONLY-IF", r"^\**\s*(only|re-test only|retry only)\b"),
    ("TRIGGERED", r"^\**\s*(if|when|once|after|should|any)\b"),
    ("NEEDS-WORK", r"^\**\s*(needs?|requires?|open|not )"),
]

CONDITION_MET = re.compile(r"CONDITION MET", re.I)

# A clause written as "once X exists" / "when X lands" is *forward-looking*: it
# names an arrival rather than a standing state, so it can become true without
# anyone touching the entry. That grammar is the whole signal.
#
# **Measured 2026-09-11, and the obvious stronger filter is worse than useless.**
# The first design asked whether the clause names an identifier that now exists
# in the tree: 123 entries matched, of which **one** was `EXPIRED` -- 0.8%
# against a 1.6% base rate, i.e. worse than random. Clauses name identifiers
# that already existed when they were written ("holds while `X` does Y") far
# more often than they name a future artifact, so existence tells you nothing.
# Adding it to the grammar filter narrows 38 entries to 6 at the same rate: all
# coverage lost, no precision gained. Do not re-add it.
#
# What the grammar alone gives: **38 entries, 16% already resolved
# (EXPIRED/LANDED/RE-TESTED) against a 5% base** -- about 3x enrichment. On
# n=38 with 6 hits against ~2 expected that is suggestive, not established, and
# it is offered as a watchlist rather than a verdict. It cannot be more than
# that: `creatures:039` had its named artifact arrive (a flier) while the
# condition stayed unmet (that flier has no pheromone economy), which no
# textual rule can see.
FORWARD_LOOKING = re.compile(
    r"\b(once|when|after|if) [^.]{0,80}\b(exists?|lands?|ships?|is built|is available|gains?|becomes)",
    re.I,
)
DATE = re.compile(r"(\d{4}-\d{2}-\d{2})")
ENTRY = re.compile(r"^- \*\*")
HEADING = re.compile(r"^## (\S+)\s*(?:\((\d+) entries\))?")


def flat(s):
    return re.sub(r"\s+", " ", s).strip()


def live_flags():
    """Env-var switches actually readable in the tree today.

    An entry citing a flag that is *gone* needs the arm rebuilt before it can be
    re-asked at all, so this column sets a floor on the cost of re-testing it.

    **It does not say the arm can be re-run, and the difference has been
    measured.** `GROUND_ROOT` is the most-cited live flag here (three entries)
    and it is correctly wired -- to a branch the scene never reaches. Paired
    arms on `scene=worldcrack strike=12 preset=rolling seed=7`, 4,502 frames,
    came back byte-identical on every column while `SCHED_PASS=1` reported
    `grounded 0 (flat 0)` on every frame: `structural.rs:463` takes the ground
    root only when a cell relaxes to `u16::MAX` *and* rests on ground, which
    never happens there. The register already recorded the same vacuity for the
    sibling flag `STRUCT_NO_GROUND_ROOT` (`structural:007`).

    So an archived arm needs three steps, not one, and the first two are seconds
    each: does the scene contain the situation (an effect counter non-zero in
    the baseline arm), does the switched path execute (a counter on the far side
    of the switch), and only then, do the arms differ? Skipping them is how a
    byte-identical null gets written up as a negative result."""
    out = subprocess.run(
        ["grep", "-rhoE", r'env::var(_os)?\("[A-Z_0-9]+"', "src", "examples"],
        cwd=ROOT, capture_output=True, text=True,
    ).stdout
    return set(re.findall(r'"([A-Z_0-9]+)"', out))


def blame_dates():
    """line number -> author date, for every line of the register.

    One subprocess for the whole file: per-entry blame would be 773 of them.
    Returns {} if blame fails, and the caller reports that rather than dating
    everything to today."""
    res = subprocess.run(
        ["git", "blame", "--line-porcelain", "--date=short", "--", str(DOC.relative_to(ROOT))],
        cwd=ROOT, capture_output=True, text=True,
    )
    if res.returncode != 0:
        return {}
    dates, line, pending = {}, 0, None
    for row in res.stdout.split("\n"):
        if row.startswith("author-time "):
            pending = int(row.split()[1])
        elif row.startswith("\t"):
            line += 1
            if pending is not None:
                import datetime
                dates[line] = datetime.datetime.utcfromtimestamp(pending).strftime("%Y-%m-%d")
    return dates


def parse():
    text = DOC.read_text(encoding="utf-8")
    lines = text.split("\n")
    dates = blame_dates()
    flags = live_flags()

    entries, section, header_count, headers = [], None, None, []
    buf, buf_line = [], 0

    def flush():
        if section and buf:
            entries.append(build(section, buf_line, "\n".join(buf), dates, flags))

    for i, line in enumerate(lines, 1):
        h = HEADING.match(line)
        if h:
            flush()
            buf = []
            section = h.group(1)
            headers.append((section, int(h.group(2)) if h.group(2) else None, i))
            continue
        if ENTRY.match(line):
            flush()
            buf, buf_line = [line], i
        elif buf:
            buf.append(line)
    flush()
    return entries, headers


def build(section, line, body, dates, flags):
    m = re.match(r"- \*\*(.+?)\*\*", body, re.S)
    address = flat(m.group(1)) if m else flat(body[:100])

    # The claim: everything after the bold address, minus the " - " that joins
    # them. It is what distinguishes two dead ends filed under one report
    # section, and without it the key collides -- see `stable_key`.
    claim = flat(body[m.end():]) if m else ""
    claim = re.sub(r"^-\s*", "", claim)

    rt = re.search(r"\*Re-test when:\*\s*(.+?)(?=\n\s*\*[A-Z]|\Z)", body, re.S)
    retest = flat(rt.group(1)) if rt else ""

    family = "UNCLASSIFIED"
    for name, pat in QUALIFIER:
        if retest and re.search(pat, retest, re.I):
            family = name
            break

    stated = DATE.findall(body)
    stated_date = max(stated) if stated else ""
    written = dates.get(line, "")
    # The later of the two: an entry edited after it was filed carries the newer
    # world in its prose, and it is the newest claim that has to be checked.
    effective = max([d for d in (stated_date, written) if d] or [""])

    numbers = len(re.findall(r"\b\d[\d,.]{2,}\b", body))
    if re.search(r"measured \d{4}-\d{2}", body):
        grade = "DATED-MEASUREMENT"
    elif numbers >= 2:
        grade = "NUMBERS"
    elif re.search(r"\bmeasured\b", body, re.I):
        grade = "CLAIMS-MEASURED"
    else:
        grade = "ARGUED-ONLY"

    cited = {t for t in re.findall(r"\b([A-Z][A-Z_0-9]{3,})\b", body)}
    live = sorted(cited & flags)
    dead_flags = sorted(t for t in cited - flags if re.search(re.escape(t) + r"\s*=", body))

    scenes = sorted(set(re.findall(r"scene=(\w+)", body)))
    noisy = sorted(k for k in NOISE_FLOOR if re.search(r"\b" + k + r"\b", body))
    cross = sorted(k for k, p in SUBSYSTEM.items() if k != section and re.search(p, body, re.I))

    target = ""
    t = re.search(r"((?:src|examples|scripts|tests|assets)/[\w/.\-]+)", address)
    if t:
        target = t.group(1)
    else:
        t = re.search(r"\b(README\.md|PLAN\.md|PLAN-log\.md|CLAUDE\.md|Reports/[\w\-.]+\.md|wiki/[\w\-]+\.md)", address)
        target = t.group(1) if t else ""

    return {
        "section": section,
        "line": line,
        "address": address,
        "claim": claim,
        "target": target,
        "family": family,
        "retest": retest,
        "stated_date": stated_date,
        "written": written,
        "effective": effective,
        "pre_cluster": "Y" if effective and effective < COUPLING_CLUSTER else ("" if effective else "?"),
        "grade": grade,
        "numbers": numbers,
        "live_flags": ",".join(live),
        "dead_flags": ",".join(dead_flags),
        "scenes": ",".join(scenes),
        "noisy_instruments": ",".join(noisy),
        "cross": ",".join(cross),
        "condition_met": "Y" if CONDITION_MET.search(body) else "",
        "chars": len(body),
        "body": body,
    }


COLUMNS = ["id", "key", "section", "line", "family", "grade", "effective", "written", "stated_date",
           "pre_cluster", "condition_met", "numbers", "chars", "target", "live_flags",
           "dead_flags", "scenes", "noisy_instruments", "cross", "address", "retest"]


def ident(e, n):
    return f"{e['section']}:{n:03d}"


def stable_key(e):
    """A content-derived id, because the positional one silently corrupts.

    `section:ordinal` reads well and is wrong: two entries landed mid-`other`
    when `main` was merged on 2026-09-11 and **eight already-labelled ids
    silently changed which entry they pointed at**, among them two revival
    candidates. A triage keyed on ordinals is one merge away from attaching
    every verdict to the wrong entry, and nothing about the file would look
    different afterwards.

    So the durable handle hashes the entry's own content. It survives
    insertion, deletion and reordering, and it changes only when the entry
    itself is edited -- which is the case where a human should look anyway.
    The ordinal stays as the readable label; this is what joins data to it.

    **Section plus address is not enough, and the first version of this
    function got that wrong in a way that moved verdicts onto the wrong
    entries.** The docstring here used to assert that same-key rows were the
    duplicates the register deliberately carries. Measured 2026-09-12 by
    reading all 33 of them: **12 shared keys over 33 rows, of which only the
    two `other:` pairs are genuine duplicates** -- the other 10 groups are 29
    rows that are *distinct dead ends filed under one report-section
    address*. `Reports/next-session-handoff.md §3` alone carries five
    different structural levers; `§1c-i` five different crack-pattern
    attempts; `open-bugs-handoff.md §1 'Whiskers'` four different whisker
    fixes. The join collapsed each group to one `screened.tsv` row, so one
    verdict stood for all of them, and it had already done damage:
    `structural:038`'s write-back (the divisor is gone, `LANDED`) was carried
    by `structural:039`, whose subject is *"stale distances are not this
    bug"* and whose re-test clause demands a `relax=1` re-baseline.

    So the hash takes the **claim** as well -- the first 160 characters after
    the bold address, which is where the subject of the rejection is stated.
    160 is measured rather than chosen: it separates all 29 while staying
    short enough that editing the tail of a long entry does not re-key it.
    Rows that still collide after this are true duplicates and may share a
    verdict, which is what the register asks for by carrying them."""
    import hashlib
    return hashlib.sha1(
        f"{e['section']}\x00{e['address']}\x00{e.get('claim', '')[:160]}".encode("utf-8")
    ).hexdigest()[:10]


# Columns derived from `git blame` rather than from the register's text. A
# shallow clone dates every pre-boundary line to the boundary commit, so these
# three are the ones that differ between a full and a truncated history. The
# compare in `--check` excludes them; the regeneration refuses to run shallow
# at all, so they are never *written* wrong either.
BLAME_COLUMNS = ("written", "effective", "pre_cluster")

# The same three, as they appear in the skeleton's tag list.
SKEL_BLAME = re.compile(r"(?:; )?(?:eff=[^;)]*|pre-09-06)")


def is_shallow():
    res = subprocess.run(["git", "rev-parse", "--is-shallow-repository"],
                         cwd=ROOT, capture_output=True, text=True)
    return res.returncode == 0 and res.stdout.strip() == "true"


def render_outputs(entries):
    """Build the TSV and skeleton *in memory*. Writes nothing.

    Split out of `write_outputs` so `--check` can compare without touching the
    tree. `--check` used to call `write_outputs` before it looked at its own
    flag, so it always rewrote both files and compared nothing but the `##`
    heading counts -- it could not tell a stale committed index from a fresh
    one, which is the single job CLAUDE.md gives a generated-file gate."""
    per = Counter()
    rows = ["\t".join(COLUMNS)]
    skel = ["# dead-ends.md skeleton -- generated by scripts/deadendindex.py",
            "",
            "Address, re-test clause and metadata for every entry. Open the full",
            "entry with `sed -n '<line>,+<n>p' Reports/dead-ends.md` when the",
            "skeleton is not enough. Do not edit this file by hand.",
            ""]
    current = None
    for e in entries:
        per[e["section"]] += 1
        eid = ident(e, per[e["section"]])
        rows.append("\t".join([eid, stable_key(e)]
                              + [str(e.get(c, "")).replace("\t", " ")
                                 for c in COLUMNS[2:]]))
        if e["section"] != current:
            current = e["section"]
            skel += ["", f"## {current}", ""]
        tags = [e["family"], e["grade"], f"eff={e['effective'] or '?'}"]
        if e["pre_cluster"] == "Y":
            tags.append("pre-09-06")
        if e["live_flags"]:
            tags.append(f"live-flag={e['live_flags']}")
        if e["noisy_instruments"]:
            tags.append(f"noisy={e['noisy_instruments']}")
        if e["cross"]:
            tags.append(f"cross={e['cross']}")
        if e["condition_met"]:
            tags.append("ALREADY-CONDITION-MET")
        skel.append(f"- **[{eid}] L{e['line']}** ({'; '.join(tags)})")
        skel.append(f"  - addr: {e['address'][:180]}")
        if e["retest"]:
            skel.append(f"  - retest: {e['retest'][:260]}")
    return per, "\n".join(rows) + "\n", "\n".join(skel) + "\n"


def render_candidates(entries):
    """`candidates.tsv`, derived from `screened.tsv` rather than maintained.

    It used to be hand-maintained, and it drifted: measured 2026-09-12 it
    carried **61 CONFOUNDED rows against screened.tsv's 59**, and its first row
    was `creatures:039 EXPIRED` -- an entry the PR body, a PR comment and its
    own check file all withdraw. A later session reading the table re-walks a
    candidate that was already closed, which is the whole cost of a derived
    file that is not derived.

    Joined on the content key, never on the `section:ordinal` id: one
    `screened.tsv` row already carried a stale ordinal (`other:107` for what is
    now `other:110`)."""
    if not SCREENED.exists():
        return None
    rows = [l.rstrip("\n").split("\t") for l in
            SCREENED.read_text(encoding="utf-8").rstrip("\n").split("\n")]
    hdr, data = rows[0], rows[1:]
    ki, li, ci, ri = (hdr.index(c) for c in ("key", "label", "confidence", "reason"))
    verdict = {r[ki]: r for r in data}

    per = Counter()
    out = ["\t".join(CANDIDATE_COLUMNS)]
    for e in entries:
        per[e["section"]] += 1
        v = verdict.get(stable_key(e))
        if not v or v[li] not in CANDIDATE_LABELS:
            continue
        out.append("\t".join(str(x).replace("\t", " ") for x in [
            v[li], ident(e, per[e["section"]]), e["line"], v[ci], e["section"],
            e["live_flags"], e["grade"], e["address"], v[ri]]))
    return "\n".join(out) + "\n"


def write_outputs(entries):
    """Regenerate both committed artifacts. Refuses on a shallow clone.

    The dates in these files come from blame over the whole history of the
    register. In a depth-1 clone blame reports the boundary commit for every
    pre-boundary line, so a regeneration there rewrites all 804 rows with
    dates that are wrong and plausible. Refusing is the only safe answer: the
    old code only failed when blame returned *no* dates, which a shallow blame
    never does."""
    if is_shallow():
        print("deadendindex: refusing to regenerate in a shallow clone -- `git blame` "
              "dates every pre-boundary line to the boundary commit, so `written`, "
              "`effective` and `pre_cluster` would all be written wrong. "
              "Run `git fetch --unshallow` first.", file=sys.stderr)
        return None
    TSV.parent.mkdir(parents=True, exist_ok=True)
    per, tsv, skel = render_outputs(entries)
    TSV.write_text(tsv, encoding="utf-8")
    SKEL.write_text(skel, encoding="utf-8")
    cand = render_candidates(entries)
    if cand is not None:
        CANDIDATES.write_text(cand, encoding="utf-8")
    return per


def strip_blame_tsv(text):
    """TSV rows with the blame-derived columns blanked, for comparison."""
    lines = text.rstrip("\n").split("\n")
    if not lines or not lines[0]:
        return []
    head = lines[0].split("\t")
    drop = {head.index(c) for c in BLAME_COLUMNS if c in head}
    return [tuple("" if i in drop else v for i, v in enumerate(r.split("\t")))
            for r in lines]


def strip_blame_skel(text):
    return [SKEL_BLAME.sub("", line) for line in text.rstrip("\n").split("\n")]


def compare_committed(entries):
    """Is what is committed what this script would generate? Writes nothing.

    Compared on everything except the blame-derived columns, so the answer is
    the same in a shallow clone as in a full one -- which is what lets CI run
    this at its default checkout depth."""
    problems = []
    per, tsv, skel = render_outputs(entries)
    if SCREENED.exists():
        rows = SCREENED.read_text(encoding="utf-8").rstrip("\n").split("\n")
        judged = {r.split("\t")[0] for r in rows[1:]}
        missing = [e for e in entries if stable_key(e) not in judged]
        if missing:
            # An entry added to the register after the screen ran is silently
            # absent from `candidates.tsv` -- it cannot be a candidate, because
            # nothing labelled it. That reads as "screened and closed".
            per_ = Counter()
            names = []
            for e in entries:
                per_[e["section"]] += 1
                if e in missing:
                    names.append(ident(e, per_[e["section"]]))
            problems.append(
                f"{len(missing)} register entr"
                f"{'y has' if len(missing) == 1 else 'ies have'} no verdict in "
                f"{SCREENED.relative_to(ROOT)}: {', '.join(names[:8])}"
                f"{' ...' if len(names) > 8 else ''}")
    checks = [(TSV, tsv, strip_blame_tsv, "index"),
              (SKEL, skel, strip_blame_skel, "skeleton")]
    cand = render_candidates(entries)
    if cand is not None:
        checks.append((CANDIDATES, cand, lambda t: t.rstrip("\n").split("\n"), "candidates"))
    for path, fresh, strip, label in checks:
        if not path.exists():
            problems.append(f"{path.relative_to(ROOT)} is missing -- run `deadendindex.py`")
            continue
        have = strip(path.read_text(encoding="utf-8"))
        want = strip(fresh)
        if have == want:
            continue
        if len(have) != len(want):
            problems.append(
                f"{path.relative_to(ROOT)} is stale: holds {len(have)} lines, "
                f"the register generates {len(want)} -- run `deadendindex.py`")
            continue
        bad = [i for i, (a, b) in enumerate(zip(have, want)) if a != b]
        problems.append(
            f"{path.relative_to(ROOT)} is stale: {len(bad)} of {len(want)} lines differ "
            f"(first at line {bad[0] + 1}) -- run `deadendindex.py`")
    return per, problems


def heading_problems(headers, per):
    out = []
    for section, claimed, line in headers:
        actual = per.get(section, 0)
        if claimed is not None and claimed != actual:
            out.append(f"L{line}: `## {section}` claims {claimed} entries, holds {actual}")
    return out


# Identifier-shaped tokens: snake_case, CamelCase, or SCREAMING_SNAKE. Plain
# English words are excluded by construction rather than by a stopword list,
# which is what the first version needed and still leaked through -- backticked
# *paths* (`Reports/open-bugs-handoff.md`) tokenise into "Reports", "open",
# "bugs", and those matched everything.
_CODEISH = re.compile(
    r"\b(?:[a-z][a-z0-9]*(?:_[a-z0-9]+)+"      # snake_case
    r"|[A-Z][a-z0-9]+(?:[A-Z][a-z0-9]+)+"       # CamelCase
    r"|[A-Z][A-Z0-9]*_[A-Z0-9_]+)\b"           # SCREAMING_SNAKE
)


def clause_identifiers(retest):
    """Identifier-shaped tokens anywhere in a `Re-test when:` clause.

    **Not only the backticked ones, and that was measured.** Requiring
    backticks lost four of the five replay controls outright: `plants:124`
    names its identifier as bare prose (*"a monotone high-water memory
    (q_peak girth memory)"*), and so does `structural:038`. The house voice
    backticks a *file* far more reliably than it backticks the thing a clause
    is waiting for."""
    return set(_CODEISH.findall(retest or ""))


def git_out(args):
    res = subprocess.run(["git"] + args, cwd=ROOT, capture_output=True, text=True)
    return None if res.returncode != 0 else res.stdout


def _existed(token, rev):
    return subprocess.run(
        ["git", "grep", "-qwF", "--", token, rev, "--", "src", "examples", "assets", "scripts"],
        cwd=ROOT, capture_output=True, text=True).returncode == 0


def touching(entries, rev_range):
    """Which register entries does this branch's diff speak to?

    **The question is event-shaped, and the test is *arrival*, not presence.**
    An earlier attempt asked a static question -- does the clause name an
    identifier that exists in the tree today -- and measured **worse than
    random**: 123 entries matched, one `EXPIRED`, 0.8% against a 1.6% base
    rate. Clauses name identifiers that already existed far more often than
    they name a future artifact.

    So an entry surfaces when its clause names a code-shaped identifier that
    this diff **adds to a tree that did not have it**: in the added lines, and
    absent from `src`/`examples`/`assets`/`scripts` at the base revision.

    **Presence in the added lines alone is not enough, and the cost of
    relaxing to it was measured 2026-09-12.** That form finds all five replay
    controls and is unusable: **43 hits on one plant-line commit against a
    single true positive**, and 5, 8, 17, 18 and 24 hits on five unrelated
    merged PRs, every one a false positive. The arrival test scores **0, 0, 0,
    0, 1** on the same five. It is recorded as a dead end beside the static
    attempt.

    **Measured recall is 2 of 5, and the three misses are structural rather
    than tuning.** `plants:124` surfaces on the commit that added `q_peak`
    (4 hits) and `structural:038` on the one that added `bearing_moment`
    (2 hits). It cannot see:

    - an entry whose clause never names the identifier (`structural:040`),
      nor one with no `Re-test when:` clause at all -- `plants:019`'s
      condition was met when `cross_section_axis` landed and there is nowhere
      for the name to appear. That control must stay silent, and does.
    - a condition met by something that is **not** an arrival: `field:027`'s
      `apply_sky_to` and `destruction:034`'s `ignition_temperature` both
      already existed, and what changed was behaviour and authored values. No
      name-matching rule can see "this field now has a finite value".

    Giving the clause-less entries a clause is the only thing that closes the
    first gap; the second is not closable this way at all."""
    names = git_out(["diff", "--name-only", rev_range])
    if names is None:
        print(f"deadendindex: `git diff --name-only {rev_range}` failed", file=sys.stderr)
        return None
    base = rev_range.split("..")[0].rstrip(".") or "origin/main"
    # Only code and assets, and for a reason that bit on the first run: a diff
    # that *edits the register* adds every identifier it quotes, so writing a
    # `Re-test when:` clause made three unrelated entries match it. Scanning
    # the same paths `_existed` scans keeps the arrival question about the
    # engine rather than about the prose describing it.
    body = git_out(["diff", "--unified=0", rev_range, "--",
                    "src", "examples", "assets", "scripts", "tests"]) or ""
    added = "\n".join(l[1:] for l in body.split("\n")
                      if l.startswith("+") and not l.startswith("+++"))

    wanted = set()
    for e in entries:
        wanted |= clause_identifiers(e["retest"])
    in_added = {t for t in wanted if re.search(r"\b" + re.escape(t) + r"\b", added)}
    arrived = {t for t in in_added if not _existed(t, base)}

    per, hits = Counter(), []
    for e in entries:
        per[e["section"]] += 1
        m = sorted(clause_identifiers(e["retest"]) & arrived)
        if m:
            hits.append((ident(e, per[e["section"]]), e, m))
    return len([n for n in names.split("\n") if n]), arrived, hits


def print_touching(entries, rev_range, brief=False):
    res = touching(entries, rev_range)
    if res is None:
        return 1
    nfiles, arrived, hits = res
    if brief:
        if hits:
            print(f"deadendindex: {len(hits)} register entr"
                  f"{'y names' if len(hits) == 1 else 'ies name'} something this branch "
                  f"ADDS -- run `python3 scripts/deadendindex.py --touching`")
        return 0
    print(f"deadendindex: {nfiles} file(s) changed in {rev_range}; "
          f"{len(arrived)} identifier(s) arrived; {len(hits)} entr"
          f"{'y' if len(hits) == 1 else 'ies'} name one.")
    print("deadendindex: a met condition is a WRITE-BACK first -- a re-test only where the "
          "entry asks for one. Recall is 2 of 5 on replay; silence is not evidence.")
    for eid, e, m in hits:
        print(f"\n  [{eid}] L{e['line']}  <- {', '.join(m)}")
        print(f"      {e['address'][:110]}")
        print(f"      retest: {e['retest'][:220]}")
    return 0


def main():
    args = sys.argv[1:]
    entries, headers = parse()

    if "--touching" in args:
        i = args.index("--touching")
        rest = [a for a in args[i + 1:] if not a.startswith("--")]
        rev_range = rest[0] if rest else "origin/main...HEAD"
        return print_touching(entries, rev_range, brief="--brief" in args)

    if "--skeleton" in args:
        want = set(args[args.index("--skeleton") + 1].split(","))
        per = Counter()
        for e in entries:
            per[e["section"]] += 1
            if e["section"] in want:
                print(f"[{ident(e, per[e['section']])}] L{e['line']} {e['family']}/{e['grade']} "
                      f"eff={e['effective'] or '?'}")
                print(f"  addr: {e['address']}")
                if e["retest"]:
                    print(f"  retest: {e['retest']}")
        return 0

    if "--watch" in args:
        watch = [e for e in entries
                 if e["retest"] and FORWARD_LOOKING.search(e["retest"])
                 and not CONDITION_MET.search(e["retest"])]
        print(f"deadendindex: {len(watch)} entries whose re-test clause names an *arrival* "
              f"rather than a standing state.")
        print("deadendindex: a watchlist, not a verdict -- the named thing arriving is not "
              "the condition being met.")
        print("deadendindex: and not coverage of anything -- measured 2026-09-12, none of the "
              "six entries the triage's own run-order kept is on this list.")
        per = Counter()
        for e in entries:
            per[e["section"]] += 1
            if e in watch:
                print(f"\n  [{ident(e, per[e['section']])}] L{e['line']}  {e['address'][:110]}")
                print(f"      {e['retest'][:220]}")
        return 0

    if "--check" in args:
        # Never writes. The old version called `write_outputs` first, so it
        # dirtied the tree on every run and compared nothing but the heading
        # counts -- including in CI, whose checkout is shallow.
        per, problems = compare_committed(entries)
        problems += heading_problems(headers, per)
        for p in problems:
            print(f"deadendindex: {p}")
        return 1 if problems else 0

    per = write_outputs(entries)
    if per is None:
        return 1
    problems = heading_problems(headers, per)

    print(f"deadendindex: {len(entries)} entries -> {TSV.relative_to(ROOT)}, {SKEL.relative_to(ROOT)}")
    print(f"deadendindex: skeleton {SKEL.stat().st_size:,} B (~{SKEL.stat().st_size // 4:,} tok) "
          f"vs register {DOC.stat().st_size:,} B (~{DOC.stat().st_size // 4:,} tok)")
    fam = Counter(e["family"] for e in entries)
    print("deadendindex: qualifier families " + ", ".join(f"{k} {v}" for k, v in fam.most_common()))
    grade = Counter(e["grade"] for e in entries)
    print("deadendindex: evidence " + ", ".join(f"{k} {v}" for k, v in grade.most_common()))
    print(f"deadendindex: dated before {COUPLING_CLUSTER}: "
          f"{sum(1 for e in entries if e['pre_cluster'] == 'Y')}")
    for p in problems:
        print(f"deadendindex: {p}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
