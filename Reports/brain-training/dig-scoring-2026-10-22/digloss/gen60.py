"""Time-matched cuts (observed/expected over 10k bins, adults only) for founders (gen 0) vs children born this round,
and children against their own parent's ratio."""
import csv, sys, statistics as st
NEST = 128; BIN = 10000; nb = 20
res = []; pk = []
for d in sys.argv[1:]:
    life = {}; gen = {}; par = {}
    for r in csv.DictReader(open(d + '/ants.csv')):
        life[int(r['id'])] = [0, 200000]; gen[int(r['id'])] = 0
    for r in csv.DictReader(open(d + '/births.csv')):
        i = int(r['id']); f = int(r['frame'])
        if r['kind'] in ('birth', 'refill', 'hall'):
            life[i] = [f, 200000]; gen[i] = 1 if r['kind'] == 'birth' else 0; par[i] = int(r['parent'])
        elif r['kind'] in ('cull', 'retire'):
            life.setdefault(i, [0, 200000])[1] = f
    cuts = {}
    for e in csv.DictReader(open(d + '/events.csv')):
        i = int(e['id']); f = int(e['frame'])
        if e['kind'] == 'death':
            if i in life and life[i][1] == 200000: life[i][1] = f
        elif e['kind'] == 'dig' and abs(int(e['x']) - NEST) <= 20:
            cuts.setdefault(i, []).append(f)
    tc = [0] * nb; tt = [0] * nb; per = {}
    for i, (b, en) in life.items():
        a = b + 8000
        t = [max(0, min(en, (k + 1) * BIN) - max(a, k * BIN)) for k in range(nb)]
        c = [0] * nb
        for f in cuts.get(i, []):
            if a <= f < en: c[min(nb - 1, f // BIN)] += 1
        per[i] = (t, c)
        for k in range(nb): tc[k] += c[k]; tt[k] += t[k]
    rate = [tc[k] / tt[k] if tt[k] else 0 for k in range(nb)]
    oe = {}
    for i, (t, c) in per.items():
        e = sum(t[k] * rate[k] for k in range(nb))
        if sum(t) >= 5000 and e > 0.3: oe[i] = (sum(c), e)
    g = {0: [0, 0], 1: [0, 0]}
    for i, (o, e) in oe.items():
        g[gen.get(i, 0)][0] += o; g[gen.get(i, 0)][1] += e
    if g[0][1] and g[1][1]:
        res.append((g[0][0] / g[0][1], g[1][0] / g[1][1]))
    for i in oe:
        if gen.get(i) == 1 and par.get(i) in oe:
            pk.append((oe[par[i]][0] / oe[par[i]][1], oe[i][0] / oe[i][1]))
print(f'founders {st.median(a for a, _ in res):.2f} vs born this round {st.median(b for _, b in res):.2f} (medians over {len(res)} rounds; born lower in {sum(b < a for a, b in res)})')
print(f'parent ratio mean {st.mean(p for p, _ in pk):.2f} -> its children {st.mean(c for _, c in pk):.2f} (n={len(pk)}); children of parents >1.5: parent {st.mean(p for p, c in pk if p > 1.5):.2f} -> child {st.mean(c for p, c in pk if p > 1.5):.2f}')
