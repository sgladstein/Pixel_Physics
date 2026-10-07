import sys, numpy as np
from scipy import ndimage
sys.path.insert(0, sys.argv[1]); sys.argv=[sys.argv[0], sys.argv[1]]
import helpers as H; G=H.G
def setup(seed,f,wd=0):
    g=G.load(f'{G.BASE}/s{seed}/map_f{f:06d}.txt'); openm,sky,nest=H.masks_v(g,'orig'); geo=G.geodesic(openm,sky)
    rows=np.arange(g.shape[0])[:,None]; mound=openm&~sky&(rows<G.GROUND_ROW)&(geo>=0); cells=nest&(geo>=0)
    return g,openm,sky,nest,geo,mound,cells
print('(a) where the level sits: mound cells holding ants, as a share of the nest top; (b) own-emission contrast vs the field\'s')
for seed,f in [(1,200000),(2,100000),(3,200000),(4,295000)]:
    g,openm,sky,nest,geo,mound,cells=setup(seed,f); s=H.srcs(seed,f,openm,1,.25,11,0); c=H.solve_v(openm,sky,s,0)
    top=np.nanmax(c[cells]); door=c[cells][np.argmin(geo[cells])]
    am=(g=='a')&mound; lv=c[am]/top
    room=cells&(ndimage.uniform_filter(openm.astype(float),size=7)>0.7)
    out=[]
    for kind,msk in (('room',room),('tunnel',cells&~room)):
        ys,xs=np.nonzero(msk); rr=np.random.default_rng(2).choice(len(ys),size=min(20,len(ys)),replace=False); sr=[];fr=[]
        for k in rr:
            y,x=ys[k],xs[k]; e=np.zeros_like(s); e[y,x]=1.0; gr=H.solve_v(openm,sky,e,0)
            fronts=[]
            for dy,dx in G.N8:
                st=6 if (dy==0 or dx==0) else 4; fy,fx=y+st*dy,x+st*dx
                if 0<=fy<g.shape[0] and 0<=fx<g.shape[1] and openm[fy,fx] and np.isfinite(c[fy,fx]): fronts.append((fy,fx))
            if not fronts: continue
            sr.append(max(abs(gr[y,x]-gr[fy,fx]) for fy,fx in fronts)/c[y,x])   # one ant-cell's own source
            fr.append(max(abs(c[fy,fx]-c[y,x])/(c[fy,fx]+c[y,x]) for fy,fx in fronts))
        out.append(f'{kind}: own {np.median(sr)*100:.2f}% vs field {np.median(fr)*100:.2f}%')
    print(f's{seed}@{f//1000}k mound ant-cells {am.sum()}: level/top p10 {np.quantile(lv,.1):.2f} med {np.median(lv):.2f} p90 {np.quantile(lv,.9):.2f}; share >= door level {np.mean(c[am]>=door):.2f} | '+'; '.join(out))
print('(c) kinesis fixed point: ants in nest+mound redistributed with density ~ 1/p(level), p = 1-(1-0.02)*c^8/(c^8+th^8), cap 4/cell, level re-solved each round')
for seed,f in [(1,200000),(3,200000)]:
    g,openm,sky,nest,geo,mound,cells=setup(seed,f); W=cells|mound; b=(g=='b')*0.25
    N=float(((g=='a')&W).sum()); s0=H.srcs(seed,f,openm,1,.25,11,0); c0=H.solve_v(openm,sky,s0,0); top0=np.nanmax(c0[cells])
    broodnear=ndimage.maximum_filter((g=='b').astype(int),size=5)>0
    dmin=geo[cells].min(); doorzone=W&(geo<=dmin+5)
    for th in (0.5,0.8,0.95):
        rho=np.where(W,s0-b,0.0)
        for it in range(60):
            c=H.solve_v(openm,sky,rho+b,0); cc=np.nan_to_num(c)
            p=1-(1-0.02)*cc**8/(cc**8+(th*top0)**8); w=np.where(W,1/p,0.0); new=N*w/w.sum()
            for _ in range(20):
                over=new>4
                if not over.any(): break
                ex=(new[over]-4).sum(); new[over]=4; free=W&(new<4); new[free]+=ex*w[free]/w[free].sum()
            rho=0.5*rho+0.5*new
        tot=rho[W].sum(); srt=np.sort(rho[W])[::-1]; k5=max(1,int(0.05*W.sum()))
        print(f'  s{seed}@{f//1000}k th={th}: nest {rho[cells].sum()/tot:.2f} mound {rho[mound].sum()/tot:.2f}; door zone (<=5 steps from nest door) {rho[doorzone].sum()/tot:.2f}; within 2 of brood {rho[W&broodnear].sum()/tot:.2f}; densest 5% of cells hold {srt[:k5].sum()/tot:.2f}; top level now {np.nanmax(c[cells])/top0:.2f}x  (start: nest {(s0-b)[cells].sum()/N:.2f})')
print('(d) slice-1 stand-ins at the switch frame (50k): ant cells by place, and crowded (>=3 other ant cells within 2)')
for seed in (1,2,3,4):
    g,openm,sky,nest,geo,mound,cells=setup(seed,50000); a=(g=='a').astype(int)
    crowd=ndimage.uniform_filter(a.astype(float),size=5,mode='constant')*25-a
    soil=(g=='s')|((g=='#')&(np.arange(g.shape[0])[:,None]>=G.GROUND_ROW))
    cov=np.zeros_like(soil)
    for d in range(1,21): cov[d:,:]|=soil[:-d,:]
    A=a.astype(bool); parts={'nest':A&cells,'mound':A&mound,'surface':A&~cells&~mound}
    print(f'  s{seed}: '+'; '.join(f'{k} {v.sum()} (covered {np.mean(cov[v]) if v.any() else 0:.2f}, crowded {np.mean(crowd[v]>=3) if v.any() else 0:.2f})' for k,v in parts.items()))
