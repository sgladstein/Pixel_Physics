"""Each rung's metric on one in-run output dir (NEST_X=128, within 20 cols, after 20k frames)."""
import sys, os, csv, glob, re, collections
import numpy as np
from collections import deque
MYARGS=list(sys.argv); sys.argv=["x"]
here=os.path.dirname(os.path.abspath(__file__))
exec(open(os.path.join(here,"inrunmap.py")).read().split('if __name__')[0])
sys.argv=MYARGS
def rung(run):
    surf=[int(l) for l in open(run+"/surface.txt")]
    new=deep=0; out=far=0; inn=0
    for r in csv.DictReader(open(run+"/events.csv")):
        k=r["kind"]; x=int(r["x"])
        if k=="dig" and abs(x-128)<=20: new+=1; deep+=int(r["v"])>=3
        if k=="out": out+=1; far+=abs(x-128)>20
        if k=="in": inn+=1
    opened=0
    for p in sorted(glob.glob(run+"/map_f*.txt")):
        f=int(re.search(r"map_f(\d+)",p).group(1))
        if f<20000: continue
        L=open(p).read().split("\n"); x0,y0,w,h=map(int,L[0].split())
        m=np.array([[c in ".af" for c in (r+" "*w)[:w]] for r in L[1:1+h]])
        seen=np.zeros_like(m); q=deque((0,xx) for xx in range(w) if m[0,xx])
        for c in q: seen[c]=True
        while q:
            yy,xx=q.popleft()
            for a,b in ((yy+1,xx),(yy-1,xx),(yy,xx+1),(yy,xx-1)):
                if 0<=a<h and 0<=b<w and m[a,b] and not seen[a,b]: seen[a,b]=True; q.append((a,b))
        nest=[(yy,xx) for xx in range(w) if abs(xx+x0-128)<=20 for yy in range(h) if m[yy,xx] and yy+y0>=surf[xx+x0]+3]
        if not nest: continue
        opened+=1 if any(seen[yy,xx] for yy,xx in nest) else 0
        globals().setdefault("_n",collections.Counter())[run]+=1
    checks=globals().get("_n",{}).get(run,0)
    m,ground,x0,y0=load(run,200000)
    s,_=ns.score(m,ground,128-x0); tot,parts,nm,cr=score(m,ground,128-x0,verbose='parts')
    roofed=s['dug']-s['pit']
    return dict(deep=deep/max(1,new), new=new, far=far/max(1,out), inn=inn, door=opened/max(1,checks), roofed=roofed, pit=s['pit'], score=tot, chambers=s['chambers'])
import contextlib, io
rows=[]
for arm,base in (("as trained","crowd"),("rim cuts banned","rim")):
    for n in (1,30):
        for sd in (1,2,4,6):
            with contextlib.redirect_stdout(io.StringIO()): r=rung(f"{sys.argv[1]}/{base}/n{n}_s{sd}")
            rows.append((arm,n,sd,r)); print(arm,n,sd,{k:(round(v,2) if isinstance(v,float) else v) for k,v in r.items()},flush=True)
