import csv, sys
from collections import Counter, defaultdict
d = sys.argv[1]
NEST = 128
surf = [int(l.split()[-1]) for l in open(f'{d}/surface.txt') if l.strip()]
rows = list(csv.DictReader(open(f'{d}/trace.csv')))
def zone(x, y):
    s = surf[x] if x < len(surf) else 64
    dx = abs(x - NEST)
    if y > s + 1 and dx < 14: return 'in nest/underground'
    if y > s + 1: return 'underground far'
    if y < s - 8: return 'air/plant/wall'
    if dx < 14: return 'surface at door'
    if dx < 40: return 'surface at food zone'
    return 'surface far (>40)'
z = Counter(); zt = defaultdict(Counter)
for r in rows:
    x, y = int(r['x']), int(r['y'])
    k = zone(x, y); z[k] += 1
    t = float(r['tank'])
    zt[k]['hungry<3' if t < 3 else 'fat>12' if t > 12 else 'mid'] += 1
n = len(rows)
for k, v in z.most_common():
    print(f'{k:24s} {100*v/n:5.1f}%   ', dict(zt[k]))
# last position of ants that starved
last = {}
for r in rows: last[int(r['id'])] = r
ev = [l.split(',') for l in open(f'{d}/events.csv')][1:]
dead = [(int(e[0]), int(e[1])) for e in ev if e[2] == 'death' and e[5] == '1']
zc = Counter()
for f, i in dead:
    r = last.get(i)
    if r: zc[zone(int(r['x']), int(r['y']))] += 1
print('starved ants last seen:', dict(zc))
