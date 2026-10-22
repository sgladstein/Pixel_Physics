"""Summary of the fix test. usage: summ.py SCRATCH_DIR [MUTLOAD_DIR]"""
import sys, csv, glob, os, statistics as st
S = sys.argv[1]; M = sys.argv[2] if len(sys.argv) > 2 else None
res = {}
for line in open(f'{S}/cg/results.txt'):
    p = line.split()
    if len(p) >= 2: res[p[0]] = None if p[1] == 'EXTINCT' else list(map(int, p[1:5]))
arms = ['base', 'old', 'ctl', 'band', 'cross', 'lex', 'prot']
print('arm    n extinct | home8 median (mean) | starved mean | cuts20 median (mean) | colonies <20 cuts | field median')
for a in arms:
    rows = [res.get(f'{a}_{s}') for s in range(1, 13)]
    ext = sum(r is None for r in rows); ok = [r for r in rows if r]
    if not ok: print(f'{a:6s} {len(rows):2d} {ext:2d} | (none)'); continue
    h = [r[0] for r in ok]; sv = [r[1] for r in ok]; c = [r[2] for r in ok]; f = [r[3] for r in ok]
    print(f'{a:6s} {len(rows):2d} {ext:2d} | {st.median(h):7.0f} ({st.mean(h):6.0f}) | {st.mean(sv):5.1f} | {st.median(c):5.0f} ({st.mean(c):5.0f}) | {sum(x < 20 for x in c):2d} of {len(c)} | {st.median(f):6.0f}')
print('\npaired by seed vs ctl (both alive): cuts higher/lower; home higher/lower; starved fewer/more')
for a in ['band', 'cross', 'lex', 'prot', 'base']:
    cu = cd = hu = hd = sf = sm = n = 0
    for s in range(1, 13):
        x, y = res.get(f'{a}_{s}'), res.get(f'ctl_{s}')
        if not x or not y: continue
        n += 1; cu += x[2] > y[2]; cd += x[2] < y[2]; hu += x[0] > y[0]; hd += x[0] < y[0]; sf += x[1] < y[1]; sm += x[1] > y[1]
    print(f'  {a:6s} n={n}: cuts {cu}/{cd}; home {hu}/{hd}; starved fewer {sf} more {sm}')
print('\ntraining: births (mean/job), crossed births, max gen at the last round, distinct families at end, rounds survived, final alive')
for a in ['ctl', 'band', 'cross', 'lex', 'prot']:
    births = []; crossed = []; gens = []; fams = []; surv = []; alive = []
    for s in range(1, 13):
        j = f'{S}/jobs/{a}_{s}'
        rs = sorted(glob.glob(f'{j}/r*'), key=lambda p: int(p.rsplit('r', 1)[1]))
        surv.append(len(rs)); b = x = 0
        for r in rs:
            bf = f'{r}/births.csv'
            if os.path.exists(bf):
                for row in csv.DictReader(open(bf)):
                    b += row['kind'] in ('birth', 'cross', 'refill', 'hall'); x += row['kind'] == 'cross'
        births.append(b); crossed.append(x)
        if rs:
            idx = f'{rs[-1]}/final/index.csv'
            if os.path.exists(idx):
                rows = list(csv.DictReader(open(idx)))
                if rows:
                    gens.append(max(int(r['gen']) for r in rows)); fams.append(len({r['family'] for r in rows})); alive.append(len(rows))
    m = lambda v: f'{st.mean(v):.1f}' if v else '-'
    print(f'  {a:6s} births {m(births)} crossed {m(crossed)} gen {m(gens)} families {m(fams)} rounds {m(surv)} alive {m(alive)}')
if M and os.path.exists(f'{M}/cg/results.txt'):
    print('\nbyte-identity check: ctl vs the mutload test\'s scoreM (same seeds, switches off) and base/old vs theirs')
    old = {}
    for line in open(f'{M}/cg/results.txt'):
        p = line.split()
        if len(p) >= 2: old[p[0]] = ' '.join(p[1:])
    same = diff = 0
    for s in range(1, 13):
        for a, b in [(f'ctl_{s}', f'scoreM_{s}'), (f'base_{s}', f'base_{s}'), (f'old_{s}', f'old_{s}')]:
            x = res.get(a); y = old.get(b)
            xs = 'EXTINCT' if x is None else ' '.join(map(str, x))
            if y is None: continue
            if xs == y: same += 1
            else: diff += 1; print(f'  DIFF {a}: {xs} vs {b}: {y}')
    print(f'  identical {same}, different {diff}')
