#!/usr/bin/env python3
"""trapped.py RUN F0 F1 -- who was shut in when the door shut at F0, and what became of each by F1.
The shut-in space is doorseal's flood from the door anchor on map F0. Ants in it are read from colony.csv at F0.
For each: zone every 1000 frames to F1 (colony.csv), its death (events), and from walk.csv the share of decisions
by drive and job, whether it ever escaped (drive escape) and cut, and its energy at F0.
Caveat: the flood stops at row 175 (doorseal's limit, 15 rows below the ground), so the "shut-in space" and the ants counted in it
are the tube and the nest's top 15 rows only; ants deeper in the same shut system are not counted."""
import sys, csv, gzip, re, collections
sys.path.insert(0, '/mnt/project-files/deep-trace/tools')
import doorseal as ds
run, f0, f1 = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
at = ds.load(f'{run}/map_f{f0:06d}.txt')
seen = {(ds.DX, ds.DY)}
q = collections.deque([(ds.DX, ds.DY)])
while q:
    x, y = q.popleft()
    for dx in (-1, 0, 1):
        for dy in (-1, 0, 1):
            n = (x + dx, y + dy)
            if n in seen or at(*n) not in ds.PASS or n[1] > 175:
                continue
            seen.add(n); q.append(n)
print(f'shut-in space at {f0}: {len(seen)} cells, rows {min(y for _,y in seen)-160}..{max(y for _,y in seen)-160}, cols {min(x for x,_ in seen)-256}..{max(x for x,_ in seen)-256}')
pos = {}
zone = collections.defaultdict(dict)
energy0 = {}
spoil0 = {}
with open(f'{run}/colony.csv') as fh:
    r = csv.reader(fh); h = next(r); ix = {k: j for j, k in enumerate(h)}
    for row in r:
        f = int(row[0])
        if f < f0: continue
        if f > f1: break
        i = row[1]
        if f == f0:
            pos[i] = (int(row[ix['hx']]), int(row[ix['hy']]))
            energy0[i] = float(row[ix['energy_j']]); spoil0[i] = row[ix['spoil']]
        zone[i][f] = row[ix['zone']]
inside = {i for i, p in pos.items() if p in seen}
print(f'ants alive at {f0}: {len(pos)}; in the shut-in space: {len(inside)}')
death = {}
for l in open(f'{run}/events.txt'):
    if ' DIED ' in l:
        f = int(l.split()[0]); i = re.search(r' id=(\d+)', l).group(1)
        if f0 <= f <= f1 + 3000:
            death[i] = (f, re.search(r'cause=(\w+)', l).group(1), re.search(r'zone=(\w+)', l).group(1))
OUT = {'surface', 'mound_top', 'food', 'heap'}
fate = collections.Counter()
rows = collections.defaultdict(list)
with gzip.open(f'{run}/walk.csv.gz', 'rt') as fh:
    r = csv.reader(fh); h = next(r); ix = {k: j for j, k in enumerate(h)}
    for row in r:
        f = int(row[0])
        if f < f0: continue
        if f > f1: break
        if row[1] in inside:
            rows[row[1]].append((f, row[ix['drive']], row[ix['job']], float(row[ix['energy']]), row[ix['cut']], int(row[ix['ax']]), int(row[ix['ay']])))
detail = []
for i in sorted(inside, key=lambda i: death.get(i, (10**9,))[0]):
    zs = zone[i]
    left = next((f for f in sorted(zs) if zs[f] in OUT), None)
    d = death.get(i)
    rs = rows[i]
    dr = collections.Counter(x[1] for x in rs); jb = collections.Counter(x[2] for x in rs)
    cuts = collections.Counter(x[4] for x in rs if x[4] != '0')
    esc = dr.get('escape', 0)
    if left is not None:
        k = 'got out' + (f' then died {d[1]}' if d else '')
    elif d:
        k = f'died inside {d[1]}'
    else:
        k = 'still inside'
    fate[k] += 1
    detail.append((i, k, left, d, energy0[i], spoil0[i], dict(dr.most_common(4)), dict(jb.most_common(3)), dict(cuts), esc))
print('fates:', dict(fate))
for x in detail:
    print(' ', x)
