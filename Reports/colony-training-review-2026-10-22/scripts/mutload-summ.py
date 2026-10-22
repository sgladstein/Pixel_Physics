"""Summary of the mutation-load test. usage: summ.py SCRATCH_DIR"""
import sys, csv, glob, os, statistics as st, collections
S = sys.argv[1]
res = {}
for line in open(f'{S}/cg/results.txt'):
    p = line.split()
    if len(p) >= 2:
        res[p[0]] = None if p[1] == 'EXTINCT' else list(map(int, p[1:5]))
arms = ['base', 'old', 'scoreM', 'randM', 'score0', 'rand0']
print('arm      n  extinct | home8 median (mean) | starved mean | cuts20 median (mean) | colonies <20 cuts | field median')
for a in arms:
    rows = [res.get(f'{a}_{s}') for s in range(1, 13)]
    ext = sum(r is None for r in rows); ok = [r for r in rows if r]
    if not ok: print(f'{a:8s} {len(rows):2d} {ext:2d} | (none)'); continue
    h = [r[0] for r in ok]; sv = [r[1] for r in ok]; c = [r[2] for r in ok]; f = [r[3] for r in ok]
    print(f'{a:8s} {len(rows):2d} {ext:2d} | {st.median(h):7.0f} ({st.mean(h):6.0f}) | {st.mean(sv):5.1f} | {st.median(c):5.0f} ({st.mean(c):5.0f}) | {sum(x < 20 for x in c):2d} of {len(c)} | {st.median(f):6.0f}')
print()
print('paired by seed (both alive): arm vs scoreM, cuts20 higher/lower; home8 higher/lower')
for a in ['randM', 'score0', 'rand0', 'base']:
    up = dn = hu = hd = 0; n = 0
    for s in range(1, 13):
        x, y = res.get(f'{a}_{s}'), res.get(f'scoreM_{s}')
        if not x or not y: continue
        n += 1; up += x[2] > y[2]; dn += x[2] < y[2]; hu += x[0] > y[0]; hd += x[0] < y[0]
    print(f'  {a:7s} n={n}: cuts higher {up} lower {dn}; home higher {hu} lower {hd}')
print()
print('training rounds: births per job (mean), max generation reached at round 5, distinct families at end (mean), rounds survived')
for a in ['scoreM', 'randM', 'score0', 'rand0']:
    births = []; gens = []; fams = []; surv = []
    for s in range(1, 13):
        j = f'{S}/jobs/{a}_{s}'
        rs = sorted(glob.glob(f'{j}/r*'), key=lambda p: int(p.rsplit('r', 1)[1]))
        surv.append(len(rs))
        b = 0
        for r in rs:
            bf = f'{r}/births.csv'
            if os.path.exists(bf):
                for row in csv.DictReader(open(bf)):
                    b += row['kind'] in ('birth', 'refill', 'hall')
        births.append(b)
        idx = f'{j}/r5/final/index.csv'
        if os.path.exists(idx):
            rows = list(csv.DictReader(open(idx)))
            if rows:
                gens.append(max(int(r['gen']) for r in rows)); fams.append(len({r['family'] for r in rows}))
    m = lambda v: f'{st.mean(v):.1f}' if v else '-'
    print(f'  {a:7s} births {m(births)}  gen@r5 {m(gens)} (max {max(gens) if gens else "-"})  families {m(fams)}  rounds {m(surv)}')
