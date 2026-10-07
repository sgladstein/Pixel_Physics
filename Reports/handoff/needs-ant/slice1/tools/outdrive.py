#!/usr/bin/env python3
"""outdrive.py RUN [from] [to]: every empty-jawed decision outside (dig record zone surface or mound_top) of every ant,
joined with the walk's drive, energy and step chance (walk.csv frame = dig row frame + 1). Per (drive, rich >= 1000 J):
decisions, ants, share that stepped, median step chance. Needs dig=1 and walktrace=1 (every ant)."""
import csv, gzip, sys, collections, statistics
run = sys.argv[1]
a = int(sys.argv[2]) if len(sys.argv) > 2 else 80000
b = int(sys.argv[3]) if len(sys.argv) > 3 else 100000
wf = csv.reader(gzip.open(f"{run}/walk.csv.gz", "rt")); df = csv.reader(gzip.open(f"{run}/digrows.csv.gz", "rt"))
wh = next(wf); dh = next(df)
wi = {k: i for i, k in enumerate(wh)}; di = {k: i for i, k in enumerate(dh)}
n = collections.Counter(); mv = collections.Counter(); pm = collections.defaultdict(list); ants = collections.defaultdict(set)
zn = collections.Counter()
wnext = next(wf, None); cur = None; wbuf = {}
for row in df:
    f = int(row[0])
    if f < a: continue
    if f >= b: break
    if f != cur:
        cur = f; wbuf = {}
        while wnext is not None and int(wnext[0]) <= f + 1:
            if int(wnext[0]) == f + 1:
                wbuf[wnext[1]] = wnext
            wnext = next(wf, None)
    if row[di['hold']] != '0' or row[di['zone']] not in ('surface', 'mound_top'):
        continue
    w = wbuf.get(row[1])
    if w is None:
        continue
    e = float(w[wi['energy']])
    k = (w[wi['drive']], 'rich' if e >= 1000 else 'poor')
    n[k] += 1; mv[k] += row[di['moved']] == '1'; ants[k].add(row[1]); zn[(k, row[di['zone']])] += 1
    try: pm[k].append(float(w[wi['p_move']]))
    except ValueError: pass
tot = sum(n.values())
print(f"{run} {a}-{b}: empty-jawed decisions outside, {tot}")
for k in sorted(n, key=lambda k: -n[k]):
    print(f"  {k[0]:<8} {k[1]:<4} {100*n[k]/tot:5.1f}% of decisions  ants {len(ants[k]):4d}  stepped {100*mv[k]/n[k]:4.1f}%  "
          f"p_move median {statistics.median(pm[k]):.2f}  mound_top {100*zn[(k,'mound_top')]/n[k]:.0f}%")
