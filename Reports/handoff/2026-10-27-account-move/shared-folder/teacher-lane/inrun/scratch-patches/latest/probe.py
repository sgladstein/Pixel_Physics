import csv,collections,sys
# For each ant alive at 20k: did it leave home (hungryout event) in 20k..23k? starved by 26k? Per arm and per genome tag.
F,W=20000,3000
def run(d):
    tag={r['id']:r['tag'] for r in csv.DictReader(open(d+'/ants.csv'))}
    alive=set(l.split()[0] for l in open(d+'/ants_f0020000.txt'))
    home={l.split()[0] for l in open(d+'/ants_f0020000.txt') if abs(int(l.split()[1])-128)<=12}
    left=set(); dead=set(); e_at={}
    for e in csv.DictReader(open(d+'/events.csv')):
        f=int(e['frame'])
        if e['kind']=='hungryout' and F<f<=F+W: left.add(e['id'])
        if e['kind']=='death' and f>F and e['v']=='1': dead.add(e['id'])
    return [(i,tag.get(i,'?'),i in home,i in left,i in dead) for i in alive]
for arm in ('ctl','st'):
    rows=[r for s in range(1,9) for r in run(f'probe/{arm}_{s}')]
    h=[r for r in rows if r[2]]
    print(arm,'alive',len(rows),'home',len(h),'home ants that left in 3k:',sum(r[3] for r in h),'starved by 26k:',sum(r[4] for r in rows))
    if arm=='st':
        g=collections.defaultdict(lambda:[0,0])
        for r in h: g[r[1]][0]+=1; g[r[1]][1]+=r[3]
        print(' by genome (home ants, left):',sorted(((k,v[0],v[1]) for k,v in g.items()),key=lambda x:-x[2]/max(x[1],1))[:12])
