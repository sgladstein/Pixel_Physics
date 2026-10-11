# Why R1 at eps 1e-6 does not converge to R0 on the R0-open maps: test the hypothesis that the
# difference is gas made in pockets R0 calls sealed (mound pockets, sealed rooms), which at any
# eps > 0 must leave through the porous spoil, partly into the nest's galleries. Control: zero
# the sources in every R0-sealed cell and the 1e-6 field must match R0 to rounding.
# Also the sensitivity of control (a): Field at eps 0.1 against expH.solve_p at eps 0.1001.
# Run: python3 -I p0b_specificity.py <deps dir>
import sys, os
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import poro_lib as P
H, G = P.H, P.G
for seed, f in P.MAPS:
    g, openm, sky, nest, geo, cells = P.setup(seed, f)
    s = H.srcs(seed, f, openm, 1, .25, 11, 0)
    c0 = P.Field(openm, sky, P.kcell(g, 'R0', 0)[0]).solve(s)
    if not np.isfinite(c0[cells]).any():
        continue
    sealed0 = openm & ~sky & ~np.isfinite(c0)
    s_clean = np.where(sealed0, 0.0, s)
    F1 = P.Field(openm, sky, P.kcell(g, 'R1', 1e-6)[0])
    c1 = F1.solve(s); c1c = F1.solve(s_clean)
    fin = cells & np.isfinite(c0)
    rel = np.nanmax(np.abs(c1[fin] - c0[fin]) / c0[fin]); relc = np.nanmax(np.abs(c1c[fin] - c0[fin]) / c0[fin])
    m0 = P.metrics(c0, openm, sky, geo, cells); m1 = P.metrics(c1c, openm, sky, geo, cells)
    rows = np.arange(g.shape[0])[:, None]
    print(f's{seed}@{f//1000}k: source in R0-sealed cells {s[sealed0].sum():.1f} (mound {s[sealed0 & (rows <= G.GROUND_ROW)].sum():.1f}) vs connected {s[openm & ~sealed0].sum():.1f};'
          f' max rel change with them {rel:.1e}, without them {relc:.1e}; without: span {m0["span"]*100:.2f}->{m1["span"]*100:.2f}%, read {m0["read"]*100:.2f}->{m1["read"]*100:.2f}%,'
          f' out {m0["readout"]*100:.2f}->{m1["readout"]*100:.2f}%, fol1 {m0["fol_1"]*100:.2f}->{m1["fol_1"]*100:.2f}%')
g, openm, sky, nest, geo, cells = P.setup(1, 200000)
s = H.srcs(1, 200000, openm, 1, .25, 11, 0)
rows = np.arange(g.shape[0])[:, None]; por = (g == 's') & (rows < G.GROUND_ROW)
a = P.Field(openm, sky, P.kcell(g, 'R1', 0.1)[0]).solve(s); b = P.solve_p(openm, sky, s, por, 0.1001)
fin = np.isfinite(a) & np.isfinite(b)
print(f'sensitivity of (a): Field eps 0.1 vs solve_p eps 0.1001 on s1@200k: max rel diff {np.abs(a[fin]-b[fin]).max()/np.abs(b[fin]).max():.2e} (must be > 0)')
