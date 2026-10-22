"""Thin a sampled copy log: keep labelled rows, 1 in K unlabelled, and each kept row's predecessor (same ant) as settle=2."""
import sys, random
src, dst, K = sys.argv[1], sys.argv[2], int(sys.argv[3])
rng = random.Random(7)
lines = [l for l in open(src) if l.split(",", 2)[1:2] == ["Q"]]
keep = [False] * len(lines); prev = {}; pidx = [-1] * len(lines)
for i, l in enumerate(lines):
    p = l.rstrip("\n").split(",")
    key = (p[0], p[3]); pidx[i] = prev.get(key, -1); prev[key] = i
    if p[6] == "0" and ((len(p) > 15 and p[15] != "-") or rng.random() < 1 / K):
        keep[i] = True
ctx = set(pidx[i] for i in range(len(lines)) if keep[i] and pidx[i] >= 0)
with open(dst, "w") as f:
    for i, l in enumerate(lines):
        if keep[i]:
            f.write(l)
        elif i in ctx:
            p = l.split(","); p[6] = "2"; f.write(",".join(p))
print(dst, sum(keep), len(ctx))
