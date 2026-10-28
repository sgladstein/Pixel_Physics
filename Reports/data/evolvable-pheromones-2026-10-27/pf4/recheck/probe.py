#!/usr/bin/env python3
"""PF4 verification: replay genomes on fresh worlds, keep events.csv, and split food delivered home by where the
delivered cell was picked up (at the heap vs near the nest = recycled)."""
import csv, json, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
BIN = os.path.abspath('target/release/examples/inrun')
OUT = sys.argv[1]
JOBS = json.load(open(sys.argv[2]))  # list of [name, genome, seed, envdict]
P = int(os.environ.get('P', 4))
BED = ['sel=off', 'frames=100000', 'dist=100', 'food=mix', 'patches=20000:30:110', 'maps=0']
BASE = {**os.environ, 'RAYON_NUM_THREADS': '1', 'PP_MIND_ALARM': 'train', 'PP_GATE': 'off', 'PP_REACH': 'off',
        'PIXEL_PHYSICS_ALARM_GROUND': '0.03:0.05'}

def one(job):
    name, genome, seed, extra = job
    out = f'{OUT}/{name}/s{seed}'
    if not os.path.exists(f'{out}/nest.csv') or not os.path.exists(f'{out}/fin'):
        os.makedirs(out, exist_ok=True)
        with open(f'{out}.log', 'w') as lf:
            subprocess.run([BIN, f'founders={genome}:30', f'seed={seed}', f'out={out}'] + BED, stdout=lf,
                           stderr=subprocess.STDOUT, env={**BASE, **extra})
        open(f'{out}/fin', 'w').close()
    return name, seed

with ThreadPoolExecutor(P) as ex:
    for name, seed in ex.map(one, JOBS):
        print('done', name, seed, flush=True)
