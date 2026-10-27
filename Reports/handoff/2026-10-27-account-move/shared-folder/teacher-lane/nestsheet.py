"""Contact sheet of end maps: rows = arms, 8 colonies each at evenly spaced quantiles of kept-nest size."""
import sys, os, glob
sys.path.insert(0, '/home/user/cloud/scratch/inrun')
import nestcount, roomshape
from PIL import Image, ImageDraw, ImageFont
out = sys.argv[1]; rows = [a.split('=', 1) for a in sys.argv[2:]]   # label=glob
COL = {'soil': (122, 92, 62), 'packedsoil': (92, 68, 46), 'spoil': (226, 150, 70), 'nest': (150, 90, 170),
       'stone': (120, 120, 125), 'corpse': (200, 40, 40), 'crumbs': (240, 210, 80), 'food': (240, 210, 80)}
PX, X0, X1 = 4, 30, 70   # window columns shown (door at 50 in a 100-wide map whose x0 = 78)
font = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf', 13)
big = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf', 16)
def tile(d):
    t = open(d + '/nestmap.txt').read().split('\n'); h = t[0].split(' '); x0, y0, leg = int(h[0]), int(h[1]), h[2].split(',')
    g = [r for r in t[1:] if r]; surf = [int(l) for l in open(d + '/surface.txt') if l.strip()]
    door = 128 - x0; a, b = door - 20, door + 20
    im = Image.new('RGB', ((b - a) * PX, len(g) * PX))
    dr = ImageDraw.Draw(im)
    for y, row in enumerate(g):
        for x in range(a, b):
            ch = row[x]
            if ch == '?': c = (90, 200, 90)
            else:
                m = leg[ord(ch) - 65]
                c = COL.get(m, (60, 60, 60)) if m != 'empty' else ((24, 20, 18) if y0 + y >= surf[x0 + x] else (52, 60, 78))
            dr.rectangle([(x - a) * PX, y * PX, (x - a + 1) * PX - 1, (y + 1) * PX - 1], fill=c)
    return im, surf[128] - y0
W = 40 * PX; H = 52 * PX; CAP = 34
sheet = Image.new('RGB', (150 + 8 * (W + 6), len(rows) * (H + CAP + 10) + 40), (14, 14, 18))
D = ImageDraw.Draw(sheet)
for i, (label, pat) in enumerate(rows):
    ds = [d for d in glob.glob(pat) if os.path.exists(d + '/nestmap.txt')]
    sc = sorted(((nestcount.count(d)['nest'], d) for d in ds))
    pick = [sc[round(k * (len(sc) - 1) / 7)] for k in range(8)] if len(sc) >= 8 else sc
    y = 10 + i * (H + CAP + 10)
    for j, w in enumerate(label.split('|')):
        D.text((8, y + 40 + j * 20), w, fill=(235, 235, 235), font=big if j == 0 else font)
    for k, (n, d) in enumerate(pick):
        x = 150 + k * (W + 6); t, top = tile(d); top = max(0, (top - 12) * PX); sheet.paste(t.crop((0, top, W, top + H)), (x, y))
        c = nestcount.count(d)
        D.text((x, y + H + 2), f"nest {c['nest']}  crater {c['crater']}", fill=(220, 220, 220), font=font)
        D.text((x, y + H + 17), f"spoil {c['spoil']}  rooms {roomshape.score(d):.0f}", fill=(220, 220, 220), font=font)
ly = sheet.height - 26; lx = 8
for name, c in [('open, below ground', (24, 20, 18)), ('sky', (52, 60, 78)), ('soil', (122, 92, 62)), ('spoil (dug, loose)', (226, 150, 70)), ('food', (240, 210, 80)), ('ants / plants', (90, 200, 90)), ('nest mouth', (150, 90, 170))]:
    D.rectangle([lx, ly, lx + 14, ly + 14], fill=c); D.text((lx + 20, ly), name, fill=(220, 220, 220), font=font); lx += 30 + int(D.textlength(name, font=font))
sheet.save(out)
