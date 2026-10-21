"""SCRATCH (teacher design section 5): the same nest with a ring mound and with a one-sided crescent, the foragers'
route to the food drawn on, for the owner to judge (design v2's question 1).

usage: python3 -I ringcrescent.py SPECS.json OUT.png

Two nests, both the harvester profile at the owner's 7-cell chambers: sized for 12 ants (the shaft and its top three
chambers -- the review's "shaft and 2-3 chambers at 12 ants") and the full stack. Each with the soil it dug heaped
**as a ring** (half each side of the mouth) and **as a crescent** (all of it on the side away from the food). The
mound holds exactly the nest's dug cells (a 2D slice: nothing spreads into a third dimension). Its profile is a lumpy
bump, not a trapezoid -- real drop points are spread round the crest -- starting 3 cells out from the mouth's edge,
kept to at most 10 cells tall by lengthening it; the crescent therefore runs further out or stands taller. Food sits 45
columns east of the mouth (the lab's goal box puts its heap 30 east; the dig ladder's rung 9 drips at 10-30), on top
of whatever the mound has put there.

The route is the surface an ant walks from the mouth to the food: it reports **how many rows it climbs**, which is the
cost a ring puts on every trip -- or the tunnel the ants would dig through it instead, which is what the old game's
ants did (`dead-ends.md`, `MOUND_DIG`).
"""
import json, math, random, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import render_blueprints as rb
from PIL import Image, ImageDraw, ImageFont

W, H, G, X0 = rb.W, rb.H, rb.G, rb.X0
FOOD_DX = 45


def twelve_ant(b):
    """The harvester sized for 12 ants: the shaft down to the third chamber, and the top three chambers."""
    import copy
    b = copy.deepcopy(b)
    order = sorted(b["chambers"], key=lambda c: c["floor_depth_bl"])
    b["chambers"] = order[:3]
    deep = order[2]["floor_depth_bl"]
    wp = b["shaft"]["waypoints_x_depth_bl"]
    b["shaft"]["waypoints_x_depth_bl"] = [p for p in wp if p[1] <= deep + 0.5] or wp[:3]
    return b


def mound(area, sides, start, hmax=10, rnd=None):
    """Heights by column offset from the mouth for a lumpy bump holding `area` cells per the sides given."""
    per = area / len(sides)
    L = max(16, per / (0.64 * hmax))                       # a half-sine bump of height h, length L holds 0.64 h L
    room = W // 2 - start - 4
    if L > room:
        L = room
    h = per / (0.64 * L)
    prof = {}
    for s in sides:
        bumps = [(rnd.uniform(0.15, 0.85), rnd.uniform(0.08, 0.2), rnd.uniform(0.1, 0.25)) for _ in range(4)]
        tot = 0
        raw = {}
        for k in range(int(L) + 1):
            t = k / L
            v = math.sin(math.pi * t) ** 0.8 * (1 + sum(a * math.exp(-((t - c) / w) ** 2) for c, w, a in bumps))
            raw[k] = v
            tot += v
        scale = per / tot
        for k, v in raw.items():
            prof[X0 + s * (start + k)] = prof.get(X0 + s * (start + k), 0) + v * scale
    return {x: int(round(v)) for x, v in prof.items() if 0 <= x < W}, h, L


def panel(b, title, sides, seed=1):
    open_ = rb.build(b)
    dug = len(open_)
    rnd = random.Random(seed)
    prof, h, L = mound(dug, sides, start=2 + 3, rnd=rnd)
    img = Image.new("RGB", (W, H), rb.SKY)
    px = img.load()
    for x in range(W):
        for y in range(G, H):
            if (x, y) in open_:
                px[x, y] = rb.VOID
            elif any((x + dx, y + dy) in open_ for dx in (-1, 0, 1) for dy in (-1, 0, 1)):
                px[x, y] = rnd.choice(rb.LINED)
            else:
                px[x, y] = rnd.choice(rb.SOIL)
        for k in range(prof.get(x, 0)):
            px[x, G - 1 - k] = rnd.choice(rb.SPOIL)
    # food heap on top of whatever is there
    fx = X0 + FOOD_DX
    for dx in range(-4, 5):
        top = G - prof.get(fx + dx, 0)
        for k in range(3 - abs(dx) // 2):
            px[fx + dx, top - 1 - k] = (196, 150, 64)
    # the walked route: the surface from the mouth's east lip to the food
    route = []
    for x in range(X0 + 2, fx - 4):
        route.append((x, G - prof.get(x, 0) - 2))
    climb = sum(max(0, route[i][1] - route[i + 1][1]) for i in range(len(route) - 1))
    peak = G - min(y for _, y in route) - 2
    for i, (x, y) in enumerate(route):
        if i % 2 == 0:
            px[x, y] = (250, 230, 70)
    z = 3
    big = img.resize((W * z, H * z), Image.NEAREST)
    canvas = Image.new("RGB", (W * z, H * z + 64), (24, 24, 28))
    canvas.paste(big, (0, 0))
    d = ImageDraw.Draw(canvas)
    try:
        f = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 17)
        fs = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 14)
    except OSError:
        f = fs = ImageFont.load_default()
    d.text((8, H * z + 4), title, fill=(235, 235, 235), font=f)
    d.text((8, H * z + 26), f"nest {dug} cells -> mound {sum(prof.values())} cells, about {h:.1f} tall, "
           f"{L:.0f} long {'each side' if len(sides) == 2 else 'on the west'}", fill=(190, 190, 190), font=fs)
    d.text((8, H * z + 44), f"trip to the food (yellow): climbs {climb} rows, over a top {peak} rows above the ground",
           fill=(240, 220, 90), font=fs)
    return canvas, dict(dug=dug, mound_h=round(h, 1), mound_len=round(L), climb=climb, peak=peak)


if __name__ == "__main__":
    spec = json.load(open(sys.argv[1]))
    harv = rb.owner_scale([b for b in spec["blueprints"] if b["id"] == "harvester_stack"][0])
    small = twelve_ant(harv)
    rows = []
    out = []
    for nest, name in ((small, "12-ant harvester"), (harv, "full harvester")):
        for sides, kind in (((-1, 1), "ring"), ((-1,), "crescent, away from the food")):
            c, m = panel(nest, f"{name}, {kind}", sides)
            out.append(c)
            rows.append((name, kind, m))
            print(name, kind, m)
    w, h = out[0].size
    sheet = Image.new("RGB", (2 * w + 6, 2 * h + 6), (10, 10, 12))
    for i, c in enumerate(out):
        sheet.paste(c, ((i % 2) * (w + 6), (i // 2) * (h + 6)))
    sheet.save(sys.argv[2])
