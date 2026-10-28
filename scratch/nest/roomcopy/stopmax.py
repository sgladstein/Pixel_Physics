# Can "c2 + max over cuts of e.softsign(Wx+b)" (the head's own when shape, h=8) predict the stop? Fresh fit, logistic loss.
import numpy as np, glob
P='/tmp/claude-0/-home-user-Pixel-Physics/bcf37ae4-04ca-5a03-8aeb-86b9f3a0ce2f/scratchpad/cp/'
def load(fs):
    X,G,Y=[],[],[]; off=0
    for f in fs:
        d=np.load(f+'.npy'); key=d[:,0]*1e4+d[:,1]; _,g=np.unique(key,return_inverse=True); n=g.max()+1
        V=d[:,4]>0; allr=np.bincount(g,V,n)==np.bincount(g,None,n)
        X.append(np.hstack([d[:,17:62],d[:,5:17]]).astype(np.float32)); G.append(g+off); Y.append(allr); off+=n
    return np.vstack(X),np.concatenate(G),np.concatenate(Y).astype(float)
X,g,Y=load(sorted(glob.glob(P+'t_*_50000[4-9].csv'))); Xt,gt,Yt=load(sorted(glob.glob(P+'t_*_50001[01].csv')))
m,sd=X.mean(0),X.std(0)+1e-6; X=(X-m)/sd; Xt=(Xt-m)/sd
H=8; rng=np.random.default_rng(0); W=rng.normal(0,.3,(H,57)); b=np.zeros(H); e=rng.normal(0,.3,H); c=0.
ss=lambda x:x/(1+np.abs(x)); dss=lambda x:1/(1+np.abs(x))**2
def fwd(X,g,G):
    z=X@W.T+b; u=ss(z); s=u@e; mx=np.full(G,-1e9); np.maximum.at(mx,g,s); arg=np.zeros(G,int)
    hit=s==mx[g]; idx=np.nonzero(hit)[0]; arg[g[idx]]=idx; return z,u,s,mx,arg
def auc(s,y):
    y=y.astype(bool); o=np.argsort(s); r=np.empty(len(s)); r[o]=np.arange(len(s)); n1=y.sum(); n0=len(y)-n1
    return (r[y].sum()-n1*(n1-1)/2)/(n1*n0)
G=g.max()+1; Gt=gt.max()+1; P_=[W,b,e]; mo=[np.zeros_like(p) for p in P_]+[0.]; vo=[np.zeros_like(p) for p in P_]+[0.]
for t in range(1,801):
    z,u,s,mx,arg=fwd(X,g,G); logit=-(c+mx)  # high when-read-out = dig; stop = low
    p=1/(1+np.exp(-logit)); dl=(p-Y)/G; dmx=-dl
    ga=np.zeros(len(X)); ga[arg]=dmx
    ge=u.T@ga; gz=np.outer(ga,e)*dss(z); gW=gz.T@X; gb=gz.sum(0); gc=dmx.sum()
    for i,(p_,gr) in enumerate(zip(P_+[None],[gW,gb,ge,gc])):
        mo[i]=0.9*mo[i]+0.1*gr; vo[i]=0.999*vo[i]+0.001*gr**2; step=0.03*(mo[i]/(1-.9**t))/(np.sqrt(vo[i]/(1-.999**t))+1e-8)
        if i<3: p_-=step
        else: c-=step
    if t%200==0:
        _,_,_,mxt,_=fwd(Xt,gt,Gt); print(t,'test AUC of -(when) for stop',round(auc(-mxt,Yt),3),flush=True)
