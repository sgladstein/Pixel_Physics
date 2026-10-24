import sys, csv, glob, re, collections, bisect
import numpy as np
NX=128
def load(p):
    L=open(p).read().split("\n"); x0,y0,w,h=map(int,L[0].split())
    m=np.array([[c in ".af" for c in (r+" "*w)[:w]] for r in L[1:1+h]]); return x0,y0,m
def skyof(m):
    s=np.zeros_like(m)
    for x in range(m.shape[1]):
        y=0
        while y<m.shape[0] and m[y,x]: s[y,x]=True; y+=1
    return s
for run in sys.argv[1:]:
    surf=[int(l) for l in open(f"{run}/surface.txt")]
    ps=sorted(glob.glob(f"{run}/map_f*.txt")); fr=[int(re.search(r"map_f(\d+)",p).group(1)) for p in ps]
    cuts=collections.defaultdict(set); ins=collections.defaultdict(set)
    for r in csv.DictReader(open(f"{run}/events.csv")):
        if r["kind"] in("dig","recut"): cuts[int(r["frame"])].add((int(r["x"]),int(r["y"])))
        if r["kind"]=="in": ins[int(r["frame"])].add((int(r["x"]),int(r["y"])))
    cf=sorted(cuts); inf=sorted(ins)
    x0,y0,prev=load(ps[0]); tot=collections.Counter()
    for i in range(1,len(ps)):
        _,_,cur=load(ps[i]); a,b=fr[i-1],fr[i]; sk=skyof(cur); skp=skyof(prev)
        cc=set().union(*[cuts[f] for f in cf[bisect.bisect_right(cf,a):bisect.bisect_right(cf,b)]] or [set()])
        ic=set().union(*[ins[f] for f in inf[bisect.bisect_right(inf,a):bisect.bisect_right(inf,b)]] or [set()])
        near=lambda x,y,S: any((x+dx,y+dy) in S for dx in(-1,0,1) for dy in(-1,0,1))
        for yy,xx in zip(*np.nonzero(cur!=prev)):
            x,y=xx+x0,yy+y0
            if abs(x-NX)>20 or y<surf[x]: continue
            where="pit" if (sk[yy,xx] if cur[yy,xx] else skp[yy,xx]) else "roofed"
            if cur[yy,xx]: tot[("opened by cut" if (x,y) in cc else "opened, no cut")+" @"+where]+=1
            else: tot[("filled by a dropped pellet" if near(x,y,ic) else "filled, no drop")+" @"+where]+=1
        prev=cur
    print(run.split("inrun/")[1]); [print("   ",k,v) for k,v in sorted(tot.items())]
