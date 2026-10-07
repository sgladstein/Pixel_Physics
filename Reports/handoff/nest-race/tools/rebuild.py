#!/usr/bin/env python3
"""Rebuild the ground of a deeptrace dig=1 run from cells.csv.gz alone.

Every ground<->non-ground change in the logged box is in the file, so the
ground at any frame is: ground below the founding surface (y >= 160), plus
every logged change up to that frame. Ground->ground changes (lining, spoil
settling) are not logged, so a ground cell's letter is the last logged one.
  # soil, or ground the log has not touched yet by this frame (its material
    then is unknown: lining and spoil settling are not logged)
  = packed soil   s spoil   (as the last logged change left them)
  . open (air, tunnel, ant, brood...)
  P a cell a pack laid (pack_behind's tail, from packs.py) still ground
Usage: rebuild.py RUNDIR x0 x1 y0 y1 F1 [F2 ...] [packs=RUNDIR/packs.txt]
"""
import csv, gzip, sys
run = sys.argv[1]
x0, x1, y0, y1 = map(int, sys.argv[2:6])
frames = sorted(int(a) for a in sys.argv[6:] if "=" not in a)
G = {"soil": "#", "packedsoil": "=", "spoil": "s"}
packed = {}
try:
    for l in open(f"{run}/packs.txt").read().splitlines()[3:]:
        p = l.split(",")
        packed[(int(p[4]), int(p[5]))] = int(p[0])
except FileNotFoundError:
    pass
FIRST = {}
for _r in csv.DictReader(gzip.open(f"{run}/cells.csv.gz", "rt")):
    FIRST.setdefault((int(_r["x"]), int(_r["y"])), _r["from"])
state = {}
def at(x, y):
    if (x, y) in state:
        return state[(x, y)]
    # Not changed yet by this frame: ground or open as the cell's first
    # logged change says it was, but never its material -- a first logged
    # `from` can be packed soil only because lining (not logged) came first.
    fr = FIRST.get((x, y))
    if fr is None:
        return "#" if y >= 160 else "."
    return "#" if fr in G else "."
def show(f):
    print(f"--- frame {f}   x {x0}..{x1} (tens on top), y {y0}..{y1}")
    print("      " + "".join(str((x // 10) % 10) if x % 10 == 0 else " " for x in range(x0, x1 + 1)))
    for y in range(y0, y1 + 1):
        row = []
        for x in range(x0, x1 + 1):
            c = at(x, y)
            if c != "." and (x, y) in packed and packed[(x, y)] <= f:
                c = "P"
            row.append(c)
        print(f"  {y:3d} " + "".join(row) + ("   <- founding ground" if y == 160 else ""))
it = iter(frames)
nxt = next(it, None)
for r in csv.DictReader(gzip.open(f"{run}/cells.csv.gz", "rt")):
    f = int(r["frame"])
    while nxt is not None and f > nxt:
        show(nxt)
        nxt = next(it, None)
    if nxt is None:
        break
    state[(int(r["x"]), int(r["y"]))] = G.get(r["to"], ".")
while nxt is not None:
    show(nxt)
    nxt = next(it, None)
