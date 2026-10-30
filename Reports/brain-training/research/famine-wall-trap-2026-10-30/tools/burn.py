import sys, collections
for f in sys.argv[1:]:
    rows=collections.defaultdict(list)
    for l in open(f):
        if l.startswith('TRACE'):
            p=l.split(); rows[int(p[2])].append((int(p[1]),float(p[3]),int(p[4]),int(p[6]),int(p[7])))
    for side in 'LR':
        sl=[];mv=[];still=[]; dead_by=[]
        for i,v in rows.items():
            if (v[0][3]<256)!=(side=='L'): continue
            w=[r for r in v if r[0]<=2500]
            if len(w)<3: continue
            # burn only over spans with no bite gained
            d=0;t=0
            for a,b in zip(w,w[1:]):
                if b[2]==a[2] and b[1]<=a[1]: d+=a[1]-b[1]; t+=b[0]-a[0]
                mv.append(abs(b[3]-a[3])+abs(b[4]-a[4])); still.append(b[3]==a[3] and b[4]==a[4])
            if t: sl.append(d/t)
            dead_by.append(v[-1][0])
        sl.sort(); dead_by.sort()
        print(f.split('/')[-2]+'/'+f.split('/')[-1],side,'burn/frame med %.4f p10 %.4f p90 %.4f'%(sl[len(sl)//2],sl[len(sl)//10],sl[9*len(sl)//10]),'cells moved per trace-step med %d mean %.1f still %.0f%%'%(sorted(mv)[len(mv)//2],sum(mv)/len(mv),100*sum(still)/len(still)),'last seen med',dead_by[len(dead_by)//2])
