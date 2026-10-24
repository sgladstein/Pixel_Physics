"""cg4.py OUTDIR CAP C0 C1 WHO: plantwean4's common garden (seed c*1000+99, sel=off, standard line) at pile cap CAP.
WHO=carry: the colony's round-31 carry; WHO=pool: the standard pool itself (round 0's founders, the control)."""
import sys, os, subprocess, csv
from concurrent.futures import ThreadPoolExecutor
R = '/home/user/Pixel_Physics'; S = '/tmp/claude-0/-home-user-Pixel-Physics/b300ed13-cc9c-5e26-8b14-2acd8b3d007f/scratchpad/g'
out, cap, c0, c1, who = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), sys.argv[5]
def carried(rdir):
    r = list(csv.DictReader(open(f'{rdir}/index.csv')))
    r.sort(key=lambda x: int(x['id'])); r.sort(key=lambda x: (-float(x['score']), int(x['id']))); r = r[:30]; n = len(r)
    return ','.join(f"{rdir}/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30 // n + (1 if i < 30 % n else 0)}" for i, x in enumerate(r))
def pool(path, od):  # colsel.expand_pool
    lines = [l for l in open(path).read().split('\n') if l.strip() and not l.startswith('#')]
    head = lines[0].split(); members = []
    for l in lines[1:]:
        if l.startswith('member '): members.append(dict(members[0]) if members else {}); continue
        i, w = l.split(); members[-1][int(i)] = w
    os.makedirs(od, exist_ok=True); o = []
    for k, m in enumerate(members):
        fp = f'{od}/p{k:02}.raw.txt'
        with open(fp, 'w') as f:
            f.write(f'genome {head[3]} {head[4]}\n'); f.writelines(f'{i} {w}\n' for i, w in sorted(m.items()) if float(w) != 0.0)
        o.append(f'{fp}@p{k:02}:1')
    return ','.join(o)
extra = open('/mnt/project-files/plant-food/plantwean4/jobs.txt').read().split()[8:]
extra = [x for x in extra if not x.startswith(('dist=', 'dists=', 'wean=', 'pilecap=', 'cap=', 'sel=', 'distramp='))]
C = ['dist=25', 'spread=10', 'grants=3', 'window=4000', 'sel=off'] + extra + ['frames=200000', f'cap={cap}']
F = pool(f'{R}/assets/minds/std_pool.txt', f'{S}/stdpool') if who == 'pool' else None
def go(c):
    d = f'{out}/c{c}'; os.makedirs(d, exist_ok=True)
    f = F or carried(f'{S}/pw4carry/c{c}')
    with open(f'{d}.log', 'w') as lf:
        subprocess.run([f'{R}/target/release/examples/inrun', f'founders={f}', f'seed={c*1000+99}'] + C + [f'out={d}'], stdout=lf, stderr=subprocess.STDOUT, cwd=R)
    for fn in os.listdir(d):
        if fn.startswith(('map_f', 'ants_f')): os.remove(f'{d}/{fn}')
with ThreadPoolExecutor(4) as ex: list(ex.map(go, range(c0, c1 + 1)))
print('DONE', flush=True)
