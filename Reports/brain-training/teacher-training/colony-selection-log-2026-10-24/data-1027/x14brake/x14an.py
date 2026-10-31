import csv,sys,math
from collections import defaultdict
x13,x14=sys.argv[1:3]
R={}
for f in (x13,x14):
    for r in csv.DictReader(open(f)): R[(r['brain'],r['box'],r['seed'])]=r
def sign(a,b):
    n=a+b;k=min(a,b)
    if n==0: return 1.0
    p=sum(math.comb(n,i) for i in range(k+1))/2**n*2
    return min(1.0,p)
boxes=sorted({k[1] for k in R})
for cap,base in (('stdcap','std'),('gatecap','gate')):
    print(f'== {cap} vs {base} (braked vs unbraked)')
    keys=[(b,s) for b in boxes for s in map(str,range(1,9))]
    miss=[k for k in keys if (cap,)+k not in R or (base,)+k not in R]
    print('pairs',len(keys)-len(miss),'missing',miss)
    for m in ('alive_end','lived','starved_share','trip_deliveries','births'):
        w=l=0
        for b,s in keys:
            a=R[(cap,b,s)];u=R[(base,b,s)]
            def v(r):
                if m=='alive_end': return 1 if int(r['alive'])>0 else 0
                if m=='starved_share': return int(r['starved'])/max(1,int(r['births'])+int(r['landed']) if False else max(1,int(r['births'])))
                return float(r[m])
            x,y=v(a),v(u)
            if m=='starved_share': x,y=-x,-y
            w+=x>y; l+=x<y
        print(f'  {m:16s} braked better/worse {w}/{l} p {sign(w,l):.3g}')
    print('  per box alive-at-end (braked / unbraked, of 8), mean lived braked/unbraked:')
    for b in boxes:
        ac=sum(int(R[(cap,b,str(s))]['alive'])>0 for s in range(1,9))
        au=sum(int(R[(base,b,str(s))]['alive'])>0 for s in range(1,9))
        lc=sum(float(R[(cap,b,str(s))]['lived']) for s in range(1,9))/8
        lu=sum(float(R[(base,b,str(s))]['lived']) for s in range(1,9))/8
        bc=sum(float(R[(cap,b,str(s))]['births']) for s in range(1,9))/8
        bu=sum(float(R[(base,b,str(s))]['births']) for s in range(1,9))/8
        flag=' HARM' if au>=6 and ac<=2 else ''
        print(f'   {b:20s} {ac}/{au}  lived {lc:6.1f}/{lu:6.1f}  births {bc:6.0f}/{bu:6.0f}{flag}')
