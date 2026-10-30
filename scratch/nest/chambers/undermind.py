"""Deep trace: every ant that starved underground (bowl/tunnel/door) in one run, decision by decision from the mind log
(pre-filtered to those ids: rows M,seed,frame,id,x,y,verb,tx,ty,last,ctx,mem,jobs,best,feats).
usage: undermind.py RUNDIR FILTERED_LOG [-v ID]
Per ant, over its last LASTF frames: decisions; share holding a pellet; when a pick-up was offered with jaws and crop empty,
what it chose; whether Eat/Feed was offered; drops of the pellet; and its tank. Buckets each ant by the gate that was shut.
"""
import csv, sys, collections, statistics as st
NEST = 128; LASTF = 3000
VERB = ['walk', 'turn', 'rest', 'dig', 'pickup', 'drop', 'eat', 'feed', 'attack']
d, logf = sys.argv[1], sys.argv[2]
VID = int(sys.argv[sys.argv.index('-v') + 1]) if '-v' in sys.argv else None
surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
def place(x, y):
    s = surf[x] if 0 <= x < len(surf) else 0
    dx = abs(x - NEST)
    if y > s: return 'bowl' if dx <= 12 else 'tun'
    if x < 12 or x > 436: return 'wall'
    return 'door' if dx <= 12 else 'fld'
last, first = {}, {}
for r in csv.DictReader(open(f'{d}/trace.csv')):
    i = int(r['id']); last[i] = r; first.setdefault(i, r)
dead = {}
for r in csv.reader(open(f'{d}/events.csv')):
    if r[0].isdigit() and r[2] == 'death' and int(float(r[5])) == 1:
        i = int(r[1])
        if i in last and place(int(last[i]['x']), int(last[i]['y'])) in ('bowl', 'tun', 'door'):
            dead[i] = int(r[0])
R = collections.defaultdict(list)
for l in open(logf):
    p = l.rstrip('\n').split(',')
    i = int(p[3])
    if i not in dead: continue
    f = int(p[2])
    if f < dead[i] - LASTF and i != VID: continue
    ctx = [float(x) for x in p[10].split()]
    best = [float(x) for x in p[13].split()]
    R[i].append((f, int(p[4]), int(p[5]), int(p[6]), int(p[7]), int(p[8]), ctx, best))
tank = lambda c: c[0] / max(1e-6, 1 - c[0])
IDS = {}
B = collections.Counter(); agg = collections.Counter(); margins = []
for i, rs in R.items():
    rs = [r for r in rs if r[0] >= dead[i] - LASTF]
    n = len(rs)
    pel = sum(r[6][2] > 0 for r in rs)
    off_free = [r for r in rs if r[6][2] == 0 and r[6][1] == 0 and r[7][4] == r[7][4]]
    took = sum(r[3] == 4 for r in off_free)
    eat_off = sum(r[7][6] == r[7][6] for r in rs)
    feed_off = sum(r[7][7] == r[7][7] for r in rs)
    drop_pel = sum(r[3] == 5 and r[6][2] > 0 for r in rs)
    drop_off_pel = sum(r[6][2] > 0 and r[7][5] == r[7][5] for r in rs)
    agg['decisions'] += n; agg['holding pellet'] += pel; agg['pick offered, jaws+crop empty'] += len(off_free); agg['...picked'] += took
    agg['eat offered'] += eat_off; agg['feed offered'] += feed_off; agg['pellet held, drop offered'] += drop_off_pel; agg['pellet dropped'] += drop_pel
    for r in off_free:
        if r[3] != 4: margins.append(r[7][4] - r[7][r[3]])
    if n == 0: b = 'no decisions logged'
    elif pel > 0.5 * n: b = 'held a pellet most of its last 3k frames'
    elif len(off_free) == 0: b = 'never beside food with jaws free'
    elif took == 0: b = 'beside food, jaws free, never picked'
    else: b = 'picked food but starved'
    age = dead[i] - int(first[i]['frame'])
    young = 'young(<8k)' if age < 8000 else 'older'
    B[(b, young)] += 1
    IDS.setdefault(b, []).append((i, dead[i], int(last[i]['x']) - NEST, int(last[i]['y']) - surf[int(last[i]['x'])], age))
print(f'{d}: underground starvers {len(dead)}, with logged decisions {len(R)}')
for k, v in sorted(B.items(), key=lambda kv: -kv[1]): print(f'  {v:4d}  {k[0]:45s} {k[1]}')
print('  decisions in their last 3k frames:', dict(agg))
if margins:
    m = sorted(margins); print(f'  when food was offered and not taken, best pick-up score minus chosen: med {m[len(m)//2]:.3f} p10 {m[len(m)//10]:.3f} p90 {m[9*len(m)//10]:.3f}')
for b, v in IDS.items(): print('   ', b, sorted(v, key=lambda t: t[1])[:6])
if VID:
    for r in R[VID][-80:]:
        c = r[6]
        b = ' '.join('-' if x != x else f'{x:.2f}' for x in r[7])
        print(f'   {r[0]} {r[1]},{r[2]} d{r[2]-surf[r[1]]} {VERB[r[3]]}->{r[4]},{r[5]} | tank {tank(c):.2f} crop {c[1]:.2f} spoil {c[2]:.0f} at_nest {c[3]:.2f} depth {c[18]:.2f} crowd {c[17]:.2f} | best {b}')
