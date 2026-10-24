"""At each birth: the parent's cuts in the 20k frames before it, against the mean adult's cuts over the same 20k
(adults present for all of it). Ratio > 1 = parent choice favours cutters. Also the same for deliveries."""
import csv, sys, statistics as st, bisect
NEST = 128; L = 20000
out = []
for d in sys.argv[1:]:
    born = {}; end = {}
    for r in csv.DictReader(open(d + '/ants.csv')):
        born[int(r['id'])] = 0
    births = []
    for r in csv.DictReader(open(d + '/births.csv')):
        i = int(r['id']); f = int(r['frame'])
        if r['kind'] in ('birth', 'refill', 'hall'):
            born[i] = f
            if r['kind'] == 'birth' and int(r['parent']):
                births.append((f, int(r['parent'])))
        elif r['kind'] in ('cull', 'retire'):
            end[i] = f
    cut = {}; dl = {}
    for e in csv.DictReader(open(d + '/events.csv')):
        i = int(e['id']); f = int(e['frame'])
        if e['kind'] == 'death':
            end.setdefault(i, f)
        elif e['kind'] == 'dig' and abs(int(e['x']) - NEST) <= 20:
            cut.setdefault(i, []).append(f)
        elif e['kind'] == 'deliver':
            dl.setdefault(i, []).append(f)
    cnt = lambda m, i, a, b: bisect.bisect_left(m.get(i, []), b) - bisect.bisect_left(m.get(i, []), a)
    pc = pe = pd = ed = 0.0
    for f, p in births:
        if f < L + 8000:
            continue
        a = f - L
        peers = [i for i in born if born[i] + 8000 <= a and end.get(i, 10**9) >= f]
        if p not in peers or not peers:
            continue
        pc += cnt(cut, p, a, f); pe += st.mean(cnt(cut, i, a, f) for i in peers)
        pd += cnt(dl, p, a, f); ed += st.mean(cnt(dl, i, a, f) for i in peers)
    if pe > 0:
        out.append((d, pc / pe, pd / ed if ed else float('nan')))
print(f'parents vs same-time adults, cuts ratio median {st.median(x[1] for x in out):.2f} (higher in {sum(x[1] > 1 for x in out)}, lower in {sum(x[1] < 1 for x in out)} of {len(out)} rounds); '
      f'deliveries ratio median {st.median(x[2] for x in out):.2f} (higher in {sum(x[2] > 1 for x in out)})')
