import gzip, sys, collections
V=['walk','turn','rest','dig','pickup','putdown','eat','feed','attack']
def rows(run, fmax=10**9):
    for line in gzip.open(f'{run}/dlog.csv.gz','rt'):
        p=line.rstrip('\n').split(',')
        fr=int(p[0])
        if fr>fmax: break
        tr=[float(v) for v in p[13].split()]
        yield dict(fr=fr,id=int(p[1]),x=int(p[2]),y=int(p[3]),e=float(p[4]),crop=int(p[5]),spoil=int(p[6]),verb=V[int(p[7])],tx=int(p[8]),ty=int(p[9]),
                   offered=[int(v) for v in p[10].split()],B=tr[0],Bd=tr[1],Bd2=tr[2],food=tr[3],soil=tr[4],wlo=tr[5],whi=tr[6], ctx=[float(v) for v in p[12].split()])
for run in sys.argv[1:]:
    fmax=2200
    vc=collections.Counter(); xs=[]; ys=[]; climb=0; walks=0; onB=0; eat_off=0; n=0; hx=collections.Counter()
    for r in rows(run,fmax):
        n+=1; vc[r['verb']]+=1; xs.append(r['x']); ys.append(r['y'])
        if r['offered'][6]>0: eat_off+=1
        if r['verb']=='walk':
            walks+=1
            if r['whi']>0.05: onB+=1
            if r['Bd']>0.05: climb+=1
        hx[(r['x']-128)//5*5]+=1
    print(run, 'decisions',n, {k:round(v/n,3) for k,v in vc.most_common()})
    print('   walks with a stronger-B option',round(onB/max(walks,1),3),' walks that climbed B',round(climb/max(walks,1),3),' eat offered',round(eat_off/n,3))
    print('   x offset from nest (5-col bins):',sorted(hx.items()))
    print('   y range',min(ys),max(ys), 'median y', sorted(ys)[len(ys)//2])
