# Who forages: per-ant field bites, deliveries, home eats, and whether it died. Args: run dirs.
import csv,sys,collections
for D in sys.argv[1:]:
    ids=[int(r['id']) for r in csv.DictReader(open(f"{D}/ants.csv"))]
    f=collections.Counter(); d=collections.Counter(); h=collections.Counter(); dead={}
    for r in csv.DictReader(open(f"{D}/events.csv")):
        i=int(r['id']); k=r['kind']
        if k=='field': f[i]+=1
        elif k=='deliver': d[i]+=1
        elif k=='homeeat': h[i]+=1
        elif k=='death': dead[i]=int(r['frame'])
    fb=sorted((f[i] for i in ids), reverse=True)
    nof=sum(1 for i in ids if f[i]==0); 
    top5=sum(fb[:5])/max(1,sum(fb))
    print(f"{D.split('/')[-1]:7s} ants {len(ids)}  never foraged {nof}  top-5 foragers take {top5:.0%} of bites  ants with a home meal {sum(1 for i in ids if h[i]>0)}  dead {len(dead)}: their field bites {[f[i] for i in dead][:12]}  home meals {[h[i] for i in dead][:12]}")
