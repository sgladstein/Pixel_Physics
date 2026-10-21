"""SCRATCH (teacher design, prototype for a Rust port): **a general nest score** -- how much of a dug nest is
well-built, by properties rather than against one fixed blueprint.

Owner, 2026-10-21: "can we reward more general things like number of well proportioned tunnels and chambers?" -- and
chambers 7 cells tall. So a nest is scored for what it *has*, graded, never pass/fail:

- **Chambers**: roofed rooms with a core at least 7 wide and 5 tall (a 7x5 box fits inside every part of one -- so
  a passage up to 6 wide, at any angle, is never a chamber; a 5x5 core let 5-wide shafts through, measured on the
  staircase blueprint), each graded on three things: **height** near the species' target (7 cells, full credit
  6-8, none at 3 or 13), **proportion** (width at least 3x height for full credit, none at 1x, real chambers are
  3-20x), and a **flat floor**. A chamber's credit is its area times its grade, with the area capped (`room_cap`), so
  one enormous room cannot stand in for several.
- **Tunnels**: everything else in the nest, graded per cell on its width across (full credit 3-6 cells, none at 1
  or 10). **Tunnels count only as much as what they serve**: their credit is capped at the chambers' credit (or a
  small allowance for the shaft before there is a chamber), so a maze of corridors is not a nest.
- **Layout** (species): top-heavy for the harvester -- the share of chamber area in the upper half of the nest's
  depth. It scales the chamber credit by 0.75-1, so it shapes but cannot dominate.
- **What is wrong**, in cells: cracks (open cells under 3 wide), a pit or crater open to the sky away from the mouth,
  and open cells not joined to the door.

**The score is in cells: well-built nest cells**, `chambers + 0.5 x tunnels - bad`. So it grows with how much is
dug, as real nests grow with their colony; `per_cell` (score / cells dug) says how well, whatever the size; with
`ants`, `target` is 25 open cells an ant (about 8% crowding, the real figure) and `fit` the share of that reached.

Only below-ground, roofed cells joined to the door count (`measuring-the-world.md`: standing void is not dug void, a
hole open to the sky is not a room). The door's own mouth may be open to the sky over `mouth` columns either side.

usage:
  python3 -I nestscore.py controls SPECS.json OUTDIR     # score the blueprints and the built-bad shapes, draw each
  python3 -I nestscore.py run RUNDIR [ANTS] [OUT.png]       # score every `map_fNNNNNN.txt` of a deeptrace run, draw the last
                                                            (`write_map`'s frame: ground at row 60, door at column 80)
"""
import json, math, sys, random
from collections import deque
import numpy as np

PROFILES = {
    # the owner's pick (2026-10-21): harvester layout, chambers 7 cells tall
    "harvester": dict(h_full=(6, 8), h_zero=(3, 13), aspect_full=3.0, aspect_zero=1.0, w_full=(3, 6), w_zero=(1, 10),
                      core=(3, 2), min_room=30, room_cap=400, tunnel_free=60, top_heavy=0.5, mouth=4, cells_per_ant=25),
}
PROFILES["general"] = dict(PROFILES["harvester"], top_heavy=None)


def ramp(v, zero, full):
    """0 at `zero`, 1 at `full`, linear between, clipped (either direction)."""
    if full == zero:
        return float(v >= full)
    return float(min(1.0, max(0.0, (v - zero) / (full - zero))))


def band(v, full, zero):
    """1 inside [full], falling linearly to 0 at [zero]."""
    lo = ramp(v, zero[0], full[0])
    hi = ramp(v, zero[1], full[1])
    return min(lo, hi)


def components(mask, conn4=True):
    h, w = mask.shape
    lab = np.zeros((h, w), np.int32)
    comps = []
    nbr = ((1, 0), (-1, 0), (0, 1), (0, -1))
    for y, x in zip(*np.nonzero(mask)):
        if lab[y, x]:
            continue
        cid = len(comps) + 1
        lab[y, x] = cid
        q, cells = deque([(y, x)]), []
        while q:
            cy, cx = q.popleft()
            cells.append((cy, cx))
            for dy, dx in nbr:
                ny, nx = cy + dy, cx + dx
                if 0 <= ny < h and 0 <= nx < w and mask[ny, nx] and not lab[ny, nx]:
                    lab[ny, nx] = cid
                    q.append((ny, nx))
        comps.append(cells)
    return lab, comps


def erode_square(m, r):
    """Cells where the box (2*rx+1 wide, 2*ry+1 tall; `r` an int or (rx, ry)) centred on them is all inside `m`."""
    ry, rx = (r, r) if isinstance(r, int) else (r[1], r[0])
    out = m.copy()
    h, w = m.shape
    for dy in range(-ry, ry + 1):
        for dx in range(-rx, rx + 1):
            sh = np.zeros_like(m)
            ys, yd = (slice(max(dy, 0), h + min(dy, 0)), slice(max(-dy, 0), h + min(-dy, 0)))
            xs, xd = (slice(max(dx, 0), w + min(dx, 0)), slice(max(-dx, 0), w + min(-dx, 0)))
            sh[yd, xd] = m[ys, xs]
            out &= sh
    return out


def dilate_square(m, r):
    ry, rx = (r, r) if isinstance(r, int) else (r[1], r[0])
    out = m.copy()
    h, w = m.shape
    for dy in range(-ry, ry + 1):
        for dx in range(-rx, rx + 1):
            sh = np.zeros_like(m)
            ys, yd = (slice(max(dy, 0), h + min(dy, 0)), slice(max(-dy, 0), h + min(-dy, 0)))
            xs, xd = (slice(max(dx, 0), w + min(dx, 0)), slice(max(-dx, 0), w + min(-dx, 0)))
            sh[yd, xd] = m[ys, xs]
            out |= sh
    return out


def runs(m, dy, dx):
    """Length of the straight run of `m` cells through each cell along direction (dy, dx)."""
    h, w = m.shape
    fwd = np.zeros((h, w), np.int32)
    bwd = np.zeros((h, w), np.int32)
    ys = range(h) if dy >= 0 else range(h - 1, -1, -1)
    order = [(y, x) for y in ys for x in (range(w) if dx >= 0 else range(w - 1, -1, -1))]
    for y, x in order:
        if m[y, x]:
            py, px = y - dy, x - dx
            fwd[y, x] = 1 + (fwd[py, px] if 0 <= py < h and 0 <= px < w else 0)
    for y, x in reversed(order):
        if m[y, x]:
            ny, nx = y + dy, x + dx
            bwd[y, x] = 1 + (bwd[ny, nx] if 0 <= ny < h and 0 <= nx < w else 0)
    return fwd + bwd - 1


def score(open_, ground, door_x, ants=None, profile="harvester"):
    """`open_`: HxW bool, True where the cell is open (air, or anything an ant walks through). `ground`: the original
    surface row of each column (array of W), or one row for all. `door_x`: the mouth's column."""
    P = PROFILES[profile]
    h, w = open_.shape
    ground = np.broadcast_to(np.asarray(ground), (w,))
    below = np.arange(h)[:, None] >= ground[None, :]
    # open to the sky: an unbroken open column from above the ground down to the cell
    sky = np.zeros((h, w), bool)
    for x in range(w):
        y = 0
        while y < h and open_[y, x]:
            sky[y, x] = True
            y += 1
    mouth = np.zeros((h, w), bool)
    mouth[:, max(0, door_x - P["mouth"]):door_x + P["mouth"] + 1] = True
    pit = sky & below & ~mouth
    # joined to the door through any open cell below ground, a crater included: a wide mouth is penalised as pit
    # but does not cut the nest under it off (it did, on the random-walk cave control: 1,072 cells "stray")
    reach = open_ & below
    lab, comps = components(reach)
    door_ids = {lab[y, x] for x in range(max(0, door_x - P["mouth"]), min(w, door_x + P["mouth"] + 1))
                for y in range(int(ground[door_x]), min(h, int(ground[door_x]) + 3)) if lab[y, x]}
    joined = np.isin(lab, list(door_ids)) if door_ids else np.zeros_like(reach)
    nest = joined & ~pit
    stray = reach & ~joined & ~pit
    # chambers: the opening of the nest by a (2*core+1) square
    r = P["core"]
    opened = dilate_square(erode_square(nest, r), r) & nest
    clab, ccomps = components(opened)
    chambers, chamber_mask = [], np.zeros_like(nest)
    nest_rows = np.nonzero(nest.any(axis=1))[0]
    top, bot = (nest_rows.min(), nest_rows.max()) if len(nest_rows) else (0, 0)
    for cells in ccomps:
        if len(cells) < P["min_room"]:
            continue
        ys = np.array([c[0] for c in cells])
        xs = np.array([c[1] for c in cells])
        cols = {}
        for y, x in cells:
            cols.setdefault(x, []).append(y)
        heights = np.array([len(v) for v in cols.values()])
        floors = np.array([max(v) for v in cols.values()])
        full = heights >= 0.5 * heights.max()              # leave out the tapered tips
        H = float(np.median(heights[full]))
        W = float(len(cols))
        fl = np.median(floors[full])
        flat = float(np.mean(np.abs(floors[full] - fl) <= 1))
        grade = band(H, P["h_full"], P["h_zero"]) * ramp(W / H, P["aspect_zero"], P["aspect_full"]) * (0.5 + 0.5 * flat)
        area = len(cells)
        chambers.append(dict(area=area, H=H, W=W, aspect=W / H, flat=flat, grade=grade,
                             credit=grade * min(area, P["room_cap"]), depth=float(ys.mean())))
        chamber_mask[ys, xs] = True
    # tunnels: the rest of the nest, graded on width across (the shortest straight run through the cell)
    tun = nest & ~chamber_mask
    s2 = math.sqrt(2)
    width = np.minimum.reduce([runs(nest, 0, 1), runs(nest, 1, 0), runs(nest, 1, 1) * s2, runs(nest, 1, -1) * s2])
    wv = width[tun]
    tgrade = np.array([band(v, P["w_full"], P["w_zero"]) for v in wv]) if len(wv) else np.zeros(0)
    crack = int(np.sum(wv < 3))
    chamber_credit = sum(c["credit"] for c in chambers)
    layout = 1.0
    if P["top_heavy"] is not None and len(chambers) >= 2 and bot > top:
        mid = top + (bot - top) / 2
        upper = sum(c["area"] for c in chambers if c["depth"] <= mid) / sum(c["area"] for c in chambers)
        layout = 0.75 + 0.25 * ramp(upper, 0.0, P["top_heavy"])
    chamber_credit *= layout
    tunnel_credit = min(float(tgrade.sum()), max(P["tunnel_free"], chamber_credit))
    bad = 0.5 * crack + int(pit.sum()) + 0.5 * int(stray.sum())
    total = chamber_credit + 0.5 * tunnel_credit - bad
    dug = int(nest.sum())
    out = dict(score=round(total, 1), dug=dug, per_cell=round(total / dug, 3) if dug else 0.0,
               chambers=len(chambers), good_chambers=round(sum(c["grade"] for c in chambers), 2),
               chamber_credit=round(chamber_credit, 1), layout=round(layout, 2),
               tunnel_cells=int(tun.sum()), tunnel_credit=round(tunnel_credit, 1), cracks=crack,
               pit=int(pit.sum()), stray=int(stray.sum()),
               rooms=[dict((k, round(v, 2)) for k, v in c.items()) for c in chambers])
    if ants:
        target = ants * P["cells_per_ant"]
        out.update(target=target, fit=round(min(1.0, max(total, 0) / target), 3))
    return out, dict(nest=nest, chambers=chamber_mask, tunnels=tun, width=width, pit=pit, stray=stray)


# --- the controls --------------------------------------------------------------------------------------------------

def grid_of(cells, h=128, w=256, g=64, sky=True):
    m = np.zeros((h, w), bool)
    if sky:
        m[:g, :] = True
    for x, y in cells:
        if 0 <= x < w and 0 <= y < h:
            m[y, x] = True
    return m


def bad_shapes(rb, g=64, x0=128):
    """Built-bad nests, each the case one way of gaming the score would produce. Areas roughly a blueprint's."""
    out = {}
    shaft = set()
    rb.tube(shaft, [(x0, g), (x0, g + 16)], 4)
    out["shaft only (the 8-row arena today)"] = shaft
    pit = {(x, y) for x in range(x0 - 25, x0 + 26) for y in range(g, g + 1 + int(14 * (1 - ((x - x0) / 25) ** 2)))}
    out["crater open to the sky"] = pit
    tall = set(shaft)
    tall |= {(x, y) for x in range(x0 - 13, x0 + 13) for y in range(g + 16, g + 44)}
    out["one tall room (26x28)"] = tall
    hall = set(shaft)
    hall |= {(x, y) for x in range(x0 - 70, x0 + 70) for y in range(g + 16, g + 23)}
    out["one huge flat hall (140x7)"] = hall
    blobs = set(shaft)
    rb.tube(blobs, [(x0, g + 16), (x0, g + 56)], 4)
    for k in range(10):
        yy = g + 18 + 4 * k
        side = 1 if k % 2 else -1
        rb.tube(blobs, [(x0, yy), (x0 + side * 10, yy)], 4)
        blobs |= {(x0 + side * 10 + dx, yy + dy) for dx in range(-2, 3) for dy in range(-2, 3)}
    out["many tiny pockets (5x5)"] = blobs
    rnd = random.Random(3)
    crack = set()
    for k in range(6):
        x, y = x0, g
        for _ in range(140):
            crack |= {(x, y), (x + 1, y)}
            x += rnd.choice((-1, 0, 1, 1, -1))
            y += rnd.choice((0, 1, 1))
            y = min(y, 125)
    out["cracks (2 wide, wandering)"] = crack
    maze = set()
    for k in range(5):
        rb.tube(maze, [(x0 - 60, g + 8 + 9 * k), (x0 + 60, g + 8 + 9 * k)], 4)
    for xx in range(x0 - 60, x0 + 61, 20):
        rb.tube(maze, [(xx, g + 8), (xx, g + 44)], 4)
    rb.tube(maze, [(x0, g), (x0, g + 8)], 4)
    out["corridor maze, no rooms"] = maze
    rnd = random.Random(5)
    cave = set()
    rb.tube(cave, [(x0, g), (x0, g + 12)], 4)
    x, y = x0, g + 12
    while len(cave) < 1200:
        rb.tube(cave, [(x, y), (x, y)], 4)
        # wanders, reflecting off a band in the middle of the soil (an earlier version clamped at the box floor and
        # dug one long flat hall along it -- a well-proportioned room, which the score rightly paid for)
        x = min(230, max(26, x + rnd.choice((-2, -1, 1, 2))))
        y += rnd.choice((-2, -1, 1, 2))
        y = g + 8 + abs((y - g - 8) % 80 - 40) if not (g + 8 <= y <= g + 48) else y
    out["random-walk cave (same area)"] = cave
    return out


def draw(grid, parts, path, title, s, ground=64):
    from PIL import Image, ImageDraw, ImageFont
    h, w = grid.shape
    img = np.zeros((h, w, 3), np.uint8)
    img[:] = (64, 46, 34)
    img[~np.zeros_like(grid)] = img[0, 0]
    img[:ground] = (150, 178, 205)
    img[~grid & (np.arange(h)[:, None] < ground)] = (90, 70, 55)    # soil above the ground: the mound
    img[grid & (np.arange(h)[:, None] >= ground)] = (14, 11, 10)
    img[parts["tunnels"]] = (70, 110, 160)                     # tunnel: blue
    img[parts["chambers"]] = (60, 150, 70)                     # chamber: green
    img[parts["tunnels"] & (parts["width"] < 3)] = (220, 60, 50)  # crack: red
    img[parts["pit"]] = (230, 160, 40)                         # open pit: orange
    img[parts["stray"]] = (150, 70, 160)                       # not joined to the door: purple
    z = 3
    im = Image.fromarray(img).resize((w * z, h * z), Image.NEAREST)
    canvas = Image.new("RGB", (w * z, h * z + 50), (24, 24, 28))
    canvas.paste(im, (0, 0))
    d = ImageDraw.Draw(canvas)
    try:
        f = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 15)
    except OSError:
        f = ImageFont.load_default()
    d.text((8, h * z + 4), title, fill=(235, 235, 235), font=f)
    d.text((8, h * z + 26), f"score {s['score']:.0f} well-built cells of {s['dug']} dug ({s['per_cell']:.2f} a cell) | "
           f"rooms {s['chambers']} (graded {s['good_chambers']:.1f}) | cracks {s['cracks']} | pit {s['pit']} | stray {s['stray']}",
           fill=(160, 200, 160), font=f)
    canvas.save(path)


def controls(spec_path, outdir):
    sys.path.insert(0, __import__("os").path.dirname(__file__) or ".")
    import render_blueprints as rb
    spec = json.load(open(spec_path))
    cases = []
    for b in spec["blueprints"]:
        cases.append((f"blueprint: {b['id']} (7-cell chambers)", rb.build(rb.owner_scale(b))))
    cases.append(("blueprint: harvester_stack (5-cell chambers, as first drawn)", rb.build(spec["blueprints"][0])))
    cases.append(("ladder plan (8-16 tall rooms)", rb.build(None, ladder=True)))
    for name, cells in bad_shapes(rb).items():
        cases.append(("bad: " + name, cells))
    rows = []
    for k, (name, cells) in enumerate(cases):
        g = grid_of(cells)
        s, parts = score(g, 64, 128)
        draw(g, parts, f"{outdir}/score-{k:02d}.png", name, s)
        rows.append((name, s))
        print(f"{name:62s} score {s['score']:7.1f}  dug {s['dug']:5d}  per cell {s['per_cell']:6.3f}  rooms {s['chambers']:2d} "
              f"graded {s['good_chambers']:4.1f}  layout {s['layout']:.2f}  tunnel {s['tunnel_credit']:6.1f}  "
              f"cracks {s['cracks']:4d}  pit {s['pit']:4d}  stray {s['stray']:4d}")
    json.dump([dict(name=n, **{k: v for k, v in s.items()}) for n, s in rows], open(f"{outdir}/controls.json", "w"), indent=1)


OPEN = set("o.aelpfcx~?b")   # deeptrace's map characters an ant walks through (scripts/deeptrace_dig.py)


def read_map(path):
    with open(path) as fh:
        x0, y0, wd, ht = map(int, fh.readline().split())
        rows = [ln.rstrip("\n") for ln in fh][:ht]
    m = np.array([[c in OPEN for c in (r + " " * wd)[:wd]] for r in rows])
    return (x0, y0), m


def run(rundir, ants=None, out=None):
    import glob, re
    rows = []
    for path in sorted(glob.glob(f"{rundir}/map_f*.txt")):
        frame = int(re.search(r"map_f(\d+)", path).group(1))
        _, m = read_map(path)
        s, parts = score(m, 60, 80, ants)
        rows.append((frame, s, m, parts))
        print(f"{rundir} frame {frame:7d}  score {s['score']:7.1f}  dug {s['dug']:5d}  per cell {s['per_cell']:6.3f}  "
              f"rooms {s['chambers']:2d} graded {s['good_chambers']:4.1f}  tunnel {s['tunnel_credit']:6.1f}  "
              f"cracks {s['cracks']:4d}  pit {s['pit']:4d}  stray {s['stray']:4d}")
    if out and rows:
        frame, s, m, parts = rows[-1]
        draw(m, parts, out, f"{rundir}, frame {frame}", s, ground=60)
    return rows


if __name__ == "__main__":
    if sys.argv[1] == "controls":
        controls(sys.argv[2], sys.argv[3])
    elif sys.argv[1] == "run":
        run(sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 and sys.argv[3] else None, sys.argv[4] if len(sys.argv) > 4 else None)
