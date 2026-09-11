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
only date that survives an author forgetting to write one. This needs full
history; a shallow clone silently dates every pre-boundary line to the boundary
commit, so `--check` fails loudly rather than reporting a wrong arc.

    python3 scripts/deadendindex.py            # regenerate TSV + skeleton
    python3 scripts/deadendindex.py --check    # verify counts; gated by docscheck
    python3 scripts/deadendindex.py --skeleton <section>[,<section>]

`--check` also verifies the per-section counts written into the `##` headings.
They are stale today in nine of fourteen sections -- `## other  (20 entries)`
holds **105** -- and a count nobody can check is worse than none, which is the
same reasoning the file's own header gives for recounting its total.
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

    So the durable handle hashes the entry's own address. It survives
    insertion, deletion and reordering, and it changes only when the address
    itself is edited -- which is the case where a human should look anyway.
    The ordinal stays as the readable label; this is what joins data to it."""
    import hashlib
    return hashlib.sha1(e["address"].encode("utf-8")).hexdigest()[:10]


def write_outputs(entries):
    TSV.parent.mkdir(parents=True, exist_ok=True)
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
    TSV.write_text("\n".join(rows) + "\n", encoding="utf-8")
    SKEL.write_text("\n".join(skel) + "\n", encoding="utf-8")
    return per


def main():
    args = sys.argv[1:]
    entries, headers = parse()

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

    per = write_outputs(entries)
    problems = []

    if not any(e["written"] for e in entries):
        problems.append("git blame produced no dates -- run `git fetch --unshallow` first; "
                        "a shallow clone dates every pre-boundary line to the boundary commit")

    for section, claimed, line in headers:
        actual = per.get(section, 0)
        if claimed is not None and claimed != actual:
            problems.append(f"L{line}: `## {section}` claims {claimed} entries, holds {actual}")

    if "--check" in args:
        for p in problems:
            print(f"deadendindex: {p}")
        return 1 if problems else 0

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
