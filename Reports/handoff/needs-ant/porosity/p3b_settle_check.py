# Check p3_settle.py's mode-based "nest within 10% from empty" for R2 by explicit stepping
# (s1@200k; R2 eps 0.02 had a 14% remainder bound, so its mode estimate is not trusted).
# Run: python3 -I p3b_settle_check.py <deps dir>
import sys, os
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import poro_lib as P
H, G = P.H, P.G
g, openm, sky, nest, geo, cells = P.setup(1, 200000)
s = H.srcs(1, 200000, openm, 1, .25, 11, 0)
for eps in (0.02, 0.1):
    F = P.Field(openm, sky, P.kcell(g, 'R2', eps, 1, 200000)[0])
    b = s[F.ky, F.kx].astype(float); nk = nest[F.ky, F.kx]
    css = F.lu.solve(b); norm = np.abs(css[nk]).sum(); beta = P.D_FRAME * 12
    M = F.M.tocsr(); c = np.zeros_like(css); fr = 0; hit = None; marks = {}
    while fr < 12_000_000:
        c = c + beta * (b - M @ c); fr += 12
        if fr % 1200 == 0:
            e = np.abs(c[nk] - css[nk]).sum() / norm
            for t in (0.5, 0.25, 0.1):
                if t not in marks and e < t:
                    marks[t] = fr
            if e < 0.1:
                break
    print(f's1@200k R2 eps {eps}: nest within 50% {marks.get(0.5)}, 25% {marks.get(0.25)}, 10% {marks.get(0.1)} frames (explicit, tested every 1,200)', flush=True)
