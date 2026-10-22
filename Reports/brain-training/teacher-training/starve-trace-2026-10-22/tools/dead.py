# usage: dead.py RUNDIR... -- one row per starved ant: its last W frames of decisions (dlog.csv), its life (events.csv),
# the nest store at death (stock.csv). Writes RUNDIR/dead.json and prints a table.
import csv,sys,json,collections,bisect
W=2000
V=["walk","turn","rest","dig","pick","drop","eat","feed","atk"]
for D in sys.argv[1:]:
    ev=list(csv.DictReader(open(f"{D}/events.csv")))
    deaths={int(r['id']):int(r['frame']) for r in ev if r['kind']=='death'}
    if not deaths: json.dump([],open(f"{D}/dead.json","w")); continue
    life=collections.defaultdict(lambda: collections.defaultdict(list))
    for r in ev:
        life[int(r['id'])][r['kind']].append((int(r['frame']),int(r['x']),int(r['y']),float(r['v'])))
    st=[(int(r['frame']),int(r['nest_food']),int(r['door_food']),int(r['field_food']),float(r['tank_mean'])) for r in csv.DictReader(open(f"{D}/stock.csv"))]
    sf=[s[0] for s in st]
    # start energy per tank: trace tank vs dlog energy
    tr={}
    for r in csv.DictReader(open(f"{D}/trace.csv")):
        tr.setdefault(int(r['id']),[]).append((int(r['frame']),int(r['x']),int(r['y']),float(r['tank']),int(r['crop']),int(r['spoil']),int(r['since_nest']),int(r['still'])))
    rows=collections.defaultdict(list)
    for line in open(f"{D}/dlog.csv"):
        p=line.rstrip().split(',')
        i=int(p[1])
        if i in deaths and deaths[i]-W<=int(p[0])<=deaths[i]:
            rows[i].append(p)
    out=[]
    for i,fd in sorted(deaths.items(), key=lambda x:x[1]):
        rs=rows[i]
        t=tr.get(i,[])
        tb=[x for x in t if x[0]<=fd-W]
        tank_then=tb[-1][3] if tb else None
        last=t[-1] if t else None
        burst=sum(1 for f in deaths.values() if abs(f-fd)<=3000)
        j=max(0,bisect.bisect_right(sf,fd)-1)
        n=len(rs)
        ch=collections.Counter(V[int(p[7])] for p in rs)
        off=collections.Counter()
        for p in rs:
            c=[int(x) for x in p[10].split()]
            for k,v in enumerate(c):
                if v: off[V[k]]+=1
        ctx=[[float(x) for x in p[12].split()] for p in rs]
        L=life[i]
        def lastf(k): return max([e[0] for e in L[k]],default=-1)
        xs=[int(p[2]) for p in rs]; ys=[int(p[3]) for p in rs]
        o=dict(run=D.split('/')[-1],id=i,death=fd,n=n,burst=burst,
          x=last[1] if last else None,y=last[2] if last else None,
          dx=(last[1]-128) if last else None,
          tank_then=tank_then, tank_last=last[3] if last else None,
          spoil_frac=sum(int(p[6]) for p in rs)/max(1,n), crop_frac=sum(int(p[5])>0 for p in rs)/max(1,n),
          chose={k:round(v/max(1,n),2) for k,v in ch.items()},
          offered={k:round(v/max(1,n),2) for k,v in off.items()},
          ctx_energy0=ctx[0][0] if ctx else None, ctx_energy1=ctx[-1][0] if ctx else None,
          at_nest_frac=sum(c[3]>0 for c in ctx)/max(1,n),
          xrange=(min(xs),max(xs)) if xs else None, yrange=(min(ys),max(ys)) if ys else None,
          field=len(L['field']),deliver=len(L['deliver']),homeeat=len(L['homeeat']),digs=len(L['dig'])+len(L['recut']),
          last_field=lastf('field'),last_homeeat=lastf('homeeat'),last_eat=lastf('eat'),last_deliver=lastf('deliver'),last_out=lastf('out'),
          nest_food=st[j][1],door_food=st[j][2],field_food=st[j][3],tank_mean=st[j][4])
        out.append(o)
    json.dump(out,open(f"{D}/dead.json","w"))
    for o in out:
        print(f"{o['run']:6s} id {o['id']:9d} d@{o['death']:6d} burst{o['burst']:3d} at dx {o['dx']} y {o['y']} tank-2k {o['tank_then']} spoil {o['spoil_frac']:.2f} crop {o['crop_frac']:.2f} nest {o['at_nest_frac']:.2f} x{o['xrange']} y{o['yrange']} life f{o['field']} d{o['deliver']} h{o['homeeat']} dig{o['digs']} lastfield {o['last_field']} lasteat {o['last_eat']} store {o['nest_food']}/{o['door_food']}/{o['field_food']}")
        print("     chose",o['chose']," offered",o['offered'])
