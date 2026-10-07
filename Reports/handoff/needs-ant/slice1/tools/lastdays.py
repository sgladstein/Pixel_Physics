#!/usr/bin/env python3
"""lastdays.py RUN LO HI [SPAN] -- for every ant that starved in (LO,HI]: its last SPAN frames from walk.csv.
Per ant: energy SPAN before death, at half and at 600 before; the frame hunger first passed 0.5; drives in the
last 600 frames; won, moved; cells visited; where it was; job; and how many decisions on the way out had a target."""
import sys, gzip, csv, collections, re
run, lo, hi = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
span = int(sys.argv[4]) if len(sys.argv) > 4 else 3000
st = {}
for l in open(run + '/events.txt'):
    if 'STARVED' in l:
        f = int(l.split()[0])
        if lo < f <= hi:
            st[re.search(r' id=(\d+)', l).group(1)] = (f, l.split('at=')[1].split()[0])
rows = collections.defaultdict(list)
with gzip.open(run + '/walk.csv.gz', 'rt') as fh:
    r = csv.reader(fh); h = next(r); ix = {k: j for j, k in enumerate(h)}
    for row in r:
        f = int(row[0])
        if f < lo - span: continue
        if f > hi: break
        i = row[1]
        if i in st and f > st[i][0] - span:
            rows[i].append((f, row[ix['drive']], row[ix['job']], float(row[ix['energy']]), float(row[ix['hunger']]),
                            row[ix['won']], row[ix['moved']], int(row[ix['ax']]) - 256, int(row[ix['ay']]) - 160,
                            row[ix['target_x']], int(row[ix['stall']]), float(row[ix['crop']])))
agg = collections.Counter()
print(f'{len(st)} starved in ({lo},{hi}]; traced {len(rows)}')
for i, rs in sorted(rows.items(), key=lambda kv: st[kv[0]][0]):
    d = st[i][0]
    def e_at(t):
        b = [x for x in rs if x[0] <= t]
        return b[-1][3] if b else float('nan')
    h50 = next((x[0] for x in rs if x[4] >= 0.5), None)
    last = [x for x in rs if x[0] > d - 600]
    dr = collections.Counter(x[1] for x in last)
    won = sum(x[5] == '1' for x in last); mv = sum(x[6] == '1' for x in last)
    cells = len({(x[7], x[8]) for x in last})
    where = collections.Counter((x[7], x[8]) for x in last).most_common(1)
    jobs = collections.Counter(x[2] for x in last).most_common(1)
    crop = max(x[11] for x in rs)
    print(f' {i:>8} died {d} at {st[i][1]} e(-{span}) {e_at(d - span):6.0f} e(-{span//2}) {e_at(d - span // 2):6.0f} e(-600) {e_at(d - 600):5.0f} '
          f'hunger>=.5 at {d - h50 if h50 else "-":>5} before; last600 drives {dict(dr)} won {won}/{len(last)} moved {mv} cells {cells} '
          f'top {where} job {jobs} maxcrop {crop:.0f}')
