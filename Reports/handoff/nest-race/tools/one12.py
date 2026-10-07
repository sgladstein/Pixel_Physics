# Per-seed table for a deeptrace run dir: ants, starved 20k-end, deep hungry falls, deep starvers, store bites, digs.
# Usage: one12.py RUNDIR  (needs stats.csv and hred.csv.gz = hungry.csv cut to frame,id,hy,zone,outcome)
import gzip,csv,re,sys,collections
run=sys.argv[1]; gy=160; GAP=40
rows=list(csv.DictReader(open(f'{run}/stats.csv'))); r=rows[-1]; r20=[x for x in rows if x['frame']=='20000'][0]
st=int(r['died_starved'])-int(r20['died_starved'])
dead={}
for L in open(f'{run}/events.txt'):
    if 'DIED' in L:
        f=int(L.split()[0]); i=int(re.search(r' id=(\d+)',L).group(1)); dead[i]=(f,'STARVED' in L)
n=fell=0; cur={}; eps=[]
for x in csv.DictReader(gzip.open(f'{run}/hred.csv.gz','rt')):
    f=int(x['frame']); i=int(x['id']); y=int(x['hy']); z=x['zone']
    deep = z=='nest' and y>gy+10
    if f>=20000 and deep:
        n+=1; fell += x['outcome']=='fell'
    e=cur.get(i)
    if e is None or f-e[1]>GAP:
        e=[f,f,deep,False,i]; eps.append(e); cur[i]=e
    e[1]=f
    if z!='nest' or y<=gy+10: e[3]=True
ds=0
for e in eps:
    if 60000<=e[0]<300000 and e[2]:
        d=dead.get(e[4])
        if d and d[1] and d[0]-e[1]<=200: ds+=1
print(f"{run}\tants {r['ants']}\tstarved {st}\tfall {100*fell/max(n,1):.1f}%\tdeep-starvers {ds}\tbites {r['nest_store_bites']}\tdigs {r['digs']}")
