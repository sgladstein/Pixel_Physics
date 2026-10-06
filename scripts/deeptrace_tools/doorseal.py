"""doorseal.py RUN_DIR... -- is the nest's door cut off from the open air? (deep trace lane, 2026-10-06)

Reads deeptrace's map snapshots (map_fNNNNNN.txt, `mapevery=`; nest_goal geometry: door anchor (256,159), ground row 160)
and, for each, walks from the door anchor through air, dug and ant cells (8-way, down to row 175) and asks whether the walk
reaches the open air: a cell at or above the door row with no ground or food within 20 rows overhead -- the cell a soil
carrier reads as Out (`carry_stage`), where it can draw its column and put its pellet down. SEALED means no such walk exists:
every ant in the mound's tunnels, the shaft and the rooms is shut in until something digs or falls.

Prints one letter per map (O open, S sealed) and, with -v, the mound's size, the food lying in it and the door system's size.
Ant cells count as passable (ants swap), and so do brood (map 'b'; 'e'/'l'/'p' in nest pictures) and crumbs (map 'c'):
under PushPast (crumbs and brood, both shipped on) a body walks through brood and through unowned food that
`carries_worth` -- crumbs, and only crumbs (`is_partable`, `cell_is_enterable`). Corpses ('x'), provisions and any
other food ('f') are walls to it, and cover overhead, as ground is.
**Corrected 2026-10-06 03:50** (deep trace lane): until then brood and every food counted as walls (`OLD_PASS`, or
pass --old). Maps written before deeptrace drew crumbs and corpses apart (scratch patch `home15/mapfood.patch`) draw
all food 'f', so on them a verdict is an upper bound on what is sealed; --food-open gives a loose lower bound (every
food cell passable, the food heap's provisions too, so a pocket can join the sky through the heap: on seed 2 with
CARRY_HOME it opened 98 of 116 ants' pockets at 220k that the exact maps show walled by soil). The flood is 8-way with no corner rule, so it can join spaces the walk cannot.
"""
import sys, glob, re, collections

DX, DY = 256, 159
SOLID = set('s#=rf') | set('cx')
OLD_PASS = set('.oa')
PASS = set('.oa') | set('belp') | set('c')


def load(p):
    L = open(p).read().split('\n')
    x0, y0, w, h = map(int, L[0].split())
    rows = [r.ljust(w, 'r')[:w] for r in L[1:1 + h]]
    return lambda x, y: rows[y - y0][x - x0] if (0 <= x - x0 < w and 0 <= y - y0 < h) else 'r'


def measure(p):
    at = load(p)
    spoil = sum(at(x, y) in 's=' for x in range(DX - 30, DX + 31) for y in range(120, 160))
    food = sum(at(x, y) == 'f' for x in range(DX - 30, DX + 31) for y in range(120, 160))
    seen = {(DX, DY)}
    q = collections.deque([(DX, DY)])
    sky = False
    while q:
        x, y = q.popleft()
        if y <= DY and not any(at(x, y - d) in SOLID for d in range(1, 21)):
            sky = True
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                n = (x + dx, y + dy)
                if n in seen or at(*n) not in PASS or n[1] > 175:
                    continue
                seen.add(n)
                q.append(n)
    return sky, spoil, food, len(seen)


def main():
    global PASS
    verbose = '-v' in sys.argv
    if '--old' in sys.argv:
        PASS = OLD_PASS
    if '--food-open' in sys.argv:
        PASS = PASS | set('f')
    for d in [a for a in sys.argv[1:] if a not in ('-v', '--old', '--food-open')]:
        maps = sorted(glob.glob(d + '/map_f*.txt'), key=lambda p: int(re.search(r'map_f(\d+)', p).group(1)))
        res = [(int(re.search(r'map_f(\d+)', p).group(1)), *measure(p)) for p in maps]
        line = ''.join('O' if r[1] else 'S' for r in res)
        print(f"{d.rstrip('/').split('/')[-1]}: {len(res)} maps, {line.count('S')} sealed")
        for i in range(0, len(res), 50):
            print(f"  {res[i][0] // 1000:4d}k {line[i:i + 50]}")
        if verbose:
            for fr, sky, spoil, food, n in res:
                print(f"    {fr // 1000:4d}k {'open  ' if sky else 'SEALED'} mound spoil {spoil:4d} food in mound {food:3d} door system {n:5d} cells")


if __name__ == '__main__':
    main()
