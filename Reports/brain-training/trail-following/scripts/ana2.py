import gzip, sys, collections
exec(open('ana1.py').read().split('for run in sys.argv')[0])
for run in sys.argv[1:]:
    per=collections.defaultdict(lambda: dict(n=0,w=0,climb=0,far=0,Bs=[],lastfr=0,e=[],feed=0))
    for r in rows(run,4000):
        a=per[r['id']]; a['n']+=1; a['far']=max(a['far'],abs(r['x']-128)); a['Bs'].append(r['ctx'][5]); a['lastfr']=r['fr']; a['e'].append(r['e'])
        if r['verb']=='walk':
            a['w']+=1; a['climb']+= r['Bd']>0.05
        if r['verb']=='feed': a['feed']+=1
    deaths={int(l.split(',')[1]):int(l.split(',')[0]) for l in open(f'{run}/events.csv') if ',death,' in l}
    print(run, 'ant: farthest cols from nest | share of walks climbing B | B at head (median, ctx) | feed share | died at')
    for i,a in sorted(per.items()):
        Bs=sorted(a['Bs'])
        print(f"  {i:3d}: far {a['far']:4d}  climb {a['climb']/max(a['w'],1):.2f}  B {Bs[len(Bs)//2]:.2f}  feed {a['feed']/a['n']:.2f}  died {deaths.get(i,'-')}")
