import re,glob,os,statistics as st
D=os.path.dirname(os.path.abspath(__file__))
rows={}
for fn in sorted(glob.glob(D+'/*_s*.txt')):
    arm,seed=re.match(r'(\w+)_s(\d+)',os.path.basename(fn)).groups()
    txt=open(fn).read()
    if 'plant_m0 done' not in txt: continue
    secs=float(re.search(r'plant_m0 done: ([\d.]+)',txt).group(1))
    for fr in (150000,450000):
        P=[dict(re.findall(r'(\w+)=(\S+)',l)) for l in txt.splitlines() if l.startswith('ROOTS f=%d '%fr)]
        leafy=[d for d in P if int(d['leaf'])>=5]
        rl=[d for d in leafy if int(d['contact'])==0]
        cen=[l for l in txt.splitlines() if l.startswith('f=%d '%fr)][0]
        sp={m[0]:(int(m[1]),int(m[2])) for m in re.findall(r'(\w+) plants=(\d+) bank=\d+ leaf=(\d+)',cen)}
        roles=sum(1 for k,v in sp.items() if v[1]>=100)
        lc=re.search(r'light_cov=([\d.]+)',cen).group(1)
        rl_leaf=sum(int(d['leaf']) for d in rl); all_leaf=sum(int(d['leaf']) for d in leafy)
        loss=[l for l in txt.splitlines() if l.startswith('ROOTLOSS f=%d'%fr)]
        adv=re.search(r'adventitious=(\d+) dieback_kept=(\d+)',loss[0]).groups() if loss else ('?','?')
        rows[(arm,int(seed),fr)]=dict(leafy=len(leafy),rootless=len(rl),rl_leaf=rl_leaf,all_leaf=all_leaf,sp=sp,roles=roles,lc=float(lc),adv=adv,secs=secs)
for fr in (150000,450000):
    print('== frame',fr)
    print('seed | arm | leafy plants | rootless | leaf on rootless | tree/shrub/herb/creeper leaf | roles(100+ leaf) | light under cover | adv/kept | secs')
    for seed in range(1,9):
        for arm in ('off','on'):
            r=rows.get((arm,seed,fr))
            if not r: continue
            sp=r['sp']; print(seed,arm,r['leafy'],r['rootless'],f"{r['rl_leaf']}/{r['all_leaf']}",'/'.join(str(sp.get(k,(0,0))[1]) for k in ('tree','shrub','herb','creeper')),r['roles'],r['lc'],'/'.join(r['adv']),r['secs'],sep=' | ')
    for arm in ('off','on'):
        R=[rows[(arm,s,fr)] for s in range(1,9) if (arm,s,fr) in rows]
        if R: print(arm,'n',len(R),'rootless sum',sum(r['rootless'] for r in R),'leafy sum',sum(r['leafy'] for r in R),'rootless leaf',sum(r['rl_leaf'] for r in R),'roles mean',round(st.mean(r['roles'] for r in R),2),
          'leaf by sp',{k:sum(r['sp'].get(k,(0,0))[1] for r in R) for k in ('tree','shrub','herb','creeper')},'light_cov mean',round(st.mean(r['lc'] for r in R),3),'secs mean',round(st.mean(r['secs'] for r in R)))
