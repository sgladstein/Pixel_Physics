# Where does field food go? Per 20k-frame window: cells bitten in the field, put down at the nest (deliver),
# dropped near the nest (churn), eaten at home from the store (homeeat); and where crops empty (from trace.csv).
import csv, sys, collections
D = sys.argv[1]; NEST = 128; W = 20000
surf = [int(l) for l in open(f"{D}/surface.txt")]
def where(x, y):
    dx = abs(x - NEST); s = surf[x] if 0 <= x < len(surf) else 0
    return 'nest' if dx <= 12 and y >= s else ('door' if dx <= 12 else 'out')
k = collections.defaultdict(collections.Counter)
dist = collections.Counter()
for r in csv.DictReader(open(f"{D}/events.csv")):
    f = int(r['frame']); kind = r['kind']
    if kind in ('field', 'deliver', 'churn', 'homeeat', 'hungryout'):
        k[f // W][kind] += 1
    if kind == 'field':
        dist[min(abs(int(r['x']) - NEST) // 10 * 10, 120)] += 1
# crop emptying: an ant's crop goes from >0 to 0 between trace rows; where was it?
prev = {}; empt = collections.defaultdict(collections.Counter); full = collections.defaultdict(collections.Counter)
for r in csv.DictReader(open(f"{D}/trace.csv")):
    f, i, x, y, c = int(r['frame']), int(r['id']), int(r['x']), int(r['y']), int(r['crop'])
    if c > 0: full[f // W][where(x, y)] += 1
    if prev.get(i, 0) > 0 and c == 0: empt[f // W][where(x, y)] += 1
    prev[i] = c
print(f"{D}  (cells; crop rows = ant-samples every 250 frames with food in the crop)")
print("window      field deliver churn homeeat hungryout | crop held out/door/nest | crop emptied out/door/nest")
for w in sorted(set(k) | set(full)):
    a, b, e = k[w], full[w], empt[w]
    print(f"{w*W:6d}+  {a['field']:6d} {a['deliver']:7d} {a['churn']:5d} {a['homeeat']:7d} {a['hungryout']:9d} | {b['out']:5d} {b['door']:4d} {b['nest']:4d}          | {e['out']:4d} {e['door']:4d} {e['nest']:4d}")
print("field bites by columns from the nest:", dict(sorted(dist.items())))
