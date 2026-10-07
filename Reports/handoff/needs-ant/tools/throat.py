import sys, numpy as np
sys.path.insert(0, sys.argv[1])
import gradcheck as G
from scipy.stats import spearmanr
print("map | nest cells | share L>=0.99 | walk-steps-out: first cell with L>=0.95 (min), nest p50, brood p50 | spearman(L, steps out) | rows below ground of first L>=0.95 cells (p50 of 10 nearest the door)")
for s in (1,2,3,4):
    for f in (100000,200000,295000):
        g = G.load(f'{G.BASE}/s{s}/map_f{f:06d}.txt')
        openm, sky, nest, mound = G.masks(g)
        geo = G.geodesic(openm, sky)
        reach = nest & (geo >= 0)
        if reach.sum() == 0: continue
        c, _ = G.solve(openm, sky, G.avg_sources(s, f, openm, 'air'), 0.0, 0.0)
        ok = reach & ~np.isnan(c)
        if ok.sum() == 0: continue
        L = c / np.nanmax(c[ok])
        hi = ok & (L >= 0.95)
        gh = geo[hi]; order = np.argsort(gh)[:10]
        rows = np.nonzero(hi)[0][order] - G.GROUND_ROW
        bn = ok & (g == 'b')
        rho = spearmanr(L[ok], geo[ok]).correlation
        print(f"s{s} f{f//1000}k | {int(ok.sum())} | {(L[ok]>=0.99).mean():.3f} | {int(gh.min())}, {int(np.median(geo[ok]))}, {int(np.median(geo[bn])) if bn.sum() else -1} | {rho:.2f} | {int(np.median(rows))}")
