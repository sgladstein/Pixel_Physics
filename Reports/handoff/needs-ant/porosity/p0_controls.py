# Controls for the porosity study. Run: python3 -I p0_controls.py <deps dir>
#  (a) solve_k (poro_lib.Field) equals expH.solve_p and helpers.solve_v to rounding
#  (b) R0 through this pipeline reproduces expH (readable/span), expF (2)+(3), expB (census)
#  (c) specificity: R1 at eps 1e-6 against R0
#  (d) toy positive control: sealed 10x10 room, only exit a 3-cell porous lid at eps 0.1
#  (e) eigen control: the 40-deep shaft of control.py, lowest eigenvalue against the formula
import sys, os, math
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import poro_lib as P
from scipy.sparse.linalg import eigsh
H, G = P.H, P.G

print('(a) solver equivalence on the 11 maps: max |c_k - c_ref| / max|c_ref| over the nest+mound, NaN pattern')
worst = {}
for seed, f in P.MAPS:
    g, openm, sky, nest, geo, cells = P.setup(seed, f)
    s = H.srcs(seed, f, openm, 1, .25, 11, 0)
    rows = np.arange(g.shape[0])[:, None]
    por = (g == 's') & (rows < G.GROUND_ROW)
    for eps in (0.0, 1e-6, 0.02, 0.05, 0.1, 0.3):
        ref = P.solve_p(openm, sky, s, por, eps)
        k, _ = P.kcell(g, 'R1' if eps > 0 else 'R0', eps)
        c = P.Field(openm, sky, k).solve(s)
        # solve_p leaves porous cells at eps=0 as NaN too; compare where either is finite
        same_nan = bool(np.array_equal(np.isfinite(ref) & (openm | por), np.isfinite(c) & (openm | por)))
        fin = np.isfinite(ref) & np.isfinite(c)
        d = np.abs(c[fin] - ref[fin]).max() / max(np.abs(ref[fin]).max(), 1e-300) if fin.any() else 0.0
        key = f'R1 eps {eps:g} vs expH.solve_p'
        worst[key] = (max(worst.get(key, (0, True))[0], d), worst.get(key, (0, True))[1] and same_nan)
    for wd in (0.0, 0.5):
        ref = H.solve_v(openm, sky, s, wd)
        c = P.Field(openm, sky, P.kcell(g, 'R0', 0)[0], wd).solve(s)
        same_nan = bool(np.array_equal(np.isfinite(ref) & openm, np.isfinite(c) & openm))
        fin = np.isfinite(ref) & np.isfinite(c) & openm
        d = np.abs(c[fin] - ref[fin]).max() / max(np.abs(ref[fin]).max(), 1e-300) if fin.any() else 0.0
        key = f'R0 diag {wd} vs helpers.solve_v'
        worst[key] = (max(worst.get(key, (0, True))[0], d), worst.get(key, (0, True))[1] and same_nan)
for k_, (d, same) in worst.items():
    print(f'  {k_:32s}: max rel diff {d:.2e}, NaN pattern identical on all 11 maps: {same}')

print('(b) R0 through this pipeline')
spans, rds, ros = [], [], []
fol = {}
los = {}
r0_open = []
for seed, f in P.MAPS:
    g, openm, sky, nest, geo, cells = P.setup(seed, f)
    s = H.srcs(seed, f, openm, 1, .25, 11, 0)
    F = P.Field(openm, sky, P.kcell(g, 'R0', 0)[0])
    c = F.solve(s)
    m = P.metrics(c, openm, sky, geo, cells)
    if not m['open']:
        continue
    r0_open.append((seed, f))
    spans.append(m['span']); rds.append(m['read']); ros.append(m['readout'])
    fol[(seed, f)] = m
print(f'  maps with a way out {len(r0_open)}/11 {r0_open}')
print(f'  span {P.q(spans)}; reads >=1% {P.q(rds)}; reads and points out {P.q(ros)}   (expH: 6.0% [2.9-8.8]; 9.1% [3.3-16.3]; 8.5% [2.2-14.5])')
for key in [(1, 200000), (2, 100000), (3, 200000), (4, 295000)]:
    m = fol[key]
    print(f'  s{key[0]}@{key[1]//1000}k follower (expF denominator): ideal {m["fol_ideal_fin"]*100:.1f}%, 1% rel {m["fol_1_fin"]*100:.1f}%;'
          f' all-cells denominator: ideal {m["fol_ideal"]*100:.1f}%, 1% {m["fol_1"]*100:.1f}%; n_cells {m["n_cells"]} n_finite {m["n_finite"]};'
          f' LOS cross {m["los_cross"]}/{m["los_tot"]}')
print('  (expF: s1 99.8/0.5, s2 99.8/0.7, s3 99.6/0.0, s4 99.5/0.0; LOS s1 54/174, s2 38/161, s3 40/122, s4 33/59)')
OPEN7 = [(1, 200000), (2, 100000), (2, 295000), (3, 100000), (3, 200000), (4, 100000), (4, 295000)]
rng = np.random.default_rng(1)
print('  census with expB\'s shared rng(1) over its 7 maps, in its order:')
for seed, f in OPEN7:
    g, openm, sky, nest, geo, cells = P.setup(seed, f)
    s = H.srcs(seed, f, openm, 1, .25, 11, 0)
    F = P.Field(openm, sky, P.kcell(g, 'R0', 0)[0])
    c = F.solve(s)
    room = P.room_mask(openm, cells); ys, xs = np.nonzero(room)
    pick = rng.choice(len(ys), size=min(25, len(ys)), replace=False)
    cs = P.census(F, c, s, cells, geo, openm, [(ys[k], xs[k]) for k in pick])
    print(f'    s{seed}@{f//1000}k door/top {cs["door_top"]:.3f} mound {cs["mound_share"]:.3f} near6 {cs["near6"]:.3f} near20 {cs["near20"]:.3f}')

print('(c) specificity: R1 at eps 1e-6 against R0')
for seed, f in P.MAPS:
    g, openm, sky, nest, geo, cells = P.setup(seed, f)
    s = H.srcs(seed, f, openm, 1, .25, 11, 0)
    c0 = P.Field(openm, sky, P.kcell(g, 'R0', 0)[0]).solve(s)
    c1 = P.Field(openm, sky, P.kcell(g, 'R1', 1e-6)[0]).solve(s)
    m0 = P.metrics(c0, openm, sky, geo, cells); m1 = P.metrics(c1, openm, sky, geo, cells)
    if m0['open']:
        fin = cells & np.isfinite(c0)
        rel = np.nanmax(np.abs(c1[fin] - c0[fin]) / c0[fin])
        print(f'  s{seed}@{f//1000}k R0-open: max rel change of nest level {rel:.1e}; span {m0["span"]*100:.2f} -> {m1["span"]*100:.2f}%,'
              f' read {m0["read"]*100:.2f} -> {m1["read"]*100:.2f}%, out {m0["readout"]*100:.2f} -> {m1["readout"]*100:.2f}%,'
              f' fol1% {m0["fol_1"]*100:.2f} -> {m1["fol_1"]*100:.2f}%; R0 nest cells sealed {m0["n_cells"]-m0["n_finite"]} -> {m1["n_cells"]-m1["n_finite"]}')
    else:
        v = c1[cells]
        print(f'  s{seed}@{f//1000}k R0-SEALED: under 1e-6 the nest is {"open" if m1["open"] else "sealed"}; level median {np.nanmedian(v):.3g},'
              f' span {m1.get("span", np.nan)*100:.4f}%, read {m1.get("read", np.nan)*100:.2f}%, fol1% {m1.get("fol_1", np.nan)*100:.2f}%')

print('(d) toy: sealed 10x10 room (rows 13-22, cols 10-19), only connection a 3-row porous lid (rows 10-12) at eps 0.1 under open sky (rows 0-9); source 1 per room cell')
h, w = 30, 30
openm = np.zeros((h, w), bool); openm[:10, :] = True; openm[13:23, 10:20] = True
sky = np.zeros((h, w), bool); sky[:10, :] = True
k = np.where(openm, 1.0, 0.0); k[10:13, 10:20] = 0.1
src = np.where(openm & ~sky, 1.0, 0.0)
Ft = P.Field(openm, sky, k); ct = Ft.solve(src)
col = ct[10:23, 14]
ana = [100, 200, 300] + [400 + sum(10 - j for j in range(1, i)) for i in range(1, 11)]
print('  column 14, rows 10..22 (lid x3, room x10):', [round(v, 2) for v in col])
print('  analytic (series faces of 0.1, then room profile):', ana)
room = ct[13:23, 10:20]
print(f'  room max at floor row (row 22) in every column: {bool((room.argmax(0) == 9).all())}; strictly decreasing upward in every column: {bool((np.diff(room, axis=0) > 0).all())};'
      f' columns identical: {bool(np.allclose(room, room[:, :1]))}; span (max-min)/max {100*(room.max()-room.min())/room.max():.1f}%')
geo_t = np.full((h, w), -1)
best, _ = H.reads(np.where(openm, ct, np.nan), openm, geo_t, 6, 4, 'rel')
rc = np.zeros((h, w), bool); rc[13:23, 10:20] = True
print(f'  6-cell sensor >=1% in {100*np.mean(best[rc] >= .01):.0f}% of room cells; best contrast median {100*np.median(best[rc][best[rc] >= 0]):.2f}% max {100*best[rc].max():.2f}%')
allv = np.stack([np.pad(ct, 1, constant_values=np.nan)[1 + dy:1 + dy + h, 1 + dx:1 + dx + w] for dy, dx in G.N8])
dn = np.nanargmin(np.where(np.isnan(allv), np.inf, allv), axis=0)
up = np.array([G.N8[i][0] for i in dn[13:23, 10:20].ravel()])
print(f'  steepest descent over all 8 neighbours points up (toward the lid) in {100*np.mean(up < 0):.0f}% of room cells')

print('(e) eigen control: the shaft of control.py (5 wide, 40 deep, sealed sides, open sky on top)')
h, w = 60, 20
openm = np.zeros((h, w), bool); openm[:10, :] = True; openm[10:50, 8:13] = True
sky = np.zeros((h, w), bool); sky[:10, :] = True
Fs = P.Field(openm, sky, np.where(openm, 1.0, 0.0))
lam = np.sort(eigsh(Fs.M, k=3, sigma=0, which='LM', return_eigenvectors=False))
L = 40
exact = 4 * math.sin(math.pi / (2 * (2 * L + 1))) ** 2
print(f'  lowest eigenvalue {lam[0]:.6e} vs 4 sin^2(pi/162) = {exact:.6e} (rel err {abs(lam[0]-exact)/exact:.1e}); next {lam[1]:.4e} vs {4*math.sin(3*math.pi/162)**2:.4e}')
tau = 1 / (P.D_FRAME * lam[0])
print(f'  tau1 = 1/(D lambda1) = {tau:,.0f} frames at D = 0.25/36 per frame; 10% settle ~ tau1*ln(10*32/pi^3) = {tau*math.log(10*32/math.pi**3):,.0f}'
      f' (gradcheck settle_frames stepped it to ~222k; formula 218k)')
