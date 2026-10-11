#!/usr/bin/env python3
"""shutspace.py RUN F0 F1 [STEP] -- per map: door open/shut, the door system's size and row span, ants in it (colony.csv),
how many of those ants are dead of starvation by F1+3000, and how many of them were in it on the previous map.
Caveat: the flood stops at row 175 (doorseal's limit, 15 rows below the ground), so the "shut-in space" and the ants counted in it
are the tube and the nest's top 15 rows only; ants deeper in the same shut system are not counted."""
import sys, csv, re, collections
sys.path.insert(0, '/mnt/project-files/deep-trace/tools')
import doorseal as ds
run, f0, f1 = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
step = int(sys.argv[4]) if len(sys.argv) > 4 else 1000
pos = collections.defaultdict(dict)
with open(f'{run}/colony.csv') as fh:
    r = csv.reader(fh); h = next(r); ix = {k: j for j, k in enumerate(h)}
    for row in r:
        f = int(row[0])
        if f < f0: continue
        if f > f1: break
        pos[f][(int(row[ix['hx']]), int(row[ix['hy']]))] = pos[f].get((int(row[ix['hx']]), int(row[ix['hy']])), []) + [row[1]]
starved = {}
for l in open(f'{run}/events.txt'):
    if 'STARVED' in l:
        f = int(l.split()[0])
        if f0 <= f <= f1 + 3000:
            starved[re.search(r' id=(\d+)', l).group(1)] = f
prev = set()
for f in range(f0, f1 + 1, step):
    at = ds.load(f'{run}/map_f{f:06d}.txt')
    sky, _, _, n = ds.measure(f'{run}/map_f{f:06d}.txt')
    seen = {(ds.DX, ds.DY)}
    q = collections.deque([(ds.DX, ds.DY)])
    while q:
        x, y = q.popleft()
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                nn = (x + dx, y + dy)
                if nn in seen or at(*nn) not in ds.PASS or nn[1] > 175:
                    continue
                seen.add(nn); q.append(nn)
    ants = {i for p, ids in pos[f].items() if p in seen for i in ids}
    st = sum(1 for i in ants if i in starved)
    print(f'{f // 1000:4d}k {"open" if sky else "SHUT"} system {len(seen):4d} cells rows {min(y for _, y in seen) - 160:+d}..{max(y for _, y in seen) - 160:+d} '
          f'ants in it {len(ants):3d}, of them starve by end {st:3d}, also in it last map {len(ants & prev):3d}')
    prev = ants
