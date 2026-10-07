# Experiment A: is "rooms are flat" an artifact of the method? Varies sources, window,
# exchange stencil, sky definition, threshold, sensor reach and reader. Imports gradcheck.
import sys, json, math
import numpy as np, scipy.sparse as sp, scipy.sparse.linalg as spla
from scipy.sparse.csgraph import connected_components
from scipy import ndimage
sys.path.insert(0, sys.argv[1]); import gradcheck as G
MAPS=[(1,100000),(1,200000),(1,295000),(2,100000),(2,200000),(2,295000),(3,100000),(3,200000),(3,295000),(4,100000),(4,295000)]
def masks_v(g, sky_mode):
    h,w=g.shape; rows=np.arange(h)[:,None]
    openm=np.isin(g,G.OPEN)
    if sky_mode=='plantsopen': openm=openm|((g=='#')&(rows<G.GROUND_ROW))
    wall=~openm; fw=np.where(wall.any(0),wall.argmax(0),h)
    sky=openm&(rows<G.GROUND_ROW) if sky_mode=='allabove' else openm&(rows<fw[None,:])&(rows<G.GROUND_ROW)
    return openm,sky,openm&(rows>G.GROUND_ROW)
def srcs(seed,f,openm,wa,wb,window,nomound):
    acc=np.zeros(openm.shape); n=0
    rng=range(-5000,5001,1000) if window==11 else [0]
    for df in rng:
        g=G.load(f'{G.BASE}/s{seed}/map_f{f+df:06d}.txt')
        if g.shape!=openm.shape: continue
        acc+=(g=='a')*wa+(g=='b')*wb; n+=1
    s=np.where(openm,acc/max(n,1),0.0)
    if nomound: s[:G.GROUND_ROW+1,:]=0.0
    return s
def solve_v(openm,sky,src,wdiag):
    h,w=openm.shape; unk=openm&~sky; idx=-np.ones((h,w),np.int64); n=int(unk.sum()); idx[unk]=np.arange(n)
    diag=np.zeros(n); R=[];C=[];V=[]
    nb=[(d,1.0) for d in G.N4]+([(d,wdiag) for d in G.N8[4:]] if wdiag>0 else [])
    for (dy,dx),wt in nb:
        y,x,ny,nx=G.pairs(h,w,dy,dx); a=unk[y,x]&openm[ny,nx]; y,x,ny,nx=y[a],x[a],ny[a],nx[a]
        i=idx[y,x]; np.add.at(diag,i,wt); m=unk[ny,nx]
        R.append(i[m]);C.append(idx[ny,nx][m]);V.append(np.full(int(m.sum()),-wt))
    A=sp.csr_matrix((np.concatenate(V),(np.concatenate(R),np.concatenate(C))),shape=(n,n))
    sink=diag+np.asarray(A.sum(1)).ravel()
    nc,lab=connected_components(A,directed=False); ok=np.zeros(nc,bool); np.logical_or.at(ok,lab,sink>1e-12)
    keep=np.nonzero(ok[lab])[0]; M=(A+sp.diags(diag)).tocsr()[keep][:,keep]
    sol=np.full(n,np.nan)
    if keep.size: sol[keep]=spla.spsolve(M.tocsc(),src[unk][keep])
    c=np.full((h,w),np.nan); c[sky]=0.0; c[unk]=sol; return c
def shift(a,dy,dx,fill):
    h,w=a.shape; o=np.full((h,w),fill,dtype=a.dtype)
    ys=slice(max(0,-dy),h-max(0,dy)); xs=slice(max(0,-dx),w-max(0,dx))
    yd=slice(max(0,dy),h-max(0,-dy) if max(0,-dy) else h); xd=slice(max(0,dx),w-max(0,-dx) if max(0,-dx) else w)
    o[ys,xs]=a[yd,xd]; return o
def reads(c,openm,geo,s_st,s_dg,reader):
    valid=openm&np.isfinite(c); best=np.full(c.shape,-1.0); bestout=np.full(c.shape,-1.0)
    for dy,dx in G.N8:
        s=s_st if (dy==0 or dx==0) else s_dg
        f=shift(np.where(valid,c,np.nan),s*dy,s*dx,np.nan); fg=shift(geo.astype(float),s*dy,s*dx,np.nan)
        with np.errstate(all='ignore'):
            r=np.abs(f-c)/(f+c) if reader=='rel' else np.abs(np.log(f/c))
        r=np.where(np.isfinite(r),r,-1.0); best=np.maximum(best,r)
        right=(f<c)&(fg<geo)&(fg>=0); bestout=np.maximum(bestout,np.where(right,r,-1.0))
    return best,bestout
VAR={'base':('orig',0,1,.25,11,0),'brood1':('orig',0,1,1,11,0),'antsonly':('orig',0,1,0,11,0),'broodonly':('orig',0,0,1,11,0),
 'nomound':('orig',0,1,.25,11,1),'window1':('orig',0,1,.25,1,0),'diag025':('orig',.25,1,.25,11,0),'diag05':('orig',.5,1,.25,11,0),
 'plantsopen':('plantsopen',0,1,.25,11,0),'allabove':('allabove',0,1,.25,11,0)}
READ={'r1':('rel',.01,6,4),'r05':('rel',.005,6,4),'log1':('log',.01,6,4),'r1_s3':('rel',.01,3,2),'r1_s10':('rel',.01,10,7)}
rows=[]
for seed,f in MAPS:
    g=G.load(f'{G.BASE}/s{seed}/map_f{f:06d}.txt')
    for vn,(sm,wd,wa,wb,win,nm) in VAR.items():
        openm,sky,nest=masks_v(g,sm); geo=G.geodesic(openm,sky); cells=nest&(geo>=0)
        if not cells.any(): continue
        c=solve_v(openm,sky,srcs(seed,f,openm,wa,wb,win,nm),wd)
        room=ndimage.uniform_filter(openm.astype(float),size=7)>0.7
        cn=c[cells]; sealed=not np.isfinite(cn).any()
        r={'seed':seed,'f':f,'var':vn,'sealed':sealed,'ncells':int(cells.sum())}
        if not sealed:
            fin=np.isfinite(cn); r['span']=float((np.nanmax(cn)-np.nanmin(cn))/np.nanmax(cn))
            for rn,(rd,thr,a,b) in READ.items():
                best,bo=reads(c,openm,geo,a,b,rd)
                r[rn]=float((best[cells]>=thr).mean()); r[rn+'_room']=float((best[cells&room]>=thr).mean()); r[rn+'_out']=float((bo[cells]>=thr).mean())
        rows.append(r)
with open(sys.argv[2],'w') as fh:
    for r in rows: fh.write(json.dumps(r)+'\n')
def q(v): v=sorted(v); return f"{np.median(v)*100:5.1f} [{v[0]*100:4.1f}-{v[-1]*100:4.1f}]" if v else '   -'
print('variant      maps  span%           r1 all          r1 room         r1 out-pointing  r05 all         log1 all        r1 sensor3      r1 sensor10')
for vn in VAR:
    rr=[r for r in rows if r['var']==vn and not r['sealed']]
    print(f"{vn:11s} {len(rr):2d}/{sum(1 for r in rows if r['var']==vn):2d} "+'  '.join(q([r[k] for r in rr]) for k in ('span','r1','r1_room','r1_out','r05','log1','r1_s3','r1_s10')))
