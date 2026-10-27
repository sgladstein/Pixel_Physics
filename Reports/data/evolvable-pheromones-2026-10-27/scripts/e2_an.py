"""E2: room for a signal. Per run and food epoch: t0 = first field pickup within 20 cols of the heap; lag = median
over finder ants of (own first pickup there - t0); lost = share of out-of-nest empty ants, in snapshots after t0,
that are NOT in the corridor between the door and the heap (+-20): the ants still searching when a nestmate knows."""
import csv, glob, json, os, statistics as st, sys
from collections import defaultdict
RUNS = sys.argv[1]
res = defaultdict(lambda: defaultdict(list))
for d in sorted(glob.glob(f'{RUNS}/*/s*/')):
    arm = d.rstrip('/').split('/')[-2]
    try:
        P = [(int(r['frame']), int(r['x'])) for r in csv.DictReader(open(d + 'patches.csv'))]
        ev = [l.split(',') for l in open(d + 'events.csv').read().split('\n')[1:] if ',field,' in l or ',deliver,' in l]
    except OSError:
        continue
    dl = [int(e[3]) for e in ev if e[2] == 'deliver']
    if not dl: continue
    door = st.median(dl)
    snaps = sorted(glob.glob(d + 'ants_f*.txt'))
    for k, (f0, px) in enumerate(P):
        f1 = P[k + 1][0] if k + 1 < len(P) else 100001
        hits = [(int(e[0]), e[1]) for e in ev if e[2] == 'field' and f0 <= int(e[0]) < f1 and abs(int(e[3]) - px) <= 20]
        if not hits:
            res[arm]['unfound'].append(1); continue
        res[arm]['unfound'].append(0)
        t0 = hits[0][0]; first = {}
        for t, i in hits: first.setdefault(i, t)
        res[arm]['find_delay'].append(t0 - f0)
        res[arm]['lag'].append(st.median(v - t0 for v in first.values()))
        res[arm]['finders'].append(len(first))
        lo, hi = sorted((door, px)); lost = out = 0
        for s in snaps:
            fr = int(s[-11:-4])
            if not (t0 < fr < f1): continue
            for l in open(s):
                p = l.split()
                if len(p) < 5: continue
                x, laden = float(p[1]), p[4] == '1'
                if abs(x - door) <= 8 or laden: continue
                out += 1
                if not (lo - 20 <= x <= hi + 20): lost += 1
        if out: res[arm]['lost'].append(lost / out)
for a in sorted(res):
    r = res[a]
    print(f"{a:12} epochs {len(r['unfound']):3} unfound {sum(r['unfound']):2}  first find {st.median(r['find_delay']):6.0f} fr  finder lag {st.median(r['lag']):6.0f} fr  finders/epoch {st.median(r['finders']):4.0f}  lost searchers {100*st.median(r['lost']):4.0f}%")
