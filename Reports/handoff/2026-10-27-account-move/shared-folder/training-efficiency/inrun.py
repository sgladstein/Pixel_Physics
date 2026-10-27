"""In-run (per-seed job) batches: per-arm food per colony-round, extinction, paired noise + subsample agreement."""
import glob, os, re, collections, itertools, random, statistics as st, json
from lib import D, call, binom_p
arms = collections.defaultdict(dict)   # (batch, arm) -> seed -> {round: food}, extinct flag
for p in glob.glob(f"{D}/*/*.inrun.csv"):
    b = p.split("/")[-2]; job = os.path.basename(p)[:-10]
    m = re.match(r"(.*)_(\d+)$", job)
    if not m: continue
    food, ext, nest = {}, False, {}
    for line in open(p).read().splitlines()[1:]:
        r, f, vals = line.split(",", 2)
        if f == "EXTINCT": ext = True; continue
        kv = dict(x.split("=", 1) for x in vals.split(";"))
        if f == "nest":
            food[int(r)] = float(kv["food_home"]); nest[int(r)] = float(kv["nest_score"])
    arms[(b, m.group(1))][int(m.group(2))] = dict(food=food, ext=ext, nest=nest)
batches = collections.defaultdict(list)
for (b, a), seeds in arms.items():
    if len(seeds) >= 32 and any(len(s["food"]) > 1 for s in seeds.values()):
        batches[b].append(a)
rng = random.Random(2)
out = {"pairs": [], "sub": collections.defaultdict(collections.Counter)}
NS = [8, 12, 16, 24, 32, 48]
for b, al in sorted(batches.items()):
    for x, y in itertools.combinations(sorted(al), 2):
        A, B = arms[(b, x)], arms[(b, y)]
        seeds = sorted(set(A) & set(B))
        d = []; va = []; vb = []
        for s in seeds:
            rr = sorted(set(A[s]["food"]) & set(B[s]["food"]))
            if not rr: continue
            fa = st.mean(A[s]["food"][r] for r in rr); fb = st.mean(B[s]["food"][r] for r in rr)
            d.append(fa - fb); va.append(fa); vb.append(fb)
        if len(d) < 32: continue
        full, p = call(d)
        ea = sum(A[s]["ext"] for s in seeds); eb = sum(B[s]["ext"] for s in seeds)
        disc_a = sum(A[s]["ext"] and not B[s]["ext"] for s in seeds); disc_b = sum(B[s]["ext"] and not A[s]["ext"] for s in seeds)
        pe = binom_p(disc_a, disc_a + disc_b)
        sdd = st.pstdev(d); mu = st.mean(d)
        out["pairs"].append(dict(batch=b, a=x, b=y, n=len(d), mean_diff=mu, sd_diff=sdd, dz=mu / sdd if sdd else 0, corr=st.correlation(va, vb) if len(set(va)) > 1 and len(set(vb)) > 1 else float('nan'), sd_a=st.pstdev(va), sd_b=st.pstdev(vb), mean_a=st.mean(va), mean_b=st.mean(vb),
            wins=sum(v > 0 for v in d), losses=sum(v < 0 for v in d), call=full, p=p, ext_a=ea, ext_b=eb, p_ext=pe))
        for N in NS:
            if N >= len(d): continue
            for _ in range(400):
                c, _ = call(rng.sample(d, N))
                out["sub"][f"{N}|{full}"][c] += 1
out["sub"] = {k: dict(v) for k, v in out["sub"].items()}
json.dump(out, open("inrun.json", "w"), indent=0)
for r in out["pairs"]:
    print(f"{r['batch']:16s} {r['a']:12s} vs {r['b']:12s} n{r['n']} food diff {r['mean_diff']:8.0f} sd {r['sd_diff']:7.0f} dz {r['dz']:+.2f} {r['wins']}/{r['losses']} call {r['call']:+d} p {r['p']:.3f} | extinct {r['ext_a']} vs {r['ext_b']} p {r['p_ext']:.2f}")
