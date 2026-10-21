"""SCRATCH (teacher design v2, step 0.5 item 3; not for main): the CONTEXT probe -- can the brain tell the states a
lesson is taught on from the kept-skill states that look like them?

usage: python3 -I contextprobe.py SOIL_LOG FETCH_LOG [FOLDS=4]

Case: walk home. Taught states = soil-arena decisions the teacher labelled "home" (empty, on the surface, 3+ columns
from the mouth, chosen Walk). Look-alikes = fetch-arena decisions of an empty forager (no pellet, empty crop) choosing
Walk among 2+ Walk options -- the state "walk home" spread to and broke fetch on (step1-results). The question is
whether *anything every scorer unit sees* separates them: the 22 senses and 12 memory units (the shared context of
every option's score), and, as a wider check, those plus the mean Walk-option features.

Held out by world (folds); AUC (0.5 = cannot tell, 1 = always can). Bar from the design (§3): >= 0.85 through the
brain channel. Controls, each able to fail: POSITIVE -- the label replaced by a threshold on one sense (energy above
its median), must read ~1; NEGATIVE -- arena labels permuted across worlds, must read ~0.5.
"""
import sys, os
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from copier import parse_log  # noqa: E402

WALK = 0
CTX = ["energy", "crop", "spoil", "at_nest", "a", "b", "alarm", "d_a", "d_b", "d_alarm", "d_energy", "home_seen",
       "home_dist", "d_home", "since_nest", "age", "self", "crowd", "depth", "roofed", "roof_cut", "hole_drop"]


def walk_feats(r):
    f = []
    for o in r["opts"]:
        if o[0] == WALK:
            v = np.zeros(45)
            for i, x in o[4].items():
                v[int(i)] = x
            f.append(v)
    return np.mean(np.array(f), axis=0) if f else np.zeros(45)


def rows(soil, fetch):
    S, _ = parse_log(soil, arena="soil")
    F, _ = parse_log(fetch, arena="fetch")
    taught = [r for r in S if r["home"]]
    look = [r for r in F if not r["carrying"] and r["ctx"][1] == 0 and r["opts"][r["pick"]][0] == WALK
            and sum(o[0] == WALK for o in r["opts"]) >= 2]
    return taught, look


def auc(score, y):
    o = np.argsort(score)
    ranks = np.empty(len(score))
    ranks[o] = np.arange(1, len(score) + 1)
    pos = y == 1
    n1, n0 = pos.sum(), (~pos).sum()
    return (ranks[pos].sum() - n1 * (n1 + 1) / 2) / (n1 * n0) if n1 and n0 else float("nan")


def fit(X, y, hidden=0, steps=800, lr=0.05, l2=1e-3, seed=0):
    rng = np.random.default_rng(seed)
    d = X.shape[1]
    if hidden:
        W1 = rng.normal(0, 0.3, (d, hidden)); b1 = np.zeros(hidden); w2 = rng.normal(0, 0.3, hidden); b2 = 0.0
    else:
        w = np.zeros(d); b = 0.0
    sq = lambda z: z / (1 + np.abs(z))
    for _ in range(steps):
        if hidden:
            H = sq(X @ W1 + b1); z = H @ w2 + b2
        else:
            z = X @ w + b
        p = 1 / (1 + np.exp(-z)); g = (p - y) / len(y)
        if hidden:
            gw2 = H.T @ g + l2 * w2; gb2 = g.sum()
            dH = np.outer(g, w2) / (1 + np.abs(X @ W1 + b1)) ** 2
            W1 -= lr * (X.T @ dH + l2 * W1); b1 -= lr * dH.sum(0); w2 -= lr * gw2; b2 -= lr * gb2
        else:
            w -= lr * (X.T @ g + l2 * w); b -= lr * g.sum()
    if hidden:
        return lambda Z: sq(Z @ W1 + b1) @ w2 + b2
    return lambda Z: Z @ w + b, w


def heldout(X, y, groups, hidden, folds):
    ug = np.unique(groups)
    rng = np.random.default_rng(1)
    rng.shuffle(ug)
    s = np.zeros(len(y))
    for k in range(folds):
        te = np.isin(groups, ug[k::folds])
        mu, sd = X[~te].mean(0), X[~te].std(0) + 1e-9
        m = fit((X[~te] - mu) / sd, y[~te], hidden)
        f = m if hidden else m[0]
        s[te] = f((X[te] - mu) / sd)
    return auc(s, y)


def main():
    soil, fetch = sys.argv[1], sys.argv[2]
    folds = int(sys.argv[3]) if len(sys.argv) > 3 else 4
    taught, look = rows(soil, fetch)
    n = min(len(taught), len(look), 6000)
    rng = np.random.default_rng(0)
    taught = [taught[i] for i in rng.choice(len(taught), n, replace=False)]
    look = [look[i] for i in rng.choice(len(look), n, replace=False)]
    allr = taught + look
    y = np.array([1] * n + [0] * n)
    groups = np.array([f"{r['arena']}{r['tag']}" for r in allr])
    ctxmem = np.array([np.concatenate([r["ctx"], r["mem"]]) for r in allr])
    drop = [CTX.index(c) for c in os.environ.get("CP_DROP", "").split(",") if c]
    if drop:
        # an input pinned by the arena rather than by the world (fetch is fed: energy 0.500, d_energy 0, every frame)
        ctxmem[:, drop] = 0.0
        print(f"inputs held at 0 (arena artefacts): {[CTX[i] for i in drop]}")
    wide = np.concatenate([ctxmem, np.array([walk_feats(r) for r in allr])], axis=1)
    print(f"taught (soil, walk home) {n}  look-alike (fetch, empty forager walking) {n}  worlds {len(np.unique(groups))}")
    for name, X in (("senses + memory (every scorer unit sees these)", ctxmem), ("... + mean Walk-option features", wide)):
        print(f"  {name:50s} linear AUC {heldout(X, y, groups, 0, folds):.3f}   mlp16 AUC {heldout(X, y, groups, 16, folds):.3f}")
    # which inputs separate them (linear, all data, standardised)
    mu, sd = ctxmem.mean(0), ctxmem.std(0) + 1e-9
    _, w = fit((ctxmem - mu) / sd, y)
    names = CTX + [f"mem{i}" for i in range(12)]
    top = np.argsort(-np.abs(w))[:6]
    print("  separating inputs (standardised weight; + means soil):", ", ".join(f"{names[i]} {w[i]:+.2f}" for i in top))
    for i in top[:4]:
        print(f"     {names[i]:10s} soil mean {ctxmem[y == 1, i].mean():+.3f}  fetch mean {ctxmem[y == 0, i].mean():+.3f}")
    # controls
    pos = (ctxmem[:, 0] > np.median(ctxmem[:, 0])).astype(int)
    print(f"  POSITIVE control (energy above its median):    linear AUC {heldout(ctxmem, pos, groups, 0, folds):.3f}  (expect ~1)")
    ug = np.unique(groups)
    perm = dict(zip(ug, rng.permutation([g.startswith('soil') for g in ug])))
    neg = np.array([int(perm[g]) for g in groups])
    print(f"  NEGATIVE control (arena labels permuted by world): linear AUC {heldout(ctxmem, neg, groups, 0, folds):.3f}  (expect ~0.5)")


if __name__ == "__main__":
    main()
