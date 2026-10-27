import json, collections, statistics as st
n = json.load(open("noise.json"))
# round stability: for pairs with a final call (+/-1), share of pairs whose call at round r already equals final
by = collections.defaultdict(lambda: [0, 0, 0])  # round -> [same, nocall, opposite]
fr = collections.Counter()
for r in n["rounds"]:
    if r["final"] == 0 or r["final_round"] < 31: continue
    k = r["round"]
    by[k][0 if r["call"] == r["final"] else (1 if r["call"] == 0 else 2)] += 1
print("round: same/no-call/opposite as final call (colsel pairs with a final call, final round >= 31)")
for k in sorted(by):
    s, nc, o = by[k]; t = s + nc + o
    print(f"  r{k:2d}: {s/t:.2f} {nc/t:.2f} {o/t:.2f}  (n={t})")
# effect sizes of colsel pair-metrics
dz = []
for p in n["pairs"]:
    if p["sd_diff"] > 0:
        d = (p["mean_a"] - p["mean_b"]) / p["sd_diff"]
        dz.append((abs(d), p))
dz.sort(key=lambda x: -x[0])
bins = collections.Counter()
for d, p in dz:
    b = "<0.2" if d < .2 else "0.2-0.5" if d < .5 else "0.5-1" if d < 1 else ">=1"
    bins[(b, p["call"] != 0)] += 1
print("colsel |dz| bins (called / not):", {k: v for k, v in sorted(bins.items())})
# pairing benefit: corr between arms per colony
cs = [p["corr"] for p in n["pairs"] if p["corr"] == p["corr"]]
print("colony correlation between arms at final round: median", round(st.median(cs), 2), "IQR", [round(x, 2) for x in st.quantiles(cs, n=4)])
by_m = collections.defaultdict(list)
for p in n["pairs"]:
    if p["corr"] == p["corr"]: by_m[p["metric"]].append(p["corr"])
print({m: round(st.median(v), 2) for m, v in by_m.items()})
