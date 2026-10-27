"""SCRATCH: does the score see the brain? Same 24 mutants of copy v2, one ant each, in N colonies.
usage: python3 -I mutcheck.py mut d25 8"""
import sys; sys.path.insert(0, ".")
import ledger, numpy as np
D, P, N = sys.argv[1], sys.argv[2], int(sys.argv[3])
W = dict(deliver=1.0, dig=1.0, clear=1.0, eat=0.0)
runs = [ledger.ledger(f"{D}/{P}_{s}", w=W) for s in range(1, N + 1)]
tags = sorted(set.intersection(*[set(r["tag"] for r in rows) for rows in runs]))
def mat(key, centre=True):
    M = np.array([[next(r[key] for r in rows if r["tag"] == t) for rows in runs] for t in tags], float)
    return M - M.mean(0, keepdims=True) if centre else M
S = mat("score")
L = mat("life", False)
print(f"{P}: {len(tags)} genomes in all {N} colonies; colony mean score {np.round(mat('score', False).mean(0), 2)}; alive at end {np.round((L >= L.max()).mean(0), 2)}")
rk = lambda a: np.argsort(np.argsort(a))
def split(M, rng=None):
    idx = np.arange(M.shape[1]) if rng is None else rng.permutation(M.shape[1])
    a, b = M[:, idx[: N // 2]].mean(1), M[:, idx[N // 2:]].mean(1)
    return np.corrcoef(a, b)[0, 1], np.corrcoef(rk(a), rk(b))[0, 1]
r, s = split(S)
print(f"  split half (colonies 1-{N//2} vs {N//2+1}-{N}): pearson {r:.2f} spearman {s:.2f}")
rng = np.random.default_rng(0)
many = np.array([split(S, rng) for _ in range(200)])
print(f"  over 200 random splits: pearson mean {many[:,0].mean():.2f}, spearman mean {many[:,1].mean():.2f}")
# one-way: genome effect vs residual, with a permutation null (shuffle tags within each colony)
F = lambda M: M.mean(1).var(ddof=1) * M.shape[1] / M.var(1, ddof=1).mean()
f0 = F(S)
null = []
for _ in range(2000):
    Q = np.array([rng.permutation(S[:, j]) for j in range(N)]).T
    null.append(F(Q))
print(f"  genome effect F = {f0:.2f}, permutation p = {np.mean(np.array(null) >= f0):.3f}")
vb = max(0.0, (S.mean(1).var(ddof=1) - S.var(1, ddof=1).mean() / N))
print(f"  share of one ant's score variance that is its brain: {vb / (vb + S.var(1, ddof=1).mean()):.2f}")
m = S.mean(1); o = np.argsort(-m)
print("  best 5:", [(tags[i], round(m[i], 2)) for i in o[:5]], " worst 5:", [(tags[i], round(m[i], 2)) for i in o[-5:]])
print("  base m00 rank:", int(np.where(np.array(tags)[o] == "m00")[0][0]) + 1 if "m00" in tags else None, "of", len(tags))
for k in ["n_deliver", "n_dig", "n_clear"]:
    K = mat(k)
    print(f"  {k:9s} split-half spearman mean {np.array([split(K, rng)[1] for _ in range(200)]).mean():.2f}")
