# usage: heading_all.py ARM -- every decision of every ant past 60 cols from the nest: chosen Walk's column step (home/out/same),
# split by what it holds (empty / crop food / soil pellet) and by tank.
import glob,collections,sys
for arm in sys.argv[1:]:
    agg=collections.defaultdict(collections.Counter)
    for D in sorted(glob.glob(f"l/{arm}[0-9]*/")):
        for line in open(D+"dlog.csv"):
            p=line.split(',',12)
            x=int(p[2])
            if abs(x-128)<60: continue
            tank=float(p[4])/200; crop=int(p[5]); sp=int(p[6]); v=int(p[7]); tx=int(p[8])
            k=("pellet" if sp else "food" if crop else "empty", "<5 tanks" if tank<5 else ">=5")
            c=agg[k]; c['n']+=1
            if v==0:
                d=abs(tx-128)-abs(x-128); c['walk']+=1; c['home' if d<0 else 'out' if d>0 else 'same']+=1
    print(arm)
    for k in sorted(agg):
        c=agg[k]; w=max(1,c['walk'])
        print(f"  {k[0]:7s} {k[1]:9s} decisions {c['n']:8d}  of walks: homeward {c['home']/w:.0%} out {c['out']/w:.0%} same {c['same']/w:.0%}   net homeward bias {(c['home']-c['out'])/w:+.1%}")
