"""walkcounts.py RUN : the walk's running totals as per-10k deltas, and the census of drives and jobs
among live ants (walk_colony.csv) at each 10k."""
import sys, csv, collections
run = sys.argv[1]
rows = {int(r["frame"]): r for r in csv.DictReader(open(f"{run}/walk_counts.csv"))}
keys = [k for k in next(iter(rows.values())).keys() if k not in ("frame",)]
print(f"{'frame':>7} " + " ".join(f"{k.replace('dec_','')[:8]:>8}" for k in keys))
prev = None
for f in sorted(rows):
    if f % 10000:
        continue
    r = rows[f]
    if prev is not None:
        print(f"{f:>7} " + " ".join(f"{(float(r[k]) - (0 if k == 'minds' else float(prev[k]))):>8.0f}" for k in keys))
    prev = r
cen = collections.defaultdict(collections.Counter)
for r in csv.DictReader(open(f"{run}/walk_colony.csv")):
    f = int(r["frame"])
    if f % 10000 == 0:
        cen[f][("d", r["drive"])] += 1
        cen[f][("j", r["job"])] += 1
drives = ["eat", "out", "escape", "seek", "forage", "carry", "haul", "dig", "home", "rest"]
jobs = ["idle", "forage", "haul", "dig"]
print(f"\n{'frame':>7} " + " ".join(f"{d:>7}" for d in drives) + " | " + " ".join(f"{j:>7}" for j in jobs))
for f in sorted(cen):
    c = cen[f]
    print(f"{f:>7} " + " ".join(f"{c[('d', d)]:>7}" for d in drives) + " | " + " ".join(f"{c[('j', j)]:>7}" for j in jobs))
