"""SCRATCH (teacher lane, 2026-10-22). Paired arm-vs-control from evreduce CSVs. usage: pairfood.py CTL_CSV:ARM ARM_CSV:ARM [...more ARM_CSV:ARM]
Per seed, over the rounds BOTH colonies reached: home food (deliveries minus the 13+ column ones) and starved per colony-round.
Extinct = fewer than ROUNDS rounds, or nobody alive at the end of the last. Exact McNemar on extinction; on food, a sign test
and the mean paired difference with a 95% bootstrap interval over seeds (fixed RNG seed, so the output repeats)."""
import sys, csv, math, collections, random
ROUNDS = 6
def load(spec):
    f, arm = spec.rsplit(':', 1); d = collections.defaultdict(dict)
    for r in csv.DictReader(open(f)):
        a, s = r['job'].rsplit('_', 1)
        if a == arm: d[int(s)][int(r['round'])] = r
    return d
def ext(rs): return len(rs) < ROUNDS or int(rs[max(rs)]['alive_end']) <= 0
def p2(k, n): return min(1, 2 * sum(math.comb(n, i) for i in range(min(k, n - k) + 1)) / 2 ** n) if n else 1
C = load(sys.argv[1]); cn = sys.argv[1].rsplit(':', 1)[1]
for spec in sys.argv[2:]:
    A = load(spec); an = spec.rsplit(':', 1)[1]; seeds = sorted(set(A) & set(C))
    ea = {s: ext(A[s]) for s in seeds}; ec = {s: ext(C[s]) for s in seeds}
    oa = sum(ea[s] and not ec[s] for s in seeds); oc = sum(ec[s] and not ea[s] for s in seeds)
    df, ds, win, lose, fa, fc, sa, sc, n = [], [], 0, 0, 0, 0, 0, 0, 0
    for s in seeds:
        common = sorted(set(A[s]) & set(C[s]))
        if not common: continue
        h = lambda R: sum(int(R[r]['deliver']) - int(R[r]['deliver_far']) for r in common) / len(common)
        st = lambda R: sum(int(R[r]['starved']) for r in common) / len(common)
        x, y = h(A[s]), h(C[s]); fa += x; fc += y; sa += st(A[s]); sc += st(C[s]); n += 1
        win += x > y; lose += x < y; df.append(x - y)
    rng = random.Random(1); bs = sorted(sum(rng.choice(df) for _ in df) / len(df) for _ in range(2000))
    print(f"{an:8s} vs {cn:6s} n={len(seeds)}: extinct {sum(ea.values()):2d} vs {sum(ec.values()):2d} (only-arm {oa}, only-ctl {oc}, p={p2(oa, oa + oc):.3f}) | "
          f"home food/colony-round {fa/n:6.0f} vs {fc/n:6.0f} (arm higher in {win}/{win+lose} seeds, p={p2(win, win + lose):.3f}; mean diff {sum(df)/len(df):+.0f}, 95% CI {bs[50]:+.0f} to {bs[1949]:+.0f}) | starved/colony-round {sa/n:5.0f} vs {sc/n:5.0f}")
