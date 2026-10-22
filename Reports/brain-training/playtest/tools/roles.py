import csv, sys
from collections import defaultdict
d = sys.argv[1]; NEST = 128
rows = list(csv.DictReader(open(f'{d}/trace.csv')))
per = defaultdict(lambda: [0, 0, 0.0, 0])  # n, at food zone, tank sum, first frame
last = {}
for r in rows:
    i = int(r['id']); x = int(r['x']); y = int(r['y'])
    p = per[i]; p[0] += 1; p[1] += 14 <= abs(x - NEST) < 40; p[2] += float(r['tank'])
    last.setdefault(i, [int(r['frame']), 0]); last[i][1] = int(r['frame'])
out = []
for i, (n, f, t, _) in per.items():
    if n >= 100:   # >= 5000 frames alive at trace=50
        out.append((f / n, t / n, n, i))
out.sort(reverse=True)
print('ants alive >=5k frames:', len(out))
print('food-zone share  mean-tank  frames-alive  id')
for fz, tk, n, i in out[:6] + out[len(out)//2-2:len(out)//2+2] + out[-6:]:
    print(f'{100*fz:6.1f}%  {tk:8.1f}  {n*50:8d}  {i}')
import statistics as st
fz = [o[0] for o in out]
print('food-zone share quartiles', [round(100*st.quantiles(fz, n=4)[k], 1) for k in range(3)])
hi = sum(1 for o in out if o[0] > 0.5); lo = sum(1 for o in out if o[0] < 0.1)
print('ants at the food zone >50% of life:', hi, '  <10%:', lo)
