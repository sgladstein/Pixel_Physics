# Copy the room rule into one dig head. Objective per decision (scorer weights taken as equal): the chance of picking a
# refused cut, P = sum_ref m / sum m, with m = (softsign(rank)+1)/2 the head's pick multiplier; plus lk * the change in
# each allowed cut's log-multiplier relative to its decision's mean (keep the head's ranks among allowed cuts), plus
# lw * the change in the "when" read-out, plus l2 to the start weights.
# usage: fit2.py START OUT lk lw l2 steps train.csv,... test.csv,...   START = a .head file or a genome .raw.txt
import sys, numpy as np
start, out, lk, lw, l2, steps = sys.argv[1], sys.argv[2], *map(float, sys.argv[3:6]), int(sys.argv[6])
tr, te = sys.argv[7].split(','), sys.argv[8].split(',')
H, ND, NF = 8, 12, 45; N = NF + ND; HL = H * N + 3 * H + 2; CORE = 2797  # layout-5 core; the 6-slot lay gate rides after the head
def read_head(p):
    L = open(p).read().split('\n')
    if L[0].startswith('genome'):
        n = int(L[0].split()[1]); g = np.zeros(n)
        for l in L[1:]:
            if l.strip(): i, x = l.split(); g[int(i)] = float(x)
        return g[CORE:CORE + HL], ('genome', g)
    return np.array([float(x) for l in L[1:] for x in l.split()]), ('head', L[0])
v0, kind = read_head(start)
def unflat(v):
    return v[:H*N].reshape(H, N), v[H*N:H*N+H], v[H*N+H:H*N+2*H], v[H*N+2*H], v[H*N+2*H+1:H*N+3*H+1], v[H*N+3*H+1]
CACHE = {}
def load(fs):
    key = tuple(fs)
    if key in CACHE: return CACHE[key]
    X, V, D = [], [], []
    for i, f in enumerate(fs):
        npy = f + '.npy'
        try: d = np.load(npy)
        except FileNotFoundError: d = np.loadtxt(f, delimiter=',', usecols=range(0, 62)); np.save(npy, d)
        X.append(np.hstack([d[:, 17:62], d[:, 5:17]])); V.append((d[:, 4] > 0).astype(float)); D.append(d[:, 0] * 1e4 + d[:, 1] + i * 1e12)
    X, V, D = np.vstack(X), np.concatenate(V), np.concatenate(D)
    _, g = np.unique(D, return_inverse=True); G = g.max() + 1
    nref = np.bincount(g, V, G); n = np.bincount(g, None, G)
    keep = ((nref > 0) & (nref < n))[g]          # decisions with a choice to make
    _, g2 = np.unique(g[keep], return_inverse=True)
    r = (X[keep], V[keep], g2, np.mean(nref == n)); CACHE[key] = r; return r
ss = lambda x: x / (1 + np.abs(x)); dss = lambda x: 1 / (1 + np.abs(x)) ** 2
def fwd(v, X):
    W, b, a, c, e, c2 = unflat(v); z = X @ W.T + b; u = ss(z); r = c + u @ a; return z, u, r, u @ e
def pref(v, X, V, g):
    _, _, r, _ = fwd(v, X); m = (ss(r) + 1) / 2; G = g.max() + 1
    return np.mean(np.bincount(g, m * V, G) / np.bincount(g, m, G))
Xtr, Vtr, gtr, allr_tr = load(tr); Xte, Vte, gte, allr_te = load(te)
G = gtr.max() + 1; cnt = np.bincount(gtr, None, G)
p0 = pref(v0, Xte, Vte, gte)
uni = np.mean(np.bincount(gte, Vte) / np.bincount(gte))
_, _, r0, w0 = fwd(v0, Xtr); lm0 = np.log((ss(r0) + 1) / 2 + 1e-6); al = 1 - Vtr
cen0 = lm0 - np.bincount(gtr, lm0 * al, G)[gtr] / np.maximum(np.bincount(gtr, al, G), 1)[gtr]
v = v0.copy(); mo = np.zeros_like(v); vo = np.zeros_like(v); lr = 0.02
for t in range(1, steps + 1):
    W, b, a, c, e, c2 = unflat(v); z = Xtr @ W.T + b; u = ss(z); r = c + u @ a; wh = u @ e; m = (ss(r) + 1) / 2
    S = np.bincount(gtr, m, G); R = np.bincount(gtr, m * Vtr, G)
    dm = (Vtr * S[gtr] - R[gtr]) / S[gtr] ** 2 / G                    # dP/dm, P averaged over decisions
    lm = np.log(m + 1e-6); na = np.maximum(np.bincount(gtr, al, G), 1)
    cen = lm - np.bincount(gtr, lm * al, G)[gtr] / na[gtr]; dev = (cen - cen0) * al
    # d/dlm of sum dev^2 (centring folds in as dev - mean dev over the decision's allowed cuts)
    dlm = 2 * (dev - (np.bincount(gtr, dev, G) / na)[gtr] * al) * lk / len(Xtr)
    dm += dlm / (m + 1e-6)
    gr = dm * dss(r) / 2; gw = lw * 2 * (wh - w0) / len(Xtr)
    gz = (np.outer(gr, a) + np.outer(gw, e)) * dss(z)
    grad = np.concatenate([(gz.T @ Xtr).ravel(), gz.sum(0), u.T @ gr, [gr.sum()], u.T @ gw, [0.0]]) + l2 * 2 * (v - v0)
    mo = 0.9 * mo + 0.1 * grad; vo = 0.999 * vo + 0.001 * grad ** 2
    v -= lr * (mo / (1 - 0.9 ** t)) / (np.sqrt(vo / (1 - 0.999 ** t)) + 1e-8)
p1 = pref(v, Xte, Vte, gte); p1tr = pref(v, Xtr, Vtr, gtr)
_, _, rt, wt = fwd(v, Xte); _, _, rt0, wt0 = fwd(v0, Xte)
print(f'{start.split("/")[-1]}: P(pick refused) test {p0:.3f} -> {p1:.3f} (train {p1tr:.3f}; uniform {uni:.3f}); '
      f'rank corr allowed {np.corrcoef(rt[Vte == 0], rt0[Vte == 0])[0, 1]:.2f}; when shift {np.abs(wt - wt0).mean():.2f}/{wt0.std():.2f}; '
      f'all-refused decisions {allr_te:.2f}; |dv| {np.abs(v - v0).mean():.3f}', flush=True)
with open(out, 'w') as f:
    if kind[0] == 'head':
        f.write(kind[1] + '\n' + ' '.join(f'{x:.7g}' for x in v) + '\n')
    else:
        g = kind[1].copy(); g[CORE:CORE + HL] = v
        f.write(f'genome {len(g)} layout=5\n' + ''.join(f'{i} {x:.8g}\n' for i, x in enumerate(g) if x != 0))
