# Can the "stop" (every cut refused) be predicted per decision? Logistic regression on decision-level aggregates.
import numpy as np, glob
P='/tmp/claude-0/-home-user-Pixel-Physics/bcf37ae4-04ca-5a03-8aeb-86b9f3a0ce2f/scratchpad/cp/'
def dec(files):
    out=[]
    for i,f in enumerate(files):
        d=np.loadtxt(f,delimiter=',')
        key=d[:,0]*1e4+d[:,1]; _,g=np.unique(key,return_inverse=True); G=g.max()+1
        V=(d[:,4]>0).astype(float); allr=np.bincount(g,V,G)==np.bincount(g,None,G)
        fe=d[:,5:17]; ft=d[:,17:62]; extra=d[:,63:67]
        def agg(M):
            mx=np.full((G,M.shape[1]),-1e9); np.maximum.at(mx,g,M); mn=np.zeros((G,M.shape[1])); np.add.at(mn,g,M); mn/=np.bincount(g,None,G)[:,None]; return np.hstack([mx,mn])
        out.append((agg(np.hstack([fe,ft])),agg(extra),allr,np.bincount(g,None,G)))
    return out
def auc(s,y):
    o=np.argsort(s); r=np.empty(len(s)); r[o]=np.arange(len(s)); n1=y.sum(); n0=len(y)-n1
    return (r[y].sum()-n1*(n1-1)/2)/(n1*n0)
def fit(X,y,Xt,yt):
    m,sd=X.mean(0),X.std(0)+1e-6; X=(X-m)/sd; Xt=(Xt-m)/sd; w=np.zeros(X.shape[1]); b=0
    for _ in range(600):
        p=1/(1+np.exp(-(X@w+b))); g=X.T@(p-y)/len(y)+1e-3*w; w-=0.5*g; b-=0.5*(p-y).mean()
    return auc(Xt@w+b,yt)
tr=dec(sorted(glob.glob(P+'t_*_50000[4-9].csv'))); te=dec(sorted(glob.glob(P+'t_*_5000{10,11}.csv'.replace('{10,11}','1[01]'))))
cat=lambda L,i: np.vstack([x[i] for x in L]) if i<2 else np.concatenate([x[i] for x in L])
A,B,Y=cat(tr,0),cat(tr,1),cat(tr,2); At,Bt,Yt=cat(te,0),cat(te,1),cat(te,2); Nt=cat(te,3); N=cat(tr,3)
print('decisions train',len(Y),'test',len(Yt),'stop share test',Yt.mean())
print('n options only           AUC', round(fit(N[:,None].astype(float),Y,Nt[:,None].astype(float),Yt),3))
print('existing senses (agg)    AUC', round(fit(np.hstack([A,N[:,None]]),Y,np.hstack([At,Nt[:,None]]),Yt),3))
print('+ open runs through cut  AUC', round(fit(np.hstack([A,B,N[:,None]]),Y,np.hstack([At,Bt,Nt[:,None]]),Yt),3))
k=A.shape[1]//2
print('max over cuts only       AUC', round(fit(A[:,:k],Y,At[:,:k],Yt),3))
print('mean over cuts only      AUC', round(fit(A[:,k:],Y,At[:,k:],Yt),3))
print('12 dig senses, max+mean  AUC', round(fit(np.hstack([A[:,:12],A[:,k:k+12]]),Y,np.hstack([At[:,:12],At[:,k:k+12]]),Yt),3))
