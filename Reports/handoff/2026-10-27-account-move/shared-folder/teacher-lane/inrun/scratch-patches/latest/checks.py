"""SCRATCH: the score checks over chk3 (mixed v2/s171/zero colonies and all-v2 clone colonies)."""
import sys; sys.path.insert(0, ".")
import ledger, numpy as np
D = sys.argv[1] if len(sys.argv) > 1 else "chk3"
N = int(sys.argv[2]) if len(sys.argv) > 2 else 8
W = dict(deliver=1.0, dig=1.0, clear=1.0, eat=0.0)
mix = {s: ledger.ledger(f"{D}/mix{s}", w=W) for s in range(1, N + 1)}
cl = {s: ledger.ledger(f"{D}/v2_{s}", w=W) for s in range(1, N + 1)}
m = lambda rows, t: np.mean([r["score"] for r in rows if r["tag"] == t]) if any(r["tag"] == t for r in rows) else np.nan
print("placed per colony:", {t: sum(r["tag"] == t for r in mix[1]) for t in ["v2", "s171", "zero"]})
for t in ["v2", "s171", "zero"]:
    pr = [m(mix[s], t) for s in mix]
    print(f"mixed  {t:5s} mean score by colony {np.round(pr, 2)}  mean {np.nanmean(pr):.2f}")
d = np.array([m(mix[s], "v2") - m(mix[s], "s171") for s in mix])
print(f"2) v2 minus s171, paired by colony: {np.round(d, 2)}  mean {d.mean():.2f} +- {d.std(ddof=1) / np.sqrt(len(d)):.2f}")
z = np.array([m(mix[s], "s171") - m(mix[s], "zero") for s in mix])
print(f"3) s171 minus random brain: mean {z.mean():.2f} +- {z.std(ddof=1) / np.sqrt(len(z)):.2f}; random-brain ants' share ranked last in their colony: "
      f"{np.mean([np.mean([q['score'] >= r['score'] for q in mix[s]]) for s in mix for r in mix[s] if r['tag'] == 'zero']):.2f} (1.00 = bottom)")
sd = [np.std([r["score"] for r in cl[s]]) for s in cl]
mu = [np.mean([r["score"] for r in cl[s]]) for s in cl]
print(f"1) noise floor: clones' score sd within a colony {np.round(sd, 2)} (mean {np.mean(sd):.2f}) around colony means {np.round(mu, 2)}")
print(f"   so one ant's score separates v2 from s171 by {abs(d.mean()) / np.mean(sd):.2f} sd")
rows = [r for s in mix for r in mix[s]] + [r for s in cl for r in cl[s]]
lg = [r for r in rows if r["life"] >= 30000 and r["tag"] != "zero"]
h1 = np.array([r["h1"] for r in lg]); h2 = np.array([r["h2"] for r in lg])
rk = lambda a: np.argsort(np.argsort(a))
print(f"6) repeat: an ant's first-half score vs its second half, n={len(lg)}: pearson {np.corrcoef(h1, h2)[0, 1]:.2f}, spearman {np.corrcoef(rk(h1), rk(h2))[0, 1]:.2f}")
real = [r for r in rows if r["tag"] != "zero"]
print(f"5) pairs: share of pellets carried out by an ant other than the cutter: {sum(r['n_pair'] for r in real) / max(1, sum(r['n_clear'] for r in real)):.3f}")
print(f"   events per ant over 60k frames: deliver {np.mean([r['n_deliver'] for r in real]):.1f}  dig {np.mean([r['n_dig'] for r in real]):.1f}  clear {np.mean([r['n_clear'] for r in real]):.1f}")
print(f"   share of ants with no event at all: {np.mean([r['n_deliver'] + r['n_dig'] + r['n_clear'] == 0 for r in real]):.2f}")
