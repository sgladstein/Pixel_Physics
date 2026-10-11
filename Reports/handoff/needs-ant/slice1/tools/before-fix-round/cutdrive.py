#!/usr/bin/env python3
"""cutdrive.py RUN [from] [to]: every decision of every ant in a dig=1 + walktrace=1 record, joined with the walk's
drive and job at that decision (walk.csv frame = dig row frame + 1). Per (drive, zone): decisions by what the ant
held, cuts, cuts per 1,000 empty-jawed decisions."""
import csv, gzip, sys, collections
run = sys.argv[1]
a = int(sys.argv[2]) if len(sys.argv) > 2 else 60000
b = int(sys.argv[3]) if len(sys.argv) > 3 else 100000
wf = gzip.open(f"{run}/walk.csv.gz", "rt"); dfh = gzip.open(f"{run}/digrows.csv.gz", "rt")
wr = csv.reader(wf); dr = csv.reader(dfh)
wh = next(wr); dh = next(dr)
wi = {k: i for i, k in enumerate(wh)}; di = {k: i for i, k in enumerate(dh)}
dec = collections.Counter(); emp = collections.Counter(); cuts = collections.Counter(); miss = 0
wbuf = {}; wnext = next(wr, None)
cur = None
for row in dr:
    f = int(row[0])
    if f < a: continue
    if f > b: break
    if f != cur:
        cur = f; wbuf = {}
        # walk rows for this decision frame are stamped f+1
        while wnext is not None and int(wnext[0]) <= f + 1:
            if int(wnext[0]) == f + 1:
                wbuf[wnext[1]] = (wnext[wi["drive"]], wnext[wi["job"]])
            wnext = next(wr, None)
    w = wbuf.get(row[1])
    if w is None:
        miss += 1; continue
    z = row[di["zone"]]; h = row[di["hold"]]
    key = (w[0], z)
    dec[key] += 1
    if h == "0": emp[key] += 1
    if row[di["dig"]] == "cut": cuts[(w[0], w[1], z)] += 1
print(f"{run} {a}-{b}: dig rows without a walk row {miss}")
tot = sum(cuts.values())
print(f"cuts {tot} by drive/job/zone:")
for k, v in cuts.most_common(16):
    e = emp.get((k[0], k[2]), 0)
    print(f"  {k[0]:>7} {k[1]:>6} {k[2]:>9}: {v:>5} ({100*v/tot:4.1f}%)  empty-jawed decisions {e:>8}  per 1k {1000*v/max(e,1):5.2f}")
print("empty-jawed decisions by drive and zone (top):")
for k, v in emp.most_common(14):
    print(f"  {k[0]:>7} {k[1]:>9}: {v}")
