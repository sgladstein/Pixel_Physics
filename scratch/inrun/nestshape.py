"""SCRATCH (chambers selection run, 2026-10-29): crumb-proof nest-shape measures of one run's end map (or a snapshot).

The question they serve: does colony selection keep a tunnel nest (the Nest lane's H3 dig head, with the chambers
lane's eat seed) when nest shape counts in the score. The gallery share the Nest lane used
(`scratch/nest/roomcopy/score.py`: nest space outside any 5x5 open block, crumbs counted as space) was fooled by a narrow
crumb-packed shaft -- copy 3 scored 20-41% on nests with no passages (`Reports/dead-ends.md` creatures:192) -- so every
measure here treats crumbs and stored food as WALL, and a passage counts only if an ant can walk it from the door
through open cells. A shaft packed with crumbs is then a dead end, not a passage.

  space        roomshape's nest space: every non-ground cell joined to the door below the datum (nest + crater), the
               flood passing crumbs and food as roomshape's does
  crumbs, food crumb cells, and stored fruit/food cells, in the space
  crumb_share  crumbs / space: the separate penalty -- a nest that is mostly crumbs scores high here whatever its shape
  open         space cells that are not crumbs or food (ants and door markers count as open, as roomshape OPEN_ONLY)
  reach        open cells an ant can walk to from the door through open cells only (4-connected)
  gallery      reach cells outside the crater and outside every 5x5 block of reach (rooms): walkable passages. **The
               deciding measure** (count, not share: a share of a tiny open space can be 100% passage)
  rooms_open   roomshape.score with OPEN_ONLY (colsel's `+shape` under `shapeopen=1`): second rooms and side tunnels
  thin_old     roomcopy/score.py's gallery share (crumbs as space), kept only to show where it disagrees

    python3 scratch/inrun/nestshape.py RUNDIR [RUNDIR...]          one line each (end map)
    python3 scratch/inrun/nestshape.py --snap RUNDIR                every nestmap_f*.txt snapshot in it, in frame order
import: measure(rundir, mapfile=None) -> dict; crumb_share(rundir) -> float (colsel's `+crumbs` case)
"""
import importlib.util, os, shutil, sys, tempfile
from collections import deque

# A private copy of roomshape, its OPEN_ONLY always False: colsel sets the shared module's flag for `shapeopen=1`, and
# its garden threads read it while this runs, so flipping the shared flag here would race them.
_spec = importlib.util.spec_from_file_location('_roomshape_nestshape', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'roomshape.py'))
roomshape = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(roomshape)
roomshape.OPEN_ONLY = False

K = 2  # a room fits a 5x5 block, as roomshape
MINC = 20  # galleryc: the smallest open piece that counts (a crumb-packed shaft's gaps are 1-10 cells)
N4 = ((1, 0), (-1, 0), (0, 1), (0, -1))
FOOD = ('crumbs', 'fruit', 'food')


def _grid(d):
    t = open(f'{d}/nestmap.txt').read().split('\n')
    head = t[0].split(' ')
    return head[2].split(','), [r for r in t[1:] if r]


def measure(d, mp=None):
    if mp is not None:  # a snapshot: roomshape reads RUNDIR/nestmap.txt, so lend it a folder holding this map
        tmp = tempfile.mkdtemp()
        try:
            shutil.copy(mp, f'{tmp}/nestmap.txt')
            shutil.copy(f'{d}/surface.txt', f'{tmp}/surface.txt')
            return measure(tmp)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
    if not os.path.exists(f'{d}/nestmap.txt'):
        return None
    space, entry, spoil, crater = roomshape.nest_cells(d)
    legend, grid = _grid(d)
    ch = lambda p: grid[p[1]][p[0]]
    crumb = {chr(65 + k) for k, m in enumerate(legend) if m == 'crumbs'}
    food = {chr(65 + k) for k, m in enumerate(legend) if m in FOOD}
    crumbs = sum(1 for p in space if ch(p) in crumb)
    nfood = sum(1 for p in space if ch(p) in food) - crumbs
    opn = {p for p in space if ch(p) not in food}
    # the door's open cell: roomshape's OPEN_ONLY entry (within a column of the door, highest), over open cells only
    door = roomshape.NEST - int(open(f'{d}/nestmap.txt').readline().split()[0])
    s0 = min((p for p in opn if abs(p[0] - door) <= 1), key=lambda p: p[1], default=None)
    # rooms_open: exactly roomshape.score under OPEN_ONLY (colsel `+shape` with `shapeopen=1`)
    po = roomshape.shape(opn, s0, spoil, crater & opn)
    reach = set()
    if s0 is not None:
        reach, q = {s0}, deque([s0])
        while q:
            p = q.popleft()
            for a, b in N4:
                r = (p[0] + a, p[1] + b)
                if r in opn and r not in reach:
                    reach.add(r)
                    q.append(r)
    blk = [(a, b) for a in range(-K, K + 1) for b in range(-K, K + 1)]
    centres = {p for p in reach if all((p[0] + a, p[1] + b) in reach for a, b in blk)}
    rooms = {(c[0] + a, c[1] + b) for c in centres for a, b in blk}
    gallery = reach - rooms - crater
    # galleryc: the same over every open piece of at least MINC cells, joined to the door or not. A crumb plug in the
    # door's shaft (the trained cone's pile under the mouth) cuts `reach` to a handful of cells whatever lies behind it,
    # so `gallery` partly measures the plug; galleryc keeps the passages behind it and still drops a crumb-packed
    # shaft, whose open cells are the crumbs' gaps (pieces of 1-10 cells).
    big = set()
    for comp in roomshape.pieces(opn - crater):
        if len(comp) >= MINC:
            big |= comp
    cc = {p for p in big if all((p[0] + a, p[1] + b) in big for a, b in blk)}
    rooms_c = {(c[0] + a, c[1] + b) for c in cc for a, b in blk}
    galleryc = big - rooms_c
    # roomcopy/score.py's thin, for comparison only (crumbs and food as space: the measure that was fooled)
    ctr = {p for p in space if all((p[0] + a, p[1] + b) in space for a, b in blk)}
    room_all = {(c[0] + a, c[1] + b) for c in ctr for a, b in blk}
    n = len(space)
    return dict(space=n, crumbs=crumbs, food=nfood, crumb_share=round(crumbs / n, 4) if n else 0.0, open=len(opn),
                reach=len(reach), gallery=len(gallery), gallery_share=round(len(gallery) / len(reach), 4) if reach else 0.0,
                galleryc=len(galleryc), big=len(big),
                crater=len(crater), rooms_open=po['score'], thin_old=round((n - len(room_all)) / n, 4) if n else 0.0)


KEYS = ('space', 'crumbs', 'food', 'crumb_share', 'open', 'reach', 'gallery', 'gallery_share', 'galleryc', 'big', 'crater',
        'rooms_open', 'thin_old')  # the column order colsel's nestshape.csv / cgshape.csv write


def crumb_share(d):
    """colsel's `+crumbs` case reads this (lower is better); 0 for a run with no map."""
    m = measure(d)
    return m['crumb_share'] if m else 0.0


if __name__ == '__main__':
    a = sys.argv[1:]
    if a and a[0] == '--snap':
        d = a[1]
        for fn in sorted(f for f in os.listdir(d) if f.startswith('nestmap_f')):
            print(fn, measure(d, f'{d}/{fn}'))
    else:
        for d in a:
            print(d.rstrip('/'), measure(d))
