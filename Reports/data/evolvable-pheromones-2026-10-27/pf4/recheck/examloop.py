#!/usr/bin/env python3
"""The exam's garden worlds (scratch/inrun/exam.py GARDEN, seeds 500001..), events.csv kept, for the loop count."""
import os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
OUT, NAME, POOL = sys.argv[1], sys.argv[2], sys.argv[3]
GARDEN = ['frames=200000', 'dist=25', 'spread=10', 'grants=3', 'window=4000', 'sel=off', 'food=mix',
          'lesson=scratch/inrun/genomes/c1.raw.txt:0.1', 'patches=20000:40:120']
SETENV = 'PP_BED_W=448,PP_MIND_DOOR_OPEN=drop'
D = f'{OUT}/{NAME}'; os.makedirs(f'{D}/pool', exist_ok=True)
lines = [l for l in open(POOL).read().split('\n') if l.strip() and not l.startswith('#')]
head = lines[0].split(); members = []
for l in lines[1:]:
    if l.startswith('member '): members.append(dict(members[0]) if members else {}); continue
    i, w = l.split(); members[-1][int(i)] = w
F = []
for k, m in enumerate(members):
    fp = f'{D}/pool/p{k:02}.raw.txt'
    with open(fp, 'w') as f:
        f.write(f'genome {head[3]} {head[4]}\n'); f.writelines(f'{i} {w}\n' for i, w in sorted(m.items()) if float(w) != 0.0)
    F.append(f'{fp}@p{k:02}:1')
F = ','.join(F)
def garden(s):
    o = f'{D}/w{s}'
    if os.path.exists(f'{o}/fin'): return s
    with open(f'{o}.log', 'w') as lf:
        subprocess.run(['target/release/examples/inrun', f'founders={F}', f'seed={s}'] + GARDEN + [f'setenv={SETENV}', f'out={o}'],
                       stdout=lf, stderr=subprocess.STDOUT, env={**os.environ, 'RAYON_NUM_THREADS': '1'})
    for fn in os.listdir(o):
        if fn.startswith(('ants_f', 'map_f', 'nestmap_f', 'occupants')): os.remove(f'{o}/{fn}')
    open(f'{o}/fin', 'w').close(); return s
with ThreadPoolExecutor(int(os.environ.get('P', 4))) as ex:
    for s in ex.map(garden, range(500001, 500017)): print('done', NAME, s, flush=True)
