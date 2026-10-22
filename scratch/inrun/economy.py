# The ant's food economy in one inrun run: what is eaten, at what tank level, and what an ant burns.
import csv, sys, collections, bisect
D = sys.argv[1]; START = 200.0
tr = collections.defaultdict(list)
for r in csv.DictReader(open(f"{D}/trace.csv")):
    tr[int(r['id'])].append((int(r['frame']), float(r['tank']), int(r['crop'])))
eat_by = collections.Counter(); n_by = collections.Counter(); field_w = []; total = 0.0
bands = [(0, 1), (1, 3), (3, 5.5), (5.5, 20), (20, 1e9)]
def band(t):
    for lo, hi in bands:
        if lo <= t < hi: return f"{lo}-{hi if hi < 1e9 else 'up'}"
for r in csv.DictReader(open(f"{D}/events.csv")):
    k = r['kind']
    if k == 'field': field_w.append(float(r['v']))
    if k != 'eat': continue
    i, f, v = int(r['id']), int(r['frame']), float(r['v'])
    t = tr.get(i)
    if not t: continue
    j = bisect.bisect_right([x[0] for x in t], f) - 1
    tank = t[max(j, 0)][1]
    eat_by[band(tank)] += v; n_by[band(tank)] += 1; total += v
# burn: tank change over 250-frame steps with the crop empty both ends and no eat in between is pure spending
eats = collections.defaultdict(list)
for r in csv.DictReader(open(f"{D}/events.csv")):
    if r['kind'] == 'eat': eats[int(r['id'])].append(int(r['frame']))
burn = []
for i, t in tr.items():
    e = sorted(eats.get(i, []))
    for a, b in zip(t, t[1:]):
        if b[0] - a[0] != 250 or a[2] or b[2]: continue
        k = bisect.bisect_left(e, a[0])
        if k < len(e) and e[k] <= b[0]: continue
        burn.append((a[1] - b[1]) * START / 250)
burn.sort()
antf = sum(len(t) for t in tr.values()) * 250
print(f"{D}: food cells bitten {len(field_w)}, mean face worth {sum(field_w)/max(1,len(field_w)):.0f} J; digested in total {total:,.0f} J")
print(f"burn while not eating: median {burn[len(burn)//2]:.3f} J/frame (p10 {burn[len(burn)//10]:.3f}, p90 {burn[9*len(burn)//10]:.3f}) = {burn[len(burn)//2]*1000/START:.2f} tanks per 1,000 frames; ant-frames lived {antf:,}")
print(f"colony burn at that rate {burn[len(burn)//2]*antf:,.0f} J")
print("digested by the eater's tank at the time (tanks of 200 J):")
for lo, hi in bands:
    b = f"{lo}-{hi if hi < 1e9 else 'up'}"
    print(f"  {b:8s} {eat_by[b]:12,.0f} J  {100*eat_by[b]/max(total,1):5.1f}%")
