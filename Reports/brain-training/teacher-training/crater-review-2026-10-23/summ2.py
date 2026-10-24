import sys,csv,collections,glob,statistics as st,os
sys.path.insert(0,'/home/user/cloud/scratch/inrun'); sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
import nestcount, roomshape, nestfix
G=collections.defaultdict(dict)
for d in sys.argv[1:]:
    d=d.rstrip('/')
    if not os.path.exists(d+'/nestmap.txt'): continue
    name=os.path.basename(d); g,seed=name.rsplit('_',1)
    c=collections.Counter()
    for e in csv.DictReader(open(d+'/events.csv')):
        k=e['kind']
        if k=='deliver': c['home8']+=abs(int(e['x'])-128)<=8
        elif k=='death': c['starved']+=e['v']=='1'
        elif k=='dig':
            x=int(e['x']); c['cuts']+=abs(x-128)<=20
            c['far_shallow']+= abs(x-128)>20 and int(e['v'])<3
    n=nestfix.count(d)
    try: sh=roomshape.score(d)
    except Exception: sh=float('nan')
    G[g][seed]=dict(home8=c['home8'],starved=c['starved'],cuts=c['cuts'],far=c['far_shallow'],nest=n['nest'],crater=n['crater'],nest_old=n['nest_old'],crater_old=n['crater_old'],shape=sh)
def show(gs):
    for g in gs:
        v=list(G[g].values())
        if not v: continue
        m=lambda k: st.median(x[k] for x in v)
        print(f"{g:14} n={len(v)} food {m('home8'):6.0f} starved {m('starved'):4.1f} cuts {m('cuts'):4.0f} shallow-cuts-beyond-20 {m('far'):4.0f} | nest {m('nest'):4.0f} crater {m('crater'):4.0f} (old meter {m('nest_old'):4.0f}/{m('crater_old'):4.0f}) rooms {m('shape'):5.1f}")
def paired(a,b,k):
    s=[s for s in G[a] if s in G[b]]
    up=sum(G[b][x][k]>G[a][x][k] for x in s); dn=sum(G[b][x][k]<G[a][x][k] for x in s); return f"{k} {up}up/{dn}down"
if __name__=='__main__':
    show(sorted(G))
    for g in sorted(set(k.split('-',1)[1] for k in G)):
        for arm in ('v3','v6','v3s','roof','both','rooms'):
            if f'{arm}-{g}' in G and f'ctl-{g}' in G:
                print('  paired',arm,'vs ctl',g, ', '.join(paired(f'ctl-{g}',f'{arm}-{g}',k) for k in ('home8','nest','crater','starved','shape')))
