# Items 1-5 of the brief, per rule x eps x map. Run: python3 -I p1_rules.py <deps dir>
# Writes p1_permap.tsv (one row per variant x map) and prints/writes p1_summary.txt.
import sys, os, json
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import poro_lib as P
H, G = P.H, P.G

VARIANTS = ([('R0', 0.0, 0.0), ('R0', 0.0, 0.5)] + [('R1', e, 0.0) for e in P.EPS] + [('R1', 0.1, 0.5)]
            + [('R2', e, 0.0) for e in P.EPS] + [('R2all', e, 0.0) for e in P.EPS])
rows_out = []
for seed, f in P.MAPS:
    g, openm, sky, nest, geo, cells = P.setup(seed, f)
    s = H.srcs(seed, f, openm, 1, .25, 11, 0)
    room = P.room_mask(openm, cells); ys, xs = np.nonzero(room)
    rng = np.random.default_rng([seed, f])
    pick = rng.choice(len(ys), size=min(25, len(ys)), replace=False)
    pick_cells = [(ys[k], xs[k]) for k in pick]
    for rule, eps, wd in VARIANTS:
        k, info = P.kcell(g, rule, eps, seed, f)
        F = P.Field(openm, sky, k, wd)
        c = F.solve(s)
        m = P.metrics(c, openm, sky, geo, cells)
        row = {'rule': rule, 'eps': eps, 'wdiag': wd, 'seed': seed, 'frame': f, **info, **m}
        if m['open']:
            row['readout_clear'] = float(P.clear_out(c, openm, geo, cells)[cells].mean())
            row.update(P.census(F, c, s, cells, geo, openm, pick_cells))
            row['level_median'] = float(np.nanmedian(c[cells]))
        rows_out.append(row)
    print(f's{seed}@{f//1000}k done', flush=True)

keys = []
for r in rows_out:
    for kk in r:
        if kk not in keys:
            keys.append(kk)
with open(f'{P.OUT}/p1_permap.tsv', 'w') as fh:
    fh.write('\t'.join(keys) + '\n')
    for r in rows_out:
        fh.write('\t'.join(f'{r[kk]:.6g}' if isinstance(r.get(kk), float) else str(r.get(kk, '')) for kk in keys) + '\n')

R0_OPEN = {(r['seed'], r['frame']) for r in rows_out if r['rule'] == 'R0' and r['wdiag'] == 0 and r['open']}
lines = []
def emit(s):
    print(s); lines.append(s)
emit(f'maps: {len(P.MAPS)}; R0-open maps (paired subset): {len(R0_OPEN)}')
hdr = ('variant', 'out', 'sealed', 'span', 'read', 'read&out', 'LOScross', 'read&out&clear', 'fol0.5', 'fol1', 'fol2', 'folideal',
       'door/top', 'near6', 'near20', 'mound', 'rho_depth', 'rho_geo', 'level')
emit('\t'.join(hdr))
for rule, eps, wd in VARIANTS:
    rs = [r for r in rows_out if r['rule'] == rule and r['eps'] == eps and r['wdiag'] == wd]
    op = [r for r in rs if r['open']]
    for tag, sub in (('all-open', op), ('R0-open7', [r for r in op if (r['seed'], r['frame']) in R0_OPEN])):
        if not sub:
            continue
        V = lambda kk: [r[kk] for r in sub]
        sealed = [1 - r['n_finite'] / r['n_cells'] for r in sub]
        losf = [r['los_cross'] / r['los_tot'] for r in sub if r['los_tot'] > 0]
        pooled = f"{sum(r['los_cross'] for r in sub)}/{sum(r['los_tot'] for r in sub)}"
        emit('\t'.join([f'{rule} eps={eps:g} diag={wd:g} [{tag}, n={len(sub)}]', f'{len(op)}/{len(rs)}', P.q(sealed), P.q(V('span')), P.q(V('read')),
                        P.q(V('readout')), P.q(losf) + f' pooled {pooled}', P.q(V('readout_clear')), P.q(V('fol_0.5')), P.q(V('fol_1')), P.q(V('fol_2')),
                        P.q(V('fol_ideal')), P.q(V('door_top')), P.q(V('near6')), P.q(V('near20')), P.q(V('mound_share')),
                        P.q(V('rho_depth'), pct=False, nd=2), P.q(V('rho_geo'), pct=False, nd=2), P.q(V('level_median'), pct=False, nd=0)]))
with open(f'{P.OUT}/p1_summary.txt', 'w') as fh:
    fh.write('\n'.join(lines) + '\n')
