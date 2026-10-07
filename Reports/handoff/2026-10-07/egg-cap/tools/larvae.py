#!/usr/bin/env python3
"""How the brood is fed, per window, from deeptrace's brood.csv, colony.csv
(laying census build), feeds.csv, broodlog.csv and stats.csv.

usage: larvae.py RUN [windows]

- standing brood per 1k sample: eggs, larvae, pupae; where the larvae lie
  (door column = within 2 columns of the door; depth below the old ground line)
- donors: adults whose head is within 2 cells of a larva and who hold over the
  200 J grant (the nurse-by-touch rule takes from a grown nestmate on one of the
  larva's eight neighbours over its grant; a head within 2 is a proxy for a body
  touching it)
- larval food by source over the window (feeds.csv kinds: ate = food beside it,
  share/bank = a nestmate's body, crop = a carrier's crop) and per pupa made
- time from larva to pupa (broodlog stage rows), larvae starved
"""
import csv, sys, re, statistics as st
from collections import defaultdict, Counter
run = sys.argv[1]
wins = [tuple(int(v) for v in w.split('-')) for w in (sys.argv[2] if len(sys.argv) > 2 else '20000-50000,50000-100000,100000-150000,150000-200000').split(',')]
m = re.search(r'FOUNDED nest_x=(\d+) food_x=(\d+) ground_y=(\d+)', open(f'{run}/events.txt').read())
if m:
    nx, gy = int(m.group(1)), int(m.group(3))
else:
    nx, gy = int(re.search(r'nest at x (\d+)', open(f'{run}.log').read()).group(1)), 160
def win(f):
    for w in wins:
        if w[0] < f <= w[1]:
            return w
brood = defaultdict(list)
for r in csv.DictReader(open(f'{run}/brood.csv')):
    w = win(int(r['frame']))
    if w: brood[w].append(r)
adults = defaultdict(lambda: defaultdict(list))
for r in csv.DictReader(open(f'{run}/colony.csv')):
    w = win(int(r['frame']))
    if w: adults[w][int(r['frame'])].append((int(r['hx']), int(r['hy']), float(r['energy_j']), r['crop_cells'] != '0'))
food = defaultdict(Counter)
for r in csv.DictReader(open(f'{run}/feeds.csv')):
    w = win(int(r['frame']))
    if w: food[w][r['kind']] += float(r['gain'])
first = {}
for r in csv.DictReader(open(f'{run}/broodlog.csv')):
    if r['event'] in ('laid', 'stage'):
        key = (r['id'], r['stage'])
        if key not in first:
            first[key] = int(r['frame'])
stats = {int(r['frame']): r for r in csv.DictReader(open(f'{run}/stats.csv'))}
at = lambda f: stats[max(k for k in stats if k <= f)]
for w in wins:
    b = brood[w]
    if not b: continue
    frames = sorted({int(r['frame']) for r in b})
    n = len(frames)
    sc = Counter(r['stage'] for r in b)
    lv = [r for r in b if r['stage'] == 'larva']
    def place(r):
        dx, d = int(r['x']) - nx, int(r['y']) - gy
        return ('door col' if abs(dx) <= 2 else 'off col') + (' mound' if d < 0 else ' 0-10' if d <= 10 else ' >10')
    pc = Counter(place(r) for r in lv)
    don = 0; doncrop = 0
    for r in lv:
        x, y = int(r['x']), int(r['y'])
        near = [a for a in adults[w].get(int(r['frame']), []) if max(abs(a[0]-x), abs(a[1]-y)) <= 2]
        don += any(a[2] > 200 for a in near)
        doncrop += any(a[3] for a in near)
    a0, a1 = at(w[0]), at(w[1])
    d = lambda k: float(a1[k]) - float(a0[k])
    pupae = d('pupae'); starved = d('larvae_starved')
    fd = food[w]; tot = sum(fd.values())
    dur = [first[(i, 'pupa')] - first[(i, 'larva')] for (i, s) in first if s == 'larva' and (i, 'pupa') in first and w[0] < first[(i, 'larva')] <= w[1]]
    print(f'== {run.split("/")[-1]} {w[0]//1000}-{w[1]//1000}k: standing per sample: eggs {sc["egg"]/n:.0f}, larvae {sc["larva"]/n:.0f}, pupae {sc["pupa"]/n:.0f}')
    print('   larvae lie: ' + ', '.join(f'{k} {100*v/len(lv):.0f}%' for k, v in pc.most_common()) if lv else '   no larvae')
    if lv:
        print(f'   larvae with an adult over 200 J within 2 cells: {100*don/len(lv):.0f}%; with a crop carrier within 2: {100*doncrop/len(lv):.0f}%')
    print(f'   larval food {tot/1000:.0f} kJ: ' + ', '.join(f'{k} {100*v/tot:.0f}%' for k, v in fd.most_common()) + f'; pupae made {pupae:.0f} ({tot/max(1,pupae):.0f} J per pupa); larvae starved {starved:.0f}')
    if dur:
        print(f'   larva -> pupa: median {st.median(dur):.0f} frames, p90 {sorted(dur)[int(.9*len(dur))]:.0f} (n={len(dur)})')
