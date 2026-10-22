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
    opn = lambda dx, dy: 0 <= dy < H and 0 <= dx < W and grid[dy][dx] in (e, '?')
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
    roofed = lambda dx, dy: any(grid[yy][dx] not in (e, '?') for yy in range(dy))
    nest = {(dx, dy) for dx, dy in seen if abs(dx - door) <= 1 or roofed(dx, dy)}
    crater = sum(1 for dy in range(H) for dx in range(door - REACH, door + REACH + 1)
                 if grid[dy][dx] == e and below(dx, dy) and (dx, dy) not in nest)
    spoil = sum(1 for dy in range(H) for dx in range(door - REACH, door + REACH + 1) if grid[dy][dx] == sp and below(dx, dy))
    out = {'nest': len(nest), 'crater': crater, 'spoil': spoil, 'nestcuts': None, 'nestcuts_late': None, 'spoil_in': None}
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
