"""SCRATCH (nest lane, 2026-10-27): the trained pool's nest beside the hand-built ant's, at 200k and 600k frames, in the
exam's held-out garden worlds and the game's own lab boxes. The nest lane's first page and the bar it sets.

Owner (2026-10-25), in order: the nest is too small for the colony; a crater instead of a mouth; one room. Measures,
all from nestcount.py / roomshape.py's own rules (the colsel cases' definitions):
  nest_open  roofed nest cells that are open air        crater  open cells near the door with no roof
  crumbs     nest cells holding crumbs (part-eaten fruit, put down)   stored  nest cells holding any food
  rooms      roomshape OPEN_ONLY (colsel `shapeopen=1`)  alive, starved (by that frame)

usage: baseline.py OUT ARM [garden|lab|both]
  ARM  std (assets/minds/std_pool.txt, every member once, as exam.py) or old (the hand-built ant: inrun `nomind=1`,
       labsurvive `founders=old`).
env P (parallel, default 4), WORLDS (16), FRAMES (600000), AT (200000,600000), LAB_SEEDS (2), BOXES.
Writes OUT/ARM/garden.csv, OUT/ARM/lab.csv (one row per world/box and checkpoint) and keeps every nestmap_f/occupants_f
for the pictures (pictures.py).
"""
import csv, os, shutil, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'inrun'))
import nestcount
import roomshape

MAIN = __name__ == '__main__'  # imported (timeline.py), only the readers below are used
OUT, ARM = (sys.argv[1], sys.argv[2]) if MAIN else ('', '')
WHAT = sys.argv[3] if MAIN and len(sys.argv) > 3 else 'both'
P = int(os.environ.get('P', '4'))
WORLDS = int(os.environ.get('WORLDS', '16'))
FRAMES = int(os.environ.get('FRAMES', '600000'))
AT = [int(x) for x in os.environ.get('AT', '200000,600000').split(',')]
LAB_SEEDS = int(os.environ.get('LAB_SEEDS', '2'))
# the exam's 8 boxes less far_larder and the_bank, where the hand-built ant dies out too (exam old-lab.csv)
BOXES = os.environ.get('BOXES', 'played_bed,scattered_larder,two_larders,herb_first_colony,crowded_bench,'
                       'feast_and_famine').split(',')
BIN, LAB_BIN = 'target/release/examples/inrun', 'target/release/examples/labsurvive'
GARDEN = ['dist=25', 'spread=10', 'grants=3', 'window=4000', 'sel=off', 'food=mix',
          'lesson=scratch/inrun/genomes/c1.raw.txt:0.1', 'patches=20000:40:120']  # exam.py's GARDEN, frames apart
SETENV = 'PP_BED_W=448,PP_MIND_DOOR_OPEN=drop'
D = f'{OUT}/{ARM}'


def pool_founders():
    lines = [l for l in open('assets/minds/std_pool.txt').read().split('\n') if l.strip() and not l.startswith('#')]
    head, members = lines[0].split(), []
    for l in lines[1:]:
        if l.startswith('member '):
            members.append(dict(members[0]) if members else {})
            continue
        i, w = l.split()
        members[-1][int(i)] = w
    os.makedirs(f'{D}/pool', exist_ok=True)
    out = []
    for k, m in enumerate(members):
        fp = f'{D}/pool/p{k:02}.raw.txt'
        with open(fp, 'w') as f:
            f.write(f'genome {head[3]} {head[4]}\n')
            f.writelines(f'{i} {w}\n' for i, w in sorted(m.items()) if float(w) != 0.0)
        out.append(f'{fp}@p{k:02}:1')
    return ','.join(out)


if MAIN:
    os.makedirs(D, exist_ok=True)
    F = pool_founders()  # the hand-built arm takes the same founder count under nomind=1


def kinds(d, nest):
    """nest cells by what fills them: crumbs, other food (fruit, stored provisions), ants and brood."""
    t = open(f'{d}/nestmap.txt').read().split('\n')
    legend = t[0].split(' ')[2].split(',')
    grid = [r for r in t[1:] if r]
    occ = [r for r in open(f'{d}/occupants.txt').read().split('\n')[1:] if r] if os.path.exists(f'{d}/occupants.txt') else None
    ch = {m: chr(65 + k) for k, m in enumerate(legend)}
    c = dict(crumbs=0, stored=0, ants=0, brood=0)
    for dx, dy in nest:
        g = grid[dy][dx]
        if g == ch.get('crumbs'):
            c['crumbs'] += 1
            c['stored'] += 1
        elif g in (ch.get('fruit'), ch.get('food')):
            c['stored'] += 1
        elif g == '?' and occ:
            o = occ[dy][dx]
            if o == 'f':
                c['stored'] += 1
            elif o in 'elp':
                c['brood'] += 1
            elif o == 'a':
                c['ants'] += 1
    return c


def nest_cells(d):
    roomshape.OPEN_ONLY = False
    space, _, _, crater = roomshape.nest_cells(d)
    return space - crater


def measure(src, mapf, occf, extra):
    """one checkpoint: copy its map into a temp run dir under the names the readers expect."""
    with tempfile.TemporaryDirectory() as t:
        shutil.copy(mapf, f'{t}/nestmap.txt')
        shutil.copy(f'{src}/surface.txt', f'{t}/surface.txt')
        if os.path.exists(occf):
            shutil.copy(occf, f'{t}/occupants.txt')
        n = nestcount.count(t)
        roomshape.OPEN_ONLY = True
        rooms = roomshape.score(t)
        k = kinds(t, nest_cells(t))
    return dict(**extra, nest=n['nest'], nest_open=n['nest_open'], crater=n['crater'], crumbs=k['crumbs'],
                stored=k['stored'], ants_in=k['ants'], brood_in=k['brood'], rooms=rooms)


def garden(s):
    o = f'{D}/w{s}'
    if not os.path.exists(f'{o}/done'):
        with open(f'{o}.log', 'w') as lf:
            subprocess.run([BIN, f'founders={F}', f'seed={s}', f'frames={FRAMES}'] + GARDEN + [f'setenv={SETENV}', f'out={o}']
                           + (['nomind=1'] if ARM == 'old' else []), stdout=lf, stderr=subprocess.STDOUT,
                           env={**os.environ, 'RAYON_NUM_THREADS': '1'})
        for fn in os.listdir(o):
            if fn.startswith('ants_f') or fn.startswith('map_f'):
                os.remove(f'{o}/{fn}')
        open(f'{o}/done', 'w').close()
    nest = list(csv.DictReader(open(f'{o}/nest.csv')))
    deaths = []
    with open(f'{o}/events.csv') as ev:
        for l in ev:
            p = l.split(',')
            if len(p) > 5 and p[2] == 'death' and p[5].strip() == '1':
                deaths.append(int(p[0]))
    rows = []
    for at in AT:
        r = [x for x in nest if int(x['frame']) <= at][-1]
        rows.append(measure(o, f'{o}/nestmap_f{at:07}.txt', f'{o}/occupants_f{at:07}.txt',
                            dict(world=s, frame=at, alive=int(r['alive']), starved=sum(f <= at for f in deaths))))
    return rows


def lab(job):
    box, sd = job
    o = f'{D}/lab_{box}_{sd}'
    if not os.path.exists(f'{o}/done'):
        os.makedirs(o, exist_ok=True)
        every = ','.join(str(f) for f in range(25000, max(AT) + 1, 25000))
        out = subprocess.run([LAB_BIN, f'founders={"old" if ARM == "old" else "std"}', f'scenario={box}', f'seed={sd}',
                              f'after={max(AT)}', f'nestmap={o}', f'nestat={every}'], capture_output=True, text=True,
                             env={**os.environ, 'RAYON_NUM_THREADS': '1', 'PIXEL_PHYSICS_LAB_SCENARIOS': 'assets/lab_scenarios'}).stdout
        open(f'{o}/log.txt', 'w').write(out)
        open(f'{o}/done', 'w').close()
    out = open(f'{o}/log.txt').read()
    st = {int(kv['f']): kv for kv in (dict(x.split('=', 1) for x in l.split()[1:]) for l in out.splitlines() if l.startswith('NESTAT'))}
    rows = []
    for at in AT:
        kv = st.get(at)
        if kv is None:
            continue
        rows.append(measure(o, f'{o}/nestmap_f{at:07}.txt', f'{o}/occupants_f{at:07}.txt',
                            dict(box=box, seed=sd, frame=at, alive=int(kv['alive']), starved=int(kv['starved']))))
    return rows


def write(path, rows):
    if not rows:
        return
    keys = list(dict.fromkeys(k for r in rows for k in r))
    with open(path, 'w') as f:
        w = csv.DictWriter(f, keys)
        w.writeheader()
        w.writerows(rows)


if MAIN:
    with ThreadPoolExecutor(P) as ex:
        if WHAT in ('garden', 'both'):
            write(f'{D}/garden.csv', [r for rs in ex.map(garden, range(500001, 500001 + WORLDS)) for r in rs])
        if WHAT in ('lab', 'both'):
            write(f'{D}/lab.csv', [r for rs in ex.map(lab, [(b, sd) for b in BOXES for sd in range(1, LAB_SEEDS + 1)]) for r in rs])
    print(f'baseline {ARM} {WHAT} -> {D}', flush=True)
