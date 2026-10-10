"""storecensus.py RUN [LO HI] -- the nest store's food, cell by cell, from deeptrace's `foodevery=` census (storefood.csv),
with the deaths beside it (Deep trace, 2026-10-07).

Per census frame: the store's cells (`is_store_cell`, what the eat pull counts against its 8-cell floor), how many an ant's
mouth can see (`visible`: diet_yield through the ant's own gut over EAT_YIELD_THRESHOLD, the line adjacent_food_counted
draws), their worth, and the starvers (ledger STARVED) that died in the next 500 frames below the old ground line, with
the nearest visible and nearest invisible store cell to each one's last decision.
"""
import sys, csv, collections, math

RUN = sys.argv[1]
LO = int(sys.argv[2]) if len(sys.argv) > 2 else 200000
HI = int(sys.argv[3]) if len(sys.argv) > 3 else 300000
STEP = 500

cen = collections.defaultdict(list)
for r in csv.DictReader(open(f'{RUN}/storefood.csv')):
    f = int(r['frame'])
    if LO - STEP <= f <= HI:
        cen[f].append(r)
led = list(csv.DictReader(open(f'{RUN}/ledger.csv')))
deaths = [x for x in led if x['died'] == '1' and x['cause'] == 'STARVED' and x['zone_end'] == 'nest' and LO <= int(x['end']) <= HI]
# last decision position of each starver, from nd_rows if there, else none
lastpos = {}
try:
    for r in csv.DictReader(open(f'{RUN}/nd_rows.csv')):
        lastpos[r['id']] = (int(r['hx']), int(r['hy']), int(r['frame']))
except FileNotFoundError:
    pass

print(f'{RUN}: store food census every {STEP} frames, {LO}-{HI}')
print('frame   store: cells visible invisible | worth J visible / invisible | visible cells with an ant beside | other loose food: cells visible | starved next 500 (nest) | their nearest store cell: visible / invisible (median cells)')
for f in sorted(cen):
    rs = cen[f]
    st = [r for r in rs if r['store'] == '1']
    vis = [r for r in st if r['visible'] == '1']
    inv = [r for r in st if r['visible'] == '0']
    oth = [r for r in rs if r['store'] == '0']
    wv = sum(float(r['worth']) for r in vis)
    wi = sum(float(r['worth']) for r in inv)
    crowded = sum(1 for r in vis if int(r['ants8']) > 0)
    ds = [d for d in deaths if f < int(d['end']) <= f + STEP]
    nv, ni = [], []
    for d in ds:
        p = lastpos.get(d['id'])
        if not p:
            continue
        if vis:
            nv.append(min(max(abs(int(r['x']) - p[0]), abs(int(r['y']) - p[1])) for r in vis))
        if inv:
            ni.append(min(max(abs(int(r['x']) - p[0]), abs(int(r['y']) - p[1])) for r in inv))
    med = lambda v: (sorted(v)[len(v) // 2] if v else '-')
    print(f'{f:>7} {len(st):>9} {len(vis):>7} {len(inv):>9} | {wv:>9.0f} / {wi:>6.0f} | {crowded:>5} | {len(oth):>5} {sum(1 for r in oth if r["visible"] == "1"):>5} | {len(ds):>4} | {med(nv)} / {med(ni)}')
