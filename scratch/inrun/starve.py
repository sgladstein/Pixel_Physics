# Trace every ant that starved: where it spent its last frames, its tank, and its last trip.
import csv, sys, collections
D = sys.argv[1]; NEST = 128
surf = [int(l) for l in open(f"{D}/surface.txt")]
ev = collections.defaultdict(list)
for r in csv.DictReader(open(f"{D}/events.csv")):
    ev[int(r['id'])].append((int(r['frame']), r['kind'], int(r['x']), int(r['y']), float(r['v'])))
tr = collections.defaultdict(list)
for r in csv.DictReader(open(f"{D}/trace.csv")):
    tr[int(r['id'])].append((int(r['frame']), int(r['x']), int(r['y']), float(r['tank']), int(r['crop']), int(r['trip']), int(r['spoil']), int(r['since_nest']), int(r['still'])))
def where(x, y):
    dx = abs(x - NEST); s = surf[x] if 0 <= x < len(surf) else 0
    if dx <= 12 and y >= s: return 'nest'
    if dx <= 12: return 'door'
    return 'out'
rows = []
for i, es in ev.items():
    d = [e for e in es if e[1] == 'death']
    if not d or d[0][4] != 1: continue
    df = d[0][0]
    t = [r for r in tr[i] if r[0] <= df]
    if not t: continue
    born = t[0][0]
    last20 = [r for r in t if r[0] >= df - 20000]
    w = collections.Counter(where(r[1], r[2]) for r in last20)
    n = max(1, len(last20))
    # last frame the ant was out (beyond 12 columns), last field bite, last delivery
    lo = max([r[0] for r in t if where(r[1], r[2]) == 'out'], default=-1)
    lf = max([e[0] for e in es if e[1] == 'field'], default=-1)
    ld = max([e[0] for e in es if e[1] == 'deliver'], default=-1)
    le = max([e[0] for e in es if e[1] in ('eat',)], default=-1)
    nf = sum(1 for e in es if e[1] == 'field')
    peak = max(t, key=lambda r: r[3])
    # when did the tank fall below 1 grant for good
    below = df
    for r in reversed(t):
        if r[3] >= 1.0: break
        below = r[0]
    still = sum(1 for r in last20 if r[8] > 200) / n
    rows.append((df, i, born, df - born, w['nest'] / n, w['door'] / n, w['out'] / n, lo, lf, nf, ld, le, peak[3], peak[0], below, still, t[-1][1] - NEST, t[-1][2] - surf[t[-1][1]]))
rows.sort()
print(f"{D}: {len(rows)} starved")
print("death   id  age    nest door  out  last_out last_field n_field last_deliv last_eat  peak@frame     <1grant_from still  dx  depth")
for r in rows:
    print(f"{r[0]:6d} {r[1]:4d} {r[3]:6d}  {r[4]:.2f} {r[5]:.2f} {r[6]:.2f} {r[7]:8d} {r[8]:9d} {r[9]:6d} {r[10]:9d} {r[11]:8d}  {r[12]:5.1f}@{r[13]:<7d} {r[14]:7d}  {r[15]:.2f} {r[16]:4d} {r[17]:4d}")
