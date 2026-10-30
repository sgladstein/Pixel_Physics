"""The hungry ant's eating loop, ant by ant, from the hungry-filtered mind log (tank < 0.5).
Funnel (high-water per ant): hungry underground -> beside food with jaws free (pick-up offered) -> picked up ->
kept it (the next decision with food in the crop was not a Drop) -> recovered (seen again above 0.5 in trace) / starved.
Also: after a home pick-up, what the next decision was.
usage: hungryloop.py RUNDIR HUNGRYLOG"""
import csv, sys, collections
NEST = 128
VERB = ['walk', 'turn', 'rest', 'dig', 'pickup', 'drop', 'eat', 'feed', 'attack']
d, logf = sys.argv[1], sys.argv[2]
surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
starved = {int(r[1]) for r in csv.reader(open(f'{d}/events.csv')) if r[0].isdigit() and r[2] == 'death' and int(float(r[5])) == 1}
stage = collections.defaultdict(int)
prev = {}
after = collections.Counter(); where_h = collections.Counter()
for l in open(logf):
    p = l.split(',', 14)
    i, x, y, v = int(p[3]), int(p[4]), int(p[5]), int(p[6])
    ctx = [float(z) for z in p[10].split()]
    best = [float(z) for z in p[13].split()]
    under = y > surf[x]
    where_h['under' if under else ('wall' if x < 12 or x > 436 else 'door' if abs(x - NEST) <= 12 else 'field')] += 1
    if under:
        s = 1
        if ctx[2] == 0 and ctx[1] == 0 and best[4] == best[4]:
            s = 2
            if v == 4: s = 3
        pv = prev.get(i)
        if pv is not None and pv[0] == 4 and pv[1]:
            after[VERB[v] + (' (crop held)' if ctx[1] > 0 else ' (crop empty)')] += 1
            if v != 5 and ctx[1] > 0: s = 4
        stage[i] = max(stage[i], s)
        prev[i] = (v, under)
    else:
        prev[i] = (v, False)
ids = list(stage)
N = ['hungry underground', 'beside food, jaws free', 'picked up', 'kept it (next not Drop)']
print(f'{d}: hungry decisions by place {dict(where_h)}')
tot = len(ids)
for k in range(1, 5):
    n = sum(stage[i] >= k for i in ids); ns = sum(stage[i] >= k and i in starved for i in ids)
    print(f'  {N[k-1]:28s} {n:4d} ants ({100*n/max(1,tot):3.0f}% of all)  of which starved {ns}')
for k in range(1, 5):
    n = sum(stage[i] == k for i in ids); ns = sum(stage[i] == k and i in starved for i in ids)
    print(f'    stopped at "{N[k-1]}": {n} ants, starved {ns}')
print('  next decision after an underground pick-up by a hungry ant:', dict(after.most_common(6)))
