#!/usr/bin/env python3
"""Draw deeptrace nest maps (nest_fNNNNNN.txt) as pictures and lay them out.

Legend (deeptrace nest_map): # untouched soil, = packed wall, s spoil, r hard,
o dug open, . open never dug (sky), a adult, e/l/p egg/larva/pupa, f food,
c crumbs, x corpse, ~ water.
Usage: nestpic.py OUT.png ROWS... where each ROW is label=run1@f1,run2@f2,...
Each cell of the grid is one map; a cell 'label' column on the left names the row."""
import sys
from PIL import Image, ImageDraw
C = {'#': (92, 72, 52), '=': (130, 104, 76), 's': (150, 124, 84), 'r': (70, 70, 74),
     'o': (22, 18, 16), '.': (28, 32, 44), 'a': (230, 60, 40), 'e': (250, 250, 240),
     'l': (245, 230, 170), 'p': (220, 190, 120), 'b': (240, 240, 200), 'f': (90, 210, 80),
     'c': (240, 200, 40), 'x': (150, 40, 150), '~': (60, 120, 230), '?': (255, 0, 255)}
S = 3
TOP = 30  # sky rows above the mound left out
def draw(path):
    L = open(path).read().splitlines()[1 + TOP:]
    h, w = len(L), max(len(l) for l in L)
    im = Image.new('RGB', (w * S, h * S))
    px = im.load()
    for y, l in enumerate(L):
        for x, ch in enumerate(l):
            col = C.get(ch, (255, 0, 255))
            for i in range(S):
                for j in range(S):
                    px[x * S + i, y * S + j] = col
    return im
out = sys.argv[1]; rows = []
for a in sys.argv[2:]:
    lab, cells = a.split('=', 1)
    rows.append((lab, [c.split('@') for c in cells.split(',')]))
ims = [[(draw(f"{r}/nest_f{int(f):06}.txt"), f"{r.rstrip('/').split('/')[-1]} {int(f)//1000}k") for r, f in cs] for _, cs in rows]
cw = max(i.width for row in ims for i, _ in row); ch = max(i.height for row in ims for i, _ in row)
LW, TH, G = 90, 16, 6
sheet = Image.new('RGB', (LW + len(ims[0]) * (cw + G), len(ims) * (ch + TH + G) + 24), (250, 250, 250))
d = ImageDraw.Draw(sheet)
for ri, ((lab, _), row) in enumerate(zip(rows, ims)):
    y = ri * (ch + TH + G)
    d.text((6, y + TH + ch // 2 - 6), lab, fill=(0, 0, 0))
    for ci, (im, cap) in enumerate(row):
        x = LW + ci * (cw + G)
        d.text((x + 2, y + 2), cap, fill=(0, 0, 0))
        sheet.paste(im, (x, y + TH))
lx, ly = 6, len(ims) * (ch + TH + G) + 4
for k, name in [('a', 'adult'), ('l', 'larva'), ('e', 'egg'), ('p', 'pupa'), ('c', 'stored food (crumbs)'), ('f', 'food'), ('x', 'corpse'), ('=', 'packed wall'), ('s', 'spoil'), ('o', 'dug open'), ('#', 'soil')]:
    d.rectangle((lx, ly, lx + 12, ly + 12), fill=C[k]); d.text((lx + 16, ly), name, fill=(0, 0, 0)); lx += 30 + 7 * len(name)
sheet.save(out)
print(out, sheet.size)
