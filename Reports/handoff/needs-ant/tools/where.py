import sys, numpy as np
sys.path.insert(0, sys.argv[1])
import gradcheck as G
rows_out = []
def nb8(m):
    o = np.zeros_like(m)
    h, w = m.shape
    for dy in (-1,0,1):
        for dx in (-1,0,1):
            if dy==0 and dx==0: continue
            s = np.zeros_like(m)
            s[max(0,dy):h+min(0,dy), max(0,dx):w+min(0,dx)] = m[max(0,-dy):h+min(0,-dy), max(0,-dx):w+min(0,-dx)]
            o |= s
    return o
print("seed frame | ants nest/mound/sky | brood nest/mound/sky | food+crumb in nest (touch brood) | air_sealed: hi95 cells, brood in hi95/brood in nest, hi95 median rows-below-ground vs brood vs food | L at brood p50, at food p50")
for s in (1,2,3,4):
    for f in (100000,200000,295000):
        g = G.load(f'{G.BASE}/s{s}/map_f{f:06d}.txt')
        openm, sky, nest, mound = G.masks(g)
        geo = G.geodesic(openm, sky)
        reach = nest & (geo >= 0)
        A = lambda ch, m: int(((g==ch) & m).sum())
        ants = (A('a',nest), A('a',mound), A('a',sky))
        brood = (A('b',nest), A('b',mound), A('b',sky))
        rows = np.arange(g.shape[0])[:,None] * np.ones(g.shape, int)
        below = rows > G.GROUND_ROW
        foodin = ((g=='f') & below & nb8(reach)) | ((g=='c') & reach)
        btouch = nb8(g=='b')
        nfood = int(foodin.sum()); ftb = int((foodin & btouch).sum())
        line = f"s{s} f{f//1000}k | {ants} | {brood} | {nfood} ({ftb})"
        if reach.sum() == 0:
            print(line + " | door shut"); continue
        src = G.avg_sources(s, f, openm, 'air')
        c, sealed = G.solve(openm, sky, src, 0.0, 0.0)
        v = np.where(reach & ~np.isnan(c), c, np.nan)
        if np.all(np.isnan(v)):
            print(line + " | sealed-air NaN (corner gap)"); continue
        L = v / np.nanmax(v)
        hi = (L >= 0.95)
        bn = (g=='b') & reach
        rb = lambda m: float(np.median(rows[m] - G.GROUND_ROW)) if m.sum() else float('nan')
        fadj = nb8(foodin) & reach & ~np.isnan(L)
        Lb = float(np.nanmedian(L[bn])) if bn.sum() else float('nan')
        Lf = float(np.nanmedian(L[fadj])) if fadj.sum() else float('nan')
        print(line + f" | {int(hi.sum())}/{int(reach.sum())}, {int((hi&bn).sum())}/{int(bn.sum())}, rows {rb(hi):.0f} vs {rb(bn):.0f} vs {rb(foodin):.0f} | {Lb:.3f}, {Lf:.3f}")
