"""cgprobe.py OUTDIR CAP C0 C1: plantwean3's round-31 carry of colonies C0..C1 in its common garden (seed c*1000+99, rules off, sel=off) at pile cap CAP."""
import sys, os, subprocess, csv
from concurrent.futures import ThreadPoolExecutor
R = '/home/user/Pixel_Physics'; S = '/tmp/claude-0/-home-user-Pixel-Physics/b300ed13-cc9c-5e26-8b14-2acd8b3d007f/scratchpad/g'
out, cap, c0, c1 = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
def carried(rdir):  # colsel.carried(), reading the copied carry folder
    r = list(csv.DictReader(open(f'{rdir}/index.csv')))
    r.sort(key=lambda x: int(x['id'])); r.sort(key=lambda x: (-float(x['score']), int(x['id']))); r = r[:30]; n = len(r)
    return ','.join(f"{rdir}/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30 // n + (1 if i < 30 % n else 0)}" for i, x in enumerate(r))
extra = open('/mnt/project-files/plant-food/plantwean3/jobs.txt').read().split()[8:]
extra = [x for x in extra if not x.startswith(('dist=', 'dists=', 'wean=', 'pilecap=', 'cap=', 'sel='))]
C = ['dist=25', 'spread=10', 'grants=3', 'window=4000', 'sel=off'] + extra + ['frames=200000', f'cap={cap}']
C = [f'{x},PP_BODYRULES_P=0.000,PP_HOMEEAT_P=0.000' if x.startswith('setenv=') else x for x in C]
def go(c):
    d = f'{out}/c{c}'; os.makedirs(d, exist_ok=True)
    with open(f'{d}.log', 'w') as lf:
        subprocess.run([f'{R}/target/release/examples/inrun', f'founders={carried(f"{S}/pw3carry/c{c}")}', f'seed={c*1000+99}'] + C + [f'out={d}'], stdout=lf, stderr=subprocess.STDOUT, cwd=R)
    for fn in os.listdir(d):
        if fn.startswith(('map_f', 'ants_f')): os.remove(f'{d}/{fn}')
with ThreadPoolExecutor(4) as ex: list(ex.map(go, range(c0, c1 + 1)))
print('DONE', flush=True)
