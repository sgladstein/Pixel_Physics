"""Common garden for the colony-selection prototype: each colony's carried ants after the last round, in a fixed fresh
world (seed c*1000+99), 200k frames, selection off, plus unselected hA1 mutants (base) and the old ant (old).
usage: cg.py S NCOL LASTROUND GENOME   -> S/cg/results.csv"""
import csv, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
S, NCOL, LR, G = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
BIN = '/home/user/sr/target/release/examples/inrun'
C = 'frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 drip=20 kevents=10 w_home_eat=2 fprice=tank progress=1'.split()


def carried(rdir):
    idx = f'{rdir}/final/index.csv'
    if not os.path.exists(idx):
        return None
    r = list(csv.DictReader(open(idx)))
    r.sort(key=lambda x: int(x['id']))
    r.sort(key=lambda x: (-float(x['score']), int(x['id'])))
    r = r[:30]
    n = len(r)
    if not n:
        return None
    return ','.join(f"{rdir}/final/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30 // n + (1 if i < 30 % n else 0)}" for i, x in enumerate(r))


jobs = []
for c in range(1, NCOL + 1):
    for arm in ('plain', 'colony'):
        f = carried(f'{S}/{arm}/c{c}/r{LR}')
        jobs.append((f'{arm}_{c}', c, [f'founders={f}'] if f else None))
    jobs.append((f'base_{c}', c, [f'founders={G}:1', 'mutants=30:0.03:1.0:7']))
    jobs.append((f'old_{c}', c, [f'founders={G}:30', 'nomind=1']))


def one(j):
    name, c, fo = j
    if fo is None:
        return name, None
    out = f'{S}/cg/{name}'
    os.makedirs(out, exist_ok=True)
    with open(out + '.log', 'w') as lf:
        subprocess.run([BIN] + fo + [f'seed={c * 1000 + 99}'] + C + [f'out={out}'], stdout=lf, stderr=subprocess.STDOUT)
    home = cuts = late = starved = 0
    with open(f'{out}/events.csv') as f:
        next(f)
        for line in f:
            p = line.split(',')
            if p[2] == 'deliver' and abs(int(p[3]) - 128) <= 8:
                home += 1
            elif p[2] == 'dig' and abs(int(p[3]) - 128) <= 20:
                cuts += 1
                late += int(p[0]) > 100000
            elif p[2] == 'death' and p[5] == '1':
                starved += 1
    rows = list(csv.DictReader(open(f'{out}/nest.csv')))
    ns = float(rows[-1]['nest_score']) if rows else 0.0
    ch = int(rows[-1]['chambers']) if rows else 0
    for fn in os.listdir(out):
        if fn.startswith(('map_f', 'ants_f')) or fn == 'events.csv':
            os.remove(f'{out}/{fn}')
    return name, (home, cuts, late, starved, ns, ch)


os.makedirs(f'{S}/cg', exist_ok=True)
with ThreadPoolExecutor(4) as ex:
    res = list(ex.map(one, jobs))
with open(f'{S}/cg/results.csv', 'w') as f:
    f.write('name,home8,cuts20,late_cuts,starved,nest_score,chambers\n')
    for name, v in res:
        f.write(f'{name},' + (','.join(str(x) for x in v) if v else 'EXTINCT,,,,,') + '\n')
print('CG DONE')
