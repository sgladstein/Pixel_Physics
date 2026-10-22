# Where starved ants were and what the nest held when they died. Args: run dirs.
import csv,sys,collections,bisect
for D in sys.argv[1:]:
    tr=collections.defaultdict(list)
    for r in csv.DictReader(open(f"{D}/trace.csv")): tr[int(r['id'])].append(r)
    st=[(int(r['frame']),int(r['nest_food']),int(r['door_food'])) for r in csv.DictReader(open(f"{D}/stock.csv"))]
    sf=[s[0] for s in st]
    rows=[]
    for r in csv.DictReader(open(f"{D}/events.csv")):
        if r['kind']!='death': continue
        i=int(r['id']); f=int(r['frame']); t=tr[i][-1] if tr[i] else None
        j=max(0,bisect.bisect_right(sf,f)-1)
        if t: rows.append((f,abs(int(t['x'])-128),int(t['y']),int(t['crop']),int(t['spoil']),st[j][1],int(t['since_nest'])))
    home=sum(1 for x in rows if x[1]<=6); spoil=sum(x[4] for x in rows); nf=sum(1 for x in rows if x[5]>0)
    fr=sorted(x[0] for x in rows)
    print(f"{D.split('/')[-1]:9s} starved {len(rows)}: within 6 cols of the nest {home}, holding spoil {spoil}, nest store non-empty at death {nf}; death frames p10/p50/p90 {fr[len(fr)//10] if fr else 0}/{fr[len(fr)//2] if fr else 0}/{fr[9*len(fr)//10] if fr else 0}")
