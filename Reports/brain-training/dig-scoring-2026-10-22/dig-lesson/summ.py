import sys, os, csv, collections, statistics as St
sys.path.insert(0,'/mnt/project-files/teacher-lane/nestmaps-1022')
import nestcount as N, roomshape as R
D='/tmp/claude-0/-home-user-Pixel-Physics/d8079a03-3861-5d88-a8a6-9ab879927dc0/scratchpad/ol/cg/run'
arms=sys.argv[1:]; res={}
for a in arms:
    for s in range(1,33):
        d=f'{D}/{a}_{s}'
        if not os.path.exists(d+'/nestmap.txt'): continue
        c=collections.Counter()
        for e in csv.DictReader(open(d+'/events.csv')):
            k=e['kind']
            if k=='deliver': c['food']+=abs(int(e['x'])-128)<=8
            elif k=='death': c['starved']+=e['v']=='1'
            elif k=='dig' and (int(e['x'])-128)**2<=400: c['cuts']+=1
        n=N.count(d); p=R.parts(d)
        res[(a,s)]=dict(shape=p['score'],rooms2=int(len(p['rooms'])>1),nest=n['nest'],crater=n['crater'],spoil_in=n['spoil_in'],kept_late=n['nestcuts_late'],**c)
keys=['shape','rooms2','nest','crater','kept_late','spoil_in','cuts','food','starved']
print('n per arm',[sum(1 for (a,s) in res if a==x) for x in arms])
print(f"{'':10s}"+''.join(f'{a:>14s}' for a in arms)+'   paired (2nd vs 1st): up/down')
for k in keys:
    row=f'{k:10s}'
    for a in arms:
        v=[res[(a,s)].get(k,0) for s in range(1,33) if (a,s) in res]
        row+=f'{(sum(v) if k=="rooms2" else St.median(v)):14.1f}'
    if len(arms)>1:
        b,t=arms[0],arms[1]; ss=[s for s in range(1,33) if (b,s) in res and (t,s) in res]
        up=sum(res[(t,s)].get(k,0)>res[(b,s)].get(k,0) for s in ss); dn=sum(res[(t,s)].get(k,0)<res[(b,s)].get(k,0) for s in ss)
        row+=f'   {up}/{dn}'
    print(row)
