import sys, numpy as np
BASE='/mnt/project-files/deep-trace/baseline/3ba1e7bd5'
GROUND_ROW=60
OPEN=set('.abfcx')
def load(p):
    with open(p) as fh:
        lines=fh.read().split('\n')
    rows=[l for l in lines[1:] if l]
    w=max(len(r) for r in rows)
    return np.array([list(r.ljust(w,'#')) for r in rows])
out=[]
for s in (1,2,3,4):
    for f in (100000,200000,295000):
        g=load(f'{BASE}/s{s}/map_f{f:06d}.txt')
        h,w=g.shape
        openm=np.isin(g,list(OPEN)); rows=np.arange(h)[:,None]
        nest=openm&(rows>GROUND_ROW)
        b=(g=='b')
        def within(r):
            acc=np.zeros_like(b)
            ys,xs=np.nonzero(b)
            for y,x in zip(ys,xs):
                acc[max(0,y-r):y+r+1,max(0,x-r):x+r+1]=True
            return acc
        n=nest.sum()
        c6=(within(6)&nest).sum(); c10=(within(10)&nest).sum()
        out.append((s,f,int(n),int(b.sum()),c6/max(n,1),c10/max(n,1)))
for r in out: print('s%d f%d nest_cells=%d brood_cells=%d within6=%.2f within10=%.2f'%r)
a6=sorted(r[4] for r in out); print('within6 median %.2f range %.2f-%.2f'%(np.median(a6),a6[0],a6[-1]))
