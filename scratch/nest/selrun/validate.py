"""SCRATCH (chambers selection run, 2026-10-29): is scratch/inrun/nestshape.py crumb-proof, and does it still see tunnels?

(1) Specificity/sensitivity on real nests: the eat-seed screen's poolrounds.sh runs (4 worlds x 4 rounds; H3 = known
    tunnels in the pictures, std_pool = known cone, H3+eat = "bigger crumb-filled cones and blobs" in the pictures).
(2) The known false positive, built on purpose: copy 3 (dead-ends creatures:192) dug a narrow shaft packed with crumbs
    that roomcopy/score.py's gallery share read as 20-41% passages. Its run folders are not in this container, so the
    case is constructed: every H3 round-3 nest is (a) emptied of its tunnels -- each nest cell outside its 5x5 rooms
    turned back to soil -- and a 3-wide shaft 30 deep cut below the door and packed with crumbs at 70%, the rest open
    (the copy-3 shape); (b) the same shaft open. A crumb-proof measure scores (a) near zero and (b) as a passage; the
    old share scores both as passages.
usage: validate.py BASE ARM:SUBDIR ...      e.g. validate.py SCRATCH h3:r4/h3 std:r4/std h3eat:eat/r4/h3n

Result, 2026-10-29 (the screen's own runs, main 678a4fb4 + logging): galleryc H3 149-412 against std_pool 65-171, higher
in 16 of 16 world-round pairs; thin_old agrees on those but read H3+eat w68 r3 (the crumb blob in its picture) as 64%
passages where galleryc gives 93, std_pool level. `gallery` (joined to the door through open cells) is not used: a crumb
plug under the mouth cuts its reach to 3-17 cells (gallery 0-3) in 5 of 16 H3 nests whose galleryc is 229-376. Constructed copy-3
shaft: thin_old cannot tell a packed shaft from an open one (same value both ways in all four); galleryc drops it where
the shaft stands apart from the rooms (w73 111 -> 21, w68 97 -> 48). In w61 and w64 the door sits in a crater, so the
constructed shaft is crater and neither version counts -- the construction, not the measure. The real copy-3 folders
(Nest lane, scratch/nest/roomcopy/) were not in this container: run `nestshape.py` on them to close the check.
"""
import os, random, shutil, sys, tempfile
sys.path.insert(0, 'scratch/inrun')
import nestshape, roomshape

base, arms = sys.argv[1], [a.split(':') for a in sys.argv[2:]]
KEYS = ('space', 'crumb_share', 'reach', 'gallery', 'galleryc', 'rooms_open', 'thin_old')
print('arm    world rnd ' + ' '.join(f'{k:>11s}' for k in KEYS))
for arm, sub in arms:
    for w in (68, 73, 61, 64):
        for r in range(4):
            d = f'{base}/{sub}_w{w}/r{r}'
            m = nestshape.measure(d)
            if m:
                print(f'{arm:6s} w{w}   r{r}  ' + ' '.join(f'{m[k]:>11}' for k in KEYS))


def shaft(d, crumbfrac, seed):
    """d's nest with its passages filled back to soil and a 3-wide, 30-deep shaft under the door (crumbs at
    crumbfrac): a folder holding the edited map and d's surface."""
    t = open(f'{d}/nestmap.txt').read().split('\n')
    head, legend = t[0], t[0].split(' ')[2].split(',')
    grid = [list(r) for r in t[1:] if r]
    x0, y0 = int(head.split(' ')[0]), int(head.split(' ')[1])
    surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    soil, crumbs, empty = (chr(65 + legend.index(k)) for k in ('soil', 'crumbs', 'empty'))
    space, _, _, crater = roomshape.nest_cells(d)
    blk = [(a, b) for a in range(-2, 3) for b in range(-2, 3)]
    cen = {p for p in space if all((p[0] + a, p[1] + b) in space for a, b in blk)}
    rooms = {(c[0] + a, c[1] + b) for c in cen for a, b in blk}
    for (x, y) in space - rooms - crater:
        grid[y][x] = soil
    rng = random.Random(seed)
    door = roomshape.NEST - x0
    top = surf[roomshape.NEST] - y0
    for y in range(max(0, top), min(len(grid), top + 30)):
        for x in range(door - 1, door + 2):
            grid[y][x] = crumbs if rng.random() < crumbfrac else empty
    tmp = tempfile.mkdtemp()
    open(f'{tmp}/nestmap.txt', 'w').write(head + '\n' + '\n'.join(''.join(r) for r in grid) + '\n')
    shutil.copy(f'{d}/surface.txt', f'{tmp}/surface.txt')
    return tmp


print('\nconstructed copy-3 shaft on each H3 round-3 nest (rooms kept, passages filled, 3x30 shaft under the door)')
print('case          world ' + ' '.join(f'{k:>11s}' for k in KEYS))
sub = dict(arms).get('h3')
for w in (68, 73, 61, 64):
    d = f'{base}/{sub}_w{w}/r3'
    for name, frac in (('packed 70%', 0.7), ('open shaft', 0.0)):
        tmp = shaft(d, frac, w)
        m = nestshape.measure(tmp)
        shutil.rmtree(tmp, ignore_errors=True)
        print(f'{name:12s}  w{w}  ' + ' '.join(f'{m[k]:>11}' for k in KEYS))
