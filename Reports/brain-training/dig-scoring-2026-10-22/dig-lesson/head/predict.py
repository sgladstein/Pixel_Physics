# How predictable is the old ant's cut cell from the head's inputs? Conditional logit over each decision's Dig options.
import sys, numpy as np, glob
S=sys.argv[1]
rows=[]
for li,path in enumerate(sorted(glob.glob(S+'/logs2/f-*.log'))):
    for line in open(path):
        f=line.rstrip('\n').split(',')
        if len(f)<17 or f[1]!='Q' or f[6]=='2' or not f[15].startswith('1|'): continue
        lab=[int(i) for i in f[15].split('|')[1].split(';')]
        X=[];S0=[];idx=[]
        for k,o in enumerate(f[13].split(';')):
            q=o.split(':',4)
            if q[0]!='3': continue
            x=np.zeros(57)
            for kv in q[4].split(' ') if q[4] else []:
                i,v=kv.split('='); i=int(i); x[i-100+45 if i>=100 else i]=float(v)
            X.append(x); S0.append(float(q[3])); idx.append(k)
        y=[j for j,k in enumerate(idx) if k in lab]
        if len(idx)<2 or not y: continue
        rows.append((li,np.array(X),np.array(S0),y,int(f[7]) in idx and idx.index(int(f[7])) in y))
print('decisions',len(rows),'mean dig options',np.mean([len(r[2]) for r in rows]))
K=0.05
def hcsp(s): w=np.maximum((s+1)/2,0); b=(K+w)**2; return b/b.sum()
print('hcs P(old cell)',np.mean([hcsp(r[2])[r[3]].sum() for r in rows]),'uniform',np.mean([len(r[3])/len(r[2]) for r in rows]))
tr=[r for r in rows if r[0] not in (5,11)]; te=[r for r in rows if r[0] in (5,11)]
mu=np.concatenate([r[1] for r in tr]).mean(0); sd=np.concatenate([r[1] for r in tr]).std(0)+1e-6
def P(w,r): z=((r[1]-mu)/sd)@w; z-=z.max(); e=np.exp(z); return e/e.sum()
w=np.zeros(57)
for it in range(300):
    g=np.zeros(57)
    for r in tr:
        p=P(w,r); xs=(r[1]-mu)/sd; t=np.zeros(len(p)); t[r[3]]=1/len(r[3]); g+=xs.T@(p-t)
    w-=0.5*g/len(tr)+1e-3*w
for name,rs in (('train',tr),('held out',te)):
    print(name,len(rs),'logit P(old cell)',round(np.mean([P(w,r)[r[3]].sum() for r in rs]),3),'top1',round(np.mean([np.argmax(P(w,r)) in r[3] for r in rs]),3),
          'hcs',round(np.mean([hcsp(r[2])[r[3]].sum() for r in rs]),3))
names=[f'f{i}' for i in range(45)]+['out','down','tdepth','sky','roof','nbopen','doordist','adepth','asky','aopen','level','bottom']
for i in np.argsort(-np.abs(w))[:14]: print(names[i], round(w[i],2))
