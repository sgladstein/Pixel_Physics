"""surfpace.py RUN [FROM TO]: traced empty-jawed decisions by zone -- how many, share that stepped,
median step chance, and (walk) the drive/job mix; the per-ant spread of stepping on the surface."""
import sys, csv, gzip, collections, os, statistics
run = sys.argv[1]
a = int(sys.argv[2]) if len(sys.argv) > 2 else 80000
b = int(sys.argv[3]) if len(sys.argv) > 3 else 100000
walk = {}
if os.path.exists(f"{run}/walk.csv.gz"):
    for r in csv.DictReader(gzip.open(f"{run}/walk.csv.gz", "rt")):
        f = int(r["frame"])
        if a <= f <= b + 1:
            walk[(f, r["id"])] = f"{r['drive']}/{r['job']}"
n = collections.Counter(); mv = collections.Counter(); pm = collections.defaultdict(list); dj = collections.defaultdict(collections.Counter)
per = collections.defaultdict(lambda: [0, 0])
for r in csv.DictReader(gzip.open(f"{run}/ticks.csv.gz", "rt")):
    f = int(r["frame"])
    if not (a <= f <= b): continue
    if r["spoil"] not in ("0", "") or r["crop_cells"] not in ("0", ""): continue
    z = r["zone"]
    n[z] += 1; mv[z] += r["moved"] == "1"
    try: pm[z].append(float(r["p_move"]))
    except ValueError: pass
    dj[z][walk.get((f + 1, r["id"]), "-")] += 1
    if z == "surface":
        per[r["id"]][0] += 1; per[r["id"]][1] += r["moved"] == "1"
print(f"{run} {a}-{b}: empty-jawed decisions")
for z in sorted(n, key=lambda z: -n[z]):
    top = ", ".join(f"{k} {100*v/n[z]:.0f}%" for k, v in dj[z].most_common(3))
    print(f"  {z:<10} n={n[z]:>6} stepped {100*mv[z]/n[z]:>3.0f}% p_move median {statistics.median(pm[z]) if pm[z] else float('nan'):.2f}  {top}")
sp = sorted((v[1] / v[0], v[0], k) for k, v in per.items() if v[0] >= 50)
print("  surface, per ant (>=50 decisions): stepped share " + " ".join(f"{s:.2f}({c})" for s, c, _ in sp))
