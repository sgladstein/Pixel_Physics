"""Hand-drawn shapes for roomshape.shape (dig-scoring thread, 2026-10-22). Door at x=0, y=0; y grows down."""
import random, sys
sys.path.insert(0, __file__.rsplit('/', 1)[0] if '/' in __file__ else '.')
import roomshape as rs


def rect(x0, y0, w, h):
    return {(x, y) for x in range(x0, x0 + w) for y in range(y0, y0 + h)}


def base(room_w=13, room_h=7, shaft=6):
    s = rect(-1, 0, 3, shaft) | rect(-room_w // 2, shaft, room_w, room_h)
    return s


def ragged(s, seed, n=40):
    r = random.Random(seed)
    s = set(s)
    edge = sorted({(x + a, y + b) for x, y in s for a, b in rs.N4} - s)
    for c in r.sample(edge, min(n, len(edge))):
        if c[1] > 0:
            s.add(c)
    return s


def tunnel(s, k, at=None, room_w=13, shaft=6):
    """A 1-wide tunnel k long out of the room's right wall, mid-height."""
    x0 = max(x for x, y in s if y == shaft + 3) + 1 if at is None else at
    return s | {(x0 + i, shaft + 3) for i in range(k)}


def score(s, crater=frozenset()):
    p = rs.shape(s | crater, (0, 0), crater=crater)
    return p['score'], p['tunnels'], p['rooms']


rows = [('one room 13x7', base()), ('one room 25x7 (bigger)', base(25)), ('one room 13x11 (taller)', base(13, 11)),
        ('one room 41x9 (huge)', base(41, 9))]
for sd in range(5):
    rows.append((f'13x7, ragged walls seed {sd}', ragged(base(), sd)))
    rows.append((f'25x7, ragged walls seed {sd}', ragged(base(25), sd)))
for k in (1, 2, 3, 4, 5, 6, 8, 10, 14, 20, 30):
    rows.append((f'13x7 + side tunnel {k}', tunnel(base(), k)))
for k in (2, 4, 6):
    rows.append((f'13x7 + 2-wide side tunnel {k}', tunnel(base(), k) | {(6 + i, 10) for i in range(k)}))
t8 = tunnel(base(), 8)
rows.append(('13x7 + tunnel 8 + room 3x3 at its end', t8 | rect(14, 8, 3, 3)))
rows.append(('13x7 + tunnel 8 + room 5x5 at its end', t8 | rect(14, 7, 5, 5)))
rows.append(('13x7 + tunnel 8 + room 7x7 at its end', t8 | rect(14, 6, 7, 7)))
rows.append(('13x7 + tunnel 8 + room 13x7 at its end', t8 | rect(14, 6, 13, 7)))
rows.append(('two tunnels 8 (both sides)', t8 | {(-8 - i, 9) for i in range(8)}))
rows.append(('rooms stacked: 13x7, 1-wide shaft 4, 13x7', base() | rect(0, 13, 1, 4) | rect(-6, 17, 13, 7)))
rows.append(('rooms stacked: 13x7, 3-wide shaft 4, 13x7', base() | rect(-1, 13, 3, 4) | rect(-6, 17, 13, 7)))
rows.append(('funnel (cone 21 wide narrowing to 3, 20 deep)', {(x, y) for y in range(20) for x in range(-10 + y // 2, 11 - y // 2)}))
rows.append(('comb: 13x7 + 6 stubs of 2 on the floor', base() | {(x, 13 + i) for x in (-6, -4, -2, 0, 2, 4) for i in range(2)}))
rows.append(('comb: 13x7 + 6 stubs of 4 on the floor', base() | {(x, 13 + i) for x in (-6, -4, -2, 0, 2, 4) for i in range(4)}))
# crater cases: the door's shaft (x -1..1) is nest, the rest of a sky-open pit is crater
pit, P = rect(-1, 0, 3, 16), rect(-6, 0, 13, 16)
rows.append(('open pit 13 wide 16 deep (crater)', (pit, P - pit)))
bowl = rect(-15, 0, 31, 6) - {(x, y) for x in (-6, 6) for y in range(1, 6)}
rows.append(('open bowl 31x6 split by two 1-wide pillars', (rect(-1, 0, 3, 6), bowl - rect(-1, 0, 3, 6))))
rows.append(('open pit 13x16 + roofed 2-wide undercut 2 deep', (pit | rect(7, 12, 2, 2), P - pit)))
rows.append(('open pit 13x16 + roofed 2-wide undercut 6 deep', (pit | rect(7, 12, 6, 2), P - pit)))
rows.append(('open pit 13x16 + tunnel 8 + roofed 7x7 room', (pit | rect(7, 12, 8, 1) | rect(15, 9, 7, 7), P - pit)))
for name, s in rows:
    sc, b, r = score(*s) if isinstance(s, tuple) else score(s)
    print(f'{name:48s} {sc:6.1f}  tunnels {b}  rooms {r}')
