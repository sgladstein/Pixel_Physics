"""Paired comparison of each arm against a base arm: median, worlds up/down, two-sided sign-test p."""
import csv, sys, statistics
from math import comb
S = sys.argv[1]; base = sys.argv[2] if len(sys.argv) > 2 else 'T'
rows = list(csv.DictReader(open(f'{S}/summary.csv')))
def sign_p(up, down):
    n = up + down
    if n == 0: return 1.0
    k = min(up, down)
    return min(1.0, 2 * sum(comb(n, i) for i in range(k + 1)) / 2 ** n)
M = ['alive', 'starved', 'oldage', 'home', 'nest', 'nest_open', 'crater', 'peak', 'low']
by = {}
for r in rows:
    by.setdefault((r['arm'], int(r['block'])), {})[r['seed']] = r
def table(pairs):
    print(f"{'arm':10}" + ''.join(f'{m:>22}' for m in M))
    for a, b, label in pairs:
        A, B = by.get(a), by.get(b)
        if not A or not B: continue
        seeds = sorted(set(A) & set(B))
        line = f'{label:10}'
        for m in M:
            x = [float(A[s][m]) for s in seeds]; y = [float(B[s][m]) for s in seeds]
            up = sum(1 for i, j in zip(x, y) if i > j); dn = sum(1 for i, j in zip(x, y) if i < j)
            line += f'{statistics.median(x):>8.0f} {up:>2}/{dn:<2} p={sign_p(up, dn):<6.3f}'[:22].rjust(22)
        print(line + f'  n={len(seeds)}')
arms = sorted({a for a, _ in by})
print('# each arm vs', base, '(median of arm; worlds where arm > base / arm < base; sign test)')
table([((a, 0), (base, 0), a) for a in arms])
