"""SCRATCH (teacher lane): an `inrun` run's maps as a GIF, alive count in the corner.

usage: python3 -I gif.py RUNDIR OUT.gif [x_lo x_hi y_lo y_hi] [scale]
"""
import sys, glob, re
from PIL import Image, ImageDraw

run, out = sys.argv[1], sys.argv[2]
xl, xh, yl, yh = (int(v) for v in sys.argv[3:7]) if len(sys.argv) > 6 else (88, 168, 40, 90)
scale = int(sys.argv[7]) if len(sys.argv) > 7 else 5
COL = {".": (214, 228, 240), "#": (120, 92, 64), "f": (90, 170, 70), "a": (20, 20, 20)}
frames = []
for p in sorted(glob.glob(f"{run}/map_f*.txt")):
    fr = int(re.search(r"map_f(\d+)", p).group(1))
    L = open(p).read().split("\n")
    x0, y0, w, h = map(int, L[0].split())
    rows = L[1:1 + h]
    im = Image.new("RGB", (xh - xl + 1, yh - yl + 1))
    px = im.load()
    for y in range(yl, yh + 1):
        r = rows[y - y0]
        for x in range(xl, xh + 1):
            px[x - xl, y - yl] = COL.get(r[x - x0], (255, 0, 255))
    im = im.resize((im.width * scale, im.height * scale), Image.NEAREST)
    try:
        alive = sum(1 for _ in open(f"{run}/ants_f{fr:07d}.txt"))
    except OSError:
        alive = -1
    d = ImageDraw.Draw(im)
    d.rectangle([0, 0, 230, 18], fill=(255, 255, 255))
    d.text((4, 3), f"frame {fr:6d}   ants alive {alive}", fill=(0, 0, 0))
    frames.append(im)
frames[0].save(out, save_all=True, append_images=frames[1:], duration=250, loop=0)
print(out, len(frames), "frames")
