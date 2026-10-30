"""Every ant that reached an end wall (surface, x<12 or x>436): its wall spells, ant by ant, bucketed by outcome.
usage: wall.py RUNDIR [-v]
spell: consecutive trace rows (250 frames) at the wall. outcome: died there / left and got home (bowl or door) / left, died
elsewhere / left, alive at end not home.
"""
import csv, sys, collections, statistics as st
NEST = 128
d = sys.argv[1]; V = '-v' in sys.argv
surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
def place(x, y):
    s = surf[x] if 0 <= x < len(surf) else 0
    dx = abs(x - NEST)
    if y > s: return 'bowl' if dx <= 12 else 'tunnel'
    if x < 12 or x > 436: return 'wall'
    return 'door' if dx <= 12 else 'field'
tr = collections.defaultdict(list)
for r in csv.DictReader(open(f'{d}/trace.csv')):
    tr[int(r['id'])].append((int(r['frame']), int(r['x']), int(r['y']), float(r['tank']), int(r['crop']), int(r['trip']), int(r['since_nest']), int(r['still'])))
death = {}
born = {}
ev = collections.defaultdict(collections.Counter)
for r in csv.reader(open(f'{d}/events.csv')):
    if not r[0].isdigit(): continue
    if r[2] == 'death': death[int(r[1])] = (int(r[0]), int(float(r[5])))
    if r[2] in ('deliver', 'field'): ev[int(r[1])][r[2]] += 1
nants = len(tr)
reached = 0
out = collections.Counter()
spells = []
for i, t in tr.items():
    sp = []; cur = None
    for k, r in enumerate(t):
        p = place(r[1], r[2])
        if p == 'wall':
            if cur is None:
                prev = [q for q in t[:k] if place(q[1], q[2]) in ('bowl', 'door', 'tunnel')]
                cur = dict(id=i, f0=r[0], tank0=r[3], crop0=r[4], sn0=r[6], side='W' if r[1] < 12 else 'E', n=0,
                           lasthome=prev[-1][0] if prev else None, born=t[0][0], tmin=r[3], x=r[1])
            cur['n'] += 1; cur['f1'] = r[0]; cur['tmin'] = min(cur['tmin'], r[3])
        elif cur is not None:
            rest = t[k:]
            home = next((q[0] for q in rest if place(q[1], q[2]) in ('bowl', 'door')), None)
            cur['out'] = 'got home' if home else ('died away' if i in death else 'alive away')
            cur['home_after'] = (home - cur['f1']) if home else None
            sp.append(cur); cur = None
    if cur is not None:
        cur['out'] = 'died at wall' if i in death and death[i][0] - cur['f1'] <= 500 else 'at wall at end'
        sp.append(cur)
    if sp:
        reached += 1
        spells += sp
        out[sp[-1]['out']] += 1
print(f'{d}: ants traced {nants}, reached a wall {reached}; last spell outcome {dict(out)}')
by = collections.defaultdict(list)
for s in spells: by[s['out']].append(s)
for o, ss in sorted(by.items()):
    dur = [250 * s['n'] for s in ss]
    print(f'  {o:14s} spells {len(ss):4d}  dur med {st.median(dur):6.0f} p90 {sorted(dur)[int(.9*len(dur))]:6d}  tank@arrive med {st.median(s["tank0"] for s in ss):.2f}'
          f'  crop@arrive>0 {sum(s["crop0"]>0 for s in ss):3d}  since_nest@arrive med {st.median(s["sn0"] for s in ss):6.0f}  side W/E {sum(s["side"]=="W" for s in ss)}/{sum(s["side"]=="E" for s in ss)}'
          f'  never home before {sum(s["lasthome"] is None for s in ss)}  age@arrive med {st.median(s["f0"]-s["born"] for s in ss):6.0f}')
if V:
    for s in sorted(spells, key=lambda s: s['f0'])[:60]:
        print('   ', s)
