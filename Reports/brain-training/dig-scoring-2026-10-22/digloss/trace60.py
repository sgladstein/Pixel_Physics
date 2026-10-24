"""Per-ant trace of one L60c1 round: does digging cost an ant its children?"""
import csv, sys, glob, statistics as st
NEST = 128
def round_ants(d):
    A = {}
    def a(i):
        return A.setdefault(i, dict(born=0, end=200000, how='alive', dig=0, dign=0, out=0, outfar=0, **{'in': 0}, deliver=0, kids=0, field=0, homeeat=0, parent=None))
    for r in csv.DictReader(open(d + '/ants.csv')):
        a(int(r['id']))
    for r in csv.DictReader(open(d + '/births.csv')):
        i = int(r['id']); f = int(r['frame'])
        if r['kind'] in ('birth', 'refill', 'hall'):
            x = a(i); x['born'] = f; x['parent'] = int(r['parent'])
            if r['kind'] == 'birth' and int(r['parent']):
                a(int(r['parent']))['kids'] += 1
        elif r['kind'] in ('cull', 'retire'):
            x = a(i); x['end'] = f; x['how'] = r['kind']
    for e in csv.DictReader(open(d + '/events.csv')):
        k = e['kind']; i = int(e['id'])
        if k == 'death':
            x = a(i)
            if x['how'] == 'alive' or e['v'] == '1':
                x['end'] = int(e['frame']); x['how'] = {'1': 'starved', '4': 'cull'}.get(e['v'], 'v' + e['v'])
        elif k == 'dig':
            x = a(i); x['dig'] += 1; x['dign'] += abs(int(e['x']) - NEST) <= 20
        elif k == 'out':
            x = a(i); x['out'] += 1; x['outfar'] += abs(int(e['x']) - NEST) >= 15
        elif k in ('in', 'deliver', 'field', 'homeeat'):
            a(i)[k] += 1
    for x in A.values():
        x['life'] = max(1, x['end'] - x['born'])
    return A

rows = []
for d in sys.argv[1:]:
    A = round_ants(d)
    for i, x in A.items():
        if x['life'] < 8000:   # never an adult: never judged
            continue
        x['src'] = d; x['id'] = i; rows.append(x)
rows.sort(key=lambda x: x['dign'] / x['life'])
q = len(rows) // 4
groups = [('no cuts', [x for x in rows if x['dign'] == 0]),
          ('some cuts (bottom half of cutters)', None), ('top quarter of cutters', None)]
cutters = [x for x in rows if x['dign'] > 0]
cutters.sort(key=lambda x: x['dign'] / x['life'])
h = len(cutters) // 2; t = len(cutters) * 3 // 4
groups[1] = (groups[1][0], cutters[:h]); groups[2] = (groups[2][0], cutters[t:])
print(f'{len(rows)} adult ants in {len(sys.argv) - 1} rounds')
for name, g in groups:
    if not g:
        continue
    life = sum(x['life'] for x in g) / 1e4
    hows = {h: sum(x['how'] == h for x in g) for h in ('alive', 'cull', 'starved', 'retire')}
    print(f'{name:38s} n={len(g):4d}  cuts/10k {sum(x["dign"] for x in g)/life:6.2f}  outs far/10k {sum(x["outfar"] for x in g)/life:5.2f} '
          f' deliveries/10k {sum(x["deliver"] for x in g)/life:6.2f}  kids/10k {sum(x["kids"] for x in g)/life:5.3f}  '
          f'ended: culled {hows["cull"]/len(g):.0%} starved {hows["starved"]/len(g):.0%} retired {hows["retire"]/len(g):.0%} alive {hows["alive"]/len(g):.0%}  '
          f'ants w/ kids {sum(x["kids"]>0 for x in g)/len(g):.0%}')

# Heritability and the hand-over.
print('\nparent -> child cut rate (adults with an adult parent in the same round):')
pairs = []; hand = []
for d in sys.argv[1:]:
    A = round_ants(d)
    for i, x in A.items():
        p = A.get(x['parent']) if x['parent'] else None
        if x['life'] >= 8000 and p and p['life'] >= 8000:
            pairs.append((p['dign'] / p['life'] * 1e4, x['dign'] / x['life'] * 1e4))
    ad = [x for x in A.values() if x['life'] >= 8000]
    alive = [x for x in A.values() if x['how'] == 'alive']
    rate = lambda g: sum(x['dign'] for x in g) / max(1, sum(x['life'] for x in g)) * 1e4
    # Founders of the next round are the living: what did their parents (the line) cut?
    par = [A[x['parent']] for x in alive if x['parent'] in A]
    hand.append((d, rate(ad), rate([x for x in alive if x['life'] >= 8000]), rate(par), len(alive), sum(x['life'] >= 8000 for x in alive)))
if pairs:
    import math
    n = len(pairs); mx = sum(p for p, _ in pairs) / n; my = sum(c for _, c in pairs) / n
    cov = sum((p - mx) * (c - my) for p, c in pairs) / n; vx = sum((p - mx) ** 2 for p, _ in pairs) / n; vy = sum((c - my) ** 2 for _, c in pairs) / n
    print(f'  n={n} pairs, slope {cov / vx:.2f}, r {cov / math.sqrt(vx * vy):.2f}; children of non-cutting parents cut {st.mean([c for p, c in pairs if p == 0]):.2f}/10k, '
          f'of top-quarter parents {st.mean([c for p, c in sorted(pairs)[3 * n // 4:]]):.2f}/10k')
print('\nround: cuts/10k by all adults | by adults alive at the end (handed over) | by the living\'s parents   (alive, adult alive)')
for h in hand:
    print(f'  {h[0]:7s} {h[1]:5.2f} | {h[2]:5.2f} | {h[3]:5.2f}   ({h[4]}, {h[5]})')
