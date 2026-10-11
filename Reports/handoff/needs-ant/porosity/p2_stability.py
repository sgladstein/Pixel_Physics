# Item 6: refresh-to-refresh stability, expB's method (seed 1, maps 150k..250k every 1,000 frames,
# sources from that map only = what one refresh would see). Run: python3 -I p2_stability.py <deps dir>
# Requested: R0, R1 eps 0.02, R1 eps 0.1 (4-neighbour). Extra: R0 and R1 0.1 with diagonal 0.5
# (R0 diag 0.5 reproduces expB's second line). Extra per pair of consecutive maps: share of nest
# cells present in both whose 1-cell steepest-descent direction changes, and whose
# "reads >=1% and points out" status flips.
import sys, os
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import poro_lib as P
H, G = P.H, P.G
VARIANTS = [('R0', 0.0, 0.0), ('R1', 0.02, 0.0), ('R1', 0.1, 0.0), ('R0', 0.0, 0.5), ('R1', 0.1, 0.5)]
FR = list(range(150000, 250001, 1000))
per = {v: [] for v in VARIANTS}
for f in FR:
    g, openm, sky, nest, geo, cells = P.setup(1, f)
    s = H.srcs(1, f, openm, 1, .25, 1, 0)
    for v in VARIANTS:
        rule, eps, wd = v
        if not cells.any():
            per[v].append(dict(f=f, state='walk-door shut')); continue
        c = P.Field(openm, sky, P.kcell(g, rule, eps)[0], wd).solve(s)
        if not np.isfinite(c[cells]).any():
            per[v].append(dict(f=f, state='gas-sealed')); continue
        valid = openm & np.isfinite(c)
        Pd = np.pad(np.where(valid, c, np.inf), 1, constant_values=np.inf)
        stack = np.stack([Pd[1 + dy:1 + dy + c.shape[0], 1 + dx:1 + dx + c.shape[1]] for dy, dx in G.N8])
        dirn = np.where(stack.min(0) < c - 1e-15, stack.argmin(0), -1)
        _, bo = H.reads(np.where(openm, c, np.nan), openm, geo, 6, 4, 'rel')
        per[v].append(dict(f=f, state='open', level=float(np.nanmedian(c[cells])), cells=cells, dirn=dirn, ro=bo >= .01,
                           fin=cells & np.isfinite(c)))
lines = []
def emit(s):
    print(s); lines.append(s)
emit(f'seed 1, {len(FR)} maps {FR[0]}..{FR[-1]} every 1,000 frames, single-map sources')
for v in VARIANTS:
    L = per[v]
    lv = np.array([d.get('level', np.nan) for d in L])
    n_sealed = sum(d['state'] == 'gas-sealed' for d in L); n_shut = sum(d['state'] == 'walk-door shut' for d in L)
    r = np.abs(np.diff(np.log(lv))); r = r[np.isfinite(r)]
    dch = []; flip = []
    for a, b in zip(L[:-1], L[1:]):
        if a['state'] == 'open' and b['state'] == 'open':
            both = a['fin'] & b['fin']
            if both.any():
                dch.append(np.mean(a['dirn'][both] != b['dirn'][both])); flip.append(np.mean(a['ro'][both] != b['ro'][both]))
    pct = lambda x: f'{(np.exp(x) - 1) * 100:.0f}%'
    emit(f'{v[0]} eps={v[1]:g} diag={v[2]:g}: gas-sealed {n_sealed}/{len(FR)} (walk door shut {n_shut}); nest median level min {np.nanmin(lv):.0f} med {np.nanmedian(lv):.0f} max {np.nanmax(lv):.0f} (x{np.nanmax(lv)/np.nanmin(lv):.1f});'
         f' per-refresh |dlog| median {np.median(r):.3f} ({pct(np.median(r))}), p90 {np.quantile(r, .9):.3f} ({pct(np.quantile(r, .9))}), max {r.max():.3f} ({pct(r.max())});'
         f' jumps >25%: {(r > np.log(1.25)).sum()} of {r.size} refreshes with both maps open;'
         f' descent direction changes per refresh: median {np.median(dch)*100:.0f}% of cells; read&out status flips: median {np.median(flip)*100:.1f}%')
with open(f'{P.OUT}/p2_stability.txt', 'w') as fh:
    fh.write('\n'.join(lines) + '\n')
