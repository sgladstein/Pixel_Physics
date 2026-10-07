#!/usr/bin/env python3
"""carryfate.py RUN LO HI -- for carry spells starting in [LO,HI): over the spell, the crop the ant lost, and how much
of that loss came back as its own energy (eaten) versus left it (put down or given away). Energy is net of upkeep,
so eaten is estimated as energy gained plus upkeep at the walk's measured burn (the ant's own decline rate when its
crop is empty is not known here; we report raw energy change)."""
import sys, gzip, csv, collections
run, lo, hi = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
cur = {}
out = []
with gzip.open(run + '/walk.csv.gz', 'rt') as fh:
    r = csv.reader(fh); h = next(r); ix = {k: j for j, k in enumerate(h)}
    for row in r:
        f = int(row[0])
        if f < lo: continue
        if f > hi + 20000: break
        i = row[1]; d = row[ix['drive']]; crop = float(row[ix['crop']]); e = float(row[ix['energy']])
        ax, ay = int(row[ix['ax']]) - 256, int(row[ix['ay']]) - 160
        if d == 'carry':
            if i not in cur:
                if f < hi:
                    cur[i] = {'f0': f, 'c0': crop, 'e0': e, 'drops': 0.0, 'gain': 0.0, 'last': (crop, e), 'door': 0, 'n': 0}
            else:
                s = cur[i]; pc, pe = s['last']
                dc = pc - crop; de = e - pe
                if dc > 0:
                    if de > 0.5 * dc: s['gain'] += dc   # crop fell and energy rose with it: eaten
                    else: s['drops'] += dc             # crop fell, energy did not: put down or given
                s['last'] = (crop, e); s['n'] += 1
                if abs(ax) <= 6 and -13 <= ay <= 2: s['door'] += 1
                s['f1'] = f
        elif i in cur:
            out.append(cur.pop(i))
tot_e = sum(s['gain'] for s in out); tot_d = sum(s['drops'] for s in out); tot_c0 = sum(s['c0'] for s in out)
print(f"{run}: {len(out)} finished carry spells; crop at start {tot_c0:.0f} J; lost to eating {tot_e:.0f} J ({100*tot_e/max(tot_c0,1):.0f}%), put down or given {tot_d:.0f} J ({100*tot_d/max(tot_c0,1):.0f}%)")
