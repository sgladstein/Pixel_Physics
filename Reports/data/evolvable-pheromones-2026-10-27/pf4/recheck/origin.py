#!/usr/bin/env python3
"""For each field pickup, what last put food on that cell: a churn drop, a field drop, a delivery, or nothing logged
(heap / original food). Tells whether near-nest pickups are food re-picked after being put down."""
import csv, sys, collections
NEST = 128
for d in sys.argv[1:]:
    heaps = [(int(r['frame']), int(r['x'])) for r in csv.DictReader(open(f'{d}/patches.csv'))]
    last = {}
    tally = collections.Counter()
    gap = collections.defaultdict(list)
    for r in csv.reader(open(f'{d}/events.csv')):
        if len(r) < 6 or r[0] == 'frame': continue
        k = r[2]
        if k in ('deliver', 'churn', 'fdrop'):
            last[(int(r[3]), int(r[4]))] = (k, int(r[0]), r[1])
        elif k == 'field':
            x, y, f = int(r[3]), int(r[4]), int(r[0])
            hx = [h for fr, h in heaps if fr <= f][-1]
            where = 'heap' if abs(x - hx) <= 15 else 'near' if abs(x - NEST) <= 30 else 'other'
            src = last.pop((x, y), None)
            tag = src[0] if src else 'unlogged'
            same = src and src[2] == r[1]
            tally[(where, tag + ('/same ant' if same else ''))] += 1
            if src: gap[(where, tag)].append(f - src[1])
    print(d.split('/r1/')[1])
    for k, v in sorted(tally.items()):
        g = sorted(gap.get((k[0], k[1].split('/')[0]), [0]))
        print(f'   {k[0]:5} {k[1]:22} {v:6}   median frames since drop {g[len(g)//2]}')
