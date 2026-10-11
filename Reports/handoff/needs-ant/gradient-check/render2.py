# Picture for the doc: inside the dug nest, each field stretched over the nest's
# own range (so a flat room shows as flat only if it really is), and a second
# row showing where an ant's 6-cell sensor reads >= 1% contrast (green) or not
# (grey), with cells whose steepest descent points into soil in red.
import numpy as np, sys, json
sys.argv = ['x']
exec(open('gradcheck.py').read().split("if __name__")[0])
from PIL import Image, ImageDraw, ImageFont

def ramp(t):
    t = min(max(t, 0.0), 1.0)
    return (int(255 * min(1, 1.6 * t)), int(255 * max(0, min(1, 1.6 * t - 0.4))), int(255 * max(0, 2.5 * t - 1.5)))

def panel_pics(seed, f, out):
    g = load(f'{BASE}/s{seed}/map_f{f:06d}.txt')
    openm, sky, nest, mound = masks(g)
    geo = geodesic(openm, sky)
    air = avg_sources(seed, f, openm, 'air')
    larva = avg_sources(seed, f, openm, 'larva')
    F = {}
    F['air, sealed walls'] = solve(openm, sky, air, 0.0, 0.0)[0]
    F['air, walls leak 2%'] = solve(openm, sky, air, 0.02, 0.0)[0]
    F['air, walls ignored (today\'s planes)'] = solve(openm, sky, air, 1.0, 0.0)[0]
    F['larva scent, 10-cell reach'] = solve(openm, sky, larva, 0.0, 1 / 100)[0]
    cells = list(zip(*np.nonzero(nest & (geo >= 0))))
    ys, xs = np.nonzero(nest)
    r0, r1 = GROUND_ROW - 4, min(g.shape[0], ys.max() + 4)
    c0, c1 = max(0, xs.min() - 6), min(g.shape[1], xs.max() + 7)
    cell = 5
    W, H = (c1 - c0) * cell, (r1 - r0) * cell
    names = ['map', 'walk out (geodesic)'] + list(F.keys())
    pad, lab = 10, 30
    img = Image.new('RGB', (len(names) * (W + pad), 2 * (H + lab) + 10), (255, 255, 255))
    dr = ImageDraw.Draw(img)
    col = {'s': (140, 105, 70), '#': (90, 70, 50), '.': (20, 20, 28), 'a': (230, 60, 50), 'b': (245, 245, 235),
           'f': (60, 170, 60), 'c': (200, 190, 80), 'x': (120, 40, 120), '~': (40, 90, 200)}
    def box(ox, oy, y, x, fill):
        dr.rectangle([ox + (x - c0) * cell, oy + (y - r0) * cell, ox + (x - c0 + 1) * cell - 1, oy + (y - r0 + 1) * cell - 1], fill=fill)
    for k, name in enumerate(names):
        ox = k * (W + pad)
        # row 1: shape on the nest's own range
        if name == 'map':
            for y in range(r0, r1):
                for x in range(c0, c1):
                    box(ox, 0, y, x, col.get(g[y, x], (255, 0, 255)))
            dr.text((ox + 2, H + 3), f'seed {seed}, frame {f}', fill=(0, 0, 0))
            dr.text((ox + 2, H + 15), 'red ant, white brood', fill=(0, 0, 0))
        else:
            v = np.where(geo >= 0, geo.astype(float), np.nan) if name.startswith('walk') else F[name]
            nv = np.array([v[p] for p in cells])
            nv = nv[np.isfinite(nv)]
            if nv.size:
                lo, hi = nv.min(), nv.max()
                rng = f'room spans {100 * (hi - lo) / hi:.0f}% of its top' if not name.startswith('walk') else f'{int(lo)}-{int(hi)} steps out'
            else:
                lo, hi, rng = 0, 1, 'no way out in this field'
            for y in range(r0, r1):
                for x in range(c0, c1):
                    if openm[y, x] and nest[y, x] and np.isfinite(v[y, x]):
                        box(ox, 0, y, x, ramp((v[y, x] - lo) / max(hi - lo, 1e-12)))
                    elif openm[y, x]:
                        box(ox, 0, y, x, (40, 40, 60))
                    else:
                        box(ox, 0, y, x, (70, 55, 40))
            dr.text((ox + 2, H + 3), name, fill=(0, 0, 0))
            dr.text((ox + 2, H + 15), rng, fill=(0, 0, 0))
        # row 2: readable at the 6-cell sensor, and wallward descent
        oy = H + lab + 10
        if name in F:
            c = F[name]
            sc = sensor_contrast(c, openm, cells)
            allv = neighbour_stack(-c, np.isfinite(c))
            allv = np.where(np.isnan(allv), -np.inf, allv)
            bi = allv.argmax(axis=0)
            nread = 0; nwall = 0
            for y in range(r0, r1):
                for x in range(c0, c1):
                    if (y, x) in sc:
                        dy, dx = N8[bi[y, x]]
                        wallward = (not openm[y + dy, x + dx]) and np.isfinite(c[y + dy, x + dx]) and c[y + dy, x + dx] < c[y, x]
                        if wallward:
                            fill = (220, 40, 40); nwall += 1
                        elif sc[(y, x)] >= 0.01:
                            fill = (60, 190, 90); nread += 1
                        else:
                            fill = (150, 150, 150)
                        box(ox, oy, y, x, fill)
                    elif openm[y, x]:
                        box(ox, oy, y, x, (40, 40, 60))
                    else:
                        box(ox, oy, y, x, (70, 55, 40))
            n = len(sc)
            dr.text((ox + 2, oy + H + 3), f'readable {100 * nread / n:.0f}%, into soil {100 * nwall / n:.0f}%', fill=(0, 0, 0))
        elif name == 'map':
            dr.text((ox + 2, oy + 3), 'Row 2: green = an ant\'s 6-cell', fill=(0, 0, 0))
            dr.text((ox + 2, oy + 15), 'sensor reads >= 1% difference;', fill=(0, 0, 0))
            dr.text((ox + 2, oy + 27), 'grey = flat to it; red = the', fill=(0, 0, 0))
            dr.text((ox + 2, oy + 39), 'field slopes down into soil.', fill=(0, 0, 0))
    img.save(out)

for seed, f in ((1, 200000), (3, 295000)):
    panel_pics(seed, f, f'/mnt/project-files/needs-ant/gradient-check/inside-nest_s{seed}_f{f:06d}.png')
print('ok')
