import re,sys,glob
import os; D=os.path.dirname(os.path.abspath(__file__))+'/'
for arm in ['o0','o1','oB','oC']:
    leaf={k:0 for k in ['creeper','herb','shrub','tree']}; beds={k:0 for k in leaf}; roles=[]; deaths=[]; floor=[]; dark=[]; cov=[]
    for s in range(1,9):
        t=open(f'{D}{arm}_s{s}.txt').read().splitlines()
        end=[l for l in t if l.startswith('f=450000 ')][0]
        r=0
        for k in leaf:
            m=re.search(rf'\| {k} plants=\d+ bank=\d+ leaf=(\d+)',end); v=int(m.group(1)); leaf[k]+=v
            if v>=100: beds[k]+=1; r+=1
        roles.append(r)
        sen=set(re.search(r'id=(\d+)',l).group(1) for l in t if l.startswith('TREE') and 'senescent=true' in l)
        deaths.append(len(sen))
        g=[l for l in t if l.startswith('GAPS f=450000')][0].split()[2]
        vals=[10 if c=='+' else int(c) for c in g]
        floor.append(sum(vals)/len(vals)); dark.append(sum(v<2 for v in vals)/len(vals))
        cov.append(float(re.search(r'light_cov=([-\d.]+)',end).group(1)))
    n=8
    print(arm,'deaths',deaths,'leaf',leaf,'beds',beds,'roles %.2f'%(sum(roles)/n),'floor %.1f%%'%(sum(floor)/n),'under2 %.0f%%'%(100*sum(dark)/n),'cov %.3f'%(sum(cov)/n))
