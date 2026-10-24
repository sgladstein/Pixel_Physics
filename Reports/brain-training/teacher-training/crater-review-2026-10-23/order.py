"""Crater columns: was each crater cell cut before or after its column's roof (the topmost ground cells, rows datum..datum+2) opened?"""
import sys,csv,collections,os
groups=collections.defaultdict(collections.Counter)
for d in sys.argv[1:]:
    d=d.rstrip('/'); g=os.path.basename(d).rsplit('_',1)[0]
    surf=[int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    t=open(f'{d}/nestmap.txt').read().split('\n'); head=t[0].split(' '); x0,y0,legend=int(head[0]),int(head[1]),head[2].split(',')
    grid=[r for r in t[1:] if r]; e=chr(65+legend.index('empty'))
    gnd={chr(65+k) for k,m in enumerate(legend) if m in ('soil','packedsoil','spoil','stone')}
    cut={}
    for r in csv.DictReader(open(d+'/events.csv')):
        if r['kind']=='dig': cut.setdefault((int(r['x']),int(r['y'])),int(r['frame']))
    for x in range(108,149):
        dx=x-x0; s=surf[x]
        col=[grid[y-y0][dx] for y in range(s,y0+len(grid))]
        if any(c in gnd for c in col[:1]): continue   # datum cell is ground: not a sky-open column
        # sky-open run from datum down
        k=0
        while k<len(col) and col[k] not in gnd: k+=1
        if k<4: continue
        roof=[cut.get((x,s+j)) for j in range(3)]
        roof=[f for f in roof if f is not None]
        if not roof: groups[g]['roof never cut (stamp or fell)']+=k; continue
        rf=max(roof)
        for j in range(3,k):
            f=cut.get((x,s+j))
            if f is None: groups[g]['cell never cut']+=1
            elif f<rf: groups[g]['cut UNDER roof, roof cut later']+=1
            else: groups[g]['cut after roof opened (from open sky)']+=1
for g,c in groups.items():
    n=sum(c.values()); print(g.ljust(9),n,' | '.join(f'{k} {v/n:.2f}' for k,v in sorted(c.items())))
