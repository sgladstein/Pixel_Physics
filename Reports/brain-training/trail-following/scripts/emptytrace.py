import gzip, sys, collections, statistics as st
exec(open('ana1.py').read().split('for run in sys.argv')[0])
for arm in sys.argv[1:]:
    bins=collections.Counter(); n=0; rev=0; steps=0; foodseen=0; Bhere=[]
    for s in (1,2,3):
        lastdx={}
        for r in rows(f'{arm}-{s}', 100000):
            if r['crop']!=0 or r['spoil'] or abs(r['x']-128)<=4 or r['verb']!='walk': continue
            n+=1; bins[min(abs(r['x']-128)//10*10,60)]+=1
            if r['food']>0: foodseen+=1
            Bhere.append(r['B'])
            dx=r['tx']-r['x']
            if dx!=0:
                steps+=1
                if r['id'] in lastdx and lastdx[r['id']]*dx<0: rev+=1
                lastdx[r['id']]=dx
    print(arm,'empty walks out of nest',n,' by |x-nest|',{k:round(v/n,2) for k,v in sorted(bins.items())},' reversals per sideways step',round(rev/max(steps,1),2),' food in view',round(foodseen/n,3))
