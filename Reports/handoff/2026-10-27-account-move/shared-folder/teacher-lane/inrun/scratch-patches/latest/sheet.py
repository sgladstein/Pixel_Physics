"""SCRATCH: final nests of several runs as one labelled contact sheet."""
import sys
from PIL import Image, ImageDraw, ImageFont
out = sys.argv[1]; items = [a.split('=', 1) for a in sys.argv[2:]]   # label=rundir
xl, xh, yl, yh, sc = 78, 178, 52, 92, 4
COL = {".": (214, 228, 240), "#": (120, 92, 64), "f": (90, 170, 70), "a": (20, 20, 20)}
try: font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 13)
except OSError: font = ImageFont.load_default()
W, H = (xh - xl + 1) * sc, (yh - yl + 1) * sc
cols = 2; rows = (len(items) + 1) // 2; pad = 50
sheet = Image.new("RGB", (cols * (W + 10) + 10, rows * (H + pad + 10) + 10), (255, 255, 255))
for k, (label, run) in enumerate(items):
    L = open(f"{run}/map_f0200000.txt").read().split("\n")
    x0, y0, w, h = map(int, L[0].split()); g = L[1:1 + h]
    surf = [int(s) for s in open(f"{run}/surface.txt").read().split()]
    im = Image.new("RGB", (xh - xl + 1, yh - yl + 1)); px = im.load()
    dug = 0
    for y in range(yl, yh + 1):
        for x in range(xl, xh + 1):
            c = g[y - y0][x - x0]; px[x - xl, y - yl] = COL.get(c, (255, 0, 255))
            if c in ".a" and y >= surf[x] and abs(x - 128) <= 20: dug += 1
    im = im.resize((W, H), Image.NEAREST)
    alive = sum(1 for _ in open(f"{run}/ants_f0200000.txt"))
    dl = sum(1 for l in open(f"{run}/events.csv") if ",deliver," in l)
    X = 10 + (k % cols) * (W + 10); Y = 10 + (k // cols) * (H + pad + 10)
    d = ImageDraw.Draw(sheet)
    d.text((X, Y), label, fill=(0, 0, 0), font=font)
    d.text((X, Y + 20), f"{alive} alive · {dl} food home · {dug} cells dug", fill=(60, 60, 60), font=font)
    sheet.paste(im, (X, Y + pad - 6))
sheet.save(out); print(out, sheet.size)
