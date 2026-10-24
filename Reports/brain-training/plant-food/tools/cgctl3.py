"""cgctl3.py OUTDIR CAP C0 C1: the round-0 founder (hcs x30, no selection) in plantwean3's common garden (seed c*1000+99, rules off, sel=off) at pile cap CAP, colonies C0..C1."""
import sys, os, subprocess
from concurrent.futures import ThreadPoolExecutor
R = '/home/user/Pixel_Physics'
out, cap, c0, c1 = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
extra = open('/mnt/project-files/plant-food/plantwean3/jobs.txt').read().split()[8:]
extra = [x for x in extra if not x.startswith(('dist=', 'dists=', 'wean=', 'pilecap=', 'cap=', 'sel='))]
C = ['dist=25', 'spread=10', 'grants=3', 'window=4000', 'sel=off'] + extra + ['frames=200000', f'cap={cap}']
C = [f'{x},PP_BODYRULES_P=0.000,PP_HOMEEAT_P=0.000' if x.startswith('setenv=') else x for x in C]
print(' '.join(C), flush=True)
def go(c):
    d = f'{out}/c{c}'; os.makedirs(d, exist_ok=True)
    with open(f'{d}.log', 'w') as lf:
        subprocess.run([f'{R}/target/release/examples/inrun', f'founders={R}/scratch/inrun/genomes/hcs.raw.txt:30', f'seed={c*1000+99}'] + C + [f'out={d}'], stdout=lf, stderr=subprocess.STDOUT, cwd=R)
    for fn in os.listdir(d):
        if fn.startswith(('map_f', 'ants_f')): os.remove(f'{d}/{fn}')
with ThreadPoolExecutor(4) as ex: list(ex.map(go, range(c0, c1 + 1)))
print('DONE', flush=True)
