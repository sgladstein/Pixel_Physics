#!/usr/bin/env python3
"""perfix.py RUN LO HI -- each fix's own behaviour from the walk trace (sampled ants):
pace: forage-job decisions outside (above the ground, off the mound) that won the roll / moved, empty crop only;
give_up: forage->idle job changes outside while empty, and where that ant is ~2,000 frames later;
lay_home: home-drive decisions on the forage job, and where;
won_stall: escape decisions, hunger at the first escape decision of each spell."""
import sys, gzip, csv, collections
run, lo, hi = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
def place(ax, ay):
    if ay >= 0: return 'deep' if ay > 10 else 'nest'
    if abs(ax) <= 25 and ay >= -20: return 'mound'
    return 'outside'
pace = collections.Counter(); lay = collections.Counter(); esc_h = []
prev = {}; gave = []; track = collections.defaultdict(list)
with gzip.open(run + '/walk.csv.gz', 'rt') as fh:
    r = csv.reader(fh); h = next(r); ix = {k: j for j, k in enumerate(h)}
    for row in r:
        f = int(row[0])
        if f < lo: continue
        if f > hi + 2500: break
        i = row[1]; d = row[ix['drive']]; job = row[ix['job']]; crop = float(row[ix['crop']])
        ax, ay = int(row[ix['ax']]) - 256, int(row[ix['ay']]) - 160; pl = place(ax, ay)
        track[i].append((f, pl))
        if f <= hi:
            if job == 'forage' and d == 'forage' and pl == 'outside' and crop <= 0:
                pace[('won', row[ix['won']])] += 1; pace[('moved', row[ix['moved']])] += 1; pace['n'] += 1
            if d == 'home' and job == 'forage':
                lay[pl] += 1
            p = prev.get(i)
            if p and p[1] == 'forage' and job == 'idle' and pl == 'outside' and crop <= 0 and d == 'home':
                gave.append((i, f))
            if d == 'escape' and (not p or p[0] != 'escape'):
                esc_h.append(float(row[ix['hunger']]))
        prev[i] = (d, job)
later = collections.Counter()
for i, f in gave:
    t = [x for x in track[i] if x[0] >= f + 2000]
    later[t[0][1] if t else 'no row (dead or untraced)'] += 1
n = max(pace['n'], 1)
print(f"{run} {lo//1000}-{hi//1000}k | pace: empty forage-drive decisions outside {pace['n']}, won {100*pace[('won','1')]/n:.0f}%, moved {100*pace[('moved','1')]/n:.0f}%"
      f" | give-ups outside {len(gave)}, 2,000 frames later: {dict(later)} | lay walks by place {dict(lay)}"
      f" | escape spells {len(esc_h)}, hunger at onset median {sorted(esc_h)[len(esc_h)//2] if esc_h else '-'}, under 0.5: {sum(1 for x in esc_h if x < 0.5)}")
