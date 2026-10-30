# every ant: when it first reached an end wall, its energy then, whether it ever came back >=40 columns towards the nest,
# its bites, and when it was last seen. Rooms from argv: list of (lo,hi,nest).
import sys, collections
f=sys.argv[1]; rooms=[tuple(map(int,r.split(':'))) for r in sys.argv[2].split(',')]
rows=collections.defaultdict(list)
for l in open(f):
    if l.startswith('TRACE'):
        p=l.split(); rows[int(p[2])].append((int(p[1]),float(p[3]),int(p[4]),int(p[6])))
end=max(r[0] for v in rows.values() for r in v)
B=collections.Counter(); per=[]
for i,v in rows.items():
    lo,hi,nest=next(r for r in rooms if r[0]<=v[0][3]<=r[1])
    wall=lambda x: x<=lo+10 or x>=hi-10
    w=next((r for r in v if wall(r[3])),None)
    back=False
    if w:
        k=v.index(w)
        back=any(abs(r[3]-nest)<=abs(w[3]-nest)-40 for r in v[k:])
    alive=v[-1][0]==end; bites=v[-1][2]-v[0][2]
    walltime=sum(wall(r[3]) for r in v)/len(v)
    key=('reached wall' if w else 'never at wall', 'came back' if back else ('-' if not w else 'stayed'), 'alive' if alive else 'dead', 'ate' if bites>0 else 'no bite')
    B[key]+=1
    per.append((i,w[0] if w else -1,round(w[1]) if w else -1,round(walltime,2),bites,v[-1][0]))
for k,n in sorted(B.items(),key=lambda x:-x[1]): print(n,k)
if len(sys.argv)>3:
    for p in sorted(per): print('  ant %d reached wall at %d E=%d share at wall %.2f bites %d last %d'%p)
