"""End map with every near-door cut of original ground: cyan = paid under cut_join, red = unjoined (not paid)."""
import sys, csv
from PIL import Image, ImageDraw, ImageFont
out = sys.argv[1]; runs = [a.split('=', 1) for a in sys.argv[2:]]
COL = {'soil': (122, 92, 62), 'packedsoil': (92, 68, 46), 'spoil': (226, 150, 70), 'nest': (150, 90, 170), 'stone': (120, 120, 125), 'corpse': (200, 40, 40), 'crumbs': (240, 210, 80)}
PX = 6; font = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf', 14); big = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf', 15)
tiles = []
for label, d in runs:
    t = open(d + '/nestmap.txt').read().split('\n'); h = t[0].split(' '); x0, y0, leg = int(h[0]), int(h[1]), h[2].split(',')
    g = [r for r in t[1:] if r]; surf = [int(l) for l in open(d + '/surface.txt') if l.strip()]
    door = 128 - x0; a, b = door - 22, door + 23; top = max(0, surf[128] - y0 - 10); bot = min(len(g), top + 45)
    im = Image.new('RGB', ((b - a) * PX, (bot - top) * PX + 44), (14, 14, 18)); dr = ImageDraw.Draw(im)
    for y in range(top, bot):
        for x in range(a, b):
            ch = g[y][x]
            c = (90, 200, 90) if ch == '?' else (COL.get(leg[ord(ch) - 65], (60, 60, 60)) if leg[ord(ch) - 65] != 'empty' else ((24, 20, 18) if y0 + y >= surf[x0 + x] else (52, 60, 78)))
            dr.rectangle([(x - a) * PX, (y - top) * PX, (x - a + 1) * PX - 1, (y - top + 1) * PX - 1], fill=c)
    un, paid = set(), set()
    for e in csv.DictReader(open(d + '/events.csv')):
        if e['kind'] == 'unjoined': un.add((int(e['x']), int(e['y'])))
        elif e['kind'] == 'dig': paid.add((int(e['x']), int(e['y'])))
    paid -= un
    for s, c in ((paid, (40, 210, 230)), (un, (255, 40, 40))):
        for X, Y in s:
            x, y = X - x0, Y - y0
            if a <= x < b and top <= y < bot:
                cx, cy = (x - a) * PX + PX // 2, (y - top) * PX + PX // 2
                dr.ellipse([cx - 2, cy - 2, cx + 2, cy + 2], fill=c)
    nu = sum(1 for X, Y in un if abs(X - 128) <= 20); npd = sum(1 for X, Y in paid if abs(X - 128) <= 20)
    dr.text((4, (bot - top) * PX + 4), label, fill=(235, 235, 235), font=big)
    dr.text((4, (bot - top) * PX + 23), f'paid {npd}   not paid {nu}', fill=(220, 220, 220), font=font)
    tiles.append(im)
W = sum(t.width for t in tiles) + 8 * (len(tiles) + 1); H = max(t.height for t in tiles) + 40
sheet = Image.new('RGB', (W, H), (14, 14, 18)); x = 8
for t in tiles: sheet.paste(t, (x, 8)); x += t.width + 8
D = ImageDraw.Draw(sheet); lx = 8
for name, c in [('cut, paid (touches the open nest)', (40, 210, 230)), ('cut, NOT paid (cut_join)', (255, 40, 40)), ('spoil', (226, 150, 70)), ('soil', (122, 92, 62)), ('open below ground', (24, 20, 18))]:
    D.ellipse([lx, H - 24, lx + 12, H - 12], fill=c); D.text((lx + 18, H - 27), name, fill=(220, 220, 220), font=font); lx += 30 + int(D.textlength(name, font=font))
sheet.save(out)
