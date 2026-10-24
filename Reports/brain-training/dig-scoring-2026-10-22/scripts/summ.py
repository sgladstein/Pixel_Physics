"""One line per run: how much of the colony's digging is new ground, where spoil goes, and whether the hole keeps."""
import sys, csv, glob, re, collections
import numpy as np
NX=128
def load(p):
    L=open(p).read().split("\n"); x0,y0,w,h=map(int,L[0].split())
    return x0,y0,np.array([[c in ".af" for c in (r+" "*w)[:w]] for r in L[1:1+h]])
print(f"{'run':16s} {'ants':>4s} {'new':>5s} {'recut':>5s} {'new%':>5s} {'out':>5s} {'out>20%':>7s} {'in':>5s} {'opened':>6s} {'filled':>6s} {'pit':>4s} {'roofed':>6s} {'deepest':>7s}")
for run in sys.argv[1:]:
    surf=[int(l) for l in open(f"{run}/surface.txt")]
    k=collections.Counter(); far=0; ants=set()
    for r in csv.DictReader(open(f"{run}/events.csv")):
        ants.add(r["id"]); k[r["kind"]]+=1
        if r["kind"]=="out" and abs(int(r["x"])-NX)>20: far+=1
    ps=sorted(glob.glob(f"{run}/map_f*.txt"))
    x0,y0,prev=load(ps[0]); op=fi=0
    for p in ps[1:]:
        _,_,cur=load(p)
        for yy,xx in zip(*np.nonzero(cur!=prev)):
            x,y=xx+x0,yy+y0
            if abs(x-NX)<=20 and y>=surf[x]: op+=cur[yy,xx]; fi+=not cur[yy,xx]
        prev=cur
    pit=roof=0; deep=0
    for xx in range(cur.shape[1]):
        x=xx+x0
        if abs(x-NX)>20: continue
        sky=True
        for yy in range(cur.shape[0]):
            y=yy+y0
            if not cur[yy,xx]: sky=False
            if y>=surf[x] and cur[yy,xx]:
                deep=max(deep,y-surf[x]+1)
                if sky: pit+=1
                else: roof+=1
    new=k["dig"]; rc=k["recut"]
    print(f"{run.split('inrun/')[1]:16s} {len(ants):4d} {new:5d} {rc:5d} {100*new/max(1,new+rc):5.0f} {k['out']:5d} {100*far/max(1,k['out']):7.0f} {k['in']:5d} {op:6d} {fi:6d} {pit:4d} {roof:6d} {deep:7d}")
