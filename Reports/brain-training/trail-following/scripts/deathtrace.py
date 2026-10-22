import gzip, sys, collections, statistics as st
exec(open('ana1.py').read().split('for run in sys.argv')[0])
for arm in sys.argv[1:]:
    tot=collections.Counter(); kinds=collections.Counter(); xs=[]; n=0
    for s in (1,2,3):
        run=f'{arm}-{s}'
        deaths={int(l.split(',')[1]):int(l.split(',')[0]) for l in open(f'{run}/events.csv') if ',death,' in l}
        hist=collections.defaultdict(lambda: collections.deque(maxlen=200))
        for r in rows(run):
            if r['id'] in deaths and r['fr']<=deaths[r['id']]: hist[r['id']].append(r)
        for i,h in hist.items():
            if not h: continue
            n+=1; last=h[-1]; xs.append(abs(last['x']-128))
            vc=collections.Counter(r['verb'] for r in h)
            far=abs(last['x']-128)
            nearfood=any(r['food']>0 for r in h)
            kind=('home' if far<=12 else 'mid' if far<=40 else 'far')+(' sawfood' if nearfood else '')+(' spoil' if last['spoil'] else '')
            kinds[kind]+=1
            for k,v in vc.items(): tot[k]+=v
            climbs=sum(1 for r in h if r['verb']=='walk' and r['Bd']>0.05); walks=sum(1 for r in h if r['verb']=='walk')
            tot['climbB']+=climbs; tot['walks2']+=walks
    T=sum(v for k,v in tot.items() if k not in('climbB','walks2'))
    print(arm,'dead ants traced',n,'| where they die',sorted(kinds.items(),key=lambda x:-x[1]))
    print('   |x-nest| at death median',st.median(xs) if xs else None,' last-200 verb mix',{k:round(v/T,2) for k,v in tot.most_common() if k not in('climbB','walks2')},' walks climbing B',round(tot['climbB']/max(tot['walks2'],1),2))
