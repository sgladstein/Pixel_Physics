#!/usr/bin/env python3
"""Do foragers keep foraging? Over `trailfollow decisioncsv` traces: per ant
that completed a loop, the frames from one delivery to the next, the time spent
at home before setting out again (with energy and chance to step while
waiting), and how long it lives after its LAST loop and how fed it is then.

Built 2026-09-27 for the owner's "are ants foraging too slowly, or is the
economy too hard?" (`Reports/lanes/foraging-loop.md`, live question): on the
90-cell bed a looper made 1.5 loops and then lived a median 11,712 frames at
full energy without going out again.

    python3 scripts/antidle.py '/tmp/trailfollow-decisions-seed*-gap90-self-TAG.csv'

A loop is booked as antloop books it: a delivery after having been at the food
(within 10 cells of nest_x + 90) since the last one. GAP is 90; edit it for
another bed.
"""
import csv, glob, sys, collections as C, statistics as st
GAP=90; NEAR=10; BAND=26
q=lambda xs,p: sorted(xs)[min(len(xs)-1,int(p*len(xs)))] if xs else float('nan')
loop_len=[]; wait=[]; wait_e=[]; wait_pm=[]; after_last=[]; after_last_e=[]; n_loopers=0; loops=0; alive_end=0
for path in sorted(glob.glob(sys.argv[1])):
    by=C.defaultdict(list)
    for r in csv.DictReader(open(path)):
        if int(r['id'])<1048576: by[r['id']].append(r)
    for aid, rs in by.items():
        rs.sort(key=lambda r:int(r['frame'])); nx=int(rs[0]['nest_x'])
        # loop completions: delivered after having been at the food since the last completion
        been=False; done=[]; start=None
        for i,r in enumerate(rs):
            dx=int(r['x2'])-nx
            if abs(dx-GAP)<=NEAR: been=True
            if been and r['drop']=='delivered':
                done.append(i); been=False
        if not done: continue
        n_loopers+=1; loops+=len(done)
        for a,b in zip(done, done[1:]):
            loop_len.append(int(rs[b]['frame'])-int(rs[a]['frame']))
            # time at home after delivery a before next reaching the food
            reach=next(i for i in range(a,b) if abs(int(rs[i]['x2'])-nx-GAP)<=NEAR)
            dep=max(i for i in range(a,reach) if abs(int(rs[i]['x2'])-nx)<=BAND)
            wait.append(int(rs[dep]['frame'])-int(rs[a]['frame']))
            seg=rs[a:dep+1]
            wait_e.append(st.median(float(r['energy']) for r in seg))
            pm=[float(r['p_move']) for r in seg if r['p_move'] not in ('NaN','')]
            if pm: wait_pm.append(st.median(pm))
        last=done[-1]
        end=int(rs[-1]['frame'])
        after_last.append(end-int(rs[last]['frame']))
        after_last_e.append(st.median(float(r['energy']) for r in rs[last:]))
        alive_end += end>23000
print(f"ants that completed a loop: {n_loopers}; loops {loops}; loops per looper {loops/n_loopers:.2f}")
print(f"frames from one delivery to the next (same ant): median {q(loop_len,.5)} (p25 {q(loop_len,.25)}, p75 {q(loop_len,.75)}), n {len(loop_len)}")
print(f"   of which waiting at home before setting out again: median {q(wait,.5)} (p25 {q(wait,.25)}, p75 {q(wait,.75)})")
print(f"   energy while waiting (fraction of start): median {q(wait_e,.5):.2f}; chance to step per decision while waiting: median {q(wait_pm,.5):.2f}")
print(f"after its LAST loop an ant lives a median {q(after_last,.5)} more frames (p25 {q(after_last,.25)}, p75 {q(after_last,.75)}); median energy then {q(after_last_e,.5):.2f}; alive at the end {alive_end} of {n_loopers}")
