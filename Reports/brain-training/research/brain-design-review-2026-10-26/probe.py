# Linear probe: can this decision's context + memory tell "the teacher fires" (labelled) from the rest?
import sys, numpy as np
X=[];Y=[];W=[]
for path in sys.argv[1:]:
    for i,l in enumerate(open(path)):
        p=l.split(',')
        if len(p)<16 or p[1]!='Q' or p[6]=='2': continue
        ctx=[float(x) for x in p[11].split(' ')]; mem=[float(x) for x in p[12].split(' ')]
        X.append(ctx+mem); Y.append(1 if p[15].startswith('1|') else 0); W.append(1 if p[15].startswith('1|') else 10)  # unlabelled kept 1 in 40 vs labelled 1 in 4
X=np.array(X); Y=np.array(Y); W=np.array(W,float)
def auc(s,y,w):
    o=np.argsort(s); s,y,w=s[o],y[o],w[o]; pos=(w*y); neg=(w*(1-y))
    cn=np.cumsum(neg); return (pos*(cn-neg/2)).sum()/(pos.sum()*neg.sum())
rng=np.random.default_rng(1); idx=rng.permutation(len(Y)); tr,te=idx[:len(Y)*2//3],idx[len(Y)*2//3:]
def fit(F):
    mu,sd=F[tr].mean(0),F[tr].std(0)+1e-6; Z=(F-mu)/sd; Z=np.c_[Z,np.ones(len(Z))]; w=np.zeros(Z.shape[1])
    for it in range(400):
        p=1/(1+np.exp(-Z[tr]@w)); g=Z[tr].T@((p-Y[tr])*W[tr])/W[tr].sum(); w-=1.0*g
    return auc(Z[te]@w,Y[te],W[te])
names="energy crop spoil at_nest a b alarm d_a d_b d_alarm d_energy home_seen home_dist d_home since_nest age self crowd depth roofed roof_cut hole_drop".split()
print("rows",len(Y),"labelled",Y.sum())
print("all ctx+mem linear AUC %.3f"%fit(X))
print("energy alone %.3f"%fit(X[:,[0]]))
e=X[:,0]; far=X[:,9]; hd=X[:,12]
print("energy + far(d_alarm slot) + home_dist %.3f"%fit(X[:,[0,9,12]]))
# products / quadratic: does a nonlinear function of the same inputs add?
Q=np.c_[X[:,:22], X[:,:22]**2, X[:,0:1]*X[:,[9,12]] ]
print("ctx + squares + energy x distance %.3f"%fit(Q))
