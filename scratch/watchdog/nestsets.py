"""SCRATCH (watchdog, 2026-10-28): nestcount.count's flood and nest/crater split, returning the cell sets.
Generated from scratch/inrun/nestcount.py (same rules); regenerate if that file changes."""
import os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "inrun"))
from nestcount import NEST, REACH  # noqa: E402


def sets(d, fn="nestmap.txt"):
    t = open(f'{d}/{fn}').read().split('\n')
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
    # crater_full (nest lane, 2026-10-28): the crater with what fills it -- `crater` plus every cell the door's flood
    # reached that is not nest and not open air (crumbs, stored food, ants), within REACH. The trained ant cuts the
    # door's rim (21 original-ground cuts at the datum beside the door by 250k against the hand-built ant's 2, world
    # 500004) and its mouth widens from 5 to 9-12 cells, but its crumb pile fills the hole, so `crater` reads it as
    # nest-coloured food: on the 16 exam worlds at 200k std_pool's crater is ~42 open cells against ~190 with the
    # filling counted, and the hand-built ant's is 0 both ways (16/16). The hole only shows as open air once a dying
    # colony eats the pile (crater 312 at 600k). colsel `craterfull=1` scores this instead of `crater`.
    crater_full = crater + sum(1 for dx, dy in seen - nest if abs(dx - door) <= REACH and grid[dy][dx] != e)
    spoil = sum(1 for dy in range(H) for dx in range(door - REACH, door + REACH + 1) if grid[dy][dx] == sp and below(dx, dy))
    # nest_open (teacher lane, 2026-10-25): the nest's cells that are still open air, not stored food or crumbs. The
    # owner's "too small" read as a nest of ~100 open cells under ~500 cells of food in every trained colony (pictures
    # https://claude.ai/artifact/7CykXk4N8PiKzo1BEomvVz), while `nest` counts the food as room.
    nest_open = sum(1 for dx, dy in nest if grid[dy][dx] == e)
    return dict(x0=x0, y0=y0, grid=grid, legend=legend, nest=nest, seen=seen, e=e, door=door)
