import numpy as np
BASE='/mnt/project-files/deep-trace/baseline/3ba1e7bd5'; GR=60; REACH=20
def load(p):
    with open(p) as f:
        x0,y0,w,h=map(int,f.readline().split()); rows=[f.readline().rstrip('\n') for _ in range(h)]
    return np.array([list(r.ljust(w,'#')[:w]) for r in rows])
tot=dict(nest=0,mound=0,open=0); per=[]
for s in (1,2,3,4):
    for f in range(100000,300001,1000):
        p=f'{BASE}/s{s}/map_f{f:06d}.txt'
        try: g=load(p)
        except FileNotFoundError: continue
        solid=np.isin(g,['s'] if __import__('os').environ.get('SONLY') else ['#','s']); h,w=g.shape
        # cover[y,x]: any solid in rows y-REACH..y-1 of column x
        cs=np.vstack([np.zeros((1,w),int),np.cumsum(solid,axis=0)])
        ys=np.arange(h)[:,None]; lo=np.clip(ys-REACH,0,h); 
        cover=(cs[ys,np.arange(w)[None,:]]-cs[lo,np.arange(w)[None,:]])>0
        a=(g=='a'); rows=np.arange(h)[:,None]
        n_nest=int((a&cover&(rows>GR)).sum()); n_mound=int((a&cover&(rows<=GR)).sum()); n_open=int((a&~cover).sum())
        tot['nest']+=n_nest; tot['mound']+=n_mound; tot['open']+=n_open
        if f in (100000,200000,295000): per.append((s,f,n_nest,n_mound,n_open))
for r in per: print('s%d f%d covered-ants: dug-nest %d, mound/above-ground %d; open %d'%r)
c=tot['nest']+tot['mound']; print('ALL maps 100k-300k, ant-cells:',tot,' share of under_cover ants that are NOT in the dug nest: %.2f'%(tot['mound']/max(c,1)))
