"""SCRATCH (dig-scoring thread, 2026-10-22): how far a kept nest is "on the way to multiple rooms", from its end map.

Owner (18:25Z): get multiple rooms, and credit the first few cuts that turn into a tunnel or a second chamber rather than
more widening of the main room. Used for colony selection (a lexicase case), so it only has to be graded and hard to game.

The space is every open cell joined to the door below the datum: nestcount's nest (roofed, or in the door's shaft) plus
its crater (the rest, open to the sky). Loose spoil lying in it counts as open for its shape (see `fill`). The crater is
infrastructure, never a room: without it in the space, a deep open pit's overhung walls read as separate rooms (cs3lex
c2 62, cs3lexlesinh c39 76), and with it but not excluded, a pocked crater bowl reads as three rooms (cs3lex c61 73).
Then:

  rooms   Where a 5x5 block of open space fits: the union of every such block (a morphological opening). Each connected
          piece of it that is mostly roofed is a ROOM (a piece mostly open to the sky is crater). The biggest is the
          main room. Making it bigger, longer or taller changes nothing.
  body    The main room, the door's way down to it (the shortest path from the door to the main room) and the crater:
          the infrastructure every nest has.
  score   Each other room scores min(cells, 50) / 2: a first 5x5 pocket 12.5, a 7x7 chamber 24.5, capped at 25.
          Each piece of nest outside the rooms (tunnels, the start of a side passage) scores by how far it reaches from
          the body along the nest, minus 2: a ragged wall's bump (1-2 deep) scores 0, a tunnel k long scores k - 2,
          capped at 15. Only the 4 best of those count, so a comb of stubs is not multiple rooms.
          A tunnel that leads to a second room: the room scores, and so does the tunnel up to it.

One room of any size, ragged or not, scores ~0 (hand-drawn shapes: roomshape_test.py). Known soft spot: a chamber
pinched to a 4-tall neck between two lobes counts as two rooms (old ant, world 12099); that is a step toward two rooms,
so it is left in.
usage: roomshape.py RUNDIR [RUNDIR...]; import: score(rundir) -> float, or parts(rundir) -> dict.
"""
import sys
from collections import deque

NEST = 128
K = 2            # a room fits a (2K+1) x (2K+1) block: 5x5
RAGGED = 2       # reach a wall bump can have without being a passage
CAP_T, CAP_R, BEST_T = 15, 50, 4
N4 = ((1, 0), (-1, 0), (0, 1), (0, -1))
N8 = tuple((a, b) for a in (-1, 0, 1) for b in (-1, 0, 1) if a or b)


def nest_cells(d):
    """the open space (nest | crater), the door's entry cell, loose spoil below the datum, and the crater."""
    t = open(f'{d}/nestmap.txt').read().split('\n')
    head = t[0].split(' ')
    x0, y0, legend = int(head[0]), int(head[1]), head[2].split(',')
    grid = [r for r in t[1:] if r]
    surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    e = chr(65 + legend.index('empty')) if 'empty' in legend else None
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
    crater = set(seen) - nest
    entry = min((p for p in nest if abs(p[0] - door) <= 1), key=lambda p: p[1], default=None)
    sp = chr(65 + legend.index('spoil')) if 'spoil' in legend else None
    spoil = {(dx, dy) for dy in range(H) for dx in range(W) if grid[dy][dx] == sp and below(dx, dy)}
    return nest | crater, entry, spoil, crater


def fill(nest, spoil, rounds=3):
    """Loose spoil lying in the nest counts as nest for its shape: a spoil cell with 3+ of its 8 neighbours in the nest
    joins, a few rounds. A rubble-strewn floor otherwise splits a room into twisting passages (cs3lex c25). A
    backfilled worm trail (the pack move leaves 1-wide lines of spoil through the ground) touches the nest with one end
    only, so it stays out. Soil is never filled: the old ant's 1-wide side tunnels are real."""
    nest = set(nest)
    for _ in range(rounds):
        add = {c for c in spoil - nest if sum((c[0] + a, c[1] + b) in nest for a, b in N8) >= 3}
        if not add:
            break
        nest |= add
    return nest


def pieces(cells):
    """4-connected components, biggest first."""
    left, out = set(cells), []
    while left:
        s = left.pop()
        comp, q = {s}, deque([s])
        while q:
            p = q.popleft()
            for a, b in N4:
                r = (p[0] + a, p[1] + b)
                if r in left:
                    left.discard(r)
                    comp.add(r)
                    q.append(r)
        out.append(comp)
    return sorted(out, key=lambda c: (-len(c), min(c)))


def shape(nest, entry, spoil=frozenset(), crater=frozenset()):
    """score and its parts for a set of nest cells, the door's entry cell and the loose spoil below the datum."""
    if not nest or entry is None:
        return dict(score=0.0, rooms=[], tunnels=[])
    nest = fill(nest, spoil)
    blk = [(a, b) for a in range(-K, K + 1) for b in range(-K, K + 1)]
    centres = {p for p in nest if all((p[0] + a, p[1] + b) in nest for a, b in blk)}
    opened = {(c[0] + a, c[1] + b) for c in centres for a, b in blk}
    # a room mostly open to the sky is crater, not a chamber: it joins the body below and never scores
    rooms = [r for r in pieces(opened) if 2 * len(r & crater) < len(r)]
    # body: the main room plus the door's shortest way down to it (the door alone if there is no room yet)
    dist, par = {entry: 0}, {}
    q = deque([entry])
    while q:
        p = q.popleft()
        for a, b in N4:
            r = (p[0] + a, p[1] + b)
            if r in nest and r not in dist:
                dist[r] = dist[p] + 1
                par[r] = p
                q.append(r)
    body = set(rooms[0]) if rooms else set()
    if body:
        p = min(body, key=lambda c: (dist.get(c, 1 << 30), c))
        while p in par:
            p = par[p]
            body.add(p)
    else:
        body = {entry}
    body |= crater
    # reach of every nest cell from the body, along the nest
    reach = {p: 0 for p in body}
    q = deque(body)
    while q:
        p = q.popleft()
        for a, b in N4:
            r = (p[0] + a, p[1] + b)
            if r in nest and r not in reach:
                reach[r] = reach[p] + 1
                q.append(r)
    rest = {p for p in nest if p not in body and p not in opened}
    tunnels = sorted((max(reach.get(p, 0) for p in c) for c in pieces(rest)), reverse=True)
    tun = [min(t - RAGGED, CAP_T) for t in tunnels if t > RAGGED][:BEST_T]
    extra = [len(r) for r in rooms[1:]]
    score = sum(tun) + sum(min(n, CAP_R) / 2 for n in extra)
    return dict(score=float(score), rooms=[len(r) for r in rooms], tunnels=tun)


def parts(d):
    return shape(*nest_cells(d))


def score(d):
    return parts(d)['score']


if __name__ == '__main__':
    for d in sys.argv[1:]:
        print(d.rstrip('/'), parts(d))
