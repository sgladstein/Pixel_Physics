"""One line per run: alive at end, hatched, deliveries, field bites, deaths by cause@place, ant-time share at the walls.
usage: summ.py RUNDIR..."""
import csv, sys, collections
NEST = 128
CAUSE = ['unk', 'starved', 'starvedfly', 'killed', 'culled', 'lostvital', 'felled', 'oldage']
def place(x, y, surf):
    s = surf[x] if 0 <= x < len(surf) else 0
    dx = abs(x - NEST)
    if y > s: return 'bowl' if dx <= 12 else 'tun'
    if x < 12 or x > 436: return 'wall'
    return 'door' if dx <= 12 else 'fld'
for d in sys.argv[1:]:
    surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    last = {}; occ = collections.Counter()
    for r in csv.DictReader(open(f'{d}/trace.csv')):
        p = place(int(r['x']), int(r['y']), surf); last[int(r['id'])] = p; occ[p] += 1
    c = collections.Counter()
    for r in csv.reader(open(f'{d}/events.csv')):
        if not r[0].isdigit(): continue
        if r[2] == 'death':
            k = int(float(r[5])); c[(CAUSE[k] if k < len(CAUSE) else str(k)) + '@' + last.get(int(r[1]), '?')] += 1
        elif r[2] in ('deliver', 'field'): c[r[2]] += 1
    br = list(csv.DictReader(open(f'{d}/brood.csv')))[-1]
    alive = list(csv.DictReader(open(f'{d}/nest.csv')))[-1]['alive']
    on = sum(occ.values()) or 1
    dth = ' '.join(f'{k}={v}' for k, v in c.most_common() if '@' in k)
    st = sum(v for k, v in c.items() if k.startswith('starved@')); sw = c['starved@wall']
    print(f'{d:22s} alive {alive:>3} hatched {br["hatched"]:>4} deliv {c["deliver"]:5d} field {c["field"]:5d} wall-time {100*occ["wall"]/on:3.0f}% '
          f'starved {st:4d} (wall {sw:4d}) | {dth}')
