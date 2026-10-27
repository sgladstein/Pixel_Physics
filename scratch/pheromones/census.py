#!/usr/bin/env python3
"""SCRATCH (evolvable-pheromones lane, 2026-10-25): signalling census -- mean emit (A, B, alarm) per decision, binned by the
ant's own state. Streams a PP_MIND_LOG log (PP_MIND_LOG_EMITS=1), usually a fifo:  python3 census.py LOG > census.txt"""
import sys, collections
log = sys.argv[1]
NAMES = ["energy","crop","spoil","at_nest","a","b","alarm","d_a","d_b","d_alarm","d_energy","home_seen","home_dist","d_home","since_nest","age","self","crowd","depth"]
ix = {n: i for i, n in enumerate(NAMES)}
pending = {}
bins = collections.defaultdict(lambda: [0, 0.0, 0.0, 0.0])
def add(key, e):
    b = bins[key]; b[0] += 1; b[1] += e[0]; b[2] += e[1]; b[3] += e[2]
with open(log) as f:
    for line in f:
        if line[0] == 'M':
            p = line.split(',', 11)
            k = (p[2], p[3])
            ctx = p[10].split()
            pending[k] = ctx
        elif line[0] == 'E':
            p = line.rstrip().split(',')
            k = (p[2], p[3])
            ctx = pending.pop(k, None)
            if ctx is None:
                continue
            e = [float(x) for x in p[4].split()]
            c = {n: float(ctx[i]) for n, i in ix.items()}
            laden = c['crop'] > 0.05
            add(('all',), e)
            add(('laden' if laden else 'empty',), e)
            add(('at_nest' if c['at_nest'] > 0.5 else 'away', 'laden' if laden else 'empty'), e)
            add(('home_dist', min(int(c['home_dist'] * 10), 9) / 10), e)
            add(('energy', min(int(c['energy'] * 10), 9) / 10), e)
            add(('depth>0' if c['depth'] > 0 else 'surface', 'laden' if laden else 'empty'), e)
for k in sorted(bins, key=str):
    n, a, b, al = bins[k]
    print(f"{' '.join(map(str, k)):28} n={n:>8}  A {a/n:.3f}  B {b/n:.3f}  alarm {al/n:.3f}")
