# usage: hs32.py ARM... -- per arm over seeds 1-32: home deliveries (|x-128|<=8), deaths, colonies with >=12 dead; paired vs the first arm
import sys,csv,os
S="/tmp/claude-0/-home-user-Pixel-Physics/e58654de-422f-584a-a4c1-44fcef22179d/scratchpad/fl/l"
def one(a,s):
    f=f"{S}/{a}{s}/events.csv"
    if not os.path.exists(f): return None
    h=d=0
    for r in csv.DictReader(open(f)):
        if r['kind']=='deliver' and abs(int(r['x'])-128)<=8: h+=1
        elif r['kind']=='death': d+=1
    return h,d
res={a:{s:one(a,s) for s in range(1,33)} for a in sys.argv[1:]}
base=sys.argv[1]
for a,v in res.items():
    ok=[s for s in v if v[s]]
    H=sum(v[s][0] for s in ok); D=sum(v[s][1] for s in ok); col=sum(v[s][1]>=12 for s in ok)
    both=[s for s in ok if res[base].get(s)]
    better=sum(v[s][1]<res[base][s][1] for s in both); worse=sum(v[s][1]>res[base][s][1] for s in both)
    print(f"{a:8s} seeds {len(ok):2d}  home/colony {H/len(ok):6.0f}  died/colony {D/len(ok):5.1f} of 24  collapsed(>=12 dead) {col:2d}  vs {base}: fewer deaths on {better}, more on {worse}")
