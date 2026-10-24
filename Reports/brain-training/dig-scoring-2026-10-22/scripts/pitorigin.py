"""Every pit cell in the last map (below the datum, open, sky-connected, within 20 cols): was that exact cell ever cut
(dig = original ground, recut = moved soil), and when did it first open."""
import sys,csv,glob,re,collections
import numpy as np
for run in sys.argv[1:]:
    surf=[int(l) for l in open(run+"/surface.txt")]
    cut={}
    for r in csv.DictReader(open(run+"/events.csv")):
        if r["kind"] in("dig","recut"):
            cut.setdefault((int(r["x"]),int(r["y"])),(r["kind"],int(r["frame"])))
    ps=sorted(glob.glob(run+"/map_f*.txt"))
    L=open(ps[-1]).read().split("\n"); x0,y0,w,h=map(int,L[0].split()); rows=L[1:1+h]
    c=collections.Counter(); first=[]
    for xx in range(w):
        x=xx+x0
        if abs(x-128)>20: continue
        for yy in range(h):
            ch=(rows[yy]+" "*w)[xx]
            if ch not in ".af": break
            y=yy+y0
            if y>=surf[x]:
                k=cut.get((x,y)); c[k[0] if k else "never cut"]+=1
                if k: first.append(k[1])
    print(run.split("inrun/")[1], dict(c), "cut frames (quartiles):", np.percentile(first,[25,50,75]).astype(int).tolist() if first else "-")
