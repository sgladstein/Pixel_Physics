"""identity.py A B: compare two deeptrace runs' stats.csv on their common columns and frames.

The identity check of the shared baselines (deep-trace/baseline/README.md, step 3): your build with your
switch off must reproduce the baseline's stats.csv row for row. Prints the frames and columns compared and
how many cells differ, with the first difference. A row still being written (a run in progress) has missing
cells; it is left out and counted, never reported as a difference.
"""
import csv, sys

def rows(d):
    out, partial = {}, 0
    for r in csv.DictReader(open(d + '/stats.csv')):
        if None in r.values() or None in r:
            partial += 1
            continue
        out[r['frame']] = r
    return out, partial

a, pa = rows(sys.argv[1])
b, pb = rows(sys.argv[2])
cols = [k for k in next(iter(a.values())) if k in next(iter(b.values())) and k != 'frame']
fr = sorted(set(a) & set(b), key=int)
diff = [(f, k, a[f][k], b[f][k]) for f in fr for k in cols if a[f][k] != b[f][k]]
only = sorted(set(next(iter(a.values()))) ^ set(next(iter(b.values()))))
name = lambda p: '/'.join(p.rstrip('/').split('/')[-2:])
print(f"{name(sys.argv[1])} vs {name(sys.argv[2])}: {len(fr)} frames x {len(cols)} common columns, "
      f"{len(diff)} cells differ" + (f"; first {diff[0]}" if diff else "")
      + (f"; columns in one only: {only}" if only else "")
      + (f"; partial rows left out: {pa}/{pb}" if pa or pb else ""))
sys.exit(1 if diff else 0)
