"""Paired nest pictures (pictures.py colours) with every ant that starved underground in the round marked: red ring =
died in a pocket with no open way to the air (4k map before death), magenta ring = had a way out. Counts in the label.
usage: overlay.py OUT.png FRAME RUNDIR:LABEL RUNDIR:LABEL"""
import sys, os, csv, glob, subprocess
sys.path.insert(0, '/home/user/Pixel_Physics/.claude/worktrees/agent-a66af3a6cb52f6ee7/scratch/nest')
import pictures
from PIL import Image, ImageDraw
NEST, HALF, Z = 128, 100, 3
out, F = sys.argv[1], int(sys.argv[2])
ims = []
for spec in sys.argv[3:]:
    d, lab = spec.split(':', 1)
    x0, y0, legend, grid, occ, surf, space, crater = pictures.load(d, F)
    top = max(0, surf[NEST] - y0 - 14)
    o = subprocess.run([sys.executable, 'pocket.py', d, '--mapf'], capture_output=True, text=True).stdout
    shut = set()
    for l in o.split('\n'):
        if l.strip().startswith('shut-in'):
            for t in l.split('(')[2:]:
                p = t.split(','); shut.add((int(p[0]), int(p[1]), int(p[2])))
    last = {}
    for r in csv.DictReader(open(f'{d}/trace.csv')): last[int(r['id'])] = (int(r['x']), int(r['y']))
    deaths = []
    for r in csv.reader(open(f'{d}/events.csv')):
        if r[0].isdigit() and r[2] == 'death' and int(float(r[5])) == 1 and int(r[1]) in last:
            x, y = last[int(r[1])]
            if y > surf[x]: deaths.append((int(r[0]) // 1000, x, y))
    ns = sum((f, x - NEST, y - surf[x]) in shut for f, x, y in deaths)
    alive = list(csv.DictReader(open(f'{d}/nest.csv')))[-1]['alive']
    label = f'{lab}: alive at end {alive}; starved underground {len(deaths)} (red: shut in {ns}) -- nest at {F // 1000}k'
    img = pictures.picture(d, F, label)
    dr = ImageDraw.Draw(img)
    for f, x, y in deaths:
        px = (x - (NEST - HALF)) * Z + 1; py = (y - y0 - top) * Z + 14 + 1
        c = (220, 20, 20) if (f, x - NEST, y - surf[x]) in shut else (200, 0, 200)
        dr.ellipse([px - 3, py - 3, px + 3, py + 3], outline=c, width=1)
    ims.append(img)
w = max(i.width for i in ims); h = sum(i.height for i in ims) + 6 * len(ims)
c = Image.new('RGB', (w, h), (255, 255, 255)); y = 0
for i in ims: c.paste(i, (0, y)); y += i.height + 6
c.save(out)
print(out, c.size)
