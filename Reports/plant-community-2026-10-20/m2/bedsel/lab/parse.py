import re,sys,glob,statistics as st
import os; D=os.path.dirname(os.path.abspath(__file__))
def parse(f):
    rows=[]
    for line in open(f):
        if line.startswith('STOP frame='):
            kv=dict(re.findall(r'(\w+)=(-?\d+)',line))
            rows.append({k:int(v) for k,v in kv.items()})
    return rows
res={}
for arm in ('on','off'):
    for s in range(1,13):
        f=f'{D}/{arm}_s{s}.txt'
        r=parse(f)
        if not r or r[-1]['frame']!=500000: continue
        res[(arm,s)]=r
def m(r,k): return st.mean(x[k] for x in r[1:])
keys=[('ants','mean ants'),('plants','mean plants'),('edible','mean edible'),('leaf_j','mean leafJ')]
print('seed | '+' | '.join(f'{n} off/on' for _,n in keys)+' | ants@500k off/on | extinct off/on | starved off/on | born off/on')
agg={k:{'on':[],'off':[]} for k,_ in keys}
fin={'on':[],'off':[]}; ext={'on':0,'off':0}; stv={'on':[],'off':[]}; brn={'on':[],'off':[]}
better={k:[0,0] for k,_ in keys}
for s in range(1,13):
    if ('on',s) not in res or ('off',s) not in res: print(s,'missing'); continue
    a,b=res[('off',s)],res[('on',s)]
    cells=[]
    for k,_ in keys:
        x,y=m(a,k),m(b,k); agg[k]['off'].append(x); agg[k]['on'].append(y)
        if y>x: better[k][0]+=1
        elif y<x: better[k][1]+=1
        cells.append(f'{x:.0f}/{y:.0f}')
    def ex(r):
        z=[x['frame'] for x in r[1:] if x['ants']==0]; return z[0] if z else None
    for arm,r in (('off',a),('on',b)):
        fin[arm].append(r[-1]['ants']); stv[arm].append(r[-1]['starved']); brn[arm].append(r[-1]['born'])
        if ex(r) is not None: ext[arm]+=1
    print(s,'|',' | '.join(cells),'|',f"{a[-1]['ants']}/{b[-1]['ants']}",'|',f'{ex(a)}/{ex(b)}','|',f"{a[-1]['starved']}/{b[-1]['starved']}",'|',f"{a[-1]['born']}/{b[-1]['born']}")
n=len(fin['on'])
print('n paired',n)
for k,name in keys:
    print(f"{name}: off median {st.median(agg[k]['off']):.0f} on {st.median(agg[k]['on']):.0f}; on higher {better[k][0]}, lower {better[k][1]}")
print('ants@500k median off',st.median(fin['off']),'on',st.median(fin['on']),'; alive at 500k off',sum(1 for x in fin['off'] if x>0),'on',sum(1 for x in fin['on'] if x>0))
print('colonies extinct by 500k off',ext['off'],'on',ext['on'])
print('starved median off',st.median(stv['off']),'on',st.median(stv['on']),'; born median off',st.median(brn['off']),'on',st.median(brn['on']))
