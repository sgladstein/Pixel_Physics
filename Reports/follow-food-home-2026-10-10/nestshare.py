#!/usr/bin/env python3
"""nestshare.py RUN [RUN...] -- adults in the dug nest, frames 100-200k: share of colony.csv rows in `nest`
and `mound_in`, and the share of nest time spent in stays over 5,000 frames (an id's consecutive nest samples)."""
import csv,sys,collections
for d in sys.argv[1:]:
    z=collections.Counter(); seq=collections.defaultdict(list); fr=set()
    for r in csv.DictReader(open(d+'/colony.csv')):
        f=int(r['frame'])
        if 100000<=f<200000:
            z[r['zone']]+=1; seq[r['id']].append((f,r['zone']=='nest')); fr.add(f)
    step=min(b-a for a,b in zip(sorted(fr),sorted(fr)[1:]))
    n=sum(z.values()); nest=z['nest']
    # stays: runs of consecutive samples in nest
    long_=0; tot=0
    for i,s in seq.items():
        run=0
        for k,(f,inn) in enumerate(s+[(None,False)]):
            if inn: run+=1
            else:
                tot+=run
                if run*step>5000: long_+=run
                run=0
    print(f"{d.split('/')[-1]:18} nest {100*nest/n:4.1f}%  mound_in {100*z['mound_in']/n:4.1f}%  stays>5k {100*long_/max(tot,1):4.1f}% of nest time  (sample every {step})")
