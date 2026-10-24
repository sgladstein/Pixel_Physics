"""Cuts per ant-time by where the cutter's head is: depth band below the datum (head), within 20 columns of the door.
Occupancy from trace (every 10 frames, only ants with empty jaws: spoil=0 and crop=0), cuts from dig+recut events via the cutter's last trace row."""
import sys,csv,collections,bisect,os
bands=[(-99,0,'above'),(0,3,'0-2'),(3,6,'3-5'),(6,10,'6-9'),(10,15,'10-14'),(15,99,'15+')]
def band(d):
    for a,b,n in bands:
        if a<=d<b: return n
G=collections.defaultdict(lambda:[collections.Counter(),collections.Counter(),collections.Counter()])
for d in sys.argv[1:]:
    d=d.rstrip('/'); g=os.path.basename(d).rsplit('_',1)[0]
    surf=[int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    pos=collections.defaultdict(list)
    for r in csv.reader(open(d+'/trace.csv')):
        if r[0]=='frame': continue
        fr,i,x,y=int(r[0]),int(r[1]),int(r[2]),int(r[3])
        pos[i].append((fr,x,y))
        if abs(x-128)<=20:
            G[g][0][band(y-surf[x])]+=10   # ant-frames (any jaws)
    for k in pos: pos[k].sort()
    for r in csv.DictReader(open(d+'/events.csv')):
        if r['kind'] not in('dig',): continue
        fr,who=int(r['frame']),int(r['id']); p=pos.get(who,[])
        i=bisect.bisect_right(p,(fr,10**9,10**9))-1
        if i<0: continue
        _,hx,hy=p[i]
        if abs(hx-128)>20: continue
        G[g][1][band(hy-surf[hx])]+=1
        G[g][2][band(int(r['y'])-surf[int(r['x'])])]+=1
for g,(occ,cut,cell) in G.items():
    to=sum(occ.values())
    print(g.ljust(9),' '.join(f"{n}: time {occ[n]/to:.2f} cuts {cut[n]:4d} per10k {1e4*cut[n]/max(occ[n],1):5.2f}" for _,_,n in bands))
