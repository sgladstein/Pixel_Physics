"""cgfilm.py C: re-run pw3mini's common garden after round 7 for colony C (cap 20), and the founder control, with film."""
import sys, os, subprocess, csv
S = '/tmp/claude-0/-home-user-Pixel-Physics/b300ed13-cc9c-5e26-8b14-2acd8b3d007f/scratchpad/g'
R = '/home/user/Pixel_Physics'
c = int(sys.argv[1]); OUT = f'{S}/pw3mini'
def carried(rdir):
    r = list(csv.DictReader(open(f'{rdir}/final/index.csv')))
    r.sort(key=lambda x: int(x['id'])); r.sort(key=lambda x: (-float(x['score']), int(x['id']))); r = r[:30]; n = len(r)
    return ','.join(f"{rdir}/final/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30 // n + (1 if i < 30 % n else 0)}" for i, x in enumerate(r))
extra = open(f'{OUT}/args.txt').read().split()
extra = [x for x in extra if not x.startswith(('dist=', 'dists=', 'wean=', 'pilecap=', 'cap='))]
C = ['dist=25', 'spread=10', 'grants=3', 'window=4000', 'sel=off'] + extra + ['frames=200000', 'cap=20', 'film=10000', 'film_hw=128']
C = [f'{x},PP_BODYRULES_P=0.000,PP_HOMEEAT_P=0.000' if x.startswith('setenv=') else x for x in C]
jobs = {f'film_sel_c{c}': carried(f'{OUT}/c{c}/r7'), f'film_ctl_c{c}': f'{R}/scratch/inrun/genomes/hcs.raw.txt:30'}
ps = []
for name, f in jobs.items():
    d = f'{S}/{name}'; os.makedirs(d, exist_ok=True)
    ps.append(subprocess.Popen([f'{R}/target/release/examples/inrun', f'founders={f}', f'seed={c*1000+99}'] + C + [f'out={d}'], stdout=open(f'{d}.log', 'w'), stderr=subprocess.STDOUT, cwd=R))
for p in ps: p.wait()
print('DONE')
