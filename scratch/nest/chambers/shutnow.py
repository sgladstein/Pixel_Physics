"""Standing state: at each 4k map, of the living ants underground, how many stand in a pocket with no open way to the air.
usage: shutnow.py RUNDIR..."""
import csv, sys, glob, os, collections
from collections import deque
for d in sys.argv[1:]:
    surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    pos = collections.defaultdict(list)
    for r in csv.DictReader(open(f'{d}/trace.csv')):
        if int(r['frame']) % 4000 == 0: pos[int(r['frame'])].append((int(r['x']), int(r['y'])))
    tot = shut = under = 0; series = []
    for p in sorted(glob.glob(f'{d}/map_f*.txt')):
        f = int(os.path.basename(p)[-11:-4])
        t = [r for r in open(p).read().split('\n') if r]; h = t[0].split(' '); x0, y0 = int(h[0]), int(h[1])
        g = [[c in '.a' for c in r] for r in t[1:]]; H, W = len(g), len(g[0])
        memo = {}
        def air(x, y):
            if (x, y) in memo: return memo[(x, y)]
            seen = {(x, y)}; q = deque([(x, y)]); ok = False
            while q and not ok:
                cx, cy = q.popleft()
                if cy <= surf[cx] - 1: ok = True; break
                for dx in (-1, 0, 1):
                    for dy in (-1, 0, 1):
                        nx, ny = cx + dx, cy + dy
                        if (nx, ny) in seen or not (0 <= ny - y0 < H and 0 <= nx - x0 < W) or not g[ny - y0][nx - x0]: continue
                        seen.add((nx, ny)); q.append((nx, ny))
            for c in seen: memo[c] = ok
            return ok
        n = s = u = 0
        for x, y in pos[f]:
            n += 1
            if y > surf[x] and 0 <= y - y0 < H and 0 <= x - x0 < W:
                u += 1; s += not air(x, y)
        tot += n; shut += s; under += u; series.append(s)
    print(f'{d:18s} ant-samples {tot:5d} underground {under:5d} shut in {shut:4d} ({100*shut/max(1,tot):4.1f}% of all ant-time) per map: {series}')
