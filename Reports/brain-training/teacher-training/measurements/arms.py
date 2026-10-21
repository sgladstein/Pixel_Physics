"""SCRATCH (teacher-training ceiling test, not for main): compare reliab CSVs of one brain under different teacher arms.
usage: python3 -I arms.py BASELINE.csv ARM.csv...
Unpaired: measurement 1 showed a brain that behaves differently decorrelates from its source world by world, so a
paired test buys nothing. **Standard errors are clustered by seed** (evidence review M4, 2026-10-21): the 96 worlds are
two sets of 48 drawn as mirrored pairs (`training_layouts`: worlds 2i and 2i+1 share a seed), and twins often run
identically, so the unit is the pair: s.e. = sd of the 48 pair means / sqrt(48). Tail counts are per seed too."""
import csv, statistics as st, sys
from collections import defaultdict


def load(p):
    pairs = defaultdict(list)
    for r in csv.DictReader(open(p)):
        pairs[(r["set"], int(r["world"]) // 2)].append((float(r["full_loads"]), float(r["score"])))
    fl = [st.fmean(x[0] for x in v) for v in pairs.values()]
    sc = [st.fmean(x[1] for x in v) for v in pairs.values()]
    return fl, sc


base = load(sys.argv[1])
se = lambda v: st.stdev(v) / len(v) ** 0.5
print(f"{'arm':26s} {'seeds':>5s} {'full loads':>15s} {'vs first':>15s} {'dig score':>15s} {'seeds <3':>8s} {'p10/med/p90 (seed means)':>24s}")
for p in sys.argv[1:]:
    fl, sc = load(p)
    d1, s1 = st.fmean(fl) - st.fmean(base[0]), (se(fl) ** 2 + se(base[0]) ** 2) ** 0.5
    q = sorted(fl)
    n = len(q)
    print(f"{p.split('/')[-1][:-4]:26s} {n:5d} {st.fmean(fl):6.2f} +- {se(fl):4.2f} {d1:+6.2f} +- {s1:4.2f} "
          f"{st.fmean(sc):6.1f} +- {se(sc):4.1f} {sum(x < 3 for x in fl):8d} {q[n // 10]:7.1f}/{q[n // 2]:4.1f}/{q[(9 * n) // 10]:4.1f}")
