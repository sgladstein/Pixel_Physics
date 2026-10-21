"""SCRATCH (teacher-training ceiling test, not for main): compare reliab CSVs of one brain under different teacher arms.
usage: python3 -I arms.py BASELINE.csv ARM.csv...
Unpaired: measurement 1 showed a brain that behaves differently decorrelates from its source world by world, so a
paired test buys nothing. Prints each arm's mean full loads and dig score with standard errors, and the difference
from the baseline with its standard error (sqrt of the two squared s.e.)."""
import csv, statistics as st, sys


def load(p):
    rows = list(csv.DictReader(open(p)))
    return [float(r["full_loads"]) for r in rows], [float(r["score"]) for r in rows]


base = load(sys.argv[1])
se = lambda v: st.stdev(v) / len(v) ** 0.5
print(f"{'arm':38s} {'n':>3s} {'full loads':>16s} {'vs brain alone':>16s} {'dig score':>16s} {'vs brain alone':>16s}")
for p in sys.argv[1:]:
    fl, sc = load(p)
    d1, s1 = st.fmean(fl) - st.fmean(base[0]), (se(fl) ** 2 + se(base[0]) ** 2) ** 0.5
    d2, s2 = st.fmean(sc) - st.fmean(base[1]), (se(sc) ** 2 + se(base[1]) ** 2) ** 0.5
    print(f"{p.split('/')[-1][:-4]:38s} {len(fl):3d} {st.fmean(fl):7.2f} +- {se(fl):5.2f} {d1:+7.2f} +- {s1:5.2f} "
          f"{st.fmean(sc):7.1f} +- {se(sc):5.1f} {d2:+7.1f} +- {s2:5.1f}")
