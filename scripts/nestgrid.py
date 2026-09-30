#!/usr/bin/env python3
"""The nest's shape from `digbox`'s `gridout=` dump: how thick the tamped
soil is, which open pockets are sealed off from the sky, and what each packed
cell was before it was packed -- and the same, paired seed by seed across two
arms. The numbers behind `Reports/nest-one-entrance-2026-09-29.md` §15-§17
("sealed-off cells", "tamped cells in blocks"), which `digbox` does not print.

    python3 scripts/nestgrid.py RUN.grid [--png OUT.png --frame F]
    python3 scripts/nestgrid.py --pair DIR FRAME ARM_A ARM_B
    python3 scripts/nestgrid.py --selftest

Below the old ground line only. Open space is `.` and `a` (an animal stands
in passable space). A packed cell's thickness is its 8-way distance to the
nearest open cell: 1 is a wall, 2 and more is the inside of a block. A pocket
(8-connected open cells below the old ground line) is sealed when none of it
touches open air above the old ground line; the 4-way count is printed
beside it because a diagonal gap is not a passage an ant can always use.

`--pair` reads `<arm>-a<ants>-s<seed>.log` with its `.grid` beside it (the
dig box run with `gridout=` and `stops=` including FRAME) and pairs the two
arms by (ants, seed): medians and how many seeds went each way, per colony
size, with the key's cardinality printed first. **Grids are the whole box**:
a 200x92 box is ~19 KB a stop, so keep `stops=` short.

Built 2026-09-29/30 by the nest lane (scratch until the handoff). What it
cannot see: why a pocket sealed -- `digbox`'s `PACK` and `FILL` lines and the
grid's `PACKED_FROM` rows say what refilled it, and a per-ant trace says who.
"""
import re, sys
from collections import deque, Counter

def parse(path):
    stops = []
    lines = open(path).read().split('\n')
    i = 0
    while i < len(lines):
        ln = lines[i]
        if ln.startswith('GRID '):
            kv = dict(p.split('=') for p in ln.split()[1:])
            f, w, h, surf, floor = (int(kv[k]) for k in ('frame', 'w', 'h', 'surface', 'floor'))
            grid = lines[i + 1:i + 1 + h]
            assert lines[i + 1 + h] == 'DUG', lines[i + 1 + h]
            dug = lines[i + 2 + h:i + 2 + 2 * h]
            assert lines[i + 2 + 2 * h] == 'PACKED_FROM'
            pfrom = lines[i + 3 + 2 * h:i + 3 + 3 * h]
            assert lines[i + 3 + 3 * h] == 'PACKED_FRAME'
            pframe = [list(map(int, r.split())) for r in lines[i + 4 + 3 * h:i + 4 + 4 * h]]
            stops.append(dict(frame=f, w=w, h=h, surface=surf, floor=floor, grid=grid, dug=dug, pfrom=pfrom, pframe=pframe))
            i += 4 + 4 * h
        else:
            i += 1
    return stops

N8 = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)]
N4 = [(0, -1), (-1, 0), (1, 0), (0, 1)]

def analyse(st, conn=N8):
    w, h, surf, floor, g = st['w'], st['h'], st['surface'], st['floor'], st['grid']
    is_open = lambda x, y: g[y][x] in '.a'
    # thickness: BFS from every open cell below the old ground line and every open cell above it
    INF = 10 ** 9
    dist = [[INF] * w for _ in range(h)]
    q = deque()
    for y in range(h):
        for x in range(w):
            if is_open(x, y):
                dist[y][x] = 0
                q.append((x, y))
    while q:
        x, y = q.popleft()
        for dx, dy in N8:
            nx, ny = x + dx, y + dy
            if 0 <= nx < w and 0 <= ny < h and dist[ny][nx] > dist[y][x] + 1:
                dist[ny][nx] = dist[y][x] + 1
                q.append((nx, ny))
    packed = [(x, y) for y in range(surf, floor) for x in range(w) if g[y][x] == 'P']
    thick = Counter(min(dist[y][x], 4) for x, y in packed)
    block = [(x, y) for x, y in packed if dist[y][x] >= 2]
    # pockets
    comp = [[-1] * w for _ in range(h)]
    pockets = []
    for y in range(surf, floor):
        for x in range(w):
            if is_open(x, y) and comp[y][x] < 0:
                cid = len(pockets)
                cells = []
                touches_air = False
                comp[y][x] = cid
                dq = deque([(x, y)])
                while dq:
                    cx, cy = dq.popleft()
                    cells.append((cx, cy))
                    for dx, dy in conn:
                        nx, ny = cx + dx, cy + dy
                        if not (0 <= nx < w and 0 <= ny < h):
                            continue
                        if ny < surf:
                            if is_open(nx, ny):
                                touches_air = True
                            continue
                        if ny >= floor:
                            continue
                        if is_open(nx, ny) and comp[ny][nx] < 0:
                            comp[ny][nx] = cid
                            dq.append((nx, ny))
                pockets.append((cells, touches_air))
    sealed = [c for c, air in pockets if not air]
    open_cells = sum(len(c) for c, _ in pockets)
    sealed_cells = sum(len(c) for c in sealed)
    ants_sealed = sum(1 for c in sealed for x, y in c if g[y][x] == 'a')
    dug = [(x, y) for y in range(surf, floor) for x in range(w) if st['dug'][y][x] == '1']
    dug_now = Counter(g[y][x] for x, y in dug)
    origin_all = Counter(st['pfrom'][y][x] for x, y in packed)
    origin_block = Counter(st['pfrom'][y][x] for x, y in block)
    return dict(packed=len(packed), thick=thick, block=block, origin_all=origin_all, origin_block=origin_block,
                pockets=len(pockets), sealed=len(sealed), sealed_cells=sealed_cells, open_cells=open_cells,
                largest_sealed=max((len(c) for c in sealed), default=0), ants_sealed=ants_sealed,
                dug=len(dug), dug_now=dug_now, dist=dist, sealed_list=sealed)

def log_stats(path, FR):
    s = open(path).read()
    m = re.search(rf'^LEDGER frame={FR} cuts (\d+) .*?above the old surface (\d+), a pellet or refill cut again (\d+), new ground open to the sky (\d+), new ground under a roof (\d+)', s, re.M)
    f = re.search(rf'^FILL frame={FR} cells dug below the old ground line since frame 0: (\d+), holding now: open (\d+), a live ant (\d+)', s, re.M)
    alive = re.search(rf'^ +{FR} +(\d+) ', s, re.M)
    put = re.search(rf'^LEDGER frame={FR} pellets put down (\d+) .*?landed above the old surface (\d+), below it (\d+) \(into a dug cell (\d+)\)', s, re.M)
    if not (m and f and alive and put):
        return None
    dug, op, ant = int(f.group(1)), int(f.group(2)), int(f.group(3))
    return dict(cuts=int(m.group(1)), recut=int(m.group(3)), new=int(m.group(4)) + int(m.group(5)), dug=dug, refilled_pct=100.0 * (dug - op - ant) / dug if dug else 0.0, alive=int(alive.group(1)),
                put=int(put.group(1)), put_inside=int(put.group(3)), put_into_dug=int(put.group(4)))


def pair(d, fr, arm_a, arm_b):
    import glob, os, re, statistics
    rows = {}
    for p in glob.glob(f'{d}/*.log'):
        m = re.match(r'([a-z]+)-a(\d+)-s(\d+)$', os.path.basename(p)[:-4])
        if not m or m.group(1) not in (arm_a, arm_b):
            continue
        ls = log_stats(p, fr)
        g = p[:-4] + '.grid'
        st = next((s for s in parse(g) if s['frame'] == fr), None) if ls and os.path.exists(g) else None
        if st is None:
            print(f'skipped {p}: no FILL/LEDGER at frame {fr} or no grid stop there')
            continue
        a = analyse(st)
        blk = sum(v for k, v in a['thick'].items() if k >= 2)
        rows[(int(m.group(2)), int(m.group(3)), m.group(1))] = dict(ls, sealed_cells=a['sealed_cells'], sealed_pockets=a['sealed'], shut_in=a['ants_sealed'], block=blk, open=a['open_cells'])
    print(f'{len(rows)} runs read: ' + ', '.join(f'{arm} {sum(1 for k in rows if k[2] == arm)}' for arm in (arm_a, arm_b)))
    keys = ['sealed_cells', 'sealed_pockets', 'shut_in', 'block', 'open', 'recut', 'new', 'dug', 'refilled_pct', 'put', 'put_inside', 'put_into_dug', 'alive']
    for n in sorted({k[0] for k in rows}):
        seeds = sorted({k[1] for k in rows if k[0] == n and (n, k[1], arm_a) in rows and (n, k[1], arm_b) in rows})
        print(f'== {n} ants, frame {fr}, {arm_a} -> {arm_b}: {len(seeds)} paired seeds {seeds}')
        for key in keys:
            x = [rows[(n, s, arm_a)][key] for s in seeds]
            y = [rows[(n, s, arm_b)][key] for s in seeds]
            lo = sum(1 for p, q in zip(x, y) if q < p)
            hi = sum(1 for p, q in zip(x, y) if q > p)
            fmt = (lambda v: f'{v:.1f}') if key == 'refilled_pct' else (lambda v: f'{v:g}')
            print(f'  {key:14} median {fmt(statistics.median(x)):>7} -> {fmt(statistics.median(y)):>7}   {arm_b} lower on {lo}, higher on {hi}')


def _stop(rows, surface):
    """A hand-built stop: `rows` of the grid, nothing dug or traced."""
    h, w = len(rows), len(rows[0])
    return dict(frame=0, w=w, h=h, surface=surface, floor=h, grid=rows, dug=['0' * w] * h,
                pfrom=['-' * w] * h, pframe=[[-1] * w for _ in range(h)])

def selftest():
    """Known answers, each with the case that must read differently."""
    fails = []
    def check(name, got, want):
        if got != want:
            fails.append(f'{name}: got {got}, want {want}')
    # A pocket under a packed roof, sealed; the same with its roof holed, open.
    sealed = _stop(['.......', 'sPPPPPs', 'sP...Ps', 'sPPPPPs', 'sssssss'], 1)
    holed = _stop(['.......', 'sPP.PPs', 'sP...Ps', 'sPPPPPs', 'sssssss'], 1)
    a, b = analyse(sealed), analyse(holed)
    check('sealed pocket', (a['sealed'], a['sealed_cells']), (1, 3))
    check('holed pocket', (b['sealed'], b['sealed_cells']), (0, 0))
    # A 3x3 packed block has one cell two deep; a one-cell wall has none.
    block = _stop(['.......', '.PPP...', '.PPP...', '.PPP...', '.......'], 1)
    wall = _stop(['.......', '.P.P...', '.P.P...', '.P.P...', '.......'], 1)
    check('block interior', len(analyse(block)['block']), 1)
    check('wall interior', len(analyse(wall)['block']), 0)
    # An animal in a sealed pocket is counted inside it.
    ant = _stop(['.......', 'sPPPPPs', 'sP.a.Ps', 'sPPPPPs', 'sssssss'], 1)
    check('ant shut in', analyse(ant)['ants_sealed'], 1)
    for f in fails:
        print('FAIL', f)
    print('selftest:', 'FAILED' if fails else 'ok (5 checks)')
    return 1 if fails else 0

def main():
    if '--selftest' in sys.argv:
        sys.exit(selftest())
    if '--pair' in sys.argv:
        i = sys.argv.index('--pair')
        d, fr, a, b = sys.argv[i + 1:i + 5]
        pair(d, int(fr), a, b)
        return
    path = sys.argv[1]
    stops = parse(path)
    for st in stops:
        a = analyse(st)
        a4 = analyse(st, N4)
        t = a['thick']
        print(f"frame {st['frame']:>6}: packed below ground {a['packed']:4} (wall {t.get(1,0)}, 2 deep {t.get(2,0)}, 3 {t.get(3,0)}, 4+ {t.get(4,0)}) | "
              f"open cells {a['open_cells']:4} in {a['pockets']:3} pockets, sealed {a['sealed']:3} pockets / {a['sealed_cells']:4} cells (largest {a['largest_sealed']}, ants inside {a['ants_sealed']}); 4-way sealed {a4['sealed']}/{a4['sealed_cells']} | "
              f"dug {a['dug']}: now {dict(a['dug_now'].most_common())} | block cells by origin {dict(sorted(a['origin_block'].items()))}, all packed by origin {dict(sorted(a['origin_all'].items()))}")
    if '--png' in sys.argv:
        from PIL import Image
        out = sys.argv[sys.argv.index('--png') + 1]
        fr = int(sys.argv[sys.argv.index('--frame') + 1])
        st = next(s for s in stops if s['frame'] == fr)
        a = analyse(st)
        w, h = st['w'], st['h']
        col = {'.': (18, 14, 12), 'a': (255, 0, 200), 's': (62, 46, 34), 'P': (0, 150, 190), 'o': (255, 140, 0), '#': (90, 90, 90), 'c': (40, 220, 40), 'n': (255, 255, 255), '?': (120, 120, 120)}
        im = Image.new('RGB', (w, h))
        px = im.load()
        for y in range(h):
            for x in range(w):
                c = st['grid'][y][x]
                px[x, y] = col.get(c, (120, 120, 120)) if y >= st['surface'] or c != '.' else (35, 40, 48)
        for x, y in a['block']:
            px[x, y] = (0, 235, 255) if a['dist'][y][x] == 2 else (170, 255, 255)
        for cells in a['sealed_list']:
            for x, y in cells:
                px[x, y] = (255, 40, 40) if st['grid'][y][x] == '.' else (255, 120, 120)
        im.save(out)
        print('wrote', out)

if __name__ == '__main__':
    main()
