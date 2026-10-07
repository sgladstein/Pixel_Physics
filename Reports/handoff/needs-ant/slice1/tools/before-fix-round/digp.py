import csv, gzip, sys, collections, statistics
run = sys.argv[1]; a, b = 60000, 100000
s = collections.defaultdict(list); won = collections.Counter(); n = collections.Counter(); cut = collections.Counter(); ahead = collections.defaultdict(collections.Counter)
with gzip.open(f"{run}/digrows.csv.gz", "rt") as fh:
    for r in csv.DictReader(fh):
        f = int(r["frame"])
        if f < a or f > b or r["hold"] != "0": continue
        z = r["zone"]
        if z not in ("mound_in", "nest", "food"): continue
        try: p = float(r["dig_p"])
        except: p = float("nan")
        s[z].append(p); n[z] += 1
        if r["dig"] not in ("roll_lost", "not_asked", "lean", ""): won[z] += 1
        if r["dig"] == "cut": cut[z] += 1
        ahead[z][r["ahead"]] += 1
for z in s:
    v = [x for x in s[z] if x == x]
    v.sort()
    print(f"{run} {z:>9}: n {n[z]}, dig_p median {v[len(v)//2]:.3f} p90 {v[int(len(v)*.9)]:.3f} mean {sum(v)/len(v):.3f}; roll won {100*won[z]/n[z]:.1f}%, cut given won {100*cut[z]/max(won[z],1):.0f}%; ahead: " + ", ".join(f"{k} {100*c/n[z]:.0f}%" for k, c in ahead[z].most_common(4)))
