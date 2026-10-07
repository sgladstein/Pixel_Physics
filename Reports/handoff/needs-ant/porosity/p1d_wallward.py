# EXTRA: share of nest cells whose steepest descent over ALL 8 neighbours (any cell holding a value,
# soil included) is a soil cell -- render2's red, gradcheck's "wallward": the slope points into the
# walls or roof rather than along a gallery. Run: python3 -I p1d_wallward.py <deps dir>
import sys, os
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import poro_lib as P
H, G = P.H, P.G
VARIANTS = [('R0', 0.0, 0.0)] + [('R1', e, 0.0) for e in P.EPS] + [('R1', 0.1, 0.5)] + [('R2', e, 0.0) for e in P.EPS] + [('R2all', e, 0.0) for e in P.EPS]
res = {}
for seed, f in P.MAPS:
    g, openm, sky, nest, geo, cells = P.setup(seed, f)
    s = H.srcs(seed, f, openm, 1, .25, 11, 0)
    for v in VARIANTS:
        c = P.Field(openm, sky, P.kcell(g, v[0], v[1], seed, f)[0], v[2]).solve(s)
        if not np.isfinite(c[cells]).any():
            continue
        h, w = c.shape
        Pd = np.pad(np.where(np.isfinite(c), c, np.inf), 1, constant_values=np.inf)
        st = np.stack([Pd[1 + dy:1 + dy + h, 1 + dx:1 + dx + w] for dy, dx in G.N8])
        bi = st.argmin(0); bv = st.min(0)
        dys = np.array([d[0] for d in G.N8]); dxs = np.array([d[1] for d in G.N8])
        ys, xs = np.mgrid[0:h, 0:w]
        ty = np.clip(ys + dys[bi], 0, h - 1); tx = np.clip(xs + dxs[bi], 0, w - 1)
        ww = cells & np.isfinite(c) & (bv < c) & ~openm[ty, tx]
        res.setdefault(v, []).append(ww[cells].mean())
lines = [f'{v[0]} eps={v[1]:g} diag={v[2]:g} (n={len(L)}): steepest descent into soil {P.q(L)} of nest cells' for v, L in res.items()]
print('\n'.join(lines))
open(f'{P.OUT}/p1d_wallward.txt', 'w').write('\n'.join(lines) + '\n')
