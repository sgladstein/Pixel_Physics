"""SCRATCH (nest lane, 2026-10-27): ant-by-ant readers over a trace.sh run (PP_MIND_LOG + PP_HUNGRYPROBE + events).

  haul    soil pellet carriers below the datum: every decision's verb; for a Walk, up / level / down; whether a step
          up was offered; each pellet from first carried decision to release (decisions, depth start/max/end, where it
          went); and the full rows (inputs and the pick) of a few pellets.
  hungry  ants under a quarter tank in the nest bowl (12 columns of the door, below the datum) with food in reach:
          was a pick-up offered, what did they pick, and the score of the best pick-up against the pick.
  drops   crop food put down (Drop with a crop, jaws empty): where (nest bowl / door ring / field), by the ant's tank.
usage: tracemind.py RUNDIR(the trace.sh OUT) SEED [haul|hungry|drops|all]
M row: M,seed,frame,id,x,y,verb,tx,ty,last,ctx(22),mem(12),jobs,best-per-verb(9),chosen feats(45)
"""
import gzip, sys
from collections import Counter, defaultdict

D, SEED = sys.argv[1], sys.argv[2]
WHAT = sys.argv[3] if len(sys.argv) > 3 else 'all'
VERB = ['walk', 'turn', 'rest', 'dig', 'pickup', 'drop', 'eat', 'feed', 'attack']
CTX = ['energy', 'crop', 'spoil', 'at_nest', 'a', 'b', 'alarm', 'd_a', 'd_b', 'd_alarm', 'd_energy', 'home_seen',
       'home_dist', 'd_home', 'since_nest', 'age', 'self', 'crowd', 'depth', 'roofed', 'roof_cut', 'hole_drop']
NEST = 128
surf = [int(l) for l in open(f'{D}/w{SEED}/surface.txt') if l.strip()]


def rows():
    with gzip.open(f'{D}/mind.log.gz', 'rt') as f:
        for l in f:
            if l[0] != 'M':
                continue
            p = l.rstrip('\n').split(',')
            ctx = [float(x) for x in p[10].split()]
            best = [float(x) for x in p[13].split()]
            yield dict(frame=int(p[2]), id=int(p[3]), x=int(p[4]), y=int(p[5]), verb=int(p[6]), tx=int(p[7]),
                       ty=int(p[8]), ctx=ctx, best=best, jobs=p[12])


def below(x, y):
    return 0 <= x < len(surf) and y > surf[x]


def bowl(x, y):
    return abs(x - NEST) <= 12 and y >= surf[x]


haul = dict(verbs=Counter(), walk=Counter(), up_offered=0, n=0)
pellets, cur, keep = [], {}, defaultdict(list)
hungry = dict(n=0, offered=0, picks=Counter(), picks_offered=Counter(), margin=[])
hsample = []
drops = Counter()
energy_of = lambda r: r['ctx'][0]
for r in rows():
    sp = r['ctx'][2] > 0
    i = r['id']
    if sp and below(r['x'], r['y']):
        haul['n'] += 1
        haul['verbs'][VERB[r['verb']]] += 1
        if r['verb'] == 0:
            haul['walk']['down' if r['ty'] > r['y'] else 'up' if r['ty'] < r['y'] else 'level'] += 1
        c = cur.setdefault(i, dict(id=i, f0=r['frame'], d0=r['y'] - surf[r['x']], dmax=0, n=0, x=r['x'], y=r['y']))
        c['n'] += 1
        c['dmax'] = max(c['dmax'], r['y'] - surf[r['x']])
        c['x'], c['y'], c['f1'] = r['x'], r['y'], r['frame']
        if len(pellets) < 6 and len(keep[i]) < 400:
            keep[i].append(r)
    elif not sp and i in cur:
        c = cur.pop(i)
        c['end'] = 'above' if not below(c['x'], c['y']) else f"in@{c['y'] - surf[c['x']]}"
        c['why'] = VERB[r['verb']]
        pellets.append(c)
    if sp and not below(r['x'], r['y']) and i in cur:
        c = cur.pop(i)
        c['end'] = 'carried out'
        pellets.append(c)
    e = energy_of(r) / max(1e-6, 1 - energy_of(r))  # tanks
    if e < 0.25 and bowl(r['x'], r['y']) and r['ctx'][1] == 0 and not sp:
        hungry['n'] += 1
        off = r['best'][4] == r['best'][4]  # not NaN: a pick-up offered
        hungry['offered'] += off
        hungry['picks'][VERB[r['verb']]] += 1
        if off:
            hungry['picks_offered'][VERB[r['verb']]] += 1
            hungry['margin'].append(r['best'][4] - r['best'][r['verb']])
            if len(hsample) < 40:
                hsample.append(r)
    if r['verb'] == 5 and r['ctx'][1] > 0 and not sp:
        where = 'bowl' if bowl(r['tx'], r['ty']) else 'field'
        drops[(where, 'hungry' if e < 0.25 else 'fed')] += 1

if WHAT in ('haul', 'all'):
    print(f"== haul: {haul['n']} decisions by pellet carriers below the datum")
    print('  verbs', dict(haul['verbs'].most_common()))
    print('  walks', dict(haul['walk']))
    import statistics as st
    done = [p for p in pellets if 'end' in p]
    print(f'  pellets released or carried out: {len(done)}; ends', Counter(p['end'] if not p['end'].startswith('in@') else 'in' for p in done))
    if done:
        print('  decisions per pellet: median', st.median(p['n'] for p in done), 'p90', sorted(p['n'] for p in done)[int(.9 * len(done))])
        print('  max depth per pellet: median', st.median(p['dmax'] for p in done), ' start depth median', st.median(p['d0'] for p in done))
        ins = [int(p['end'][3:]) for p in done if p['end'].startswith('in@')]
        if ins:
            print('  released below the datum at depth: median', st.median(ins), 'share 0-1 rows', sum(d <= 1 for d in ins) / len(ins))
    for i, rs in list(keep.items())[:3]:
        print(f'  -- ant {i}: first {len(rs)} carried decisions (frame x y depth verb -> target | energy crop depth roofed home_dist d_home crowd)')
        for r in rs[:60]:
            c = r['ctx']
            print(f"   {r['frame']} {r['x']},{r['y']} d{r['y'] - surf[r['x']]} {VERB[r['verb']]}->{r['tx']},{r['ty']} | "
                  f"{c[0]:.2f} {c[1]:.2f} {c[18]:.2f} {c[19]:.0f} {c[12]:.2f} {c[13]:.2f} {c[17]:.2f}")
if WHAT in ('hungry', 'all'):
    print(f"== hungry at home, crop empty, jaws free: {hungry['n']} decisions; a pick-up offered in {hungry['offered']}")
    print('  picks (all)', dict(hungry['picks'].most_common()))
    print('  picks when a pick-up was offered', dict(hungry['picks_offered'].most_common()))
    if hungry['margin']:
        m = sorted(hungry['margin'])
        print(f'  best pick-up score minus the pick\'s: median {m[len(m) // 2]:.3f}, p10 {m[len(m) // 10]:.3f}, p90 {m[9 * len(m) // 10]:.3f}')
    for r in hsample[:25]:
        b = ' '.join('-' if x != x else f'{x:.2f}' for x in r['best'])
        print(f"   {r['frame']} ant {r['id']} {r['x']},{r['y']} e{r['ctx'][0]:.2f} pick {VERB[r['verb']]} | best/verb {b}")
if WHAT in ('drops', 'all'):
    print('== crop food put down (Drop, crop held, jaws empty):', dict(drops))
