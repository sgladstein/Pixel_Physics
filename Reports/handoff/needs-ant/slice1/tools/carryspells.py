#!/usr/bin/env python3
"""carryspells.py RUN LO HI [SAMPLE] -- carry spells (consecutive decisions of one ant on drive carry) that START in
[LO,HI): duration, crop at start and at end, how the spell ended (crop dropped in one go = put down; crop worn down by
eating = eaten; still carrying at HI+20000 = open; died), and where the ant was when it ended."""
import sys, gzip, csv, collections
run, lo, hi = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
cur = {}
spells = []
with gzip.open(run + '/walk.csv.gz', 'rt') as fh:
    r = csv.reader(fh); h = next(r); ix = {k: j for j, k in enumerate(h)}
    for row in r:
        f = int(row[0])
        if f < lo: continue
        if f > hi + 20000: break
        i = row[1]; d = row[ix['drive']]; crop = float(row[ix['crop']]); ax, ay = int(row[ix['ax']]) - 256, int(row[ix['ay']]) - 160
        if d == 'carry':
            if i not in cur:
                if f < hi:
                    cur[i] = [f, crop, crop, f, ax, ay, 0.0]
            else:
                s = cur[i]
                s[6] = max(s[6], s[2] - crop)  # biggest one-decision fall
                s[2] = crop; s[3] = f; s[4] = ax; s[5] = ay
        elif i in cur:
            s = cur.pop(i)
            spells.append((i, s[0], s[3], s[1], s[2], crop, s[6], s[4], s[5], d))
for i, s in cur.items():
    spells.append((i, s[0], s[3], s[1], s[2], None, s[6], s[4], s[5], 'open'))
c = collections.Counter(); dur = collections.defaultdict(list)
for i, f0, f1, c0, c1, after, bigfall, ax, ay, nd in spells:
    if after is None:
        k = 'still carrying'
    elif after <= 0.01 and c1 > 0.25 * c0:
        k = 'put down in one go'
    elif c1 <= 0.25 * c0:
        k = 'worn down (eaten or shared) before it ended'
    else:
        k = 'ended holding food (drive changed)'
    place = 'nest' if ay >= 0 else ('door column' if abs(ax) <= 6 and ay >= -13 else ('mound' if abs(ax) <= 25 and ay >= -20 else 'outside'))
    c[(k, place)] += 1; dur[k].append(f1 - f0)
print(run, 'carry spells starting in', lo, hi, ':', len(spells))
for k, v in sorted(c.items(), key=lambda kv: -kv[1]):
    print(f'  {v:5d}  {k[0]:45s} ended at {k[1]}')
for k, v in dur.items():
    v.sort(); print(f'  {k}: median {v[len(v)//2]} frames, p90 {v[int(len(v)*0.9)]}')
