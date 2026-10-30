"""SCRATCH (chambersense lane, 2026-10-30): paired sign tests from sheet.py's counts.tsv, each arm against BASE.
    python3 scratch/nest/chambersense/pairs.py counts.tsv BASE
Missing / extinct runs count as alive 0, passages 0, rooms 0 (the worse of the pair). Ties are dropped (n printed).
"""
import sys, math
from collections import defaultdict
rows = [l.rstrip('\n').split('\t') for l in open(sys.argv[1])][1:]
base = sys.argv[2]
keys = ['alive', 'passages', 'open_rooms', 'room_cells']
v = defaultdict(dict)
for r in rows:
    w, a = r[0], r[1]
    v[a][w] = dict(zip(keys, map(float, r[2:6]))) if r[2] != 'missing' else dict.fromkeys(keys, 0.0)


def sign(up, dn):
    n = up + dn
    if n == 0:
        return 1.0
    k = min(up, dn)
    return min(1.0, 2 * sum(math.comb(n, i) for i in range(k + 1)) / 2 ** n)


for a in v:
    if a == base:
        continue
    out = []
    for k in keys:
        ws = sorted(set(v[a]) & set(v[base]))
        up = sum(v[a][w][k] > v[base][w][k] for w in ws)
        dn = sum(v[a][w][k] < v[base][w][k] for w in ws)
        med = lambda arm: sorted(v[arm][w][k] for w in ws)[len(ws) // 2]
        out.append(f'{k} {up}/{dn} p {sign(up, dn):.3f} (median {med(a):g} vs {med(base):g})')
    print(f'{a} vs {base}: ' + ' | '.join(out))
