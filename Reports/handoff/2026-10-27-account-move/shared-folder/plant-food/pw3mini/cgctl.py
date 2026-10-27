"""cgctl.py OUTDIR CAP: the round-0 founder (hcs x30, no selection) in pw3mini's common garden, same seeds (c*1000+99), at pile cap CAP."""
import sys, os, subprocess, csv, statistics as st
from concurrent.futures import ThreadPoolExecutor
S = '/tmp/claude-0/-home-user-Pixel-Physics/b300ed13-cc9c-5e26-8b14-2acd8b3d007f/scratchpad/g'
R = '/home/user/Pixel_Physics'
out, cap = sys.argv[1], sys.argv[2]
extra = open(f'{S}/pw3mini/args.txt').read().split()
extra = [x for x in extra if not x.startswith(('dist=', 'dists=', 'wean=', 'pilecap=', 'cap='))]
C = ['dist=25', 'spread=10', 'grants=3', 'window=4000', 'sel=off'] + extra + ['frames=200000', f'cap={cap}']
C = [f'{x},PP_BODYRULES_P=0.000,PP_HOMEEAT_P=0.000' if x.startswith('setenv=') else x for x in C]
def go(c):
    d = f'{out}/c{c}'; os.makedirs(d, exist_ok=True)
    with open(f'{d}.log', 'w') as lf:
        subprocess.run([f'{R}/target/release/examples/inrun', f'founders={R}/scratch/inrun/genomes/hcs.raw.txt:30', f'seed={c*1000+99}'] + C + [f'out={d}'], stdout=lf, stderr=subprocess.STDOUT, cwd=R)
    return c
with ThreadPoolExecutor(4) as ex: list(ex.map(go, range(1, 9)))
print('DONE')
