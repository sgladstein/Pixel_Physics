"""Whole-colony time budget from trace rows (every 250 frames, every ant): share holding a soil pellet, by place and by
tank; hungry ant-rows (tank<0.5) holding a pellet; home anchor off the door ring; and the hatch bank.
usage: budget.py RUNDIR..."""
import csv, sys, collections, statistics as st
NEST = 128
for d in sys.argv[1:]:
    surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    n = collections.Counter(); seen = set(); hb = []
    for r in csv.DictReader(open(f'{d}/trace.csv')):
        x, y, t, sp = int(r['x']), int(r['y']), float(r['tank']), int(r['spoil'])
        i = int(r['id'])
        if i not in seen:
            seen.add(i)
            if int(r['frame']) > 250: hb.append(t)
        under = y > surf[x]
        n['rows'] += 1; n['pellet'] += sp; n['under'] += under; n['under+pellet'] += under and sp
        if t < 0.5:
            n['hungry'] += 1; n['hungry+pellet'] += sp; n['hungry+under'] += under
        if 'ax' in r:
            n['anchor off door ring (|dx|>12)'] += abs(int(r['ax']) - NEST) > 12
    R = n['rows']
    print(f"{d:22s} pellet {100*n['pellet']/R:4.1f}% of ant-time (underground {100*n['under+pellet']/max(1,n['under']):4.1f}% of underground time) | "
          f"hungry(<0.5) {100*n['hungry']/R:4.1f}% of ant-time, of which pellet {100*n['hungry+pellet']/max(1,n['hungry']):4.1f}% underground {100*n['hungry+under']/max(1,n['hungry']):4.1f}% | "
          f"anchor off ring {100*n['anchor off door ring (|dx|>12)']/R:4.1f}% | hatch bank med {st.median(hb) if hb else 0:.2f} (n {len(hb)})")
