"""Prototype of the climb fix: a chamber is a flat room at least 7 wide whose few openings say it is a room, not a
through-corridor. Graded on height from 1 row, so a niche pays as it rises."""
import sys, json, math; sys.path.insert(0, "/home/user/Pixel_Physics/Reports/brain-training/teacher-training/measurements/blueprints")
import numpy as np, nestscore as ns, render_blueprints as rb
from nestscore import ramp, band, components, erode_square, dilate_square, runs

def openings(cells_mask, nest):
    """Contacts: the nest cells outside the room 4-adjacent to it, grouped 8-connected."""
    h, w = nest.shape
    adj = np.zeros_like(nest)
    adj[1:] |= cells_mask[:-1]; adj[:-1] |= cells_mask[1:]; adj[:, 1:] |= cells_mask[:, :-1]; adj[:, :-1] |= cells_mask[:, 1:]
    contact = adj & nest & ~cells_mask
    # 8-connected grouping
    lab = np.zeros((h, w), int); n = 0
    for y, x in zip(*np.nonzero(contact)):
        if lab[y, x]: continue
        n += 1; st = [(y, x)]; lab[y, x] = n
        while st:
            cy, cx = st.pop()
            for dy in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    yy, xx = cy + dy, cx + dx
                    if 0 <= yy < h and 0 <= xx < w and contact[yy, xx] and not lab[yy, xx]:
                        lab[yy, xx] = n; st.append((yy, xx))
    return n

P = dict(ns.PROFILES["harvester"], core=(3, 0), h_zero=(1, 13), min_room=14, open_full=2, open_zero=4)

def score(open_, ground, door_x, P=P, verbose=False):
    h, w = open_.shape
    base, parts = ns.score(open_, ground, door_x)  # reuse pit/stray/nest
    nest, pit, stray = parts["nest"], parts["pit"], parts["stray"]
    r = P["core"]
    opened = dilate_square(erode_square(nest, r), r) & nest
    clab, ccomps = components(opened)
    chambers, cm = [], np.zeros_like(nest)
    rows = np.nonzero(nest.any(axis=1))[0]; top, bot = (rows.min(), rows.max()) if len(rows) else (0, 0)
    for cells in ccomps:
        if len(cells) < P["min_room"]: continue
        m = np.zeros_like(nest); ys = np.array([c[0] for c in cells]); xs = np.array([c[1] for c in cells]); m[ys, xs] = True
        cols = {}
        for y, x in cells: cols.setdefault(x, []).append(y)
        heights = np.array([len(v) for v in cols.values()]); floors = np.array([max(v) for v in cols.values()])
        full = heights >= 0.5 * heights.max()
        H = float(np.median(heights[full])); W = float(len(cols)); fl = np.median(floors[full])
        flat = float(np.mean(np.abs(floors[full] - fl) <= 1))
        k = openings(m, nest)
        og = ramp(k, P["open_zero"], P["open_full"])
        grade = band(H, P["h_full"], P["h_zero"]) * ramp(W / H, P["aspect_zero"], P["aspect_full"]) * (0.5 + 0.5 * flat) * og
        chambers.append(dict(area=len(cells), H=H, W=W, k=k, grade=grade, credit=grade * min(len(cells), P["room_cap"]), depth=ys.mean()))
        # a room that is a corridor (graded 0 by its openings) stays tunnel, and is graded as tunnel
        if og > 0: cm |= m
    tun = nest & ~cm
    s2 = math.sqrt(2)
    width = np.minimum.reduce([runs(nest, 0, 1), runs(nest, 1, 0), runs(nest, 1, 1) * s2, runs(nest, 1, -1) * s2])
    wv = width[tun]; tg = np.array([band(v, P["w_full"], P["w_zero"]) for v in wv]) if len(wv) else np.zeros(0)
    crack = int(np.sum(wv < 3))
    cc = sum(c["credit"] for c in chambers)
    layout = 1.0
    good = [c for c in chambers if c["grade"] > 0]
    if len(good) >= 2 and bot > top:
        mid = top + (bot - top) / 2
        up = sum(c["area"] for c in good if c["depth"] <= mid) / sum(c["area"] for c in good)
        layout = 0.75 + 0.25 * ramp(up, 0, 0.5)
    cc *= layout
    tc = min(float(tg.sum()), max(P["tunnel_free"], cc))
    total = cc + 0.5 * tc - (0.5 * crack + int(pit.sum()) + 0.5 * int(stray.sum()))
    if verbose:
        print("   rooms", [(c["area"], c["H"], c["W"], c["k"], round(c["grade"], 2)) for c in chambers])
    return round(total, 1), crack

spec = json.load(open("/home/user/Pixel_Physics/Reports/brain-training/teacher-training/measurements/blueprints/nest-blueprint-specs.json"))
cases = [(b["id"], ns.grid_of(rb.build(rb.owner_scale(b)))) for b in spec["blueprints"]]
cases.append(("ladder 8-16", ns.grid_of(rb.build(None, ladder=True))))
cases += [(k, ns.grid_of(v)) for k, v in ns.bad_shapes(rb).items()]
V = "-v" in sys.argv
for name, g in cases:
    a = ns.score(g, 64, 128)[0]["score"]
    print(f"{name:40s} as built {a:7.1f}   new {score(g, 64, 128, verbose=V)}")
G, X0 = 64, 128
for wid in (28, 14):
    m = np.zeros((128, 256), bool); m[:G] = True; m[G:G + 16, X0 - 2:X0 + 2] = True
    out = [(0, ns.score(m, G, X0)[0]["score"], score(m, G, X0)[0])]
    for r in range(8):
        m[G + 15 - r, X0 + 2:X0 + 2 + wid] = True
        out.append((r + 1, ns.score(m, G, X0)[0]["score"], score(m, G, X0)[0]))
    print(f"climb {wid} wide (rows, as built, new):", out)
