"""SCRATCH (nest lane, 2026-10-27): a nest picture from a run's nestmap_fF.txt + occupants_fF.txt (inrun's format, or
labsurvive `nestmap=`), in fixed colours that say what fills each cell, so open room, crumbs and crater read apart:

  open nest   cream        crater (open, no roof)  pink        sky / above ground   pale blue
  crumbs      orange       stored food             yellow      ant  black   brood  white
  open below ground, not joined to the door  grey-lilac
  soil        brown        packed soil  dark brown spoil (loose)  tan   stone  grey   plant/root  green   water  blue

usage: pictures.py png RUNDIR FRAME OUT.png        one picture
       pictures.py gif RUNDIR OUT.gif [STEP]       every nestmap_f in RUNDIR, oldest first (STEP: keep every Nth)
Crop: 100 columns each side of the door, from 14 rows above the datum to 4 rows under the deepest nest cell (or 60).
"""
import glob, os, shutil, sys, tempfile
from PIL import Image, ImageDraw
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'inrun'))
import roomshape

NEST, HALF, Z = 128, 100, 3
COL = dict(sky=(214, 230, 242), open=(250, 240, 214), crater=(242, 170, 180), crumbs=(240, 140, 40),
           food=(240, 210, 60), ant=(20, 20, 20), brood=(255, 255, 255), soil=(135, 95, 60), packedsoil=(95, 62, 38),
           spoil=(196, 160, 112), stone=(128, 128, 128), plant=(70, 150, 60), water=(60, 110, 220), other=(170, 130, 170),
           cutoff=(200, 196, 220))


def load(d, f):
    t = open(f'{d}/nestmap_f{f:07}.txt').read().split('\n')
    head = t[0].split(' ')
    x0, y0, legend = int(head[0]), int(head[1]), head[2].split(',')
    grid = [r for r in t[1:] if r]
    occ = None
    if os.path.exists(f'{d}/occupants_f{f:07}.txt'):
        occ = [r for r in open(f'{d}/occupants_f{f:07}.txt').read().split('\n')[1:] if r]
    surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    with tempfile.TemporaryDirectory() as tdir:
        shutil.copy(f'{d}/nestmap_f{f:07}.txt', f'{tdir}/nestmap.txt')
        shutil.copy(f'{d}/surface.txt', f'{tdir}/surface.txt')
        roomshape.OPEN_ONLY = False
        space, _, _, crater = roomshape.nest_cells(tdir)
    return x0, y0, legend, grid, occ, surf, space, crater


def picture(d, f, label=None):
    x0, y0, legend, grid, occ, surf, space, crater = load(d, f)
    door = NEST - x0
    deep = max((dy for _, dy in space), default=0)
    top = max(0, surf[NEST] - y0 - 14)
    bot = min(len(grid), max(deep + 5, surf[NEST] - y0 + 60))
    cols = range(door - HALF, door + HALF + 1)
    img = Image.new('RGB', (len(cols) * Z, (bot - top) * Z + (14 if label else 0)), (255, 255, 255))
    px = ImageDraw.Draw(img)
    for dy in range(top, bot):
        for i, dx in enumerate(cols):
            g = grid[dy][dx]
            name = legend[ord(g) - 65] if 'A' <= g <= 'Z' and ord(g) - 65 < len(legend) else None
            o = occ[dy][dx] if occ else '.'
            above = y0 + dy < surf[x0 + dx] if 0 <= x0 + dx < len(surf) else True
            if g == '%':
                c = COL['plant']
            elif g == '?':
                c = COL['ant'] if o == 'a' else COL['brood'] if o in 'elp' else COL['food']
            elif name == 'crumbs':
                c = COL['crumbs']
            elif name in ('fruit', 'food'):
                c = COL['food']
            elif name in ('soil', 'packedsoil', 'spoil', 'stone', 'water'):
                c = COL[name]
            elif (dx, dy) in crater:
                c = COL['crater']
            elif (dx, dy) in space:
                c = COL['open']
            elif above:
                c = COL['sky']
            elif name in ('empty', 'nest', None):
                c = COL['cutoff']  # open below the datum but not joined to the door within the window
            else:
                c = COL['other']
            yy = (dy - top) * Z + (14 if label else 0)
            px.rectangle([i * Z, yy, i * Z + Z - 1, yy + Z - 1], fill=c)
    if label:
        px.text((3, 1), label, fill=(0, 0, 0))
    return img


def frames(d):
    return sorted(int(os.path.basename(p)[9:16]) for p in glob.glob(f'{d}/nestmap_f*.txt'))


if __name__ == '__main__':
    if sys.argv[1] == 'png':
        picture(sys.argv[2], int(sys.argv[3])).save(sys.argv[4])
    else:
        step = int(sys.argv[4]) if len(sys.argv) > 4 else 1
        fs = frames(sys.argv[2])[::step]
        ims = [picture(sys.argv[2], f, f'{f // 1000}k') for f in fs]
        w, h = max(i.width for i in ims), max(i.height for i in ims)
        canvas = []
        for i in ims:
            c = Image.new('RGB', (w, h), (255, 255, 255))
            c.paste(i, (0, 0))
            canvas.append(c.convert('P', palette=Image.ADAPTIVE, colors=32))
        canvas[0].save(sys.argv[3], save_all=True, append_images=canvas[1:], duration=350, loop=0)
