#!/usr/bin/env python3
"""The two-pile bed (`trailfollow pile2=west`), read by seed: per arm, the
median per seed of food taken (all / east phases / west phases), the take in
the first 1,500 frames after each swap (`early`), stale visits per 1,000
ant-frames, and D1 (first pickup at the new pile); then paired sign tests
oracle/self/mute. Keyed on (arm, seed, gap), and prints the key cardinality.

    python3 scripts/twopile.py 'runs/b5-*.log'
"""
import re,glob,sys,collections,statistics as st
from math import comb
pat=sys.argv[1] if len(sys.argv)>1 else 'b5-*.log'
R=collections.defaultdict(lambda: collections.defaultdict(list))  # (arm,seed) -> field -> list over k
for f in glob.glob(pat):
    for line in open(f):
        if 'SWAP seed=' not in line: continue
        d=dict(kv.split('=',1) for kv in line.split() if '=' in kv)
        key=(d['arm'],int(d['seed']),int(d['gap']))
        k=int(d['k'])
        R[key]['k'].append(k)
        for fld in ['taken','early','stale','d1']:
            v=d[fld]; R[key][fld].append(None if v=='-' else float(v))
        R[key]['kaf'].append(float(d['stale_per_kaf']))
        R[key]['side'].append(d['side'])
keys=sorted(R); arms=sorted({k[0] for k in keys}); seeds=sorted({k[1] for k in keys}); gaps=sorted({k[2] for k in keys})
print('key cardinality', len(keys), 'arms',arms,'seeds',len(seeds),'gaps',gaps)
def summ(a,s,g):
    r=R[(a,s,g)]; ks=range(len(r['k']))
    tot=sum(r['taken']); E=sum(r['taken'][i] for i in ks if r['side'][i]=='E'); W=tot-E
    later=[i for i in ks if r['k'][i]>=1]
    return dict(taken=tot,E=E,W=W,early=sum(r['early'][i] for i in later),
        kaf=st.mean(r['kaf'][i] for i in later), d1=st.median([r['d1'][i] if r['d1'][i] is not None else 6000 for i in later]))
def sign(x,y):
    up=sum(a>b for a,b in zip(x,y)); dn=sum(a<b for a,b in zip(x,y)); n=up+dn
    p=min(1,2*sum(comb(n,i) for i in range(0,min(up,dn)+1))/2**n) if n else 1
    return f'{up}/{dn} p={p:.3f}'
for g in gaps:
    S={a:[summ(a,s,g) for s in seeds] for a in arms}
    print(f'gap {g}, {len(seeds)} seeds, medians per seed:')
    for a in arms:
        print(f'  {a:7}', ' '.join(f'{m} {st.median(x[m] for x in S[a]):.1f}' for m in ['taken','E','W','early','kaf','d1']),
              f' sumE/sumW {sum(x["E"] for x in S[a])/max(1,sum(x["W"] for x in S[a])):.2f}')
    for a,b in [('oracle','mute'),('self','mute'),('oracle','self')]:
        if a in S and b in S:
            print(f'  {a} vs {b}: taken {sign([x["taken"] for x in S[a]],[x["taken"] for x in S[b]])}; early {sign([x["early"] for x in S[a]],[x["early"] for x in S[b]])}; stale/kaf {sign([x["kaf"] for x in S[a]],[x["kaf"] for x in S[b]])}; d1 lower {sign([-x["d1"] for x in S[a]],[-x["d1"] for x in S[b]])}')
