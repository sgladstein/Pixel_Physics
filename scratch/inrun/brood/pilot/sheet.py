"""Contact sheet: rows = arms, columns = runs (or frames); each tile = nestmap + occupants (food, brood, ants).
usage: sheet.py OUT.png 'label|sub=dir1,dir2,...' ...   (a dir may name a snapshot: dir@0050000)"""
import sys
from PIL import Image, ImageDraw, ImageFont
COL = {'soil': (122, 92, 62), 'packedsoil': (92, 68, 46), 'spoil': (226, 150, 70), 'nest': (150, 90, 170),
       'stone': (120, 120, 125), 'corpse': (200, 40, 40), 'crumbs': (200, 170, 60)}
OCC = {'f': (240, 210, 80), 'e': (255, 255, 255), 'l': (255, 120, 170), 'p': (190, 150, 255), 'a': (90, 200, 90)}
PX, HALF, H = 4, 22, 50
font = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf', 13)
big = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf', 15)


def tile(spec):
    d, _, f = spec.partition('@')
    suf = f'_f{f}' if f else ''
    t = [r for r in open(f'{d}/nestmap{suf}.txt').read().split('\n')]
    h = t[0].split(' '); x0, y0, leg = int(h[0]), int(h[1]), h[2].split(',') if len(h) > 2 else []
    g = [r for r in t[1:] if r]
    o = [r for r in open(f'{d}/occupants{suf}.txt').read().split('\n')[1:] if r]
    surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    door = 128 - x0; a, b = door - HALF, door + HALF
    top = max(0, surf[128] - y0 - 10)
    im = Image.new('RGB', ((b - a) * PX, H * PX))
    dr = ImageDraw.Draw(im)
    n = {'f': 0, 'e': 0, 'l': 0, 'p': 0, 'c': 0}
    for y in range(top, min(len(g), top + H)):
        for x in range(a, b):
            ch, oc = g[y][x], o[y][x]
            if oc in n:
                n[oc] += 1
            if ch != '?' and leg[ord(ch) - 65] == 'crumbs' and oc not in OCC:
                n['c'] += 1
            if oc in OCC:
                c = OCC[oc]
            elif ch == '?':
                c = (60, 60, 60)
            else:
                m = leg[ord(ch) - 65]
                c = COL.get(m, (60, 60, 60)) if m != 'empty' else ((24, 20, 18) if y0 + y >= surf[x0 + x] else (52, 60, 78))
            dr.rectangle([(x - a) * PX, (y - top) * PX, (x - a + 1) * PX - 1, (y - top + 1) * PX - 1], fill=c)
    return im, n


rows = [r.split('=', 1) for r in sys.argv[2:]]
W, CAP = 2 * HALF * PX, 36
ncol = max(len(r[1].split(',')) for r in rows)
sheet = Image.new('RGB', (170 + ncol * (W + 6), len(rows) * (H * PX + CAP + 10) + 34), (14, 14, 18))
D = ImageDraw.Draw(sheet)
for i, (label, specs) in enumerate(rows):
    y = 10 + i * (H * PX + CAP + 10)
    for j, w in enumerate(label.split('|')):
        D.text((8, y + 40 + j * 20), w, fill=(235, 235, 235), font=big if j == 0 else font)
    for k, sp in enumerate(specs.split(',')):
        x = 170 + k * (W + 6)
        t, n = tile(sp)
        sheet.paste(t, (x, y))
        D.text((x, y + H * PX + 2), f"crumbs {n['c']}  brood {n['e'] + n['l'] + n['p']}", fill=(220, 220, 220), font=font)
        if '@' in sp:
            D.text((x, y + H * PX + 18), f"frame {int(sp.split('@')[1]):,}", fill=(160, 160, 160), font=font)
ly = sheet.height - 26; lx = 10
for name, c in [('soil', COL['soil']), ('spoil', COL['spoil']), ('open', (24, 20, 18)), ('sky', (52, 60, 78)), ('crumbs', COL['crumbs']), ('food', OCC['f']),
                ('egg', OCC['e']), ('larva', OCC['l']), ('pupa', OCC['p']), ('ant', OCC['a'])]:
    D.rectangle([lx, ly, lx + 12, ly + 12], fill=c); D.text((lx + 16, ly - 1), name, fill=(220, 220, 220), font=font); lx += 30 + 9 * len(name)
sheet.save(sys.argv[1])
