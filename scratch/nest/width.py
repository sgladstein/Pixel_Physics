"""SCRATCH (nest lane, 2026-10-28): how far the nest reaches sideways. Floods the open space joined to the door below
the datum (anything but ground passes, as nestcount's flood: soil, packed soil, spoil, stone and plant cells stop it)
and reports its width (columns spanned), its reach left/right of the door, and how many of its cells lie more than
20 columns out (beyond inrun's nest_r band). Open-only variants count only empty cells in the flood.
usage: width.py RUNDIR [MAP]   (MAP defaults to RUNDIR/nestmap.txt) -> one line of key=value"""
import sys

NEST = 128


def width(d, mp=None):
    t = open(mp or f'{d}/nestmap.txt').read().split('\n')
    head = t[0].split(' ')
    x0, y0, legend = int(head[0]), int(head[1]), head[2].split(',')
    grid = [r for r in t[1:] if r]
    surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    gnd = {chr(65 + k) for k, m in enumerate(legend) if m in ('soil', 'packedsoil', 'spoil', 'stone')} | {'%'}
    e = chr(65 + legend.index('empty')) if 'empty' in legend else None
    H, W = len(grid), len(grid[0])
    below = lambda dx, dy: 0 <= x0 + dx < len(surf) and y0 + dy >= surf[x0 + dx]
    opn = lambda dx, dy: 0 <= dy < H and 0 <= dx < W and grid[dy][dx] not in gnd and below(dx, dy)
    door = NEST - x0
    seen, st = set(), [(door, dy) for dy in range(H) if opn(door, dy)]
    while st:
        p = st.pop()
        if p in seen:
            continue
        seen.add(p)
        x, y = p
        st += [q for q in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)) if q not in seen and opn(*q)]
    empty = {p for p in seen if grid[p[1]][p[0]] == e}
    xs = [x - door for x, _ in empty] or [0]
    return dict(cells=len(seen), open=len(empty), width=max(xs) - min(xs) + 1 if empty else 0, left=-min(xs),
                right=max(xs), out20=sum(1 for x, _ in empty if abs(x - door) > 20),
                depth=max((y0 + y - surf[x0 + x] for x, y in empty), default=0))


if __name__ == '__main__':
    r = width(sys.argv[1], sys.argv[2] if len(sys.argv) > 2 else None)
    print(' '.join(f'{k}={v}' for k, v in r.items()))
