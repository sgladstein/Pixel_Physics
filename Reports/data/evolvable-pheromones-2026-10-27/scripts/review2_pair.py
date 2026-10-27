import csv, glob, statistics as st, sys, os
def m(d):
    R=list(csv.DictReader(open(d+'stock.csv'))); B=list(csv.DictReader(open(d+'brood.csv'))); N=list(csv.DictReader(open(d+'nest.csv')))
    r=dict(stored=int(R[-1]['nest_food'])+int(R[-1]['door_food']), alive=int(R[-1]['alive']),
           tank=st.mean(float(x['tank_mean']) for x in R if int(x['frame'])>50000),
           hc=st.mean(int(x['alive']) for x in R), eggs=int(B[-1]['laid']), home=int(N[-1]['food_home']))
    try:
        st_=0; dl=0; fp=0; fd=0
        for l in open(d+'events.csv'):
            p=l.split(',')
            if len(p)<6: continue
            if p[2]=='death' and p[5].strip()=='1': st_+=1
            elif p[2]=='deliver': dl+=1
            elif p[2]=='field': fp+=1
            elif p[2]=='fdrop': fd+=1
        r.update(starved=st_, deliver=dl, netpick=fp-fd)
    except OSError: pass
    return r
root,a,b=sys.argv[1:4]
A={os.path.basename(d.rstrip('/')):m(d) for d in glob.glob(f'{root}/{a}/s*/')}
B={os.path.basename(d.rstrip('/')):m(d) for d in glob.glob(f'{root}/{b}/s*/')}
ks=sorted(set(A)&set(B))
for k in A[ks[0]]:
    if k not in B[ks[0]]: continue
    xa=[A[s][k] for s in ks]; xb=[B[s][k] for s in ks]
    print(f'{k:8} {a} {st.median(xa):8.1f}  {b} {st.median(xb):8.1f}   {a} higher on {sum(x>y for x,y in zip(xa,xb))}/{len(ks)}')
