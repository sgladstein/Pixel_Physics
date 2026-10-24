"""Per original-ground cut near the door: the column's state at the last 500-frame map before it, and the cutter's head
(trace=10 row at or before the cut). usage: cuts.py RUNDIR... -> summary per group"""
import sys,csv,glob,collections,bisect,os
def load_map(p):
    L=open(p).read().split('\n'); x0,y0,w,h=map(int,L[0].split()); return x0,y0,[r for r in L[1:1+h]]
def analyse(d):
    surf=[int(l) for l in open(d+'/surface.txt') if l.strip()]
    maps=sorted(glob.glob(d+'/map_f*.txt')); mf=[int(os.path.basename(m)[5:12]) for m in maps]
    cache={}
    def m_at(fr):
        i=bisect.bisect_right(mf,fr)-1
        if i<0: return None
        if i not in cache: cache.clear(); cache[i]=load_map(maps[i])
        return cache[i]
    # trace: positions by (id) -> list of (frame,x,y,spoil)
    pos=collections.defaultdict(list)
    for r in csv.reader(open(d+'/trace.csv')):
        if r[0]=='frame': continue
        pos[int(r[1])].append((int(r[0]),int(r[2]),int(r[3])))
    for k in pos: pos[k].sort()
    out=[]
    ev=[r for r in csv.DictReader(open(d+'/events.csv')) if r['kind'] in ('dig','unjoined')]
    unj=set((r['frame'],r['x'],r['y']) for r in ev if r['kind']=='unjoined')
    for r in ev:
        if r['kind']!='dig': continue
        fr,x,y,who=int(r['frame']),int(r['x']),int(r['y']),int(r['id'])
        if abs(x-128)>20: continue
        m=m_at(fr)
        sky=None
        if m:
            x0,y0,rows=m
            cx=x-x0
            col=[rows[yy-y0][cx] if 0<=yy-y0<len(rows) and 0<=cx<len(rows[0]) else '#' for yy in range(surf[x],y)]
            sky = all(c!='#' for c in col)   # no ground/spoil between the datum and the cell: the cut opens to the sky
        p=pos.get(who,[]); i=bisect.bisect_right(p,(fr,10**9,10**9))-1
        hx=hy=None
        if i>=0: _,hx,hy=p[i]
        above = hy is not None and hy < surf[min(max(hx,0),len(surf)-1)]
        out.append(dict(fr=fr,x=x,y=y,depth=y-surf[x],sky=sky,above=above,hx=hx,hy=hy,paid=(r['frame'],r['x'],r['y']) not in unj))
    return out
if __name__=='__main__':
    groups=collections.defaultdict(list)
    for d in sys.argv[1:]:
        g=os.path.basename(d.rstrip('/')).rsplit('_',1)[0]
        groups[g]+=analyse(d.rstrip('/'))
    for g,o in groups.items():
        n=len(o); f=lambda c: sum(1 for z in o if c(z))
        print(f"{g:9} cuts {n:5}  sky-exposed {f(lambda z:z['sky'])/n:.2f}  cutter above datum {f(lambda z:z['above'])/n:.2f}  "
              f"depth<3 {f(lambda z:z['depth']<3)/n:.2f}  depth>=8 {f(lambda z:z['depth']>=8)/n:.2f}  "
              f"sky&depth<3 {f(lambda z:z['sky'] and z['depth']<3)/n:.2f}  sky&deeper {f(lambda z:z['sky'] and z['depth']>=3)/n:.2f}  roofed {f(lambda z:not z['sky'])/n:.2f}")
