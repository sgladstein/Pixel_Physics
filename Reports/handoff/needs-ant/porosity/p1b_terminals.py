# Where does the slope lead? For each variant and map, classify the end of every nest cell's
# steepest walkable descent (ideal, and the 1% follower): open sky / a covered mound passage
# (open, at or above the ground line, not sky) / inside the nest (a local minimum below ground)
# / never moved. For mound and nest terminals: walk steps left to the sky from there (geodesic).
# Run: python3 -I p1b_terminals.py <deps dir>
import sys, os
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import poro_lib as P
H, G = P.H, P.G
VARIANTS = [('R0', 0.0, 0.0), ('R0', 0.0, 0.5)] + [('R1', e, 0.0) for e in P.EPS] + [('R1', 0.1, 0.5)] + [('R2', e, 0.0) for e in P.EPS]
lines = []
def emit(s):
    print(s); lines.append(s)
agg = {}
for seed, f in P.MAPS:
    g, openm, sky, nest, geo, cells = P.setup(seed, f)
    s = H.srcs(seed, f, openm, 1, .25, 11, 0)
    g0 = geo[cells]
    for rule, eps, wd in VARIANTS:
        k, _ = P.kcell(g, rule, eps, seed, f)
        c = P.Field(openm, sky, k, wd).solve(s)
        if not np.isfinite(c[cells]).any():
            continue
        for tag, thr in (('ideal', None), ('1%', 0.01)):
            ty, tx, st = P.follower_terms(c, openm, sky, cells, thr)
            fin = np.isfinite(c[cells])
            at_sky = sky[ty, tx]
            mound = ~at_sky & (ty <= G.GROUND_ROW)
            innest = ~at_sky & (ty > G.GROUND_ROW) & (st > 0)
            still = ~at_sky & (st == 0)
            tg = geo[ty, tx]
            nterm = len(set(zip(ty[mound | innest].tolist(), tx[mound | innest].tolist())))
            frac_left = np.median(tg[mound | innest] / np.maximum(g0[mound | innest], 1)) if (mound | innest).any() else np.nan
            rec = dict(sky=at_sky.mean(), mound=mound.mean(), nest=innest.mean(), still=still.mean(),
                       mound_geo=np.median(tg[mound]) if mound.any() else np.nan,
                       nest_geo=np.median(tg[innest]) if innest.any() else np.nan, nterm=nterm, frac_left=frac_left,
                       start_geo=np.median(g0))
            agg.setdefault((rule, eps, wd, tag), []).append(((seed, f), rec))
emit('variant | follower | maps | ends at sky | ends in a covered mound passage | ends at a local minimum in the nest | never moves | walk steps left from mound end | from nest end | start median steps | distinct non-sky end cells per map | share of the start walk still left at a non-sky end')
for key, lst in agg.items():
    rule, eps, wd, tag = key
    R = [r for _, r in lst]
    emit(f'{rule} eps={eps:g} diag={wd:g} | {tag} | {len(R)} | {P.q([r["sky"] for r in R])} | {P.q([r["mound"] for r in R])} | {P.q([r["nest"] for r in R])} | '
         f'{P.q([r["still"] for r in R])} | {P.q([r["mound_geo"] for r in R], pct=False, nd=0)} | {P.q([r["nest_geo"] for r in R], pct=False, nd=0)} | '
         f'{P.q([r["start_geo"] for r in R], pct=False, nd=0)} | {P.q([r["nterm"] for r in R], pct=False, nd=0)} | {P.q([r["frac_left"] for r in R])}')
with open(f'{P.OUT}/p1b_terminals.txt', 'w') as fh:
    fh.write('\n'.join(lines) + '\n')
