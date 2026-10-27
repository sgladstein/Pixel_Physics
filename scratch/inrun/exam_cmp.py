"""SCRATCH (teacher lane, 2026-10-27): compare exam.py results, paired by world and by lab box + seed.

usage: exam_cmp.py DIR BASE NAME [NAME...]
For every measure: both medians, how many pairs NAME beats BASE in (ties left out), and the two-sided sign-test p.
Owner, 2026-10-27: quote every difference with its paired count and p, and say which survive the number of
measures compared (Bonferroni: p < 0.05 / k).
"""
import csv, math, statistics as st, sys

D, BASE, NAMES = sys.argv[1], sys.argv[2], sys.argv[3:]
HIGHER = {'alive': 1, 'starved': -1, 'deliveries': 1, 'nest': 1, 'nest_open': 1, 'crater': -1, 'rooms': 1, 'lived': 1}


def load(name, kind, key):
    try:
        return {tuple(r[k] for k in key): r for r in csv.DictReader(open(f'{D}/{name}/{kind}.csv'))}
    except FileNotFoundError:
        return {}


def sign_p(w, l):
    n = w + l
    if n == 0:
        return 1.0
    k = min(w, l)
    return min(1.0, 2 * sum(math.comb(n, i) for i in range(k + 1)) / 2 ** n)


for kind, key, ms in (('garden', ('world',), ('alive', 'starved', 'deliveries', 'nest', 'nest_open', 'crater', 'rooms')),
                      ('lab', ('box', 'seed'), ('lived', 'alive', 'starved', 'deliveries'))):
    base = load(BASE, kind, key)
    for name in NAMES:
        other = load(name, kind, key)
        common = sorted(set(base) & set(other))
        if not common:
            continue
        print(f'== {kind}: {name} vs {BASE}, {len(common)} pairs (Bonferroni bar over {len(ms)} measures: {0.05 / len(ms):.3f})')
        for m in ms:
            a = [float(other[k][m] or 0) for k in common]
            b = [float(base[k][m] or 0) for k in common]
            sg = HIGHER.get(m, 1)
            w = sum(1 for x, y in zip(a, b) if (x - y) * sg > 0)
            l = sum(1 for x, y in zip(a, b) if (x - y) * sg < 0)
            print(f'  {m:11s} {name} {st.median(a):9.1f}  {BASE} {st.median(b):9.1f}   better in {w}/{w + l}  p={sign_p(w, l):.2g}')
