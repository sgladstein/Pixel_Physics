"""SCRATCH: mutation size, offline. Children of s171 under four mutation settings, each on reliab's two 48-world sets.
usage: python3 -I mutsize.py mut.csv
Per arm: the children's own distribution (mean over all 96 worlds, s.e. clustered by mirrored pair), and what one
generation of truncation selection would gain: pick the top quarter by set 0, judge the picked on set 1 (worlds they
were not picked on), against the parent on set 1. Also the same pick with racing (first 8 worlds of set 0, keep half)."""
import csv, sys, random, statistics as st
from collections import defaultdict

rows = defaultdict(lambda: defaultdict(dict))  # file -> set -> world -> full loads
for r in csv.DictReader(open(sys.argv[1])):
    rows[r["file"]][int(r["set"])][int(r["world"])] = float(r["full_loads"])
files = list(rows)
par = [f for f in files if f.endswith("/parent.txt")][0]
cp = [f for f in files if f.endswith("/parent-copy.txt")][0]
n_w = len(rows[par][0])
print(f"{len(files)} genomes x 2 sets x {n_w} worlds = {sum(len(rows[f][s]) for f in files for s in (0, 1))} rows (expect {len(files) * 2 * n_w})")
same = all(rows[par][s][w] == rows[cp][s][w] for s in (0, 1) for w in range(n_w))
print(f"control: the parent under a second name scores identically on every world: {same}")


def pairs(f, s):  # mirrored pair means: worlds 2i, 2i+1 share a seed
    v = rows[f][s]
    return [(v[2 * i] + v[2 * i + 1]) / 2 for i in range(n_w // 2)]


def mean_se(vals):
    return st.fmean(vals), st.stdev(vals) / len(vals) ** 0.5


P_all = pairs(par, 0) + pairs(par, 1)
pm, pse = mean_se(P_all)
p1 = st.fmean(rows[par][1].values())
print(f"parent s171: {pm:.2f} +- {pse:.2f} over 96 worlds (set 1 alone {p1:.2f})\n")
arms = sorted({f.split("/")[-2] for f in files if f not in (par, cp)})
print(f"{'arm':4s} {'n':>3s} {'median':>6s} {'p90':>5s} {'best':>5s} {'clearly worse':>13s} {'clearly better':>14s} "
      f"{'pick top 1/4 by set 0 -> set 1 gain':>36s} {'racing pick':>12s} {'best-1 pick gain':>16s}")
rng = random.Random(5)
for a in arms:
    kids = [f for f in files if f.split("/")[-2] == a]
    m = {}
    for f in kids:
        v = pairs(f, 0) + pairs(f, 1)
        d = [x - y for x, y in zip(v, P_all)]
        m[f] = (st.fmean(v), st.fmean(d), st.stdev(d) / len(d) ** 0.5)
    means = sorted(x[0] for x in m.values())
    worse = sum(x[1] < -2 * x[2] for x in m.values())
    better = sum(x[1] > 2 * x[2] for x in m.values())
    s0 = {f: st.fmean(rows[f][0].values()) for f in kids}
    s1 = {f: st.fmean(rows[f][1].values()) for f in kids}
    k = len(kids) // 4
    top = sorted(kids, key=lambda f: -s0[f])[:k]
    gain = st.fmean(s1[f] for f in top) - p1
    # racing: first 8 worlds of set 0, keep half, then all 48
    r8 = {f: st.fmean(rows[f][0][w] for w in range(8)) for f in kids}
    half = sorted(kids, key=lambda f: -r8[f])[: (len(kids) + 1) // 2]
    rtop = sorted(half, key=lambda f: -s0[f])[:k]
    rgain = st.fmean(s1[f] for f in rtop) - p1
    best1 = max(kids, key=lambda f: s0[f])
    print(f"{a:4s} {len(kids):3d} {means[len(means) // 2]:6.2f} {means[int(0.9 * len(means))]:5.2f} {means[-1]:5.2f} "
          f"{worse:13d} {better:14d} {gain:+36.2f} {rgain:+12.2f} {s1[best1] - p1:+16.2f}")
print("\n(clearly = more than 2 s.e. from the parent, paired over 96 worlds, clustered by pair; gains on set 1 only)")
