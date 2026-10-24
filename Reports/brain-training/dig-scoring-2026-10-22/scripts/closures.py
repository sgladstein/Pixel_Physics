"""Did a closed door cost the colony anything? (dig-scoring thread, 2026-10-22; owner: charge a closure only when it
shuts out foraging or eating.)  usage: closures.py RUN_DIR [RUN_DIR ...]

Offline, from an in-run dir with maps on (map_f*.txt), ants_f*.txt and stock.csv at the same frames. At each map after
20k frames: the nest's largest piece (open cells 3+ rows under the datum within 20 columns of x=128, 4-connected), as
lab::nestscore::nest_piece finds it, and whether it joins the air. For each CLOSED check it counts
  in_hungry  ants whose head is in the piece with under 1 tank, while the store (stock.csv nest_food) is under 1 per ant
  out_laden  ants outside the piece, above the datum, holding a pellet (the file cannot say food from soil, so this is
             an upper bound on foragers shut out)
A closure with both zero is the kind the owner wants left alone.
"""
import csv, glob, os, re, sys
from collections import deque

NEST, ROWS, REACH = 128, 3, 20

def piece(path, surf):
    L = open(path).read().split('\n'); x0, y0, w, h = map(int, L[0].split())
    m = [[c in '.af' for c in (r + ' ' * w)[:w]] for r in L[1:1 + h]]
    nest = {(y, x) for x in range(w) if abs(x + x0 - NEST) <= REACH for y in range(h)
            if m[y][x] and y + y0 >= surf[x + x0] + ROWS}
    if not nest:
        return None
    best = set()
    left = set(nest)
    while left:
        s = left.pop(); comp = {s}; st = [s]
        while st:
            y, x = st.pop()
            for n in ((y + 1, x), (y - 1, x), (y, x + 1), (y, x - 1)):
                if n in left:
                    left.discard(n); comp.add(n); st.append(n)
        if len(comp) > len(best):
            best = comp
    seen = set((0, x) for x in range(w) if m[0][x]); q = deque(seen)
    while q:
        y, x = q.popleft()
        for a, b in ((y + 1, x), (y - 1, x), (y, x + 1), (y, x - 1)):
            if 0 <= a < h and 0 <= b < w and m[a][b] and (a, b) not in seen:
                seen.add((a, b)); q.append((a, b))
    return dict(open=bool(best & seen), cells={(y + y0, x + x0) for y, x in best})

for run in sys.argv[1:]:
    surf = [int(l) for l in open(run + '/surface.txt')]
    stock = {int(r['frame']): r for r in csv.DictReader(open(run + '/stock.csv'))}
    # tank = energy / start energy: recover start energy from the first ants file and stock's tank_mean
    checks = closed = costly = 0; worst = []; ctl = []
    start_e = None
    for p in sorted(glob.glob(run + '/map_f*.txt')):
        f = int(re.search(r'map_f(\d+)', p).group(1))
        ap = f'{run}/ants_f{f:07d}.txt'
        if not os.path.exists(ap) or f not in stock:
            continue
        ants = [l.split() for l in open(ap) if l.strip()]
        if start_e is None and ants and float(stock[f]['tank_mean']) > 0:
            start_e = sum(float(a[3]) for a in ants) / len(ants) / float(stock[f]['tank_mean'])
        if f < 20000:
            continue
        pc = piece(p, surf)
        if pc is None:
            continue
        checks += 1
        low = int(stock[f]['nest_food']) < len(ants)
        inh = sum(1 for a in ants if (int(a[2]), int(a[1])) in pc['cells'] and float(a[3]) / start_e < 1 and low)
        outl = sum(1 for a in ants if (int(a[2]), int(a[1])) not in pc['cells'] and int(a[2]) < surf[int(a[1])] and a[4] == '1')
        if pc['open']:
            # The control: the same counts while the door is open. Ants carry spoil out of an open nest all the time.
            ctl.append((inh, outl))
            continue
        closed += 1
        costly += (inh + outl) > 0
        worst.append((f, inh, outl, len(ants), int(stock[f]['nest_food'])))
    mean = lambda v: sum(v) / len(v) if v else float('nan')
    print(f'{run}: {checks} checks with a nest, {closed} closed, {costly} of them costly; '
          f'mean hungry in / laden out: closed {mean([w[1] for w in worst]):.1f} / {mean([w[2] for w in worst]):.1f}, '
          f'open (control) {mean([c[0] for c in ctl]):.1f} / {mean([c[1] for c in ctl]):.1f}')
    for f, inh, outl, n, store in worst[:12]:
        print(f'   f{f:7d}  hungry shut in {inh:2d}  laden shut out {outl:2d}  (alive {n}, store {store})')
