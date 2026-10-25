"""Per-ant summary of replay trace.csv: one row per ant, then per-drop tallies.

usage: python3 -I ants.py trace.csv [start_energy]
"""
import csv, sys, collections, statistics

rows = collections.defaultdict(list)
for r in csv.DictReader(open(sys.argv[1])):
    rows[int(r['id'])].append(r)
SE = None

ants = []
for aid, rs in rows.items():
    f0, f1 = int(rs[0]['frame']), int(rs[-1]['frame'])
    site = int(rs[0]['site_x'])
    e0 = float(rs[0]['energy'])
    b0, b1 = int(rs[0]['bites']), int(rs[-1]['bites'])
    n = len(rs)
    home = sum(1 for r in rs if abs(int(r['x']) - site) <= 8)
    under = sum(1 for r in rs if int(r['h']) < -1)
    nearplant = sum(1 for r in rs if int(r['plant_d']) <= 3)
    first_plant = next((int(r['frame']) - f0 for r in rs if int(r['plant_d']) <= 3), None)
    maxd = max(abs(int(r['x']) - site) for r in rs)
    # returns home after first reaching a plant
    trips, at_home = 0, True
    for r in rs:
        h = abs(int(r['x']) - site) <= 8
        if h and not at_home:
            trips += 1
        at_home = h
    laden_home = sum(1 for r in rs if abs(int(r['x']) - site) <= 8 and r['crop'])
    ants.append(dict(id=aid, colony=rs[0]['colony'], site=site, born=f0, last=f1, life=f1 - f0, e0=e0,
                     bites=b1 - b0, home=home / n, under=under / n, nearplant=nearplant / n, first_plant=first_plant,
                     maxd=maxd, returns=trips, laden_home=laden_home, endx=int(rs[-1]['x']) - site,
                     end_e=float(rs[-1]['energy'])))

by = collections.defaultdict(list)
for a in ants:
    by[a['site']].append(a)
print("site  ants  med_life  reached_plant  med_frames_to_plant  med_bites  ate>0  ever_home_after_out  med_maxd  frac_time: home/under/nearplant")
for s, L in sorted(by.items(), key=lambda kv: min(a['born'] for a in kv[1])):
    rp = [a for a in L if a['first_plant'] is not None]
    print(s, len(L), statistics.median(a['life'] for a in L), len(rp),
          statistics.median(a['first_plant'] for a in rp) if rp else '-',
          statistics.median(a['bites'] for a in L), sum(1 for a in L if a['bites'] > 0),
          sum(1 for a in L if a['returns'] > 0), statistics.median(a['maxd'] for a in L),
          '%.2f/%.2f/%.2f' % (statistics.mean(a['home'] for a in L), statistics.mean(a['under'] for a in L),
                              statistics.mean(a['nearplant'] for a in L)))
if len(sys.argv) > 3:
    for a in sorted(ants, key=lambda a: -a['life'])[: int(sys.argv[3])]:
        print(a)
