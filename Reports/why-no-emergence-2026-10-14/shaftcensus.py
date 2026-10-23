"""shaftcensus.py RUN [RUN...] -- who is in the door shaft, and how does the nest's population move over time?

Reads colony.csv (every ant's head every 1,000 frames), stats.csv and events.txt from a deeptrace run.

The door shaft is the 7 columns x 16 rows under the door: x in [nest_x-3, nest_x+3], y in [ground_y, ground_y+15]
(rows 160-175 on the goal box; the ground row is included). Window LO-HI (env, default 100k-300k). Per run:
  heads        ant heads in the shaft, mean over census frames (heads, not bodies: a body is 2 cells)
  cells        distinct shaft cells holding at least one head, of 112
  at cap       share of occupied shaft cells holding 4 heads (the default stack cap)
  of colony    shaft heads as a share of the whole colony
  of under     shaft heads as a share of ants below the ground line
  carrying     share of shaft heads with food in the crop
  lean         share of shaft heads under 200 J (start_energy)
With BINS=1 it also prints a 20k-frame timeline: shaft heads, colony, ants below ground, starvers (in the nest),
births, deliveries and eats. Adapted from the adversarial review's shaft.py and timeline.py (2026-10-14).
"""
import collections
import csv
import os
import re
import statistics
import sys

LO, HI = int(os.environ.get("LO", 100_000)), int(os.environ.get("HI", 300_001))
CAP = 4


def geometry(path):
    founded = next(l for l in open(f"{path}/events.txt") if "FOUNDED" in l)
    nx = int(re.search(r"nest_x=(\d+)", founded).group(1))
    gy = int(re.search(r"ground_y=(\d+)", founded).group(1))
    return nx, gy


def census(path):
    nx, gy = geometry(path)
    in_shaft = lambda x, y: nx - 3 <= x <= nx + 3 and gy <= y <= gy + 15
    frames = collections.defaultdict(lambda: dict(heads=0, colony=0, under=0, carry=0, lean=0, cells=collections.Counter()))
    for r in csv.DictReader(open(f"{path}/colony.csv")):
        f = int(r["frame"])
        if not LO <= f < HI:
            continue
        x, y = int(r["hx"]), int(r["hy"])
        c = frames[f]
        c["colony"] += 1
        if y > gy:
            c["under"] += 1
        if in_shaft(x, y):
            c["heads"] += 1
            c["cells"][(x, y)] += 1
            c["carry"] += int(r["crop_cells"] or 0) > 0
            c["lean"] += float(r["energy_j"] or 0) < 200
    m = lambda k: statistics.mean(c[k] for c in frames.values())
    occ = [len(c["cells"]) for c in frames.values()]
    capped = [sum(1 for n in c["cells"].values() if n >= CAP) for c in frames.values()]
    heads = sum(c["heads"] for c in frames.values())
    return dict(
        heads=m("heads"),
        cells=statistics.mean(occ),
        at_cap=sum(capped) / max(1, sum(occ)),
        of_colony=statistics.mean(c["heads"] / c["colony"] for c in frames.values() if c["colony"]),
        of_under=statistics.mean(c["heads"] / c["under"] for c in frames.values() if c["under"]),
        carrying=sum(c["carry"] for c in frames.values()) / max(1, heads),
        lean=sum(c["lean"] for c in frames.values()) / max(1, heads),
        colony=m("colony"),
        under=m("under"),
    )


def timeline(path, B=20_000):
    nx, gy = geometry(path)
    sh, tot, und = (collections.defaultdict(list) for _ in range(3))
    cur, c = None, [0, 0, 0]
    for r in csv.DictReader(open(f"{path}/colony.csv")):
        f = int(r["frame"])
        if f != cur:
            if cur is not None:
                b = cur // B
                sh[b].append(c[0]); tot[b].append(c[1]); und[b].append(c[2])
            cur, c = f, [0, 0, 0]
        x, y = int(r["hx"]), int(r["hy"])
        c[1] += 1
        c[2] += y > gy
        c[0] += nx - 3 <= x <= nx + 3 and gy <= y <= gy + 15
    b = cur // B
    sh[b].append(c[0]); tot[b].append(c[1]); und[b].append(c[2])
    st, stn = collections.Counter(), collections.Counter()
    for line in open(f"{path}/events.txt"):
        p = line.split()
        if len(p) > 1 and p[1] == "DIED":
            kv = dict(x.split("=", 1) for x in p[2:] if "=" in x)
            if kv.get("cause") == "STARVED":
                st[int(p[0]) // B] += 1
                stn[int(p[0]) // B] += kv.get("zone") == "nest"
    cum = {int(r["frame"]): (int(r["births"]), int(r["deliveries"]), int(r["eats"])) for r in csv.DictReader(open(f"{path}/stats.csv"))}
    at = lambda f: cum[max(k for k in cum if k <= f)]
    for b in sorted(sh):
        lo, hi = b * B, b * B + B
        if lo >= 300_000:
            continue
        a0, a1 = at(max(lo, min(cum))), at(min(hi, max(cum)))
        print(f"    {lo // 1000:3d}-{hi // 1000:3d}k  shaft {statistics.mean(sh[b]):4.0f}  colony {statistics.mean(tot[b]):5.0f}  "
              f"below ground {statistics.mean(und[b]):4.0f}  starved {st[b]:4d} (in nest {stn[b]:4d})  "
              f"births {a1[0] - a0[0]:4d}  deliveries {a1[1] - a0[1]:5d}  eats {a1[2] - a0[2]:5d}")


def main():
    for path in sys.argv[1:]:
        r = census(path)
        print(f"== {path.rstrip('/').split('/')[-1]}  window {LO}-{HI}")
        print(f"  shaft heads {r['heads']:.0f}  occupied cells {r['cells']:.0f}/112  at cap {r['at_cap']:.1%}  "
              f"of colony {r['of_colony']:.1%}  of below-ground {r['of_under']:.0%}  "
              f"carrying food {r['carrying']:.0%}  under 200 J {r['lean']:.0%}  (colony {r['colony']:.0f}, below ground {r['under']:.0f})")
        if os.environ.get("BINS") == "1":
            timeline(path)


if __name__ == "__main__":
    main()
