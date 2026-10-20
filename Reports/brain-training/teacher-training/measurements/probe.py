"""SCRATCH (teacher-training measurement 2, not for main): can the brain copy a teacher's choice from what it senses?

usage: python3 -I probe.py LOG [FOLDS=4] [D1,D1b,D2,D3]

LOG is a PP_MIND_LOG of the soil arena written with threads=1 by the extended logger (G, M, O, C, D, E rows). For each
decision type a teacher is written for, this builds the teacher's label from PRIVILEGED facts (the frame-0 ground, the
depth datum, the hole as it stands at that frame), then asks how well a model sees it from the brain's OWN inputs
(senses, memory, option features), scored on held-out worlds (folds by world):

  floor      what a guess scores (target choice: a uniform pick; yes/no: always the commoner answer)
  brain now  how often s161's own choice already agrees with the teacher
  linear     a linear model (conditional logit over options / logistic) on the brain's inputs
  mlp16      one hidden layer of 16 squash units -- the size of the brain's target scorer

Controls (the repo's positive/negative-control rule): POSITIVE, a label that is a pure function of the inputs (the
option with the largest UP feature; "home_dist > 0.2"), must read near 100%; NEGATIVE, the teacher's labels moved to
decisions from other worlds, must read near the floor. Either failing means the probe, not the brain, is being measured.

Teachers (decision types):
  D1  dig target   chosen verb Dig, a foot cell (first cut of frame-0 ground, depth >= SHAFT) and a non-foot cell both
                   offered: label = the foot options.
  D1b dig or not   empty jaws, some Dig option offered: label = 1 if any offered Dig option is a foot cell.
  D2  put down     carrying spoil, some Drop option offered: label = 1 if any offered put-down cell is clear of the hole
                   (soil_away: 3 cells, Chebyshev, from every hole cell at that frame).
  D3  walk home    empty, on the surface, chosen verb Walk, 2+ Walk options, 3+ columns from the shaft mouth:
                   label = the Walk options whose target column is nearest the mouth.
"""
import sys
import numpy as np

LOG = sys.argv[1]
FOLDS = int(sys.argv[2]) if len(sys.argv) > 2 else 4
CTX = ["energy", "crop", "spoil", "at_nest", "a", "b", "alarm", "d_a", "d_b", "d_alarm", "d_energy", "home_seen", "home_dist",
       "d_home", "since_nest", "age", "self", "crowd", "depth", "roofed", "roof_cut", "hole_drop"]
C = {n: i for i, n in enumerate(CTX)}
WALK, DIG, DROP = 0, 3, 5
NF = 45
UP = 25


# ---------------------------------------------------------------- parse, one world at a time
def worlds(path):
    w = None
    for line in open(path):
        k = line[0]
        p = line.rstrip("\n").split(",")
        if k == "G":
            seed, x0, y0, nw, nh, shaft = (int(v) for v in p[1:7])
            datum = [None if v == "x" else int(v) for v in p[7].split()]
            founding = [tuple(int(v) for v in c.split(":")) for c in p[8].split()] if p[8] else []
            w = dict(seed=seed, x0=x0, y0=y0, nw=nw, nh=nh, shaft=shaft, datum=datum, founding=founding,
                     ground0=p[9], dec=[], cuts=[], ends=None)
        elif k == "M":
            w["dec"].append(dict(f=int(p[2]), id=int(p[3]), x=int(p[4]), y=int(p[5]), verb=int(p[6]),
                                 t=(int(p[7]), int(p[8])), ctx=np.array(p[10].split(), float),
                                 mem=np.array(p[11].split(), float)))
        elif k == "O":
            d = w["dec"][-1]
            assert d["f"] == int(p[2]) and d["id"] == int(p[3]), "O row does not follow its M row"
            opts = []
            for o in p[5].split(";") if p[5] else []:
                head, feats = o.split("|")
                v, tx, ty, s = head.split()
                fv = np.zeros(NF)
                for kv in feats.split():
                    i, x = kv.split(":")
                    fv[int(i)] = float(x)
                opts.append((int(v), int(tx), int(ty), float(s), fv))
            d["opts"] = opts
        elif k == "C":
            w["cuts"].append((int(p[2]), int(p[4]), int(p[5])))
        elif k == "E":
            w["ends"] = (int(p[2]), int(p[3]))
            yield w
            w = None


def facts(w):
    x0, y0, nw = w["x0"], w["y0"], w["nw"]
    g0 = w["ground0"]

    def ground0(x, y):
        return 0 <= x - x0 < nw and 0 <= y - y0 < w["nh"] and g0[(y - y0) * nw + (x - x0)] == "1"

    def depth(x, y):
        lows = [w["datum"][c - x0] for c in range(x - 3, x + 4) if 0 <= c - x0 < len(w["datum"]) and w["datum"][c - x0] is not None]
        return y - max(lows) if lows else 0

    cuts = sorted(w["cuts"])
    founding = set(w["founding"])
    top = min(y for _, y in founding) if founding else 0
    mouth = np.mean([x for x, y in founding if y == top]) if founding else 0.0
    return ground0, depth, cuts, founding, mouth


# ---------------------------------------------------------------- build the four data sets (cached beside the log)
import os, pickle
ONLY = sys.argv[3].split(",") if len(sys.argv) > 3 else ["D1", "D1b", "D2", "D3"]
CACHE = LOG + ".probe.pkl"
D1, D1b, D2, D3 = [], [], [], []
n_worlds, n_dec = 0, 0
for wi, w in enumerate(worlds(LOG) if not os.path.exists(CACHE) else []):
    n_worlds += 1
    ground0, depth, cuts, founding, mouth = facts(w)
    S = w["shaft"]
    ci, hole, cut_before = 0, set(founding), set()
    for d in w["dec"]:
        n_dec += 1
        while ci < len(cuts) and cuts[ci][0] < d["f"]:
            _, cx, cy = cuts[ci]
            if ground0(cx, cy):
                hole.add((cx, cy))
            cut_before.add((cx, cy))
            ci += 1
        opts = d.get("opts", [])
        ctx, mem = d["ctx"], d["mem"]
        carrying = ctx[C["spoil"]] > 0.5

        def foot(tx, ty):
            return ground0(tx, ty) and (tx, ty) not in cut_before and depth(tx, ty) >= S

        def away(tx, ty):
            return not any((tx + dx, ty + dy) in hole for dx in (-2, -1, 0, 1, 2) for dy in (-2, -1, 0, 1, 2))

        digs = [o for o in opts if o[0] == DIG]
        drops = [o for o in opts if o[0] == DROP]
        walks = [o for o in opts if o[0] == WALK]
        agg = lambda os: np.concatenate([np.mean([o[4] for o in os], 0), np.max([o[4] for o in os], 0)])
        base = np.concatenate([ctx, mem])
        if d["verb"] == DIG and len(digs) >= 2:
            lab = np.array([foot(o[1], o[2]) for o in digs])
            if lab.any() and not lab.all():
                chosen = [i for i, o in enumerate(digs) if (o[1], o[2]) == d["t"]]
                D1.append(dict(w=wi, F=np.array([o[4] for o in digs]), base=base, lab=lab, brain=chosen[0] if chosen else -1))
        if not carrying and digs:
            D1b.append(dict(w=wi, x=np.concatenate([base, agg(digs)]), y=int(any(foot(o[1], o[2]) for o in digs)),
                            brain=int(d["verb"] == DIG), hd=ctx[C["home_dist"]]))
        if carrying and drops:
            D2.append(dict(w=wi, x=np.concatenate([base, agg(drops)]), y=int(any(away(o[1], o[2]) for o in drops)),
                           brain=int(d["verb"] == DROP), hd=ctx[C["home_dist"]]))
        if (not carrying and d["verb"] == WALK and len(walks) >= 2 and depth(d["x"], d["y"]) <= 0
                and abs(d["x"] - mouth) >= 3):
            dist = np.array([abs(o[1] - mouth) for o in walks])
            lab = dist == dist.min()
            if not lab.all():
                chosen = [i for i, o in enumerate(walks) if (o[1], o[2]) == d["t"]]
                D3.append(dict(w=wi, F=np.array([o[4] for o in walks]), base=base, lab=lab, brain=chosen[0] if chosen else -1))

if os.path.exists(CACHE):
    D1, D1b, D2, D3, n_worlds, n_dec = pickle.load(open(CACHE, "rb"))
else:
    pickle.dump((D1, D1b, D2, D3, n_worlds, n_dec), open(CACHE, "wb"))
print(f"worlds {n_worlds}, decisions {n_dec}")
print(f"samples: D1 dig target {len(D1)}, D1b dig or not {len(D1b)}, D2 put down {len(D2)}, D3 walk home {len(D3)}")


# ---------------------------------------------------------------- models (numpy, full batch, Adam)
def sq(z):
    return z / (1 + np.abs(z))


def dsq(z):
    return 1 / (1 + np.abs(z)) ** 2


def adam(params, grads, st, lr=0.02):
    st["t"] += 1
    for k in params:
        st.setdefault(k, [np.zeros_like(params[k]), np.zeros_like(params[k])])
        m, v = st[k]
        m[:] = 0.9 * m + 0.1 * grads[k]
        v[:] = 0.999 * v + 0.001 * grads[k] ** 2
        params[k] -= lr * (m / (1 - 0.9 ** st["t"])) / (np.sqrt(v / (1 - 0.999 ** st["t"])) + 1e-8)


def standardise(tr, te):
    mu, sd = tr.reshape(-1, tr.shape[-1]).mean(0), tr.reshape(-1, tr.shape[-1]).std(0) + 1e-6
    return (tr - mu) / sd, (te - mu) / sd


def fit_choice(X, M, L, hidden, steps=600, l2=1e-3, seed=0):
    """X: decisions x options x dims; M: mask of real options; L: label mask. Softmax over options; loss -log P(label set)."""
    r = np.random.default_rng(seed)
    dm = X.shape[-1]
    P = {"w": r.normal(0, 0.1, (dm, hidden)), "b": np.zeros(hidden), "o": r.normal(0, 0.1, hidden)} if hidden else {"w": np.zeros(dm)}
    st = {"t": 0}
    for _ in range(steps):
        if hidden:
            Z = X @ P["w"] + P["b"]
            H = sq(Z)
            s = H @ P["o"]
        else:
            s = X @ P["w"]
        s = np.where(M, s, -1e9)
        s = s - s.max(1, keepdims=True)
        e = np.exp(s) * M
        p = e / e.sum(1, keepdims=True)
        pl = (p * L).sum(1, keepdims=True)
        g = (p - p * L / np.maximum(pl, 1e-12)) / len(X)  # d(-log sum_L p)/ds
        if hidden:
            G = {"o": np.einsum("no,noh->h", g, H) + l2 * P["o"]}
            dZ = g[..., None] * P["o"] * dsq(Z)
            G["w"] = np.einsum("nod,noh->dh", X, dZ) + l2 * P["w"]
            G["b"] = dZ.sum((0, 1))
        else:
            G = {"w": np.einsum("no,nod->d", g, X) + l2 * P["w"]}
        adam(P, G, st)
    return P


def score_choice(P, X, M):
    s = sq(X @ P["w"] + P["b"]) @ P["o"] if "o" in P else X @ P["w"]
    return np.where(M, s, -1e9).argmax(1)


def fit_binary(X, y, hidden, steps=600, l2=1e-3, seed=0):
    r = np.random.default_rng(seed)
    dm = X.shape[1]
    P = {"w": r.normal(0, 0.1, (dm, hidden)), "b": np.zeros(hidden), "o": r.normal(0, 0.1, hidden), "c": np.zeros(1)} if hidden \
        else {"w": np.zeros(dm), "c": np.zeros(1)}
    st = {"t": 0}
    for _ in range(steps):
        if hidden:
            Z = X @ P["w"] + P["b"]
            H = sq(Z)
            s = H @ P["o"] + P["c"]
        else:
            s = X @ P["w"] + P["c"]
        p = 1 / (1 + np.exp(-np.clip(s, -30, 30)))
        g = (p - y) / len(X)
        if hidden:
            G = {"o": H.T @ g + l2 * P["o"], "c": np.array([g.sum()])}
            dZ = np.outer(g, P["o"]) * dsq(Z)
            G["w"] = X.T @ dZ + l2 * P["w"]
            G["b"] = dZ.sum(0)
        else:
            G = {"w": X.T @ g + l2 * P["w"], "c": np.array([g.sum()])}
        adam(P, G, st)
    return P


def pred_binary(P, X):
    s = (sq(X @ P["w"] + P["b"]) @ P["o"] if "o" in P else X @ P["w"]) + P["c"]
    return (s > 0).astype(int)


# ---------------------------------------------------------------- evaluation
def folds_of(ws):
    u = sorted(set(ws))
    return [set(u[i::FOLDS]) for i in range(FOLDS)]


def choice_report(name, data, labkey="lab"):
    if len(data) < 40:
        print(f"\n{name}: only {len(data)} samples, skipped")
        return
    n = max(len(d["F"]) for d in data)
    dims = NF + 34
    X = np.zeros((len(data), n, dims))
    M = np.zeros((len(data), n), bool)
    L = np.zeros((len(data), n), bool)
    Lpos = np.zeros((len(data), n), bool)
    for i, d in enumerate(data):
        k = len(d["F"])
        X[i, :k, :NF] = d["F"]
        X[i, :k, NF:] = d["base"]
        M[i, :k] = True
        L[i, :k] = d[labkey]
        up = d["F"][:, UP]
        Lpos[i, :k] = up == up.max()
    ws = np.array([d["w"] for d in data])
    # negative control: each decision takes the label pattern of a same-size decision from another world
    rng = np.random.default_rng(1)
    Lneg = np.zeros_like(L)
    by_k = {}
    for i, d in enumerate(data):
        by_k.setdefault(len(d["F"]), []).append(i)
    for i, d in enumerate(data):
        pool = [j for j in by_k[len(d["F"])] if ws[j] != ws[i]] or by_k[len(d["F"])]
        Lneg[i] = L[rng.choice(pool)]
    floor = np.mean([L[i].sum() / M[i].sum() for i in range(len(data))])
    brain = np.mean([d["brain"] >= 0 and d[labkey][d["brain"]] for d in data])
    out = {}
    for lab_name, LL in (("teacher", L), ("POSITIVE ctrl", Lpos), ("NEGATIVE ctrl", Lneg)):
        for model, h in (("linear", 0), ("mlp16", 16)):
            hits = []
            for te in folds_of(ws):
                tri = ~np.isin(ws, list(te))
                tei = ~tri
                Xtr, Xte = standardise(X[tri], X[tei])
                P = fit_choice(Xtr, M[tri], LL[tri], h)
                pick = score_choice(P, Xte, M[tei])
                hits += list(LL[tei][np.arange(tei.sum()), pick])
            out[(lab_name, model)] = np.mean(hits)
    fl_pos = np.mean([Lpos[i].sum() / M[i].sum() for i in range(len(data))])
    fl_neg = np.mean([Lneg[i].sum() / M[i].sum() for i in range(len(data))])
    print(f"\n{name}  (n={len(data)}, {len(set(ws))} worlds, options per decision median {int(np.median(M.sum(1)))})")
    print(f"  teacher        floor {floor:.3f}   brain now {brain:.3f}   linear {out[('teacher', 'linear')]:.3f}   mlp16 {out[('teacher', 'mlp16')]:.3f}")
    print(f"  POSITIVE ctrl  floor {fl_pos:.3f}   linear {out[('POSITIVE ctrl', 'linear')]:.3f}   mlp16 {out[('POSITIVE ctrl', 'mlp16')]:.3f}   (must be near 1)")
    print(f"  NEGATIVE ctrl  floor {fl_neg:.3f}   linear {out[('NEGATIVE ctrl', 'linear')]:.3f}   mlp16 {out[('NEGATIVE ctrl', 'mlp16')]:.3f}   (must be near its floor)")


def binary_report(name, data):
    if len(data) < 40:
        print(f"\n{name}: only {len(data)} samples, skipped")
        return
    X = np.array([d["x"] for d in data])
    y = np.array([d["y"] for d in data])
    ypos = np.array([int(d["hd"] > 0.2) for d in data])
    ws = np.array([d["w"] for d in data])
    rng = np.random.default_rng(1)
    yneg = y.copy()
    # move whole worlds' label sequences onto the next world's decisions (a rotation: no world keeps its own labels)
    wl = sorted(set(ws))
    perm = {wl[i]: wl[(i + 1) % len(wl)] for i in range(len(wl))}
    for i in range(len(y)):
        src = np.where(ws == perm[ws[i]])[0]
        yneg[i] = y[src[i % len(src)]]
    brain = np.mean(np.array([d["brain"] for d in data]) == y)
    out = {}
    for lab_name, yy in (("teacher", y), ("POSITIVE ctrl", ypos), ("NEGATIVE ctrl", yneg)):
        for model, h in (("linear", 0), ("mlp16", 16)):
            hits = []
            for te in folds_of(ws):
                tri = ~np.isin(ws, list(te))
                tei = ~tri
                Xtr, Xte = standardise(X[tri], X[tei])
                P = fit_binary(Xtr, yy[tri], h)
                hits += list(pred_binary(P, Xte) == yy[tei])
            out[(lab_name, model)] = np.mean(hits)
    fl = lambda yy: max(yy.mean(), 1 - yy.mean())
    print(f"\n{name}  (n={len(data)}, {len(set(ws))} worlds, teacher says yes {y.mean():.3f})")
    print(f"  teacher        floor {fl(y):.3f}   brain now {brain:.3f}   linear {out[('teacher', 'linear')]:.3f}   mlp16 {out[('teacher', 'mlp16')]:.3f}")
    print(f"  POSITIVE ctrl  floor {fl(ypos):.3f}   linear {out[('POSITIVE ctrl', 'linear')]:.3f}   mlp16 {out[('POSITIVE ctrl', 'mlp16')]:.3f}   (must be near 1)")
    print(f"  NEGATIVE ctrl  floor {fl(yneg):.3f}   linear {out[('NEGATIVE ctrl', 'linear')]:.3f}   mlp16 {out[('NEGATIVE ctrl', 'mlp16')]:.3f}   (must be near its floor)")


if "D1" in ONLY:
    choice_report("D1  dig target: cut the foot", D1)
if "D1b" in ONLY:
    binary_report("D1b dig or not: dig only with a foot cell in reach", D1b)
if "D2" in ONLY:
    binary_report("D2  put down: drop at the first cell clear of the hole", D2)
if "D3" in ONLY:
    choice_report("D3  walk home: empty on the surface, head for the mouth", D3)
