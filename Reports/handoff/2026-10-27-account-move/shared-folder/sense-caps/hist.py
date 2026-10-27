import sys, collections, math
CTX=["energy","crop","spoil","at_nest","a","b","alarm","d_a","d_b","far(d_alarm)","d_energy","home_seen","home_dist","d_home","since_nest","age","self","crowd","depth","roofed","roof_cut","hole_drop"]
FN=["air","soil","spoil","food","brood","corpse","stone","water","ant","a","a_up","a_up2","b","b_up","b_up2","al","al_up","al_up2","building","stand","fall","climb","cut","hard","wet","up","depth","drop_below","headroom","roofed","nestmate","stranger","laden","hungry","larva","larva_hunger","dead","crowd","room","home","farhome(last_food)","warmth","back","lined","roof_drop"]
path=sys.argv[1]; stride=int(sys.argv[2]) if len(sys.argv)>2 else 1
ctx=[[] for _ in CTX]; feats=collections.defaultdict(list); nopt=0; nm=0
dist=[]
import collections as C
hd=[]
for k,l in enumerate(open(path)):
    if l.startswith('M,'):
        nm+=1
        if nm%stride: continue
        p=l.rstrip('\n').split(',')
        c=[float(x) for x in p[10].split()]
        for i,x in enumerate(c): ctx[i].append(x)
        h=c[12]; hd.append(32*h/(1-h) if h<1 else 1e9)
    elif l.startswith('O,'):
        if nm%stride: continue
        p=l.rstrip('\n').split(',',5)
        for op in p[5].split(';'):
            if '|' not in op: continue
            _,nz=op.split('|'); nopt+=1
            for kv in nz.split():
                i,v=kv.split(':'); feats[int(i)].append(float(v))
def q(v,p): 
    return v[min(len(v)-1,int(p*len(v)))]
print(f"decisions {nm} sampled {len(ctx[0])} options {nopt}")
print(f"{'ctx':16s} {'zero%':>6s} {'p1':>7s} {'p10':>7s} {'p50':>7s} {'p90':>7s} {'p99':>7s} {'max':>7s} {'%>=.9max':>8s} distinct")
for i,n in enumerate(CTX):
    v=sorted(ctx[i]); 
    if not v: continue
    z=sum(1 for x in v if x==0)/len(v)*100; mx=v[-1]
    top=sum(1 for x in v if mx>0 and x>=0.9*mx)/len(v)*100
    print(f"{n:16s} {z:6.1f} {q(v,.01):7.3f} {q(v,.1):7.3f} {q(v,.5):7.3f} {q(v,.9):7.3f} {q(v,.99):7.3f} {mx:7.3f} {top:8.1f} {len(set(v))}")
print(f"\n{'feature (walk/dig/drop opts)':28s} {'nz%':>6s} {'p1':>7s} {'p10':>7s} {'p50':>7s} {'p90':>7s} {'p99':>7s} {'max':>7s} distinct")
for i,n in enumerate(FN):
    v=sorted(feats.get(i,[]))
    if not v: print(f"{n:28s} {0:6.1f}"); continue
    print(f"{n:28s} {len(v)/nopt*100:6.1f} {q(v,.01):7.3f} {q(v,.1):7.3f} {q(v,.5):7.3f} {q(v,.9):7.3f} {q(v,.99):7.3f} {v[-1]:7.3f} {len(set(v))}")

hd.sort()
print("\nhome distance in cells (from home_dist):", " ".join(f"p{int(p*100)}={q(hd,p):.0f}" for p in (.1,.25,.5,.75,.9,.99)), f"max={hd[-1]:.0f}")
for lim in (40,70,100,150): print(f"  share of decisions past {lim}: {sum(1 for x in hd if x>lim)/len(hd)*100:.1f}%")
