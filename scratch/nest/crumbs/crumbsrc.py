"""SCRATCH (chambers crumbs lane, 2026-10-30): where the crumbs standing in a nest at the end of a run came from, ant by
ant, from a PP_CRUMB_LOG (mind.rs `crumb_log`) and the run's final nestmap.txt.

Every crumbs cell in the nest space (roomshape.nest_cells) at the end is joined to the LAST put-down that landed on it
(the D row whose landing cell -- the drop target, fallen straight down through empty cells, as a crumb falls -- is that
cell, with no pick-up from that cell after it). The share joined is printed: it is the reader's own control (a crumb
that landed on an ant and fell later, or was shifted by a dig, does not join).

Each joined crumb is classed by the load it came from:
  field      the load's first cell was picked up away from home (trip_cells), carried home and put down here
  rehandled  the load's first cell was picked up at home (a store cell taken up and put down again)
and by the dropper's state: hungry (under half a tank: the eat seed's sense bar 1/3) or fed; whether it Ate from the crop since the
load started; and where it lies in the final nest: neck (within 4 columns of the door and 12 rows of the surface),
room (inside the 5x5 opening of the dug cavity, crumbs counted as cavity), passage (the rest).

usage: crumbsrc.py RUNDIR [LOG]   (LOG defaults to RUNDIR/crumb.log)   -- one line of shares, then a table
import: classify(rundir, log) -> dict of counters
"""
import collections, os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'inrun'))
import roomshape

NEST = 128
K = 2
FALL = 10
FELL = os.environ.get('FELL', '1') == '1'
BLK = [(a, b) for a in range(-K, K + 1) for b in range(-K, K + 1)]
VERBS = ['walk', 'turn', 'rest', 'dig', 'pick', 'drop', 'eat', 'feed', 'attack']


def opening(c):
    ctr = {p for p in c if all((p[0] + a, p[1] + b) in c for a, b in BLK)}
    return {(q[0] + a, q[1] + b) for q in ctr for a, b in BLK}


def final_cells(d):
    t = open(f'{d}/nestmap.txt').read().split('\n')
    head = t[0].split(' ')
    x0, y0, legend = int(head[0]), int(head[1]), head[2].split(',')
    grid = [r for r in t[1:] if r]
    roomshape.OPEN_ONLY = False
    space, _, _, crater = roomshape.nest_cells(d)
    ci = chr(65 + legend.index('crumbs')) if 'crumbs' in legend else None
    fi = {chr(65 + k) for k, m in enumerate(legend) if m in ('fruit', 'food', 'provisions')}
    crumbs = {(x0 + dx, y0 + dy) for dx, dy in space if grid[dy][dx] == ci}
    food = {(x0 + dx, y0 + dy) for dx, dy in space if grid[dy][dx] in fi}
    sp = {(x0 + dx, y0 + dy) for dx, dy in space}
    cr = {(x0 + dx, y0 + dy) for dx, dy in crater}
    return sp, cr, crumbs, food


def place(p, room, surf):
    x, y = p
    if abs(x - NEST) <= 4 and y - surf[x] <= 12:
        return 'neck'
    return 'room' if p in room else 'passage'


def classify(d, logp=None):
    logp = logp or f'{d}/crumb.log'
    surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    sp, cr, crumbs, food = final_cells(d)
    room = opening(sp - cr)
    last = {}   # landing cell -> D row
    col = collections.defaultdict(list)  # column -> D rows landing there (for the fallen-crumb join)
    picked = {}  # cell -> frame of last pick-up there
    nd = collections.Counter()
    ev = collections.Counter()  # event level: every crumb put down at the nest, by the same classes
    for l in open(logp):
        if l[0] == 'P':
            p = l.rstrip().split(',')
            picked[(int(p[3]), int(p[4]))] = int(p[1])
        elif l[0] == 'D':
            p = l.rstrip().split(',')
            r = dict(frame=int(p[1]), id=int(p[2]), x=int(p[3]), y=int(p[4]), ly=int(p[5]), mat=p[8], worth=float(p[9]),
                     unit=float(p[10]), cells=int(p[11]), at_nest=int(p[12]), energy=float(p[13]), trip=int(p[14]),
                     first_frame=int(p[15]), first=(int(p[16]), int(p[17])), first_home=int(p[18]), picks=int(p[19]),
                     home_picks=int(p[20]), eats=int(p[21]), verbs=int(p[22], 16))
            last[(r['x'], r['ly'])] = r
            col[r['x']].append(r)
            if r['mat'] == 'crumbs' and r['at_nest']:
                ev[('src', 'field' if not r['first_home'] else 'rehandled')] += 1
                ev[('hun', 'hungry' if r['energy'] < 0.5 else 'fed')] += 1
                ev[('move', min(20, abs(r['x'] - r['first'][0]) + abs(r['ly'] - r['first'][1])) // 5 * 5)] += 1
            if r['mat'] == 'crumbs':
                nd['crumb_drops'] += 1
                nd['crumb_drops_nest'] += r['at_nest']
            nd['drops'] += 1
    out = collections.Counter()
    rows = []
    for c in crumbs:
        r = last.get(c)
        if r is None or picked.get(c, -1) > r['frame']:
            # A crumb under it was taken and it fell: the latest drop in this column that landed at most FALL rows
            # above, after the last pick-up from this cell. Looser; counted apart (`fell`). FELL=0: exact joins only.
            pk = picked.get(c, -1)
            cand = [q for q in col.get(c[0], []) if c[1] - FALL <= q['ly'] <= c[1] and q['frame'] > pk]
            if not cand or not FELL:
                out['unjoined'] += 1
                continue
            r = max(cand, key=lambda q: q['frame'])
            out['fell'] += 1
        src = 'field' if not r['first_home'] else 'rehandled'
        hun = 'hungry' if r['energy'] < 0.5 else 'fed'  # energy in tanks; the eat seed's sense 1/3 is half a tank
        ate = 'ate' if r['eats'] else 'noeat'
        pl = place(c, room, surf)
        prev = VERBS[(r['verbs'] >> 4) & 0xF]  # the low nibble is this drop itself
        out[(src, hun, ate, pl)] += 1
        out[('src', src)] += 1
        out[('hun', hun)] += 1
        out[('ate', ate)] += 1
        out[('place', pl)] += 1
        out[('prev', prev)] += 1
        out[('id', r['id'])] += 1
        out[('held', min(5, (r['frame'] - r['first_frame']) // 1000))] += 1
        out[('cells', min(5, r['cells']))] += 1
        out[('worthq', min(9, int(10 * r['worth'] / max(1.0, r['unit']))))] += 1
        rows.append(dict(r, _cell=c, _place=pl))
    return dict(space=len(sp), crumbs=len(crumbs), food=len(food), out=out, nd=nd, rows=rows, ev=ev, crumb_cells=crumbs)


def report(d, logp=None):
    c = classify(d, logp)
    o, n = c['out'], c['crumbs']
    j = n - o['unjoined']
    print(f"{d}: space {c['space']} crumbs {n} ({100 * n / max(1, c['space']):.0f}%) food {c['food']} | joined {j}/{n} "
          f"({100 * j / max(1, n):.0f}%) | drops {c['nd']['drops']} of which crumbs {c['nd']['crumb_drops']} "
          f"(at nest {c['nd']['crumb_drops_nest']}) | of the joined, fell {o['fell']}")
    e = c['ev']; ne = c['nd']['crumb_drops_nest']
    for key in ('src', 'hun', 'move'):
        items = sorted(((k[1], v) for k, v in e.items() if k[0] == key), key=lambda t: str(t[0]))
        print(f'  EVENTS {key:6}', ', '.join(f'{k} {100 * v / max(1, ne):.0f}%' for k, v in items))
    for key in ('src', 'hun', 'ate', 'place', 'prev', 'held', 'cells', 'worthq'):
        items = sorted(((k[1], v) for k, v in o.items() if isinstance(k, tuple) and k[0] == key), key=lambda t: -t[1])
        print(f'  {key:6}', ', '.join(f'{k} {v} ({100 * v / max(1, j):.0f}%)' for k, v in items))
    ids = sorted((v for k, v in o.items() if isinstance(k, tuple) and k[0] == 'id'), reverse=True)
    print(f'  droppers {len(ids)}; top 5 hold {sum(ids[:5])} ({100 * sum(ids[:5]) / max(1, j):.0f}%)')
    combos = sorted(((k, v) for k, v in o.items() if isinstance(k, tuple) and len(k) == 4), key=lambda t: -t[1])
    for k, v in combos[:10]:
        print(f'    {"/".join(k):34} {v:5} ({100 * v / max(1, j):.0f}%)')
    return c


if __name__ == '__main__':
    report(sys.argv[1], sys.argv[2] if len(sys.argv) > 2 else None)
