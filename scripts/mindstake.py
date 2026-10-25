#!/usr/bin/env python3
"""The colony mind demo's stake: does a player policy carry a colony through the `demo_mind` bed's famines?

    python3 scripts/mindstake.py DIR       # DIR holds <arm>_h<hold>_s<seed>.txt and none_s<seed>.txt from
                                           # `mindprobe scenario=demo_mind warm=1 window=150000 every=2500 ...`

Per arm, over seeds: extinctions by the end, the mean headcount over the run (`lived`, graded where extinct is
all-or-nothing), and the headcount at the bottom of each famine (income stops 20,000 frames into every 35,000, from
the press at the colony's landing). Paired against `none` on the same seed: lived difference and how many seeds it
is up on.
"""
import glob
import os
import re
import statistics
import sys


def read(path):
    rows = []
    for line in open(path):
        if line.startswith("S "):
            d = dict(kv.split("=") for kv in line.split()[1:])
            rows.append((int(d["t"]), int(d["alive"])))
    return rows


def troughs(rows, period=35000, feed=20000):
    out = []
    k = 0
    while (k + 1) * period <= rows[-1][0] + 1:
        lo, hi = k * period + feed, (k + 1) * period + 2500
        v = [a for t, a in rows if lo <= t <= hi]
        if v:
            out.append(min(v))
        k += 1
    return out


def main():
    d = sys.argv[1]
    runs = {}
    for p in glob.glob(os.path.join(d, "*_s*.txt")):
        b = os.path.basename(p)
        m = re.match(r"(.+?)(?:_h\d+)?_s(\d+)\.txt$", b)
        rows = read(p)
        if rows:
            runs.setdefault(m.group(1), {})[m.group(2)] = rows
    base = runs.get("none", {})
    for arm in sorted(runs, key=lambda a: (a != "none", a)):
        seeds = runs[arm]
        ext = sum(1 for r in seeds.values() if r[-1][1] == 0)
        lived = {s: statistics.mean(a for _, a in r) for s, r in seeds.items()}
        tr = [troughs(r) for r in seeds.values()]
        ncyc = min(len(t) for t in tr)
        tmed = [statistics.median(t[k] for t in tr) for k in range(ncyc)]
        line = f"{arm:12} n={len(seeds):2} extinct={ext:2} lived_med={statistics.median(lived.values()):6.1f} trough_medians={tmed}"
        if arm != "none" and base:
            diff = sorted(lived[s] - statistics.mean(a for _, a in base[s]) for s in seeds if s in base)
            up = sum(1 for x in diff if x > 0)
            line += f" lived_vs_none med={statistics.median(diff):+.1f} [{diff[0]:+.1f},{diff[-1]:+.1f}] up={up}/{len(diff)}"
        print(line)


if __name__ == "__main__":
    main()
