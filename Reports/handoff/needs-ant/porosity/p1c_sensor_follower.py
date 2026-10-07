# EXTRA (not in the brief): a follower that uses the ant's own 6-cell sensor instead of a 1-cell
# drop. At each cell take the heading whose sample (6 cells ahead, 4 on a diagonal; sample must be
# open with a value, no line-of-sight test, as the shipped sensor) is lowest relative to here,
# (here - front)/(here + front), AMONG headings whose next cell is walkable; step one cell that
# way if that contrast >= thr; else stop. (A first version took the best heading even when its
# next cell was soil and stopped there; it reached the sky from 0% everywhere for that reason.) Deterministic per cell, so followed by pointer jumping, 400-step cap
# (cycles allowed and counted as not reaching). Run: python3 -I p1c_sensor_follower.py <deps dir>
import sys, os
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import poro_lib as P
H, G = P.H, P.G
VARIANTS = [('R0', 0.0, 0.0), ('R0', 0.0, 0.5)] + [('R1', e, 0.0) for e in P.EPS] + [('R1', 0.1, 0.5)] + [('R2', e, 0.0) for e in P.EPS]

def sensor_follow(c, openm, sky, cells, thr, cap=400):
    h, w = c.shape
    co = np.where(openm & np.isfinite(c), c, np.nan)
    bestr = np.full((h, w), -np.inf); bestd = np.zeros((h, w), int)
    for i, (dy, dx) in enumerate(G.N8):
        s = 6 if (dy == 0 or dx == 0) else 4
        fv = H.shift(co, s * dy, s * dx, np.nan)
        with np.errstate(all='ignore'):
            r = (co - fv) / (co + fv)
        r = np.where(np.isfinite(r), r, -np.inf)
        r = np.where(H.shift(openm, dy, dx, False), r, -np.inf)  # next cell must be walkable
        better = r > bestr
        bestr = np.where(better, r, bestr); bestd = np.where(better, i, bestd)
    dys = np.array([d[0] for d in G.N8]); dxs = np.array([d[1] for d in G.N8])
    ys, xs = np.mgrid[0:h, 0:w]
    ty = np.clip(ys + dys[bestd], 0, h - 1); tx = np.clip(xs + dxs[bestd], 0, w - 1)
    ok = (bestr >= thr) & openm[ty, tx] & np.isfinite(co)
    ny = np.where(ok, ty, ys); nx = np.where(ok, tx, xs)
    py, px = np.nonzero(cells); moved_any = np.zeros(py.size, bool)
    for _ in range(cap):
        stop = sky[py, px]
        qy = np.where(stop, py, ny[py, px]); qx = np.where(stop, px, nx[py, px])
        mv = (qy != py) | (qx != px)
        moved_any |= mv
        if not mv.any():
            break
        py, px = qy, qx
    return sky[py, px], moved_any, py

lines = []
def emit(s):
    print(s, flush=True); lines.append(s)
res = {}
for seed, f in P.MAPS:
    g, openm, sky, nest, geo, cells = P.setup(seed, f)
    s = H.srcs(seed, f, openm, 1, .25, 11, 0)
    for v in VARIANTS:
        rule, eps, wd = v
        c = P.Field(openm, sky, P.kcell(g, rule, eps, seed, f)[0], wd).solve(s)
        if not np.isfinite(c[cells]).any():
            continue
        for thr in (0.005, 0.01, 0.02):
            reach, moved, ty = sensor_follow(c, openm, sky, cells, thr)
            res.setdefault((v, thr), []).append((reach.mean(), moved.mean(), np.mean(~reach & moved & (ty <= G.GROUND_ROW))))
emit('6-cell-sensor follower: share of nest cells reaching open sky | share that move at all | share ending in a covered mound passage; medians [range] over open maps')
for v in VARIANTS:
    parts = []
    for thr in (0.005, 0.01, 0.02):
        L = res.get((v, thr), [])
        parts.append(f'thr {thr*100:.1f}%: sky {P.q([a for a,_,_ in L])}, moves {P.q([b for _,b,_ in L])}, mound {P.q([m for _,_,m in L])}')
    emit(f'{v[0]} eps={v[1]:g} diag={v[2]:g} (n={len(res.get((v,0.01),[]))}): ' + ' | '.join(parts))
with open(f'{P.OUT}/p1c_sensor_follower.txt', 'w') as fh:
    fh.write('\n'.join(lines) + '\n')
