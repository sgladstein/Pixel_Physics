"""crumbcheck.py RUN [LO HI] -- did this run's nest starvers die in Deep trace's crumb trap? (Nest building, 2026-10-07)

Deep trace's finding (nest-race/store-arms/way-foot/residual-nest-deaths-deep-trace-2026-10-07.md): the store counts every
loose crumb worth above 0 J, the mouth only one worth about 15 J or more. So the store can read "can feed" (8+ counted
cells) with little a mouth can take, and its pull leads the hungry to crumbs they cannot eat.

Reads storefood.csv (deeptrace `foodevery=500`, the probe-v4-foodcensus patch), ledger.csv (hungry=1) and lastrows.csv
(runfood.sh: each ant's last decision row) or else digrows.csv.gz (dig=1). For every ant that STARVED with its head below the old ground line (ledger zone_end nest) in [LO, HI], at its
last decision row and the census just before that row:
  store cells        what is_store_cell counts (the 8-cell "can feed" floor reads this)
  edible store cells those of them the mouth can take (`visible`)
  nearest edible store cell / nearest inedible crumb (any loose food cell the mouth cannot take) / nearest edible loose
  food of any kind, in Chebyshev cells; 10 is the store's smell range.
Buckets, first match wins:
  empty store   the store reads "can feed" (8+ counted) and has fewer than 8 edible cells        (Deep trace's seed 4)
  wrong crumb   otherwise, an inedible crumb within 10 and no edible store cell within 10       (Deep trace's seed 3)
  food near     an edible store cell within 10 (food it could eat was in smell range)
  far           none of those: no edible store cell and no inedible crumb within 10
Measures only; the bucket names describe the census, not a traced cause.
"""
import sys, csv, gzip, bisect, collections

RUN = sys.argv[1]
LO = int(sys.argv[2]) if len(sys.argv) > 2 else 20000
HI = int(sys.argv[3]) if len(sys.argv) > 3 else 300000
R = 10

cen = collections.defaultdict(list)
for r in csv.DictReader(open(f'{RUN}/storefood.csv')):
    f = int(r['frame'])
    if LO - 2000 <= f <= HI:
        cen[f].append((int(r['x']), int(r['y']), r['visible'] == '1', r['store'] == '1', float(r['worth'])))
frames = sorted(cen)

deaths = {x['id']: int(x['end']) for x in csv.DictReader(open(f'{RUN}/ledger.csv'))
          if x['died'] == '1' and x['cause'] == 'STARVED' and x['zone_end'] == 'nest' and LO <= int(x['end']) <= HI}
allstarved = sum(1 for x in csv.DictReader(open(f'{RUN}/ledger.csv'))
                 if x['died'] == '1' and x['cause'] == 'STARVED' and LO <= int(x['end']) <= HI)

last = {}
try:  # runfood.sh keeps only each ant's last decision row
    for r in csv.DictReader(open(f'{RUN}/lastrows.csv')):
        if r['id'] in deaths and int(r['frame']) <= deaths[r['id']]:
            last[r['id']] = (int(r['frame']), int(r['hx']), int(r['hy']))
except FileNotFoundError:
    with gzip.open(f'{RUN}/digrows.csv.gz', 'rt') as fh:
        rd = csv.reader(fh)
        hdr = next(rd)
        iF, iI, iX, iY = hdr.index('frame'), hdr.index('id'), hdr.index('hx'), hdr.index('hy')
        for row in rd:
            i = row[iI]
            if i in deaths:
                f = int(row[iF])
                if f <= deaths[i]:
                    last[i] = (f, int(row[iX]), int(row[iY]))

buckets = collections.Counter()
by_window = collections.defaultdict(collections.Counter)
canfeed = 0
dv, di, de, sc, es = [], [], [], [], []
missing = 0
for i, end in deaths.items():
    p = last.get(i)
    k = bisect.bisect_right(frames, p[0]) - 1 if p else -1
    if not p or k < 0:
        missing += 1
        continue
    cells = cen[frames[k]]
    d = lambda c: max(abs(c[0] - p[1]), abs(c[1] - p[2]))
    store = [c for c in cells if c[3]]
    est = [c for c in store if c[2]]
    near_es = min((d(c) for c in est), default=999)
    near_inv = min((d(c) for c in cells if not c[2]), default=999)
    near_edible = min((d(c) for c in cells if c[2]), default=999)
    reads_feed = len(store) >= 8
    canfeed += reads_feed
    if reads_feed and len(est) < 8:
        b = 'empty store'
    elif near_inv <= R and near_es > R:
        b = 'wrong crumb'
    elif near_es <= R:
        b = 'food near'
    else:
        b = 'far'
    buckets[b] += 1
    by_window[(end // 40000) * 40][b] += 1
    dv.append(near_es); di.append(near_inv); de.append(near_edible); sc.append(len(store)); es.append(len(est))

n = sum(buckets.values())
med = lambda v: sorted(v)[len(v) // 2] if v else '-'
print(f'{RUN} {LO}-{HI}: starved {allstarved}, of them in the nest {len(deaths)} (with a last row and a census: {n}; missing {missing})')
print(f'  store read "can feed" (8+ counted cells) at the death: {canfeed} of {n}')
print(f'  Deep trace\'s headline (storecensus2 near_stub_only): inedible crumb within {R} and no edible store cell within {R}: '
      f'{sum(1 for a, b in zip(di, dv) if a <= R and b > R)} of {n}')
for b in ('empty store', 'wrong crumb', 'food near', 'far'):
    print(f'  {b:<12} {buckets[b]:>5} of {n}')
print(f'  median at the death: store cells {med(sc)}, edible store cells {med(es)}; nearest edible store cell {med(dv)}, '
      f'nearest inedible crumb {med(di)}, nearest edible loose food {med(de)}')
print('  by 40k window of death: ' + ' | '.join(
    f'{w}k ' + ' '.join(f'{b.split()[0]}{by_window[w][b]}' for b in ('empty store', 'wrong crumb', 'food near', 'far'))
    for w in sorted(by_window)))
