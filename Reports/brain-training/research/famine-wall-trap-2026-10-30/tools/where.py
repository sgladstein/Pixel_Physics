import sys, collections
f=sys.argv[1]; lo,hi=int(sys.argv[2]),int(sys.argv[3]); xmin,xmax=int(sys.argv[4]),int(sys.argv[5])
rows=collections.defaultdict(list)
for l in open(f):
    if l.startswith('TRACE'):
        p=l.split(); rows[int(p[2])].append((int(p[1]),float(p[3]),int(p[4]),int(p[6]),int(p[5])))
xs=[]; pd=0; n=0; bites=0
for i,v in rows.items():
    if not xmin<=v[0][3]<=xmax: continue
    for a,b in zip(v,v[1:]):
        if lo<=b[0]<=hi:
            xs.append(b[3]); n+=1; pd+= b[4]<=2; bites+=b[2]-a[2]
xs.sort()
print(f.split('/')[-1],'samples',n,'x deciles',[xs[len(xs)*k//10] for k in range(10)] if xs else '-','within 2 of a plant %.0f%%'%(100*pd/max(n,1)),'bites',bites)
