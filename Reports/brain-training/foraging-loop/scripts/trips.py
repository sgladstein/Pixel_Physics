# Per-load economy for one inrun run DIR (needs trace=N). A load = an ant's trip_cells going 0 -> >0 -> 0.
# Per load: cells picked up in the field, cells delivered at the nest, frames from the last field pickup to the load's end,
# distance from the nest of the last pickup, and how far the ant was from the nest when the load ended.
import csv, sys, collections, statistics as st
D = sys.argv[1]; NEST = int(sys.argv[2]) if len(sys.argv) > 2 else None
ev = collections.defaultdict(list)
for r in csv.DictReader(open(f"{D}/events.csv")):
    if r['kind'] in ('field', 'deliver', 'churn', 'homeeat'):
        ev[int(r['id'])].append((int(r['frame']), r['kind'], int(r['x'])))
tr = collections.defaultdict(list)
for r in csv.DictReader(open(f"{D}/trace.csv")):
    tr[int(r['id'])].append((int(r['frame']), int(r['x']), int(r['trip']), int(r['crop']), float(r['tank'])))
if NEST is None:
    xs = [x for v in ev.values() for f, k, x in v if k == 'deliver']
    NEST = int(st.median(xs)) if xs else 0
loads = []
for i, rows in tr.items():
    e = sorted(ev.get(i, []))
    start = None
    for (f0, x0, t0, c0, k0), (f1, x1, t1, c1, k1) in zip(rows, rows[1:]):
        if start is None and t1 > 0:
            start = (f0, k1)
        if start is not None and t1 == 0:
            a, b = start[0], f1
            fe = [z for z in e if a < z[0] <= b]
            picks = [z for z in fe if z[1] == 'field']
            dels = [z for z in fe if z[1] == 'deliver']
            if picks:
                last = picks[-1]
                end = dels[0][0] if dels else b
                loads.append(dict(id=i, n=len(picks), d=len(dels), walk=end - last[0], dist=abs(last[2] - NEST),
                                  endx=abs(x1 - NEST), tank=start[1], span=last[0] - picks[0][0]))
            start = None
def q(v, p): v = sorted(v); return v[min(len(v) - 1, int(p * len(v)))] if v else 0
n = len(loads)
print(f"{D}: nest x {NEST}, loads {n}")
if not n: sys.exit()
tot_p = sum(l['n'] for l in loads); tot_d = sum(l['d'] for l in loads)
print(f"  cells per load: mean {tot_p/n:.2f}  p50 {q([l['n'] for l in loads],.5)}  p90 {q([l['n'] for l in loads],.9)};  share of loads of 1 cell {sum(l['n']==1 for l in loads)/n:.0%}")
print(f"  delivered: {tot_d} of {tot_p} cells ({tot_d/max(1,tot_p):.1%});  loads delivering anything {sum(l['d']>0 for l in loads)/n:.0%}")
w = [l['walk'] for l in loads]; wd = [l['walk'] for l in loads if l['d'] > 0]; wn = [l['walk'] for l in loads if l['d'] == 0]
print(f"  frames from last pickup to delivery/empty: p25 {q(w,.25)} p50 {q(w,.5)} p75 {q(w,.75)} p90 {q(w,.9)}")
print(f"     delivering loads p50 {q(wd,.5)}   loads eaten on the road p50 {q(wn,.5)}")
print(f"  pickup distance from nest p50 {q([l['dist'] for l in loads],.5)}  where an eaten load ran out, distance from nest p50 {q([l['endx'] for l in loads if l['d']==0],.5)}")
print(f"  filling (first to last pickup) frames p50 {q([l['span'] for l in loads],.5)}")
for lo, hi in [(0, 1), (1, 3), (3, 5.5), (5.5, 20), (20, 1e9)]:
    s = [l for l in loads if lo <= l['tank'] < hi]
    if s:
        print(f"  tank {lo}-{hi if hi<1e9 else 'up':>4}: loads {len(s):5d}  cells/load {sum(l['n'] for l in s)/len(s):.2f}  delivered {sum(l['d'] for l in s)/max(1,sum(l['n'] for l in s)):.1%}  walk p50 {q([l['walk'] for l in s],.5)}")
