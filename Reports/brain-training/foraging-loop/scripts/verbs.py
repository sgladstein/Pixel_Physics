import sys,collections
V="Walk Turn Rest Dig PickUp Drop Eat Feed Attack".split()
for fn in sys.argv[1:]:
    c=collections.Counter(); off=collections.Counter()
    for line in open(fn):
        p=line.split(",")
        if len(p)<14 or p[1]!="Q": continue
        opts=p[13].split(";"); k=int(p[7])
        if 0<=k<len(opts): c[V[int(opts[k].split(":")[0])]]+=1
        for o in set(int(x.split(":")[0]) for x in opts if x): off[V[o]]+=1
    n=sum(c.values()); print(fn.split("/")[-1], n, " ".join(f"{v}:{100*c[v]/n:.1f}%(off {100*off[v]/n:.0f}%)" for v in V if c[v] or off[v]))
