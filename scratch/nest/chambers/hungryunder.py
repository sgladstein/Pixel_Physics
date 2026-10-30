"""For every ant that starved underground: its hungry-probe rows (tank < 0.25, every 250 frames) -- was food touching
its head, was a pick-up offered, jaws, last verb -- and the open-cell path distance from where it died to the nearest
stored food cell (provisions/crumbs) in the nestmap nearest its death.
usage: hungryunder.py RUNDIR"""
import csv, sys, collections, glob, os, statistics as st
from collections import deque
NEST = 128
d = sys.argv[1]
surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
def place(x, y):
    s = surf[x] if 0 <= x < len(surf) else 0
    dx = abs(x - NEST)
    if y > s: return 'bowl' if dx <= 12 else 'tun'
    if x < 12 or x > 436: return 'wall'
    return 'door' if dx <= 12 else 'fld'
last = {}
for r in csv.DictReader(open(f'{d}/trace.csv')):
    last[int(r['id'])] = (int(r['frame']), int(r['x']), int(r['y']))
dead = {}
for r in csv.reader(open(f'{d}/events.csv')):
    if r[0].isdigit() and r[2] == 'death' and int(float(r[5])) == 1:
        i = int(r[1])
        if i in last and place(*last[i][1:]) in ('bowl', 'tun', 'door'):
            dead[i] = int(r[0])
H = collections.defaultdict(list)
for r in csv.DictReader(open(f'{d}/hungry.csv')):
    i = int(r['id'])
    if i in dead: H[i].append(r)
maps = sorted(glob.glob(f'{d}/nestmap_f*.txt'))
def loadmap(p):
    t = open(p).read().split('\n'); h = t[0].split(' '); x0, y0 = int(h[0]), int(h[1]); leg = h[2].split(',')
    occ = [r for r in open(p.replace('nestmap_', 'occupants_')).read().split('\n')[1:] if r]
    g = [r for r in t[1:] if r]
    def cls(i, j):
        c = g[i][j]
        if 'A' <= c <= 'Z':
            n = leg[ord(c) - 65] if ord(c) - 65 < len(leg) else '?'
            return 'f' if n in ('crumbs', 'provisions') else '.' if n == 'empty' else '#'
        if c == '?':
            o = occ[i][j]
            return '.' if o in 'aelp' else 'f'
        return '#'
    return x0, y0, leg, [''.join(cls(i, j) for j in range(len(g[i]))) for i in range(len(g))]
cache = {}
def food_dist(f, x, y):
    if not maps: return None
    p = min(maps, key=lambda p: abs(int(os.path.basename(p)[9:16]) - f))
    if p not in cache: cache[p] = loadmap(p)
    x0, y0, leg, g = cache[p]
    H_, W_ = len(g), len(g[0])
    def ch(xx, yy):
        i, j = yy - y0, xx - x0
        return g[i][j] if 0 <= i < H_ and 0 <= j < W_ else None
    seen = {(x, y)}; q = deque([(x, y, 0)])
    while q:
        cx, cy, n = q.popleft()
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                nx, ny = cx + dx, cy + dy
                if (nx, ny) in seen: continue
                c = ch(nx, ny)
                if c is None: continue
                seen.add((nx, ny))
                if c == 'f': return n + 1
                if c == '.': q.append((nx, ny, n + 1))
    return 999
print(maps[:1] and open(maps[0]).readline().strip())
rows = collections.Counter(); fd = []
for i, f in dead.items():
    hs = H[i]
    for r in hs:
        rows['rows'] += 1
        rows['food touching'] += (int(r['crumbs8']) + int(r['prov8'])) > 0
        rows['pick offered'] += int(r['picks']) > 0
        rows['jaws ' + r['jaws']] += 1
        rows['verb ' + r['verb']] += 1
    fd.append(food_dist(f, *last[i][1:]))
print(f'{d}: underground starvers {len(dead)}; their hungry rows', dict(rows))
v = sorted(x for x in fd if x is not None)
if v: print('  path (open cells) to nearest stored food at death, nearest nestmap: med', st.median(v), 'p10', v[len(v)//10], 'p90', v[9*len(v)//10], ' unreachable(999)', sum(x == 999 for x in v))
