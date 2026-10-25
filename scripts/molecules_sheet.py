#!/usr/bin/env python3
"""Contact sheet for examples/molecules.rs: one row per molecule, one column per
snapshot, on a fixed log ramp (dark = nothing, bright = full deposit), so a faint
tail an ant could still read stays visible and rows compare directly."""
import math, struct, sys
from PIL import Image, ImageDraw

W, H, SCALE, Z = 160, 48, 256, 3
ROWS = ["road", "today_A", "today_B", "fresh", "flash", "alarm", "call"]
LABEL = {"road": "road (ground, no fade, slow spread)", "today_A": "today's A (ground, no fade)",
         "today_B": "today's B (ground, fade 0.03)", "fresh": "fresh (ground, fade 0.10)",
         "flash": "flash (ground, fade 0.25, fast blend)", "alarm": "alarm (air, fade 0.35)",
         "call": "call (air, fade 0.10, far reach)"}
FRAMES = [0, 60, 180, 1200, 4800]
src, out = sys.argv[1], sys.argv[2]
top, left, gap = 18, 230, 6
img = Image.new("RGB", (left + len(FRAMES) * (W * Z + gap), top + len(ROWS) * (H * Z + gap)), (12, 12, 16))
d = ImageDraw.Draw(img)
for j, f in enumerate(FRAMES):
    d.text((left + j * (W * Z + gap) + 4, 3), f"frame {f}", fill=(220, 220, 220))
def ramp(v):
    if v == 0:
        return (0, 0, 0)
    t = min(1.0, math.log1p(v / SCALE * 16) / math.log1p(40 * 16))  # 1/16 of a unit .. a full trail deposit
    return (int(255 * min(1, t * 1.6)), int(255 * max(0, min(1, t * 1.6 - 0.45))), int(255 * max(0, t * 2.2 - 1.2)))
for i, r in enumerate(ROWS):
    y0 = top + i * (H * Z + gap)
    d.text((6, y0 + H * Z // 2 - 6), LABEL[r], fill=(220, 220, 220))
    for j, f in enumerate(FRAMES):
        raw = open(f"{src}/{r}_{f}.u16", "rb").read()
        vals = struct.unpack(f"<{W*H}H", raw)
        tile = Image.new("RGB", (W, H))
        tile.putdata([ramp(v) for v in vals])
        img.paste(tile.resize((W * Z, H * Z), Image.NEAREST), (left + j * (W * Z + gap), y0))
img.save(out)
print(out)
