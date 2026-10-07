#!/usr/bin/env python3
"""seal63.py RUN LO HI -- the walk's decisions inside a window, for the ants that starved at its end.
Prints: per-drive decision counts by zone, escape spells, every cut by code with place and drive,
and for the ants that starved in (HI-3000, HI]: their last decisions summarised."""
import sys, gzip, csv, collections, re
run, lo, hi = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
starved = {}
for l in open(run + '/events.txt'):
    if 'STARVED' in l:
        f = int(l.split()[0])
        if hi - 3000 < f <= hi + 2000:
            i = re.search(r' id=(\d+)', l).group(1)
            starved[i] = f
print('starved in window end:', len(starved))
DX, GY = 256, 160
cuts = collections.Counter()
cutplace = collections.defaultdict(collections.Counter)
per = collections.defaultdict(list)
drive_zone = collections.Counter()
esc = collections.Counter()
outrows = collections.Counter()
with gzip.open(run + '/walk.csv.gz', 'rt') as fh:
    r = csv.reader(fh)
    h = next(r)
    ix = {k: j for j, k in enumerate(h)}
    for row in r:
        f = int(row[0])
        if f < lo:
            continue
        if f > hi:
            break
        i = row[1]
        ax, ay = int(row[ix['ax']]), int(row[ix['ay']])
        d = row[ix['drive']]
        c = row[ix['cut']]
        if c != '0':
            cuts[(c, d)] += 1
            cutplace[c][(ax - DX, ay - GY)] += 1
        if d in ('out', 'escape'):
            outrows[(d, row[ix['won']], row[ix['moved']])] += 1
        if i in starved:
            per[i].append((f, ax - DX, ay - GY, d, row[ix['job']], float(row[ix['energy']]), float(row[ix['crop']]),
                           float(row[ix['hunger']]), float(row[ix['p_move']]), row[ix['moved']], int(row[ix['stall']]),
                           row[ix['won']], c))
print('cuts by (code, drive):', sorted(cuts.items()))
for c, pl in cutplace.items():
    print(f' code {c} places (col vs door, row vs ground), top 15:', pl.most_common(15))
print('out/escape rows by (drive, won, moved):', sorted(outrows.items()))
# per starved ant: last 4000 frames summary
summ = collections.Counter()
for i, rows in sorted(per.items(), key=lambda kv: starved[kv[0]]):
    last = [x for x in rows if x[0] > starved[i] - 4000]
    if not last:
        summ['no rows in last 4000'] += 1
        continue
    dm = collections.Counter(x[3] for x in last)
    pos = collections.Counter((x[1], x[2]) for x in last)
    won = sum(1 for x in last if x[11] == '1')
    mv = sum(1 for x in last if x[9] == '1')
    maxstall = max(x[10] for x in last)
    cutn = collections.Counter(x[12] for x in last if x[12] != '0')
    e0 = last[0][5]
    print(f'ant {i} died {starved[i]} rows {len(last)} energy {e0:.0f}->{last[-1][5]:.0f} crop0 {last[0][6]:.0f} '
          f'drives {dict(dm)} won {won} moved {mv} maxstall {maxstall} cuts {dict(cutn)} '
          f'distinct cells {len(pos)} top {pos.most_common(3)} job_end {last[-1][4]}')
