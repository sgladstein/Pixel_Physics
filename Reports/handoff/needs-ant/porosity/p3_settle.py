# Item 7: how long a physically STEPPED gas would take to settle to the solved field.
# Run: python3 -I p3_settle.py <deps dir>
# ASSUMPTION (no nest-air code exists): it would diffuse like the trail planes in pheromone.rs,
# a 3x3 blend of DIFFUSE = 0.25 per pass, one pass per 12 frames (PASS interval), i.e. a
# continuum diffusivity D = 0.25/36 cells^2 per frame -- the constant gradcheck.settle_frames
# used, validated by control.py's shaft (222k stepped vs 218k formula). Every cell, porous or
# open, holds gas at unit capacity (also an assumption). Frames scale as 1/D for any other rate.
# For the 8-neighbour operator (diagonal 0.5) the continuum diffusivity per unit conductance is
# 2, so it is converted at D/2 to describe the same gas.
#   lambda_1    smallest eigenvalue of the sink-included matrix (eigsh, shift-invert at 0)
#   tau_1       1/(D_eff lambda_1) frames: e-folding time of the slowest mode
#   nest share  sum of v_1^2 over nest cells (v_1 unit norm): is the slowest mode in the nest?
#   t10         frames for the nest, from empty, to come within 10% (L1) of the solved field,
#               from the 60 slowest modes; the rest bounded by exp(-D lambda_61 t); one case
#               checked by explicit stepping.
import sys, os, math
import numpy as np
from scipy.sparse.linalg import eigsh
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import poro_lib as P
H, G = P.H, P.G
VARIANTS = [('R0', 0.0, 0.0), ('R0', 0.0, 0.5)] + [('R1', e, 0.0) for e in P.EPS] + [('R1', 0.1, 0.5)] + [('R2', e, 0.0) for e in P.EPS]
K = 60
lines = []
def emit(s):
    print(s, flush=True); lines.append(s)

def t10_modes(F, b, nestk, lam, V, D):
    css = F.lu.solve(b)
    a = V.T @ css                       # mode amplitudes of the steady state
    rem0 = np.linalg.norm(css - V @ a)  # what the slow modes do not carry
    norm = np.abs(css[nestk]).sum()
    def err(t):
        e = V[nestk] @ (a * np.exp(-D * lam * t))
        return np.abs(e).sum() / norm
    if err(0) < 0.1:
        return 0.0, rem0, 0.0
    lo, hi = 0.0, 1.0
    while err(hi) > 0.1:
        hi *= 2
        if hi > 1e12:
            return float('inf'), rem0, 0.0
    for _ in range(60):
        mid = 0.5 * (lo + hi)
        lo, hi = (mid, hi) if err(mid) > 0.1 else (lo, mid)
    bound = rem0 * math.sqrt(nestk.sum()) * math.exp(-D * lam[-1] * hi) / norm  # L1 <= sqrt(n) L2
    return hi, rem0 / np.linalg.norm(css), bound

def t10_step(F, b, nestk, D, pass_frames=12, cap=60_000_000):
    css = F.lu.solve(b); norm = np.abs(css[nestk]).sum()
    beta = D * pass_frames
    c = np.zeros_like(css); M = F.M.tocsr(); fr = 0
    while fr < cap:
        c = c + beta * (b - M @ c); fr += pass_frames
        if fr % 1200 == 0 and np.abs(c[nestk] - css[nestk]).sum() / norm < 0.1:
            return fr
    return None

for seed, f in [(1, 200000), (3, 200000)]:
    g, openm, sky, nest, geo, cells = P.setup(seed, f)
    s = H.srcs(seed, f, openm, 1, .25, 11, 0)
    rows = np.arange(g.shape[0])[:, None]
    for rule, eps, wd in VARIANTS:
        k, _ = P.kcell(g, rule, eps, seed, f)
        F = P.Field(openm, sky, k, wd)
        D = P.D_FRAME / (2.0 if wd > 0 else 1.0)
        lam, V = eigsh(F.M, k=K, sigma=0, which='LM')
        o = np.argsort(lam); lam, V = lam[o], V[:, o]
        nestk = nest[F.ky, F.kx]
        mound_open = (openm & (rows <= G.GROUND_ROW))[F.ky, F.kx]
        soil = (~openm)[F.ky, F.kx]
        sh = lambda v, m: float((v[m] ** 2).sum())
        b = s[F.ky, F.kx].astype(float)
        t10, remfrac, bound = t10_modes(F, b, nestk, lam, V, D)
        first_nest = next((i for i in range(K) if sh(V[:, i], nestk) >= 0.10), None)
        emit(f's{seed}@{f//1000}k {rule} eps={eps:g} diag={wd:g}: unknowns {F.M.shape[0]}; lambda_1 {lam[0]:.3e} -> tau_1 {1/(D*lam[0]):,.0f} frames;'
             f' v_1 share nest {sh(V[:,0], nestk):.2f} / mound passages {sh(V[:,0], mound_open):.2f} / soil {sh(V[:,0], soil):.2f};'
             f' first mode with >=10% in the nest: #{(first_nest + 1) if first_nest is not None else "none<=60"}'
             f'{"" if first_nest is None else f" tau {1/(D*lam[first_nest]):,.0f} frames"};'
             f' nest within 10% from empty: {t10:,.0f} frames (slow modes miss {remfrac*100:.2f}% of the field; remainder bound at t10 {bound*100:.3f}%)')
    if seed == 1:
        F = P.Field(openm, sky, P.kcell(g, 'R0', 0)[0])
        st = t10_step(F, s[F.ky, F.kx].astype(float), nest[F.ky, F.kx], P.D_FRAME)
        emit(f'  check, s1@200k R0 by explicit stepping (beta = D*12 per pass, tested every 1,200 frames): {st:,} frames')
        F = P.Field(openm, sky, P.kcell(g, 'R1', 0.1)[0])
        st = t10_step(F, s[F.ky, F.kx].astype(float), nest[F.ky, F.kx], P.D_FRAME)
        emit(f'  check, s1@200k R1 0.1 by explicit stepping: {st:,} frames')
with open(f'{P.OUT}/p3_settle.txt', 'w') as fh:
    fh.write('\n'.join(lines) + '\n')
