#!/usr/bin/env python3
"""Where eggs are laid and by whom, from broodlog.csv `laid` rows (first row
per egg id only; later ones are re-sightings).

usage: eggwhere.py RUN [windows]

Egg cell relative to the door: dx = x - nest_x, depth = y - ground_y (rows
below the old ground line; negative is above it, in the mound). The layer's
zone and energy after the egg (`parent_energy`, read the frame the egg is
first seen, i.e. after the 120 J charge).
"""
import csv, sys, statistics as st, re
from collections import Counter
run = sys.argv[1]
wins = [tuple(int(v) for v in w.split('-')) for w in (sys.argv[2] if len(sys.argv) > 2 else '20000-50000,50000-100000,100000-150000,150000-200000').split(',')]
ev = open(f'{run}/events.txt').read()
m = re.search(r'FOUNDED nest_x=(\d+) food_x=(\d+) ground_y=(\d+)', ev)
if m:
    nx, fx, gy = map(int, m.groups())
else:  # events.txt is written at the end; the log has the site, the goal box's ground row is 160
    m = re.search(r'nest at x (\d+), food spot x (\d+)', open(f'{run}.log').read())
    nx, fx, gy = int(m.group(1)), int(m.group(2)), 160
seen = set(); eggs = []
with open(f'{run}/broodlog.csv') as fh:
    for r in csv.DictReader(fh):
        if r['event'] != 'laid' or r['id'] in seen:
            continue
        seen.add(r['id'])
        eggs.append(r)
for w in wins:
    es = [r for r in eggs if w[0] < int(r['frame']) <= w[1]]
    if not es:
        continue
    def place(r):
        dx, d = int(r['x']) - nx, int(r['y']) - gy
        col = 'door col' if abs(dx) <= 2 else ('east' if dx > 0 else 'west')
        band = 'mound' if d < 0 else ('0-2' if d <= 2 else ('3-10' if d <= 10 else '>10'))
        return f'{col} {band}'
    pc = Counter(place(r) for r in es)
    zc = Counter(r['parent_zone'] for r in es)
    pe = [float(r['parent_energy']) for r in es if r['parent_energy']]
    print(f'== {run.split("/")[-1]} {w[0]//1000}-{w[1]//1000}k: {len(es)} eggs (door x={nx}, ground row {gy})')
    print('   egg cell: ' + ', '.join(f'{k} {100*v/len(es):.0f}%' for k, v in pc.most_common(8)))
    print('   layer zone: ' + ', '.join(f'{k} {100*v/len(es):.0f}%' for k, v in zc.most_common()))
    print(f'   layer energy after the egg: median {st.median(pe):.0f}, under 200 J {100*sum(e<200 for e in pe)/len(pe):.1f}%')
