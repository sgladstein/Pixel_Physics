import csv, collections, sys, bisect
from PIL import Image, ImageDraw
S = sys.argv[1]; SC = 2; Y0 = 34
arms = [('free', 'Signalling colony (hears the new scent)'), ('deaf', 'Scent-free colony')]
def heapcum(a):
    heaps = [(int(r['frame']), int(r['x'])) for r in csv.DictReader(open(f'{S}/film/{a}/patches.csv'))]
    q = collections.defaultdict(collections.deque); t = []
    for r in csv.reader(open(f'{S}/film/{a}/events.csv')):
        if len(r) < 6 or r[0] == 'frame': continue
        k = r[2]
        if k == 'field':
            f = int(r[0]); hx = [x for fr, x in heaps if fr <= f][-1]; q[r[1]].append(abs(int(r[3]) - hx) <= 15)
        elif k in ('deliver', 'churn', 'fdrop'):
            ok = q[r[1]].popleft() if q[r[1]] else False
            if k == 'deliver' and ok: t.append(int(r[0]))
    return t
def laden(a):
    d = collections.defaultdict(list)
    for r in csv.DictReader(open(f'{S}/film/{a}/trace.csv')):
        if int(r['crop']) > 0: d[int(r['frame'])].append((int(r['x']), int(r['y'])))
    return d
H = {a: heapcum(a) for a, _ in arms}; L = {a: laden(a) for a, _ in arms}
frames = []
for fr in range(400, 100001, 400):
    panels = []
    for a, label in arms:
        im = Image.open(f'{S}/film/{a}/film/f{fr:07}.png').convert('RGB').crop((0, 4, 448, 72))
        im = im.resize((im.width * SC, im.height * SC), Image.NEAREST)
        d = ImageDraw.Draw(im)
        for x, y in L[a].get(fr, []):
            cx, cy = x * SC, (y - Y0) * SC
            d.rectangle([cx - 2, cy - 2, cx + 3, cy + 3], fill=(255, 225, 0))
        d.rectangle([0, 0, im.width, 16], fill=(0, 0, 0))
        n = bisect.bisect_right(H[a], fr)
        d.text((4, 3), f'{label}   frame {fr}   food from the heap delivered: {n}', fill=(255, 255, 255))
        d.line([(128 * SC, 16), (128 * SC, 26)], fill=(255, 255, 255), width=2)
        panels.append(im)
    out = Image.new('RGB', (panels[0].width, sum(p.height for p in panels) + 6), (0, 0, 0))
    out.paste(panels[0], (0, 0)); out.paste(panels[1], (0, panels[0].height + 6))
    frames.append(out.quantize(colors=200, method=Image.Quantize.FASTOCTREE))
frames[0].save(sys.argv[2], save_all=True, append_images=frames[1:], duration=80, loop=0, optimize=True)
print({a: len(H[a]) for a in H})
