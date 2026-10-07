import sys, numpy as np
sys.path.insert(0, sys.argv[1]); sys.argv=[sys.argv[0], sys.argv[1]]
import helpers as H; G=H.G
OPEN7=[(1,200000),(2,100000),(2,295000),(3,100000),(3,200000),(4,100000),(4,295000)]
rng=np.random.default_rng(1)
print('map        doorlvl/top  mound-share-of-top  near6  near20  (room cells, median share of own level)')
for seed,f in OPEN7:
    g=G.load(f'{G.BASE}/s{seed}/map_f{f:06d}.txt'); openm,sky,nest=H.masks_v(g,'orig'); geo=G.geodesic(openm,sky)
    cells=nest&(geo>=0); s=H.srcs(seed,f,openm,1,.25,11,0); c=H.solve_v(openm,sky,s,0)
    top=np.nanmax(np.where(cells,c,np.nan)); gc=np.where(cells,geo,10**9); dy,dx=np.unravel_index(gc.argmin(),gc.shape)
    sm=s.copy(); sm[G.GROUND_ROW+1:,:]=0; cm=H.solve_v(openm,sky,sm,0)
    ty,tx=np.unravel_index(np.nanargmax(np.where(cells,c,-1)),c.shape)
    from scipy import ndimage
    room=cells&(ndimage.uniform_filter(openm.astype(float),size=7)>0.7); ys,xs=np.nonzero(room)
    pick=rng.choice(len(ys),size=min(25,len(ys)),replace=False); n6=[];n20=[]
    for k in pick:
        y,x=ys[k],xs[k]; e=np.zeros_like(s); e[y,x]=1.0; gr=H.solve_v(openm,sky,e,0)  # Green's row (A symmetric)
        gr=np.nan_to_num(gr); tot=(gr*s).sum()
        yy,xx=np.mgrid[0:s.shape[0],0:s.shape[1]]; d=np.maximum(abs(yy-y),abs(xx-x))
        n6.append((gr*s*(d<=6)).sum()/tot); n20.append((gr*s*(d<=20)).sum()/tot)
    print(f's{seed}@{f//1000}k    {c[dy,dx]/top:6.3f}       {cm[ty,tx]/top:6.3f}            {np.median(n6):5.3f}  {np.median(n20):5.3f}')
print('--- s1, 150k..250k every 1000 frames, single-map sources (what a refresh sees)')
for wd in (0,.5):
    lv=[];sealed=0;nan_ok=0
    for f in range(150000,250001,1000):
        g=G.load(f'{G.BASE}/s1/map_f{f:06d}.txt'); openm,sky,nest=H.masks_v(g,'orig'); geo=G.geodesic(openm,sky)
        cells=nest&(geo>=0)
        if not cells.any(): lv.append(np.nan); sealed+=1; continue
        c=H.solve_v(openm,sky,H.srcs(1,f,openm,1,.25,1,0),wd); v=c[cells]
        if not np.isfinite(v).any(): lv.append(np.nan); sealed+=1; continue
        lv.append(float(np.nanmedian(v)))
    lv=np.array(lv); ok=np.isfinite(lv); r=np.abs(np.diff(np.log(lv)))
    r=r[np.isfinite(r)]
    print(f'diag={wd}: maps 101, sealed/door-shut {sealed}; nest median level min {np.nanmin(lv):.0f} med {np.nanmedian(lv):.0f} max {np.nanmax(lv):.0f} (max/min {np.nanmax(lv)/np.nanmin(lv):.1f}); 1000-frame change |dlog|: median {np.median(r):.3f}, p90 {np.quantile(r,.9):.3f}, max {r.max():.3f}; jumps >25%: {(r>np.log(1.25)).sum()} of {r.size}')
