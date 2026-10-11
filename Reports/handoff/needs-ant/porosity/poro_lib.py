# Shared code for the porosity study ("should loose soil pass air, and at what
# conductance"). Reuses the reviewer's code verbatim from DEPS (= sys.argv[1]):
#   gradcheck.py  load, geodesic, pairs, N4/N8, GROUND_ROW, OPEN, BASE
#   helpers.py    masks_v (sink = open sky over nothing), srcs (11-map averaged
#                 sources: ant 1, brood 0.25), reads (the 6-cell sensor, 4 on a
#                 diagonal; "points out" = front lower AND fewer walk steps out)
#   expH.py       solve_p, the porous solver (exec'd from the file, not copied)
# and adds ONE generalised solver, solve_k: a conductance per cell (open 1,
# porous eps, packed eps/20, sealed 0), a pair passes w_dir * min(k_i, k_j),
# w_dir = 1 on the 4 faces and wdiag on the 4 corners. With R1's k it is
# expH's rule exactly (open-open 1, open-spoil and spoil-spoil eps), and
# p0_controls.py checks it against solve_p and helpers.solve_v to rounding.
import sys, os
import numpy as np
import scipy.sparse as sp
import scipy.sparse.linalg as spla
from scipy.sparse.csgraph import connected_components
from scipy import ndimage, stats

DEPS = os.path.abspath(sys.argv[1])
OUT = '/mnt/project-files/needs-ant/porosity'
_argv = sys.argv[:]
sys.path.insert(0, DEPS)
sys.argv = [_argv[0], DEPS]
import helpers as H  # noqa: E402  (helpers inserts DEPS and imports gradcheck)
G = H.G
_ns = {}
exec(open(os.path.join(DEPS, 'expH.py')).read().split('MAPS=')[0], _ns)
solve_p = _ns['solve_p']
sys.argv = _argv

MAPS = list(H.MAPS)  # the 11 maps with a way out for the walk (same list as expH.MAPS)
EPS = (0.02, 0.05, 0.1, 0.3)
D_FRAME = 0.25 / 36  # continuum diffusivity of the trail planes' 3x3 blend: 0.25 per pass, 1 pass / 12 frames
ROWS = None


def load_map(seed, f):
    return G.load(f'{G.BASE}/s{seed}/map_f{f:06d}.txt')


def header(seed, f):
    with open(f'{G.BASE}/s{seed}/map_f{f:06d}.txt') as fh:
        return tuple(map(int, fh.readline().split()))


_hist = {}


def ever_open(seed, f, shape):
    """Cells open ('.abc') in ANY snapshot of this seed from 6,000 up to f
    (snapshots every 1,000 frames). Used only by R2's proxy for 'loose spoil
    below the ground line': a powder cell there now that was open before."""
    key = (seed, f)
    if key in _hist:
        return _hist[key]
    h0 = header(seed, f)
    acc = np.zeros(shape, bool)
    used = 0
    for ff in range(6000, f + 1, 1000):
        p = f'{G.BASE}/s{seed}/map_f{ff:06d}.txt'
        if not os.path.exists(p) or header(seed, ff) != h0:
            continue
        g = G.load(p)
        if g.shape != shape:
            continue
        acc |= np.isin(g, G.OPEN)
        used += 1
    _hist[key] = (acc, used)
    return acc, used


def kcell(g, rule, eps, seed=None, f=None):
    """Per-cell conductance. R0: solid sealed. R1: 's' above the ground line
    passes eps (expH's por). R2: loose spoil (all 's' above the ground line,
    plus 's' at/below it that was open in an earlier snapshot) passes eps;
    every other solid cell ('s' never opened, '#', food 'f', corpse 'x')
    passes eps/20."""
    h, w = g.shape
    rows = np.arange(h)[:, None]
    openm = np.isin(g, G.OPEN)
    k = np.where(openm, 1.0, 0.0)
    if rule == 'R0':
        return k, {}
    spoil_above = (g == 's') & (rows < G.GROUND_ROW)
    if rule == 'R1':
        k[spoil_above] = eps
        return k, {'porous': int(spoil_above.sum())}
    if rule in ('R2', 'R2all'):
        eo, used = ever_open(seed, f, g.shape)
        if rule == 'R2':
            loose_below = (g == 's') & (rows >= G.GROUND_ROW) & eo
        else:  # bracket, not a requested rule: every powder cell is loose
            loose_below = (g == 's') & (rows >= G.GROUND_ROW)
        k[~openm] = eps / 20.0
        k[spoil_above | loose_below] = eps
        return k, {'porous': int(spoil_above.sum()), 'loose_below': int(loose_below.sum()),
                   'packed': int((~openm & ~spoil_above & ~loose_below).sum()), 'hist_maps': used}
    raise ValueError(rule)


class Field:
    """Sink-included conductance operator over the cells that pass gas, sky held
    at 0 (Dirichlet), components with no path to a sink dropped (sealed)."""

    def __init__(self, openm, sky, k, wdiag=0.0):
        h, w = openm.shape
        self.shape = (h, w)
        self.sky = sky
        dom = (k > 0) & ~sky
        idx = -np.ones((h, w), np.int64)
        n = int(dom.sum())
        idx[dom] = np.arange(n)
        diag = np.zeros(n)
        R, C, V = [], [], []
        nb = [(d, 1.0) for d in G.N4] + ([(d, wdiag) for d in G.N8[4:]] if wdiag > 0 else [])
        for (dy, dx), wt in nb:
            y, x, ny, nx = G.pairs(h, w, dy, dx)
            a = dom[y, x] & (k[ny, nx] > 0)
            y, x, ny, nx = y[a], x[a], ny[a], nx[a]
            gg = wt * np.minimum(k[y, x], k[ny, nx])
            i = idx[y, x]
            np.add.at(diag, i, gg)
            m = dom[ny, nx]
            R.append(i[m]); C.append(idx[ny, nx][m]); V.append(-gg[m])
        A = sp.csr_matrix((np.concatenate(V), (np.concatenate(R), np.concatenate(C))), shape=(n, n))
        sink = diag + np.asarray(A.sum(1)).ravel()
        nc, lab = connected_components(A, directed=False)
        ok = np.zeros(nc, bool)
        np.logical_or.at(ok, lab, sink > 1e-12)
        keep = np.nonzero(ok[lab])[0]
        self.dom, self.idx, self.keep, self.n = dom, idx, keep, n
        self.M = (A + sp.diags(diag)).tocsc()[keep][:, keep]
        self.lu = spla.splu(self.M) if keep.size else None
        # grid position of each kept unknown
        ys, xs = np.nonzero(dom)
        self.ky, self.kx = ys[keep], xs[keep]
        self.pos = -np.ones((h, w), np.int64)
        self.pos[self.ky, self.kx] = np.arange(keep.size)

    def solve(self, src):
        c = np.full(self.shape, np.nan)
        c[self.sky] = 0.0
        if self.lu is not None:
            c[self.ky, self.kx] = self.lu.solve(src[self.ky, self.kx].astype(float))
        return c


def setup(seed, f):
    g = load_map(seed, f)
    openm, sky, nest = H.masks_v(g, 'orig')
    geo = G.geodesic(openm, sky)
    cells = nest & (geo >= 0)
    return g, openm, sky, nest, geo, cells


def follower(c, openm, sky, cells, thr, cap=400):
    """expF part 2, vectorised: steepest 1-cell descent over the 8 walkable
    neighbours with a value; step only if the drop passes (ideal: b < a - 1e-15;
    else (a-b)/(a+b) >= thr). Returns the terminal-is-sky mask over cells."""
    h, w = c.shape
    valid = openm & np.isfinite(c)
    P = np.pad(np.where(valid, c, np.inf), 1, constant_values=np.inf)
    stack = np.stack([P[1 + dy:1 + dy + h, 1 + dx:1 + dx + w] for dy, dx in G.N8])
    best = stack.argmin(0)
    bv = stack.min(0)
    with np.errstate(all='ignore'):
        if thr is None:
            ok = bv < c - 1e-15
        else:
            ok = (c - bv) / (c + bv) >= thr
    ok &= valid & np.isfinite(bv)
    dys = np.array([d[0] for d in G.N8]); dxs = np.array([d[1] for d in G.N8])
    ys, xs = np.mgrid[0:h, 0:w]
    ny = np.where(ok, ys + dys[best], ys); nx = np.where(ok, xs + dxs[best], xs)
    py, px = np.nonzero(cells)
    for _ in range(cap):
        qy, qx = ny[py, px], nx[py, px]
        stop = sky[py, px]  # expF stops at the sky
        qy = np.where(stop, py, qy); qx = np.where(stop, px, qx)
        if (qy == py).all() and (qx == px).all():
            break
        py, px = qy, qx
    return sky[py, px]


def los_cross(c, openm, cells, thr=0.01):
    """expF part 3, vectorised: of (cell, heading) reads >= thr whose front is
    open with a value, how many have a soil cell on the straight line between."""
    co = np.where(openm & np.isfinite(c), c, np.nan)
    tot = 0; cross = 0
    for dy, dx in G.N8:
        s = 6 if (dy == 0 or dx == 0) else 4
        fv = H.shift(co, s * dy, s * dx, np.nan)
        with np.errstate(all='ignore'):
            r = np.abs(fv - co) / (fv + co)
        rd = cells & np.isfinite(r) & (r >= thr)
        blocked = np.zeros(c.shape, bool)
        for kk in range(1, s):
            blocked |= ~H.shift(openm, kk * dy, kk * dx, False)
        tot += int(rd.sum()); cross += int((rd & blocked).sum())
    return cross, tot


def room_mask(openm, cells):
    return cells & (ndimage.uniform_filter(openm.astype(float), size=7) > 0.7)


def census(F, c, s, cells, geo, openm, pick_cells):
    """expB's three census numbers on field F (any rule): door level / top
    level; share of the top cell's level from sources above the ground line
    (expB zeroes rows > GROUND_ROW); per picked room cell, share of its level
    from sources within 6 and 20 cells (Green's row via the symmetric operator)."""
    cv = np.where(cells, c, np.nan)
    top = np.nanmax(cv)
    gc = np.where(cells, geo, 10 ** 9)
    dy, dx = np.unravel_index(gc.argmin(), gc.shape)
    door = c[dy, dx] / top
    sm = s.copy(); sm[G.GROUND_ROW + 1:, :] = 0
    cm = F.solve(sm)
    ty, tx = np.unravel_index(np.nanargmax(np.where(cells, c, -1)), c.shape)
    mound = cm[ty, tx] / top
    n6 = []; n20 = []
    yy, xx = np.mgrid[0:s.shape[0], 0:s.shape[1]]
    for (y, x) in pick_cells:
        e = np.zeros(s.shape); e[y, x] = 1.0
        gr = np.nan_to_num(F.solve(e)) if F.pos[y, x] >= 0 else np.zeros(s.shape)
        tot = (gr * s).sum()
        if not tot > 0:
            continue
        d = np.maximum(abs(yy - y), abs(xx - x))
        n6.append((gr * s * (d <= 6)).sum() / tot); n20.append((gr * s * (d <= 20)).sum() / tot)
    return {'door_top': float(door), 'mound_share': float(mound),
            'near6': float(np.median(n6)) if n6 else np.nan, 'near20': float(np.median(n20)) if n20 else np.nan,
            'n_pick': len(n6)}


def metrics(c, openm, sky, geo, cells):
    cn = c[cells]
    fin = np.isfinite(cn)
    m = {'n_cells': int(cells.sum()), 'n_finite': int(fin.sum()), 'open': bool(fin.any())}
    if not fin.any():
        return m
    hi, lo = np.nanmax(cn), np.nanmin(cn)
    m['span'] = float((hi - lo) / hi)
    best, bo = H.reads(np.where(openm, c, np.nan), openm, geo, 6, 4, 'rel')
    m['read'] = float(np.mean(best[cells] >= .01))
    m['readout'] = float(np.mean(bo[cells] >= .01))
    cr, tot = los_cross(c, openm, cells)
    m['los_cross'] = cr; m['los_tot'] = tot
    for name, thr in (('fol_ideal', None), ('fol_0.5', 0.005), ('fol_1', 0.01), ('fol_2', 0.02)):
        reach = follower(c, openm, sky, cells, thr)
        m[name] = float(reach.mean())                      # denominator: every walk-reachable nest cell
        m[name + '_fin'] = float(reach[fin].mean())        # expF's denominator: cells with a value
    rows = np.nonzero(cells)[0] - G.GROUND_ROW
    m['rho_depth'] = float(stats.spearmanr(cn[fin], rows[fin]).statistic)
    m['rho_geo'] = float(stats.spearmanr(cn[fin], geo[cells][fin]).statistic)
    return m


def q(v, pct=True, nd=1):
    v = [x for x in v if x is not None and np.isfinite(x)]
    if not v:
        return 'n/a'
    k = 100 if pct else 1
    return f'{np.median(v) * k:.{nd}f}{"%" if pct else ""} [{min(v) * k:.{nd}f}-{max(v) * k:.{nd}f}]'


def clear_out(c, openm, geo, cells, thr=0.01):
    """Per nest cell: does it have a read >= thr that points out (front lower and
    fewer walk steps from the sky, as helpers.reads) AND whose straight line to
    the sample is open air (expF part 3's test)? An extra beside the requested
    numbers: readable-and-out restricted to reads that do not look through soil."""
    co = np.where(openm & np.isfinite(c), c, np.nan)
    gf = geo.astype(float)
    has = np.zeros(c.shape, bool)
    for dy, dx in G.N8:
        s = 6 if (dy == 0 or dx == 0) else 4
        fv = H.shift(co, s * dy, s * dx, np.nan); fg = H.shift(gf, s * dy, s * dx, np.nan)
        with np.errstate(all='ignore'):
            r = np.abs(fv - co) / (fv + co)
        right = (fv < co) & (fg < gf) & (fg >= 0)
        blocked = np.zeros(c.shape, bool)
        for kk in range(1, s):
            blocked |= ~H.shift(openm, kk * dy, kk * dx, False)
        has |= cells & np.isfinite(r) & (r >= thr) & right & ~blocked
    return has


def follower_terms(c, openm, sky, cells, thr=None, cap=400):
    """follower() but returning the terminal cell of every nest cell's descent
    (rows, cols in np.nonzero(cells) order) and the number of steps taken."""
    h, w = c.shape
    valid = openm & np.isfinite(c)
    P_ = np.pad(np.where(valid, c, np.inf), 1, constant_values=np.inf)
    stack = np.stack([P_[1 + dy:1 + dy + h, 1 + dx:1 + dx + w] for dy, dx in G.N8])
    best = stack.argmin(0); bv = stack.min(0)
    with np.errstate(all='ignore'):
        ok = (bv < c - 1e-15) if thr is None else ((c - bv) / (c + bv) >= thr)
    ok &= valid & np.isfinite(bv)
    dys = np.array([d[0] for d in G.N8]); dxs = np.array([d[1] for d in G.N8])
    ys, xs = np.mgrid[0:h, 0:w]
    ny = np.where(ok, ys + dys[best], ys); nx = np.where(ok, xs + dxs[best], xs)
    py, px = np.nonzero(cells)
    steps = np.zeros(py.size, int)
    for _ in range(cap):
        stop = sky[py, px]
        qy = np.where(stop, py, ny[py, px]); qx = np.where(stop, px, nx[py, px])
        moved = (qy != py) | (qx != px)
        if not moved.any():
            break
        steps += moved
        py, px = qy, qx
    return py, px, steps
