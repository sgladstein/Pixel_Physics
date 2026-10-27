# mkarms.py POOL OUTDIR : one folder of member files per knock-out / ramp arm of the hand-laid latch
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from layout import *
src, out = sys.argv[1], sys.argv[2]
lines=[l.rstrip('\n') for l in open(src) if not l.startswith('#')]
h=lines[0].split(); n=int(h[3]); pool=[]
for l in lines[1:]:
    if l.startswith('member '): pool.append(list(pool[0]) if pool else [0.0]*n); continue
    i,w=l.split(' '); pool[-1][int(i)]=float(w)
slot={name(s):s for s in range(CORE)}
def S(nm): return slot[nm]
ARMS={
 'full':      {},
 'noread':    {'gate8.mem9':0.0},            # the gate stops reading the latch
 'noself':    {'mem9.mem9':0.0},             # the latch does not hold itself
 'noset':     {'mem9.mem8':0.0},             # "home had food" never sets it
 'noreset':   {'mem9.mem11':0.0},            # "home was empty" never resets it
 'nostart':   {'mem9.bias':0.0},             # starts unset rather than "empty"
 'nounit':    {'out.unit8':0.0},             # the come-home unit itself off (latch kept)
 'read1.5':   {'gate8.mem9':1.5},            # half the gate's read
 'self1':     {'mem9.mem9':1.0},             # self-weight at the edge of holding
 'set7.5':    {'mem9.mem8':7.5},             # half the set input
 'noset_noread': {'mem9.mem8':0.0,'gate8.mem9':0.0},
 'noset_shut':   {'mem9.mem8':0.0,'gate8.bias':-100.0},
 'shut':         {'gate8.bias':-100.0},       # the gate can never open: unit 8 present but silent
}
for a,ed in [(k,v) for k,v in ARMS.items() if len(sys.argv)<4 or k in sys.argv[3].split(',')]:
    d=f'{out}/{a}'; os.makedirs(d, exist_ok=True)
    for k,g in enumerate(pool):
        g=list(g)
        for nm,v in ed.items():
            assert g[S(nm)]!=0.0 or nm=='x', (a,nm)
            g[S(nm)]=v
        with open(f'{d}/m{k:02d}.raw.txt','w') as f:
            f.write(f'genome {n} layout=7\n'); f.write(''.join(f'{j} {x!r}\n' for j,x in enumerate(g) if x!=0.0))
    print(a, len(pool), ed)
