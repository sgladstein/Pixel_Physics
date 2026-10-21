"""SCRATCH (teacher design, not for main code): render the candidate nest blueprints as ant-farm cross-sections at
game scale, for the owner to judge by eye.

usage: python3 -I render_blueprints.py SPECS.json OUTDIR

Reads `nest-blueprint-specs.json` (the nest research agent's spec, body-length units, 1 BL = 2 cells) and draws each
blueprint in a 256x128 box with the ground at row 64, the entrance at column 128, at the *legible* scale (chambers 2.5
BL = 5 cells tall, passages 2 BL = 4 cells; the owner's "passage ~4"). Colours are the game's: soil, packed lining
round every void, placed spoil for the mound.

**The mound is sized to hold the soil the nest's void took out** (in a 2D slice nothing dilutes: the spoil's area is
the dug area). It keeps the spec's inner toe and a 45-degree slope (a 1:1 diagonal slide), and a flat top out to the
spec's outer radius; when that would stand more than 8 cells tall, the top is widened instead, up to the box edge.
The caption says how tall and wide it came out. That is the conservation problem the research flagged, drawn rather
than described.

Tunnel paths in the spec are prose for three of the five; the polylines below are my reading of that prose, marked.
A sixth panel draws the dig ladder's current chamber plan (`dig-rung-design.md` steps 10-11: 8-16 tall, width >=
height) for comparison."""
import json, math, sys, random
from PIL import Image, ImageDraw, ImageFont

W, H, G, X0 = 256, 128, 64, 128   # box, ground row, entrance column
C = 2                             # cells per body length
SOIL = [(64, 46, 34), (74, 54, 40), (56, 40, 30), (82, 60, 44)]
LINED = [(48, 38, 32), (56, 45, 38), (42, 33, 28), (62, 50, 42)]
SPOIL = [(39, 30, 23), (45, 36, 28), (48, 38, 32), (42, 33, 28)]
SKY, VOID, ANT = (150, 178, 205), (14, 11, 10), (150, 40, 30)


def cx(xbl):
    return X0 + xbl * C


def cy(dbl):
    return G + dbl * C


def lens(open_, x0, x1, floor, h, taper=0.6):
    """A flat-floored chamber, roof domed: full height h in the middle, taper*h at the ends (cells)."""
    for x in range(int(round(x0)), int(round(x1)) + 1):
        t = (x - x0) / max(x1 - x0, 1)
        hh = h * (taper + (1 - taper) * math.sin(math.pi * min(max(t, 0), 1)))
        for y in range(int(round(floor - hh)), int(round(floor))):
            open_.add((x, y))


def tube(open_, pts, width):
    """A passage `width` cells wide along a polyline of (x, y) cell points."""
    r = width / 2
    for (ax, ay), (bx, by) in zip(pts, pts[1:]):
        n = int(max(abs(bx - ax), abs(by - ay)) * 2) + 1
        for i in range(n + 1):
            px, py = ax + (bx - ax) * i / n, ay + (by - ay) * i / n
            for x in range(int(px - r) - 1, int(px + r) + 2):
                for y in range(int(py - r) - 1, int(py + r) + 2):
                    if (x - px + 0.5) ** 2 + (y - py + 0.5) ** 2 <= r * r + 0.25:
                        open_.add((x, y))


def bl(pts):
    return [(cx(x), cy(d)) for x, d in pts]


# Tunnel polylines in (x BL, depth BL). Where the spec gives waypoints they are used as given; the rest are my
# reading of the spec's prose ("down-left at ~30 deg", "from E1's left edge to E2's centre", ...).
def tunnels(b):
    i = b["id"]
    if i in ("harvester_stack", "carpenter_shallow"):
        return [b["shaft"]["waypoints_x_depth_bl"]]
    if i == "garden_ant_farm":
        return [
            [(0, 0), (-2.5, 1.5), (-4.5, 3)],                 # G1 to B1
            [(-13, 3), (-15, 5.5), (-17, 9)],                  # B1 on to B3
            [(0, 0), (2.5, 1.5), (4, 3.5), (5.5, 5)],          # G2/G3 to B2
            [(13, 5), (14.5, 8), (16, 11)],                    # B2 to B4, shallow right
            [(7.5, 5), (8, 10), (7, 16)],                      # B2 to B5, steeper
            [(-6, 3), (-5.5, 10), (-4.5, 17), (-4, 24)],       # B1 to B6, the steep one
            [(7, 16), (0, 14.5), (-9, 11.5), (-16, 9)],        # B5 to B3, the cross-link loop
        ]
    if i == "round_room_budding":
        return [
            [(0, 0), (0, 6.5)],                                # the 4-BL shaft into the room's roof
            [(-4.5, 9), (-10, 11), (-14, 13.5)],               # left wall to C1
            [(4.5, 9), (9, 12.5), (12.5, 16.5)],               # right wall to C2
            [(0, 10), (0, 15), (-0.5, 20.5)],                  # floor to C3
            ("bud", [(-4, 7.5), (-5.5, 7)]), ("bud", [(4, 7.5), (5.5, 7.2)]),  # buds on the sides (none in the roof),
            ("bud", [(-3, 10), (-3.8, 11.2)]), ("bud", [(3, 10), (3.8, 11.2)]),  # drawn at their true 1 BL width
        ]
    if i == "staircase_kebab":
        return [
            [(0, 0), (0, 1)],                                  # into the vestibule
            [(2, 1.5), (2, 8)],                                # E0 right edge to E1 centre
            [(-1.6, 1.5), (-1.6, 4.5), (0.5, 7.5)],            # a parallel shaft (upper quarter)
            [(-1.25, 8), (-1.25, 16)],                         # E1 left edge to E2 centre
            [(2.25, 16), (2.25, 25)],                          # E2 right edge to E3 centre
        ]
    raise KeyError(i)


def chambers(b, open_):
    for c in b["chambers"]:
        w, h = c["width_bl"] * C, c["height_legible_bl"] * C
        if "x_centre_bl" in c:
            x0 = cx(c["x_centre_bl"]) - w / 2
        elif c["side"] == "left":
            x0 = cx(c["x_inner_edge_bl"]) - w
        else:
            x0 = cx(c["x_inner_edge_bl"])
        taper = 0.45 if "lens" in c["shape"] or "wedge" in c["shape"] else 0.6
        # the legible height (2.5 BL) on a true-scale floor depth can put the roof above the ground (harvester A0,
        # floor 1.5 BL): keep at least a 2-cell roof, so a chamber is never an open trench
        floor = max(cy(c["floor_depth_bl"]), G + h + 2)
        lens(open_, x0, x0 + w, floor, h, taper)
        if "curl" in c["shape"]:   # C. socius: the far end curls up to ~1 BL under the surface, never through
            tip = x0 + w if c["side"] != "left" else x0
            tube(open_, [(tip, floor - h / 2), (tip + (3 if c["side"] != "left" else -3), G + 2 + 1)], 4)


def mound(b, dug):
    """Height per column (cells) of a mound holding `dug` cells, with the spec's inner toe; see the module doc."""
    m = b["surface_mound"]["box_fit"]
    one_side = "one side" in m.get("side", "")
    ri = m["inner_radius_bl"] * C
    ro = m["outer_radius_bl"] * C
    sides = 1 if one_side else 2

    def profile(h, ro):
        prof = {}
        for s in ([1] if one_side else [-1, 1]):
            for r in range(int(ri), W):
                x = X0 + s * r
                if not (0 <= x < W):
                    continue
                up = r - ri                          # 45 degrees up from the inner toe
                down = (ro + h) - r                  # flat top to ro, then 45 degrees down
                hh = max(0, min(h, up, down))
                if hh > 0:
                    prof[x] = int(round(hh))
        return prof

    while True:
        # area of one side: flat top of width (ro - ri - h) at height h, plus two 45-degree triangles = h*(ro - ri)
        h = dug / sides / max(ro - ri, 1)
        if h <= 8 or ro + 8 >= W / 2 - 2:
            break
        ro += 2
    return profile(h, ro), h, ro, one_side


def owner_scale(b, height_bl=3.5, widen=1.4, deepest_bl=28.0):
    """**The owner's chamber size** (2026-10-21, "7 cells tall"): every chamber 3.5 BL = 7 cells tall and 1.4x as wide
    as the spec, so width:height stays where the spec had it. Each floor moves down by the height added above it
    (chambers stacked on one shaft must not merge), then all depths are squeezed so the deepest floor sits at
    `deepest_bl` and nothing is cut off by the box's floor."""
    import copy
    b = copy.deepcopy(b)
    order = sorted(b["chambers"], key=lambda c: c["floor_depth_bl"])
    olds = [c["floor_depth_bl"] for c in order]
    added = 0.0
    for c in order:
        if c["height_legible_bl"] >= height_bl:   # already as tall (the round room's 4 BL): left as the spec drew it
            c["floor_depth_bl"] += added
            continue
        added += (height_bl - c["height_legible_bl"]) * 0.55
        c["floor_depth_bl"] += added
        c["height_legible_bl"] = height_bl
        c["width_bl"] = round(c["width_bl"] * widen, 2)
    deep = max(c["floor_depth_bl"] for c in order)
    k = min(1.0, deepest_bl / deep)
    for c in order:
        c["floor_depth_bl"] = round(c["floor_depth_bl"] * k, 2)
    # one monotone depth map, old floor -> new floor, so the tunnels move with the chambers they join
    b["_remap"] = [(0.0, 0.0)] + [(o, c["floor_depth_bl"]) for o, c in zip(olds, order)]
    return b


def remap(pairs):
    """Piecewise-linear depth map through `pairs` (old, new), extended flat-sloped past the last pair."""
    if not pairs:
        return lambda d: d
    pairs = sorted(set(pairs))

    def f(d):
        for (a, fa), (b2, fb) in zip(pairs, pairs[1:]):
            if d <= b2:
                return fa + (fb - fa) * (d - a) / max(b2 - a, 1e-9)
        return pairs[-1][1] + (d - pairs[-1][0])
    return f


def build(b, ladder=False):
    """The open cells of a blueprint (or of the ladder's plan), clipped to the soil."""
    open_ = set()
    if ladder:
        tube(open_, [(X0, G), (X0, G + 16)], 4)
        lens(open_, X0 - 10, X0 + 10, G + 16 + 12, 12, 0.85)
        tube(open_, [(X0 + 10, G + 22), (X0 + 26, G + 30)], 4)
        lens(open_, X0 + 22, X0 + 38, G + 30 + 8, 8, 0.85)
    else:
        f = remap(b.get("_remap"))
        tunnels_ = [(t[0], [(x, f(d)) for x, d in t[1]]) if t[0] == "bud" else [(x, f(d)) for x, d in t] for t in tunnels(b)]
        for t in tunnels_:
            if t[0] == "bud":
                tube(open_, bl(t[1]), 2)
            else:
                tube(open_, bl(t), 4)
        chambers(b, open_)
    return {(x, y) for x, y in open_ if 0 <= x < W and G <= y < H}


def render(b, out, ladder=False):
    open_ = build(b, ladder)
    if ladder:
        name, sig = "The ladder's current plan (dig-rung-design steps 10-11)", "chambers 8-16 cells tall, width >= height"
        b = {"surface_mound": {"box_fit": {"inner_radius_bl": 2.5, "crest_radius_bl": 4, "outer_radius_bl": 10}}}
    else:
        name, sig = b["name"], b["visual_signature"]
    # the mouth: the top of the shaft is open to the sky at the entrance
    dug = len(open_)
    prof, mh, mro, one = mound(b, dug)
    rnd = random.Random(7)
    img = Image.new("RGB", (W, H), SKY)
    px = img.load()
    for x in range(W):
        for y in range(G, H):
            if (x, y) in open_:
                px[x, y] = VOID
            elif any((x + dx, y + dy) in open_ for dx in (-1, 0, 1) for dy in (-1, 0, 1)):
                px[x, y] = rnd.choice(LINED)
            else:
                px[x, y] = rnd.choice(SOIL)
        for k in range(prof.get(x, 0)):
            px[x, G - 1 - k] = rnd.choice(SPOIL)
    # a two-cell ant in the first chamber, for scale
    for (x, y) in sorted(open_, key=lambda p: (-p[1], p[0])):
        if y > G + 6 and (x + 1, y) in open_ and (x, y + 1) not in open_ and (x + 1, y + 1) not in open_:
            px[x, y], px[x + 1, y] = ANT, ANT
            break
    z = 4
    big = img.resize((W * z, H * z), Image.NEAREST)
    canvas = Image.new("RGB", (W * z, H * z + 92), (24, 24, 28))
    canvas.paste(big, (0, 0))
    d = ImageDraw.Draw(canvas)
    try:
        f = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 18)
        fs = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 14)
    except OSError:
        f = fs = ImageFont.load_default()
    d.text((10, H * z + 6), name, fill=(235, 235, 235), font=f)
    import textwrap
    for k, line in enumerate(textwrap.wrap(sig, 128)[:2]):
        d.text((10, H * z + 32 + 20 * k), line, fill=(190, 190, 190), font=fs)
    side = "one side" if one else "both sides"
    d.text((10, H * z + 72),
           f"dug {dug} cells (25 cells an ant: room for ~{dug // 25} ants) | mound to hold it, {side}: "
           f"{mh:.1f} cells tall, out to {mro} from the mouth | 1 body length = 2 cells",
           fill=(160, 200, 160), font=fs)
    canvas.save(out)
    return dug, mh, mro


if __name__ == "__main__":
    spec, outdir = json.load(open(sys.argv[1])), sys.argv[2]
    rows = []
    for k, b in enumerate(spec["blueprints"], 1):
        b = owner_scale(b)
        r = render(b, f"{outdir}/{k}-{b['id']}.png")
        rows.append((b["id"], *r))
    rows.append(("ladder_plan", *render(None, f"{outdir}/6-ladder-plan.png", ladder=True)))
    for r in rows:
        print("%-20s dug %5d  mound %.1f cells tall, to %d cells" % r)
