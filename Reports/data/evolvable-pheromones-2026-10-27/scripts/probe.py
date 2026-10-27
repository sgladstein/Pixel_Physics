"""Run inrun on the emergence patch bed for a list of arms x seeds; score like emerge.py, plus per-family
survival (founders alive at end, starved, births) from ants.csv/events.csv/births.csv.
usage: probe.py OUT ARMS.json SEEDS P"""
import csv, json, os, subprocess, sys
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0, '/home/user/Pixel_Physics/scratch/inrun')
import nestcount
BIN = os.environ.get('BIN', '/tmp/claude-0/-home-user-Pixel-Physics/2ef7e65c-531a-5db4-866b-18b502ff6e79/scratchpad/em/inrun.bin')
OUT, arms, seeds, P = sys.argv[1], json.load(open(sys.argv[2])), [int(s) for s in sys.argv[3].split(',')], int(sys.argv[4])
FRAMES = 100000
BED = ['sel=off', f'frames={FRAMES}', 'dist=100', 'food=mix', 'patches=20000:30:110', 'maps=0']
def one(job):
    name, a, seed = job
    d = f'{OUT}/{name}/s{seed}'
    os.makedirs(d, exist_ok=True)
    env = {**os.environ, 'RAYON_NUM_THREADS': '1', 'PP_MIND_ALARM': 'train', **a.get('env', {})}
    if not os.path.exists(f'{d}/.done'):
        with open(f'{d}.log', 'w') as lf:
            subprocess.run([BIN, f'founders={a["founders"]}', f'seed={seed}', f'out={d}'] + BED + a.get('args', []), stdout=lf, stderr=subprocess.STDOUT, env=env)
        open(f'{d}/.done', 'w').close()
    r = dict(arm=name, seed=seed, stored=0, nest=0, alive=0)
    try:
        last = list(csv.DictReader(open(f'{d}/stock.csv')))[-1]
        r['stored'] = int(last['nest_food']) + int(last['door_food']); r['alive'] = int(last['alive'])
    except Exception: pass
    if r['alive'] and os.path.exists(f'{d}/nestmap.txt'):
        r['nest'] = nestcount.count(d, FRAMES).get('nest', 0)
    r['score'] = r['stored'] + r['nest'] if r['alive'] else 0
    fam = {}
    for row in csv.DictReader(open(f'{d}/ants.csv')): fam[row['id']] = row['tag']
    n = defaultdict(int); dead = defaultdict(int); starv = defaultdict(int)
    for t in fam.values(): n[t] += 1
    with open(f'{d}/events.csv') as f:
        next(f)
        for line in f:
            p = line.rstrip().split(',')
            if p[2] == 'death' and p[1] in fam:
                dead[fam[p[1]]] += 1
                if p[5] == '1': starv[fam[p[1]]] += 1
    r['fam'] = {t: dict(n=n[t], alive=n[t] - dead[t], starved=starv[t]) for t in n}
    return r
jobs = [(nm, a, s) for nm, a in arms.items() for s in seeds]
with ThreadPoolExecutor(P) as ex:
    res = list(ex.map(one, jobs))
json.dump(res, open(f'{OUT}/results.json', 'w'))
print('done', len(res))
