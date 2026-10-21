import glob, os, re, sys
D=sys.argv[1]; arms=sys.argv[2:] or ["rb"]
SP=["creeper","grass","herb","shrub"]
def parse(path):
    rows=[]
    for line in open(path):
        if not line.startswith("f="): continue
        parts=line.rstrip("\n").split(" | "); r={"f":int(parts[0][2:]),"sp":{}}
        for p in parts[1:]:
            m=re.match(r"([a-z]+) plants=(\d+) bank=(\d+) leaf=(\d+) est=(\d+) rootwet=([-0-9.]+) rootdepth=([-0-9.]+) leaflight=([-0-9.]+)",p)
            if m:
                r["sp"][m.group(1)]=dict(plants=int(m.group(2)),bank=int(m.group(3)),leaf=int(m.group(4)),est=int(m.group(5)),rootwet=float(m.group(6)),rootdepth=float(m.group(7)),light=float(m.group(8)));continue
            for k,v in re.findall(r"([a-z_0-9]+)=([^ ]+)",p): r[k]=v
        rows.append(r)
    return rows
def med(v):
    v=sorted(x for x in v if x==x); return v[len(v)//2] if v else float("nan")
for arm in arms:
    runs={}
    for path in sorted(glob.glob(os.path.join(D,"runs",f"{arm}-s*.log"))):
        if "plant_m0 done" in open(path).read():
            runs[int(re.search(r"-s(\d+)\.log",path).group(1))]=parse(path)
    if not runs: continue
    print(f"\n=== {arm}: seeds {sorted(runs)}")
    for f in [120000,240000,360000,480000]:
        rs=[next(r for r in rows if r["f"]==f) for rows in runs.values()]
        print(f"  f={f:>6}  standing {[int(r['standing']) for r in rs]}  seed above soil {[int(r['seed']) for r in rs]}  deepest heap {[int(r['seedmax']) for r in rs]}  spread {[int(r.get('spread',0)) for r in rs]}")
    end=[rows[-1] for rows in runs.values()]
    tips=[sum(int(r["tips"]) for r in rows) for rows in runs.values()]
    for key in ["contact_other","touch_seed","touch_kin","touch_foreign","up_blocked_other"]:
        tot=[sum(int(r[key]) for r in rows) for rows in runs.values()]
        print(f"  tips touching ({key}): per seed {[round(t/max(n,1),3) for t,n in zip(tot,tips)]}")
    print("  per species at 480k: plants | leaf | established in last 100k | leaf light (median over run) | root water after dry/wet | deepest root")
    for s in SP:
        pl=[rows[-1]["sp"][s]["plants"] for rows in runs.values()]
        lf=[rows[-1]["sp"][s]["leaf"] for rows in runs.values()]
        est=[sum(r["sp"][s]["est"] for r in rows[-10:]) for rows in runs.values()]
        light=med([r["sp"][s]["light"] for rows in runs.values() for r in rows if r["sp"][s]["light"]>=0])
        rw=lambda ph: med([r["sp"][s]["rootwet"] for rows in runs.values() for r in rows if r["f"]>=360000 and r["f"]%60000==ph and r["sp"][s]["rootwet"]>=0])
        dep=med([r["sp"][s]["rootdepth"] for rows in runs.values() for r in rows if r["f"]>=360000 and r["sp"][s]["rootdepth"]>=0])
        print(f"    {s:<8} plants {pl} leaf {lf} est {est} light {light:.3f} rootwater {rw(0):.2f}/{rw(20000):.2f} depth {dep:.1f}")
    alive=[sum(1 for s in SP if rows[-1]["sp"][s]["leaf"]>=100) for rows in runs.values()]
    print(f"  kinds with 100+ leaf at 480k, per seed: {alive}")
