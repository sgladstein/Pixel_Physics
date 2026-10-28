"""SCRATCH (nest lane, 2026-10-27): the width of the nest's mouth over time -- the run of columns at the datum, through
the door column, whose datum cell is not ground (nestcount's `gap`; nestcount calls a gap wider than MOUTH=5 a crater
mouth, and every unroofed open cell under it crater). Also what fills the space under the mouth: open cells, crumbs and
other food in those columns below the datum.
usage: mouth.py RUNDIR [RUNDIR...]  -> one line per run: frame:width/open-under/food-under ..."""
import glob, os, sys

NEST = 128
for d in sys.argv[1:]:
    surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    out = []
    for p in sorted(glob.glob(f'{d}/nestmap_f*.txt')):
        f = int(os.path.basename(p)[9:16])
        t = open(p).read().split('\n')
        head = t[0].split(' ')
        x0, y0, legend = int(head[0]), int(head[1]), head[2].split(',')
        grid = [r for r in t[1:] if r]
        ground = {chr(65 + k) for k, m in enumerate(legend) if m in ('soil', 'packedsoil', 'spoil', 'stone')} | {'%'}
        food = {chr(65 + k) for k, m in enumerate(legend) if m in ('crumbs', 'fruit', 'food')} | {'?'}
        empty = chr(65 + legend.index('empty')) if 'empty' in legend else None
        top = lambda dx: surf[x0 + dx] - y0
        gap = lambda dx: grid[top(dx)][dx] not in ground
        door = NEST - x0
        if not gap(door):
            out.append(f'{f // 1000}:shut')
            continue
        a = b = door
        while gap(a - 1):
            a -= 1
        while gap(b + 1):
            b += 1
        cols = range(a, b + 1)
        under = [grid[y][dx] for dx in cols for y in range(top(dx) + 1, len(grid))]
        # only the space joined straight under the mouth: stop each column at its first ground cell
        o = fo = 0
        for dx in cols:
            for y in range(top(dx) + 1, len(grid)):
                c = grid[y][dx]
                if c in ground:
                    break
                o += c == empty
                fo += c in food
        out.append(f'{f // 1000}:{b - a + 1}/{o}/{fo}')
    print(d.rstrip('/').split('/')[-2:], ' '.join(out))
