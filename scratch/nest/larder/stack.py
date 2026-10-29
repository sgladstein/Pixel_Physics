"""SCRATCH (chambers lane, larder seed): nest pictures stacked, each with a label line of counts.
usage: stack.py OUT.png RUNDIR:LABEL [RUNDIR:LABEL ...]   (end-of-round map, frame 200000; run from the repo root)"""
import sys, os
sys.path.insert(0, 'scratch/nest'); sys.path.insert(0, 'scratch/nest/larder')
from PIL import Image
import pictures
ims = []
for spec in sys.argv[2:]:
    d, label = spec.split(':', 1)
    ims.append(pictures.picture(d, int(os.environ.get('FRAME', 200000)), label))
W = max(i.width for i in ims); H = sum(i.height for i in ims) + 8 * len(ims)
o = Image.new('RGB', (W, H), 'white'); y = 0
for i in ims:
    o.paste(i, (0, y)); y += i.height + 8
o.save(sys.argv[1])
