"""SCRATCH (copy test): paired comparison of two arms in a `boxes.py` CSV, per box and pooled: medians, how many seeds
A beats B, and a two-sided sign test (ties dropped).

usage: cmp.py CSV A B [metrics=lived,alive,starved,deliveries]
"""
import csv, sys
from math import comb
from statistics import median

rows = list(csv.DictReader(open(sys.argv[1])))
A, Bn = sys.argv[2], sys.argv[3]
metrics = (sys.argv[4].split('=', 1)[1] if len(sys.argv) > 4 else 'lived,alive,starved,deliveries').split(',')


def sign_p(w, l):
    n = w + l
    if n == 0:
        return 1.0
    k = min(w, l)
    return min(1.0, 2 * sum(comb(n, i) for i in range(k + 1)) / 2 ** n)


by = {(r['arm'], r['box'], r['seed']): r for r in rows}
boxes = sorted({r['box'] for r in rows})
for m in metrics:
    allw = alll = 0
    for b in boxes + ['ALL']:
        pairs = [(float(by[(A, bb, s)][m]), float(by[(Bn, bb, s)][m])) for (a, bb, s) in by
                 if a == A and (b == 'ALL' or bb == b) and (Bn, bb, s) in by]
        if not pairs:
            continue
        w = sum(x > y for x, y in pairs)
        l = sum(x < y for x, y in pairs)
        print(f'{m:11s} {b:17s} n={len(pairs):2d}  {A} med {median(x for x, _ in pairs):8.1f} | {Bn} med {median(y for _, y in pairs):8.1f}'
              f' | {A} higher {w}/{len(pairs)}, lower {l}  sign p={sign_p(w, l):.3g}')
