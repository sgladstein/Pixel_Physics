#!/usr/bin/env python3
"""Paired reading of `deeptrace dig=1` runs: did a switch change the nest, and what do the diggers do.

    python3 scripts/deeptrace_pairs.py tables ROOT ARM_A ARM_B SEED... [--md]
    python3 scripts/deeptrace_pairs.py enrich ROOT ARM SEED...
    python3 scripts/deeptrace_pairs.py diggers ROOT ARM[,ARM...] SEED... [--from F] [--to F] [--reach R] [--gap G] [--min N] [--ref F] [--md]
    python3 scripts/deeptrace_pairs.py --selftest

A run is the directory ROOT/ARM-sSEED that `deeptrace dig=1 out=...` wrote (`stats.csv`, `cuts.csv`, `events.txt` and the
`nest_fNNNNNN.txt` maps). Built for `Reports/brood-and-the-dig-2026-10-09.md`; the room census is the lane's own
(`deeptrace_dig.py room_census`), so "a room of 30+ cells" means what it means everywhere else.

`tables`: per seed, ARM_A -> ARM_B for the colony (ants, births, starved), the nest shape at 100k/200k/300k (open
cells, deepest open row, width, biggest room, rooms of 30+ cells and how many are separate), the census counters
`BROOD_BLIND` adds to `stats.csv`, and where the cuts are; with the median change and how many seeds are higher and
lower (a sign count, never a pooled mean). `--md` prints the three tables the report quotes, as markdown.

`enrich`: is digging drawn to brood? At each nest map (100k, 150k, ... 300k) it takes every cuttable cell under the old
ground line that touches open space -- what the room OFFERS a digger -- and the share of those within 2 and 5 cells of
brood; then the share of the nest cuts in the 25k frames round that map (from `cuts.csv`'s `brood_d`) that were within
the same distances -- what the diggers DID. The ratio is the enrichment: 1.0 means digging ignores brood, above 1 it is
drawn to it, below 1 it keeps away from it. The offer is a fair comparison because it is taken from the same map the
cuts were made against, not from the nest as a whole.

`diggers`: where and how the nest cuts are made. Over the cuts in the window (`--from`/`--to`, default 100k to the end),
per run: how far from the door column the cuts are, against how far from it the open cells of the nest at `--ref`
(default 200k) are; how many cuts a digger makes (a digger is a `cuts.csv` `id`); how often its next cut is within 2 and
5 cells of its last; and its BOUTS -- a digger's run of cuts, each within `--reach` cells (default 5) and `--gap` frames
(default 500) of the one before. For bouts of `--min` cuts or more (default 8): their share of all nest cuts, the net
distance from first cut to last (Chebyshev), the cells advanced per cut, and the bounding box of the bout's cuts. A
tunnel advances about one cell per cut in a box one cell wide; a bay widened from where a digger stands stays in a box a
few cells across and advances a fraction of a cell per cut. The three thresholds are not natural constants; `--reach 3
--gap 200` is the same picture. It also splits the cuts into TRIMS (the cut cell already has 5 or more of its 8 neighbours open: a bump or a spike
sticking into the room) and BORES (3 or fewer: a flat wall, or a tunnel's end). The instrument is checked on a constructed
tunnel and a constructed patch by `--selftest`. `--md` prints a table, one column per arm, each cell the range over seeds.

Nothing here writes into a run.
"""
import csv
import importlib.util
import os
import statistics as st
import sys
import tempfile
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location("deeptrace_dig", os.path.join(HERE, "deeptrace_dig.py"))
dd = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(dd)

SHAPE_FRAMES = [100000, 200000, 300000]
ENRICH_FRAMES = [100000, 150000, 200000, 250000, 300000]
OPEN = set("o.aelpfcx~?b")
BROOD = set("elpb")
CUTTABLE = set("#=s")
DEFAULT_DOOR, DEFAULT_GROUND = 256, 160


def run_dir(root, arm, seed):
    return f"{root}/{arm}-s{seed}"


def csv_rows(path):
    """A run still writing leaves a short last line; those rows are left out, never reported."""
    return [r for r in csv.DictReader(open(path)) if None not in r.values() and None not in r]


def door_and_ground(run):
    try:
        return dd.geo(run)
    except SystemExit:  # events.txt is written when a run ends; a partial run is read at the lab's own geometry
        print(f"  (partial run {run}: no FOUNDED line, using door {DEFAULT_DOOR} and ground_y {DEFAULT_GROUND})")
        return DEFAULT_DOOR, DEFAULT_GROUND


def at(rows, frame):
    for r in rows:
        if int(r["frame"]) == frame:
            return r
    return rows[-1]


def nest_map(path):
    with open(path) as fh:
        x0, y0, wd, ht = map(int, fh.readline().split())
        rows = [ln.rstrip("\n").ljust(wd, "#") for ln in fh]
    return x0, y0, wd, ht, rows


# ---------------------------------------------------------------- tables

def shape(run, frame, gy):
    p = f"{run}/nest_f{frame:06}.txt"
    if not os.path.exists(p):
        return None
    opened, deepest, brood, big, others = dd.room_census(p, gy)
    x0, y0, wd, ht, rows = nest_map(p)
    xs = [x for y in range(ht) for x in range(wd) if rows[y][x] in dd.OPEN and y0 + y > gy]
    return dict(
        open=opened,
        deepest=deepest,
        width=(max(xs) - min(xs) + 1) if xs else 0,
        big=big[0] if big else 0,
        rooms30=(1 if big else 0) + len(others),
        sep=sum(1 for o in others if o[1] > 3),
        brood_under=brood,
    )


def cut_shares(run):
    rows = csv_rows(f"{run}/cuts.csv")
    out = {}
    for name, sel in [("all", lambda r: True), ("nest", lambda r: r["zone"] == "nest"),
                      ("late_nest", lambda r: r["zone"] == "nest" and int(r["frame"]) >= 100000)]:
        rs = [r for r in rows if sel(r)]
        n = len(rs)
        bd = [int(r["brood_d"]) if r["brood_d"] != "" else 99 for r in rs]

        def share(f):
            return sum(1 for r in rs if f(r)) / n if n else float("nan")

        out[name] = dict(
            n=n,
            near2=sum(d <= 2 for d in bd),
            f_le1=sum(d <= 1 for d in bd) / n if n else float("nan"),
            f_le2=sum(d <= 2 for d in bd) / n if n else float("nan"),
            f_le5=sum(d <= 5 for d in bd) / n if n else float("nan"),
            f_down=share(lambda r: int(r["flags"]) & 1 > 0),
            f_faced=share(lambda r: int(r["flags"]) & 2 > 0),
            f_home=share(lambda r: r["home"] == "1"),
            f_below=share(lambda r: int(r["y"]) > int(r["hy"])),
            f_level=share(lambda r: int(r["y"]) == int(r["hy"])),
            f_bore=share(lambda r: int(r["open8"]) <= 3),
            f_trim=share(lambda r: int(r["open8"]) >= 5),
        )
    return out


def collect(root, arm, seed):
    run = run_dir(root, arm, seed)
    rows = csv_rows(f"{run}/stats.csv")
    _, gy = door_and_ground(run)
    last = rows[-1]
    m = {"frames": int(last["frame"])}
    for k in ["ants", "digs", "eggs_laid", "births", "larvae_starved", "died_starved", "dig_rolls_near_brood",
              "dig_enclosed_flips", "dig_sky_flips", "cuts_under_brood", "cuts_near_brood"]:
        m[k] = int(last[k])
    for k in ["dig_rolls", "digs_aimed_down", "digs_faced", "digs_down_refused", "spoil_cue_applied", "digs_refused_roof"]:
        if k in last:
            m[k] = int(last[k])
    if m.get("dig_rolls"):
        for name, key in [("near_brood", "dig_rolls_near_brood"), ("enclosed_flip", "dig_enclosed_flips"),
                          ("sky_flip", "dig_sky_flips"), ("aimed_down", "digs_aimed_down"), ("faced", "digs_faced")]:
            m[f"pct_rolls_{name}"] = 100 * m[key] / m["dig_rolls"]
    if m["digs"]:
        m["pct_cuts_under_brood"] = 100 * m["cuts_under_brood"] / m["digs"]
        m["pct_cuts_near_brood"] = 100 * m["cuts_near_brood"] / m["digs"]
    for f in SHAPE_FRAMES:
        r = at(rows, f)
        m[f"ants@{f // 1000}k"] = int(r["ants"])
        s = shape(run, f, gy)
        if s:
            for k, v in s.items():
                m[f"{k}@{f // 1000}k"] = v
    c = cut_shares(run)
    for scope in ("all", "nest", "late_nest"):
        for k, v in c[scope].items():
            m[f"cut_{scope}_{k}"] = v
    return m


def fmt(v):
    if isinstance(v, float):
        if v.is_integer():
            return str(int(v))
        return f"{v:.3f}" if abs(v) < 1 else (f"{v:.1f}" if abs(v) < 100 else f"{v:.0f}")
    return str(v)


def paired_row(A, B, seeds, key, scale=1.0):
    cells, diffs = [], []
    for s in seeds:
        a, b = A[s].get(key), B[s].get(key)
        if a is None or b is None:
            cells.append("-")
            continue
        cells.append(f"{fmt(a * scale)} -> {fmt(b * scale)}")
        diffs.append((b - a) * scale)
    return cells, diffs


def tables(root, arm_a, arm_b, seeds, md=False):
    A = {s: collect(root, arm_a, s) for s in seeds}
    B = {s: collect(root, arm_b, s) for s in seeds}
    print(f"seeds {seeds}; frames reached {arm_a} {[A[s]['frames'] for s in seeds]}, {arm_b} {[B[s]['frames'] for s in seeds]}")

    def text_table(title, keys):
        print(f"\n== {title}   ({arm_a} -> {arm_b})")
        for k in keys:
            cells, diffs = paired_row(A, B, seeds, k)
            tail = ""
            if diffs:
                tail = f"  median change {fmt(float(st.median(diffs)))}; {arm_b} higher on {sum(d > 0 for d in diffs)}, lower on {sum(d < 0 for d in diffs)} of {len(diffs)}"
            print(f"  {k:<26} " + "  ".join(f"{c:>17}" for c in cells) + tail)

    def md_table(title, rows):
        print(f"\n**{title}**\n")
        print("| | " + " | ".join(f"seed {s}" for s in seeds) + f" | median change | {arm_b} higher / lower |")
        print("|---|" + "---|" * (len(seeds) + 2))
        for label, key in rows:
            cells, diffs = paired_row(A, B, seeds, key)
            hi, lo = sum(d > 0 for d in diffs), sum(d < 0 for d in diffs)
            print(f"| {label} | " + " | ".join(cells) + f" | {fmt(float(st.median(diffs)))} | {hi} / {lo} |")

    def md_single(title, rows, M):
        print(f"\n**{title}**\n")
        print("| | " + " | ".join(f"seed {s}" for s in seeds) + " | median |")
        print("|---|" + "---|" * (len(seeds) + 1))
        for label, key in rows:
            vals = [M[s][key] for s in seeds]
            print(f"| {label} | " + " | ".join(fmt(v) for v in vals) + f" | {fmt(float(st.median(vals)))} |")

    if md:
        md_table(f"Nest shape and colony at 300,000 frames, `{arm_a}` -> `{arm_b}`", [
            ("rooms of 30+ cells", "rooms30@300k"), ("...of them separate (a passage away)", "sep@300k"),
            ("biggest room, cells", "big@300k"), ("open cells under the ground line", "open@300k"),
            ("deepest open row", "deepest@300k"), ("width of the open region", "width@300k"),
            ("ants alive", "ants"), ("births", "births"), ("larvae starved", "larvae_starved"),
            ("adults starved", "died_starved"), ("cuts (engine digs)", "digs")])
        md_single(f"Brood in the dig's senses, `{arm_a}` (shares of won dig rolls and of cuts)", [
            ("won dig rolls", "dig_rolls"), ("% with a larva in the head's 5x5", "pct_rolls_near_brood"),
            ("% where brood decides 'enclosed'", "pct_rolls_enclosed_flip"), ("% where brood changes the sky test", "pct_rolls_sky_flip"),
            ("% that turn the jaw down (dig-down)", "pct_rolls_aimed_down"), ("cuts", "digs"),
            ("% of cuts directly under a larva", "pct_cuts_under_brood"), ("% of cuts within 2 cells of brood", "pct_cuts_near_brood")], A)
        md_single(f"The same shares with `{arm_b}`", [
            ("% of won rolls that turn down", "pct_rolls_aimed_down"), ("% where brood would decide 'enclosed'", "pct_rolls_enclosed_flip")], B)
        return
    text_table("population and deaths (end of run)", ["ants", "births", "eggs_laid", "larvae_starved", "died_starved", "digs"])
    text_table("nest shape (the lane's room_census)", [f"{k}@{f // 1000}k" for f in SHAPE_FRAMES
                                                       for k in ("open", "deepest", "width", "big", "rooms30", "sep", "ants", "brood_under")])
    text_table("census counters (always on; the 'it fired' half)", ["dig_rolls", "dig_rolls_near_brood", "dig_enclosed_flips",
                                                                    "dig_sky_flips", "cuts_under_brood", "cuts_near_brood", "digs"])
    text_table("census as shares (percent of dig rolls won / of cuts)", ["pct_rolls_near_brood", "pct_rolls_enclosed_flip", "pct_rolls_sky_flip",
                                                                         "pct_rolls_aimed_down", "pct_rolls_faced", "pct_cuts_under_brood", "pct_cuts_near_brood"])
    text_table("where the cuts are (share with brood within d cells; DOWN / FACED / home flags)",
               [f"cut_{sc}_{k}" for sc in ("all", "nest", "late_nest") for k in ("n", "f_le1", "f_le2", "f_le5", "f_down", "f_faced", "f_home")])
    text_table("which way the nest cuts go (below / level with the head; a TRIM takes a cell with 5+ of its 8 neighbours already open, "
               "a bump or spike; a BORE one with 3 or fewer, a flat wall or a tunnel's end)",
               [f"cut_{sc}_{k}" for sc in ("nest", "late_nest") for k in ("f_below", "f_level", "f_bore", "f_trim")])
    print("\ncross-check of two instruments (stats.csv cuts_near_brood vs cuts.csv rows with brood_d <= 2):")
    for arm, M in ((arm_a, A), (arm_b, B)):
        print(f"  {arm}: " + ", ".join(f"s{s}: {M[s]['cuts_near_brood']} vs {M[s]['cut_all_near2']}" for s in seeds))


# ---------------------------------------------------------------- enrich

def offered(path, gy):
    """(cuttable cells touching open space below the old ground line, share within 2 of brood, share within 5), or None shares with no brood."""
    x0, y0, wd, ht, rows = nest_map(path)
    op = [[ch in OPEN for ch in r] for r in rows]
    cut = [[ch in CUTTABLE for ch in r] for r in rows]
    br = [(y, x) for y, r in enumerate(rows) for x, ch in enumerate(r) if ch in BROOD]
    cand = []
    for y in range(ht):
        if y0 + y <= gy:
            continue
        for x in range(wd):
            if not cut[y][x]:
                continue
            if any(0 <= y + dy < ht and 0 <= x + dx < wd and op[y + dy][x + dx] for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1))):
                cand.append((y, x))
    if not br:
        return len(cand), None, None
    d = [min(max(abs(y - by), abs(x - bx)) for by, bx in br) for y, x in cand]
    n = len(d) or 1
    return len(cand), sum(v <= 2 for v in d) / n, sum(v <= 5 for v in d) / n


def did(run, frame, half=12500):
    n = c2 = c5 = 0
    for r in csv_rows(f"{run}/cuts.csv"):
        if r["zone"] != "nest" or abs(int(r["frame"]) - frame) > half:
            continue
        d = int(r["brood_d"]) if r["brood_d"] != "" else 99
        n += 1
        c2 += d <= 2
        c5 += d <= 5
    return n, (c2 / n if n else float("nan")), (c5 / n if n else float("nan"))


def enrich(root, arm, seeds):
    v2, v5 = [], []
    print(f"arm {arm}: per nest map, the share of cuttable wall within 2 / 5 cells of brood (offered) against the share of "
          "nest cuts within 2 / 5 (did), and the ratio")
    for s in seeds:
        run = run_dir(root, arm, s)
        _, gy = door_and_ground(run)
        for f in ENRICH_FRAMES:
            p = f"{run}/nest_f{f:06}.txt"
            if not os.path.exists(p):
                continue
            n_off, o2, o5 = offered(p, gy)
            n_cut, c2, c5 = did(run, f)
            if o2 is None:
                print(f"  s{s} {f // 1000:3d}k  no brood on the map")
                continue
            e2 = c2 / o2 if o2 else float("nan")
            e5 = c5 / o5 if o5 else float("nan")
            v2.append(e2)
            v5.append(e5)
            print(f"  s{s} {f // 1000:3d}k  wall cells {n_off:5d}  offered <=2 {100 * o2:5.1f}%  <=5 {100 * o5:5.1f}% | cuts {n_cut:6d}  "
                  f"did <=2 {100 * c2:5.1f}%  <=5 {100 * c5:5.1f}% | enrichment <=2 {e2:4.2f}  <=5 {e5:4.2f}")
    v2 = [v for v in v2 if v == v]
    v5 = [v for v in v5 if v == v]
    if v2:
        print(f"  arm {arm} over {len(v2)} (seed, frame) points: enrichment within 2 cells median {st.median(v2):.2f} "
              f"(min {min(v2):.2f}, max {max(v2):.2f}); within 5 cells median {st.median(v5):.2f} (min {min(v5):.2f}, max {max(v5):.2f}); "
              f"points above 1.0: {sum(v > 1 for v in v2)} and {sum(v > 1 for v in v5)}")


# ---------------------------------------------------------------- diggers

def bouts_of(cuts, reach, gap):
    """Split one digger's cuts [(frame, x, y)] into bouts: each cut within `reach` cells and `gap` frames of the last."""
    cuts = sorted(cuts)
    out, cur = [], [cuts[0]]
    for c in cuts[1:]:
        p = cur[-1]
        if c[0] - p[0] <= gap and max(abs(c[1] - p[1]), abs(c[2] - p[2])) <= reach:
            cur.append(c)
        else:
            out.append(cur)
            cur = [c]
    out.append(cur)
    return out


def pctl(xs, p):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(p * len(xs)))]


def digger_stats(run, lo, hi, reach, gap, min_cuts, ref):
    door, gy = door_and_ground(run)
    rows = [r for r in csv_rows(f"{run}/cuts.csv") if r["zone"] == "nest" and lo <= int(r["frame"]) < hi]
    if not rows:
        return None
    by = defaultdict(list)
    for r in rows:
        by[r["id"]].append((int(r["frame"]), int(r["x"]), int(r["y"])))
    out = {"cuts": len(rows), "diggers": len(by)}
    per = [len(v) for v in by.values()]
    out["per_med"], out["per_mean"], out["per_max"] = st.median(per), st.mean(per), max(per)
    n = w2 = w5 = 0
    big = []
    for v in by.values():
        v.sort()
        for a, b in zip(v, v[1:]):
            d = max(abs(a[1] - b[1]), abs(a[2] - b[2]))
            n += 1
            w2 += d <= 2
            w5 += d <= 5
        big += [b for b in bouts_of(v, reach, gap) if len(b) >= min_cuts]
    out["next2"], out["next5"] = (w2 / n if n else float("nan")), (w5 / n if n else float("nan"))
    tot = sum(len(b) for b in big)
    out["bouts"], out["bout_share"] = len(big), tot / len(rows)
    if big:
        net = [max(abs(b[-1][1] - b[0][1]), abs(b[-1][2] - b[0][2])) for b in big]
        out["bout_len"] = st.median([len(b) for b in big])
        out["net_med"], out["net_p90"] = st.median(net), pctl(net, 0.9)
        out["adv"] = st.median([x / len(b) for x, b in zip(net, big)])
        out["box_w"] = st.median([max(c[1] for c in b) - min(c[1] for c in b) + 1 for b in big])
        out["box_h"] = st.median([max(c[2] for c in b) - min(c[2] for c in b) + 1 for b in big])
    dx = [abs(int(r["x"]) - door) for r in rows]
    out["dx_med"], out["dx_p10"], out["dx_p90"] = st.median(dx), pctl(dx, 0.1), pctl(dx, 0.9)
    out["below"] = sum(int(r["y"]) > int(r["hy"]) for r in rows) / len(rows)
    out["trim"] = sum(int(r["open8"]) >= 5 for r in rows) / len(rows)
    out["bore"] = sum(int(r["open8"]) <= 3 for r in rows) / len(rows)
    p = f"{run}/nest_f{ref:06}.txt"
    if os.path.exists(p):
        x0, y0, wd, ht, g = nest_map(p)
        cav = [abs(x0 + x - door) for j in range(ht) for x in range(wd) if y0 + j > gy and g[j][x] in OPEN]
        if cav:
            out["cav_med"], out["cav_p90"] = st.median(cav), pctl(cav, 0.9)
            out["outer"] = sum(d > out["cav_med"] for d in dx) / len(dx)
    return out


def diggers(root, arms, seeds, lo=100000, hi=10**9, reach=5, gap=500, min_cuts=8, ref=200000, md=False):
    print(f"nest cuts in frames [{lo}, {hi}); a bout is a digger's run of cuts each within {reach} cells and {gap} frames of the "
          f"one before, counted from {min_cuts} cuts")
    per_arm = {}
    for arm in arms:
        per_arm[arm] = []
        for s in seeds:
            d = digger_stats(run_dir(root, arm, s), lo, hi, reach, gap, min_cuts, ref)
            per_arm[arm].append(d)
    if md:
        def span(arm, key, scale=1.0, f="{:.0f}"):
            vals = [d[key] * scale for d in per_arm[arm] if d is not None and key in d]
            if not vals:
                return "-"
            a, b = f.format(min(vals)), f.format(max(vals))
            return a if a == b else f"{a} to {b}"

        def med(arm, key, f="{:.0f}"):
            vals = [d[key] for d in per_arm[arm] if d is not None and key in d]
            return f.format(st.median(vals)) if vals else "-"
        rows = [
            ("nest cuts", "cuts", 1, "{:.0f}"), ("diggers that cut", "diggers", 1, "{:.0f}"),
            ("cuts per digger: median", "per_med", 1, "{:.0f}"), ("cuts per digger: mean", "per_mean", 1, "{:.0f}"),
            ("cuts per digger: the most", "per_max", 1, "{:.0f}"),
            ("a digger's next cut within 2 cells of its last, %", "next2", 100, "{:.0f}"),
            ("...within 5 cells, %", "next5", 100, "{:.0f}"),
            ("share of nest cuts that sit in a bout, %", "bout_share", 100, "{:.0f}"),
            ("a bout: cuts in it, median", "bout_len", 1, "{:.0f}"),
            ("a bout: cells from its first cut to its last, median", "net_med", 1, "{:.0f}"),
            ("a bout: cells advanced per cut, median", "adv", 1, "{:.2f}"),
            ("a bout: bounding box of its cuts, width, median", "box_w", 1, "{:.0f}"),
            ("a bout: bounding box of its cuts, height, median", "box_h", 1, "{:.0f}"),
            ("cut: columns from the door, median", "dx_med", 1, "{:.0f}"),
            ("open cell of the nest at the reference map: columns from the door, median", "cav_med", 1, "{:.0f}"),
            ("cuts farther out than that median, %", "outer", 100, "{:.0f}"),
            ("cuts of a cell below the digger's head, %", "below", 100, "{:.0f}"),
            ("cuts that TRIM a bump (5+ of the cell's 8 neighbours open), %", "trim", 100, "{:.0f}"),
            ("cuts that BORE (3 or fewer open), %", "bore", 100, "{:.0f}"),
        ]
        print("\n| | " + " | ".join(f"`{a}`: range over {len(seeds)} seeds" for a in arms) + " |")
        print("|---|" + "---|" * len(arms))
        for label, key, scale, f in rows:
            print(f"| {label} | " + " | ".join(span(a, key, scale, f) for a in arms) + " |")
        return
    for arm in arms:
        print(f"arm {arm}")
        print("run        nest cuts diggers | cuts/digger med mean max | next cut within 2, 5 cells | bouts: share of cuts, net med (p90), "
              "advance/cut, box WxH | cuts |x-door| med (p10-p90) vs open cells at ref: med, p90 | share of cuts beyond that median | aimed below head")
        for s, d in zip(seeds, per_arm[arm]):
            if d is None:
                print(f"{arm}-s{s}  no nest cuts in the window")
                continue
            print(f"{arm}-s{s}  {d['cuts']:8d} {d['diggers']:7d} | {d['per_med']:5.1f} {d['per_mean']:5.1f} {d['per_max']:4d} | "
                  f"{100 * d['next2']:5.1f}% {100 * d['next5']:5.1f}% | {100 * d['bout_share']:5.1f}% {d.get('net_med', 0):4.1f} ({d.get('net_p90', 0)}) "
                  f"{d.get('adv', 0):5.2f} {d.get('box_w', 0):3.0f}x{d.get('box_h', 0):.0f} | {d['dx_med']:4.1f} ({d['dx_p10']}-{d['dx_p90']}) vs "
                  f"{d.get('cav_med', float('nan')):4.1f}, {d.get('cav_p90', float('nan'))} | {100 * d.get('outer', float('nan')):5.1f}% | {100 * d['below']:5.1f}%")


# ---------------------------------------------------------------- selftest

CUT_HEADER = "frame,id,age,worker,hx,hy,x,y,zone,depth,mat,flags,dig_p,at_nest,crowding,open8,ground24,joins,brood_d,brood5,ants3,roofed,home"


def write_run(path, cuts, map_rows, x0=200, y0=150):
    """A constructed run: `cuts` is [(frame, id, x, y, brood_d)], `map_rows` the nest map drawn at 200k."""
    os.makedirs(path)
    with open(f"{path}/events.txt", "w") as fh:
        fh.write("6000 FOUNDED nest_x=256 food_x=346 ground_y=160 start_j=200\n")
    with open(f"{path}/cuts.csv", "w") as fh:
        fh.write(CUT_HEADER + "\n")
        for frame, ant, x, y, bd in cuts:
            fh.write(f"{frame},{ant},100,1,{x},{y - 1},{x},{y},nest,10,soil,1,0.8,1,0.9,3,12,1,{'' if bd is None else bd},0,0,1,1\n")
    with open(f"{path}/nest_f200000.txt", "w") as fh:
        fh.write(f"{x0} {y0} {len(map_rows[0])} {len(map_rows)}\n")
        fh.write("\n".join(map_rows) + "\n")


def selftest():
    ok = True

    def check(label, cond, detail=""):
        nonlocal ok
        print(f"  {'ok  ' if cond else 'FAIL'} {label} {detail}")
        ok &= bool(cond)

    with tempfile.TemporaryDirectory() as td:
        # Two diggers: ant 1 cuts a straight tunnel, one cell further each cut; ant 2 works a 3x3 patch, three passes.
        cuts = [(100000 + 40 * i, 1, 200 + i, 170, None) for i in range(40)]
        patch = [(x, y) for y in range(180, 183) for x in range(220, 223)]
        cuts += [(100000 + 40 * i, 2, patch[i % 9][0], patch[i % 9][1], None) for i in range(27)]
        room = ["." * 80 for _ in range(30)]
        # each digger in a run of its own, so the one's cuts cannot join the other's bout
        write_run(f"{td}/line-s1", [c for c in cuts if c[1] == 1], room)
        write_run(f"{td}/patch-s1", [c for c in cuts if c[1] == 2], room)
        line = digger_stats(f"{td}/line-s1", 100000, 10**9, 5, 500, 8, 200000)
        patch_d = digger_stats(f"{td}/patch-s1", 100000, 10**9, 5, 500, 8, 200000)
        print("diggers: a constructed tunnel and a constructed patch")
        check("the tunnel is one bout of 40 cuts", line["bouts"] == 1 and line["bout_share"] == 1.0, f"(bouts {line['bouts']})")
        check("the tunnel advances about one cell per cut", line["adv"] >= 0.9, f"(advance/cut {line['adv']:.2f})")
        check("the tunnel's box is one cell tall and wide as the tunnel is long", line["box_h"] == 1 and line["box_w"] == 40, f"({line['box_w']:.0f}x{line['box_h']:.0f})")
        check("the patch is one bout of 27 cuts in a 3x3 box", patch_d["bouts"] == 1 and (patch_d["box_w"], patch_d["box_h"]) == (3, 3))
        check("the patch advances a fraction of a cell per cut", patch_d["adv"] <= 0.1, f"(advance/cut {patch_d['adv']:.2f})")
        check("the patch's next cut is always within 2 cells and the tunnel's is always 1 away", patch_d["next2"] == 1.0 and line["next2"] == 1.0)
        # a gap longer than `gap` frames starts a new bout
        gapped = [(100000 + 2000 * i, 1, 200 + i, 170, None) for i in range(10)]
        write_run(f"{td}/gap-s1", gapped, room)
        g = digger_stats(f"{td}/gap-s1", 100000, 10**9, 5, 500, 8, 200000)
        check("cuts 2,000 frames apart are not one bout", g["bouts"] == 0, f"(bouts of 8+: {g['bouts']})")

        # enrichment: a soil bank under the old ground line (rows 161 down) with open space above it and larvae lying at
        # its left end. The cuts say how far each was from brood (`brood_d`), as deeptrace writes it.
        bank = [list("." * 80), list("." * 80), list("." * 80)] + [list("#" * 80) for _ in range(3)]
        for x in range(0, 4):
            bank[2][x] = "l"
        bank = ["".join(r) for r in bank]
        near = [(200000 + i, 1, 1 + (i % 3), 164, 1) for i in range(60)]
        far = [(200000 + i, 1, 50 + (i % 3), 164, 30) for i in range(60)]
        write_run(f"{td}/near-s1", near, bank, x0=200, y0=161)
        write_run(f"{td}/far-s1", far, bank, x0=200, y0=161)
        print("enrich: a constructed bank with larvae at one end, cuts drawn to them and cuts kept away")
        for label, arm, want in (("cuts at the larvae are enriched", "near", lambda e: e > 2.0), ("cuts away from the larvae are depleted", "far", lambda e: e < 0.5)):
            n_off, o2, o5 = offered(f"{td}/{arm}-s1/nest_f200000.txt", 160)
            _, c2, _ = did(f"{td}/{arm}-s1", 200000)
            e = c2 / o2
            check(label, want(e), f"(offered {100 * o2:.0f}%, did {100 * c2:.0f}%, enrichment {e:.2f})")
    print("selftest:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


def main(argv):
    if len(argv) >= 2 and argv[1] == "--selftest":
        return selftest()
    if len(argv) < 4:
        print(__doc__)
        return 2
    cmd, root = argv[1], argv[2]
    args = argv[3:]
    opts = {}
    for flag in ("--from", "--to", "--reach", "--gap", "--min", "--ref"):
        if flag in args:
            i = args.index(flag)
            opts[flag] = int(args[i + 1])
            del args[i:i + 2]
    md = "--md" in args
    args = [a for a in args if a != "--md"]
    if cmd == "tables":
        tables(root, args[0], args[1], [int(a) for a in args[2:]], md)
    elif cmd == "enrich":
        enrich(root, args[0], [int(a) for a in args[1:]])
    elif cmd == "diggers":
        diggers(root, args[0].split(","), [int(a) for a in args[1:]], opts.get("--from", 100000), opts.get("--to", 10**9),
                opts.get("--reach", 5), opts.get("--gap", 500), opts.get("--min", 8), opts.get("--ref", 200000), md)
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
