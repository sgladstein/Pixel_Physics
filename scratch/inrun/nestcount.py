"""SCRATCH (teacher lane): what a run's digging left behind, from its end map (nestmap.txt), surface.txt and events.csv.

Owner (2026-10-22 16:54Z): count the cuts that actually expand the nest, and the spoil that falls into or is left in it.

  nest      open cells joined to the door below the datum that are roofed (ground above them in the column) or in the
            door's shaft (NEST +-1): the same rule as inrun's `inherit_nest=1`; the sky-open crater is not nest
  crater    open cells below the datum within 20 columns of the door that are not nest (sky-open)
  nestcuts  cuts of original ground (`dig` events) whose cell is nest at the end: digging that lasted and made room
  spoil     loose spoil cells below the datum within 20 columns of the door at the end (in the nest, the crater, or
            refilling old tunnels)
  nestcuts_late  the same, cut in the second half of the run (FRAMES/2, or half the last event's frame)
  spoil_in  pellets let go below the datum within 20 columns during the run (`in` events)
usage: nestcount.py RUNDIR [RUNDIR...] -> one line each; import count(rundir) -> dict
"""
import csv, gzip, os, sys

NEST, REACH = 128, 20


def count(d, frames=None):
    t = open(f'{d}/nestmap.txt').read().split('\n')
    head = t[0].split(' ')
    x0, y0, legend = int(head[0]), int(head[1]), head[2].split(',')
    grid = [r for r in t[1:] if r]
    surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    e = chr(65 + legend.index('empty')) if 'empty' in legend else None
    sp = chr(65 + legend.index('spoil')) if 'spoil' in legend else None
    H, W = len(grid), len(grid[0])
    # The flood passes anything that is not ground: stored food, crumbs, door markers and bodies (fixed 2026-10-23 by
    # the crater review: a larder of stored food cut the chamber behind it off, so a roofed room scored as crater --
    # 16-22 of 64 colonies a round off by >20 cells, the old ant's nest 302 where it is ~390). Same as cut_join's flood.
    gnd = {chr(65 + k) for k, m in enumerate(legend) if m in ('soil', 'packedsoil', 'spoil', 'stone')}
    gnd.add('%')  # a plant's own cell (inrun `plants=`): roots are ground, not nest
    opn = lambda dx, dy: 0 <= dy < H and 0 <= dx < W and grid[dy][dx] not in gnd
    below = lambda dx, dy: 0 <= x0 + dx < len(surf) and y0 + dy >= surf[x0 + dx]
    door = NEST - x0
    seen, stack = set(), [(door, dy) for dy in range(H) if opn(door, dy) and below(door, dy)]
    while stack:
        p = stack.pop()
        if p in seen:
            continue
        seen.add(p)
        dx, dy = p
        for q in ((dx - 1, dy), (dx + 1, dy), (dx, dy - 1), (dx, dy + 1)):
            if opn(*q) and below(*q) and q not in seen:
                stack.append(q)
    # Nest vs crater (fixed 2026-10-22 20:50Z). A cell is crater when nothing roofs it -- no ground (soil, packed soil,
    # spoil, stone) at or below the datum above it in its column -- AND its column opens to the sky through a gap at the
    # datum wider than MOUTH cells. The old rule counted any non-open cell above as roof, so the stamped door markers
    # ('nest', on the datum) and a crumb in flight roofed whole columns of an open pit, and the door's 3-wide column
    # counted all the way down a crater (cyan stripes through every selected crater in the colsel6 picture). A first fix
    # that kept only a walled shaft turned the old ant's own 3-wide mouth, and the chamber under it, into crater.
    MOUTH = 5  # the stamped mouth: door markers + its 2-3 open cells (old ant: NN..N)
    ground = {chr(65 + k) for k, m in enumerate(legend) if m in ('soil', 'packedsoil', 'spoil', 'stone')} | {'%'}
    top = lambda dx: max(0, surf[x0 + dx] - y0) if 0 <= x0 + dx < len(surf) else 0
    roofed = lambda dx, dy: any(grid[yy][dx] in ground for yy in range(top(dx), dy))
    gap = [0 <= top(dx) < H and grid[top(dx)][dx] not in ground for dx in range(W)]  # door markers, crumbs: open
    wide = [False] * W
    dx = 0
    while dx < W:
        if not gap[dx]:
            dx += 1
            continue
        k = dx
        while k < W and gap[k]:
            k += 1
        for q in range(dx, k):
            wide[q] = k - dx > MOUTH
        dx = k
    nest = {(dx, dy) for dx, dy in seen if roofed(dx, dy) or not wide[dx]}
    crater = sum(1 for dy in range(H) for dx in range(door - REACH, door + REACH + 1)
                 if grid[dy][dx] == e and below(dx, dy) and (dx, dy) not in nest)
    spoil = sum(1 for dy in range(H) for dx in range(door - REACH, door + REACH + 1) if grid[dy][dx] == sp and below(dx, dy))
    # nest_open (teacher lane, 2026-10-25): the nest's cells that are still open air, not stored food or crumbs. The
    # owner's "too small" read as a nest of ~100 open cells under ~500 cells of food in every trained colony (pictures
    # https://claude.ai/artifact/7CykXk4N8PiKzo1BEomvVz), while `nest` counts the food as room.
    nest_open = sum(1 for dx, dy in nest if grid[dy][dx] == e)
    out = {'nest': len(nest), 'nest_open': nest_open, 'crater': crater, 'spoil': spoil, 'nestcuts': None, 'nestcuts_late': None, 'spoil_in': None}
    ev = f'{d}/events.csv'
    if not os.path.exists(ev) and not os.path.exists(ev + '.gz'):
        return out
    f = open(ev) if os.path.exists(ev) else gzip.open(ev + '.gz', 'rt')
    rows = [l.split(',') for l in f][1:]
    half = max((int(p[0]) for p in rows), default=0) // 2 if frames is None else frames // 2
    out['nestcuts'] = out['nestcuts_late'] = out['spoil_in'] = 0
    for p in rows:
        if p[2] == 'dig' and (int(p[3]) - x0, int(p[4]) - y0) in nest:
            out['nestcuts'] += 1
            out['nestcuts_late'] += int(p[0]) > half
        elif p[2] == 'in' and abs(int(p[3]) - NEST) <= REACH:
            out['spoil_in'] += 1
    return out


if __name__ == '__main__':
    for d in sys.argv[1:]:
        print(d.rstrip('/').rsplit('/', 2)[-2:], count(d))
