#!/usr/bin/env python3
"""aheaddrive.py RUN [from] [to]: empty-jawed decisions in the mound's tunnels and the nest, by the walk's drive:
what the jaw faces, the dig urge, how far the dig got."""
import csv, gzip, sys, collections, statistics
run = sys.argv[1]; a = int(sys.argv[2]) if len(sys.argv) > 2 else 60000; b = int(sys.argv[3]) if len(sys.argv) > 3 else 100000
wf = gzip.open(f"{run}/walk.csv.gz", "rt"); dfh = gzip.open(f"{run}/digrows.csv.gz", "rt")
wr = csv.reader(wf); dr = csv.reader(dfh); wh = next(wr); dh = next(dr)
wi = {k: i for i, k in enumerate(wh)}; di = {k: i for i, k in enumerate(dh)}
ahead = collections.defaultdict(collections.Counter); dig = collections.defaultdict(collections.Counter); urge = collections.defaultdict(list); n = collections.Counter()
wbuf = {}; wnext = next(wr, None); cur = None
for row in dr:
    f = int(row[0])
    if f < a: continue
    if f > b: break
    if f != cur:
        cur = f; wbuf = {}
        while wnext is not None and int(wnext[0]) <= f + 1:
            if int(wnext[0]) == f + 1: wbuf[wnext[1]] = wnext[wi["drive"]]
            wnext = next(wr, None)
    d = wbuf.get(row[1])
    z = row[di["zone"]]
    if d is None or row[di["hold"]] != "0" or z not in ("mound_in", "nest"): continue
    k = (d, z); n[k] += 1
    ahead[k][row[di["ahead"]]] += 1; dig[k][row[di["dig"]]] += 1
    if len(urge[k]) < 200000:
        try: urge[k].append(float(row[di["dig_p"]]))
        except: pass
for k in sorted(n, key=lambda k: -n[k])[:10]:
    u = [x for x in urge[k] if x == x]
    print(f"{k[0]:>7} {k[1]:>9} n={n[k]:>7}: faces " + ", ".join(f"{a} {100*v/n[k]:.0f}%" for a, v in ahead[k].most_common(3)) + f"; dig urge mean {sum(u)/max(len(u),1):.2f}; " + ", ".join(f"{a} {100*v/n[k]:.1f}%" for a, v in dig[k].most_common(4)))
