"""SCRATCH (chambers crumbs lane, 2026-10-30): one PNG per world, the arms side by side at the end of round ROUND
(pictures.py colours; crumbs recoloured by source with overlay=1: red re-handled at home, orange forager delivery),
each labelled with arm, alive, passages (tunnels.py) and crumb share (nestshape).
usage: sheet.py RUNS ROUND OUTDIR ARM[:LABEL] ...   (env OVERLAY=1 for the source colours)
"""
import os, subprocess, sys, tempfile
from PIL import Image, ImageDraw
here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(here, '..')); sys.path.insert(0, os.path.join(here, '..', 'selrun'))
sys.path.insert(0, os.path.join(here, '..', '..', 'inrun')); sys.path.insert(0, here)
import pictures, tunnels, nestshape

runs, rnd, outd = sys.argv[1], int(sys.argv[2]), sys.argv[3]
arms = [a.split(':') if ':' in a else (a, a) for a in sys.argv[4:]]
os.makedirs(outd, exist_ok=True)
FR = 200000
worlds = sorted({int(p.rsplit('_w', 1)[1]) for p in os.listdir(runs) if p.startswith(arms[0][0] + '_w')})
for w in worlds:
    ims = []
    for a, lab in arms:
        d = f'{runs}/{a}_w{w}/r{rnd}'
        if not os.path.exists(f'{d}/.done'):
            im = Image.new('RGB', (603, 120), (255, 255, 255))
            ImageDraw.Draw(im).text((4, 4), f'{lab} w{w} r{rnd}: line EXTINCT before this round', fill=(0, 0, 0))
            ims.append(im); continue
        alive = int(open(f'{d}/stock.csv').read().strip().split('\n')[-1].split(',')[1])
        t = tunnels.measure(d)['tunnels']
        n = nestshape.measure(d)
        label = f'{lab} w{w} r{rnd}: alive {alive}, passages {t}, crumbs {n["crumbs"]}/{n["space"]} ({100 * n["crumb_share"]:.0f}%)'
        if os.environ.get('OVERLAY') == '1':
            tmp = tempfile.mktemp(suffix='.png')
            subprocess.run([sys.executable, f'{here}/overlay.py', d, f'{runs}/{a}_w{w}/r{rnd}.crumb.log', str(FR), tmp, label],
                           check=True, capture_output=True)
            im = Image.open(tmp).convert('RGB'); os.remove(tmp)
        else:
            im = pictures.picture(d, FR, label)
        ims.append(im)
    W = max(i.width for i in ims); H = sum(i.height for i in ims) + 4 * len(ims)
    sheet = Image.new('RGB', (W, H), (255, 255, 255)); y = 0
    for i in ims:
        sheet.paste(i, (0, y)); y += i.height + 4
    sheet.save(f'{outd}/w{w}_r{rnd}.png')
    print(f'{outd}/w{w}_r{rnd}.png')
