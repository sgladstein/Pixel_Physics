# usage: jam.py ARM -- ants that starved within 15 cols of the nest holding a soil pellet: how long they held it, how often
# Drop / PickUp / Eat / Feed were offered and taken in their last 2k frames, which way they walked (up/down), store at death.
import csv,glob,collections,sys,json
for arm in sys.argv[1:]:
    rows=[]
    for D in sorted(glob.glob(f"l/{arm}[0-9]*/")):
        dd=json.load(open(D+"dead.json"))
        jam={o['id']:o for o in dd if abs(o['dx'])<=15 and o['spoil_frac']>0.5}
        if not jam: continue
        hist=collections.defaultdict(list)
        for line in open(D+"dlog.csv"):
            p=line.split(',',12); i=int(p[1])
            if i in jam: hist[i].append((int(p[0]),int(p[2]),int(p[3]),float(p[4])/200,int(p[6]),int(p[7]),int(p[9]),[int(v) for v in p[10].split()]))
        for i,o in jam.items():
            h=hist[i]; fd=o['death']
            free=[r[0] for r in h if r[4]==0]
            held_since=(max(free) if free else 0)
            tank_at_pick=next((r[3] for r in h if r[0]>held_since),None)
            w=[r for r in h if r[0]>=fd-2000]
            n=max(1,len(w))
            drop_off=sum(r[7][5]>0 for r in w)/n; drop_took=sum(r[5]==5 for r in w)
            walks=[r for r in w if r[5]==0]
            up=sum(r[6]<r[2] for r in walks); down=sum(r[6]>r[2] for r in walks)
            ys=[r[2] for r in w]
            rows.append(dict(run=D.split('/')[-2],id=i,death=fd,held=fd-held_since,tank_at_pick=tank_at_pick,drop_off=drop_off,drop_took=drop_took,up=up,down=down,ymin=min(ys),ymax=max(ys),burst=o['burst'],store=o['nest_food'],feed_off=o['offered'].get('feed',0)))
    print(arm, len(rows), "home-pellet deaths")
    hs=sorted(r['held'] for r in rows); tp=sorted(r['tank_at_pick'] or 0 for r in rows)
    print(f"  held the pellet for p10/50/90 {hs[len(hs)//10]}/{hs[len(hs)//2]}/{hs[9*len(hs)//10]} frames before death; tank when it picked the pellet up p10/50/90 {tp[len(tp)//10]:.1f}/{tp[len(tp)//2]:.1f}/{tp[9*len(tp)//10]:.1f}")
    print(f"  Drop offered in {sum(r['drop_off'] for r in rows)/len(rows):.0%} of last-2k decisions, taken {sum(r['drop_took'] for r in rows)} times in all; Drop never offered for {sum(r['drop_off']==0 for r in rows)} ants")
    print(f"  walks up {sum(r['up'] for r in rows)} down {sum(r['down'] for r in rows)}; in a burst (>=5 deaths within 3k frames) {sum(r['burst']>=5 for r in rows)}; store non-empty at death {sum(r['store']>0 for r in rows)}")
    for r in rows[:8]: print("   ",r)
