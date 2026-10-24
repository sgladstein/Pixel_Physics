"""Below the datum within 20 cols of the nest: cells opened with a cut at that spot in the interval vs opened with no
cut (fell/slid away), cells filled back; and the pit (open to sky) over time."""
import sys, csv, glob, re, collections
import numpy as np
NX=128
def load(p):
    L=open(p).read().split("\n"); x0,y0,w,h=map(int,L[0].split())
    m=np.array([[c in ".af" for c in (r+" "*w)[:w]] for r in L[1:1+h]]); return x0,y0,m
for run in sys.argv[1:]:
    surf=[int(l) for l in open(f"{run}/surface.txt")]
    ps=sorted(glob.glob(f"{run}/map_f*.txt")); fr=[int(re.search(r"map_f(\d+)",p).group(1)) for p in ps]
    cuts=collections.defaultdict(set)
    for r in csv.DictReader(open(f"{run}/events.csv")):
        if r["kind"] in("dig","recut"): cuts[int(r["frame"])].add((int(r["x"]),int(r["y"]),r["kind"]))
    cf=sorted(cuts)
    x0,y0,prev=load(ps[0]); 
    def below(x,y): return y>=surf[x] if x<len(surf) else False
    # surface.txt indexed by world x? check length
    tot=collections.Counter(); series=[]
    import bisect
    for i in range(1,len(ps)):
        _,_,cur=load(ps[i]); a,b=fr[i-1],fr[i]
        cutcells={}
        for f in cf[bisect.bisect_right(cf,a):bisect.bisect_right(cf,b)]:
            for (x,y,k) in cuts[f]: cutcells[(x,y)]=k
        for yy,xx in zip(*np.nonzero(cur!=prev)):
            x,y=xx+x0,yy+y0
            if abs(x-NX)>20 or y<surf[x]: continue
            if cur[yy,xx]: tot["opened by "+cutcells[(x,y)] if (x,y) in cutcells else "opened, no cut"]+=1
            else: tot["filled"]+=1
        prev=cur
        # pit = sky-connected open below datum
        h,w=cur.shape; pit=0; nest=0
        for xx in range(w):
            x=xx+x0
            if abs(x-NX)>20: continue
            sky=True
            for yy in range(h):
                y=yy+y0
                if not cur[yy,xx]: sky=False
                if y>=surf[x] and cur[yy,xx]:
                    if sky: pit+=1
                    else: nest+=1
        series.append((b,pit,nest))
    print(run.split("inrun/")[1], dict(tot))
    print("   frame:pit/roofed-open", " ".join(f"{b//1000}k:{p}/{n}" for b,p,n in series[::5]+[series[-1]]))
