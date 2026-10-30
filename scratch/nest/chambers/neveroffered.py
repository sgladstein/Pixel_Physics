"""Hungry underground ants that were never once beside food with jaws free (and starved): what held them."""
import csv, collections
NEST = 128
V = ['walk', 'turn', 'rest', 'dig', 'pickup', 'drop', 'eat', 'feed', 'attack']
for arm in ('std', 'h3'):
    d = f'deep/{arm}_w68/r3'
    surf = [int(l) for l in open(d + '/surface.txt') if l.strip()]
    starved = {int(r[1]): int(r[0]) for r in csv.reader(open(d + '/events.csv')) if r[0].isdigit() and r[2] == 'death' and int(float(r[5])) == 1}
    rows = collections.defaultdict(list)
    for l in open(f'deep/{arm}.hungry.log'):
        p = l.split(',', 14); i = int(p[3]); x, y = int(p[4]), int(p[5])
        if y <= surf[x]: continue
        ctx = [float(z) for z in p[10].split()]; best = [float(z) for z in p[13].split()]
        rows[i].append((int(p[2]), x, y, int(p[6]), ctx, best))
    never = [i for i, rs in rows.items() if not any(r[4][2] == 0 and r[4][1] == 0 and r[5][4] == r[5][4] for r in rs)]
    ns = [i for i in never if i in starved]
    pel = collections.Counter(); verbs = collections.Counter(); dxs = []; deps = []; frames = []; atn = []
    for i in ns:
        rs = rows[i]
        pel['pellet most' if sum(r[4][2] > 0 for r in rs) > len(rs) / 2 else 'jaws free most'] += 1
        pel['crop>0 most' if sum(r[4][1] > 0 for r in rs) > len(rs) / 2 else 'crop empty most'] += 1
        for r in rs: verbs[r[3]] += 1
        atn.append(sum(r[4][3] for r in rs) / len(rs))
        dxs.append(rs[-1][1] - NEST); deps.append(rs[-1][2] - surf[rs[-1][1]]); frames.append(starved[i] // 1000)
    print(arm, 'hungry underground, never beside food with jaws free:', len(never), 'starved', len(ns), dict(pel), {V[k]: v for k, v in verbs.most_common()})
    print('   last dx', sorted(dxs), '\n   depth', sorted(deps), '\n   death frame (k)', sorted(frames), '\n   at_nest sense mean per ant', sorted(round(a, 2) for a in atn))
