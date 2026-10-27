# SCRATCH (teacher lane, 2026-10-26): builds scratch/inrun/genomes/latch_pool.txt (log section "the gated brain").
# Superseded as a design by the write-gated memory (layout 8, PR #942); kept so the latch_pool can be rebuilt.
# mklatch.py SRCDIR GATEFILE OUTDIR [k]: layout-7 pool = gated come-home pool (layout 5 + scratch gate rows) with a
# hand-laid "home had food" latch on the three least-used memory units, read by unit 8's gate.
import sys,glob,os,collections
src,gatef,out=sys.argv[1],sys.argv[2],sys.argv[3]; karg=sys.argv[4] if len(sys.argv)>4 else '3'; hijack=karg!='none'; k=float(karg) if hijack else 0.0
CTX,MEMN,VERBS,UNITS,HIDDEN,EMITS=22,12,9,16,8,3
RATE=11; S_CTX=23; S_IN=CTX+MEMN+1; JOB=1464; J_IN=CTX+MEMN+2; MEM=JOB+VERBS*J_IN; M_IN=CTX+2+VERBS+MEMN+1
EMIT=MEM+MEMN*M_IN; E_IN=CTX+MEMN+1; HID=EMIT+EMITS*E_IN; H_IN=CTX+MEMN+1
GATE=2797; GATE_IN=35; REACH_MEM=GATE+UNITS*GATE_IN; REACH_GATE=REACH_MEM+MEMN*VERBS; LEN7=REACH_GATE+UNITS*VERBS
assert MEM==1788 and LEN7==3609
AT_NEST=3; PICKUP=4; U=8
def readers(m):  # every slot that reads memory unit m
    r=[S_CTX+u*S_IN+CTX+m for u in range(UNITS)]+[JOB+v*J_IN+CTX+m for v in range(VERBS)]
    r+=[MEM+j*M_IN+CTX+2+VERBS+m for j in range(MEMN)]+[EMIT+e*E_IN+CTX+m for e in range(EMITS)]+[HID+h*H_IN+CTX+m for h in range(HIDDEN)]
    return r
files=sorted(glob.glob(src+'/*.raw.txt'))
G=[{int(l.split()[0]):float(l.split()[1]) for l in open(f).read().splitlines()[1:] if l.strip()} for f in files]
use=collections.Counter()
for g in G:
    for m in range(MEMN): use[m]+=sum(abs(g.get(i,0)) for i in readers(m))
P_,N_,L_=sorted(range(MEMN),key=lambda m:use[m])[:3]
print('memory use',[round(use[m]/len(G),1) for m in range(MEMN)],'-> P N L =',P_,N_,L_)
gate={int(l.split()[0]):float(l.split()[1]) for l in open(gatef).read().split('\n') if l.strip()}
os.makedirs(out,exist_ok=True)
for f,g in zip(files,G):
    h={}
    for i,w in g.items():
        h[i if i<GATE else i+(LEN7-GATE)]=w
    for i,w in gate.items(): h[GATE+i]=w
    for m in ((P_,N_,L_) if hijack else ()):
        for i in readers(m): h.pop(i,None)
        for i in range(MEM+m*M_IN,MEM+(m+1)*M_IN): h.pop(i,None)
        h[RATE+m]=-0.5                     # rate 0: the unit is its own sum, no leak
    row=lambda m,j: MEM+m*M_IN+j
    if not hijack: n=max(h)+1; open(os.path.join(out,os.path.basename(f)),'w').write(f"genome {LEN7+488} layout=7\n"+"".join(f"{i} {h[i]!r}\n" for i in sorted(h) if h[i]!=0)); continue
    h[row(P_,AT_NEST)]=20; h[REACH_MEM+P_*VERBS+PICKUP]=20; h[row(P_,M_IN-1)]=-22
    h[row(N_,AT_NEST)]=20; h[REACH_MEM+N_*VERBS+PICKUP]=-80; h[row(N_,0)]=-60; h[row(N_,M_IN-1)]=2
    h[row(L_,CTX+2+VERBS+L_)]=10; h[row(L_,CTX+2+VERBS+P_)]=15; h[row(L_,CTX+2+VERBS+N_)]=-15; h[row(L_,M_IN-1)]=-2
    if k:
        h[GATE+U*GATE_IN+CTX+L_]=k; h[GATE+U*GATE_IN+GATE_IN-1]=gate.get(U*GATE_IN+GATE_IN-1,0)-0.9*k
    n=max(h)+1
    open(os.path.join(out,os.path.basename(f)),'w').write(f"genome {LEN7+488} layout=7\n"+"".join(f"{i} {h[i]!r}\n" for i in sorted(h) if h[i]!=0))
print(out,len(files),'members; genome',n)
