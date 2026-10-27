import csv, os, glob, math, random, statistics as st
D = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data")
METRICS = ["nest", "crater", "starved", "home8", "alive"]
HIGHER_BETTER = {"nest": 1, "crater": -1, "starved": -1, "home8": 1, "alive": 1, "food": 1, "extinct": -1}
def colsel_arms():
    out = {}
    for p in sorted(glob.glob(f"{D}/*/*/cg.csv")):
        b, a = p.split("/")[-3], p.split("/")[-2]
        out[(b, a)] = p
    return out
def load_cg(p):
    by = {}
    for r in csv.DictReader(open(p)):
        try:
            by.setdefault(int(r["round"]), {})[int(r["colony"])] = {k: float(r[k]) for k in METRICS if r.get(k) not in (None, "")}
        except ValueError:
            pass
    return by
def load_sel(p):
    by = {}
    for r in csv.DictReader(open(p)):
        by.setdefault(int(r["round"]), {})[int(r["colony"])] = r
    return by
def binom_p(k, n):
    # two-sided exact sign test
    if n == 0: return 1.0
    k = min(k, n - k)
    s = sum(math.comb(n, i) for i in range(0, k + 1)) / 2 ** n
    return min(1.0, 2 * s)
def call(diffs, alpha=0.05):
    """+1 arm better, -1 worse, 0 no call; diffs already oriented so positive = better"""
    w = sum(d > 0 for d in diffs); l = sum(d < 0 for d in diffs)
    p = binom_p(w, w + l)
    if p < alpha: return (1 if w > l else -1), p
    return 0, p
