#!/usr/bin/env python3
"""Read a `planttrace` run: from `life=1`, every plant from seed to grave
(`lives.csv`), its sampled economy (`plants.csv.gz`), its starving spells
(`spells.csv`) and its events (`events.txt`); from `growlog=1`, every
decision its growing tips made (`growlog.csv.gz`).

    python3 scripts/planttrace.py funnel OUT [OUT...] [--species S]
    python3 scripts/planttrace.py deaths OUT [OUT...] [--species S]
    python3 scripts/planttrace.py spells OUT [OUT...] [--species S]
    python3 scripts/planttrace.py tips   OUT [OUT...] [--species S]
    python3 scripts/planttrace.py life   OUT ID [BORN]
    python3 scripts/planttrace.py tip    OUT ID [X Y]
    python3 scripts/planttrace.py check  OUT [OUT...] [--drop cont|light|wind|up|crowd]

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

`tips`: why growing tips grow or stop (`plant::GrowWhy`), per species,
shoot and root apart: every visit of the `Grow` rule by reason, every tip
retired by reason (the reason of its last stale visit), and how many plants'
tips stopped mainly on each. A tip retires after four visits in a row that
found nowhere to go; `height_limit` is the turgor bound, `too_poor` carbon,
`boxed` no cell to grow into, `ground_too_hard` none it could pay to enter,
`no_good_direction` every way open scored against it, `root_share` a root
on a plant with as much root as its shoot can feed, `tip_cap` the species'
cap on growing tips. `not_asked` is a gap in the census and should be zero.

`life`: one plant's biography -- its row, its events, its sampled economy, its
spells and its offspring. With no BORN, every life that held the id.

`tip` and `check` read `growlog.csv.gz` (`planttrace growlog=1`), every
visit of a tip's `Grow` rule as production computed it. `tip OUT ID` lists a
plant's visits by reason and the cells its tips started from; `tip OUT ID X Y`
follows the tip at that cell visit by visit -- what it could pay for, how each
direction scored, the draw, where it went -- until it retires or the log ends.
`check` rebuilds every logged score from its steering terms and every logged
pick from its draw, in `f32` (rounding after every operation, which is exact
for the operations the rule uses), and exits 1 on any mismatch. `--drop TERM`
rebuilds with one term's weight at zero: it must fail, which is the control
that says the check can see a missing term. Wind is the exception in the lab:
the sealed box has none (every logged wind direction is zero), so dropping it
changes nothing and the check passes -- correctly, and only a scene with wind
can test that term.

Added 2026-10-07 with the ledger (`examples/planttrace.rs`), step 2 of
tracing plants one individual at a time as the ant line traces ants; `tips`
came with step 3 and `tip`/`check` with the decision log, step 4.
"""
import collections
import math
import struct
import csv
import gzip
import os
import signal
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


def census_rows(out, species=None):
    with open(os.path.join(out, "census.csv")) as f:
        return [r for r in csv.DictReader(f) if species is None or r["species"] == species]


def tips_of(census, rows, title):
    print(f"\n=== {title}")
    by = collections.defaultdict(lambda: collections.defaultdict(lambda: [0, 0]))
    for r in census:
        cell = by[(r["species"], r["kind"])][r["why"]]
        cell[0] += int(r["visits"])
        cell[1] += int(r["retired"])

    def shares(d, i, total):
        ranked = sorted(((v[i], why) for why, v in d.items() if v[i] > 0), reverse=True)
        return ", ".join(f"{why} {100.0 * n / total:.0f}%" for n, why in ranked[:5]) or "-"

    for (sp, kind) in sorted(by):
        d = by[(sp, kind)]
        visits = sum(v[0] for v in d.values())
        retired = sum(v[1] for v in d.values())
        print(f"  {sp:10} {kind:5} {visits:9,} visits:  {shares(d, 0, visits)}")
        print(f"  {'':10} {'':5} {retired:9,} retired: {shares(d, 1, retired)}")
    plants = [r for r in rows if r["kind"] == "plant"]
    print("  plants by the reason most of their tips stopped on:")
    for kind in ("shoot", "root"):
        col = f"stopped_{kind}"
        for sp in sorted({r["species"] for r in plants}):
            c = collections.Counter(r[col] or "none stopped" for r in plants if r["species"] == sp)
            print(f"    {sp:10} {kind:5} {top(c, 5)}")


def tips(outs, args):
    sp = flags(args, "--species")
    pooled_c, pooled_r = [], []
    for out in outs:
        c, r = census_rows(out, sp), lives(out, sp)
        pooled_c += c
        pooled_r += r
        tips_of(c, r, out)
    if len(outs) > 1:
        tips_of(pooled_c, pooled_r, f"pooled over {len(outs)} runs")


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
              "declared", "buried", "retired_shoot", "retired_root", "stopped_shoot", "stopped_root"]:
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


F32 = struct.Struct("f")


def f32(x):
    """Round to the nearest `f32`, as the rule's arithmetic does."""
    return F32.unpack(F32.pack(x))[0]


def growlog(out):
    with gzip.open(os.path.join(out, "growlog.csv.gz"), "rt") as f:
        yield from csv.DictReader(f)


def scored_of(r):
    found = []
    for part in filter(None, r["scored"].split("|")):
        dx, dy, density, score = part.split(":")
        found.append((int(dx), int(dy), f32(float(density)), f32(float(score))))
    return found


def normalize(x, y):
    length = f32(math.sqrt(f32(f32(x * x) + f32(y * y))))
    if length < 1e-6:
        return (0.0, 0.0)
    return (f32(x / length), f32(y / length))


def dot(a, b):
    return f32(f32(a[0] * b[0]) + f32(a[1] * b[1]))


TERMS = ("cont", "light", "wind", "up", "crowd")


def rebuild(r, drop=None):
    """Every scored direction of one visit, rebuilt the way `Grow` scores
    it: the four steering terms over their weight sum, divided by the
    crowding term. `drop` zeroes one term's weight."""
    v = lambda k: f32(float(r[k]))
    terms = [(v("hx"), v("hy")), (v("px"), v("py")), (v("wx"), v("wy")), (v("ux"), v("uy"))]
    weights = [v("w_cont"), v("w_light"), v("w_wind"), v("w_up")]
    crowd = v("w_crowd")
    if drop in TERMS[:4]:
        weights[TERMS.index(drop)] = 0.0
    elif drop == "crowd":
        crowd = 0.0
    total_weight = v("w_sum")
    built = []
    for dx, dy, density, logged in scored_of(r):
        d = normalize(float(dx), float(dy))
        pref = f32(dot(d, terms[0]) * weights[0])
        if r["rigid"] != "1":
            for term, weight in zip(terms[1:], weights[1:]):
                pref = f32(pref + f32(dot(d, term) * weight))
        pref = f32(pref / total_weight)
        built.append((dx, dy, f32(pref / f32(1.0 + f32(density * crowd))), logged))
    return built


def replay(r, built):
    """The cell the logged draw picks among the rebuilt positive scores."""
    positive = [(dx, dy, s) for dx, dy, s, _ in built if s > 0]
    if not positive:
        return None
    total = 0.0
    for _, _, s in positive:
        total = f32(total + s)
    pick = f32(f32(int(r["draw"]) / 10000.0) * total)
    chosen = positive[0]
    for c in positive:
        if pick < c[2]:
            chosen = c
            break
        pick = f32(pick - c[2])
    return (int(r["x"]) + chosen[0], int(r["y"]) + chosen[1])


def check(outs, args):
    drop = flags(args, "--drop")
    if drop is not None and drop not in TERMS:
        sys.exit(f"--drop takes one of {', '.join(TERMS)}")
    bad = 0
    for out in outs:
        visits = scores = picks = wrong_scores = wrong_picks = 0
        worst = 0.0
        for r in growlog(out):
            visits += 1
            built = rebuild(r, drop)
            for _, _, score, logged in built:
                scores += 1
                if F32.pack(score) != F32.pack(logged):
                    wrong_scores += 1
                    worst = max(worst, abs(score - logged) / max(abs(logged), 1e-30))
            if r["draw"]:
                picks += 1
                if replay(r, built) != (int(r["chosen_x"]), int(r["chosen_y"])):
                    wrong_picks += 1
        print(f"{out}: {visits:,} visits, {scores:,} scores rebuilt, {wrong_scores:,} differ (worst {worst:.2e} relative); "
              f"{picks:,} picks replayed, {wrong_picks:,} differ" + (f"   [--drop {drop}]" if drop else ""))
        bad += wrong_scores + wrong_picks
    print("CHECK " + ("FAILED" if bad else "PASSED") + (" -- as the control expects" if drop and bad else ""))
    sys.exit(1 if bad else 0)


def tip(outs, args):
    out = outs[0]
    rest = [a for a in args[1:] if not a.startswith("--")]
    if not rest:
        sys.exit("tip needs an organism id: planttrace.py tip OUT ID [X Y]")
    want = rest[0]
    rows = [r for r in growlog(out) if r["id"] == want]
    if not rows:
        sys.exit(f"no Grow visit of organism {want} in {out}/growlog.csv.gz (was it filtered out?)")
    if len(rest) < 3:
        print(f"=== organism {want}: {len(rows):,} Grow visits, frames {rows[0]['frame']}-{rows[-1]['frame']}")
        for kind in ("shoot", "root"):
            c = collections.Counter(r["why"] for r in rows if r["kind"] == kind)
            if c:
                print(f"  {kind:5} {sum(c.values()):6,}: {top(c, 6)}")
        grown_into = {(r["step_x"], r["step_y"]) for r in rows if r["step_x"]}
        starts = []
        for r in rows:
            cell = (r["x"], r["y"])
            if cell not in grown_into and cell not in [s[1] for s in starts]:
                starts.append((r, cell))
        print(f"  {len(starts)} tips started from cells no logged step grew into (germination, a branch, a regrown tiller); the first:")
        for r, (x, y) in starts[:10]:
            print(f"    {r['kind']:5} at ({x},{y}) from frame {r['frame']} -- follow with: tip {out} {want} {x} {y}")
        return
    x, y = rest[1], rest[2]
    after = -1
    print(f"=== the tip of organism {want} at ({x},{y}), visit by visit")
    while True:
        here = [r for r in rows if r["x"] == x and r["y"] == y and int(r["frame"]) > after]
        if not here:
            print("  -- no later visit in the log")
            return
        moved = False
        for r in here:
            options = sorted(scored_of(r), key=lambda c: -c[3])[:3]
            opts = ", ".join(f"({dx:+d},{dy:+d}) {score:.3f}" for dx, dy, _, score in options) or "-"
            margin = f"{float(r['margin']):.3f}" if r["margin"] else "-"
            print(f"  f{r['frame']:>7} ({x},{y}) {r['kind']:5} path {r['path']:>4} carbon {float(r['carbon']):.3f}/{float(r['cost']):.3f} "
                  f"headroom {margin:>6} {r['why']:18} open {r['open']}/{r['affordable']} best {opts}"
                  + (f" -> draw {r['draw']} chose ({r['chosen_x']},{r['chosen_y']}) grew into ({r['step_x']},{r['step_y']})" if r["draw"] else ""))
            if r["retired"] == "1":
                print("  -- retired here")
                return
            if r["step_x"]:
                after = int(r["frame"])
                x, y = r["step_x"], r["step_y"]
                moved = True
                break
        if not moved:
            print("  -- no step and no retirement in the log after this")
            return


if __name__ == "__main__":
    # `| head` closes the pipe early; die quietly as other tools do (no
    # SIGPIPE on Windows, where the owner also runs these).
    if hasattr(signal, "SIGPIPE"):
        signal.signal(signal.SIGPIPE, signal.SIG_DFL)
    cmds = {"funnel": funnel, "deaths": deaths, "spells": spells, "tips": tips, "life": life, "tip": tip, "check": check}
    if len(sys.argv) < 3 or sys.argv[1] not in cmds:
        sys.exit(__doc__)
    args = sys.argv[2:]
    if sys.argv[1] in ("life", "tip"):
        cmds[sys.argv[1]]([args[0]], args)
    else:
        outs = [x for i, x in enumerate(args) if not x.startswith("--") and (i == 0 or args[i - 1] not in ("--species", "--drop"))]
        cmds[sys.argv[1]](outs, args)
