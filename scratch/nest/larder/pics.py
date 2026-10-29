"""SCRATCH (chambers lane, larder seed): one picture per world, round R's end map for each arm, labelled with the
screen's counts. usage: pics.py BASEDIR OUTDIR R W[,W..] ARM:NAME [ARM:NAME ...]   (run from the repo root)"""
import sys, os
sys.path.insert(0, 'scratch/nest'); sys.path.insert(0, 'scratch/nest/larder')
from PIL import Image
import pictures
from larder import measure
base, out, R = sys.argv[1], sys.argv[2], int(sys.argv[3])
os.makedirs(out, exist_ok=True)
for w in sys.argv[4].split(','):
    ims = []
    for spec in sys.argv[5:]:
        a, name = spec.split(':', 1)
        d = f'{base}/{a}_w{w}/r{R}'
        if not os.path.exists(f'{d}/nest.csv'):
            continue
        m = measure(d)
        ps = 'n/a' if m['passage'] is None else f"{100 * m['passage']:.0f}%"
        label = (f"{name} w{w} r{R}: alive {m['alive']} starved-under {m['under']} | passage {ps} crumbs {m['crumbs']} "
                 f"pile {m['pile']} in-passage {m['inpass']} open {m['open']}")
        ims.append(pictures.picture(d, 200000, label))
    W = max(i.width for i in ims); H = sum(i.height for i in ims) + 8 * len(ims)
    o = Image.new('RGB', (W, H), 'white'); y = 0
    for i in ims:
        o.paste(i, (0, y)); y += i.height + 8
    o.save(f'{out}/w{w}_r{R}.png')
    print('wrote', f'{out}/w{w}_r{R}.png')
