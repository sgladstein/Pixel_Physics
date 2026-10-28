import os, subprocess, sys, json
from concurrent.futures import ThreadPoolExecutor
S = sys.argv[1]
BASE = {**os.environ, 'RAYON_NUM_THREADS': '1', 'PP_MIND_ALARM': 'train', 'PP_GATE': 'off', 'PP_REACH': 'off',
        'PIXEL_PHYSICS_ALARM_GROUND': '0.03:0.05', 'PP_BED_W': '448'}
BED = ['sel=off', 'frames=100000', 'dist=100', 'food=mix', 'patches=20000:100:250', 'maps=0']
G = {'free': (f'{S}/x/free/pf4w_free/g039/c00.raw.txt', 'A,B'), 'freedeaf': (f'{S}/x/free/pf4w_free/g039/c00.raw.txt', 'A,B,alarm'),
     'deaf': (f'{S}/x/deaf/pf4w_deaf/g039/c12.raw.txt', 'A,B,alarm')}
jobs = [(g, s) for g in G for s in range(5000500, 5000516)]
def one(j):
    g, s = j; out = f'{S}/h2h/{g}/s{s}'
    if not os.path.exists(f'{out}/fin'):
        os.makedirs(out, exist_ok=True)
        subprocess.run(['target/release/examples/inrun', f'founders={G[g][0]}:30', f'seed={s}', f'out={out}'] + BED,
                       stdout=open(f'{out}.log', 'w'), stderr=subprocess.STDOUT, env={**BASE, 'PP_SCENT_DEAF': G[g][1]})
        open(f'{out}/fin', 'w').close()
    return j
with ThreadPoolExecutor(4) as ex:
    for j in ex.map(one, jobs): print('done', *j, flush=True)
