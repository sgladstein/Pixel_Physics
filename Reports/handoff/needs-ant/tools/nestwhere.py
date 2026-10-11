"""Where is today's 'time in the dug nest' (zone nest = head below the old ground line)?
Joins colony.csv samples with the map at the same frame. room = open cell with >=7 of its 3x3 open (scorecard's rule);
sealed = not 8-connected through open cells to the map's top row; depth = rows below ground_y."""
import csv, sys, os, collections
B = sys.argv[1]; seeds = [int(s) for s in sys.argv[2].split(',')]; a, b = int(sys.argv[3]), int(sys.argv[4])
OPEN = lambda ch: ch not in 's#'
tot = collections.Counter()
for s in seeds:
    d = f'{B}/s{s}'
    ev = [l for l in open(f'{d}/events.txt') if 'FOUNDED' in l][0]
    kv = dict(t.split('=') for t in ev.split() if '=' in t); nx, gy = int(kv['nest_x']), int(kv['ground_y'])
    cache = {}
    def load(f):
        if f not in cache:
            p = f'{d}/map_f{f}.txt'
            if not os.path.exists(p): cache[f] = None; return None
            L = open(p).read().split('\n'); x0, y0, w, h = map(int, L[0].split()); g = L[1:1 + h]
            # reach from top row, 8-connected
            seen = set(); st = [(x, 0) for x in range(w) if OPEN(g[0][x])]
            seen.update(st)
            while st:
                x, y = st.pop()
                for dx in (-1, 0, 1):
                    for dy in (-1, 0, 1):
                        u, v = x + dx, y + dy
                        if 0 <= u < w and 0 <= v < h and (u, v) not in seen and OPEN(g[v][u]): seen.add((u, v)); st.append((u, v))
            cache.clear(); cache[f] = (x0, y0, w, h, g, seen)
        return cache[f]
    c = collections.Counter()
    for r in csv.DictReader(open(f'{d}/colony.csv')):
        f = int(r['frame'])
        if not (a <= f <= b): continue
        c['samples'] += 1
        if r['zone'] != 'nest': continue
        c['nest'] += 1
        m = load(f)
        if m is None: c['nomap'] += 1; continue
        x0, y0, w, h, g, seen = m; x, y = int(r['hx']) - x0, int(r['hy']) - y0
        if not (0 <= x < w and 0 <= y < h): c['offmap'] += 1; continue
        dep = int(r['hy']) - gy
        c['d1-3' if dep <= 3 else 'd4-10' if dep <= 10 else 'd11-25' if dep <= 25 else 'd26+'] += 1
        nb = sum(OPEN(g[v][u]) for u in (x - 1, x, x + 1) for v in (y - 1, y, y + 1) if 0 <= u < w and 0 <= v < h)
        c['room' if nb >= 7 else 'tunnel'] += 1
        if (x, y) not in seen: c['sealed'] += 1
        if abs(int(r['hx']) - nx) <= 4 and dep <= 10: c['door_col_top10'] += 1
        if int(r['spoil']) > 0: c['holding_spoil'] += 1
    n = max(1, c['nest'] - c['nomap'] - c['offmap'])
    print(f"s{s} {a//1000}-{b//1000}k: in-nest {100*c['nest']/c['samples']:.1f}% of {c['samples']} samples; of in-nest (n={n}): "
          + ' '.join(f"{k} {100*c[k]/n:.0f}%" for k in ('d1-3','d4-10','d11-25','d26+','room','tunnel','sealed','door_col_top10','holding_spoil'))
          + f" | nomap {c['nomap']} offmap {c['offmap']}")
    tot.update(c)
