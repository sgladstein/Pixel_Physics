"""Each final crater / nest cell (nestcount's rule): was its own cell cut as original ground ('dig'), re-cut ('recut'),
or never cut at that spot (opened by ground falling away)? And per crater column: how the datum-row (roof) cell went."""
import sys,csv,collections,os
sys.path.insert(0,'/home/user/cloud/scratch/inrun'); import nestcount
def cells(d):
    t=open(f'{d}/nestmap.txt').read().split('\n'); head=t[0].split(' '); x0,y0,legend=int(head[0]),int(head[1]),head[2].split(',')
    grid=[r for r in t[1:] if r]; surf=[int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    e=chr(65+legend.index('empty'))
    return x0,y0,grid,surf,e
groups=collections.defaultdict(collections.Counter)
for d in sys.argv[1:]:
    d=d.rstrip('/'); g=os.path.basename(d).rsplit('_',1)[0]
    x0,y0,grid,surf,e=cells(d)
    first={}
    for r in csv.DictReader(open(d+'/events.csv')):
        if r['kind'] in ('dig','recut'):
            first.setdefault((int(r['x']),int(r['y'])),r['kind'])
    # reproduce nestcount's nest set
    n=nestcount.count(d,200000)
    # recompute sets: crater = open below datum within 20 cols not nest. Use nestcount internals by re-flood: simplest -> copy logic
    import importlib
    src=open('/home/user/cloud/scratch/inrun/nestcount.py').read()
    H,W=len(grid),len(grid[0])
    # crater/nest labelling via nestcount code path: monkeypatch to expose sets
    ns={}
    code=src.replace("    out = {'nest': len(nest)","    ns_out.update(nest=nest, seen=seen, below=below, e=e, door=door)\n    out = {'nest': len(nest)")
    g2={'ns_out':ns,'__name__':'x'}; exec(code,g2); g2['count'](d,200000)
    nest=ns['nest']
    for dy in range(H):
        for dx in range(ns['door']-20,ns['door']+21):
            if grid[dy][dx]==e and ns['below'](dx,dy):
                k='nest' if (dx,dy) in nest else 'crater'
                groups[g][k+':'+first.get((x0+dx,y0+dy),'never')]+=1
for g,c in groups.items():
    for k in ('crater','nest'):
        tot=sum(v for kk,v in c.items() if kk.startswith(k+':'))
        print(g.ljust(9),k.ljust(6),tot,' '.join(f"{kk.split(':')[1]} {v/tot:.2f}" for kk,v in sorted(c.items()) if kk.startswith(k+':')) if tot else '')
