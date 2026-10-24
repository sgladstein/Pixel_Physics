"""Time-matched hand-over check: each adult's cuts in 60k-200k against what an average adult cut over the same frames
(10k bins). Ratio observed/expected for the ants alive at the end vs the ants that left."""
import csv, sys, statistics as st
NEST = 128; W0 = 120000; BIN = 10000
res = []
for d in sys.argv[1:]:
    life = {}
    for r in csv.DictReader(open(d + '/ants.csv')):
        life[int(r['id'])] = [0, 200000, 'alive']
    for r in csv.DictReader(open(d + '/births.csv')):
        i = int(r['id']); f = int(r['frame'])
        if r['kind'] in ('birth', 'refill', 'hall'):
            life[i] = [f, 200000, 'alive']
        elif r['kind'] in ('cull', 'retire'):
            life.setdefault(i, [0, 200000, 'alive'])[1:] = [f, r['kind']]
    ev = list(csv.DictReader(open(d + '/events.csv')))
    for e in ev:
        if e['kind'] == 'death' and e['v'] == '1':
            life.setdefault(int(e['id']), [0, 200000, 'alive'])[1:] = [int(e['frame']), 'starved']
    cuts = {}
    for e in ev:
        f = int(e['frame'])
        if e['kind'] == 'dig' and f >= W0 and abs(int(e['x']) - NEST) <= 20 and int(e['id']) in life:
            cuts.setdefault(int(e['id']), []).append(f)
    nb = (200000 - W0) // BIN
    tot_c = [0] * nb; tot_t = [0] * nb; per = {}
    for i, (b, en, how) in life.items():
        a = max(b + 8000, W0)
        t = [max(0, min(en, W0 + (k + 1) * BIN) - max(a, W0 + k * BIN)) for k in range(nb)]
        c = [0] * nb
        for f in cuts.get(i, []):
            if a <= f < en:
                c[(f - W0) // BIN] += 1
        per[i] = (t, c, how)
        for k in range(nb):
            tot_c[k] += c[k]; tot_t[k] += t[k]
    rate = [tot_c[k] / tot_t[k] if tot_t[k] else 0 for k in range(nb)]
    g = {'alive': [0, 0.0], 'left': [0, 0.0]}
    for i, (t, c, how) in per.items():
        if sum(t) < 5000:
            continue
        k = 'alive' if how == 'alive' else 'left'
        g[k][0] += sum(c); g[k][1] += sum(t[j] * rate[j] for j in range(nb))
    if g['alive'][1] > 0 and g['left'][1] > 0:
        res.append((d, g['alive'][0] / g['alive'][1], g['left'][0] / g['left'][1]))
lo = sum(a < l for _, a, l in res); hi = sum(a > l for _, a, l in res)
print(f'time-matched cuts (observed/expected), 60k-200k: alive at end {st.median(a for _, a, _ in res):.2f} vs left {st.median(l for _, _, l in res):.2f} (medians); alive lower in {lo}, higher in {hi} of {len(res)} rounds')
