#!/usr/bin/env python3
"""drops.py RUN LO HI -- every pellet put down (digging record cause drop) in (LO,HI], by place, with the dropping ant's
drive and job from walk.csv (its row at that frame or the nearest earlier) and whether it died that frame."""
import sys, gzip, csv, collections, re
run, lo, hi = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
drops = []
with gzip.open(run + '/cells.csv.gz', 'rt') as fh:
    r = csv.reader(fh); next(r)
    for row in r:
        f = int(row[0])
        if f <= lo: continue
        if f > hi: break
        if row[5] == 'drop':
            x, y = int(row[1]), int(row[2])
            drops.append((f, x, y, row[6]))
died = {}
for l in open(run + '/events.txt'):
    if ' DIED ' in l:
        died[re.search(r' id=(\d+)', l).group(1)] = int(l.split()[0])
want = collections.defaultdict(list)
for d in drops:
    want[d[3]].append(d[0])
last = {}
info = {}
with gzip.open(run + '/walk.csv.gz', 'rt') as fh:
    r = csv.reader(fh); h = next(r); ix = {k: j for j, k in enumerate(h)}
    for row in r:
        f = int(row[0])
        if f < lo - 200: continue
        if f > hi + 1: break
        i = row[1]
        if i in want:
            for tf in want[i]:
                if f <= tf + 1:
                    info[(tf, i)] = (row[ix['drive']], row[ix['job']], float(row[ix['hunger']]), row[ix['cut']])
c = collections.Counter()
for f, x, y, i in drops:
    place = 'below' if y >= 160 else ('door5' if abs(x - 256) <= 5 else 'mound')
    dv = info.get((f, i), ('?', '?', 0, '0'))
    death = 'DIED' if died.get(i) in (f, f - 1, f + 1) else ''
    c[(place, dv[0], dv[1], 'hungry' if dv[2] > 0.5 else '', death)] += 1
for k, v in sorted(c.items(), key=lambda kv: -kv[1])[:30]:
    print(f'  {v:5d}  place={k[0]:6s} drive={k[1]:7s} job={k[2]:7s} {k[3]:6s} {k[4]}')
print('total drops', len(drops))
