"""SCRATCH (teacher-training measurement 1, not for main): how correlated are a parent and its children on the same
worlds, and what does pairing save?  usage: python3 -I rho.py CSV PARENT PARENT_COPY [OTHER...]

Reads examples/reliab.rs output (file,set,world,k,score,full_loads). For every genome G against the parent P, over the
same worlds: rho (Pearson, per world), the mean difference, and the PAIRING FACTOR var(G - P) / var(G) -- the share of
worlds a race on paired differences needs against a well-measured parent, compared with raw scores (2(1 - rho) when the
two variances are equal; under 1 means pairing saves).
Controls: PARENT_COPY is the same genome under another name, so its rho must be exactly 1 and every difference exactly
0 (the run is deterministic and the pairing is right); the shuffled control pairs P and G on DIFFERENT worlds and must
read rho near 0 (the correlation is the worlds', not an artefact of the code)."""
import csv, random, statistics as st, sys
from collections import defaultdict

path, parent, copy = sys.argv[1], sys.argv[2], sys.argv[3]
others = sys.argv[4:]
rows = defaultdict(dict)
for r in csv.DictReader(open(path)):
    rows[r["file"]][(int(r["set"]), int(r["world"]))] = (float(r["score"]), float(r["full_loads"]))
keys = sorted(rows[parent])
print(f"genomes {len(rows)}, worlds per genome {[len(v) for v in rows.values()][:3]}..., parent worlds {len(keys)}")
assert all(len(v) == len(keys) for v in rows.values()), "a genome is missing worlds: the run did not finish"


def pear(a, b):
    ma, mb = st.fmean(a), st.fmean(b)
    sa, sb = st.pstdev(a), st.pstdev(b)
    return sum((x - ma) * (y - mb) for x, y in zip(a, b)) / (len(a) * sa * sb) if sa and sb else float("nan")


def stats(g, m):
    p = [rows[parent][k][m] for k in keys]
    c = [rows[g][k][m] for k in keys]
    d = [y - x for x, y in zip(p, c)]
    rnd = random.Random(7)
    shuf = []
    for _ in range(200):
        c2 = c[:]
        rnd.shuffle(c2)
        shuf.append(pear(p, c2))
    vd, vc = st.pvariance(d), st.pvariance(c)
    return dict(rho=pear(p, c), mean_p=st.fmean(p), mean_c=st.fmean(c), diff=st.fmean(d), se=st.pstdev(d) / len(d) ** 0.5,
                factor=vd / vc if vc else float("nan"), shuf=st.fmean(shuf), maxdiff=max(abs(x) for x in d))


for m, name in ((0, "dig score"), (1, "full loads")):
    print(f"\n=== {name} (parent {parent}) ===")
    cp = stats(copy, m)
    print(f"POSITIVE CONTROL  parent vs its copy: rho {cp['rho']:.6f}, largest |difference| {cp['maxdiff']:.6g} (must be 1 and 0)")
    gs = [g for g in rows if g not in (parent, copy)]
    res = {g: stats(g, m) for g in gs}
    print(f"NEGATIVE CONTROL  shuffled worlds: mean rho {st.fmean(r['shuf'] for r in res.values()):+.3f} (must be near 0)")
    print(f"{'genome':58s} {'mean':>6s} {'diff':>7s} {'se':>5s} {'rho':>6s} {'factor':>6s}")
    for g in sorted(gs, key=lambda g: (g not in others, -res[g]['diff'])):
        r = res[g]
        print(f"{g[-58:]:58s} {r['mean_c']:6.2f} {r['diff']:+7.2f} {r['se']:5.2f} {r['rho']:6.3f} {r['factor']:6.3f}")
    kids = [g for g in gs if g not in others]
    near = [g for g in kids if res[g]["diff"] > -2 * res[g]["se"]]
    worse = [g for g in kids if g not in near]
    for label, grp in (("children within 2 s.e. of the parent or above", near), ("children clearly worse", worse), ("all children", kids)):
        if grp:
            print(f"  {label:46s} n={len(grp):2d}  median rho {st.median(res[g]['rho'] for g in grp):.3f}  "
                  f"median factor {st.median(res[g]['factor'] for g in grp):.3f}")
