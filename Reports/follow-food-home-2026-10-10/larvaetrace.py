#!/usr/bin/env python3
"""larvaetrace.py RUN [RUN...] [--from F] [--to F] -- every larva that starved or pupated, followed.

Reads `deeptrace foodlog=1` output. A larva's life ends in a `starved` or
`pupated` row of food.csv.gz (the engine's own events; an id is reused by
later brood, so a life is the rows of that id since its previous end). For
each ending in the window: where it lay (way band), the nearest store cell
and nearest loose food cell at the census before it (foodcells.csv), and
every meal it was given in that life by kind, with the frames since its last
meal and since a nest worker last fed it. Positive control: the count of
`starved` rows equals stats.csv's larvae_starved over the same window.
"""
import csv, gzip, sys, collections, statistics, math

args = [a for a in sys.argv[1:] if not a.startswith('--')]
opt = {sys.argv[i][2:]: sys.argv[i + 1] for i in range(len(sys.argv) - 1) if sys.argv[i].startswith('--')}
F0 = int(opt.get('from', 100000)); F1 = int(opt.get('to', 200000))
runs = [a for a in args if not a.isdigit()]


def band(z, d):
    d = int(d)
    if z != 'nest':
        return z
    return 'off way' if d < 0 else 'door 0-9' if d < 10 else 'way 10-19' if d < 20 else 'deep 20+'


for run in runs:
    food = collections.defaultdict(list)
    for r in csv.DictReader(open(f'{run}/foodcells.csv')):
        food[int(r['frame'])].append((int(r['x']), int(r['y']), r['store'] == '1'))
    life = collections.defaultdict(list)
    ends = []
    for r in csv.DictReader(gzip.open(f'{run}/food.csv.gz', 'rt')):
        f = int(r['frame'])
        if f >= F1:
            break
        k = r['kind']
        if k.startswith('feed_'):
            life[r['who']].append((f, k[5:], float(r['worth']), r['worker'] == '1'))
        elif k in ('starved', 'pupated'):
            meals = life.pop(r['who'], [])
            if f >= F0:
                ends.append((k, f, int(r['x']), int(r['y']), band(r['zone'], r['depth']), meals))
    st = list(csv.DictReader(open(f'{run}/stats.csv')))
    s0 = next(int(x['larvae_starved']) for x in reversed(st) if int(x['frame']) <= F0)
    s1 = next(int(x['larvae_starved']) for x in reversed(st) if int(x['frame']) <= F1 - 1)
    print(f'\n{run}  ({F0//1000}-{F1//1000}k; stats.csv larvae_starved {s1 - s0})')
    for name in ('starved', 'pupated'):
        g = [e for e in ends if e[0] == name]
        if not g:
            print(f'  {name}: 0'); continue
        bands = collections.Counter(e[4] for e in g)
        ds, df, nm, since, since_w, got = [], [], [], [], [], collections.Counter()
        for _, f, x, y, b, meals in g:
            cells = food.get(f // 1000 * 1000, [])
            s = [math.hypot(cx - x, cy - y) for cx, cy, isst in cells if isst]
            a = [math.hypot(cx - x, cy - y) for cx, cy, _ in cells]
            ds.append(min(s) if s else 999); df.append(min(a) if a else 999)
            nm.append(len(meals))
            since.append(f - max((m[0] for m in meals), default=f - 99999))
            since_w.append(f - max((m[0] for m in meals if m[3]), default=f - 99999))
            for m in meals:
                got[m[1]] += m[2] / len(g)
        print(f'  {name}: {len(g)}  at ' + ', '.join(f'{b} {c}' for b, c in bands.most_common()))
        print(f'    nearest store cell median {statistics.median(ds):.0f} cells (no store cell anywhere: {sum(d == 999 for d in ds)}); nearest loose food median {statistics.median(df):.0f}')
        print(f'    meals in its life median {statistics.median(nm):.0f}; J a larva by kind: ' + ', '.join(f'{k} {w:.0f}' for k, w in sorted(got.items())))
        print(f'    frames since its last meal median {statistics.median(since):.0f}; since a nest worker fed it {statistics.median(since_w):.0f}')
