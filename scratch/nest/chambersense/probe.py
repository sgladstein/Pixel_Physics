"""SCRATCH (chambersense lane, 2026-10-30): the room sense's control. Reads a PP_COPY_LOG written with PP_DIGROOM=1 and
the run's final nestmap.txt; for every Dig decision in the last LAST frames it prints the sense the ant read (dig
feature 8, feature index 108 in the log) beside the same quantity recomputed in Python on the end map, and draws the
decisions on the map in a fixed dark->bright ramp (full replace: black = inside a room, yellow = 24+ cells from one).

    python3 scratch/nest/chambersense/probe.py RUNDIR COPYLOG OUT.png [LAST=2000]
"""
import sys, os
from collections import deque
from PIL import Image

d, log, out = sys.argv[1], sys.argv[2], sys.argv[3]
LAST = int(sys.argv[4]) if len(sys.argv) > 4 else 2000
t = open(f'{d}/nestmap.txt').read().split('\n')
x0, y0, legend = int(t[0].split()[0]), int(t[0].split()[1]), t[0].split()[2].split(',')
grid = [r for r in t[1:] if r]
surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
H, W = len(grid), len(grid[0])
e = chr(65 + legend.index('empty'))
# open as the Rust sense: empty, or a body (`?` is a body, food or an animal: counted open, a known over-count)
def opn(x, y):
    dx, dy = x - x0, y - y0
    return 0 <= dy < H and 0 <= dx < W and grid[dy][dx] in (e, '?')
def wide(x, y):
    return all(opn(x + i, y + j) for i in range(-2, 3) for j in range(-2, 3))
def rd(x, y):
    q, seen = deque([((x, y), 0)]), {(x, y)}
    while q:
        (a, b), k = q.popleft()
        if wide(a, b):
            return k
        if k >= 24:
            continue
        for n in ((a + 1, b), (a - 1, b), (a, b + 1), (a, b - 1)):
            if n not in seen and opn(*n):
                seen.add(n); q.append((n, k + 1))
    return 24
rows = []
fmax = 0
for l in open(log):
    p = l.split(',', 13)
    if len(p) < 14 or p[1] != 'Q':
        continue
    fr, x, y = int(p[2]), int(p[4]), int(p[5])
    fmax = max(fmax, fr)
    v = None
    for o in p[13].split(';'):
        kv = o.split(':', 4)
        if len(kv) == 5 and kv[0] == '3':  # Verb::Dig
            fe = dict(z.split('=') for z in kv[4].split() if '=' in z)
            v = float(fe.get('108', 0.0)) * 8
            break
    if v is not None:
        rows.append((fr, x, y, v))
late = [r for r in rows if r[0] >= fmax - LAST]
agree = [(v, rd(x, y)) for _, x, y, v in late]
n = len(agree)
same = sum(1 for a, b in agree if abs(a - b) < 0.01)
near = sum(1 for a, b in agree if abs(a - b) <= 3)
print(f'dig decisions {len(rows)}, last {LAST} frames {n}: sense == end-map recompute {same}/{n}, within 3 cells {near}/{n}')
hist = {}
for _, _, _, v in rows:
    b = 'room(0)' if v == 0 else '1-4' if v <= 4 else '5-11' if v <= 11 else '12-23' if v < 24 else '24+'
    hist[b] = hist.get(b, 0) + 1
print('all dig decisions by sense:', hist)
# picture: the end map (grey ground, white open), the late decisions coloured by the ant's reading
Z = 3
im = Image.new('RGB', (W * Z, H * Z))
px = im.load()
for dy in range(H):
    for dx in range(W):
        c = (250, 245, 230) if grid[dy][dx] in (e, '?') and y0 + dy > surf[x0 + dx] else (214, 230, 242) if y0 + dy <= surf[x0 + dx] else (120, 100, 80)
        for i in range(Z):
            for j in range(Z):
                px[dx * Z + i, dy * Z + j] = c
ramp = lambda v: (int(255 * min(v, 24) / 24), int(220 * min(v, 24) / 24), int(60 + 0 * v))
for _, x, y, v in late:
    dx, dy = x - x0, y - y0
    if 0 <= dx < W and 0 <= dy < H:
        for i in range(Z):
            for j in range(Z):
                px[dx * Z + i, dy * Z + j] = ramp(v)
im.save(out)
print('wrote', out)
