#!/usr/bin/env python3
"""What holds an affordable egg, from deeptrace's laying census (colony.csv
with the census columns, Laying 2026-10-07).

usage: laycensus.py RUN [windows e.g. 20000-50000,50000-100000,...]

Per window, over every adult sampled every 1k frames:
- adults, and the share at or above their own egg bar (`bar`, the body bar
  under LAY_BAR=body) -- "affordable";
- where the affordable ones stand (zone), and whether a home cell is within
  the pile's reach of 4 (`home_d` >= 0);
- of affordable ants with home in reach: whether the pile walk finds an empty
  home cell (`pile`), and if not, what fills the 9x9 round the head, and
  whether the walk would succeed if it could pass brood (`pile_thru_brood`).
- the stats counters over the window: eggs laid, held ticks (no pile site),
  declined, larvae starved.
"""
import csv, sys, statistics as st
from collections import Counter, defaultdict

run = sys.argv[1]
wins = [tuple(int(v) for v in w.split('-')) for w in (sys.argv[2] if len(sys.argv) > 2 else '20000-50000,50000-100000,100000-150000,150000-200000').split(',')]

rows = defaultdict(list)
with open(f'{run}/colony.csv') as fh:
    for r in csv.DictReader(fh):
        f = int(r['frame'])
        for w in wins:
            if w[0] < f <= w[1]:
                rows[w].append(r)

stats = {}
with open(f'{run}/stats.csv') as fh:
    for r in csv.DictReader(fh):
        stats[int(r['frame'])] = r

def at(f):
    k = max(k for k in stats if k <= f)
    return stats[k]

def q(xs, p):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(p * len(xs)))] if xs else float('nan')

for w in wins:
    rs = rows[w]
    if not rs:
        continue
    n = len(rs)
    frames = len({r['frame'] for r in rs})
    e = [float(r['energy_j']) for r in rs]
    aff = [r for r in rs if float(r['energy_j']) >= float(r['bar'])]
    bar = st.median(float(r['bar']) for r in rs)
    a, b = at(w[0]), at(w[1])
    d = lambda k: int(float(b[k])) - int(float(a[k]))
    print(f'== {run.split("/")[-1]} {w[0]//1000}-{w[1]//1000}k: {frames} samples, adults/sample {n/frames:.0f}, bar {bar:.0f} J')
    print(f'   energy all: median {st.median(e):.0f}  p75 {q(e,.75):.0f}  p90 {q(e,.9):.0f}  max {max(e):.0f}')
    print(f'   affordable (>= bar): {len(aff)/frames:.1f} per sample = {100*len(aff)/n:.1f}% of adults; their energy median {st.median([float(r["energy_j"]) for r in aff]) if aff else 0:.0f}')
    print(f'   counters over window: eggs laid {d("eggs_laid")}, held no pile site {d("buds_held_for_nest")}, declined {d("lays_declined")}, larvae starved {d("larvae_starved")}, births {d("births")}')
    if not aff:
        continue
    z = Counter(r['zone'] for r in aff)
    zall = Counter(r['zone'] for r in rs)
    print('   affordable by zone (share of affordable | share of that zone that is affordable): ' + ', '.join(f'{k} {100*v/len(aff):.0f}% | {100*v/zall[k]:.0f}%' for k, v in z.most_common()))
    inr = [r for r in aff if int(r['home_d']) >= 0]
    out = len(aff) - len(inr)
    print(f'   home within 4: {100*len(inr)/len(aff):.0f}% of affordable (no home in reach: {100*out/len(aff):.0f}%)')
    hd = Counter(min(int(r['home_d']), 4) for r in inr)
    print('     nearest home cell at: ' + ', '.join(f'd{k} {100*v/len(aff):.0f}%' for k, v in sorted(hd.items())))
    pile = sum(r['pile'] == '1' for r in inr)
    print(f'     pile site found: {pile} ({100*pile/max(1,len(aff)):.1f}% of affordable) -- an ant with one lays at its next tick unless its brain declines')
    held = [r for r in inr if r['pile'] == '0']
    if held:
        m = lambda k: st.mean(int(r[k]) for r in held)
        thru = sum(r['pile_thru_brood'] == '1' for r in held)
        print(f'     at home with no site ({100*len(held)/len(aff):.0f}% of affordable): 9x9 round head mean brood {m("r4_brood"):.1f}, food {m("r4_food"):.1f}, other ants {m("r4_ant"):.1f}, empty home cells {m("r4_emptyhome"):.1f}; near home cells free {m("near_free"):.2f} / held by ants {st.mean(int(r["home_ants"]) for r in held):.1f}')
        print(f'       would find a cell if brood were passable: {100*thru/len(held):.0f}%;  zero empty home cells in the 9x9: {100*sum(r["r4_emptyhome"]=="0" for r in held)/len(held):.0f}%')
        hz = Counter(r['zone'] for r in held)
        print('       their zones: ' + ', '.join(f'{k} {100*v/len(held):.0f}%' for k, v in hz.most_common()))
