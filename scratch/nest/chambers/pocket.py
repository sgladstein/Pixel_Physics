"""Was each underground starver shut in? From the last nestmap before its death: a flood through open cells (ants and
brood count as open) from where it last stood -- does it reach the open air above ground, the door (within 3 columns
of the door at the datum), stored food? Size of the pocket.
usage: pocket.py RUNDIR"""
import csv, sys, collections, glob, os
from collections import deque
NEST = 128
d = sys.argv[1]
import os
surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
MF = '--mapf' in sys.argv
maps = sorted(glob.glob(f'{d}/map_f*.txt' if MF else f'{d}/nestmap_f*.txt'), key=lambda p: int(os.path.basename(p)[-11:-4]))
cache = {}
def load(p):
    if p in cache: return cache[p]
    if MF:
        t = [r for r in open(p).read().split('\n') if r]; h = t[0].split(' ')
        cache[p] = (int(h[0]), int(h[1]), [['.' if c in '.a' else 'f' if c == 'f' else '#' for c in r] for r in t[1:]])
        return cache[p]
    t = open(p).read().split('\n'); h = t[0].split(' '); x0, y0 = int(h[0]), int(h[1]); leg = h[2].split(',')
    occ = [r for r in open(p.replace('nestmap_', 'occupants_')).read().split('\n')[1:] if r]
    g = [r for r in t[1:] if r]
    def cls(i, j):
        c = g[i][j]
        if 'A' <= c <= 'Z':
            n = leg[ord(c) - 65] if ord(c) - 65 < len(leg) else '?'
            return 'f' if n in ('crumbs', 'provisions') else '.' if n == 'empty' else '#'
        if c == '?': return '.' if occ[i][j] in 'aelp' else 'f'
        return '#'
    cache[p] = (x0, y0, [[cls(i, j) for j in range(len(g[i]))] for i in range(len(g))])
    return cache[p]
last = {}
for r in csv.DictReader(open(f'{d}/trace.csv')):
    last[int(r['id'])] = (int(r['frame']), int(r['x']), int(r['y']))
out = collections.Counter(); rows = []
for r in csv.reader(open(f'{d}/events.csv')):
    if not (r[0].isdigit() and r[2] == 'death' and int(float(r[5])) == 1): continue
    i, f = int(r[1]), int(r[0])
    if i not in last: continue
    _, x, y = last[i]
    if y <= surf[x]: continue
    prior = [p for p in maps if int(os.path.basename(p)[-11:-4]) <= f]
    if not prior: continue
    x0, y0, g = load(prior[-1])
    H, W = len(g), len(g[0])
    seen = {(x, y)}; q = deque([(x, y)]); sky = door = food = False
    while q:
        cx, cy = q.popleft()
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                nx, ny = cx + dx, cy + dy
                if (nx, ny) in seen: continue
                i2, j2 = ny - y0, nx - x0
                if not (0 <= i2 < H and 0 <= j2 < W): continue
                c = g[i2][j2]
                if c == 'f': food = True; continue
                if c != '.': continue
                seen.add((nx, ny))
                if ny <= surf[nx] - 1:
                    sky = True
                    if abs(nx - NEST) <= 3: door = True
                    continue  # stop at the air: a way out exists
                q.append((nx, ny))
    k = ('way out to the air' if sky else 'SHUT IN') + (', door reachable' if door else '') + (', food in pocket' if food else ', no food reachable')
    out[k] += 1
    rows.append((f // 1000, x - NEST, y - surf[x], len(seen), k))
print(f'{d}: underground starvers {len(rows)}')
for k, v in out.most_common(): print(f'  {v:4d}  {k}')
shut = [r for r in rows if r[4].startswith('SHUT')]
if shut: print('  shut-in: (death k-frame, dx, depth, pocket cells)', sorted(shut)[:400])
