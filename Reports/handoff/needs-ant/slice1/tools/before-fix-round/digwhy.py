import csv, gzip, sys, collections
run = sys.argv[1]; a, b = 60000, 100000
c = collections.defaultdict(collections.Counter); fl = collections.Counter(); en = collections.defaultdict(list)
with gzip.open(f"{run}/digrows.csv.gz", "rt") as fh:
    for r in csv.DictReader(fh):
        f = int(r["frame"])
        if f < a or f > b or r["hold"] != "0": continue
        z = r["zone"]
        if z not in ("mound_in", "nest", "food"): continue
        c[z][r["dig"]] += 1
        if r["dig"] == "cut":
            fl[(z, r["dig_flags"])] += 1
            try: en[z].append(float(r["energy"]))
            except: pass
for z in c:
    print(run, z, dict(c[z].most_common()))
print("cut flags:", dict(fl.most_common(10)))
for z, v in en.items():
    v.sort(); print(f"  {z} cutters' energy share: p10 {v[len(v)//10]:.2f} median {v[len(v)//2]:.2f}")
