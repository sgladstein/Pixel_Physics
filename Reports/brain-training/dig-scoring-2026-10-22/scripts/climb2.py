"""Climb fix, take 2: the score as built, plus **niches** -- what is left of the nest after the chambers, opened by a
7x1 bar (at least 7 wide in every row), with at most two openings into the rest of the nest (a dead end, or a bulge
on a passage), a flat floor, and width at least 3x height -- graded on height from one row. So a chamber pays as it
is dug, and a corridor (many openings) does not."""
import sys, json, math; sys.path.insert(0, "/home/user/Pixel_Physics/Reports/brain-training/teacher-training/measurements/blueprints")
sys.path.insert(0, ".")
import numpy as np, nestscore as ns, render_blueprints as rb
exec(open('climb.py').read().split('P = dict')[0].split('from nestscore import')[1].split('\n',1)[1])
from nestscore import ramp, band, components, erode_square, dilate_square, runs


NICHE = dict(bar=(3, 0), min_cells=7, h_zero=0.0, h_full=6.0, open_full=2, open_zero=3, free=120.0, share=0.5)

def score(open_, ground, door_x, N=NICHE, verbose=False):
    s, parts = ns.score(open_, ground, door_x)
    nest, cham = parts["nest"], parts["chambers"]
    rest = nest & ~cham
    opened = dilate_square(erode_square(rest, N["bar"]), N["bar"]) & rest
    _, comps = components(opened)
    niches, nm, niche = [], np.zeros_like(nest), 0.0
    for cells in comps:
        if len(cells) < N["min_cells"]: continue
        m = np.zeros_like(nest); ys = np.array([c[0] for c in cells]); xs = np.array([c[1] for c in cells]); m[ys, xs] = True
        cols = {}
        for y, x in cells: cols.setdefault(x, []).append(y)
        heights = np.array([len(v) for v in cols.values()]); floors = np.array([max(v) for v in cols.values()])
        full = heights >= 0.5 * heights.max()
        H = float(np.median(heights[full])); W = float(len(cols)); fl = np.median(floors[full])
        flat = float(np.mean(np.abs(floors[full] - fl) <= 1))
        k = openings(m, nest)
        shape = ramp(W / H, 1.0, 3.0) * (0.5 + 0.5 * flat) * ramp(k, N["open_zero"], N["open_full"])
        g = ramp(H, N["h_zero"], N["h_full"]) * shape
        niches.append((len(cells), H, W, k, round(g, 2)))
        if shape > 0:
            nm |= m
            niche += g * len(cells)
    # rooms under way: at most a fixed allowance, or half what the finished chambers earn
    s["chamber_credit"] += min(niche, max(N["free"], N["share"] * s["chamber_credit"]))
    # niche cells are no longer tunnel (nor crack)
    tun = rest & ~nm
    width = parts["width"]
    wv = width[tun]; tg = np.array([band(v, (3, 6), (1, 10)) for v in wv]) if len(wv) else np.zeros(0)
    crack = int(np.sum(wv < 3))
    cc = s["chamber_credit"]
    tc = min(float(tg.sum()), max(60.0, cc))
    total = cc + 0.5 * tc - (0.5 * crack + s["pit"] + 0.5 * s["stray"])
    if verbose: print("   niches", niches)
    if verbose == 'parts': return round(total, 1), parts, nm, tun & (width < 3)
    return round(total, 1)

spec = json.load(open("/home/user/Pixel_Physics/Reports/brain-training/teacher-training/measurements/blueprints/nest-blueprint-specs.json"))
cases = [(b["id"], ns.grid_of(rb.build(rb.owner_scale(b)))) for b in spec["blueprints"]]
cases.append(("first-drawn harvester (5-cell)", ns.grid_of(rb.build(spec["blueprints"][0]))))
cases.append(("ladder 8-16", ns.grid_of(rb.build(None, ladder=True))))
cases += [(k, ns.grid_of(v)) for k, v in ns.bad_shapes(rb).items()]
V = "-v" in sys.argv
for name, g in cases:
    a = ns.score(g, 64, 128)[0]["score"]
    print(f"{name:40s} as built {a:7.1f}   with niches {score(g, 64, 128, verbose=V):7.1f}")
G, X0 = 64, 128
for wid in (28, 14, 7):
    m = np.zeros((128, 256), bool); m[:G] = True; m[G:G + 16, X0 - 2:X0 + 2] = True
    out = [(0, ns.score(m, G, X0)[0]["score"], score(m, G, X0))]
    for r in range(8):
        m[G + 15 - r, X0 + 2:X0 + 2 + wid] = True
        out.append((r + 1, ns.score(m, G, X0)[0]["score"], score(m, G, X0)))
    print(f"climb {wid} wide:", [(r, b) for r, a, b in out])
# a niche dug mid-shaft (two openings) and a corridor spur comb
m = np.zeros((128, 256), bool); m[:G] = True; m[G:G + 30, X0 - 2:X0 + 2] = True
out = []
for r in range(7):
    m[G + 15 - r, X0 + 2:X0 + 30] = True; out.append(score(m, G, X0))
print("mid-shaft niche:", out)
