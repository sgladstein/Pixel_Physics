# Cost of keeping a sealed-wall nest-air field current: red-black SOR sweeps,
# warm-started from the field 1,000 frames earlier, to reach 1% of the direct
# solve on the new map. Cold start (from zero) for comparison.
import numpy as np, sys
sys.argv = ['x']
exec(open('gradcheck.py').read().split("if __name__")[0])

def sor(openm, sky, src, c0, target, omega=1.9, tol=0.01, max_sweeps=20000):
    h, w = openm.shape
    unk = openm & ~sky
    c = np.where(unk, np.nan_to_num(c0), 0.0)
    G = {}
    for dy, dx in N4:
        nbo = np.zeros((h, w), bool)
        ys, xs, ny, nx = pairs(h, w, dy, dx)
        nbo[ys, xs] = openm[ny, nx]
        G[(dy, dx)] = nbo & openm
    deg = sum(G.values()).astype(float)
    ys, xs = np.mgrid[0:h, 0:w]
    red = ((ys + xs) % 2 == 0)
    tmask = unk & np.isfinite(target)
    tv = target[tmask]
    for s in range(1, max_sweeps + 1):
        for colour in (red, ~red):
            acc = np.zeros((h, w))
            for (dy, dx), m in G.items():
                sh = np.zeros((h, w))
                y0s, y1s = max(0, -dy), h - max(0, dy)
                x0s, x1s = max(0, -dx), w - max(0, dx)
                sh[y0s:y1s, x0s:x1s] = c[y0s + dy:y1s + dy, x0s + dx:x1s + dx]
                acc += np.where(m, sh, 0.0)
            upd = unk & colour & (deg > 0)
            new = (acc + src) / np.where(deg > 0, deg, 1.0)
            c = np.where(upd, (1 - omega) * c + omega * new, c)
        if s % 5 == 0:
            err = np.max(np.abs(c[tmask] - tv) / np.abs(tv).max())
            if err < tol:
                return s
    return None

for seed, f in ((1, 200000), (2, 100000), (3, 200000), (4, 100000)):
    g0 = load(f'{BASE}/s{seed}/map_f{f - 1000:06d}.txt'); g1 = load(f'{BASE}/s{seed}/map_f{f:06d}.txt')
    o0, s0, n0, m0 = masks(g0); o1, s1_, n1, m1 = masks(g1)
    a0 = avg_sources(seed, f - 1000, o0, 'air'); a1 = avg_sources(seed, f, o1, 'air')
    c_prev, _ = solve(o0, s0, a0, 0.0, 0.0)
    c_new, sealed = solve(o1, s1_, a1, 0.0, 0.0)
    opened = int((o1 & ~o0).sum()); closed = int((o0 & ~o1).sum())
    unk = o1 & ~s1_
    warm = sor(o1, s1_, a1, np.where(np.isfinite(c_prev), c_prev, 0.0), c_new)
    cold = sor(o1, s1_, a1, np.zeros(o1.shape), c_new)
    print(f'seed {seed} f{f}: open cells solved {int((unk & np.isfinite(c_new)).sum())}, cells opened/closed in 1000 frames {opened}/{closed}, sweeps to 1%: warm {warm}, cold {cold}')
