#!/usr/bin/env python3
"""cutrate.py RUN [from] [to]: from a dig=1 record, per zone: decisions with empty jaws, cuts made, cuts per 1,000
empty-jawed decisions, and the share of those decisions that stepped; plus the cut's dig funnel (how far each
empty-jawed decision's dig got)."""
import csv, gzip, sys, collections
run = sys.argv[1]
a = int(sys.argv[2]) if len(sys.argv) > 2 else 50000
b = int(sys.argv[3]) if len(sys.argv) > 3 else 100000
dec = collections.Counter(); cut = collections.Counter(); moved = collections.Counter(); why = collections.defaultdict(collections.Counter)
hold = collections.Counter()
with gzip.open(f"{run}/digrows.csv.gz", "rt") as fh:
    for r in csv.DictReader(fh):
        f = int(r["frame"])
        if f < a or f > b: continue
        z = r["zone"]
        hold[(z, r["hold"])] += 1
        if r["hold"] != "0": continue
        dec[z] += 1
        moved[z] += r["moved"] == "1"
        why[z][r["dig"]] += 1
        if r["dig"] == "cut": cut[z] += 1
print(f"{run} {a}-{b}")
print(f"{'zone':>10} {'empty dec':>10} {'cuts':>6} {'per 1k':>7} {'stepped':>7}  dig funnel (top)")
for z in sorted(dec, key=lambda z: -dec[z]):
    top = ", ".join(f"{k} {100*v/dec[z]:.0f}%" for k, v in why[z].most_common(5))
    print(f"{z:>10} {dec[z]:>10} {cut[z]:>6} {1000*cut[z]/max(dec[z],1):>7.2f} {100*moved[z]/max(dec[z],1):>6.0f}%  {top}")
print("decisions by zone and hold:")
for (z, h), v in sorted(hold.items(), key=lambda t: -t[1])[:14]:
    print(f"  {z:>10} {h:>8} {v}")
