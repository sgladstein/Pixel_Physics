"""SCRATCH (old-ant thread, 2026-10-25): the standard pool on the training bed's common garden (inrun, sel=off, as
colsel.py's garden runs it) at several link prices (PIXEL_PHYSICS_LINK_PRICE), paired by seed. Prints colsel's own
numbers per run: nest, crater, rooms, starved, alive, home food, cuts.
usage: bedcheck.py OUT FRAMES "PRICES" "SEEDS" [P]"""
import os, sys, subprocess, csv
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0, 'scratch/inrun')
import nestcount, roomshape
OUT, F, PRICES, SEEDS = sys.argv[1], int(sys.argv[2]), sys.argv[3].split(), sys.argv[4].split()
P = int(sys.argv[5]) if len(sys.argv) > 5 else 4
BIN = 'target/release/examples/inrun'
os.makedirs(f'{OUT}/pool', exist_ok=True)
lines = [l for l in open('assets/minds/std_pool.txt').read().split('\n') if l.strip() and not l.startswith('#')]
head = lines[0].split(); members = []
for l in lines[1:]:
    if l.startswith('member '):
        members.append(dict(members[0]) if members else {}); continue
    i, w = l.split(); members[-1][int(i)] = w
fs = []
for k, m in enumerate(members):
    fp = f'{OUT}/pool/p{k:02}.raw.txt'
    with open(fp, 'w') as f:
        f.write(f'genome {head[3]} {head[4]}\n'); f.writelines(f'{i} {w}\n' for i, w in sorted(m.items()) if float(w) != 0.0)
    fs.append(f'{fp}@p{k:02}:1')
def one(job):
    price, seed = job
    d = f'{OUT}/p{price}_s{seed}'
    if not os.path.exists(f'{d}/done'):
        os.makedirs(d, exist_ok=True)
        with open(f'{d}.log', 'w') as lf:
            subprocess.run([BIN, f"founders={','.join(fs)}", f'seed={seed}', f'frames={F}', 'dist=25', 'spread=10', 'grants=3',
                            'window=4000', 'sel=off', f'out={d}'], stdout=lf, stderr=subprocess.STDOUT,
                           env={**os.environ, 'PIXEL_PHYSICS_LINK_PRICE': price})
        open(f'{d}/done', 'w').close()
    home = starved = cuts = 0
    for line in open(f'{d}/events.csv').readlines()[1:]:
        p = line.split(',')
        if p[2] == 'deliver' and abs(int(p[3]) - 128) <= 8: home += 1
        elif p[2] == 'dig' and abs(int(p[3]) - 128) <= 20: cuts += 1
        elif p[2] == 'death' and p[5] == '1': starved += 1
    rows = list(csv.DictReader(open(f'{d}/nest.csv'))) if os.path.exists(f'{d}/nest.csv') else []
    n = nestcount.count(d, F) if os.path.exists(f'{d}/nestmap.txt') else {}
    rooms = int(rows[-1]['chambers']) + int(rows[-1]['niches']) if rows else 0
    st = list(csv.DictReader(open(f'{d}/stock.csv'))) if os.path.exists(f'{d}/stock.csv') else []
    alive = int(st[-1]['alive']) if st else 0
    return price, seed, n.get('nest', 0), n.get('crater', 0), rooms, starved, alive, home, cuts, round(roomshape.score(d), 3) if n else 0
jobs = [(p, s) for s in SEEDS for p in PRICES]
with ThreadPoolExecutor(P) as ex:
    res = list(ex.map(one, jobs))
print('price seed nest crater rooms starved alive home cuts shape')
for r in sorted(res):
    print(*r)
