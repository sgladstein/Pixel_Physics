"""x15 (teacher lane, 2026-10-31): env9's winners (fh3 = F_h3 c22, fstd = F_std c9) against std_pool and N_gate c10,
6 exam boxes x 8 seeds at 600k, one build. Usage: python3 x15an.py lab600k.csv"""
import csv, math, sys
R = {(r['brain'], r['box'], r['seed']): r for r in csv.DictReader(open(sys.argv[1]))}
boxes = sorted({k[1] for k in R}); seeds = sorted({k[2] for k in R}, key=int)
def sign(a, b):
    n = a + b
    return 1.0 if n == 0 else min(1.0, 2 * sum(math.comb(n, i) for i in range(min(a, b) + 1)) / 2 ** n)
def cmp(a, b):
    out = {}
    for m in ('lived', 'alive_end', 'starved_share', 'trip_deliveries', 'digs'):
        w = l = 0
        for bx in boxes:
            for s in seeds:
                x, y = R[(a, bx, s)], R[(b, bx, s)]
                if m == 'alive_end': u, v = int(x['alive']) > 0, int(y['alive']) > 0
                elif m == 'starved_share': u, v = -int(x['starved']) / max(1, int(x['births'])), -int(y['starved']) / max(1, int(y['births']))
                else: u, v = float(x[m]), float(y[m])
                w += u > v; l += u < v
        out[m] = (w, l, sign(w, l))
    return out
for a, b in (('fstd', 'std'), ('fh3', 'std'), ('fstd', 'gate'), ('fh3', 'gate'), ('gate', 'std'), ('fstd', 'fh3')):
    c = cmp(a, b)
    print(f'{a} vs {b}: ' + '  '.join(f'{m} {w}/{l} p {p:.3g}' for m, (w, l, p) in c.items()))
print('per box: alive-at-end of 8, mean lived')
for bx in boxes:
    print(f'  {bx:18s} ' + '  '.join(f"{a} {sum(int(R[(a,bx,s)]['alive'])>0 for s in seeds)}/{sum(float(R[(a,bx,s)]['lived']) for s in seeds)/len(seeds):6.1f}" for a in ('fstd', 'fh3', 'gate', 'std')))
