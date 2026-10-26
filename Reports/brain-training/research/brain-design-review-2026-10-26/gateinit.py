# Least-squares the gate's linear part onto the teacher's own fuel gate, from logged context + memory.
import sys, numpy as np
X=[];T=[];W=[]
for path in sys.argv[2:]:
    for i,l in enumerate(open(path)):
        if i>300000: break
        p=l.split(',')
        if len(p)<16 or p[1]!='Q' or p[6]=='2': continue
        ctx=[float(x) for x in p[11].split(' ')]; mem=[float(x) for x in p[12].split(' ')]
        c=ctx[0]; e=c/max(1-c,1e-6); dx=abs(int(p[4])-128)
        t=min(max(1-e/(1+0.012*dx),0),1)
        X.append(ctx+mem+[1.0]); T.append(t); W.append(1 if p[15].startswith('1|') else 10)
X=np.array(X);T=np.array(T);W=np.array(W,float)
y=np.where(T>0, T-1, -1.5)   # gate = clamp(1+y,0,2): off states pushed past the closed edge
sw=np.sqrt(W); w,*_=np.linalg.lstsq(X*sw[:,None], y*sw, rcond=None)
g=np.clip(1+X@w,0,2)
print("rows",len(T),"corr(gate,target) %.3f"%np.corrcoef(g,T)[0,1], "gate on off-states: mean %.3f, share >0.1: %.3f"%(g[T==0].mean(), (g[T==0]>0.1).mean()), "gate on on-states mean %.3f"%g[T>0].mean())
U=8; base=3285
with open(sys.argv[1],'w') as f:
    for i,v in enumerate(w):
        f.write(f"{base+U*35+i} {v:.6f}\n")
    f.write(f"{583+45*U+39} 3.0\n{1447+U} 20.0\n")   # f::HOME weight, output weight
