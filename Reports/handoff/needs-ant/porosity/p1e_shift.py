# Is mound porosity (R1) a lever on the nest's SLOPE or only on its LEVEL? If the nest's only gas
# connection is its door, the nest field under R1 should be R0's field minus a near-constant, so
# absolute differences between nest cells (what a slope is) stay put and only the denominator of the
# sensor's relative contrast (f-c)/(f+c) shrinks. Measures, over nest cells with a value under both:
# the shift (R0 - R1) median, its spread (p95-p5) against R0's nest span (max-min), and the same for R2.
# Also: readable share if the sensor read R0's field with the R1 level subtracted (pure baseline shift).
# Run: python3 -I p1e_shift.py <deps dir>
import sys, os
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import poro_lib as P
H, G = P.H, P.G
lines = []
for seed, f in P.MAPS:
    g, openm, sky, nest, geo, cells = P.setup(seed, f)
    s = H.srcs(seed, f, openm, 1, .25, 11, 0)
    c0 = P.Field(openm, sky, P.kcell(g, 'R0', 0)[0]).solve(s)
    if not np.isfinite(c0[cells]).any():
        continue
    out = []
    for rule, eps in (('R1', 0.02), ('R1', 0.1), ('R1', 0.3), ('R2', 0.1)):
        c1 = P.Field(openm, sky, P.kcell(g, rule, eps, seed, f)[0]).solve(s)
        both = cells & np.isfinite(c0) & np.isfinite(c1)
        d = (c0 - c1)[both]
        span0 = np.nanmax(c0[both]) - np.nanmin(c0[both])
        spread = np.quantile(d, .95) - np.quantile(d, .05)
        # pure baseline shift: R0 field minus R1's median shift, read by the same sensor
        cs = np.where(np.isfinite(c0), c0 - np.median(d), np.nan)
        best_s, bo_s = H.reads(np.where(openm, cs, np.nan), openm, geo, 6, 4, 'rel')
        best_1, bo_1 = H.reads(np.where(openm, c1, np.nan), openm, geo, 6, 4, 'rel')
        out.append(f'{rule} {eps:g}: shift {np.median(d):.0f}, spread/R0 span {spread/span0*100:.2f}%, read&out shifted-R0 {np.mean(bo_s[cells]>=.01)*100:.1f}% vs solved {np.mean(bo_1[cells]>=.01)*100:.1f}%')
    lines.append(f's{seed}@{f//1000}k (R0 nest span {span0:.0f}): ' + ' | '.join(out))
print('\n'.join(lines))
open(f'{P.OUT}/p1e_shift.txt', 'w').write('\n'.join(lines) + '\n')
