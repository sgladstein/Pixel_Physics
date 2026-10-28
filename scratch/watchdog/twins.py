#!/usr/bin/env python3
"""SCRATCH (score-gaming watchdog, 2026-10-28): every colsel case beside a ground-truth twin, per colony-round.

A case is gamed when selection raises it without the outcome it stands for. So for each case this pairs the number
colsel ranked on (read back from colsel.csv, exactly as voted) with a twin read from the same round folder:

  home8    deliveries within 8 of the door      twin: of those, deliveries whose trip began at a cell no ant had
                                                put down (loop.py's 'source'): food that came from outside. Needs
                                                events.csv(.gz), which colsel keeps for the last round only
  nest     dug nest cells, food-filled included twin: nest_open (open air only)
  crater   empty cells of the sky-open hole     twin: crater_full (with the crumbs and food that fill it)
  shape    roomshape score, food counts as room twin: roomshape with OPEN_ONLY
  starved  adult starvation deaths              twin: alive at the end, and eggs laid (fewer eggs, fewer to starve)
  scale    nest_open / need(alive, brood, food) twin: nest_open itself (a small colony needs little)
  spoil    spoil on the surface near the door   twin: spoil_in (soil carried back into the nest)

usage: twins.py ARM_DIR [ARM_DIR ...] > table.csv     ARM_DIR: a colsel OUT (colsel.csv + c<k>/r<r>/ folders)
Columns: arm,round,colony,rank,donor(children this round),<case>,<twin>... ; missing folder -> blank twins.
"""
import csv, gzip, os, sys, collections
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'inrun'))
import nestcount, roomshape  # noqa: E402

DOOR = 128


def loop(d):
    """loop.py's tracer (Reports/data/evolvable-pheromones-2026-10-27/pf4/recheck/loop.py), restricted to colsel's
    home8 window (deliveries within 8 columns of the door)."""
    fn = f'{d}/events.csv'
    fh = open(fn) if os.path.exists(fn) else gzip.open(fn + '.gz', 'rt') if os.path.exists(fn + '.gz') else None
    if fh is None:
        return None
    last, q, o = {}, collections.defaultdict(collections.deque), collections.Counter()
    for r in csv.reader(fh):
        if len(r) < 6 or r[0] == 'frame':
            continue
        k = r[2]
        if k == 'field':
            q[r[1]].append(last.pop((r[3], r[4]), 'source'))
        elif k in ('deliver', 'churn', 'fdrop'):
            last[(r[3], r[4])] = k
            tag = q[r[1]].popleft() if q[r[1]] else 'none'
            if k == 'deliver' and abs(int(r[3]) - DOOR) <= 8:
                o['home8'] += 1
                o['home8_' + tag] += 1
                o['home8_j'] += int(r[5])
                if tag == 'source':
                    o['home8_source_j'] += int(r[5])
    return o


def last(path):
    try:
        rows = list(csv.DictReader(open(path)))
        return rows[-1] if rows else {}
    except OSError:
        return {}


def twins(d):
    out = {}
    if not os.path.exists(f'{d}/nestmap.txt'):
        return out
    n = nestcount.count(d, 200000)
    out.update(nest_open=n['nest_open'], crater_full=n['crater_full'], nest_all=n['nest'], crater_empty=n['crater'])
    roomshape.OPEN_ONLY = True
    try:
        out['shape_open'] = roomshape.score(d)
    except Exception:
        out['shape_open'] = ''
    roomshape.OPEN_ONLY = False
    b = last(f'{d}/brood.csv')
    out['laid'] = b.get('laid', '')
    out['larvae_starved'] = b.get('larvae_starved', '')
    s = last(f'{d}/stock.csv')
    out['alive_end'] = s.get('alive', '')
    out['store_cells'] = int(s.get('nest_food', 0) or 0) + int(s.get('door_food', 0) or 0) if s else ''
    out['tank_mean'] = s.get('tank_mean', '')
    l = last(f'{d}/larder.csv')
    out['store_kj'] = round(sum(float(l.get(k, 0) or 0) for k in ('prov_kj', 'crumb_kj', 'other_kj')), 1) if l else ''
    lp = loop(d)
    if lp is not None:
        out.update(home8_traced=lp['home8'], home8_source=lp['home8_source'], home8_recarry=lp['home8_deliver'],
                   home8_kj=round(lp['home8_j'] / 1000, 1), home8_source_kj=round(lp['home8_source_j'] / 1000, 1))
    return out


TW = ['nest_all', 'nest_open', 'crater_empty', 'crater_full', 'shape_open', 'laid', 'larvae_starved', 'alive_end',
      'store_cells', 'store_kj', 'tank_mean', 'home8_traced', 'home8_source', 'home8_recarry', 'home8_kj', 'home8_source_kj']

if __name__ == '__main__':
    w = None
    for arm in sys.argv[1:]:
        rows = list(csv.DictReader(open(f'{arm}/colsel.csv')))
        kids = collections.Counter((r['round'], r['refounded_from']) for r in rows if r['refounded_from'])
        for r in rows:
            d = f"{arm}/c{r['colony']}/r{r['round']}"
            t = twins(d)
            row = {'arm': os.path.basename(arm.rstrip('/')), **r, 'children': kids[(r['round'], r['colony'])],
                   **{k: t.get(k, '') for k in TW}}
            if w is None:
                w = csv.DictWriter(sys.stdout, list(row))
                w.writeheader()
            w.writerow(row)
        sys.stdout.flush()
