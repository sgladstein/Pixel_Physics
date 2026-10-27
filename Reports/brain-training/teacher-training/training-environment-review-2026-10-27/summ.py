"""Per-run metrics for the environment ladder. usage: summ.py S [ARM...]  -> S/summary.csv"""
import csv, gzip, os, sys, shutil, tempfile
sys.path.insert(0, '/home/user/Pixel_Physics/scratch/inrun')
import nestcount
S = sys.argv[1]

def events(d):
    p = f'{d}/events.csv'
    f = gzip.open(p + '.gz', 'rt') if not os.path.exists(p) else open(p)
    next(f)
    for line in f:
        yield line.rstrip('\n').split(',')

def nest_at(d, frame=None):
    """nestcount at the final map, or at nestmap_fNNNNNNN (events cut to that frame)."""
    if frame is None:
        return nestcount.count(d)
    t = tempfile.mkdtemp()
    shutil.copy(f'{d}/nestmap_f{frame:07d}.txt', f'{t}/nestmap.txt')
    shutil.copy(f'{d}/surface.txt', f'{t}/surface.txt')
    with open(f'{t}/events.csv', 'w') as o:
        o.write('frame,id,kind,x,y,v,cutter\n')
        for p in events(d):
            if int(p[0]) <= frame:
                o.write(','.join(p) + '\n')
    r = nestcount.count(t, frame)
    shutil.rmtree(t)
    return r

def run(d, blocks):
    stock = list(csv.DictReader(open(f'{d}/stock.csv')))
    alive = {int(r['frame']): int(r['alive']) for r in stock}
    out = []
    ev = list(events(d))
    for (a, b) in blocks:
        st = sum(1 for p in ev if p[2] == 'death' and p[5] == '1' and a < int(p[0]) <= b)
        age = sum(1 for p in ev if p[2] == 'death' and p[5] == '7' and a < int(p[0]) <= b)
        home = sum(1 for p in ev if p[2] == 'deliver' and abs(int(p[3]) - 128) <= 8 and a < int(p[0]) <= b)
        al = [v for f, v in alive.items() if a < f <= b]
        end = alive.get(b, al[-1] if al else 0)
        n = nest_at(d, None if b == max(x[1] for x in blocks) else b)
        out.append(dict(alive=end, peak=max(al) if al else 0, low=min(al) if al else 0, starved=st, oldage=age, home=home,
                        nest=n['nest'], nest_open=n['nest_open'], crater=n['crater'], spoil=n['spoil']))
    return out

rows = []
arms = sys.argv[2:] or sorted(os.listdir(f'{S}/runs'))
for arm in arms:
    for seed in sorted(os.listdir(f'{S}/runs/{arm}')):
        d = f'{S}/runs/{arm}/{seed}'
        if not os.path.exists(f'{d}/done'):
            continue
        blocks = [(0, 200000), (200000, 400000), (400000, 600000)] if arm.startswith('L') else [(0, 200000)]
        for k, r in enumerate(run(d, blocks)):
            rows.append(dict(arm=arm, seed=seed, block=k, secs=open(f'{d}/time.txt').read().strip(), **r))
with open(f'{S}/summary.csv', 'w') as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
    w.writeheader(); w.writerows(rows)
print(len(rows), 'rows')
