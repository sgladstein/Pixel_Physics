"""SCRATCH (teacher lane, 2026-10-27): the held-out exam -- worlds and lab boxes that colony selection never sees.

Owner, 2026-10-27: "Anything else that we should be exploring to make sure we are training properly?" Colony selection
scores colonies on training worlds (seed c*1000+r), a common garden (c*1000+99) and three lab boxes in
scratch/inrun/labscen (lab_far_mix, lab_far200_fruit, lab_herbs). So a good score on any of those is not a fair test,
and the 16-world harness this lane used until now (seeds s*1000+7) is colony s's round-7 training world. The exam is:

  garden: 16 worlds at seeds 500001..500016 (outside c*1000+r for any c <= 400), the job's garden settings, 200k frames;
  lab:    the game's own boxes (assets/lab_scenarios), as the lab ships them (no founding grant), 2 seeds each,
          after=60000 (four times the selection case's 15,000).

usage: exam.py OUT NAME FOUNDERS [ENV]
  FOUNDERS  a pool file (*pool.txt, every member once), a round folder with final/index.csv (its carry: top 30 by
            ant score, as colsel's carried()), or a founders string as inrun takes it.
  ENV       extra env for both, e.g. PP_HOMELOW=0.006,PP_HOMELOW_STORE=1 (the instinct arm).
Writes OUT/NAME/garden.csv and OUT/NAME/lab.csv. Compare two names with exam_cmp.py (paired sign tests).
env P (parallel runs, default 4), WORLDS (default 16), LAB_SEEDS (default 2), BOXES (comma list), AFTER (lab frames
after landing, default 60000; G1 judges at 600000, plan 2026-10-27).
"""
import csv, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import nestcount
import roomshape

OUT, NAME, FOUNDERS = sys.argv[1], sys.argv[2], sys.argv[3]
ENV = dict(x.split('=', 1) for x in sys.argv[4].split(',')) if len(sys.argv) > 4 and sys.argv[4] else {}
P = int(os.environ.get('P', '4'))
WORLDS = int(os.environ.get('WORLDS', '16'))
LAB_SEEDS = int(os.environ.get('LAB_SEEDS', '2'))
AFTER = int(os.environ.get('AFTER', '60000'))
BOXES = os.environ.get('BOXES', 'played_bed,far_larder,scattered_larder,two_larders,the_bank,herb_first_colony,'
                       'crowded_bench,feast_and_famine').split(',')
BIN, LAB_BIN = 'target/release/examples/inrun', 'target/release/examples/labsurvive'
# the garden settings of every colsel job since nest11 (scratch/inrun/jobs/elite1.txt), selection off
GARDEN = ['frames=200000', 'dist=25', 'spread=10', 'grants=3', 'window=4000', 'sel=off', 'food=mix',
          'lesson=scratch/inrun/genomes/c1.raw.txt:0.1', 'patches=20000:40:120']
SETENV = 'PP_BED_W=448,PP_MIND_DOOR_OPEN=drop'
D = f'{OUT}/{NAME}'
os.makedirs(D, exist_ok=True)


def founders():
    if FOUNDERS.endswith('pool.txt'):
        lines = [l for l in open(FOUNDERS).read().split('\n') if l.strip() and not l.startswith('#')]
        head = lines[0].split()
        members = []
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
    if os.path.exists(f'{FOUNDERS}/final/index.csv'):
        r = list(csv.DictReader(open(f'{FOUNDERS}/final/index.csv')))
        r.sort(key=lambda x: int(x['id']))
        r.sort(key=lambda x: (-float(x['score']), int(x['id'])))
        r = r[:30]
        n = len(r)
        return ','.join(f"{FOUNDERS}/final/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30 // n + (1 if i < 30 % n else 0)}"
                        for i, x in enumerate(r))
    return FOUNDERS


F = founders()
open(f'{D}/founders.txt', 'w').write(F + '\n')
setenv = SETENV + ''.join(f',{k}={v}' for k, v in ENV.items())


def garden(s):
    o = f'{D}/w{s}'
    with open(f'{o}.log', 'w') as lf:
        subprocess.run([BIN, f'founders={F}', f'seed={s}'] + GARDEN + [f'setenv={setenv}', f'out={o}'], stdout=lf,
                       stderr=subprocess.STDOUT, env={**os.environ, 'RAYON_NUM_THREADS': '1'})
    for fn in os.listdir(o):
        if fn.startswith('ants_f') or (fn.startswith('map_f') and not fn.startswith('map_f0200000')):
            os.remove(f'{o}/{fn}')
    r = list(csv.DictReader(open(f'{o}/nest.csv')))[-1]
    n = nestcount.count(o, 200000)
    starved = deliv = 0
    for l in open(f'{o}/events.csv'):
        if ',death,' in l and l.split(',')[5] == '1':
            starved += 1
        elif ',deliver,' in l:
            deliv += 1
    roomshape.OPEN_ONLY = True
    row = dict(world=s, alive=int(r['alive']), starved=starved, deliveries=deliv, nest=n.get('nest'),
               nest_open=n.get('nest_open'), crater=n.get('crater'), rooms=roomshape.score(o))
    subprocess.run(['gzip', '-f', f'{o}/events.csv'])
    return row


def lab(job):
    box, sd = job
    out = subprocess.run([LAB_BIN, f'founders={F}', f'scenario={box}', f'seed={sd}', f'after={AFTER}'], capture_output=True,
                         text=True, env={**os.environ, **ENV, 'RAYON_NUM_THREADS': '1',
                                         'PIXEL_PHYSICS_LAB_SCENARIOS': 'assets/lab_scenarios'}).stdout
    line = next((l for l in out.splitlines() if l.startswith('RESULT')), '')
    kv = dict(x.split('=', 1) for x in line.split()[1:])
    return dict(box=box, seed=sd, **kv)


def write(path, rows):
    if not rows:
        return
    keys = list(dict.fromkeys(k for r in rows for k in r))
    with open(path, 'w') as f:
        w = csv.DictWriter(f, keys)
        w.writeheader()
        w.writerows(rows)


with ThreadPoolExecutor(P) as ex:
    labrows = list(ex.map(lab, [(b, sd) for b in BOXES for sd in range(1, LAB_SEEDS + 1)]))
    write(f'{D}/lab.csv', labrows)
    gardenrows = list(ex.map(garden, range(500001, 500001 + WORLDS)))
    write(f'{D}/garden.csv', gardenrows)
print(f'exam {NAME}: {len(gardenrows)} worlds, {len(labrows)} lab boxes -> {D}', flush=True)
