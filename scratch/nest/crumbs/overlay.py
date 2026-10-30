"""SCRATCH (chambers crumbs lane, 2026-10-30): the end-of-round nest picture (scratch/nest/pictures.py, its colours and
crop, from RUNDIR/nestmap_f<FRAME>.txt) with each standing crumb in the nest recoloured by who left it there
(crumbsrc.py's join, run on RUNDIR/nestmap.txt -- the same map as the last snapshot):
  red      re-handled: the load's first cell was picked up AT HOME, and it was put down again here
  orange   field: a forager's load, carried home and put down here
  violet   not joined to any put-down
usage: overlay.py RUNDIR LOG FRAME OUT.png [LABEL]
"""
import os, sys
from PIL import ImageDraw
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pictures, crumbsrc

d, logp, fr, outp = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
label = sys.argv[5] if len(sys.argv) > 5 else None
c = crumbsrc.classify(d, logp)
src = {r['_cell']: ('rehandled' if r['first_home'] else 'field') for r in c['rows']}
img = pictures.picture(d, fr, label)
x0, y0, legend, grid, occ, surf, space, crater = pictures.load(d, fr)
door = pictures.NEST - x0
top = max(0, surf[pictures.NEST] - y0 - 14)
Z, H = pictures.Z, pictures.HALF
off = 14 if label else 0
COL = dict(rehandled=(200, 30, 30), field=(240, 140, 40), none=(150, 80, 200))
px = ImageDraw.Draw(img)
for (x, y) in c['crumb_cells']:
    i, dy = x - x0 - (door - H), y - y0
    if 0 <= i <= 2 * H and dy >= top and (dy - top) * Z + off < img.height:
        yy = (dy - top) * Z + off
        px.rectangle([i * Z, yy, i * Z + Z - 1, yy + Z - 1], fill=COL[src.get((x, y), 'none')])
img.save(outp)
n = len(c['crumb_cells'])
k = sum(1 for p in c['crumb_cells'] if src.get(p) == 'rehandled')
print(f'{outp}: crumbs {n}, re-handled {k}, field {sum(1 for p in c["crumb_cells"] if src.get(p) == "field")}')
