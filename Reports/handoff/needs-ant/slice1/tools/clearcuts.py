#!/usr/bin/env python3
"""clearcuts.py RUN LO HI -- every door cut (walk cut code 4) and escape cut (2) in (LO,HI]: where the ant stood, the cell
it cut (from the digging record: the ground->open change by that ant within 2 frames), and what refilled that cell next
(frame, cause, ant) -- or whether it was still open at HI."""
import sys, gzip, csv, collections
run, lo, hi = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
cuts = []
with gzip.open(run + '/walk.csv.gz', 'rt') as fh:
    r = csv.reader(fh); h = next(r); ix = {k: j for j, k in enumerate(h)}
    for row in r:
        f = int(row[0])
        if f <= lo: continue
        if f > hi: break
        if row[ix['cut']] in ('2', '4'):
            cuts.append((f, row[1], int(row[ix['ax']]), int(row[ix['ay']]), row[ix['cut']], row[ix['drive']], float(row[ix['energy']])))
cells = collections.defaultdict(list)
with gzip.open(run + '/cells.csv.gz', 'rt') as fh:
    r = csv.reader(fh); next(r)
    for row in r:
        f = int(row[0])
        if f <= lo - 10: continue
        if f > hi + 5000: break
        cells[(int(row[1]), int(row[2]))].append((f, row[3], row[4], row[5], row[6]))
summary = collections.Counter()
for f, i, ax, ay, code, drive, e in cuts:
    # the cell this ant cut: a change by this ant, cause cut, within a few frames, next to its head
    cand = [(xy, ev) for xy, evs in cells.items() if abs(xy[0] - ax) <= 1 and abs(xy[1] - ay) <= 1
            for ev in evs if f - 2 <= ev[0] <= f + 2 and ev[4] == i]
    if not cand:
        cand = [(xy, ev) for xy, evs in cells.items() if abs(xy[0] - ax) <= 1 and abs(xy[1] - ay) <= 1
                for ev in evs if f - 2 <= ev[0] <= f + 2 and ev[2] in ('empty', 'ant') and ev[1] not in ('empty', 'ant')]
    if not cand:
        summary[(code, 'no cut logged')] += 1
        print(f'{f} code {code} ant {i} at ({ax-256:+d},{ay-160:+d}) {drive} e{e:.0f}: no ground change logged beside it')
        continue
    (x, y), ev = cand[0]
    after = [q for q in cells[(x, y)] if q[0] > ev[0] and q[2] not in ('empty', 'ant')]
    if after:
        q = after[0]
        how = f'refilled after {q[0]-ev[0]} frames by {q[3] or "a fall"}{" ant " + q[4] if q[4] else ""}'
        summary[(code, 'refilled ' + (q[3] or 'fall') + (' <500' if q[0] - ev[0] < 500 else ' >=500'))] += 1
    else:
        how = 'still open at the end'
        summary[(code, 'stayed open')] += 1
    print(f'{f} code {code} ant {i} at ({ax-256:+d},{ay-160:+d}) {drive} e{e:.0f}: cut ({x-256:+d},{y-160:+d}) {ev[1]}>{ev[2]} cause {ev[3] or "-"}; {how}')
print('SUMMARY', sorted(summary.items()))
