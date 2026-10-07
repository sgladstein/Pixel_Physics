#!/usr/bin/env python3
"""Read a `planttrace life=1` run: every plant from seed to grave
(`lives.csv`), its sampled economy (`plants.csv.gz`), its starving spells
(`spells.csv`) and its events (`events.txt`).

    python3 scripts/planttrace.py funnel OUT [OUT...] [--species S]
    python3 scripts/planttrace.py deaths OUT [OUT...] [--species S]
    python3 scripts/planttrace.py spells OUT [OUT...] [--species S]
    python3 scripts/planttrace.py life   OUT ID [BORN]

A life is its id and its born frame: ids are slots, reused after a death.
A plant is a life that germinated; a seed is one that never did. Several OUT
directories (one run each, say four seeds) are read and printed one by one,
then pooled.

`funnel`: per species, every life booked once at the furthest stage it
reached -- set as a seed, germinated, established (grew the shoot cells its
species needs before it may set seed), set a seed of its own -- with counts,
`of prev` and `of all`, and how the lives that stopped at each stage ended:
the cause they died of, or `alive` at the end of the run.

`deaths`: every death of a plant or seed, by species, kind and cause; the
age at death (a seed's from its setting, a plant's from its germination) as
median and p10-p90; and for a death a rule declared, the frames from marked
to gone (the remains rotting).

`spells`: per species, every starving spell (starving ticks above zero, read
every `track=` frames): how many plants ever starved, how each spell ended
(`recovered`, the cause the plant died of, or `ongoing`), and its length --
and apart from them, the same for dormant seeds, which starve on the grown
plant's clock (`Reports/open-bugs-handoff.md` §V5).

`life`: one plant's biography -- its row, its events, its sampled economy, its
spells and its offspring. With no BORN, every life that held the id.

Added 2026-10-07 with the ledger (`examples/planttrace.rs`), step 2 of
tracing plants one individual at a time as the ant line traces ants.
"""
import collections
import csv
import gzip
import os
import statistics
import sys

STAGES = ["set as a seed", "germinated", "established", "set a seed"]


def num(v):
    return None if v in ("", None) else int(v)


def lives(out, species=None):
    with open(os.path.join(out, "lives.csv")) as f:
        rows = list(csv.DictReader(f))
    return [r for r in rows if species is None or r["species"] == species]


def stage(r):
    """The furthest stage this life reached, as an index into STAGES."""
    if num(r["first_seed"]) is not None:
        return 3
    if num(r["established"]) is not None:
        return 2
    if num(r["germinated"]) is not None:
        return 1
    return 0


def ending(r):
    return r["cause"] if r["died"] else "alive"


def pct(a, b):
    return f"{100.0 * a / b:5.1f}%" if b else "    -"


def spread(xs):
    if not xs:
        return "-"
    xs = sorted(xs)
    q = lambda p: xs[min(len(xs) - 1, int(p * len(xs)))]
    return f"median {statistics.median(xs):,.0f} (p10 {q(0.1):,.0f}, p90 {q(0.9):,.0f}, n {len(xs):,})"


def top(counter, k=4):
    return ", ".join(f"{name} {n:,}" for name, n in counter.most_common(k)) or "-"


def flags(args, key, default=None):
    return args[args.index(key) + 1] if key in args else default


def funnel_of(rows, title):
    print(f"\n=== {title}")
    by = collections.defaultdict(list)
    for r in rows:
        by[r["species"]].append(r)
    for sp in sorted(by):
        rs = by[sp]
        reached = [sum(1 for r in rs if stage(r) >= i) for i in range(len(STAGES))]
        print(f"  {sp}")
        for i, name in enumerate(STAGES):
            stopped = collections.Counter(ending(r) for r in rs if stage(r) == i)
            prev = reached[i - 1] if i else reached[0]
            print(f"    {name:14} {reached[i]:7,}  of prev {pct(reached[i], prev)}  of all {pct(reached[i], reached[0])}"
                  f"   stopped here {sum(stopped.values()):6,}: {top(stopped)}")


def funnel(outs, args):
    sp = flags(args, "--species")
    pooled = []
    for out in outs:
        rs = lives(out, sp)
        pooled += rs
        funnel_of(rs, out)
    if len(outs) > 1:
        funnel_of(pooled, f"pooled over {len(outs)} runs")


def deaths_of(rows, title):
    print(f"\n=== {title}")
    dead = [r for r in rows if r["died"]]
    table = collections.Counter((r["species"], r["kind"], r["cause"]) for r in dead)
    for (sp, kind) in sorted({(s, k) for s, k, _ in table}):
        causes = collections.Counter({c: n for (s, k, c), n in table.items() if s == sp and k == kind})
        print(f"  {sp:10} {kind:5} {sum(causes.values()):7,}: {top(causes, 8)}")
    print("  age at death (a seed's from its setting, a plant's from its germination):")
    ages = collections.defaultdict(list)
    for r in dead:
        start = num(r["germinated"]) if r["kind"] == "plant" else num(r["born"])
        ages[(r["kind"], r["cause"])].append(num(r["died"]) - start)
    for key in sorted(ages):
        print(f"    {key[0]:5} {key[1]:16} {spread(ages[key])}")
    rot = [num(r["died"]) - num(r["marked"]) for r in dead if r["declared"] == "1" and r["marked"]]
    print(f"  marked to gone, every declared death: {spread(rot)}")


def deaths(outs, args):
    sp = flags(args, "--species")
    pooled = []
    for out in outs:
        rs = lives(out, sp)
        pooled += rs
        deaths_of(rs, out)
    if len(outs) > 1:
        deaths_of(pooled, f"pooled over {len(outs)} runs")


def spells_rows(out, species=None):
    with open(os.path.join(out, "spells.csv")) as f:
        return [r for r in csv.DictReader(f) if species is None or r["species"] == species]


def spells_of(rows, spell_rows, title):
    print(f"\n=== {title}")
    for kind in ("plant", "seed"):
        lived = collections.Counter(r["species"] for r in rows if kind == "seed" or r["kind"] == "plant")
        ss_kind = [s for s in spell_rows if s["kind"] == kind]
        starved = collections.defaultdict(set)
        by = collections.defaultdict(list)
        for s in ss_kind:
            starved[s["species"]].add((s["id"], s["born"]))
            by[s["species"]].append(s)
        what = "plants" if kind == "plant" else "lives, as a dormant seed (§V5)"
        print(f"  starving {what}:")
        for sp in sorted(set(lived) | set(by)):
            ss = by.get(sp, [])
            print(f"    {sp:10} {len(starved[sp]):6,} of {lived[sp]:6,} ({pct(len(starved[sp]), lived[sp])}), {len(ss):,} spells")
            if not ss:
                continue
            ends = collections.Counter(s["outcome"] for s in ss)
            print(f"      ended: {top(ends, 6)}")
            print(f"      length (frames): {spread([int(s['end']) - int(s['start']) for s in ss])}")
            print(f"      worst starving ticks: {spread([int(s['worst']) for s in ss])}")


def spells(outs, args):
    sp = flags(args, "--species")
    pooled, pooled_spells = [], []
    for out in outs:
        rs, ss = lives(out, sp), spells_rows(out, sp)
        pooled += rs
        pooled_spells += ss
        spells_of(rs, ss, out)
    if len(outs) > 1:
        spells_of(pooled, pooled_spells, f"pooled over {len(outs)} runs")


SAMPLE_COLS = ["frame", "cells", "shoot", "root", "income", "upkeep", "unpaid", "starving", "water", "fund", "dying", "tips", "root_tips", "light", "above"]


def life(outs, args):
    out = outs[0]
    rest = [a for a in args[1:] if not a.startswith("--")]
    if not rest:
        sys.exit("life needs an id: planttrace.py life OUT ID [BORN]")
    want_id, want_born = rest[0], (rest[1] if len(rest) > 1 else None)
    rows = [r for r in lives(out) if r["id"] == want_id and (want_born is None or r["born"] == want_born)]
    if not rows:
        sys.exit(f"no life with id {want_id}" + (f" born {want_born}" if want_born else "") + f" in {out}/lives.csv")
    if len(rows) > 1 and want_born is None:
        print(f"{len(rows)} lives held id {want_id}; add BORN to pick one:")
        for r in rows:
            print(f"  born {r['born']:>7} {r['species']:10} {r['kind']:5} {ending(r)}")
        return
    r = rows[0]
    key = (r["id"], r["born"])
    print(f"=== {r['species']} id {r['id']} born {r['born']}, generation {r['generation']}, lineage {r['lineage']}")
    print(f"  parent: {r['parent']} born {r['parent_born']}" if r["parent"] != "0" else "  parent: none (a founder)")
    for k in ["kind", "germinated", "origin_x", "origin_y", "peak_cells", "maturity", "established", "first_seed", "seeds_set",
              "offspring", "offspring_germinated", "offspring_established", "max_starving", "marked", "marked_cause", "died", "cause",
              "declared", "buried"]:
        print(f"  {k:22} {r[k] or '-'}")
    print("  events:")
    with open(os.path.join(out, "events.txt")) as f:
        for line in f:
            if f" id={key[0]} born={key[1]} " in line + " " or line.rstrip().endswith(f"id={key[0]} born={key[1]}"):
                print("    " + line.rstrip())
    print("  sampled economy:")
    print("    " + " ".join(f"{c:>8}" for c in SAMPLE_COLS))
    with gzip.open(os.path.join(out, "plants.csv.gz"), "rt") as f:
        for s in csv.DictReader(f):
            if (s["id"], s["born"]) == key:
                print("    " + " ".join(f"{s[c]:>8}" for c in SAMPLE_COLS))
    print("  starving spells:")
    for s in spells_rows(out):
        if (s["id"], s["born"]) == key:
            print(f"    {s['start']}-{s['end']} worst {s['worst']} -> {s['outcome']}")
    kids = [k for k in lives(out) if (k["parent"], k["parent_born"]) == key]
    print(f"  offspring: {len(kids)}, germinated {sum(1 for k in kids if k['germinated'])}, established {sum(1 for k in kids if k['established'])}")
    for k in kids:
        if k["germinated"]:
            print(f"    id {k['id']} born {k['born']}: germinated {k['germinated']}, peak {k['peak_cells']} cells, {ending(k)}")


if __name__ == "__main__":
    cmds = {"funnel": funnel, "deaths": deaths, "spells": spells, "life": life}
    if len(sys.argv) < 3 or sys.argv[1] not in cmds:
        sys.exit(__doc__)
    args = sys.argv[2:]
    if sys.argv[1] == "life":
        cmds["life"]([args[0]], args)
    else:
        outs = [x for i, x in enumerate(args) if not x.startswith("--") and (i == 0 or args[i - 1] != "--species")]
        cmds[sys.argv[1]](outs, args)
