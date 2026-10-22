# usage: heading.py ARM -- for ants that starved past 60 cols: from the last time inside 40 cols until death, the chosen
# Walk's column step relative to the nest (home / out / same column), binned by tank; and whether a homeward Walk was offered.
import csv,glob,collections,sys
B=[(20,1e9,">=20 tanks"),(5,20,"5-20"),(2,5,"2-5"),(0,2,"<2")]
for arm in sys.argv[1:]:
    agg={b[2]:collections.Counter() for b in B}
    wallrows=collections.Counter()
    for D in sorted(glob.glob(f"l/{arm}[0-9]*/")):
        deaths={int(r['id']):int(r['frame']) for r in csv.DictReader(open(D+"events.csv")) if r['kind']=='death'}
        last={}
        for r in csv.DictReader(open(D+"trace.csv")):
            i=int(r['id'])
            if i in deaths: last.setdefault(i,[]).append((int(r['frame']),int(r['x'])))
        roam={}
        for i,t in last.items():
            if abs(t[-1][1]-128)>=60:
                roam[i]=max([f for f,x in t if abs(x-128)<=40],default=0)
        if not roam: continue
        for line in open(D+"dlog.csv"):
            p=line.split(',',12)
            i=int(p[1])
            if i not in roam or int(p[0])<roam[i]: continue
            tank=float(p[4])/200; x=int(p[2]); v=int(p[7]); tx=int(p[8])
            b=next(b[2] for b in B if b[0]<=tank<b[1])
            c=agg[b]; c['n']+=1
            if v==0:
                d=abs(tx-128)-abs(x-128)
                c['walk']+=1; c['home' if d<0 else 'out' if d>0 else 'same']+=1
            else: c['v%d'%v]+=1
            if abs(x-128)>=115: c['atwall']+=1
    print(arm)
    for b in B:
        c=agg[b[2]]; n=max(1,c['n']); w=max(1,c['walk'])
        print(f"  {b[2]:10s} decisions {c['n']:7d}  walk {c['walk']/n:.0%} (homeward {c['home']/w:.0%} out {c['out']/w:.0%} same col {c['same']/w:.0%})  rest {c['v2']/n:.0%} turn {c['v1']/n:.0%} dig {c['v3']/n:.0%}  within 13 cols of a wall {c['atwall']/n:.0%}")
