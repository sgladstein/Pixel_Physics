#!/usr/bin/env python3
"""Do foragers keep foraging? Over `trailfollow decisioncsv` traces: per ant
that completed a loop, the frames from one delivery to the next, the time spent
at home before setting out again (with energy and chance to step while
waiting), and how long it lives after its LAST loop and how fed it is then.

Built 2026-09-27 for the owner's "are ants foraging too slowly, or is the
economy too hard?" (`Reports/lanes/foraging-loop.md`, live question): on the
90-cell bed a looper made 1.5 loops and then lived a median 11,712 frames at
full energy without going out again.

    python3 scripts/antidle.py '/tmp/trailfollow-decisions-seed*-gap90-self-TAG.csv' [gap]
    python3 scripts/antidle.py '<arm glob>' [gap] --vs '<base glob>'

`--vs` pairs the arm against a baseline seed by seed, with a sign test, on the
lane's target quantities: loops per looper, loops, and the share of a looper's
life after its first delivery spent fed at home (and how much of that it holds
food off the nest). Keyed by seed from the file name; the gap in the name must
match. `antloop.py --vs` pairs the checks (starved, net food into home).

A loop is booked as antloop books it: a delivery after having been at the food
(within 10 cells of nest_x + gap) since the last one. The gap defaults to 90 and
must be given for any other bed: at 140 a 90-cell test books false loops.
"Alive at the end" is judged against the last frame any ant in the file decided.

Also prints, where the trace carries `energy_j` (2026-09-27 on), how a looper
spends the rest of its life after its FIRST delivery: fed (at or above the
200 J grant) or hungry, at home (within 26 cells of the nest centre) or out,
and how much of its fed time at home it is holding food -- which, at home, is
food it took off the nest.
"""
import csv, glob, sys, re, math, collections as C, statistics as st
args=sys.argv[1:]
VS=None
if '--vs' in args:
    i=args.index('--vs'); VS=args[i+1]; del args[i:i+2]
GAP=int(args[1]) if len(args)>1 else 90; NEAR=10; BAND=26; START=200.0
q=lambda xs,p: sorted(xs)[min(len(xs)-1,int(p*len(xs)))] if xs else float('nan')
loop_len=[]; wait=[]; wait_e=[]; wait_pm=[]; after_last=[]; after_last_e=[]; n_loopers=0; loops=0; alive_end=0
budget=C.Counter(); held=C.Counter(); has_j=False
for path in sorted(glob.glob(args[0])):
    by=C.defaultdict(list)
    last_frame=0
    for r in csv.DictReader(open(path)):
        last_frame=max(last_frame,int(r['frame']))
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
        alive_end += end>last_frame-1000
        if 'energy_j' in rs[0]:
            has_j=True
            tail=rs[done[0]:]
            for a,b in zip(tail,tail[1:]):
                dt=int(b['frame'])-int(a['frame']); dx=int(a['x2'])-nx
                k=('fed' if float(a['energy_j'])>=START else 'hungry')+(' at home' if abs(dx)<=BAND else ' out')
                budget[k]+=dt
                if a['leg']=='laden': held[k]+=dt
print(f"ants that completed a loop: {n_loopers}; loops {loops}; loops per looper {loops/n_loopers:.2f}")
print(f"frames from one delivery to the next (same ant): median {q(loop_len,.5)} (p25 {q(loop_len,.25)}, p75 {q(loop_len,.75)}), n {len(loop_len)}")
print(f"   of which waiting at home before setting out again: median {q(wait,.5)} (p25 {q(wait,.25)}, p75 {q(wait,.75)})")
print(f"   energy while waiting (fraction of start): median {q(wait_e,.5):.2f}; chance to step per decision while waiting: median {q(wait_pm,.5):.2f}")
print(f"after its LAST loop an ant lives a median {q(after_last,.5)} more frames (p25 {q(after_last,.25)}, p75 {q(after_last,.75)}); median energy then {q(after_last_e,.5):.2f}; alive at the end {alive_end} of {n_loopers}")
if has_j:
    T=sum(budget.values())
    print(f"after its FIRST delivery a looper's life ({T} frames pooled): " + ", ".join(f"{k} {budget[k]/T:.1%}" for k in ['fed at home','hungry at home','fed out','hungry out']))
    print(f"   holding food while fed at home: {held['fed at home']/max(1,budget['fed at home']):.1%} of that time (at home, food taken off the nest)")


def per_seed(pattern):
    """The target quantities per seed, keyed on the seed in the file name."""
    out={}
    for path in sorted(glob.glob(pattern)):
        m=re.search(r'seed(\d+)-gap(\d+)', path)
        assert m and int(m.group(2))==GAP, f"{path}: not a gap-{GAP} trace"
        by=C.defaultdict(list)
        for r in csv.DictReader(open(path)):
            if int(r['id'])<1048576: by[r['id']].append(r)
        loopers=loops=0; fed_home=total=held=0
        for rs in by.values():
            rs.sort(key=lambda r:int(r['frame'])); nx=int(rs[0]['nest_x'])
            been=False; done=[]
            for i,r in enumerate(rs):
                if abs(int(r['x2'])-nx-GAP)<=NEAR: been=True
                if been and r['drop']=='delivered': done.append(i); been=False
            if not done: continue
            loopers+=1; loops+=len(done)
            tail=rs[done[0]:]
            for a,b in zip(tail,tail[1:]):
                dt=int(b['frame'])-int(a['frame']); total+=dt
                if 'energy_j' in a and float(a['energy_j'])>=START and abs(int(a['x2'])-nx)<=BAND:
                    fed_home+=dt
                    if a['leg']=='laden': held+=dt
        out[int(m.group(1))]=dict(lpl=loops/loopers if loopers else 0.0, loops=loops,
                                  fedhome=fed_home/total if total else 0.0, held=held/fed_home if fed_home else 0.0)
    return out


def sign_p(better, worse):
    n=better+worse
    if n==0: return 1.0
    k=min(better,worse)
    return min(1.0, 2*sum(math.comb(n,i) for i in range(k+1))/2**n)


if VS:
    base=per_seed(VS); arm=per_seed(args[0]); seeds=sorted(set(base)&set(arm))
    print(f"\nPAIRED AGAINST {VS}: seeds keyed base {len(base)}, arm {len(arm)}, paired {len(seeds)}")
    for key,label,higher in [('lpl','loops per looper (median over seeds)',True),('loops','full loops (sum)',True),
                             ('fedhome','life after 1st delivery fed at home (median)',False),('held','  ... of it holding food off the nest (median)',False)]:
        a=[base[s][key] for s in seeds]; b=[arm[s][key] for s in seeds]
        better=sum((y>x) if higher else (y<x) for x,y in zip(a,b)); worse=sum((y<x) if higher else (y>x) for x,y in zip(a,b))
        agg=(lambda v: sum(v)) if key=='loops' else (lambda v: st.median(v))
        print(f"  {label:48s} {agg(a):9.3f} -> {agg(b):9.3f}   better on {better}, worse on {worse}, sign p {sign_p(better,worse):.3f}")
