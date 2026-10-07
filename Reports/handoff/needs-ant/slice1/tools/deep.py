#!/usr/bin/env python3
"""deep.py RUN... -- deep time from colony.csv (one row per ant per 1,000 frames): ants deeper than 10 rows below the
ground (row 160), as a mean N of M per sample, in 60-85k, 100-150k and 100k-300k, plus how many of the deep were
off the door column (more than 3 columns from x=256) and "fed and staying" (energy >= 120 J, the walk's hunger onset
at 200 J x 0.6, and deep at the sample 1,000 frames earlier too)."""
import csv, sys, collections
WIN = [(60000, 85000), (100000, 150000), (100000, 300000)]
for run in sys.argv[1:]:
    deep = collections.defaultdict(dict); allc = collections.Counter()
    with open(run + '/colony.csv') as fh:
        r = csv.reader(fh); h = next(r); ix = {k: j for j, k in enumerate(h)}
        for row in r:
            if len(row) != len(h): continue
            f = int(row[0])
            if f < 59000: continue
            allc[f] += 1
            y = int(row[ix['hy']]); x = int(row[ix['hx']])
            if y - 160 > 10:
                deep[f][row[1]] = (abs(x - 256) > 3, float(row[ix['energy_j']]))
    out = []
    for a, b in WIN:
        fs = [f for f in allc if a <= f < b]
        if not fs:
            out.append(f'{a//1000}-{b//1000}k: -'); continue
        n = sum(len(deep[f]) for f in fs) / len(fs); m = sum(allc[f] for f in fs) / len(fs)
        off = sum(sum(1 for v in deep[f].values() if v[0]) for f in fs) / len(fs)
        stay = sum(sum(1 for i, v in deep[f].items() if v[1] >= 120 and i in deep.get(f - 1000, {})) for f in fs) / len(fs)
        out.append(f'{a//1000}-{b//1000}k: {n:.1f} of {m:.0f} ({100*n/max(m,1):.2f}%), off door col {off:.1f}, fed+staying {stay:.1f}')
    print(f"{run.split('/')[-1]:7s} " + ' | '.join(out))
