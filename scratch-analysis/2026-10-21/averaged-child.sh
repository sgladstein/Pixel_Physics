#!/bin/bash
# SCRATCH, recovered verbatim from the 2026-10-21 session: the averaged-child test (handoff section 0, next 2).
# Needs: $M/mut.csv from reliab over mutants children in $M/{A,C,D}/ (mutants.rs), $M/parent.txt; a reliab build.
# 1) average the changes of the top quarter (by set 0) of each arm's children onto the parent -> mean-<arm>.txt
# 2) score parent + averaged brains on reliab's two 48-world sets; compare set-1 gain with truncation's.
# (The 96-test-world rerun used train score= test=96 noise=1 on mean-*.txt, as in the handoff's section 4.)
M=${M:?set M to the mutation-test folder}
python3 -I - $M <<'EOF'
import sys,csv,statistics as st
from collections import defaultdict
M=sys.argv[1]
rows=defaultdict(lambda: defaultdict(dict))
for r in csv.DictReader(open(f"{M}/mut.csv")): rows[r["file"]][int(r["set"])][int(r["world"])]=float(r["full_loads"])
def load(f):
    L=open(f).read().splitlines(); return L[0],{int(l.split()[0]):float(l.split()[1]) for l in L[1:] if l.strip()}
head,par=load(f"{M}/parent.txt")
for arm in ("A","C","D"):
    kids=[f for f in rows if f.split("/")[-2]==arm]
    s0={f:st.fmean(rows[f][0].values()) for f in kids}
    top=sorted(kids,key=lambda f:-s0[f])[:len(kids)//4]
    acc=defaultdict(float)
    for f in top:
        _,g=load(f.replace("../mut",M))
        for k in set(g)|set(par): acc[k]+=g.get(k,0.0)-par.get(k,0.0)
    new={k:par.get(k,0.0)+v/len(top) for k,v in acc.items()}
    for k in par:
        new.setdefault(k,par[k])
    with open(f"{M}/mean-{arm}.txt","w") as o:
        o.write(head+"\n"); [o.write(f"{k} {v!r}\n") for k,v in sorted(new.items()) if v!=0.0]
    print(arm,"top",len(top),"written mean-"+arm)
EOF
PIXEL_PHYSICS_MUTATION=off target/release/examples/reliab 8 12000 $M/meanrec.csv $M/parent.txt $M/mean-A.txt $M/mean-C.txt $M/mean-D.txt > $M/meanrec.log 2>&1; python3 -I - $M <<'EOF'
import sys,csv,statistics as st
from collections import defaultdict
M=sys.argv[1]
R=defaultdict(lambda: defaultdict(dict)); T=defaultdict(lambda: defaultdict(dict))
for r in csv.DictReader(open(f"{M}/meanrec.csv")): R[r["file"].split("/")[-1]][int(r["set"])][int(r["world"])]=float(r["full_loads"])
for r in csv.DictReader(open(f"{M}/mut.csv")): T[r["file"]][int(r["set"])][int(r["world"])]=float(r["full_loads"])
p1=st.fmean(R["parent.txt"][1].values())
pairs=lambda d:[(d[2*i]+d[2*i+1])/2 for i in range(24)]
print(f"parent on set 1: {p1:.2f}")
for arm in ("A","C","D"):
    kids=[f for f in T if f.split("/")[-2]==arm]; s0={f:st.fmean(T[f][0].values()) for f in kids}
    top=sorted(kids,key=lambda f:-s0[f])[:len(kids)//4]
    trunc=st.fmean(st.fmean(T[f][1].values()) for f in top)-p1
    m=R[f"mean-{arm}.txt"][1]; v=pairs(m); mm=st.fmean(v); se=st.stdev(v)/24**.5
    print(f"arm {arm}: truncation (top quarter's mean on set 1) {trunc:+.2f}; averaged brain {mm-p1:+.2f} (its own s.e. {se:.2f})")
EOF
