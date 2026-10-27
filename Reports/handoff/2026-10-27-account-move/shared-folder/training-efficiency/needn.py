"""Per pair-metric with a 64-colony call: smallest N (from a grid) at which >=80% of random N-colony subsets give the same call; split by effect size."""
import json, random, collections, csv
from lib import *
rng = random.Random(3)
arms = colsel_arms(); cg = {k: load_cg(p) for k, p in arms.items()}
batches = collections.defaultdict(list)
for (b, a) in arms: batches[b].append(a)
import itertools
GRID = [4, 6, 8, 10, 12, 16, 20, 24, 32, 40, 48, 56]
rows = []
bybin = collections.defaultdict(lambda: collections.defaultdict(lambda: [0, 0, 0]))
for b, al in batches.items():
    for x, y in itertools.combinations(sorted(al), 2):
        A, B = cg[(b, x)], cg[(b, y)]
        R = max(set(A) & set(B))
        for m in METRICS:
            cols = [c for c in set(A[R]) & set(B[R]) if m in A[R][c] and m in B[R][c]]
            if len(cols) < 48: continue
            d = [HIGHER_BETTER[m] * (A[R][c][m] - B[R][c][m]) for c in cols]
            full, p = call(d)
            sd = st.pstdev(d); dz = abs(st.mean(d)) / sd if sd else 0
            bn = "small (<0.5 sd)" if dz < .5 else ("medium (0.5-1 sd)" if dz < 1 else "large (>1 sd)")
            need = None
            for N in GRID:
                if N >= len(cols): break
                same = opp = 0
                for _ in range(300):
                    c, _ = call(rng.sample(d, N))
                    same += c == full; opp += (c == -full) and full != 0
                if full != 0:
                    bybin[bn][N][0] += same; bybin[bn][N][1] += opp; bybin[bn][N][2] += 300
                if need is None and full != 0 and same / 300 >= .8: need = N
            rows.append(dict(batch=b, a=x, b=y, metric=m, dz=dz, call=full, need=need, n=len(cols)))
json.dump(dict(rows=rows, bybin={k: {n: v for n, v in d.items()} for k, d in bybin.items()}), open("needn.json", "w"))
for k in sorted(bybin):
    print(k, " ".join(f"N{n}:{v[0]/v[2]:.2f}/{v[1]/v[2]:.3f}" for n, v in sorted(bybin[k].items())))
c = collections.Counter((r["need"] if r["need"] else ">=64") for r in rows if r["call"] != 0)
print("smallest N with >=80% same call, over called pair-metrics:", sorted(c.items(), key=lambda x: str(x[0]).zfill(3)))
