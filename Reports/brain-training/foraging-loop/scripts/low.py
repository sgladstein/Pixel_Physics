# Decisions by ants under 1 tank: picked verb, with whether Eat/Drop/Dig was on offer, split by holding spoil.
import sys,collections,csv,bisect
V="Walk Turn Rest Dig PickUp Drop Eat Feed Attack".split()
log,trace=sys.argv[1],sys.argv[2]
tk=collections.defaultdict(list)
for r in csv.DictReader(open(trace)): tk[int(r['id'])].append((int(r['frame']),float(r['tank']),int(r['spoil']),abs(int(r['x'])-128)))
c=collections.Counter()
for line in open(log):
    p=line.split(",")
    if len(p)<14 or p[1]!="Q": continue
    f=int(p[2]); i=int(p[3]); v=tk.get(i)
    if not v: continue
    j=bisect.bisect_right(v,(f,9e9))-1
    if j<0 or v[j][1]>=1.0: continue
    opts=p[13].split(";"); k=int(p[7]); offered=set(V[int(x.split(":")[0])] for x in opts if x)
    pv=V[int(opts[k].split(":")[0])] if 0<=k<len(opts) else "?"
    c[("spoil" if v[j][2] else "free", "home" if v[j][3]<=6 else "away", pv, "eat-offered" if "Eat" in offered else "-", "drop-offered" if "Drop" in offered else "-")]+=1
n=sum(c.values()); print(log.split("/")[-1], "rows under 1 tank:", n)
for k,x in sorted(c.items(),key=lambda kv:-kv[1])[:14]: print(f"  {x:6d} {100*x/n:5.1f}% {k}")
