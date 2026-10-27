"""Colony selection batches: spread, pairing benefit, subsample agreement, round stability. usage: noise.py"""
import itertools, random, statistics as st, json, collections
from lib import *
arms = colsel_arms()
cg = {k: load_cg(p) for k, p in arms.items()}
batches = collections.defaultdict(list)
for (b, a) in arms: batches[b].append(a)
pairs = [(b, x, y) for b, al in batches.items() for x, y in itertools.combinations(sorted(al), 2)]
res = {"pairs": [], "spread": [], "sub": {}, "rounds": []}
NS = [8, 12, 16, 24, 32, 48]
rng = random.Random(1)
sub = collections.defaultdict(lambda: collections.Counter())
for b, x, y in pairs:
    A, B = cg[(b, x)], cg[(b, y)]
    common_rounds = sorted(set(A) & set(B))
    if not common_rounds: continue
    R = common_rounds[-1]
    for m in METRICS:
        cols = sorted(c for c in set(A[R]) & set(B[R]) if m in A[R][c] and m in B[R][c])
        if len(cols) < 40: continue
        sgn = HIGHER_BETTER[m]
        d = [sgn * (A[R][c][m] - B[R][c][m]) for c in cols]
        va = [A[R][c][m] for c in cols]; vb = [B[R][c][m] for c in cols]
        full, p = call(d)
        sda, sdb = st.pstdev(va), st.pstdev(vb)
        sdd = st.pstdev(d)
        try: corr = st.correlation(va, vb)
        except Exception: corr = float("nan")
        res["pairs"].append(dict(batch=b, a=x, b=y, metric=m, round=R, n=len(cols), med_a=st.median(va), med_b=st.median(vb),
            mean_a=st.mean(va), mean_b=st.mean(vb), sd_a=sda, sd_b=sdb, sd_diff=sdd, corr=corr,
            wins=sum(v > 0 for v in d), losses=sum(v < 0 for v in d), call=full, p=p))
        for N in NS:
            if N >= len(cols): continue
            for _ in range(400):
                s = rng.sample(d, N)
                c, _p = call(s)
                sub[(N, full)][c] += 1
        # round stability: call at each earlier common round vs final
        for r in common_rounds:
            cc = sorted(c for c in set(A[r]) & set(B[r]) if m in A[r][c] and m in B[r][c])
            if len(cc) < 20: continue
            c, _ = call([sgn * (A[r][k][m] - B[r][k][m]) for k in cc])
            res["rounds"].append(dict(batch=b, a=x, b=y, metric=m, round=r, final_round=R, call=c, final=full))
res["sub"] = {f"{N}|{f}": dict(cnt) for (N, f), cnt in sub.items()}
json.dump(res, open("noise.json", "w"), indent=0)
print(len(res["pairs"]), "pair-metrics")
for N in NS:
    for f in (1, -1, 0):
        cnt = sub.get((N, f))
        if not cnt: continue
        tot = sum(cnt.values())
        print(f"N={N:2d} full call {f:+d}: same {cnt[f]/tot:.2f}  no-call {cnt[0]/tot:.2f}  opposite {cnt[-f]/tot if f else (cnt[1]+cnt[-1])/tot:.2f}  (n={tot//400})")
