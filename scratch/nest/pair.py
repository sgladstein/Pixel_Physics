"""SCRATCH (nest lane): paired table of two baseline.py arms -- medians, per-world wins and a two-sided sign test.
usage: pair.py A.csv B.csv [KEY (world|box,seed)] -> per frame and measure: median A, median B, A>B, A<B, ties, p"""
import csv, sys
from math import comb
from statistics import median

A, B = list(csv.DictReader(open(sys.argv[1]))), list(csv.DictReader(open(sys.argv[2])))
key = lambda r: (r.get('world') or (r['box'] + ':' + r['seed']), r.get('frame', '0'))
b = {key(r): r for r in B}
M = ["alive", "starved", "nest_open", "crater", "crumbs", "rooms", "ants_in"] + (["deliveries"] if "deliveries" in A[0] else [])


def sign_p(w, l):
    n = w + l
    if n == 0:
        return 1.0
    k = min(w, l)
    return min(1.0, 2 * sum(comb(n, i) for i in range(k + 1)) / 2 ** n)


for fr in sorted({r.get('frame', '0') for r in A}, key=int):
    pairs = [(r, b[key(r)]) for r in A if r.get('frame', '0') == fr and key(r) in b]
    print(f'frame {fr}: {len(pairs)} pairs')
    for m in M:
        x = [(float(p[m]), float(q[m])) for p, q in pairs]
        w, l = sum(u > v for u, v in x), sum(u < v for u, v in x)
        print(f'  {m:10s} A {median(u for u, _ in x):7.1f}  B {median(v for _, v in x):7.1f}   A>B {w:2d}  A<B {l:2d}  tie {len(x) - w - l:2d}  p {sign_p(w, l):.4f}')
