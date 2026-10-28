import csv, collections, sys
from PIL import Image, ImageDraw
S = sys.argv[1]; F0, F1, STEP, SC = int(sys.argv[2]), int(sys.argv[3]), 8, 5
X0, Y0 = 128 - 40, 34
arms = [('deaf', 'Scent-deaf best'), ('free', 'Signalling best')]
def marks(a):
    last = {}; rep = collections.defaultdict(list); n = collections.Counter()
    for r in csv.reader(open(f'{S}/film/{a}/events.csv')):
        if len(r) < 6 or r[0] == 'frame': continue
        f, k = int(r[0]), r[2]
        if k in ('deliver', 'churn', 'fdrop'): last[(r[3], r[4])] = k
        if k == 'deliver' and F0 <= f < F1: n['deliver'] += 1
        if k == 'field':
            t = last.pop((r[3], r[4]), None)
            if t == 'deliver':
                rep[f].append((int(r[3]), int(r[4])))
                if F0 <= f < F1: n['repick'] += 1
    return rep, n
M = {a: marks(a) for a, _ in arms}
frames = []
for fr in range(F0, F1, STEP):
    panels = []
    for a, label in arms:
        im = Image.open(f'{S}/film/{a}/film/f{fr:07}.png').convert('RGB').crop((0, 4, 81, 72))
        im = im.resize((im.width * SC, im.height * SC), Image.NEAREST)
        d = ImageDraw.Draw(im)
        rep = M[a][0]
        for f in range(fr - 120, fr + 1):
            for x, y in rep.get(f, []):
                cx, cy = (x - X0) * SC + SC // 2, (y - Y0) * SC + SC // 2
                r = 8
                d.ellipse([cx - r, cy - r, cx + r, cy + r], outline=(255, 0, 255), width=3)
        d.rectangle([0, 0, im.width, 16], fill=(0, 0, 0))
        d.text((4, 3), f'{label}  frame {fr}', fill=(255, 255, 255))
        panels.append(im)
    W = sum(p.width for p in panels) + 8
    out = Image.new('RGB', (W, panels[0].height), (0, 0, 0))
    x = 0
    for p in panels: out.paste(p, (x, 0)); x += p.width + 8
    frames.append(out.quantize(colors=200, method=Image.Quantize.FASTOCTREE))
frames[0].save(sys.argv[4], save_all=True, append_images=frames[1:], duration=50, loop=0, optimize=True)
for a, _ in arms: print(a, dict(M[a][1]))
