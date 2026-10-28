"""x1 + g1base exam, 600k after landing: paired sign tests over box x seed. usage: python3 exam_cmp2.py (from this folder)"""
import csv,statistics as st
from math import comb
R={f"{r['brain']}|{r['box']}|{r['seed']}": r for r in csv.DictReader(open("lab600k.csv"))}
boxes=["two_larders","scattered_larder","feast_and_famine","played_bed","herb_first_colony","crowded_bench"]
def v(br,key,bx=None):
    out=[]
    for b in (bx or boxes):
        for s in range(1,9): out.append(float(R[f"{br}|{b}|{s}"][key]))
    return out
def sign(a,b):
    up=sum(1 for x,y in zip(a,b) if x>y+1e-9); dn=sum(1 for x,y in zip(a,b) if x<y-1e-9); n=up+dn
    k=min(up,dn); p=1.0 if n==0 else min(1.0,2*sum(comb(n,i) for i in range(k+1))/2**n)
    return f"{up} higher / {dn} lower, p={p:.2g}"
import sys
pairs=[("env1w","env1c"),("env1wf","env1c"),("env1w6","env1c"),("env1c","std"),("env1wf","std"),("env1w6","std"),("c3p","std"),("std","old"),("env1w6","old"),("c3p","old")]
for key in ("lived","starved","alive"):
    print("###",key)
    for a,b in pairs:
        print(f"  {a:7s} vs {b:6s} all 48: {sign(v(a,key),v(b,key))}")
print("### per-box lived, env1w6 vs env1c")
for bx in boxes: print("  ",bx,sign(v("env1w6","lived",[bx]),v("env1c","lived",[bx])))
print("### per-box lived, c3p vs std")
for bx in boxes: print("  ",bx,sign(v("c3p","lived",[bx]),v("std","lived",[bx])))
print("### more lived")
for a,b in [("k3","std"),("e3","std"),("e13","std"),("env1c","k3"),("env1w6","env1w"),("env1w6","k3"),("e13","env1w6")]:
    print(f"  {a:7s} vs {b:6s}: {sign(v(a,'lived'),v(b,'lived'))}")
print("### lived medians over 48")
for br in ["old","std","k3","e3","e13","env1c","env1w","env1wf","env1w6","c3p"]:
    print(f"  {br:7s} {st.median(v(br,'lived')):6.1f}  mean {st.mean(v(br,'lived')):6.1f}")
