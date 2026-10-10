"""storecensus2.py RUN [LO HI] -- the traced problem as one number per run, defined so it does NOT depend on what the
store counts (Deep trace, 2026-10-07). For Nest race's `edible` test: under `edible`, `is_store_cell` drops the crumbs
too small to bite, so storecensus.py's "invisible store cells" would read zero by construction on that arm.

Reads storefood.csv (deeptrace `foodevery=500`), ledger.csv and nd_rows.csv (the starvers' decision rows, from
nestdeaths.py). For every ant that starved below the old ground line in LO-HI, at its last decision row and the census
just before it: the nearest loose food cell its mouth CANNOT take (any such cell in the nest region, store or not), and
the nearest cell that is edible AND a store cell (Chebyshev cells). Counts:
  near_stub_only  = an inedible cell within 10 and no edible store cell within 10 (the traced cause, both arms alike)
  no_edible_store = no edible store cell within 10, whatever else is there
"""
import sys, csv, collections, bisect
RUN = sys.argv[1]
LO = int(sys.argv[2]) if len(sys.argv) > 2 else 200000
HI = int(sys.argv[3]) if len(sys.argv) > 3 else 300000
R = 10
cen = collections.defaultdict(list)
for r in csv.DictReader(open(f'{RUN}/storefood.csv')):
    f = int(r['frame'])
    if LO - 1000 <= f <= HI:
        cen[f].append((int(r['x']), int(r['y']), r['visible'] == '1', r['store'] == '1'))
frames = sorted(cen)
deaths = {x['id']: int(x['end']) for x in csv.DictReader(open(f'{RUN}/ledger.csv'))
          if x['died'] == '1' and x['cause'] == 'STARVED' and x['zone_end'] == 'nest' and LO <= int(x['end']) <= HI}
last = {}
for r in csv.DictReader(open(f'{RUN}/nd_rows.csv')):
    if r['id'] in deaths and int(r['frame']) <= deaths[r['id']]:
        last[r['id']] = (int(r['frame']), int(r['hx']), int(r['hy']))
n = both = noedible = missing = 0
dv, di = [], []
for i, end in deaths.items():
    p = last.get(i)
    k = bisect.bisect_right(frames, p[0]) - 1 if p else -1
    if not p or k < 0:
        missing += 1
        continue
    n += 1
    cells = cen[frames[k]]
    d = lambda c: max(abs(c[0] - p[1]), abs(c[1] - p[2]))
    near_inv = min((d(c) for c in cells if not c[2]), default=999)
    near_edible_store = min((d(c) for c in cells if c[2] and c[3]), default=999)
    dv.append(near_edible_store); di.append(near_inv)
    noedible += near_edible_store > R
    both += near_inv <= R and near_edible_store > R
med = lambda v: sorted(v)[len(v) // 2] if v else '-'
print(f'{RUN} {LO}-{HI}: nest starvers {len(deaths)} (with a last row and a census before it: {n}; missing {missing})')
print(f'  inedible crumb within {R} and no edible store cell within {R}: {both} of {n}')
print(f'  no edible store cell within {R}: {noedible} of {n}')
print(f'  median distance to the nearest edible store cell {med(dv)}, to the nearest inedible crumb {med(di)}')
