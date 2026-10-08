# Where do larvae starve, and does any egg get laid near one while it starves?
# Usage: starvers.py RUNDIR   (a deeptrace dig=1 run: broodlog.csv)
# A larva on upkeep alone loses 0.0083 J/frame, so the last WINDOW frames before it
# starves are spent below half of egg_cost (60 J): the band FEED_FIRST's default line
# reads. For each larva that starved (gone, cell_now=corpse), its position over that
# window (from moved rows), then every egg laid in the window with the layer's head
# within 2 / 6 cells of it -- the eggs reach 2 / 6 would have held.
import csv,sys,collections,math
run=sys.argv[1]; WINDOW=7200
last={}; seen=set(); track=collections.defaultdict(list); stage={}; starved=[]; lays=[]
for r in csv.DictReader(open(f'{run}/broodlog.csv')):
    f=int(r['frame']); i=r['id']; ev=r['event']
    if ev in ('laid','moved','stage'):
        x,y=int(r['x']),int(r['y']); track[i].append((f,x,y)); stage[i]=r['stage']
        if ev=='laid' and r['parent_x']:
            if i not in seen: seen.add(i); lays.append((f,int(r['parent_x']),int(r['parent_y']),r['parent_zone']))
    if ev=='gone': last[i]=(f,int(r['x']),int(r['y']),r['stage'],r['cell_now'])
    else: last.pop(i,None)
# A larva whose last row is `gone` onto an empty cell starved: no corpse cell is left
# (checked 2026-10-08: 145 such larvae against stats' 142 larvae_starved, seed 1).
starved=[(i,f,x,y) for i,(f,x,y,st,now) in last.items() if st=='larva' and now in ('empty','corpse')]
def where(i,f):
    p=None
    for t in track[i]:
        if t[0]>f: break
        p=t
    return p
lays.sort()
near2=near6=0; any6=0; zones=collections.Counter(); depth=[]; dmin=[]
for i,F,x,y in starved:
    depth.append(y); n2=n6=0; best=1e9
    for (lf,lx,ly,lz) in lays:
        if lf<F-WINDOW: continue
        if lf>F: break
        p=where(i,lf)
        if not p: continue
        d=max(abs(lx-p[1]),abs(ly-p[2]))
        best=min(best,d); n2+=d<=2; n6+=d<=6
    near2+=n2; near6+=n6; any6+=n6>0; dmin.append(best)
print(f"{run}")
print(f"  eggs laid {len(lays)}, larvae starved {len(starved)}")
print(f"  eggs laid within 2 / 6 cells of a starving larva (Chebyshev): {near2} / {near6}; starvers with any such egg: {any6}")
if starved:
    dm=sorted(dmin); print(f"  nearest egg to each starver in its window: median {dm[len(dm)//2]}, p10 {dm[len(dm)//10]}, no egg at all {sum(1 for d in dm if d>=1e9)}")
    ys=sorted(depth); print(f"  starver y: median {ys[len(ys)//2]} (range {ys[0]}-{ys[-1]})")
    ly=sorted(l[2] for l in lays); print(f"  layer y at lay: median {ly[len(ly)//2]}; lay zones {collections.Counter(l[3] for l in lays).most_common(4)}")
    fr=sorted(s[1] for s in starved); print(f"  starve frames: first {fr[0]}, median {fr[len(fr)//2]}, last {fr[-1]}")
# How each starver got from where it was laid to where it starved: rows dropped,
# and how many of its moves were straight down by one (a fall; brood is a powder)
# against anything else (a carry or a slide).
drop=[]; falls=carries=0
for i,F,x,y in starved:
    t=track[i]
    if not t: continue
    drop.append(y-t[0][2])
    for (f0,x0,y0),(f1,x1,y1) in zip(t,t[1:]):
        if (x1,y1)==(x0,y0): continue
        if x1==x0 and y1==y0+1: falls+=1
        else: carries+=1
if drop:
    d=sorted(drop); print(f"  rows from laying to starving: median {d[len(d)//2]}, p10 {d[len(d)//10]}, p90 {d[9*len(d)//10]}; moves one row down {falls}, other moves {carries}")
