#!/usr/bin/env python3
"""Is the nest open to the sky anywhere but its door?

Rebuilds the ground from cells.csv.gz (as rebuild.py), then floods the open
cells from the top row of the box, 8-connected, never entering the door's
columns (DOOR_X0..DOOR_X1) between the surface and DOOR_FOOT. Prints the
deepest row the flood reaches below the founding ground and, at rows >= 166
(the room), the x span it reaches. The door shaft was cut before the log
began, so the rebuild holds it as ground unless a later change shows it.
Usage: second_way.py RUNDIR F1 F2 ...   [door=253-259 foot=168]"""
import csv, gzip, sys
run = sys.argv[1]
kw = dict(a.split("=", 1) for a in sys.argv[2:] if "=" in a)
frames = sorted(int(a) for a in sys.argv[2:] if "=" not in a)
D0, D1 = map(int, kw.get("door", "253-259").split("-")); FOOT = int(kw.get("foot", 168))
G = {"soil", "packedsoil", "spoil"}
X0, X1, Y0, Y1 = 170, 345, 131, 230
FIRST, state = {}, {}
for r in csv.DictReader(gzip.open(f"{run}/cells.csv.gz", "rt")):
    FIRST.setdefault((int(r["x"]), int(r["y"])), r["from"] in G)
def ground(x, y):
    if (x, y) in state:
        return state[(x, y)]
    return FIRST.get((x, y), y >= 160)
def flood(f):
    seen = set((x, Y0) for x in range(X0, X1 + 1) if not ground(x, Y0))
    st = list(seen)
    while st:
        x, y = st.pop()
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                n = (x + dx, y + dy)
                if n in seen or not (X0 <= n[0] <= X1 and Y0 <= n[1] <= Y1):
                    continue
                if D0 <= n[0] <= D1 and 158 <= n[1] <= FOOT:
                    continue
                if ground(*n):
                    continue
                seen.add(n); st.append(n)
    deep = max((y for x, y in seen), default=None)
    room = [x for x, y in seen if y >= 166]
    print(f"{f}: deepest open row reached from the sky avoiding the door: {deep}" + (f"; reaches the room (rows >= 166) at x {min(room)}-{max(room)}, {len(room)} cells" if room else "; does not reach the room"))
it = iter(frames); nxt = next(it, None)
for r in csv.DictReader(gzip.open(f"{run}/cells.csv.gz", "rt")):
    f = int(r["frame"])
    while nxt is not None and f > nxt:
        flood(nxt); nxt = next(it, None)
    if nxt is None:
        break
    state[(int(r["x"]), int(r["y"]))] = r["to"] in G
while nxt is not None:
    flood(nxt); nxt = next(it, None)
