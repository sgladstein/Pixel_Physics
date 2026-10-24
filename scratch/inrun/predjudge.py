"""predjudge.py run|sum OUT (env S, SIGHT, COLS, EVO, ORIG, START, P): the arms-race cross judge for predator stage three.

Colonies {founder pool, coevo's last carry, fixed's last carry} x spiders {the original pool score-1, coevo's last
spool}, each colony c on its garden world (seed c*1000+99), sel=off, 200k frames, spiders from frame START every GAP.
Writes OUT/rows.csv: colony_set,spider_set,c,home8,starved,killed,spiders_slain,placed,alive.
"""
import csv, glob, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

S = os.environ.get('S', '.')  # holds ce/ (the colsel arms) and bin/inrun_def
CE = f'{S}/ce'
BIN = f'{S}/bin/inrun_def'
SIGHT = os.environ.get('SIGHT', 'off')
COLS = os.environ.get('COLS', 'founder,coevo,fixed').split(',')
EVO = os.environ.get('EVO', 'coevo')
START, GAP = os.environ.get('START', '60000'), '50000'
ORIG = os.environ.get('ORIG', '/mnt/project-files/predator/pools/score-1')


def carry(arm, c):
    """colsel's carry of colony c's last finished round."""
    rds = sorted((d for d in glob.glob(f'{CE}/{arm}/c{c}/r*') if os.path.exists(f'{d}/final/index.csv')),
                 key=lambda d: int(d.rsplit('/r', 1)[1]))
    if not rds:
        return None
    rd = rds[-1]
    r = list(csv.DictReader(open(f'{rd}/final/index.csv')))
    r.sort(key=lambda x: int(x['id']))
    r.sort(key=lambda x: (-float(x['score']), int(x['id'])))
    r = r[:30]
    n = len(r)
    if not n:
        return None
    return ','.join(f"{rd}/final/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30 // n + (1 if i < 30 % n else 0)}"
                    for i, x in enumerate(r))


def founder():
    return ','.join(f'{p}@{os.path.basename(p)[:3]}:1' for p in sorted(glob.glob(f'{CE}/coevo/pool/p*.raw.txt')))


def last_spool():
    ds = sorted(glob.glob(f'{CE}/{EVO}/spool/r*'), key=lambda d: int(d.rsplit('/r', 1)[1]))
    return ds[-1]


def measure(d):
    home = starved = killed = slain = placed = 0
    ev = f'{d}/events.csv'
    if os.path.exists(ev):
        with open(ev) as f:
            next(f)
            for line in f:
                p = line.split(',')
                k = p[2]
                if k == 'deliver' and abs(int(p[3]) - 128) <= 8:
                    home += 1
                elif k == 'death':
                    starved += p[5] == '1'
                    killed += p[5] == '3'
                elif k == 'spdeath':
                    slain += p[5] == '3'
                elif k == 'spider':
                    placed += 1
    alive = 0
    if os.path.exists(f'{d}/nest.csv'):
        rows = list(csv.DictReader(open(f'{d}/nest.csv')))
        alive = int(rows[-1]['alive']) if rows else 0
    return home, starved, killed, slain, placed, alive


def main():
    mode, out = sys.argv[1], sys.argv[2]
    if mode == 'run':
        spools = {'orig': ORIG, 'evolved': last_spool()}
        cols = {n: {c: (founder() if n == 'founder' else carry(n, c)) for c in range(1, 17)} for n in COLS}
        jobs = [(cs, ss, c) for cs in cols for ss in spools for c in range(1, 17) if cols[cs][c]]
        base = ['frames=200000', 'dist=25', 'spread=10', 'grants=3', 'window=4000', 'sel=off',
                'lesson=scratch/inrun/genomes/c1.raw.txt:0.1', f'spider_gap={GAP}', f'spider_start={START}']

        def one(j):
            cs, ss, c = j
            d = f'{out}/{cs}-{ss}-c{c}'
            os.makedirs(d, exist_ok=True)
            with open(f'{d}.log', 'w') as lf:
                subprocess.run([BIN, f'founders={cols[cs][c]}', f'seed={c * 1000 + 99}', f'spiders={spools[ss]}:1']
                               + base + [f'out={d}'], stdout=lf, stderr=subprocess.STDOUT, cwd='/home/user/Pixel_Physics',
                               env=dict(os.environ, PIXEL_PHYSICS_MIND_SIGHT=SIGHT))
            v = measure(d)
            for fn in os.listdir(d):
                if fn.startswith(('map_f', 'ants_f')) or fn.endswith('.raw.txt'):
                    os.remove(f'{d}/{fn}')
            return (cs, ss, c) + v
        with ThreadPoolExecutor(int(os.environ.get('P', '4'))) as ex:
            rows = list(ex.map(one, jobs))
        with open(f'{out}/rows.csv', 'w') as f:
            f.write('colony_set,spider_set,c,home8,starved,killed,spiders_slain,placed,alive\n')
            for r in rows:
                f.write(','.join(str(x) for x in r) + '\n')
        print('spools', spools)
    rows = list(csv.DictReader(open(f'{out}/rows.csv')))
    med = lambda v: sorted(v)[len(v) // 2]
    print('colonies  spiders   n  killed(med/mean)  slain(mean)  home8(med)  starved(med)  alive(med)')
    for cs in COLS:
        for ss in ('orig', 'evolved'):
            g = [r for r in rows if r['colony_set'] == cs and r['spider_set'] == ss]
            if not g:
                continue
            k = [int(r['killed']) for r in g]
            print(f"{cs:9} {ss:8} {len(g):3}  {med(k):5} / {sum(k) / len(k):5.1f}  {sum(int(r['spiders_slain']) for r in g) / len(g):8.2f}"
                  f"  {med([int(r['home8']) for r in g]):9}  {med([int(r['starved']) for r in g]):9}  {med([int(r['alive']) for r in g]):9}")


main()
