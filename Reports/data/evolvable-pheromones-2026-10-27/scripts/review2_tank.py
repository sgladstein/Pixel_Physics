import csv, glob, statistics as st, sys
for arm in sys.argv[2:]:
    T=[];C=[];L=[]
    for d in sorted(glob.glob(f'{sys.argv[1]}/{arm}/*/')):
        try: R=list(csv.DictReader(open(d+'stock.csv'))); B=list(csv.DictReader(open(d+'brood.csv'))); 
        except OSError: continue
        T.append(st.mean(float(r['tank_mean']) for r in R if int(r['frame'])>50000)); L.append(int(B[-1]['laid']))
        try: C.append(sum(1 for l in open(d+'events.csv') if ',field,' in l))
        except OSError: pass
    print(f'{arm:8} tank 2nd half {st.median(T):.2f} eggs laid {st.median(L):.0f} field pickups {st.median(C) if C else "n/a"}')
