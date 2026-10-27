"""SCRATCH (teacher lane, 2026-10-21, not for main): can the ant tell the bottom of its hole from a wall?

usage: python3 -I dig_bottom.py COPIER_DIR LOG [LOG...]

LOG: a copylog soil log written with PP_TEACH_LABEL=1 by the patched logger that appends the 15th Q field (per Dig
option, target row minus the cavity's deepest open row, for reach 2 3 4 6). Decisions: empty jaws, Dig offered, not
shut in (teacher label M = a foot cell in reach, N = none).

Per log:
  rates      P(the brain picks Dig) on M and on N decisions (the per-decision wall rate step 0.5 quotes)
  channel    AUC (M vs N) of the brain's own best Dig score -- the job stage's only view of the options
  features   AUC of single candidate inputs: ctx depth, the deepest Dig option's DEPTH feature, the bottom flag
  probes     logistic on context + memory (+ one candidate), 4 folds by world: held-out AUC, and the N-decision
             false-alarm rate at 0.9 recall on M (how often it would dig a wall while digging the foot 9 times in 10)
Controls: POSITIVE, a label that is the bottom flag itself (must read ~1); NEGATIVE, labels shuffled across decisions
within a world (must read ~0.5).
"""
import sys
import numpy as np

sys.path.insert(0, sys.argv[1])
import copier  # noqa: E402

DIG, DEPTH_F = 3, 26
REACH = [2, 3, 4, 6]


def bottoms(path):
    """The 15th field of every non-settle Q row, in log order (parse_log's order)."""
    out = []
    for line in open(path):
        p = line.rstrip("\n").split(",")
        if p[1] == "Q" and p[6] != "1":
            out.append([None if s == "-" else [int(v) for v in s.split(" ")] for s in p[14].split(";")] if len(p) > 14 and p[14] else [])
    return out


def auc(score, y):
    score, y = np.asarray(score, float), np.asarray(y, bool)
    order = np.argsort(score, kind="mergesort")
    s = score[order]
    ranks = np.empty(len(s))
    i = 0
    while i < len(s):
        j = i
        while j + 1 < len(s) and s[j + 1] == s[i]:
            j += 1
        ranks[order[i:j + 1]] = (i + j) / 2 + 1
        i = j + 1
    n1, n0 = y.sum(), (~y).sum()
    return (ranks[y].sum() - n1 * (n1 + 1) / 2) / (n1 * n0)


def fit_logistic(X, y, steps=1500, lr=0.5, l2=1e-3):
    mu, sd = X.mean(0), X.std(0) + 1e-9
    Z = (X - mu) / sd
    w, b = np.zeros(X.shape[1]), 0.0
    pw = np.where(y, 0.5 / max(y.mean(), 1e-9), 0.5 / max(1 - y.mean(), 1e-9))  # balanced classes
    for _ in range(steps):
        p = 1 / (1 + np.exp(-(Z @ w + b)))
        g = (p - y) * pw
        w -= lr * (Z.T @ g / len(y) + l2 * w)
        b -= lr * g.mean()
    return lambda Xn: ((Xn - mu) / sd) @ w + b


def heldout(X, y, tags, folds=4):
    ut = np.unique(tags)
    s = np.zeros(len(y))
    for f in range(folds):
        test = np.isin(tags, ut[f::folds])
        m = fit_logistic(X[~test], y[~test])
        s[test] = m(X[test])
    return s


def fa_at_recall(s, y, recall=0.9):
    thr = np.quantile(s[y], 1 - recall)
    return (s[~y] >= thr).mean()


for path in sys.argv[2:]:
    dec, _ = copier.parse_log(path)
    bot = bottoms(path)
    assert len(bot) == len(dec), (len(bot), len(dec))
    rows = []
    for r, b in zip(dec, bot):
        if r["dig"] not in ("M", "N"):
            continue
        digs = [k for k, o in enumerate(r["opts"]) if o[0] == DIG]
        assert digs and all(b[k] is not None for k in digs), "a Dig option without a bottom reading"
        rows.append(dict(
            tag=r["tag"], m=r["dig"] == "M", picked=r["opts"][r["pick"]][0] == DIG,
            best=max(r["opts"][k][3] for k in digs),
            depth=max(r["opts"][k][4].get(DEPTH_F, 0.0) for k in digs),
            cdepth=r["ctx"][18],
            bot=[max(b[k][i] for k in digs) for i in range(len(REACH))],
            ctxmem=np.concatenate([r["ctx"], r["mem"]]),
        ))
    y = np.array([r["m"] for r in rows])
    tags = np.array([r["tag"] for r in rows])
    picked = np.array([r["picked"] for r in rows])
    nw = len(np.unique(tags))
    print(f"== {path}: {len(rows)} decisions ({y.sum()} with a foot cell in reach) over {nw} worlds")
    print(f"   rates     P(dig | foot in reach) {picked[y].mean():.3f}   P(dig | walls only) {picked[~y].mean():.4f}"
          f"   wall digs a world {picked[~y].sum() / nw:.1f}")
    print(f"   channel   best Dig score AUC {auc([r['best'] for r in rows], y):.3f}")
    print(f"   features  ctx depth {auc([r['cdepth'] for r in rows], y):.3f}   deepest Dig DEPTH {auc([r['depth'] for r in rows], y):.3f}   "
          + "   ".join(f"bottom(R{R}) {auc([r['bot'][i] for r in rows], y):.3f}" for i, R in enumerate(REACH)))
    for i, R in enumerate(REACH):
        flag = np.array([r["bot"][i] >= 0 for r in rows])
        print(f"             bottom flag R{R}: on {flag[y].mean():.3f} of foot decisions, {flag[~y].mean():.4f} of wall-only")
    CM = np.array([r["ctxmem"] for r in rows])
    nodepth = CM.copy()
    nodepth[:, 18] = 0.0
    col = lambda k: np.array([[r[k]] for r in rows], float)
    botc = lambda i: np.array([[r["bot"][i]] for r in rows], float)
    arms = [("senses + memory", CM), ("+ deepest Dig DEPTH", np.hstack([CM, col("depth")])),
            ("senses + memory, depth cut", nodepth)]
    arms += [(f"depth cut + bottom R{R}", np.hstack([nodepth, botc(i)])) for i, R in enumerate(REACH)]
    for name, X in arms:
        s = heldout(X, y, tags)
        print(f"   probe     {name:32s} AUC {auc(s, y):.3f}   wall false alarms at 0.9 recall {fa_at_recall(s, y):.4f}")
    rng = np.random.default_rng(1)
    yneg = y.copy()
    for t in np.unique(tags):
        ix = np.where(tags == t)[0]
        yneg[ix] = rng.permutation(y[ix])
    s = heldout(np.hstack([nodepth, botc(1)]), yneg, tags)
    print(f"   control   NEGATIVE (labels shuffled within world) AUC {auc(s, yneg):.3f}")
    ypos = np.array([r["bot"][1] >= 0 for r in rows])
    s = heldout(np.hstack([nodepth, botc(1)]), ypos, tags)
    print(f"   control   POSITIVE (label = bottom flag R3) AUC {auc(s, ypos):.3f}")
