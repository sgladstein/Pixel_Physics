"""SCRATCH (chambers lane, eat seed): is the seed still in the ants selection keeps? For each round's top 30 by score
(the next round's founders, as poolrounds.sh picks them): Eat's switch (slot 6, offered while > 0), its job's energy
weight (1680) and constant (1715), and how many other Eat-only slots have drifted off zero.
usage: survive.py RUNDIR...   (a poolrounds.sh OUT: reads OUT/r*/final)"""
import csv, sys, os, glob, statistics as st
JOB, J_IN, HID_OUT, MEM, M_IN = 1464, 36, 2725, 1788, 46
others = [k for k in range(JOB + 6 * J_IN, JOB + 7 * J_IN) if k not in (1680, 1715)] + list(range(HID_OUT + 48, HID_OUT + 56)) + [MEM + m * M_IN + 30 for m in range(12)]
def genome(p):
    g = {}
    for l in open(p).read().split('\n')[1:]:
        if l.strip(): i, x = l.split(); g[int(i)] = float(x)
    return g
for base in sys.argv[1:]:
    for d in sorted(glob.glob(f'{base}/r*/final'), key=lambda s: int(s.split('/r')[-1].split('/')[0])):
        r = list(csv.DictReader(open(f'{d}/index.csv')))
        r.sort(key=lambda x: int(x['id'])); r.sort(key=lambda x: (-float(x['score']), int(x['id'])))
        top = [genome(f'{d}/{x["id"]}.raw.txt') for x in r[:30]]
        if not top: print(base, d, 'none'); continue
        sw = [g.get(6, 0) for g in top]; w = [g.get(1680, 0) for g in top]; b = [g.get(1715, 0) for g in top]
        drift = [sum(1 for k in others if g.get(k, 0) != 0) for g in top]
        print(f'{d:70s} n {len(top):2d} switch>0 {sum(s > 0 for s in sw):2d}/{len(top)} (median {st.median(sw):+.2f}) '
              f'energy w median {st.median(w):+.1f} [{min(w):+.1f},{max(w):+.1f}] const median {st.median(b):+.1f} '
              f'thr {st.median([-bb / ww if ww else float("nan") for bb, ww in zip(b, w)]):.3f} other eat slots live median {st.median(drift)}')
