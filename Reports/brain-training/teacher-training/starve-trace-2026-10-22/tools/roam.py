# usage: roam.py ARM -- over runs l/ARM*: share of ant-time past 40 / at the walls; excursions past 40 cols and how many came back;
# for starved ants at the walls: tank when they last left the 40-col band, frames out, home sense (anchor) at death.
import csv,glob,collections,sys
for arm in sys.argv[1:]:
    tot=far=wall=0; exc=0; back=0; dead_out=0
    rows=[]
    for D in sorted(glob.glob(f"l/{arm}[0-9]*/")):
        deaths={int(r['id']):int(r['frame']) for r in csv.DictReader(open(D+"events.csv")) if r['kind']=='death'}
        tr=collections.defaultdict(list)
        for r in csv.DictReader(open(D+"trace.csv")):
            tr[int(r['id'])].append((int(r['frame']),int(r['x']),int(r['y']),float(r['tank']),int(r.get('ax') or -1),int(r['spoil']),int(r['crop'])))
        for i,t in tr.items():
            out=False
            for (f,x,y,tk,ax,sp,cr) in t:
                d=abs(x-128); tot+=1; far+=d>40; wall+=d>=100
                if d>60 and not out: out=True; exc+=1
                elif d<=30 and out: out=False; back+=1
            if out: dead_out+= i in deaths
            if i in deaths and abs(t[-1][1]-128)>=60:
                lastin=max([k for k,r in enumerate(t) if abs(r[1]-128)<=40],default=0)
                L=t[lastin]
                first_wall=next((r[0] for r in t[lastin:] if abs(r[1]-128)>=100),None)
                rows.append((D.split('/')[-2],i,deaths[i],L[0],L[3],deaths[i]-L[0],first_wall,t[-1][1],t[-1][4],max(r[3] for r in t[lastin:])))
    print(f"{arm}: ant-time past 40 cols {far/tot:.1%}, at walls (>=100) {wall/tot:.1%}; trips past 60 cols {exc}, came back to 30 {back}; died out there {dead_out}")
    if rows:
        tk=sorted(r[4] for r in rows); out=sorted(r[5] for r in rows)
        print(f"   starved past 60 cols: {len(rows)}; tank when last inside 40 cols p10/50/90 {tk[len(tk)//10]:.1f}/{tk[len(tk)//2]:.1f}/{tk[9*len(tk)//10]:.1f}; frames from leaving to death p10/50/90 {out[len(out)//10]}/{out[len(out)//2]}/{out[9*len(out)//10]}")
        ah=sum(1 for r in rows if r[8]>=0 and abs(r[8]-128)<=15)
        print(f"   home sense (forage_anchor) within 15 cols of the nest at death: {ah} of {len(rows)}; examples", [(r[0],r[1],'x',r[7],'anchor',r[8]) for r in rows[:6]])
