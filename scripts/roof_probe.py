"""Can the ant's own touch features predict its roof flag? (Reports/soil-physics-realism-2026-10-21.md s3)

usage: python3 -I scripts/roof_probe.py CSV [FOLDS=4]

CSV is `examples/roof_cues.rs`'s output: one row per Dig option an ant was offered, with the 45 option features the
brain sees (f_roof_drop last), the engine's verdict on a subsample (`truth`: did cutting it drop ground within 3
frames), and four touch cues computed from the cells within two of the cut as they stand now:

  hold_min  fewest other grains holding any grain that rests on the cut cell or on air (9 = nothing rests)
  resting   some grain beside the cut rests on it or on air
  pellet    a spoil pellet rests on the cut
  thin      the ground straight above the cut is one cell thick

The method is the teacher session's probe (`probe.py` on ccr-439ee31b-pn36yq): a label, a set of the brain's inputs,
a linear model (logistic) and one hidden layer of 16 squash units (the size of the brain's target scorer), scored on
held-out WORLDS (folds by world, never by row: rows from one world are not independent).

Scored by AUC (does the model rank flagged options above unflagged ones, 0.5 = chance, 1 = perfect) and by recall and
precision at the threshold that maximises F1 on the training folds. The flag is rare (about 7% of Dig options), so
accuracy is not reported: always answering "no" scores 93%.

Controls (the repo's positive/negative-control rule): POSITIVE, a label that is a pure function of the inputs
(`lined`), must read near AUC 1; NEGATIVE, the roof flag shuffled between worlds, must read near 0.5. Either failing
means the probe, not the features, is being measured.
"""
import sys
import numpy as np

CSV = sys.argv[1]
FOLDS = int(sys.argv[2]) if len(sys.argv) > 2 else 4

raw = np.genfromtxt(CSV, delimiter=",", names=True, dtype=float, missing_values="", filling_values=np.nan)
cols = raw.dtype.names
X_all = np.column_stack([raw[c] for c in cols])
C = {c: i for i, c in enumerate(cols)}
feat_names = [c for c in cols if c.startswith("f_") and c != "f_roof_drop"]
world = raw["world"].astype(int)
flag = (raw["f_roof_drop"] > 0.5).astype(float)
truth = raw["truth"]
print(f"{CSV}: {len(world)} Dig options over {len(set(world))} worlds; flagged {flag.mean():.3%}; "
      f"engine verdict on {np.isfinite(truth).sum()} rows, {np.nanmean(truth):.3%} drop ground")


def cols_of(names):
    return np.column_stack([X_all[:, C[n]] for n in names])


def hold_onehot():
    h = X_all[:, C["hold_min"]].astype(int)
    return np.column_stack([(h == k).astype(float) for k in range(0, 7)])


def auc(score, y):
    order = np.argsort(score, kind="mergesort")
    s, yy = score[order], y[order]
    # average ranks for ties
    ranks = np.empty(len(s))
    i = 0
    while i < len(s):
        j = i
        while j + 1 < len(s) and s[j + 1] == s[i]:
            j += 1
        ranks[i:j + 1] = (i + j) / 2 + 1
        i = j + 1
    pos = yy == 1
    n1, n0 = pos.sum(), (~pos).sum()
    if n1 == 0 or n0 == 0:
        return float("nan")
    return (ranks[pos].sum() - n1 * (n1 + 1) / 2) / (n1 * n0)


def fit_logistic(X, y, steps=400, lr=0.5, l2=1e-4):
    w = np.zeros(X.shape[1])
    b = 0.0
    # balance the classes so a rare flag is not ignored
    pw = 0.5 / max(y.mean(), 1e-6)
    nw = 0.5 / max(1 - y.mean(), 1e-6)
    sw = np.where(y == 1, pw, nw)
    for _ in range(steps):
        z = X @ w + b
        p = 1 / (1 + np.exp(-np.clip(z, -30, 30)))
        g = (p - y) * sw
        w -= lr * (X.T @ g / len(y) + l2 * w)
        b -= lr * g.mean()
    return lambda Z: Z @ w + b


def fit_mlp(X, y, hidden=16, steps=600, lr=0.01, seed=0):
    rng = np.random.default_rng(seed)
    W1 = rng.normal(0, 1 / np.sqrt(X.shape[1]), (X.shape[1], hidden))
    b1 = np.zeros(hidden)
    W2 = rng.normal(0, 1 / np.sqrt(hidden), hidden)
    b2 = 0.0
    pw = 0.5 / max(y.mean(), 1e-6)
    nw = 0.5 / max(1 - y.mean(), 1e-6)
    sw = np.where(y == 1, pw, nw)
    params = [W1, b1, W2, np.array([b2])]
    m = [np.zeros_like(p) for p in params]
    v = [np.zeros_like(p) for p in params]
    for t in range(1, steps + 1):
        H = np.tanh(X @ params[0] + params[1])
        z = H @ params[2] + params[3][0]
        p = 1 / (1 + np.exp(-np.clip(z, -30, 30)))
        g = (p - y) * sw / len(y)
        gW2 = H.T @ g
        gb2 = np.array([g.sum()])
        gH = np.outer(g, params[2]) * (1 - H ** 2)
        gW1 = X.T @ gH
        gb1 = gH.sum(0)
        for i, gr in enumerate([gW1, gb1, gW2, gb2]):
            m[i] = 0.9 * m[i] + 0.1 * gr
            v[i] = 0.999 * v[i] + 0.001 * gr ** 2
            params[i] -= lr * (m[i] / (1 - 0.9 ** t)) / (np.sqrt(v[i] / (1 - 0.999 ** t)) + 1e-8)
    return lambda Z: np.tanh(Z @ params[0] + params[1]) @ params[2] + params[3][0]


def best_threshold(score, y):
    qs = np.quantile(score, np.linspace(0.5, 0.995, 120))
    best, bt = -1, qs[0]
    for q in qs:
        pred = score >= q
        tp = (pred & (y == 1)).sum()
        if pred.sum() == 0 or tp == 0:
            continue
        prec, rec = tp / pred.sum(), tp / (y == 1).sum()
        f1 = 2 * prec * rec / (prec + rec)
        if f1 > best:
            best, bt = f1, q
    return bt


def probe(name, X, y, rows=None, models=("linear", "mlp16")):
    if rows is None:
        rows = np.ones(len(y), bool)
    X, y, wv = X[rows], y[rows], world[rows]
    mu, sd = X.mean(0), X.std(0) + 1e-6
    X = (X - mu) / sd
    worlds = np.array(sorted(set(wv)))
    rng = np.random.default_rng(7)
    rng.shuffle(worlds)
    folds = np.array_split(worlds, FOLDS)
    out = []
    for model in models:
        scores = np.zeros(len(y))
        thr_pred = np.zeros(len(y), bool)
        for fw in folds:
            te = np.isin(wv, fw)
            tr = ~te
            f = fit_logistic(X[tr], y[tr]) if model == "linear" else fit_mlp(X[tr], y[tr])
            s_tr = f(X[tr])
            th = best_threshold(s_tr, y[tr])
            s = f(X[te])
            scores[te] = s
            thr_pred[te] = s >= th
        tp = (thr_pred & (y == 1)).sum()
        rec = tp / max((y == 1).sum(), 1)
        prec = tp / max(thr_pred.sum(), 1)
        out.append(f"{model}: AUC {auc(scores, y):.3f}  recall {rec:.2f}  precision {prec:.2f}")
    print(f"  {name:<44} n={len(y):>6}  " + "   ".join(out))


touch = ["f_drop_below", "f_headroom", "f_roofed", "f_cut", "f_lined", "f_hard", "f_wet",
         "f_air", "f_soil", "f_spoil", "f_stone", "f_water", "f_ant", "f_stand", "f_fall", "f_climb", "f_up", "f_depth"]
everything = feat_names

print("\nCONTROLS")
probe("POSITIVE: label `lined`, all features", cols_of(everything), (X_all[:, C["f_lined"]] > 0.5).astype(float))
ws = np.array(sorted(set(world)))
perm = dict(zip(ws, np.random.default_rng(3).permutation(ws)))
shuf = np.zeros(len(flag))
for wv in ws:
    src = flag[world == perm[wv]]
    n = (world == wv).sum()
    shuf[world == wv] = np.resize(src, n)
probe("NEGATIVE: flag moved between worlds, all features", cols_of(everything), shuf)

print("\nLABEL: the ant's own roof flag (f::ROOF_DROP)")
probe("the touch features the brief names", cols_of(touch), flag)
probe("every option feature but the flag (44)", cols_of(everything), flag)
probe("+ hold_min (one-hot 0..6)", np.column_stack([cols_of(everything), hold_onehot()]), flag)
probe("+ resting, pellet, thin (no count)", np.column_stack([cols_of(everything), cols_of(["resting", "pellet", "thin"])]), flag)
probe("hold_min alone (one-hot)", hold_onehot(), flag)
probe("hold_min<=2 or pellet, as one bit", ((X_all[:, C["hold_min"]] <= 2) | (X_all[:, C["pellet"]] > 0.5)).astype(float)[:, None], flag)

has = np.isfinite(truth)
yt = np.nan_to_num(truth)
print("\nLABEL: the engine's verdict (cut it in a clone, did ground fall within 3 frames) -- subsample")
probe("the flag itself", flag[:, None], yt, has, models=("linear",))
probe("every option feature but the flag (44)", cols_of(everything), yt, has)
probe("+ hold_min (one-hot 0..6)", np.column_stack([cols_of(everything), hold_onehot()]), yt, has)

print("\nAGREEMENT of the one-bit touch cue with the flag, row by row")
bit = (X_all[:, C["hold_min"]] <= 2) | (X_all[:, C["pellet"]] > 0.5)
f1 = flag > 0.5
print(f"  both {np.sum(bit & f1)}   flag only {np.sum(~bit & f1)}   cue only {np.sum(bit & ~f1)}   neither {np.sum(~bit & ~f1)}")
