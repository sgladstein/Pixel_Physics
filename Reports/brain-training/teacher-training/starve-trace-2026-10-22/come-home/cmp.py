# usage: cmp.py DIR BASE ARM... -- per arm over seeds 1-32 in DIR: starved, starved past 60 cols, colonies with >=12 dead,
# home deliveries (|x-128|<=8), new cuts (dig events), ant-time past 60 cols; paired against BASE by seed.
import csv,os,sys,collections
D=sys.argv[1]
def one(a,s):
    p=f"{D}/{a}{s}"
    if not os.path.exists(p+"/events.csv") or "done:" not in open(p+".log").read(): return None
    h=dead=dig=0; ids=set()
    for r in csv.DictReader(open(p+"/events.csv")):
        k=r['kind']
        if k=='deliver' and abs(int(r['x'])-128)<=8: h+=1
        elif k=='death': dead+=1; ids.add(int(r['id']))
        elif k=='dig': dig+=1
    last={}; far=tot=0
    for r in csv.DictReader(open(p+"/trace.csv")):
        d=abs(int(r['x'])-128); last[int(r['id'])]=d; tot+=1; far+=d>60
    roam=sum(1 for i in ids if last.get(i,0)>=60)
    return dict(h=h,dead=dead,roam=roam,dig=dig,far=far/max(1,tot))
base=sys.argv[2]
res={a:{s:one(a,s) for s in range(1,33)} for a in sys.argv[2:]}
for a,v in res.items():
    ok=[s for s in v if v[s]]
    if not ok: continue
    n=len(ok); S=lambda k: sum(v[s][k] for s in ok)
    both=[s for s in ok if res[base].get(s)]
    fewer=sum(v[s]['dead']<res[base][s]['dead'] for s in both); more=sum(v[s]['dead']>res[base][s]['dead'] for s in both)
    print(f"{a:10s} seeds {n:2d} starved/colony {S('dead')/n:5.2f} (past 60 cols {S('roam')/n:4.2f})  collapsed {sum(v[s]['dead']>=12 for s in ok):2d}  home/colony {S('h')/n:6.0f}  new cuts/colony {S('dig')/n:5.0f}  time past 60 cols {S('far')/n:.1%}   vs {base}: fewer deaths {fewer}, more {more}")
