import csv, os, statistics as st, sys
from math import comb
# x3 exam, 600k after landing: paired sign tests over box x seed. usage: python3 x3an.py (from this folder)
R={(r['brain'],r['box'],int(r['seed'])): r for r in csv.DictReader(open('lab600k.csv'))}
boxes=["two_larders","scattered_larder","feast_and_famine","played_bed","herb_first_colony","crowded_bench"]
brains=sorted({k[0] for k in R})
print('rows',len(R),'brains',brains)
# share = starved / (founders + births); not meaningful in two_larders, where starved counts more than ever lived
def val(b,bx,s,key):
    r=R[(b,bx,s)]
    if key=='share': return int(r['starved'])/max(1,int(r['births'])+founders(bx))
    return float(r[key])
F={}
def founders(bx): return {'two_larders':104,'feast_and_famine':104}.get(bx,52 if bx!='crowded_bench' else 1000)
def v(b,key,bxs=boxes): return [val(b,bx,s,key) for bx in bxs for s in range(1,9)]
def sign(a,c):
    up=sum(x>y+1e-9 for x,y in zip(a,c)); dn=sum(x<y-1e-9 for x,y in zip(a,c)); n=up+dn; k=min(up,dn)
    p=1.0 if n==0 else min(1.0,2*sum(comb(n,i) for i in range(k+1))/2**n); return f'{up:2d}/{dn:2d} p={p:.2g}'
for bx in boxes:
    print('==',bx)
    for b in brains:
        print(f'  {b:5s} lived {st.median(v(b,"lived",[bx])):7.1f} alive>0 {sum(val(b,bx,s,"alive")>0 for s in range(1,9))}/8 share {st.median(v(b,"share",[bx])):.2f}')
pairs=[('famac9','famc7'),('famakc32','famc7'),('famc7','std'),('famac9','std'),('famakc32','std'),('g1tw','std'),('g1tw','c3p'),('std','old')]
for key in ('lived','alive','share'):
    print('###',key)
    for a,c in pairs: print(f'  {a:5s} vs {c:5s}: {sign(v(a,key),v(c,key))}')
print('### lived median/mean over 48')
for b in brains: print(f'  {b:5s} {st.median(v(b,"lived")):6.1f} {st.mean(v(b,"lived")):6.1f}')
