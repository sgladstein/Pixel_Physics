# Of the ants that starved, how many were walled in holding spoil within 300 frames of dying. Args: run dirs.
import csv,sys
tot=[0,0]
for D in sys.argv[1:]:
    w={}
    for l in open(f"{D}.log"):
        if l.startswith("WALLED"):
            for t in l.split()[1:]:
                i,f=t.split(':'); w[int(i)]=int(f)
    dead=[(int(r['id']),int(r['frame'])) for r in csv.DictReader(open(f"{D}/events.csv")) if r['kind']=='death']
    n=sum(1 for i,f in dead if i in w and f-w[i]<=300)
    tot[0]+=len(dead); tot[1]+=n
    print(f"{D.split('/')[-1]:8s} starved {len(dead):3d}  walled in holding soil at death {n:3d}   ants ever walled in {len(w)}")
print("total", tot)
