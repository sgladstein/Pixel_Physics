"""storestate.py RUN -- the store over time from storefood.csv: per 40k, the median counted store cells, median edible
store cells, and the share of census frames where the store reads "can feed" (8+ counted) with fewer than 8 edible
(the empty-store trap). Nest building, 2026-10-07."""
import sys, csv, collections
RUN = sys.argv[1]
fr = collections.defaultdict(lambda: [0, 0])
for r in csv.DictReader(open(f'{RUN}/storefood.csv')):
    if r['store'] == '1':
        c = fr[int(r['frame'])]
        c[0] += 1
        c[1] += r['visible'] == '1'
allf = sorted({int(r['frame']) for r in csv.DictReader(open(f'{RUN}/storefood.csv'))})
med = lambda v: sorted(v)[len(v) // 2] if v else '-'
out = []
for lo in range(0, 300000, 40000):
    fs = [f for f in allf if lo <= f < lo + 40000]
    if not fs:
        continue
    st = [fr[f][0] for f in fs]; ed = [fr[f][1] for f in fs]
    trap = sum(1 for f in fs if fr[f][0] >= 8 and fr[f][1] < 8)
    out.append(f'{lo // 1000}k: cells {med(st)} edible {med(ed)} trap {100 * trap // len(fs)}%')
print(RUN.split('/')[-1] + '  ' + ' | '.join(out))
