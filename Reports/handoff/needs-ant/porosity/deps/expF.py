import sys, numpy as np
from scipy import ndimage
sys.path.insert(0, sys.argv[1]); sys.argv=[sys.argv[0], sys.argv[1]]
import helpers as H; G=H.G
rng=np.random.default_rng(7)
print('(1) shipped larva_scent (brood.rs:1281): vector sum need*(dx,dy)/d2 over 13x13, walls ignored; strength len/(len+0.25)')
for seed,f in [(1,200000),(3,200000),(4,295000)]:
    g=G.load(f'{G.BASE}/s{seed}/map_f{f:06d}.txt'); openm,sky,nest=H.masks_v(g,'orig'); B=(g=='b'); h,w=g.shape
    free=nest&openm&~B; dB=ndimage.distance_transform_cdt(~B,metric='chessboard')
    lay=free&(dB==1); far=free&(dB>=4)&(dB<=6)
    for tag,frac in (('all hungry',1.0),('20% hungry',0.2),('5% hungry',0.05)):
        need=(B&(rng.random(B.shape)<frac)).astype(float)
        vx=np.zeros((h,w)); vy=np.zeros((h,w)); P=np.pad(need,6)
        for dy in range(-6,7):
            for dx in range(-6,7):
                if dx==0 and dy==0: continue
                sh=P[6+dy:6+dy+h,6+dx:6+dx+w]; d2=dx*dx+dy*dy; vx+=sh*dx/d2; vy+=sh*dy/d2
        L=np.hypot(vx,vy); st=L/(L+0.25); D=np.array(G.N8)
        cosb=np.stack([(vx*ddx+vy*ddy)/np.maximum(L,1e-12)/np.hypot(ddy,ddx) for ddy,ddx in D]); hd=cosb.argmax(0)
        ty=np.clip(np.arange(h)[:,None]+D[hd][...,0],0,h-1); tx=np.clip(np.arange(w)[None,:]+D[hd][...,1],0,w-1)
        intowall=free&(L>0)&~openm[ty,tx]
        hung=need>0; adjH=ndimage.maximum_filter(hung.astype(int),size=3)>0
        starts=list(zip(*np.nonzero(free&(L>0)))); ok=0
        for (y,x) in starts:
            for _ in range(40):
                if adjH[y,x]: break
                best=None;bv=-2
                for ddy,ddx in G.N8:
                    ny,nx=y+ddy,x+ddx
                    if 0<=ny<h and 0<=nx<w and openm[ny,nx]:
                        cv=(vx[y,x]*ddx+vy[y,x]*ddy)/max(L[y,x],1e-12)/np.hypot(ddy,ddx)
                        if cv>bv: bv,best=cv,(ny,nx)
                if best is None or L[y,x]==0: break
                y,x=best
            ok+=adjH[y,x]
        print(f'  s{seed}@{f//1000}k {tag:10s}: strength at lay sites (touching brood) med {np.median(st[lay]):.2f} vs 4-6 cells out {np.median(st[far]) if far.any() else float("nan"):.2f}; heading into soil {intowall.sum()/max((free&(L>0)).sum(),1)*100:.0f}% of cells in reach; follower ends by a hungry larva {ok/max(len(starts),1)*100:.0f}% (already there {np.mean(adjH[free&(L>0)])*100:.0f}%)')
print('(2) sealed field: share of nest cells whose descent reaches sky when a step needs a minimum difference')
for seed,f in [(1,200000),(2,100000),(3,200000),(4,295000)]:
    g=G.load(f'{G.BASE}/s{seed}/map_f{f:06d}.txt'); openm,sky,nest=H.masks_v(g,'orig'); geo=G.geodesic(openm,sky); cells=nest&(geo>=0)
    c=H.solve_v(openm,sky,H.srcs(seed,f,openm,1,.25,11,0),0); top=np.nanmax(c[cells]); res=[]
    for name,ok in (('ideal',lambda a,b: b<a-1e-15),('u16 step',lambda a,b: a-b>=top/65536),('8-bit step',lambda a,b: a-b>=top/256),('1% rel',lambda a,b: (a-b)/(a+b)>=0.01)):
        n=0;out=0
        for (y,x) in zip(*np.nonzero(cells)):
            if not np.isfinite(c[y,x]): continue
            n+=1
            for _ in range(400):
                if sky[y,x]: break
                nb=[(c[y+a,x+b],y+a,x+b) for a,b in G.N8 if 0<=y+a<c.shape[0] and 0<=x+b<c.shape[1] and openm[y+a,x+b] and np.isfinite(c[y+a,x+b])]
                v,ny,nx=min(nb)
                if not ok(c[y,x],v): break
                y,x=ny,nx
            out+=sky[y,x]
        res.append(f'{name} {out/max(n,1)*100:.1f}%')
    print(f'  s{seed}@{f//1000}k: '+', '.join(res))
print('(3) readable sensor reads (>=1%) whose straight line to the sample crosses soil')
for sm in ('orig','allabove'):
    acc=[]
    for seed,f in [(1,200000),(2,100000),(3,200000),(4,295000)]:
        g=G.load(f'{G.BASE}/s{seed}/map_f{f:06d}.txt'); openm,sky,nest=H.masks_v(g,sm); geo=G.geodesic(openm,sky); cells=nest&(geo>=0)
        c=H.solve_v(openm,sky,H.srcs(seed,f,openm,1,.25,11,0),0); tot=0;cross=0
        for (y,x) in zip(*np.nonzero(cells)):
            for dy,dx in G.N8:
                s=6 if (dy==0 or dx==0) else 4; fy,fx=y+s*dy,x+s*dx
                if 0<=fy<g.shape[0] and 0<=fx<g.shape[1] and openm[fy,fx] and np.isfinite(c[fy,fx]) and np.isfinite(c[y,x]) and abs(c[fy,fx]-c[y,x])/(c[fy,fx]+c[y,x])>=0.01:
                    tot+=1; cross+=not all(openm[y+k*dy,x+k*dx] for k in range(1,s))
        acc.append(f's{seed}: {cross}/{tot}')
    print(f'  sink={sm}: '+', '.join(acc))
