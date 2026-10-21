import csv, sys, os, random

rows = list(csv.DictReader(open(sys.argv[1])))
for r in rows:
    r["name"] = os.path.basename(r["file"]).replace(".txt", "")
    for k in list(r):
        if k not in ("file", "name"):
            r[k] = float(r[k])

def ranks(v):
    o = sorted(range(len(v)), key=lambda i: v[i])
    rk = [0.0] * len(v)
    i = 0
    while i < len(o):
        j = i
        while j + 1 < len(o) and v[o[j + 1]] == v[o[i]]:
            j += 1
        for k in range(i, j + 1):
            rk[o[k]] = (i + j) / 2
        i = j + 1
    return rk

def spear(a, b):
    ra, rb = ranks(a), ranks(b)
    n = len(a); ma, mb = sum(ra) / n, sum(rb) / n
    cov = sum((x - ma) * (y - mb) for x, y in zip(ra, rb))
    va = sum((x - ma) ** 2 for x in ra) ** 0.5; vb = sum((y - mb) ** 2 for y in rb) ** 0.5
    return cov / (va * vb) if va and vb else float("nan")

def topk_overlap(a, b, k):
    ta = set(sorted(range(len(a)), key=lambda i: -a[i])[:k]); tb = set(sorted(range(len(b)), key=lambda i: -b[i])[:k])
    return len(ta & tb)

def boot(a, b, n=2000):
    rnd = random.Random(1); idx = range(len(a)); out = []
    for _ in range(n):
        s = [rnd.choice(idx) for _ in idx]
        out.append(spear([a[i] for i in s], [b[i] for i in s]))
    out.sort(); return out[int(0.05 * n)], out[int(0.95 * n)]

for label, sub in (("all 40", rows), ("round-2 pool only", [r for r in rows if r["name"].startswith("r2-")])):
    print(f"\n== {label} (n={len(sub)})")
    for m in ("score", "full"):
        col = lambda c: [r[f"{c}_{m}"] for r in sub]
        A, B, C1, C2 = col("A"), col("B"), col("C1"), col("C2")
        Cm = [(x + y) / 2 for x, y in zip(C1, C2)]
        k = max(1, len(sub) // 4)
        print(f"  {m}: mean A {sum(A)/len(A):.2f}  B {sum(B)/len(B):.2f}  C {sum(Cm)/len(Cm):.2f} (per ant, in a 4-box)")
        for nm, x, y in (("A vs B  solo sealed -> solo new worlds (reference)", A, B),
                         ("C1 vs B one 4-box pass -> solo new worlds", C1, B),
                         ("C1 vs A same worlds, alone vs boxed", C1, A),
                         ("C1 vs C2 two boxmate shuffles, same worlds", C1, C2),
                         ("Cmean vs B two 4-box passes -> solo new worlds", Cm, B)):
            lo, hi = boot(x, y)
            print(f"    {nm:52s} rho {spear(x, y):+.2f}  [90%: {lo:+.2f},{hi:+.2f}]  top-{k} shared {topk_overlap(x, y, k)}/{k}")
print("\nper brain (score A / B / C1 / C2, full loads A / C1):")
for r in sorted(rows, key=lambda r: -r["A_score"]):
    print(f"  {r['name']:28s} {r['A_score']:6.1f} {r['B_score']:6.1f} {r['C1_score']:6.1f} {r['C2_score']:6.1f}   {r['A_full']:.2f} {r['C1_full']:.2f}")
