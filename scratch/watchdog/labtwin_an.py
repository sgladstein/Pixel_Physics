#!/usr/bin/env python3
"""SCRATCH (watchdog, 2026-10-28): labtwin.sh logs against the voted lab case. usage: labtwin_an.py OUTDIR COLSEL.csv ROUND"""
import csv, glob, math, os, re, statistics as st, sys
out, cs, R = sys.argv[1], sys.argv[2], sys.argv[3]
vote = {r['colony']: r for r in csv.DictReader(open(cs)) if r['round'] == R}
runs = {}
for f in glob.glob(f'{out}/r{R}c*_*.log'):
    m = re.match(rf'r{R}c(\d+)_(.+)\.log', os.path.basename(f))
    t = open(f).read()
    if 'RESULT' not in t:
        continue
    ns = {int(a): (int(b), int(c)) for a, b, c in re.findall(r'NESTAT f=(\d+) alive=(\d+) starved=(\d+)', t)}
    res = dict(re.findall(r'(\w+)=([\d.-]+)', t.split('RESULT')[1]))
    runs.setdefault(m.group(1), {})[m.group(2)] = (ns, res)


def rank(v):
    o = sorted(range(len(v)), key=lambda i: v[i]); r = [0.0] * len(v); i = 0
    while i < len(o):
        j = i
        while j + 1 < len(o) and v[o[j + 1]] == v[o[i]]:
            j += 1
        for k in range(i, j + 1):
            r[o[k]] = (i + j) / 2
        i = j + 1
    return r


def sp(a, b):
    ra, rb = rank(a), rank(b); ma, mb = st.mean(ra), st.mean(rb)
    d = math.sqrt(sum((x - ma) ** 2 for x in ra) * sum((y - mb) ** 2 for y in rb))
    return sum((x - ma) * (y - mb) for x, y in zip(ra, rb)) / d if d else float('nan')


def mw(a, b):
    """Mann-Whitney U, normal approximation, two-sided p."""
    n1, n2 = len(a), len(b); r = rank(a + b); u = sum(r[:n1]) + n1 - n1 * (n1 + 1) / 2
    mu, sd = n1 * n2 / 2, math.sqrt(n1 * n2 * (n1 + n2 + 1) / 12)
    z = (u - mu) / sd if sd else 0
    return u, math.erfc(abs(z) / math.sqrt(2))


rows = []
for c, sc in sorted(runs.items(), key=lambda x: int(x[0])):
    if len(sc) < 2:
        continue
    late = sum(st.mean(ns[f][0] for f in ns if f >= 50000) for ns, _ in sc.values())
    end = sum(ns[max(ns)][0] for ns, _ in sc.values())
    starved = sum(int(r['starved']) for _, r in sc.values())
    births = sum(int(r['births']) for _, r in sc.values())
    a15 = sum(ns[15000][0] for ns, _ in sc.values())
    rows.append(dict(c=int(c), lab=int(vote[c]['lab']), rank=int(vote[c]['rank']), alive15=a15, late=late, end300=end,
                     starved=starved, births=births, spb=starved / max(1, births)))
print('colony lab rank alive@15k meanalive50-300k alive@300k starved births starved/birth')
for r in sorted(rows, key=lambda r: -r['lab']):
    print(f"c{r['c']:<3} {r['lab']:5} {r['rank']:3} {r['alive15']:5} {r['late']:7.0f} {r['end300']:5} {r['starved']:6} {r['births']:6} {r['spb']:.2f}")
L = [r['lab'] for r in rows]
print(f'n={len(rows)}')
for k in ('alive15', 'late', 'end300', 'starved', 'births', 'spb'):
    print(f'  Spearman(lab case, {k}) = {sp(L, [r[k] for r in rows]):+.2f}')
s = sorted(rows, key=lambda r: -r['lab']); h = len(s) // 2
u, p = mw([r['late'] for r in s[:h]], [r['late'] for r in s[h:]])
print(f"top half by lab: mean alive 50-300k median {st.median(r['late'] for r in s[:h]):.0f} vs bottom {st.median(r['late'] for r in s[h:]):.0f}, Mann-Whitney p={p:.2g}")
