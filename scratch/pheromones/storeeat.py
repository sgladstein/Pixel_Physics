#!/usr/bin/env python3
"""SCRATCH (evolvable-pheromones lane, 2026-10-25): **do homecoming foragers graze the store instead of digging?**
The per-ant reading behind `Reports/brain-training/evolvable-pheromones-2026-10-25.md` §6, from `events.csv` alone
(no `trace=` needed), so it runs on any inrun or colsel garden directory.

An ant is a *forager* if it took food in the field (`field` event) inside the window, else a *stayer*. Per class:
store eating (`homeeat`), food put down at home that was not a delivery (`churn`), deliveries, and cells dug.
Slow-fading B showed up as foragers' homeeat and churn up and dig down (4222 / 3352 / 221 vs 3024 / 2225 / 275 at
B fade 0.25, 8 seeds, second half), with stayers unchanged.

  python3 scratch/pheromones/storeeat.py [FROM=100000] [TO=inf] DIR [DIR ...]
  python3 scratch/pheromones/storeeat.py FROM=100000 ctl/s*/ -- b25/s*/      # two arms: medians and paired wins

With `--`, the directories before and after are two arms paired in the order given (sort your globs the same way).
"""
import statistics as st
import sys
from collections import defaultdict

KINDS = ('homeeat', 'churn', 'deliver', 'dig')


def one(d, lo, hi):
    ev = defaultdict(lambda: defaultdict(int))
    field = set()
    with open(f'{d.rstrip("/")}/events.csv') as f:
        next(f)
        rows = []
        for line in f:
            p = line.split(',', 3)
            fr = int(p[0])
            if fr <= lo or fr > hi:
                continue
            if p[2] == 'field':
                field.add(p[1])
            elif p[2] in KINDS:
                rows.append((p[1], p[2]))
    for who, k in rows:
        ev['forager' if who in field else 'stayer'][k] += 1
    r = {'foragers': len(field)}
    for c in ('forager', 'stayer'):
        for k in KINDS:
            r[f'{c}_{k}'] = ev[c][k]
    return r


def main(argv):
    lo, hi = 100000, float('inf')
    dirs = []
    for a in argv:
        if a.startswith('FROM='):
            lo = int(a[5:])
        elif a.startswith('TO='):
            hi = int(a[3:])
        else:
            dirs.append(a)
    if '--' in dirs:
        i = dirs.index('--')
        a, b = [one(d, lo, hi) for d in dirs[:i]], [one(d, lo, hi) for d in dirs[i + 1:]]
        assert len(a) == len(b), f'unpaired: {len(a)} vs {len(b)} directories'
        print(f'n={len(a)} pairs, frames ({lo}, {hi}]')
        print(f'{"measure":20} {"arm1":>10} {"arm2":>10}  arm2>arm1')
        for k in a[0]:
            x, y = [r[k] for r in a], [r[k] for r in b]
            print(f'{k:20} {st.median(x):>10.1f} {st.median(y):>10.1f}  {sum(q > p for p, q in zip(x, y))}/{len(a)}')
    else:
        rs = [(d, one(d, lo, hi)) for d in dirs]
        keys = list(rs[0][1])
        print('dir,' + ','.join(keys))
        for d, r in rs:
            print(d + ',' + ','.join(str(r[k]) for k in keys))


if __name__ == '__main__':
    main(sys.argv[1:])
