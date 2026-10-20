"""Part 4 (selection reliability) and part 5 (racing, offline) from reliab CSVs.

usage: reliab.py CSV [CSV...]
Set 0 is used to select, set 1 to judge (independent worlds). Metric: full loads (and soil score).
"""
import sys, csv, random, statistics as st, collections

def load(p):
    d = collections.defaultdict(lambda: collections.defaultdict(dict))
    for r in csv.DictReader(open(p)):
        d[r["file"]][int(r["set"])][int(r["world"])] = (float(r["score"]), float(r["full_loads"]))
    return d

def ranks(v):
    o = sorted(range(len(v)), key=lambda i: v[i]); rk = [0.0] * len(v); i = 0
    while i < len(o):
        j = i
        while j + 1 < len(o) and v[o[j + 1]] == v[o[i]]: j += 1
        for k in range(i, j + 1): rk[o[k]] = (i + j) / 2
        i = j + 1
    return rk

def spear(a, b):
    ra, rb = ranks(a), ranks(b); n = len(a); ma, mb = sum(ra) / n, sum(rb) / n
    c = sum((x - ma) * (y - mb) for x, y in zip(ra, rb)); va = sum((x - ma) ** 2 for x in ra) ** .5; vb = sum((y - mb) ** 2 for y in rb) ** .5
    return c / (va * vb) if va and vb else float("nan")

def fold(vals):  # worse side of each mirrored pair (worlds 2i, 2i+1), as training scores
    return [min(vals[i], vals[i + 1]) for i in range(0, len(vals) - 1, 2)]

for p in sys.argv[1:]:
    d = load(p); brains = sorted(d)
    n_w = len(d[brains[0]][0])
    keycount = sum(len(d[b][s]) for b in brains for s in (0, 1))
    print(f"\n=== {p.rsplit('/', 1)[-1]}: {len(brains)} brains x 2 sets x {n_w} worlds = {keycount} rows (expect {len(brains) * 2 * n_w})")
    for mi, mname in ((1, "full loads"), (0, "soil score")):
        A = {b: [d[b][0][w][mi] for w in range(n_w)] for b in brains}
        B = {b: [d[b][1][w][mi] for w in range(n_w)] for b in brains}
        meanA = [st.mean(A[b]) for b in brains]; meanB = [st.mean(B[b]) for b in brains]
        foldA = [st.mean(fold(A[b])) for b in brains]
        # variance split: between-brain spread of true means vs per-world noise
        within = st.mean(st.pvariance(A[b] + B[b]) for b in brains)
        tot_means = [st.mean(A[b] + B[b]) for b in brains]
        between = max(st.pvariance(tot_means) - within / (2 * n_w), 0.0)
        rho = spear(meanA, meanB)
        k = len(brains) // 4
        topA = set(sorted(range(len(brains)), key=lambda i: -meanA[i])[:k]); topB = set(sorted(range(len(brains)), key=lambda i: -meanB[i])[:k])
        need = lambda r: within / between * r / (1 - r) if between > 0 else float("inf")   # worlds so that reliability of a mean = r
        print(f"  {mname}: mean {st.mean(tot_means):.2f}; spread between brains (sd) {between ** .5:.2f}; per-world noise (sd) {within ** .5:.2f}")
        print(f"     rank agreement between the two 48-world sets: {rho:+.2f} (worse-side fold as trained: {spear(foldA, meanB):+.2f}); top-{k} shared {len(topA & topB)}/{k}")
        print(f"     worlds needed for test reliability 0.5 / 0.7 / 0.9: {need(.5):.0f} / {need(.7):.0f} / {need(.9):.0f}")
    # ---- part 5, offline: how well does each scheme pick parents, per world-evaluation spent?
    A = {b: [d[b][0][w][1] for w in range(n_w)] for b in brains}
    truth = {b: st.mean(d[b][1][w][1] for w in range(n_w)) for b in brains}
    best_possible = st.mean(sorted(truth.values(), reverse=True)[:len(brains) // 4])
    rnd = random.Random(7); k = len(brains) // 4
    def judge(sel): return st.mean(truth[b] for b in sel)
    print(f"  SELECTION SCHEMES (pick {k} of {len(brains)}; judged on the other set; best possible {best_possible:.2f}, random {st.mean(truth.values()):.2f})")
    for name, scheme in (("uniform 16 worlds", [(16, 1.0)]), ("uniform 24 worlds", [(24, 1.0)]), ("uniform 48 worlds", [(48, 1.0)]),
                         ("race 8 -> keep 50% -> +16 -> keep 50% -> +24", [(8, .5), (16, .5), (24, 1.0)]),
                         ("race 12 -> keep 33% -> +36", [(12, 1 / 3), (36, 1.0)])):
        vals = []; costs = []
        for rep in range(300):
            order = list(range(n_w)); rnd.shuffle(order)
            alive = list(brains); used = 0; scores = {b: [] for b in brains}; cost = 0
            for add, keep in scheme:
                ws = order[used: used + add]; used += add
                for b in alive:
                    scores[b] += [A[b][w] for w in ws]; cost += len(ws)
                alive = sorted(alive, key=lambda b: -st.mean(scores[b]))[: max(k, int(round(len(alive) * keep)))]
            vals.append(judge(alive[:k])); costs.append(cost / len(brains))
        print(f"     {name:46s} cost {st.mean(costs):5.1f} worlds/brain   picked parents' true full loads {st.mean(vals):.2f}")
