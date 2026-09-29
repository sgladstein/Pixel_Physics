"""What the `returns` forage drive books, one return at a time.

Every booking (`trip_load` 1 -> 0 at a put-down at home) in a `trailfollow decisioncsv` trace: where the pickup that
set `trip_load` happened (the pile, the road, the home band within 26 cells of the nest, or west / past the pile), the
excursion depth then (`forage_max`), how long the mark stood, and whether the crop went empty while it stood (a stale
mark). Bucketed by phase of a refill cycle, so a pulsed pile (`food=30 refill=6000`) shows what keeps the drive up
once the pile is gone. Built 2026-09-29 (`Reports/ant-scenes-2026-09-23.md` section 22t): 46% of returns on the
granary were home-band pickups after a 16-cell roam.

Needs traces with the `trip_load` and `forage_max` columns (trailfollow from 2026-09-29).
usage: python3 scripts/tripsrc.py <csv dir> <gap> <refill cycle, frames>"""
import csv, glob, sys, collections as C, statistics as st
D, GAP, CYC = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]); BAND, NEAR = 26, 10
B = 6; W = CYC / B
tab = C.Counter(); held = C.defaultdict(list); depth = C.defaultdict(list); n = 0; ate_between = C.Counter()
def where(x, nx):
    d = x - nx
    return 'pile' if abs(x - (nx + GAP)) <= NEAR else ('home band' if abs(d) <= BAND else ('road' if 0 < d < GAP else 'west/past'))
for path in sorted(glob.glob(D + '/*.csv')):
    prev = {}; mark = {}
    with open(path) as fh:
        rd = csv.reader(fh); h = next(rd); ix = {k: i for i, k in enumerate(h)}
        for r in rd:
            f = int(r[ix['frame']]); a = r[ix['id']]; x = int(r[ix['x']]); nx = int(r[ix['nest_x']])
            t = r[ix['trip_load']] == '1'; leg = r[ix['leg']]; fm = int(r[ix['forage_max']])
            p = prev.get(a)
            if t and not (p and p[0]):
                mark[a] = [f, where(x, nx), fm, False]
            if t and a in mark and f > mark[a][0] and leg != 'laden': mark[a][3] = True   # crop emptied while still marked (rows after the marking one; `leg` is read before act)
            if p and p[0] and not t and a in mark:
                f0, src, fm0, emptied = mark.pop(a); n += 1
                ph = int((f % CYC) // W) if f >= CYC else -1
                tab[(ph, src)] += 1; held[src].append(f - f0); depth[src].append(fm0)
                if emptied: ate_between[src] += 1
            prev[a] = (t,)
srcs = ['pile', 'road', 'home band', 'west/past']
print(f'{n} returns booked. By where the pickup that marked the trip happened, and frames after a refill (cycles 2+; -1 = first cycle):')
print('  phase        ' + ''.join(f'{s:>11}' for s in srcs))
for b in [-1] + list(range(B)):
    lab = 'first cycle' if b < 0 else f'{int(b*W):5d}-{int((b+1)*W):5d}'
    print(f'  {lab:11}  ' + ''.join(f'{tab[(b, s)]:>11}' for s in srcs))
for s in srcs:
    if held[s]:
        print(f'  {s:10}: {len(held[s])} returns; mark stood median {st.median(held[s])} frames (p90 {sorted(held[s])[int(.9*len(held[s]))]}); excursion depth at the marking pickup median {st.median(depth[s])} cells; crop went empty while marked on {ate_between[s]}')
