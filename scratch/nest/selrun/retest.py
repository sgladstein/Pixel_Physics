"""SCRATCH (chambers selection run, 2026-10-29): does a nest-shape case repeat from world to world for H3 + eat colonies?

The Selection lane found room shape does not (Spearman ~0 between a kept colony's two rounds, PR #985; nest size
0.25-0.29), so a weight on it would spend its vote on luck. This asks the same of the crumb-proof measures
(scratch/inrun/nestshape.py) before the batch picks a weight: K distinct H3+eat carries (the eat-seed screen's
poolrounds.sh lines, 4 worlds x 4 rounds, top 30 by ant score as colsel's carried() takes them) each run in the SAME two
fresh worlds with the batch's training args; Spearman across carries of each measure, world 1 against world 2.
Positive control: the same with std_pool carries mixed in (two families whose nests differ: H3 tunnels, std_pool cone),
which must come out clearly positive or the instrument cannot see a heritable difference at all.
usage: retest.py OUT SEED1 SEED2 FRAMES NAME:LINEDIR[:ROUNDS] ...     (LINEDIR holds r0..r3 with final/)
env BIN (inrun), P (parallel runs, default 4)

Result, 2026-10-29 (worlds 1099/2099, 200k, main 678a4fb4; 16 H3+eat carries = the eat screen's 4 lines x rounds 0-3,
8 std_pool carries = its std lines' rounds 2-3), Spearman world 1 vs world 2:
                     galleryc  crumb_share  rooms_open  gallery  thin_old  space  alive
  H3+eat (n=16)        0.44       0.38        0.45      -0.15     -0.03    0.29   0.03
  std_pool (n=8)       0.02       0.12       -0.17      -0.34     -0.55    0.40   0.58
  mixed (n=24)         0.59       0.49        0.24      -0.20      0.64    0.30   0.08
Medians H3+eat / std_pool: galleryc 221 / 112, crumb share 0.28 / 0.52. Caveats: n=16 (0.44 is p ~0.09 alone), and the
carries come from 4 lines, so they are related; a batch's elite colonies at kpm 0.003 may differ less than these.
"""
import csv, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0, 'scratch/inrun')
import nestshape

OUT, S1, S2, F = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
BIN = os.environ.get('BIN', 'target/release/examples/inrun')
P = int(os.environ.get('P', '4'))
# the batch's training bed (k3's line, scratch/inrun/jobs/clean1.txt, colsel's own C list) with nest_r=0
ARGS = ['dist=25', 'spread=10', 'grants=3', 'window=4000', 'sel=score', 'lesson=scratch/inrun/genomes/c1.raw.txt:0.1',
        'food=mix', 'patches=20000:40:120', 'setenv=PP_BED_W=448,PP_MIND_DOOR_OPEN=drop', 'kpm=0.003', 'nest_r=0']


def carried(rdir):
    r = list(csv.DictReader(open(f'{rdir}/final/index.csv')))
    r.sort(key=lambda x: int(x['id']))
    r.sort(key=lambda x: (-float(x['score']), int(x['id'])))
    r = r[:30]
    n = len(r)
    return ','.join(f"{os.path.abspath(rdir)}/final/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30 // n + (1 if i < 30 % n else 0)}"
                    for i, x in enumerate(r)) if n else None


carries = []
for spec in sys.argv[5:]:
    name, line, *rr = spec.split(':')
    for r in (rr[0].split(',') if rr else range(4)):
        c = carried(f'{line}/r{r}') if os.path.exists(f'{line}/r{r}/final/index.csv') else None
        if c:
            carries.append((f'{name}_r{r}', name.split('_')[0], c))
os.makedirs(OUT, exist_ok=True)


def one(job):
    tag, fam, fs, seed = job
    d = f'{OUT}/{tag}_s{seed}'
    if not os.path.exists(f'{d}/nestmap.txt'):
        with open(f'{d}.log', 'w') as lf:
            subprocess.run([BIN, f'founders={fs}', f'seed={seed}', f'frames={F}'] + ARGS + [f'out={d}'], stdout=lf,
                           stderr=subprocess.STDOUT, env={**os.environ, 'RAYON_NUM_THREADS': '1'})
        for fn in os.listdir(d):
            if fn.startswith(('map_f', 'ants_f')):
                os.remove(f'{d}/{fn}')
    m = nestshape.measure(d) or {}
    try:
        m['alive'] = int(list(csv.DictReader(open(f'{d}/nest.csv')))[-1]['alive'])
    except (OSError, IndexError):
        m['alive'] = 0
    return (tag, seed), m


jobs = [(t, f, c, s) for t, f, c in carries for s in (S1, S2)]
with ThreadPoolExecutor(P) as ex:
    got = dict(ex.map(one, jobs))


def rank(v):
    o = sorted(range(len(v)), key=lambda i: v[i])
    rk = [0.0] * len(v)
    i = 0
    while i < len(o):
        j = i
        while j + 1 < len(o) and v[o[j + 1]] == v[o[i]]:
            j += 1
        for t in range(i, j + 1):
            rk[o[t]] = (i + j) / 2
        i = j + 1
    return rk


def spearman(a, b):
    ra, rb = rank(a), rank(b)
    n = len(a)
    ma, mb = sum(ra) / n, sum(rb) / n
    cov = sum((x - ma) * (y - mb) for x, y in zip(ra, rb))
    va, vb = sum((x - ma) ** 2 for x in ra), sum((y - mb) ** 2 for y in rb)
    return cov / (va * vb) ** 0.5 if va and vb else float('nan')


KEYS = ('galleryc', 'crumb_share', 'rooms_open', 'gallery', 'thin_old', 'space', 'alive')
with open(f'{OUT}/retest.csv', 'w') as f:
    f.write('carry,family,seed,' + ','.join(KEYS) + '\n')
    for t, fam, _ in carries:
        for s in (S1, S2):
            m = got[(t, s)]
            f.write(f'{t},{fam},{s},' + ','.join(str(m.get(k, 0)) for k in KEYS) + '\n')
fams = sorted({fam for _, fam, _ in carries})
print(f'{len(carries)} carries x worlds {S1},{S2}, {F} frames; Spearman world 1 vs world 2 across carries')
print('set'.ljust(22) + ''.join(f'{k:>12s}' for k in KEYS))
for label, sel in [(f'{fm} only (n={sum(1 for c in carries if c[1] == fm)})', [c for c in carries if c[1] == fm]) for fm in fams] + \
        [(f'all mixed (n={len(carries)})', carries)]:
    if len(sel) < 4:
        continue
    row = [spearman([got[(t, S1)].get(k, 0) for t, _, _ in sel], [got[(t, S2)].get(k, 0) for t, _, _ in sel]) for k in KEYS]
    print(label.ljust(22) + ''.join(f'{x:12.2f}' for x in row))
for fm in fams:
    sel = [c for c in carries if c[1] == fm]
    med = lambda k: sorted(got[(t, s)].get(k, 0) for t, _, _ in sel for s in (S1, S2))[len(sel)]
    print(f'median {fm}: ' + ' '.join(f'{k}={med(k)}' for k in KEYS))
