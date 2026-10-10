"""moundfood.py RUN [BASE] -- what food lay round the ants that starved in the mound tunnels (zonedeaths.py's
zd_mound_in_rows.csv), from the census (storefood.csv, foodevery=500): per ant at its last decision row, the loose food
cells within 3 and 10 cells that its mouth can and cannot take (visible), and the nearest of each. Then, per 5k, the loose
food in the mound pocket box (x 226-248, y 140-159) for RUN and BASE: cells its mouth can / cannot take (Deep trace,
2026-10-07)."""
import sys, csv, collections, statistics as st
RUN = sys.argv[1]; BASE = sys.argv[2] if len(sys.argv) > 2 else None
BOX = (226, 248, 140, 159)
last = {}
for r in csv.DictReader(open(f'{RUN}/zd_mound_in_rows.csv')):
    if r.get('hy') not in (None, ''):
        last[r['id']] = (int(r['frame']), int(r['hx']), int(r['hy']))
want = {f // 500 * 500 for f, _, _ in last.values()}
def scan(run, keep):
    cen = collections.defaultdict(list); box = {}
    for r in csv.DictReader(open(f'{run}/storefood.csv')):
        f = int(r['frame']); x, y = int(r['x']), int(r['y'])
        if keep and f in keep:
            cen[f].append((x, y, r['visible'] == '1', r['mat'], float(r['worth'])))
        if BOX[0] <= x <= BOX[1] and BOX[2] <= y <= BOX[3]:
            b = box.setdefault(f // 5000 * 5000, collections.Counter())
            b['vis' if r['visible'] == '1' else 'invis'] += 1
            b['n'] += 0
    return cen, box
cen, box = scan(RUN, want)
rows = []
for i, (f, x, y) in sorted(last.items(), key=lambda kv: kv[1][0]):
    cells = cen.get(f // 500 * 500, [])
    d = [(max(abs(cx - x), abs(cy - y)), v, m, w) for cx, cy, v, m, w in cells]
    v3 = sum(1 for k, v, m, w in d if k <= 3 and v); i3 = sum(1 for k, v, m, w in d if k <= 3 and not v)
    v10 = sum(1 for k, v, m, w in d if k <= 10 and v); i10 = sum(1 for k, v, m, w in d if k <= 10 and not v)
    nv = min((k for k, v, m, w in d if v), default=None); ni = min((k for k, v, m, w in d if not v), default=None)
    iw = [w for k, v, m, w in d if k <= 10 and not v]
    rows.append((i, f, x, y, v3, i3, v10, i10, nv, ni, collections.Counter(m for k, v, m, w in d if k <= 10 and not v), iw))
print(f'{RUN}: {len(rows)} mound starvers, food at the last row (census <=500 fr before):')
print('  id        frame   x,y     edible<=3 inedible<=3 | edible<=10 inedible<=10 | nearest edible / inedible | inedible kinds')
for i, f, x, y, v3, i3, v10, i10, nv, ni, kinds, iw in rows:
    print(f'  {i:>9} {f:>7} {x},{y}   {v3:>3} {i3:>3} | {v10:>3} {i10:>3} | {nv} / {ni} | {dict(kinds)}')
allw = [w for r in rows for w in r[11]]
print(f'  inedible cells within 10 (summed over ants): worth median {st.median(allw) if allw else None}, max {max(allw) if allw else None}')
print(f'  ants with an inedible cell within 3: {sum(1 for r in rows if r[5])}; with an edible one within 10: {sum(1 for r in rows if r[6])}')
_, bb = scan(BASE, None) if BASE else (None, {})
print(f'\n  loose food in the pocket box x {BOX[0]}-{BOX[1]}, y {BOX[2]}-{BOX[3]} (cell-samples per 5k, 10 censuses): edible/inedible')
for k in sorted(set(box) | set(bb)):
    if k < 150000: continue
    a = box.get(k, {}); b = bb.get(k, {})
    print(f'  {k//1000:>4}k  {RUN.split("/")[-1]}: {a.get("vis",0):>4}/{a.get("invis",0):<5}  {BASE.split("/")[-1] if BASE else ""}: {b.get("vis",0):>4}/{b.get("invis",0)}')
