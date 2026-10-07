"""deathmap.py OUT.png RUN:FRAME:LABEL[:IDS[:X0,Y0,X1,Y1]] ... (DM_SCALE=px per cell, default 4) -- nest maps (deeptrace nest_fNNNNNN.txt) side by side, with marked cells
(Deep trace, 2026-10-07). IDS is a file of 'x,y,kind' lines to mark (kind: s starved -> white cross; v store food a mouth can see -> cyan dot; i store crumbs too small to see -> pink dot); marks are drawn as a white
cross (s) or a cyan dot (f) over the map. Pure-python PNG (no PIL here). Palette as Nest race's nest-over-time pictures.
"""
import sys, zlib, struct

PAL = {
    '.': (27, 30, 46),      # open, never dug (sky / air)
    'o': (18, 16, 14),      # dug open
    '#': (92, 72, 52),      # ground
    '=': (120, 100, 70),    # packed wall
    's': (150, 120, 85),    # spoil
    'r': (70, 70, 70),      # hard ground
    '~': (60, 110, 200),    # water
    'a': (230, 50, 30),     # adult
    'e': (250, 240, 220),   # egg
    'l': (245, 225, 165),   # larva
    'p': (220, 190, 120),   # pupa
    'f': (60, 200, 70),     # food
    'c': (240, 190, 20),    # stored food (crumbs)
    'x': (150, 30, 150),    # corpse
}
S = int(__import__('os').environ.get('DM_SCALE', '4'))  # pixels per cell (DM_SCALE)


def load(path):
    L = open(path).read().rstrip('\n').split('\n')
    x0, y0, w, h = map(int, L[0].split())
    return x0, y0, w, h, L[1:1 + h]


def png(path, W, H, px):
    raw = b''.join(b'\x00' + bytes(v for p in row for v in p) for row in px)
    def chunk(t, d):
        return struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    open(path, 'wb').write(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', W, H, 8, 2, 0, 0, 0))
                           + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b''))


panels = []
for spec in sys.argv[2:]:
    parts = spec.split(':')
    run, frame, label = parts[0], int(parts[1]), parts[2]
    marks = []
    if len(parts) > 3 and parts[3]:
        for ln in open(parts[3]):
            x, y, k = ln.strip().split(',')
            marks.append((int(x), int(y), k))
    x0, y0, w, h, rows = load(f'{run}/nest_f{frame:06d}.txt')
    if len(parts) > 4 and parts[4]:
        cx0, cy0, cx1, cy1 = map(int, parts[4].split(','))
        rows = [r[cx0 - x0:cx1 - x0] for r in rows[cy0 - y0:cy1 - y0]]
        x0, y0, w, h = cx0, cy0, cx1 - cx0, cy1 - cy0
    panels.append(((x0, y0, w, h, rows), marks, label))

GAP = 6
W = sum(p[0][2] * S for p in panels) + GAP * (len(panels) - 1)
H = max(p[0][3] * S for p in panels)
img = [[(250, 250, 250)] * W for _ in range(H)]
ox = 0
for (x0, y0, w, h, rows), marks, label in panels:
    for j, row in enumerate(rows):
        for i, ch in enumerate(row[:w]):
            c = PAL.get(ch, (255, 0, 255))
            for dy in range(S):
                r = img[j * S + dy]
                for dx in range(S):
                    r[ox + i * S + dx] = c
    for x, y, k in marks:
        i, j = x - x0, y - y0
        if not (0 <= i < w and 0 <= j < h):
            continue
        cx, cy = ox + i * S + S // 2, j * S + S // 2
        R = max(1, S // 2)
        if k == 's':
            for d in range(-S + 1, S):
                for (px, py) in ((cx + d, cy + d), (cx + d, cy - d)):
                    if 0 <= px < W and 0 <= py < H:
                        img[py][px] = (255, 255, 255)
        else:
            col = {'v': (0, 230, 255), 'i': (255, 60, 200), 'f': (0, 230, 255)}.get(k, (0, 230, 255))
            for dy in range(-R + 1, R):
                for dx in range(-R + 1, R):
                    if 0 <= cx + dx < W and 0 <= cy + dy < H:
                        img[cy + dy][cx + dx] = col
    ox += w * S + GAP
png(sys.argv[1], W, H, img)
print('wrote', sys.argv[1], W, 'x', H, '; panels:', ', '.join(p[2] for p in panels))
