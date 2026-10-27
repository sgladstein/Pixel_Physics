# mkwrite.py POOL OUTDIR [ARMS] : the hand-laid "home had food" memory rebuilt on one write-gated unit (layout 8), and its
# knock-out / ramp arms, one folder of member files each. POOL is the layout-7 latch pool (scratch/inrun/genomes/
# latch_pool.txt): the flip-flop unit mem9 (set by mem8, reset by mem11) is cleared and becomes
#   value  squash(60 reach.PickUp - 10)          ~ +0.83..+0.98 with something to pick up, -0.91 with nothing
#   hold   rate gene 0.495 (rate 0.995)
#   write  y = 96 at_nest + 84 reach.PickUp - 30 energy - 84, z = clamp(0.005 + y, 0, 1)
#          -> opens at the nest when there is food to pick up, or when hungry (energy < ~0.37, the old reset's
#             threshold) and there is none; shut away from the nest and at the nest when fed with nothing to take
# Unit 8's gate keeps reading it as before (gate8.mem9 3, bias -2.74).
import sys, os; sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'latch-landscape-2026-10-27'))
from layout import *
WRITE=CORE; W_IN=CONTEXT+MEMORY+VERBS+1; CORE8=WRITE+MEMORY*W_IN
src, out = sys.argv[1], sys.argv[2]
lines=[l.rstrip('\n') for l in open(src) if not l.startswith('#')]
h=lines[0].split(); n=int(h[3]); assert 'layout=7' in h, h; pool=[]
for l in lines[1:]:
    if l.startswith('member '): pool.append(list(pool[0]) if pool else [0.0]*n); continue
    i,w=l.split(' '); pool[-1][int(i)]=float(w)
slot={name(s):s for s in range(CORE)}
def wslot(m, nm):
    i = CN.index(nm) if nm in CN else (CONTEXT+MEMORY+VN.index(nm[6:]) if nm.startswith('reach.') else W_IN-1)
    return WRITE+m*W_IN+i
def mem_row(m): return [MEM+m*M_IN+i for i in range(M_IN)]+[REACH_MEM+m*VERBS+v for v in range(VERBS)]
FULL={'value': {'mem9.reach.PickUp':60.0,'mem9.bias':-10.0}, 'rate':0.495,
      'write': {'at_nest':96.0,'reach.PickUp':84.0,'energy':-30.0,'bias':-84.0}}
def arm(**kw):
    a={'value':dict(FULL['value']),'rate':FULL['rate'],'write':dict(FULL['write']),'read':None}
    for k,v in kw.items():
        if k in ('rate','read'): a[k]=v
        else: a[k].update(v)
    return a
ARMS={
 'wfull':      arm(),
 'w_nohunger': arm(write={'energy':0.0}),          # writes at every nest visit: "empty" at the door each trip
 'w_nopick':   arm(write={'reach.PickUp':0.0}),    # writes only when hungry at the nest
 'w_noat':     arm(write={'at_nest':0.0}),         # never opens
 'w_half':     arm(write={k:v/2 for k,v in FULL['write'].items()}),
 'w_atnest':   arm(write={'at_nest':1.0,'reach.PickUp':0.0,'energy':0.0,'bias':-0.005}),  # one weight: write at the nest, hold away
 'w_nowrite':  arm(write={k:0.0 for k in FULL['write']}),  # no gate: a slow average at rate 0.995
 'w_lowrate':  arm(rate=0.0),                      # rate 0.5: matters only where the write row sits near 0 (a shut gate clamps z to 0 whatever the rate)
 'w_novalue':  arm(value={'mem9.reach.PickUp':0.0}),  # writes "empty" always
 'w_noread':   arm(read=0.0),                      # the come-home gate stops reading it
}
for a,ed in [(k,v) for k,v in ARMS.items() if len(sys.argv)<4 or k in sys.argv[3].split(',')]:
    d=f'{out}/{a}'; os.makedirs(d, exist_ok=True)
    for k,g in enumerate(pool):
        g=g[:CORE]+[0.0]*(CORE8-CORE)+g[CORE:]
        for s in mem_row(9): g[s]=0.0  # units 8 and 11 still compute P and N (gate8 reads both at ~0.001); nothing else changes
        for nm,v in ed['value'].items(): g[slot[nm]]=v
        g[RATE+9]=ed['rate']
        for nm,v in ed['write'].items(): g[wslot(9,nm)]=v
        if ed['read'] is not None: g[slot['gate8.mem9']]=ed['read']
        assert g[slot['gate8.mem9']]!=0.0 or ed['read']==0.0
        with open(f'{d}/m{k:02d}.raw.txt','w') as f:
            f.write(f'genome {len(g)} layout=8\n'); f.write(''.join(f'{j} {x!r}\n' for j,x in enumerate(g) if x!=0.0))
    print(a, len(pool), {k:v for k,v in ed.items() if k!='value' or v!=FULL['value']})
