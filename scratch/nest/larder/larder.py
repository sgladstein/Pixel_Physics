"""SCRATCH (chambers lane, larder seed): the screen's deciding measures for one run dir (an inrun round).

  space      nest space joined to the door (roomshape.nest_cells, food and crumbs pass the flood)
  passage    share of that space outside any 5x5 open block (roomcopy/score.py's `thin`); None under 230 cells
  crumbs     crumb cells in the nest space;  pile = the largest 8-joined group of food cells (crumbs or fruit) there
  strewn     crumb cells in the nest space NOT in the largest pile -- the crumbs that wall passages
  inpass     crumb cells in passage cells (nest space outside every 5x5 block of nest space): crumbs in the way
  open       nestcount's nest_open (nest cells still open air)
  alive, starved, under (starved below the surface, by the dead ant's last trace row, as eatseed/table.py)
usage: larder.py RUNDIR [RUNDIR ...]   or import measure(d). Run from the repo root."""
import sys, os, csv, io, contextlib
sys.path.insert(0, 'scratch/inrun'); sys.path.insert(0, 'scratch/nest')
import roomshape, nestcount
roomshape.OPEN_ONLY = False


def rooms(space):
    S = set(space); K = 2; room = set()
    for (x, y) in S:
        if all((x + a, y + b) in S for a in range(-K, K + 1) for b in range(-K, K + 1)):
            room.update((x + a, y + b) for a in range(-K, K + 1) for b in range(-K, K + 1))
    return room


def thin(space):
    return (len(set(space)) - len(rooms(space))) / max(len(set(space)), 1)


def measure(d, frame=None):
    t = open(f'{d}/nestmap.txt').read().split('\n')
    head = t[0].split(' '); legend = head[2].split(','); grid = [r for r in t[1:] if r]
    space, _, _, _ = roomshape.nest_cells(d)
    S = set(space)
    code = {chr(65 + k): m for k, m in enumerate(legend)}
    food = {p for p in S if code.get(grid[p[1]][p[0]]) in ('crumbs', 'fruit', 'food')}
    crumbs = {p for p in food if code.get(grid[p[1]][p[0]]) == 'crumbs'}
    best, seen = set(), set()
    for p in food:
        if p in seen: continue
        comp, st = set(), [p]
        while st:
            q = st.pop()
            if q in comp: continue
            comp.add(q)
            for a in (-1, 0, 1):
                for b in (-1, 0, 1):
                    r = (q[0] + a, q[1] + b)
                    if r in food and r not in comp: st.append(r)
        seen |= comp
        if len(comp) > len(best): best = comp
    with contextlib.redirect_stdout(io.StringIO()):
        nc = nestcount.count(d)
    surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    last = {}
    if os.path.exists(f'{d}/trace.csv'):
        for r in csv.DictReader(open(f'{d}/trace.csv')): last[int(r['id'])] = (int(r['x']), int(r['y']))
    st = und = 0
    for r in csv.reader(open(f'{d}/events.csv')):
        if not r[0].isdigit(): continue
        if r[2] == 'death' and int(float(r[5])) == 1:
            st += 1; p = last.get(int(r[1]))
            und += bool(p and p[1] > surf[p[0]])
    alive = int(list(csv.DictReader(open(f'{d}/nest.csv')))[-1]['alive'])
    return dict(space=len(S), passage=thin(space) if len(S) >= 230 else None, crumbs=len(crumbs), pile=len(best),
                strewn=len(crumbs - best), inpass=len(crumbs - rooms(space)), open=nc['nest_open'], alive=alive, starved=st, under=und)


if __name__ == '__main__':
    for d in sys.argv[1:]:
        m = measure(d)
        ps = 'n/a' if m['passage'] is None else f"{100 * m['passage']:.0f}%"
        print(f"{d}: space {m['space']} passage {ps} crumbs {m['crumbs']} pile {m['pile']} strewn {m['strewn']} inpass {m['inpass']} "
              f"open {m['open']} alive {m['alive']} starved {m['starved']} under {m['under']}")
