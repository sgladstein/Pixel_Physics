"""SCRATCH (chambersense lane, 2026-10-30): a contact sheet of end-of-round nests, one row per world, one column per
arm, with alive / passages (tunnels.py) / open rooms (roomshape OPEN_ONLY) printed on each picture.

    python3 scratch/nest/chambersense/sheet.py OUT.png ROUND ROOT ARM[,ARM...] WORLD[,WORLD...] [counts.tsv]
ROOT/ARM/wW/rROUND is a poolrounds round folder. Also prints the counts as a table (and writes it to counts.tsv).
"""
import os, sys, csv
from PIL import Image, ImageDraw
here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(here, '..'))
sys.path.insert(0, os.path.join(here, '..', 'selrun'))
sys.path.insert(0, os.path.join(here, '..', '..', 'inrun'))
import pictures, tunnels, roomshape

out, rnd, root, arms, worlds = sys.argv[1], int(sys.argv[2]), sys.argv[3], sys.argv[4].split(','), sys.argv[5].split(',')
tsv = sys.argv[6] if len(sys.argv) > 6 else None


def alive(d):
    r = list(csv.reader(open(f'{d}/sel.csv')))
    return int(r[-1][1])


def counts(d):
    if not os.path.exists(f'{d}/.done'):
        return None
    t = tunnels.measure(d) or {}
    roomshape.OPEN_ONLY = True
    p = roomshape.parts(d)
    roomshape.OPEN_ONLY = False
    return dict(alive=alive(d), tunnels=t.get('tunnels', 0), rooms=len(p.get('rooms', [])), roomcells=sum(p.get('rooms', [])),
                open=t.get('open', 0), score=p.get('score', 0))


cells, rows = {}, []
for w in worlds:
    for a in arms:
        d = f'{root}/{a}/w{w}/r{rnd}'
        c = counts(d)
        rows.append((w, a, c))
        if c is None:
            continue
        lab = f'{a} w{w} r{rnd}: alive {c["alive"]}  passages {c["tunnels"]}  open rooms {c["rooms"]} ({c["roomcells"]} cells)'
        cells[(w, a)] = pictures.picture(d, 200000, lab) if os.path.exists(f'{d}/nestmap_f0200000.txt') else None
ims = [im for im in cells.values() if im is not None]
if ims:
    cw, ch = max(i.width for i in ims), max(i.height for i in ims)
    sheet = Image.new('RGB', (cw * len(arms), ch * len(worlds)), (255, 255, 255))
    for (w, a), im in cells.items():
        if im is not None:
            sheet.paste(im, (arms.index(a) * cw, worlds.index(w) * ch))
    sheet.save(out)
print('world\tarm\talive\tpassages\topen_rooms\troom_cells\topen\troomshape')
lines = []
for w, a, c in rows:
    s = f'{w}\t{a}\t' + ('\t'.join(str(c[k]) for k in ('alive', 'tunnels', 'rooms', 'roomcells', 'open', 'score')) if c else 'missing')
    print(s)
    lines.append(s)
if tsv:
    open(tsv, 'w').write('world\tarm\talive\tpassages\topen_rooms\troom_cells\topen\troomshape\n' + '\n'.join(lines) + '\n')
