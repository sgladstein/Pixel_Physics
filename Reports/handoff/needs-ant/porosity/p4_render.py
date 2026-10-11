# The picture: seed 1 at 200k, R0 (all soil sealed) beside R1 eps 0.1 (mound spoil passes 0.1).
# render2.py's style: top row = the field over NEST cells stretched over the nest's own range on a
# fixed dark->bright ramp (a full replace, never a blend); bottom row = green where the 1% sensor
# reads AND points out (helpers.reads), grey where it does not, red where the steepest descent over
# all 8 neighbours is a soil cell (render2's "slopes into soil"). Extras on the top row: light brown
# = spoil passing gas (R1 only); cyan squares = a cell where the exact steepest walkable descent of
# 5 or more nest cells ends when that is not the open sky (one-cell dead-end notches not marked). Run: python3 -I p4_render.py <deps dir>
import sys, os
import numpy as np
from PIL import Image, ImageDraw
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import poro_lib as P
H, G = P.H, P.G
seed, f = 1, 200000
g, openm, sky, nest, geo, cells = P.setup(seed, f)
s = H.srcs(seed, f, openm, 1, .25, 11, 0)
rows = np.arange(g.shape[0])[:, None]
panels = []
for rule, eps, title in (('R0', 0.0, 'R0: all soil sealed'), ('R1', 0.1, 'R1: mound spoil passes 0.1')):
    k, _ = P.kcell(g, rule, eps)
    c = P.Field(openm, sky, k).solve(s)
    m = P.metrics(c, openm, sky, geo, cells)
    _, bo = H.reads(np.where(openm, c, np.nan), openm, geo, 6, 4, 'rel')
    ty, tx, _ = P.follower_terms(c, openm, sky, cells, None)
    cnt = {}
    for y, x in zip(ty.tolist(), tx.tolist()):
        if not sky[y, x]:
            cnt[(y, x)] = cnt.get((y, x), 0) + 1
    ends = {kk: v for kk, v in cnt.items() if v >= 5}  # attractors only, not one-cell dead-end notches
    panels.append((rule, eps, title, k, c, m, bo, ends))

ys, xs = np.nonzero(nest)
spo = (g == 's') & (rows < G.GROUND_ROW)
r0 = max(0, int(np.nonzero(spo[:, max(0, xs.min() - 6):xs.max() + 7].any(1))[0].min()) - 2)
r1 = min(g.shape[0], ys.max() + 4)
c0, c1 = max(0, xs.min() - 6), min(g.shape[1], xs.max() + 7)
cell = 5
W, Hh = (c1 - c0) * cell, (r1 - r0) * cell
pad, lab = 10, 44
img = Image.new('RGB', (3 * (W + pad), 2 * (Hh + lab) + 10), (255, 255, 255))
dr = ImageDraw.Draw(img)
col = {'s': (140, 105, 70), '#': (90, 70, 50), '.': (20, 20, 28), 'a': (230, 60, 50), 'b': (245, 245, 235),
       'f': (60, 170, 60), 'c': (200, 190, 80), 'x': (120, 40, 120), '~': (40, 90, 200)}

def ramp(t):
    t = min(max(t, 0.0), 1.0)
    return (int(255 * min(1, 1.6 * t)), int(255 * max(0, min(1, 1.6 * t - 0.4))), int(255 * max(0, 2.5 * t - 1.5)))

def box(ox, oy, y, x, fill):
    dr.rectangle([ox + (x - c0) * cell, oy + (y - r0) * cell, ox + (x - c0 + 1) * cell - 1, oy + (y - r0 + 1) * cell - 1], fill=fill)

def gline(ox, oy):
    gy = oy + (G.GROUND_ROW - r0) * cell + cell // 2
    dr.line([ox, gy, ox + W, gy], fill=(80, 160, 255))

# column 0: the map, and the legend
for y in range(r0, r1):
    for x in range(c0, c1):
        box(0, 0, y, x, col.get(g[y, x], (255, 0, 255)))
gline(0, 0)
dr.text((2, Hh + 3), f'seed {seed}, frame {f}: red ant, white brood,', fill=(0, 0, 0))
dr.text((2, Hh + 15), 'brown spoil/soil; blue line = ground line', fill=(0, 0, 0))
oy = Hh + lab + 10
for i, t in enumerate(['Top: nest air over NEST cells on the nest\'s', 'own range (dark = low, bright = high).',
                       'Light brown = spoil passing gas. Cyan = where', '5+ nest cells\' exact walkable descent ends.',
                       'Bottom: green = the 6-cell sensor reads >= 1%', 'AND that heading leads out; grey = flat to', 'it; red = steepest descent goes into soil.',
                       'Sources: ants 1, brood 0.25, 11 maps', f'{f - 5000}-{f + 5000}; sink = open sky over nothing.']):
    dr.text((2, oy + 3 + 12 * i), t, fill=(0, 0, 0))
for j, (rule, eps, title, k, c, m, bo, ends) in enumerate(panels):
    ox = (j + 1) * (W + pad)
    nv = c[cells]; nv = nv[np.isfinite(nv)]
    lo, hi = nv.min(), nv.max()
    for y in range(r0, r1):
        for x in range(c0, c1):
            if cells[y, x] and np.isfinite(c[y, x]):
                box(ox, 0, y, x, ramp((c[y, x] - lo) / max(hi - lo, 1e-12)))
            elif openm[y, x]:
                box(ox, 0, y, x, (40, 40, 60))
            elif k[y, x] > 0:
                box(ox, 0, y, x, (175, 145, 110))
            else:
                box(ox, 0, y, x, (70, 55, 40))
    for (y, x) in ends:
        if r0 <= y < r1 and c0 <= x < c1:
            dr.rectangle([ox + (x - c0) * cell - 2, (y - r0) * cell - 2, ox + (x - c0 + 1) * cell + 1, (y - r0 + 1) * cell + 1], outline=(0, 255, 255), width=2)
    gline(ox, 0)
    dr.text((ox + 2, Hh + 3), title, fill=(0, 0, 0))
    dr.text((ox + 2, Hh + 15), f'nest level {lo:.0f}-{hi:.0f}: spans {100 * (hi - lo) / hi:.1f}% of its top', fill=(0, 0, 0))
    att = sum(ends.values()) / cells.sum()
    dr.text((ox + 2, Hh + 27), f'exact descent: sky {100 * m["fol_ideal"]:.1f}%, cyan cell {100 * att:.1f}% of nest cells', fill=(0, 0, 0))
    # bottom row
    allv = np.stack([np.pad(np.where(np.isfinite(c), -c, np.nan), 1, constant_values=np.nan)[1 + dy:1 + dy + c.shape[0], 1 + dx:1 + dx + c.shape[1]] for dy, dx in G.N8])
    allv = np.where(np.isnan(allv), -np.inf, allv)
    bi = allv.argmax(0)
    nread = nwall = ncell = 0
    for y in range(r0, r1):
        for x in range(c0, c1):
            if cells[y, x]:
                ncell += 1
                dy, dx = G.N8[bi[y, x]]
                yy, xx = y + dy, x + dx
                wallward = np.isfinite(c[y, x]) and (not openm[yy, xx]) and np.isfinite(c[yy, xx]) and c[yy, xx] < c[y, x]
                if wallward:
                    fill = (220, 40, 40); nwall += 1
                elif bo[y, x] >= 0.01:
                    fill = (60, 190, 90); nread += 1
                else:
                    fill = (150, 150, 150)
                box(ox, oy, y, x, fill)
            elif openm[y, x]:
                box(ox, oy, y, x, (40, 40, 60))
            elif k[y, x] > 0:
                box(ox, oy, y, x, (175, 145, 110))
            else:
                box(ox, oy, y, x, (70, 55, 40))
    gline(ox, oy)
    dr.text((ox + 2, oy + Hh + 3), f'reads >= 1% and points out: {100 * nread / ncell:.1f}% of nest cells', fill=(0, 0, 0))
    dr.text((ox + 2, oy + Hh + 15), f'slopes into soil: {100 * nwall / ncell:.1f}%;  1% follower reaches sky {100 * m["fol_1"]:.1f}%', fill=(0, 0, 0))
    print(f'{rule} eps {eps}: nest {lo:.0f}-{hi:.0f}, read&out {100*nread/ncell:.1f}% (metrics: {100*m["readout"]:.1f}%), into soil {100*nwall/ncell:.1f}%, non-sky attractors {ends}')
img.save(f'{P.OUT}/porosity_s1_f200000.png')
print('crop rows', r0, r1, 'cols', c0, c1, 'size', img.size)
