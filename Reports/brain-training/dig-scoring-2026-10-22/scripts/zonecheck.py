"""Paired check of the nest zone (nest=zone) against today's score (dig-scoring thread, 2026-10-22).

usage: python3 zonecheck.py DIR [CONTROL_ARM] [ZONE_ARM]     (defaults st0, zone)

DIR holds CONTROL_SEED/rN and ZONE_SEED/rN in rounds.sh's layout. Per arm, per colony-round, then paired by seed over
the rounds both reached:
  home     deliveries ('deliver' events within 12 columns of the door: the control's own home; the zone arm books
           drops outside its zone as 'tdrop', so its 'deliver' rows are already zone drops)
  starved  deaths with cause 1; extinct = an EXTINCT file
  pit      nest.csv's pit at the round's end; zone / reach = the zone's cells and its furthest column from the door
The owner's design left four risks open (Teacher lane, 2026-10-22), counted here for the zone arm:
  crater   share of PAID zone cuts ('zpay') under 3 rows deep: the crater still paying
  creep    the zone's reach in columns (food heaps sit 25 columns out, spread 10): a tunnel dragging the nest out
  buried   share of zone deliveries whose cell, in the next map (<= 4k frames later), no longer joins the open air:
           food counted in a part that filled in
  spoil    share of spoil let go 20+ columns out
"""
import csv, glob, os, re, sys, statistics
from collections import deque

D = sys.argv[1].rstrip('/')
CTL = sys.argv[2] if len(sys.argv) > 2 else 'st0'
ZON = sys.argv[3] if len(sys.argv) > 3 else 'zone'
NEST = 128

def maps_of(run):
    return sorted((int(re.search(r'map_f(\d+)', p).group(1)), p) for p in glob.glob(run + '/map_f*.txt'))

def air(path):
    """Open cells joined to the top row, as a set of (x, y) world cells."""
    L = open(path).read().split('\n'); x0, y0, w, h = map(int, L[0].split())
    m = [[c in '.af' for c in (r + ' ' * w)[:w]] for r in L[1:1 + h]]
    seen = {(0, x) for x in range(w) if m[0][x]}; q = deque(seen)
    while q:
        y, x = q.popleft()
        for a, b in ((y + 1, x), (y - 1, x), (y, x + 1), (y, x - 1)):
            if 0 <= a < h and 0 <= b < w and m[a][b] and (a, b) not in seen:
                seen.add((a, b)); q.append((a, b))
    return {(x + x0, y + y0) for y, x in seen}

def round_stats(run, buried=False):
    st = dict(home=0, tdrop=0, starved=0, zpay=0, zshallow=0, out=0, far=0)
    dels = []
    for e in csv.DictReader(open(run + '/events.csv')):
        k = e['kind']; x = int(e['x'])
        if k == 'deliver' and abs(x - NEST) <= 12 or k == 'deliver' and buried:
            st['home'] += 1; dels.append((int(e['frame']), x, int(e['y'])))
        elif k == 'tdrop':
            st['tdrop'] += 1
        elif k == 'death':
            st['starved'] += e['v'] == '1'
        elif k == 'zpay':
            st['zpay'] += 1; st['zshallow'] += int(e['v']) < 3
        elif k == 'out':
            st['out'] += 1; st['far'] += abs(x - NEST) >= 20
    rows = list(csv.DictReader(open(run + '/nest.csv')))
    end = rows[-1] if rows else {}
    st['pit'] = int(end.get('pit', 0)); st['nest'] = float(end.get('nest_score', 0))
    st['zone'] = int(end.get('zone', 0)); st['reach'] = int(end.get('zone_reach', 0))
    if buried and dels:
        ms = maps_of(run); cache = {}; gone = 0
        for f, x, y in dels:
            nxt = next((p for mf, p in ms if mf >= f), None)
            if nxt is None:
                continue
            if nxt not in cache:
                cache[nxt] = air(nxt)
            gone += (x, y) not in cache[nxt]
        st['buried'] = gone / len(dels)
    return st

def arm(name, buried=False):
    out = {}
    for sd in sorted(glob.glob(f'{D}/{name}_*')):
        seed = int(sd.rsplit('_', 1)[1])
        rs = sorted(glob.glob(sd + '/r*'), key=lambda p: int(p.rsplit('r', 1)[1]))
        out[seed] = dict(ext=os.path.exists(sd + '/EXTINCT'),
                         rounds={int(r.rsplit('r', 1)[1]): round_stats(r, buried) for r in rs if os.path.exists(r + '/events.csv')})
    return out

C, Z = arm(CTL), arm(ZON, buried=True)
seeds = sorted(set(C) & set(Z))
med = lambda v: statistics.median(v) if v else float('nan')
print(f'{len(seeds)} seeds; extinct {CTL} {sum(C[s]["ext"] for s in seeds)}, {ZON} {sum(Z[s]["ext"] for s in seeds)}')
rows = []
for s in seeds:
    common = sorted(set(C[s]['rounds']) & set(Z[s]['rounds']))
    for r in common:
        c, z = C[s]['rounds'][r], Z[s]['rounds'][r]
        rows.append((s, r, c, z))
        print(f'seed {s} r{r}: home food {c["home"]:5d} -> {z["home"]:5d} (zone tdrop {z["tdrop"]})  starved {c["starved"]:3d} -> {z["starved"]:3d}  '
              f'pit {c["pit"]:3d} -> {z["pit"]:3d}  nest {c["nest"]:6.0f} -> {z["nest"]:6.0f}  | zone {z["zone"]} cells, reach {z["reach"]} cols, '
              f'paid cuts {z["zpay"]} ({100 * z["zshallow"] / max(1, z["zpay"]):.0f}% shallow), buried {100 * z.get("buried", 0):.0f}%, '
              f'spoil far {100 * c["far"] / max(1, c["out"]):.0f}% -> {100 * z["far"] / max(1, z["out"]):.0f}%')
for k in ('home', 'starved', 'pit', 'nest'):
    cv = [c[k] for _, _, c, _ in rows]; zv = [z[k] for _, _, _, z in rows]
    up = sum(b > a for a, b in zip(cv, zv)); dn = sum(b < a for a, b in zip(cv, zv))
    print(f'{k:8s} median {med(cv):8.1f} -> {med(zv):8.1f}   zone higher in {up}, lower in {dn} of {len(rows)} colony-rounds')
