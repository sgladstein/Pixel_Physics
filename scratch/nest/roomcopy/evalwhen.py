# Per head: AUC of the when read-out for the rule's stop, and the median when by colony age, stop vs not (held-out seeds).
import numpy as np, glob, sys
P='/tmp/claude-0/-home-user-Pixel-Physics/bcf37ae4-04ca-5a03-8aeb-86b9f3a0ce2f/scratchpad/'
H,N=8,57;HL=H*N+3*H+2
def rh(p):
    L=open(p).read().split('\n'); n=int(L[0].split()[1]); g=np.zeros(n)
    for l in L[1:]:
        if l.strip(): i,x=l.split(); g[int(i)]=float(x)
    return g[2797:2797+HL]
ss=lambda x:x/(1+np.abs(x))
X,g,fr,V=[],[],[],[];off=0
for f in sorted(glob.glob(P+'cp/t_*_50001[01].csv')):
    d=np.load(f+'.npy'); _,gg=np.unique(d[:,0]*1e4+d[:,1],return_inverse=True); X.append(np.hstack([d[:,17:62],d[:,5:17]])); g.append(gg+off); off+=gg.max()+1; fr.append(d[:,0]); V.append(d[:,4]>0)
X,g,fr,V=np.vstack(X),np.concatenate(g),np.concatenate(fr),np.concatenate(V); G=off
stop=np.bincount(g,V,G)==np.bincount(g,None,G); F=np.zeros(G); F[g]=fr
def auc(s,y):
    o=np.argsort(s); r=np.empty(len(s)); r[o]=np.arange(len(s)); n1=y.sum(); n0=len(y)-n1
    return (r[y].sum()-n1*(n1-1)/2)/(n1*n0)
for p in sys.argv[1:]:
    v=rh(p); W=v[:H*N].reshape(H,N); b=v[H*N:H*N+H]; e=v[H*N+2*H+1:H*N+3*H+1]; c2=v[H*N+3*H+1]
    w=ss(X@W.T+b)@e; m=np.full(G,-np.inf); np.maximum.at(m,g,w); m+=c2
    s=f'{p.split("/")[-2]}/{p.split("/")[-1]}: AUC {auc(-m,stop):.2f};'
    for lo,hi in ((0,25000),(25000,50000),(50000,100000)):
        k=(F>=lo)&(F<hi); s+=f' {lo//1000}-{hi//1000}k go {np.median(m[k&~stop]):.1f} stop {np.median(m[k&stop]):.1f};'
    print(s)
