# F1 robustness: keep the check's sink (open sky over nothing), but let loose spoil ABOVE the
# ground line pass gas at conductance eps (ground below stays sealed). Intermediate between the
# check (eps 0) and "everything above ground is outside".
import sys, numpy as np, scipy.sparse as sp, scipy.sparse.linalg as spla
from scipy.sparse.csgraph import connected_components
from scipy import ndimage
sys.path.insert(0, sys.argv[1]); sys.argv=[sys.argv[0], sys.argv[1]]
import helpers as H; G=H.G
def solve_p(openm,sky,src,por,eps):
    h,w=openm.shape; dom=(openm|por)&~sky; idx=-np.ones((h,w),np.int64); n=int(dom.sum()); idx[dom]=np.arange(n); diag=np.zeros(n);R=[];C=[];V=[]
    for dy,dx in G.N4:
        y,x,ny,nx=G.pairs(h,w,dy,dx); a=dom[y,x]&(openm[ny,nx]|por[ny,nx]); y,x,ny,nx=y[a],x[a],ny[a],nx[a]
        gg=np.where(openm[y,x]&openm[ny,nx],1.0,eps); k=gg>0; y,x,ny,nx,gg=y[k],x[k],ny[k],nx[k],gg[k]
        i=idx[y,x]; np.add.at(diag,i,gg); m=dom[ny,nx]; R.append(i[m]);C.append(idx[ny,nx][m]);V.append(-gg[m])
    A=sp.csr_matrix((np.concatenate(V),(np.concatenate(R),np.concatenate(C))),shape=(n,n)); sink=diag+np.asarray(A.sum(1)).ravel()
    nc,lab=connected_components(A,directed=False); ok=np.zeros(nc,bool); np.logical_or.at(ok,lab,sink>1e-12); keep=np.nonzero(ok[lab])[0]
    sol=np.full(n,np.nan); sol[keep]=spla.spsolve((A+sp.diags(diag)).tocsc()[keep][:,keep],src[dom][keep])
    c=np.full((h,w),np.nan); c[sky]=0; c[dom]=sol; return c
MAPS=[(1,100000),(1,200000),(1,295000),(2,100000),(2,200000),(2,295000),(3,100000),(3,200000),(3,295000),(4,100000),(4,295000)]
for eps in (0.0,0.02,0.1,0.3):
    rd=[];sp_=[];ro=[];nopen=0
    for seed,f in MAPS:
        g=G.load(f'{G.BASE}/s{seed}/map_f{f:06d}.txt'); openm,sky,nest=H.masks_v(g,'orig'); geo=G.geodesic(openm,sky); cells=nest&(geo>=0)
        por=(g=='s')&(np.arange(g.shape[0])[:,None]<G.GROUND_ROW)
        c=solve_p(openm,sky,H.srcs(seed,f,openm,1,.25,11,0),por,eps); cn=c[cells]
        if not np.isfinite(cn).any(): continue
        nopen+=1; best,bo=H.reads(np.where(openm,c,np.nan),openm,geo,6,4,'rel')
        rd.append(np.mean(best[cells]>=.01)); ro.append(np.mean(bo[cells]>=.01)); sp_.append((np.nanmax(cn)-np.nanmin(cn))/np.nanmax(cn))
    q=lambda v: f'{np.median(v)*100:.1f}% [{min(v)*100:.1f}-{max(v)*100:.1f}]'
    print(f'mound spoil conductance {eps}: maps with a way out {nopen}/11; room spans {q(sp_)}; reads >=1% {q(rd)}; >=1% and points out {q(ro)}')
