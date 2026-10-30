"""Every ant that starved underground (or at the door): its last 20k frames, ant by ant.
usage: under.py RUNDIR [-v N]
Per ant: born, died, depth/dx at death, how long since last on the surface beyond the door ring (out) and since last
underground->surface exit, tank at last surface, crop at death, distance moved in the last 5k frames, last verbs, the
home anchor (if traced) against its position, food (provisions/crumbs) within 3 cells at death from the nearest nestmap.
"""
import csv, sys, collections, statistics as st
NEST = 128
d = sys.argv[1]; V = int(sys.argv[sys.argv.index('-v') + 1]) if '-v' in sys.argv else 0
surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
def place(x, y):
    s = surf[x] if 0 <= x < len(surf) else 0
    dx = abs(x - NEST)
    if y > s: return 'bowl' if dx <= 12 else 'tun'
    if x < 12 or x > 436: return 'wall'
    return 'door' if dx <= 12 else 'fld'
tr = collections.defaultdict(list)
for r in csv.DictReader(open(f'{d}/trace.csv')):
    tr[int(r['id'])].append(r)
dead = {}
evs = collections.defaultdict(list)
for r in csv.reader(open(f'{d}/events.csv')):
    if not r[0].isdigit(): continue
    i = int(r[1])
    if r[2] == 'death': dead[i] = (int(r[0]), int(float(r[5])))
    elif r[2] in ('deliver', 'field', 'homeeat', 'eat', 'dig', 'in', 'out'): evs[i].append((int(r[0]), r[2]))
rows = []
for i, (df, c) in dead.items():
    if c != 1 or not tr[i]: continue
    t = tr[i]; L = t[-1]
    pl = place(int(L['x']), int(L['y']))
    if pl in ('wall', 'fld'): continue
    fr = lambda r: int(r['frame'])
    last_out = max((fr(r) for r in t if place(int(r['x']), int(r['y'])) in ('fld', 'wall')), default=None)
    last_surf = max((fr(r) for r in t if place(int(r['x']), int(r['y'])) not in ('bowl', 'tun')), default=None)
    t5 = [r for r in t if fr(r) >= df - 5000]
    path = sum(abs(int(a['x']) - int(b['x'])) + abs(int(a['y']) - int(b['y'])) for a, b in zip(t5, t5[1:]))
    span = (max(int(r['x']) for r in t5) - min(int(r['x']) for r in t5), max(int(r['y']) for r in t5) - min(int(r['y']) for r in t5))
    verbs = collections.Counter(r.get('verb', '?') for r in t5)
    e = evs[i]
    last_eat = max((f for f, k in e if k in ('eat',)), default=None)
    n_deliv = sum(k == 'deliver' for f, k in e); n_field = sum(k == 'field' for f, k in e); n_dig = sum(k == 'dig' for f, k in e)
    peak = max(float(r['tank']) for r in t)
    tank_ls = next((float(r['tank']) for r in reversed(t) if fr(r) == last_surf), None) if last_surf else None
    anc = (int(L['ax']), int(L['ay'])) if 'ax' in L else None
    rows.append(dict(id=i, born=fr(t[0]), died=df, place=pl, x=int(L['x']), dep=int(L['y']) - surf[int(L['x'])], dx=int(L['x']) - NEST,
                     since_out=(df - last_out) if last_out is not None else None, since_surf=(df - last_surf) if last_surf is not None else None,
                     tank_ls=tank_ls, peak=peak, crop=int(L['crop']), spoil=int(L['spoil']), path5k=path, span5k=span, verbs=dict(verbs.most_common(4)),
                     anchor=anc, since_eat=(df - last_eat) if last_eat else None, deliv=n_deliv, field=n_field, dig=n_dig))
print(f'{d}: starved underground/door {len(rows)}')
if not rows: sys.exit()
def med(k):
    v = [r[k] for r in rows if r[k] is not None]
    return f'{st.median(v):.0f} (n {len(v)})' if v else '-'
print('  place', dict(collections.Counter(r['place'] for r in rows)))
print('  age at death med', st.median(r['died'] - r['born'] for r in rows), ' never out on the field:', sum(r['since_out'] is None for r in rows),
      ' never on the surface at all:', sum(r['since_surf'] is None for r in rows))
print('  frames since last on the field', med('since_out'), ' since last on the surface', med('since_surf'), ' since last bite', med('since_eat'))
print('  depth med', st.median(r['dep'] for r in rows), ' |dx| med', st.median(abs(r['dx']) for r in rows), ' path in last 5k med', st.median(r['path5k'] for r in rows))
print('  ever delivered', sum(r['deliv'] > 0 for r in rows), ' ever field-bit', sum(r['field'] > 0 for r in rows), ' ever dug', sum(r['dig'] > 0 for r in rows),
      ' holding spoil at death', sum(r['spoil'] for r in rows), ' crop>0 at death', sum(r['crop'] > 0 for r in rows))
vv = collections.Counter()
for r in rows: vv.update(r['verbs'])
print('  last-verb rows, last 5k frames:', dict(vv.most_common()))
if rows[0]['anchor']:
    ad = [max(abs(r['anchor'][0] - r['x']), abs(r['anchor'][1] - (r['dep'] + surf[r['x']]))) for r in rows]
    print('  home anchor Chebyshev distance from the ant at death: med', st.median(ad), ' 0-2:', sum(a <= 2 for a in ad), ' anchor |dx| from door med', st.median(abs(r['anchor'][0] - NEST) for r in rows))
for r in sorted(rows, key=lambda r: r['died'])[:V]:
    print('   ', r)
def kind(r):
    if r['since_surf'] is None and r['peak'] < 1: return 'hatchling'
    if r['span5k'][0] <= 2 and r['span5k'][1] <= 2: return 'stuck'
    if r['since_surf'] is None: return 'neverout'
    return 'other'
K = collections.Counter(kind(r) for r in rows)
print('  KINDS', dict(K), '| stuck holding spoil', sum(kind(r) == 'stuck' and r['spoil'] for r in rows),
      '| stuck in tunnel', sum(kind(r) == 'stuck' and r['place'] == 'tun' for r in rows))
