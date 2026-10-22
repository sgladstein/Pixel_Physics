import csv, sys, glob
from PIL import Image, ImageDraw
from collections import defaultdict
d, f0, f1, step, out = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]), sys.argv[5]
X0, Y0, X1, Y1 = 93, 52, 173, 90   # world coords
Z = 12
tr = defaultdict(list)
for r in csv.DictReader(open(f'{d}/trace.csv')):
    tr[int(r['frame'])].append(r)
frames = []
for f in range(f0, f1 + 1, step):
    im = Image.open(f'{d}/film/f{f:07d}.png').convert('RGB')   # film x = world x - 48, y = world y - 30
    im = im.crop((X0 - 48, Y0 - 30, X1 - 48, Y1 - 30)).resize(((X1 - X0) * Z, (Y1 - Y0) * Z), Image.NEAREST)
    dr = ImageDraw.Draw(im)
    for r in tr.get(f, []):
        x, y = int(r['x']), int(r['y'])
        if not (X0 <= x < X1 and Y0 <= y < Y1): continue
        tank = float(r['tank']); crop = int(r['crop']); sp = int(r['spoil'])
        col = (255, 60, 60) if tank < 3 else (60, 255, 60) if crop > 0 else (255, 230, 0) if sp else (255, 255, 255)
        px, py = (x - X0) * Z, (y - Y0) * Z
        dr.rectangle([px, py, px + Z - 1, py + Z - 1], outline=col, width=2)
    dr.text((6, 6), f'frame {f}   red=tank<3  green=carrying food  yellow=carrying soil  white=other', fill=(255, 255, 255))
    frames.append(im)
frames[0].save(out + '.gif', save_all=True, append_images=frames[1:], duration=100, loop=0)
for i in (0, len(frames) // 2, len(frames) - 1):
    frames[i].save(f'{out}_{i}.png')
print(len(frames))
