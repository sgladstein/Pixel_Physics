"""Who sealed each pocket an underground starver died in? For each shut-in starver (4k maps): the last map in which a
flood from its death cell reached the air (OPEN) and the next one (SHUT). The cells open in OPEN's flood but closed in
SHUT are the seal candidates; each is matched to an `in` event (a soil pellet let go below the datum) at that cell (or
one beside it) between the two maps -- an ant's drop -- or not (soil that fell or slid in). One row per distinct seal.
usage: sealwho.py RUNDIR"""
import csv, sys, collections, glob, os
from collections import deque
NEST = 128
d = sys.argv[1]
surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
maps = sorted(glob.glob(f'{d}/map_f*.txt'), key=lambda p: int(os.path.basename(p)[-11:-4]))
fr = lambda p: int(os.path.basename(p)[-11:-4])
cache = {}
def load(p):
    if p not in cache:
        t = [r for r in open(p).read().split('\n') if r]; h = t[0].split(' ')
        cache[p] = (int(h[0]), int(h[1]), [[c in '.a' for c in r] for r in t[1:]])
    return cache[p]
def flood(p, x, y):
    x0, y0, g = load(p); H, W = len(g), len(g[0])
    if not (0 <= y - y0 < H and 0 <= x - x0 < W) or not g[y - y0][x - x0]: return set(), False
    seen = {(x, y)}; q = deque([(x, y)]); air = False
    while q:
        cx, cy = q.popleft()
        if cy <= surf[cx] - 1: air = True; continue
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                nx, ny = cx + dx, cy + dy
                if (nx, ny) in seen or not (0 <= ny - y0 < H and 0 <= nx - x0 < W) or not g[ny - y0][nx - x0]: continue
                seen.add((nx, ny)); q.append((nx, ny))
    return seen, air
last = {}
for r in csv.DictReader(open(f'{d}/trace.csv')): last[int(r['id'])] = (int(r['x']), int(r['y']))
ins = []
for r in csv.reader(open(f'{d}/events.csv')):
    if r[0].isdigit() and r[2] == 'in': ins.append((int(r[0]), int(r[3]), int(r[4]), int(r[1])))
seals = {}
out = collections.Counter()
for r in csv.reader(open(f'{d}/events.csv')):
    if not (r[0].isdigit() and r[2] == 'death' and int(float(r[5])) == 1): continue
    i, f = int(r[1]), int(r[0])
    if i not in last or last[i][1] <= surf[last[i][0]]: continue
    x, y = last[i]
    prior = [p for p in maps if fr(p) <= f]
    if not prior: continue
    cells, air = flood(prior[-1], x, y)
    if air or not cells: continue
    # walk back to the last map where the death cell's region reached the air
    k = len(prior) - 1
    while k > 0:
        c2, a2 = flood(prior[k - 1], x, y)
        if a2: break
        k -= 1
    if k == 0: out['shut in from the first map (or cell not open earlier)'] += 1; continue
    po, ps = prior[k - 1], prior[k]
    key = (fr(po), fr(ps))
    if key not in seals:
        reg, _ = flood(po, x, y)
        x0, y0, g = load(ps)
        closed = [(cx, cy) for cx, cy in reg if cy > surf[cx] - 1 and not g[cy - y0][cx - x0]]
        f0, f1 = fr(po), fr(ps)
        drops = [e for e in ins if f0 < e[0] <= f1]
        by_drop = sum(any(abs(e[1] - cx) <= 1 and abs(e[2] - cy) <= 1 for e in drops) for cx, cy in closed)
        droppers = {e[3] for e in drops if any(abs(e[1] - cx) <= 1 and abs(e[2] - cy) <= 1 for cx, cy in closed)}
        seals[key] = dict(closed=len(closed), by_drop=by_drop, droppers=len(droppers), deaths=0, dx=sorted({cx - NEST for cx, _ in closed})[:6], depth=sorted({cy - surf[cx] for cx, cy in closed})[:6])
    seals[key]['deaths'] += 1
    out['shut in, seal found'] += 1
print(f'{d}: {dict(out)}')
for k, s in sorted(seals.items()):
    print(f'  sealed between f{k[0]} and f{k[1]}: deaths {s["deaths"]:3d}  cells closed {s["closed"]:4d}  of them beside a pellet drop {s["by_drop"]:4d} ({s["droppers"]} ants dropped)  dx {s["dx"]} depth {s["depth"]}')
