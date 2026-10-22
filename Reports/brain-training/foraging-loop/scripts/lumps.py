# Where and when the crop's food is swallowed whole by choice (a digest tick paying > 10 face: the mind's Eat verb,
# or the game's packed-lunch finish), against the crop's slow digestion.
import csv,sys,collections,bisect
D=sys.argv[1]; NEST=128
tr=collections.defaultdict(list)
for r in csv.DictReader(open(f"{D}/trace.csv")):
    tr[int(r['id'])].append((int(r['frame']),int(r['x']),float(r['tank']),int(r['crop']),int(r['trip'])))
fr={i:[t[0] for t in v] for i,v in tr.items()}
def at(i,f):
    v=tr.get(i); 
    if not v: return None
    j=max(0,bisect.bisect_right(fr[i],f)-1); return v[j]
dist=collections.Counter(); tank=collections.Counter(); trip=collections.Counter(); n=0; w=0
for r in csv.DictReader(open(f"{D}/events.csv")):
    if r['kind']!='eat' or float(r['v'])<=10: continue
    s=at(int(r['id']),int(r['frame']))
    if not s: continue
    n+=1; w+=float(r['v'])
    d=abs(s[1]-NEST); dist['home (<=12)' if d<=12 else 'field (>12)']+=1
    t=s[2]; tank['<3' if t<3 else '3-5.5' if t<5.5 else '5.5-20' if t<20 else '20+']+=1
    trip['trip load' if s[4]>0 else 'not a trip load']+=1
print(D, n, "lumps", f"{w:,.0f} J"); print("  where:",dict(dist)); print("  eater's tank:",dict(tank)); print("  load:",dict(trip))
