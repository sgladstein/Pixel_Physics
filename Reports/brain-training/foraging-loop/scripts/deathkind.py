# Last trace row of every dead ant: crop / trip / spoil / where (columns from the hole, and depth row), and its tank 2,000 frames before.
import csv,sys,collections
c=collections.Counter(); ex={}
for D in sys.argv[1:]:
    tr=collections.defaultdict(list)
    for r in csv.DictReader(open(f"{D}/trace.csv")): tr[int(r['id'])].append(r)
    for r in csv.DictReader(open(f"{D}/events.csv")):
        if r['kind']!='death': continue
        t=tr[int(r['id'])]
        if not t: continue
        l=t[-1]; d=abs(int(l['x'])-128); still=sum(int(x['still']) for x in t[-40:])
        k=("crop" if int(l['crop'])>0 else "nocrop", "spoil" if int(l['spoil']) else "-", "home" if d<=8 else "near" if d<=15 else "field", "y>=64" if int(l['y'])>=64 else "y<64")
        c[k]+=1; ex.setdefault(k,(D,r['id'],r['frame'],l['x'],l['y']))
for k,v in c.most_common(): print(v,k,ex[k])
