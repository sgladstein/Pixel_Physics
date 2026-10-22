# One line per run: where field food went, and how the colony ended. Args: run dirs.
import csv,sys,collections,bisect
def q(v,p): v=sorted(v); return v[min(len(v)-1,int(p*len(v)))] if v else 0
print(f"{'run':8s} {'bitten':>6s} {'deliv':>5s} {'%':>5s} {'ateByChoice':>11s} {'fieldDrop':>9s} {'restDigested':>12s} {'alive':>5s} {'starved':>7s} {'tankMin':>7s} {'tankMed':>7s} {'tankMax':>7s} {'nestFoodEnd':>11s}")
for D in sys.argv[1:]:
    tr=collections.defaultdict(list)
    for r in csv.DictReader(open(f"{D}/trace.csv")):
        tr[int(r['id'])].append((int(r['frame']),int(r['trip'])))
    fr={i:[t[0] for t in v] for i,v in tr.items()}
    c=collections.Counter(); lumps=0; starved=0
    for r in csv.DictReader(open(f"{D}/events.csv")):
        k=r['kind']; c[k]+=1
        if k=='eat' and float(r['v'])>10:
            i=int(r['id']); v=tr.get(i)
            if v:
                j=max(0,bisect.bisect_right(fr[i],int(r['frame']))-1)
                lumps+= v[j][1]>0
        if k=='death' and r['v']=='0': starved+=1
    deaths=collections.Counter(r['v'] for r in csv.DictReader(open(f"{D}/events.csv")) if r['kind']=='death')
    last=list(csv.DictReader(open(f"{D}/stock.csv")))[-1]
    b=c['field']; d=c['deliver']
    rest=b-d-lumps-c['fdrop']
    tanks=[]
    lf=max(int(x['frame']) for x in csv.DictReader(open(f"{D}/trace.csv")))
    tanks=[float(x['tank']) for x in csv.DictReader(open(f"{D}/trace.csv")) if int(x['frame'])==lf]
    print(f"{D.split('/')[-1]:8s} {b:6d} {d:5d} {100*d/max(1,b):5.1f} {lumps:11d} {c['fdrop']:9d} {rest:12d} {last['alive']:>5s} {sum(deaths.values()):7d} {min(tanks) if tanks else 0:7.1f} {q(tanks,.5):7.1f} {max(tanks) if tanks else 0:7.1f} {last['nest_food']:>11s}")
